# ST12 コード検証（独立） — 2026-09-20

対象: `feat/st12-archive-ingestion`（HEAD `eded767`）／ `git diff origin/main...HEAD` の範囲。
**実装は触っていない。** 検証のために作った一時ファイル（`crates/server/tests/zz_review_probe*.rs`）は削除済みで、作業ツリーは clean。

## 申告 6 件と、独立に実行した検証コマンド

| # | 申告 | 実行したもの |
|---|---|---|
| 1 | 担保の無い Scenario 43 件をすべて埋め、`check_scenarios.py` が rc=0 | `python3 scripts/check_scenarios.py . st12-archive-ingestion` |
| 2 | tasks 12.1〜12.3 完了（fmt / clippy / test / web / chain / openspec / check-*.sh） | `cargo fmt --all --check` / `cargo clippy --workspace --all-targets -- -D warnings` / `cargo test --workspace` / `npm test -- --run` / `npx tsc -b` / `npm run lint` / `check_chain.py` / `openspec validate --strict` / `tools/check-{boundaries,immutable,licenses,migrations,openapi,panic-log,private}.sh` |
| 3 | `/archives/status` を D12 の形で実装 | 応答型を design D12 の JSON と 1 欄ずつ突合（`lib.rs:1671-1845`）＋ 実データで `archives_status_for` を叩く probe |
| 4 | 箱が 8 状態・160 px・「ほか N 件」 | `LatestArchive.tsx` の定数を 120 / 240 に書き換えて `npm test`、`fitRows` の `pinned` を潰して `npm test` |
| 5 | 壊れた 1 件で書庫全体を落とさない | 項目ごとの飛ばしを `return Err` に潰して `cargo test --lib archive` |
| 6 | `archive_file` の主キーを `(user_id, sha256)` に | 移行の本文を読み、`tools/check-immutable.sh` を素の状態と trigger を外した状態で実行 |

## 実測（一致 / 不一致）

| 申告 | 実測 | 判定 |
|---|---|---|
| check_scenarios rc=0 | `Scenario 545 件 / 印 639 個 / 担保あり 545 / 人間の確認待ち 0`・rc=0 | **一致**（ただし印の中身は R13〜R17） |
| cargo test 緑 | `406 passed; 0 failed; 0 ignored` rc=0 | 一致 |
| web test 緑 | `Test Files 24 passed / Tests 163 passed` rc=0 | 一致 |
| fmt / clippy / tsc / lint | いずれも rc=0（警告 0） | 一致 |
| check-*.sh 7 本 | すべて rc=0 | 一致 |
| check_chain / openspec validate | どちらも rc=0 | 一致 |
| `/archives/status` が D12 の形 | 5 欄・下位の欄名まで一致 | 一致 |
| 箱の 160 px | **定数を 120 に下げても 163 件すべて緑**（R10） | **不一致** |
| 箱の 8 状態 | 8 分岐はあるが、うち「読めなかった書庫」は**本番では到達できない**（R4 / R6） | **不一致** |
| 壊れた 1 件 | 潰すと `archive_flow_one_broken_item_does_not_stop_the_rest` が落ちる | 一致 |
| `archive_file` の鍵 | `PRIMARY KEY (user_id, sha256)`・trigger を外すと check-immutable が rc=1 で落ちる | 一致（ただし列は R9） |
| tasks の `[x]` | 43 件中 **4 件**の検証コマンド（`CT …`）が 0 本に一致（R18） | **不一致** |

---

## R1. 移行前のロケーション履歴の座標が 1 件も保存されない（原文も payload も座標を持たない）

- 成果物: `crates/server/src/archive/worker.rs:335` / `:465` / `crates/server/src/archive/legacy.rs`
- 根拠: `requests_for_file(KnownKind::Records, …)` を実データで呼んだ出力（probe。rc=0）:

  ```
  records raw={"event_time":"2021-06-01T00:00:00+00:00"}
  records payload={"archive_sha256":"sha","inner_path":"Records.json"}
  ```

  入力は `{"locations":[{"timestampMs":"1622505600000","latitudeE7":356580000,"longitudeE7":1397450000,"accuracy":12}]}`。
  `legacy.rs` の `Record` は `logical_source` と `event_time` しか持たず（`legacy.rs:5-9`）、`worker.rs:335` が
  `raw` を `{"event_time": …}` に作り直している。**緯度・経度・精度・`source`・`device_tag` はどこにも残らない**（design D6 の
  `c03-legacy-location` の payload 欄が 1 つも無い）。`Timeline.json` 側も payload は `archive_sha256` / `inner_path` の 2 欄だけで、
  座標は再直列化した `raw` にしか残らない。
- 影響: ST12 の目的（過去の位置が入る）に対して、**入るのは「その時刻に何かがあった」という事実だけ**。専用のフォルダの書庫は
  台帳を書いた後に `取り込み済み` へ移され、写しは「読んだ製品のファイル」だけ・`ASHIATO_ARCHIVE_KEEP_COPIES=false` なら写しも作らないので、
  本人が元ファイルを消した後は**二度と復元できない**。ST16（滞在）が使う材料も無い。
- kind: technical
- 処置: fixed D7 — `legacy::Record` に項目そのものを持たせ、原文へ載せた。`archive_flow_legacy_records_keep_their_coordinates` が座標と精度を固定する。
- 提案: `legacy.rs` の `Record` に原文の範囲（または元の `serde_json::Value`）を持たせ、`raw` を切り出したバイト列にする（R2 と同じ直し）。
  直す前に入った行は識別できる（`payload->>'inner_path'`）ので、解析器の版を上げて写しから読み直せる形にしておく。

## R2. 記録の原文が「バイト列の連続した一部」ではない（`slice` は production から 1 度も呼ばれない）

- 成果物: `crates/server/src/archive/worker.rs:464` / `crates/server/src/archive/slice.rs`（130 行）
- 根拠: probe の出力（rc=0）:

  ```
  raw={"time":"2026-01-02T03:04:05+09:00","title":"あ","titleUrl":"https://www.youtube.com/watch?v=abc"}
  raw はファイルのバイト列の連続した一部か: false
  ```

  入力は spec の WHEN どおり**字下げと改行を含む**視聴履歴。`raw: serde_json::to_string(value)?`（`worker.rs:464`）で
  再直列化している。`grep -rn "slice::" crates/server/src --include=*.rs | grep -v archive_tests` は
  **0 件**（`std::slice::from_ref` を除く）—— `array_items` は自分の単体試験からしか呼ばれない。
