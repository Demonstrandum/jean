#!/usr/bin/env python3
"""A stand-in for sox's `rec`, for running the voice path without a microphone: streams the raw 16 kHz mono 16-bit PCM file named by $FAKE_MIC to stdout at
real-time pace (what the harness reads, in 100 ms chunks), then silence forever, exactly as an idle microphone would."""
import os, sys, time
RATE = 16000
CHUNK = RATE // 10 * 2
data = open(os.environ["FAKE_MIC"], "rb").read()
out = sys.stdout.buffer
t0 = time.monotonic()
i = 0
while True:
    chunk = data[i:i + CHUNK] if i < len(data) else b""
    chunk = chunk + b"\0" * (CHUNK - len(chunk))
    try:
        out.write(chunk)
        out.flush()
    except BrokenPipeError:
        break
    i += CHUNK
    time.sleep(max(0, t0 + i / (RATE * 2) - time.monotonic()))
