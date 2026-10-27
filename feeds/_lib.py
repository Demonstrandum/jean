"""Shared helpers for feeds. Files starting with _ are not run by the server.

A feed is a long-running script: print JSON objects (one per line) and the server turns each key into live data.
"""
import json, os, sys, time, urllib.request

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))


def config():
    try:
        with open(os.path.join(ROOT, "config.json")) as f:
            return json.load(f)
    except Exception as e:
        log("no config.json:", e)
        return {}


def emit(**kw):
    """emit(**{"wx.temp": 12.3}) or emit(d)"""
    print(json.dumps(kw), flush=True)


def put(d):
    print(json.dumps(d), flush=True)


def log(*a):
    print(*a, file=sys.stderr, flush=True)


def get_json(url, timeout=20):
    req = urllib.request.Request(url, headers={"User-Agent": "display-feed/1.0"})
    with urllib.request.urlopen(req, timeout=timeout) as r:
        return json.load(r)


# The server that started a feed supervises it, but if the server is killed its feeds would live on (every run and every test used to leave a
# full set behind, all polling the same APIs). A feed whose parent has changed has been orphaned: it stops.
_PARENT = os.getppid()


def orphaned():
    return os.getppid() != _PARENT


def nap(seconds):
    """Sleep, in short slices, and exit if the server that started this feed is gone."""
    end = time.time() + seconds
    while True:
        if orphaned():
            sys.exit(0)
        left = end - time.time()
        if left <= 0:
            return
        time.sleep(min(2, left))


def forever(period, fn, retry=60):
    """Call fn() every `period` seconds; on an exception log it and retry after `retry` seconds."""
    while True:
        if orphaned():
            sys.exit(0)
        try:
            fn()
            nap(period)
        except Exception as e:  # a feed must outlive a bad network moment
            log("error:", repr(e))
            nap(retry)


def secrets():
    """secrets/secrets.json (never in git, mode 600): API keys and account ids."""
    try:
        with open(os.path.join(ROOT, "secrets", "secrets.json")) as f:
            return json.load(f)
    except Exception as e:
        log("no secrets/secrets.json:", e)
        return {}
