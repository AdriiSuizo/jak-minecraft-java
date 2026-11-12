"""Windows installer. All game content is read/converted on the owner's PC.

The same engine serves the bilingual GUI and unattended, isolated verification.
No Minecraft credentials are read, transmitted or included in a receipt.
"""
from pathlib import Path
import argparse, contextlib, datetime, hashlib, json, os, queue, re, shutil, struct
import subprocess, sys, tempfile, threading, time, traceback, urllib.error, urllib.request, uuid, webbrowser, zipfile
from urllib.parse import urlsplit

BASE = Path(os.environ.get('JAKMC_INSTALLER_BUNDLE', getattr(sys, '_MEIPASS', Path(__file__).resolve().parent)))
PAYLOAD = BASE / 'payload'
CREATE_NO_WINDOW = getattr(subprocess, 'CREATE_NO_WINDOW', 0)
SHOWCASE = 'Jak Showcase (JakMod)'
SHOWCASE_ARCHIVE = 'showcase-world.zip'

def words(es, en, lang): return es if lang == 'es' else en
def digest(path, kind='sha256'):
    h = hashlib.new(kind)
    with path.open('rb') as f:
        for b in iter(lambda:f.read(1024*1024), b''): h.update(b)
    return h.hexdigest()

def atomic(path, data):
    path.parent.mkdir(parents=True, exist_ok=True)
    temp = path.with_name(path.name + '.' + uuid.uuid4().hex + '.tmp')
    try:
        temp.write_bytes(data); os.replace(temp, path)
    finally: temp.unlink(missing_ok=True)

def write_json(path, data): atomic(path, (json.dumps(data, indent=2, ensure_ascii=False)+'\n').encode('utf-8'))

class InstallError(RuntimeError):
    """A message safe to display and share, with optional detail in the local log."""
    def __init__(self, message, detail=None):
        super().__init__(message)
        self.detail = detail

def fetch(url, dest, expected=None, kind='sha256'):
    if dest.is_file() and expected and digest(dest, kind) == expected: return
    req = urllib.request.Request(url, headers={'User-Agent':'JakMod-Installer/0.1'})
    dest.parent.mkdir(parents=True, exist_ok=True)
    temp = dest.with_name(dest.name+'.'+uuid.uuid4().hex+'.tmp')
    try:
        for attempt in range(3):
            try:
                with urllib.request.urlopen(req, timeout=60) as response, temp.open('wb') as output:
                    shutil.copyfileobj(response,output,length=1024*1024)
                break
            except (urllib.error.URLError, TimeoutError, ConnectionError) as failure:
                if attempt==2 or isinstance(failure,urllib.error.HTTPError) and failure.code not in (408,429,500,502,503,504):
                    raise
                time.sleep(attempt+1)
        if expected and digest(temp,kind)!=expected:
            raise InstallError('Download checksum mismatch.',url)
        os.replace(temp,dest)
    finally:
        temp.unlink(missing_ok=True)

def run(args, log, env=None):
    with log.open('ab') as output:
        result = subprocess.run([str(a) for a in args], stdout=output, stderr=subprocess.STDOUT,
                                env=env, creationflags=CREATE_NO_WINDOW)
    if result.returncode: raise InstallError(f'{Path(args[0]).name} (exit {result.returncode}).',str(log))

def minecraft_default(): return Path(os.environ.get('APPDATA', Path.home())) / '.minecraft'

def normalize_opengoal(path):
    path = Path(path).expanduser().resolve()
    candidates = [path, path/'active', path/'OpenGOAL'/'active', path/'games'/'active']
    if path.name in ('jak1','jak2','jak3'): candidates.insert(0, path.parent)
    if path.name == 'data': candidates.insert(0, path.parent.parent)
    return next((p for p in candidates if all((p/g/'data').is_dir() for g in ('jak1','jak2','jak3'))), None)

def opengoal_default():
    candidates = [Path.home()/'Documents/Jak OpenGOAL',
        Path(os.environ.get('APPDATA', Path.home()))/'OpenGOAL-Launcher',
        Path(os.environ.get('LOCALAPPDATA', Path.home()))/'OpenGOAL-Launcher']
    # Launcher settings can contain a custom installation folder. Only path
    # values are inspected; the document is never logged or transmitted.
    for root in candidates[1:]:
        try:
            config = json.loads((root/'settings.json').read_text(encoding='utf-8'))
            def paths(v):
                if isinstance(v, dict):
                    for x in v.values(): yield from paths(x)
                elif isinstance(v, list):
                    for x in v: yield from paths(x)
                elif isinstance(v, str) and (':\\' in v or ':/' in v): yield Path(v)
            candidates.extend(paths(config))
        except (OSError, ValueError): pass
    return next((p for p in candidates if normalize_opengoal(p)), candidates[0])

def export_root(active, game): return active/game/'data/decompiler_out'/game

EXTRACTION_GB = {'jak1':12,'jak2':26,'jak3':31}

REQUIRED = {
 'jak1':['levels/beach/flutflut-lod0.glb', 'textures/beach-vis-pris/flut-eye-16x16.png',
         'levels/common/eichar-lod0.glb','levels/common/sidekick-lod0.glb','raw_obj/eichar-ag.go',
         'textures/common/autoeye-pupil.png','textures/beach-vis-pris/flut-eyelid.png'],
 'jak2':['levels/ctywide/ctywide-background.glb','levels/lwidea/citizen-norm-lod0.glb'],
 'jak3':['levels/factorya/hellcat-lod0.glb','raw_obj/gun-ag.go',
         'textures/progress-minimap/hud-skullgem.png']}

def model_tokens(*lists):
    """Every word of the mod's sources (tools/build_installer.py): a model it can name. A word ending in - starts a name it builds."""
    words_ = set()
    for name in lists: words_.update((PAYLOAD/name).read_text(encoding='utf-8').split())
    return words_, tuple(w for w in words_ if w.endswith('-'))

def needed_model(name, tokens, prefixes):
    stem = name.split('-lod0')[0]
    # "flutflutegg-lod0.glb" written whole is the word flutflutegg-lod0.
    return stem in tokens or name.removesuffix('.glb') in tokens or stem.startswith(prefixes)

