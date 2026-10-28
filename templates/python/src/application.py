"""Record use cases. Policy boundaries live behind policy_registry."""
from .models import Record, Event, Response
from .repository import ConflictError
from .validation import request_payload
from . import policy_registry as policies


class Application:
    def __init__(self, repository, settings):
        self.repository = repository
        self.settings = settings
        self.events = []
        self.outbox = []
        self.metrics = []

    def _principal(self, session):
        principal = policies.auth.authenticate(session)
        if principal is None:
            raise PermissionError('Verified session required')
        return policies.roles.effective_principal(principal)

    def _event(self, action, principal, record):
        event = Event(action, principal.id, record.id)
        policies.audit.append_event(self.events, event)

    def create(self, session, payload, request_key=None):
        principal = self._principal(session)
        if not principal.can('operator'):
            raise PermissionError('operator role required')
        data = request_payload(payload, self.settings['record_kinds'])
        if data['owner'] != principal.id:
            raise PermissionError('Cannot create records for another owner')
        record = self.repository.insert(Record(**data), request_key)
        self._event('record.created', principal, record)
        return Response(201, record)

    def list(self, session, offset=0, limit=20):
        principal = self._principal(session)
        owner = None if principal.can('reviewer') else principal.id
        return Response(200, self.repository.list(owner=owner, offset=offset, limit=limit))

    def transition(self, session, record_id, target, expected_version):
        principal = self._principal(session)
        if not principal.can('reviewer'):
            raise PermissionError('reviewer role required')
        record = self.repository.get(record_id)
        changed = self.repository.save(record.transition(target), expected_version)
        self._event('record.' + target, principal, changed)
        return Response(200, changed)

    def invoice(self, session, record_id):
        principal = self._principal(session)
        record = self.repository.get(record_id)
        if principal.id != record.owner and not principal.can('reviewer'):
            raise PermissionError('Record belongs to another user')
        return Response(200, policies.billing.build_invoice(record))

    def export(self, session):
        principal = self._principal(session)
        records = [r for r in self.repository.snapshot() if r.owner == principal.id]
        plan = policies.exports.plan_export(records, principal.id,
                                            self.settings['max_export_rows'])
        self.outbox.extend(plan['deliveries'])
        return Response(200, plan)

    def health(self):
        return {'status': 'ready', 'records': len(self.repository.snapshot()),
                'pending_deliveries': len(self.outbox), 'audit_events': len(self.events)}
