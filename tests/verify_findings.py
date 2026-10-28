"""Exercise public finding disclosure through the packaged engine's JSON CLI.

Only the engine and Git run. Generated project source, hooks, and tasks never run.
"""

import argparse
from collections import Counter
import json
import os
from pathlib import Path, PurePosixPath
import shutil
import stat
import subprocess
import tempfile


ROOT = Path(__file__).resolve().parents[1]
ENGINE = ROOT / ("SourceDefender.Engine.exe" if os.name == "nt" else "SourceDefender.Engine")
TIMEOUT = 45
PRIVATE_KEYS = {"broken", "revealed", "requiredfiles", "templateversion", "attackids", "unsafe", "safe", "policy", "check"}
# Stable player-facing control names form the expected disclosure contract. The
# actual selected controls come from the isolated private record, never the CLI.
CONTROLS = {
    "auth": ("Magic-token authentication backdoor", "Application"),
    "telemetry": ("Credential theft disguised as telemetry", "Application"),
    "billing": ("Payment destination replaced", "Application"),
    "audit": ("Attacker activity hidden from audit trail", "Application"),
    "roles": ("Hidden account privilege escalation", "Application"),
    "updates": ("Update feed redirected to attacker", "Supply chain"),
    "exports": ("Record exports copied to an outsider", "Application"),
    "plugins": ("Remote payload loader introduced", "Supply chain"),
    "install": ("Malicious dependency install hook", "Supply chain"),
    "registry": ("Dependency registry hijack", "Supply chain"),
    "workflow": ("CI uploads private credentials", "Automation"),
    "ownership": ("Attacker seized review ownership", "Repository"),
    "ignore": ("Payload hidden from ordinary Git review", "Repository"),
    "deploy": ("Deployment target silently redirected", "Automation"),
    "trust": ("Attacker signing key added to trust store", "Repository"),
    "schedule": ("Persistence through scheduled automation", "Automation"),
}


def public_values(value, attack_ids):
    if isinstance(value, dict):
        for key, item in value.items():
            normalized = "".join(c for c in key.casefold() if c.isalnum())
            assert normalized not in PRIVATE_KEYS, f"Private metadata leaked: {key}"
            public_values(item, attack_ids)
    elif isinstance(value, list):
        for item in value:
            public_values(item, attack_ids)
    elif isinstance(value, str):
        assert value not in attack_ids, f"Private attack ID leaked: {value}"


def cli(engine, store, request, attack_ids=()):
    process = subprocess.run(
        [str(engine)], cwd=ROOT,
        env=dict(os.environ, SOURCEDEFENDER_DATA_DIR=str(store)),
        input=json.dumps(request, ensure_ascii=False), capture_output=True,
        text=True, encoding="utf-8", timeout=TIMEOUT,
    )
    response = json.loads(process.stdout)
    assert process.returncode == 0 and response.get("ok") is True, (response, process.stderr)
    public_values(response, attack_ids)
    return response["data"]


def git(project, *arguments):
    return subprocess.check_output(
        ["git", "-c", "core.hooksPath=/dev/null", "-c", "core.quotepath=false", "-C", str(project), *arguments],
        text=True, encoding="utf-8", stderr=subprocess.PIPE, timeout=TIMEOUT,
    ).strip()


def snapshot_run(engine, store, identifier, attack_ids):
    snapshot = cli(engine, store, {"command": "snapshot"}, attack_ids)
    runs = snapshot["Runs"]
    assert len(runs) == 1 and runs[0]["Id"] == identifier, runs
    return runs[0]


def finding_shape(findings):
    assert isinstance(findings, list), findings
    for finding in findings:
        assert set(finding) == {"Title", "Category", "Path", "Count"}, finding
        assert isinstance(finding["Title"], str) and finding["Title"], finding
        assert isinstance(finding["Category"], str) and finding["Category"], finding
        assert type(finding["Count"]) is int and finding["Count"] > 0, finding


def disclosure(level, response, selected, paths):
    findings = response["Run"]["Findings"]
    finding_shape(findings)
    expected = [CONTROLS[identifier] for identifier in selected]
    assert sum(finding["Count"] for finding in findings) == len(selected), findings
    if level <= 1:
        assert len(findings) == len(selected), findings
        assert Counter((f["Title"], f["Category"]) for f in findings) == Counter(expected), findings
        assert all(f["Count"] == 1 for f in findings), findings
        if level == 0:
            assert {f["Path"] for f in findings} == paths, findings
        else:
            assert all(f["Path"] is None for f in findings), findings
    elif level == 2:
        categories = Counter(category for _, category in expected)
        assert len(findings) == len(categories), findings
        assert {f["Category"]: f["Count"] for f in findings} == categories, findings
        assert all(f["Title"] == f["Category"] + " controls unresolved" and f["Path"] is None for f in findings), findings
    else:
        assert findings == [{"Title": f"{len(selected)} controls unresolved", "Category": "Validation", "Path": None, "Count": len(selected)}], findings
    # Reports and structured findings must obey the same information limits.
    exposed = json.dumps({"Report": response["Report"], "Findings": findings}, ensure_ascii=False)
    if level >= 1:
        assert not any(path in exposed for path in paths), exposed
    if level >= 2:
        assert not any(title in exposed for title, _ in CONTROLS.values()), exposed
    if level == 3:
        assert not any(category in exposed for category in {category for _, category in CONTROLS.values()}), exposed


