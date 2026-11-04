"""Windows installer. All game content is read/converted on the owner's PC.

The same engine serves the bilingual GUI and unattended, isolated verification.
No Minecraft credentials are read, transmitted or included in a receipt.
"""
from pathlib import Path
import argparse, contextlib, datetime, hashlib, json, locale, os, queue, shutil, struct
import subprocess, sys, threading, urllib.request, uuid, zipfile

BASE = Path(os.environ.get('JAKMC_INSTALLER_BUNDLE', getattr(sys, '_MEIPASS', Path(__file__).resolve().parent)))
PAYLOAD = BASE / 'payload'
CREATE_NO_WINDOW = getattr(subprocess, 'CREATE_NO_WINDOW', 0)

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

def fetch(url, dest, expected=None, kind='sha256'):
    if dest.is_file() and expected and digest(dest, kind) == expected: return
    req = urllib.request.Request(url, headers={'User-Agent':'JakMod-Installer/0.1'})
    with urllib.request.urlopen(req, timeout=90) as r: data = r.read()
    if expected and hashlib.new(kind, data).hexdigest() != expected:
        raise ValueError('Download checksum mismatch: ' + url)
    atomic(dest, data)

def run(args, log, env=None):
    with log.open('ab') as output:
        result = subprocess.run([str(a) for a in args], stdout=output, stderr=subprocess.STDOUT,
                                env=env, creationflags=CREATE_NO_WINDOW)
    if result.returncode: raise RuntimeError(f'{Path(args[0]).name}: {result.returncode}. Log: {log}')

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

REQUIRED = {
 'jak1':['levels/beach/flutflut-lod0.glb', 'textures/beach-vis-pris/flut-eye-16x16.png',
         'textures/common/autoeye-pupil.png','textures/beach-vis-pris/flut-eyelid.png'],
 'jak2':['levels/ctywide/ctywide-background.glb','levels/lwidea/citizen-norm-lod0.glb'],
 'jak3':['levels/factorya/hellcat-lod0.glb','raw_obj/gun-ag.go',
         'textures/progress-minimap/hud-skullgem.png']}

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
    for game in ('jak1','jak2','jak3'):
        source = active/game
        root = export_root(active, game)
        complete = all((root/name).is_file() for name in REQUIRED[game])
        complete = complete and (root/'audio/sfx').is_dir() and (source/'data/goal_src'/game).is_dir()
        target = canonical/game
        if complete:
            junction(target, source)
            continue
        cached=export_root(canonical,game)
        if target.exists() and not target.is_symlink() and all((cached/name).is_file() for name in REQUIRED[game]) and (cached/'audio/sfx').is_dir():
            # Resume an interrupted installation after successful extraction.
            # The canonical directory belongs exclusively to this instance.
            continue
        extractor = next((p for p in (active/'bin/extractor.exe', source/'extractor.exe',
                          source/'bin/extractor.exe') if p.is_file()), None)
        iso = source/'data/iso_data'/game
        if not extractor or not iso.is_dir() or not (source/'data/decompiler/config').is_dir():
            raise ValueError(words(
                f'Faltan exportaciones de {game}. Instala los tres juegos en OpenGOAL y selecciona su carpeta (debe contener extractor.exe, data/iso_data y data/decompiler/config).',
                f'{game} exports are missing. Install all three games in OpenGOAL and select their installation folder (extractor.exe, data/iso_data and data/decompiler/config are required).', lang))
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
        run([extractor, iso, '--folder','--decompile','--game',game,'--proj-path',data,
             '--decomp-config-override',json.dumps(override),'--disable-ansi'], workspace/f'extract-{game}.log')
        missing = [name for name in REQUIRED[game] if not (export_root(canonical, game)/name).is_file()]
        if missing: raise ValueError(f'{game}: incomplete OpenGOAL extraction: '+', '.join(missing))
    return canonical

