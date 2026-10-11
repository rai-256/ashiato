#!/usr/bin/env python3
"""人間が読むとよい文書を、スマホでも読める HTML にまとめて出す（操作盤の「読み物」から開く）。

    python3 tools/reading_brief.py                     # docs/briefs/reading/ に書く
    python3 tools/reading_brief.py -o <dir>

出すもの（ファイル名 = 操作盤に並ぶ順）:
    01-requirements.html   要件（tools/requirements_brief.py に任せる）
    02-ui-direction.html   UI の方向（要点 + 全文）。playground も 02-ui-playground.html として添える
    03-production-prep.html 製造準備（要点 + 全文）
    04-specs.html          openspec/specs/ の正典（capability ごとの要求の一覧。機械で引くだけ）
    05-handoff.html        凍結した Story への申し送り（全文を並べるだけ）
    06-license.html        ライセンスの地図（docs/audit/2026-10-09-license-map.html の写し）
    07-cla.html            CLA（全文）

本文は原文から機械で引く（簡単な markdown の描画だけ）。**「要点」の枠だけが AI の書いたもの**で、
画面でもそう明示する。要点は原文が変わっても追従しないので、日付を添えて古くなったことが分かるようにする。
"""
import argparse
import html
import re
import shutil
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
e = html.escape

# ---------------------------------------------------------------- AI が書いたもの（要点）

SUMMARY_DATE = "2026-10-11"

UI_POINTS = [
    "画面は 8 つ（S-1 稼働状況 / S-2 1 日を見る / S-3 主観入力 / S-4 検索 / S-5 記録の詳細 / S-6 場所・人物・属性 / S-7 設定 / S-8 ログイン）。入口は S-2",
    "骨格: 時系列を主にしたリスト。1 行の単位は「滞在」、詳細は画面を移らずその場で開く。いちばん大きく出すのは場所の名前",
    "表面: 苔の緑（色相 132°）・彩度 30%・暗く沈んだ地・角の丸み 15px・ゆったりした密度。決めた理由は本人の「落ち着いていてゆったりと見れる感じなので」",
    "守る下限は要件の NFR-17〜23（明暗は OS に追従・文字のコントラスト 4.5:1・触れる対象 24px・削除と停止は 44px・原色と角張りを避ける・フォーカスが見える）",
    "決めていないもの: アイコン・アニメーション・色のトークン値・画面ごとの幅の閾値",
    "2026-10-11 の要件の改訂とずれる所: S-7 設定の「感度・プラグイン」は、衛星ごとの承認の画面（PERM-11〜14）に変わる。UI を一新するならこの文書を書き直す",
]

PREP_POINTS = [
    "言語: サーバ Rust / Android の収集 Kotlin / Windows の収集 Rust / 画面 TypeScript + React（ブラウザ 1 本で PC とスマホの両方に載せる）",
    "書き込みは取り込み口 1 本に集める（重複の判定・原文の保存・稼働記録を必ず通す）",
    "**ずれ**: 読み出しは「PostgREST でスキーマから自動生成」と書いてあるが、実装は自前の読み出し（`/events` など）で、PostgREST は動いていない",
    "ライセンス: 本体は AGPL-3.0、貢献は許諾型の CLA の下でだけ受ける、衛星は別プロセスの HTTP だけ（FR-77）",
    "移行は前進のみで、名前は作成時刻。ログには位置・本文・人名などの私的データを出さない。バックアップの鍵は Windows の資格情報ストアと紙の写し",
    "C（検査）: 依存のライセンス・層の境界・移行の安全・API 契約のずれ・ログの私的データ・人間の確認に名前を要求、などを CI が機械で見る",
    "2026-10-11 の要件の改訂とずれる所: 「呼び出し元ごとの権限は ST24 / ST27 / ST29 の担当」とあるが、権限は衛星ごとの承認（PERM-11〜15）に変わった",
]

SPECS_NOTE = ("**data-sensitivity** は 2026-10-11 の要件の改訂とずれる —— Purpose に「記録ごとの感度とプラグインの権限も、"
              "この capability に積む」とあるが、権限は衛星ごと・種類ごとの承認と、記録ごとの「出さない」の印に変わった。"
              "承認を作る Story がこの spec を書き換える")

# ---------------------------------------------------------------- 簡単な markdown の描画（機械）


