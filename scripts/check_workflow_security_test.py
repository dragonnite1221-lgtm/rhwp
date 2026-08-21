from pathlib import Path
from tempfile import TemporaryDirectory
import unittest

from scripts import check_workflow_security as policy


class WorkflowSecurityPolicyTest(unittest.TestCase):
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


if __name__ == "__main__":
    unittest.main()
