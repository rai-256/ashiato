#!/usr/bin/env python3
"""毎晩の検査結果を issue にする処理。"""
from __future__ import annotations

import json
import os
import subprocess
import sys
import importlib.util
from pathlib import Path

_spec = importlib.util.spec_from_file_location("check_vulns", Path(__file__).with_name("check-vulns.py"))
assert _spec and _spec.loader
_module = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(_module)
ExceptionTableError = _module.ExceptionTableError
exception_row_error = _module.exception_row_error
exception_rows = _module.exception_rows
raw_exception_rows = _module.raw_exception_rows
lockfiles = _module.lockfiles
scan_root = _module.scan_root


ISSUE_LIMIT = 1000


def run(command: list[str], *, input_text: str | None = None) -> subprocess.CompletedProcess[str]:
    return subprocess.run(command, input=input_text, text=True, capture_output=True)


def write_material(directory: Path, rows: list[dict]) -> None:
    directory.mkdir(parents=True, exist_ok=True)
    (directory / "vulnerabilities.json").write_text(json.dumps(rows, ensure_ascii=False), encoding="utf-8")


def nightly(root: Path, scanner: Path, directory: Path) -> int:
    try:
        config = root / "osv-scanner.toml"
        raw_rows = raw_exception_rows(config)
        invalid_rows = []
        valid_rows = {}
        for ident, row in raw_rows.items():
            problem = exception_row_error(ident, row)
            if problem:
                invalid_rows.append({"kind": "exception_table", "id": ident, "reason": problem.split(" (", 1)[0]})
            else:
                valid_rows[ident] = row
        files = lockfiles(root)
        findings, failure = scan_root(scanner, root, files, require_all=True)
        if failure:
            print(f"error: 検査できなかった（{failure}）")
            return 2
    except ExceptionTableError as exc:
        print(f"error: 検査できなかった（{exc}）")
        return 2
    except ValueError as exc:
        print(f"error: 検査できなかった（{exc}）")
        return 2
    allowed = set(valid_rows)
    # 全体の検査（check-vulns.py の main）と同じく、今夜 1 度も当たらなかった行は「勧告がもう出ない」として材料にする。
    finding_ids = {ident for row in findings for ident in [row["id"], *row["aliases"]]}
    invalid_rows.extend({"kind": "exception_table", "id": ident, "reason": "勧告がもう出ない"}
                        for ident in sorted(allowed) if ident not in finding_ids)
    outstanding = [row for row in findings
                   if row["id"] not in allowed and not any(alias in allowed for alias in row["aliases"])]
    if not outstanding and not invalid_rows:
        return 0
    material = invalid_rows + [{"kind": "vulnerability", **row} for row in outstanding]
    write_material(directory, material)
    return 1


def gh_json(args: list[str]) -> object:
    proc = run(["gh", *args])
    if proc.returncode != 0:
        raise RuntimeError(proc.stderr.strip() or "gh の実行")
    try:
        return json.loads(proc.stdout)
    except json.JSONDecodeError as exc:
        raise RuntimeError(f"gh の応答: {exc}") from exc


def issue_body(row: dict) -> str:
    if row["kind"] == "vulnerability":
        return "\n".join([
            f"依存: {row['name']}", f"版: {row['version']}", f"勧告: {row['id']}",
            f"重大度: {row.get('severity') or '不明'}", f"URL: https://osv.dev/vulnerability/{row['id']}",
        ])
    if row["kind"] == "exception_table":
        return f"勧告: {row['id']}\n理由: {row['reason'].split(':', 1)[0]}"
    return row["body"]


def latest_line(row: dict) -> str:
    return next((line for line in issue_body(row).splitlines() if line.startswith("最新:")), issue_body(row))


def title_for(row: dict) -> str:
    if row["kind"] == "vulnerability":
        return f"脆弱性: {row['ecosystem']} {row['name']}"
    if row["kind"] == "exception_table":
        return "脆弱性: 例外の表"
    return row["title"]


