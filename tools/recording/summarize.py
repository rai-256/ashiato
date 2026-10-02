#!/usr/bin/env python3
"""録画 1 回ぶんの記録（run.json / RUN.md）を書き、一覧（runs.tsv）に 1 行足す。record-st22.sh が最後に呼ぶ。

**別々に記録する**: テストの合否 / 録画が残ったか / 再生できるか / 片付けに残りが無いか / 止まった工程。
**判定しないもの**: 動画の見やすさ・人間の承認（常に「未実施」と書く）。
入力が壊れていても（途中で切れた results.json など）、記録は必ず書く。

rc: 0 = テストが通り・本命の動画が再生でき・写しが原本と一致し・片付けに残りが無く・止まった工程が無い。1 = それ以外。
"""
import argparse
import hashlib
import json
import platform
import subprocess
from datetime import datetime
from pathlib import Path

CMD_EXE = "/mnt/c/Windows/System32/cmd.exe"
MAIN_SPEC = "st22-erase-reload.rec.ts"
REUSED_SPEC = "day-erase.spec.ts"
# Playwright は出力フォルダの名前を縮める（`e2e-recording-st22-erase-r-<hash>-…`）ので、頭で見分ける
MAIN_DIR_PREFIX = "e2e-recording-st22-erase-r"
INDEX_COLUMNS = ["run_id", "commit", "overall", "test", "recording", "playable", "cleanup", "failed_step", "human_approval"]
INDEX_HEADER = "\t".join(INDEX_COLUMNS) + "\n"


def sh(cmd, cwd=None):
    try:
        return subprocess.run(cmd, cwd=cwd, capture_output=True, text=True, timeout=60,
                              stdin=subprocess.DEVNULL).stdout.strip()
    except Exception as e:  # 記録の材料が取れなくても、記録そのものは書く
        return f"（取れなかった: {e}）"


def win(cmd):
    return sh([CMD_EXE, "/c", cmd], cwd="/mnt/c").replace("\r", "").strip()


