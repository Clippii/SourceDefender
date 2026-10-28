"""Exercise the packaged MCP server over real stdio, using only Python's stdlib."""
import concurrent.futures
import json
import os
import pathlib
import queue
import shutil
import stat
import subprocess
import sys
import tempfile
import threading
import time


ROOT = pathlib.Path(__file__).resolve().parent.parent
ENGINE = ROOT / ("SourceDefender.Engine.exe" if os.name == "nt" else "SourceDefender.Engine")
TIMEOUT = 45
TOOLS = {
    "list_scenarios", "start_run", "get_run_status",
    "validate_changes", "request_hint", "end_run",
}
PRIVATE_KEYS = {"broken", "revealed", "requiredfiles", "templateversion", "attackids"}
FINDING_TITLES = (
    "Magic-token authentication backdoor",
    "Credential theft disguised as telemetry",
    "Payment destination replaced",
    "Attacker activity hidden from audit trail",
    "Hidden account privilege escalation",
    "Update feed redirected to attacker",
    "Record exports copied to an outsider",
    "Remote payload loader introduced",
    "Malicious dependency install hook",
    "Dependency registry hijack",
    "CI uploads private credentials",
    "Attacker seized review ownership",
    "Payload hidden from ordinary Git review",
    "Deployment target silently redirected",
    "Attacker signing key added to trust store",
    "Persistence through scheduled automation",
)


def normalized(value):
    return "".join(c for c in value.casefold() if c.isalnum())


def public_values(value, attack_ids=()):
    """Check decoded data as well as the JSON embedded in MCP text content."""
    if isinstance(value, dict):
        for key, item in value.items():
            assert normalized(key) not in PRIVATE_KEYS, f"Private metadata leaked: {key}"
            public_values(item, attack_ids)
    elif isinstance(value, list):
        for item in value:
            public_values(item, attack_ids)
    elif isinstance(value, str):
        assert value not in attack_ids, f"Private attack ID leaked: {value}"


class MCP:
    """A timeout on every response keeps a wedged child from hanging CI."""

    def __init__(self, store):
        self.process = subprocess.Popen(
            [str(ENGINE), "--mcp"], cwd=ROOT,
            env=dict(os.environ, SOURCEDEFENDER_DATA_DIR=str(store)),
            stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
            text=True, encoding="utf-8", bufsize=1,
        )
        self.responses = queue.Queue()
        self.stderr = []
        self.sequence = 0
        self.readers = [
            threading.Thread(target=self._read_stdout, daemon=True),
            threading.Thread(target=self._read_stderr, daemon=True),
        ]
        for reader in self.readers:
            reader.start()

    def _read_stdout(self):
        try:
            for line in self.process.stdout:
                if not line.strip():
                    continue
                try:
                    self.responses.put(json.loads(line))
                except (ValueError, TypeError) as error:
                    self.responses.put(RuntimeError(f"Non-JSON server stdout: {line!r}: {error}"))
        except Exception as error:
            self.responses.put(error)
        finally:
            self.responses.put(EOFError("MCP stdout closed"))

    def _read_stderr(self):
        try:
            for line in self.process.stderr:
                self.stderr.append(line)
                if len(self.stderr) > 100:
                    del self.stderr[0]
        except (OSError, ValueError):
            pass

    def diagnostics(self):
        return "".join(self.stderr[-30:])

    def send(self, message):
        try:
            self.process.stdin.write(json.dumps(message, ensure_ascii=False) + "\n")
            self.process.stdin.flush()
        except (OSError, ValueError) as error:
            raise AssertionError(f"Cannot write to MCP server: {error}\n{self.diagnostics()}") from error

    def request(self, method, params=None):
        self.sequence += 1
        identifier = self.sequence
        message = {"jsonrpc": "2.0", "id": identifier, "method": method}
        if params is not None:
            message["params"] = params
        self.send(message)
        try:
            reply = self.responses.get(timeout=TIMEOUT)
        except queue.Empty as error:
            raise AssertionError(f"MCP {method} timed out after {TIMEOUT}s\n{self.diagnostics()}") from error
        if isinstance(reply, Exception):
            raise AssertionError(f"MCP {method} failed: {reply}\n{self.diagnostics()}") from reply
        assert isinstance(reply, dict), reply
        assert reply.get("jsonrpc") == "2.0", reply
        assert reply.get("id") == identifier, f"Unexpected MCP response: {reply}"
        assert ("result" in reply) != ("error" in reply), reply
        public_values(reply)
        return reply

    def initialize(self):
        reply = self.request("initialize", {
            "protocolVersion": "2025-06-18", "capabilities": {},
            "clientInfo": {"name": "sourcedefender-integration-test", "version": "1.0"},
        })
        assert "error" not in reply, reply
        result = reply["result"]
        assert result["protocolVersion"] == "2025-06-18", result
        assert isinstance(result.get("capabilities", {}).get("tools"), dict), result
        assert result.get("serverInfo", {}).get("name"), result
        assert result.get("serverInfo", {}).get("version"), result
        self.send({"jsonrpc": "2.0", "method": "notifications/initialized"})
        self.ping()
        return self

    def ping(self):
        reply = self.request("ping", {})
        assert reply.get("result") == {}, reply

    def tool_reply(self, name, arguments):
        return self.request("tools/call", {"name": name, "arguments": arguments})

    @staticmethod
    def unpack(reply):
        assert "error" not in reply, reply
        result = reply["result"]
        assert not result.get("isError", False), result
        assert isinstance(result.get("content"), list) and result["content"], result
        text = [item["text"] for item in result["content"] if item.get("type") == "text"]
        assert len(text) == 1, result
        decoded = json.loads(text[0])
        assert isinstance(decoded, dict), decoded
        assert result.get("structuredContent") == decoded, result
        public_values(decoded)
        return decoded

    def tool(self, name, arguments=None):
        return self.unpack(self.tool_reply(name, arguments or {}))

    def expect_tool_error(self, name, arguments):
        reply = self.tool_reply(name, arguments)
        assert "error" in reply or reply["result"].get("isError") is True, reply
        return reply

    def close(self):
        if self.process.stdin:
            try:
                self.process.stdin.close()
            except OSError:
                pass
        try:
            self.process.wait(timeout=5)
        except subprocess.TimeoutExpired:
            self.process.terminate()
            try:
                self.process.wait(timeout=5)
            except subprocess.TimeoutExpired:
                self.process.kill()
                self.process.wait(timeout=5)
        for reader in self.readers:
            reader.join(timeout=1)
        self.process.stdout.close()
        self.process.stderr.close()

    def __enter__(self):
        try:
            return self.initialize()
        except BaseException:
            self.close()
            raise

    def __exit__(self, *_):
        self.close()


