#!/usr/bin/env python3
"""Run osv-scanner and apply the repository's exception table."""
from __future__ import annotations

import datetime as dt
import json
import os
import subprocess
import tomllib
from pathlib import Path


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
    return paths + [root / line for line in result.stdout.splitlines()]


def exceptions(path: Path) -> set[str]:
    try:
        with path.open("rb") as f:
            config = tomllib.load(f)
    except (OSError, tomllib.TOMLDecodeError) as exc:
        raise ValueError(f"例外の表: {exc}") from exc
    allowed: set[str] = set()
    today = dt.date.today()
    for row in config.get("IgnoredVulns", []):
        ident = row.get("id")
        until = row.get("ignoreUntil")
        reason = row.get("reason")
        if not isinstance(ident, str) or not isinstance(until, str) or not isinstance(reason, str):
            raise ValueError("例外の表: id / ignoreUntil / reason が必要")
        try:
            expiry = dt.date.fromisoformat(until)
        except ValueError as exc:
            raise ValueError(f"例外の表: 期限が不正 ({ident})") from exc
        if expiry < today:
            raise ValueError(f"例外の表: 期限切れ ({ident})")
        if expiry > today + dt.timedelta(days=90):
            raise ValueError(f"例外の表: 期限が 90 日を越える ({ident})")
        if not reason.startswith("CH11:"):
            raise ValueError(f"例外の表: 理由の形が不正 ({ident})")
        allowed.add(ident)
    return allowed


def scan(scanner: Path, root: Path, lock: Path) -> tuple[int, list[dict]]:
    command = [str(scanner), "scan", "source", "--lockfile", str(lock), "--format", "json",
               "--all-vulns", "--all-packages", "--config", "/dev/null"]
    proc = subprocess.run(command, cwd=root, text=True, capture_output=True)
    if proc.returncode not in (0, 1):
        return error_code("osv.dev への問い合わせ" if proc.returncode else "osv-scanner"), []
    try:
        data = json.loads(proc.stdout)
    except json.JSONDecodeError:
        return error_code("JSON の読み取り"), []
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
    return proc.returncode, findings


def error_code(_: str) -> int:
    return 2


def main() -> int:
    root = Path(os.environ.get("CHECK_VULNS_ROOT", Path.cwd()))
    scanner = Path(os.environ.get("OSV_SCANNER_BIN", ""))
    config = root / "osv-scanner.toml"
    if not scanner:
        return error("osv-scanner の場所")
    try:
        allowed = exceptions(config)
    except ValueError as exc:
        return error(str(exc))
    files = lockfiles(root)
    missing = [p for p in files if not p.is_file()]
    if missing:
        return error("見つからないロックファイル: " + ", ".join(str(p.relative_to(root)) for p in missing))
    findings: list[dict] = []
    for path in files:
        rc, rows = scan(scanner, root, path)
        if rc == 2:
            return error("osv.dev への問い合わせ")
        findings.extend(rows)
    outstanding = [row for row in findings if row["id"] not in allowed and not any(a in allowed for a in row["aliases"])]
    for row in outstanding:
        print(f"{row['ecosystem']} {row['name']} {row['version']} {row['id']} aliases={','.join(row['aliases'])} severity={row['severity']}")
    return 1 if outstanding else 0


if __name__ == "__main__":
    raise SystemExit(main())
