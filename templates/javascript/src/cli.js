'use strict';
const { buildApplication, exampleSession } = require('./bootstrap');
const { reconcile } = require('./workflows');

function main(command) {
  const app = buildApplication();
  switch (command) {
    case 'health': return app.health();
    case 'list': return app.list(exampleSession());
    case 'export': return app.export(exampleSession());
    case 'reconcile': return reconcile(app.repository);
    default: throw new Error('Usage: node src/cli.js health|list|export|reconcile');
  }
}

if (require.main === module) {
  try { console.log(JSON.stringify(main(process.argv[2] || 'health'), null, 2)); }
  catch (error) { console.error(error.message); process.exitCode = 1; }
}

module.exports = { main };
