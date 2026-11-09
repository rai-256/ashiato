#!/usr/bin/env python3
"""Run osv-scanner and apply the repository's exception table."""
from __future__ import annotations

import datetime as dt
import json
import os
import subprocess
import tomllib
import tempfile
from pathlib import Path


class ExceptionTableError(ValueError):
    """An exception row violates the repository's rules."""


def error(message: str) -> int:
    print(f"error: 検査できなかった（{message}）")
    return 2


def lockfiles(root: Path) -> list[Path]:
    paths = [root / "Cargo.lock", root / "web/package-lock.json"]
    result = subprocess.run(
        ["git", "-C", str(root), "ls-files", "collector-android/**.lockfile", "collector-android/*.lockfile"],
        text=True, capture_output=True,
    )
    if result.returncode != 0:
        return paths + [root / "collector-android/gradle.lockfile"]
    gradle = [root / line for line in result.stdout.splitlines()]
    if not gradle:
        raise ValueError("携帯の gradle.lockfile が無い")
    return paths + gradle


def raw_exception_rows(path: Path) -> dict[str, dict[str, str]]:
    try:
        with path.open("rb") as f:
            config = tomllib.load(f)
    except (OSError, tomllib.TOMLDecodeError) as exc:
        raise ValueError(f"例外の表: {exc}") from exc
    rows: dict[str, dict[str, str]] = {}
    for row in config.get("IgnoredVulns", []):
        ident = row.get("id") if isinstance(row, dict) else None
        key = ident if isinstance(ident, str) else ""
        if key in rows:
            # 同じ ID の行が 2 つあると、後の行が前の行（期限切れ・理由の無い行）を隠す。行の問題として落とす。
            rows[key]["duplicate"] = "1"
            continue
        rows[key] = {
            "id": key,
            "ignoreUntil": _text(row.get("ignoreUntil", "")) if isinstance(row, dict) else "",
            "reason": _text(row.get("reason", "")) if isinstance(row, dict) else "",
        }
    return rows


def _text(value: object) -> str:
    """TOML の値を文字列で持つ。引用符の無い日付（TOML の日付型）も文字列の日付と同じに扱う。"""
    if isinstance(value, dt.date) and not isinstance(value, dt.datetime):
        return value.isoformat()
    return value if isinstance(value, str) else str(value)


def exception_row_error(ident: str, row: dict[str, str]) -> str | None:
    until = row["ignoreUntil"]
    reason = row["reason"]
    if not ident:
        return "勧告の ID が無い"
    if row.get("duplicate"):
        return f"ID が重複 ({ident})"
    if not reason:
        return f"理由が無い ({ident})"
    if not until:
        return f"期限が無い ({ident})"
    try:
        expiry = dt.date.fromisoformat(until)
    except ValueError:
        return f"期限が不正 ({ident})"
    today = dt.datetime.now(dt.timezone.utc).date()
    if expiry < today:
        return f"期限切れ ({ident})"
    if expiry > today + dt.timedelta(days=90):
        return f"期限が 90 日を越える ({ident})"
    if not reason.startswith(("届かない: ", "上げる予定: ")):
        return f"理由の形が違う ({ident})"
    return None


def exception_rows(path: Path, *, validate: bool = True) -> dict[str, dict[str, str]]:
    rows = raw_exception_rows(path)
    if not validate:
        return rows
    for ident, row in rows.items():
        problem = exception_row_error(ident, row)
        if problem:
            raise ExceptionTableError(f"例外の表: {problem}")
    return rows


def exceptions(path: Path) -> set[str]:
    return set(exception_rows(path))


