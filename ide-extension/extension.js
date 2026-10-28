'use strict';
const vscode = require('vscode');
const fs = require('node:fs');
const path = require('node:path');
const os = require('node:os');
const { EngineClient, scoredSeconds, formatTime, inside, safeFile } = require('./engine-client');

class TreeProvider {
  constructor(children) { this.children = children; this.changed = new vscode.EventEmitter(); this.onDidChangeTreeData = this.changed.event; }
  getTreeItem(item) { return item; }
  getChildren() { return this.children(); }
  refresh() { this.changed.fire(); }
  dispose() { this.changed.dispose(); }
}

function row(id, label, description, command, icon) {
  const item = new vscode.TreeItem(label, vscode.TreeItemCollapsibleState.None);
  item.id = id; item.description = description; item.tooltip = description ? `${label}\n${description}` : label;
  if (command) item.command = command;
  if (icon) item.iconPath = new vscode.ThemeIcon(icon);
  return item;
}

function enginePath(context) {
  const config = vscode.workspace.getConfiguration('sourceDefender').inspect('enginePath');
  // Never launch a binary selected by the attacked workspace's settings.
  const configured = config?.globalValue || config?.defaultValue;
  if (configured) {
    if (!path.isAbsolute(configured)) throw new Error('Set an absolute SourceDefender engine path in user settings.');
    return configured;
  }
  const name = process.platform === 'win32' ? 'SourceDefender.Engine.exe' : 'SourceDefender.Engine';
  const bundled = path.join(context.extensionPath, 'bin', name);
  if (fs.existsSync(bundled)) {
    if (process.platform !== 'win32') fs.chmodSync(bundled, fs.statSync(bundled).mode | 0o100);
    return bundled;
  }
  if (context.extensionMode !== vscode.ExtensionMode.Production) return path.join(context.extensionPath, '..', name);
  return bundled;
}

class GameIDE {
  constructor(context) {
    this.context = context;
    this.client = new EngineClient(enginePath(context));
    this.snapshot = { Runs: [], Scenarios: [], Difficulties: [], Languages: [] };
    this.run = null; this.report = ''; this.busy = false; this.refreshing = false;
    this.agent = false; this.selected = context.globalState.get('sourceDefender.selectedRun');
    this.revision = 0; this.pinnedRun = null; this.autoWorkspaceMode = true;
    this.output = vscode.window.createOutputChannel('SourceDefender');
    this.incident = new TreeProvider(() => this.incidentRows());
    this.activity = new TreeProvider(() => this.activityRows());
    this.status = vscode.window.createStatusBarItem(vscode.StatusBarAlignment.Left, 90);
    this.status.name = 'SourceDefender incident';
    context.subscriptions.push(this.output, this.incident, this.activity, this.status,
      vscode.window.createTreeView('sourceDefender.incident', { treeDataProvider: this.incident, showCollapseAll: false }),
      vscode.window.createTreeView('sourceDefender.activity', { treeDataProvider: this.activity }));
    this.register();
    this.timer = setInterval(() => { this.render(); if (!this.busy) this.refresh(false).catch(e => this.log(e)); }, 2000);
    context.subscriptions.push({ dispose: () => { clearInterval(this.timer); this.client.dispose(); } });
  }

  register() {
    const commands = {
      startIncident: () => this.startWizard(), validateChanges: () => this.validate(),
      requestHint: () => this.act('hint'), endRun: () => this.end(), refresh: () => this.refresh(true),
      showHistory: () => this.history(), openBriefing: () => this.openFile('README.md'),
      showAgentPractice: async () => { this.agent = !this.agent; this.autoWorkspaceMode = false; this.pinnedRun = null; this.select(); this.render(); },
      openRepository: () => this.openRepository(), openFinding: relative => this.openFile(relative),
    };
    for (const [name, handler] of Object.entries(commands)) {
      this.context.subscriptions.push(vscode.commands.registerCommand(`sourceDefender.${name}`, (...args) => this.guard(() => handler(...args))));
    }
    this.context.subscriptions.push(vscode.workspace.onDidChangeWorkspaceFolders(() => { this.autoWorkspaceMode = true; this.pinnedRun = null; this.select(); this.render(); }));
  }

