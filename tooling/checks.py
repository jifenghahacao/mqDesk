from __future__ import annotations

import argparse
import json
import os
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from tooling import guardrails

BIOME_FILE_SUFFIXES = {".js", ".jsx", ".json", ".jsonc", ".css"}
OXLINT_FILE_SUFFIXES = {".js", ".jsx"}


def command_exists(command: str) -> bool:
    return shutil.which(command) is not None


def make_result(name: str, command: str, status: str, exit_code: int = 0, detail: str = "") -> dict:
    result = {
        "name": name,
        "command": command,
        "status": status,
        "exitCode": exit_code,
    }
    if detail:
        result["detail"] = detail.strip()
    return result


def run_command(name: str, command: list[str], cwd: Path | None = None) -> dict:
    try:
        process = subprocess.run(
            command,
            cwd=cwd or ROOT,
            text=True,
            encoding="utf-8",
            errors="replace",
            capture_output=True,
            check=False,
        )
    except FileNotFoundError as error:
        return make_result(name, " ".join(command), "failed", 127, str(error))
    detail = ((process.stdout or "") + (process.stderr or "")).strip()
    status = "ok" if process.returncode == 0 else "failed"
    return make_result(name, " ".join(command), status, process.returncode, detail)


def skip_result(name: str, command: str, reason: str) -> dict:
    return make_result(name, command, "skipped", 0, reason)


def fail_result(name: str, command: str, reason: str, exit_code: int = 127) -> dict:
    return make_result(name, command, "failed", exit_code, reason)


def npm_exec(*args: str) -> list[str]:
    npm_binary = "npm.cmd" if os.name == "nt" else "npm"
    return [npm_binary, "exec", "--", *args]


def npm_run(*args: str) -> list[str]:
    npm_binary = "npm.cmd" if os.name == "nt" else "npm"
    return [npm_binary, "run", *args]


def file_post_write_results(file_path: str) -> list[dict]:
    test_mode = os.environ.get("MQDESK_HOOK_TEST_MODE")
    if test_mode == "success":
        return [
            make_result("formatter", "biome format", "ok"),
            make_result("linter", "oxlint", "ok"),
            make_result("typecheck", "esbuild", "ok"),
        ]
    if test_mode == "fail":
        return [
            make_result("formatter", "biome format", "ok"),
            make_result("linter", "oxlint", "failed", 1, "simulated lint failure"),
            make_result("typecheck", "esbuild", "skipped", 0, "stopped after linter failure"),
        ]

    path = ROOT / guardrails.normalize_path(file_path)
    suffix = path.suffix.lower()
    results: list[dict] = []

    if suffix in BIOME_FILE_SUFFIXES:
        results.append(run_command("formatter", npm_exec("biome", "format", "--write", str(path))))
    else:
        results.append(skip_result("formatter", "n/a", f"no formatter configured for {suffix or 'no-extension'}"))

    if results[-1]["status"] == "failed":
        results.append(skip_result("linter", "n/a", "stopped after formatter failure"))
        results.append(skip_result("typecheck", "n/a", "stopped after formatter failure"))
        return results

    if suffix in OXLINT_FILE_SUFFIXES:
        results.append(run_command("linter", npm_exec("oxlint", str(path))))
    else:
        results.append(skip_result("linter", "n/a", f"no linter configured for {suffix or 'no-extension'}"))

    if results[-1]["status"] == "failed":
        results.append(skip_result("typecheck", "n/a", "stopped after linter failure"))
        return results

    if suffix in OXLINT_FILE_SUFFIXES:
        results.append(run_esbuild_check(path))
    else:
        results.append(skip_result("typecheck", "n/a", f"no typecheck configured for {suffix or 'no-extension'}"))

    return results


def run_esbuild_check(path: Path) -> dict:
    with tempfile.TemporaryDirectory() as temp_dir:
        outfile = Path(temp_dir) / f"{path.stem}.js"
        command = npm_exec(
            "esbuild",
            str(path),
            "--platform=browser",
            "--format=esm",
            "--loader:.js=jsx",
            "--log-level=error",
            f"--outfile={outfile}",
        )
        return run_command("typecheck", command)


def parse_file_args(raw_files: list[str] | None) -> list[str]:
    if not raw_files:
        return []
    return [guardrails.normalize_path(item) for item in raw_files if item]


def run_lint(profile: str, files: list[str] | None = None) -> int:
    commands = []
    normalized_files = parse_file_args(files)

    if profile == "precommit":
        biome_targets = [item for item in normalized_files if Path(item).suffix.lower() in BIOME_FILE_SUFFIXES]
        oxlint_targets = [item for item in normalized_files if Path(item).suffix.lower() in OXLINT_FILE_SUFFIXES]
        if biome_targets:
            commands.append(run_command("biome-check", npm_exec("biome", "check", *biome_targets)))
        else:
            commands.append(skip_result("biome-check", "biome check", "no staged Biome-supported files"))
        if oxlint_targets:
            commands.append(run_command("oxlint", npm_exec("oxlint", *oxlint_targets)))
        else:
            commands.append(skip_result("oxlint", "oxlint", "no staged JS/JSX files"))
    elif profile == "ci":
        commands.append(run_command("biome-check", npm_exec("biome", "check", ".")))
        commands.append(run_command("oxlint", npm_exec("oxlint", "src")))
        commands.append(run_command("cargo-clippy", ["cargo", "clippy", "--", "-D", "warnings"], cwd=ROOT / "src-tauri"))
    else:
        raise ValueError(f"unsupported lint profile: {profile}")
    return summarize(commands)