def slim_models(root, game, lists=('model-tokens.txt',)):
    """Each game holds some 10 GB of models the mod never loads: all but those it names go. Backgrounds stay for the conversion."""
    tokens, prefixes = model_tokens(*lists)
    required = {Path(name).name for name in REQUIRED[game]}
    for path in (root/'levels').rglob('*.glb'):
        name = path.name
        if name.endswith('-background.glb') or name in required: continue
        if '-lod0' not in name or not needed_model(name, tokens, prefixes): path.unlink()

def slim_audio(root):
    """import_audio reads audio/sfx and the ISO; the streams ripped beside it were 14 GB of Jak II alone."""
    for path in (root/'audio').iterdir():
        if path.is_dir() and path.name!='sfx': shutil.rmtree(path)
        elif path.is_file(): path.unlink()

def slim_converted(root):
    """What only the conversion read."""
    resolved = root.resolve()
    for folder in ('audio', 'entities', 'collision'):
        target = root/folder
        if target.is_dir() and not target.is_junction() and not target.is_symlink() and target.resolve().is_relative_to(resolved):
            shutil.rmtree(target)
    for path in (root/'levels').rglob('*-background.glb'): path.unlink()

def owned_games(canonical):
    return [path for path in (canonical/g for g in REQUIRED)
            if path.is_dir() and not path.is_junction() and not path.is_symlink()
            and path.resolve().is_relative_to(canonical.resolve())]

def conversion_stamp(manifest):
    """Use the same converter identity in the source bundle and the frozen EXE."""
    parts = sorted((k,v) for k,v in manifest['files'].items() if k not in {'jakmc-code.jar',SHOWCASE_ARCHIVE})
    parts += sorted(manifest.get('converters',{}).items())
    return hashlib.sha256(json.dumps(parts).encode()).hexdigest()

def junction(target, source):
    # A local directory junction requires no elevation and does not change the
    # source. Keeping the canonical active layout preserves native sibling reads.
    import _winapi
    target.parent.mkdir(parents=True, exist_ok=True)
    if target.is_junction() and target.resolve()!=source.resolve(): target.rmdir()
    if target.exists():
        if target.resolve() != source.resolve(): raise ValueError('Existing asset link differs: '+str(target))
    else: _winapi.CreateJunction(str(source.resolve()), str(target))

def prepare_owned(active, workspace, notify, lang):
    canonical = workspace/'owned/active'
    # The biggest first: the disk's fullest moment is one whole game beside the others already cut.
    for game in ('jak3','jak2','jak1'):
        source = active/game
        root = export_root(active, game)
        complete = all((root/name).is_file() for name in REQUIRED[game])
        complete = complete and (root/'audio/sfx').is_dir() and (source/'data/goal_src'/game).is_dir()
        target = canonical/game
        if complete:
            junction(target, source)
            continue
        if target.is_junction() or target.is_symlink():
            raise InstallError(words(f'Cambió la exportación de {game}. Vuelve a seleccionar la carpeta de OpenGOAL.',
                f'The {game} export changed. Select the OpenGOAL folder again.',lang))
        cached=export_root(canonical,game)
        if (target.is_dir() and not target.is_junction() and not target.is_symlink()
            and all((cached/name).is_file() for name in REQUIRED[game]) and (cached/'audio/sfx').is_dir()
            and any((cached/'levels').rglob('*-background.glb'))):
            # Resume an interrupted installation after successful extraction.
            # The canonical directory belongs exclusively to this instance.
            continue
        # The OpenGOAL Launcher keeps it by version, next to active: versions/official/v0.3.8/extractor.exe.
        versions = sorted((active.parent/'versions').glob('*/*/extractor.exe'), key=lambda p: p.stat().st_mtime, reverse=True)
        extractor = next((p for p in (active/'bin/extractor.exe', source/'extractor.exe',
                          source/'bin/extractor.exe', *versions) if p.is_file()), None)
        iso = source/'data/iso_data'/game
        if not extractor or not iso.is_dir() or not (source/'data/decompiler/config').is_dir():
            raise InstallError(words(
                f'Faltan exportaciones de {game}. Instala los tres juegos en OpenGOAL y selecciona su carpeta (debe contener extractor.exe, data/iso_data y data/decompiler/config).',
                f'{game} exports are missing. Install all three games in OpenGOAL and select their installation folder (extractor.exe, data/iso_data and data/decompiler/config are required).', lang))
        # One game whole took up to 29 GB in a clean test: better said here than halfway through.
        if shutil.disk_usage(workspace).free<EXTRACTION_GB[game]*10**9:
            raise InstallError(words(f'Libera al menos {EXTRACTION_GB[game]} GB en el disco de Minecraft para extraer {game}.',
                f'Free at least {EXTRACTION_GB[game]} GB on the Minecraft drive to extract {game}.',lang))
        notify(words(f'Extrayendo {game} desde tu instalación. Puede tardar varios minutos…',
                     f'Extracting {game} from your installation. This may take several minutes…', lang))
        data = target/'data'; data.mkdir(parents=True, exist_ok=True)
        for folder in ('decompiler', 'goal_src'):
            if (source/'data'/folder).is_dir(): shutil.copytree(source/'data'/folder, data/folder, dirs_exist_ok=True)
        # Config and game input remain in the selected install. Extraction output
        # goes only to the private Minecraft instance, never back to OpenGOAL.
        override = dict(decompile_code=False, process_tpages=True, process_art_groups=True,
            dump_objs=True, levels_extract=True, rip_levels=True, extract_collision=True,
            save_texture_pngs=True, rip_streamed_audio=True, allowed_objects=[], banned_objects=[])
        try:
            run([extractor, iso, '--folder','--decompile','--game',game,'--proj-path',data,
                 '--decomp-config-override',json.dumps(override),'--disable-ansi'], workspace/f'extract-{game}.log')
        except InstallError:
            # Audio comes last, and Windows refuses a Jak II sound named SHIP_GUN_HIT-*: the extractor stops there with
            # the art done. import_audio reads the named banks from the ISO data. Anything else missing fails just below.
            if not (export_root(canonical, game)/'audio/sfx').is_dir(): raise
        missing = [name for name in REQUIRED[game] if not (export_root(canonical, game)/name).is_file()]
        if missing: raise InstallError(words(f'La extracción de {game} está incompleta.',f'The {game} extraction is incomplete.',lang),', '.join(missing))
        # Game by game: what the conversion names as well stays, so the three never lie on the disk whole together.
        slim_models(export_root(canonical, game), game, ('model-tokens.txt','conversion-tokens.txt'))
        slim_audio(export_root(canonical, game))
    return canonical

