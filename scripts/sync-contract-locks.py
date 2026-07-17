#!/usr/bin/env python3
"""Check standard local wkg manifests and exact dependency locks."""

from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
PACKAGES = ROOT / "crates/litehouse-contracts/packages"
DEPENDENCY = re.compile(
    r"use (litehouse:[a-z][a-z0-9-]*)/[a-z][a-z0-9-]*@"
    r"([0-9]+\.[0-9]+\.[0-9]+)\.\{[^}]+\};"
)


def package_roots() -> list[Path]:
    roots = sorted(PACKAGES.glob("*/1.0.0"))
    if len(roots) != 16:
        raise RuntimeError(f"expected 16 package roots, found {len(roots)}")
    return roots


def dependencies(root: Path) -> list[tuple[str, str]]:
    found: set[tuple[str, str]] = set()
    for wit in sorted((root / "wit").glob("*.wit")):
        for line_number, line in enumerate(wit.read_text().splitlines(), start=1):
            stripped = line.strip()
            if not stripped.startswith("use litehouse:"):
                continue
            match = DEPENDENCY.fullmatch(stripped)
            if match is None:
                raise RuntimeError(
                    f"unparsed WIT dependency {wit.relative_to(ROOT)}:{line_number}"
                )
            found.add((match.group(1), match.group(2)))
    return sorted(found)


def manifest(root: Path, deps: list[tuple[str, str]]) -> str:
    name = root.parent.name
    lines = [
        "[metadata]",
        'authors = ["Litehouse contributors"]',
        f'description = "Litehouse {name} contract"',
        'license = "MIT OR Apache-2.0"',
        'repository = "https://github.com/arlyon/litehouse"',
    ]
    if deps:
        lines.extend(["", "[overrides]"])
        for package, _ in deps:
            dependency_name = package.split(":", 1)[1]
            lines.append(
                f'"{package}" = {{ path = "../../{dependency_name}/1.0.0/wit" }}'
            )
    return "\n".join(lines) + "\n"


def lock(deps: list[tuple[str, str]], digests: dict[str, str]) -> str:
    lines = ["version = 1"]
    for package, version in deps:
        digest = digests.get(package)
        if digest is None:
            raise RuntimeError(f"missing ABI digest for {package}")
        lines.extend(
            [
                "",
                "[[packages]]",
                f'name = "{package}"',
                'registry = "local.litehouse.invalid"',
                "",
                "[[packages.versions]]",
                f'requirement = "={version}"',
                f'version = "{version}"',
                f'digest = "sha256:{digest}"',
            ]
        )
    return "\n".join(lines) + "\n"


def check_file(path: Path, expected: str) -> None:
    try:
        actual = path.read_text()
    except OSError as error:
        raise RuntimeError(f"cannot read {path.relative_to(ROOT)}: {error}") from error
    if actual != expected:
        raise RuntimeError(f"contract lock drift: {path.relative_to(ROOT)}")


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--artifacts", required=True, type=Path)
    arguments = parser.parse_args()

    try:
        artifacts = json.loads(arguments.artifacts.read_text())
    except (OSError, json.JSONDecodeError) as error:
        raise RuntimeError(
            f"cannot read artifact manifest {arguments.artifacts}: {error}"
        ) from error
    if not isinstance(artifacts, list):
        raise RuntimeError("artifact manifest must be a JSON array")
    digests = {artifact["package"]: artifact["abi_sha256"] for artifact in artifacts}
    if len(digests) != 16:
        raise RuntimeError(f"expected 16 artifact digests, found {len(digests)}")

    for root in package_roots():
        deps = dependencies(root)
        check_file(root / "wkg.toml", manifest(root, deps))
        check_file(root / "wkg.lock", lock(deps, digests))
    sys.stdout.write("contract locks valid (16 packages)\n")


if __name__ == "__main__":
    main()
