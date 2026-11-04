"""Run one bounded native scene and exit. Never leaves an idle test window open."""
from pathlib import Path
import argparse,datetime,json,os,shutil,subprocess,time
from bridge import call,ROOT

def owned_game_pid(parent_pid):
    # Before the bridge starts, only a game descending from this one Gradle launch is ours.
    script=r"""
    $rootProcess=__PARENT__
    $tree=@{}
    Get-CimInstance Win32_Process | ForEach-Object {$tree[[int]$_.ProcessId]=$_}
    foreach($candidate in $tree.Values){
        if($candidate.CommandLine -notlike '*net.fabricmc.devlaunchinjector.Main*' -or
           $candidate.CommandLine -notlike '*jak-minecraft-java*'){continue}
        $parent=[int]$candidate.ParentProcessId
        $seen=[System.Collections.Generic.HashSet[int]]::new()
        while($parent -gt 0 -and $seen.Add($parent)){
            if($parent -eq $rootProcess){[int]$candidate.ProcessId; break}
            if(!$tree.ContainsKey($parent)){break}
            $parent=[int]($tree[$parent].ParentProcessId)
        }
    }
    """.replace('__PARENT__',str(parent_pid))
    completed=subprocess.run(['powershell','-NoProfile','-Command',script],
        creationflags=subprocess.CREATE_NO_WINDOW,capture_output=True,text=True,timeout=10)
    pids=[int(line) for line in completed.stdout.splitlines() if line.strip().isdigit()]
    return pids[0] if len(pids)==1 else None


