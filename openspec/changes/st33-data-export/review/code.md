# ST33 コードレビュー — st33-data-export

## final review（6d7d65f..b74736b）

席: final reviewer（`superpowers:requesting-code-review` の `code-reviewer.md`。review package `.superpowers/sdd/tasks/review-6d7d65f7..b74736b0.diff`）。
判定: **With fixes**（Critical 0 / Important 4 / Minor 12）。本人の決定（Q1〜Q9）と R2（削除済みを出さない）・C3（0700 / 0600）は構造とテストで担保されている、という評価。
Task の ⚠️ のうち T2（`App::with_export`）・T4（LateMark の結線と失敗の差し込み）・T5 F7（`Progress` を毎回作る）・T6（`docs/openapi.json` の `/export`）・T8（24px は e2e 9.5 で実測）・T9（DB を作り直せば e2e 全体 1.7 分）は解消を確かめた。

## R1. `GET /export` が読むたびに除く範囲の CTE を全件で評価し直し、画面は 1 秒ごとに前の応答を待たずに重ねて読む
- 分類: Important / production readiness（性能・プールの枯渇）
- 成果物: crates/server/src/export/ledger.rs:373（`erased_after`）、crates/server/src/export/job.rs（`read_status`）、web/src/SettingsView.tsx:84-87
- 根拠: `overlap_event` は `event_live` の全行に座標の判定と (c) の相関副問い合わせを当てる。Task 3 の報告で共有 DB 1 回約 10 秒、Task 5 の報告で座標つき 1 行約 8ms。`setInterval` は前の応答を待たない。プールは `max_connections(5)`（lib.rs:2840）で 1 本は写しが握るので、残り 4 本を状態の読み出しが奪い、取り込み（`POST /events`）を詰まらせうる。D17（仮）の前提「1 秒ごとでも軽い」が成り立たず、D17 自身の反転条件「重くなれば（`last` の `export_id`・削除の台帳の最後の行）で結果を覚える」に当たる
- kind: technical
- 提案: `erased_after` を（`last` の `export_id`, 削除の台帳の最後の行, 作り直しの印の最新）の鍵で覚える。画面は応答を待ってから次を読む `setTimeout` の連鎖にする。design D17 の（仮）を書き換える
- 処置: fixed D17 仮 — `erased_after` を（`last` の export_id・`core.deletion_ledger` / `core.event_version` / `core.stay_absorbed` の行数）の鍵で覚える（542cfbe5）。鍵に無い書き込みは数を減らすか動かさない側にしか働かないことを re-review が書き手ごとに確かめた。画面は応答を待つ `setTimeout` の連鎖（39c6fcea）。試験 `export_status_erased_count_is_remembered_but_recounted_after_a_content_update`

## R2. 書いた zip の大きさかハッシュを取れなかったとき、名前の付いた zip が誰にも見えないまま残る（T6 F3 の未修正）
- 分類: Important / correctness（本人の決定 Q4「1 つだけ置く」が黙って崩れる）
- 成果物: crates/server/src/export/job.rs（`finish_done` の `_ =>` の枝）
- 根拠: `rename` の後に `insert_failed` を書く。そのファイルは `done` でないので `removable_previous` にも `leftover` にも出ず、`finish` を持つので `interrupted` にも出ない
- kind: technical
- 提案: その枝で `remove_file` してから `insert_failed` を書く（「失敗したら残さない」と揃える）。失敗を差し込むテストを 1 本
- 処置: fixed D16 — `Stage::Measure` で失敗したら zip を消してから失敗の行を書く（cdfe10b2）。試験 `export_ledger_rows_a_file_that_cannot_be_measured_is_not_left`

## R3. 失敗・止まった・消せなかったファイルの 1 行が「前回の書き出し」の節にあり、spec と D13 の「書き出しの節」と違う（T8 ⚠️）
- 分類: Important / plan alignment
- 成果物: web/src/SettingsView.tsx（`lines` が `export-last` の section の中）、web/e2e/settings-export.spec.ts（`getByTestId("export-line")` が節を絞らない）
- 根拠: spec の 543〜547 行と design D13 は「書き出しの節に」と書き、D13 は「全記録の書き出し」と「前回の書き出し」を別の節に定める
- kind: technical
- 提案: `lines` を `export-section` へ移し、e2e の locator をその節に絞る
- 処置: fixed D13 仮 — `lines` を `export-section` へ移し、vitest と e2e の locator を節に絞った（39c6fcea）

