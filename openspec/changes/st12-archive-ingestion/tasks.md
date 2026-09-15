# ST12 実装タスク — 書庫を置くだけで過去のデータが入る

読む順: `deep.md`（**最優先。本人が決めたこと**）→ このファイル → `specs/external-ingestion/spec.md` → `design.md`
→ `docs/stories/ST12.md` → `docs/handoff/`（開始時と PR 前の 2 回）→ `CLAUDE.md`。

**移行は 1 本だけ足す**（design D14）。**名前は作成時刻 `YYYYMMDDHHMM_archive_ingestion.sql`**（連番にしない）で、
`crates/server/src/lib.rs` の `MIGRATIONS` 配列の末尾に足す。
**`record-envelope` と `collection-coverage` の判定、`/ingest` と `/heartbeat` と `/coverage` の応答の形、端末側・PC 側の収集には触らない**（design D13）。

検証は各タスクの本文に書いてある。「動いた」ではなくコマンドと終了コードで判定する。
DB を使う検査は `docker compose up -d db` が前提。

## 0. 規律（**最初に読む**）

- **テストには `Scenario: <名前>` の印を置く。** Rust / TypeScript はコメント（`// Scenario: 同じ書庫をもう一度置いても行が増えない`）、bash は `echo`。
  `scripts/check_scenarios.py` が spec の全 Scenario と突き合わせ、**印の無い Scenario を FAIL にする**。印の名前は spec の `#### Scenario:` と**一字一句合わせる**
- **この change が足す Scenario は 58 本**（すべて `external-ingestion`）
- **件数つき検証**: `cargo test <絞り込み>` は一致するテストが 0 本でも rc=0 になる。このファイルで **`CT <絞り込み>`** と書いたものは、
  `bash -o pipefail -c 'cargo test -p ashiato-server <絞り込み> 2>&1 | tee /tmp/ct.log' && grep -Eq 'test result: ok\. [1-9][0-9]* passed' /tmp/ct.log` が rc=0 になることを指す。
  **`VT <ファイル>`** は `bash -o pipefail -c 'cd web && npx vitest run src/__tests__/<ファイル> 2>&1 | tee /tmp/vt.log' && grep -Eq 'Tests +[1-9][0-9]* passed' /tmp/vt.log` が rc=0
- **本人の決定（下流は変えない）**: 読む中身は 6 つすべて（Q1）/ 読んだ製品のファイルの写しを残し、専用のフォルダの書庫は取り込み済みへ移す、写しは設定で残さない側にでき既定は残す、本人のファイルは消さない（Q2）/
  置き場は専用のフォルダ + ダウンロードのフォルダの `takeout-*.zip`（Q3）/ 最終日はいちばん新しい出来事の日（Q4）/
  書庫のソースにも格子・見出しに最終日と何日前・群の頭に直近の書庫 1 件・まだ無いは「まだ無い」（Q5）/ 重複は内容の鍵だけ（Q6）/
  タイムラインは 60 日で登録（Q7）/ 運ぶ仕組みを作らない（Q8）
- **D1 / D2 のマイアクティビティの名前 / D3 / D5 の大きなファイルの読み方 / D6 / D7 の場所の上限 / D9 の写しの置き場 / D10 / D11 は（仮）決め。** 反転条件は `design.md`。変えたらその D 番号を書き直す
- **ログに出すのは件数・論理ソースの名前・書庫の中のファイルの種類・所要時間・失敗の種別だけ**（製造準備 A-2。spec の最後の Requirement）。題名・URL・検索語・座標は出さない
- **本人の書庫を commit しない。** 試験の書庫は `crates/server/tests/fixtures/archive/` に公開されている形から手で作る（design D15）
- **`id` は記録ごとに毎回新しい uuid**（FR-21）。外部サービスの値から導かない

## 1. 移行（design D14 / D7 / D8 / D2）

