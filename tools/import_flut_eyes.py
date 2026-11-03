"""Bake the original Jak 1 Flut Flut eye layers for the Minecraft texture backend."""
from pathlib import Path
from PIL import Image
import sys

def bake(source, destination):
    iris, pupil, lid = [Image.open(source / (name + '.png')).convert('RGBA')
                        for name in ('flut-eye-16x16','autoeye-pupil','flut-eyelid')]
    eye = Image.new('RGB', (64, 64), iris.getpixel((0, 0))[:3])
    for texture, width in ((iris, 70), (pupil, 32)):
        start = (64-width)//2
        layer = texture.resize((width, width),Image.Resampling.NEAREST)
        mask = layer.getchannel('A').point(lambda a:min(255, round(a*255/128)))
        eye.paste(layer, (start,start), mask)
    destination.mkdir(parents=True,exist_ok=True)
    eye.save(destination/'eye_open.png')
    lid.convert('RGB').resize((64,64),Image.Resampling.NEAREST).save(destination/'eye_closed.png')

if __name__=='__main__':
    bake(Path(sys.argv[1]),Path(__file__).resolve().parents[1]/'src/generated/resources/assets/jakmc/flut')
