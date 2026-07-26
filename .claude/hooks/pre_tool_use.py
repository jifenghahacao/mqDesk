from __future__ import annotations

import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from tooling import guardrails


def main() -> int:
    payload = json.load(sys.stdin)
    tool_name = payload.get("tool_name", "")
    tool_input = payload.get("tool_input", {})

    if tool_name == "Bash":
        decision = guardrails.evaluate_bash_command(tool_input.get("command", ""))
    elif tool_name in {"Write", "Edit", "MultiEdit"}:
        decision = guardrails.evaluate_file_write(tool_input.get("file_path", ""))
    else:
        decision = guardrails.Decision(False)

    if decision.blocked:
        print(decision.reason)
        return 2

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