## R4. 1 年分（35 万行）の量を測っておらず、tasks 5.6 の「5 分を超えたら D5 を考え直す」が未判定
- 分類: Important / plan alignment（5.6 の判定が残っている）
- 成果物: crates/server/src/export_file_tests.rs（`export_volume`）、tasks.md 5.6
- 根拠: `export_volume` は既定の 2 万行（`c02-window`、座標つき 1%）しか走っていない。Task 5 の報告は座標つき 1 行約 8ms で、35 万行で 5 分を超える見込みと書く（`.superpowers/sdd/st33-task-5/task-5-report.md:48`）
- kind: technical
- 提案: 使い捨ての DB で `ASHIATO_EXPORT_VOLUME_ROWS=350000` を 1 回走らせ、時間と最大メモリを記録する。5 分を超えたら PR 本文に既知の課題として残し、R1 と合わせて判定する
- 処置: fixed 5.6 — 使い捨ての database で 35 万行を 2 回: 書き出し 42.8〜43.0 秒（5 分未満。D5 は替えない）、テスト内の VmHWM 約 25 MB。summary の数え 1 回 6.5 秒が D13（仮）の反転条件（3 秒）に当たる —— 形は替えず、判定を design D13 に書き、Ruling として PR 本文で本人に見せる（開いたときに 1 回だけ数えるので繰り返しの負荷ではない）

## R5. `excluded_sql` の `erased_as_of: Some(T)` は使われない分岐で、doc が事実と違う（T3 ⚠️ の再分類）
- 分類: Minor / code quality
- 成果物: crates/server/src/export/excluded.rs:103-110
- 根拠: 本番の 3 か所とテストはすべて `None`。D17 は spec-review R25 に従いブルームフィルタで実装されている。doc の「D17 が写しの時点 `T` で使う」は事実と違う
- kind: technical
- 提案: 引数ごと消す
- 処置: fixed D17 — 引数ごと消した（6c91d13b）

## R6. 書く途中で失敗したとき `.partial` を消す経路にテストが無い（T5 ⚠️）
- 分類: Minor / testing
- 成果物: crates/server/src/export/（`write_zip` の `if result.is_err() { remove_file }`）、crates/server/src/export_tests.rs:290
- 根拠: 既存のテストは `Stage::Write` の前で落とすので `.partial` がまだ無い
- kind: technical
- 提案: D2 の差し込み口（`before_file`）で stream の途中から失敗させ、`.partial` が残らないことを確かめる
- 処置: fixed 5.2 — `Stage::Item` で stream の途中から落とす試験 `failure_in_the_middle_of_the_stream_leaves_no_partial_file`（f1b395f6）

## R7. handoff C1 の安全網「`core.source` の全ソースが緯度経度の有無と幅の読み方に明示で置かれている」試験が無い
- 分類: Minor / testing（申し送り）
- 成果物: crates/server/src/export/excluded.rs（`NO_COORD_SOURCES` / `SPAN_RULES`）
- 根拠: docs/handoff/ST33.md の C1。「緯度経度を持つ」側が暗黙の既定なので、ST17 の `s01-feeling` のように一覧に無いソースが足されても名前つきで落ちない。ST17 は未 archive（片割れは docs/handoff/ST17.md:29）
- kind: technical
- 提案: 緯度経度を持つ側も明示の一覧にし、`core.source` の全行がどちらか一方にだけ置かれていることを DB と突き合わせる試験を置く（ST17 がソースを足した時点で名前つきで落ちる）
- 処置: fixed D5 — `COORD_SOURCES` / `SPAN_NEXT_SOURCES` を明示にし、まっさらな DB の `core.source` の全行がちょうど一方にあることを突き合わせる試験 `export_excluded_sql_every_registered_source_is_classified_by_name`（b5462413）。SQL は一覧を読まないので除く範囲は変わらない。`s01-feeling` は ST17 が足す（docs/handoff/ST17.md）

## R8. design.md が実装からの逸脱を記録していない
- 分類: Minor / documentation
- 成果物: openspec/changes/st33-data-export/design.md（D5 の (c)、D3 / D7 の `.partial` の名前）
- 根拠: D5 の (c) は `lead()` の窓関数のはずが行ごとの相関副問い合わせ（T3 F3）。`.partial-…-<8 桁の 16 進>.zip` の接尾辞が design に無い
- kind: technical
- 提案: design に実装の形と理由を書く
- 処置: fixed D5 — design D5 に (c) の相関副問い合わせの形と理由、D3 に `.partial-…-<8 桁の 16 進>.zip` を書いた（7edd5044）

## R9. 古い doc が残っている
- 分類: Minor / documentation
- 成果物: crates/server/src/export.rs:4-5（「この段階の本体は…空の zip」。T5 F6）、crates/server/src/lib.rs:61-62（「書き出しの台帳の移行と錠」が `export_file_tests` に付いている。T5 F5）
- 根拠: `export.rs:4-5` は Task 5 の前の段階の説明のまま、`lib.rs:61-62` の doc は別のモジュールに付いている（Task 5 の review F5 / F6）
- kind: technical
- 提案: いまの形に合わせて書き直す
- 処置: fixed 5.2 — 古い doc を書き直した（6c91d13b）