REQUIRED_RESOURCES = (
    'assets/jakmc/sounds.json','assets/jakmc/hellcat/deck.json','assets/jakmc/helldog/deck.json',
    'assets/jakmc/flut/eye_open.png','assets/jakmc/textures/block/dark_eco_still.png',
    'assets/jakmc/textures/block/dark_eco_surface.png',
    'data/jakmc/catalog/haven.jbp','data/jakmc/catalog/spargus.jbp','data/jakmc/catalog/palace.jbp',
    'data/jakmc/catalog/objects/104.json',
)

def import_resources(active, workspace, notify, lang, stage=lambda _:None):
    sys.path.insert(0, str(BASE/'converters'))
    import extract_skate_tower_support as support
    import import_content as content
    import import_flut_eyes as eyes
    import import_hellcat_deck as deck
    import import_audio as audio
    output = workspace/'src/generated/resources'
    (workspace/'.local').mkdir(parents=True, exist_ok=True)
    root3 = export_root(active, 'jak3'); root2 = export_root(active, 'jak2')
    notify(words('Adaptando ciudades, bloques y decoración…','Converting cities, blocks and decoration…',lang))
    support.extract(root2/'levels/ctywide/ctywide-background.glb', workspace)
    env = os.environ.copy()
    env['JAKMC_SUPPORT_PROFILE'] = str(workspace/'support-f32le.bin')
    env['JAKMC_SKULL_ICON'] = str(root3/'textures/progress-minimap/hud-skullgem.png')
    env['MINECRAFTOSS_ROOT'] = str(workspace)
    for game in ('jak1','jak2','jak3'): env['IW4L_'+game.upper()+'_ASSETS'] = str(export_root(active,game))
    run([PAYLOAD/'jak-content-exporter.exe',workspace/'.local/content'], workspace/'content.log',env)
    content.ROOT=workspace; content.SOURCE=workspace/'.local/content'; content.DEST=output
    content.main()
    # The dark eco liquid's textures (06/11: never made by the installer, the liquid drew as missing).
    import import_dark_eco as dark
    dark.DEST=output/'assets/jakmc/textures/block'
    os.environ['IW4L_JAK1_TEXTURES']=str(export_root(active,'jak1')/'textures')
    dark.main()
    # A procedural neutral particle is mod code, not an extracted game texture.
    from PIL import Image
    white=output/'assets/jakmc/textures/particle/common_white.png'
    white.parent.mkdir(parents=True,exist_ok=True)
    Image.new('RGBA',(4,4),(255,255,255,128)).save(white)
    eye_root = workspace/'eye-layers'; eye_root.mkdir(exist_ok=True)
    for folder,name in [('beach-vis-pris','flut-eye-16x16'),('common','autoeye-pupil'),('beach-vis-pris','flut-eyelid')]:
        shutil.copyfile(export_root(active,'jak1')/f'textures/{folder}/{name}.png', eye_root/(name+'.png'))
    eyes.bake(eye_root,output/'assets/jakmc/flut')
    deck.__file__ = str(workspace/'tools/import_hellcat_deck.py')
    saved = sys.argv[:]
    try:
        sys.argv=['import_hellcat_deck',str(root3)]; deck.main()
        sys.argv=['import_hellcat_deck',str(root2),'--helldog']; deck.main()
        stage('audio')
        notify(words('Convirtiendo efectos de sonido originales…','Converting original sound effects…',lang))
        os.environ['PATH']=str(PAYLOAD/'ffmpeg')+os.pathsep+os.environ.get('PATH','')
        audio.P=workspace
        sys.argv=['import_audio','--opengoal',str(active),'--vgmstream',str(PAYLOAD/'vgmstream/vgmstream-cli.exe')]
        audio.main()
    finally: sys.argv=saved
    for path in REQUIRED_RESOURCES:
        if not (output/path).is_file(): raise ValueError('Missing converted resource: '+path)
    report=json.loads((workspace/'.local/audio-import-receipt.json').read_text())
    if report['unavailable_exports'] or report['clips']<2000: raise ValueError('Incomplete audio conversion')
    return output

def merge_mod(code, resources, dest):
    temp=dest.with_suffix('.tmp.jar'); dest.parent.mkdir(parents=True,exist_ok=True)
    with zipfile.ZipFile(code) as src, zipfile.ZipFile(temp,'w',zipfile.ZIP_DEFLATED,compresslevel=6) as out:
        generated={p.relative_to(resources).as_posix():p for p in resources.rglob('*') if p.is_file()}
        for info in src.infolist():
            if info.filename not in generated: out.writestr(info,src.read(info))
        for name,path in sorted(generated.items()): out.write(path,name)
    os.replace(temp,dest)

class InstallationFiles:
    """Stage the profile's changes together and restore files if committing fails."""
    def __init__(self, staging, backups):
        self.staging = staging
        self.backups = backups
        self.changes = {}
        self.guards = {}
        self.world = None

    def write(self, target, data):
        source = self.staging / uuid.uuid4().hex
        source.write_bytes(data)
        self.changes[target] = source

    def json(self, target, data):
        self.write(target,(json.dumps(data,indent=2,ensure_ascii=False)+'\n').encode('utf-8'))

    def replace(self, target, source):
        self.changes[target] = source

    def archive(self, target):
        self.changes[target] = None

    def add_showcase(self, instance, archive):
        for name in ('Jak Showcase',SHOWCASE):
            if (instance/'saves'/name/'level.dat').is_file():return name
        target=instance/'saves'/SHOWCASE
        if os.path.lexists(target):return SHOWCASE
        staged=self.staging/'showcase'
        with zipfile.ZipFile(archive) as world:
            world.getinfo('level.dat')
            for member in world.infolist():
                parts=member.filename.rstrip('/').split('/')
                if any(part in ('','.','..') or '\\' in part or ':' in part for part in parts):
                    raise ValueError(f'Unexpected showcase entry: {member.filename}')
                world.extract(member,staged)
        self.world=target
        self.changes[target]=staged
        return SHOWCASE

    def commit(self, lang):
        applied = []
        try:
            # Publish the launcher entries after the mods and configuration are ready.
            changes=sorted(self.changes.items(),key=lambda entry:entry[0].name.startswith('launcher_profiles'))
            for target,source in changes:
                if target==self.world:
                    target.parent.mkdir(parents=True,exist_ok=True)
                    try:
                        # On Windows rename fails if the player has already created this save.
                        if os.path.lexists(target):continue
                        source.rename(target)
                    except FileExistsError:
                        continue
                    applied.append((target,None))
                    continue
                if target in self.guards:
                    current = target.read_bytes() if target.exists() else None
                    if current!=self.guards[target]:
                        raise InstallError(words('El launcher cambió sus perfiles. Ciérralo y vuelve a intentarlo.',
                            'The launcher changed its profiles. Close it and try again.',lang))
                backup = None
                if target.exists():
                    if source and target.stat().st_size==source.stat().st_size and digest(target)==digest(source):
                        continue
                    self.backups.mkdir(parents=True,exist_ok=True)
                    backup = self.backups/(target.stem+'-'+uuid.uuid4().hex+target.suffix)
                    if source is not None:
                        shutil.copy2(target,backup)
                if source is None:
                    if backup is None: continue
                    os.replace(target,backup)
                else:
                    target.parent.mkdir(parents=True,exist_ok=True)
                    os.replace(source,target)
                applied.append((target,backup))
        except Exception:
            for target,backup in reversed(applied):
                if target==self.world:
                    target.rename(self.changes[target])
                elif backup is None:
                    target.unlink(missing_ok=True)
                else:
                    os.replace(backup,target)
            raise

