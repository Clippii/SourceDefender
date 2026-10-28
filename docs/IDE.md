# Play in VS Code

SourceDefender's VS Code extension puts the incident controls beside the real
repository files. Start an incident, investigate its source and Git history,
save repairs, and validate them from the editor. The game and its bundled engine
run locally; no web service or separate runtime installation is required.

## Install

Use VS Code 1.90 or newer. Download the matching extension from
[GitHub Releases](https://github.com/Clippii/SourceDefender/releases/latest):

| Platform | Extension |
|---|---|
| Windows x64 | `SourceDefender-win-x64.vsix` |
| Linux x64 | `SourceDefender-linux-x64.vsix` |
| macOS Apple Silicon | `SourceDefender-osx-arm64.vsix` |
| macOS Intel | `SourceDefender-osx-x64.vsix` |

In VS Code, open **Extensions**, select **… → Install from VSIX…**, and choose the
download. Reload if prompted. The SourceDefender shield appears in the Activity
Bar. Install updated VSIX files the same way.

The extension works on its own. The native desktop app is optional, and both use
the same local game records when run under the same account and environment.

## Start and investigate

1. Open the Command Palette and run **SourceDefender: New incident**, or select
   **New incident** from the SourceDefender sidebar.
2. Choose the scenario, source language, difficulty, optional seed, and parent
   folder. A separate repository is generated and the timer begins.
3. The repository opens in a new VS Code window with its README and SourceDefender
   sidebar. Read the briefing, architecture notes, and handbook, then investigate
   the source, tests, automation, and Git history.
4. Repair the actual files and select **Validate changes** in the Incident view's
   toolbar. A clean result contains the incident and stops the clock.

The Incident view shows elapsed time including penalties, scan attempts, hints,
and the latest findings. The Activity view lists recent game events. The status
bar also shows the active timer; selecting it validates an active human incident.
**Open briefing** returns to the repository's README.

Validation saves edited files inside the incident folder by default. Unrelated
files and unsaved new documents are left alone. Set
`sourceDefender.saveBeforeValidation` to `false` if you prefer to save manually;
the engine always validates files saved on disk.

Findings open a source file only when a path is available. Intern difficulty
reveals titles and paths, Maintainer reveals titles, Incident commander reveals
category counts, and Zero trust reveals only the remaining count. A charged hint
can reveal a path. Failed scans and hints add the same difficulty-dependent time
penalties as the desktop game.

Closing VS Code does not pause a run. **Run history** opens previous results;
**End run** abandons the selected active human incident and preserves its files.
Starting another incident offers to open or end an existing active human run.

## Open from the desktop app

Install VS Code and the matching SourceDefender extension first. In the native
app, select **Open in IDE** on an incident to open its repository in a new VS Code
window. The extension loads the same run and its current time and feedback.

The desktop shortcut locates a standard VS Code installation on Windows, uses
VS Code in `/Applications` on macOS, and uses `code` on the Linux command path.
If your editor is installed elsewhere, use **Open folder** and open the incident
manually in VS Code or your preferred editor.

## Agent practice

**SourceDefender: Agent practice** switches the sidebar to an agent's run. It
shows the timer, tool calls, and activity while the agent edits its generated
repository. Human controls cannot validate, hint, or end an agent run. Use the
command again to return to human incidents, or select a run from history.

Agents play through the engine's local stdio MCP tools. See
[MCP setup and examples](MCP.md) for configuration and practice-mode limits.

## Settings and local files

| Setting | Purpose |
|---|---|
| `sourceDefender.enginePath` | Optional absolute path to another engine. Set this in user settings; an attacked workspace cannot select the executable. |
| `sourceDefender.repositoryParent` | Default parent folder offered when generating an incident. |
| `sourceDefender.saveBeforeValidation` | Save edited incident files before validation; enabled by default. |

The extension supports Restricted Mode and never executes generated source code
or workspace tasks. Validation uses bounded static checks for the game's known
attacks. Incidents must be stored on a file system the extension host can access.
Install the VSIX for the operating system and architecture where that host runs.

Records use the same location as the native app. `SOURCEDEFENDER_DATA_DIR` can
override it when launching VS Code and the desktop app. Generated repositories
stay in the parent folder you choose.
