"""Run one bounded native scene and exit. Never leaves an idle test window open."""
from pathlib import Path
import argparse,datetime,json,os,shutil,subprocess,time
from bridge import call,ROOT

def run():
    parser=argparse.ArgumentParser();parser.add_argument('--scene',choices=['scale','effects','enemies','systems','combat'],default='scale');scene=parser.parse_args().scene
    stamp=datetime.datetime.now().strftime('%Y%m%d-%H%M%S')
    evidence=ROOT/'.local/evidence'/(scene+'-'+stamp);evidence.mkdir(parents=True)
    result=ROOT/f'run/jakmc/evidence/{scene}/result.json'
    image=ROOT/('run/screenshots/scale-two-block-ceiling.png' if scene=='scale' else 'run/screenshots/port-blaster.png' if scene=='effects' else 'run/screenshots/enemy-babak-idle.png' if scene=='enemies' else 'run/screenshots/enemy-chase-step.png')
    if result.exists():shutil.copyfile(result,evidence/'previous-result.json')
    if image.exists():shutil.copyfile(image,evidence/'previous-screen.png')
    ready=ROOT/'run/jakmc/bridge/ready.json'
    prior=json.loads(ready.read_text(encoding='utf-8'))['session'] if ready.exists() else None
    env=os.environ.copy()
    env['JAVA_HOME']=str(Path.home()/'AppData/Local/Packages/Microsoft.4297127D64EC6_8wekyb3d8bbwe/LocalCache/Local/runtime/java-runtime-epsilon/windows-x64/java-runtime-epsilon')
    gradle=next((Path.home()/'.gradle/wrapper/dists/gradle-9.8.0-bin').glob('*/gradle-9.8.0/bin/gradle.bat'))
    log=(evidence/'game.log').open('w',encoding='utf-8')
    process=subprocess.Popen([str(gradle),'runClient','--no-daemon','--console=plain'],cwd=ROOT,env=env,stdout=log,stderr=subprocess.STDOUT,creationflags=subprocess.CREATE_NO_WINDOW)
    start=time.monotonic();sent=False;owned=None
    try:
        while process.poll() is None:
            if time.monotonic()-start>150:raise TimeoutError('Native scene startup exceeded 150 seconds')
            if ready.exists():
                current=json.loads(ready.read_text(encoding='utf-8'))
                if current['session']!=prior:
                    owned=current
                    observation=call('observe')
                    if 'player' in observation:
                        (evidence/'before.json').write_text(json.dumps(observation,indent=2),encoding='utf-8')
                        call(scene+'_scene');sent=True;break
            time.sleep(.25)
        if not sent:raise RuntimeError('Game exited before the requested scene started')
        process.wait(timeout=120 if scene=='enemies' else 60 if scene=='systems' else 40)
        if process.returncode!=0:raise RuntimeError(f'Native game exited {process.returncode}')
        shutil.copyfile(result,evidence/'result.json');shutil.copyfile(image,evidence/'screen.png')
        if scene=='effects':
            shutil.copyfile(ROOT/'run/screenshots/port-board-zap.png',evidence/'board.png')
            shutil.copyfile(ROOT/'run/screenshots/port-wave.png',evidence/'wave.png')
        if scene in ('enemies','systems','combat'):
            for capture in (ROOT/'run/screenshots').glob('enemy-*.png'):
                shutil.copyfile(capture,evidence/capture.name)
        outcome=json.loads(result.read_text(encoding='utf-8'))
        print(json.dumps({'evidence':str(evidence),'result':outcome},indent=2),flush=True)
    except Exception as failure:
        (evidence/'runner-error.json').write_text(json.dumps({'error':str(failure),'owned_session':owned},indent=2),encoding='utf-8')
        if owned and process.poll() is None:
            # Exact session PID, never a wildcard Java kill. Preserve the actual
            # shutdown/build failure instead of leaving an idle test window.
            subprocess.run([str(Path(env['JAVA_HOME'])/'bin/jcmd.exe'),str(int(owned['pid'])),'Thread.print'],stdout=(evidence/'timeout-threads.txt').open('w',encoding='utf-8'),stderr=subprocess.STDOUT,timeout=10,creationflags=subprocess.CREATE_NO_WINDOW)
            pid=int(owned['pid'])
            script=f"$p=Get-CimInstance Win32_Process -Filter 'ProcessId={pid}'; if($p.CommandLine -like '*net.fabricmc.devlaunchinjector.Main*' -and $p.CommandLine -like '*jak-minecraft-java*'){{Stop-Process -Id {pid}}}"
            subprocess.run(['powershell','-NoProfile','-Command',script],creationflags=subprocess.CREATE_NO_WINDOW,timeout=10)
            try:process.wait(timeout=20)
            except subprocess.TimeoutExpired:pass
        raise
    finally:
        # Copy evidence on failure too. A result is not a normal-shutdown pass.
        if sent and result.exists():shutil.copyfile(result,evidence/'result.json')
        if sent and scene in ('enemies','systems','combat'):
            for capture in (ROOT/'run/screenshots').glob('enemy-*.png'):shutil.copyfile(capture,evidence/capture.name)
        log.close()

if __name__=='__main__':run()