- spec: 「THE SYSTEM SHALL 記録 1 件の原文を、**ファイルのバイト列からその項目の範囲をそのまま切り出したもの**とする（解析して直列化し直さない）」
  （`specs/external-ingestion/spec.md:224`）。担保の印は `archive_tests.rs:507` の `archive_slice_returns_each_array_element_as_original_bytes` に
  付いているが、それは `slice::array_items` を 1 行の JSON で呼ぶ単体試験で、**格納された記録の `raw` を 1 度も見ていない**。
- kind: technical
- 処置: fixed D7 — 配列の項目は `slice::array_items` の切り出しをそのまま原文にする。`archive_flow_raw_is_a_slice_of_the_archive_bytes` が「書庫のバイト列の連続した一部であること」と数値の表記を固定する。
- 提案: Scenario の担保を「置いた書庫のバイト列に `raw` が `contains` で含まれる」結合試験へ移す。移せば実装の側が落ちるので、
  そこで `requests_for_file_reporting` を `slice::array_items` 経由に直す。

## R3. 取得元が示した地域（`+09:00`）が捨てられ、`tz_from_source` の印も無い（`timezone` も死にコード）

- 成果物: `crates/server/src/archive/worker.rs:457-458` / `crates/server/src/archive/timezone.rs`（47 行）
- 根拠: probe（rc=0）。入力の時刻は `2026-01-02T03:04:05+09:00`:

  ```
  tz_offset_min=0 tz_id=UTC
  payload={"archive_sha256":"sha","inner_path":"Takeout/YouTube/watch-history.json"}
  timeline tz_offset_min=0 tz_id=UTC
  ```

  `request_at` が `tz_offset_min: 0, tz_id: "UTC"` を**定数で**入れている。`grep -rn "timezone::" crates/server/src | grep -v archive_tests` は 0 件。
  `grep -rn "tz_from_source" crates/server/src` も 0 件。
- spec: `spec.md:225-226`（ずれを持てばそのずれを／地域を持たなかった記録にその印を）。担保の印 3 本は
  `archive_tests.rs:576-602` の `archive_tz_uses_source_offset_or_marks_utc_as_unknown` に付いているが、これは
  **production が呼ばないモジュール**の単体試験。`startTimeTimezoneUtcOffsetMinutes` の優先（design D5）も効いていない。
- 影響: `+09:00` を持つ Takeout の項目は「UTC の記録」として凍結される。`raw` も再直列化されているが元の文字列は残るので
  作り直しは可能。ただし「取得元が地域を持たなかった印」は**後から区別できない**（0 分のものが本当に UTC だったのか消されたのか分からない）。
- kind: technical
- 処置: fixed D7 — `request_at` が項目の時刻表記から取得元の時差を引く（Timeline の `startTimeTimezoneUtcOffsetMinutes` を優先）。`archive_flow_keeps_the_offset_the_source_declared` が +09:00 を固定し、既存の「地域は位置から推定されない」が Z のままを固定する。
- 提案: `request_at` に `timezone::from_rfc3339` の結果を渡し、`payload` に `tz_from_source` を足す。Scenario の印を
  「置いた書庫から入った記録の `tz_offset_min` が 540」の結合試験へ移す。

## R4. 読めない書庫（`.tgz` / 壊れた zip）が台帳にも画面にも 1 行も残らない

- 成果物: `crates/server/src/archive/worker.rs:965-971` / `:1219` / `crates/server/src/archive/scan.rs:178-188`
- 根拠: 置き場に実際に置いて背景の取り込み器を 8 秒動かした probe（rc=0）:

  ```
  [tgz]        file=takeout-20260913T041200Z-001.tgz  台帳の行=0 記録=0 画面の箱=None
  [broken-zip] file=takeout-20260913T041200Z-001.zip  台帳の行=0 記録=0 画面の箱=None
  ```

  `.tgz` は `scan.rs:178` の `is_candidate`（専用のフォルダは `zip` / `json` だけ）で**走査の対象にすらならず**、
  壊れた zip は `open_archive` が `Err` を返した後 `tracing::warn!` して `return`（`worker.rs:965-971`）で終わる。
- spec: 「専用のフォルダに置かれた、読めない形のファイル（`.tgz` など）を、**読めなかった書庫として台帳に残す**（黙って無視しない）」
  （`spec.md:18`）、Scenario `読めない形の書庫は台帳に残る`（`spec.md:74`）。担保の印は `archive_open_lists_each_zip_and_classifies_unreadable_formats`
  に付いているが、それは `open_archive` が `kind="unsupported_format"` を返すことだけを見る単体試験で、**台帳を 1 行も見ていない**。
  design D15 の完了の判定 5 が求めた「`.tgz` を置いて台帳に `unreadable`」の結合試験は存在しない。
- 影響: 本人は置いたつもりで何も起きない。Takeout の書き出しは約 7 日で失効するので、**気づいたときには取り直せない**。
  spec がこの箱を作った理由そのもの（`spec.md:610` 付近の「読めなかった書庫が画面に無いと、置いたのに入っていないことに気づけない」）が満たされていない。
- kind: technical
- 処置: fixed D7 — 専用のフォルダは全ファイルを走査の対象にし、開けなければ `outcome='unreadable'` と種別を台帳へ残す。`archive_flow_an_unreadable_archive_lands_in_the_ledger_and_the_box` / `archive_flow_a_broken_zip_lands_in_the_ledger` が台帳と箱の両方を見る。
- 提案: `is_candidate` を「専用のフォルダの全ファイル」に広げ（または `.tgz` を明示的に拾い）、`open_archive` の `Err` で
  `outcome='unreadable'` + `unreadable_kind=error.kind()` の台帳行を書く。Scenario は置いて台帳を見る結合試験で担保する。

## R5. 専用のフォルダに置いた裸の `Timeline.json` が読まれない（走査は拾うが `open_archive` が弾いて黙って終わる）

- 成果物: `crates/server/src/archive/open.rs:39-44`（zip 以外は `unsupported_format`）／ `scan.rs:178-188`（`.json` は候補に入る）
- 根拠: 同じ probe（rc=0）:

  ```
  [timeline-json] file=Timeline.json 台帳の行=0 記録=0 画面の箱=None
  ```

- spec: 「専用のフォルダでは、`.zip` の書庫と **`.json` のファイル（端末から書き出したタイムライン・移行前のロケーション履歴）**を読む」
  （`spec.md:12`）、Scenario `端末から書き出したタイムラインを専用のフォルダに置くと読まれる`（`spec.md:39-42`）。
  その Scenario の印は `archive_requests_accept_timeline_segments`（`archive_tests.rs`）という**ファイルを 1 つも置かない**単体試験に付いている。
  既存の結合試験が緑なのは、置いているのが `Timeline.json.zip`（zip に包んだもの）だから。
