## A standalone investigation IDE

SourceDefender now opens as its own game and source editor. Use the main menu,
configure an incident, investigate and repair its real repository, and validate
your work without installing another editor.

- A main menu provides **New operation**, **Resume operation**, **Operation archive**,
  **Field manual**, **Settings**, and **Agent watch**.
- Incident setup brings the scenario, language, difficulty, seed, and folder
  choices together before the clock starts.
- The native Rust workspace includes a file explorer, editor tabs, syntax colors,
  line numbers, source search, and Git history and working-tree diff views.
- The game HUD keeps time, penalties, findings, hints, and validation close to the
  code. **Validate repairs** saves and scans your work; a successful scan leads
  to the contained result and final score.
- Validation saves edited incident files first. Saves detect external changes,
  and unsaved edits are guarded when leaving a workspace or closing the app.
- Editor font preferences are saved locally. Archived operations open read-only.
- Agent practice remains available through local MCP, with a read-only observer
  workspace in the game. The optional VS Code extension remains available too.

Download the standalone app for Windows x64, Linux x64, macOS Apple Silicon, or
macOS Intel. Each includes its game engine; no VS Code or additional language
runtime is required. The app and optional extension share existing local game
records. `IDE.md` explains installation and play.

Every platform must pass engine checks, Rust tests, generated-project tests,
MCP tests, extension tests, and the packaged native-window smoke test before
publication. The Linux job also exercises the optional VSIX in real VS Code.
`SHA256SUMS.txt` covers all downloads.

Windows uses Direct3D; Linux and macOS use OpenGL. Mac desktop builds are ad-hoc
signed and are not Apple-notarized. Validation checks the game's known incidents;
agent practice remains local training rather than a competitive benchmark.
