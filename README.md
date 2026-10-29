# SourceDefender

A standalone blue-team game and source investigation IDE, built in Rust.
Choose an incident, investigate an attacked repository, repair its real files,
and contain the breach against the clock. The main menu, setup, editor, Git
review, validation, and results live in one local application.

No separate editor or VS Code installation is required. An optional VS Code
extension remains available for players who prefer their usual editor.

**[Download the latest release](https://github.com/Clippii/SourceDefender/releases/latest)**

## Downloads

| Platform | Standalone game IDE | Optional VS Code extension |
|---|---|---|
| Windows x64 | [SourceDefender-win-x64.zip](https://github.com/Clippii/SourceDefender/releases/latest/download/SourceDefender-win-x64.zip) | [SourceDefender-win-x64.vsix](https://github.com/Clippii/SourceDefender/releases/latest/download/SourceDefender-win-x64.vsix) |
| Linux x64 | [SourceDefender-linux-x64.tar.gz](https://github.com/Clippii/SourceDefender/releases/latest/download/SourceDefender-linux-x64.tar.gz) | [SourceDefender-linux-x64.vsix](https://github.com/Clippii/SourceDefender/releases/latest/download/SourceDefender-linux-x64.vsix) |
| macOS Apple Silicon | [SourceDefender-osx-arm64.tar.gz](https://github.com/Clippii/SourceDefender/releases/latest/download/SourceDefender-osx-arm64.tar.gz) | [SourceDefender-osx-arm64.vsix](https://github.com/Clippii/SourceDefender/releases/latest/download/SourceDefender-osx-arm64.vsix) |
| macOS Intel | [SourceDefender-osx-x64.tar.gz](https://github.com/Clippii/SourceDefender/releases/latest/download/SourceDefender-osx-x64.tar.gz) | [SourceDefender-osx-x64.vsix](https://github.com/Clippii/SourceDefender/releases/latest/download/SourceDefender-osx-x64.vsix) |

Extract the standalone archive and launch `SourceDefender.exe` on Windows or
`./SourceDefender` on Linux. On macOS, move `SourceDefender.app` to Applications.
Open the main menu, configure an incident, and begin investigating in the built-in
workspace. See the **[IDE guide](docs/IDE.md)** for controls and the full play flow.

The game engine is bundled with each download. No .NET, Rust, Python, Node, or Go
installation is required to play. Keep the app and engine together; on macOS both
are inside the application bundle. `SHA256SUMS.txt` on the release page covers all
downloads. Git is optional, but enables repository history and diff review.

For the optional extension, use VS Code 1.90 or newer and choose
**Extensions → … → Install from VSIX…**. Install the file for your platform, then
run **SourceDefender: New incident** from the Command Palette. The extension also
bundles its engine and shares local game records with the standalone app.

Linux builds target Ubuntu 24.04 or newer with a graphical desktop and OpenGL.
The X11 backend needs `libxkbcommon-x11-0` (install with `sudo apt-get install libxkbcommon-x11-0` if absent).
Folder browsing uses `zenity` or `kdialog`; entering a path directly also works.
macOS builds target macOS 14 or newer and are ad-hoc signed, **not Apple notarized**.
If macOS blocks the first launch, review the source/checksum and approve the app
through System Settings → Privacy & Security. Windows builds target current x64 Windows.
Windows uses the native Direct3D graphics backend; Linux and macOS use OpenGL.

## Play

1. Choose **New operation** from the main menu, or **Resume operation** for an active run.
2. Select a scenario, source language, difficulty, optional seed, and parent
   folder. Starting creates a unique repository and begins the clock.
3. Read the briefing, architecture notes, and handbook in the integrated editor.
   Explore files, search the source, and review Git evidence.
4. Repair the actual files using the tabbed editor. Validation saves your edited
   incident files before scanning; it cancels if a file cannot be saved.
5. Select **Validate repairs**. Unresolved controls add a time penalty; a clean
   result contains the incident, stops the clock, and records the score.

The built-in workspace includes a file explorer, syntax-colored editing, line
numbers, search, Git history and working-tree diff views, and a game HUD with
time, penalties, hints, and validation. Findings open files only when the selected
difficulty allows their paths to be revealed. The debrief and **Operation archive** keep completed runs available for review.

Failed validation and hints add time. Returning to the menu or closing the app
does not pause a run. Ending a run preserves its repository. Unsaved edits are
protected when changing workspaces or exiting, and saves detect changes made by
another editor before overwriting a file.

Intern and Maintainer can include a trusted baseline and attacker commits when Git
is installed. New Incident commander and Zero trust runs start without Git history or a
recoverable baseline. The game never executes generated project code. You can also open the
repository in another editor, or play through the optional VS Code extension.

## Agent practice over MCP

Coding agents can play through the bundled engine's **local stdio MCP mode**:
launch `SourceDefender.Engine.exe --mcp` on Windows, or `SourceDefender.Engine --mcp`
on Linux/macOS. The six tools list scenarios, start an agent incident, report
status, validate saved changes, provide hints, and end a run. The agent edits
the generated repository using its normal file tools.

Open **Agent watch** from the main menu to observe its timer, game tool
calls, validation attempts, and activity in a read-only workspace. Human and agent runs are labelled
separately and cannot control each other. A run's attack IDs and private manifests
are never returned by the MCP tools or desktop protocol.

See **[MCP setup and examples](docs/MCP.md)** for platform paths, client
configuration, tool arguments, and practice-mode limits. A copy of `MCP.md` is
also included with the downloads. This is local practice, not a competitive
benchmark or security sandbox; local agents have the same history restrictions as players.

## Incidents

Six scenarios cover a release service, trading gateway, multiplayer backend,
records service, satellite relay, and abandoned package. Each provides domain
categories, configuration, six synthetic users, and 48 records.

Choose **Python, JavaScript, Go, or C#**. Repositories contain roughly 44–50 files
and 770–920 lines of source and tests, with connected models, request handling,
storage, authentication, billing, audit, exports, telemetry, review workflows,
retrying jobs, reconciliation, and plugin/update planning. Each includes a local
demo and regression suite. Source runtimes are only needed if you choose to run
those commands after reviewing the code.

The attack library includes authentication backdoors, credential collection,
payment diversion, audit suppression, privilege escalation, update hijacking,
shadow exports, remote plugin sources, install hooks, registry hijacking, CI
credential upload, review ownership takeover, hidden source paths, deployment
redirection, signer poisoning, and scheduled persistence.

| Difficulty | Attacks | Encoding layers | Failed validation | Hint | Feedback |
|---|---:|---:|---:|---:|---|
| Intern | 3 | 0 | +15s | +20s | Titles and paths |
| Maintainer | 6 | 1 | +45s | +45s | Titles |
| Incident commander | 10 | 2 | +90s | +75s | Category counts |
| Zero trust | 16 | 3 | +180s | +120s | Remaining count |

Higher levels add nested Base64 values and misleading adapter names. Incident
commander and Zero trust omit Git history. A seed reproduces attack selection for the same
settings and generator version.

These are local training services. Network delivery, payments, and payload execution
are represented as inspectable in-memory plans. Endpoints use reserved `.invalid`
domains. Validation uses bounded static checks for the known incidents, including
missing/emptied source and tests; it is not a general-purpose malware scanner.
The project's own regression tests provide additional behavioral checks after review.

## Build from source

Install Rust stable, the .NET 10 SDK, and Git. Linux additionally needs its native
windowing/OpenGL development libraries; on Ubuntu:

```sh
sudo apt-get install build-essential pkg-config libx11-dev libxrandr-dev libxi-dev \
  libxcursor-dev libxinerama-dev libxkbcommon-dev libxkbcommon-x11-dev libwayland-dev \
  libgl1-mesa-dev libegl1-mesa-dev libudev-dev zenity
bash build.sh --test
```

On macOS, install Xcode Command Line Tools, Rust, and the .NET 10 SDK, then:

```sh
bash build.sh --test
```

On Windows, install Rust with the MSVC C++ build tools or GNU/MinGW-w64, then:

```powershell
.\build.ps1 -Test
```

Output is under `dist/<platform>/`. The build publishes a self-contained engine
and builds the native Rust interface. Mac builds create an application bundle.
`build.ps1 -EngineOnly -Test` rebuilds only the Windows engine.

The Rust interface is in `src/`, the .NET engine in `engine/`, and the embedded
project templates in `templates/`. The components communicate through JSON over
standard input/output. There is no HTTP service or web UI.

## Verification and releases

GitHub Actions builds Windows x64, Linux x64, macOS arm64, and macOS x64 on native
runners. Every package must pass:

- 1,632 engine checks covering languages, difficulty, legacy/current layouts,
  attack detection, restored projects, code size, and missing files.
- Rust tests for timing, saved records, Unicode paths, actual engine interaction,
  bounded repository access, editing, and conflict detection.
- `python tests/verify_mcp.py` for real stdio negotiation, schemas, argument
  validation, response privacy, shared-store concurrency, mode isolation,
  timing/penalties, and restart persistence.
- `python tests/verify_projects.py`, which creates incidents, repairs them using a
  test-only trusted fixture, runs all four generated project suites and demos, and verifies containment.
- A packaged native-window smoke test (Xvfb with software OpenGL on Linux).
- Extension helper and platform packaging tests, plus installation and gameplay in
  a real VS Code extension host on Linux.

The generated-project verification step needs Python 3.10+, Node 18+, Go 1.20+,
Git, and the .NET 10 SDK. It uses temporary records and does not alter player runs.

Pushing a `v*` tag starts the release workflow. GitHub Releases receives the four
desktop archives, four platform VSIX packages, and SHA-256 checksums only after
every platform job succeeds. Unsigned
or non-notarized distribution limitations are described above.

## Local records

- Windows: `%LOCALAPPDATA%\SourceDefender`
- Linux: `$XDG_DATA_HOME/SourceDefender` or `~/.local/share/SourceDefender`
- macOS: the user's local application-data directory under `~/Library/Application Support`

`SOURCEDEFENDER_DATA_DIR` overrides the record location. Generated repositories
stay in the folder you select. Older Windows incident records remain compatible;
start a new incident to use the latest generated project layout.