## R10. テストがプロセス全体の `TMPDIR` を `set_var` で書き換える
- 分類: Minor / testing（並行のテストとの競合）
- 成果物: crates/server/src/export_file_tests.rs:596-612（`TmpdirGuard`）
- 根拠: `EXPORT_SERIAL` が直列にするのは書き出しのテストだけ。並んで走る他のテストは差し替えた `temp_dir()` を見る。glibc の getenv / setenv は並行に呼ぶと競合する
- kind: technical
- 提案: `TMPDIR` を差し替えず、プロセスの一時フォルダに新しくできたファイルだけを見る形にする（同じテストはもう `process_tmp` を見ている）
- 処置: fixed 5.2 — `TmpdirGuard` を消し、プロセスの一時フォルダに新しくできたファイルだけを見る（44f81e23）

## R11. `cargo test --workspace` で `drops_tests::drops_api_idempotent` が 2 回 `40P01 deadlock` で落ちている（T10 F3）
- 分類: Minor / testing
- 成果物: crates/server/src/drops_tests.rs
- 根拠: 書き出しのテストの失敗は無い。共有 DB で移行を当て直す既存のテストの形が原因と見られるが、この branch が頻度を上げたかは特定できていない
- kind: technical
- 提案: 原因を特定できなければ PR 本文に既知の揺れとして残す
- 処置: rejected: `drops_tests.rs` はこの branch の差分に無く（`git diff --stat 6d7d65f7 HEAD -- crates/server/src/drops_tests.rs` は空）、fix wave 後の `cargo test --workspace` は 816 passed / 0 failed で再現しなかった。共有 DB で移行を当て直す既存のテストの揺れで、ST33 の持ち物ではない

## R12. 失敗を黙って捨てている（T6 F2）
- 分類: Minor / error handling
- 成果物: crates/server/src/export/job.rs（`delete_previous` と `finish_done` の `let _ =`。`spawn_blocking` の JoinError、`insert_failed` の結果）
- 根拠: `let _ =` で `spawn_blocking` の JoinError と `insert_failed` の結果を捨て、ログにも出さない（Task 6 の review F2）
- kind: technical
- 提案: `tracing::warn!(kind = …)` を 1 行ずつ足す（値は載せない）
- 処置: fixed D16 — 種別だけを `tracing::warn!` / `error!` に出す（cdfe10b2）

## R13. `GET /export` が読むたびに `leftover` の候補の SHA-256 を全体で計算する（T6 F4）
- 分類: Minor / performance
- 成果物: crates/server/src/export/job.rs
- 根拠: `read_status` が読むたびに残った候補のファイル全体を SHA-256 で読む。1 秒ごとの読み出しで大きな zip を毎回読む（Task 6 の review F4）
- kind: technical
- 提案: R1 と同じく、（パス・大きさ・更新時刻）が同じなら前の結果を使う
- 処置: fixed D16 — （パス・大きさ・更新時刻）で覚える。消す側の判定は毎回計算のまま（542cfbe5）。試験 `export_status_leftover_digest_is_not_reused_after_the_file_changes`

## R14. `last_failure.kind` を `reason` の文字列を `:` で割って取っている（T6 F6）
- 分類: Minor / type safety
- 成果物: crates/server/src/export/job.rs
- 根拠: `reason` の文字列の先頭を `:` で割って種別にしている。書く側の文言が変わると黙って種別がずれる（Task 6 の review F6）
- kind: technical
- 提案: 台帳の行から種別を構造で取る（列か、`reason` を作る側と読む側で同じ関数を使う）
- 処置: fixed D10 — `FailKind::of_reason` と `insert_failed(&ExportError)` で書く側と読む側を同じ形に（cdfe10b2）

## R15. `messages.ts` の `sourcesTitle` が使われていない（T8 F2）
- 分類: Minor / code quality
- 成果物: web/src/messages.ts
- 根拠: `sourcesTitle` を参照する箇所が `web/src` に無い（Task 8 の review F2）
- kind: technical
- 提案: 消す
- 処置: fixed D13 仮 — 消した（6c91d13b）

## R16. `export_post` の `password` が `serde_json::Value` と `Bytes` の写しに残り、zeroize されない
- 分類: Minor / security（できる範囲での消去）
- 成果物: crates/server/src/lib.rs（`export_post`）
- 根拠: D の決定には反しない（できる範囲）
- kind: technical
- 提案: `serde_json::Value` を経ず、`Zeroizing<String>` を持つ型へ直接読む
- 処置: fixed D1 — 自前の Visitor で `Zeroizing<String>` に直に読み、`Json` 抽出子は使わない（e426552a）。serde_json の作業領域の写しは「できる範囲」の外