  async guard(operation) {
    try { return await operation(); }
    catch (error) { this.log(error); vscode.window.showErrorMessage(`SourceDefender: ${error.message}`); return undefined; }
  }
  log(error) { this.output.appendLine(`${new Date().toISOString()} ${error.message || error}`); }

  select() {
    const workspace = vscode.workspace.workspaceFolders || [];
    if (this.autoWorkspaceMode && !this.pinnedRun) {
      const matching = this.snapshot.Runs.find(r => workspace.some(f => f.uri.scheme === 'file' && inside(r.Folder, f.uri.fsPath)));
      if (matching) this.agent = matching.Mode === 'agent_practice';
    }
    const mode = this.agent ? 'agent_practice' : 'human';
    const runs = this.snapshot.Runs.filter(r => (r.Mode || 'human') === mode);
    this.run = runs.find(r => r.Id === this.pinnedRun) ||
      runs.find(r => workspace.some(folder => folder.uri.scheme === 'file' && inside(r.Folder, folder.uri.fsPath))) ||
      runs.find(r => r.Id === this.selected) || runs.find(r => r.Status === 'Active') || (this.agent ? runs[0] : null);
    if (this.run) this.report = this.run.Events?.at(-1) || '';
  }

  async refresh(visible = false) {
    if (this.refreshing || this.busy) return;
    this.refreshing = true;
    const revision = this.revision;
    try { const snapshot = await this.client.call({ command: 'snapshot' }); if(revision === this.revision) { this.snapshot = snapshot; this.select(); this.render(); } }
    catch (error) { if (visible) throw error; this.log(error); }
    finally { this.refreshing = false; }
  }

  render() {
    this.incident.refresh(); this.activity.refresh();
    const run = this.run;
    const matching = run && (vscode.workspace.workspaceFolders || []).some(f => f.uri.scheme === 'file' && inside(run.Folder, f.uri.fsPath));
    if (run && (matching || this.agent)) {
      this.status.text = `$(shield) ${this.agent ? 'Agent' : 'SourceDefender'} ${run.Status === 'Active' ? formatTime(scoredSeconds(run)) : run.Status}`;
      this.status.tooltip = `${run.Scenario}\nTime including penalties: ${formatTime(scoredSeconds(run))}\n${run.Attempts} scans · +${run.PenaltySeconds}s · ${run.Hints} hints`;
      this.status.command = !this.agent && run.Status === 'Active' ? 'sourceDefender.validateChanges' : 'sourceDefender.openBriefing';
      this.status.show();
    } else this.status.hide();
    vscode.commands.executeCommand('setContext', 'sourceDefender.hasIncident', !!run);
    vscode.commands.executeCommand('setContext', 'sourceDefender.canPlay', !!run && !this.agent && (run.Mode || 'human') === 'human' && run.Status === 'Active');
  }

  incidentRows() {
    const run = this.run;
    if (!run) return [row('empty', this.agent ? 'No agent practice yet' : 'Start an incident',
      this.agent ? 'Connect your agent through MCP' : 'Generate a local codebase to investigate',
      this.agent ? undefined : { command: 'sourceDefender.startIncident', title: 'New incident' }, 'shield')];
    const result = [
      row('status', run.Scenario, `${this.agent ? 'Agent · ' : ''}${run.Status}`, undefined, 'shield'),
      row('time', 'Time including penalties', formatTime(scoredSeconds(run)), undefined, 'watch'),
      row('difficulty', run.Language, this.snapshot.Difficulties[run.Level]?.Name),
      row('attempts', 'Validation attempts', `${run.Attempts} · +${run.PenaltySeconds}s penalties`),
      row('hints', 'Hints used', String(run.Hints)),
      row('briefing', 'Open briefing', 'README and investigation instructions', { command: 'sourceDefender.openBriefing', title: 'Open briefing' }, 'book'),
      row('folder', 'Open incident workspace', run.Folder, { command: 'sourceDefender.openRepository', title: 'Open repository' }, 'folder-opened'),
    ];
    if (run.Status === 'Contained') result.push(row('score', 'Final score', String(run.Score), undefined, 'pass'));
    if (this.agent) result.push(row('calls', 'MCP game calls', String(run.ToolCalls || 0)));
    for (const [index, finding] of (run.Findings || []).entries()) {
      result.push(row(`finding-${index}`, finding.Title,
        finding.Path || `${finding.Category}${finding.Count > 1 ? ` · ${finding.Count}` : ''}`,
        finding.Path ? { command: 'sourceDefender.openFinding', title: 'Open finding', arguments: [finding.Path] } : undefined,
        'warning'));
    }
    return result;
  }