def import_resources(active, workspace, notify, lang):
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
        notify(words('Convirtiendo efectos de sonido originales…','Converting original sound effects…',lang))
        os.environ['PATH']=str(PAYLOAD/'ffmpeg')+os.pathsep+os.environ.get('PATH','')
        audio.P=workspace
        sys.argv=['import_audio','--opengoal',str(active),'--vgmstream',str(PAYLOAD/'vgmstream/vgmstream-cli.exe')]
        audio.main()
    finally: sys.argv=saved
    must=['assets/jakmc/sounds.json','assets/jakmc/hellcat/deck.json','assets/jakmc/helldog/deck.json',
          'assets/jakmc/flut/eye_open.png','data/jakmc/catalog/haven.jbp',
          'data/jakmc/catalog/spargus.jbp','data/jakmc/catalog/palace.jbp','data/jakmc/catalog/objects/104.json']
    for path in must:
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

def install_launcher(mc, instance, manifest, lang, download=fetch):
    version=manifest['minecraft']; ident=version+'.jakmod'
    base=mc/'versions'/version/(version+'.json')
    if not base.is_file():
        # The official launcher downloads the vanilla game on first launch.
        index_path=instance/'jakmc/setup/version-manifest.json'
        download('https://piston-meta.mojang.com/mc/game/version_manifest_v2.json',index_path)
        index=json.loads(index_path.read_text())
        row=next((v for v in index['versions'] if v['id']==version),None)
        if not row: raise ValueError('Minecraft version not available: '+version)
        download(row['url'],base,row.get('sha1'),'sha1')
    profile_path=instance/'jakmc/setup/fabric-profile.json'
    download(f'https://meta.fabricmc.net/v2/versions/loader/{version}/{manifest["loader"]}/profile/json',profile_path)
    profile=json.loads(profile_path.read_text())
    if profile.get('inheritsFrom')!=version: raise ValueError('Unexpected Fabric parent version')
    for lib in profile['libraries']:
        group,name,revision=lib['name'].split(':')
        relative=f'{group.replace(".","/")}/{name}/{revision}/{name}-{revision}.jar'
        url=lib.get('url','https://libraries.minecraft.net/').rstrip('/')+'/'+relative
        checksum=instance/'jakmc/setup/checksums'/f'{name}-{revision}.sha1'
        download(url+'.sha1',checksum)
        download(url,mc/'libraries'/relative,checksum.read_text().strip().split()[0],'sha1')
    api=f'fabric-api-{manifest["fabric_api"]}.jar'
    api_url=f'https://maven.fabricmc.net/net/fabricmc/fabric-api/fabric-api/{manifest["fabric_api"]}/{api}'
    download(api_url,instance/'mods'/api,manifest['fabric_api_sha512'],'sha512')
    profile['id']=ident
    profile.setdefault('arguments',{}).setdefault('jvm',[]).append('--enable-native-access=ALL-UNNAMED')
    write_json(mc/'versions'/ident/(ident+'.json'),profile)
    # Merge into every existing official launcher profile store, preserving all
    # authentication fields and unrelated profiles. Backups stay local.
    stores=[p for p in [mc/'launcher_profiles.json',mc/'launcher_profiles_microsoft_store.json'] if p.is_file()]
    if not stores: stores=[mc/'launcher_profiles.json']
    for store in stores:
        before=store.read_bytes() if store.exists() else None
        data=json.loads(before.decode('utf-8-sig')) if before else {'profiles':{}}
        profiles=data.setdefault('profiles',{})
        current=profiles.get('jakmod',{})
        if current and current.get('gameDir')!=str(instance): raise ValueError('JakMod profile already points to a different instance')
        row=dict(current);row.update(name=ident,type='custom',lastVersionId=ident,gameDir=str(instance),
            javaArgs='-Xmx4G --enable-native-access=ALL-UNNAMED',icon='Grass')
        row.setdefault('created',datetime.datetime.now(datetime.timezone.utc).isoformat())
        profiles['jakmod']=row
        if before:
            backup=instance/'jakmc/backups'/(store.stem+'-'+datetime.datetime.now().strftime('%Y%m%d-%H%M%S-%f')+'.json')
            atomic(backup,before)
        # Detect a launcher saving its profiles while the installer was working.
        if before is not None and store.read_bytes()!=before: raise ValueError('Launcher profiles changed. Close the launcher and retry.')
        write_json(store,data)
    options=instance/'options.txt'
    if not options.exists(): atomic(options,('lang:'+('es_es' if lang=='es' else 'en_us')+'\n').encode())
    return ident