def sha256(path: Path):
    h = hashlib.sha256()
    with path.open("rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def kv(path: Path):
    """`key=value` の行を読む（同じ key が複数あれば並べる）。無ければ空。"""
    out = {}
    if path.is_file():
        for ln in path.read_text(encoding="utf-8", errors="replace").splitlines():
            if "=" in ln:
                k, v = ln.split("=", 1)
                out.setdefault(k, []).append(v)
    return out


def first(d, k):
    return (d.get(k) or [""])[0]


def tests_from(results_path: Path):
    """Playwright の JSON reporter から、全体・テストごとの結果を引く。壊れていれば読めない理由を返す。"""
    if not results_path.is_file():
        return None, [], None
    try:
        d = json.loads(results_path.read_text(encoding="utf-8"))
    except Exception as e:
        return None, [], f"results.json を読めない: {e}"
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
    return d, rows, None


def judge_tests(d, rows, read_error, pw_rc, failed_step):
    """テストの合否。**本命のテストと既存の ST22 のテストが実際に通ったこと**まで見る（数の緑だけで通さない）。"""
    if read_error:
        return "unreadable", read_error
    if d is None:
        return "not_run", failed_step or "Playwright の結果が無い（logs/playwright.log）"
    st = d.get("stats", {})
    note = (f"expected {st.get('expected', 0)} / unexpected {st.get('unexpected', 0)} / "
            f"flaky {st.get('flaky', 0)} / skipped {st.get('skipped', 0)} / Playwright rc={pw_rc or '?'}")
    errors = [(e.get("message") or "")[:200] for e in d.get("errors", [])]
    main = [r for r in rows if Path(r["file"]).name == MAIN_SPEC]
    reused = [r for r in rows if Path(r["file"]).name == REUSED_SPEC]
    reasons = []
    if st.get("unexpected", 0) or st.get("flaky", 0):
        reasons.append("落ちたテストがある")
    if st.get("skipped", 0):
        reasons.append("飛ばされたテストがある")
    if errors:
        reasons.append("全体のエラー: " + " / ".join(errors))
    if not main or any(r["status"] != "passed" for r in main):
        reasons.append(f"本命のテスト（{MAIN_SPEC}）が通っていない・無い")
    if not reused or any(r["status"] != "passed" for r in reused):
        reasons.append(f"既存の ST22 のテスト（{REUSED_SPEC}）が通っていない・無い")
    if reasons:
        return "failed", note + "。" + "・".join(reasons)
    if pw_rc != "0":
        return "inconsistent", note + "。結果は緑だが Playwright の rc が 0 でない"
    return "passed", note


def update_index(index: Path, values: dict):
    """一覧に 1 行足す（追記のみ）。見出しが前の版（2026-10-02 の初版）なら、列の名前で今の並びに組み直す。"""
    if not index.exists():
        index.write_text(INDEX_HEADER, encoding="utf-8")
    else:
        text = index.read_text(encoding="utf-8")
        head, _, rest = text.partition("\n")
        old = head.split("\t")
        if old != INDEX_COLUMNS:
            rows = [dict(zip(old, r.split("\t"))) for r in rest.splitlines() if r]
            index.write_text(INDEX_HEADER + "".join(
                "\t".join(r.get(c, "") for c in INDEX_COLUMNS) + "\n" for r in rows), encoding="utf-8")
    with index.open("a", encoding="utf-8") as f:
        f.write("\t".join(str(values.get(c, "")).replace("\t", " ") for c in INDEX_COLUMNS) + "\n")


def main():
    ap = argparse.ArgumentParser()
    for a in ("out", "run-id", "sha", "started", "invocation", "failed-step", "pw-cmd", "pw-rc",
              "ports", "project", "hold-ms", "slowmo-ms"):
        ap.add_argument(f"--{a}", default="")
    o = ap.parse_args()
    out = Path(o.out)

    # ---- テストの合否
    d, rows, read_error = tests_from(out / "playwright/results/results.json")
    test_status, test_note = judge_tests(d, rows, read_error, o.pw_rc, o.failed_step)

    # ---- 録画の有無と再生できるか（ファイルそのもので見る。確かめの工程まで進まずに止まっても、撮れた動画はある）
    checks = []
    vc = out / "video-check.jsonl"
    if vc.is_file():
        for ln in vc.read_text(encoding="utf-8", errors="replace").splitlines():
            if not ln.strip():
                continue
            try:
                checks.append(json.loads(ln))
            except Exception as e:
                checks.append({"file": "?", "check": {"playable": False, "error": f"確かめの行を読めない: {e}"}})
    main_video = next(iter(sorted((out / "playwright/results").glob(f"{MAIN_DIR_PREFIX}*/video.webm"))), None)
    main_check = next((c for c in checks if f"/{MAIN_DIR_PREFIX}" in c.get("file", "")), None)
    recording_status = "recorded" if main_video else "missing"
    if not main_video or main_check is None:
        playable_status = "unchecked"
    else:
        playable_status = "playable" if main_check.get("check", {}).get("playable") else "not_playable"
    # 写し（ST22-erase-reload.webm）が原本と同じか（写している最中に止まったら不一致になる）
    copy = out / "ST22-erase-reload.webm"
    video_sha = sha256(main_video) if main_video else ""
    copy_status = ("一致" if sha256(copy) == video_sha else "不一致") if (main_video and copy.is_file()) else "なし"

    # ---- 片付け
    csf = out / "cleanup-status.txt"
    cs = csf.read_text(encoding="utf-8").strip() if csf.is_file() else "unknown"

    # ---- 対象・道具・ビルドの版
    tool = kv(out / "tool-status.txt")
    build = kv(out / "build-info.txt")
    overlay = {p.name: sha256(p) for p in sorted((out / "overlay").glob("*"))}
    browser = next((a.get("description") for r in rows for a in r["annotations"] if a.get("type") == "browser"), "")
    env = {
        "wsl_kernel": platform.release(),
        "docker": sh(["docker", "version", "--format", "{{.Server.Version}}"]),
        "cargo": sh(["cargo", "--version"]),
        "node_wsl": sh(["node", "-v"]),
        "playwright_wsl（再生の確かめ）": first(build, "playwright_wsl"),
        "windows": win("ver"),
        "node_windows": win("node -v"),
        "npm_windows": first(build, "npm_windows"),
        "playwright_windows（録画）": ((d or {}).get("config") or {}).get("version", ""),
        "browser_windows（録画）": browser,
    }
    db_port, api_port, web_port = (o.ports.split("/") + ["", "", ""])[:3]
    ok = (test_status == "passed" and playable_status == "playable" and copy_status == "一致"
          and cs == "ok" and not o.failed_step)
    run = {
        "story": "ST22",
        "scenario": "st22-erase-reload",
        "run_id": o.run_id,
        "started": o.started,
        "finished": datetime.now().astimezone().isoformat(timespec="seconds"),
        "overall": "ok" if ok else "ng",
        "target": {
            "commit": o.sha,
            "checkout": "一時 worktree に対象コミットをそのまま取り出した（未コミットの変更は含まれない）",
            "overlay_sha256": overlay,
            "server_sha256": first(build, "server_sha256"),
            "web_dist_sha256": first(build, "web_dist_sha256"),
        },
        "tool": {
            "commit": first(tool, "tool_commit"),
            "uncommitted_in_tools_recording_at_start": tool.get("dirty", []),
        },
        "env": env,
        "isolation": {"compose_project": o.project, "db_port": db_port, "api_port": api_port,
                      "web_port": web_port, "seed": "normal（STACK_RESET=1 で作り直した直後）"},
        "recording_options": {"REC_HOLD_MS": o.hold_ms, "REC_SLOWMO_MS": o.slowmo_ms},
        "commands": {"invocation": o.invocation, "playwright_on_windows": o.pw_cmd, "playwright_rc": o.pw_rc},
        "test": {"status": test_status, "note": test_note, "retries": 0,
                 "errors": [(e.get("message") or "")[:400] for e in (d or {}).get("errors", [])], "tests": rows},
        "recording": {"status": recording_status, "playable": playable_status,
                      "main_video_sha256": video_sha, "copy": copy_status, "checks": checks},
        "cleanup": cs,
        "failed_step": o.failed_step,
        "visibility": "判定しない（人間が見る）",
        "human_approval": "未実施",
    }
    (out / "run.json").write_text(json.dumps(run, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")

    # ---- RUN.md（人間が読む）
    lp = out / "logs/cleanup.log"
    cleanup_log = lp.read_text(encoding="utf-8", errors="replace") if lp.is_file() else "（片付けの記録が無い）"
    mc = (main_check or {}).get("check", {})
    lines = [f"# ST22 録画 {o.run_id}", ""]
    if o.failed_step:
        lines += [f"**止まった工程: {o.failed_step}** —— この実行は成功として扱わない", ""]
    lines += [
        "| 項目 | 結果 |",
        "|---|---|",
        f"| 全体 | **{'OK' if ok else 'NG'}**（rc={'0' if ok else '1'}） |",
        f"| テストの合否 | **{test_status}**（{test_note}。再試行 0） |",
        f"| 録画 | **{recording_status}**"
        + (f"（写し `ST22-erase-reload.webm`: {copy_status}）" if main_video else "") + " |",
        f"| 再生できるか | **{playable_status}**"
        + (f"（{mc.get('duration_s')} 秒・{mc.get('width')}×{mc.get('height')}・再生して {mc.get('played_s')} 秒進んだ）"
           if main_check else "") + " |",
        f"| 片付け | **{cs}** |",
        "| 見やすさ | 判定しない（人間が見る） |",
        "| 人間の承認 | **未実施** |",
        "",
        "## 対象",
        "",
        f"- コミット: `{o.sha}`（一時 worktree にそのまま取り出した。**未コミットの変更は含まれない**）",
        "- 録画用に重ねたファイル（`tools/recording/st22/`。写しは `overlay/`）:",
    ] + [f"  - `{k}` sha256 `{v}`" for k, v in overlay.items()] + [
        f"- 道具のコミット: `{first(tool, 'tool_commit')}` / `tools/recording` の未コミット変更（開始時）: "
        + ("**あり** " + "; ".join(f"`{x}`" for x in tool.get("dirty", [])) if tool.get("dirty") else "なし"),
        f"- サーバ sha256: `{run['target']['server_sha256']}`",
        f"- 画面（web/dist 全体）sha256: `{run['target']['web_dist_sha256']}`",
        f"- 本命の動画 sha256: `{video_sha}`",
        "",
        "## 実行環境",
        "",
    ] + [f"- {k}: {v}" for k, v in env.items()] + [
        f"- 専用環境: compose project `{o.project}`、DB `127.0.0.1:{db_port}`、API `127.0.0.1:{api_port}`、"
        f"画面 `127.0.0.1:{web_port}`、偽データ normal（作り直した直後）",
        f"- 録画の設定: REC_HOLD_MS={o.hold_ms} / REC_SLOWMO_MS={o.slowmo_ms}",
        "",
        "## 実行コマンド",
        "",
        f"- `{o.invocation}`",
        f"- Windows: `{o.pw_cmd or '（ここまで進まなかった）'}`（WEB_PASSWORD などは WSLENV で渡した）→ rc={o.pw_rc or '-'}",
        "",
        "## テスト",
        "",
        "| テスト | 結果 | 所要 |",
        "|---|---|---|",
    ] + [f"| {Path(r['file']).name} › {r['title']} | {r['status']} | {(r['duration_ms'] or 0) / 1000:.1f}s |"
         for r in rows] + [
        "",
        "## 録画（再生できるかの機械の確かめ）",
        "",
        "| ファイル | 長さ | 再生 |",
        "|---|---|---|",
    ] + [f"| {c.get('file')} | {c.get('check', {}).get('duration_s')} 秒 | "
         + ("できた" if c.get("check", {}).get("playable") else "できない " + str(c.get("check", {}).get("error") or ""))
         + " |" for c in checks] + [
        "",
        "## 開き方",
        "",
        "- 動画: `ST22-erase-reload.webm` を Edge / Chrome で開く",
        "- trace: `npx playwright show-trace ST22-erase-reload.trace.zip`、"
        "または https://trace.playwright.dev に落とす（ブラウザの中で処理される）",
        "- 全テストぶんのレポート: `npx playwright show-report playwright\\report`",
        "- ログ: `logs/`（worktree / build / stack / windows-setup / playwright / cleanup）",
        "",
        "## 片付け",
        "",
        "```",
        cleanup_log.strip(),
        "```",
    ]
    (out / "RUN.md").write_text("\n".join(lines) + "\n", encoding="utf-8")

    update_index(out.parent / "runs.tsv", {
        "run_id": o.run_id, "commit": o.sha[:12], "overall": "ok" if ok else "ng", "test": test_status,
        "recording": recording_status + ("" if copy_status in ("一致", "なし") else f"（写し{copy_status}）"),
        "playable": playable_status, "cleanup": cs, "failed_step": o.failed_step, "human_approval": "未実施"})
    print(f"テスト: {test_status} / 録画: {recording_status} / 再生: {playable_status} / 片付け: {cs}"
          + (f" / 止まった工程: {o.failed_step}" if o.failed_step else "") + " / 人間の承認: 未実施")
    return 0 if ok else 1


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except SystemExit:
        raise
    except Exception as e:  # 記録の途中で落ちても、一覧には「記録できなかった」の 1 行を残す（rc=3）
        import sys
        a = sys.argv
        out = Path(a[a.index("--out") + 1]) if "--out" in a else None
        if out is not None:
            try:
                rid = a[a.index("--run-id") + 1] if "--run-id" in a else out.name
                sha = a[a.index("--sha") + 1][:12] if "--sha" in a else ""
                update_index(out.parent / "runs.tsv", {"run_id": rid, "commit": sha, "overall": "ng",
                                                       "test": "unrecorded", "failed_step": f"記録の書き出しで落ちた: {e}",
                                                       "human_approval": "未実施"})
            except Exception:
                pass
        print(f"error: 記録の書き出しで落ちた: {e}", file=sys.stderr)
        raise SystemExit(3)