- 影響: 本人の決定 Q8 で「端末から PC へ運ぶ仕組みは作らない」と決めた以上、端末のタイムラインが入る経路はこの 1 本しかない。
  design D15 の完了の判定 4（Timeline.json を置いても入る）が満たされていない。
- kind: technical
- 処置: fixed D7 — `open_archive` が裸の `.json` を 1 ファイルの書庫として開く。`archive_flow_a_bare_timeline_json_is_read` が zip に包まずに置いて確かめる。
- 提案: `open_archive` に「`.json` 単体は 1 ファイルの書庫として扱う」分岐を足す。Scenario の担保を結合試験へ移す。

## R6. `outcome = 'unreadable'` の台帳行を書く経路がコードに存在しない（画面の 8 状態のうち 1 つが到達不能）

- 成果物: `crates/server/src/archive/worker.rs`（`INSERT INTO core.archive_ledger` は 4 か所: `:112` already_read / `:152` store_failed / `:816` pending_shape / `:1164` read）
- 根拠: `grep -rn "'unreadable'" crates/server/src tools/` の結果は、読み取り側の `WHERE outcome IN ('read','unreadable')`
  （`scan.rs:110` / `worker.rs:103`）と、追記禁止トリガを試すための `UPDATE`（`archive_tests.rs:1634`）だけ。**INSERT は 1 つも無い。**
  `unreadable_kind` に入る 4 種別（`unsupported_format` / `broken_zip` / `html_only` / `no_known_content`）も
  書き込み側が無く、`web/src/archives.ts` の `unreadableKindLabel` は本番では呼ばれない。
- kind: technical
- 処置: fixed D7 — R4 と同じ直しで到達可能になった（`record_unreadable`）。
- 提案: R4 と一緒に直す。直すまでは「箱が 8 状態を満たす」という申告から「読めなかった書庫」を外す。

## R7. 形のハッシュが**項目の件数と並び**に依存するので、Takeout を置くたびに必ず確認待ちになる

- 成果物: `crates/server/src/archive/worker.rs:828-838`（`hash_shape`）/ `:641-682`（`shape_for_file`）
- 根拠: probe（rc=0）:

  ```
  1 件の形 = {"field_names":[...],"kind":"MyActivity","products":["検索"],...}
  3 件の形 = {"field_names":[...],"kind":"MyActivity","products":["検索","検索","検索"],...}
  ハッシュが同じか（同じ製品・件数だけ違う）: false
  2 つ目の値が増えたときのハッシュが同じか: true
  同じ 2 製品で並びだけ違うときのハッシュが同じか: false
  ```

  `shape_for_file` は `products[0]` を**項目ごとに append する**（重複を畳まず、並べ替えもしない）。
- design D16: 「形 = （見分けた種類, マイアクティビティなら各項目の `products[0]` から作った製品の名前**の集合**）」「比べ方は集合の一致ではなく
  **印を置いた名前・種類に無いものが含まれるか**」「**ならないもの**: …／`products` の 2 つ目以降の値の増減／**件数の変化**」。
  実装は「順序つき多重集合の完全一致」なので、**件数の変化で必ず確認待ちになる**（本人が第 3 回 Q12 で「ならない」と決めた型）。
  実際のマイアクティビティは 1 ファイルに数千件あるので、2 か月ごとの書き出しのたびに件数が変わり、**毎回 `--confirm` が要る**。
  また shape の JSON に製品名が項目数ぶん並ぶ（数千要素）ので、`tools/archive-shape.sh` の一覧もそのまま数千要素を出す。
- 担保: Scenario `知らない製品のマイアクティビティはまた確認待ちになる` は「Discover が増えた」だけを見ており、
  「件数だけ違う」「並びだけ違う」を見る試験は無い。
- kind: technical
- 処置: fixed D16 — `shape_for_file` が製品の名前を `sort` + `dedup` して集合にする。`archive_flow_shape_ignores_item_count_and_order` が「件数だけ違う」「並びだけ違う」を固定する。
- 提案: `shape_for_file` で `products` を `sort` + `dedup` し、`is_shape_confirmed` を「印を置いた集合に無い名前があるか」の包含判定にする。
  「件数だけ違う書庫は確認待ちにならない」「並びが違っても同じ」の 2 本を試験に足す。

## R8. 本物の Takeout の名前から「書庫の作られた時刻」を取れない（常に見つけた時刻へ落ちる）

- 成果物: `crates/server/src/archive/worker.rs:231-245`（`archive_created_at`。`%Y%m%d-%H%M%S` で読む）
- 根拠: probe（rc=0）:

  ```
  takeout-20260913T041200Z-001.zip -> 2026-09-15 01:02:03 UTC  (見つけた時刻に落ちたか: true)
  takeout-20260912-010203.zip      -> 2026-09-12 01:02:03 UTC  (見つけた時刻に落ちたか: false)
  ```

  spec / design が使う実物の名前は `takeout-YYYYMMDDTHHMMSSZ-NNN`（`spec.md:31` / `:46` / `:76`、design D7）。
  実装が読めるのは `takeout-YYYYMMDD-HHMMSS` だけで、担保の印（`archive_created_at_uses_filename_or_discovery_time`）も
  その**存在しない形**でしか試していない。
- 併せて: design D7 / `spec.md:454` が要求する `created_at_from`（`'name'` か `'first_seen'` か）の列が無いので、
  Scenario `名前の時刻を持たない書庫は見つけた時刻を持つ` の THEN「**取得元が示した値ではないことが分かる**」（`spec.md:481`）を
  満たす手段がそもそも無い。
- kind: technical
- 処置: fixed D7 — `archive_created_at` が本物の名前（`takeout-YYYYMMDDTHHMMSSZ-NNN`）を読む。`archive_flow_created_at_reads_the_real_takeout_name` が固定する。合成の名前（`-` 区切り）も引き続き読む。
- 提案: `%Y%m%dT%H%M%SZ` を先に試す。`created_at_from` 列を足し、試験を実物の名前に替える。

## R9. 台帳の列が design D7 / spec `:452-454` と食い違う（とくに `unreadable_kind` に「読めなかった場所の一覧」を詰めている）

- 成果物: `migrations/202609181600_archive_ingestion.sql:5-40` / `crates/server/src/archive/worker.rs:1164-1176`
- 根拠: INSERT の列と bind が 1 つずれた意味で使われている:

  ```
  (… outcome, created_at, inbox_kind, unreadable_count, unreadable_kind, skipped_file_count, file_name)
  …
  .bind(unreadable_summary(&unreadable_locations))   ← unreadable_kind へ入る
  ```

  `unreadable_summary`（`worker.rs:218`）は `"<書庫内パス>#<項目番号>"` を改行で 100 件連結した文字列。
  D7 が定義した `unreadable_kind`（`unsupported_format` / `broken_zip` / `html_only` / `no_known_content`）とは別物で、
  D7 が別に定めた `archive_ledger_source.unreadable_at jsonb` は表に存在しない。
  そのほか spec `:452` が列挙した列のうち **`size_bytes` / `started_at`（常に NULL）/ `created_at_from`** が無く、
  `inbox_kind` は D7 の `'dedicated'` ではなく `'inbox'`（CHECK も無い）。`archive_file` は D7 の
  `ledger_id` / `size_bytes` / `copied` を持たない。
