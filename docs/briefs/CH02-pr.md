# CH02 書き出しの「次の記録」の引きを行数の 2 乗で遅くならないようにする（上流）

change: `openspec/changes/ch02-export-next-record-window/`。触る capability は `data-durability`（MODIFIED 1・ADDED 1）。依頼の逐語と完了の判定は `docs/changes/CH02.md`。

> **第 1 回の深掘りの答えを待たずに、推奨の答えで先に書いた**（2026-10-20 の流れ）。下の Q1〜Q4 は**本人の答えではなく推奨**で、`deep.md` は `（答え待ち・推奨: …）` のまま。
> 答えが推奨と違えば `spec --answers` が仕様に写し直す。

## 深掘り（第 1 回。推奨で書いた）

| Q | 段 | 推奨（仕様はこれで書いた） | どこに効くか |
|---|---|---|---|
| Q1 同じ時刻に並んだ記録の幅 | **A**（`loss: exported`。本人の答えが要る） | 時刻が後の最初の記録にする（同じ時刻の記録は飛ばす）。除く行は増える側にだけ変わる | spec の (c) の本文・Scenario 1 本、design D1 / D4、tasks 3.2 |
| Q2 変わらないことの確かめ方 | B | いまの SQL を試験に残し、まっさらな DB の偽データで新旧を突き合わせる ＋ 既存の試験を全部通す | design D6、tasks Task 1・3・6 |
| Q3 「線形に近い」の示し方 | B | 読んだ行の数を N と 4N で比べる試験 ＋ 量の試験で時間を 1 回記録 | spec の ADDED、design D7 / D8、tasks Task 4・5 |
| Q4 消した時間が無い・古いときの絞り込み | B | 残す（0 件なら記録の表を読まない。消した時間の終わりより後は組ごとの最初の 1 行だけ） | spec の ADDED、design D3、tasks 2.2・4.3 |

Q3 には spec-review の前提の訂正（R3: いまの引きが 2 乗になるかは計画しだい）を同じ 1 枚で載せた。推奨は変わらない。

## 仮決め（B）と反転条件（design の（仮））

- **D3（仮）** 窓の前に時刻だけで絞る（One-Time Filter の門・`max(end_at)` 以下の行と組ごとの後の最初の 1 行）。反転条件: 絞り込みの読み違いが突き合わせで残るなら外して全行に窓を当てる／120 万行で 3 秒以上なら形を見直す
- **D7（仮）** 線形の固定は、端末が入り交じる配置で `ANALYZE` 後の読んだ行の数を N と 4N で比べる（8 倍未満。いまの SQL では 8 倍以上を assert）。窓の枠の退行は断片の文字列で固定する。反転条件: 読んだ行の数が計画で揺れるならバッファの数に替える
- **D8（仮）** 時間は既存の量の試験で 1 万・10 万・120 万行 × 消した滞在あり・無しを 1 回測り、ST33 D13（件数の数え 3 秒以上）の判定に渡す（替えるかは本人）。反転条件: 120 万行が 30 分で回らなければ 10 万行まで
- **D9（仮）** `docs/changes/CH02.md` の `capability` は空のまま（ST09 と Requirement も行も重ならない）。反転条件: 同じ Requirement を MODIFIED する上流が入るなら書く
- **D10（仮）** ST09 の export の試験は CH02 の worktree で ST09 のブランチを一時に merge して走らせ、abort で戻す（commit も push もしない）。反転条件: ST09 が先に main に入れば通常の試験で見る

仮でない決定: D1（逆向きの並びで頭を固定した `RANGE` 枠の `min`。順向きで尾を固定した枠は 2 乗に戻る）・D2（窓には鍵の列だけ）・D4（記録と前の版を 1 つの並びで）・D5（変えないものと、それを何が固定するか）・D6（新旧の突き合わせ）。
ST33 design D5 の「`lead()` ではなく相関副問い合わせ」の判断を覆す（理由は proposal）。移行・索引は足さない。

## capability の前倒し

なし。`data-durability` の既存の Requirement「書き出しは本人が消した場面を出さない」を MODIFIED（全文と既存 17 本の Scenario を逐語で写し、(c) の書き足しと Scenario 9 本を足した）、
「書き出しから除く範囲の判定は、記録の行数の 2 乗で伸びない」を ADDED（Scenario 2 本）。走っている ST09 は同じ capability に別の Requirement を ADDED するだけで重ならない。

## tasks

6 Task・18 項目。新しく足した Scenario 11 本は全部 Task 2・4 が名指しする。画面は持たないので e2e と人間の確認待ちは無い。確認バッチでは、本人が計測の表（design の「計測の記録」）を見て ST33 D13 を替えるかを決める。

## レビュー

- deep-review（`review/deep.md`）: 10 件（escalated 4 → 問い Q1〜Q4・fixed 5・defer 1）
- spec-review（`review/spec.md`）: 16 件 —— fixed 12・fixed 仮 2（R12 → D9、R13 → D10）・escalated 1（R3 → Q3 に前提の訂正）・rejected 1（R9: 「壊してはいけないもの」は record の段が書く）。
  実測で見つかった 2 件（R2: 同じ端末の配置ではいまの SQL も 4 倍で、試験が作れない／R16: 読んだ行の数では窓の枠の退行が見えない）は、配置と固定の仕方を直した
- `openspec validate --strict` rc=0・`review_triage.py` rc=0・`check_chain.py` rc=0。`check_scenarios.py` は新しい 11 本が実装前で担保なし（想定どおり）

🤖 Generated with [Claude Code](https://claude.com/claude-code)
