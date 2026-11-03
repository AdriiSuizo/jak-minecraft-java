"""Drive the isolated dev game through its own main-thread bridge; never sends OS input."""
from pathlib import Path
import argparse,json,time,uuid,os

ROOT=Path(__file__).resolve().parents[1]
def call(op,**data):
    directory=ROOT/'run/jakmc/bridge'
    ready=json.loads((directory/'ready.json').read_text(encoding='utf-8'))
    if os.name=='nt':
        import ctypes
        from ctypes import wintypes
        kernel=ctypes.WinDLL('kernel32',use_last_error=True)
        kernel.OpenProcess.argtypes=[wintypes.DWORD,wintypes.BOOL,wintypes.DWORD];kernel.OpenProcess.restype=wintypes.HANDLE
        kernel.GetExitCodeProcess.argtypes=[wintypes.HANDLE,ctypes.POINTER(wintypes.DWORD)]
        kernel.CloseHandle.argtypes=[wintypes.HANDLE]
        handle=kernel.OpenProcess(0x1000,False,ready['pid'])
        if not handle:raise RuntimeError('The recorded game process has exited; request was not queued')
        code=wintypes.DWORD()
        try:
            if not kernel.GetExitCodeProcess(handle,ctypes.byref(code)) or code.value!=259:raise RuntimeError('Game process has exited')
        finally:kernel.CloseHandle(handle)
    else:
        os.kill(ready['pid'],0)
    request=dict(id='probe_'+uuid.uuid4().hex,session=ready['session'],op=op,**data)
    destination=directory/'request.json'
    if destination.exists():
        raise RuntimeError('Bridge has an unprocessed request; inspect the running game before replacing it')
    temporary=directory/'request.tmp'
    temporary.write_text(json.dumps(request),encoding='utf-8')
    temporary.replace(destination)
    output=directory/(request['id']+'.json')
    start=time.monotonic()
    while not output.exists():
        if time.monotonic()-start>30:raise TimeoutError('Game did not process bridge request')
        time.sleep(.05)
    result=json.loads(output.read_text(encoding='utf-8'))
    if not result.get('ok'):raise RuntimeError(result)
    return result
if __name__=='__main__':
    parser=argparse.ArgumentParser();parser.add_argument('op');parser.add_argument('data',nargs='?',default='{}');args=parser.parse_args()
    print(json.dumps(call(args.op,**json.loads(args.data)),ensure_ascii=True))
