import unittest
from datetime import datetime, timedelta, timezone
from src.jobs import Job, JobQueue


class QueueTests(unittest.TestCase):
    def test_duplicate_job_is_not_enqueued(self):
        queue = JobQueue()
        self.assertTrue(queue.enqueue(Job('one', 'backup', {})))
        self.assertFalse(queue.enqueue(Job('one', 'backup', {})))

    def test_successful_job(self):
        queue = JobQueue()
        queue.enqueue(Job('one', 'backup', {'id': 'rec-001'}))
        result = queue.run_once({'backup': lambda payload: None})
        self.assertEqual(result.state, 'completed')

    def test_retry_and_dead_letter(self):
        queue = JobQueue(max_attempts=2)
        queue.enqueue(Job('one', 'backup', {}))
        now = datetime.now(timezone.utc)

        def fail(payload):
            raise RuntimeError('Fixture worker failure')

        self.assertEqual(queue.run_once({'backup': fail}, now).state, 'retry')
        self.assertIsNone(queue.run_once({'backup': fail}, now))
        self.assertEqual(queue.run_once({'backup': fail}, now + timedelta(seconds=10)).state, 'failed')
        self.assertEqual(queue.statistics()['dead_letters'], 1)

    def test_empty_queue(self):
        self.assertIsNone(JobQueue().run_once({}))