def run_typecheck(profile: str, files: list[str] | None = None) -> int:
    commands = []
    normalized_files = parse_file_args(files)

    if profile == "precommit":
        js_targets = [item for item in normalized_files if Path(item).suffix.lower() in OXLINT_FILE_SUFFIXES]
        if js_targets:
            commands.append(run_frontend_syntax_check(js_targets))
        else:
            commands.append(skip_result("frontend-syntax", "esbuild", "no staged JS/JSX files"))
    elif profile == "ci":
        commands.append(run_frontend_syntax_check())
        commands.append(run_command("cargo-check", ["cargo", "check"], cwd=ROOT / "src-tauri"))
    else:
        raise ValueError(f"unsupported typecheck profile: {profile}")
    return summarize(commands)


def run_tests(profile: str) -> int:
    if profile == "unit":
        results = [
            run_command("frontend-unit", npm_run("test")),
            # 仅运行核心库单元测试；Tauri 壳层测试需 WebView2，integration/smoke 测试需真实 RabbitMQ
            run_command("rust-unit", ["cargo", "test", "-p", "mqdesk-core", "--lib"], cwd=ROOT / "src-tauri"),
        ]
        return summarize(results)
    if profile == "e2e":
        return subprocess.call([sys.executable, "-m", "unittest", "discover", "-s", "tests/e2e", "-t", ".", "-p", "test_*.py"], cwd=ROOT)
    raise ValueError(f"unsupported test profile: {profile}")


def run_build() -> int:
    results = [
        run_command("frontend-build", npm_run("build")),
        run_command("rust-build", ["cargo", "build"], cwd=ROOT / "src-tauri"),
    ]
    return summarize(results)


def run_frontend_syntax_check(files: list[str] | None = None) -> dict:
    if files:
        source_files = [ROOT / item for item in files]
    else:
        source_files = sorted((ROOT / "src").rglob("*.js")) + sorted((ROOT / "src").rglob("*.jsx"))
    if not source_files:
        return skip_result("frontend-syntax", "esbuild", "no frontend source files found")

    with tempfile.TemporaryDirectory() as temp_dir:
        for source_file in source_files:
            outfile = Path(temp_dir) / f"{source_file.stem}.js"
            result = run_command(
                "frontend-syntax",
                npm_exec(
                    "esbuild",
                    str(source_file),
                    "--platform=browser",
                    "--format=esm",
                    "--loader:.js=jsx",
                    "--log-level=error",
                    f"--outfile={outfile}",
                ),
            )
            if result["status"] == "failed":
                return result

    return make_result("frontend-syntax", "esbuild src/**/*.js src/**/*.jsx", "ok", 0, f"checked {len(source_files)} files")


def run_guard() -> int:
    for step in (run_build, lambda: run_lint("ci"), lambda: run_typecheck("ci"), lambda: run_tests("unit"), lambda: run_tests("e2e")):
        exit_code = step()
        if exit_code != 0:
            return exit_code
    return 0


def run_verify() -> int:
    return run_guard()


def install_lefthook() -> int:
    if not (ROOT / ".git").exists():
        print("Skipping Lefthook install: repository has no .git directory.")
        return 0
    return subprocess.call(["npm", "exec", "--", "lefthook", "install"], cwd=ROOT)


def summarize(results: list[dict]) -> int:
    failed = [result for result in results if result.get("status") == "failed"]
    for result in results:
        print(json.dumps(result, ensure_ascii=False))
    return 1 if failed else 0


def main() -> int:
    parser = argparse.ArgumentParser()
    subparsers = parser.add_subparsers(dest="command", required=True)

    lint_parser = subparsers.add_parser("lint")
    lint_parser.add_argument("--profile", required=True)
    lint_parser.add_argument("--files", nargs="*")

    typecheck_parser = subparsers.add_parser("typecheck")
    typecheck_parser.add_argument("--profile", required=True)
    typecheck_parser.add_argument("--files", nargs="*")

    test_parser = subparsers.add_parser("test")
    test_parser.add_argument("--profile", required=True)

    post_write_parser = subparsers.add_parser("post-write")
    post_write_parser.add_argument("--file", required=True)

    subparsers.add_parser("build")
    subparsers.add_parser("guard")
    subparsers.add_parser("verify")
    subparsers.add_parser("install-hooks")

    args = parser.parse_args()

    if args.command == "lint":
        return run_lint(args.profile, args.files)
    if args.command == "typecheck":
        return run_typecheck(args.profile, args.files)
    if args.command == "test":
        return run_tests(args.profile)
    if args.command == "build":
        return run_build()
    if args.command == "guard":
        return run_guard()
    if args.command == "verify":
        return run_verify()
    if args.command == "install-hooks":
        return install_lefthook()
    if args.command == "post-write":
        print(guardrails.build_additional_context(args.file, file_post_write_results(args.file)))
        return 0

    return 1


if __name__ == "__main__":
    raise SystemExit(main())
