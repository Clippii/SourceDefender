const test = require('node:test');
const assert = require('node:assert/strict');
const { summarize } = require('./fixture-report');

test('fixture report groups records and sums integer amounts', () => {
  const report = summarize([{ id: 'one', kind: 'release', owner: 'user-1', amount: 100 },
    { id: 'two', kind: 'release', owner: 'user-2', amount: 200 }]);
  assert.equal(report.count, 2);
  assert.equal(report.amount, 300);
  assert.equal(report.kinds.release, 2);
});

test('fixture report rejects duplicated identities', () => {
  const row = { id: 'one', kind: 'release', owner: 'user-1', amount: 100 };
  assert.throws(() => summarize([row, row]), /Duplicate/);
});

test('fixture report rejects fractional currency', () => {
  assert.throws(() => summarize([{ id: 'one', amount: 1.5 }]), /Invalid/);
});
