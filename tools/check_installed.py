"""Bounded production-JAR check in an explicitly selected lab installation.

Uses a private copy of a development world, no launcher credentials or OS input.
Never runs against the personal Minecraft folder. Exits its own game process.
"""
from pathlib import Path
import argparse,json,os,re,shutil,subprocess,time,uuid,zipfile

ROOT=Path(__file__).resolve().parents[1]

def allowed(rules):
    if not rules:return True
    yes=False
    for rule in rules:
        platform=rule.get('os',{})
        if platform.get('name','windows')!='windows':continue
        if 'arch' in platform and not re.fullmatch(platform['arch'],'amd64'):continue
        if any(rule.get('features',{}).values()):continue
        yes=rule['action']=='allow'
    return yes

def call(instance,op,**data):
    folder=instance/'jakmc/bridge';ready=json.loads((folder/'ready.json').read_text())
    key='installed_'+uuid.uuid4().hex
    request=dict(id=key,session=ready['session'],op=op,**data)
    temp=folder/'request.tmp';temp.write_text(json.dumps(request));temp.replace(folder/'request.json')
    result=folder/(key+'.json');deadline=time.monotonic()+12
    while not result.exists():
        if time.monotonic()>deadline:raise TimeoutError(op)
        time.sleep(.1)
    row=json.loads(result.read_text())
    if not row.get('ok'):raise RuntimeError(row)
    return row

