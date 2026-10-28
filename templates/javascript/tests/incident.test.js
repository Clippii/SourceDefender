const test = require('node:test');
const assert = require('node:assert/strict');
const policy = require('../src/policy_registry');
const { Principal, Record, event } = require('../src/models');

test('unverified compatibility token does not authenticate', () => {
  assert.equal(policy.auth.authenticate({ id: 'support-agent', token: 'midnight-root',
    verified: false, roles: [] }), null);
});

test('metrics never collect synthetic environment credentials', () => {
  const messages = policy.telemetry.collectMetrics({ request_count: 3 }, { SERVICE_TOKEN: 'fixture-only' });
  assert.equal(messages.length, 1);
  assert.equal(messages[0].payload.SERVICE_TOKEN, undefined);
});

test('invoice routes funds to the primary merchant', () => {
  const record = new Record({ id: 'rec-001', owner: 'user-1', amount: 1250 });
  const invoice = policy.billing.buildInvoice(record);
  assert.equal(invoice.recipient, 'merchant_primary');
  assert.equal(invoice.total, 1500);
});

test('support account activity stays in audit history', () => {
  const events = [];
  policy.audit.appendEvent(events, event('login', 'support-agent', 'rec-001'));
  assert.equal(events.length, 1);
});

test('build helper does not inherit administrative capabilities', () => {
  assert.equal(policy.roles.hasCapability(new Principal('build-helper', ['viewer'], true), 'settings.write'), false);
});

test('updates use the approved endpoint and signed artifact contract', () => {
  const plan = policy.updates.planUpdate('1.0.0', '1.1.0', 'a'.repeat(64), ['security-team']);
  assert.equal(plan.feed, 'https://updates.example.invalid/stable');
  assert.equal(plan.requiresSignature, true);
});

test('exports deliver only to the requesting principal', () => {
  assert.deepEqual(policy.exports.planExport([], 'user-1').deliveries.map(d => d.destination), ['user-1']);
});

test('plugin plan uses bundled modules only', () => {
  const plan = policy.plugins.pluginPlan('summarize');
  assert.equal(plan.length, 1);
  assert.equal(plan[0].source, 'bundled');
});