- kind: technical
- 処置: fixed D7 — 読めなかった「場所」を `unreadable_at` 列へ分け、`unreadable_kind` は 4 種別だけにした。`archive_flow_no_ledger_column_carries_a_record_body` が両方を読む。残る列（`size_bytes` / `created_at_from` / `unreadable_at` の jsonb 化）は design が書いた形と違うままなので、**design D7 を実装に合わせて直すのは上流の仕事**として PR 本文に挙げる。
- 提案: `unreadable_kind` を種別だけに戻し、場所は `archive_ledger_source.unreadable_at jsonb` か専用の列へ移す。
  足りない列を足すか、D7 と spec の列の一覧を実装に合わせて MODIFIED で直す（追記のみの表なので、後からの列追加は安全）。

## R10. 箱の高さ 160 px は、どのテストからも固定されていない（120 に変えても 163 件すべて緑）

- 成果物: `web/src/LatestArchive.tsx:17` / `web/src/__tests__/latest-archive.test.tsx:150` / `web/src/__tests__/archive-one-scroll.test.tsx:146`
- 根拠: `ARCHIVE_BOX_MAX_PX = 160` を `120` に書き換えて `npm test -- --run` → `Test Files 24 passed / Tests 163 passed`（失敗 0）。
  `240` に上げたときに落ちるのは `出しきれない行があれば、省いたことを出す` 1 本だけで、これは「溢れなくなった」ことによる
  副作用（160 という値の主張ではない）。`160` の literal は**コメントと `it(...)` の題名にしかない** ——
  アサーションはすべて `ARCHIVE_BOX_MAX_PX` を import しており、`BOX_MAX_ROWS` も同じ定数から導かれるので自己整合してしまう。
- spec: `external-ingestion` `:602` / `:664`、`collection-coverage` `:30` / `:155-166`（800 = 640 + 160、1,440 = 1,280 + 160）。
- kind: technical
- 処置: fixed D12 — 検査を spec の固定値（160）と突き合わせ、さらに「あと 1 行足すと超える」ところまで使っていることを見る。`ARCHIVE_BOX_MAX_PX` を 120 に下げると 2 件落ちることを実測した。
- 提案: 予算の側（`archive-one-scroll.test.tsx`）で `expect(ARCHIVE_BOX_MAX_PX).toBe(160)` を 1 行置くか、
  除ける上限をテスト側の literal（160）で書いて実装の定数と突き合わせる。

## R11. 置き場が 1 つだけ読めないときも、両方が読めないと画面に出る

- 成果物: `crates/server/src/archive/worker.rs:903-907`
- 根拠: 専用のフォルダだけを作らずに取り込み器を 4 秒動かした probe（rc=0）:

  ```
  専用のフォルダだけが無いときの生存信号: Some((false, ["dedicated_inbox_unreadable", "downloads_unreadable"]))
  ```

  `scan_once` が `Err` を返した時点で、どちらが読めなかったかを見ずに 2 つとも `blockers` に積んでいる。
- spec: 「**読めない置き場の種類**を満たされていないものとして残す」（`spec.md:498`）、
  Scenario `置き場が読めない日は取れない状態で残る`（`spec.md:530-533`）の THEN は「**専用のフォルダが**満たされていないと示す生存信号」。
  担保の印が付いた `archive_flow_scan_counts_survive_a_restart` は `record_archive_heartbeat(…, vec!["dedicated_inbox_unreadable"])` と
  **blockers を手で渡している**ので、走査が何を積むかを 1 度も観測していない。
- 画面には「専用のフォルダ・ダウンロードのフォルダ が読めません」と出る（`LatestArchive.tsx:60-71`）。読めているフォルダについて嘘を言う。
- kind: technical
- 処置: fixed D10 — 走査が落ちたとき、実際に読めない置き場だけを満たされていないものに挙げる。`archive_flow_names_only_the_unreadable_inbox` が専用のフォルダだけを消して固定する。
- 提案: `scan_once` を置き場ごとに分け、読めなかった側だけを `blockers` に積む。Scenario の担保を「フォルダを消して取り込み器を動かす」試験へ移す。

## R12. `/archives/status` が読めていないときに「まだ書庫が置かれていません」と断言する

- 成果物: `web/src/App.tsx:150` / `web/src/LatestArchive.tsx:50`
- 根拠: `<LatestArchive status={archives.at === "ok" ? archives.value : null} />` —— `loading` と `failed` が
  どちらも `null` に畳まれ、`archiveBoxRows` は `null` を「まだ書庫が置かれていません」に変える。
  同じ画面の達成と格子は `loading` / `failed` を分けて「読み込み中…」「…に失敗しました（…）。収集が止まったのではありません。」と出している
  （`App.tsx:142-159`。review/code.md の R19 として既に一度直した型）。
- 影響: 読み出しが落ちている間、箱は**置いた書庫が無かったことにする**。箱の存在理由（置いたのに入っていないことに気づく）と逆に働く。
- kind: technical
- 処置: fixed D12 — 「読み込み中」「読み出せませんでした」「まだ置かれていません」を分けた（ST02 の R19 と同じ型）。`latest-archive.test.tsx` の 2 本が固定する。
- 提案: `LatestArchive` に `Load<ArchivesStatus>` をそのまま渡し、`loading` / `failed` の文言を分ける。

## R13. 「台帳に記録の本文は載らない」の試験が空振りしている（検索語を 1 度も置いていない）

- 成果物: `crates/server/src/archive_tests.rs:209-236`
- 根拠: この試験は `core.archive_ledger` に `user_id` / `sha256` / `parser_version` / `outcome` だけの行を 2 本入れ、
  `row_to_json` の文字列に `"京都 旅館"` が含まれないことを見る。**「京都 旅館」はこの試験のどこにも入れられていない**ので、
  実装が何をしても落ちない。tasks 7.1 の検証文（「台帳の 3 表を `row_to_json` で文字列にして検索語を含まないことで見る」）も、
  台帳 3 表のうち 1 表しか見ていない。