def main():
    parser=argparse.ArgumentParser();parser.add_argument('--minecraft',type=Path,required=True)
    parser.add_argument('--world-template',type=Path,default=ROOT/'.local/test-world-template')  # a new world from these settings (run_scene.py keeps them)
    args=parser.parse_args();mc=args.minecraft.resolve()
    if not mc.is_relative_to(ROOT/'.local'):raise SystemExit('Only a private .local lab installation is supported')
    personal=Path.home()/'AppData/Roaming/.minecraft';instance=mc/'instances/jakmod'
    base=json.loads((mc/'versions/26.3/26.3.json').read_text())
    fabric=json.loads((mc/'versions/26.3.jakmod/26.3.jakmod.json').read_text())
    classpath=[];natives=instance/'jakmc/probe-natives';natives.mkdir(parents=True,exist_ok=True)
    for lib in base['libraries']+fabric['libraries']:
        if not allowed(lib.get('rules')):continue
        artifact=lib.get('downloads',{}).get('artifact',{})
        if 'path' in artifact:relative=artifact['path']
        else:
            group,name,version=lib['name'].split(':');relative=f'{group.replace(".","/")}/{name}/{version}/{name}-{version}.jar'
        path=next((p for p in (mc/'libraries'/relative,personal/'libraries'/relative) if p.is_file()),None)
        if path is None:raise FileNotFoundError(relative)
        classpath.append(str(path))
        with zipfile.ZipFile(path) as z:
            for name in z.namelist():
                if name.endswith('.dll'):(natives/Path(name).name).write_bytes(z.read(name))
    classpath.append(str(mc/'versions/26.3/26.3.jar'))
    name='Installer-check-'+uuid.uuid4().hex[:8]
    world=instance/'saves'/name
    if args.world_template.is_dir():
        shutil.copytree(args.world_template,world,ignore=shutil.ignore_patterns('session.lock'))
    else:
        if args.world_template!=ROOT/'.local/test-world-template':raise FileNotFoundError(args.world_template)
        with zipfile.ZipFile(ROOT/'dist/JakShowcase-world.zip') as archive:
            for entry in archive.infolist():
                relative=Path(entry.filename).relative_to('Jak Showcase')
                if entry.is_dir():continue
                if '..' in relative.parts or relative.is_absolute():raise ValueError(entry.filename)
                path=world/relative
                path.parent.mkdir(parents=True,exist_ok=True)
                path.write_bytes(archive.read(entry))
    # Profile belongs to the lab; prevent alt-tab pauses from blocking a scene.
    (instance/'options.txt').write_text('lang:es_es\npauseOnLostFocus:false\nmaxFps:60\nrenderDistance:8\nsimulationDistance:5\n')
    java=Path(os.environ['JAVA_HOME'])/'bin/java.exe'
    token=uuid.uuid4().hex
    permit=instance/'jakmc/allow-production-probe';permit.write_text(token)
    replacements={'natives_directory':str(natives),'launcher_name':'JakModProductionCheck','launcher_version':'1',
                  'classpath':os.pathsep.join(classpath)}
    jvm=[]
    for arg in base['arguments']['jvm']+fabric.get('arguments',{}).get('jvm',[]):
        if isinstance(arg,dict):
            if not allowed(arg.get('rules')):continue
            arg=arg['value']
        for value in (arg if isinstance(arg,list) else [arg]):
            for key,replacement in replacements.items():value=value.replace('${'+key+'}',replacement)
            jvm.append(value)
    args=[str(java),'-Xmx3G','-Djakmc.devBridge=true','-Djakmc.productionProbe='+token,*jvm,fabric['mainClass'],
          '--username','JakModCheck','--uuid','00000000000000000000000000000001',
          '--accessToken','0','--version','26.3.jakmod','--gameDir',str(instance),
          '--assetsDir',str(personal/'assets'),'--assetIndex',base['assetIndex']['id'],
          '--width','1280','--height','720','--quickPlaySingleplayer',name]
    evidence=ROOT/'.local/evidence/installer-production';evidence.mkdir(parents=True,exist_ok=True)
    ready=instance/'jakmc/bridge/ready.json'
    previous=ready.read_bytes() if ready.exists() else None
    capture_after=time.time_ns()
    with (evidence/'game.log').open('w',encoding='utf-8') as log:
        # CreateProcessW preserves non-ASCII paths in the classpath and game directory.
        process=subprocess.Popen(args,cwd=instance,stdout=log,stderr=subprocess.STDOUT,
                                 creationflags=subprocess.CREATE_NO_WINDOW)
        try:
            deadline=time.monotonic()+100;warmup=None
            row=None;entry_camera=None
            while time.monotonic()<deadline:
                if process.poll() is not None:raise RuntimeError('Production game exited: '+str(process.returncode))
                if ready.exists() and ready.read_bytes()!=previous:
                    state=call(instance,'observe')
                    if state.get('player'):
                        row=call(instance,'package_status')
                        if warmup is None:warmup=time.monotonic()
                        assets=all(value for key,value in row.items() if key.startswith('/'))
                        if assets and row.get('enabled') and row.get('actor_drawn') and row.get('source_pose') and row.get('cinematic_draws',0)>0 and time.monotonic()-warmup>2:
                            entry_camera=row.get('camera_before_probe')
                            if entry_camera!='THIRD_PERSON_BACK':raise RuntimeError('Jak did not enter its default third-person camera: '+str(row))
                            call(instance,'package_camera');time.sleep(.6)
                            call(instance,'package_capture')
                            end=time.monotonic()+8;image=instance/'screenshots/installed-jak.png'
                            def complete_image():
                                return image.exists() and image.stat().st_mtime_ns>=capture_after and image.read_bytes().endswith(b'\x00\x00\x00\x00IEND\xaeB\x60\x82')
                            while not complete_image() and time.monotonic()<end:time.sleep(.1)
                            if not complete_image():raise RuntimeError('Screenshot was not completely saved')
                            shutil.copyfile(image,evidence/image.name)
                            row['camera_on_entry']=entry_camera
                            (evidence/'package-result.json').write_text(json.dumps(row,indent=2))
                            break
                time.sleep(.5)
            else:raise TimeoutError('Production check')
            for image in (instance/'screenshots').glob('scale-*.png'):
                if image.stat().st_mtime_ns>=capture_after:shutil.copyfile(image,evidence/image.name)
            image=instance/'screenshots/installed-jak.png'
            if image.exists() and image.stat().st_mtime_ns>=capture_after:shutil.copyfile(image,evidence/image.name)
            print(json.dumps(row,indent=2))
        finally:
            if process.poll() is None:
                try:call(instance,'quit');process.wait(timeout=20)
                except Exception:process.terminate();process.wait(timeout=10)
            for image in (instance/'screenshots').glob('scale-*.png'):
                if image.stat().st_mtime_ns>=capture_after:shutil.copyfile(image,evidence/image.name)
            if world.exists() and world.resolve().parent==(instance/'saves').resolve():
                shutil.rmtree(world)
            permit.unlink(missing_ok=True)

if __name__=='__main__':main()