- [ ] 1.1 移行 `migrations/YYYYMMDDHHMM_archive_ingestion.sql` と `.down.sql` を足す —— `core.archive_ledger` / `core.archive_ledger_source` / `core.archive_file`（`user_id` あり。
  UPDATE / DELETE / TRUNCATE を拒むトリガ）、`core.archive_sighting`（書き換えてよい）、索引 3 本（design D14）、
  登録簿の固定の 10 本（`external_id_kind = 'none'`・`expected_gap_sec = 5184000`・`ON CONFLICT DO NOTHING`）。**当て直せる形**。`MIGRATIONS` 配列の末尾に足す。
  Scenario: `書庫のソースは 60 日で登録されている` / `本人が変えた想定間隔は移行を当て直しても戻らない`。
  検証: `tools/check-migrations.sh` rc=0、`CT archive_migration`（2 回当てて rc=0 / 10 本の行 / 30 日に変えて当て直しても 30 日）
- [ ] 1.2 `tools/check-immutable.sh` に台帳の 3 表を足す（全列の UPDATE と DELETE と TRUNCATE が拒まれる）。
  Scenario: `台帳の行は書き換えられない`。検証: `tools/check-immutable.sh` rc=0

## 2. 格納の関門の切り出し（design D4 / D10）

- [ ] 2.1 `ingest_one` を「JSON の解釈」と `store_one(pool, IngestRequest) -> StoreOutcome` に分ける。`DuplicateOfDeleted` を分ける。
  **HTTP の応答と拒否の理由は変えない。**
  検証: 既存の `CT api_tests` と `CT dedup_tests` と `CT registry_tests` がすべて通る、`git diff --exit-code origin/main -- crates/server/src/api_tests.rs crates/server/src/dedup_tests.rs` rc=0（既存の試験を書き換えずに通す）、
  `CT store_one_outcome`（新規・重複・削除済みの内容の重複・拒否の 4 つを見分ける）
- [ ] 2.2 `heartbeat_one` の本体を同じ形で `store_heartbeat(pool, HeartbeatRequest)` に切り出す。
  検証: 既存の `CT heartbeat` がすべて通る、`tools/check-openapi.sh` rc=0（`/heartbeat` の形が変わらない）

## 3. 置き場の走査（design D1 / D8）

- [ ] 3.1 `crates/server/src/archive/config.rs` —— 環境変数（D1 の表）を読む。`ASHIATO_ARCHIVE_KEEP_COPIES` の綴り違いは起動を止め、`ASHIATO_ARCHIVE_USER_ID` が無ければ取り込み器を起こさない。
  Scenario: `設定を指定しなければ写しが残る`。
  検証: `CT archive_config`（未設定 → 既定の値 / `KEEP_COPIES=flase` → Err / 利用者識別子が無い → 取り込み器なし）
- [ ] 3.2 `crates/server/src/archive/scan.rs` —— 2 つの置き場の一覧（専用: `.zip` / `.json`、ダウンロード: `takeout-*.zip`、`取り込み済み` は見ない、一時ファイルの名前は見ない）、
  `core.archive_sighting` による安定の確認とハッシュの省略、読み終えた書庫の判定（`sha256` + `PARSER_VERSION`）、置き直しの `already_read`。
  Scenario: `ダウンロードのフォルダの他のファイルは読まれない` / `書き込み途中のファイルは読まれない` / `大きさが変わり続けているファイルは読まれない` /
  `読んだ書庫は次の走査で読み直されない` / `同じ書庫を置き直すと台帳に 1 行残る`。
  検証: `CT archive_scan`（一時ディレクトリで。2 回目の走査でハッシュの計算が呼ばれないことを数える）
- [ ] 3.3 `run()` から取り込み器を `tokio::spawn` で起こす（間隔 `ASHIATO_ARCHIVE_SCAN_SEC`）。1 冊ずつ直列。
  検証: `CT archive_worker_starts`（間隔 1 秒で起こし、置いた書庫が 10 秒以内に台帳に入る）