def inline(s: str) -> str:
    s = e(s)
    s = re.sub(r"\*\*(.+?)\*\*", r"<b>\1</b>", s)
    s = re.sub(r"`(.+?)`", r"<code>\1</code>", s)
    s = re.sub(r"\[([^\]]+)\]\((https?://[^)\s]+)\)", r'<a href="\2">\1</a>', s)
    s = re.sub(r"\[([^\]]+)\]\(([^)\s]+)\)", r"\1", s)
    s = re.sub(r"~~(.+?)~~", r"<s>\1</s>", s)
    return s


def md(text: str) -> str:
    lines = text.splitlines()
    if lines and lines[0].strip() == "---":           # frontmatter
        end = next((i for i, l in enumerate(lines[1:], 1) if l.strip() == "---"), 0)
        lines = lines[end + 1:]
    out, para, i = [], [], 0

    def flush():
        if para:
            out.append(f"<p>{inline(' '.join(para))}</p>")
            para.clear()

    while i < len(lines):
        ln = lines[i]
        s = ln.strip()
        if s.startswith("```"):
            flush()
            j = i + 1
            while j < len(lines) and not lines[j].strip().startswith("```"):
                j += 1
            out.append(f"<pre>{e(chr(10).join(lines[i + 1:j]))}</pre>")
            i = j + 1
            continue
        m = re.match(r"^(#{1,6}) (.*)", ln)
        if m:
            flush()
            n = len(m.group(1))
            out.append(f"<h{min(n + 1, 6)}>{inline(m.group(2))}</h{min(n + 1, 6)}>")
            i += 1
            continue
        if s.startswith("|"):
            flush()
            rows = []
            while i < len(lines) and lines[i].strip().startswith("|"):
                cells = [c.strip() for c in lines[i].strip().strip("|").split("|")]
                if not all(re.fullmatch(r":?-{3,}:?", c) for c in cells if c):
                    rows.append(cells)
                i += 1
            if rows:
                head = "".join(f"<th>{inline(c)}</th>" for c in rows[0])
                body = "".join("<tr>" + "".join(f"<td>{inline(c)}</td>" for c in r) + "</tr>" for r in rows[1:])
                out.append(f'<div class="tbl"><table><thead><tr>{head}</tr></thead><tbody>{body}</tbody></table></div>')
            continue
        if s.startswith(">"):
            flush()
            q = []
            while i < len(lines) and lines[i].strip().startswith(">"):
                q.append(re.sub(r"^\s*>\s?", "", lines[i]))
                i += 1
            out.append(f"<blockquote>{md(chr(10).join(q))}</blockquote>")
            continue
        m = re.match(r"^(\s*)(?:[-*]|\d+\.) (.*)", ln)
        if m:
            flush()
            items = []
            while i < len(lines):
                m = re.match(r"^(\s*)(?:[-*]|\d+\.) (.*)", lines[i])
                if m:
                    items.append([len(m.group(1)), m.group(2)])
                elif lines[i].strip() and lines[i].startswith(" ") and items:
                    items[-1][1] += " " + lines[i].strip()
                else:
                    break
                i += 1
            lis = "".join(f'<li style="margin-left:{ind // 2 * 1.2}em">{inline(t)}</li>' for ind, t in items)
            out.append(f"<ul>{lis}</ul>")
            continue
        if not s:
            flush()
        elif s == "---":
            flush()
            out.append("<hr>")
        else:
            para.append(s)
        i += 1
    flush()
    return "\n".join(out)


def md_sections(text: str) -> str:
    """`## ` ごとに畳む。長い文書をスマホで読むため（見出しだけ並んで、開いた所だけ読む）。"""
    parts = re.split(r"(?m)^(?=## )", text)
    out = [md(parts[0])]
    for p in parts[1:]:
        title, _, body = p.partition("\n")
        out.append(f'<details class="sec"><summary>{inline(title[3:])}</summary>{md(body)}</details>')
    return "\n".join(out)


# ---------------------------------------------------------------- ページ