def records(store):
    result = {}
    for path in store.rglob("*.json"):
        data = json.loads(path.read_text(encoding="utf-8-sig"))
        if isinstance(data, dict):
            fields = {normalized(key): value for key, value in data.items()}
            if fields.get("id") and fields.get("folder"):
                result[fields["id"]] = fields
    return result


def check_schemas(client):
    response = client.request("tools/list", {})
    assert "error" not in response, response
    listing = response["result"]["tools"]
    tools = {item["name"]: item for item in listing}
    assert len(listing) == len(tools) and set(tools) == TOOLS, listing
    for name, tool in tools.items():
        assert tool.get("description"), tool
        schema = tool["inputSchema"]
        assert schema.get("type") == "object", tool
        properties = schema.get("properties", {})
        required = set(schema.get("required", []))
        if name == "start_run":
            assert required == {"scenario", "language", "difficulty", "folder"}, schema
            for key in ("scenario", "difficulty"):
                assert properties[key]["type"] == "integer", schema
                assert properties[key]["minimum"] == 0, schema
            assert properties["scenario"]["maximum"] == 5, schema
            assert properties["difficulty"]["maximum"] == 3, schema
            for key in ("language", "seed", "folder"):
                assert properties[key]["type"] == "string", schema
        elif name == "get_run_status":
            assert not required, schema
            assert properties["run_id"]["type"] == "string", schema
        elif name == "list_scenarios":
            assert not required, schema
        else:
            assert required == {"run_id"}, schema
            assert properties["run_id"]["type"] == "string", schema


def check_invalid_starts(client, store, folder):
    valid = {"scenario": 0, "language": "Python", "difficulty": 3, "folder": str(folder)}
    invalid = [
        dict(valid, scenario=-1), dict(valid, scenario=6),
        dict(valid, scenario=0.5), dict(valid, scenario=True),
        dict(valid, difficulty=-1), dict(valid, difficulty=4),
        dict(valid, difficulty="3"), dict(valid, language="Ruby"),
        dict(valid, folder=None), dict(valid, seed=99),
        {key: value for key, value in valid.items() if key != "folder"},
        {},
    ]
    before = records(store)
    directories = set(folder.iterdir())
    for arguments in invalid:
        client.expect_tool_error("start_run", arguments)
        assert records(store) == before, f"Invalid arguments persisted a run: {arguments}"
        assert set(folder.iterdir()) == directories, f"Invalid arguments created a project: {arguments}"
    client.expect_tool_error("validate_changes", {})
    client.expect_tool_error("request_hint", {"run_id": "no-such-run"})
    client.expect_tool_error("end_run", {"run_id": "no-such-run"})
    client.expect_tool_error("no_such_tool", {})
    assert client.tool("get_run_status")["run"] is None
    client.ping()


