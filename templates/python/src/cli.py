"""Local demonstration. Performs no network requests or subprocess execution."""
import argparse
import json
from dataclasses import asdict, is_dataclass
from .bootstrap import build_application, example_session
from .workflows import reconcile


def serialize(value):
    if is_dataclass(value):
        return asdict(value)
    raise TypeError(type(value).__name__)


def main():
    parser = argparse.ArgumentParser(description='@@SCENARIO@@ local service')
    parser.add_argument('command', choices=['health', 'list', 'reconcile', 'export'])
    args = parser.parse_args()
    app = build_application()
    if args.command == 'health':
        result = app.health()
    elif args.command == 'reconcile':
        result = reconcile(app.repository)
    elif args.command == 'export':
        result = app.export(example_session()).data
    else:
        result = app.list(example_session()).data
    print(json.dumps(result, default=serialize, indent=2))


if __name__ == '__main__':
    main()
