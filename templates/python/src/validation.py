"""Input checks shared by the command router and import workers."""
import re

ID = re.compile(r'^[a-z][a-z0-9-]{1,63}$')


def require_id(value, field='id'):
    if not isinstance(value, str) or not ID.fullmatch(value):
        raise ValueError(f'{field} must be a lowercase identifier')
    return value


def clean_title(value):
    if not isinstance(value, str):
        raise ValueError('title must be text')
    value = ' '.join(value.split())
    if not 3 <= len(value) <= 160:
        raise ValueError('title must contain 3 to 160 characters')
    return value


def money(value):
    if isinstance(value, bool) or not isinstance(value, int):
        raise ValueError('amount must be an integer number of cents')
    if not 0 <= value <= 100_000_000:
        raise ValueError('amount is out of range')
    return value


def tags(values):
    if not isinstance(values, (list, tuple)) or len(values) > 10:
        raise ValueError('at most ten tags are allowed')
    return tuple(sorted(set(require_id(v, 'tag') for v in values)))


def request_payload(payload, kinds):
    unknown = set(payload) - {'id', 'kind', 'owner', 'title', 'amount', 'tags'}
    if unknown:
        raise ValueError(f'Unknown fields: {sorted(unknown)}')
    if payload.get('kind') not in kinds:
        raise ValueError('Unsupported record kind')
    return {'id': require_id(payload['id']), 'kind': payload['kind'],
            'owner': require_id(payload['owner'], 'owner'),
            'title': clean_title(payload['title']),
            'amount': money(payload.get('amount', 0)),
            'tags': tags(payload.get('tags', []))}