def check_lifecycle(root):
    store = root / "records"
    folder = root / "incident workspaces \u00e9 \u6e2c\u8a66"
    folder.mkdir()
    with MCP(store) as client:
        check_schemas(client)
        scenarios = client.tool("list_scenarios")
        assert scenarios["mode"] == "agent_practice", scenarios
        assert [item["id"] for item in scenarios["scenarios"]] == list(range(6)), scenarios
        assert all(item.get("name") and item.get("description") and item.get("mission")
                   for item in scenarios["scenarios"]), scenarios
        assert set(scenarios["languages"]) == {"Python", "JavaScript", "Go", "C#"}, scenarios
        assert [item["id"] for item in scenarios["difficulties"]] == list(range(4)), scenarios
        assert scenarios["difficulties"][3]["attack_count"] == 16, scenarios
        assert scenarios["difficulties"][3]["failed_validation_penalty_seconds"] == 180, scenarios
        assert scenarios["difficulties"][3]["hint_penalty_seconds"] == 120, scenarios
        assert client.tool("get_run_status")["run"] is None
        check_invalid_starts(client, store, folder)

        started = client.tool("start_run", {
            "scenario": 0, "language": "Python", "difficulty": 3,
            "seed": "mcp-integration-\u00e9", "folder": str(folder),
        })
        run = started["run"]
        identifier = run["id"]
        assert isinstance(identifier, str) and identifier, run
        assert run["status"] == "Active" and run["mode"] == "agent_practice", run
        assert run["language"] == "Python" and run["difficulty"] == 3, run
        assert run["seed"] == "mcp-integration-\u00e9" and run["attack_count"] == 16, run
        assert run["penalty_seconds"] == run["validation_attempts"] == run["hints"] == 0, run
        assert isinstance(run["elapsed_seconds"], (int, float)) and run["elapsed_seconds"] >= 0, run
        project = pathlib.Path(run["folder"]).resolve()
        assert project.is_dir() and folder.resolve() in project.parents, run
        assert (project / "README.md").is_file(), run
        assert isinstance(started["report"], str) and started["report"], started
        saved = records(store)
        assert len(saved) == 1 and identifier in saved, saved
        private = saved[identifier]
        attack_ids = set(private.get("broken", []))
        assert len(attack_ids) == 16, "Test must inspect the real persisted private attack selection"
        public_values(started, attack_ids)

        status = client.tool("get_run_status", {"run_id": identifier})
        assert status["run"]["id"] == identifier and status["run"]["status"] == "Active", status
        assert client.tool("get_run_status")["run"]["id"] == identifier
        client.expect_tool_error("start_run", {
            "scenario": 1, "language": "Python", "difficulty": 0, "folder": str(folder),
        })
        assert len(records(store)) == 1

        failed = client.tool("validate_changes", {"run_id": identifier})
        assert failed["run"]["status"] == "Active", failed
        assert failed["run"]["validation_attempts"] == 1, failed
        assert failed["run"]["penalty_seconds"] == 180 and failed["run"]["hints"] == 0, failed
        assert isinstance(failed["report"], str) and failed["report"], failed
        for response in (started, status, failed):
            public_values(response, attack_ids)
            report = response.get("report", "")
            assert not any(title in report for title in FINDING_TITLES), report
            assert not any(path in report for path in private.get("requiredfiles", [])), report

        hinted = client.tool("request_hint", {"run_id": identifier})
        assert hinted["run"]["status"] == "Active", hinted
        assert hinted["run"]["hints"] == 1 and hinted["run"]["penalty_seconds"] == 300, hinted
        assert hinted["run"]["validation_attempts"] == 1, hinted
        assert isinstance(hinted["report"], str) and hinted["report"], hinted
        assert hinted["report"] != failed["report"], hinted
        public_values(hinted, attack_ids)

        ended = client.tool("end_run", {"run_id": identifier})
        assert ended["run"]["status"] == "Abandoned", ended
        assert ended["run"]["penalty_seconds"] == 300 and project.is_dir(), ended
        public_values(ended, attack_ids)
        for arguments in ({}, {"run_id": identifier}):
            status = client.tool("get_run_status", arguments)
            assert status["run"]["id"] == identifier and status["run"]["status"] == "Abandoned", status
            public_values(status, attack_ids)
        for name in ("validate_changes", "request_hint", "end_run"):
            client.expect_tool_error(name, {"run_id": identifier})
        client.ping()

    with MCP(store) as restarted:
        for arguments in ({}, {"run_id": identifier}):
            status = restarted.tool("get_run_status", arguments)
            run = status["run"]
            assert run["id"] == identifier and run["status"] == "Abandoned", status
            assert run["mode"] == "agent_practice" and run["penalty_seconds"] == 300, status
            assert run["validation_attempts"] == 1 and run["hints"] == 1, status
            public_values(status, attack_ids)
        restarted.ping()
    print("PASS: MCP negotiation, schemas, input validation, private metadata, run lifecycle, and persistence", flush=True)


