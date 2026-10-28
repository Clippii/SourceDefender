"""Launch the packaged native window with isolated records, then close our process."""
import os
import json
import pathlib
import subprocess
import sys
import tempfile
import time

binary = pathlib.Path(sys.argv[1]).resolve()
with tempfile.TemporaryDirectory(prefix="sourcedefender-window-") as records:
    engine=binary.parent / ("SourceDefender.Engine.exe" if os.name=="nt" else "SourceDefender.Engine")
    result=subprocess.run([str(engine)],input=json.dumps({"command":"snapshot"}),text=True,encoding="utf-8",
                          capture_output=True,env=dict(os.environ,SOURCEDEFENDER_DATA_DIR=records),check=True)
    reply=json.loads(result.stdout)
    assert reply["ok"] and len(reply["data"]["Scenarios"])==6, "Packaged engine did not load"
    print("PASS: packaged local engine and embedded templates are readable")
    with tempfile.TemporaryFile() as log:
        process = subprocess.Popen([str(binary)], env=dict(os.environ, SOURCEDEFENDER_DATA_DIR=records),
                                   stdout=log, stderr=log)
        try:
            time.sleep(8)
            if process.poll() is not None:
                log.seek(0)
                raise RuntimeError(f"Native window exited with {process.returncode}:\n" + log.read().decode(errors="replace"))
            print("PASS: packaged native UI remained running for 8 seconds")
        finally:
            if process.poll() is None:
                process.terminate()
                try:
                    process.wait(timeout=10)
                except subprocess.TimeoutExpired:
                    process.kill()
                    process.wait()