def allow_ghsas(root: Path) -> int:
    try:
        rows = exception_rows(root / "osv-scanner.toml")
    except (ExceptionTableError, ValueError) as exc:
        print(f"error: {exc}", file=os.sys.stderr)
        return 2

    allowed: list[str] = []
    for ident in rows:
        if ident.startswith("GHSA-"):
            allowed.append(ident)
            continue
        url = "https://api.osv.dev/v1/vulns/" + ident
        proc = subprocess.run(
            ["curl", "--fail", "--location", "--silent", "--show-error", url],
            text=True, capture_output=True,
        )
        if proc.returncode != 0:
            detail = proc.stderr.strip()
            suffix = f": {detail}" if detail else ""
            print(f"error: 許可の一覧を組めない（osv.dev への問い合わせ{suffix}）", file=os.sys.stderr)
            return 2
        try:
            aliases = json.loads(proc.stdout)["aliases"]
        except (KeyError, TypeError, json.JSONDecodeError) as exc:
            print(f"error: 許可の一覧を組めない（osv.dev の応答: {exc}）", file=os.sys.stderr)
            return 2
        if not isinstance(aliases, list):
            print("error: 許可の一覧を組めない（osv.dev の aliases が不正）", file=os.sys.stderr)
            return 2
        allowed.extend(alias for alias in aliases if isinstance(alias, str) and alias.startswith("GHSA-"))
    print(",".join(dict.fromkeys(allowed)))
    return 0


def scan(scanner: Path, root: Path, lock: Path) -> tuple[int, list[dict], str | None]:
    command = [str(scanner), "scan", "source", "--lockfile", str(lock), "--format", "json",
               "--all-vulns", "--all-packages", "--config", "/dev/null"]
    proc = subprocess.run(command, cwd=root, text=True, capture_output=True)
    if proc.returncode not in (0, 1):
        detail = proc.stderr.strip()
        if detail:
            return 2, [], f"osv.dev への問い合わせ: {detail}"
        return 2, [], "osv-scanner の実行"
    try:
        data = json.loads(proc.stdout)
    except json.JSONDecodeError as exc:
        return 2, [], f"JSON の読み取り: {exc}"
    findings: list[dict] = []
    for result in data.get("results", []):
        for package in result.get("packages", []):
            metadata = package.get("package", {})
            for group in package.get("groups", []):
                ids = group.get("ids", [])
                for ident in ids:
                    findings.append({
                        "ecosystem": metadata.get("ecosystem", ""),
                        "name": metadata.get("name", ""),
                        "version": metadata.get("version", ""),
                        "id": ident,
                        "aliases": group.get("aliases", []),
                        "severity": group.get("max_severity", ""),
                    })
    return proc.returncode, findings, None


def scan_root(scanner: Path, root: Path, files: list[Path], require_all: bool) -> tuple[list[dict], str | None]:
    if require_all:
        missing = [p for p in files if not p.is_file()]
        if missing:
            return [], "見つからないロックファイル: " + ", ".join(str(p.relative_to(root)) for p in missing)
    findings: list[dict] = []
    for path in files:
        rc, rows, failure = scan(scanner, root, path)
        if failure:
            return [], failure
        findings.extend(rows)
    return findings, None


def git_file(root: Path, revision: str, relative: str) -> bytes | None:
    proc = subprocess.run(["git", "-C", str(root), "show", f"{revision}:{relative}"], capture_output=True)
    return proc.stdout if proc.returncode == 0 else None


def resolve_base(root: Path, revision: str) -> str | None:
    proc = subprocess.run(["git", "-C", str(root), "rev-parse", "--verify", f"{revision}^{{commit}}"], text=True, capture_output=True)
    return proc.stdout.strip() if proc.returncode == 0 else None


def finding_key(row: dict) -> tuple[str, str, str]:
    return row["ecosystem"], row["name"], row["id"]


def _row_state(row: dict[str, str]) -> tuple[str | None, ...]:
    return row.get("ignoreUntil"), row.get("reason"), row.get("duplicate")


def print_exception_changes(base: dict[str, dict[str, str]], head: dict[str, dict[str, str]]) -> None:
    print("== 例外の表で足した・変えた行")
    changed = [head[ident] for ident in sorted(head)
               if ident not in base or _row_state(base[ident]) != _row_state(head[ident])]
    if not changed:
        print("なし")
    else:
        for row in changed:
            print(f"{row['id']} ignoreUntil={row['ignoreUntil']} reason={row['reason']}"
                  + (" （ID が重複）" if row.get("duplicate") else ""))
    summary = os.environ.get("GITHUB_STEP_SUMMARY")
    if summary:
        with open(summary, "a", encoding="utf-8") as out:
            out.write("\n== 例外の表で足した・変えた行\n")
            out.write("なし\n" if not changed else "".join(
                f"{row['id']} ignoreUntil={row['ignoreUntil']} reason={row['reason']}\n" for row in changed))