CSS = """
:root{--bg:#f3f6f4;--surface:#fff;--fg:#1d2621;--muted:#5d6b63;--line:#d5ddd8;--accent:#2e6b4f;--accent-soft:#e2efe8;
--ai:#a5560a;--ai-soft:#fbecd8;--body:"Noto Sans JP","Hiragino Sans","Yu Gothic",sans-serif;--mono:ui-monospace,Menlo,monospace}
@media (prefers-color-scheme:dark){:root:not([data-theme="light"]){--bg:#121815;--surface:#1a221e;--fg:#e2ebe6;--muted:#97a69e;
--line:#2b3731;--accent:#6cc79a;--accent-soft:#1b3328;--ai:#f0a85a;--ai-soft:#3a2a14;color-scheme:dark}}
:root[data-theme="dark"]{--bg:#121815;--surface:#1a221e;--fg:#e2ebe6;--muted:#97a69e;--line:#2b3731;--accent:#6cc79a;
--accent-soft:#1b3328;--ai:#f0a85a;--ai-soft:#3a2a14;color-scheme:dark}
*{box-sizing:border-box}
body{background:var(--bg);color:var(--fg);font-family:var(--body);font-size:15px;line-height:1.75;margin:0}
.wrap{max-width:820px;margin:0 auto;padding:24px 16px 72px}
h1{font-size:1.5rem;margin:0 0 .3em;line-height:1.4}
h2,h3,h4,h5{line-height:1.45;margin:1.4em 0 .4em}
a{color:var(--accent)}
.lead{color:var(--muted);font-size:.9rem}
.back{font-size:.85rem}
.ai{background:var(--ai-soft);border:1px solid var(--ai);border-radius:10px;padding:12px 16px;margin:16px 0}
.ai h2{margin:0 0 .3em;font-size:1rem;color:var(--ai)}
.ai ul{margin:0;padding-left:1.2em}
.ai .tag{font-size:.75rem;color:var(--ai)}
details.sec,details.card{background:var(--surface);border:1px solid var(--line);border-radius:10px;padding:8px 14px;margin:8px 0}
details>summary{cursor:pointer;font-weight:700;min-height:28px}
details.req{border-top:1px solid var(--line);padding:4px 0}
details.req>summary{font-weight:500}
code{font-family:var(--mono);font-size:.85em;background:var(--accent-soft);padding:0 4px;border-radius:4px;overflow-wrap:anywhere}
pre{font-family:var(--mono);font-size:.8rem;background:var(--bg);padding:10px;border-radius:6px;overflow-x:auto;white-space:pre-wrap}
blockquote{margin:.6em 0;padding:2px 12px;border-left:3px solid var(--line);color:var(--muted)}
.tbl{overflow-x:auto}
table{border-collapse:collapse;font-size:.85rem;margin:.6em 0;min-width:100%}
th,td{border:1px solid var(--line);padding:4px 8px;vertical-align:top;text-align:left}
th{background:var(--accent-soft)}
ul{padding-left:1.2em}
.count{font-family:var(--mono);color:var(--muted);font-weight:400;font-size:.85em}
"""


def page(title: str, desc: str, body: str) -> str:
    return (f'<!doctype html><html lang="ja"><meta charset="utf-8">'
            f'<meta name="viewport" content="width=device-width, initial-scale=1"><title>{e(title)}</title>'
            f'<meta name="description" content="{e(desc)}"><style>{CSS}</style>'
            f'<div class="wrap"><p class="back"><a href="./">← 読み物の一覧</a></p><h1>{e(title)}</h1>{body}</div></html>')


def points(items: list[str]) -> str:
    lis = "".join(f"<li>{inline(x)}</li>" for x in items)
    return (f'<section class="ai"><h2>要点 <span class="tag">AI が {SUMMARY_DATE} に書いた要約。正しいのは下の原文</span></h2>'
            f"<ul>{lis}</ul></section>")


def src(path: str) -> str:
    rev = subprocess.run(["git", "log", "-1", "--format=%cs %h", "--", path], cwd=ROOT,
                         capture_output=True, text=True).stdout.strip()
    return f'<p class="lead">原文: <code>{e(path)}</code>（最後の変更 {e(rev or "—")}）。見出しを押すと開きます。</p>'


def specs_page() -> str:
    out = [f'<p class="lead">原文: <code>openspec/specs/*/spec.md</code>。Story を archive したときに確定した「実際の振る舞い」の正典です。'
           "capability ごとに要求の題名を並べ、押すと本文と Scenario が開きます（機械で引くだけ）。</p>",
           f'<section class="ai"><h2>注意 <span class="tag">AI が {SUMMARY_DATE} に書いた</span></h2><p>{inline(SPECS_NOTE)}</p></section>']
    for spec in sorted((ROOT / "openspec/specs").glob("*/spec.md")):
        t = spec.read_text(encoding="utf-8")
        purpose = re.search(r"## Purpose\n(.*?)\n## ", t, re.S)
        reqs = re.split(r"(?m)^### Requirement: ", t)[1:]
        inner = "".join(f'<details class="req"><summary>{inline(r.partition(chr(10))[0])}</summary>'
                        f"{md(r.partition(chr(10))[2])}</details>" for r in reqs)
        out.append(f'<details class="sec"><summary>{e(spec.parent.name)} <span class="count">{len(reqs)}</span></summary>'
                   f'<p class="lead">{inline(" ".join((purpose.group(1) if purpose else "").split()))}</p>{inner}</details>')
    return "".join(out)