## 4. 書庫を開いて見分ける（design D3 / D5 / D7）

- [ ] 4.1 `zip` の依存を足し（許諾を `tools/check-licenses.sh` で確かめる）、書庫の中のファイルを列挙する。壊れた zip は `broken_zip`、`.tgz` などは `unsupported_format`。
  Scenario: `読めない形の書庫は台帳に残る` / `分割書庫は 1 本ずつ読まれる`。
  検証: `tools/check-licenses.sh` rc=0、`CT archive_open`
- [ ] 4.2 形と名前で見分ける（design D3 の表。日本語の名前を含む）。当たらないファイルは読まなかったに数え、HTML は読めなかったに数える。
  Scenario: `対象でないファイルは読まれず数だけ残る` / `HTML のマイアクティビティは読めなかったものとして残る`。
  検証: `CT archive_classify`（英語と日本語の名前の書庫の両方で、同じ見分けになる）
- [ ] 4.3 字句の走査で配列の項目の境目を 1 MiB ずつ探し、`raw` をバイト列の範囲そのままで切り出す（64 MiB の上限。UTF-8 でない範囲は読めなかった項目）。
  Scenario: `原文は書庫のバイト列の一部と一致する`。
  検証: `CT archive_slice`（字下げ・改行・エスケープされた `"` と `]` を含む配列で、各 `raw` がファイルの部分列と一致し、境目が 1 MiB の読みの切れ目をまたいでも同じ）
- [ ] 4.4 検証: 大きなファイルでメモリに載せないこと —— `CT archive_slice_large`（200 MiB の合成の `Records.json` を読み、1 件ずつ受け取る側で同時に持った件数の最大が 1 であることを数える）

## 5. 解析器（design D2 / D5 / D6）

- [ ] 5.1 地域の決め方（ずれを持てばそのずれと `Etc/GMT±N`、持たなければ 0 と `UTC`、`startTimeTimezoneUtcOffsetMinutes` を優先、`tz_from_source`）。
  Scenario: `ずれを持つ時刻はそのずれで残る` / `UTC しか持たない時刻は UTC と印で残る`。検証: `CT archive_tz`
- [ ] 5.2 `Timeline.json`（訪問 / 移動 / 経路の点 / 生の信号 → 4 本の論理ソース）。
  Scenario: `タイムラインの訪問と経路の点は別の論理ソースに入る` / `書庫の記録は収集したに分類される`。検証: `CT archive_parse_timeline`
- [ ] 5.3 移行前のロケーション履歴（`Records.json` の `locations`、Semantic Location History の `placeVisit` / `activitySegment`。`E7` を度に、`timestamp` と `timestampMs` の両方）。
  検証: `CT archive_parse_legacy`
- [ ] 5.4 YouTube の視聴履歴と検索履歴（`titleUrl` で見分ける。検索語は `search_query` を復号）。検証: `CT archive_parse_youtube`
- [ ] 5.5 マイアクティビティ（`products[0]` から論理ソースの名前を作り、登録簿に無ければ 1 行足してから格納する。design D2）。
  検証: `CT archive_parse_myactivity`（ASCII の名前 / 日本語の名前 → `u` + 12 桁 / 同じ名前の 2 回目で登録簿の行が増えない）
- [ ] 5.6 Chrome の履歴（`time_usec` を UTC に）。検証: `CT archive_parse_chrome`
- [ ] 5.7 6 つの中身を 1 冊ずつ格納する経路を通す（`store_one` を呼ぶ。`device_id = s01-c03`・`origin = collected`・外部識別子なし・`archive_sha256` と `inner_path` を payload に）。
  Scenario: `専用のフォルダに置いた書庫が読まれる` / `ダウンロードのフォルダの Takeout の書庫が読まれる` / `端末から書き出したタイムラインを専用のフォルダに置くと読まれる` /
  `6 つの中身がそれぞれ読まれる` / `記録から運んだ書庫が分かる` / `書庫の位置は携帯端末の位置に入らない` / `書庫の論理ソースは成功条件 1 の達成に数えられない`。
  検証: `CT archive_end_to_end`（合成の書庫と `Timeline.json` と `Records.json` を置き場に置き、走査 1 回で 6 つの中身から 1 件以上 / `c01-location` の件数と収集開始日が変わらない / `/coverage/achievement` の対象が 5 本のまま）

