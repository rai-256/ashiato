# ST33 全記録を外部の道具で読める形に書き出す（下流・実装）

change: `openspec/changes/st33-data-export/`。capability `data-durability`（新規）。
S-7（`#/settings`）の書き出しの節から、全記録を合言葉で暗号化した 1 つの zip（AES-256）として自宅 PC の決まったフォルダへ書き出す。

## 未決（A）—— 本人の答えを待っている

- **Q12（code-verify R20。irreversible / loss: rewrite-all）**: 台帳に置く「書いた記録」のフィルタ（`written`）の形を、いまの版で凍結するか。
  形を変えた後のサーバは、それより前の書き出しの「後に消した記録の件数」を 0 と出す（少なく数える側）。台帳は追記のみなので行を書き直せない。
  推奨は「いまの形で凍結し、読めない形の行は『数えられない』と出す」。問いは `docs/briefs/ST33-deep-r4.html`（`deep-questions-r4.json`）、本文は `deep.md` の第 5 回。
  答えの後に、固定の期待値の試験と「数えられない」の表示を入れる

## 仮決め（B）と Ruling

全 Task の ledger（`.superpowers/sdd/st33-task-*/progress.md`）に `Ruling:` の行は **0 件**（ledger のファイル自体が無い。Task の判断は `design.md` の（仮）と各 Task の findings に残っている）。
`design.md` の（仮）と反転条件:

- D2 書き出しは同時に 1 本だけ（2 本目は 409）
- D3 ファイル名の形と、失敗した `.partial-…` を消すこと
- D7 zip の中に JSON Lines（Q2）
- D13 状態を 1 秒ごとに読む（応答を待つ `setTimeout` の連鎖）・件数を開くたびに数える・確認の文面。
  **判定（final review R4）**: 35 万行（1 年分）で件数の数え 1 回 6.5 秒。D13 の反転条件（3 秒）に当たるが、開いたときに 1 回だけ数えるので形は替えなかった。本人に見てほしい点
- D16 置き場には書き終えた書き出しを 1 つだけ置く（Q4）。`done` にできなかった zip は消す（R2 / R17）
- D17 後に消した記録の件数はブルームフィルタで数え、（`last`・削除の台帳・前の版・吸収の台帳の行数）の鍵で覚える（R1）
- D18 確認に出す見込み（行・大きさ・時間）
- **D20（新）** 台帳の始まりの行は `POST /export` の求めの中で書き、書けなければ 500 `ledger_unavailable`（code-verify R22）。反転条件: `POST` の応答が 1 秒を超えるなら背景で書き、失敗をメモリに持って `last_failure` に出す

## Task

10 Task（32 項目）すべて `[x]`。残した Task は無い（`ci` / `human` の残りも無し）。Scenario 98 本（サーバ 77 は Rust、画面 21 は `web/e2e`）。

## 検証（`evidence.jsonl`。項目ごとに最新）

| 項目 | コマンド | 結果 | head |
|---|---|---|---|
| 1.1 | `tools/check-migrations.sh` | PASS | 6d7d65f |
| 1.3 | `tools/check-immutable.sh`（export_ledger） | PASS | 6d7d65f |
| 7.1 | `tools/check-openapi.sh` | PASS | b647afd |
| 8.1〜8.3 | `npm run test -- export` / `SettingsView` / `settings-contrast` | PASS | cf115de |
| 9.1 | `npm run test:e2e -- offsite` | PASS | cbaf400 |
| 10.1 | `check_scenarios.py` / `review_triage.py` / `check_chain.py` | PASS（rc=0） | cf643f1 |
| 10.2 | `cargo fmt --check` / `clippy -D warnings` / `cargo test --workspace`（816 passed）/ web の lint・test・build・e2e（65 passed）/ `check-immutable` / `check-offsite` / `check-log-private` / `smoke` | PASS（rc=0） | 7edd504 |

**10.2 の証跡は code-verify の fix（13b123b）より前の木。** fix の後に走らせたのは `cargo test -p ashiato-server --lib export`（129 passed / 0 failed）・`cargo fmt` / `clippy -D warnings`（rc=0）・`tools/check-openapi.sh`（rc=0）。web のコードは変えていない。gate の前に 10.2 を取り直す必要がある。

## レビュー

- final review（6d7d65f..b74736b）: Important 4 / Minor 12（R1〜R16）。R11（`drops_tests` の deadlock）は rejected（この branch の差分の外・再現せず）、他は fixed
- code-verify（d81e27a）: R17〜R24。fixed 6（R17 / R18 / R19 / R21 / R22 / R24）・escalated 1（R20 → Q12）・deferred 1（R23 → ST30）。scoped re-review は全部 ADDRESSED・新しい壊れ無し

## 申し送り（未処置）

- **R18 の残り**: 途中で止まった書き出しが 2 つ以上残るときは新しい 1 件だけ状態に出る。`.partial-…` は状態に出ない。止まった zip の中の、後に消した場面は数えない
- **R23 → ST30**: D-01 を戻した後、台帳に無い前の書き出しが置き場に残り続ける（`docs/handoff/ST30.md`）
- **ST17**: `s01-feeling` を書き出しの分類（`NO_COORD_SOURCES`）に足すのは後から archive される側（`docs/handoff/ST17.md` / `ST33.md` の C1）
- R11: `drops_tests::drops_api_idempotent` が共有 DB で `40P01 deadlock` を出したことがある（ST33 の差分の外）

🤖 Generated with [Claude Code](https://claude.com/claude-code)
