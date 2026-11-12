"""Pack the world the `showcase` scene built: named, zipped for the release and copied to the installed game's saves."""
import argparse,gzip,os,pathlib,shutil,struct,zipfile

ROOT=pathlib.Path(__file__).resolve().parents[1]
NAME='Jak Showcase'
INSTALLER_NAME='Jak Showcase (JakMod)'
PRIVATE=shutil.ignore_patterns('session.lock','level.dat_old','players','playerdata','advancements','stats')

def without_dev_pack(data):
    # NBT DataPacks.Enabled is a list of length-prefixed strings.
    key=b'\x0a\x00\x09DataPacks\x09\x00\x07Enabled\x08'
    if key not in data:return data
    start=data.index(key)+len(key)
    count,=struct.unpack_from('>I',data,start)
    end=start+4
    packs=[]
    for _ in range(count):
        size,=struct.unpack_from('>H',data,end)
        entry=data[end:end+2+size]
        if entry[2:]!=b'fabric-gametest-api-v1':packs.append(entry)
        end+=2+size
    return data[:start]+struct.pack('>I',len(packs))+b''.join(packs)+data[end:]

def rename_level(data,name):
    # LevelName's value in the NBT, swapped in place: the rest of the file is left as the game wrote it.
    data=gzip.decompress(data)
    if b'\x0a\x00\x06Player' in data:
        raise ValueError('Export a generated showcase world without embedded player data')
    key=b'\x08'+struct.pack('>H',9)+b'LevelName'
    at=data.index(key)+len(key)
    size,=struct.unpack('>H',data[at:at+2])
    name=name.encode('utf-8')
    data=data[:at]+struct.pack('>H',len(name))+name+data[at+2+size:]
    return gzip.compress(without_dev_pack(data),mtime=0)

def bundle_for_installer(source,target):
    """Convert the published showcase into a fresh, profile-specific copy."""
    with zipfile.ZipFile(source) as world,zipfile.ZipFile(target,'w',zipfile.ZIP_DEFLATED) as bundled:
        for member in world.infolist():
            root,*parts=member.filename.split('/')
            if root!=NAME or member.is_dir() or any(PRIVATE('',parts)):
                continue
            if not parts or any(part in ('','.','..') or '\\' in part or ':' in part for part in parts):
                raise ValueError(f'Unexpected showcase entry: {member.filename}')
            data=world.read(member)
            if parts==['level.dat']:
                data=rename_level(data,INSTALLER_NAME)
            bundled.writestr('/'.join(parts),data)
        bundled.getinfo('level.dat')

def run():
    parser=argparse.ArgumentParser();parser.add_argument('evidence',help='.local/evidence/showcase-<stamp>')
    parser.add_argument('--install',action='store_true')
    args=parser.parse_args()
    world=pathlib.Path(args.evidence)/'world'
    stage=ROOT/'dist/showcase'/NAME
    shutil.rmtree(stage.parent,ignore_errors=True)
    shutil.copytree(world,stage,ignore=PRIVATE)
    level=stage/'level.dat'
    level.write_bytes(rename_level(level.read_bytes(),NAME))
    archive=ROOT/'dist/JakShowcase-world.zip'
    with zipfile.ZipFile(archive,'w',zipfile.ZIP_DEFLATED,compresslevel=6) as out:
        for path in sorted(stage.rglob('*')):
            if path.is_file():out.write(path,path.relative_to(stage.parent).as_posix())
    print(archive,round(archive.stat().st_size/2**20,1),'MiB')
    if args.install:
        saves=pathlib.Path(os.environ['APPDATA'])/'.minecraft/instances/jakmod/saves'
        target=saves/NAME
        if target.exists():raise SystemExit(f'{target} already exists: left as it is')
        shutil.copytree(stage,target)
        print('Installed',target)

if __name__=='__main__':run()
