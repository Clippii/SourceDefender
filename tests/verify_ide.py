"""Install the real VSIX in an isolated VS Code profile and run editor-host tests."""
import argparse
import hashlib
import json
import os
import pathlib
import shutil
import subprocess
import sys
import tarfile
import tempfile
import urllib.request

ROOT = pathlib.Path(__file__).resolve().parent.parent


def vscode_binary(override):
    if override:
        return pathlib.Path(override).resolve()
    if os.name == 'nt':
        found = pathlib.Path(os.environ['LOCALAPPDATA']) / 'Programs/Microsoft VS Code/Code.exe'
        if found.is_file():
            return found
    if sys.platform != 'linux':
        raise RuntimeError('Pass --code with the native VS Code executable path.')
    cache = ROOT / '.build/vscode-test'
    binary = cache / 'VSCode-linux-x64/code'
    if binary.is_file():
        return binary
    cache.mkdir(parents=True, exist_ok=True)
    with urllib.request.urlopen('https://update.code.visualstudio.com/api/update/linux-x64/stable/latest') as reply:
        release = json.load(reply)
    archive = cache / 'vscode.tar.gz'
    with urllib.request.urlopen(release['url']) as response, archive.open('wb') as stream:
        shutil.copyfileobj(response, stream)
    expected = release.get('sha256hash')
    if expected:
        assert hashlib.sha256(archive.read_bytes()).hexdigest() == expected, 'VS Code checksum mismatch'
    with tarfile.open(archive) as bundle:
        bundle.extractall(cache, filter='data')
    return binary


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--vsix', required=True)
    parser.add_argument('--code')
    args = parser.parse_args()
    code = vscode_binary(args.code)
    vsix = pathlib.Path(args.vsix).resolve()
    with tempfile.TemporaryDirectory(prefix='sourcedefender-ide-') as temporary:
        temp = pathlib.Path(temporary)
        profile, extensions, projects = temp / 'profile', temp / 'extensions', temp / 'projects'
        projects.mkdir()
        env = dict(os.environ, SOURCEDEFENDER_DATA_DIR=str(temp / 'records'),
                   SOURCEDEFENDER_IDE_TEST_ROOT=str(projects))
        cli = code.parent / 'resources/app/out/cli.js'
        if not cli.is_file():
            # Recent Windows installers keep application resources in a versioned directory.
            candidates = list(code.parent.glob('*/resources/app/out/cli.js'))
            if not candidates:
                raise RuntimeError('Cannot locate the VS Code CLI beside the executable.')
            cli = max(candidates, key=lambda item: item.stat().st_mtime)
        install_env = dict(env, ELECTRON_RUN_AS_NODE='1')
        install = subprocess.run([str(code), str(cli), '--user-data-dir', str(profile), '--extensions-dir', str(extensions),
                        '--install-extension', str(vsix), '--force'], env=install_env, capture_output=True,
                        text=True,encoding='utf-8',errors='replace',timeout=120)
        print(install.stdout)
        if install.returncode:
            raise RuntimeError('VSIX installation failed: '+install.stderr)
        candidates = []
        for manifest in extensions.glob('*/package.json'):
            package = json.loads(manifest.read_text(encoding='utf-8'))
            if package.get('name') == 'source-defender' and package.get('publisher', '').lower() == 'clippii':
                candidates.append(manifest.parent)
        assert len(candidates) == 1, f'Installed extension missing: {candidates}'
        settings = profile / 'User/settings.json'
        settings.parent.mkdir(parents=True, exist_ok=True)
        settings.write_text(json.dumps({'workbench.startupEditor': 'none', 'update.mode': 'none',
                                       'telemetry.telemetryLevel': 'off'}), encoding='utf-8')
        # A fresh profile and workspace isolate test-only trust/extension settings from the user.
        arguments = [str(code), '--user-data-dir', str(profile), '--extensions-dir', str(extensions),
                     '--extensionDevelopmentPath=' + str(candidates[0]),
                     '--extensionTestsPath=' + str(ROOT / 'ide-extension/integration/run.js'),
                     '--disable-extensions', '--disable-workspace-trust', '--skip-welcome',
                     '--skip-release-notes', '--disable-gpu', str(projects)]
        if sys.platform == 'linux' and os.environ.get('CI'):
            arguments.insert(1, '--no-sandbox')
        env.pop('ELECTRON_RUN_AS_NODE', None)
        result = subprocess.run(arguments, env=env, capture_output=True, text=True, encoding='utf-8',
                                errors='replace', timeout=180)
        print('\n'.join(line for line in result.stdout.splitlines() if 'PASS:' in line))
        if result.returncode:
            print(result.stdout[-12000:], file=sys.stderr)
            print(result.stderr[-12000:], file=sys.stderr)
            raise RuntimeError(f'VS Code editor-host tests failed: {result.returncode}')
        report = json.loads((projects / 'ide-results.json').read_text(encoding='utf-8'))
        assert report['ok'] and report['run']['Status'] == 'Contained'
        # Git object files are read-only on Windows.
        for item in projects.rglob('*'):
            if item.is_file():
                item.chmod(0o666)
        print('PASS: actual VSIX install and editor-host gameplay on', sys.platform)


if __name__ == '__main__':
    main()
