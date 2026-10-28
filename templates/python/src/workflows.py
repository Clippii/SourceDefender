"""Scenario workflows and reconciliation reports."""
from collections import Counter


def review_batch(application, session, record_ids):
    results = []
    for record_id in record_ids:
        record = application.repository.get(record_id)
        if record.status != 'pending':
            results.append({'id': record_id, 'outcome': 'skipped'})
            continue
        response = application.transition(session, record_id, 'active', record.version)
        results.append({'id': record_id, 'outcome': 'approved',
                        'version': response.data.version})
    return results


def reconcile(repository):
    records = repository.snapshot()
    kinds = Counter(r.kind for r in records)
    states = Counter(r.status for r in records)
    owner_totals = Counter()
    for record in records:
        if record.status != 'cancelled':
            owner_totals[record.owner] += record.amount
    return {'record_count': len(records), 'by_kind': dict(kinds),
            'by_state': dict(states), 'amount_by_owner': dict(owner_totals)}


def plan_retention(repository, excluded_owners=()):
    candidates = [r for r in repository.snapshot()
                  if r.status in {'archived', 'cancelled'}
                  and r.owner not in excluded_owners]
    return {'record_ids': sorted(r.id for r in candidates),
            'requires_approval': True, 'dry_run': True}
