#!/usr/bin/env python3
"""録画 1 回ぶんの記録（run.json / RUN.md）を書き、一覧（runs.tsv）に 1 行足す。record-st22.sh が最後に呼ぶ。

**3 つを別々に記録する**: テストの合否 / 録画が残ったか / 再生できるか。
**判定しないもの**: 動画の見やすさ・人間の承認（常に「未実施」と書く）。

rc: 0 = テストが全部通り、本命の動画が再生できた。1 = それ以外（どれが欠けたかは RUN.md の先頭）。
"""
import argparse
import json
import platform
import subprocess
from datetime import datetime
from pathlib import Path

CMD_EXE = "/mnt/c/Windows/System32/cmd.exe"


def sh(cmd, cwd=None):
    try:
        return subprocess.run(cmd, cwd=cwd, capture_output=True, text=True, timeout=60).stdout.strip()
    except Exception as e:  # 記録の材料が取れなくても、記録そのものは書く
        return f"（取れなかった: {e}）"


def win(cmd):
    return sh([CMD_EXE, "/c", cmd], cwd="/mnt/c").replace("\r", "").strip()


def tests_from(results_path: Path):
    """Playwright の JSON reporter から、テストごとの結果と注記を引く。"""
    if not results_path.is_file():
        return None, []
    d = json.loads(results_path.read_text(encoding="utf-8"))
    rows = []

    def walk(suite):
        for spec in suite.get("specs", []):
            for t in spec.get("tests", []):
                for r in t.get("results", []):
                    rows.append({
                        "file": spec.get("file", ""), "title": spec.get("title", ""),
                        "status": r.get("status"), "retry": r.get("retry", 0),
                        "duration_ms": r.get("duration"),
                        "annotations": t.get("annotations", []) + r.get("annotations", []),
                        "error": ((r.get("error") or {}).get("message") or "")[:400],
                    })
        for s in suite.get("suites", []):
            walk(s)

    for s in d.get("suites", []):
        walk(s)
    return d, rows


