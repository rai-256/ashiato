#!/usr/bin/env python3
"""Run osv-scanner and apply the repository's exception table."""
from __future__ import annotations

import datetime as dt
import json
import os
import subprocess
import tomllib
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


def exceptions(path: Path) -> set[str]:
    try:
        with path.open("rb") as f:
            config = tomllib.load(f)
    except (OSError, tomllib.TOMLDecodeError) as exc:
        raise ValueError(f"例外の表: {exc}") from exc
    allowed: set[str] = set()
    today = dt.datetime.now(dt.timezone.utc).date()
    for row in config.get("IgnoredVulns", []):
        ident = row.get("id")
        until = row.get("ignoreUntil")
        reason = row.get("reason")
        if not isinstance(ident, str):
            raise ExceptionTableError("例外の表: 勧告の ID が無い")
        if not isinstance(reason, str):
            raise ExceptionTableError(f"例外の表: 理由が無い ({ident})")
        if not isinstance(until, str):
            raise ExceptionTableError(f"例外の表: 期限が無い ({ident})")
        try:
            expiry = dt.date.fromisoformat(until)
        except ValueError as exc:
            raise ExceptionTableError(f"例外の表: 期限が不正 ({ident})") from exc
        if expiry < today:
            raise ExceptionTableError(f"例外の表: 期限切れ ({ident})")
        if expiry > today + dt.timedelta(days=90):
            raise ExceptionTableError(f"例外の表: 期限が 90 日を越える ({ident})")
        if not reason.startswith(("届かない: ", "上げる予定: ")):
            raise ExceptionTableError(f"例外の表: 理由の形が違う ({ident})")
        allowed.add(ident)
    return allowed


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


def main() -> int:
    root = Path(os.environ.get("CHECK_VULNS_ROOT", Path.cwd()))
    scanner = Path(os.environ.get("OSV_SCANNER_BIN", ""))
    config = root / "osv-scanner.toml"
    if not scanner:
        return error("osv-scanner の場所")
    try:
        allowed = exceptions(config)
    except ExceptionTableError as exc:
        print(f"error: {exc}")
        return 1
    except ValueError as exc:
        return error(str(exc))
    try:
        files = lockfiles(root)
    except ValueError as exc:
        return error(str(exc))
    missing = [p for p in files if not p.is_file()]
    if missing:
        return error("見つからないロックファイル: " + ", ".join(str(p.relative_to(root)) for p in missing))
    findings: list[dict] = []
    for path in files:
        rc, rows, failure = scan(scanner, root, path)
        if failure:
            return error(failure)
        findings.extend(rows)
    finding_ids = {ident for row in findings for ident in [row["id"], *row["aliases"]]}
    stale = sorted(ident for ident in allowed if ident not in finding_ids)
    for ident in stale:
        print(f"error: 例外の表: 勧告がもう出ない ({ident})")
    if stale:
        return 1
    outstanding = [row for row in findings if row["id"] not in allowed and not any(a in allowed for a in row["aliases"])]
    for row in outstanding:
        print(f"{row['ecosystem']} {row['name']} {row['version']} {row['id']} aliases={','.join(row['aliases'])} severity={row['severity']}")
    return 1 if outstanding else 0


if __name__ == "__main__":
    raise SystemExit(main())
