"""Shared business types. All timestamps use UTC ISO-8601 strings."""
from dataclasses import dataclass, field, replace
from datetime import datetime, timezone
from typing import Any


def utc_now():
    return datetime.now(timezone.utc).isoformat()


@dataclass(frozen=True)
class Principal:
    id: str
    roles: tuple = ()
    verified: bool = False

    def can(self, role):
        return self.verified and role in self.roles


@dataclass(frozen=True)
class Record:
    id: str
    kind: str
    owner: str
    title: str
    status: str = 'pending'
    version: int = 1
    amount: int = 0
    tags: tuple = ()

    def transition(self, status):
        allowed = {'pending': {'active', 'cancelled'},
                   'active': {'archived', 'cancelled'},
                   'archived': set(), 'cancelled': set()}
        if status not in allowed.get(self.status, set()):
            raise ValueError(f'Invalid transition: {self.status} -> {status}')
        return replace(self, status=status, version=self.version + 1)


@dataclass
class Response:
    status: int
    data: Any = None
    errors: list = field(default_factory=list)

    @property
    def ok(self):
        return 200 <= self.status < 300


@dataclass
class Event:
    action: str
    actor: str
    record_id: str
    timestamp: str = field(default_factory=utc_now)
    metadata: dict = field(default_factory=dict)