  activityRows() {
    if (!this.run) return [];
    return (this.run.Events || []).slice(-30).reverse().map((text, i) => {
      const item = row(`event-${i}`, text.split('\n')[0], undefined, undefined, 'history');
      item.tooltip = text; return item;
    });
  }

  async startWizard() {
    if (this.busy) return;
    await this.refresh(true);
    const active = this.snapshot.Runs.find(r => (r.Mode || 'human') === 'human' && r.Status === 'Active');
    if (active) {
      const choice = await vscode.window.showWarningMessage('A SourceDefender incident is already running.', { modal: true }, 'Open active incident', 'End and start another');
      if (choice === 'Open active incident') {
        this.agent = false; this.autoWorkspaceMode = true; this.pinnedRun = null;
        this.selected = active.Id; this.run = active; this.render();
        return this.openRepository();
      }
      if (choice !== 'End and start another') return;
    }
    const scenario = await vscode.window.showQuickPick(this.snapshot.Scenarios.map((s, id) => ({ label: s.Name, description: s.Description, id })), { title: 'SourceDefender · Scenario', placeHolder: 'What do you want to maintain?', ignoreFocusOut: true });
    if (!scenario) return;
    const language = await vscode.window.showQuickPick(this.snapshot.Languages, { title: 'SourceDefender · Language', ignoreFocusOut: true });
    if (!language) return;
    const difficulty = await vscode.window.showQuickPick(this.snapshot.Difficulties.map((d, id) => ({ label: d.Name, description: `${d.Bugs} attacks · ${id} encoding layers · +${d.Penalty}s per failed validation`, id })), { title: 'SourceDefender · Difficulty', ignoreFocusOut: true });
    if (!difficulty) return;
    const seed = await vscode.window.showInputBox({ title: 'SourceDefender · Seed', prompt: 'Optional reproducible seed. Leave blank for a new attack selection.', ignoreFocusOut: true, validateInput: text => text.length > 80 ? 'Use at most 80 characters.' : undefined });
    if (seed === undefined) return;
    const configured = vscode.workspace.getConfiguration('sourceDefender').get('repositoryParent');
    const parent = configured || path.join(os.homedir(), 'SourceDefender', 'runs');
    if (!path.isAbsolute(parent)) throw new Error('Repository parent must be an absolute path.');
    fs.mkdirSync(parent, { recursive: true });
    const selected = await vscode.window.showOpenDialog({ title: 'Choose a parent folder for the incident', defaultUri: vscode.Uri.file(parent), canSelectFolders: true, canSelectFiles: false, canSelectMany: false, openLabel: 'Create incident here' });
    if (!selected?.length) return;
    if (active) await this.client.call({ command: 'end', id: active.Id });
    return this.startWithSettings({ scenario: scenario.id, language, level: difficulty.id, seed, folder: selected[0].fsPath });
  }

  async startWithSettings(settings, options = {}) {
    if (this.busy) throw new Error('An operation is already running.');
    this.busy = true; this.revision++;
    try {
      const data = await vscode.window.withProgress({ location: vscode.ProgressLocation.Notification, title: 'Creating SourceDefender incident…', cancellable: false }, () => this.client.call({ command: 'generate', ...settings }));
      this.agent = false; this.autoWorkspaceMode = true; this.pinnedRun = null; this.selected = data.Run.Id;
      await this.context.globalState.update('sourceDefender.selectedRun', this.selected);
      this.snapshot.Runs = [data.Run, ...this.snapshot.Runs.filter(r => r.Id !== data.Run.Id)];
      this.run = data.Run; this.report = data.Report; this.render();
      if (options.open !== false) await this.openRepository();
      return data;
    } finally { this.busy = false; }
  }

