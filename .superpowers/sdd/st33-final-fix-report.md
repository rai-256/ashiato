STATUS: DONE_WITH_CONCERNS

# ST33 whole-branch review の fix wave（R1〜R16、R11 は対象外）

FIX_BASE = 4258bfe6。RED のログは `.superpowers/sdd/logs/`（下の表）と `.superpowers/sdd/st33-final-fix/`。

## R ごと

| R | 何をしたか | commit | テスト / D |
|---|---|---|---|
| R1 | `erased_after` を `ErasedKey`（`last` の export_id ＋ `core.deletion_ledger`・`core.event_version`・`core.stay_absorbed` の**行の数**）で覚える（`ExportJobs` のメモ）。3 表は追記のみなので行の数はコミットごとに必ず増える（`max(seq)` はコミットの順が入れ替わると見落とすので使わない）。鍵に無い書き込み（新しい記録の取り込み・`rebuild:erased-range`・消去）は数を減らすか動かさない側にしか働かず、D17「誤りは多く数える側だけ」を保つ。鍵は数える前に読む。画面は `setInterval` をやめ、応答を待ってから次を読む `setTimeout` の連鎖にした | 542cfbe5（サーバ）・39c6fcea（画面）・7edd5044（design） | `export_status_erased_count_is_remembered_but_recounted_after_a_content_update`（RED: 鍵から前の版の数を外すと delta 0 ≠ 1。`logs/20261014-235145-red-r1.log`）。鍵から削除の台帳の数を外すと既存の `export_status_erasing_after_the_export_shows_the_erased_count` が落ちることも確かめた（`logs/20261014-235300-red-r1-del.log`）。vitest `読み直しは前の応答を待ってから次を読む`（RED: 6 ≠ 2。`st33-final-fix/red-r1-r3-web.log`）。D17（仮）の本文と反転条件を書き直した。D13（仮）の判定は下 |
| R2 | `Stage::Measure` を足し、大きさとハッシュを取る段を `stage()` に通した。失敗の枝で名前を付けた zip を消してから `insert_failed` | cdfe10b2 | `export_ledger_rows_a_file_that_cannot_be_measured_is_not_left`（RED: zip が 2 つ残る。`logs/20261014-232723-red-r2.log`） |
| R3 | spec 543〜547 行と D13 は「書き出しの節に」と明記していて曖昧ではない（Ruling 不要）。`lines` を `export-section` へ移し、e2e の locator を `getByTestId("export-section").getByTestId("export-line")` に絞った | 39c6fcea | vitest `失敗・途中で止まった・消せなかったファイルは 1 行ずつ`（`export-last` に 0 行・`export-section` に 3 行。RED: `st33-final-fix/red-r1-r3-web.log`）。D13 の文言も直した |
| R4 | 使い捨ての database（ST33 の compose の DB に `st33_volume_<時刻>` を作り、終わって `DROP ... WITH (FORCE)`）で `ASHIATO_EXPORT_VOLUME_ROWS=350000` を 2 回。台本 `.superpowers/sdd/st33-final-fix/volume.sh` | 7edd5044（summary の時間も出すようにした） | 下の数字 |
| R5 | `excluded_sql` の `erased_as_of` を引数ごと消し、呼び出し 6 か所と doc を直した | 6c91d13b | — |
| R6 | `Stage::Item`（zip の項目ごとに `fail` だけを呼ぶ）を足し、記録と前の版を書いた後の 3 つ目の項目の手前で落とす | f1b395f6 | `failure_in_the_middle_of_the_stream_leaves_no_partial_file`（RED: 後片付けを止めると `.partial-…` が残る。`logs/20261014-234005-red-r6.log`） |
| R7 | `COORD_SOURCES`・`SPAN_NEXT_SOURCES`・`ARCHIVE_PRODUCT_PREFIX`（`c03-myactivity-`）を明示の一覧として置いた。**SQL は読まない**（一覧に無いソースは厳しい側の既定のまま。分類は変えず、除く範囲を狭めていない）。`s01-feeling` は足していない。全移行だけを当てたまっさらな DB（`stay_tests::fresh_db`）の `core.source` の全行が、緯度経度と幅のそれぞれで一覧のちょうど一方にあることを確かめる（共有の DB は他のテストの `t-…` と台本の `smoke`・`gate-check` などが混ざるので使わない） | b5462413・7edd5044 | `export_excluded_sql_every_registered_source_is_classified_by_name`（RED: `s01-archive-inbox` を外すと名前つきで落ちる。`logs/20261014-234927-red-r7.log`）。D5a / D5b に追記 |
| R8 | D5 に (c) が行ごとの相関副問い合わせであることとその理由（`lead()` は全行を並べ替えてから絞ることになる。相関なら「消した時間の終わりより前に始まり座標を持つ行」に先に絞れる。前の版の次の記録も `core.event_live` から引く）、D3 に `.partial-ashiato-export-<YYYYMMDD-HHMMSS>-<8 桁の 16 進>.zip` を書いた | 7edd5044 | — |
| R9 | `export.rs` の先頭の doc（「空の zip まで」）と、`lib.rs` のテストのモジュールの doc を直した | 6c91d13b | — |
| R10 | `TmpdirGuard` を消し、`TMPDIR` を差し替えずに、プロセスの一時フォルダに始まってから新しくできたファイルを見る形にした | 44f81e23 | `export_zip_never_writes_record_values_unencrypted_midway`。一時フォルダに行を書く 1 行をわざと `dump.rs` に入れると落ちることを確かめてから戻した（`logs/20261014-234126-red-r10.log`） |
| R12 | `delete_previous` の `spawn_blocking` の JoinError、`finish_done` の `insert_failed`（`write_failed` にまとめた）と前のファイルを消す段の結果を `tracing::warn!/error!(kind = …)` にした（値・パス・合言葉は載せない） | cdfe10b2 | — |
| R13 | `leftover` の SHA-256 を（パス・大きさ・更新時刻）で覚える（`ledger::is_untouched_with`）。消す側の `removable_previous` は毎回計算する | 542cfbe5 | `export_status_leftover_digest_is_not_reused_after_the_file_changes`（RED: パスだけで覚えると書き換えたファイルが残る。`logs/20261014-235337-red-r13.log`） |
| R14 | `FailKind::ALL` と `FailKind::of_reason`（`ExportError` の `Display` と同じ形から戻す）。`insert_failed` は `&ExportError` を受けて `reason` を作る | cdfe10b2 | `export_status_failure_kind_is_read_back_from_the_reason_by_its_structure`（RED: `C:\…` の reason から kind `C` が出た。`st33-final-fix/red-r14.log`） |
| R15 | `messages.ts` の `sourcesTitle` を消した（grep で参照 0 件を確かめた） | 6c91d13b | — |
| R16 | `export_body`（自前の Visitor）で、`password` の文字列だけを `Zeroizing<String>` に直に読み、ほかの鍵の値は `IgnoredAny` で捨てる。`Json<…>` 抽出子には戻していない（D1）。拒否は種別だけ。`Passphrase::from_zeroizing` を足した。本体の `Bytes` は持ち主が 1 人なら読んだ後に 0 で埋める | e426552a | `export_body_reads_the_password_and_refuses_by_kind_only`（12 形）。既存の `export_endpoint_*`（7 本）・`export_responses_*`（4 本）が緑 |

