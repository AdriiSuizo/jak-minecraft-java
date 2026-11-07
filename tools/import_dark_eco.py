"""Dark eco liquid textures from the owner's Jak 1 export and the light eco icon from Jak 3 (private, into src/generated).

Rust: assets::jak_model::jak1_texture (same roots), render_anim::minecraft_world::celestial_image
(alpha x2)."""
import os, pathlib
from PIL import Image

ROOT=pathlib.Path(__file__).resolve().parent.parent
DEST=ROOT/'src/generated/resources/assets/jakmc/textures/block'

def jak1_texture(folder,name):
    roots=[os.environ.get('IW4L_JAK1_TEXTURES')]
    if os.environ.get('LOCALAPPDATA'):roots.append(os.path.join(os.environ['LOCALAPPDATA'],r'iw4l-mashup\jak1-proj\decompiler_out\jak1\textures'))
    if os.environ.get('USERPROFILE'):roots.append(os.path.join(os.environ['USERPROFILE'],r'Documents\Jak OpenGOAL\active\jak1\data\decompiler_out\jak1\textures'))
    for root in filter(None,roots):
        path=pathlib.Path(root)/folder/(name+'.png')
        if path.is_file():
            img=Image.open(path).convert('RGBA')
            # celestial_image: the PS2's alpha scale doubled.
            img.putdata([(r,g,b,min(255,a*2)) for r,g,b,a in img.getdata()])
            return img
    raise FileNotFoundError(f'{folder}/{name}.png: no Jak 1 texture export found')

def jak3_texture(folder,name):
    roots=[os.environ.get('IW4L_JAK3_TEXTURES')]
    if os.environ.get('USERPROFILE'):roots.append(os.path.join(os.environ['USERPROFILE'],r'Documents\Jak OpenGOAL\active\jak3\data\decompiler_out\jak3\textures'))
    for root in filter(None,roots):
        path=pathlib.Path(root)/folder/(name+'.png')
        if path.is_file():
            img=Image.open(path).convert('RGBA')
            img.putdata([(r,g,b,min(255,a*2)) for r,g,b,a in img.getdata()])
            return img
    raise FileNotFoundError(f'{folder}/{name}.png: no Jak 3 texture export found')

def main():
    water=jak1_texture('finalboss-vis-pris','cv-darkecowater')
    shine=jak1_texture('finalboss-vis-pris','environment-darkeco')
    # As they are: the shader lays the pool on from the two (jakmc:dark_eco.glsl).
    DEST.mkdir(parents=True,exist_ok=True)
    water.save(DEST/'dark_eco_still.png');shine.save(DEST/'dark_eco_flow.png')
    # Adrii 06/11: the light eco item drew as its white additive pill in the inventory, lost on the slot. Jak 3's
    # own inventory icon for it (progress-minimap2 inv-items-lighteco-01) instead, the PS2 alpha doubled.
    icon=jak3_texture('progress-minimap2','inv-items-lighteco-01')
    items=DEST.parent/'item';items.mkdir(parents=True,exist_ok=True);icon.save(items/'light_eco_icon.png')
    print('dark eco textures:',DEST)

if __name__=='__main__':main()