def handoff_page() -> str:
    d = ROOT / "docs/handoff"
    out = [f'<p class="lead">原文: <code>docs/handoff/</code>。issue ができて凍結した Story へ、ほかの Story が見つけたことを置く場所です'
           "（走っている Story へは差し戻さない）。残っている借りの一覧として読めます。</p>"]
    readme = d / "README.md"
    if readme.exists():
        out.append(f'<details class="sec"><summary>この置き場の決まり（README）</summary>{md(readme.read_text(encoding="utf-8"))}</details>')
    for f in sorted(d.glob("ST*.md")):
        t = f.read_text(encoding="utf-8")
        out.append(f'<details class="sec"><summary>{e(f.stem)} <span class="count">{t.count(chr(10))} 行</span></summary>{md(t)}</details>')
    return "".join(out)


def build(outdir: Path) -> None:
    outdir.mkdir(parents=True, exist_ok=True)
    for old in outdir.glob("*.html"):
        old.unlink()
    r = subprocess.run([sys.executable, str(ROOT / "tools/requirements_brief.py"), "-o", str(outdir / "01-requirements.html")],
                       capture_output=True, text=True)
    print(r.stdout.strip() or r.stderr.strip())
    req = outdir / "01-requirements.html"
    if req.exists():                                   # 一覧へ戻る道と、操作盤の一覧に出す説明を足す
        t = req.read_text(encoding="utf-8")
        t = t.replace("<title>", '<meta name="description" content="要件定義を、まとまりごとに一行で。新規・改訂の印つき"><title>', 1)
        t = t.replace('<div class="wrap">', '<div class="wrap"><p style="font-size:.85rem"><a href="./">← 読み物の一覧</a></p>', 1)
        req.write_text(t, encoding="utf-8")

    ui = (ROOT / "docs/ui-direction.md").read_text(encoding="utf-8")
    (outdir / "02-ui-direction.html").write_text(page(
        "UI の方向", "画面の骨格・主役・色と形をどう決めたか。要点つき",
        points(UI_POINTS) + '<p><a href="02-ui-playground.html">決めたときの playground（触れる見本）を開く</a></p>'
        + src("docs/ui-direction.md") + md_sections(ui)), encoding="utf-8")
    pg = ROOT / "docs/ui-direction-playground.html"
    if pg.exists():
        shutil.copyfile(pg, outdir / "02-ui-playground.html")

    prep = (ROOT / "docs/production-prep.md").read_text(encoding="utf-8")
    (outdir / "03-production-prep.html").write_text(page(
        "製造準備", "言語・ライセンス・移行・検査など、全 Story に効く決めごと。要点つき",
        points(PREP_POINTS) + src("docs/production-prep.md") + md_sections(prep)), encoding="utf-8")

    (outdir / "04-specs.html").write_text(page(
        "仕様の正典（openspec/specs）", "archive した Story が確定させた振る舞い。capability ごとの要求の一覧",
        specs_page()), encoding="utf-8")
    (outdir / "05-handoff.html").write_text(page(
        "申し送り（handoff）", "凍結した Story へ残っている借りの一覧", handoff_page()), encoding="utf-8")

    lic = ROOT / "docs/audit/2026-10-09-license-map.html"
    if lic.exists():
        t = lic.read_text(encoding="utf-8")
        t = t.replace("<title>", '<meta name="description" content="AGPL・CLA・衛星の有料化など、ライセンスで何が要るか（2026-10-09）"><title>', 1)
        (outdir / "06-license.html").write_text(t, encoding="utf-8")

    cla = (ROOT / "docs/CLA.md").read_text(encoding="utf-8")
    (outdir / "07-cla.html").write_text(page(
        "貢献者ライセンス同意（CLA）", "貢献を受けるときに署名してもらう文書（全文）",
        src("docs/CLA.md") + md(cla)), encoding="utf-8")
    print(f"{outdir}: " + " ".join(sorted(p.name for p in outdir.glob("*.html"))))


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("-o", "--out", default=str(ROOT / "docs/briefs/reading"))
    build(Path(ap.parse_args().out))


if __name__ == "__main__":
    main()
