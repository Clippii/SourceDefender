# SourceDefender standalone IDE

SourceDefender combines the game and its investigation workspace in one native
Rust application. It creates real source repositories, lets you inspect and
repair their files, and validates the saved result. Everything runs locally.
VS Code and other editors are optional.

## Install and start

Download the standalone archive for your operating system and processor from
[GitHub Releases](https://github.com/Clippii/SourceDefender/releases/latest):

| Platform | Standalone archive |
|---|---|
| Windows x64 | `SourceDefender-win-x64.zip` |
| Linux x64 | `SourceDefender-linux-x64.tar.gz` |
| macOS Apple Silicon | `SourceDefender-osx-arm64.tar.gz` |
| macOS Intel | `SourceDefender-osx-x64.tar.gz` |

Extract the archive and launch `SourceDefender.exe` on Windows or
`./SourceDefender` on Linux. On macOS, move `SourceDefender.app` to Applications
and open it. Keep the game and engine together; the Mac application bundle
already contains both. No language runtime is needed to play.

Install Git if you want the generated maintenance commits, history, and diff
views. Incidents can still be generated and repaired without it.

## Main menu and incident setup

Choose **New operation** to configure an incident, or **Resume operation** to
continue an active run. **Operation archive** opens local results, **Agent watch**
observes an agent, **Field manual** explains the rules, and **Settings** adjusts
the editor.

Choose a scenario, source language, difficulty, optional seed, and repository
parent folder. Starting creates a separate incident subfolder and begins the
clock. Read the scenario mission and difficulty rules, then choose **Begin operation**.

| Difficulty | Attacks | Encoding layers | Failed validation | Hint | Feedback |
|---|---:|---:|---:|---:|---|
| Intern | 3 | 0 | +15s | +20s | Titles and paths |
| Maintainer | 6 | 1 | +45s | +45s | Titles |
| Incident commander | 10 | 2 | +90s | +75s | Category counts |
| Zero trust | 16 | 3 | +180s | +120s | Remaining count |

Only one human incident can be active at a time. Agent practice has its own run.
Returning to the menu or closing the app does not pause the timer.

## Investigate and edit

The workspace keeps the game clock and validation controls beside your code.
The top HUD provides **Save all**, **Validate repairs**, **Request intel**,
**End operation**, and **Main menu**.
Begin with `README.md`, `docs/architecture.md`, and `HANDBOOK.md`.

- Use the explorer to open files, including hidden repository configuration.
- Keep multiple files open in tabs. Modified tabs indicate unsaved changes.
- Edit source directly with syntax colors and line numbers.
- Search the repository for suspicious values or trace a name across files.
- Review Git history and the current working-tree diff when Git is installed.
- Read findings and activity in the game panel. Select a finding to open its file
  when the difficulty, or a charged hint, has revealed a path.

The editor writes to the generated repository. You may also inspect and repair
those files with another editor. A save checks whether a file changed on disk
and reports conflicts instead of overwriting another editor's changes. Repository
access is bounded to the incident folder; Git internals and paths outside it are
not editable through the workspace.

### Keyboard controls

Use Ctrl on Windows/Linux or Command on macOS:

| Shortcut | Action |
|---|---|
| Ctrl/Command + S | Save the active file |
| Ctrl/Command + Shift + S | Save all open files |
| Ctrl/Command + F | Find in the active file |
| Ctrl/Command + Shift + F | Search saved repository files |
| Ctrl/Command + W | Close the active tab, with an unsaved-change prompt |

Current-file find is case-sensitive and wraps. Repository search is
case-insensitive and reads saved files, so save edits before searching for newly
written text. The Git view offers **Working changes** and **Recent commits**;
**View diff** reviews changes to the active file.

## Validate and finish

Select **Save all** to write edited incident files. **Validate repairs** saves
them and scans the repository. If saving fails, resolve the error before validating.
An unsuccessful scan adds the difficulty's penalty and updates the findings.
A hint also adds time when it reveals a new control.

A clean scan contains the incident, stops its clock, and records the final score.
Review the result, activity, and saved source afterward. Ending a run records it
as abandoned and keeps its files. The app guards unsaved edits when closing tabs,
changing workspaces, and exiting. If another interface ends the operation while
you have unsaved edits, those buffers stay available for explicit saving; the
recorded result does not change.

The game uses bounded static checks for its known incidents. It never runs the
generated source, installation hooks, or project tasks. Git views only inspect
history and differences; they do not apply repairs or restore the baseline.

## Agent practice

Open **Agent watch** from the main menu to watch the agent's run, timer,
validation attempts, game tool calls, and activity. Its repository is read-only
in the observer workspace. Agents edit files with their normal tools and play
through the engine's local stdio MCP interface.

Agent and archived repositories are read-only in the built-in workspace.
Human controls cannot validate, hint, or end an agent run. See
[MCP setup and examples](MCP.md) for configuration and practice-mode limits.

## Editor settings

Open **Settings** from the main menu to change the editor font size. Preferences
are saved locally. These display settings do not change difficulty or an active
operation's timing rules.

## Optional VS Code extension

Players who prefer VS Code can download the matching `.vsix` from the same
release. VS Code 1.90 or newer is required. Open **Extensions**, select
**… → Install from VSIX…**, and choose the file for your platform. Run
**SourceDefender: New incident** from the Command Palette.

The generated repository opens in a new VS Code window with a briefing and the
SourceDefender sidebar. Its Incident toolbar provides validation and hints;
Activity shows recent events. The status-bar timer also validates an active
human run when selected. Findings open files when paths are available.

The extension saves edited incident files before validation by default and
leaves unrelated files and untitled documents alone. Its settings are:

| Setting | Purpose |
|---|---|
| `sourceDefender.enginePath` | Optional absolute engine path in user settings. An attacked workspace cannot select the executable. |
| `sourceDefender.repositoryParent` | Default parent folder offered for new incidents. |
| `sourceDefender.saveBeforeValidation` | Save edited incident files before validation; enabled by default. |

The extension supports Restricted Mode and does not execute project source or
tasks. Install the VSIX for the operating system and architecture where the
extension host runs. It bundles its own engine and can be used independently.

## Local files and records

The standalone app, optional extension, and MCP server share game records when
run under the same account and environment. `SOURCEDEFENDER_DATA_DIR` can override
the records directory when launching them. Generated repositories remain in the
parent folder you choose. See the project README for the default record paths
and platform requirements.