## 6. 重複・削除済み・読めない項目（design D4 / D7）

- [ ] 6.1 Scenario: `同じ書庫をもう一度置いても行が増えない` / `同じ出来事を含む別の書庫を置いても行が増えない` / `題名が変わった同じ視聴は別の記録として残る` /
  `古い書庫を後から置いても新しい記録は書き換わらない` / `消した記録は書庫を置き直しても戻らない` / `書庫の位置と端末の位置が同じでも取りやめない`。
  検証: `CT archive_dedup`（「別の書庫」は中身のハッシュが違う合成の書庫で。「消した」は `core.event` の `deleted_at` を立ててから置き直す）
- [ ] 6.2 1 件が読めなくても続け、読めなかった件数と場所（先頭 100 件）を残す。DB の失敗で中断したら台帳に書かない。
  Scenario: `壊れた 1 件があっても残りは格納される` / `格納が落ちた書庫は次の走査で読み直される`。
  検証: `CT archive_partial`（DB の失敗は `store_one` を差し替えた試験用の格納で 5 件目に Err を返し、台帳に行が無く、差し替えを戻した次の走査で全件）

## 7. 台帳・写し・本人のファイル（design D7 / D8 / D9）

- [ ] 7.1 台帳の 1 行を読み終えてから INSERT する（件数・作られた時刻の出所・置き場の種類・`unreadable_kind`・読まなかったファイルの数）。
  Scenario: `読んだ書庫の件数が台帳に残る` / `削除済みで入れなかった件数が台帳に残る` / `名前の時刻を持たない書庫は見つけた時刻を持つ` / `台帳に記録の本文は載らない`。
  検証: `CT archive_ledger`（本文が載らないことは台帳の 3 表を `row_to_json` で文字列にして検索語を含まないことで見る）
- [ ] 7.2 写し（中身の名前・同じ中身は 1 つ・読んだ製品のファイルだけ・設定が `false` なら作らない）と `core.archive_file` の目録。
  Scenario: `読んだ製品のファイルの写しが残る` / `残さない設定では写しを作らない` / `同じファイルの写しは 1 つ`。検証: `CT archive_copy`
- [ ] 7.3 専用のフォルダの書庫を台帳の後で `取り込み済み` へ移す（同じ名前は ` (2)`）。ダウンロードのフォルダは読むだけ。
  Scenario: `専用のフォルダの書庫は取り込み済みへ移る` / `ダウンロードのフォルダの書庫は動かない`。
  検証: `CT archive_move`（移した後のハッシュが一致 / ダウンロードのフォルダのファイルの大きさ・更新時刻・ハッシュが読む前と同じ）
- [ ] 7.4 解析器の版が上がったら、覚えている書庫を写しから（無ければ置き場から）読み直す。
  Scenario: `解析器の版が上がると読み直される`。検証: `CT archive_reparse`（版を 1 つ上げた試験用の定数で起こす）

## 8. 取り込み器の生存信号（design D10）

- [ ] 8.1 走査の回数と成功の回数を数え、`Asia/Tokyo` の日が変わって最初の走査で `c03-*` の全論理ソースに 1 件ずつ残す（`store_heartbeat` を呼ぶ）。
  Scenario: `取り込み器が動いている日に生存信号が残る` / `置き場が読めない日は取れない状態で残る`。
  検証: `CT archive_heartbeat`（時計を差し替えて日をまたがせる / 専用のフォルダを消して `dedicated_inbox_unreadable`）

## 9. 最終日と API（design D11 / D12）

