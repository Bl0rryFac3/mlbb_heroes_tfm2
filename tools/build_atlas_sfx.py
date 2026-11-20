from math import exp, pi, sin
from pathlib import Path
from random import Random
from shutil import copyfile
from struct import pack
import wave

rate = 44100
rng = Random(507)
for event, duration in [('ejection', .45), ('reunion', .52)]:
    samples = bytearray()
    previous = 0
    for index in range(int(duration * rate)):
        t = index / rate
        noise = rng.uniform(-1, 1)
        click = noise - previous
        previous = noise
        if event == 'ejection':
            motor = sin(2*pi*(190*t + 820*t*t)) + .32*sin(2*pi*(380*t + 1640*t*t))
            sound = .25*motor*exp(-9*t) + .23*click*exp(-29*t)
        else:
            impact = sin(2*pi*(110*t + 13*(1-exp(-8*t))))
            ring = sin(2*pi*613*t) + .55*sin(2*pi*931*t)
            sound = .40*impact*exp(-17*t) + .11*ring*exp(-8*t) + .18*click*exp(-50*t)
        envelope = min(1, t/.003, (duration-t)/.035)
        samples.extend(pack('<h', int(32760 * envelope * sound)))
    path = Path(f'sound/sfx/mlbb_atlas_{event}.wav')
    with wave.open(str(path), 'wb') as output:
        output.setparams((1, 2, rate, 0, 'NONE', 'not compressed'))
        output.writeframes(samples)
    copyfile(path, path.with_name('mlbb_heroes_clip_'+path.name))