## R17. 台帳に終わりの行を書けなかった枝（`Stage::Finish`）で、名前を付けた完成済みの zip が誰にも見えないまま残る
- 分類: Important / correctness（本人の決定 Q4「1 つだけ置く」が黙って崩れる。R2 と同じ型）
- 成果物: crates/server/src/export/job.rs:274-277
- 根拠: scoped re-review（4258bfe..0f2aff9）の範囲外の観察。`insert_done` が落ちると `write_failed` だけを書き、`rename` 済みの zip を消さない。その zip は `done` でないので `removable_previous` / `leftover` に出ず、`finish` を持つので `interrupted` にも出ない
- kind: technical
- 提案: R2 と同じく、zip を消してから失敗の行を書く。`Stage::Finish` の差し込みで落とし、ファイルが残らないことを確かめる試験を 1 本
- 処置: fixed D16 — `Stage::Finish` で落ちたら、R2 と同じ `remove_unfinished` で zip を消してから失敗の行を書く。試験 `export_ledger_rows_a_file_whose_finish_row_cannot_be_written_is_not_left`

## final review（6d7d65f..dc61bd6）: 新しい指摘なし（再入）
- 前回の final review（6d7d65f..b74736b）・fix wave（4258bfe..0f2aff9）・scoped re-review の後に増えたのは `evidence.jsonl` の 3 行（dc61bd6。Task 10 の証跡の取り直し）だけで、コードと文書の差分は 0（`git diff --stat bdc229b dc61bd6`）。base も同じ 6d7d65f。同じ木を 2 度目の whole-branch review にかけない（final は 1 回）
- `docs/handoff/ST33.md` は開始時から増えていない（最終の変更は daa4ed7）
- R1〜R16 の処置はそのまま。**R17 は処置が無いまま finish に渡す**

## code-verify（d81e27a。独立検証 2026-10-15）

席: `code-verify`。読んだ印象ではなく、走らせた結果だけを書く。値を変えて試すもの（変異）と観測用の試験は使い捨ての worktree（`/tmp/cv33-mut`。終わったら消した）と別の database（`cv33mut`）でやり、作業ツリーのコードは触っていない。
DB はこの worktree の `ashiato2-st33`（`127.0.0.1:55533`）。origin/main はこの branch より 169 commit 先にある（移行 `202610121040_*` などを含む）。ここでの検証は branch の HEAD のまま。

**申告: 32/32 `[x]`**（`grep -c '^- \[x\]'` = 32、`- [ ]` = 0）。検証コマンドは tasks.md の各項目の本文に書かれているもの。

| 項目 | 実行したコマンド（要点） | 実測 |
|---|---|---|
| 1.1 | `tools/check-migrations.sh` / `CT export_ledger_migration 1` | 一致（rc=0 / 1 本 ok） |
| 1.2〜6.4 の CT 19 本 | `cargo test --workspace` を 1 回走らせ、ログの `test … ok` を絞り込みごとに数えた（`export_dir` 5 ≥ 3、`export_excluded_sql` 20 ≥ 19、`export_file_excludes` 19 ≥ 19、`export_status` 13 ≥ 10、`mark_late_for_user` 6 ≥ 5、`late_arrival_is_marked` 2 ≥ 1 ほか全部が下限以上。FAILED 0） | 一致 |
| 1.3 | `bash -o pipefail -c 'tools/check-immutable.sh \| tee /tmp/ci.log' && grep -q export_ledger` | 一致（rc=0。`OK export_ledger は UPDATE / DELETE / TRUNCATE を拒む`）。**注**: 終わりに `down -v` で DB を消す |
| 7.1 | `tools/check-openapi.sh` | 一致（rc=0） |
| 7.2 | `grep -q '#/settings' docs/screens.md` | 一致（rc=0） |
| 8.1〜8.3 | `npm run test -- export` / `-- SettingsView` / `-- settings-contrast` | 一致（5 / 15 / 5 passed） |
| 9.1〜9.5 | 10.2 の e2e の中で（65 passed）。`ET` の `--list` は 9.2 = 10・9.3 = 5・9.4 = 1・9.5 = 6 | 一致 |
| 10.1 | `check_scenarios.py` / `review_triage.py` / `check_chain.py` | **不一致**: `review_triage.py` が **rc=1**（`[FAIL] code.md R17: 処置が無い`）。他の 2 本は rc=0。evidence の最新（bdc229b）も FAIL のまま。R17 に処置が付くまで merge_gate はここで落ちる |
| 10.2 | `cargo fmt --all --check` / `cargo clippy --workspace --all-targets -- -D warnings` / `cargo test --workspace` / `tools/db.sh down -v && cd web && npm run lint && npm run test && npm run build && STACK_RESET=1 npm run test:e2e` / `check-immutable` / `check-offsite` / `check-log-private` / `smoke` | 一致（全部 rc=0。`cargo test --workspace` は lib 676 + collector 133 + `server_startup` 7 = 816 passed。証跡の 7edd504 のログも sha256 が記録と同じで 816 passed。7edd504 から HEAD までコードの差分は 0）。**注**: 最初の 1 回は `.env` を素のまま読んだので `server_startup` の 2 本が `migrate が失敗した`（URL の port が 55432）。`ashiato_db_url` で port を直して `--test server_startup` を走らせ直すと 7 passed |
| 10.3 | `grep -c 'st22-record-deletion' docs/handoff/ST33.md` / `grep -c 'Scenario: 削除の印の付いた記録は出ない' crates/server/src/export_file_tests.rs` | 一致（4 / 1） |

