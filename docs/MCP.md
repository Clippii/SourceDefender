# Agents can play SourceDefender

SourceDefender includes a local MCP server in the bundled game engine. It uses
stdio; your agent client launches it as a child process. No server port, account,
additional runtime installation, or cloud service is required by SourceDefender.

## Connect

Add a local stdio server to your MCP client. Set its **command** to the absolute
path of the engine and its **arguments** to `["--mcp"]`.

| Platform | Engine path example |
|---|---|
| Windows | `C:\Games\SourceDefender\SourceDefender.Engine.exe` |
| Linux | `/home/you/SourceDefender/SourceDefender.Engine` |
| macOS | `/Applications/SourceDefender.app/Contents/MacOS/SourceDefender.Engine` |

For clients that use a `mcpServers` configuration, the Windows example is:

```json
{
  "mcpServers": {
    "sourcedefender": {
      "command": "C:\\Games\\SourceDefender\\SourceDefender.Engine.exe",
      "args": ["--mcp"]
    }
  }
}
```

Replace the path with your actual installation. Linux/macOS use the same argument
and their platform's engine path. Some clients use a `servers` key instead; use
your client's local-server configuration format. The engine speaks MCP through
stdout and sends diagnostics to stderr.

## Tools

| Tool | Purpose |
|---|---|
| `list_scenarios` | List scenario IDs, languages, difficulties, and penalties |
| `start_run` | Generate an incident and start the timer |
| `get_run_status` | Read an agent incident's clock, counters, status, and latest feedback |
| `validate_changes` | Check saved repairs; add a penalty on failure or finish on success |
| `request_hint` | Buy one hint using the normal difficulty penalty |
| `end_run` | Abandon the agent incident while preserving its files |

Example `start_run` arguments:

```json
{
  "scenario": 4,
  "language": "JavaScript",
  "difficulty": 1,
  "folder": "/home/you/SourceDefender/agent-workspaces",
  "seed": "agent-practice-01"
}
```

Scenario IDs and difficulty IDs come from `list_scenarios`. Difficulty is 0–3;
3 is Zero trust. On Windows use an absolute Windows path for `folder`. A unique
incident subfolder is created beneath that parent. The result contains its path
and a `run.id`; pass that ID as `run_id` to the other tools.

The agent investigates and edits that directory using its normal coding/file
tools. SourceDefender does not grant arbitrary shell execution or file access
through MCP. Give the agent client access to the generated workspace separately.
Review the repository, save edits, then call `validate_changes`.

## Watch an agent

Open the desktop app and select **Agent practice** in the sidebar. It refreshes
the most recent agent run's status and activity automatically, including the
timer, hints, validation attempts, and game-tool-call count. Human incidents and
agent incidents can each have one active run. Agent tools cannot alter human
runs, and desktop human controls cannot alter an agent run.

By default, the app and MCP server share the normal user data directory. If your
MCP client sets `SOURCEDEFENDER_DATA_DIR` for an isolated practice profile, launch
the desktop app with the same environment value to observe it. Different data
directories can host independent agent sessions concurrently.

## Rules and limits

- The engine owns time, penalties, hints, completion, and run persistence.
- The clock starts on generation and continues after the client disconnects.
- Tool responses expose public run information and difficulty-appropriate
  feedback, not injected attack IDs, answer templates, or internal manifests.
- A hint intentionally reveals a location and remediation after charging its
  normal time penalty.
- `tool_calls` counts the start and successful run-scoped game calls, including
  status polls. It does not count the agent client's separate file tools, model
  tokens, failed requests, or scenario listings.
- Human and agent results are labelled separately in local history. There is no
  competitive cross-model leaderboard in this release.

This is **practice mode**, not a secure evaluation sandbox. A local coding agent
can inspect its generated Git history; restoring the trusted baseline remains
valid under the existing practice rules. The known-incident static validator is
not a general security scanner. Hidden behavioral grading and stronger isolation
would be required for a competitive benchmark. The server never executes
generated source, install hooks, workflows, or remote payloads.

## Verify locally

After building the app/engine:

```sh
python tests/verify_mcp.py
```

The integration suite drives the actual stdio server through negotiation,
schema discovery, invalid arguments, highest-difficulty feedback, penalties,
hints, abandonment, restart persistence, Unicode paths, and simultaneous starts
from two processes. It uses isolated temporary data, leaving player records alone.
