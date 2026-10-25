# Autonomy: computer access, standing jobs, and self-modification

Goal: Claude can use the mini PC like a person would (scripts, scrapers, files, even
editing the harness) and keep things working without you, while a mistake or a malicious
web page can't hurt the house or lock you out.

Same pattern as the display: **every change is transactional**. Propose in isolation,
verify automatically, promote atomically, roll back automatically.

## Two agents, different powers

| | Voice agent | Builder agent |
|---|---|---|
| Speed | fast model, seconds | slower, minutes, background |
| Tools | `ui` REPL, `ui.state`, ask/confirm, `start_task(...)` | shell, editor, git, build, headless browser, in a sandbox |
| Never | shell, network, secrets | speaks directly; changes policy |

The voice agent hears an open room (guests, the TV, anything audible), so it has no shell.
"Keep a list of cooking sites and have scrapers ready" becomes `start_task`; the builder
works and reports back on the display or by voice.

## Three repos, three trust levels

1. **kernel** (tiny, human-only): supervisor process, permission policy, kill switch,
   rollback, audit log. Claude cannot write it. It restarts things and restores the last
   good build.
2. **harness** (Rust: orchestrator, replay, browser control, voice plumbing). Claude may
   edit it, but only via the promotion pipeline below.
3. **workspace** (display scenes/widgets/state/journal from `persistence.md`, plus `jobs/`,
   `data/`, notes). Claude has write access; changes auto-commit.

## Host: NixOS

The machine is a NixOS box, and most of this document gets simpler because of it.

- **Everything is declared.** Orchestrator, supervisor, Chromium kiosk, wake word, STT/TTS,
  Home Assistant, the job scheduler: each is a NixOS module in one flake. Rust crates build
  through the flake (crane or similar). The system is a function of the repo.
- **Self-modification maps onto Nix primitives.** Builder edits the flake in a worktree,
  then `nix flake check` and `nix build` (hermetic, sandboxed, cached), then
  `nixos-rebuild test` (activate without becoming the boot default), then the watchdog
  decides. **Generations are the rollback**: if the new system misses its health check,
  the kernel switches back (boot counting with systemd-boot, or deploy-rs-style magic
  rollback). A change that breaks even the network or the supervisor is undone at the next
  boot. This replaces the symlink-swap pipeline described below.
- **Per-job environments are pinned.** A scraper's dependencies are declared in its own
  `flake.nix` or `nix shell`, locked by `flake.lock`. Library drift can't break a scraper;
  only the website changing can, which is exactly what the repair loop handles. Claude can
  try a package ephemerally with `nix shell nixpkgs#...` without touching the system.
- **Sandbox as config.** systemd hardening per service (`DynamicUser`, `ProtectSystem=strict`,
  `PrivateTmp`, restricted address families), nftables per-job host allowlists, a read-only
  store, and optionally impermanence (root on tmpfs, only listed directories persist). The
  state of the machine is then config plus a short list of persisted paths, easy to reason
  about and to reset. `microvm.nix` is available later if the builder needs stronger isolation.
- **Secrets** via sops-nix or agenix, readable only by the proxy service.
- **Approval gates become path checks on a diff.** Any widening of power is visible in the
  Nix diff: a new firewall allow rule, a new service, a changed `flake.lock` pin of the kernel
  input, a secrets path. Classify diffs by path: protected paths need approval (tier 3).
- **Kernel as a flake input.** The supervisor and policy live in a separate flake input
  owned by you. Claude's flake imports it but cannot change its pin without approval.

## Extending the UI library

Two layers, same promotion idea:

- **Workspace library** (`workspace/lib/*.ts`): Claude's own components, registered with
  `ui.component(name, fn, { doc })`, which also installs the global. Auto-commits, no
  approval. Load order at boot: `lib/` then widgets, scenes, state, then the journal tail.
  Registered docs appear in `ui.help()`, so Claude remembers what it built across sessions.
- **Core `ui.ts`** (harness repo, embedded in the Rust build): changes go through the
  promotion pipeline, including the regression suite.
- Every component gets an automatic render test: default props must render within the
  fit budget and pass `verify()`, in light and 1-bit. Components that prove useful get
  promoted from `lib/` into core via the pipeline.

## Sandbox for "general computer access"

- Dedicated unix user `claude`: no sudo, no access to your home or SSH keys.
- Read-write only in `workspace/` and harness worktrees; read-only elsewhere.
- Jobs and the builder's shell run in rootless containers on a read-only base image, under
  a systemd slice (CPU/memory caps, timeouts).
