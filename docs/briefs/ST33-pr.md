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

## main への rebase（2026-10-16。merge gate が衝突で落ちた）

gate の rebase が `crates/server/src/lib.rs`（`MIGRATIONS`）と `tools/check-immutable.sh` で衝突した。rebase して解き、main と合わさって初めて出た食い違いを直した:

- **移行の順**: main（#97）で `migrations/ORDER` に移ったので、`202610131200_export_ledger` を ORDER の末尾へ 1 行。`check-immutable.sh` の戻しの検査は ST21 の場所の版より先に ST33 を戻す順（本体の途中の既存の節。`tools/check-shared-appends.sh` は OK）
- **ST21 の場所**: 書き出しが「分類の無いもの: 表 place」で全部落ちた（D8 の検査が狙いどおり名前を挙げた）。design の「下流が始まる時点で入っていれば、そこで分類を足す」に従い、`core.place`（記録を束ねる器。座標も名前も持たない）を `attribute_kind` と同じく `Include`（`places.jsonl`。D7 の項目は 12 → 13）、`s01-place` を緯度経度を持つ側（D5a）・幅は既定の (c)（D5b）に置いた。design D7 と冒頭の「まだ main に無いもの」に ★ 2026-10-16 で書いた。tasks.md 5.2 の Scenario の注記「D7 の 12 個」（123 行）は凍結のため直していない（試験は `ENTRIES` から数える）
- **ST25 の画面**: ルートが 1 日の画面になり S-1 は `#/coverage` へ移った。設定への入口（S-1 の見出しの行）を見る e2e と単体試験を `#/coverage` から開くようにした。`Root.tsx` の振り分けに `#/settings` を残した
- **契約の名前**: `export::Summary` が ST25 の `day::Summary` と同じ名前で、openapi では片方が黙って上書きされる。`ExportSummary` にして `docs/openapi.json` を生成し直した

## Task

10 Task（32 項目）すべて `[x]`。残した Task は無い（`ci` / `human` の残りも無し）。Scenario 98 本（サーバ 77 は Rust、画面 21 は `web/e2e`）。

## 検証（`evidence.jsonl`。項目ごとに最新）

| 項目 | コマンド | 結果 | head |
|---|---|---|---|
| 1.1 | `tools/check-migrations.sh` | PASS | ebf910b |
| 1.3 | `tools/check-immutable.sh`（export_ledger） | PASS | ebf910b |
| 7.1 | `tools/check-openapi.sh` | PASS | ebf910b |
| 8.1〜8.3 | `npm run test -- export` / `SettingsView` / `settings-contrast` | PASS | ebf910b |
| 9.1 | `npm run test:e2e -- offsite` | PASS | ebf910b |
| 10.1 | `check_scenarios.py` / `review_triage.py` / `check_chain.py` | PASS（rc=0） | ebf910b |
| 10.2 | `cargo fmt --check` / `clippy -D warnings` / `cargo test --workspace` / web の lint・test・build・e2e（`STACK_RESET=1`）/ `check-immutable` / `check-offsite` / `check-log-private` / `smoke` | PASS（rc=0。`cargo test` は 929 本） | ebf910b |

origin/main（c6109b7）に rebase した木で取り直した。前の版にあった「`cargo test --workspace` の `server_startup` が繋ぎ先の port で落ちる」は、main の ST21 `a9b84803` を取り込んで消えた。
取り直しの途中で `cargo test` が 2 回落ちた原因（どちらも直した、または ST33 のコードの外）:
- 1 回目: 止めたはずの前の `cargo test` の実行が残り、同じ DB を取り合った（7 本。単独では全部通った）。うち 1 本（`export_dir_writes_under_the_chosen_dir`）は本当に項目数 12 の固定値で落ちていたので 13 に直した（d31fedc）
- 2 回目: `export_status_exporting_again_resets_the_erased_count` だけが `erased_after = 11` で落ちた。後に消した件数は DB 全体を数えるので、並んで走る試験の消去が入る。絶対値の 0 を 1 回だけ見ていたのを、5 回まで書き出し直して 0 が読めれば通す形にした（ebf910b）
- e2e の 1 回目は、`tools/stack.sh` が rebase 前に作った `target/release/ashiato-server` をそのまま使い、seed が `/places` で 404 になった（作り直して通った。`stack.sh` は実体があれば作り直さない）

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
