"""Convert owned OpenGOAL SFX to Minecraft Vorbis, preserving bank provenance.

Assets are generated/ignored. No downloads, synthesis, or changes to source games.
Duplicate names resolve in the same bank order as the former Jak audio host;
every bank-specific variant remains addressable as well.
"""
from pathlib import Path
from concurrent.futures import ThreadPoolExecutor
import argparse, hashlib, json, re, shutil, subprocess, wave

P = Path(__file__).resolve().parents[1]
BANKS = ('MODELIT MODEDARK MODEBORD MODEGUY1 MODEGUY2 MODEGUY3 MODEGUR1 '
         'MODEGUR2 MODEGUR3 MODEGUB1 MODEGUB2 MODEGUB3 MODEGUD1 MODEGUD2 '
         'MODEGUD3 COMMONJ COMMON BLOWBAR3 CITYFFH CITYKGF CTYWIDE3 BOARD '
         'FACTORY2 BLOW3H CITYCARH DESERT1 FOREST3 FACTORY6 WASCITY1 '
         'WASALL1 WASTOAD WASSNAKE').split()
JAK1_BANKS = ('COMMON COMMONJ VILLAGE1 VILLAGE2 VILLAGE3 BEACH JUNGLE JUNGLEB '
              'MISTY TRAINING FIRECANY ROLLING SWAMP OGRE SNOW SUNKEN MAINCAVE '
              'DARKCAVE ROBOCAVE LAVATUBE CITADEL FINALBOS').split()

# The Rust host's mix (crates/audio/src/jak.rs mix_gain, Adrii 19/10: Jak's
# steps and effects sounded faint). The exported banks play far quieter than
# Minecraft's own files; Minecraft clamps a sound's volume at 1, so the gain is
# baked into each clip instead of applied at playback. Jak 1 x1.6, Jak 2/3 x4.5,
# Jak 2/3 footsteps (walk, run, land, roll, slide) four times that again.
STEP_PREFIXES = ('walk_', 'run_', 'land_', 'roll_', 'slide_')
def mix_gain(game, name):
    if game == 'jak1':
        return 1.6
    return 4.5 * 4.0 if name.startswith(STEP_PREFIXES) else 4.5

def resource(digest, gain):
    return f'sfx/{digest}' if gain == 1 else f'sfx/{digest}_g{round(gain * 10)}'

def key(s):
    return re.sub('[^a-z0-9_.-]', '_', s.lower()).replace('-', '_')