def main():
    ap = argparse.ArgumentParser()
    for a in ("out", "run-id", "sha", "repo", "tool-dir", "started", "invocation", "failed-step",
              "pw-cmd", "server-sha", "web-sha", "ports", "project", "pw-rc"):
        ap.add_argument(f"--{a}", default="")
    o = ap.parse_args()
    out = Path(o.out)
    repo = o.repo

    # ---- テストの合否
    d, rows = tests_from(out / "playwright/results/results.json")
    if d is None:
        test_status = "not_run"
        test_note = o.failed_step or "Playwright の結果が無い（logs/playwright.log）"
    else:
        st = d.get("stats", {})
        bad = st.get("unexpected", 0) + st.get("flaky", 0)
        test_status = "passed" if bad == 0 and st.get("expected", 0) > 0 else "failed"
        test_note = (f"expected {st.get('expected', 0)} / unexpected {st.get('unexpected', 0)} / "
                     f"flaky {st.get('flaky', 0)} / skipped {st.get('skipped', 0)}")

    # ---- 録画の有無と再生できるか
    checks = []
    vc = out / "video-check.jsonl"
    if vc.is_file():
        checks = [json.loads(ln) for ln in vc.read_text(encoding="utf-8").splitlines() if ln.strip()]
    # Playwright は出力フォルダの名前を縮める（`e2e-recording-st22-erase-r-<hash>-…`）ので、頭で見分ける
    main_check = next((c for c in checks if "/e2e-recording-st22-erase-r" in c["file"]), None)
    recorded = bool(main_check and main_check["check"].get("exists"))
    playable = bool(main_check and main_check["check"].get("playable"))
    recording_status = "recorded" if recorded else "missing"
    playable_status = ("playable" if playable else "not_playable") if recorded else "unchecked"

    # ---- 対象と未コミット変更
    tool_head = sh(["git", "-C", repo, "rev-parse", "HEAD"])
    tool_dirty = sh(["git", "-C", repo, "status", "--porcelain", "--", "tools/recording"])
    overlay = {p.name: sh(["sha256sum", str(p)]).split()[0][:16] for p in sorted((out / "overlay").glob("*"))}
    browser = next((a.get("description") for r in rows for a in r["annotations"] if a.get("type") == "browser"), "")

    env = {
        "wsl_kernel": platform.release(),
        "docker": sh(["docker", "version", "--format", "{{.Server.Version}}"]),
        "cargo": sh(["cargo", "--version"]),
        "node_wsl": sh(["node", "-v"]),
        "windows": win("ver"),
        "node_windows": win("node -v"),
        "playwright": (d or {}).get("config", {}).get("version", ""),
        "browser": browser,
    }
    db_port, api_port, web_port = (o.ports.split("/") + ["", "", ""])[:3]
    run = {
        "story": "ST22",
        "scenario": "st22-erase-reload",
        "run_id": o.run_id,
        "started": o.started,
        "finished": datetime.now().astimezone().isoformat(timespec="seconds"),
        "target": {
            "commit": o.sha,
            "checkout": "一時 worktree に対象コミットをそのまま取り出した（未コミットの変更は含まれない）",
            "overlay": overlay,
            "server_sha256": o.server_sha,
            "web_dist_sha256": o.web_sha,
        },
        "tool": {
            "commit": tool_head,
            "uncommitted_changes_in_tools_recording": tool_dirty.splitlines() if tool_dirty else [],
        },
        "env": env,
        "isolation": {"compose_project": o.project, "db_port": db_port, "api_port": api_port,
                      "web_port": web_port, "seed": "normal（STACK_RESET=1 で作り直した直後）"},
        "commands": {"invocation": o.invocation, "playwright_on_windows": o.pw_cmd,
                     "playwright_rc": o.pw_rc},
        "test": {"status": test_status, "note": test_note, "retries": 0, "tests": rows},
        "recording": {"status": recording_status, "playable": playable_status, "checks": checks},
        "failed_step": o.failed_step,
        "visibility": "判定しない（人間が見る）",
        "human_approval": "未実施",
    }
    (out / "run.json").write_text(json.dumps(run, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")

    # ---- RUN.md（人間が読む）
    cleanup_log = (out / "logs/cleanup.log").read_text(encoding="utf-8", errors="replace") \
        if (out / "logs/cleanup.log").is_file() else "（片付けの記録が無い）"
    lines = [
        f"# ST22 録画 {o.run_id}",
        "",
        "| 項目 | 結果 |",
        "|---|---|",
        f"| テストの合否 | **{test_status}**（{test_note}。再試行 0） |",
        f"| 録画 | **{recording_status}**（`ST22-erase-reload.webm`） |",
        f"| 再生できるか | **{playable_status}**"
        + (f"（{main_check['check'].get('duration_s')} 秒・{main_check['check'].get('width')}×{main_check['check'].get('height')}・"
           f"再生して {main_check['check'].get('played_s')} 秒進んだ）" if recorded else "") + " |",
        "| 見やすさ | 判定しない（人間が見る） |",
        "| 人間の承認 | **未実施** |",
    ]
    if o.failed_step:
        lines += ["", f"**止まった工程: {o.failed_step}**"]
    lines += [
        "",
        "## 対象",
        "",
        f"- コミット: `{o.sha}`（一時 worktree にそのまま取り出した。**未コミットの変更は含まれない**）",
        f"- 録画用に重ねたファイル（`tools/recording/st22/`、sha256 の先頭）: "
        + ", ".join(f"`{k}` {v}" for k, v in overlay.items()),
        f"- 道具のコミット: `{tool_head}` / `tools/recording` の未コミット変更: "
        + ("**あり** " + "; ".join(f"`{x}`" for x in tool_dirty.splitlines()) if tool_dirty else "なし"),
        f"- サーバ sha256 の先頭: `{o.server_sha}` / 画面（web/dist 全体）: `{o.web_sha}`",
        "",
        "## 実行環境",
        "",
    ] + [f"- {k}: {v}" for k, v in env.items()] + [
        f"- 専用環境: compose project `{o.project}`、DB `127.0.0.1:{db_port}`、API `127.0.0.1:{api_port}`、"
        f"画面 `127.0.0.1:{web_port}`、偽データ normal（作り直した直後）",
        "",
        "## 実行コマンド",
        "",
        f"- `{o.invocation}`",
        f"- Windows: `{o.pw_cmd}`（WEB_PASSWORD / REC_BASE_URL / REC_OUT / REC_REPORT は WSLENV で渡した）→ rc={o.pw_rc}",
        "",
        "## テスト",
        "",
        "| テスト | 結果 | 所要 |",
        "|---|---|---|",
    ] + [f"| {Path(r['file']).name} › {r['title']} | {r['status']} | {(r['duration_ms'] or 0) / 1000:.1f}s |" for r in rows] + [
        "",
        "## 録画（再生できるかの機械の確かめ）",
        "",
        "| ファイル | 有無 | 長さ | 再生 |",
        "|---|---|---|---|",
    ] + [f"| {c['file']} | {'あり' if c['check'].get('exists') else 'なし'} | {c['check'].get('duration_s')} 秒 | "
         f"{'できた' if c['check'].get('playable') else 'できない ' + str(c['check'].get('error') or '')} |" for c in checks] + [
        "",
        "## 開き方",
        "",
        "- 動画: `ST22-erase-reload.webm` を Edge / Chrome で開く",
        "- trace: `npx playwright show-trace ST22-erase-reload.trace.zip`、または https://trace.playwright.dev に落とす（ブラウザの中で処理される）",
        "- 4 本ぶんのレポート: `npx playwright show-report playwright\\report`",
        "- ログ: `logs/`（build / stack / windows-setup / playwright / cleanup）",
        "",
        "## 片付け",
        "",
        "```",
        cleanup_log.strip(),
        "```",
    ]
    (out / "RUN.md").write_text("\n".join(lines) + "\n", encoding="utf-8")

    # ---- 一覧に 1 行（上書きしない）
    index = out.parent / "runs.tsv"
    if not index.exists():
        index.write_text("run_id\tcommit\ttest\trecording\tplayable\tfailed_step\thuman_approval\n", encoding="utf-8")
    with index.open("a", encoding="utf-8") as f:
        f.write(f"{o.run_id}\t{o.sha[:12]}\t{test_status}\t{recording_status}\t{playable_status}\t{o.failed_step}\t未実施\n")

    print(f"テスト: {test_status} / 録画: {recording_status} / 再生: {playable_status} / 人間の承認: 未実施")
    return 0 if (test_status == "passed" and playable) else 1


if __name__ == "__main__":
    raise SystemExit(main())