## R4 の数字（使い捨ての DB、35 万行の `c02-window`・座標つき 1%）

| 回 | 書き出し 1 回 | `GET /export/summary` の数え 1 回 | テスト内の VmHWM | `/usr/bin/time -v` の最大 RSS（テストのプロセス全体） |
|---|---|---|---|---|
| 1（`logs/20261014-235552-r4-volume.log`） | 43.0 秒 | （未計測） | 24,960 KiB | 1,335,640 KB |
| 2（`logs/20261014-235807-r4-volume-2.log`） | 42.8 秒 | 6.5 秒 | 25,088 KiB | 1,335,768 KB |

- 5 分（300 秒）を超えていない。D5 は触っていない
- VmHWM（書き出しの直後に `/proc/self/status` から読む）と `time` の最大 RSS が桁で違う。`time` の値は書き出しの後の確かめ（`scan` が `events.jsonl` を復号して全部読む）を含むプロセス全体の最大で、書き出しの寄与を分けて測っていない。PostgreSQL 側のメモリは測れていない
- **D13（仮）の反転条件「件数の数えに 3 秒以上」に当たる**（6.5 秒）。design D13 に判定を書いた。替えるかは controller・本人の判断として残した（開いたときに 1 回だけ読む）

## 懸念・残したもの

- **D13（仮）の反転条件に当たった**（`GET /export/summary` の数えが 35 万行で 6.5 秒 > 3 秒）。判定を design に書いただけで、形は替えていない（controller の判断）

- R16: serde_json はエスケープを含む文字列を内部の作業領域に写すので、その写しは消せない。`Bytes` が共有されていれば 0 で埋められない（「できる範囲」）
- R1 の前提: 始まりが未来の消した滞在があると、鍵が動かないまま時間の経過で数が増えうる（D17 に前提として書いた。時計の狂った端末からしか起きない）

## 検証

`scripts/verify-run --change st33-data-export 10.2`（repo の根。`.env` を読み込んで）: **全 8 項目 PASS**。全文 `.superpowers/sdd/st33-final-fix/verify-10.2.log`、証跡 `openspec/changes/st33-data-export/evidence.jsonl`

| コマンド | rc | 時間 |
|---|---|---|
| `cargo fmt --all --check` | 0 | 2s |
| `cargo clippy --workspace --all-targets -- -D warnings` | 0 | 6s |
| `cargo test --workspace`（816 passed / 0 failed。R11 の揺れは今回出ていない） | 0 | 1935s |
| `tools/db.sh down -v && cd web && npm run lint && npm run test && npm run build && STACK_RESET=1 npm run test:e2e`（e2e 65 passed） | 0 | 120s |
| `tools/check-immutable.sh` | 0 | 58s |
| `tools/check-offsite.sh` | 0 | 4s |
| `tools/check-log-private.sh` | 0 | 1s |
| `tools/smoke.sh` | 0 | 34s |

ほかに `tools/check-openapi.sh` も OK（API の契約は変わっていない）。`cargo test` に時間がかかるのは、Task 5 の測りで座標つきの行が足されたままのこのワークツリーの DB で走らせたため（verify の後半の `db.sh down -v` で作り直された）。
