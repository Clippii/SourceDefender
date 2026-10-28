"""Exercise generated projects using their real runtimes on every supported OS."""
import json
import os
import pathlib
import shutil
import subprocess
import sys
import tempfile

ROOT = pathlib.Path(__file__).resolve().parent.parent
ENGINE = ROOT / ("SourceDefender.Engine.exe" if os.name == "nt" else "SourceDefender.Engine")


def run(*args, cwd=None):
    subprocess.run([str(a) for a in args], cwd=cwd or ROOT, check=True)


def call(request, store):
    env = dict(os.environ, SOURCEDEFENDER_DATA_DIR=str(store))
    result = subprocess.run([str(ENGINE)], input=json.dumps(request),
                            text=True, encoding="utf-8", capture_output=True, env=env)
    response = json.loads(result.stdout)
    if result.returncode or not response["ok"]:
        raise RuntimeError(response.get("error", result.stderr))
    return response["data"]


def main():
    with tempfile.TemporaryDirectory(prefix="sourcedefender-platform-") as temporary:
        root = pathlib.Path(temporary)
        for index, language in enumerate(["Python", "JavaScript", "Go", "C#"]):
            store = root / f"records-{index}"
            data = call(dict(command="generate", level=3, scenario=index,
                             language=language, seed="platform-review", folder=str(root)), store)
            record = data["Run"]
            folder = pathlib.Path(record["Folder"])
            attacked = call(dict(command="validate", id=record["Id"]), store)
            assert attacked["Run"]["Status"] == "Active"
            assert attacked["Run"]["PenaltySeconds"] == 180
            baseline = subprocess.check_output(
                ["git", "-C", str(folder), "rev-list", "--max-parents=0", "HEAD"], text=True).strip()
            run("git", "restore", "--source", baseline, "--", ".", cwd=folder)
            if language == "Python":
                run(sys.executable, "-m", "unittest", "discover", "-s", "tests", "-v", cwd=folder)
                run(sys.executable, "-m", "src.cli", "reconcile", cwd=folder)
            elif language == "JavaScript":
                run("node", "--test", *sorted((folder / "tests").glob("*.test.js")), cwd=folder)
                run("node", "src/cli.js", "reconcile", cwd=folder)
            elif language == "Go":
                run("go", "test", "./...", cwd=folder)
                run("go", "run", "./cmd/demo", "reconcile", cwd=folder)
            else:
                run("dotnet", "run", "--project", "Service.csproj", "-c", "Release", "--", "--test", cwd=folder)
                run("dotnet", "run", "--project", "Service.csproj", "-c", "Release", "--no-build", "--", "reconcile", cwd=folder)
            run("node", "--test", "tools/fixture-report.test.js", cwd=folder)
            clean = call(dict(command="validate", id=record["Id"]), store)
            assert clean["Run"]["Status"] == "Contained", clean["Report"]
            print(f"PASS: {language} project, regression tests, demonstration, and containment", flush=True)
        # Windows Git marks object files read-only.
        for path in root.rglob("*"):
            if path.is_file():
                path.chmod(0o666)
    print("PASS: generated projects on", sys.platform)


if __name__ == "__main__":
    main()
