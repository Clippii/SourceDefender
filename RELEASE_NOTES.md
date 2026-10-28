## Agent practice is here

SourceDefender now includes a local MCP server. Point your MCP client at the
bundled game engine with the `--mcp` argument. No web layer or additional runtime
installation is needed.

- Six tools: list scenarios, start a run, get status, validate repairs, request a hint, and end a run.
- Same clock, difficulty feedback, and time penalties as human play.
- Sanitized responses keep attack IDs and internal manifests out of tool output.
- Human and agent incidents are separate, with shared-store locking and atomic saves.
- An **Agent practice** desktop view observes the latest agent incident and its activity.
- `MCP.md` is included with platform-specific paths and configuration examples.

This release provides agent practice, not competitive benchmarking or a security
sandbox. Agents use their normal file tools, and the trusted Git baseline remains
available under the existing rules.

Downloads cover Windows x64, Linux x64, macOS Apple Silicon, and macOS Intel.
The local engine is self-contained. Windows uses Direct3D; Linux/macOS use OpenGL.
Mac builds are ad-hoc signed and are not Apple-notarized.

Every platform must pass the game checks, Rust tests, generated-project tests,
MCP protocol/lifecycle/concurrency tests, and packaged-window smoke test before
publication. SHA256SUMS.txt contains checksums for the downloads.
