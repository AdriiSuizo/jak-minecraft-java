"""Build the bilingual Windows installer and RAR without any retail assets.

Prerequisites for maintainers: Java 25, Cargo, PyInstaller, Pillow, NumPy,
ffmpeg and WinRAR. End users only need Minecraft and their OpenGOAL games.
"""
from pathlib import Path
import hashlib, importlib.metadata, json, os, re, shutil, subprocess, sys, urllib.request, uuid, zipfile

ROOT=Path(__file__).resolve().parents[1]
OUT=ROOT/'dist'; STAGE=ROOT/'.local/installer-bundle'

def run(args):subprocess.run([str(x) for x in args],cwd=ROOT,check=True)
def copy(src,dst):dst.parent.mkdir(parents=True,exist_ok=True);shutil.copyfile(src,dst)
def sha(path):return hashlib.sha256(path.read_bytes()).hexdigest()

def personal_leaks(payload):
    """Nothing shipped may carry this machine's user name: a compiler writes build paths into what it makes."""
    name=os.environ['USERNAME'].lower().encode();hits=[]
    for path in payload.rglob('*'):
        if not path.is_file():continue
        if name in path.read_bytes().lower():hits.append(path.name)
        if path.suffix=='.jar':
            with zipfile.ZipFile(path) as jar:hits+=[entry for entry in jar.namelist() if name in jar.read(entry).lower()]
    if hits:raise SystemExit('Personal paths in the package: '+', '.join(hits[:20]))

def model_tokens(folders=('src/main','src/client','native/jak-core/src','native/jak-jni/src','tools/content-exporter/src','tools'),skip=('content-source',)):
    """What the installer keeps of the games' models: every word of these sources, and the "name-" pieces it builds names from."""
    words=set()
    for folder in folders:
        for path in (ROOT/folder).rglob('*'):
            if path.suffix not in ('.java','.rs','.py','.json','.toml','.txt') or not path.is_file():continue
            if any(part in ('target','__pycache__','build',*skip) for part in path.parts):continue
            text=path.read_text(encoding='utf-8',errors='ignore')
            words.update(w for w in re.findall(r'[a-z0-9][a-z0-9_-]{2,}',text) if not w.endswith('-'))
            words.update(m.rsplit('/',1)[-1] for m in re.findall(r'"([a-z0-9][a-z0-9_/-]*-)"',text) if len(m)>=4)
    return {w for w in words if os.environ['USERNAME'].lower() not in w}