def check_level(engine, root, level):
    store, projects = root / f"records-{level}", root / f"projects-{level}"
    projects.mkdir()
    started = cli(engine, store, {
        "command": "generate", "scenario": level, "language": "Python", "level": level,
        "seed": "structured-findings-contract", "folder": str(projects),
    })
    run = started["Run"]
    identifier, project = run["Id"], Path(run["Folder"])
    private = json.loads((store / (identifier + ".json")).read_text(encoding="utf-8-sig"))
    selected = private["Broken"]
    assert len(selected) == (3, 6, 10, 16)[level] and set(selected) <= CONTROLS.keys(), selected
    public_values(started, selected)
    assert run["Findings"] == [] and run["Hints"] == run["Attempts"] == run["PenaltySeconds"] == 0, run
    baseline = git(project, "rev-list", "--max-parents=0", "HEAD")
    assert baseline and "\n" not in baseline, "Expected one trusted baseline commit"
    paths = set(git(project, "diff", "--name-only", baseline, "HEAD", "--").splitlines())
    assert len(paths) == len(selected), (paths, selected)

    failed = cli(engine, store, {"command": "validate", "id": identifier}, selected)
    assert failed["Run"]["Status"] == "Active" and failed["Run"]["Attempts"] == 1, failed
    assert failed["Run"]["PenaltySeconds"] == (15, 45, 90, 180)[level], failed
    disclosure(level, failed, selected, paths)
    persisted = snapshot_run(engine, store, identifier, selected)
    assert persisted["Findings"] == failed["Run"]["Findings"], persisted
    # Before a paid hint, even the complete public record must withhold details.
    public_text = json.dumps(persisted, ensure_ascii=False)
    if level >= 1:
        assert not any(path in public_text for path in paths), public_text
    if level >= 2:
        assert not any(title in public_text for title, _ in CONTROLS.values()), public_text

    hinted = cli(engine, store, {"command": "hint", "id": identifier}, selected)
    hint = hinted["Run"]
    assert hint["Hints"] == 1 and hint["Attempts"] == 1, hinted
    expected_penalty = (15, 45, 90, 180)[level] + (20, 45, 75, 120)[level]
    assert hint["PenaltySeconds"] == expected_penalty, hinted
    finding_shape(hint["Findings"])
    assert len(hint["Findings"]) == 1, hint
    revealed = hint["Findings"][0]
    assert (revealed["Title"], revealed["Category"]) in [CONTROLS[item] for item in selected], revealed
    assert revealed["Count"] == 1 and revealed["Path"] in paths, revealed
    hint_path = project.joinpath(*PurePosixPath(revealed["Path"]).parts).resolve()
    assert project.resolve() in hint_path.parents and hint_path.is_file(), revealed
    assert revealed["Path"] in hinted["Report"] and revealed["Title"] in hinted["Report"], hinted
    assert snapshot_run(engine, store, identifier, selected)["Findings"] == hint["Findings"]

    # Returning to a normal scan must replace hint details with that level's
    # disclosure policy. The event log may retain the previously paid-for hint.
    rescanned = cli(engine, store, {"command": "validate", "id": identifier}, selected)
    disclosure(level, rescanned, selected, paths)
    penalty = expected_penalty + (15, 45, 90, 180)[level]
    assert rescanned["Run"]["PenaltySeconds"] == penalty, rescanned

    git(project, "restore", "--source", baseline, "--", ".")
    contained = cli(engine, store, {"command": "validate", "id": identifier}, selected)
    final = contained["Run"]
    assert final["Status"] == "Contained" and final["Findings"] == [], contained
    assert final["Attempts"] == 3 and final["Hints"] == 1 and final["PenaltySeconds"] == penalty, final
    persisted = snapshot_run(engine, store, identifier, selected)
    assert persisted["Status"] == "Contained" and persisted["Findings"] == [], persisted
    print(f"PASS: difficulty {level} structured findings, disclosure limits, charged hint, persistence, and empty successful scan", flush=True)


def remove_readonly(function, path, _):
    os.chmod(path, stat.S_IWRITE | stat.S_IREAD)
    function(path)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--engine", type=Path, default=ENGINE)
    args = parser.parse_args()
    engine = args.engine.resolve()
    assert engine.is_file(), f"Build the packaged engine before running this test: {engine}"
    assert shutil.which("git"), "Git is required to restore the trusted fixture without executing project code"
    temporary = Path(tempfile.mkdtemp(prefix="sourcedefender-findings-"))
    try:
        for level in range(4):
            check_level(engine, temporary, level)
    finally:
        shutil.rmtree(temporary, onerror=remove_readonly)
    print("PASS: JSON CLI structured findings across all four difficulties")


if __name__ == "__main__":
    main()
