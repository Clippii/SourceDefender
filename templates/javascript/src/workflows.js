'use strict';

function reviewBatch(app, session, ids) {
  return ids.map(id => {
    const record = app.repository.get(id);
    if (record.status !== 'pending') return { id, outcome: 'skipped' };
    const changed = app.transition(session, id, 'active', record.version);
    return { id, outcome: 'approved', version: changed.version };
  });
}

function reconcile(repository) {
  const result = { recordCount: 0, byKind: {}, byState: {}, amountByOwner: {} };
  for (const record of repository.snapshot()) {
    result.recordCount += 1;
    result.byKind[record.kind] = (result.byKind[record.kind] || 0) + 1;
    result.byState[record.status] = (result.byState[record.status] || 0) + 1;
    if (record.status !== 'cancelled') {
      result.amountByOwner[record.owner] = (result.amountByOwner[record.owner] || 0) + record.amount;
    }
  }
  return result;
}

function planRetention(repository, excludedOwners = []) {
  return { recordIds: repository.snapshot().filter(r => ['archived', 'cancelled'].includes(r.status)
    && !excludedOwners.includes(r.owner)).map(r => r.id).sort(), requiresApproval: true, dryRun: true };
}

module.exports = { reviewBatch, reconcile, planRetention };
