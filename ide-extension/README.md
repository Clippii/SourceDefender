# SourceDefender for VS Code

Investigate an attacked source repository, repair its files, and validate your
changes without leaving your editor. SourceDefender uses native sidebar views,
editor files, and commands. Everything runs locally.

## Install

Download the `.vsix` for your operating system and processor from the
[SourceDefender releases](https://github.com/Clippii/SourceDefender/releases).
In VS Code 1.90 or newer, open **Extensions**, choose **Install from VSIX…** from
the **…** menu, and select the download. Open **SourceDefender** in the Activity Bar.

Each platform package includes its own game engine. No .NET, Rust, Python, Node,
or Go installation is required to play.

## Play

1. Choose **New incident** and select a scenario, language, difficulty, seed, and
   parent folder. The game creates a separate incident repository.
2. The repository opens in a new editor window with its briefing, timer, and
   SourceDefender sidebar. Investigate its files and Git history. Use **Open
   briefing** to return to the repository's instructions.
3. Save your repairs and choose **Validate changes** from the Incident toolbar.
   Edited incident files are saved before validation by default; unrelated files
   and unsaved new documents are left alone.
4. Use **Request hint** when needed. Failed validation and hints add time;
   containing the incident stops the clock.

The Incident view shows the current run and its feedback. The Activity view keeps
recent game events in reach. More actions are available from the Incident view's
**…** menu and the Command Palette under **SourceDefender**: **End run**,
**Run history**, **Agent practice**, and **Refresh**.

Closing the editor does not pause a run. Ending a run preserves its repository.
Human incidents and agent practice use separate runs. See the project's
[MCP guide](https://github.com/Clippii/SourceDefender/blob/main/docs/MCP.md) to set
up an agent with the engine's local stdio tools.

## Feedback and files

Findings and hints open a file when the engine reveals a file path. Higher
difficulties can provide only a title, category, or remaining count, so some
feedback has no file to open. Validation does not invent line numbers or add
editor diagnostics for locations the engine has not supplied.

Validation uses bounded static checks for the game's known incidents. The
extension never executes generated project code or workspace tasks. Reviewing
the repository and repairing its real files is the game.

## Settings

| Setting | Default | Purpose |
| --- | --- | --- |
| `sourceDefender.enginePath` | Empty | Absolute path to an external engine; otherwise use the bundled engine. User settings only. |
| `sourceDefender.repositoryParent` | Empty | Default parent folder for new incidents; otherwise choose a folder when starting. |
| `sourceDefender.saveBeforeValidation` | `true` | Save edited incident files before reading repairs for validation. |

SourceDefender works in Restricted Mode because its engine does not execute
project code. A repository cannot override the user-only engine path. Incident
repositories must be stored on a file system accessible to the extension host.
