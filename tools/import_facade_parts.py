"""Facade parts, loose Jak objects and Jak 1 Sandover roofs from the source export. Writes only new files."""
from pathlib import Path
from import_facades import FACADES
import gzip,json,math,struct

PIECES={'pared','suelo','tejado','marco','escalera','escalones'}

def rotation(raw):
    # model.rs ElementRotation::parse/apply: single axis about `origin` (blocks), optional rescale.
    rot=raw.get('rotation')
    if not rot:return None
    origin=[v/16 for v in rot.get('origin',[0,0,0])]
    angle=math.radians(rot['angle']);s,c=math.sin(angle),math.cos(angle)
    m={'x':[[1,0,0],[0,c,s],[0,-s,c]],'y':[[c,0,-s],[0,1,0],[s,0,c]],'z':[[c,s,0],[-s,c,0],[0,0,1]]}[rot['axis']]
    if rot.get('rescale') and m!=[[1,0,0],[0,1,0],[0,0,1]]:
        m=[[v/max(abs(x) for x in col) for v in col] for col in m]
    def apply(p):
        d=[p[i]-origin[i] for i in range(3)]
        return [m[0][r]*d[0]+m[1][r]*d[1]+m[2][r]*d[2]+origin[r] for r in range(3)]
    return apply

def shaped_collision(model):
    # jak_facades.rs shaped_collision: every element's box, a turned one sliced in eighths
    # along its longest side; stand-in: cut to the cell (Minecraft only collides inside it).
    out=[]
    for e in model['elements']:
        lo0,hi0=[v/16 for v in e['from']],[v/16 for v in e['to']]
        apply=rotation(e)
        axis=max(range(3),key=lambda k:hi0[k]-lo0[k])
        slices=max(1,math.ceil((hi0[axis]-lo0[axis])*8)) if apply else 1
        for i in range(slices):
            lo,hi=list(lo0),list(hi0)
            lo[axis]=lo0[axis]+(hi0[axis]-lo0[axis])*i/slices
            hi[axis]=lo0[axis]+(hi0[axis]-lo0[axis])*(i+1)/slices
            points=[[hi[k] if bits>>k&1 else lo[k] for k in range(3)] for bits in range(8)]
            if apply:points=[apply(p) for p in points]
            a=[min(max(min(p[k] for p in points),0),1) for k in range(3)]
            b=[min(max(max(p[k] for p in points),0),1) for k in range(3)]
            if all(b[k]-a[k]>0.001 for k in range(3)):out.append([round(v*16,5) for v in a+b])
    return out

def loot(dest,path):
    target=dest/'data/jak/loot_table/blocks'/f'{path}.json'
    if target.exists():return
    table={'type':'minecraft:block','pools':[{'rolls':1,'entries':[{'type':'minecraft:item','name':'jak:'+path,
        'modifier':[{'type':'minecraft:explosion_decay'}]}]}]}
    target.write_text(json.dumps(table,separators=(',',':')),encoding='utf-8')

def new_file(path,text):
    path.write_text(text,encoding='utf-8')

def facade_parts(source,dest,catalog):
    # The 19 shaped parts the cell-sized authored path refused (rotated or overhanging boxes).
    rows=json.loads((catalog/'blocks.json').read_text(encoding='utf-8'))
    baked={name for name,_,_ in FACADES.values()}
    parts={}
    for row in rows:
        path=row['path']
        if row['ready'] or path.startswith('objeto_') or path in baked or row['look'].get('vanilla')!='carved_pumpkin':continue
        model=json.loads((source/'source-pack/assets/jak/models/block'/f'{path}.json').read_text(encoding='utf-8'))
        parts[path]=shaped_collision(model)
        loot(dest,path)
        out=dest/'assets/jak/models/block'/f'{path}.json'
        if not out.exists():
            # stand-in: the original draws its niche frame two blocks down (to -32/16);
            # vanilla cuboids stop at -16/16, so the lower course is left out.
            model['elements']=[e for e in model['elements'] if min(e['from'])>=-16 and max(e['to'])<=32]
            out.write_text(json.dumps(model,separators=(',',':')),encoding='utf-8')
    new_file(catalog/'facade_parts.json',json.dumps(parts,separators=(',',':')))
    return parts