**手 1（固定値を独立に計算し直す）: 不一致は無かった。** e2e が書いた最後の zip（`.export/ashiato-export-20261015-231915.zip`）を、`zip` crate を通さない自作の読み手（Python の `zipfile` で中央ディレクトリを読み、PBKDF2-HMAC-SHA1 1,000 回・AES-CTR・HMAC-SHA1 の 10 バイト・raw deflate を自分で組んだもの）で開いた:
- 12 項目すべてが `compress_type 99`・`0x9901` の strength 3（AES-256）。20 バイト以上は AE-1（CRC-32 も平文と一致）、空の 2 項目は AE-2。違う合言葉は `password verification mismatch` で開かない
- `sha256sum` と大きさが台帳の `digest` / `bytes` と一致（`59b7637b…856f` / 157123）。注記は `ashiato-export <その export_id>`
- 全 10 ファイルで目録の `rows` と行の数が一致。2,985 個の時刻の値がすべて `YYYY-MM-DDTHH:MM:SS.ffffffZ`
- `events.jsonl` の 1,451 行の `raw` の UTF-8 の SHA-256 と `event_time` が、DB の `core.event_live` の同じ id の値と 1,451 件すべて一致。列は `core.event` の 21 列と同じ
- 台帳の `written` を自作のブルームフィルタの読み手（`bloom.rs` の形を写したもの）で読むと、1,451 件すべてが入り、無作為の 20,000 件は 0 件当たった。1 要素 28.76 ビット・ハッシュ 20 個は `-ln(10⁻⁶)/(ln 2)²` = 28.755・`28.755 × ln 2` = 19.93 と一致
- README の畳む規則は `pg_get_viewdef('core.event_folded')` の `GROUP BY user_id, logical_source, content_hash` と各列の取り方に一致
- つまり Scenario「このシステムを使わずに中身を読める」は、試験の読み手（書き手と同じ `zip` crate）より外側でも成り立った

**手 2（守りをわざと壊す）**: 次の 5 つは、守りを潰すと実際に落ちた。
- 移行の行トリガを `BEFORE UPDATE` だけにする（DELETE を通す）→ `check-immutable.sh` が rc=1（`NG export_ledger を変更できた: DELETE …`）
- `export_file_tests.rs` の `// Scenario: 削除の印の付いた記録は出ない` を消す → `check_scenarios.py` が rc=1（`担保の無い Scenario`）
- 差し替え口ではなく本物の DB に `CREATE TABLE core.cv_zzz` と `ALTER TABLE core.heartbeat ADD COLUMN cv_extra` を置く → `export_catalog_every_table_and_column_is_classified` ほか 3 本が FAILED
- spec の Scenario 98 本は、それぞれ印がちょうど 1 つ（`書き出しの台帳の行は書き換えも削除もできない` だけ Rust と `check-immutable.sh` の 2 つ）
- 合言葉をログに出す変異（非同期の仕事の中で `tracing::info!`）→ `export_responses_never_keep_the_passphrase` が FAILED。ただし受け手の外側があった（R24）

**手 4（本人の決定が test で固定されているか）**: 1 つずつ書き換えて、対応する絞り込みを走らせた。

| 変異 | 結果 |
|---|---|
| `MIN_PASSPHRASE_CHARS` 12→11（Q9 / C23） | FAILED 2 本 |
| web の `MIN_PASSWORD_CHARS` 12→11 | vitest 2 本が FAILED |
| `AesMode::Aes256` → `Aes128`（Q9 / D19） | FAILED 1 本 |
| `0o700` / `0o600` → `0o755` / `0o644`（C25） | FAILED 1 本 |
| 前のファイルを消さない（Q4） | FAILED 4 本 |
| `SPAN_RULES` から `c01-photo` を外す（C21） | FAILED 3 本 |
| plus code の形を抜く（C20 / Q8） | FAILED 3 本 |
| 消した滞在の前の版の時間を数えない（C19） | FAILED 2 本 |
| `POLL_MS` 1000→2000 / →300（D13 仮） | vitest 2 本 / 1 本が FAILED |
| 見込みの既定 180 バイト・20,000 行/秒を変える（D18 仮） | FAILED 2 本 |
| D17 の鍵から `deletion_ledger` / `event_version` を外す | FAILED 2 本 / 1 本 |
| **D17 の鍵から `stay_absorbed` を外す** | **全部通る**（R21） |
| **後に消した記録の数えで、作り直しの印（`rebuild:`）も数える** | **全部通る**（R19） |
| **ブルームフィルタの位置の取り方を変える（`digest[0..8]` → `[16..24]`）** | **全部通る**（R20） |

