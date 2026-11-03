"""Build private Minecraft resources from the source export. No retail assets are tracked."""
from pathlib import Path
import gzip,hashlib,json,shutil,struct

ROOT=Path(__file__).resolve().parents[1]
SOURCE=ROOT/'.local/content'
DEST=ROOT/'src/generated/resources'

def packed_blueprint(data):
    out=bytearray(b'JAKB\x00\x01')
    out.extend(struct.pack('>I',len(data['palette'])))
    for state in data['palette']:
        encoded=state.encode('utf-8');out.extend(struct.pack('>H',len(encoded)));out.extend(encoded)
    out.extend(struct.pack('>I',len(data['blocks'])))
    for x,y,z,palette in data['blocks']:
        if not all(-32768<=v<32768 for v in (x,y,z)) or not 0<=palette<65536:raise ValueError('Blueprint field overflow')
        out.extend(struct.pack('>hhhH',x,y,z,palette))
    props=json.dumps(data.get('props',[]),separators=(',',':')).encode('utf-8')
    out.extend(struct.pack('>I',len(props)));out.extend(props)
    return gzip.compress(out,mtime=0)

def main():
    if not (SOURCE/'objects/104.json').is_file():raise RuntimeError('Source export is incomplete; run tools/content-exporter first')
    DEST.mkdir(parents=True,exist_ok=True)
    shutil.copytree(SOURCE/'source-pack/assets/jak',DEST/'assets/jak',dirs_exist_ok=True)
    white=Path.home()/'Documents/Jak OpenGOAL/active/jak3/data/decompiler_out/jak3/textures/common/common-white.png'
    particle=DEST/'assets/jakmc/textures/particle/common_white.png';particle.parent.mkdir(parents=True,exist_ok=True);shutil.copyfile(white,particle)
    catalog=DEST/'data/jakmc/catalog';catalog.mkdir(parents=True,exist_ok=True)
    shutil.copyfile(SOURCE/'blocks.json',catalog/'blocks.json')
    shutil.copytree(SOURCE/'objects',catalog/'objects',dirs_exist_ok=True)
    for name in ['haven','spargus','palace']:
        data=json.loads((SOURCE/f'{name}.json').read_text(encoding='utf-8'))
        (catalog/f'{name}.jbp').write_bytes(packed_blueprint(data))
        print(name,len(data['blocks']),'cells',len(data['props']),'whole props')
    receipts=[]
    for path in sorted(DEST.rglob('*')):
        if path.is_file():receipts.append(dict(path=str(path.relative_to(DEST)).replace('\\','/'),sha256=hashlib.sha256(path.read_bytes()).hexdigest(),size=path.stat().st_size))
    (ROOT/'.local/content-resource-receipt.json').write_text(json.dumps(receipts,indent=2),encoding='utf-8')
    print('Imported',len(receipts),'private resource files. Source geometry remains a build-time export, not a runtime dependency.')

if __name__=='__main__':main()