def check_concurrent_start(root):
    store = root / "concurrent-records"
    folder = root / "concurrent-projects"
    folder.mkdir()
    with MCP(store) as first, MCP(store) as second:
        barrier = threading.Barrier(2, timeout=TIMEOUT)

        def start(client):
            barrier.wait()
            return client.tool_reply("start_run", {
                "scenario": 1, "language": "Python", "difficulty": 0,
                "seed": "concurrent-start", "folder": str(folder),
            })

        with concurrent.futures.ThreadPoolExecutor(max_workers=2) as workers:
            replies = list(workers.map(start, (first, second)))
        successes = [reply for reply in replies
                     if "result" in reply and not reply["result"].get("isError", False)]
        assert len(successes) == 1, f"Exactly one concurrent start must succeed: {replies}"
        run = MCP.unpack(successes[0])["run"]
        assert run["status"] == "Active" and run["mode"] == "agent_practice", run
        assert len(records(store)) == 1, "Concurrent starts created duplicate records"
        assert len(list(folder.iterdir())) == 1, "Rejected concurrent start left a project behind"
        for client in (first, second):
            status = client.tool("get_run_status")
            assert status["run"]["id"] == run["id"] and status["run"]["status"] == "Active", status
        first.tool("end_run", {"run_id": run["id"]})
        assert second.tool("get_run_status")["run"]["status"] == "Abandoned"
        first.ping()
        second.ping()
    print("PASS: two MCP processes serialize starts and observe shared persisted status", flush=True)


def cli(store, request, expect_error=False):
    response = subprocess.run(
        [str(ENGINE)], cwd=ROOT,
        env=dict(os.environ, SOURCEDEFENDER_DATA_DIR=str(store)),
        input=json.dumps(request, ensure_ascii=False), capture_output=True,
        text=True, encoding="utf-8", timeout=TIMEOUT,
    )
    reply = json.loads(response.stdout)
    public_values(reply)
    if expect_error:
        assert response.returncode != 0 and reply.get("ok") is False, reply
        assert isinstance(reply.get("error"), str) and reply["error"], reply
        return reply
    assert response.returncode == 0 and reply.get("ok") is True, (reply, response.stderr)
    return reply["data"]