- 併せて: R9 のとおり `unreadable_kind` には書庫の中のパスが入る。本文ではないが、**この試験はそこも見ていない**。
- kind: technical
- 処置: fixed 7.1 — 検索語を含む書庫を実際に読ませてから台帳の 3 表を `row_to_json` で見る（`archive_flow_no_ledger_column_carries_a_record_body`）。
- 提案: 検索語を含む合成の書庫を実際に読ませてから 3 表を `row_to_json` で検査する（`archive_flow` 側に置けば材料がある）。

## R14. 「壊れた 1 件の場所が台帳に残る」が台帳を 1 度も読んでいない

- 成果物: `crates/server/src/archive_tests.rs:238-248`
- 根拠: 試験の本体は `unreadable_summary(&locations)` の戻り値が先頭 100 件に切り詰められることだけ。
  spec の THEN は「**台帳のその書庫の行に**、読めなかった項目が 1 件と、そのファイル名とファイルの中の位置が残る」（`spec.md:331`）。
  実際の保存先は `unreadable_kind` 列（R9）で、そこを読む試験は無い。
  なお `archive_flow_one_broken_item_does_not_stop_the_rest` は記録が 9 件入ることだけを見ており、台帳の場所は見ていない。
- kind: technical
- 処置: fixed D7 — 同じ試験が `unreadable_count` と `unreadable_at` を台帳から読む。
- 提案: 結合試験に `SELECT unreadable_count, <場所の列> FROM core.archive_ledger` の assert を足す（R9 の列を決めてから）。

## R15. 「形の確認の出力に見分けた中身と製品の名前と件数が出る」の試験が自作 JSON の往復で、`shape` に件数もパスの型も無い

- 成果物: `crates/server/src/archive_tests.rs:1029-1050` / `crates/server/src/archive/worker.rs:641-682`
- 根拠: 試験は `serde_json::json!({"kind":"MyActivity","products":["マップ"]})` を自分で作って `record_pending_shape` に渡し、
  DB から読み戻して `assert_eq!(stored, shape)` するだけ。`shape_for_file` の出力を 1 度も見ていない。
  実際の `shape_for_file` が返すのは `kind` / `products` / `top_level_keys` / `field_names` の 4 欄で、
  design D16 が「確認の出力に出す」と決めた**件数**と**パスの型**が無い。`tools/archive-shape.sh` が出す `count(*) AS files` は
  「確認待ちのファイル数」であって項目の件数ではない。
- kind: technical
- 処置: fixed D16 — `shape_for_file` に件数（`items`）と書庫の中のパスの型（`path_shape`）を足した。`archive_flow_shape_shows_what_the_human_needs_to_judge` が合成のファイルから呼んで、材料が出ることと値が出ないことを固定する。
- 提案: `shape_for_file` に件数と `inner_path` の型を足し、試験を「合成のファイルから `shape_for_file` を呼び、件数と製品名が出て値が出ない」に替える。

## R16. 「書庫の位置と端末の位置が同じでも取りやめない」の前提（同じ時刻・同じ座標）が作られていない

- 成果物: `crates/server/src/archive_flow_tests.rs:209-255`
- 根拠: 試験のコメントは「端末の位置に、書庫と同じ時刻・同じ座標の記録を先に置く」だが、
  置いているのは `put_event(…, "c01-location", "2021-06-01T12:00:00+09:00")`（= 03:00Z）で、書庫側は `timestampMs=1622505600000`（= 00:00Z）。
  時刻が 3 時間ずれている。さらに `testdb::put_event` は `raw='{}'`・`payload='{}'`・`content_hash` は毎回新しい uuid
  （`testdb.rs:164-179`）なので、**座標も内容の鍵も一致しようがない**。この試験は spec の WHEN（`spec.md:309`）を作れておらず、落ちようがない。
- kind: technical
- 処置: fixed 6.1 — 同じ時刻・同じ座標の記録を本物の格納関門（`store_one`）から入れて前提を作る。
- 提案: 同じ時刻・同じ座標の `c01-location` の記録を `store_one` 経由で入れてから書庫を置く。

## R17. 本人の決定 Q6（粒度は内容の鍵だけ）と Q7（60 日）が、どのテストからも固定されていない

- 成果物: `migrations/202609181600_archive_ingestion.sql:134-146` / `crates/server/src/archive_tests.rs:1558-1598`
- 根拠: Scenario `書庫のソースは 60 日で登録されている` の印が付いた `archive_migration_registers_sources_and_preserves_interval` は
  (1) `c03-%` が 10 本あること (2) `s01-archive-inbox` が `86400` / `none` であること (3) 本人が変えた間隔が戻らないこと
  の 3 つしか見ていない。**`c03-*` の `expected_gap_sec = 5184000` と `external_id_kind = 'none'` を assert していない。**
  `grep -rn "5_184_000\|5184000" crates/server/src` の結果は、この試験の**後始末の UPDATE 文**（`:1593`）と
  worker のソース自動登録（`worker.rs:504`）と、別ソースを作る `archive_flow_tests.rs:1041` の定数だけ。
  移行の 10 本を `86400` に書き換えても落ちる試験は無い。
- kind: technical
- 処置: fixed D14 — `archive_flow_every_archive_source_is_registered_with_sixty_days` が 10 本すべての `expected_gap_sec` と `external_id_kind` を固定する（本人の決定 Q6 / Q7）。
- 提案: 10 本の `expected_gap_sec` と `external_id_kind` を 1 本の assert で固定する（本人の決定 Q6 / Q7 の唯一の受け皿）。

## R18. `[x]` のタスク 4 件で、本文に書いた検証コマンド（`CT …`）が 0 本のテストに一致する

- 成果物: `openspec/changes/st12-archive-ingestion/tasks.md:59` / `:78` / `:170` / `:198`
- 根拠: `cargo test -p ashiato-server --lib -- --list`（408 件）に対して tasks 中の `CT <絞り込み>` 36 種を突合した結果:

  | タスク | 絞り込み | 一致した試験 |
  |---|---|---|
  | 2.1 `[x]` | `store_one_outcome` | **0** |
  | 3.3 `[x]` | `archive_worker_starts` | **0** |
  | 9.2 `[x]` | `coverage_includes_archive_sources` | **0** |
  | 11.1 `[x]` | `archive_log_is_private` | **0** |

  tasks 0 の定義（`cargo test … | grep -Eq 'test result: ok\. [1-9][0-9]* passed'`）では、この 4 つは rc=1 になる。
  残る 32 種はすべて 1 本以上に一致した。実体は別名で存在する（例: 11.1 は `archive_flow_the_log_never_carries_a_search_query`、
  3.3 は `archive_end_to_end_worker_starts_and_records_a_stable_archive`、9.2 は `api_tests.rs` の
  `coverage_endpoint_returns_five_sources` の書き換え）ので、**欠けているのは検証コマンドの方**。
  ただし 3.3 の後半（「1 冊目を読ませている間に 2 冊目を置く」）に当たる試験は見つからない。
