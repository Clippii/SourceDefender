# SourceDefender

A native desktop blue-team game about investigating attacked source repositories.
Generate a codebase, review it in your own editor, remove attacker changes, and
validate your repairs against the clock. Everything runs locally.

**[Download the latest release](https://github.com/Clippii/SourceDefender/releases/latest)**

## Downloads

| Platform | Download | Launch |
|---|---|---|
| Windows x64 | `SourceDefender-win-x64.zip` | Extract, then open `SourceDefender.exe` |
| Linux x64 | `SourceDefender-linux-x64.tar.gz` | Extract, then run `./SourceDefender` |
| macOS Apple Silicon | `SourceDefender-osx-arm64.tar.gz` | Extract, move `SourceDefender.app` to Applications |
| macOS Intel | `SourceDefender-osx-x64.tar.gz` | Extract, move `SourceDefender.app` to Applications |

The game engine is bundled with each release. No .NET, Rust, Python, Node, or Go
installation is required to play. Keep the app and engine together; on macOS both
are inside the application bundle. Downloads include `SHA256SUMS.txt` on the release page.

Linux builds target Ubuntu 24.04 or newer with a graphical desktop and OpenGL.
The X11 backend needs `libxkbcommon-x11-0` (install with `sudo apt-get install libxkbcommon-x11-0` if absent).
Folder browsing uses `zenity` or `kdialog`; entering a path directly also works.
macOS builds target macOS 14 or newer and are ad-hoc signed, **not Apple notarized**.
If macOS blocks the first launch, review the source/checksum and approve the app
through System Settings → Privacy & Security. Windows builds target current x64 Windows.

## Play

1. Choose a scenario, source language, difficulty, and optional seed.
2. Select a parent folder and start. A unique incident repository is created and the clock begins.
3. Open that folder in your preferred editor. Read its README, architecture notes, and handbook.
4. Investigate source, tests, automation, and Git history. Save repairs to the actual files.
5. Return to SourceDefender and select **Validate changes**. A clean result stops the timer.

Failed validation and hints add time. Closing the app does not pause a run.
The next launch restores the active incident. Ending a run preserves its repository.
Git is optional for generation, but installing Git gives you a trusted baseline and
attacker commits disguised as maintenance. Comparing with or restoring that baseline
is a valid containment strategy. The game never executes generated project code.

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

Higher levels add nested Base64 values, misleading adapter names, and changes
spread across separate commits. A seed reproduces attack selection for the same
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
- Rust tests for timing, saved records, Unicode paths, and actual engine interaction.
- `python tests/verify_projects.py`, which creates incidents, restores the trusted
  baseline, runs all four generated project suites and demos, and verifies containment.
- A packaged native-window smoke test (Xvfb with software OpenGL on Linux).

The generated-project verification step needs Python 3.10+, Node 18+, Go 1.20+,
Git, and the .NET 10 SDK. It uses temporary records and does not alter player runs.

Pushing a `v*` tag starts the release workflow. GitHub Releases receives the four
archives and SHA-256 checksums only after every platform job succeeds. Unsigned
or non-notarized distribution limitations are described above.

## Local records

- Windows: `%LOCALAPPDATA%\SourceDefender`
- Linux: `$XDG_DATA_HOME/SourceDefender` or `~/.local/share/SourceDefender`
- macOS: the user's local application-data directory under `~/Library/Application Support`

`SOURCEDEFENDER_DATA_DIR` overrides the record location. Generated repositories
stay in the folder you select. Older Windows incident records remain compatible;
start a new incident to use the latest generated project layout.