def load_material(directory: Path) -> list[dict]:
    rows: list[dict] = []
    json_paths = [directory] if directory.is_file() and directory.suffix == ".json" else sorted(directory.glob("*.json"))
    for path in json_paths:
        rows.extend(json.loads(path.read_text(encoding="utf-8")))
    pg = directory if directory.is_file() and directory.name == "postgresql-minor.txt" else directory / "postgresql-minor.txt"
    if pg.is_file():
        lines = pg.read_text(encoding="utf-8").splitlines()
        if lines:
            rows.append({"kind": "text", "title": lines[0], "body": "\n".join(lines[1:])})
    return rows


def file_issues(directory: Path) -> int:
    if not directory.exists():
        print("error: issue を記録できなかった（材料が無い）", file=sys.stderr)
        return 2
    rows = load_material(directory)
    if not rows:
        print("error: issue を記録できなかった（材料が無い）", file=sys.stderr)
        return 2
    grouped: dict[str, list[dict]] = {}
    for row in rows:
        grouped.setdefault(title_for(row), []).append(row)
    try:
        labels = gh_json(["label", "list", "--search", "脆弱性", "--json", "name"])
        if not any(item.get("name") == "脆弱性" for item in labels):
            proc = run(["gh", "label", "create", "脆弱性", "--color", "B60205"])
            if proc.returncode != 0:
                raise RuntimeError(proc.stderr.strip() or "ラベルを作れない")
        # gh issue list の既定は 30 件。開いた issue は勧告が消えても閉じないので増える一方で、
        # 31 件目からを読まないと毎晩同じ題の issue を立て直す。
        issues = gh_json(["issue", "list", "--state", "open", "--label", "脆弱性", "--limit", str(ISSUE_LIMIT),
                          "--json", "number,title,body"])
        if len(issues) >= ISSUE_LIMIT:
            raise RuntimeError(f"開いた 脆弱性 の issue が {ISSUE_LIMIT} 件以上あり、重複を判定できない")
        for title, title_rows in grouped.items():
            row = title_rows[0]
            existing = next((issue for issue in issues if issue.get("title") == title), None)
            body = "\n".join(issue_body(item) for item in title_rows)
            if existing is None:
                proc = run(["gh", "issue", "create", "--title", title, "--label", "脆弱性", "--body", body])
                if proc.returncode != 0:
                    raise RuntimeError(proc.stderr.strip() or "issue を作れない")
                continue
            comments = gh_json(["issue", "view", str(existing["number"]), "--json", "comments"])
            text = str(existing.get("body", "")) + "\n" + "\n".join(
                comment.get("body", "") for comment in comments.get("comments", []))
            # 開いた issue に無いものだけを追記する。勧告は ID、例外の表の行と PostgreSQL の遅れは本文の行で見る
            # （題が同じまま 2 つ目の期限切れの行・新しい minor が出ても、黙って捨てない）。
            if row["kind"] == "vulnerability":
                missing = [f"勧告: {item['id']}" for item in title_rows if item["id"] not in text]
            elif row["kind"] == "text":
                # PostgreSQL の遅れは題と `最新:` の行で見る。EDB の状態や公開日の行は夜ごとに揺れるので比べない（R29）。
                missing = [issue_body(item) for item in title_rows if latest_line(item) not in text.splitlines()]
            else:
                missing = [issue_body(item) for item in title_rows if issue_body(item) not in text]
            if not missing:
                continue
            proc = run(["gh", "issue", "comment", str(existing["number"]), "--body", "\n".join(missing)])
            if proc.returncode != 0:
                raise RuntimeError(proc.stderr.strip() or "issue に追記できない")
        return 0
    except (RuntimeError, OSError) as exc:
        print(f"error: issue を記録できなかった（{exc}）", file=sys.stderr)
        return 2


def main() -> int:
    root = Path(os.environ.get("CHECK_VULNS_ROOT", Path.cwd()))
    scanner = Path(os.environ.get("OSV_SCANNER_BIN", ""))
    if sys.argv[1] == "nightly" and len(sys.argv) == 3:
        return nightly(root, scanner, Path(sys.argv[2]))
    if sys.argv[1] == "file-issues" and len(sys.argv) == 3:
        return file_issues(Path(sys.argv[2]))
    print("error: 検査できなかった（使い方）")
    return 2


if __name__ == "__main__":
    raise SystemExit(main())