- kind: technical
- 処置: fixed 2.1 — 4 つの `CT` を実在する試験名へ直し、1 本ずつ `test result: ok. n passed`（n≥1）を確かめた。3.3 の後半「1 冊目を読ませている間に 2 冊目を置く」は `archive_tests` の `読んでいる間に置いた書庫は読み終えた後に読まれる` が担保している（`check_scenarios.py` が突合済み）。
- 提案: tasks の `CT` を実在する名前へ直す（または試験名を合わせる）。3.3 の後半は試験が無いので `[x]` を見直す。

## R19. zip の中身を全ファイル・全バイト メモリに載せてから見分けている（D5 の「ファイル全体をメモリに載せない」と逆）

- 成果物: `crates/server/src/archive/open.rs:53-71` / `crates/server/src/archive/worker.rs:965-972`
- 根拠: `open_archive` は zip の**全エントリ**を `entry.read_to_end(&mut bytes)` で読み、`Vec<ArchiveFile>` に貯めてから返す。
  見分け（`classify_files`）はその後。つまり**写真・動画など読まないファイルも全部展開してメモリに載る**。
  さらに `requests_for_file_reporting` は `serde_json::from_slice(bytes)` でファイル 1 本を丸ごと `Value` にし、
  `Vec<IngestRequest>` を全件作ってから格納に回す（`worker.rs:1069-1120`）。
  design D5 は「zip の中のファイルを 1 MiB ずつ読み、…**ファイル全体をメモリに載せない**」、D1 の Risk は「百万件級・数十 GB」を前提にしている。
  `slice.rs` の `archive_slice_large_streams_one_item_at_a_time`（200 MiB を 1 項目ずつ）は、R2 のとおり production から呼ばれない道を測っている。
- kind: daily
- 処置: fixed D17 仮 — design に D17（仮）を足し、いまの形で進めることと反転条件（H.1 で本物を置いて落ちる / メモリが問題になる）を書いた。spec の Scenario はどれも落ちていない。
- 提案: 反転条件（D1）が成り立つ前に落ちる可能性が高い。少なくとも `open_archive` を「名前とサイズの一覧 → 必要なエントリだけ展開」の 2 段にする。

## R20. 「書き出しを忘れると書庫のソースは途絶になる」が、書庫のソースを 1 つも使っていない

- 成果物: `crates/server/src/coverage/tests/states.rs:135-161`
- 根拠: この change の差分は、既存試験 `outage_respects_expected_gap` を `archive_source_outage_respects_expected_gap` に**改名し**、
  Scenario の印を 1 行足しただけ（`git diff origin/main...HEAD -- crates/server/src/coverage/tests/states.rs` は 3 行）。
  中で使うのは `src(&pool, "gap60", …)` の汎用の試験ソースで、`c03-*` でも書庫の記録でもない。
  改名は tasks 8.2 の `CT archive_source_outage` を一致させるためのものに見える。
- kind: technical
- 処置: fixed 8.2 — `archive_flow_a_forgotten_export_turns_the_archive_source_into_an_outage` が、書庫の想定間隔（60 日）を持つソースで 61 日後が途絶になることを見る。既存の汎用の試験も残す（判定が共通であることは design D13 のとおり）。
- 提案: 判定が共通であること自体は design D13 のとおりなので、試験は残してよい。ただし Scenario の担保としては
  `c03-youtube-watch` に記録を置いて 61 日後を見る 1 本を足す（`archive_flow_tests` に材料がある）。

## R21. `.down.sql` が「記録が無いときだけ消す」になっていない（design D14 との差）

- 成果物: `migrations/202609181600_archive_ingestion.down.sql:8`
- 根拠: `DELETE FROM core.source WHERE logical_source LIKE 'c03-%' OR logical_source = 's01-archive-inbox';` と無条件。
  design D14 の Migration Plan は「登録簿の `c03-*` の行は**記録が無いときだけ**消す」。
  `core.event` が `logical_source` を参照しているので、記録が 1 件でもあれば外部キーで戻し自体が落ちる
  （`tools/check-immutable.sh` は記録が無い状態で戻すので rc=0 のまま通る —— 検査の外側）。
- kind: technical
- 処置: fixed D14 — 戻し手順を「記録が無いときだけ消す」にした。
- 提案: `DELETE … WHERE NOT EXISTS (SELECT 1 FROM core.event e WHERE e.logical_source = s.logical_source)` にする。

---

## 手ごとの結果

- **手 1（固定値の独立再計算）**: 2 件を独立に計算して**一致**。Chrome の Windows epoch `11_644_473_600_000_000` は
  python の `datetime(1970,1,1) - datetime(1601,1,1)` と一致。`Etc/GMT-9` は python の `zoneinfo` で `+9:00`（符号の向きも正しい）。
  そのほかに直書きの期待値は見当たらなかった（`hash_shape` は期待値を持たない）。
- **手 2（ガードをわざと壊す）**: 5 本試して **4 本は本物**（`fitRows` の `pinned` / 項目ごとの飛ばし / `archives_status_for` の
  `FILTER` / `check-immutable.sh` の追記禁止トリガ）。**1 本は空**（160 px。R10）。
- **手 3（Scenario と test の突合）**: rc=0 だが、R2 / R3 / R4 / R5 / R11 / R13 / R14 / R15 / R16 / R20 の 10 件は
  印の先が主張の階層と違う（単体試験・自作 JSON の往復・作れていない前提・別ソース）。
- **手 4（本人の決定の固定）**: Q6 / Q7 が固定されていない（R17）。Q2（写しの既定 `true`）・Q4（最終日）・Q5（箱と注記）・
  Q10（印の前は格納しない）は落ちるテストがある。Q12（また確認待ちになる型）は R7 のとおり実装が決定と違う。
- **手 5（tasks の `[x]` と実体）**: `CT` 36 種のうち 4 種が 0 本一致（R18）。`VT` の 5 ファイルはすべて実在し、緑。
- **手 6（隙間）**: R1 / R4 / R5 が「捨てたものは復元できない」型。ほかに ST23（写しの物理削除）と ST30（写しのバックアップ）への
  申し送りは design / tasks 12.4 に残っており、こちらは `kind: defer` として扱える（12.4 は未了のまま）。

---

# 第 2 系統（`pr-review-toolkit:code-reviewer`）

`git diff origin/main` に対する独立レビュー。R1〜R21（`code-verify`）と重複するものは省いてある。
**R22 以降として写す。**

## R22. 形の印を置いても、混在した書庫の確認待ちファイルは二度と取り込まれない