def named_banks(root, decoder, clips, sounds):
    local = P/'.local/vehicle-audio'
    local.mkdir(parents=True, exist_ok=True)
    def run(*args):
        return subprocess.check_output([decoder, *map(str, args)], text=True,
            creationflags=getattr(subprocess, 'CREATE_NO_WINDOW', 0))
    imported = []
    # The zoomer hop (vehicle-util.gc `sound-play "bike-hop"`) is Jak 2's
    # CTYWIDE2 stream, which the decompiler's SFX export leaves out.
    for game, bank_name, prefixes in [('jak3', 'WASTOAD', ('toad_',)), ('jak3', 'WASSNAKE', ('snake_',)),
                                ('jak3', 'WASCITY1', ('wlander_', 'was_', 'wasf_')),
                                ('jak2', 'CTYWIDE2', ('bike_hop',))]:
        bank = root/f'{game}/data/iso_data/{game}/SBK/{bank_name}.SBK'
        if not bank.is_file():
            continue
        bank_hash = hashlib.sha256(bank.read_bytes()).hexdigest()
        total = json.loads(run('-I', '-s', 1, bank))['streamInfo']['total']
        events = {}
        for index in range(1, total+1):
            meta = json.loads(run('-I', '-s', index, bank))
            name = key(meta['streamInfo']['name'].rsplit('/', 1)[-1])
            if not name.startswith(prefixes):
                continue
            stem = f'{bank_name}-{bank_hash[:12]}-{index}-{name}'
            clip = local/f'{stem}.wav'
            loop = meta['loopingInfo']
            if not clip.exists():
                decoded = local/f'{stem}-decoded.wav'
                run('-i', '-s', index, '-o', decoded, bank)
                if loop:
                    with wave.open(str(decoded), 'rb') as original, wave.open(str(clip), 'wb') as output:
                        output.setparams(original.getparams())
                        original.setpos(loop['start'])
                        output.writeframes(original.readframes(loop['end']-loop['start']))
                    decoded.unlink()
                else:
                    decoded.replace(clip)
            digest = hashlib.sha256(clip.read_bytes()).hexdigest()
            gain = mix_gain(game, name)
            clips.setdefault((digest, gain), clip)
            events.setdefault(name, {})[resource(digest, gain)] = bool(loop)
            imported.append({'name':name, 'game':game, 'bank':str(bank), 'bank_sha256':bank_hash, 'stream':index,
                'loop':loop, 'wav_sha256':digest})
        if bank_name == 'WASCITY1':
            # WLANDER_FIRE is an authored bank-level random cue with individual shot streams.
            shots = {digest:loop for name, variants in events.items() if re.fullmatch(r'wlander_shot[1-7]',name)
                     for digest,loop in variants.items()}
            if shots:
                events['wlander_fire'] = shots
        for name, variants in events.items():
            prefer_loop = any(variants.values())
            files = [{'name':f'jakmc:{res}', 'attenuation_distance':32}
                for res,looped in variants.items() if looped or not prefer_loop]
            event = {'sounds':files}
            sounds[f'sfx/{game}/{key(bank_name)}/{name}'] = event
            sounds[f'sfx/{game}/{name}'] = event
            sounds[f'sfx/{name}'] = event
            # Jak 3's zoomers are Jak 2's bikes: their cues look the hop up there.
            sounds.setdefault(f'sfx/jak3/{name}', event)
    return imported


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--opengoal', type=Path, default=Path.home()/'Documents/Jak OpenGOAL/active')
    parser.add_argument('--vgmstream', type=Path, default=P/'.local/vgmstream/vgmstream-cli.exe')
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
            gain = mix_gain(game, name)
            clips.setdefault((digest, gain), src)
            sounds[f'sfx/{game}/{key(src.parent.name)}/{name}'] = {'sounds': [{'name': f'jakmc:{resource(digest, gain)}', 'attenuation_distance': 32}]}
        for name, src in chosen.items():
            digest = hashlib.sha256(src.read_bytes()).hexdigest()
            event = {'sounds': [{'name': f'jakmc:{resource(digest, mix_gain(game, name))}', 'attenuation_distance': 32}]}
            sounds[f'sfx/{game}/{name}'] = event
            if game in ('jak3', 'jak2'):
                defaults.setdefault(name, event)
    for name, event in defaults.items():
        sounds[f'sfx/{name}'] = event
    owned_banks = named_banks(args.opengoal, args.vgmstream, clips, sounds) if args.vgmstream.is_file() else []
    target = out/'sounds/sfx'
    target.mkdir(parents=True, exist_ok=True)
    def convert(pair):
        (digest, gain), src = pair
        dest = out/'sounds'/f'{resource(digest, gain)}.ogg'
        if not dest.exists():
            temp = dest.with_suffix('.tmp.ogg')
            # A limiter that only acts above full scale keeps the loudest clips from clipping.
            mix = [] if gain == 1 else ['-af', f'volume={gain},alimiter=limit=0.98:level=disabled']
            result = subprocess.run([ffmpeg, '-hide_banner', '-loglevel', 'error', '-nostdin', '-y',
                '-i', str(src), '-vn', '-ac', '1', *mix, '-c:a', 'libvorbis', '-q:a', '5', str(temp)],
                capture_output=True, text=True, creationflags=getattr(subprocess, 'CREATE_NO_WINDOW', 0))
            if result.returncode:
                raise RuntimeError(f'{src}: {result.stderr}')
            temp.replace(dest)
        return {'sha256': digest, 'gain': gain, 'source': str(src), 'output': str(dest.relative_to(P)), 'bytes': dest.stat().st_size}
    receipt = []
    with ThreadPoolExecutor(max_workers=4) as pool:
        for i, row in enumerate(pool.map(convert, clips.items()), 1):
            receipt.append(row)
            if i % 200 == 0: print(f'Converted {i}/{len(clips)} unique original SFX', flush=True)
    (out/'sounds.json').write_text(json.dumps(sounds, separators=(',', ':')), encoding='utf-8')
    report = {'clips': len(receipt), 'events': len(sounds), 'unavailable_exports': missing, 'sources': receipt, 'decoded_banks':owned_banks}
    (P/'.local/audio-import-receipt.json').write_text(json.dumps(report, indent=2), encoding='utf-8')
    print(json.dumps({k:v for k,v in report.items() if k not in ('sources','decoded_banks')}), flush=True)

if __name__ == '__main__': main()
