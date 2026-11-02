#!/usr/bin/env python3
"""Compare intro loudness: the original's recorded audio against the port's master bus.

Both inputs are 16-bit mono WAV at one rate. Record the port with
ASTROLABE_INTRO_WAV set for tests/new_game_intro_acceptance.gd; extract the
original's from a session recording's audio.mka with ffmpeg.
  uv run --no-project --with numpy python tools/intro_loudness.py ORIGINAL PORT [ENTRY_SECONDS]
ENTRY_SECONDS is manoir entry in the original's audio, 38 for the Oct 3 take.
Voice waits run longer in the original, so only the first minute lines up.
"""
import sys
import wave
import numpy as np


def load(path):
    with wave.open(path) as source:
        return np.frombuffer(source.readframes(source.getnframes()), dtype=np.int16).astype(float)/32768, source.getframerate()


def seconds(samples, rate):
    return np.array([10*np.log10(np.mean(samples[i*rate:(i+1)*rate]**2)+1e-12) for i in range(len(samples)//rate)])


original, rate = load(sys.argv[1])
port, _ = load(sys.argv[2])
entry = int(sys.argv[3]) if len(sys.argv) > 3 else 38
a, b = seconds(original, rate)[entry:], seconds(port, rate)
for first, last, name in [(1, 13, 'music only'), (16, 19, 'voice 1'), (29, 33, 'voice 1a'), (39, 50, 'voice 1b')]:
    print(f'{name}: original {a[first:last].mean():.1f} dB, port {b[first:last].mean():.1f} dB')
print('second original port')
for second in range(12, 56):
    print(second, round(a[second]), round(b[second]))