def launcher_stores(mc, instance, lang):
    instance=instance.resolve()
    stores=[p for p in [mc/'launcher_profiles.json',mc/'launcher_profiles_microsoft_store.json'] if p.is_file()]
    if not stores: stores=[mc/'launcher_profiles.json']
    snapshots=[]
    for store in stores:
        before=store.read_bytes() if store.is_file() else None
        try:
            data=json.loads(before.decode('utf-8-sig')) if before is not None else {'profiles':{}}
            profiles=data.setdefault('profiles',{})
            current=profiles.get('jakmod',{})
            if current and (not current.get('gameDir') or Path(current['gameDir']).resolve()!=instance):
                raise InstallError(words('El perfil JakMod apunta a otra carpeta de juego. Revisa su directorio en el launcher.',
                    'The JakMod profile points to another game directory. Check its directory in the launcher.',lang))
        except (ValueError, AttributeError, TypeError) as failure:
            raise InstallError(words('El archivo de perfiles del launcher no se puede leer.',
                'The launcher profile file could not be read.',lang),store.name) from failure
        snapshots.append((store,before,data))
    return snapshots

def archive_old_mods(instance, selected, transaction):
    """Identify managed mods by their Fabric ID, even when their files were renamed."""
    targets=set(selected.values())
    for jar in sorted((instance/'mods').glob('*.jar')):
        if jar in targets: continue
        try:
            with zipfile.ZipFile(jar) as archive:
                mod_id=json.loads(archive.read('fabric.mod.json'))['id']
        except (OSError, zipfile.BadZipFile, KeyError, ValueError, TypeError):
            continue
        if isinstance(mod_id,str) and mod_id in selected:
            transaction.archive(jar)

def prepare_launcher(mc, instance, manifest, lang, transaction, snapshots, download=fetch):
    version=manifest['minecraft']; ident=version+'.jakmod'
    cache=instance/'jakmc/setup/downloads'
    base=mc/'versions'/version/(version+'.json')
    if not base.is_file():
        index_path=cache/'version-manifest.json'
        download('https://piston-meta.mojang.com/mc/game/version_manifest_v2.json',index_path)
        index=json.loads(index_path.read_text())
        row=next((v for v in index['versions'] if v['id']==version),None)
        if not row: raise ValueError('Minecraft version not available: '+version)
        staged=transaction.staging/'vanilla.json'
        download(row['url'],staged,row.get('sha1'),'sha1')
        transaction.replace(base,staged)
    profile_path=cache/f'fabric-{version}-{manifest["loader"]}.json'
    profile_url=f'https://meta.fabricmc.net/v2/versions/loader/{version}/{manifest["loader"]}/profile/json'
    if not profile_path.is_file(): download(profile_url,profile_path)
    profile=json.loads(profile_path.read_text())
    if profile.get('inheritsFrom')!=version: raise ValueError('Unexpected Fabric parent version')
    for lib in profile['libraries']:
        group,name,revision=lib['name'].split(':')
        relative=f'{group.replace(".","/")}/{name}/{revision}/{name}-{revision}.jar'
        url=lib.get('url','https://libraries.minecraft.net/').rstrip('/')+'/'+relative
        checksum=cache/'checksums'/f'{group}.{name}-{revision}.sha1'
        if not checksum.is_file(): download(url+'.sha1',checksum)
        expected=checksum.read_text().strip().split()[0]
        if not re.fullmatch('[0-9a-fA-F]{40}',expected): raise ValueError('Invalid Fabric library checksum')
        download(url,mc/'libraries'/relative,expected.lower(),'sha1')
    api=f'fabric-api-{manifest["fabric_api"]}.jar'
    api_url=f'https://maven.fabricmc.net/net/fabricmc/fabric-api/fabric-api/{manifest["fabric_api"]}/{api}'
    dependencies=[('fabric-api',api,api_url,manifest['fabric_api_sha512'])]
    for mod in manifest.get('performance_mods', []):
        mod_id={'ferrite-core':'ferritecore'}.get(mod['project'],mod['project'])
        dependencies.append((mod_id,mod['filename'],mod['url'],mod['sha512']))
    selected={}
    for mod_id,filename,url,expected in dependencies:
        target=instance/'mods'/filename
        selected[mod_id]=target
        if target.is_file() and digest(target,'sha512')==expected: continue
        source=cache/filename
        download(url,source,expected,'sha512')
        staged=transaction.staging/filename
        shutil.copy2(source,staged)
        transaction.replace(target,staged)
    profile['id']=ident
    jvm=profile.setdefault('arguments',{}).setdefault('jvm',[])
    if '--enable-native-access=ALL-UNNAMED' not in jvm: jvm.append('--enable-native-access=ALL-UNNAMED')
    transaction.json(mc/'versions'/ident/(ident+'.json'),profile)
    # Merge into every existing official launcher profile store, preserving all
    # authentication fields and unrelated profiles. Backups stay local.
    for store,before,data in snapshots:
        profiles=data['profiles']
        row=dict(profiles.get('jakmod',{}))
        row.update(lastVersionId=ident,gameDir=str(instance))
        for key,value in dict(name=ident,type='custom',javaArgs='-Xmx4G',icon='Grass').items():
            row.setdefault(key,value)
        row.setdefault('created',datetime.datetime.now(datetime.timezone.utc).isoformat())
        if row!=profiles.get('jakmod'):
            profiles['jakmod']=row
            transaction.guards[store]=before
            transaction.json(store,data)
    options=instance/'options.txt'
    if not options.exists():
        transaction.guards[options]=None
        transaction.write(options,('lang:'+('es_es' if lang=='es' else 'en_us')+'\n').encode())
    return ident,selected

