from __future__ import annotations

import json
import re
from dataclasses import dataclass
from pathlib import PurePath


@dataclass(frozen=True)
class Decision:
    blocked: bool
    reason: str = ""


DESTRUCTIVE_COMMAND_PATTERNS = (
    (re.compile(r"\brm\s+-rf\b", re.IGNORECASE), "rm -rf"),
    (re.compile(r"\bdrop\s+table\b", re.IGNORECASE), "drop table"),
    (re.compile(r"\bdrop\s+database\b", re.IGNORECASE), "drop database"),
    (re.compile(r"\btruncate\s+table\b", re.IGNORECASE), "truncate table"),
)

NO_VERIFY_PATTERN = re.compile(r"(^|\s)git\s+commit\b.*\s--no-verify(\s|$)", re.IGNORECASE)

PROTECTED_LINTER_PATHS = {
    "biome.json",
    "biome.jsonc",
    "lefthook.yml",
    ".eslintrc",
    ".eslintrc.js",
    ".eslintrc.cjs",
    ".eslintrc.json",
    "oxlint.json",
    ".oxlintrc.json",
    ".claude/settings.json",
}


def normalize_path(path: str) -> str:
    normalized = str(PurePath(path)).replace("\\", "/")
    if normalized.startswith("./"):
        normalized = normalized[2:]
    return normalized


def evaluate_bash_command(command: str) -> Decision:
    if NO_VERIFY_PATTERN.search(command):
        return Decision(True, "blocked: git commit --no-verify bypasses repository hooks")

    for pattern, label in DESTRUCTIVE_COMMAND_PATTERNS:
        if pattern.search(command):
            return Decision(True, f"blocked destructive command: {label}")

    return Decision(False)


def evaluate_file_write(file_path: str) -> Decision:
    normalized = normalize_path(file_path)
    if normalized in PROTECTED_LINTER_PATHS:
        return Decision(True, f"blocked write to linter/hook config: {normalized}")
    return Decision(False)


def build_additional_context(file_path: str, results: list[dict]) -> str:
    status = "failed" if any(result.get("status") == "failed" for result in results) else "ok"
    payload = {
        "file": normalize_path(file_path),
        "status": status,
        "results": results,
    }
    return json.dumps(payload, ensure_ascii=False)
