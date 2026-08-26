#!/usr/bin/env python3
"""Fail CI when workflow trust and supply-chain invariants regress."""

from __future__ import annotations

import re
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
WORKFLOWS = ROOT / ".github" / "workflows"
PINNED_ACTION = re.compile(r"^\s*-?\s*uses:\s*[^\s@]+@[0-9a-f]{40}(?:\s+#.*)?$")
NETWORK_TO_SHELL = re.compile(r"\b(?:curl|wget)\b[^\n|]*\|\s*(?:ba|z)?sh\b")
CHECKSUM_MISMATCH_GUARD = re.compile(
    r"if\s+\[\[\s*\"\$\{actual_sha256\}\"\s*!=\s*"
    r"\"\$\{WASM_PACK_SHA256\}\"\s*\]\];\s*then\b"
    r".*?\bexit\s+[1-9][0-9]*\b.*?\bfi\b",
    re.DOTALL,
)


def fail(message: str) -> None:
    raise SystemExit(f"workflow security policy failed: {message}")


def workflow_paths() -> list[Path]:
    return sorted({*WORKFLOWS.glob("*.yml"), *WORKFLOWS.glob("*.yaml")})


def pipes_network_response_to_shell(text: str) -> bool:
    normalized = re.sub(r"(?:\\\s*)?\n\s*(?=\|)", " ", text)
    return NETWORK_TO_SHELL.search(normalized) is not None


def wasm_pack_installer_violations(text: str) -> list[str]:
    """Return missing portable checksum invariants for the wasm-pack installer."""
    required_tokens = {
        "declared expected SHA-256": "WASM_PACK_SHA256",
        "Linux SHA-256 calculation": 'sha256sum "${ARCHIVE_PATH}"',
        "macOS SHA-256 calculation": 'shasum -a 256 "${ARCHIVE_PATH}"',
        "HTTPS-only curl": "--proto '=https'",
    }
    violations = [
        description
        for description, token in required_tokens.items()
        if token not in text
    ]
    if CHECKSUM_MISMATCH_GUARD.search(text) is None:
        violations.append("digest mismatch failure guard")
    return violations


def main() -> None:
    workflow_text: dict[str, str] = {}
    for workflow in workflow_paths():
        text = workflow.read_text(encoding="utf-8")
        workflow_text[workflow.name] = text
        for number, line in enumerate(text.splitlines(), 1):
            if "uses:" in line and not PINNED_ACTION.match(line):
                fail(f"{workflow.name}:{number} action is not pinned to a full commit SHA")
        if pipes_network_response_to_shell(text):
            fail(f"{workflow.name} pipes a network response into a shell")

    for dangerous in (
        "curl https://example.invalid/install.sh | sh",
        "curl https://example.invalid/install.sh \\\n  | bash",
        "curl https://example.invalid/install.sh\n  | bash",
        "wget -qO- https://example.invalid/install.sh | zsh",
    ):
        if not pipes_network_response_to_shell(dangerous):
            fail("network-to-shell detector failed its synthetic bypass test")

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
    violations = wasm_pack_installer_violations(installer)
    if violations:
        fail(f"wasm-pack installer is missing {', '.join(violations)}")

    print(f"workflow security policy passed for {len(workflow_text)} workflows")


if __name__ == "__main__":
    main()
