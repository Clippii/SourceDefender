"""Queue mechanics used by maintenance and export workers."""
from collections import deque
from dataclasses import dataclass
from datetime import datetime, timedelta, timezone


@dataclass
class Job:
    id: str
    kind: str
    payload: dict
    attempts: int = 0
    state: str = 'pending'
    retry_at: object = None
    error: str = ''


class JobQueue:
    def __init__(self, max_attempts=3):
        self.pending = deque()
        self.completed = {}
        self.dead_letters = []
        self.known = set()
        self.max_attempts = max_attempts

    def enqueue(self, job):
        if job.id in self.known:
            return False
        self.known.add(job.id)
        self.pending.append(job)
        return True

    def run_once(self, handlers, now=None):
        now = now or datetime.now(timezone.utc)
        if not self.pending:
            return None
        job = self.pending.popleft()
        if job.retry_at and now < job.retry_at:
            self.pending.append(job)
            return None
        job.attempts += 1
        try:
            handlers[job.kind](job.payload)
            job.state = 'completed'
            self.completed[job.id] = job
        except Exception as error:
            job.error = str(error)
            if job.attempts >= self.max_attempts:
                job.state = 'failed'
                self.dead_letters.append(job)
            else:
                job.state = 'retry'
                job.retry_at = now + timedelta(seconds=2 ** job.attempts)
                self.pending.append(job)
        return job

    def statistics(self):
        return {'pending': len(self.pending), 'completed': len(self.completed),
                'dead_letters': len(self.dead_letters)}
