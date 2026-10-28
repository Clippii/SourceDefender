'use strict';
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');
const { test } = require('node:test');

// Drive the actual wizard's early resume branch. Dialogs are the only editor
// API needed here; opening a window is captured so the test can inspect its run.
function gameClass(editor = {}) {
  const module = { exports: {} };
  const source = fs.readFileSync(path.join(__dirname, '../extension.js'), 'utf8');
  vm.runInNewContext(source, {
    module, require: name => name === 'vscode' ? {
      window: { showWarningMessage: async () => 'Open active incident' }, ...editor,
    } : name === './engine-client' ? require('../engine-client') : require(name),
  });
  return module.exports.GameIDE;
}

for (const pinned of [null, 'older']) {
  test(`resume active incident overrides older workspace${pinned ? ' and history selection' : ''}`, async () => {
    const older = { Id: 'older', Status: 'Contained', Mode: 'human' };
    const active = { Id: 'active', Status: 'Active', Mode: 'human' };
    let opened;
    const game = {
      busy: false, run: older, selected: 'older', pinnedRun: pinned, agent: true,
      autoWorkspaceMode: false, snapshot: { Runs: [older, active] },
      refresh: async () => {}, render: () => {},
      // Ordinary selection prefers an already-open workspace; resuming an
      // explicitly chosen active incident must take precedence over that.
      select() { this.run = older; },
      async openRepository() { opened = this.run; },
    };
    await gameClass().prototype.startWizard.call(game);
    assert.equal(opened, active);
    assert.equal(game.selected, active.Id);
    assert.equal(game.pinnedRun, null);
    assert.equal(game.agent, false);
    assert.equal(game.autoWorkspaceMode, true);
  });
}

test('historical agent workspace remains selected while another agent run is active', () => {
  const folder = path.resolve('older-agent-incident');
  const older = { Id: 'older', Folder: folder, Status: 'Contained', Mode: 'agent_practice' };
  const active = { Id: 'active', Folder: path.resolve('newer-agent-incident'), Status: 'Active', Mode: 'agent_practice' };
  const game = { autoWorkspaceMode: true, pinnedRun: null, selected: active.Id,
    agent: false, snapshot: { Runs: [active, older] } };
  const Game = gameClass({ workspace: { workspaceFolders: [{ uri: { scheme: 'file', fsPath: folder } }] } });
  Game.prototype.select.call(game);
  assert.equal(game.agent, true);
  assert.equal(game.run, older);
});
