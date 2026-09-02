#!/usr/bin/env python3
"""Write a short public-domain melody (Ode to Joy phrase) as 48 kHz S16 stereo WAV."""
import math
import struct
import sys
import wave

RATE = 48000
AMP = 0.38
# Beethoven Op. 125 theme is public domain. Not a square beep.
NOTES = [
    (659.25, 0.28),  # E5
    (659.25, 0.28),
    (698.46, 0.28),  # F5
    (783.99, 0.28),  # G5
    (783.99, 0.28),
    (698.46, 0.28),
    (659.25, 0.28),
    (587.33, 0.28),  # D5
    (523.25, 0.28),  # C5
    (523.25, 0.28),
    (587.33, 0.28),
    (659.25, 0.28),
    (659.25, 0.42),
    (587.33, 0.20),
    (587.33, 0.45),
]


def tone(freq, dur):
    n = int(RATE * dur)
    att = max(1, int(RATE * 0.012))
    rel = max(1, int(RATE * 0.05))
    out = []
    for i in range(n):
        s = math.sin(2.0 * math.pi * freq * i / RATE)
        if i < att:
            env = i / att
        elif i > n - rel:
            env = max(0.0, (n - i) / rel)
        else:
            env = 1.0
        out.append(s * AMP * env)
    return out


def main():
    path = sys.argv[1] if len(sys.argv) > 1 else "test.wav"
    samples = []
    for freq, dur in NOTES:
        samples.extend(tone(freq, dur))
    samples.extend([0.0] * int(RATE * 0.12))
    frames = bytearray()
    for s in samples:
        v = max(-1.0, min(1.0, s))
        i = int(v * 22000)
        frames.extend(struct.pack("<hh", i, i))
    with wave.open(path, "wb") as w:
        w.setnchannels(2)
        w.setsampwidth(2)
        w.setframerate(RATE)
        w.writeframes(bytes(frames))
    print("wrote %s (%d ms stereo 48k S16)" % (path, len(samples) * 1000 // RATE))


if __name__ == "__main__":
    main()
