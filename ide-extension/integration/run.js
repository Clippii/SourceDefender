'use strict';
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const { spawnSync } = require('node:child_process');
const vscode = require('vscode');

async function run() {
  const extension = vscode.extensions.getExtension('Clippii.source-defender');
  assert.ok(extension, 'Installed VSIX was not discovered');
  const api = await extension.activate();
  assert.ok(api && api.startWithSettings && api.validate, 'Game API did not activate');
  const commands = await vscode.commands.getCommands(true);
  for (const name of ['startIncident', 'validateChanges', 'requestHint', 'showHistory', 'openBriefing']) {
    assert.ok(commands.includes(`sourceDefender.${name}`), `Missing command ${name}`);
  }
  const root = process.env.SOURCEDEFENDER_IDE_TEST_ROOT;
  assert.ok(root, 'Isolated test workspace not configured');
  const started = await api.startWithSettings({ scenario: 4, language: 'JavaScript', level: 0,
    seed: 'ide-integration', folder: root }, { open: false });
  assert.equal(started.Run.Status, 'Active');
  assert.equal(started.Run.Mode, 'human');
  const folder = started.Run.Folder;
  const baseline = spawnSync('git', ['rev-list', '--max-parents=0', 'HEAD'], { cwd: folder, encoding: 'utf8' });
  assert.equal(baseline.status, 0, baseline.stderr);

  const failed = await api.validate();
  assert.equal(failed.Run.Attempts, 1);
  assert.equal(failed.Run.PenaltySeconds, 15);
  assert.equal(failed.Run.Findings.length, 3);
  for (const finding of failed.Run.Findings) {
    assert.ok(finding.Path, 'Intern findings should identify files');
    const document = await api.openFile(finding.Path);
    assert.equal(document.uri.fsPath, vscode.Uri.file(path.join(folder, finding.Path)).fsPath);
  }

  const hinted = await api.act('hint');
  assert.equal(hinted.Run.Hints, 1);
  assert.equal(hinted.Run.PenaltySeconds, 35);
  assert.ok(hinted.Run.Findings[0].Path);

  const restore = spawnSync('git', ['restore', '--source', baseline.stdout.trim(), '--', '.'], { cwd: folder, encoding: 'utf8' });
  assert.equal(restore.status, 0, restore.stderr);
  // Open a new game file and an unrelated file. Validation should save only the game edit.
  const gameFile = path.join(folder, 'notes.txt');
  const otherFile = path.join(root, 'unrelated.txt');
  fs.writeFileSync(gameFile, 'before'); fs.writeFileSync(otherFile, 'before');
  const gameDoc = await vscode.workspace.openTextDocument(gameFile);
  const otherDoc = await vscode.workspace.openTextDocument(otherFile);
  const edits = new vscode.WorkspaceEdit();
  edits.insert(gameDoc.uri, new vscode.Position(0, 0), 'game edit\n');
  edits.insert(otherDoc.uri, new vscode.Position(0, 0), 'unrelated edit\n');
  assert.equal(await vscode.workspace.applyEdit(edits), true);
  assert.equal(gameDoc.isDirty, true); assert.equal(otherDoc.isDirty, true);
  const contained = await api.validate();
  assert.equal(contained.Run.Status, 'Contained');
  assert.equal(contained.Run.Findings.length, 0);
  assert.equal(gameDoc.isDirty, false, 'Game edits were not saved');
  assert.equal(otherDoc.isDirty, true, 'Unrelated edits were saved unexpectedly');
  assert.equal(fs.readFileSync(otherFile, 'utf8'), 'before');
  assert.ok(fs.readFileSync(gameFile, 'utf8').startsWith('game edit'));
  await api.refresh(true);
  assert.equal(api.run.Status, 'Contained');
  assert.ok(api.incidentRows().some(item => item.id === 'score'));

  // Agent view is observational. It must never spend a human's time or alter their run.
  await vscode.commands.executeCommand('sourceDefender.showAgentPractice');
  assert.equal(api.agent, true);
  assert.throws(() => api.humanActive(), /active human/);
  await vscode.commands.executeCommand('sourceDefender.showAgentPractice');
  // The assertion above verified that validation leaves this test document alone.
  // Explicitly save our own temporary document so editor-host shutdown has no prompt.
  await otherDoc.save();
  console.log('PASS: installed IDE extension activation, generation, clickable findings, hints, save scope, containment, and agent guard');
  fs.writeFileSync(path.join(root, 'ide-results.json'), JSON.stringify({ ok: true, run: contained.Run, extension: extension.id }, null, 2));
}

module.exports = { run };