def _install(opengoal, minecraft, lang='es', notify=print, assets_only=False):
    if sys.platform!='win32': raise ValueError('Windows x86-64 is required')
    active=normalize_opengoal(opengoal)
    if active is None: raise ValueError(words('No encuentro Jak 1, Jak II y Jak 3 instalados en esa carpeta de OpenGOAL.',
        'Jak 1, Jak II and Jak 3 installations were not found in that OpenGOAL folder.',lang))
    mc=Path(minecraft).resolve();instance=mc/'instances/jakmod'
    workspace=instance/'jakmc/setup';workspace.mkdir(parents=True,exist_ok=True)
    manifest=json.loads((PAYLOAD/'manifest.json').read_text(encoding='utf-8'))
    for name,expected in manifest['files'].items():
        if digest(PAYLOAD/name)!=expected: raise ValueError('Installer payload checksum mismatch: '+name)
    # Native block templates are derived locally from the installed vanilla jar.
    vanilla=mc/'versions'/manifest['minecraft']/(manifest['minecraft']+'.jar')
    if not vanilla.is_file():
        raise ValueError(words(f'Abre Minecraft Java {manifest["minecraft"]} una vez en el launcher y ciérralo antes de instalar.',
            f'Launch Minecraft Java {manifest["minecraft"]} once in the launcher, then close it before installing.',lang))
    owned=prepare_owned(active,workspace,notify,lang)
    host=workspace/f'resourcepacks/local/minecraft-{manifest["minecraft"]}'
    with zipfile.ZipFile(vanilla) as z:
        for info in z.infolist():
            if info.filename.startswith(('assets/minecraft/blockstates/','assets/minecraft/models/')):
                dest=host/info.filename
                if '..' in Path(info.filename).parts: raise ValueError('Invalid Minecraft archive path')
                if info.is_dir():
                    if dest.is_file() and dest.stat().st_size==0: dest.unlink()
                    dest.mkdir(parents=True,exist_ok=True)
                    continue
                atomic(dest,z.read(info))
    resources=import_resources(owned,workspace,notify,lang)
    mod=instance/'mods'/f'jakmc-{manifest["mod_version"]}.jar'
    if mod.exists():
        backup=instance/'jakmc/backups'/(mod.stem+'-'+uuid.uuid4().hex+'.jar')
        backup.parent.mkdir(parents=True,exist_ok=True);shutil.copyfile(mod,backup)
    merge_mod(PAYLOAD/'jakmc-code.jar',resources,mod)
    for old in (instance/'mods').glob('jakmc-*.jar'):
        if old==mod:continue
        with zipfile.ZipFile(old) as z:
            if json.loads(z.read('fabric.mod.json'))['id']!='jakmc':continue
        backup=instance/'jakmc/backups'/(old.stem+'-'+uuid.uuid4().hex+'.jar')
        backup.parent.mkdir(parents=True,exist_ok=True);old.rename(backup)
    config=instance/'config/jakmc.properties'
    previous=config.read_bytes().decode('latin-1').splitlines() if config.exists() else []
    if config.exists(): shutil.copyfile(config,workspace/('config-'+uuid.uuid4().hex+'.bak'))
    path=str(export_root(owned,'jak3')).replace('\\','/')
    # java.util.Properties uses Latin-1 and \u escapes, including Unicode paths.
    units=path.encode('utf-16-be')
    escaped=''.join(chr(n) if n<128 else '\\u%04x'%n for (n,) in struct.iter_unpack('>H',units))
    import re
    previous=[line for line in previous if not re.match(r'\s*assets\s*[=:]',line)]
    atomic(config,('\n'.join(previous+['assets='+escaped])+'\n').encode('latin-1'))
    ident=None if assets_only else install_launcher(mc,instance,manifest,lang)
    receipt=dict(version=manifest['mod_version'],minecraft=manifest['minecraft'],profile=ident,
        resources=sum(1 for p in resources.rglob('*') if p.is_file()),mod_sha256=digest(mod),
        completed=datetime.datetime.now(datetime.timezone.utc).isoformat())
    write_json(workspace/'installation-receipt.json',receipt)
    notify(words(f'Instalación completada. Abre el launcher y selecciona {ident}.',
                 f'Installation complete. Open the launcher and select {ident}.',lang))
    return receipt

