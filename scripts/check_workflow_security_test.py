from pathlib import Path
from tempfile import TemporaryDirectory
import unittest

from scripts import check_workflow_security as policy


class WorkflowSecurityPolicyTest(unittest.TestCase):
    PORTABLE_INSTALLER = """\
readonly WASM_PACK_SHA256="expected"
curl --proto '=https' --output "${ARCHIVE_PATH}" "${WASM_PACK_URL}"
if command -v sha256sum >/dev/null 2>&1; then
  actual_sha256="$(sha256sum "${ARCHIVE_PATH}" | awk '{print $1}')"
elif command -v shasum >/dev/null 2>&1; then
  actual_sha256="$(shasum -a 256 "${ARCHIVE_PATH}" | awk '{print $1}')"
fi
if [[ "${actual_sha256}" != "${WASM_PACK_SHA256}" ]]; then
  exit 1
fi
"""

    def test_scans_both_workflow_extensions(self) -> None:
        with TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "one.yml").write_text("name: one\n", encoding="utf-8")
            (root / "two.yaml").write_text("name: two\n", encoding="utf-8")
            (root / "ignored.txt").write_text("name: ignored\n", encoding="utf-8")
            original = policy.WORKFLOWS
            try:
                policy.WORKFLOWS = root
                self.assertEqual(
                    [path.name for path in policy.workflow_paths()],
                    ["one.yml", "two.yaml"],
                )
            finally:
                policy.WORKFLOWS = original

    def test_detects_single_line_and_multiline_network_to_shell(self) -> None:
        dangerous = (
            "curl https://example.invalid/install.sh | sh",
            "curl https://example.invalid/install.sh \\\n  | bash",
            "curl https://example.invalid/install.sh\n  | zsh",
            "wget -qO- https://example.invalid/install.sh | bash",
        )
        for script in dangerous:
            with self.subTest(script=script):
                self.assertTrue(policy.pipes_network_response_to_shell(script))

        self.assertFalse(policy.pipes_network_response_to_shell(
            "curl -o installer.sh https://example.invalid/install.sh\n"
            "sha256sum --check installer.sha256\n"
            "bash installer.sh\n"
        ))

    def test_accepts_portable_wasm_pack_checksum_verification(self) -> None:
        self.assertEqual(
            policy.wasm_pack_installer_violations(self.PORTABLE_INSTALLER),
            [],
        )

    def test_rejects_each_missing_wasm_pack_checksum_invariant(self) -> None:
        mutations = (
            ("WASM_PACK_SHA256", "EXPECTED_DIGEST", "declared expected SHA-256"),
            ("sha256sum", "sha512sum", "Linux SHA-256 calculation"),
            ("shasum -a 256", "shasum -a 512", "macOS SHA-256 calculation"),
            ("--proto '=https'", "--proto '=http'", "HTTPS-only curl"),
            (
                '"${actual_sha256}" !=',
                '"${actual_sha256}" ==',
                "digest mismatch failure guard",
            ),
            ("exit 1", "exit 0", "digest mismatch failure guard"),
        )
        for original, replacement, expected in mutations:
            with self.subTest(expected=expected):
                installer = self.PORTABLE_INSTALLER.replace(original, replacement)
                self.assertIn(
                    expected,
                    policy.wasm_pack_installer_violations(installer),
                )


if __name__ == "__main__":
    unittest.main()