  async openRepository() {
    if (!this.run) return;
    await this.context.globalState.update('sourceDefender.selectedRun', this.run.Id);
    await this.context.globalState.update('sourceDefender.openBriefing', this.run.Id);
    return vscode.commands.executeCommand('vscode.openFolder', vscode.Uri.file(this.run.Folder), { forceNewWindow: true });
  }

  async openFile(relative) {
    if (!this.run) return;
    const filename = safeFile(this.run.Folder, relative);
    const document = await vscode.workspace.openTextDocument(vscode.Uri.file(filename));
    await vscode.window.showTextDocument(document, { preview: false });
    return document;
  }

  humanActive() {
    if (!this.run || this.agent || (this.run.Mode || 'human') !== 'human' || this.run.Status !== 'Active') throw new Error('Choose an active human incident first.');
    return this.run;
  }

  async validate() {
    const run = this.humanActive();
    if (vscode.workspace.getConfiguration('sourceDefender').get('saveBeforeValidation', true)) {
      for (const document of vscode.workspace.textDocuments.filter(d => d.isDirty && d.uri.scheme === 'file' && inside(run.Folder, d.uri.fsPath))) {
        if (!await document.save()) throw new Error('Could not save an incident file; validation was cancelled.');
      }
    }
    return this.act('validate');
  }

  async act(command) {
    const run = this.humanActive();
    if (this.busy) throw new Error('An operation is already running.');
    this.busy = true; this.revision++;
    try {
      const data = await this.client.call({ command, id: run.Id });
      this.run = data.Run; this.report = data.Report;
      this.snapshot.Runs = [data.Run, ...this.snapshot.Runs.filter(r => r.Id !== data.Run.Id)];
      this.output.appendLine(data.Report); this.render();
      if (data.Run.Status === 'Contained') vscode.window.showInformationMessage(`Incident contained · ${formatTime(scoredSeconds(data.Run))} · Score ${data.Run.Score}`);
      else if (command === 'validate') vscode.window.showWarningMessage(`Validation failed · +${this.snapshot.Difficulties[run.Level]?.Penalty || 0}s. Findings are in the SourceDefender sidebar.`);
      return data;
    } finally { this.busy = false; }
  }

  async end() {
    this.humanActive();
    if (await vscode.window.showWarningMessage('End this run as abandoned? Its files will stay on disk.', { modal: true }, 'End run') === 'End run') return this.act('end');
  }

  async history() {
    await this.refresh(true);
    const choice = await vscode.window.showQuickPick(this.snapshot.Runs.map(r => ({ label: `${r.Scenario} · ${r.Status}`, description: `${r.Mode === 'agent_practice' ? 'Agent' : 'Human'} · ${r.Language} · ${formatTime(scoredSeconds(r))}`, run: r })), { title: 'SourceDefender · Run history' });
    if (choice) { this.agent = choice.run.Mode === 'agent_practice'; this.selected = choice.run.Id; this.pinnedRun = choice.run.Id; this.run = choice.run; this.report = choice.run.Events?.at(-1) || ''; this.render(); }
  }
}

async function activate(context) {
  const game = new GameIDE(context);
  await game.refresh(false);
  const requested = context.globalState.get('sourceDefender.openBriefing');
  const matchingWorkspace=game.run && (vscode.workspace.workspaceFolders || []).some(f=>f.uri.scheme==='file' && inside(game.run.Folder,f.uri.fsPath));
  if (game.run && (requested === game.run.Id || (matchingWorkspace && !vscode.window.activeTextEditor))) {
    await context.globalState.update('sourceDefender.openBriefing', undefined);
    await vscode.commands.executeCommand('sourceDefender.incident.focus');
    await game.openFile('README.md');
  }
  return game;
}

module.exports = { activate, GameIDE };
