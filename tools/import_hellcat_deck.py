"""Bake the source HellCat hull relief from an owned OpenGOAL export."""
import argparse
import json
import math
import struct
from pathlib import Path

CELL = .192
SCALE = .64

def rows(blob, document, index, fields):
    accessor = document['accessors'][index]
    view = document['bufferViews'][accessor['bufferView']]
    row = struct.Struct('<' + fields)
    stride = view.get('byteStride', row.size)
    start = view.get('byteOffset', 0) + accessor.get('byteOffset', 0)
    return [row.unpack_from(blob, start + i*stride) for i in range(accessor['count'])]

def source_mesh(path):
    data = path.read_bytes()
    _, _, _, size, kind = struct.unpack_from('<4sIIII', data)
    if kind != 0x4E4F534A:
        raise ValueError('Expected the JSON chunk of an OpenGOAL GLB')
    document = json.loads(data[20:20+size])
    blob = memoryview(data)[28+size:]
    meshes = document['meshes'][0]['primitives']
    verts = [tuple(p*SCALE for p in row) for row in rows(blob, document, meshes[0]['attributes']['POSITION'], 'fff')]
    surfaces = []
    for primitive in meshes:
        index = primitive['indices']
        component = document['accessors'][index]['componentType']
        surfaces.append([row[0] for row in rows(blob, document, index, {5123:'H',5125:'I'}[component])])
    return verts, surfaces

def damage_stages(raw, surfaces):
    # Same merc-effect fragment/triangle table as the original asset adapter.
    word = struct.Struct('<H')
    for header in range(0,len(raw)-40,4):
        count = struct.unpack_from('<I',raw,header)[0]
        fragments,triangles = struct.unpack_from('<HH',raw,header+8)
        if not 13<=count<=40 or fragments<count or triangles<50:
            continue
        for start in range(header+16,min(header+600,len(raw)-count*32),4):
            records = range(start,start+count*32,32)
            frags = [word.unpack_from(raw,record+18)[0] for record in records]
            if not all(0<n<=800 for n in frags) or sum(frags)!=fragments:
                continue
            tris = [word.unpack_from(raw,record+22)[0] for record in records]
            if sum(tris)!=sum(len(indices)//3 for indices in surfaces):
                continue
            stages = []
            effect,used = 0,0
            for indices in surfaces:
                while effect<count and used==tris[effect]:
                    effect+=1
                    used=0
                if effect>=count or used+len(indices)//3>tris[effect]:
                    break
                used+=len(indices)//3
                stages.append(2-effect//4 if effect<12 else 3)
            if len(stages)==len(surfaces) and effect==count-1 and used==tris[-1]:
                return stages
    raise ValueError('The HellCat merc-effect stage table did not match the GLB surfaces')

def relief(verts, indices):
    used = [verts[i] for i in set(indices)]
    min_x, min_y, min_z = map(min,zip(*used))
    max_x, max_y, max_z = map(max,zip(*used))
    nx, nz = math.ceil((max_x-min_x)/CELL), math.ceil((max_z-min_z)/CELL)
    top = [None]*(nx*nz)
    for ai,bi,ci in zip(indices[::3],indices[1::3],indices[2::3]):
        a,b,c = verts[ai],verts[bi],verts[ci]
        bx,bz = b[0]-a[0], b[2]-a[2]
        cx,cz = c[0]-a[0], c[2]-a[2]
        area = bx*cz-cx*bz
        if abs(area) < 1e-8:
            continue
        ix = math.ceil((min(a[0],b[0],c[0])-min_x)/CELL-.5)
        jz = math.ceil((min(a[2],b[2],c[2])-min_z)/CELL-.5)
        end_x = min(nx-1,math.floor((max(a[0],b[0],c[0])-min_x)/CELL-.5))
        end_z = min(nz-1,math.floor((max(a[2],b[2],c[2])-min_z)/CELL-.5))
        for j in range(max(jz,0),end_z+1):
            dz = min_z+(j+.5)*CELL-a[2]
            for i in range(max(ix,0),end_x+1):
                dx = min_x+(i+.5)*CELL-a[0]
                u,v = (dx*cz-cx*dz)/area,(bx*dz-dx*bz)/area
                if min(u,v,1-u-v) < -1e-4:
                    continue
                y = a[1]+(b[1]-a[1])*u+(c[1]-a[1])*v
                k = j*nx+i
                if top[k] is None or top[k]<y:
                    top[k]=y
    def adjacent(grid,i,j):
        return [grid[z*nx+x] if 0<=x<nx and 0<=z<nz else None
                for z in range(j-1,j+2) for x in range(i-1,i+2)]
    # Source hull closes narrow cockpit holes without lowering visible surfaces.
    dilated = [max((y for y in adjacent(top,k%nx,k//nx) if y is not None),default=None)
               for k in range(len(top))]
    for k,y in enumerate(top):
        around = adjacent(dilated,k%nx,k//nx)
        if None not in around:
            filled = min(around)
            top[k] = filled if y is None else max(y,filled)
    if not any(y is not None for y in top):
        raise ValueError('HellCat hull contains no collision surface')
    return dict(min=[min_x,min_y,min_z],max=[max_x,max_y,max_z],size=[nx,nz],cell=CELL,top=top)

if __name__ == '__main__':
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('export',type=Path)
    parser.add_argument('--helldog',action='store_true',help='Read the original black Hell Dog from a Jak II export')
    args=parser.parse_args()
    name,level=('helldog','lhelldog') if args.helldog else ('hellcat','factorya')
    vertices,surfaces=source_mesh(args.export/f'levels/{level}/{name}-lod0.glb')
    stages=damage_stages((args.export/f'raw_obj/{name}-ag.go').read_bytes(),surfaces)
    deck=relief(vertices,[i for part,stage in zip(surfaces,stages) if stage in (0,3) for i in part])
    deck['stages']=stages
    dest=Path(__file__).resolve().parents[1]/f'src/generated/resources/assets/jakmc/{name}/deck.json'
    dest.parent.mkdir(parents=True,exist_ok=True)
    dest.write_text(json.dumps(deck,separators=(',',':'))+'\n',encoding='utf-8')
    print(f"{deck['size'][0]}x{deck['size'][1]} cells, {sum(y is not None for y in deck['top'])} occupied")