def install(opengoal, minecraft, lang='es', notify=print, assets_only=False):
    import msvcrt
    lock=Path(minecraft).resolve()/'instances/jakmod/jakmc/setup/install.lock'
    lock.parent.mkdir(parents=True,exist_ok=True)
    with lock.open('a+b') as file:
        if file.tell()==0:file.write(b'0');file.flush()
        file.seek(0)
        try:msvcrt.locking(file.fileno(),msvcrt.LK_NBLCK,1)
        except OSError:raise RuntimeError(words('Ya hay otra instalación de JakMod en curso.',
            'Another JakMod installation is already running.',lang))
        try:return _install(opengoal,minecraft,lang,notify,assets_only)
        finally:file.seek(0);msvcrt.locking(file.fileno(),msvcrt.LK_UNLCK,1)

TEXT = {
 'es':dict(title='JakMod — Instalador',language='Idioma / Language',open='Instalación de OpenGOAL (Jak 1, II y 3)',
    mc='Carpeta de Minecraft Java',browse='Examinar…',install='Instalar JakMod',
    intro='Usa tus juegos instalados en OpenGOAL para generar los recursos localmente.\nCierra Minecraft y su launcher antes de instalar. Se necesita Internet para Fabric.\nTus mundos normales se conservan; JakMod tendrá su propia carpeta de partidas.',
    error='No se ha completado la instalación',done='Instalación completada',busy='Instalando…',
    finished='Reabre Minecraft Launcher y selecciona {version}.\nSi no aparece, activa «Modificadas» en Instalaciones.\nLee README-ES.md para los controles y los requisitos.'),
 'en':dict(title='JakMod — Installer',language='Language / Idioma',open='OpenGOAL installation (Jak 1, II and 3)',
    mc='Minecraft Java folder',browse='Browse…',install='Install JakMod',
    intro='Uses your OpenGOAL games to create the resources locally.\nClose Minecraft and its launcher before installing. Internet is required for Fabric.\nYour normal worlds are kept; JakMod uses a separate save folder.',
    error='Installation did not complete',done='Installation complete',busy='Installing…',
    finished='Reopen Minecraft Launcher and select {version}.\nIf missing, enable “Modded” in Installations.\nRead README-EN.md for controls and requirements.')}

