"""Read owned Jak II and Jak 3 GLB clocks; emit metadata, never copy retail model/texture bytes."""
from pathlib import Path
import argparse,hashlib,json,struct
ROOT=Path(__file__).resolve().parents[1]

def gun_muzzle(document,binary,model,seconds):
    import bisect
    import numpy as np

    def data(index,components):
        a=document['accessors'][index];v=document['bufferViews'][a['bufferView']]
        offset=v.get('byteOffset',0)+a.get('byteOffset',0)
        return np.ndarray((a['count'],components),dtype='<f4',buffer=binary,offset=offset,
                          strides=(v.get('byteStride',components*4),4))

    nodes=document['nodes']
    parents={child:i for i,n in enumerate(nodes) for child in n.get('children',[])}
    gun=next(i for i,n in enumerate(nodes) if n.get('name')=='gunblast')
    anim=next(a for a in document['animations'] if a['name']==model+'-shoot0')
    channels={(c['target']['node'],c['target']['path']):anim['samplers'][c['sampler']] for c in anim['channels']}

    def channel(i,path,default):
        sampler=channels.get((i,path))
        if sampler is None or nodes[i].get('name')=='align':
            return np.asarray(default,dtype=float)
        times=data(sampler['input'],1)[:,0]
        values=data(sampler['output'],len(default))
        right=bisect.bisect_right(times,seconds)
        if right==0:return values[0]
        if right==len(times):return values[-1]
        mix=(seconds-times[right-1])/(times[right]-times[right-1])
        a,b=values[right-1].astype(float),values[right].astype(float)
        if path=='rotation' and np.dot(a,b)<0:b=-b
        result=(1-mix)*a+mix*b
        return result/np.linalg.norm(result) if path=='rotation' else result

    def transform(i):
        n=nodes[i]
        raw=np.asarray(n.get('matrix',np.eye(4).T.ravel()),dtype=float).reshape(4,4).T
        pos=channel(i,'translation',n.get('translation',raw[:3,3]))
        if (i,'rotation') not in channels and (i,'scale') not in channels and 'matrix' in n:
            local=raw.copy()
        else:
            x,y,z,w=channel(i,'rotation',n.get('rotation',[0,0,0,1]))
            scale=channel(i,'scale',n.get('scale',np.linalg.norm(raw[:3,:3],axis=0)))
            local=np.eye(4)
            local[:3,:3]=np.array([[1-2*(y*y+z*z),2*(x*y-w*z),2*(x*z+w*y)],
                                  [2*(x*y+w*z),1-2*(x*x+z*z),2*(y*z-w*x)],
                                  [2*(x*z-w*y),2*(y*z+w*x),1-2*(x*x+y*y)]])*scale
        local[:3,3]=pos
        return transform(parents[i])@local if i in parents else local
    return [round(float(v),7) for v in transform(gun)[:3,3]]

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
  row=dict(model=model,asset=relative,clips=clips,**({"game":game} if game!="jak2" else {}))
  if model.startswith('wlander-'):
   # The original art-group event-shoot is .1 artist frame for male, 1 for female.
   row['shotMuzzle']=gun_muzzle(j,binary,model,(.1 if model=='wlander-male' else 1)/15)
  rows.append(row)
  art=root/f'raw_obj/{model}-ag.go'
  receipt.append(dict(model=model,glb_sha256=hashlib.sha256(data).hexdigest(),art_sha256=hashlib.sha256(art.read_bytes()).hexdigest(),clips=len(clips)))
 destination=ROOT/'src/main/resources/data/jakmc/citizens.json';destination.write_text(json.dumps(rows,indent=2)+'\n',encoding='utf-8')
 (ROOT/'.local/citizen-assets-receipt.json').write_text(json.dumps(receipt,indent=2),encoding='utf-8');print(json.dumps(receipt,indent=2))
if __name__=='__main__':main()
