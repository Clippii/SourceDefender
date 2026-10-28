const test = require('node:test');
const assert = require('node:assert/strict');
const { Repository } = require('../src/repository');
const { Application } = require('../src/application');
const { Router } = require('../src/router');
const { JobQueue } = require('../src/jobs');
const { reconcile, reviewBatch } = require('../src/workflows');

const session = { id: 'user-1', verified: true, roles: ['operator', 'reviewer'] };
const payload = { id: 'rec-001', kind: 'release', owner: 'user-1', title: 'Example release', amount: 2500 };
const createApp = () => new Application(new Repository(), { record_kinds: ['release'], max_export_rows: 1000 });

test('create, review and audit the lifecycle', () => {
  const app = createApp();
  app.create(session, payload);
  const changed = app.transition(session, 'rec-001', 'active', 1);
  assert.equal(changed.version, 2);
  assert.equal(app.events.length, 2);
});

test('viewer and cross-owner creation are forbidden', () => {
  const app = createApp();
  assert.throws(() => app.create({ ...session, roles: ['viewer'] }, payload), /operator/);
  assert.throws(() => app.create(session, { ...payload, owner: 'user-2' }), /Owner/);
});

test('unknown domain and fields are rejected', () => {
  assert.throws(() => createApp().create(session, { ...payload, kind: 'other' }), /kind/);
  assert.throws(() => createApp().create(session, { ...payload, admin: true }), /Unknown/);
});

test('router handles unknown operations and authorization', () => {
  const router = new Router(createApp());
  assert.equal(router.dispatch('unknown', {}).status, 404);
  assert.equal(router.dispatch('records.list', {}).status, 403);
});

test('batch review and reconciliation agree', () => {
  const app = createApp(); app.create(session, payload);
  assert.equal(reviewBatch(app, session, ['rec-001'])[0].outcome, 'approved');
  assert.equal(reconcile(app.repository).recordCount, 1);
});

test('queue deduplicates, retries and dead-letters failed jobs', async () => {
  const queue = new JobQueue(2);
  assert.equal(queue.enqueue('job-1', 'backup', {}), true);
  assert.equal(queue.enqueue('job-1', 'backup', {}), false);
  const handlers = { backup: () => { throw new Error('Fixture worker failure'); } };
  assert.equal((await queue.runOnce(handlers, 1000)).state, 'retry');
  assert.equal(await queue.runOnce(handlers, 1000), null);
  assert.equal((await queue.runOnce(handlers, 20000)).state, 'failed');
  assert.equal(queue.statistics().deadLetters, 1);
});
