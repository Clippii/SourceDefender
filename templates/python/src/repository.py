"""In-memory storage with optimistic concurrency and isolated snapshots."""
from copy import deepcopy
from threading import RLock
from .models import Record


class ConflictError(Exception):
    pass


class Repository:
    def __init__(self, records=()):
        self._lock = RLock()
        self._records = {r.id: r for r in records}
        self._idempotency = {}

    def get(self, record_id):
        with self._lock:
            if record_id not in self._records:
                raise KeyError(record_id)
            return deepcopy(self._records[record_id])

    def insert(self, record, request_key=None):
        with self._lock:
            if request_key and request_key in self._idempotency:
                return self.get(self._idempotency[request_key])
            if record.id in self._records:
                raise ConflictError('Record already exists')
            self._records[record.id] = deepcopy(record)
            if request_key:
                self._idempotency[request_key] = record.id
            return self.get(record.id)

    def save(self, record, expected_version):
        with self._lock:
            previous = self.get(record.id)
            if previous.version != expected_version:
                raise ConflictError('Record changed since it was read')
            if record.version != expected_version + 1:
                raise ConflictError('Version must advance exactly once')
            self._records[record.id] = deepcopy(record)
            return self.get(record.id)

    def list(self, owner=None, status=None, kind=None, offset=0, limit=20):
        if not 1 <= limit <= 100 or offset < 0:
            raise ValueError('Invalid page bounds')
        with self._lock:
            rows = sorted(self._records.values(), key=lambda r: r.id)
            rows = [r for r in rows if (owner is None or r.owner == owner)
                    and (status is None or r.status == status)
                    and (kind is None or r.kind == kind)]
            return {'items': deepcopy(rows[offset:offset + limit]),
                    'total': len(rows), 'offset': offset, 'limit': limit}

    def snapshot(self):
        with self._lock:
            return deepcopy(list(self._records.values()))
