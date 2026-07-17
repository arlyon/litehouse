#!/usr/bin/env python3
"""Validate the non-publishing T0 workspace and generated-source baseline."""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
import re
import subprocess
import sys
import tempfile
import tomllib
import typing

ROOT = Path(__file__).resolve().parents[1]
LOCK_PATH = ROOT / "t0-baseline.lock.json"
SCHEMA_PATH = ROOT / "crates/registry/registry.fbs"
WIT_ROOT = ROOT / "crates/plugin-macro/wit"
WORKFLOW_ROOT = ROOT / ".github/workflows"
FLATC_INSTALLER = ROOT / "scripts/install-flatc.sh"
FORBIDDEN_WORKFLOW_MARKERS = {
    "secret context": "secrets.",
    "GitHub token context": "github.token",
    "GitHub token variable": "GITHUB_TOKEN",
    "npm publication token": "NPM_TOKEN",
    "container registry login": "docker/login-action",
    "container push": "push: true",
    "Docker push command": "docker push",
    "Podman push command": "podman push",
    "npm publication": "npm publish",
    "Cargo publication": "cargo publish",
    "cargo-dist command": "cargo dist",
    "GitHub release command": "gh release",
    "Git push command": "git push",
    "Wrangler deployment": "wrangler deploy",
    "floating latest image": ":latest",
}


def fail(message: str) -> typing.NoReturn:
    raise RuntimeError(message)


def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def tree_digest(root: Path, suffix: str) -> str:
    digest = hashlib.sha256(b"litehouse-t0-tree-v1\0")
    paths = sorted(path for path in root.rglob(f"*{suffix}") if path.is_file())
    if not paths:
        fail(f"no {suffix} files found under {root.relative_to(ROOT)}")

    for path in paths:
        relative = path.relative_to(ROOT).as_posix().encode()
        content = path.read_bytes()
        digest.update(len(relative).to_bytes(8, "big"))
        digest.update(relative)
        digest.update(len(content).to_bytes(8, "big"))
        digest.update(content)
    return digest.hexdigest()


def tracked_manifest_paths() -> list[Path]:
    result = subprocess.run(
        [
            "git",
            "ls-files",
            "-z",
            "--",
            "crates/*/Cargo.toml",
            "plugins/*/Cargo.toml",
        ],
        cwd=ROOT,
        check=True,
        capture_output=True,
    )
    return [ROOT / path.decode() for path in result.stdout.split(b"\0") if path]


def workspace_state() -> list[dict[str, str]]:
    root_manifest = tomllib.loads((ROOT / "Cargo.toml").read_text())
    workspace = root_manifest.get("workspace", {})
    if workspace.get("members") != ["crates/*", "plugins/*"]:
        fail("workspace members must remain exactly crates/* and plugins/*")
    if workspace.get("package", {}).get("edition") != "2024":
        fail("workspace.package.edition must be 2024")

    members: list[dict[str, str]] = []
    for path in tracked_manifest_paths():
        manifest = tomllib.loads(path.read_text())
        package = manifest.get("package", {})
        edition = package.get("edition")
        if edition == {"workspace": True}:
            edition_kind = "workspace"
        elif edition == "2024":
            edition_kind = "2024"
        else:
            relative = path.relative_to(ROOT)
            raise RuntimeError(f"{relative} must inherit or explicitly use Rust 2024")
        members.append(
            {
                "edition": edition_kind,
                "name": package["name"],
                "path": path.relative_to(ROOT).as_posix(),
            }
        )
    return sorted(members, key=lambda member: member["path"])


def workflow_action_state() -> list[dict[str, str]]:
    actions: set[tuple[str, str, str]] = set()
    for path in sorted(WORKFLOW_ROOT.glob("*.yml")):
        lines = path.read_text().splitlines()
        for index, line in enumerate(lines):
            stripped = line.strip()
            if stripped.startswith("- "):
                stripped = stripped[2:].lstrip()
            if not stripped.startswith("uses:"):
                continue
            value = stripped.partition(":")[2].strip()
            if value in {">", ">-", "|", "|-"}:
                value = next(
                    (
                        candidate.strip()
                        for candidate in lines[index + 1 :]
                        if candidate.strip()
                    ),
                    "",
                )
            if value.startswith("./"):
                continue
            action, separator, reference = value.rpartition("@")
            if not separator or not re.fullmatch(r"[0-9a-f]{40}", reference):
                fail(f"{path.relative_to(ROOT)} uses an unpinned action: {value!r}")
            actions.add((path.relative_to(ROOT).as_posix(), action, reference))
    return [
        {"action": action, "sha": reference, "workflow": workflow}
        for workflow, action, reference in sorted(actions)
    ]


def check_read_only_permissions(path: Path, lines: list[str]) -> None:
    permission_lines = [
        (index, len(line) - len(line.lstrip()))
        for index, line in enumerate(lines)
        if line.strip() == "permissions:"
    ]
    if len(permission_lines) != 1 or permission_lines[0][1] != 0:
        fail(f"{path.relative_to(ROOT)} needs one top-level permissions block")

    start = permission_lines[0][0]
    values: list[str] = []
    for line in lines[start + 1 :]:
        if not line.strip() or line.lstrip().startswith("#"):
            continue
        indent = len(line) - len(line.lstrip())
        if indent == 0:
            break
        values.append(line.strip())
    if values != ["contents: read"]:
        fail(f"{path.relative_to(ROOT)} permissions must be contents: read")


