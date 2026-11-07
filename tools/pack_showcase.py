"""Pack the world the `showcase` scene built: named, zipped for the release and copied to the installed game's saves."""
import argparse,gzip,os,pathlib,shutil,struct,zipfile

ROOT=pathlib.Path(__file__).resolve().parents[1]
NAME='Jak Showcase'

def rename(level):
    # LevelName's value in the NBT, swapped in place: the rest of the file is left as the game wrote it.
    data=gzip.decompress(level.read_bytes())
    key=b'\x08'+struct.pack('>H',9)+b'LevelName'
    at=data.index(key)+len(key)
    size,=struct.unpack('>H',data[at:at+2])
    name=NAME.encode('utf-8')
    data=data[:at]+struct.pack('>H',len(name))+name+data[at+2+size:]
    level.write_bytes(gzip.compress(data))

def run():
    parser=argparse.ArgumentParser();parser.add_argument('evidence',help='.local/evidence/showcase-<stamp>')
    parser.add_argument('--install',action='store_true')
    args=parser.parse_args()
    world=pathlib.Path(args.evidence)/'world'
    stage=ROOT/'dist/showcase'/NAME
    shutil.rmtree(stage.parent,ignore_errors=True)
    shutil.copytree(world,stage,ignore=shutil.ignore_patterns('session.lock','level.dat_old'))
    rename(stage/'level.dat')
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