def main() -> int:
    root = Path(os.environ.get("CHECK_VULNS_ROOT", Path.cwd()))
    scanner = Path(os.environ.get("OSV_SCANNER_BIN", ""))
    config = root / "osv-scanner.toml"
    if len(os.sys.argv) == 2 and os.sys.argv[1] == "--allow-ghsas":
        return allow_ghsas(root)
    if not scanner:
        return error("osv-scanner の場所")
    try:
        rows = exception_rows(config)
    except ExceptionTableError as exc:
        print(f"error: {exc}")
        if len(os.sys.argv) == 3 and os.sys.argv[1] == "--base":
            print("新しく持ち込んだ: 比べなかった（例外の表を検査できなかった）")
            try:
                head_rows = raw_exception_rows(config)
                base_revision = resolve_base(root, os.sys.argv[2])
                if base_revision:
                    base_config = git_file(root, base_revision, "osv-scanner.toml")
                    with tempfile.TemporaryDirectory(prefix="check-vulns-base-") as temp:
                        base_path = Path(temp) / "osv-scanner.toml"
                        base_path.write_bytes(base_config or b"# no exceptions\n")
                        print_exception_changes(raw_exception_rows(base_path), head_rows)
                else:
                    print("== 例外の表で足した・変えた行\n比べなかった")
            except ValueError:
                print("== 例外の表で足した・変えた行\n比べなかった")
        return 1
    except ValueError as exc:
        return error(str(exc))
    try:
        files = lockfiles(root)
    except ValueError as exc:
        return error(str(exc))
    findings, failure = scan_root(scanner, root, files, require_all=True)
    if failure:
        return error(failure)
    allowed = set(rows)
    finding_ids = {ident for row in findings for ident in [row["id"], *row["aliases"]]}
    stale = sorted(ident for ident in allowed if ident not in finding_ids)
    base_revision = None
    if len(os.sys.argv) == 3 and os.sys.argv[1] == "--base":
        base_revision = resolve_base(root, os.sys.argv[2])
        if not base_revision:
            return error("base の commit")
        with tempfile.TemporaryDirectory(prefix="check-vulns-base-") as temp:
            base_root = Path(temp)
            base_files: list[Path] = []
            for path in files:
                relative = str(path.relative_to(root))
                content = git_file(root, base_revision, relative)
                if content is not None:
                    target = base_root / relative
                    target.parent.mkdir(parents=True, exist_ok=True)
                    target.write_bytes(content)
                    base_files.append(target)
            base_config = git_file(root, base_revision, "osv-scanner.toml")
            if base_config is None:
                base_config = b"# no exceptions\n"
            (base_root / "osv-scanner.toml").write_bytes(base_config)
            try:
                base_rows = exception_rows(base_root / "osv-scanner.toml", validate=False)
            except ValueError as exc:
                return error(str(exc))
            base_findings, failure = scan_root(scanner, base_root, base_files, require_all=False)
            if failure:
                return error(failure)
        base_keys = {finding_key(row) for row in base_findings}
        new_findings = [row for row in findings if finding_key(row) not in base_keys]
        outstanding = [row for row in new_findings if row["id"] not in allowed and not any(a in allowed for a in row["aliases"])]
        print(f"新しく持ち込んだ: {len(outstanding)} 件")
    else:
        outstanding = [row for row in findings if row["id"] not in allowed and not any(a in allowed for a in row["aliases"])]
    for row in outstanding:
        print(f"{row['ecosystem']} {row['name']} {row['version']} {row['id']} aliases={','.join(row['aliases'])} severity={row['severity']}")
    if base_revision is not None:
        print_exception_changes(base_rows, rows)
    for ident in stale:
        print(f"error: 例外の表: 勧告がもう出ない ({ident})")
    return 1 if outstanding or stale else 0


if __name__ == "__main__":
    raise SystemExit(main())