def gui():
    import tkinter as tk
    from tkinter import ttk,filedialog,messagebox
    root=tk.Tk();root.geometry('760x440');root.resizable(True,False)
    lang=tk.StringVar(value='es' if (locale.getlocale()[0] or '').startswith('es') else 'en')
    choice=tk.StringVar(value='Español' if lang.get()=='es' else 'English')
    og=tk.StringVar(value=str(opengoal_default()));mc=tk.StringVar(value=str(minecraft_default()))
    frame=ttk.Frame(root,padding=22);frame.pack(fill='both',expand=True)
    language_row=ttk.Frame(frame);language_row.pack(fill='x')
    ttk.Label(language_row,text='Idioma / Language').pack(side='left')
    menu=ttk.Combobox(language_row,textvariable=choice,values=['Español','English'],state='readonly',width=12);menu.pack(side='right')
    intro=ttk.Label(frame,wraplength=710);intro.pack(anchor='w',pady=(5,15))
    labels=[];buttons=[]
    for key,var in [('open',og),('mc',mc)]:
        label=ttk.Label(frame);label.pack(anchor='w');labels.append((label,key))
        row=ttk.Frame(frame);row.pack(fill='x',pady=(3,14))
        ttk.Entry(row,textvariable=var).pack(side='left',fill='x',expand=True)
        def choose(v=var):
            selected=filedialog.askdirectory(initialdir=v.get())
            if selected:v.set(selected)
        button=ttk.Button(row,command=choose);button.pack(side='right',padx=(8,0));buttons.append(button)
    status=ttk.Label(frame,wraplength=710);status.pack(anchor='w',pady=10)
    progress=ttk.Progressbar(frame,mode='indeterminate');progress.pack(fill='x')
    messages=queue.Queue();running=False
    def start():
        nonlocal running
        if running:return
        running=True;install_button.configure(state='disabled');menu.configure(state='disabled');progress.start()
        status.configure(text=TEXT[lang.get()]['busy'])
        chosen=lang.get();op=og.get();mp=mc.get()
        def work():
            try:
                log=Path(mp)/'instances/jakmod/jakmc/setup/installer.log'
                log.parent.mkdir(parents=True,exist_ok=True)
                with log.open('a',encoding='utf-8') as output, contextlib.redirect_stdout(output), contextlib.redirect_stderr(output):
                    result=install(op,mp,chosen,lambda s:messages.put(('status',s)))
                messages.put(('done',result))
            except Exception as e:messages.put(('error',str(e)))
        threading.Thread(target=work,daemon=False).start()
    install_button=ttk.Button(frame,command=start);install_button.pack(anchor='e',pady=16)
    def translate(*_):
        lang.set('es' if choice.get()=='Español' else 'en')
        t=TEXT[lang.get()];root.title(t['title']);intro.configure(text=t['intro'])
        for label,key in labels:label.configure(text=t[key])
        for button in buttons:button.configure(text=t['browse'])
        install_button.configure(text=t['install'])
    menu.bind('<<ComboboxSelected>>',translate);translate()
    def poll():
        nonlocal running
        try:
            while True:
                kind,value=messages.get_nowait();t=TEXT[lang.get()]
                if kind=='status':status.configure(text=value)
                else:
                    running=False;progress.stop();install_button.configure(state='normal');menu.configure(state='readonly')
                    if kind=='error':messagebox.showerror(t['error'],value)
                    else:messagebox.showinfo(t['done'],t['finished'].format(version=value['profile']))
        except queue.Empty:pass
        root.after(150,poll)
    root.protocol('WM_DELETE_WINDOW',lambda:None if running else root.destroy())
    poll();root.mainloop()

if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--opengoal',type=Path);parser.add_argument('--minecraft',type=Path)
    parser.add_argument('--language',choices=['es','en'],default='es');parser.add_argument('--assets-only',action='store_true')
    args=parser.parse_args()
    if args.opengoal:
        if getattr(sys,'frozen',False):
            log=(args.minecraft or minecraft_default())/'instances/jakmod/jakmc/setup/installer-cli.log'
            log.parent.mkdir(parents=True,exist_ok=True)
            with log.open('a',encoding='utf-8') as output, contextlib.redirect_stdout(output),contextlib.redirect_stderr(output):
                result=install(args.opengoal,args.minecraft or minecraft_default(),args.language,assets_only=args.assets_only)
                print(json.dumps(result))
        else:
            result=install(args.opengoal,args.minecraft or minecraft_default(),args.language,assets_only=args.assets_only)
            print(json.dumps(result))
    else:gui()
