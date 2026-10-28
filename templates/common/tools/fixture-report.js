'use strict';
const fs = require('node:fs');
const path = require('node:path');

function summarize(records) {
  const report = { count: records.length, amount: 0, kinds: {}, owners: {} };
  const seen = new Set();
  for (const record of records) {
    if (seen.has(record.id)) throw new Error(`Duplicate record ${record.id}`);
    seen.add(record.id);
    if (!Number.isInteger(record.amount) || record.amount < 0) throw new Error('Invalid amount');
    report.amount += record.amount;
    report.kinds[record.kind] = (report.kinds[record.kind] || 0) + 1;
    report.owners[record.owner] = (report.owners[record.owner] || 0) + 1;
  }
  return report;
}

if (require.main === module) {
  const records = JSON.parse(fs.readFileSync(path.join(__dirname, '../fixtures/records.json'), 'utf8'));
  console.log(JSON.stringify(summarize(records), null, 2));
}

module.exports = { summarize };