def run():
    parser=argparse.ArgumentParser();parser.add_argument('--scene',choices=['scale','effects','enemies','systems','combat','weapons','rings','input','input_actions','menus','gems','gems_visual','collision_perf','citizens','player_sleep','yakow','spydroid','powers','dark_motion','leaper','flut','hellcat','helldog','hellcat_crossing','materials','vents','weather','altar','track','ladder','surface','cars','cities','sites','zoomers'],default='scale');parser.add_argument('--origin',nargs=3,type=float);parser.add_argument('--fresh-world',action='store_true');args=parser.parse_args();scene=args.scene
    stamp=datetime.datetime.now().strftime('%Y%m%d-%H%M%S')
    evidence=ROOT/'.local/evidence'/(scene+'-'+stamp);evidence.mkdir(parents=True)
    result=ROOT/f'run/jakmc/evidence/{scene}/result.json'
    image=ROOT/('run/screenshots/zoomers-bike.png' if scene=='zoomers' else 'run/screenshots/cities-haven.png' if scene=='cities' else 'run/screenshots/cars-toad.png' if scene=='cars' else 'run/screenshots/jak-track.png' if scene=='track' else 'run/screenshots/jak-ladder.png' if scene=='ladder' else 'run/screenshots/jak-surface-ice.png' if scene=='surface' else 'run/screenshots/jak-altar.png' if scene=='altar' else 'run/screenshots/weather-storm.png' if scene=='weather' else 'run/screenshots/jak-vents.png' if scene=='vents' else 'run/screenshots/jak-materials.png' if scene=='materials' else 'run/screenshots/hellcat-idle.png' if scene=='helldog' else f'run/screenshots/{scene}-idle.png' if scene in ('leaper','flut','hellcat','helldog','hellcat_crossing') else 'run/screenshots/powers-dark.png' if scene=='powers' else 'run/screenshots/jak-dark-run.png' if scene=='dark_motion' else 'run/screenshots/citizens-standing.png' if scene=='citizens' else 'run/screenshots/jak-sleep.png' if scene=='player_sleep' else 'run/screenshots/jak-yakow.png' if scene=='yakow' else 'run/screenshots/collision-perf.png' if scene=='collision_perf' else 'run/screenshots/gems-palm-day.png' if scene=='gems_visual' else 'run/screenshots/gems-world-110.png' if scene=='gems' else 'run/screenshots/menus-anvil.png' if scene=='menus' else 'run/screenshots/input-actions-gui.png' if scene=='input_actions' else 'run/screenshots/input-move-86.png' if scene=='input' else 'run/screenshots/scale-two-block-ceiling.png' if scene=='scale' else 'run/screenshots/weapon-10-28.png' if scene=='rings' else 'run/screenshots/weapon-7-28.png' if scene in ('weapons','rings','input','input_actions') else 'run/screenshots/port-blaster.png' if scene=='effects' else 'run/screenshots/enemy-babak-idle.png' if scene=='enemies' else 'run/screenshots/spydroid-explosion.png' if scene=='spydroid' else 'run/screenshots/enemy-chase-step.png')
    if result.exists():shutil.copyfile(result,evidence/'previous-result.json')
    if scene!='sites' and image.exists():shutil.copyfile(image,evidence/'previous-screen.png')
    ready=ROOT/'run/jakmc/bridge/ready.json'
    prior=json.loads(ready.read_text(encoding='utf-8'))['session'] if ready.exists() else None
    env=os.environ.copy()
    env['JAVA_HOME']=str(Path.home()/'AppData/Local/Packages/Microsoft.4297127D64EC6_8wekyb3d8bbwe/LocalCache/Local/runtime/java-runtime-epsilon/windows-x64/java-runtime-epsilon')
    gradle=next((Path.home()/'.gradle/wrapper/dists/gradle-9.8.0-bin').glob('*/gradle-9.8.0/bin/gradle.bat'))
    log=(evidence/'game.log').open('w',encoding='utf-8')
    command=[str(gradle),'runClient','--no-daemon','--console=plain']
    if args.fresh_world:
        dev_save=ROOT/'run/saves/New World (1)'
        world='Jak worldgen '+stamp
        fresh=ROOT/'run/saves'/world
        fresh.mkdir()
        for metadata in ('level.dat','data/minecraft/world_gen_settings.dat'):
            path=fresh/metadata
            path.parent.mkdir(parents=True,exist_ok=True)
            shutil.copyfile(dev_save/metadata,path)
        command.append('-PjakmcDevWorld='+world)
    process=subprocess.Popen(command,cwd=ROOT,env=env,stdout=log,stderr=subprocess.STDOUT,creationflags=subprocess.CREATE_NO_WINDOW)
    start=time.monotonic();capture_after=time.time_ns();sent=False;owned=None
    try:
        while process.poll() is None:
            if time.monotonic()-start>150:raise TimeoutError('Native scene startup exceeded 150 seconds')
            if ready.exists():
                current=json.loads(ready.read_text(encoding='utf-8'))
                if current['session']!=prior:
                    owned=current
                    observation=call('observe')
                    if 'player' in observation and observation.get('screen') is None:
                        (evidence/'before.json').write_text(json.dumps(observation,indent=2),encoding='utf-8')
                        call(scene+'_scene',**({'origin':args.origin} if args.origin is not None else {}));sent=True;break
            time.sleep(.25)
        if not sent:raise RuntimeError('Game exited before the requested scene started')
        process.wait(timeout=120 if scene in ('enemies','weapons','input','gems','citizens','cities','sites') else 90 if scene in ('powers','hellcat_crossing','weather') else 60 if scene=='systems' else 40)
        if process.returncode!=0:raise RuntimeError(f'Native game exited {process.returncode}')
        shutil.copyfile(result,evidence/'result.json')
        if scene in ('weapons','rings','input','input_actions','menus','gems','gems_visual','collision_perf','citizens','player_sleep','yakow','spydroid','powers','dark_motion','leaper','flut','hellcat','helldog','hellcat_crossing','materials','vents','weather','altar','track','ladder','surface','cars','cities','sites','zoomers') and json.loads(result.read_text(encoding='utf-8')).get('reason')!='complete':raise RuntimeError(f'{scene} scene did not complete; see result.json')
        if scene!='sites':
            if scene in ('dark_motion','surface','player_sleep','yakow') and (not image.exists() or image.stat().st_mtime_ns<capture_after):
                raise RuntimeError(f'{scene} completed without a fresh screenshot')
            shutil.copyfile(image,evidence/'screen.png')
        if scene=='surface':
            shutil.copyfile(ROOT/'run/screenshots/jak-surface-swim.png',evidence/'swim.png')
        if scene=='sites':
            for name in ('sites-haven.png','sites-spargus.png'):
                shot=ROOT/'run/screenshots'/name
                if shot.exists() and shot.stat().st_mtime_ns>=capture_after:shutil.copyfile(shot,evidence/name)
        if scene=='menus':
            for name in ('menus-search.png','menus-trade.png'):
                shutil.copyfile(ROOT/'run/screenshots'/name,evidence/name)
        if scene=='weather':
            shutil.copyfile(ROOT/'run/screenshots/weather-clear.png',evidence/'weather-clear.png')
        if scene=='scale':
            detail=ROOT/'run/screenshots/scale-forward-eyes.png'
            if detail.exists() and detail.stat().st_mtime_ns>=capture_after:shutil.copyfile(detail,evidence/detail.name)
        if scene=='helldog':
            for shot in (ROOT/'run/screenshots').glob('hellcat-*.png'):
                if shot.stat().st_mtime_ns>=capture_after:shutil.copyfile(shot,evidence/shot.name)
        if scene=='cities':
            for name in ('cities-spargus.png','cities-guard.png','cities-wastelander.png'):
                shot=ROOT/'run/screenshots'/name
                if shot.exists() and shot.stat().st_mtime_ns>=capture_after:shutil.copyfile(shot,evidence/name)
        if scene=='track':
            grind=ROOT/'run/screenshots/jak-track-grind.png'
            if grind.exists() and grind.stat().st_mtime_ns>=capture_after:
                shutil.copyfile(grind,evidence/grind.name)
        if scene=='altar':
            for name in ('jak-gate-closed.png','jak-gate-open.png'):
                shot=ROOT/'run/screenshots'/name
                if shot.exists() and shot.stat().st_mtime_ns>=capture_after:
                    shutil.copyfile(shot,evidence/name)
        if scene=='materials':
            for name in ('jak-assemblies.png','jak-facade.png'):
                shot=ROOT/'run/screenshots'/name
                if shot.exists() and shot.stat().st_mtime_ns>=capture_after:
                    shutil.copyfile(shot,evidence/name)
        if scene in ('gems','gems_visual'):
            for capture in (ROOT/'run/screenshots').glob('gems-*.png'):shutil.copyfile(capture,evidence/capture.name)
        if scene in ('input','input_actions'):
            for capture in (ROOT/'run/screenshots').glob('input-*.png'):shutil.copyfile(capture,evidence/capture.name)
        if scene in ('weapons','rings'):
            for capture in (ROOT/'run/screenshots').glob('weapon-*.png'):shutil.copyfile(capture,evidence/capture.name)
        if scene=='effects':
            shutil.copyfile(ROOT/'run/screenshots/port-board-zap.png',evidence/'board.png')
            shutil.copyfile(ROOT/'run/screenshots/port-wave.png',evidence/'wave.png')
        if scene in ('enemies','systems','combat'):
            for capture in (ROOT/'run/screenshots').glob('enemy-*.png'):
                shutil.copyfile(capture,evidence/capture.name)
        if scene=='citizens':
            for capture in (ROOT/'run/screenshots').glob('citizens-*.png'):shutil.copyfile(capture,evidence/capture.name)
        outcome=json.loads(result.read_text(encoding='utf-8'))
        print(json.dumps({'evidence':str(evidence),'result':outcome},indent=2),flush=True)
    except Exception as failure:
        (evidence/'runner-error.json').write_text(json.dumps({'error':str(failure),'owned_session':owned},indent=2),encoding='utf-8')
        if process.poll() is None:
            try:
                pid=int(owned['pid']) if owned else owned_game_pid(process.pid)
            except subprocess.SubprocessError:
                pid=None
            if pid:
                try:
                    with (evidence/'timeout-threads.txt').open('w',encoding='utf-8') as report:
                        subprocess.run([str(Path(env['JAVA_HOME'])/'bin/jcmd.exe'),str(pid),'Thread.print'],
                            stdout=report,stderr=subprocess.STDOUT,timeout=10,creationflags=subprocess.CREATE_NO_WINDOW)
                except subprocess.SubprocessError:
                    pass
                script=f"$p=Get-CimInstance Win32_Process -Filter 'ProcessId={pid}'; if($p.CommandLine -like '*net.fabricmc.devlaunchinjector.Main*' -and $p.CommandLine -like '*jak-minecraft-java*'){{Stop-Process -Id {pid}}}"
                subprocess.run(['powershell','-NoProfile','-Command',script],creationflags=subprocess.CREATE_NO_WINDOW,timeout=10)
            else:
                process.terminate()
            try:process.wait(timeout=20)
            except subprocess.TimeoutExpired:pass
        raise
    finally:
        # Copy evidence on failure too. A result is not a normal-shutdown pass.
        if sent and result.exists():shutil.copyfile(result,evidence/'result.json')
        if sent and scene!='sites' and image.exists() and image.stat().st_mtime_ns>=capture_after:
            shutil.copyfile(image,evidence/'screen.png')
        if sent and scene in ('leaper','flut','hellcat','helldog','hellcat_crossing'):
            for capture in (ROOT/'run/screenshots').glob(f'{scene}-*.png'):
                if capture.stat().st_mtime_ns>=capture_after:shutil.copyfile(capture,evidence/capture.name)
        if sent and scene=='cities':
            for capture in (ROOT/'run/screenshots').glob('cities-*.png'):
                if capture.stat().st_mtime_ns>=capture_after:shutil.copyfile(capture,evidence/capture.name)
        if sent and scene in ('cars','zoomers'):
            for capture in (ROOT/'run/screenshots').glob(scene+'-*.png'):
                if capture.stat().st_mtime_ns>=capture_after:shutil.copyfile(capture,evidence/capture.name)
        if sent and scene=='powers':
            for capture in (ROOT/'run/screenshots').glob('powers-*.png'):shutil.copyfile(capture,evidence/capture.name)
        if sent and scene=='citizens':
            for capture in (ROOT/'run/screenshots').glob('citizens-*.png'):shutil.copyfile(capture,evidence/capture.name)
        if sent and scene in ('enemies','systems','combat'):
            for capture in (ROOT/'run/screenshots').glob('enemy-*.png'):shutil.copyfile(capture,evidence/capture.name)
        if sent and scene in ('gems','gems_visual'):
            for capture in (ROOT/'run/screenshots').glob('gems-*.png'):shutil.copyfile(capture,evidence/capture.name)
        if sent and scene in ('input','input_actions'):
            for capture in (ROOT/'run/screenshots').glob('input-*.png'):shutil.copyfile(capture,evidence/capture.name)
        if sent and scene in ('weapons','rings'):
            for capture in (ROOT/'run/screenshots').glob('weapon-*.png'):shutil.copyfile(capture,evidence/capture.name)
        log.close()

if __name__=='__main__':run()
