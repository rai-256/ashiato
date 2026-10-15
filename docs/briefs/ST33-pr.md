# ST33 全記録を外部の道具で読める形に書き出す（下流・実装）

change: `openspec/changes/st33-data-export/`。capability `data-durability`（新規）。
S-7（`#/settings`）の書き出しの節から、全記録を合言葉で暗号化した 1 つの zip（AES-256）として自宅 PC の決まったフォルダへ書き出す。

## 未決（A）

なし。

- **Q12（code-verify R20。irreversible / loss: rewrite-all）は答えが出た**（2026-10-16、`deep-answers-7.txt`）: (a) いまの形で凍結する・読めない形の行は「数えられない」と出す（推奨と同じ）。
  入れたもの（4a7af06）: 固定の 3 つの UUID を入れたフィルタのバイト列を別の実装（Python の `hashlib`）で作って直書きした試験 `export::bloom::tests::the_frozen_shape_is_kept`。
  読めない形・知らない版の `written` は `last.erased_after: null` で返し、画面は「件数を数えられません」の 1 行を出す（`docs/openapi.json` の `erased_after` は `integer | null` になった）。design D11 / D17 に書いた

## 仮決め（B）と Ruling

全 Task の ledger（`.superpowers/sdd/st33-task-*/progress.md`）のファイルは無い。`Ruling:` の行は Task 3 の報告（`task-3-report.md`）に 4 件あり、ここへ逐語で写した（workspace はこの後に消す）:

- Ruling: 見込みの `bytes/rows` の rows は前回 finish の `rows->>'events.jsonl'` — D18 の rows が記録の行だけだから — 違えば見込みの係数がずれるだけ
- Ruling: `seconds` は切り上げて 1 未満は 1 — 見込みは長めが安全 — 表示が 1 秒ずれるだけ
- Ruling: 書庫のソースを `Point` に足さない — ST12 の型で出来事 1 点と確かめていない — 除き過ぎの側（戻る）
- Ruling: (a) は「最初に読めた鍵」を使い、それが始まりより前なら (b) へ（次の鍵は試さない）— design 文言どおり

Task の他の判断は `design.md` の（仮）と各 Task の findings に残っている。
`design.md` の（仮）と反転条件:

- D2 書き出しは同時に 1 本だけ（2 本目は 409）
- D3 ファイル名の形と、失敗した `.partial-…` を消すこと
- D7 zip の中に JSON Lines（Q2）
- D13 状態を 1 秒ごとに読む（応答を待つ `setTimeout` の連鎖）・件数を開くたびに数える・確認の文面。
  **判定（final review R4）**: 35 万行（1 年分）で件数の数え 1 回 6.5 秒。D13 の反転条件（3 秒）に当たるが、開いたときに 1 回だけ数えるので形は替えなかった。本人に見てほしい点
- D16 置き場には書き終えた書き出しを 1 つだけ置く（Q4）。`done` にできなかった zip は消す（R2 / R17）
- D17 後に消した記録の件数はブルームフィルタで数え、（`last`・削除の台帳・前の版・吸収の台帳の行数）の鍵で覚える（R1）。フィルタの形は凍結（Q12 で本人が決めた。仮ではない）
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
| 8.1〜8.3 | `npm run test -- export` / `SettingsView` / `settings-contrast` | PASS | 8b92ef2 |
| 9.1 | `npm run test:e2e -- offsite` | PASS | cbaf400 |
| 10.1 | `check_scenarios.py` / `review_triage.py` / `check_chain.py` | PASS（rc=0） | 8b92ef2 |
| 10.2 | `cargo fmt --check` / `clippy -D warnings` / web の lint・test・build・e2e（`STACK_RESET=1`）/ `check-immutable` / `check-offsite` / `check-log-private` / `smoke` | PASS（rc=0） | 8b92ef2 |
| 10.2 | `cargo test --workspace` | **FAIL（rc=101）** —— lib 681 passed・`server_startup` 7 本中 2 本 FAILED（`migrate が失敗した`） | 8b92ef2 |

**10.2 の `cargo test --workspace` の FAIL は ST33 の差分の外**（R20 の fix 4a7af06 の後に、いまの木で取り直した）:
- この branch の `crates/server/tests/server_startup.rs` は `.env` の `DATABASE_OWNER_URL` の port（55432 = main の DB）へそのまま繋ぐ。この worktree の DB は 55533（`tools/ports.sh`）。55432 に DB が無いと `migrate` が `pool timed out` で落ちる
- main は ST21 の `a9b84803`（起動の統合試験も DB の port を worktree の規則に合わせる）で直している。ST33 はこのファイルを触っていない
- 確かめたこと: port だけを 55533 に替えて（`tools/db-roles.sh` で役割を整えた後）`cargo test -p ashiato-server --test server_startup` → **7 passed / 0 failed**
- gate が origin/main に載せた木で 10.2 を取り直せば通るはず（載せていない木では、手で繋ぎ先を替えないかぎり落ちる）

## レビュー

- final review（6d7d65f..b74736b）: Important 4 / Minor 12（R1〜R16）。R11（`drops_tests` の deadlock）は rejected（この branch の差分の外・再現せず）、他は fixed
- code-verify（d81e27a）: R17〜R24。fixed 6（R17 / R18 / R19 / R21 / R22 / R24）・escalated 1（R20 → Q12。答えの後に入れた）・deferred 1（R23 → ST30）。scoped re-review は全部 ADDRESSED・新しい壊れ無し
- R20 の fix（085b6f7..4a7af06）の scoped re-review: ADDRESSED・新しい壊れ無し

## 申し送り（未処置）

- **R18 の残り**: 途中で止まった書き出しが 2 つ以上残るときは新しい 1 件だけ状態に出る。`.partial-…` は状態に出ない。止まった zip の中の、後に消した場面は数えない
- **R23 → ST30**: D-01 を戻した後、台帳に無い前の書き出しが置き場に残り続ける（`docs/handoff/ST30.md`）
- **ST17**: `s01-feeling` を書き出しの分類（`NO_COORD_SOURCES`）に足すのは後から archive される側（`docs/handoff/ST17.md` / `ST33.md` の C1）
- R11: `drops_tests::drops_api_idempotent` が共有 DB で `40P01 deadlock` を出したことがある（ST33 の差分の外）

🤖 Generated with [Claude Code](https://claude.com/claude-code)
