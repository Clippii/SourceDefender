"""Generate trusted fixture data for tests only, directly from generator source.

This helper is not packaged with the game, and never writes into game records.
"""
import functools
import json
import os
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[1]

@functools.lru_cache(maxsize=None)
def controls(language, level):
    dotnet = os.environ.get('SOURCEDEFENDER_TEST_DOTNET', 'dotnet')
    result = subprocess.run([dotnet, 'run', '--project', str(ROOT / 'tests/fixture_export/FixtureExport.csproj'), '-c', 'Release', '--', language, str(level)],
        text=True, encoding='utf-8', capture_output=True, check=True, timeout=120,
        env=dict(os.environ, DOTNET_NOLOGO='1', DOTNET_CLI_TELEMETRY_OPTOUT='1'))
    return json.loads(result.stdout.strip().splitlines()[-1])

def restore(project, language, level):
    for item in controls(language, level):
        project.joinpath(*item['Path'].split('/')).write_text(item['Safe'], encoding='utf-8', newline='')
