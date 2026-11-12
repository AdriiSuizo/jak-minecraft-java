"""Dark eco liquid textures from the owner's Jak 1 export (private, into src/generated).

Rust: assets::jak_model::jak1_texture (same roots), render_anim::minecraft_world::celestial_image
(alpha x2)."""
import os, pathlib
import numpy as np
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

def sample(image,u,v):
    height,width,_=image.shape
    sx=(u%1)*width-.5;sz=(v%1)*height-.5
    ix=np.floor(sx).astype(np.int32);iz=np.floor(sz).astype(np.int32)
    fx=(sx-ix)[...,None];fz=(sz-iz)[...,None]
    top=image[iz%height,ix%width]*(1-fx)+image[iz%height,(ix+1)%width]*fx
    bottom=image[(iz+1)%height,ix%width]*(1-fx)+image[(iz+1)%height,(ix+1)%width]*fx
    return top*(1-fz)+bottom*fz


def surface_frames(water,reflection,size=128,frames=32):
    """Bake the silo pool's waves and reflected sky on an eight-block periodic sheet."""
    mask=np.asarray(water,dtype=np.float32)/255
    source=np.asarray(reflection.convert('RGB'),dtype=np.float32)/255
    z,x=(np.mgrid[:size,:size].astype(np.float32)+.5)*np.float32(8/size)
    images=[]
    for frame in range(frames):
        phase=frame*2*np.pi/frames

        def wave(x,z):
            return (.5208/128)*(25*np.cos(x*np.pi/2+phase)
                +15*np.cos((z-x)*np.pi/2-2*phase)+4*np.cos((5*x+3*z)*np.pi/2-3*phase))

        dx=wave(x+.5,z)-wave(x-.5,z)
        dz=wave(x,z+.5)-wave(x,z-.5)
        normal=np.stack((-6*dx,np.ones_like(x),-6*dz),axis=-1)
        normal/=np.linalg.norm(normal,axis=-1,keepdims=True)
        eye=np.array([.18,-.65,.738])
        ray=eye-2*(normal@eye)[...,None]*normal
        denom=2*np.sqrt(np.maximum(2+2*ray[:,:,1],.001))
        shine=sample(source,.5+ray[:,:,0]/denom,.5+ray[:,:,2]/denom)
        opacity=sample(mask,x/4+.08*np.sin(phase),z/4+.05*np.cos(phase))[:,:,3,None]
        colour=2.4*shine**2*opacity
        colour=np.clip(colour+np.array([.025,.012,.048]),0,1)
        rgba=np.empty((size,size,4),dtype=np.uint8)
        rgba[:,:,:3]=np.rint(colour*255).astype(np.uint8)
        rgba[:,:,3]=255
        images.append(rgba)
    return Image.fromarray(np.concatenate(images,axis=0))


def main():
    water=jak1_texture('finalboss-vis-pris','cv-darkecowater')
    shine=jak1_texture('finalboss-vis-pris','environment-darkeco')
    # As they are: the shader lays the pool on from the two (jakmc:dark_eco.glsl).
    DEST.mkdir(parents=True,exist_ok=True)
    water.save(DEST/'dark_eco_still.png');shine.save(DEST/'dark_eco_flow.png')
    surface_frames(water,shine).save(DEST/'dark_eco_surface.png')
    print('dark eco textures:',DEST)

if __name__=='__main__':main()
