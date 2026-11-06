"""Dark eco liquid textures from the owner's Jak 1 export (private, into src/generated).

Rust: assets::jak_model::jak1_texture (same roots), render_anim::minecraft_world::celestial_image
(alpha x2) and render_gpu drawsurf/minecraft_world.rs darkeco_fragment (the colour)."""
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

def main():
    water=jak1_texture('finalboss-vis-pris','cv-darkecowater')
    shine=jak1_texture('finalboss-vis-pris','environment-darkeco')
    # stand-in: the sphere-mapped shine depends on the view; Minecraft's fluid quads can't, so its
    # mean over the sphere map's disc is baked in.
    w,h=shine.size;disc=[shine.getpixel((x,y)) for y in range(h) for x in range(w) if (x+.5-w/2)**2+(y+.5-h/2)**2<=(w/2)**2]
    mean=[sum(p[i] for p in disc)/len(disc)/255 for i in range(3)]
    purple=(.42,.16,.58)
    def colour(p):
        r,g,b,a=p;a/=255
        # darkeco_fragment: texel x pool purple x 2, plus the shine x 2 x texel alpha; alpha 0.94.
        rgb=[min(1,(c/255)*purple[i]*2+mean[i]*2*a) for i,c in enumerate((r,g,b))]
        return tuple(round(c*255) for c in rgb)+(round(.94*255),)
    still=Image.new('RGBA',water.size);still.putdata([colour(p) for p in water.getdata()])
    # The flowing sprite is sampled over its first half (FluidRenderer), as vanilla's 2x flow sheets.
    flow=Image.new('RGBA',(water.width*2,water.height*2))
    for x in (0,water.width):
        for y in (0,water.height):flow.paste(still,(x,y))
    DEST.mkdir(parents=True,exist_ok=True)
    still.save(DEST/'dark_eco_still.png');flow.save(DEST/'dark_eco_flow.png')
    print('dark eco textures:',DEST,'mean shine',[round(c*255) for c in mean])

if __name__=='__main__':main()
