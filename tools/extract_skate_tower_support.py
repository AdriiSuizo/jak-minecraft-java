"""Regenerate the source7 support profile from the local original Jak II GLB.

Only low original palace surfaces plus the authored basament/corridor are
projected. This is generation metadata, not an approximation of collision.
The runtime adds visible footings and derives their collision from that mesh.
Usage: python tools/extract_skate_tower_support.py path/to/ctywide-background.glb
"""
import argparse
import json
import math
from pathlib import Path
import re
import struct


def clip(poly, axis, at, greater):
    out = []
    for a, b in zip(poly, poly[1:] + poly[:1]):
        inside_a = a[axis] >= at if greater else a[axis] <= at
        inside_b = b[axis] >= at if greater else b[axis] <= at
        if inside_a:
            out.append(a)
        if inside_a != inside_b:
            t = (at - a[axis]) / (b[axis] - a[axis])
            out.append(tuple(a[k] + (b[k] - a[k]) * t for k in range(3)))
    return out


def area(poly):
    cross = [0.0] * 3
    for i in range(1, len(poly) - 1):
        a = [poly[i][k] - poly[0][k] for k in range(3)]
        b = [poly[i + 1][k] - poly[0][k] for k in range(3)]
        c = (a[1]*b[2]-a[2]*b[1], a[2]*b[0]-a[0]*b[2], a[0]*b[1]-a[1]*b[0])
        cross = [cross[k] + c[k] for k in range(3)]
    return math.sqrt(sum(v*v for v in cross)) / 2


def f32(x):
    return struct.unpack('<f', struct.pack('<f', x))[0]


def extract(path, repo):
    raw = path.read_bytes()
    size = struct.unpack_from('<I', raw, 12)[0]
    gltf = json.loads(raw[20:20 + size])
    data = raw[28 + size:]

    def accessor(index):
        a = gltf['accessors'][index]
        view = gltf['bufferViews'][a['bufferView']]
        fmt = {5126: 'f', 5123: 'H', 5125: 'I', 5121: 'B'}[a['componentType']]
        count = {'SCALAR': 1, 'VEC2': 2, 'VEC3': 3, 'VEC4': 4}[a['type']]
        stride = view.get('byteStride', struct.calcsize(fmt) * count)
        offset = view.get('byteOffset', 0) + a.get('byteOffset', 0)
        return [struct.unpack_from('<' + fmt * count, data, offset + i * stride) for i in range(a['count'])]

    catalog = (Path(__file__).resolve().parent / 'content-source/terrain/src/spargus_city_blocks.rs').read_text(encoding='utf-8')
    catalog = catalog.split('pub const HAVEN_SLUMS_TEXTURES:', 1)[1].split('];', 1)[0]
    names = set(re.findall(r'"([^"]+)"', catalog))
    cells = {}

    def project(poly):
        poly = clip(poly, 1, 0, False)
        if len(poly) < 3:
            return
        for x in range(math.floor(min(p[0] for p in poly)), math.floor(max(p[0] for p in poly)) + 1):
            for z in range(math.floor(min(p[2] for p in poly)), math.floor(max(p[2] for p in poly)) + 1):
                q = poly
                for axis, at, greater in [(0, x, True), (0, x+1, False), (2, z, True), (2, z+1, False)]:
                    q = clip(q, axis, at, greater)
                if len(q) >= 3 and area(q) >= 1e-5:
                    cells[x, z] = min(cells.get((x, z), float('inf')), min(p[1] for p in q))

    pivot = list(map(f32, [189.56, 67.875, 599.72]))
    for mesh in gltf['meshes']:
        for primitive in mesh['primitives']:
            material = gltf['materials'][primitive['material']]['name'].removesuffix('-envmap')
            if material not in names:
                continue
            points = accessor(primitive['attributes']['POSITION'])
            indices = [v[0] for v in accessor(primitive['indices'])]
            for i in range(0, len(indices), 3):
                tri = [points[k] for k in indices[i:i+3]]
                centre = [sum(p[k] for p in tri)/3 for k in range(3)]
                if not (10 <= centre[0] <= 382 and 410 <= centre[2] <= 780 and 25 <= centre[1] <= 670):
                    continue
                if area(tri) < 5e-6:
                    continue
                tri = [tuple(f32(f32(p[k]-pivot[k])*.375) for k in range(3)) for p in tri]
                project(tri)

    ring = [(-58,-42),(58,-42),(62,-38),(62,38),(58,42),(-58,42),(-62,38),(-62,-38)]
    for i, a in enumerate(ring):
        b = ring[(i+1) % len(ring)]
        # Real floor, wall base and the low bottom of every authored pier.
        project([(0,-7,0),(b[0],-7,b[1]),(a[0],-7,a[1])])
        project([(a[0],-7,a[1]),(a[0],0,a[1]),(b[0],0,b[1]),(b[0],-7,b[1])])
        length = math.dist(a,b)
        count = math.ceil(length/8)
        outward = ((b[1]-a[1])/length,-(b[0]-a[0])/length)
        for n in range(count+1):
            p = [a[k]+(b[k]-a[k])*n/count+outward[k]*.12 for k in range(2)]
            project([(p[0]-.22,-7,p[1]-.22),(p[0]+.22,-7,p[1]-.22),
                     (p[0]+.22,-7,p[1]+.22),(p[0]-.22,-7,p[1]+.22)])
    # Only the porch outside the closed basament. Its indoor span is a bridge
    # above the circuit and must not become a solid wall inside skatea.
    project([(62,-1/4096,-12),(67,-1/4096,-12),(67,-1/4096,-8),(62,-1/4096,-8)])
    out = [float('inf')] * (160*160)
    for (x,z), y in cells.items():
        if -68 <= x < 92 and -80 <= z < 80:
            out[(z+80)*160+x+68] = y
        else:
            raise ValueError(f'support outside source7 footprint: {x},{z}')
    target = repo / 'support-f32le.bin'
    target.parent.mkdir(parents=True, exist_ok=True)
    target.write_bytes(struct.pack('<'+'f'*len(out), *out))
    print(f'{len(cells)} support columns; {sum(math.isfinite(y) and y != math.floor(y) for y in out)} fractional footings; {target.stat().st_size} bytes')


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('glb', type=Path)
    args = parser.parse_args()
    extract(args.glb, Path(__file__).resolve().parents[1])
