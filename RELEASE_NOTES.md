## Play inside VS Code

Start an incident and open its real repository with the briefing, timer, findings,
hints, and validation controls together in VS Code. SourceDefender uses native
editor views and commands, and everything runs locally.

- Four platform-specific VSIX downloads bundle the game engine. No additional
  runtime is required. Install through **Extensions → … → Install from VSIX…**.
- Run **SourceDefender: New incident** to choose a scenario, language, difficulty,
  seed, and folder. The generated incident opens in its own editor window.
- Validation saves edited incident files by default and leaves unrelated files
  alone. Findings and charged hints open files when their paths are available.
- Difficulty still controls feedback, encoding layers, and time penalties.
- Run history and the read-only agent practice view are available in the sidebar.
- The Rust desktop app gains **Open in IDE** to hand an incident to VS Code.
- `IDE.md` is included with setup and play instructions. Existing local MCP
  practice and the standalone desktop app remain available.

Download the desktop archive, the VS Code extension, or both for Windows x64,
Linux x64, macOS Apple Silicon, or macOS Intel. Both interfaces share local game
records. VS Code 1.90 or newer is required for the extension.

Every platform must pass engine checks, Rust tests, generated-project tests,
MCP tests, extension helper/packaging tests, and the packaged native-window smoke
test. The Linux release also installs the VSIX and plays an incident in a real
VS Code extension host before publication. `SHA256SUMS.txt` covers all downloads.

Windows uses Direct3D; Linux/macOS use OpenGL. Mac desktop builds are ad-hoc
signed and are not Apple-notarized. Agent practice remains local training, not
competitive benchmarking or a security sandbox.
