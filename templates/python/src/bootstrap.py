"""Construct the service from repository-local, synthetic fixtures."""
import json
from pathlib import Path
from .models import Record
from .repository import Repository
from .application import Application
from .router import Router

ROOT = Path(__file__).resolve().parent.parent


def load_json(relative):
    with (ROOT / relative).open(encoding='utf-8') as stream:
        return json.load(stream)


def build_application():
    rows = load_json('fixtures/records.json')
    records = [Record(**dict(row, tags=tuple(row.get('tags', [])))) for row in rows]
    return Application(Repository(records), load_json('config/scenario.json'))


def build_router():
    return Router(build_application())


def example_session():
    return {'id': 'user-1', 'verified': True,
            'roles': ['operator', 'reviewer'], 'token': 'fixture-session'}
