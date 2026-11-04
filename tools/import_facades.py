"""Bake legacy facade cuboids into native cells without losing source UVs or rotations."""
import itertools
import json
import math

FACADES={
    108:("barrio_entramado","Entramado de Villa Refugio",True),
    109:("barrio_toldo","Toldo de Villa Refugio",True),
    110:("barrio_andamio","Andamio de Villa Refugio",False),
    111:("sparg_cuerno","Remate de Spargus",True),
    112:("barrio_portal_doble","Portal doble de Villa Refugio",True),
    113:("sparg_portal_doble","Portal doble de Spargus",True),
}
FACES={
    "north":((1,1,0),(1,0,0),(0,0,0),(0,1,0)),
    "south":((0,1,1),(0,0,1),(1,0,1),(1,1,1)),
    "west":((0,1,0),(0,0,0),(0,0,1),(0,1,1)),
    "east":((1,1,1),(1,0,1),(1,0,0),(1,1,0)),
    "up":((0,1,0),(0,1,1),(1,1,1),(1,1,0)),
    "down":((0,0,1),(0,0,0),(1,0,0),(1,0,1)),
}


def rotate(point, transform, inverse=False):
    if not transform:
        return point
    axis="xyz".index(transform["axis"])
    a,b=(axis+1)%3,(axis+2)%3
    origin=transform["origin"]
    angle=math.radians(transform["angle"]*(-1 if inverse else 1))
    sin,cos=math.sin(angle),math.cos(angle)
    x,y=point[a]-origin[a],point[b]-origin[b]
    result=list(point)
    result[a]=origin[a]+cos*x-sin*y
    result[b]=origin[b]+sin*x+cos*y
    return result


def clip(poly,axis,edge,sign):
    if not poly:
        return poly
    out=[]
    a=poly[-1]
    da=(a[axis]-edge)*sign
    for b in poly:
        db=(b[axis]-edge)*sign
        if (da>=-1e-8)!=(db>=-1e-8):
            t=max(0,min(1,da/(da-db)))
            out.append(tuple(x+(y-x)*t for x,y in zip(a,b)))
        if db>=-1e-8:
            out.append(b)
        a,da=b,db
    return out


def nonzero(triangle):
    a,b,c=triangle
    ab=[b[i]-a[i] for i in range(3)]
    ac=[c[i]-a[i] for i in range(3)]
    return any(abs(ab[i]*ac[(i+1)%3]-ab[(i+1)%3]*ac[i])>1e-7 for i in range(3))


def bake(source):
    cells={}
    for element in source["elements"]:
        lo,hi=element["from"],element["to"]
        transform=element.get("rotation")
        verts={corner:rotate([((hi if bit else lo)[i]) for i,bit in enumerate(corner)],transform)
               for corner in itertools.product(range(2),repeat=3)}
        low=[min(v[i] for v in verts.values()) for i in range(3)]
        high=[max(v[i] for v in verts.values()) for i in range(3)]
        ranges=[range(math.floor(a/16+1e-8),math.ceil(b/16-1e-8)) for a,b in zip(low,high)]
        faces=[]
        for side,face in element["faces"].items():
            u0,v0,u1,v1=face["uv"]
            uv=[(u0,v0),(u0,v1),(u1,v1),(u1,v0)]
            offset=face.get("rotation",0)//90
            texture=source["textures"][face["texture"].removeprefix("#")].split("/")[1]
            poly=[tuple(verts[corner])+uv[(i+offset)%4] for i,corner in enumerate(FACES[side])]
            faces.append((texture,poly))
        for cell in itertools.product(*ranges):
            origin=[v*16 for v in cell]
            triangles,solid=cells.setdefault(cell,([],set()))
            for texture,poly in faces:
                polygon=poly
                for i,edge in enumerate(origin):
                    polygon=clip(clip(polygon,i,edge,1),i,edge+16,-1)
                for i in range(1,len(polygon)-1):
                    triangle=(polygon[0],polygon[i],polygon[i+1])
                    if not nonzero(triangle):
                        continue
                    triangles.append({
                        "p":[[round(max(0,min(1,(point[axis]-origin[axis])/16)),6)
                              for axis in range(3)] for point in triangle],
                        "uv":[[round(v/16,6) for v in point[3:]] for point in triangle],
                        "color":[[128,128,128,128]]*3,"texture":texture,"blend":False})
            spans=[range(max(0,math.floor(low[i]-origin[i])),min(16,math.ceil(high[i]-origin[i])))
                   for i in range(3)]
            for voxel in itertools.product(*spans):
                p=rotate([origin[i]+voxel[i]+.5 for i in range(3)],transform,inverse=True)
                if all(lo[i]-.28<=p[i]<=hi[i]+.28 for i in range(3)):
                    solid.add(voxel)
    return {cell:data for cell,data in cells.items() if data[0] or data[1]}


def boxes(solid):
    out=[]
    for y,z in itertools.product(range(16),repeat=2):
        start=None
        for x in range(17):
            if (x,y,z) in solid:
                if start is None:
                    start=x
            elif start is not None:
                out.append([v/16 for v in (start,y,z,x,y+1,z+1)])
                start=None
    return out


def export_facades(source,objects):
    anchors={}
    for id,(name,title,wall) in FACADES.items():
        model=json.loads((source/f"{name}.json").read_text(encoding="utf-8"))
        cells=bake(model)
        if (0,0,0) not in cells:
            raise ValueError(f"No original anchor cell for {name}")
        low=[min(p[i] for p in cells) for i in range(3)]
        size=[max(p[i] for p in cells)-low[i]+1 for i in range(3)]
        anchors[id]=[-v for v in low]
        data=[{"cell":[p[i]-low[i] for i in range(3)],"triangles":triangles,"boxes":boxes(solid)}
              for p,(triangles,solid) in sorted(cells.items())]
        output={"id":id,"path":name,"title":title,"group":"facade","size":size,
                "wallMounted":wall,"soft":False,"cells":data}
        (objects/f"{id}.json").write_text(json.dumps(output,separators=(",",":")),encoding="utf-8")
        print(name,len(data),"facade cells")
    return anchors