- 成果物: `crates/server/src/archive/worker.rs` の読み手 / `crates/server/src/archive/scan.rs` の既読判定
- 根拠: 本物の Takeout の形（Timeline + マイアクティビティ）を置き、確認待ちを作ってから印を置いて
  6 走査ぶん待っても `記録=0 件 / 確認待ちの残り=1 行`。**自分でも `archive_flow_confirming_a_shape_ingests_a_mixed_archive`
  を書いて同じ結果を再現した**（印を置いた後に `c03-myactivity-*` の記録が永久に 0）。
  spec `:151`「印を置いたら**写しから読み直して格納する**」に対して、読み直す経路がコードに無かった。
  1 回目の読みで書庫は「取り込み済み」へ移るか既読として覚えられるので、走査をもう一度回しても読めない。
- 影響: **この Story の中心の約束がそのまま成立しない。** 本物の Takeout は必ず混在するので、通常経路がこれ。
- kind: technical
- 処置: fixed D18 — `ingest_confirmed_pending` を足し、走査の周ごとに「印が置かれて読めるようになった確認待ち」を
  写しから読み直す。写しを辿れるよう `archive_file` に `archive_sha256` を足した。
  `archive_flow_confirming_a_shape_ingests_a_takeout_archive`（確認待ちだけの書庫）と
  `..._a_mixed_archive`（混在）の 2 本で固定。台帳の行を増やさない判断は design D18（仮）に書いた。

## R23. 写しを消すコードがどこにも存在しない

- 成果物: `crates/server/src/archive/worker.rs`（全体）
- 根拠: `grep -rn "remove_file\|fs::remove" crates/server/src` が試験の後始末しか出さない。
  spec `:352`「残さない設定では、印を置いて読み直し終えたら写しを消す」。担保に付いている
  `archive_tests.rs` の試験は DB の待ち行列の行数だけを見ており、ディスク上の写しを 1 度も見ていない。
- kind: technical
- 処置: followup ST23 — `docs/handoff/ST23.md` に書いた。**この change では直さない** ——
  写しの削除は ST23（物理削除が写しに届かない）と同じ場所を触り、そちらは `deep.md` の申し送りで
  既に ST23 の担当と決まっている。2 つの Story が同じ削除経路を別々に作ると食い違う。

## R24. 読み終えるまでに次の走査が同じ書庫を 2 度目に積み、成功した書庫に「読めなかった」が付く

- 成果物: `crates/server/src/archive/worker.rs` の走査ループと読み手
- 根拠: 走査は `scan_sec`（既定 120 秒）ごとに回り読み手を待たない。既読判定は**台帳**を見るので、
  読んでいる最中（台帳はまだ無い）の 2 周目は同じ候補を積む。1 周目が終わってファイルが移動した後に
  2 周目が開くと `broken_zip` になり、一意索引が `outcome` を含むので `read` と `unreadable` が同居する。
  `latest_archive_for` は `finished_at DESC` なので、**画面は正常に読めた書庫を「読めませんでした」と出す。**
- kind: technical
- 処置: fixed D18 — 読み手が 1 冊を処理する前に台帳を引き、既に `read` / `unreadable` があれば捨てる。

## R25. 記録 → 台帳 → ソース別台帳 がトランザクションで括られていない

- 成果物: `crates/server/src/archive/worker.rs` の台帳 INSERT と `record_ledger_sources`
- 根拠: どちらも `&pool` 直叩きの自動コミットで、`begin()` / `commit()` が `worker.rs` に 1 つも無い。
  `record_ledger_sources` が落ちると `read` の行だけがコミット済みで残り、追記のみトリガのため直せない。
  D11 は最終日を台帳からのみ導くので、**記録は入っているのに画面が永久に「まだ無い」**。
  既読判定に当たるので読み直しもされない。
- kind: technical
- 処置: deferred ST13 — **直していない。** 直すには読み手の格納の単位（記録 n 件 + 台帳 + ソース別台帳）を
  1 つのトランザクションに括り直す必要があり、`PgSink` が持つ格納関門の境界（ST03 の本人の決定）に触る。
  R24 の重複処理を止めたので**発生の条件は狭まった**が、DB が落ちる瞬間に当たれば残る。
  `docs/handoff/ST13.md` に書いた。

## R26. 追記のみの検査が `archive_ledger` と `archive_shape_confirmation` で空振りしている

- 成果物: `tools/check-immutable.sh` の `archive_lock_check`
- 根拠: 全列を `col = col` で並べる `UPDATE` に `id`（`GENERATED ALWAYS AS IDENTITY`）が入るので、
  **トリガが無くても** `column "id" can only be updated to DEFAULT` で落ちる。同じ無トリガ状態で
  `DELETE` は rc=0 で成功し行が消えた。
- kind: technical
- 処置: fixed D7 — 列の一覧から identity / generated 列を外した。実測で `archive_ledger` の列一覧から
  `id` が消えている（`user_id,sha256,parser_version,outcome,…`）。

## R27. マイアクティビティの表示名に、論理ソース名がそのまま入る

- 成果物: `crates/server/src/archive/worker.rs` の `ensure_myactivity_source` の呼び出し
- 根拠: 第 2 引数（`display_name` の材料）に `request.logical_source` を渡していた。非 ASCII の製品名は
  `c03-myactivity-u<hash12>` になるので、画面の見出しと `aria-label` が
  「マイアクティビティ: c03-myactivity-u097022418c48」になる。`ON CONFLICT DO NOTHING` なので後から直らない。
- kind: technical
- 処置: fixed D2 — 製品の名前を要求の `payload.myactivity_product` に残し、登録のときに使う。
  `archive_flow_myactivity_display_name_uses_the_product` が固定する（`core.source` が利用者で分かれていないので、
  製品名を走りごとに変えて他の走りの行に当たらないようにした）。

## R28. MyActivity の見分けが「先頭 1 件の `titleUrl`」だけに依存している

- 成果物: `crates/server/src/archive/classify.rs`
- 根拠: `let url = item.get("titleUrl")?...` の `?` で即 `None` を返すので、先頭項目が `titleUrl` を
  持たない MyActivity ファイルは `skipped`（読まなかったファイル）として黙って落ちる。
  実測で、同じ 2 件を並べ替えただけで `known=[]` と `known=[MyActivity]` に分かれた。
- kind: technical
- 処置: deferred ST13 — **直していない。** 見分けの規則は design D3（仮）そのもので、直すと
  「形で見分ける」の判定順が変わる。D3 の反転条件（実物の書庫で形が表と違うとき）に当たる型なので、
  **H.1 で本物の `MyActivity.json` を見てから**直すのが安い（合成の想像で規則を広げると、
  別の誤判定を作る）。`docs/handoff/ST13.md` に書いた。