Q7（利用者を問わない）は変異にかけていない（SQL の形を大きく変えないと書けないため）。

**手 6（隙間）**: 次の 4 つを実行で観測した。R17（終わりの行を書けなかった zip が残る）も実行で再現した —— 1 回目を書き終えた後、2 回目を `fail(Stage::Finish)` で落とすと置き場に zip が 2 つ。状態は `last_failure: db`・`leftover: []`。3 回目を書き終えても 2 つのままで、状態は `last_failure: null`・`leftover: []`・`interrupted: null` になる（どこにも出ない）。

## R18. 途中で止まった書き出しは、次の書き出しが書き終えると状態から消える。ファイルは残るのに、どの欄にも出ない（spec の「始まりの行だけがあれば止まったと返す」と食い違う）
- 成果物: crates/server/src/export/ledger.rs:304-313（`derive` の `interrupted` が `rfind(is_start)` で**最後の**始まりの行しか見ない）/ crates/server/src/export_job_tests.rs:902（`assert!(st["interrupted"].is_null())` が食い違ったほうを固定している）/ crates/server/src/export/ledger.rs:406-416（`erased_after` は `last` の 1 件の `written` しか読まない）/ specs/data-durability/spec.md:460-461
- 根拠: 使い捨ての試験 `cv_probe_interrupted_vanishes_after_the_next_export` で、1 回目を書き終え、2 回目を `with_halt(Stage::Finish)` で止め（`rename` の後）、新しい `App` で 3 回目を書き終えた。止めた直後の状態は `interrupted.file = …-223749.zip`。3 回目の後は置き場に `…-223749.zip` と `…-223749-2.zip` の 2 つがあり、台帳にはこの置き場の「始まりの行だけ」の書き出しが 1 件ある（SQL で数えて 1）。それでも状態は `interrupted: null`・`leftover: []` になる。
  spec 460 は「台帳に始まりの行だけがあり、その書き出しが走っていない」なら止まったことを返すと定めている。止まったのが最後の書き出しかどうかは条件にない。
  さらに、止まった zip は全記録の写しなので、その後に消した場面も中に残る。けれども `erased_after` は `last` しか数えないので、件数にも出ない。Q4「1 つだけ置く」が見えないまま崩れる。
  同じ型が `.partial-…` にもある。D3 は「止まったときの `.partial-…` は残る」と決めているが、状態にも画面にも一度も出ない。この worktree の `.export/` には `.partial-ashiato-export-20261013-223426-56d89ba2.zip`（9.6 MB）など 4 つが残っていた（R23 の観測と同じ場所）
- kind: technical
- 提案: `interrupted` を「`finish` の無い始まりの行（走っているものを除く）すべて」の配列にし、名前のファイルか `.partial-…` が残っているものを返す。画面はそれを 1 行ずつ出す。902 行の `is_null` は、3 回目の後も止まったファイルが出ていることの確認に替える。spec 461 の「消さない」はそのままでよい
- 処置: fixed D11 — `interrupted` は終わりの行の無い始まりの行を新しい順に見て、最後の始まりの行か、名前のファイルが残っているものを返す（形は 1 件のまま）。902 行の `is_null` を「3 回目の後も止まったファイルが出る」と「本人が片付ければ出ない」に替えた。止まったものが 2 つ以上残るときは新しい 1 つだけ出る・`.partial-…` を状態に出さない・止まった zip の中の後に消した場面を数えない、の 3 つは直していない（PR 本文の申し送り）

## R19. Scenario「作り直しで置き換わった滞在は後に消した記録に数えない」の試験は、R1 で足した覚え（memo）越しにしか観測しておらず、数え方を壊しても通る
- 成果物: crates/server/src/export_job_tests.rs:984-1001 / crates/server/src/export/ledger.rs:420-428（`erased_after` の `deleted_by NOT LIKE 'rebuild:%'`）/ crates/server/src/export/job.rs:65-76（覚え）
- 根拠: 変異 M6（`erased_after` の作り直しの印を除く条件を外し、`rebuild:` の印も数える）で `export_status` の 13 本が全部 ok。試験は同じ `App` で前と後を読む。`mark_deleted(…, "rebuild:test")` は削除の台帳・前の版・吸収の台帳のどれにも行を足さないので、鍵（D17）が変わらず、2 回目は覚えた値が返る。
  同じ試験の読み手だけを新しい `App`（覚えが空）に替えると、変異なしでは ok、M6 では `export_job_tests.rs:997` で FAILED になった。つまりこの Scenario が主張する「数えない」を観測しているのは、覚えの側の振る舞いだけ（主張の層は数え方、試験の層は覚え）。
  本物の作り直し（`POST /stays/rebuild`）が吸収の台帳を書いて鍵を動かした後や、サーバを起動し直した後に数え違えても、どの試験も落ちない