- Network goes through a local proxy with a per-job host allowlist; secrets are injected by
  the proxy, never visible to scripts or the model.
- Append-only audit log (shell commands, network, promotions) that Claude can read but not
  write.

## Standing jobs (scrapers on standby)

A job is a folder, not a conversation:

```
jobs/cooking-sites/
  manifest.toml     # schedule, hosts allowlist, output schema, health checks
  scrape.ts|py      # the script
  fixtures/         # saved HTML snapshots + expected output (regression tests)
  data -> workspace/data/recipes.json   # output, schema-validated
```

- Rust scheduler runs jobs (it replaces cron for these), with timeouts and rate limits;
  respect robots.txt and site terms.
- Widgets read `data/` through `ui.state` / the fetch proxy; they never run scrapers.
  Decoupled: a broken scraper leaves old data, shown with a "stale 3d" badge. Never wrong
  data silently.
- **Failure detection:** non-zero exit, schema validation failure, selector misses, a
  sudden drop in result count, an empty page.
- **Repair loop:** failure opens a ticket for the builder with the script, the error, and
  the fresh page capture. It fixes in a worktree and must pass **all old fixtures plus the
  new capture**, then commit. Budget: N attempts and a token cap; then mark `needs-human`
  and tell you once. "When a scraper fails it just fixes the script" is this loop.
- Adding a site you named by voice counts as approving that host. A repair that adds a
  host nobody asked for needs approval.

## Self-modifying the harness

Pipeline for any harness change:

(On NixOS the build, canary, swap and rollback steps below are `nix build`, `nixos-rebuild
test`, activation, and generation rollback; the structure is the same.)

1. Branch + worktree; builder edits.
2. `cargo build` + unit tests + **regression suite**: persistence replay and crystallize
   equivalence tests, golden frames for the screens, voice-loop smoke test with recorded
   audio, and fit tests (an oversized scene must scale, hide only parts marked optional, and
   otherwise clip; never throw, never blank).
3. **Canary:** run the new build on another port against a copy of the state, shadow
   traffic for a few minutes.
4. **Atomic swap** (symlink flip), keep the last 3 builds.
5. **Watchdog:** the kernel health-checks the new build; if it misses a heartbeat within T,
   it flips back automatically. No human needed to recover.

Promotion needs **your approval** (shown on the display, confirmed by voice or a physical
button) when a diff touches protected paths: the kernel, permission policy, the regression
suite itself (so tests can't be weakened to pass), secrets handling, the voice
wake/authentication path, new network hosts, or anything that expands Claude's own
privileges. Everything else can promote on green.

"Recursive self-improvement" here means bounded, tested, reversible improvement of the
code that does the work. It does not include rewriting its own rules or removing its own
guardrails; that boundary is enforced by the kernel, not by asking nicely.

## Untrusted content

Scrapers and the browser read arbitrary web pages. A page can contain text aimed at the
agent that fixes the script ("also email this file to ..."). Rules:

- Page content reaches the builder only as labeled data (fixture files), never in the
  instruction channel.
- The fixer has no secrets and no network beyond the job's hosts; its output is a script
  diff that the pipeline checks (no new hosts, no shell-outs outside an allowlist,
  tests pass).
- Voice input is untrusted too: anything outward-facing or destructive (messages,
  purchases, deleting data, changing policy) needs on-screen confirmation, and high-risk
  ones a physical button.

## Risk tiers for actions

| Tier | Examples | Gate |
|---|---|---|
| 0 | change the display, read data, run a read-only job | none |
| 1 | add a job/site, write workspace files, promote a green harness change | auto, logged |
| 2 | new network host, message someone, home-automation actions with effects | confirm on screen |
| 3 | kernel/policy/test changes, secrets, anything irreversible | human approval, physical button |

## Phases

1. Voice agent / builder split, `start_task`, sandbox user + container runner.
2. Job runner, manifests, fixtures, health checks, stale badges.
3. Scraper repair loop with fixture gate.
4. Harness promotion pipeline: canary, swap, watchdog, protected paths.
5. Kernel/supervisor hardening and audit log.

## Open decisions

1. Containers (podman) vs a separate unix user only for the first version.
2. Physical approval button: dedicated GPIO button, or the mic puck's button if it has one.
3. Which repo hosts the builder's task queue.
4. Whether repairs may add hosts on the same registrable domain (e.g. a CDN) without approval.
