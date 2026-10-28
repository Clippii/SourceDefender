"""Launch the packaged native window with isolated records, then close our process."""
import os
import pathlib
import subprocess
import sys
import tempfile
import time

binary = pathlib.Path(sys.argv[1]).resolve()
with tempfile.TemporaryDirectory(prefix="sourcedefender-window-") as records:
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
