"""Reject accidentally bundled retail content, credentials and private outputs."""
from pathlib import Path
import argparse,json,re,subprocess,zipfile

ROOT=Path(__file__).resolve().parents[1]

def main():
    parser=argparse.ArgumentParser();parser.add_argument('--package',type=Path);args=parser.parse_args()
    files=subprocess.check_output(['git','ls-files','-z','--cached','--others','--exclude-standard'],cwd=ROOT).decode().split('\0')
    failures=[]
    for name in set(files)-{''}:
        p=ROOT/name
        if any(part in ('.local','src/generated','dist','run') for part in p.relative_to(ROOT).parts) or name.startswith('src/generated/'):
            failures.append('private path: '+name)
        if p.suffix.lower() in ('.glb','.ogg','.wav','.go','.gc','.iso','.dll','.exe','.rar') or p.name.startswith('.env'):
            failures.append('unexpected binary/game source: '+name)
        if p.is_file() and p.suffix.lower() in ('.py','.java','.rs','.json','.properties','.gradle','.md','.yml'):
            text=p.read_text(encoding='utf-8-sig',errors='replace')
            patterns=[r'gh[pousr]_[A-Za-z0-9]{30,}',r'github_pat_[A-Za-z0-9_]{40,}',r'-----BEGIN (?:RSA |OPENSSH )?PRIVATE KEY-----',r'sk-(?:proj-)?[A-Za-z0-9_-]{48,}']
            if any(re.search(pattern,text) for pattern in patterns):failures.append('possible credential: '+name)
    code=next((ROOT/'build/libs').glob('*-installer-code.jar'))
    private={f.relative_to(ROOT/'src/generated/resources').as_posix() for f in (ROOT/'src/generated/resources').rglob('*') if f.is_file()}
    with zipfile.ZipFile(code) as z:
        overlap=private&set(z.namelist())
        if overlap:failures.append(f'code jar contains {len(overlap)} private resources')
        if z.testzip():failures.append('invalid code jar CRC')
    if args.package:
        # PyInstaller includes redistributable library data; game file formats
        # have no place in the installer tree, except the code-only mod jar.
        for p in args.package.rglob('*'):
            if p.is_file() and p.suffix.lower() in ('.glb','.ogg','.wav','.go','.gc','.iso','.jbp'):
                failures.append('retail content in package: '+str(p.relative_to(args.package)))
            if p.is_file() and p.suffix.lower()=='.jar':
                if p.name!='jakmc-code.jar':failures.append('unexpected jar: '+p.name)
                else:
                    with zipfile.ZipFile(p) as z:
                        if private&set(z.namelist()):failures.append('private resources in packaged mod')
    result=dict(source_files=len(set(files)-{''}),code_jar_bytes=code.stat().st_size,
                private_jar_intersections=len(overlap),failures=failures)
    print(json.dumps(result,indent=2))
    if failures:raise SystemExit(1)

if __name__=='__main__':main()
