#!/usr/bin/env python3
"""Fail CI when workflow trust and supply-chain invariants regress."""

from __future__ import annotations

import re
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
WORKFLOWS = ROOT / ".github" / "workflows"
PINNED_ACTION = re.compile(r"^\s*-?\s*uses:\s*[^\s@]+@[0-9a-f]{40}(?:\s+#.*)?$")


def fail(message: str) -> None:
    raise SystemExit(f"workflow security policy failed: {message}")


def main() -> None:
    workflow_text: dict[str, str] = {}
    for workflow in sorted(WORKFLOWS.glob("*.yml")):
        text = workflow.read_text(encoding="utf-8")
        workflow_text[workflow.name] = text
        for number, line in enumerate(text.splitlines(), 1):
            if "uses:" in line and not PINNED_ACTION.match(line):
                fail(f"{workflow.name}:{number} action is not pinned to a full commit SHA")
        if re.search(r"curl[^\n|]*\|\s*(?:ba)?sh", text):
            fail(f"{workflow.name} pipes a network response into a shell")

    claude = workflow_text["claude.yml"]
    for required in (
        "author_association",
        "contents: read",
        "pull-requests: write",
        "issues: write",
    ):
        if required not in claude:
            fail(f"claude.yml is missing {required!r}")
    for forbidden in ("contents: write", "id-token: write", "@beta"):
        if forbidden in claude:
            fail(f"claude.yml contains forbidden permission or mutable ref {forbidden!r}")

    installer = (ROOT / ".github" / "scripts" / "install-wasm-pack.sh").read_text(
        encoding="utf-8"
    )
    for required in ("WASM_PACK_SHA256", "sha256sum --check --strict", "--proto '=https'"):
        if required not in installer:
            fail(f"wasm-pack installer is missing {required!r}")

    print(f"workflow security policy passed for {len(workflow_text)} workflows")


if __name__ == "__main__":
    main()