def check_containment_and_mode_isolation(root):
    store = root / "mode-records"
    folder = root / "mode-projects"
    folder.mkdir()
    with MCP(store) as client:
        barrier = threading.Barrier(2, timeout=TIMEOUT)

        def start_human():
            barrier.wait()
            return cli(store, {
                "command": "generate", "scenario": 3, "language": "Python",
                "level": 0, "seed": "human-isolation", "folder": str(folder),
            })

        def start_agent():
            barrier.wait()
            return client.tool("start_run", {
                "scenario": 2, "language": "Python", "difficulty": 0,
                "seed": "agent-containment", "folder": str(folder),
            })

        with concurrent.futures.ThreadPoolExecutor(max_workers=2) as workers:
            human_future = workers.submit(start_human)
            agent_future = workers.submit(start_agent)
            human = human_future.result()["Run"]
            agent = agent_future.result()["run"]
        human_id, agent_id = human["Id"], agent["id"]
        assert human_id != agent_id and human["Status"] == agent["status"] == "Active"
        assert human["Mode"] == "human" and agent["mode"] == "agent_practice"
        assert human["AttackCount"] == agent["attack_count"] == 3
        assert len(list(folder.iterdir())) == 2, "Both modes must have their own generated repository"
        before = records(store)
        assert set(before) == {human_id, agent_id}, before
        attack_ids = set(before[human_id]["broken"]) | set(before[agent_id]["broken"])
        public_values(human, attack_ids)
        public_values(agent, attack_ids)

        # Each interface may act only on its own mode, even with a valid foreign ID.
        for name in ("get_run_status", "validate_changes", "request_hint", "end_run"):
            client.expect_tool_error(name, {"run_id": human_id})
        for command in ("validate", "hint", "end"):
            cli(store, {"command": command, "id": agent_id}, expect_error=True)
        assert records(store) == before, "Rejected cross-mode access changed a saved run"

        snapshot = cli(store, {"command": "snapshot"})
        public_values(snapshot, attack_ids)
        visible = {run["Id"]: run for run in snapshot["Runs"]}
        assert set(visible) == {human_id, agent_id}, snapshot
        assert visible[human_id]["Mode"] == "human" and visible[agent_id]["Mode"] == "agent_practice"
        assert all(run["Status"] == "Active" and run["AttackCount"] == 3 for run in visible.values())
        assert client.tool("get_run_status")["run"]["id"] == agent_id

        failed = client.tool("validate_changes", {"run_id": agent_id})
        assert failed["run"]["status"] == "Active" and failed["run"]["penalty_seconds"] == 15, failed
        hinted = client.tool("request_hint", {"run_id": agent_id})
        assert hinted["run"]["hints"] == 1 and hinted["run"]["penalty_seconds"] == 35, hinted
        project = pathlib.Path(agent["folder"])
        assert shutil.which("git"), "Git is required to restore the trusted containment fixture"
        baseline = subprocess.check_output(
            ["git", "-C", str(project), "rev-list", "--max-parents=0", "HEAD"],
            text=True, encoding="utf-8", timeout=TIMEOUT,
        ).strip()
        assert baseline and "\n" not in baseline, f"Expected one trusted Git baseline: {baseline!r}"
        subprocess.run(
            ["git", "-C", str(project), "restore", "--source", baseline, "--", "."],
            check=True, capture_output=True, text=True, encoding="utf-8", timeout=TIMEOUT,
        )
        contained = client.tool("validate_changes", {"run_id": agent_id})
        result = contained["run"]
        assert result["status"] == "Contained" and result["mode"] == "agent_practice", contained
        assert result["validation_attempts"] == 2 and result["hints"] == 1, contained
        assert result["penalty_seconds"] == 35 and result["elapsed_seconds"] >= 35, contained
        frozen_seconds = result["elapsed_seconds"]
        frozen_score = max(100, 5000 - int(frozen_seconds) - 2 * 50 - 100)
        assert result["score"] == frozen_score, contained
        public_values(contained, attack_ids)
        assert project.is_dir() and (project / "README.md").is_file()
        time.sleep(0.2)
        status = client.tool("get_run_status", {"run_id": agent_id})
        assert status["run"]["elapsed_seconds"] == frozen_seconds, "Contained timer kept running"
        assert status["run"]["score"] == frozen_score, "Contained score changed after completion"
        for name in ("validate_changes", "request_hint", "end_run"):
            client.expect_tool_error(name, {"run_id": agent_id})
        snapshot = cli(store, {"command": "snapshot"})
        public_values(snapshot, attack_ids)
        visible = {run["Id"]: run for run in snapshot["Runs"]}
        assert visible[human_id]["Status"] == "Active", "Agent containment ended the human incident"
        assert visible[agent_id]["Status"] == "Contained" and visible[agent_id]["Mode"] == "agent_practice"
        assert visible[agent_id]["Seconds"] == frozen_seconds and visible[agent_id]["Score"] == frozen_score
        assert cli(store, {"command": "end", "id": human_id})["Run"]["Status"] == "Abandoned"

    with MCP(store) as restarted:
        for arguments in ({}, {"run_id": agent_id}):
            status = restarted.tool("get_run_status", arguments)
            result = status["run"]
            assert result["id"] == agent_id and result["status"] == "Contained", status
            assert result["elapsed_seconds"] == frozen_seconds and result["score"] == frozen_score, status
            assert result["penalty_seconds"] == 35 and result["validation_attempts"] == 2, status
            assert result["hints"] == 1 and result["mode"] == "agent_practice", status
            public_values(status, attack_ids)
        restarted.ping()
    print("PASS: human/agent isolation, sanitized snapshots, containment, frozen score/timer, and persistence", flush=True)


def remove_readonly(function, path, _):
    # Git marks loose object files read-only on Windows.
    os.chmod(path, stat.S_IWRITE | stat.S_IREAD)
    function(path)


def main():
    assert ENGINE.is_file(), f"Build the packaged engine before running this test: {ENGINE}"
    temporary = pathlib.Path(tempfile.mkdtemp(prefix="sourcedefender-mcp-"))
    try:
        check_lifecycle(temporary)
        check_concurrent_start(temporary)
        check_containment_and_mode_isolation(temporary)
    finally:
        shutil.rmtree(temporary, onerror=remove_readonly)
    print("PASS: packaged MCP integration on", sys.platform)


if __name__ == "__main__":
    main()