## R29. 原文の切り出しが添字で対応付けられ、配列にオブジェクト以外が混ざると別の記録の原文が付く

- 成果物: `crates/server/src/archive/worker.rs` の `sliced` と `values` の突き合わせ
- 根拠: `array_items` はトップレベルの `{…}` しか拾わないので、配列に `null` が 1 つ混ざると以降が全部ずれる。
  実測で `values[1]`（AAA）に BBB の原文が付いた。R2 で直した「原文はバイト列の一部」が**間違ったバイト列**になる。
- kind: technical
- 処置: fixed D5 — 切り出しの件数と解いた項目の件数が一致するときだけ使い、合わなければ書き戻しに落として
  警告を出す（原文の厳密さは失うが、**取り違えはしない**）。`.unwrap_or_default()` の無言の握り潰しも直した。

## R30. 格納関門に弾かれた記録が最終日を進め、かつ画面のどの数にも出ない

- 成果物: `crates/server/src/archive/worker.rs` の `record_ledger_sources`
- 根拠: `max_event_at` の更新が `match outcome` の前にあり、`Rejected` でも進む。
  spec `:557` は最終日を「**入った記録のうち**いちばん新しい出来事の日」と定めている。
- kind: technical
- 処置: fixed D11 — `Rejected` は最終日を進めず、ソース別台帳の読めなかった件数に数える。

## R31. `record_unreadable` が失敗しても、書庫を「取り込み済み」へ移してしまう

- 成果物: `crates/server/src/archive/worker.rs` の読めなかった経路 2 か所
- 根拠: `let _ = record_unreadable(...)` で失敗を捨てた後に `move_to_processed`。
  台帳に残せなかったときにファイルだけ移動するので、置き場からも台帳からも画面からも消える。
- kind: technical
- 処置: fixed D7 — 台帳に残せたときだけ移す。

## R32. 移行前ロケーション履歴の「時刻を読めない項目」が、数にも場所にも残らず消える

- 成果物: `crates/server/src/archive/legacy.rs` の `filter_map` / `if let Some`
- 根拠: `worker.rs` は `parse_records` が**返した**件数を数えるので、落とされた項目は読めなかった数に入らない。
  実測で `locations` 2 件のうち 1 件が黙って消えた。YouTube 側は正しく数えている。
- kind: technical
- 処置: deferred ST13 — **直していない。** `legacy` の解析器が「落とした件数」を返す形に変える必要があり、
  R28 と同じく実物の形を見てからのほうが安い（どの欄が欠けるのが正常かが分からないと、
  正常な項目まで「読めなかった」に数える）。`docs/handoff/ST13.md` に書いた。

## R33. 30 分刻みの地域で、壊れた `tz_id` を作る

- 成果物: `crates/server/src/archive/timezone.rs` の `from_offset`
- 根拠: `offset.unsigned_abs() / 60` の整数除算で分を捨てる。実測で `+05:30 -> Etc/GMT-5`。
  `offset_min` と `tz_id` が食い違ったまま記録に凍結される（記録は書き換えられないので後から直せない）。
- kind: technical
- 処置: fixed D5 — 正時のずれだけ `Etc/GMT±h` にし、それ以外は `UTC±HH:MM` にする。
  `archive_flow_half_hour_offsets_keep_their_minutes` が固定する。

## R34. 置き場の片方が読めないと、もう片方も走査されない

- 成果物: `crates/server/src/archive/scan.rs` の `list_dir` 2 本の `?`
- 根拠: 専用フォルダの `list_dir` が `Err` なら、ダウンロードのフォルダは走らない。
  既定は相対パスで、作る処理も無い。
- kind: technical
- 処置: deferred ST13 — **直していない。** 片方ずつ走らせる形にすると、
  「2 つの置き場をどちらも読めた走査の回数」（spec の生存信号の成功回数）の定義に触る。
  いまは `blockers` に読めない置き場が出るので**気づける**（R11 で直した）。`docs/handoff/ST13.md` に書いた。

## R35. `retire_legacy_sources` / `c03-myactivity-*` が利用者で絞られていない

- 成果物: `crates/server/src/archive/worker.rs` の `retire_legacy_sources` と `ensure_myactivity_source`、
  `crates/server/src/lib.rs` の `archives_status_for` の sources クエリ
- 根拠: `core.source` は `user_id` 列を持つが ST12 が入れる 11 行は全部 NULL。
  実測で、別の利用者が作った `c03-myactivity-*` が、まったく別の `user_id` の `/archives/status` に返った。
  表示名が `マイアクティビティ: <製品>` なので、他人がどの製品の履歴を置いたかが名前で漏れる。
- kind: technical
- 処置: deferred ST13 — **直していない。** 登録簿を利用者で分けるのは `core.source` を共有している
  **全 Story の前提**（`coverage.rs` の `must_sources()` も `testdb::source` も利用者を持たない）で、
  ST12 だけで変えると他の Story の判定が割れる。**いまの運用は単独の利用者**（`ASHIATO_ARCHIVE_USER_ID`
  が 1 つ）なので実害は出ていない。`docs/handoff/ST13.md` に書いた。

## R36. 移行の途中 `RAISE` が、psql 経路で半適用を残す

- 成果物: `migrations/202609181600_archive_ingestion.sql` の guard の位置
- 根拠: guard がトリガを作る文より**前**にあり、`tools/check-immutable.sh` の psql は
  `--single-transaction` を付けていない。実測で「台帳はできたが追記のみトリガ 0 本」が残った。
- kind: technical
- 処置: fixed D14 — guard を移行の**先頭**（表を作る前）へ移した。

## R37〜R46（中程度）

- 成果物 / 根拠は上記レビューの M1〜M10 のとおり
- kind: technical
- 処置: deferred ST13 — **直していない。** 内訳と理由:
  M1（`reparse_path` の鍵が一致しない・解析器の版上げの読み直しに実体が無い）/ M2（同じ JSON を 4〜5 回解く）/
  M3（`hash_file` が書庫全体をメモリに載せる）/ M4（sources クエリの直積と使われない索引）は、
  いずれも **D17（仮）と同じ「実データの規模が分かってから」の領域**。
  M5（`unreadable_at` / `skipped_file_count` / `first_seen_at` を読む経路が無い）/ M6（`already_read_ledger_id` に
  参照制約と利用者条件が無い）/ M7（`consecutive_failures` を戻す経路が無い）/ M8（`.json` の I/O 失敗を
  `unsupported_format` と呼ぶ）/ M9（guard のスキーマ条件）/ M10（新旧 DB で `shape` の DEFAULT が食い違う）は
  小さいが、**この PR で触った範囲の外**か、H.1 の後に形が変わる見込みのもの。
  まとめて `docs/handoff/ST13.md` に書いた。