def install_launcher(mc, instance, manifest, lang, download=fetch):
    snapshots=launcher_stores(mc,instance,lang)
    instance.mkdir(parents=True,exist_ok=True)
    with tempfile.TemporaryDirectory(dir=instance) as temporary:
        transaction=InstallationFiles(Path(temporary),instance/'jakmc/backups')
        ident,selected=prepare_launcher(mc,instance,manifest,lang,transaction,snapshots,download)
        archive_old_mods(instance,selected,transaction)
        transaction.commit(lang)
    return ident

def asset_identity(active, vanilla, manifest):
    # Key the conversion to the selected OpenGOAL export and the exact vanilla block templates.
    signature=[]
    for game, names in REQUIRED.items():
        data=active/game/'data'
        for source in [*(export_root(active,game)/name for name in names),data/'iso_data'/game,
                       data/'decompiler/config',export_root(active,game)/'audio/sfx']:
            if source.exists():
                stat=source.stat()
                signature.append((source.relative_to(active).as_posix(),stat.st_size,stat.st_mtime_ns))
    parts=[os.path.normcase(str(active)),digest(vanilla),conversion_stamp(manifest),signature]
    return hashlib.sha256(json.dumps(parts).encode()).hexdigest()

def vanilla_templates(vanilla, host):
    marker=host/'.ready'
    if marker.is_file(): return
    with zipfile.ZipFile(vanilla) as z:
        for info in z.infolist():
            if info.filename.startswith(('assets/minecraft/blockstates/','assets/minecraft/models/')):
                dest=host/info.filename
                if '..' in Path(info.filename).parts or '\\' in info.filename or ':' in info.filename:
                    raise ValueError('Invalid Minecraft archive path')
                if not info.is_dir(): atomic(dest,z.read(info))
    atomic(marker,b'1')

def cached_assets(generation, identity):
    marker=generation/'complete.json'
    if not marker.is_file(): return None
    owned=generation/'owned/active'
    resources=generation/'src/generated/resources'
    try:
        files=json.loads(marker.read_text(encoding='utf-8'))
        if files['identity']!=identity: return None
        if set(files['required'])!=set(REQUIRED_RESOURCES): return None
        if any(not (export_root(owned,g)/REQUIRED[g][0]).is_file() for g in REQUIRED): return None
        if any((resources/name).stat().st_size!=size for name,size in files['required'].items()): return None
        if sum(p.is_file() for p in resources.rglob('*'))!=files['count']: return None
    except (OSError, ValueError, KeyError, TypeError):
        return None
    return owned,resources,files['count']

def prepare_assets(active, vanilla, workspace, manifest, notify, lang, stage):
    wanted=asset_identity(active,vanilla,manifest)
    assets=workspace/'assets'
    prefix=wanted[:16]
    for generation in sorted(assets.glob(prefix+'*'),reverse=True):
        if generation.is_junction() or generation.is_symlink(): continue
        cached=cached_assets(generation,wanted)
        if cached:
            notify(words('Reutilizando los recursos de Jak ya preparados…','Reusing the prepared Jak resources…',lang))
            return cached
    generation=assets/prefix
    if (generation/'complete.json').exists() or generation.is_junction() or generation.is_symlink():
        generation=assets/(prefix+'-'+uuid.uuid4().hex[:8])
    generation.mkdir(parents=True,exist_ok=True)
    notify(words('Preparando los recursos de OpenGOAL…','Preparing OpenGOAL resources…',lang))
    owned=prepare_owned(active,generation,notify,lang)
    vanilla_templates(vanilla,generation/f'resourcepacks/local/minecraft-{manifest["minecraft"]}')
    resources=import_resources(owned,generation,notify,lang,stage)
    count=sum(p.is_file() for p in resources.rglob('*'))
    write_json(generation/'complete.json',{'identity':wanted,'count':count,
        'required':{name:(resources/name).stat().st_size for name in REQUIRED_RESOURCES}})
    for game in owned_games(owned):
        root=export_root(owned,game.name)
        slim_models(root,game.name);slim_converted(root)
    converted=generation/'.local/content'
    if converted.is_dir() and converted.resolve().is_relative_to(generation.resolve()):
        shutil.rmtree(converted)
    return owned,resources,count

def asset_config(config, owned):
    previous=config.read_bytes().decode('latin-1').splitlines() if config.exists() else []
    path=str(export_root(owned,'jak3')).replace('\\','/')
    # java.util.Properties uses Latin-1 and \u escapes, including Unicode paths.
    units=path.encode('utf-16-be')
    escaped=''.join(chr(n) if n<128 else '\\u%04x'%n for (n,) in struct.iter_unpack('>H',units))
    previous=[line for line in previous if not re.match(r'\s*assets(?:\s*[=:]|\s+|$)',line)]
    return ('\n'.join(previous+['assets='+escaped])+'\n').encode('latin-1')

