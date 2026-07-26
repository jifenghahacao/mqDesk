from __future__ import annotations

import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from tooling import checks, guardrails


def main() -> int:
    payload = json.load(sys.stdin)
    tool_name = payload.get("tool_name", "")
    tool_input = payload.get("tool_input", {})

    file_path = tool_input.get("file_path", "")
    if tool_name not in {"Write", "Edit", "MultiEdit"} or not file_path:
        print(json.dumps({"hookSpecificOutput": {"hookEventName": "PostToolUse", "additionalContext": json.dumps({"status": "skipped", "reason": "tool not handled"})}}))
        return 0

    results = checks.file_post_write_results(file_path)
    context = guardrails.build_additional_context(file_path, results)

    response: dict[str, object] = {
        "hookSpecificOutput": {
            "hookEventName": "PostToolUse",
            "additionalContext": context,
        }
    }
    if any(result.get("status") == "failed" for result in results):
        response["decision"] = "block"
        response["reason"] = f"post-write validation failed for {guardrails.normalize_path(file_path)}"

    print(json.dumps(response, ensure_ascii=False))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