- [ ] 9.1 `GET /archives/status`（`sources[].last_event_on` と `last_archive_created_at`、`latest_archive`）を足し、OpenAPI に載せる。削除済みの行も最終日に含める。
  Scenario: `最終日はいちばん新しい出来事の日` / `古い書庫を後から置いても最終日は戻らない` / `日本時間で日をまたぐ出来事は日本時間の日になる`。
  検証: `CT archives_status`、`cargo run -p ashiato-server --bin openapi > docs/openapi.json && tools/check-openapi.sh` rc=0、`grep -c '"/archives/status"' docs/openapi.json` が 1 以上
- [ ] 9.2 `coverage_get` の名前の並びに登録簿の `c03-*`（`display_name` 順）を Must の 5 本の後ろに足す。`achievement_get` は変えない。
  Scenario: `記録の無い日も同じ判定で出る`（サーバ側: 取り込み器の生存信号があり記録が無い日が `alive_no_record`）。
  検証: `CT coverage_includes_archive_sources`、既存の `CT coverage` がすべて通る

## 10. 画面（design D12）

- [ ] 10.1 `web/src/archives.ts`（`/archives/status` の型と「`YYYY-MM-DD` まで（N 日前）」の文字列）と、`App.tsx` の並び（Must → 書庫のソース → 退役）と別々の読み出し。
  Scenario: `書庫のソースは Must の後ろで退役の前に並ぶ`。検証: `VT archive-order.test.tsx`
- [ ] 10.2 `CoverageGrid.tsx` に見出しの任意の注記を足す。書庫のソースに最終日と何日前、まだ無いソースに「まだ無い」。
  Scenario: `書庫のソースの見出しに最終日と何日前が出る` / `まだ無いソースはまだ無いと出る` / `記録の無い日も同じ判定で出る`（画面側: 週を選ぶと「動いていた・記録なし」の文字）。
  検証: `VT archive-heading.test.tsx`（今日を 2026-09-15 に固定して「2026-09-12 まで（3 日前）」）
- [ ] 10.3 書庫のソースの格子の群の頭に「直近に置いた書庫」の箱（読めた / 読めなかった / 台帳が空）。
  Scenario: `直近に置いた書庫の結果が群の頭に出る` / `読めなかった書庫は文字で出る` / `書庫が 1 つも置かれていないことが出る`。
  検証: `VT latest-archive.test.tsx`
- [ ] 10.4 ひとスクロールと 360 px。
  Scenario: `書庫のソースを足しても Must の 5 本はひとスクロール以内` / `書庫のソースの格子は 360 px に収まる`。
  検証: `VT archive-one-scroll.test.tsx`（書庫のソース 12 本と箱を足した応答で Must の最後の格子の下端 ≤ 1,280 px、横スクロール無し、週の帯 ≥ 24 px）、
  既存の `VT one-scroll.test.tsx` と `VT target-size.test.tsx` が書き換えずに通る（`git diff --exit-code origin/main -- web/src/__tests__/one-scroll.test.tsx web/src/__tests__/target-size.test.tsx` rc=0）
- [ ] 10.5 検証: `cd web && npx tsc -b && npm run lint && npm run build` rc=0、`tools/check-boundaries.sh` rc=0

## 11. ログ・道具・手順書（design D15）

- [ ] 11.1 取り込み器のログを件数・論理ソースの名前・ファイルの種類・所要時間・失敗の種別だけにする。
  Scenario: `取り込みのログに検索語が出ない`。
  検証: `CT archive_log_is_private`（`tracing` の出力を集める試験用の層で、検索語「京都 旅館」・題名・URL・座標の文字列が 0 件）
- [ ] 11.2 `tools/archive-shape.sh <書庫>` —— 書庫の中のパス・最上位の鍵・項目の欄の名前・件数・`products` の値の種類だけを出す（値は出さない）。
  検証: 合成の書庫で `tools/archive-shape.sh crates/server/tests/fixtures/archive/takeout-sample.zip | grep -c '京都'` が 0 かつ rc=0、`watch-history` の件数の行が出る
