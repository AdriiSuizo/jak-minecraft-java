"""Snapshot the owned Jak construction sources into this project's ignored lab.

Only the copied crate is made readable to the exporter. The reference project is
never edited or built in-place; the exported data has no old host at runtime.
"""
from pathlib import Path
import hashlib,json,re

ROOT=Path(__file__).resolve().parents[1]
SOURCE=ROOT.parent/'2010-rust-rewrite-mashup-trabajo'
DEST=ROOT/'.local/export-source/minecraft_terrain'

def put(path,data):
    path.parent.mkdir(parents=True,exist_ok=True)
    if not path.exists() or path.read_bytes()!=data:path.write_bytes(data)

def prepare():
    receipt=[]
    for source in (SOURCE/'crates/minecraft_terrain/src').rglob('*.rs'):
        data=source.read_bytes();receipt.append(dict(path=str(source.relative_to(SOURCE)),sha256=hashlib.sha256(data).hexdigest()))
        if source.name=='jak_town.rs':
            data=data.replace(b'pub(crate) palace:',b'pub palace:').replace(b'pub(crate) fn known_props_near',b'pub fn known_props_near')
            data+=b'''\npub fn export_foliage(seed:i64)->Vec<crate::jak_objects::Instance>{
 FOLIAGE.lock().ok().and_then(|held|held.as_ref().map(|cache|cache.iter().filter(|((s,_),_)|*s==seed).flat_map(|(_,made)|made.instances.iter().copied()).collect())).unwrap_or_default()
}\n'''
        if source.name=='jak_barrio.rs':
            for name in [b'layout',b'natural_layout',b'village_from_layout']:
                data=data.replace(b'pub(crate) fn '+name+b'(',b'pub fn '+name+b'(')
        if source.name=='jak_pieces.rs':data=data.replace(b'pub(crate) fn collision_boxes(&self)',b'pub fn collision_boxes(&self)')
        put(DEST/'src'/source.relative_to(SOURCE/'crates/minecraft_terrain/src'),data)
    cargo=(SOURCE/'crates/minecraft_terrain/Cargo.toml').read_text(encoding='utf-8')
    for asset in (SOURCE/'crates/minecraft_terrain/assets').rglob('*'):
        if asset.is_file():put(DEST/'assets'/asset.relative_to(SOURCE/'crates/minecraft_terrain/assets'),asset.read_bytes())
    cargo=re.sub(r'path = "(\.\./\.\./[^"]+)"',lambda m:'path = "'+str((SOURCE/'crates/minecraft_terrain'/m[1]).resolve()).replace('\\','/')+'"',cargo)
    cargo=cargo.replace('diag.workspace = true','diag = { path = "'+str(SOURCE/'crates/diag').replace('\\','/')+'" }')
    put(DEST/'Cargo.toml',cargo.encode())
    exporter=ROOT/'tools/content-exporter/Cargo.toml'
    text=exporter.read_text(encoding='utf-8').replace('../../../2010-rust-rewrite-mashup-trabajo/crates/minecraft_terrain','../content-source/terrain')
    put(exporter,text.encode())
    put(ROOT/'.local/export-source/provenance.json',json.dumps(receipt,indent=2).encode())
    print(f'Prepared isolated source snapshot: {len(receipt)} files; reference unchanged')

if __name__=='__main__':prepare()
