'use strict';

const assert = require('node:assert/strict');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const test = require('node:test');
const { scoredSeconds, formatTime, inside, safeFile } = require('../engine-client');

function repository(t) {
  const directory = fs.mkdtempSync(path.join(os.tmpdir(), 'source-defender-helpers-'));
  t.after(() => fs.rmSync(directory, { recursive: true, force: true }));
  const root = path.join(directory, 'incident');
  const outside = path.join(directory, 'outside');
  fs.mkdirSync(path.join(root, 'src', 'nested'), { recursive: true });
  fs.mkdirSync(path.join(root, '.git'));
  fs.mkdirSync(outside);
  const source = path.join(root, 'src', 'nested', 'service.js');
  const external = path.join(outside, 'private.txt');
  fs.writeFileSync(source, 'module.exports = true;\n');
  fs.writeFileSync(path.join(root, '.git', 'config'), '[core]\n');
  fs.writeFileSync(external, 'outside repository\n');
  return { directory, root, outside, source, external };
}

test('active time includes elapsed wall time and penalties', () => {
  const started = Date.parse('2026-10-28T12:00:00.000Z');
  const run = { StartedUtc: new Date(started).toISOString(), PenaltySeconds: 45 };
  assert.equal(scoredSeconds(run, started + 125_500), 170.5);
  assert.equal(scoredSeconds(run, started - 5_000), 45);
  assert.equal(scoredSeconds(null, started), 0);
});

test('finished time remains fixed when the current time changes', () => {
  const started = Date.parse('2026-10-28T12:00:00.000Z');
  const run = {
    StartedUtc: new Date(started).toISOString(),
    FinishedUtc: new Date(started + 90_000).toISOString(),
    PenaltySeconds: 20,
  };
  assert.equal(scoredSeconds(run, started + 1_000_000), 110);
  assert.equal(scoredSeconds(run, started + 86_400_000), 110);
});

test('time formatting floors fractions and crosses minute and hour boundaries', () => {
  for (const [seconds, expected] of [
    [undefined, '00:00'], [-3, '00:00'], [0, '00:00'],
    [59.999, '00:59'], [60, '01:00'], [3599, '59:59'],
    [3600, '1:00:00'], [3661, '1:01:01'], [360001, '100:00:01'],
  ]) assert.equal(formatTime(seconds), expected, String(seconds));
});

test('repository membership uses directory boundaries and normalized paths', t => {
  const { root, source, outside } = repository(t);
  assert.equal(inside(root, root), true);
  assert.equal(inside(root, source), true);
  assert.equal(inside(root, path.join(root, '..notes', 'file.js')), true);
  assert.equal(inside(root, path.join(root, 'src', '..', 'README.md')), true);
  assert.equal(inside(root, `${root}-other${path.sep}file.js`), false);
  assert.equal(inside(root, path.dirname(root)), false);
  assert.equal(inside(root, outside), false);
  assert.equal(inside(root, path.join(root, 'src', '..', '..', 'outside')), false);
  if (process.platform === 'win32') {
    const otherDrive = path.parse(root).root.toLowerCase().startsWith('c:') ? 'D:\\' : 'C:\\';
    assert.equal(inside(root, path.join(otherDrive, 'another-incident')), false);
  }
});

test('safe file navigation opens existing nested files with either separator style', t => {
  const { root, source } = repository(t);
  const expected = fs.realpathSync(source);
  assert.equal(safeFile(root, 'src/nested/service.js'), expected);
  assert.equal(safeFile(root, 'src\\nested\\service.js'), expected);
  assert.throws(() => safeFile(root, 'src/nested/missing.js'));
});

test('safe file navigation rejects invalid, absolute, and traversing paths', t => {
  const { root, external, source } = repository(t);
  for (const relative of [
    undefined, null, 42, '', 'src/\0service.js', source, external,
    '../outside/private.txt', '..\\outside\\private.txt',
    'src/../../outside/private.txt', 'src\\..\\..\\outside\\private.txt',
    'src/../nested/service.js',
  ]) assert.throws(() => safeFile(root, relative), `must reject ${JSON.stringify(relative)}`);
});

test('safe file navigation excludes Git metadata including dotted path prefixes', t => {
  const { root } = repository(t);
  for (const relative of ['.git/config', '.git\\config', './.git/config', '.\\.git\\config']) {
    assert.throws(() => safeFile(root, relative), `must reject ${relative}`);
  }
});

test('safe file navigation excludes Git metadata with Windows case folding', {
  skip: process.platform !== 'win32',
}, t => {
  const { root } = repository(t);
  assert.throws(() => safeFile(root, '.GIT/config'));
});

test('safe file navigation rejects a symlink or junction escaping the repository', t => {
  const { root, outside } = repository(t);
  const link = path.join(root, 'external-link');
  try {
    fs.symlinkSync(outside, link, process.platform === 'win32' ? 'junction' : 'dir');
  } catch (error) {
    if (['EPERM', 'EACCES', 'ENOSYS', 'ENOTSUP'].includes(error.code)) {
      t.skip(`Directory links are unavailable: ${error.code}`);
      return;
    }
    throw error;
  }
  assert.throws(() => safeFile(root, 'external-link/private.txt'), /outside the incident repository/);
});