def _install(opengoal, minecraft, lang='es', notify=print, assets_only=False, stage=lambda _:None):
    stage('preflight')
    if sys.platform!='win32': raise InstallError('Windows x86-64 is required.')
    active=normalize_opengoal(opengoal)
    if active is None: raise InstallError(words('Selecciona la carpeta de OpenGOAL que contiene Jak 1, Jak II y Jak 3.',
        'Select the OpenGOAL folder containing Jak 1, Jak II and Jak 3.',lang))
    mc=Path(minecraft).resolve();instance=mc/'instances/jakmod'
    workspace=instance/'jakmc/setup'
    try:
        manifest=json.loads((PAYLOAD/'manifest.json').read_text(encoding='utf-8'))
        for name,expected in manifest['files'].items():
            if digest(PAYLOAD/name)!=expected: raise ValueError(name)
    except (OSError, ValueError, KeyError) as failure:
        raise InstallError(words('Extrae todo el archivo del instalador y vuelve a abrir el EXE.',
            'Extract the entire installer archive and reopen the EXE.',lang),str(failure)) from failure
    vanilla=mc/'versions'/manifest['minecraft']/(manifest['minecraft']+'.jar')
    if not vanilla.is_file():
        raise InstallError(words(f'Abre Minecraft Java {manifest["minecraft"]} una vez en el launcher y ciérralo antes de instalar.',
            f'Launch Minecraft Java {manifest["minecraft"]} once in the launcher, then close it before installing.',lang))
    snapshots=[] if assets_only else launcher_stores(mc,instance,lang)
    with tempfile.TemporaryDirectory(dir=workspace,prefix='install-') as temporary:
        transaction=InstallationFiles(Path(temporary),instance/'jakmc/backups')
        ident=None
        selected={}
        if not assets_only:
            stage('dependencies')
            notify(words('Comprobando Fabric y los mods de rendimiento…','Checking Fabric and performance mods…',lang))
            ident,selected=prepare_launcher(mc,instance,manifest,lang,transaction,snapshots)
        stage('assets')
        owned,resources,count=prepare_assets(active,vanilla,workspace,manifest,notify,lang,stage)
        mod=instance/'mods'/f'jakmc-{manifest["mod_version"]}.jar'
        staged=transaction.staging/mod.name
        merge_mod(PAYLOAD/'jakmc-code.jar',resources,staged)
        transaction.replace(mod,staged)
        selected['jakmc']=mod
        archive_old_mods(instance,selected,transaction)
        config=instance/'config/jakmc.properties'
        transaction.write(config,asset_config(config,owned))
        receipt=dict(version=manifest['mod_version'],minecraft=manifest['minecraft'],profile=ident,
            resources=count,mod_sha256=digest(staged),completed=datetime.datetime.now(datetime.timezone.utc).isoformat())
        if not assets_only:
            stage('showcase')
            notify(words('Preparando el mapa de pruebas…','Preparing the test world…',lang))
            receipt['showcase']=transaction.add_showcase(instance,PAYLOAD/SHOWCASE_ARCHIVE)
        stage('commit')
        notify(words('Aplicando los cambios al perfil JakMod…','Applying changes to the JakMod profile…',lang))
        transaction.json(workspace/'installation-receipt.json',receipt)
        transaction.commit(lang)
    notify(words('Instalación completada.','Installation complete.',lang))
    return receipt

def install(opengoal, minecraft, lang='es', notify=print, assets_only=False, stage=lambda _:None):
    import msvcrt
    lock=Path(minecraft).resolve()/'instances/jakmod/jakmc/setup/install.lock'
    lock.parent.mkdir(parents=True,exist_ok=True)
    with lock.open('a+b') as file:
        if file.tell()==0:file.write(b'0');file.flush()
        file.seek(0)
        try:msvcrt.locking(file.fileno(),msvcrt.LK_NBLCK,1)
        except OSError:raise InstallError(words('Ya hay otra instalación de JakMod en curso.',
            'Another JakMod installation is already running.',lang))
        try:return _install(opengoal,minecraft,lang,notify,assets_only,stage)
        finally:file.seek(0);msvcrt.locking(file.fileno(),msvcrt.LK_UNLCK,1)

TEXT = {
 'es':dict(title='Instalador de JakMod',open='1. Juegos de OpenGOAL',mc='2. Minecraft Java',
    open_hint='Necesitas Jak 1, Jak II y Jak 3 instalados en OpenGOAL.\n'
              'Usa tus ISOs originales para instalar los juegos en OpenGOAL.\n'
              'Para JakMod, selecciona la instalación o su carpeta active. Conserva los datos extraídos y guarda las ISOs donde quieras.',
    mc_hint='Usa la carpeta .minecraft del launcher oficial.',
    browse='Examinar…',install='3. Instalar o actualizar JakMod',
    intro='Abre Minecraft Java {version} una vez y después cierra el juego y el launcher.',
    ready='La primera instalación puede tardar varios minutos.',
    error='La instalación se detuvo',done='Instalación completada',busy='Comprobando las carpetas…',
    finished='Abre Minecraft Launcher y elige {version}. Si falta, activa «Modificadas» en Instalaciones.\n'
             'El mapa de pruebas {showcase} está en Un jugador.',
    channel='Canal de AdriiSuizo',video='Vídeo de instalación',opengoal='Tutorial de OpenGOAL',copy='Copiar para comentar',copied='Texto copiado.',
    close='Cerrar',log='Abrir registro local',share='Pega este texto en los comentarios del vídeo de instalación.',
    report='Problema al instalar JakMod',step='Paso',problem='Error',game='Minecraft Java',build='Versión de JakMod',
    add_context='Qué hice antes del error',network='Comprueba la conexión e inténtalo de nuevo.',
    files='Comprueba las carpetas y el espacio disponible. Si se repite, copia el informe.',
    conflict='Cierra Minecraft y el launcher y vuelve a intentarlo.',
    manual='Abre el registro local para ver el detalle técnico. El texto de abajo está listo para compartir.',
    cannot_open='No se pudo abrir el enlace o archivo.'),
 'en':dict(title='JakMod Installer',open='1. OpenGOAL games',mc='2. Minecraft Java',
    open_hint='You need Jak 1, Jak II and Jak 3 installed in OpenGOAL.\n'
              'Use your original ISOs to install the games in OpenGOAL.\n'
              'For JakMod, select the installation or its active folder. Keep the extracted data and store the ISOs wherever you like.',
    mc_hint='Use the official launcher’s .minecraft folder.',
    browse='Browse…',install='3. Install or update JakMod',
    intro='Launch Minecraft Java {version} once, then close the game and its launcher.',
    ready='The first installation can take several minutes.',
    error='Installation stopped',done='Installation complete',busy='Checking your folders…',
    finished='Open Minecraft Launcher and choose {version}. If it is missing, enable “Modded” under Installations.\n'
             'The {showcase} test world is under Singleplayer.',
    channel='AdriiSuizo’s channel',video='Installation video',opengoal='OpenGOAL tutorial',copy='Copy for a comment',copied='Text copied.',
    close='Close',log='Open local log',share='Paste this text into the comments on the installation video.',
    report='Problem installing JakMod',step='Step',problem='Error',game='Minecraft Java',build='JakMod version',
    add_context='What I did before the error',network='Check your internet connection and try again.',
    files='Check the selected folders and free disk space. If it happens again, copy the report.',
    conflict='Close Minecraft and its launcher, then try again.',
    manual='Open the local log for technical details. The text below is ready to share.',
    cannot_open='The link or file could not be opened.')}

