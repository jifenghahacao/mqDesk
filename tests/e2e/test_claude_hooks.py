import json
import os
import subprocess
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
PRE_TOOL = ROOT / ".claude" / "hooks" / "pre_tool_use.py"
POST_TOOL = ROOT / ".claude" / "hooks" / "post_tool_use.py"


class ClaudeHooksE2ETests(unittest.TestCase):
    def run_hook(self, script: Path, payload: dict, extra_env: dict | None = None) -> subprocess.CompletedProcess[str]:
        env = os.environ.copy()
        if extra_env:
            env.update(extra_env)
        return subprocess.run(
            [sys.executable, str(script)],
            input=json.dumps(payload),
            text=True,
            capture_output=True,
            cwd=ROOT,
            env=env,
            check=False,
        )

    def test_pre_tool_use_blocks_no_verify_commit(self) -> None:
        result = self.run_hook(
            PRE_TOOL,
            {
                "tool_name": "Bash",
                "tool_input": {"command": "git commit --no-verify -m test"},
            },
        )
        self.assertEqual(result.returncode, 2)
        self.assertIn("no-verify", result.stdout + result.stderr)

    def test_pre_tool_use_blocks_rm_rf(self) -> None:
        result = self.run_hook(
            PRE_TOOL,
            {
                "tool_name": "Bash",
                "tool_input": {"command": "rm -rf node_modules"},
            },
        )
        self.assertEqual(result.returncode, 2)
        self.assertIn("rm -rf", result.stdout + result.stderr)

    def test_pre_tool_use_blocks_linter_config_write(self) -> None:
        result = self.run_hook(
            PRE_TOOL,
            {
                "tool_name": "Write",
                "tool_input": {"file_path": "biome.json"},
            },
        )
        self.assertEqual(result.returncode, 2)
        self.assertIn("linter/hook config", result.stdout + result.stderr)

    def test_post_tool_use_injects_json_context_on_success(self) -> None:
        result = self.run_hook(
            POST_TOOL,
            {
                "tool_name": "Write",
                "tool_input": {"file_path": "src/app.jsx"},
            },
            extra_env={"MQDESK_HOOK_TEST_MODE": "success"},
        )
        self.assertEqual(result.returncode, 0)
        data = json.loads(result.stdout)
        context = json.loads(data["hookSpecificOutput"]["additionalContext"])
        self.assertEqual(context["status"], "ok")

    def test_post_tool_use_blocks_on_failed_validation(self) -> None:
        result = self.run_hook(
            POST_TOOL,
            {
                "tool_name": "Edit",
                "tool_input": {"file_path": "src/app.jsx"},
            },
            extra_env={"MQDESK_HOOK_TEST_MODE": "fail"},
        )
        self.assertEqual(result.returncode, 0)
        data = json.loads(result.stdout)
        self.assertEqual(data["decision"], "block")
        context = json.loads(data["hookSpecificOutput"]["additionalContext"])
        self.assertEqual(context["status"], "failed")


if __name__ == "__main__":
    unittest.main()
