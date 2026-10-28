import unittest
from src.models import Record
from src.repository import Repository
from src.application import Application
from src.router import Router
from src.validation import clean_title, money, tags
from src.workflows import reconcile, review_batch


class ApplicationTests(unittest.TestCase):
    def setUp(self):
        self.settings = {'record_kinds': ['release'], 'max_export_rows': 1000}
        self.app = Application(Repository(), self.settings)
        self.session = {'id': 'user-1', 'verified': True, 'roles': ['operator', 'reviewer']}
        self.payload = {'id': 'rec-001', 'kind': 'release', 'owner': 'user-1',
                        'title': 'Example release', 'amount': 2500}

    def test_create_then_approve(self):
        response = self.app.create(self.session, self.payload)
        self.assertEqual(response.status, 201)
        changed = self.app.transition(self.session, 'rec-001', 'active', 1)
        self.assertEqual(changed.data.version, 2)
        self.assertEqual(len(self.app.events), 2)

    def test_viewer_cannot_create(self):
        with self.assertRaises(PermissionError):
            self.app.create(dict(self.session, roles=['viewer']), self.payload)

    def test_cross_owner_create_is_rejected(self):
        with self.assertRaises(PermissionError):
            self.app.create(self.session, dict(self.payload, owner='user-2'))

    def test_unknown_kind(self):
        with self.assertRaises(ValueError):
            self.app.create(self.session, dict(self.payload, kind='unknown'))

    def test_router_maps_errors(self):
        router = Router(self.app)
        self.assertEqual(router.dispatch('unknown', {}).status, 404)
        self.assertEqual(router.dispatch('records.list', {}).status, 403)

    def test_review_workflow(self):
        self.app.create(self.session, self.payload)
        result = review_batch(self.app, self.session, ['rec-001'])
        self.assertEqual(result[0]['outcome'], 'approved')
        self.assertEqual(reconcile(self.app.repository)['record_count'], 1)

    def test_input_normalization(self):
        self.assertEqual(clean_title(' Example   record '), 'Example record')
        self.assertEqual(tags(['priority', 'priority']), ('priority',))
        with self.assertRaises(ValueError):
            money(-1)
