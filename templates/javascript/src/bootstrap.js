'use strict';
const fs = require('node:fs');
const path = require('node:path');
const { Record } = require('./models');
const { Repository } = require('./repository');
const { Application } = require('./application');
const { Router } = require('./router');
const ROOT = path.resolve(__dirname, '..');

function loadJson(relative) {
  return JSON.parse(fs.readFileSync(path.join(ROOT, relative), 'utf8'));
}

function buildApplication() {
  const records = loadJson('fixtures/records.json').map(row => new Record(row));
  return new Application(new Repository(records), loadJson('config/scenario.json'));
}

function exampleSession() {
  return { id: 'user-1', verified: true, roles: ['operator', 'reviewer'], token: 'fixture-session' };
}

module.exports = { buildApplication, buildRouter: () => new Router(buildApplication()), exampleSession, loadJson };