def main():
    if sys.platform!='win32':raise SystemExit('The packaged native core currently targets Windows x86-64')
    props=dict(line.split('=',1) for line in (ROOT/'gradle.properties').read_text().splitlines() if '=' in line)
    # The binaries must not name this machine's folders: panics keep source paths, the linker the debug file's.
    os.environ['CARGO_ENCODED_RUSTFLAGS']=f'--remap-path-prefix={Path.home()}=~\x1f-Clink-arg=/PDBALTPATH:%_PDB%'
    run(['cargo','build','--release','--locked','--manifest-path','tools/content-exporter/Cargo.toml'])
    # Gradle takes its library as up to date: built here, with these flags.
    run(['cargo','build','--release','--locked','--manifest-path','native/Cargo.toml','-p','jak-jni'])
    run([ROOT/'gradlew.bat','distributionJar','--no-daemon','--console=plain'])
    payload=STAGE/'payload';payload.mkdir(parents=True,exist_ok=True)
    copy(ROOT/f'build/libs/jak-minecraft-java-{props["version"]}-installer-code.jar',payload/'jakmc-code.jar')
    copy(ROOT/'tools/content-exporter/target/release/jak-content-exporter.exe',payload/'jak-content-exporter.exe')
    (payload/'model-tokens.txt').write_text('\n'.join(sorted(model_tokens())),encoding='utf-8')
    (payload/'conversion-tokens.txt').write_text('\n'.join(sorted(model_tokens(('tools/content-source',),()))),encoding='utf-8')
    # These are redistributable codec utilities, never game files. Retain their
    # licenses and original notices along with the versions actually bundled.
    ffmpeg=Path(shutil.which('ffmpeg') or '')
    if not ffmpeg.is_file():raise SystemExit('ffmpeg is required')
    for f in ffmpeg.parent.iterdir():
        if f.suffix.lower()=='.dll' or f.name.lower()=='ffmpeg.exe':copy(f,payload/'ffmpeg'/f.name)
    for folder in ('licenses','doc'):
        if (ffmpeg.parent.parent/folder).is_dir():shutil.copytree(ffmpeg.parent.parent/folder,payload/'ffmpeg'/folder,dirs_exist_ok=True)
    for f in ffmpeg.parent.parent.glob('*'):
        if f.is_file() and any(x in f.name.lower() for x in ('license','copying','readme')):copy(f,payload/'ffmpeg'/f.name)
    decoder=ROOT/'.local/vgmstream'
    if not (decoder/'vgmstream-cli.exe').is_file():raise SystemExit('Prepare .local/vgmstream from the official vgmstream Windows release')
    for f in decoder.iterdir():
        if f.is_file() and (f.suffix.lower() in ('.dll','.exe','.md') or f.name=='COPYING'):copy(f,payload/'vgmstream'/f.name)
    manifest=dict(minecraft=props['minecraft_version'],loader=props['loader_version'],
        fabric_api=props['fabric_api_version'],mod_version=props['version'],platform='windows-x86_64')
    api=f'https://maven.fabricmc.net/net/fabricmc/fabric-api/fabric-api/{props["fabric_api_version"]}/fabric-api-{props["fabric_api_version"]}.jar.sha512'
    manifest['fabric_api_sha512']=urllib.request.urlopen(api,timeout=45).read().decode().strip()
    manifest['performance_mods']=json.loads((ROOT/'installer/performance-mods.json').read_text(encoding='utf-8'))
    manifest['files']={p.relative_to(payload).as_posix():sha(p) for p in sorted(payload.rglob('*')) if p.is_file() and p.name!='manifest.json'}
    (payload/'manifest.json').write_text(json.dumps(manifest,indent=2)+'\n',encoding='utf-8')
    # Keep a runnable source bundle for headless verification of exactly the
    # payload that PyInstaller packages.
    converters=['import_content','import_dark_eco','nombres_es','import_facades','import_facade_parts','import_flut_eyes','import_hellcat_deck','import_audio','extract_skate_tower_support']
    for name in converters:copy(ROOT/'tools'/f'{name}.py',STAGE/'converters'/f'{name}.py')
    catalog='content-source/terrain/src/spargus_city_blocks.rs'
    copy(ROOT/'tools'/catalog,STAGE/'converters'/catalog)
    args=[sys.executable,'-m','PyInstaller','--noconfirm','--clean','--onedir','--windowed',
        '--name','JakMod-Installer','--distpath',OUT,'--workpath',ROOT/'.local/pyinstaller',
        '--specpath',ROOT/'.local','--paths',ROOT/'tools','--add-data',f'{payload};payload',
        '--add-data',f'{ROOT/"tools"/catalog};content-source/terrain/src']
    for name in converters:args+=['--hidden-import',name]
    # Python imports used by the converters must be collected on an asset-free
    # build machine as well; they are third-party tools, not game assets.
    args+=['--hidden-import','PIL.Image','--hidden-import','numpy',ROOT/'installer/install.py']
    run(args)
    package=OUT/'JakMod-Installer'
    for name in ('README-ES.md','README-EN.md','THIRD-PARTY.md'):
        copy(ROOT/'installer'/name,package/name)
    notices=package/'licenses'
    for name in ('pyinstaller','Pillow','numpy'):
        distribution=importlib.metadata.distribution(name)
        for member in distribution.files or []:
            if any('license' in part.lower() or 'copying' in part.lower() for part in member.parts):
                path=Path(distribution.locate_file(member))
                if path.is_file():copy(path,notices/name/Path(*member.parts))
    python_license=Path(sys.base_prefix)/'LICENSE.txt'
    if python_license.is_file():copy(python_license,notices/'Python-LICENSE.txt')
    copy(ROOT/'LICENSE',package/'LICENSE')
    for name in ('opengoal','jak-enhanced-trilogy'):
        for notice in ('LICENSE','revision.txt'):copy(ROOT/'third_party'/name/notice,notices/name/notice)
    copy(ROOT/'CONTROLES.md',package/'CONTROLES.md')
    personal_leaks(payload)
    rar=Path(os.environ.get('ProgramFiles','C:/Program Files'))/'WinRAR/Rar.exe'
    if not rar.is_file():raise SystemExit('WinRAR is required to create a real RAR archive')
    archive=OUT/f'JakMod-{props["minecraft_version"]}-{props["version"]}.rar'
    temporary=OUT/(archive.stem+'-build-'+uuid.uuid4().hex+'.rar')
    run([rar,'a','-r','-ep1','-m3','-idq',temporary,package])
    run([rar,'t','-idq',temporary])
    os.replace(temporary,archive)
    receipt=dict(archive=archive.name,sha256=sha(archive),bytes=archive.stat().st_size,
        manifest=manifest,source_commit=subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT,text=True).strip())
    (OUT/'package-receipt.json').write_text(json.dumps(receipt,indent=2)+'\n',encoding='utf-8')
    print(json.dumps({k:v for k,v in receipt.items() if k!='manifest'},indent=2))

if __name__=='__main__':main()
