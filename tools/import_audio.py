"""Convert owned OpenGOAL SFX to Minecraft Vorbis, preserving bank provenance.

Assets are generated/ignored. No downloads, synthesis, or changes to source games.
Duplicate names resolve in the same bank order as the former Jak audio host;
every bank-specific variant remains addressable as well.
"""
from pathlib import Path
from concurrent.futures import ThreadPoolExecutor
import argparse, hashlib, json, re, shutil, subprocess

P = Path(__file__).resolve().parents[1]
BANKS = ('MODELIT MODEDARK MODEBORD MODEGUY1 MODEGUY2 MODEGUY3 MODEGUR1 '
         'MODEGUR2 MODEGUR3 MODEGUB1 MODEGUB2 MODEGUB3 MODEGUD1 MODEGUD2 '
         'MODEGUD3 COMMONJ COMMON BLOWBAR3 CITYFFH CITYKGF CTYWIDE3 BOARD '
         'FACTORY2 BLOW3H CITYCARH DESERT1 FOREST3 FACTORY6 WASCITY1 '
         'WASALL1 WASTOAD WASSNAKE').split()
JAK1_BANKS = ('COMMON COMMONJ VILLAGE1 VILLAGE2 VILLAGE3 BEACH JUNGLE JUNGLEB '
              'MISTY TRAINING FIRECANY ROLLING SWAMP OGRE SNOW SUNKEN MAINCAVE '
              'DARKCAVE ROBOCAVE LAVATUBE CITADEL FINALBOS').split()

def key(s):
    return re.sub('[^a-z0-9_.-]', '_', s.lower()).replace('-', '_')

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--opengoal', type=Path, default=Path.home()/'Documents/Jak OpenGOAL/active')
    args = parser.parse_args()
    ffmpeg = shutil.which('ffmpeg')
    if not ffmpeg:
        raise SystemExit('ffmpeg is required to convert the owned WAV exports')
    out = P/'src/generated/resources/assets/jakmc'
    out.mkdir(parents=True, exist_ok=True)
    sounds, clips, defaults, missing = {}, {}, {}, []
    for game in ['jak3', 'jak2', 'jak1']:
        root = args.opengoal/game/'data/decompiler_out'/game/'audio/sfx'
        if not root.is_dir():
            missing.append(str(root)); continue
        order = JAK1_BANKS if game == 'jak1' else BANKS
        files = sorted(root.rglob('*.wav'), key=lambda f: (
            order.index(f.parent.name) if f.parent.name in order else 999, str(f)))
        chosen = {}
        for src in files:
            name = key(src.stem)
            chosen.setdefault(name, src)
            digest = hashlib.sha256(src.read_bytes()).hexdigest()
            clips.setdefault(digest, src)
            resource = f'sfx/{digest}'
            sounds[f'sfx/{game}/{key(src.parent.name)}/{name}'] = {'sounds': [{'name': f'jakmc:{resource}', 'attenuation_distance': 32}]}
        for name, src in chosen.items():
            digest = hashlib.sha256(src.read_bytes()).hexdigest()
            event = {'sounds': [{'name': f'jakmc:sfx/{digest}', 'attenuation_distance': 32}]}
            sounds[f'sfx/{game}/{name}'] = event
            if game in ('jak3', 'jak2'):
                defaults.setdefault(name, event)
    for name, event in defaults.items():
        sounds[f'sfx/{name}'] = event
    target = out/'sounds/sfx'
    target.mkdir(parents=True, exist_ok=True)
    def convert(pair):
        digest, src = pair
        dest = target/f'{digest}.ogg'
        if not dest.exists():
            temp = dest.with_suffix('.tmp.ogg')
            result = subprocess.run([ffmpeg, '-hide_banner', '-loglevel', 'error', '-nostdin', '-y',
                '-i', str(src), '-vn', '-ac', '1', '-c:a', 'libvorbis', '-q:a', '5', str(temp)],
                capture_output=True, text=True, creationflags=getattr(subprocess, 'CREATE_NO_WINDOW', 0))
            if result.returncode:
                raise RuntimeError(f'{src}: {result.stderr}')
            temp.replace(dest)
        return {'sha256': digest, 'source': str(src), 'output': str(dest.relative_to(P)), 'bytes': dest.stat().st_size}
    receipt = []
    with ThreadPoolExecutor(max_workers=4) as pool:
        for i, row in enumerate(pool.map(convert, clips.items()), 1):
            receipt.append(row)
            if i % 200 == 0: print(f'Converted {i}/{len(clips)} unique original SFX', flush=True)
    (out/'sounds.json').write_text(json.dumps(sounds, separators=(',', ':')), encoding='utf-8')
    report = {'clips': len(receipt), 'events': len(sounds), 'unavailable_exports': missing, 'sources': receipt}
    (P/'.local/audio-import-receipt.json').write_text(json.dumps(report, indent=2), encoding='utf-8')
    print(json.dumps({k:v for k,v in report.items() if k != 'sources'}), flush=True)

if __name__ == '__main__': main()
