"""Read owned Jak II and Jak 3 GLB clocks; emit metadata, never copy retail model/texture bytes."""
from pathlib import Path
import argparse,hashlib,json,struct
ROOT=Path(__file__).resolve().parents[1]
def main():
 p=argparse.ArgumentParser();p.add_argument('--source',type=Path,default=Path.home()/'Documents/Jak OpenGOAL/active/jak2/data/decompiler_out/jak2');args=p.parse_args();rows=[];receipt=[]
 sources=[('jak2',args.source,'lwidea',model) for model in ('citizen-norm','citizen-fat','citizen-chick')]
 jak3=args.source.parents[3]/'jak3/data/decompiler_out/jak3'
 sources+= [('jak3',jak3,level,model) for level,model in (('waswide','wlander-male'),('waswide','wlander-female'),('wasleapr','monk'))]
 for game,root,level,model in sources:
  relative=f'levels/{level}/{model}-lod0.glb';path=root/relative;data=path.read_bytes();magic,version,size=struct.unpack_from('<III',data)
  if magic!=0x46546c67 or version!=2 or size!=len(data):raise ValueError(f'Invalid GLB: {path}')
  at=12;j=None;binary=None
  while at<len(data):
   length,kind=struct.unpack_from('<II',data,at);chunk=data[at+8:at+8+length];at+=8+length
   if kind==0x4e4f534a:j=json.loads(chunk)
   elif kind==0x004e4942:binary=chunk
  clips={}
  for animation in j['animations']:
   duration=0
   for channel in animation['channels']:
    accessor=j['accessors'][animation['samplers'][channel['sampler']]['input']];view=j['bufferViews'][accessor['bufferView']]
    if accessor['componentType']!=5126 or accessor['type']!='SCALAR':raise ValueError('Expected float animation times')
    offset=view.get('byteOffset',0)+accessor.get('byteOffset',0);stride=view.get('byteStride',4)
    duration=max(duration,max(struct.unpack_from('<f',binary,offset+i*stride)[0] for i in range(accessor['count'])))
   clips[animation['name']]=duration
  rows.append(dict(model=model,asset=relative,clips=clips,**({"game":game} if game!="jak2" else {})))
  art=root/f'raw_obj/{model}-ag.go'
  receipt.append(dict(model=model,glb_sha256=hashlib.sha256(data).hexdigest(),art_sha256=hashlib.sha256(art.read_bytes()).hexdigest(),clips=len(clips)))
 destination=ROOT/'src/main/resources/data/jakmc/citizens.json';destination.write_text(json.dumps(rows,indent=2)+'\n',encoding='utf-8')
 (ROOT/'.local/citizen-assets-receipt.json').write_text(json.dumps(receipt,indent=2),encoding='utf-8');print(json.dumps(receipt,indent=2))
if __name__=='__main__':main()
