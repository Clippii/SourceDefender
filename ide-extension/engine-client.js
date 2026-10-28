'use strict';
const fs = require('node:fs');
const path = require('node:path');
const { spawn } = require('node:child_process');

class EngineClient {
  constructor(enginePath, options = {}) {
    if (!path.isAbsolute(enginePath)) throw new Error('SourceDefender engine path must be absolute.');
    this.enginePath = enginePath;
    this.dataDir = options.dataDir;
    this.timeout = options.timeout || 60000;
    this.children = new Set();
  }

  call(request) {
    if (!fs.existsSync(this.enginePath)) {
      return Promise.reject(new Error('The bundled engine is missing. Install the SourceDefender VSIX for your operating system, or set SourceDefender: Engine Path in user settings.'));
    }
    return new Promise((resolve, reject) => {
      const child = spawn(this.enginePath, [], {
        windowsHide: true, shell: false, stdio: ['pipe', 'pipe', 'pipe'],
        env: this.dataDir ? { ...process.env, SOURCEDEFENDER_DATA_DIR: this.dataDir } : process.env,
      });
      this.children.add(child);
      let stdout = '', stderr = '', settled = false;
      const finish = (error, result) => {
        if (settled) return;
        settled = true; clearTimeout(timer); this.children.delete(child);
        if (error) reject(error); else resolve(result);
      };
      const timer = setTimeout(() => { child.kill(); finish(new Error('The local engine timed out. Refresh to recover the saved run.')); }, this.timeout);
      child.stdout.setEncoding('utf8'); child.stderr.setEncoding('utf8');
      child.stdout.on('data', chunk => {
        stdout += chunk;
        if (stdout.length > 16 * 1024 * 1024) { child.kill(); finish(new Error('The engine response exceeded the size limit.')); }
      });
      child.stderr.on('data', chunk => { stderr = (stderr + chunk).slice(-8192); });
      child.on('error', error => finish(new Error(`Cannot launch the SourceDefender engine: ${error.message}`)));
      child.stdin.on('error', error => finish(new Error(`Cannot send request to engine: ${error.message}`)));
      child.on('close', code => {
        try {
          const reply = JSON.parse(stdout);
          if (!reply.ok) throw new Error(reply.error || 'The game operation failed.');
          if (code !== 0) throw new Error(stderr || `The engine exited with code ${code}.`);
          finish(null, reply.data);
        } catch (error) { finish(new Error(error instanceof SyntaxError ? 'The engine returned an invalid response.' : error.message)); }
      });
      child.stdin.end(JSON.stringify(request), 'utf8');
    });
  }

  dispose() {
    for (const child of this.children) child.kill();
    this.children.clear();
  }
}

function scoredSeconds(run, now = Date.now()) {
  if (!run) return 0;
  const end = run.FinishedUtc ? Date.parse(run.FinishedUtc) : now;
  return Math.max(0, (end - Date.parse(run.StartedUtc)) / 1000) + run.PenaltySeconds;
}

function formatTime(seconds) {
  seconds = Math.max(0, Math.floor(seconds || 0));
  const minutes = Math.floor(seconds / 60), rest = String(seconds % 60).padStart(2, '0');
  return minutes >= 60 ? `${Math.floor(minutes / 60)}:${String(minutes % 60).padStart(2, '0')}:${rest}` : `${String(minutes).padStart(2, '0')}:${rest}`;
}

function inside(root, file) {
  const relative = path.relative(path.resolve(root), path.resolve(file));
  return relative === '' || (!relative.startsWith(`..${path.sep}`) && relative !== '..' && !path.isAbsolute(relative));
}

function safeFile(root, relative) {
  if (!relative || typeof relative !== 'string' || path.isAbsolute(relative) || path.win32.isAbsolute(relative) || relative.includes('\0')) throw new Error('Invalid incident file path.');
  const segments = relative.replace(/\\/g, '/').split('/').filter(segment => segment && segment !== '.');
  if (segments.includes('..') || segments[0]?.toLowerCase() === '.git') throw new Error('File is outside the incident source.');
  const actualRoot = fs.realpathSync(root);
  const candidate = fs.realpathSync(path.join(actualRoot, ...segments));
  if (!inside(actualRoot, candidate)) throw new Error('File points outside the incident repository.');
  if (!fs.statSync(candidate).isFile()) throw new Error('Select an incident source file.');
  return candidate;
}

module.exports = { EngineClient, scoredSeconds, formatTime, inside, safeFile };
