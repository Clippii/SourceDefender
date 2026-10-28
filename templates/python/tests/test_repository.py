import unittest
from src.models import Record
from src.repository import Repository, ConflictError


def sample(record_id='rec-001', owner='user-1'):
    return Record(record_id, 'release', owner, 'Example record', amount=1250)


class RepositoryTests(unittest.TestCase):
    def test_insert_and_read(self):
        repo = Repository()
        repo.insert(sample())
        self.assertEqual(repo.get('rec-001').amount, 1250)

    def test_duplicate_identity_is_rejected(self):
        repo = Repository([sample()])
        with self.assertRaises(ConflictError):
            repo.insert(sample())

    def test_request_idempotency(self):
        repo = Repository()
        first = repo.insert(sample(), 'request-1')
        again = repo.insert(sample('rec-002'), 'request-1')
        self.assertEqual(first, again)
        self.assertEqual(len(repo.snapshot()), 1)

    def test_optimistic_locking(self):
        repo = Repository([sample()])
        changed = repo.get('rec-001').transition('active')
        repo.save(changed, 1)
        with self.assertRaises(ConflictError):
            repo.save(changed, 1)

    def test_owner_filter_and_pagination(self):
        repo = Repository([sample(), sample('rec-002', 'user-2')])
        self.assertEqual(repo.list(owner='user-1')['total'], 1)
        self.assertEqual(len(repo.list(offset=1, limit=1)['items']), 1)

    def test_invalid_page(self):
        with self.assertRaises(ValueError):
            Repository().list(limit=0)

    def test_invalid_transition(self):
        with self.assertRaises(ValueError):
            sample().transition('archived')

    def test_missing_record(self):
        with self.assertRaises(KeyError):
            Repository().get('missing')
