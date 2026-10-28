const test = require('node:test');
const assert = require('node:assert/strict');
const { Record } = require('../src/models');
const { Repository } = require('../src/repository');

const sample = (id = 'rec-001', owner = 'user-1') => new Record({ id, owner,
  kind: 'release', title: 'Example record', amount: 1250 });

test('insert and retrieve a record', () => {
  const repo = new Repository();
  repo.insert(sample());
  assert.equal(repo.get('rec-001').amount, 1250);
});

test('duplicates fail but request keys are idempotent', () => {
  const repo = new Repository();
  repo.insert(sample(), 'request-1');
  assert.equal(repo.insert(sample('rec-002'), 'request-1').id, 'rec-001');
  assert.throws(() => repo.insert(sample()), /already exists/);
});

test('stale version cannot overwrite a newer record', () => {
  const repo = new Repository([sample()]);
  const changed = repo.get('rec-001').transition('active');
  repo.save(changed, 1);
  assert.throws(() => repo.save(changed, 1), /concurrency/);
});

test('reads and snapshots do not expose stored mutable tags', () => {
  const repo = new Repository([sample()]);
  repo.get('rec-001').tags.push('changed');
  assert.deepEqual(repo.get('rec-001').tags, []);
});

test('filtering, sorting and pagination', () => {
  const repo = new Repository([sample(), sample('rec-002', 'user-2')]);
  assert.equal(repo.list({ owner: 'user-1' }).total, 1);
  assert.equal(repo.list({ offset: 1, limit: 1 }).items[0].id, 'rec-002');
  assert.throws(() => repo.list({ limit: 0 }), /bounds/);
});

test('state transition rules reject skipped review', () => {
  assert.throws(() => sample().transition('archived'), /transition/);
});
