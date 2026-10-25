# Persistence: journal, commits, crystallize, resume

Goal: Claude's UI changes survive restarts, every accepted change is versioned, and the
history stays small and understandable. The browser is disposable; Rust owns durable state.

## Principle

The JS heap and DOM cannot be serialized, so we never try. **The accepted evals are the
source of truth** (event sourcing), and "crystallize" periodically rewrites that history
into clean source files. The DOM snapshot is only a cache.

Three kinds of state, kept apart on purpose:

| Kind | Lives in | Example |
|---|---|---|
| Code | `widgets/*.ts`, `scenes/*.ts` | `bus-times` widget, the ambient scene |
| Data | `state/*.json` (`ui.state`) | todo items, pinned stops, mode |
| Ephemeral layout | journal tail (evals) | "make that bigger", a one-off banner |

Claude is told: reusable logic goes in a named widget, mutable facts go in `ui.state`, and
raw evals are for one-offs. Crystallize is what moves ephemeral things into code and data.

## The display repo

A separate git repo on the mini PC (not the code monorepo), e.g. `~/display-state`:

```
scenes/ambient.ts      widgets/bus-times.ts      state/kv.json
journal/000142.ts      # accepted evals since the last checkpoint, one file each
snapshot.html          # normalized DOM at HEAD (cache + divergence check)
mode.json              # current scene + schedule
```

Single writer: the Rust orchestrator serializes evals. Backups: push to a private remote.
No secrets in the repo; widgets name a secret and Rust's fetch proxy injects it.

## Commit per verified eval

Flow in the orchestrator:

1. Write the eval to `journal/NNNNNN.ts` marked **pending** (survives a crash).
2. Send it with CDP `Runtime.evaluate { replMode: true }` (top-level `const` persists
   between evals, the way the REPL needs).
3. Call `ui.verify()`. Size is never a reason to reject: the screen makes every scene fit
   (scale, then hide parts marked optional, then clip), so `verify()` always succeeds and
   returns the fit report (`scale`, `dropped`, `clipped`, `natural`). Refresh
   `snapshot.html`, write state diffs, `git commit`, and hand the fit report back to Claude
   so it can trim the scene. Only a JavaScript **exception** in the eval makes the page undo
   everything the eval did (atomic); then delete the pending file and return the error.

Commit message = Claude's one-line intent. Trailers: `Utterance:`, `Turn:`, `Free-Y:`,
`Rejected-before: n`. One commit per eval keeps `git revert` and `git bisect` meaningful;
the `Turn:` trailer groups the several evals of one voice interaction.

Undo and time travel come for free: "undo that" = reset to `HEAD~1` and replay.
"What did the screen look like yesterday" = check out a commit in a scratch tab.

## Resume on restart

1. Recover: discard uncommitted junk; a **pending** journal file is replayed and committed
   if it verifies, dropped if not.
2. Launch Chromium; open the page served by Rust; inject the stripped `ui.js`.
3. Load the base: widgets, then scenes (`ui.defineWidget`, `ui.defineScene`), then
   `ui.state.load(kv.json)`.
4. Replay the journal tail in order, each entry through `verify()`. An entry that now
   fails is skipped, logged as `broken`, and reported to Claude. Boot never blocks.
5. Pick the scene from `mode.json`. Compare the frame hash with the last pushed frame and
   **only repaint the panel if it differs**, so a reboot doesn't cause an e-ink flash.
   The panel holds its image with no power, so a slow boot is invisible.
6. Report to the harness: "resumed: checkpoint cp-12 + 3 tail evals, 0 broken".

Replay rules:
- **Replay must be pure.** Anything with outside effects (Home Assistant calls,
  notifications) is refused while `ui.replaying` is true and is never part of the journal.
- Time-dependent output uses a clock the orchestrator controls: during replay and
  comparison, `Date`/`performance.now` are frozen to the entry's timestamp.
- Data fetching belongs to widgets (live by design), not to evals, so evals are mostly
  deterministic DOM construction plus `ui.state` writes.

## Timers and other non-DOM state (`setInterval` in the REPL)

The DOM journal can't undo timers, so they need their own rules.

- **Persisting:** nothing special. The eval that called `setInterval` is in the journal, so
  replay calls it again and the interval restarts. Its callback and the variables it closes
  over are rebuilt by the evals before it.
- **Rollback:** the orchestrator calls `ui.begin()` before each eval. Intervals created
  between `begin()` and `verify()` belong to that change and are cancelled if it is rolled
  back. Unrelated timers (background widgets, one-shot timeouts) are never touched.
  (Tracking every timer since the last commit was tried first; it cancelled unrelated
  timers, so attribution to a change is required.)