- [ ] 11.3 `tools/smoke.sh` に 1 段足す: 一時ディレクトリを置き場にしてサーバを起こし、合成の Takeout の書庫を置き、`/archives/status` の `last_event_on` が合成の最後の日になり、同じ書庫を別名で置いて `/coverage` の件数が変わらないことを `jq -e` で見る。
  検証: `tools/smoke.sh` rc=0
- [ ] 11.4 `tools/seed.sh normal` に、書庫のソースの記録（視聴履歴の最終日を今日の 3 日前）と台帳の 1 行（読めた書庫）を足す（確認バッチの画面で見出しと箱が見える材料）。
  検証: `tools/seed.sh normal` rc=0 の後、`curl -sf -H "authorization: Bearer $API_TOKEN" http://127.0.0.1:18787/archives/status | jq -e '.latest_archive.outcome=="read"'` rc=0
- [ ] 11.5 `docs/archive-inbox.md` —— 置き場の設定（D1 の環境変数）、Takeout の予約エクスポートで **JSON の形を選ぶ**こと、端末でタイムラインを書き出して**網の外に出ない手段**（Tailscale のファイル送信・USB）で専用のフォルダへ運ぶ手順（本人の決定 Q8）、
  最初の本物の書庫の前に `tools/archive-shape.sh` を回すこと。検証: `grep -c 'ASHIATO_INBOX_DIR' docs/archive-inbox.md` と `grep -c 'archive-shape' docs/archive-inbox.md` がどちらも 1 以上

## 12. まとめの検査

- [ ] 12.1 検証: `cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace` rc=0、`cd web && npm run test && npm run lint && npm run build` rc=0
- [ ] 12.2 検証: `python3 scripts/check_scenarios.py . st12-archive-ingestion` rc=0（58 本すべてに印）
- [ ] 12.3 検証: `python3 scripts/check_chain.py .` rc=0、`openspec validate st12-archive-ingestion --strict` rc=0、
  `tools/check-migrations.sh` / `tools/check-openapi.sh` / `tools/check-boundaries.sh` / `tools/check-immutable.sh` / `tools/check-licenses.sh` がすべて rc=0
- [ ] 12.4 PR 本文に **仮決め（D1 / D2 / D3 / D5 / D6 / D7 / D9 / D10 / D11）と反転条件**、**写しをバックアップ（ST30）に入れる申し送り**（design D9）を列挙する。
  検証: `gh pr view --json body -q .body | grep -cE "D(1|2|3|5|6|7|9|10|11)（仮）"` が 9 以上、`gh pr view --json body -q .body | grep -c ST30` が 1 以上
- [ ] 12.5 `docs/handoff/` を読み直す（開始時と PR 前の 2 回）。検証: `ls docs/handoff/ST12.md 2>/dev/null` が空か、あればその各項目に PR 本文で触れている

## 人間の確認待ち

**機械で確かめられないのは「違和感」と、本物の Google の書き出しだけ**（2026-09-14 の決定）。正しさは 11 章までのテストが持つ。

- [ ] H.1 本物の Takeout の書庫（YouTube・マイアクティビティ・Chrome を JSON で）と、端末から書き出した `Timeline.json` を専用のフォルダに置き、
  稼働状況の「直近に置いた書庫」で**読めなかった 0**、各ソースの見出しに最終日が出るか（合成の書庫は Google の実物の形の揺れを再現できない。design D3 / D2）。
  置く前に `tools/archive-shape.sh` の出力（値を含まない）を貼り、マイアクティビティの `products` の値を確かめる
- 確認バッチ（`/verify`）の手順書が、この Story について「触ってみて違和感は無かったか」を 1 問だけ聞く
  （見るもの: `SEED=normal` の稼働状況で、Must の 5 本の後ろの書庫のソースの格子・見出しの「まで（N 日前）」・直近に置いた書庫の箱）