def placement_kind(objects,kind):
    # jak_objects.rs placement_kind: old items of a kind place its corrected whole version.
    o=objects[kind];path=o['path']
    def find(name):return next(i for i,v in objects.items() if v['path']==name)
    if path.startswith('objeto_haven_bidon_'):return find('objeto_haven_bidon_fuego')
    if path=='objeto_spargus_planta_2' or o['group']=='palmera':return find('objeto_spargus_palmera_completa')
    if path in ('objeto_haven_propaganda','objeto_haven_propaganda_ajustada'):return find('objeto_haven_propaganda_bloque')
    hangs=any(t['texture'].startswith('city-slum-shrub-overhang') for c in o['cells'] for t in c['triangles'])
    if hangs and not path.startswith('objeto_haven_colgante_'):
        suffix=path.rsplit('_',1)[1]
        return next(i for i,v in objects.items() if v['path'].startswith('objeto_haven_colgante_') and v['path'].endswith('_'+suffix))
    return kind

def loose_objects(dest,catalog):
    objects={}
    for file in (catalog/'objects').glob('*.json'):
        data=json.loads(file.read_text(encoding='utf-8'));objects[data['id']]=data
    placed={int(i) for i in json.loads((catalog/'assemblies.json').read_text(encoding='utf-8'))}
    placed|=set(json.loads((catalog/'decorations.json').read_text(encoding='utf-8')))
    anchors,aliases={},{}
    for kind in sorted(objects):
        o=objects[kind]
        if kind in placed or o['group'] in PIECES or o['group'] in ('facade','track','rail'):continue
        target=placement_kind(objects,kind)
        if target!=kind:aliases[kind]=target;continue
        # jak_objects.rs placed: the box's middle over the block, its foot level with it.
        # stand-in: an assembly needs a real base cell there, so the nearest bottom cell.
        w,_,d=o['size'];cells=[tuple(c['cell']) for c in o['cells']]
        anchors[kind]=list(min(cells,key=lambda c:(c[1],(c[0]-w//2)**2+(c[2]-d//2)**2,c[0],c[2])))
        loot(dest,o['path'])
    new_file(catalog/'loose_objects.json',json.dumps(anchors))
    new_file(catalog/'object_aliases.json',json.dumps(aliases))
    return anchors,aliases

def read_blueprint(path):
    b=gzip.decompress(path.read_bytes());i=6
    (count,)=struct.unpack('>I',b[i:i+4]);i+=4;palette=[]
    for _ in range(count):
        (n,)=struct.unpack('>H',b[i:i+2]);i+=2;palette.append(b[i:i+n].decode('utf-8'));i+=n
    return b[:6],palette,b[i:]

def sandover_roofs(catalog):
    # jak_structures.rs decorate: Sandover only in Jak 1's lands, village_roofed with that land's
    # tiles and ridge; the plan is otherwise the same village (the export's flat land allows every roof).
    head,palette,rest=read_blueprint(catalog/'sandover.jbp')
    for name,tiles,ridge in (('sandover_snow','teja_nevada','cumbrera_nevada'),('sandover_swamp','teja_pantano','cumbrera_pantano')):
        out=bytearray(head);out.extend(struct.pack('>I',len(palette)))
        for state in palette:
            state=state.replace('jak:teja_roja[','jak:'+tiles+'[').replace('jak:cumbrera_roja[','jak:'+ridge+'[')
            encoded=state.encode('utf-8');out.extend(struct.pack('>H',len(encoded)));out.extend(encoded)
        out.extend(rest)
        (catalog/f'{name}.jbp').write_bytes(gzip.compress(bytes(out),mtime=0))

def export(source,dest):
    catalog=dest/'data/jakmc/catalog'
    parts=facade_parts(source,dest,catalog)
    anchors,aliases=loose_objects(dest,catalog)
    sandover_roofs(catalog)
    print(len(parts),'facade parts',len(anchors),'loose objects',len(aliases),'object aliases, Sandover snow/swamp roofs')

if __name__=='__main__':
    root=Path(__file__).resolve().parents[1]
    export(root/'.local/content',root/'src/generated/resources')
