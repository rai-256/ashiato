#!/usr/bin/env python3
"""ST12 の tasks 0.1 —— 正典と change の Requirement の差が「許した差」だけかを見る。stdlib のみ。

    python3 tools/st12_delta_diff.py [<root>]      # 既定の root はカレント

**なぜ harness 側にあるか**: プロジェクトの `scripts/` は harness2 への symlink なので、
Story の change はそこへファイルを足せない（worktree の外で、コミットにも入らない）。
実測 2026-09-18: ST12 の下流（Codex）が task 0.1 でここに当たり、1 行も実装せずに止まった
（`Failed to write file .../ashiato2-st12/scripts/test_st12_delta_diff.py`）。
**tasks が `scripts/` に新しいものを要求したら、harness 側が用意する。**

見るもの: ST04 archive 後の正典 `openspec/specs/collection-coverage/spec.md` と
`openspec/changes/st12-archive-ingestion/specs/collection-coverage/spec.md` の同名 Requirement。

**行を目で分類しない。** Scenario は**ブロック単位で帰属**させ、前書き（Scenario の前）だけ
行ごとに見る —— 行の中身で当てにいくと、強調記号  1 つで分類が外れる（実測: 最初の実装が
`除いて**数える` を取り逃した）。

許した差（tasks 0.1 が名指ししたもの）:

  前書き  予算の 1 文 / 導出元に FR-55 / 「2026-09-15 の変更」の注記 5 行
  既存 Scenario  WHEN/THEN を直した 2 本
  新しい Scenario  3 本（中身は丸ごと許す。**本数は固定する**）

exit 0 = 許した差だけ / 1 = ほかの差がある / 2 = 前提不備（ST04 が archive 前など）
"""
import re
import sys
from pathlib import Path

NAME = "稼働状況は 1 年を週に畳んだ格子で見える"
CANON = "openspec/specs/collection-coverage/spec.md"
CHANGE = "openspec/changes/st12-archive-ingestion/specs/collection-coverage/spec.md"
NEW_SCENARIOS = 3          # tasks 0.1 が名指しした「足した Scenario 3 本」
NOTE_LINES = 5             # 「2026-09-15 の変更」の注記
FIXED_THEN = 2             # WHEN/THEN を直した 2 本


def requirement(path: Path):
    """Requirement の本文を（前書きの行, {Scenario 名: 行}）に分ける。"""
    if not path.is_file():
        print(f"[skip] {path} が無い（ST04 の archive がまだか、置き場が違う）", file=sys.stderr)
        sys.exit(2)
    text = path.read_text(encoding="utf-8")
    m = re.search(rf"^###\s+Requirement:\s*{re.escape(NAME)}\s*$", text, re.M)
    if not m:
        print(f"[skip] {path} に Requirement「{NAME}」が無い（ST04 が archive 済みか確かめる）",
              file=sys.stderr)
        sys.exit(2)
    rest = text[m.end():]
    nxt = re.search(r"^###\s", rest, re.M)
    lines = [ln.rstrip() for ln in (rest[: nxt.start()] if nxt else rest).splitlines() if ln.strip()]
    pre, scen, cur = [], {}, None
    for ln in lines:
        head = re.match(r"^####\s+Scenario:\s*(.+?)\s*$", ln)
        if head:
            cur = head.group(1)
            scen[cur] = []
            continue
        (scen[cur] if cur else pre).append(ln)
    return pre, scen


def main() -> int:
    root = Path(sys.argv[1] if len(sys.argv) > 1 else ".").resolve()
    c_pre, c_scen = requirement(root / CANON)
    g_pre, g_scen = requirement(root / CHANGE)

    fails, notes = [], []

    # ---- 前書き
    added = [l for l in g_pre if l not in set(c_pre)]
    removed = [l for l in c_pre if l not in set(g_pre)]
    budget = [l for l in added if l.startswith("THE SYSTEM SHALL") and "箱の高さ" in l and "除いて" in l]
    derives_new = [l for l in added if l.startswith("導出元:") and "FR-55" in l]
    derives_old = [l for l in removed if l.startswith("導出元:") and "FR-55" not in l]
    note = [l for l in added if l.startswith(">")]
    for label, got, want in (("予算の 1 文", len(budget), 1),
                             ("導出元に FR-55", len(derives_new), 1),
                             ("導出元（正典の側）", len(derives_old), 1),
                             ("「2026-09-15 の変更」の注記", len(note), NOTE_LINES)):
        notes.append((label, got, want))
        if got != want:
            fails.append(f"{label}: {got} 行（許したのは {want} 行）")
    for l in added:
        if l not in budget and l not in derives_new and l not in note:
            fails.append(f"前書きに許していない差 + {l[:130]}")
    for l in removed:
        if l not in derives_old:
            fails.append(f"前書きに許していない差 - {l[:130]}")

    # ---- Scenario（ブロック単位）
    new_names = [n for n in g_scen if n not in c_scen]
    gone_names = [n for n in c_scen if n not in g_scen]
    notes.append(("足した Scenario", len(new_names), NEW_SCENARIOS))
    if len(new_names) != NEW_SCENARIOS:
        fails.append(f"足した Scenario: {len(new_names)} 本（許したのは {NEW_SCENARIOS} 本）"
                     + (f" —— {', '.join(new_names)}" if new_names else ""))
    for n in gone_names:
        fails.append(f"正典にあった Scenario が change に無い: {n}")

    fixed = 0
    for n, g_lines in g_scen.items():
        if n in new_names:
            continue                      # 新しい Scenario は丸ごと許す（本数で縛ってある）
        c_lines = c_scen[n]
        a = [l for l in g_lines if l not in set(c_lines)]
        r = [l for l in c_lines if l not in set(g_lines)]
        ok_a = [l for l in a if l.startswith("- **THEN**") and "箱の高さを除いて数えると" in l]
        ok_r = [l for l in r if l.startswith("- **THEN**") and "箱の高さ" not in l]
        fixed += len(ok_a)
        for l in a:
            if l not in ok_a:
                fails.append(f"Scenario「{n}」に許していない差 + {l[:120]}")
        for l in r:
            if l not in ok_r:
                fails.append(f"Scenario「{n}」に許していない差 - {l[:120]}")
        if len(ok_a) != len(ok_r):
            fails.append(f"Scenario「{n}」の THEN の入れ替えが揃っていない（+{len(ok_a)} / -{len(ok_r)}）")
    notes.append(("WHEN/THEN を直した", fixed, FIXED_THEN))
    if fixed != FIXED_THEN:
        fails.append(f"WHEN/THEN を直した: {fixed} 本（許したのは {FIXED_THEN} 本）")

    print(f"=== ST12 の delta（Requirement「{NAME}」）===")
    print(f"  前書き 正典 {len(c_pre)} 行 / change {len(g_pre)} 行 ・ "
          f"Scenario 正典 {len(c_scen)} 本 / change {len(g_scen)} 本")
    for label, got, want in notes:
        print(f"  [{'ok' if got == want else 'FAIL'}] {label}: {got}（許したのは {want}）")
    for f in fails:
        print(f"  [FAIL] {f}")
    if fails:
        print(f"\nst12-delta: FAIL（{len(fails)} 件）—— 正典に合わせて写し直す（tasks 0.1）")
        return 1
    print("\nst12-delta: OK（許した差だけ）")
    return 0


if __name__ == "__main__":
    sys.exit(main())