- **Duplicates:** re-evaluating "setInterval(update, 60000)" after tweaking `update` would
  stack two intervals. Prefer `ui.every(name, ms, fn)`: same name replaces, minimum 1s
  (e-ink doesn't need faster). `ui.timers()` and `ui.stop()` let Claude inspect and clear.
- **Runaway ticks:** a tick that makes the scene bigger is simply fitted again (scaled, or
  clipped at worst), so the screen stays valid. If `clipped` keeps coming back, the
  orchestrator should tell Claude so it can add `clamp()` or trim the widget.
- **Crystallize:** raw intervals in the tail are ephemeral. The agent turns them into a
  widget using `ui.every` (or `live()`), whose data comes from `ui.state`; the equivalence
  gate advances a virtual clock to check that the behaviour over time still matches.
- **Replay purity:** timer callbacks that fire later are live behaviour and may do effects;
  only code running during the replay loop itself is blocked from effects.

## Crystallize (compaction)

When: tail longer than N evals, nightly while the room is idle, or on request.

Run by the slow builder agent in a git worktree of the display repo:

1. Freeze an epoch: `base_seq = N`.
2. Agent reads base, tail, snapshot, `ui.outline()`, state. It rewrites scenes/widgets/state
   so that booting from the new base alone reproduces the display. Typical moves: extract
   repeated structure into widgets, replace literals with `ui.state` references, delete
   dead nodes.
3. **Equivalence gate** (orchestrator, scratch tab, frozen clock): boot A = old base + tail,
   boot B = new base alone. Compare normalized DOM and the dithered frame hash, with live
   regions masked. Then advance a virtual clock by +1h and compare again, so widgets that
   change over time are checked too. Also require `fits()`.
4. Equal: fast-forward `main`, tag `cp-<n>`, drop the tail. Not equal: the diff goes back to
   the agent as feedback; after a few attempts, give up and leave the tail untouched.
5. Evals that arrived during the run (`seq > base_seq`) stay in the tail and are replayed on
   top of the new base; if that doesn't match the live screen, abort.

The gate is what makes compaction safe: history can only be replaced by something proven
to produce the same display.

## Failure modes

| Failure | Result |
|---|---|
| Browser crash | restart, replay |
| Orchestrator crash mid-eval | pending file replayed on boot; verified or dropped |
| Power loss | panel keeps its frame; boot replays; repaint only if different |
| Replay entry breaks (font change, widget API change) | entry skipped, flagged; display falls back to last snapshot |
| Bad crystallize | gate rejects it; main untouched |
| Disk loss | restore from remote |
| Side effects re-fired on replay | prevented by `ui.replaying` |

## Library changes needed in `ui.ts`

- `ui.state`: key-value store with a change hook (Rust persists it).
- `ui.defineWidget(name, fn)` / `ui.widget(name, props)` and `ui.defineScene(name, fn)`.
- `ui.snapshot()`: normalized outerHTML (live regions marked `data-live` so they can be masked).
- `ui.replaying` flag; effectful APIs check it.
- Hook so the orchestrator is told on each commit point (`verify()` already is that point).

## Crates (monorepo)

- `store`: journal files, snapshot, state, git via the `git` CLI (simple, and worktrees are
  needed for crystallize).
- `replay`: boot sequence, CDP wiring, clock freeze, replay-with-verify.
- `crystallize`: worktree, equivalence gate, agent hand-off.

## Phases

1. Journal + commit per eval + snapshot + boot replay (no crystallize). Small, and already
   gives restarts and undo.
2. Widgets/scenes/state in the library, with the base loader.
3. Crystallize with the equivalence gate.
4. Time travel UI, remote backup, hermetic fetch recording if needed.

## De-risk result (done)

In the preview page: 5 evals run as separate classic scripts and verified; the one that
overflowed was rejected and not journaled; `const s` from eval 1 was used by evals 2 and 5.
Fresh page load, replay of the 4 journaled evals: 0 failures, DOM byte-identical.

## Open decisions

1. Commit per eval (recommended) vs per voice turn.
2. Where the display repo lives and whether it pushes to a remote.
3. How aggressively to auto-crystallize (tail length, nightly, only on request).
4. Retention: keep full journal history in git forever (cheap) or squash old epochs.

## Fit is structural, not a gate

An earlier version rejected scenes that overflowed and rolled them back, which left a blank
screen and an error. That is the wrong failure mode for a display. Now the screen contains
the scene by construction (`adapt()` in `ui.ts`):

1. Scale the whole scene down, never below 0.72 so text stays readable. Layout is re-flowed
   at the larger virtual width first, so text also gets more room per line.
2. Still too big: hide only the parts marked optional (`{priority: 1..9}`, lowest first).
   Unmarked content is never silently removed.
3. Still too big: clip at the screen edge.

The fit report goes to Claude (`scale`, `dropped`, `clipped`, `natural`) as information to
trim the scene, never as an error. `clamp(lines, text)` caps dynamic text; SVG widgets are
fluid (`max-width: 100%`), so they cannot spill out of their container.