- kind: technical
- 提案: 後の読み出しを新しい `App` で行うか、`ExportJobs::erased_counts()` が 1 増えたこと（実際に数えたこと）を確かめてから値を比べる。覚えの振る舞いは既存の `export_status_erased_count_is_remembered_but_recounted_after_a_content_update` が見ている
- 処置: fixed D17 — 後の読み出しを覚えの空の新しい `App` で行い、`erased_counts() == 1`（実際に数えた）を確かめる。変異（`rebuild:` の印も数える）で FAILED を確かめた

## R20. ブルームフィルタの位置の取り方（`written` の形）がどの試験にも固定されておらず、変えると前に書いた台帳の行が 0 件と読まれる
- 成果物: crates/server/src/export/bloom.rs:1-6（「台帳に置いたフィルタは後の版のサーバが読む」）, 34-40（`positions`）/ crates/server/src/export/bloom.rs:76-96（試験は同じ版で書いて同じ版で読む往復だけ）
- 根拠: 変異 M4（`h1` を `digest[0..8]` から `digest[16..24]` へ）で `bloom` の 2 本と `export_status` の 13 本が全部 ok。
  実物で効き目を測るため、e2e が書いた書き出し（`d7bf7676-…`）の `written` を、`bloom.rs` の位置の決め方を写した自作の読み手で読んだ。元の形では `events.jsonl` の 1,451 件すべてが入る。変異の形で読むと 0 件になる（`contained=0`）。
  台帳は追記のみで `written` を書き直せないので、位置の取り方・形の版を変えた後のサーバは、前回の書き出しの「後に消した記録」を 0 と出す。D17 が言う「少なく数える経路が無い」が、版をまたぐと崩れる。`VERSION` の 1 バイトはあるが、読む側は形が違えば `None` → 0 件と扱う（ledger.rs:416-418。数え落とす側）
- kind: irreversible
- loss: rewrite-all
- 提案: 固定の UUID 数個を入れたフィルタのバイト列（か、その SHA-256）を期待値として直書きする試験を 1 本置く。期待値はこの節の自作の読み手と同じように別の実装で作る。`from_bytes` が知らない版を読んだときは、0 ではなく「数えられない」を状態に出す（少なく数える側に倒さない）
- 処置: escalated — `deep.md` Q12（第 5 回。`deep-questions-r4.json` / `docs/briefs/ST33-deep-r4.html`）。固定の期待値の試験と「数えられない」の表示は、答えを待ってから入れる

## R21. D17 の鍵の 3 つ目（`core.stay_absorbed` の行の数）は、外しても全部の試験が通る
- 成果物: crates/server/src/export/ledger.rs:387-402（`erased_key`）/ design.md D17（鍵は 3 つの表の行の数）
- 根拠: 変異 M5（`(SELECT count(*) FROM core.stay_absorbed)` → `0`）で `export_status` の 13 本が全部 ok。比べると、`deletion_ledger` を外すと 2 本、`event_version` を外すと 1 本が FAILED になった（この 2 つは固定されている）。
  D17 は吸収（`rebuild:absorbed`）を「件数を増やしうる書き込み」として鍵に入れた。外しても見張る試験が無いので、吸収の後に覚えた古い値を出し続けても気づかない
- kind: technical
- 提案: 書き出し → 状態を 1 回読む（覚える）→ 作り直しで滞在を吸収させる（`stay_absorbed` に 1 行）→ 状態を読み、`erased_counts()` が増えた（数え直した）ことを確かめる試験を 1 本足す
- 処置: fixed D17 — 試験 `export_status_erased_count_is_recounted_after_a_stay_is_absorbed`（吸収の 1 行で `erased_counts()` が増える）。変異（鍵の `stay_absorbed` を 0 に）で FAILED を確かめた

## R22. 台帳に始まりの行を書けないと、`POST /export` は 202 を返したまま何も残さず、状態にも画面にも失敗が出ない
- 成果物: crates/server/src/export/job.rs:168-171（`insert_start` が落ちるとログを 1 行出して `return`）/ web/src/SettingsView.tsx（`started` なら何も出さない）
- 根拠: 使い捨ての database で `ALTER TABLE core.export_ledger ADD CONSTRAINT cv_reject CHECK (false) NOT VALID` を置き、観測用の試験 `cv_probe_start_row_failure_is_silent` を走らせた。`POST` は `202 {"export_id": "cb783064-…"}`、仕事が終わった後の状態は `last_failure` が前後とも `null`、`last` は前と同じ、置き場のファイルは 0。
  画面は `running` が一度出て消えるか、一度も出ないまま元の形に戻る。押した本人には「書き出した」とも「失敗した」とも出ない。spec の「失敗した書き出しも台帳に残る」は台帳が書ける前提で、その外側にあたる
