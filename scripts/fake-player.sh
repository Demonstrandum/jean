#!/bin/sh
# A stand-in for a WAV player (JEAN_PLAYER): keeps what the assistant "said" in $SPOKEN_DIR instead of playing it, and takes as long as the audio does.
mkdir -p "$SPOKEN_DIR"
n=$(ls "$SPOKEN_DIR" 2>/dev/null | wc -l | tr -d ' ')
cp "$1" "$SPOKEN_DIR/reply-$((n + 1)).wav"
secs=$(afinfo "$1" 2>/dev/null | awk '/estimated duration/ {print int($3) + 1}')
sleep "${secs:-2}"
