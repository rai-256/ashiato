# CH01: ST17 の /code-review の 9 件を直す

ST17（毎日の気分。archive 済み）に組み込みの `/code-review` を当てて出た 9 件（依頼は `docs/changes/CH01.md`）を、直すか、直さない理由を残すかで閉じる change。
change: `openspec/changes/ch01-st17-review-fixes/`

## 未決（A）

なし（`deep.md` に `(未回答)` は無い）。

## 仮決め（B）と Ruling

- 仮決め: D1・D2・D5・D6・D7・D8・D9（深掘りの Q1〜Q7。本人の答えは推奨どおり）・D11（review/code.md R1・R19・R22。台帳 `provisional.jsonl` に載せた。別の席の確かめは `provisional_review`）・D12（review/code.md R13）。反転条件はどれも `design.md` にある
- Task の ledger（`.superpowers/sdd/ch01-task-*/progress.md`）の `Ruling:` 行: **無い**（9 Task とも ledger に Ruling / parked の行を持たない）
- finish の scoped re-review の後に park した Minor 2 件の Ruling（`review/code.md` の末尾）:
  - `tools/stack.sh` の作り直しの判定はサーバの依存の別 crate を見ない —— Ruling: 今のサーバは path 依存を持たないので起きない。依存を足す Story が監視の対象も足す
  - 主観の画面の受理された枝のひとことの控えは最後に押した 1 件かを見ない —— Ruling: 起きる窓はほぼ無く、起きても同じひとことを 1 件多く送るだけで値は失われない

## Task

9 Task をすべて埋めた（`- [x]` 22・`- [ ]` 0）。残した Task は無い（`ci` / `human` の残りも無い）。画面の Scenario は `web/e2e/feel-ch01.spec.ts`。

## レビューの処置（`review/code.md`。51 件 —— deep 7・spec 20・code 24）

- final review R1〜R9: fix wave 1 回 + scoped re-review で ADDRESSED（R1 は D11（仮））。R10 は finish へ
- code_verify R11〜R15・security review（指摘なし）・`/code-review` R16〜R24 を finish で処置し、scoped re-review を 1 回（全件 ADDRESSED / rejected の理由が成り立つ）
  - 直した: 主観の画面で応答の順が入れ替わると後の値の印・送り直し・ひとことの控えが消える（R10・R17・R18・R21）/ 日の欄は断られても値を残す（Q4）の試験（R15）/
    `check-shared-appends.sh` の守る範囲が節の前の準備で狭まっていた（R19）/ `app_psql` の二重定義（R22）/ `stack.sh` が古い dist・バイナリで e2e を緑にする（R11）/
    録画の Windows の写しにトークンが残る（R16）/ 主観の届かない枝と順の固定（R23）/ ST23 への申し送りの論理ソースの誤り（R14）/ 見出しの型の限界を文書に（R12）
  - **kind を付け替えたもの 1 件**: R11 は premise → technical（検証コマンドの文字列は変えず、コマンドが頼る `tools/stack.sh` の欠陥を直した。失うものは無い）
  - rejected 2 件: R20（`# --- ST<NN>:` の小見出しを落とすのは D2 と Q2 の決定どおり。注記と self-test を足した）/ R24（`refactor(CH01)` の接頭辞。直すと 40 本ほどの commit の SHA が変わり、記録の参照を失う）

## 通した検証（`evidence.jsonl` の各項目の最後の PASS。終了コードはどれも 0）

| 項目 | コマンド（要約） | rc |
|---|---|---|
| 1.1 / 1.2 | `tools/check-shared-appends.sh --self-test` | 0 |
| 2.1 / 2.2 / 2.3 | `tools/check-immutable.sh`（移す前・後の出力の比較、壊した down.sql） | 0 |
| 2.4 | `tools/check-shared-appends.sh "$(git branch --show-current)"` | 0 |
| 3.1 / 3.2 | `bash -n tools/record-env.sh` / gradle の BuildConfig | 0 |
| 4.1 / 4.2 | `CT ch01_authored_` ほか / `cargo clippy -p ashiato-server --all-targets -- -D warnings` | 0 |
| 5.1 / 5.3 | `CT ch01_stay_open_` / `tools/check-handoff-reaches-brief.sh ST23 ch01-st17-review-fixes`（5.3 は plan_fix の機械の承認で訂正） | 0 |
| 6.1 / 6.2 / 7.1 / 7.2 / 8.1 | `VT FeelView.test.tsx`・`ET feel-ch01.spec.ts` ほか・typecheck / lint | 0 |
| 9.1 | validate・check_scenarios・log-private・fmt・check-openapi・`npm run build` ほか | 0 |

finish の直しの後に手元で走らせたもの: vitest `FeelView.test.tsx` 40 passed（新しい 3 本は直す前の `FeelView.tsx` で落ちる）・e2e `feel` `feel-ch01` `feel-parts` `feel-limits` 51 passed（`stack.sh` がソースより古いサーバと dist を作り直した）・
`CT ch01_authored_` 15 passed・`check-immutable.sh` rc=0（OK 138 行は前と同じ、NG なし）・`check-shared-appends.sh --self-test` 26 件 ok・clippy / fmt / typecheck / lint / `openspec validate --strict` / `check_scenarios.py` / `check-log-private.sh` / `check-handoff-reaches-brief.sh` rc=0・
shellcheck `-S warning`（変えた 3 本）rc=0。

## 申し送り

- この change から ST23 へ: `docs/handoff/ST23.md` の `## ch01-st17-review-fixes D9`（本文を消した滞在を紐づけ先にする気分の受け付け方は ST23 が決める）
- この change への申し送り（`docs/handoff/CH01.md`）: 無い
- 未処置（範囲外）: security review の範囲外の観察 —— `tools/record-env.sh` の Windows の写しが `.env`（DB・画面の合言葉と token）を C: のキャッシュに写し、片付けでも消さない（差分の前から。値は実行ごとの乱数）

🤖 Generated with [Claude Code](https://claude.com/claude-code)