STAGES = {
    'preflight':('Comprobar carpetas y versión','Check folders and version'),
    'dependencies':('Descargar Fabric y mods de rendimiento','Download Fabric and performance mods'),
    'assets':('Preparar los recursos de OpenGOAL','Prepare OpenGOAL resources'),
    'audio':('Convertir los sonidos','Convert sounds'),
    'showcase':('Copiar el mapa de pruebas','Copy the test world'),
    'commit':('Actualizar el perfil del launcher','Update the launcher profile'),
}

def display_manifest():
    try:
        return json.loads((PAYLOAD/'manifest.json').read_text(encoding='utf-8'))
    except (OSError, ValueError):
        return {}

def youtube_link(value):
    if not isinstance(value,str) or not value or any(char.isspace() for char in value): return None
    try:
        url=urlsplit(value)
    except ValueError:
        return None
    if url.scheme=='https' and url.netloc.lower() in {'youtube.com','www.youtube.com','youtu.be'} and url.path.strip('/'):
        return value
    return None

def support_report(failure, phase, manifest, lang):
    t=TEXT[lang]
    if isinstance(failure,InstallError):
        summary=str(failure)
    elif isinstance(failure,urllib.error.HTTPError):
        summary=f'HTTP {failure.code}. '+t['network']
    elif isinstance(failure,(urllib.error.URLError,TimeoutError,ConnectionError)):
        summary=t['network']
    elif isinstance(failure,PermissionError):
        summary=t['conflict']
    elif isinstance(failure,OSError):
        summary=t['files']
    else:
        summary=t['manual']
    stage_name=words(*STAGES.get(phase,STAGES['preflight']),lang)
    lines=[t['report']]
    for field,key in [('minecraft','game'),('mod_version','build')]:
        value=manifest.get(field)
        if isinstance(value,str) and re.fullmatch(r'[0-9A-Za-z.+_-]{1,40}',value):
            lines.append(f'{t[key]} · {value}')
    lines += [f'{t["step"]} · {stage_name} [{phase}]',
              f'{t["problem"]} · {summary} ({type(failure).__name__})','',t['add_context']+' · ']
    return summary,'\n'.join(lines)

def open_resource(resource, parent, lang):
    from tkinter import messagebox
    try:
        if isinstance(resource,Path):
            os.startfile(resource)
        elif not webbrowser.open(resource):
            raise OSError()
    except OSError:
        messagebox.showwarning(TEXT[lang]['title'],TEXT[lang]['cannot_open'],parent=parent)

def error_window(parent, summary, report, log, support, lang):
    import tkinter as tk
    from tkinter import ttk
    from tkinter.scrolledtext import ScrolledText
    t=TEXT[lang]
    window=tk.Toplevel(parent)
    window.title(t['error'])
    window.minsize(580,330)
    frame=ttk.Frame(window,padding=20)
    frame.pack(fill='both',expand=True)
    ttk.Label(frame,text=summary,font=('Segoe UI',12,'bold'),wraplength=600).pack(anchor='w')
    ttk.Label(frame,text=t['share'],wraplength=600).pack(anchor='w',pady=(8,10))
    body=ScrolledText(frame,wrap='word',width=72,height=10,font=('Segoe UI',10),padx=10,pady=8)
    body.pack(fill='both',expand=True)
    body.insert('1.0',report)
    body.configure(state='disabled')
    actions=ttk.Frame(frame)
    actions.pack(fill='x',pady=(12,0))
    def copy():
        window.clipboard_clear()
        window.clipboard_append(report)
        copy_button.configure(text=t['copied'])
    copy_button=ttk.Button(actions,text=t['copy'],command=copy)
    copy_button.pack(side='left',padx=(0,6))
    video=youtube_link(support.get('video'))
    channel=youtube_link(support.get('channel'))
    if video or channel:
        ttk.Button(actions,text=t['video' if video else 'channel'],
            command=lambda:open_resource(video or channel,window,lang)).pack(side='left',padx=(0,6))
    if log.is_file():
        ttk.Button(actions,text=t['log'],command=lambda:open_resource(log,window,lang)).pack(side='left')
    ttk.Button(actions,text=t['close'],command=window.destroy).pack(side='right')
    window.bind('<Escape>',lambda _:window.destroy())
    copy_button.bind('<Return>',lambda _:copy())
    copy_button.focus_set()
    if parent.winfo_viewable():
        window.transient(parent)
        window.grab_set()
    return window

