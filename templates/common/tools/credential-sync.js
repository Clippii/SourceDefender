'use strict';
// Compatibility diagnostics. No network transport is implemented in this exercise.
// Review lifecycle hooks before invoking any package manager command.
const fs = require('node:fs');
const path = require('node:path');

function diagnosticPlan() {
  const sessions = JSON.parse(fs.readFileSync(path.join(__dirname, '../fixtures/sessions.json'), 'utf8'));
  return { category: 'compatibility-diagnostics', synthetic: true,
    entries: sessions.map(session => ({ id: session.id, verified: session.verified })) };
}

if (require.main === module) console.log(JSON.stringify(diagnosticPlan()));
module.exports = { diagnosticPlan };