- kind: daily
- 提案: 始まりの行を `spawn` の前（`POST` の中）で書き、書けなければ 500（種別だけ）を返す。そうすれば画面の既存の「書き出しを始められませんでした」の 1 行に乗る。あるいは失敗をメモリの `ExportJobs` に 1 件持ち、`last_failure` に出す
- 処置: fixed D20 仮 — 始まりの行を `spawn` の中（背景の仕事を起こす前）で書き、書けなければ印を外して `500 {"error": "ledger_unavailable"}`。差し込み口 `Stage::Start`、試験 `export_ledger_rows_a_start_row_that_cannot_be_written_is_refused`。画面は既存の `refused.other` に乗る。`docs/openapi.json` を更新

## R23. 台帳が失われる（DB を作り直す・戻す）と、置き場にある前の書き出しは二度と消えず、どこにも出ない。この worktree の置き場には書き出しが 12 個たまっていた
- 成果物: crates/server/src/export/ledger.rs:217-221（`removable_previous` は台帳の `done` の行しか見ない）/ design.md D16（「台帳に無いパスには触らない」）/ docs/stories/ST30.md・ST32.md
- 根拠: e2e（`STACK_RESET=1`）の後の `ls -la .export/` には `ashiato-export-*.zip` が 12 個あった（`20261014-053428` 〜 `20261015-231915`。3.1 MB と 5.9 MB のものを含む）。加えて `.partial-…` が 4 個あった。その時点の台帳（18 行）にあったのは `20261015-2318xx` / `2319xx` の 4 回分だけで、残りの 11 個の名前は台帳に無い。
  D16 は本人の置いたファイルを守るため、台帳に無いパスを消さない。DB を作り直す・バックアップから戻すたびに、それより前の書き出しは「このシステムが書いていないもの」になる。それらは消えず、`leftover` にも出ず、その後に消した場面を含んだまま積み上がる。本番では、バックアップ（ST30）から D-01 を戻したとき（復元テストの ST32 を含む）に同じことが起きる。
  `docs/stories/ST30.md`・`ST32.md` に「書き出し」「export」は 0 件で、この申し送りを持つ Story が無い（隙間に落ちている）
- kind: defer
- 提案: `docs/handoff/ST30.md`（か ST32）に「戻した後は、置き場の `ashiato-export-*.zip` のうち台帳に無いものを本人に見せる（消すかは本人）」を書く。ST33 の側では、台帳に無い `ashiato-export-*.zip` / `.partial-…` の名前を状態に「見覚えの無い書き出しのファイル」として出す（消さない）だけで扉は開いたまま残る。確認バッチの `run.sh` と e2e の置き場は、走らせるたびに空にしてよい
- 処置: deferred ST30 — `docs/handoff/ST30.md` に書いた（ST30 は change が無い）。ST33 の側で台帳に無い書き出しのファイルを状態に出す案は入れていない（PR 本文の申し送り）

## R24. Scenario「合言葉はどこにも残らない」のログの受け手は試験のスレッドの上だけで、`spawn_blocking` の中のログは素通りする
- 成果物: crates/server/src/export_job_tests.rs:1218-1223（`tracing::subscriber::set_default`。スレッドごとの既定）/ crates/server/src/export/job.rs:214, 238, 300, 327（`spawn_blocking` の中で `tracing::warn!` を出す経路）
- 根拠: 変異 M11（非同期の仕事の中で合言葉を `tracing::info!`）は FAILED になった。一方、変異 M11b（同じ値を `spawn_blocking` の中で `tracing::warn!`）では `export_responses_never_keep_the_passphrase` が ok のまま。
  いまの本体は合言葉を blocking のスレッドへ渡していない（`dump` は同期で書く）ので、いま漏れているわけではない。ただ、書き込みを blocking に移す変更（`dump` が runtime のスレッドを塞いでいるので、ありうる変更）を入れても、この試験は守らない
- kind: technical
- 提案: 受け手を `tracing::subscriber::set_global_default`（試験のプロセスで 1 回）にする。あるいは `tracing_subscriber::fmt().with_writer(…)` を `Dispatch` にして `spawn_blocking` の中でも `dispatcher::with_default` が効く形にし、blocking の中で出したログを拾えることを正の対照として 1 行確かめる
- 処置: fixed D15 — 受け手を試験のプロセス全体の口（`archive_flow_tests::captured_log`。`set_global_default`）に替え、`spawn_blocking` の中で出したログが届くことを正の対照として先に確かめる

## scoped re-review（a0a07b5..13b123b。code-verify の fix wave）
- 席: 独立の re-reviewer（SDD の `re-review-prompt.md`）。R17 / R18 / R19 / R21 / R22 / R24 は ADDRESSED、R20 は escalated（Q12 未回答・コードは先取りしていない）、R23 は deferred（`docs/handoff/ST30.md`）で妥当。fix diff の新しい壊れ: なし
- R18 は提案より狭い（止まったものが複数なら新しい 1 件・`.partial-…` は出ない・止まった zip の中の後に消した場面は数えない）。PR 本文の申し送りに残す