def check_workflow_authority() -> None:
    write_permission = re.compile(
        r"(?m)^\s+[A-Za-z][A-Za-z-]*:\s*(?:write|write-all)\s*$"
    )
    for path in sorted(WORKFLOW_ROOT.glob("*.yml")):
        text = path.read_text()
        check_read_only_permissions(path, text.splitlines())
        if write_permission.search(text):
            fail(f"{path.relative_to(ROOT)} contains write permission")
        for label, marker in FORBIDDEN_WORKFLOW_MARKERS.items():
            if marker in text:
                fail(f"{path.relative_to(ROOT)} contains forbidden {label}")

    for name in ("release.yml", "napi.yml"):
        guard = (WORKFLOW_ROOT / name).read_text()
        if '"on":\n  workflow_dispatch:' not in guard:
            fail(f"{name} must remain explicit workflow_dispatch only")
        if re.search(r"(?m)^\s{2}(?:pull_request|push):", guard):
            fail(f"{name} must not run from pull requests or Git pushes")

    rust = (WORKFLOW_ROOT / "rust.yml").read_text()
    if rust.count("toolchain: nightly-2026-07-15") != 2:
        fail("Rust CI must use the reviewed nightly-2026-07-15 toolchain")
    if rust.count("bash scripts/install-flatc.sh") != 2:
        fail("Rust test/build jobs must install the pinned flatc")
    if rust.count("python3 scripts/check-t0-baseline.py") != 3:
        fail("Rust CI must run one source and two full baseline checks")


def flatc_generated_digest(expected_version: str) -> str:
    version = subprocess.run(
        ["flatc", "--version"],
        cwd=ROOT,
        check=True,
        capture_output=True,
        text=True,
    ).stdout.strip()
    if version != f"flatc version {expected_version}":
        fail(f"expected flatc {expected_version}, found {version!r}")

    outputs: list[bytes] = []
    for _ in range(2):
        with tempfile.TemporaryDirectory(prefix="litehouse-flatc-") as temp:
            subprocess.run(
                ["flatc", "--rust", "-o", temp, str(SCHEMA_PATH)],
                cwd=ROOT,
                check=True,
            )
            outputs.append((Path(temp) / "registry_generated.rs").read_bytes())
    if outputs[0] != outputs[1]:
        fail("flatc produced different bytes from identical inputs")
    return sha256(outputs[0])


def collect_state(flatc_version: str, skip_flatc: bool) -> dict[str, object]:
    state: dict[str, object] = {
        "ci_policy_sha256": sha256(Path(__file__).read_bytes()),
        "flatc_installer_sha256": sha256(FLATC_INSTALLER.read_bytes()),
        "flatc_version": flatc_version,
        "registry_schema_sha256": sha256(SCHEMA_PATH.read_bytes()),
        "tracked_workspace_members": workspace_state(),
        "wit_tree_sha256": tree_digest(WIT_ROOT, ".wit"),
        "workflow_actions": workflow_action_state(),
        "workflow_tree_sha256": tree_digest(WORKFLOW_ROOT, ".yml"),
    }
    if not skip_flatc:
        state["registry_generated_sha256"] = flatc_generated_digest(flatc_version)
    return state


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--skip-flatc",
        action="store_true",
        help="check locked sources without invoking the pinned flatc binary",
    )
    parser.add_argument(
        "--print-lock",
        action="store_true",
        help="print reviewed lock JSON for explicit shell redirection",
    )
    args = parser.parse_args()

    check_workflow_authority()

    if args.print_lock:
        if args.skip_flatc:
            fail("--print-lock requires the pinned flatc binary")
        state = collect_state("25.12.19", skip_flatc=False)
        print(json.dumps(state, indent=2, sort_keys=True))
        return 0

    if not LOCK_PATH.exists():
        fail("missing t0-baseline.lock.json; generate it after review")
    try:
        expected = json.loads(LOCK_PATH.read_text())
    except (OSError, json.JSONDecodeError) as error:
        fail(f"cannot read T0 lock: {error}")
    actual = collect_state(expected["flatc_version"], args.skip_flatc)
    if args.skip_flatc:
        expected = {
            key: value
            for key, value in expected.items()
            if key != "registry_generated_sha256"
        }
    if actual != expected:
        print("expected:", json.dumps(expected, indent=2, sort_keys=True))
        print("actual:", json.dumps(actual, indent=2, sort_keys=True))
        fail("T0 baseline differs from its reviewed lock")

    mode = "source-only" if args.skip_flatc else "full"
    print(f"T0 baseline valid ({mode})")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, RuntimeError, subprocess.CalledProcessError) as error:
        print(f"error: {error}", file=sys.stderr)
        raise SystemExit(1) from error