def gui(opengoal=None,minecraft=None,language=None):
    import tkinter as tk
    from tkinter import ttk,filedialog,messagebox
    root=tk.Tk();root.geometry('760x520');root.minsize(700,510)
    style=ttk.Style(root)
    style.theme_use('clam')
    paper,ink,muted,accent='#f7f7fb','#202036','#505369','#4b37a1'
    root.configure(background=paper)
    style.configure('TFrame',background=paper)
    style.configure('TLabel',background=paper,foreground=ink,font=('Segoe UI',10))
    style.configure('Hint.TLabel',foreground=muted)
    style.configure('TEntry',padding=6,fieldbackground='#ffffff',foreground=ink)
    style.configure('TButton',padding=(12,7),font=('Segoe UI',10),foreground=ink,background='#eeeef7')
    style.configure('Primary.TButton',foreground='#ffffff',background=accent,font=('Segoe UI',10,'bold'))
    style.map('Primary.TButton',background=[('disabled','#dddbea'),('active','#392780')],
        foreground=[('disabled',muted)])
    style.configure('Horizontal.TProgressbar',background=accent,troughcolor='#e8e6f2',borderwidth=0)
    manifest=display_manifest()
    support=manifest.get('support',{})
    lang=tk.StringVar(value=language or 'es')
    choice=tk.StringVar(value='Español' if lang.get()=='es' else 'English')
    og=tk.StringVar(value=str(opengoal or opengoal_default()));mc=tk.StringVar(value=str(minecraft or minecraft_default()))
    frame=ttk.Frame(root,padding=24);frame.pack(fill='both',expand=True)
    language_row=ttk.Frame(frame);language_row.pack(fill='x')
    ttk.Label(language_row,text='JakMod',font=('Segoe UI',24,'bold')).pack(side='left')
    menu=ttk.Combobox(language_row,textvariable=choice,values=['Español','English'],state='readonly',width=12);menu.pack(side='right')
    intro=ttk.Label(frame,wraplength=620);intro.pack(anchor='w',pady=(8,20))
    labels=[];buttons=[];entries=[]
    for key,var in [('open',og),('mc',mc)]:
        label=ttk.Label(frame,font=('Segoe UI',11,'bold'));label.pack(anchor='w');labels.append((label,key))
        row=ttk.Frame(frame);row.pack(fill='x',pady=(4,4))
        entry=ttk.Entry(row,textvariable=var);entry.pack(side='left',fill='x',expand=True)
        entries.append(entry)
        def choose(v=var):
            selected=filedialog.askdirectory(parent=root,initialdir=v.get())
            if selected:v.set(selected)
        button=ttk.Button(row,command=choose);button.pack(side='right',padx=(8,0));buttons.append(button)
        hint=ttk.Label(frame,style='Hint.TLabel',wraplength=620);hint.pack(anchor='w',pady=(0,12));labels.append((hint,key+'_hint'))
    activity=ttk.Frame(frame,height=52)
    activity.pack(fill='x')
    activity.pack_propagate(False)
    status=ttk.Label(activity,wraplength=620);status.pack(anchor='w',pady=(3,5))
    progress=ttk.Progressbar(activity,mode='indeterminate')
    messages=queue.Queue();running=False
    actions=ttk.Frame(frame);actions.pack(fill='x',pady=(15,0))
    links=[]
    for key in ('opengoal','channel','video'):
        url=youtube_link(support.get(key))
        if url:
            link=ttk.Button(actions,command=lambda dest=url:open_resource(dest,root,lang.get()))
            link.pack(side='left',padx=(0,8));links.append((link,key))
    def set_busy(value):
        install_button.configure(state='disabled' if value else 'normal')
        menu.configure(state='disabled' if value else 'readonly')
        for widget in (*entries,*buttons):
            widget.configure(state='disabled' if value else 'normal')
        if value:
            progress.pack(fill='x')
            progress.start()
        else:
            progress.stop()
            progress.pack_forget()
    def start():
        nonlocal running
        if running:return
        running=True;set_busy(True)
        status.configure(text=TEXT[lang.get()]['busy'])
        chosen=lang.get();op=og.get().strip();mp=mc.get().strip()
        log=Path(mp)/'instances/jakmod/jakmc/setup/installer.log'
        def work():
            phase='preflight'
            def set_stage(value):
                nonlocal phase
                phase=value
            try:
                if not op or not mp:
                    raise InstallError(words('Selecciona las dos carpetas antes de continuar.',
                        'Select both folders before continuing.',chosen))
                log.parent.mkdir(parents=True,exist_ok=True)
                with log.open('a',encoding='utf-8') as output, contextlib.redirect_stdout(output), contextlib.redirect_stderr(output):
                    try:
                        result=install(op,mp,chosen,lambda s:messages.put(('status',s)),stage=set_stage)
                    except (Exception,SystemExit) as failure:
                        traceback.print_exc()
                        if isinstance(failure,InstallError) and failure.detail: print(failure.detail)
                        raise
                messages.put(('done',result))
            except (Exception,SystemExit) as failure:
                summary,report=support_report(failure,phase,manifest,chosen)
                messages.put(('error',(summary,report,log)))
        threading.Thread(target=work,daemon=False).start()
    install_button=ttk.Button(actions,style='Primary.TButton',command=start);install_button.pack(side='right')
    install_button.bind('<Return>',lambda _:start())
    root.after_idle(install_button.focus_set)
    def translate(*_):
        lang.set('es' if choice.get()=='Español' else 'en')
        t=TEXT[lang.get()];root.title(t['title'])
        intro.configure(text=t['intro'].format(version=manifest.get('minecraft','26.3')))
        status.configure(text=t['ready'])
        for label,key in labels:label.configure(text=t[key])
        for button in buttons:button.configure(text=t['browse'])
        for link,key in links:link.configure(text=t[key])
        install_button.configure(text=t['install'])
    menu.bind('<<ComboboxSelected>>',translate);translate()
    def poll():
        nonlocal running
        try:
            while True:
                kind,value=messages.get_nowait();t=TEXT[lang.get()]
                if kind=='status':status.configure(text=value)
                else:
                    running=False;set_busy(False)
                    if kind=='error':
                        status.configure(text=t['error'])
                        error_window(root,*value,support,lang.get())
                    else:
                        status.configure(text=t['finished'].format(version=value['profile'],showcase=value['showcase']))
                        messagebox.showinfo(t['done'],status.cget('text'),parent=root)
        except queue.Empty:pass
        root.after(150,poll)
    root.protocol('WM_DELETE_WINDOW',lambda:None if running else root.destroy())
    poll();root.mainloop()

if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--opengoal',type=Path);parser.add_argument('--minecraft',type=Path)
    parser.add_argument('--language',choices=['es','en']);parser.add_argument('--assets-only',action='store_true')
    parser.add_argument('--gui',action='store_true',help='Open the installer window with the supplied folders')
    args=parser.parse_args()
    if args.gui or not args.opengoal:
        gui(args.opengoal,args.minecraft,args.language)
    else:
        args.language=args.language or 'es'
        if getattr(sys,'frozen',False):
            log=(args.minecraft or minecraft_default())/'instances/jakmod/jakmc/setup/installer-cli.log'
            phase='preflight'
            def set_phase(value):
                global phase
                phase=value
            try:
                log.parent.mkdir(parents=True,exist_ok=True)
                with log.open('a',encoding='utf-8') as output, contextlib.redirect_stdout(output),contextlib.redirect_stderr(output):
                    try:
                        result=install(args.opengoal,args.minecraft or minecraft_default(),args.language,assets_only=args.assets_only,stage=set_phase)
                        print(json.dumps(result))
                    except (Exception,SystemExit) as failure:
                        traceback.print_exc()
                        if isinstance(failure,InstallError) and failure.detail: print(failure.detail)
                        raise
            except (Exception,SystemExit) as failure:
                import tkinter as tk
                root=tk.Tk();root.withdraw()
                manifest=display_manifest()
                summary,report=support_report(failure,phase,manifest,args.language)
                window=error_window(root,summary,report,log,manifest.get('support',{}),args.language)
                root.wait_window(window);root.destroy()
                sys.exit(1)
        else:
            result=install(args.opengoal,args.minecraft or minecraft_default(),args.language,assets_only=args.assets_only)
            print(json.dumps(result))
