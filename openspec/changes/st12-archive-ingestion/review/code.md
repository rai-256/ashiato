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

---

# final review（7d08921..2a98724）— 2026-10-04

席: final reviewer（SDD の `code-reviewer.md`。ブランチ全体の review package を読む）。判定は **Ready to merge: No**。
入口 2 回目の申し送り（`docs/handoff/ST12.md` の st25-day-timeline R3）は、この段で design D2 / D6 の名前を実装（`c03-timeline-move` / `c03-timeline-route`）に揃えて閉じた（ST25 の design は実装の名前で書かれているので触らない）。

## R47. 画面の `/api/archives/status` が必ず 400 になり、箱はいつも「読み出せませんでした」になる
- 成果物: `crates/server/src/lib.rs:2297`（`ArchivesStatusQuery.user_id: uuid::Uuid` が必須）/ `web/src/App.tsx:88`（`user_id` を付けずに呼ぶ）
- 根拠: 他のエンドポイントは `Option<Uuid>` + `unwrap_or_default()`。ここだけ必須なので axum の `Query` が 400 を返す。smoke は `?user_id=` を付けて叩き、jsdom の試験は fetch を通らないので緑。tasks 11.4 の `curl …/archives/status | jq -e …` も `user_id` 無しでは落ちるはず。画面の Scenario に web/e2e の担保が無い
- kind: technical
- 処置: fixed 11.4 — `user_id` を `Option<Uuid>` + `unwrap_or_default()` に揃えた。`archive_flow_status_answers_without_a_user_id` と e2e `web/e2e/latest-archive.spec.ts` が固定する

## R48. 本物の `Timeline.json` では訪問・移動・生の信号が 1 件も入らない
- 成果物: `crates/server/src/archive/worker.rs:325-366` / `:523-539` / `:603-607`
- 根拠: `visit` / `activity` の中身を `event_time` に渡すが、実物の書き出しでは `startTime` / `endTime` / `startTimeTimezoneUtcOffsetMinutes` はセグメントの側にある。`rawSignals` は `{"position":{…,"timestamp"}}` / `{"wifiScan":{"deliveryTime"}}` / `{"activityRecord":{"timestamp"}}` と 1 段入れ子。試験の素材（`archive_tests.rs:607`、`archive_flow_tests.rs:846` など）は合成の形で、コードの思い込みと同じ
- kind: technical
- 処置: fixed D6 — セグメントから時刻と時差、`rawSignals` は入れ子の中の時刻。素材を実物の形に。`archive_requests_read_the_real_timeline_shape` / `archive_flow_a_bare_timeline_json_is_read`

## R49. `tools/archive-shape.sh` の一覧表示（引数なし）が必ず失敗する
- 成果物: `tools/archive-shape.sh:24`
- 根拠: `psql -c` の文字列では `:'user'` が展開されない。reviewer の実測で `ERROR: syntax error at or near ":"`。tasks 7b.2 の `archive-shape.sh | grep -c '京都'` も落ちるはず。付随: `--confirm` は確認待ちのファイル数だけ `archive_shape_confirmation` に行を足す（`DISTINCT` も一意制約も無い。追記のみ表）
- kind: technical
- 処置: fixed D16 — 一覧を here-doc に、`--confirm` は形ごとに 1 行。`tools/smoke.sh` 9b が一覧と印の行数を見る

## R50. 印を置いた後の読み直し（`ingest_confirmed_pending`）が 1 件の失敗で永久に止まる。削除済みの動画が検索のソースへ入る
- 成果物: `crates/server/src/archive/worker.rs:1069` / `:1084` / `:1114` / `:439`
- 根拠: `?` で関数ごと抜け、毎走査同じ順で同じ行から始まる。`watch-history.json` の `titleUrl` の無い項目が `c03-youtube-search` へ回り、`search-history.json` が先に書いた `archive_ledger_source` の PK `(ledger_id, logical_source)` に当たる
- kind: technical
- 処置: fixed D18 仮 — 読み直しを書庫ごとにし失敗は warn で次へ。URL の無い視聴は検索へ回さない。同じ論理ソースが印あり・確認待ちの両方から来たときのソース別台帳は足さずに warn（反転条件は D18）。`archive_flow_confirming_never_sticks_on_a_source_the_first_read_wrote`

## R51. 確認待ちの経路が写し・目録の失敗を黙って捨て、中身が同じ 2 冊目の確認待ちが永久に残る
- 成果物: `crates/server/src/archive/worker.rs:1349`（`if let Ok`）/ `let _ = record_copy` / `:1366` `let _ = record_pending_shape`
- 根拠: `archive_file` の PK `(user_id, sha256)` で 2 冊目の目録が `ON CONFLICT` で落ち、`confirmed_pending_copies` の JOIN に当たらない
- kind: technical
- 処置: fixed D18 — 失敗を捨てず、写しを中身のハッシュ（`archive_pending_shape.file_sha256`）で引く。`archive_flow_a_second_archive_with_the_same_file_is_read_after_confirming`

## R52. 確認待ちだけの書庫は走査のたびに全体を読み直される
- 成果物: `crates/server/src/archive/worker.rs:1515`
- 根拠: 置き場に残し、台帳は `pending_shape` だけなので `scan.rs` が毎回 `Read` で積む。120 秒ごとに zip 全体を 2 回展開する
- kind: technical
- 処置: fixed D14 — 既読判定に `pending_shape` を入れた。`archive_flow_a_pending_only_archive_is_not_read_again_on_every_scan`

## R53. 解析器の版を上げても専用のフォルダの書庫は読み直されない（tasks 7.4 は `[x]` だが本番から呼ばれていない）
- 成果物: `reparse_path` / `copied_files_for_reparse`（`crates/server/src/archive/`）
- 根拠: grep で本番からの呼び出し 0。R37〜R46 の M1 を deferred にしていたが、計画した機能の欠落
- kind: technical
- 処置: fixed 7.4 — `reparse_older_versions` を走査の周ごとに呼ぶ。`archive_flow_a_parser_version_bump_rereads_from_the_copies`

## R54. payload が design D6 と spec（地域を持たなかった印・`parser_version`・各欄）を満たしていない
- 成果物: `crates/server/src/archive/worker.rs:599`
- 根拠: payload は `archive_sha256` / `inner_path` の 2 欄。spec `:226` の印を payload で見る試験が無い（`archive_flow_tests.rs:1340` は名前と中身が別の Scenario）
- kind: technical
- 処置: fixed D6 — payload に D6 の全欄。`archive_payload_carries_the_design_d6_fields` / `archive_flow_stored_payload_marks_a_utc_only_time`

## R55. 原文の切り出しが YouTube とマイアクティビティにしか効いていない
- 成果物: `crates/server/src/archive/worker.rs`（Timeline / 移行前は `raw=None`、Chrome は `sliced = Vec::new()`）
- 根拠: spec `:224` の SHALL は全記録に掛かる
- kind: technical
- 処置: fixed D5 — 全種類の原文をバイト列から切り出す。`archive_raw_is_sliced_for_every_kind`

## R56. 本人の決定 Q2「残さない設定でも、確認待ちの写しは読み直し終えたら消す」の消す側が無い（R23 を ST23 送り）
- 成果物: `crates/server/src/archive/worker.rs`（`ingest_confirmed_pending` の後）/ spec `:352`
- 根拠: 消す経路が無い。tasks 7b.2「残さない設定では読み直しの後に写しが 0」が `[x]` のまま
- kind: technical
- 処置: fixed D9 — 残さない設定では、確認待ちのために作った写しを読み直し後に消す。`archive_flow_pending_copies_are_removed_after_rereading_when_copies_are_off`

## R57. MyActivity / YouTube の見分けが先頭 1 件だけを見る（R28）
- 成果物: `crates/server/src/archive/classify.rs:79`
- 根拠: `watch-history.json` の先頭が削除済みの動画だと、ファイルごと「読まなかった」になる。「先頭で `titleUrl` を持つ項目を探す」なら誤判定を新しく作らない
- kind: technical
- 処置: fixed D3 — `titleUrl` を持つ最初の項目で見分ける。`archive_classify_skips_items_without_a_title_url`

## R58. 印を置いた後の読み直しが書く `read` 行の `file_name` / `created_at` が NULL、`inbox_kind` が既定の `inbox`
- 成果物: `crates/server/src/archive/worker.rs:1094`
- 根拠: 読み直しの `read` 行は `record_read_ledger` に名前と時刻を渡さず、`inbox_kind` は列の既定（`inbox`）になる（final reviewer がコードを読んで確認）
- kind: technical
- 処置: fixed D7 — 確認待ちの台帳の作られた時刻と置き場の種類を引き継ぐ。`archive_flow_the_reread_ledger_row_keeps_the_archive_name_and_place`

## R59. 生存信号は 1 日の最初の 1 回だけなので、昼に置き場が読めなくなっても翌日まで箱に出ない
- 成果物: `record_archive_heartbeat`
- 根拠: 生存信号は日に 1 回しか書かれず、箱の「取り込み器」はその信号だけを見ていた（D10）
- kind: daily
- 処置: fixed D19 仮 — 箱の取り込み器は直近の走査（`archive_scan_counter.last_capturable`）を見る。`archive_flow_the_box_shows_an_unreadable_inbox_on_the_same_day`

## R60. `docs/archive-inbox.md` の `ASHIATO_ARCHIVE_USER_ID=<利用者 UUID>` は、nil 以外を入れると画面にも格子にも出ない
- 成果物: `docs/archive-inbox.md`
- 根拠: 画面と `tools/seed.sh` は nil UUID（`ASHIATO_USER_ID` の既定）を引くので、別の UUID で入れた記録はどこにも出ない
- kind: technical
- 処置: fixed 7.1 — `docs/archive-inbox.md` に `ASHIATO_USER_ID` と同じ値にする旨を書いた

## R61. `DuplicateOfDeleted` でも最終日が進む
- 成果物: `record_ledger_sources`
- 根拠: `DuplicateOfDeleted` の項目も `max_event_at` の計算に入っている（final reviewer がコードを読んで確認）
- kind: daily
- 処置: rejected: spec `external-ingestion/spec.md:458` と design D11（`design.md:250`）が「`max_event_at` は削除済みで入れなかった項目も含めて数える」と明示している。いまの振る舞いはそのとおりで、scoped re-review も裁定を支持した

## R62. `PgSink::store` が 1 件ごとに `App::new` と JSON の往復をする
- 成果物: `crates/server/src/archive/`（`PgSink::store`）
- 根拠: 1 件ごとに `App::new` を作り、要求を JSON に直列化して解き直していた（百万件級の書庫で無駄）
- kind: technical
- 処置: fixed D4 — `PgSink` が App を 1 度だけ持ち `ingest_request` を直に呼ぶ。既存の格納の試験が覆う

## R63. R37〜R46 の申し送り先が ST13（アカウント系の定期取得）で、ST12 の不具合を受け取る Story ではない
- 成果物: 上の R37〜R46 の処置 / `docs/handoff/ST13.md`
- 根拠: ST13 はアカウント系の定期取得で、ST12 の取り込み器の不具合を受け取る Story ではない（final reviewer）
- kind: defer
- 処置: rejected: 申し送りの器は Story 単位しか無く、ST13 はまだ `tasks.md` を持たないので `deferred ST13` が規則どおり（上流が深掘りの前に読む）。M1 は R53 で閉じたので `docs/handoff/ST13.md` から外した

## scoped re-review（8be2abf..dd6cda8）: R47〜R62 すべて ADDRESSED（R61 は裁定を支持）。新しい Critical / Important なし

re-review が挙げた Minor 3 件は、2 回目の fix wave を出さない規則（SDD の Final Review）に従い、ledger（`.superpowers/sdd/st12-task-final/progress.md`）に ruling つきで park した。

---

# code-verify 第 3 回（`7d08921..7b95619`）— 2026-10-05

対象: `feat/st12-archive-ingestion` の HEAD `7b95619`（PR #10 の head はまだ `2a98724` で、push 前）。**実装は触っていない。**
変異試験は作業ツリーの外の複製（`git archive HEAD`）で行い、元に戻してから次へ進んだ。

> 作業の副作用（報告）: (1) 画面の実寸の probe を複製で回したとき、複製の `web/node_modules` を worktree へのシンボリックリンクにしていたので、
> `tools/stack.sh` の `npm ci` が **worktree の `web/node_modules` を空にした**。`cd web && npm ci` で戻し、`tsc -b` / lint / build / vitest 195 本が緑に戻ったことを確かめた。
> (2) 変異試験が試験用 DB の登録簿に `c03-myactivity-u772c89a697d3`（30 日）と `c03-myactivity-u8904d416c8a9`（`record`）を残したので、`none` / 60 日へ戻した。
> (3) `tools/check-immutable.sh` と `tools/smoke.sh` は DB を `down -v` で作り直す（試験用 DB の中身は消える）。

## 申告: 63/63 件を処置済み・tasks の `[x]` は 43/44（13.1 だけ未了）・CI 緑。独立に実行した検証コマンド

| # | 申告 | 実行したもの |
|---|---|---|
| 1 | `cargo test` が全部緑 | `set -a; . ./.env; set +a; cargo test --workspace`（4 回） |
| 2 | 全 Scenario に印がある | `python3 scripts/check_scenarios.py . st12-archive-ingestion` |
| 3 | tasks の `[x]` の検証が通る | 0.1〜12.5 の検証コマンドを本文のまま 1 つずつ（下の表） |
| 4 | 画面の Scenario を担保している | `cd web && npm run test:e2e`（本物の Chromium）＋ 360×640 で箱と格子の実寸を測る probe（`page.route` で状態を差し替え） |
| 5 | 決定値を試験が固定している | 定数を 1 つずつ書き換えて `cargo test -p ashiato-server archive` / `npx vitest run` |
| 6 | 守りの検査が効く | 複製で `archive_file` のトリガを外して `tools/check-immutable.sh`、spec に 1 行混ぜて `tools/st12_delta_diff.py` |
| 7 | 置き場と写しの既定 | release のサーバを別々の作業ディレクトリから起動し、目録の `stored_path` と、印を置いた後の読み直しを観測 |

## 実測（一致 / 不一致）

| 申告 | 実測 | 判定 |
|---|---|---|
| cargo test 緑 | 1 回目は `505 passed; 1 failed`（`drops_tests::drops_api_idempotent` が `40P01 deadlock detected`）、2〜4 回目は `506 passed` | おおむね一致（ST12 の外の試験が 4 回に 1 回不安定。原因の表は特定していない） |
| check_scenarios rc=0 | `Scenario 691 / 印 836 / 担保あり 691`・rc=0（spec に無い名前を指す印の warn が 6 件） | 一致（中身は R64 / R65） |
| vitest 緑 | `27 files / 195 passed` | 一致 |
| e2e 緑 | `22 passed`（ST12 のものは `latest-archive.spec.ts` の 2 本だけ） | 一致（ただし R65） |
| fmt / clippy / tsc / lint / build / check-boundaries / check-openapi / check-migrations / check-licenses / check-private / check_chain / openspec validate | いずれも rc=0 | 一致 |
| 0.1 `st12_delta_diff.py` | rc=0。spec に 1 行混ぜた複製では rc=1（`[FAIL] 前書きに許していない差`） | 一致（検査は本物） |
| 1.2 `check-immutable.sh` | rc=0。`archive_file` の `archive_append_only` を外した複製では rc=1（`NG archive_file の全列を書き換えられた` ほか 2 行） | 一致（検査は本物） |
| 9.1 openapi | 生成物が `docs/openapi.json` と一致・`"/archives/status"` が 1 件 | 一致 |
| 11.3 `tools/smoke.sh` | rc=0（9b を通る。ただし `psql` を docker に差し替えて通している。R68） | 一致 |
| 11.4 seed → curl | **本文のまま（`127.0.0.1:18787`）だと rc=4**。この worktree の `.env` は `BIND=127.0.0.1:18797` で、そこへ叩くと `latest_archive.outcome=="read"` が rc=0 | 環境差（R70） |
| 12.4 PR 本文 | rc=0 | 一致 |
| **12.5 handoff** | **rc=1**（PR 本文に `R6` が無い。R66） | **不一致** |
| 箱は 160 px・溢れても読めなかった行を省かない | jsdom は緑。**本物の Chromium の 360×640 では外寸 178 px で、「ほか N 件」と読めなかった書庫の行が箱の外にはみ出して見えない**（R64） | **不一致** |
| 箱が上限のとき 2 ソースは 800 以内 / 5 ソースは 1,440 以内 | jsdom は緑。**本物の Chromium では 894 / 1,457**（R65） | **不一致** |
| 決定値の固定 | 失敗 3 回 / 先頭 100 件 / 120 秒 / 写しの既定 `true` / 3 日 / 160 px / myactivity の粒度は、書き換えると落ちる試験がある。**myactivity の 60 日・日本語名のハッシュ・1 時間・1,000 件ごとは書き換えても全部緑**（R69） | 一部不一致 |

---

## R64. 本物のブラウザでは箱の外寸が 178 px になり、溢れたとき「ほか N 件」と「読めなかった書庫」の行が箱の外にはみ出して見えない
- 成果物: `web/src/LatestArchive.tsx:17-36`（`BOX_MAX_ROWS` は 1 行 = 22.4 px として数える）/ `:167-170`（`maxHeight: 160`・`overflow: "hidden"`。`box-sizing` は既定の `content-box`）/ `web/src/__tests__/latest-archive.test.tsx`（`declaredHeight` だけで測る）
- 根拠: 複製で `tools/stack.sh` を立て、playwright を 360×640 で開いた。`/archives/status` は「読んでいる途中・両方の置き場が読めない・最後の確認が 4 日前・確認待ち 2 冊・直近の書庫」に差し替えて測った（rc=0）:
  ```
  BOX {"h":178,"boxSizing":"content-box","maxH":"160px","pad":"8px","scrollH":195}
  ROW archive-row-reading top=31 bottom=99 見える=true   ← 長いファイル名で 3 行に折り返す
  ROW archive-row-inbox-unreadable top=99 bottom=143 見える=true
  ROW archive-row-inbox-stale top=143 bottom=166 見える=true
  ROW archive-row-more top=166 bottom=188 見える=false  ← 「ほか 2 件」
  ```
  直近の書庫を `unreadable`（`broken_zip`）にすると `archive-row-latest top=143 bottom=211 見える=false`・`scrollH=240`。
  `maxHeight: 160` は中身だけの高さなので、上下の余白 8 px と枠 1 px が足されて外寸は 178 px になる。行数は折り返しを考えずに数えているので、中身の行が溢れて `overflow: hidden` で切られる。
  jsdom は文字を折り返さず、`style` に書かれた値を足すだけなので、どの試験も緑になる。
- 影響: 本人が開く幅で、spec `external-ingestion:664`「箱の高さは 160 CSS px 以下で、出しきれない文字があれば省いたことが示される」と `:654`「箱が溢れても読めなかった書庫は省かれない」が両方とも成り立たない。文字は DOM には残るので、jsdom の `textContent` の試験は通る。
  省かない行（`pinned`）は「置いたのに入っていないことに気づけないまま、書庫が約 7 日で失効する」のを防ぐためにある。見えなければその役目を果たさない。
- kind: technical
- 提案: `boxSizing: "border-box"` にし、各行に `whiteSpace: nowrap` と `textOverflow: ellipsis` を付けて 1 行に収める（または `pinned` の行を先頭に置く）。
  `web/e2e/latest-archive.spec.ts` に試験を足す: 360×640 で状態を `page.route` で差し替え、`boundingBox().height <= 160` であることと、`pinned` の行と「ほか N 件」の行が箱の中にあることを測る。
- 処置: fixed D12 — 箱を `box-sizing: border-box` にし、見出しと各行を 1 行（`nowrap` + `text-overflow: ellipsis`。全文は `title`）にした。本物の Chromium の 360×640 で、上限まで埋めた箱の外寸 130 px・「ほか N 件」と読めなかった書庫の行が箱の中（`web/e2e/archive-layout.spec.ts` の「箱は 160 px を超えない」「箱が溢れても読めなかった書庫は省かれない」）

## R65. 実寸を主張する画面の Scenario 18 本が、jsdom の宣言値だけで担保されている。本物のブラウザでは 800 / 1,440 px の予算を超える
- 成果物: `web/src/__tests__/archive-one-scroll.test.tsx` / `latest-archive.test.tsx` / `archive-heading.test.tsx` / `archive-order.test.tsx` / `web/e2e/latest-archive.spec.ts`（2 本だけ）/ `AGENTS.md`「画面の Scenario は `web/e2e`（本物のブラウザ）で担保する。jsdom へも〜逃がさない」/ `docs/testing.md:70-73`（実寸・スクロール量・横溢れは `web/e2e` が測る。2026-09-18 から）
- 根拠: 画面の Scenario 20 本の印の置き場を grep で数えた。`web/e2e` にあるのは `直近に置いた書庫の結果が箱に出る` と `直近に置いた書庫の箱は Must の前にある` の 2 本だけで、残りの 18 本（`箱は 160 px を超えない` / `書庫のソースの格子は 360 px に収まる` / `書庫のソースの週の帯は 24 px 以上` / `箱が上限の高さのとき 2 ソースが 800 px に収まる` / `箱が上限の高さのとき 5 ソースが 1,440 px に収まる` / `箱の高さを除く量は 160 px を超えない` ほか）は jsdom にしか無い。
  R64 と同じ probe で、`SEED=normal`・箱を溢れさせた状態の格子の位置を測った:
  ```
  SECTION#achievement top=119 h=253 / SECTION#latest-archive top=385 h=178
  GRID grid-c01-location        weeks=4 top=610  bottom=706
  GRID grid-c01-app-usage       weeks=4 top=798  bottom=894   ← 2 本目。spec は 800 以内
  GRID grid-c02-browser-history weeks=4 top=1361 bottom=1457  ← 5 本目。spec は 1,440 以内
  ```
  横溢れは無かった（`document.documentElement.scrollWidth = 360`）。箱（178）と余白（12）を除いても 2 本目の下端は約 704 px で、ST02 の 640 px も超えると推定できる（達成の欄が実寸で 253 px ある）。
  **予算超えの一部は ST12 より前からある見込み。ただし 800 / 1,440 は ST12 が MODIFIED で書いた THEN で、それを確かめた試験は本物の寸法を一度も測っていない。**
- kind: conflict
- 提案: 実寸の Scenario を `web/e2e` に移す（`setViewportSize(360, 640)` にし、`page.route` で箱を上限の高さにして、`boundingBox` で 800 / 1,440 / 24 px を測る）。jsdom の試験は「指定と勘定」の確認として残す。
  予算を実際に超えているなら、まず e2e の数字で原因を分ける —— 達成の欄の高さが前提とずれているのか（ST02 の前提）、ST12 の箱が足した分なのか。そのうえで `collection-coverage` の予算の文を直すかを決める。
- 処置: fixed D20 仮 — 実寸の Scenario を `web/e2e/archive-layout.spec.ts` で測る（160 px・押し下げ 160 px 以下・5 本目 1,440 px 以内・360 px・24 px）。R64 の後の実測で 5 本目は 1,409 px で収まり、**2 本目は 846 px で 800 を超える** —— 箱を除いた土台が約 703 px で、ST02 の 640 px が箱の無い画面で既に超えている（達成の欄 253 px）。800 の Scenario は `test.fail` で「いまは成り立たない」を固定し、反転条件を D20 に書いた。PR 本文の冒頭に出す

## R66. ST22 からの申し送り R6「消した場面の位置が、書庫の別のソースから生きた記録として入る」が、この change のどこにも無い。tasks 12.5 は rc=1 のまま `[x]`
- 成果物: `docs/handoff/ST12.md:22-28` / `openspec/changes/st12-archive-ingestion/{deep,design,tasks}.md`（`ST22` / `st22` は 0 件）/ `tasks.md:214`（12.5）/ `crates/server/src/archive/worker.rs`（削除の時間帯を見ていない）
- 根拠: `tasks.md` 12.5 の検証を本文のまま実行すると **rc=1**（`PR 本文に無い: R6`）。`grep -n 'st22\|ST22' openspec/changes/st12-archive-ingestion/*.md` の結果も PR 本文の中も 0 件。
  ST22 は 2026-10-04 に archive 済み。`openspec/changes/archive/2026-10-04-st22-record-deletion/deep.md:46` は「書庫から別のソースで入る同じ場面（ST12）は隠れない…後者は `docs/handoff/ST12.md` へ」と書いている。
  ST22 の削除は `c01-location` にしか掛からない（`stay.rs:31` の `sources`）。`archive/` には `stay_erased` や削除の時間帯を見るコードが無い（grep で 0 件）。
  一方 ST12 は `c03-timeline-visit` / `route` / `signal` と `c03-legacy-location` に座標を入れる。つまり、本人が ST22 で滞在とその時間の位置を消した後に `Timeline.json` を置くと、**消した場面の位置が別の論理ソースから生きた記録として読み出しに出る**。
  加えて、12.5 の検査は `grep -oE 'R[0-9]+'` の部分一致で見ている。申し送りの `R3` / `R4` は、PR 本文の `R37〜R46` や `R25` に当たって通っている（R3 / R4 は外部識別子を持つソースの話で、ST12 は全ソースを `none` にしたので実害は無い。ただし読んだ跡も無い）。
- 影響: loss: exported —— 本人が消したつもりの場所が、書庫を置くたびに戻り、読み出しと書き出しへ流れる。滞在を消した画面からは本人は気づけない。
- kind: irreversible
- 提案: `deep.md` に A の問いとして立てる（「消した時間帯に書庫から入る位置を、削除済みとして入れるか」。選択肢は ST22 第 2 回 Q2 / 第 3 回 Q6 と同じ）。
  12.5 の検査は、R 番号を単語の境界つき（`\bR6\b` のように）で、さらに申し送りの見出し（`st22-record-deletion R6`）ごとに突き合わせる形に直す。
- 処置: escalated — `deep.md` 第 4 回 Q13（loss: exported）。問いは `deep-questions-r4.json` / `docs/briefs/ST12-deep-r4.html`。あわせて tasks 12.5 の検査を、申し送りの見出し「`<change> R<n>`」ごとの突き合わせに直した（R 番号の部分一致をやめた）。PR 本文に st22-record-deletion R3 / R4 / R6 と st25-day-timeline R3 の扱いを書く。**答え（2026-10-05）**: 推奨のまま「印を付けて入れる」→ spec の Requirement「本人が滞在を消した時間帯の書庫の位置は、削除済みの印を付けて入る」（Scenario 5 本）・design D22・tasks Task 15（14.1〜14.3）。実装はグラフの Task ループが回す

## R67. 置き場と写しの既定がサーバの作業ディレクトリからの相対パスで、目録にも相対パスのまま残る。別の場所から起動すると、印を置いた後の読み直しが永久に止まる
- 成果物: `crates/server/src/archive/config.rs:36-44`（`Documents/ashiato/取り込み待ち` / `Downloads` / `AppData/Local/ashiato/archive-copies`）/ `worker.rs:1400`（`std::fs::read(stored_path)` が失敗すると `continue`）/ design D1 の表（既定は `%USERPROFILE%\…` / `%LOCALAPPDATA%\…`）
- 根拠: release のサーバを、`ASHIATO_INBOX_DIR` / `ASHIATO_DOWNLOADS_DIR` だけ絶対パスで渡し、`ASHIATO_ARCHIVE_COPY_DIR` は既定のままにして起動した。
  - 置き場も既定のままだと、起動のログに `書庫の置き場を読めない: Documents/ashiato/取り込み待ち` が出る（`/tmp` から起動した場合）
  - `run1/` から起動して視聴履歴の Takeout を置いた → 目録の `stored_path` は `AppData/Local/ashiato/archive-copies/da/dad79a…`（相対）、台帳は `pending_shape`、書庫は `取り込み済み` へ移った
  - `tools/archive-shape.sh --confirm` で印を置き、**`run2/` から起動した** → `events 0 / pending 1`、ログに `書庫の写しを読めない` が 8 回
  - 同じ DB のまま **`run1/` から起動し直した** → `events 1 / pending 0`
- 影響: Windows のサービスやタスクスケジューラから起動すると、作業ディレクトリは `C:\Windows\System32` などになる。写しはそこに作られ、次に別の場所から起動したときには読めない。
  確認待ちの書庫はもう `取り込み済み` へ移っているので走査でも拾われず、本人が `--confirm` を叩いても何も起きない（ログに警告が出るだけで、画面の箱は「確認待ち」のまま）。
  `docs/archive-inbox.md` は 3 つとも設定するよう書いているので、手順どおりなら起きない。ただし design の既定とは違う。
- kind: technical
- 提案: 既定を design D1 のとおり `%USERPROFILE%` / `%LOCALAPPDATA%`（Linux なら `$HOME`）から作るか、相対パスなら起動を止める（`KEEP_COPIES` の綴り違いと同じ扱い）。目録には `canonicalize` した絶対パスを書く。
- 処置: fixed D1 — 既定を `%USERPROFILE%`（無ければ `$HOME`）・`%LOCALAPPDATA%`（無ければ `<ホーム>/AppData/Local`）から作り、相対パスとホームの無い環境は起動を止める（写しの置き場が絶対なので目録の `stored_path` も絶対になる）。`archive_config_places_are_absolute_from_the_home`、`docs/archive-inbox.md` に既定と絶対パスを書いた

## R68. `tools/archive-shape.sh` はホストの `psql` を前提にしているが、smoke は docker の `psql` に差し替えて緑になっている。この機械では `psql: command not found` で止まる
- 成果物: `tools/archive-shape.sh:15` / `:30` / `tools/smoke.sh:23-28`（`psql()` を `docker compose exec -T db psql` に差し替えて `export -f`）/ `docs/archive-inbox.md`（`psql` の記載は 0 件）/ tasks 13.1（人間が `archive-shape.sh` で印を置く）
- 根拠: `ASHIATO_ARCHIVE_USER_ID=… tools/archive-shape.sh --confirm <hash>` → `tools/archive-shape.sh: line 15: psql: command not found`。`command -v psql` も見つからない。smoke と同じ関数を定義すると通る。
- 影響: 本人の決定（第 2 回 Q10）により、Takeout の中身は印を置くまで 1 件も入らない。印を置く道具が本人の機械で動かなければ、最初の Takeout から先へ進めない。smoke は Fake で素通りしているので、どの検査もこれに気づかない（ST01 の「Bearer の検査が Fake で素通り」と同じ型）。
- kind: daily
- 提案: `psql` が無ければ `docker compose exec -T db psql` に回すよう、smoke の差し替えを `archive-shape.sh` 自身へ移す。あるいは `docs/archive-inbox.md` に前提として書き、`psql` が無ければ止まる 1 行を道具の先頭に置く。
- 処置: fixed D21 仮 — `tools/archive-shape.sh` 自身が、`psql` が無ければ `docker compose exec -T db psql` に回す。`tools/smoke.sh` の `psql` の差し替えを外し、道具をそのまま呼ぶ。`docs/archive-inbox.md` に書いた

## R69. マイアクティビティの製品ソースについて、凍結される名前（日本語名のハッシュ）と登録の値（60 日）をどの試験も固定していない
- 成果物: `crates/server/src/archive/myactivity.rs:16-19` / `worker.rs:873`（`expected_gap_sec = 5184000`）/ `archive_tests.rs:1092-1094`（`starts_with` と長さしか見ない）/ `archive_flow_tests.rs:1519`（60 日の試験は `NOT LIKE 'c03-myactivity-%'` で除外している）
- 根拠: 複製で 1 つずつ書き換えて、`cargo test -p ashiato-server archive` を走らせた:

  | 書き換え | 結果 |
  |---|---|
  | ハッシュの入力を `format!("v2:{product}")` に | `106 passed; 0 failed` |
  | myactivity の登録の `5184000` → `2592000`（30 日） | `106 passed; 0 failed`（試験用 DB に 30 日の行が残った） |
  | 同じく粒度 `'none'` → `'record'` | 1 本落ちる（`archive_flow_myactivity_display_name_uses_the_product`） |
  | 参考: 失敗 3 回・先頭 100 件・走査 120 秒・写しの既定 `true` | どれも 1 本以上落ちる |
  | 参考: 失敗後の `interval '1 hour'` → 10 分、`READING_STEP` 1,000 → 500 | `106 passed`（どちらも design の仮の値で、本人の決定ではない） |

  期待値は独立に計算した（python の `hashlib.sha256`）: `マップ` → `c03-myactivity-u097022418c48`、`検索` → `c03-myactivity-u1b6b1a6f8931`。Chrome の epoch `11644473600000000` は python の `datetime` で計算した値と一致した。
- 影響: 名前は登録簿と記録に凍結される（loss: rewrite-all の側）。ハッシュの作り方が変わると、次の書庫から同じ製品が別の論理ソースに割れ、格子も最終日も 2 本になる。60 日（C13 / design D2）を変えても気づけない。
- kind: technical
- 提案: 独立に計算した値で `source_name("マップ") == "c03-myactivity-u097022418c48"` を固定する。60 日の試験に、取り込み器が足した `c03-myactivity-*` の行も含める（試験の中で 1 本足してから見る）。
- 処置: fixed D2 — `source_name("マップ") == "c03-myactivity-u097022418c48"` / `source_name("検索") == "c03-myactivity-u1b6b1a6f8931"`（python で独立に計算）を固定し、`archive_myactivity_source_is_registered_with_sixty_days`（毎回知らない製品で足して 60 日・`none` を見る）を足した。変異で確かめた: ハッシュの入力を `v2:` 付きにすると前者が、`5184000` → `2592000` にすると後者が落ちる

## R70. tasks 11.4 の検証はポート `18787` を直書きしていて、worktree ごとの `BIND`（ここでは `18797`）では rc=4 になる
- 成果物: `tasks.md:201-202` / `.env`（`BIND=127.0.0.1:18797`）/ `tools/stack.sh`（`BIND` の既定は 18787 だが、`.env` が上書きする）
- 根拠: `tools/stack.sh up` の後、本文のままの `curl … http://127.0.0.1:18787/archives/status | jq -e …` は **rc=4**（繋がらない）。`http://$BIND/archives/status` に叩けば rc=0（`{"o":"read","s":10,"inbox":null}`）。
- kind: daily
- 提案: 検証を `http://${BIND:-127.0.0.1:18787}/…` にする。いまは、`verify-run` がこの worktree で走らせると落ちる形のまま `[x]` になっている。
- 処置: fixed D21 仮 — 11.4 の検証を `"http://${BIND:-127.0.0.1:18787}/archives/status"` にした

## R71. tasks 4.4（大きなファイルをメモリに載せない）は、本番から呼ばれない `stream_array_items` を測って `[x]` になっている
- 成果物: `tasks.md:91` / `crates/server/src/archive/slice.rs:13`（`stream_array_items`）/ `worker.rs`（使っているのは `array_elements` / `object_member` で、どちらもファイル全体を受け取る）/ design D17（仮）
- 根拠: `grep -n 'slice::' crates/server/src/archive/worker.rs` の結果は `object_member` / `array_elements` だけ。`stream_array_items` を呼んでいるのは `archive_tests.rs:561` だけ。
  本番は R19 で指摘したとおり書庫を全部展開し、ファイル 1 本を丸ごと `Value` にする（design D17（仮）が「いまの形で進める」と記録している）。
- kind: technical
- 提案: D17 の決定に合わせて、4.4 の本文を「本番の経路ではまだ守っていない（D17 仮）」と書き直す（`[x]` が D5 を守った証跡として読まれないように）。D17 の反転条件が来たら、`stream_array_items` を本番の経路に入れて同じ試験で測る。
- 処置: fixed D17 — D17 と tasks 4.4 の本文に「本番の読み手はまだ `stream_array_items` を通らない。4.4 は関数の性質の証跡で、本番が D5 を守った証跡ではない」と書いた（切り替えは D17 の反転条件のまま）

---

## 手ごとの結果

- **手 1（固定値を独立に再計算する）**: Chrome の epoch は python で計算した値と一致した。マイアクティビティの日本語名は**期待値が試験に無い**（長さしか見ない）ので、python で計算した値を R69 に挙げた。content_hash の固定値は ST03 の試験が python の式つきで持っている（ST12 は触っていない）。
- **手 2（守りをわざと壊す）**: `check-immutable.sh`（`archive_file` のトリガを外すと rc=1）・`st12_delta_diff.py`（1 行混ぜると rc=1）・`KEEP_COPIES=flase` で起動が止まる（rc=1）の 3 つは本物だった。**検査の外側**に素通りが 3 つある: `archive-shape.sh` の `psql` は smoke の差し替えで素通り（R68）、画面の実寸は jsdom の宣言値で素通り（R64 / R65）、12.5 の handoff の検査は R 番号の部分一致で素通り（R66）。`check-licenses.sh` は `--all-features` で `zip` の依存（bzip2 / zstd / lzma 系）まで見ていて、不許可は 0 件。
- **手 3（Scenario と試験を突き合わせる）**: rc=0。サーバ側は 2 本を読んで、主張と試験の階層が合っていることを確かめた（`原文は書庫のバイト列の一部と一致する` は `core.event.raw`（`text`）を取り出し、書庫の本文の部分列かを見ている / `書庫の位置と端末の位置が同じでも取りやめない` は、同じ点を本物の格納関門で先に入れている）。**画面の 18 本は、主張が実寸なのに試験は宣言値**（R64 / R65）。
- **手 4（本人の決定を試験が固定しているか）**: 書き換えて落ちることを実行で確かめたのは Q2（写しの既定 `true`）/ C22（3 日）/ D12（160 px）/ D1（120 秒）/ D7（3 回・先頭 100 件）。
  Q6 / Q7（固定 10 本の `none` と 60 日）と C19（取り込み器の 1 日）は、`archive_flow_every_archive_source_is_registered_with_sixty_days` / `archive_migration_registers_sources_and_preserves_interval` が値を assert しているのをコードを読んで確かめた
  （移行の値を書き換える変異は、使い回している試験用 DB では `ON CONFLICT DO NOTHING` に隠れるので実行していない）。
  **書き換えても落ちないのは myactivity の 60 日と名前のハッシュ**（R69）と、design の仮の値（1 時間・1,000 件ごと）。
- **手 5（tasks の `[x]` と実体）**: `CT` の絞り込み 35 種は、どれも 1 本以上の試験に一致した（`-- --list` で数えた）。`VT` の 5 ファイルは実在して緑。**12.5 は rc=1**（R66）、**11.4 は本文のままだと rc=4**（R70）、4.4 は本番で使われない関数を測っている（R71）。
- **手 6（隙間）**: 「捨てたもの・外へ出たものは戻らない」型は 2 件: R66（消した場面が書庫から戻る。loss: exported）と R67（起動場所によって写しを見失い、確認待ちが永久に進まない）。
  記録 → 台帳 → ソース別台帳の途中で落ちると最終日が戻らない件は、R25 として既に deferred（ST13）なので重ねていない。

## scoped re-review（7b95619..4ae2c48）: R64〜R71 すべて ADDRESSED（R66 は正しく escalated）。新しい Critical / Important なし

re-review が挙げた Minor 3 件（取り込み器を起こさないときの相対パスでも起動を止める / 800 px の `test.fail` にも印が付く / docker の `psql` の `-q`）は、
2 回目の fix wave を出さない規則に従い、ledger（`.superpowers/sdd/st12-task-final/progress.md`）に ruling つきで park した。PR 本文に写した。


# final review 第 2 回（7d08921..f29dfea。7b95619..f29dfea を厚く）— 2026-10-05

席: final reviewer（SDD の `code-reviewer.md`。ブランチ全体の review package）。判定は **Ready to merge: With fixes**（Critical 0 / Important 2 / Minor 5）。
入口 2 回目の申し送り: `docs/handoff/ST12.md` は前回の final review（2026-10-04）以降に増えていない（最終更新 2026-10-01）。
Task 15 の ⚠️（`archive_flow_confirming_a_shape_ingests_a_takeout_archive` が証跡 :598 で 1 度落ちた件）: 試験の待ち方の競合で 75d6294 が直した。reviewer が f29dfea で `archive_` 5 回・server 全体 3 回走らせ全て緑。
parked 6 件（`.superpowers/sdd/st12-task-final/progress.md`）: どれも merge を止めない（park のまま）。

## R72. 印付けに失敗した書庫を置き場に残しても、次の走査は読み直さない（Task 15 F1 の ADDRESSED は実態と違う）
- 成果物: `crates/server/src/archive/worker.rs:2092-2137`（spawn_inspecting）/ `:1463-1498`（reread_archive）/ `crates/server/src/archive/scan.rs:112`
- 根拠: `read` の台帳の行を INSERT した**後**に `mark_archive_arrivals` を呼び、失敗で `return` する。`read` 行は残るので、次の走査で `scan.rs` の既読の判定（`outcome IN ('read','unreadable','pending_shape')`）が `AlreadyRead` を返し、`worker.rs:1783-1793` は印を付け直さずに畳む。`reparse_older_versions` も新しい版の `read` 行が残るので次の周の対象から外れる。コメントと 8588e7f の「次の走査で読み直して付け直す」は成り立たない
- 影響: 印付けが一時的な DB エラーで落ちると、消した時間帯の書庫の位置が生きた記録のまま残る（Q13 の loss: exported の経路）
- kind: technical
- 処置: fixed D22 — 印付けを `read` 台帳の INSERT の前へ（spawn_inspecting / reread_archive）。失敗すれば台帳が無いまま次の走査・次の周で読み直される。`archive_erased_window_marks_after_a_failed_marking_on_the_next_scan` / `archive_erased_window_reparse_marks_after_a_failed_marking`（直す前に 2 本とも落ちるのを確認）
- 提案: `mark_archive_arrivals` を `read` 行の INSERT の前へ（格納は commit 済み・印付けは冪等）。reread_archive も同じ順に。印付けの失敗を差し込み、次の走査で印が付く試験を足す

## R73. 消すときの連鎖と書庫の印付けが、利用者の位置の全期間を助言ロックを握ったまま走査する（Task 15 F2 の格上げ）
- 成果物: `crates/server/src/deletion.rs:284-289` / `:300-313` / `crates/server/src/stay_store.rs`（`mark_archive_arrivals`）
- 根拠: `event_time >= $3` を外し `event_time <= $4 AND {event_end} >= $3` にしたため、索引 `event_by_source_time (logical_source, event_time)` が上側しか絞れない。基準の `c01-location`（年 50 万行）も含めて全履歴の payload を読む。基準のソースは `end_time` を持たないので意味は前と同じで索引だけを失っている。`mark_archive_arrivals` も書庫 1 冊ごとに書庫の位置の全期間 × 消した滞在を評価する
- kind: technical
- 処置: fixed D22 — 基準・点のソースは `event_time` の上下限、区間 4 本（`archive::INTERVAL_SOURCES`）だけ終わりで重なりを見る（`stay_store::overlaps_erased_sql`）。`mark_archive_arrivals` はその書庫が入れた位置の時刻の範囲だけ。区間のソースに下限が無いことは D22 に明記。`stay_erase_overlap_bounds_the_base_source_by_index`（EXPLAIN で Index Cond に上下限）
- 提案: 下限 `event_time >= $3 - <区間の最長>` を足すか、基準のソースは `BETWEEN` のまま区間を持つ書庫のソースにだけ終わりの重なりを見る。`mark_archive_arrivals` はその書庫が入れた時刻の範囲に絞る

## R74. `tools/archive-shape.sh` の psql が無いときの回り道が `DATABASE_URL` を見ない
- 成果物: `tools/archive-shape.sh:12-19`
- 根拠: `docker compose exec db` の DB へ書き、`DATABASE_URL` が別の DB を指していても黙って成功する
- kind: technical
- 処置: fixed D16 — 回り道で `DATABASE_URL` の利用者・DB 名を `-U` / `-d` に渡し、回り道と接続先を stderr に出す（合言葉は出さない）。`tools/smoke.sh` 緑
- 提案: 回り道に入ったことと接続先を出力に示す（または `DATABASE_URL` の DB 名・利用者を渡す）

## R75. `archive_erased_cascade_marks_locations_stored_before_the_erase` のコメント「置き直しても行は増えない」を確かめるコードが無い（Task 15 F4）
- 成果物: `crates/server/src/archive_flow_tests.rs`
- 根拠: `git show f29dfea:crates/server/src/archive_flow_tests.rs` の :2574 にコメント「同じ内容の書庫を置き直しても行は増えない（内容の鍵）」があり、続く assert は `Criteria::default_values().sources` だけ。書庫を置き直す `inbox.put` も行数の比較も無い
- kind: technical
- 処置: fixed D22 — 同じ Timeline.json を別の書庫として実際に置き直し、位置の行数と連鎖の台帳の行数が変わらないことを見る（`archive_flow_tests.rs:2587-2613`）
- 提案: 置き直しを実際に行って行数を見るか、コメントを消す

## R76. `event_end_sql` のミリ秒の分岐と、移行前の区間（`c03-legacy-visit` / `-activity` の `endTimestampMs`）を通る試験が無い（Task 15 F5）
- 成果物: `crates/server/src/deletion.rs`（`event_end_sql`）
- 根拠: `f29dfea` の `stay_store::event_end_sql`（:633）は `end_time` が数字だけならミリ秒として `to_timestamp(…/1000.0)` に回す分岐を持つが、`archive_flow_tests.rs` に `endTimestampMs` は 0 件（`grep -c`）。移行前の区間が消した滞在に重なる場面を通る試験が無い
- kind: technical
- 処置: fixed D22 — `archive_erased_legacy_millisecond_spans_follow_the_erased_stay`。ミリ秒の分岐を壊すと落ちることを確認
- 提案: 移行前の区間を持つ位置が消した滞在に重なれば印が付く試験を足す

## R77. `archive_erased_window_marks_the_overlapping_archive_locations` の最後の `stays()==1` は作り直しが走っていても通る（Task 15 F3）
- 成果物: `crates/server/src/archive_flow_tests.rs`
- 根拠: `f29dfea` の `archive_flow_tests.rs:2520` は `assert_eq!(inbox.stays().await, 1, "滞在の作り直しが走っている")` だけ。作り直しは同じ区間の滞在を 1 件に作り直すので、走っても件数は 1 のままで通る
- kind: technical
- 処置: fixed D22 — `stay_criteria` の行と `rebuild:` の印が 0、消した滞在の印が `user` のままであることも見る
- 提案: 作り直しの印か台帳の行が無いことを見る

## R78. `mark_archive_arrivals` が位置を 1 件も入れなかった書庫（YouTube・マイアクティビティだけ）でも全期間を走査する
- 成果物: `crates/server/src/stay_store.rs`（`mark_archive_arrivals`）/ `worker.rs` の呼び出し
- 根拠: `f29dfea` の `stay_store.rs:646-658` は書庫の中身を受け取らず、錠を取って `from = UNIX_EPOCH - 1000 年` 〜 `UNIX_EPOCH + 7000 年` で `mark_late_arrivals` を呼ぶ。`worker.rs:1499` / `:2129` は書庫の種類を問わず毎回呼ぶ
- kind: technical
- 処置: fixed D22 — 位置のソースの要求が 0 件なら錠も取らずに返す。`archive_marking_skips_an_archive_without_locations`
- 提案: R73 の絞り込みと一緒に、位置を格納しなかったときは飛ばす

## scoped re-review（b8fc9f6..d1035a8）: R72〜R78 すべて ADDRESSED。新しい Critical / Important なし
- Minor（park）: `mark_archive_arrivals` の範囲の絞り込みは「その書庫より前に格納された行は、消すときの連鎖か前の書庫の印付けで処理済み」という前提に依る（ledger に ruling）


# code-verify 第 4 回（`4ae2c48..3739245`。Task 15 / design D22 と R72〜R78 の処置を厚く）— 2026-10-05

対象: `feat/st12-archive-ingestion` の HEAD `3739245`（PR #10 の head はまだ `bbf66a0` で、push 前）。**実装は触っていない。**
前回（第 3 回）の後のコードの差分は `crates/server/src/{archive/mod.rs, archive/worker.rs, deletion.rs, stay_store.rs, archive_flow_tests.rs, archive_tests.rs, stay_tests.rs}` と `tools/archive-shape.sh` だけ（画面・移行・API の形は触っていない）ので、第 3 回で確かめた手は繰り返さず、この差分と Task 15 を見た。
変異試験と probe は作業ツリーの外の複製（`git archive HEAD`。`~/.cache` の下。`.env` は複製せず元の worktree から読んだ）で行い、終わってから消した。

> 作業の副作用（報告）: (1) `tools/smoke.sh` が DB を `down -v` で作り直したので、試験用 DB の中身は消えた。`docker compose up -d db` と `tools/db-roles.sh` で戻し、`CT archive_erased` が 7 passed に戻ったことを確かめた。
> (2) probe の試験は共有の試験用 DB に固有の利用者で行と登録簿の行（`c03-myactivity-u1b6b1a6f8931`）を書いたが、その後の smoke の `down -v` で消えている。

## 申告: tasks の `[x]` は 48/49（13.1（人間）だけ未了）・R72〜R78 はすべて ADDRESSED。独立に実行した検証コマンド

| # | 申告 | 実行したもの |
|---|---|---|
| 1 | 14.3: fmt / clippy / `cargo test --workspace` が緑 | `set -a; . ./.env; set +a; scripts/quiet-run full -- bash -c 'cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace'` |
| 2 | 14.1 / 14.2 の `CT` | `CT archive_erased_window` / `CT archive_erased_cascade`（本文の定義のまま `tee` + `grep 'test result: ok\. [1-9]'`）、R73 / R78 の `CT stay_erase_overlap_bounds` / `CT archive_marking` |
| 3 | 14.3 / 12.2 の Scenario と validate | `python3 scripts/check_scenarios.py . st12-archive-ingestion`、`openspec validate st12-archive-ingestion --strict` |
| 4 | 12.4 / 12.5 | 本文のコマンドをそのまま（`gh pr view --json body`） |
| 5 | 11.3（R74 で `archive-shape.sh` が変わった） | `tools/smoke.sh`、`ASHIATO_ARCHIVE_USER_ID=<nil> tools/archive-shape.sh`（この機械に `psql` は無い） |
| 6 | 重なりの判定・余白・ソースの並びを試験が固定している | 複製で 3 つの変異（下の表）を入れて `cargo test -p ashiato-server` |
| 7 | 隙間 | 複製に probe の試験を 3 本足して観測（経路の点と移行前の点 / 印付けが落ち続けたとき / 位置の欄を持つマイアクティビティ） |

## 実測（一致 / 不一致）

| 申告 | 実測 | 判定 |
|---|---|---|
| 14.3 cargo | rc=0（server 517 passed / 88 passed / 7 passed。fmt・clippy も rc=0） | 一致 |
| 14.1 `CT archive_erased_window` | rc=0・4 passed | 一致 |
| 14.2 `CT archive_erased_cascade` | rc=0・2 passed | 一致 |
| R73 / R78 の試験 | `CT stay_erase_overlap_bounds` 1 passed / `CT archive_marking` 1 passed | 一致 |
| 12.2 check_scenarios | rc=0（`Scenario 696 / 印 853 / 担保あり 696`。spec に無い名前を指す印の warn 6 件は第 3 回と同じ） | 一致（中身は R79） |
| openspec validate --strict | rc=0 | 一致 |
| 12.4 / 12.5 | どちらも rc=0 | 一致（ただし PR 本文は `bbf66a0` 時点のもので「Task 15 はこの本文を書いた時点で未実装」「45/49」と書いてある。push と finish の前なので指摘にしない） |
| 11.3 smoke | rc=0。9b の段で `psql が無いので…（接続先: コンテナ db / 利用者 ashiato_app / DB ashiato…）` が 2 行出て通る（R74 の処置どおり） | 一致 |
| 試験の固定のミリ秒（R76 の材料） | python で時刻に直した: `1789178400000..1789182000000` = 02:00〜03:00Z（終わりが触れる）、`1789180200000..1789183800000` = 02:30〜03:30Z、`1789180200000..1789185540000` = 02:30〜03:59Z。消す滞在 03:00〜04:00Z とコメントの主張が一致 | 一致 |
| 変異 M2: 端が触れる重なりを外す（`{e} >= {start}` → `>`、`{end} >= s.event_time` → `>`） | `archive_erased_*` 6 本が FAILED | 一致（判定は固定されている） |
| 変異 M3: 印付けの範囲の余白 1 ms → 0 | `archive_erased_legacy_millisecond_spans_follow_the_erased_stay` が FAILED | 一致 |
| **変異 M1: `LOCATION_SOURCES` から `c03-timeline-route` と `c03-legacy-location` を外す** | **server 517 passed・0 failed** | **不一致（R79）** |
| 印付けに落ちた書庫は次の走査で印が付く（R72） | 一時的な失敗では成り立つ（試験 2 本が緑）。**落ち続けると台帳にも画面にも何も出ない** | **不一致（R80）** |

---

## R79. 書庫の位置の論理ソース 7 本のうち、件数の大半を占める `c03-timeline-route` と `c03-legacy-location` を外しても、全試験が緑のまま
- 成果物: `crates/server/src/archive/mod.rs:19-27`（`LOCATION_SOURCES`）/ `crates/server/src/archive_flow_tests.rs:2354-2370`（`ERASE_TIMELINE` / `OUTSIDE_TIMELINE`。`semanticSegments` の訪問・移動と `rawSignals` だけで、`timelinePath` が無い）/ `:2625-2650`（`archive_erased_cascade_restore_brings_the_locations_back`）/ spec `external-ingestion` の Requirement「本人が滞在を消した時間帯の書庫の位置は、削除済みの印を付けて入る」（7 本を名指し）
- 根拠: 複製で `LOCATION_SOURCES` を 5 本（`c03-timeline-route` と `c03-legacy-location` を削る）にして `cargo test -p ashiato-server` → `test result: ok. 517 passed; 0 failed`（rc=0）。
  D22 の試験の材料（`ERASE_TIMELINE` と `LEGACY_SEMANTIC_*`）に経路の点（`timelinePath`）と `Records.json` の点が 1 件も無く、`archive_locations()` の数え方も `LOCATION_SOURCES` そのものを使うので、並びから外したソースは数からも消えて試験が通る。
  いまの実装が正しく動くことは probe で確かめた: 滞在を消した後に `timelinePath` の点 2 つと `Records.json` の点 2 つ（中と外 1 つずつ）を置くと `[("c03-legacy-location", Some("user:late")), ("c03-legacy-location", None), ("c03-timeline-route", Some("user:late")), ("c03-timeline-route", None)]`、`deletion::restore` の後は `locations=2` ですべて `None`。
  同じ probe で、**後着の印（`user:late`）を戻す経路**も初めて観測した —— Scenario `滞在の削除を戻すと書庫の位置も戻る` の試験は、先に格納して後から消した（`user:cascade`）場合だけを戻している。
- 影響: 経路の点は 1 本の移動に何十もの点を持ち、移行前の点は deep の見積もりで 8 年 約 117 万行と、書庫の位置の件数の大半を占める。並びを書き換える変更（ソースの改名・並びの整理）でこの 2 本が抜けても試験は止めず、消した場面の位置が書庫から生きた記録として入る（第 4 回 Q13 の loss: exported）。
- kind: technical
- 提案: `LOCATION_SOURCES` を spec の 7 本の文字列のリテラルと `assert_eq!` で比べる試験を置く。あわせて D22 の試験の材料に `timelinePath` の点と `Records.json` の点を足し（上の probe の形）、消した後に置いた書庫の位置を `deletion::restore` で戻す場合（`user:late`）も試験で固定する。
- 処置: fixed D22 — `location_sources_are_the_seven_named_by_the_spec`（spec の 7 本のリテラルと比べる）、`ERASE_TIMELINE` に `timelinePath` の点（中・外）を足し、`archive_erased_late_marks_on_route_and_records_points_are_restored`（後着の印 `user:late` を `deletion::restore` で戻す）/ `archive_erased_cascade_marks_and_restores_records_points`。変異: `c03-timeline-route` を外すと 7 本、`c03-legacy-location` を外すと 3 本落ちる（5ce3a7d）

## R80. 書庫の位置への印付けが落ち続けると、書庫は走査のたびに丸ごと読み直され続け、台帳にも画面にも何も出ない（箱は「書庫が置かれていない」と出す）
- 成果物: `crates/server/src/archive/worker.rs:2093-2108`（`mark_archive_arrivals` が失敗したら `warn` を出して `return`。台帳も `record_store_failure` も書かない）/ `:128-163`（`record_store_failure`。3 回続いたら `store_failed` を 1 行書き、1 時間に 1 回へ落とす仕組みは格納の失敗にしか効かない）/ spec `external-ingestion`「格納に続けて失敗した書庫は台帳と画面に出る」
- 根拠: 複製に probe を足した。滞在を消し、`LateMarkFault`（この利用者の `user:late` の台帳の行を拒む trigger）を入れたまま、`Timeline.json.zip` を置いて取り込み器（`scan_sec = 1`）を 15 秒動かした:
  ```
  PROBE fired=14 ledger=[] consecutive_failures=Some(0) live_hidden=(0, 5)
  PROBE status={…,"latest_archive":null,"reading":null,"pending_shape":null,"inbox":{"capturable":true,"blockers":[],…}}
  ```
  15 秒で 14 回読み直し、そのたびに書庫を展開して全件を格納し直している（内容の鍵で行は増えない）。台帳は 0 行、`archive_sighting.consecutive_failures` は 0 のまま、`/archives/status` の `latest_archive` は `null`（画面の箱は「まだ書庫が置かれていません」と出す）。消した時間帯の 3 件は生きた記録のまま（`(hidden, live) = (0, 5)`）。
  R72 の処置は「一時的な失敗なら次の走査で印が付く」を直したもので、試験 2 本（`…_after_a_failed_marking_…`）も trigger を 1 回で外している。**落ち続ける場合は誰も止めず、誰にも見えない**。
- 影響: 本番の間隔は 120 秒なので、移行前のロケーション履歴（約 1 GB）のような書庫だと 2 分ごとに全件を読み直し続ける。そのあいだ消した場面の位置は生きた記録のまま読み出せ、本人には「置いていない」と見えるので気づけない。専用のフォルダの書庫は取り込み済みへ移らないまま残る。
- kind: technical
- 提案: 印付けの失敗も `record_store_failure` に数え、3 回続いたら `store_failed`（か印付けの失敗と分かる outcome）を台帳に 1 行書いて箱に出し、1 時間に 1 回へ落とす。試験は `LateMarkFault` を外さずに置いたまま、走査を 4 回以上回して台帳の行と `/archives/status` の `latest_archive.outcome` を見る。
- 処置: fixed D22 仮 — D22-a。置き場の書庫の印付けの失敗を `record_store_failure` に数え、3 回で `store_failed` を台帳に 1 行・以後 1 時間に 1 回（outcome は流用。反転条件は D22-a）。`archive_erased_window_persistent_marking_failure_is_ledgered_and_throttled`（fault を外さず走査 4 回以上）。写しからの読み直し（`reread_archive`）は数えない —— 下の re-review の Minor として park（5ce3a7d）

## R81. マイアクティビティの項目が位置（`locationInfos`）を持っていれば、消した滞在の時間帯でも座標を原文に持ったまま生きて入る。Q13 の「書庫が位置を入れるのは 7 本」はマイアクティビティの原文を確かめていない
- 成果物: `openspec/changes/st12-archive-ingestion/deep.md:313-315`（Q13 の「事実」: 書庫が位置を入れるのは `c03-timeline-*` と `c03-legacy-*` の 7 本）/ spec `external-ingestion` の Q13 の Requirement / `crates/server/src/archive/mod.rs:19`（`LOCATION_SOURCES`）/ `crates/server/src/archive/worker.rs:728-743`（マイアクティビティの payload は `product` / `title` / `url` / `details` だけを持ち、原文はそのまま残す）
- 根拠: 複製に probe を足した。滞在（03:00〜04:00Z）を消した後、`locationInfos` に `center=35.658,139.745` を持つマイアクティビティの項目（03:30Z・`products: ["検索"]`）を `requests_for_file` → `store_requests` → `mark_archive_arrivals` の順で格納した:
  ```
  PROBE myactivity: [("c03-myactivity-u1b6b1a6f8931", None, true, "{\"url\": …, \"title\": \"「ラーメン」を検索しました\", \"product\": \"検索\", …}")]
  ```
  `deleted_by` は `None`（生きた記録）、原文（`raw`）には座標の文字列が残る（`true`）。`mark_archive_arrivals` は `LOCATION_SOURCES` だけを見るので、位置を持つ項目でもマイアクティビティには印を付けない。
  **Google の Takeout のマイアクティビティの JSON が項目ごとに `locationInfos`（地図の URL に中心の緯度経度）を持つことがある、というのは検証者の知識で、この repo の文書（deep / design / `reference`）にも試験の材料にも出てこない**（`grep -rn locationInfos` は 0 件）。実物の書庫ではまだ確かめていない（13.1 は未了）。
- 影響: その欄が実物にあれば、本人が ST22 で消した場面の「この付近」の位置が、検索・マップ・アシスタントなどのマイアクティビティのソースから生きた記録として入り、読み出しと書き出しへ流れる（Q13 が避けた loss: exported と同じ型）。ソースの名前は形の確認の印で凍結されるので、後からソースを並びに足すことはできるが、それまでに出た分は戻らない。
- kind: premise
- 提案: 13.1（本物の Takeout を置く）の見るものに「マイアクティビティの項目に `locationInfos` があるか」を足し、`tools/archive-shape.sh` の形の出力（欄の名前）でも分かるようにする。あれば、どこまでを「書庫の位置」とするか（マイアクティビティの項目ごと印を付けるか）を deep の問い（loss: exported）として本人へ返す。
- 処置: escalated — `deep.md` 第 5 回 Q14（premise / loss: exported）。問いは `deep-questions-r5.json` / `docs/briefs/ST12-deep-r5.html`。Takeout の中身は形の確認の印まで入らない（第 2 回 Q10）ので、答えまでに外へ出るものは無い。**答え（2026-10-05）**: 推奨のまま「位置を持つ項目は印を付けて入れる（後から消したときも同じ）」→ spec の Requirement「本人が滞在を消した時間帯の書庫の位置は、削除済みの印を付けて入る」に対象を足した（Scenario 4 本）・design D22-b・tasks Task 16（15.1〜15.3）・13.1 に `field_names` の `locationInfos` を見ることを足した。実装はグラフの Task ループが回す

## 手ごとの結果

- **手 1（固定値を独立に再計算する）**: R76 で足した移行前の区間の固定のミリ秒 4 組を python で時刻に直し、コメントの主張（始まりは前・終わりが触れる / 中で終わる / 外）と一致した。この差分にハッシュの直書きは無い。
- **手 2（守りをわざと壊す）**: 重なりの判定（端が触れる）と 1 ms の余白は、外すと落ちる試験がある（M2 で 6 本・M3 で 1 本）。**位置のソースの並びは 2 本外しても全部緑**（R79）。`tools/archive-shape.sh` の psql の回り道は、`psql` の無いこの機械で一覧（rc=0）と smoke の `--confirm`（rc=0）が通り、接続先を stderr に出す。
- **手 3（Scenario と試験を突き合わせる）**: rc=0。Q13 の 5 本を 1 本ずつ読んだ。`書庫の位置の印は滞在の作り直しを待たずに付く` は `stay_criteria` の行と `rebuild:` の印が 0 であることまで見ていて主張と合う。**`滞在の削除を戻すと書庫の位置も戻る` は WHEN が「書庫の位置に削除済みの印が付いた後」で後着の印（`user:late`）も含むのに、試験は連鎖の印（`user:cascade`）だけを戻している**（実装は probe で戻ることを確かめた。R79 に含めた）。
- **手 4（本人の決定を試験が固定しているか）**: 第 4 回 Q13 の「捨てずに印を付けて入れる」は件数 `(hidden, live) = (3, 2)` の assert が、「後から消したときも印を付ける」は `archive_erased_cascade_*` が、D22 の C（端が触れるだけでも印）は M2 で落ちることを確かめた。**spec が名指しする 7 本の並びは、値を変えても全部通る**（`c03-timeline-route` / `c03-legacy-location`。R79）。
- **手 5（tasks の `[x]` と実体）**: 14.1〜14.3 の検証はすべて実在し rc=0（`CT` は 4 本・2 本、`-- --list` で数えた `archive_erased` は 7 本）。12.4 / 12.5 も rc=0。第 3 回で見た 0.1〜12.5 は、その後にコードが触っていない範囲なので繰り返していない。
- **手 6（隙間）**: 「捨てたもの・外へ出たものは戻らない」型は 2 件: R80（印付けが落ち続けると消した場面の位置が生きたまま、画面は「置いていない」）と R81（位置を持つマイアクティビティの項目が印の対象の外。前提は実物で未確認）。
  確かめて隙間でなかったもの: (1) 格納の途中で落ちた書庫を読み直すとき、前回に入った行は `Duplicate` になるが、`stored_requests` は結果を問わず全要求を持つので範囲に入り印が付く（`worker.rs:2059`）。(2) 消すときと書庫の印付けは同じ利用者の助言の錠（`LOCK_KEY`）を取るので、格納の commit が消す transaction の途中に挟まっても、印付けは消す側の commit を待ってから見る。(3) 戻した日の作り直しは、戻した滞在の最後の台帳の行が `restore` なので書庫の位置に印を付け直さない。

## scoped re-review（fef41a8..5ce3a7d）: R79・R80 とも ADDRESSED。新しい Critical / Important なし（R81 は escalated で対象外）
- Minor（park）: 写しからの読み直し（`reread_archive` / `reparse_older_versions`）で印付けが落ち続けると、毎周 写しを読み直し続け、台帳にも画面にも出ない（R80 と同型。入力は手元の写しで、版を上げたときの経路に限る。D22-a に記録済み。ledger に ruling）

# final review 第 3 回（94c427c..3482251。724de38..3482251 = Task 15 / 16・R79〜R80 の処置・rebase を厚く）— 2026-10-05

席: final reviewer（SDD の `code-reviewer.md`。ブランチ全体の review package。本文は `.superpowers/sdd/st12-task-final/final-review-3.md`）。判定は **Ready to merge: With fixes**（Critical 0 / Important 1 / Minor 7）。
入口 2 回目の申し送り: `docs/handoff/ST12.md` は前回（最終更新 2026-10-01）から増えていない。st25 R3（タイムラインの論理ソースの名前）は design D6 / D22 がすでに実装の名前（`c03-timeline-move` / `-route`）に揃っている。
package の base: `94c427c` は `origin/main` との merge-base（rebase 後）で正しい。reviewer の「ST05 が混ざる」という注記は、古いローカルの `main` で merge-base を取ったためで、事実ではない。
試験: reviewer の複製での `cargo test` は環境の都合で走らなかった（`/tmp` が他のセッションで満杯・試験用 DB のコンテナが他から消された）。走らせて rc=0 を確かめたのは `openspec validate --strict`・`check_scenarios.py`・`check-migrations.sh`・`check-immutable.sh`。
ledger の仕分け: parked 9 件のうち 8 件は park のまま（理由は本文の Ledger triage）。Task 15 の F2 / F6 / ⚠️ と Task 16 の ⚠️（validate）は解決済み。Task 16 の ⚠️（実物に `locationInfos` があるか）は 13.1 に残る。

## R82. 写しからの読み直しで印付けが落ち続けても数えないという park は、根拠が事実と違う。この経路は、最初の Takeout のマイアクティビティが入る主経路である
- 成果物: `crates/server/src/archive/worker.rs:1540`（`ingest_confirmed_pending` → `reread_archive(…, false)`）/ `:1465`（`mark_archive_arrivals(...).await?`）/ `:1547-1552`（Err は `warn` を出して `continue`）/ `:7-15`（形の確認が要る種類）/ `.superpowers/sdd/st12-task-final/progress.md:16`（ruling「解析器の版を上げたときに限る」）
- 根拠: 形の確認の印が置かれるまで、Takeout の中身は格納されない（D16）。そのため、最初に置く全期間の Takeout のマイアクティビティは必ず「置く → `pending_shape` → `--confirm` → `ingest_confirmed_pending` → `reread_archive`」の順で入る。R80 で失敗を数えるようにした `spawn_inspecting` を通るのは、形が確認済みになった 2 冊目以降だけである。`reread_archive` の呼び出し元が `reparse_older_versions` だけでないことは、`grep -n reread_archive worker.rs` で確かめた（:1540 / :1625）。
- 影響: この経路で印付けが落ち続けると、`pending_shape` が残ったまま、120 秒ごとに写しを全件読み直して格納し直すことになる。台帳にも `/archives/status` にも何も出ない。そのあいだ、消した時間帯の位置を持つマイアクティビティの項目は、生きた記録として読み出せる（Q14 の loss: exported。R80 と同じ型）。
- kind: technical
- 提案: 読み直しの経路でも、書庫の sha ごとに印付けの失敗を数える。3 回続いたら `store_failed` を台帳に 1 行書き、以後は 1 時間に 1 回へ落とす（R80 / D22-a と同じ扱い）。試験は fault を入れたまま印を置き、4 周以上回して台帳と `/archives/status` を見る。
- 処置: fixed D22 仮 — D22-a を読み直しの経路へ広げた。新しい移行 `202610051730_archive_reread_failure`（書き換えてよい観測表 `core.archive_reread_failure`、鍵は書庫の sha。`archive_sighting` は置き場のパスが鍵で、確認待ちの書庫には行が無いので使えない）。`reread_archive` の格納と印付けの失敗を数え、3 回で `store_failed` を台帳に 1 行・以後 1 時間に 1 回。`ingest_confirmed_pending` / `reparse_older_versions` の両方が待ちを見る。`archive_erased_pending_reread_persistent_marking_failure_is_ledgered_and_throttled`（直す前に落ちるのを確認）。code-verify 第 4 回 re-review Minor 1 の park はこれで閉じた。移行は 3 本になり、tasks.md の前置き「移行は 1 本だけ」（凍結）とは食い違う —— D14 に本人の決定は無く（——）、2 本目は第 3 回の処置で既に足していた（9119e7a）

## R83. `mark_archive_arrivals` の doc が D22-b の後の挙動と食い違う（Task 16 F1）
- 成果物: `crates/server/src/stay_store.rs:714`（「位置を 1 件も入れなかった書庫では何もしない（YouTube・マイアクティビティだけの書庫。R78）」）
- 根拠: 3482251 の `stay_store.rs:714` の doc の文言と、同じ関数の D22-b の分岐（`locationInfos` を持つ項目があれば錠を取って印を付ける。Task 16 F1）を読み比べた
- 影響: いまは `locationInfos` を持つマイアクティビティだけの書庫でも、錠を取って印を付ける。読んだ人が R78 の条件を取り違える。
- kind: technical
- 提案: 「位置を持たない項目だけの書庫」に直す。`mark_late_arrivals` の doc にも D22-b の対象を一言足す。
- 処置: fixed D22 — `mark_archive_arrivals` / `mark_late_arrivals` の doc（9119e7a）

## R84. D22 / D22-b の試験は、形を先に確認した経路だけを通っている（現実の最初の 1 冊の順が無い）
- 成果物: `crates/server/src/archive_flow_tests.rs:3273-3281`（`archive_erased_myactivity_window` は `confirm` を `put` より前に呼ぶ）
- 根拠: `archive_flow_tests.rs:3273-3281` で `archive_erased_myactivity_window` は `confirm` を `put` より前に呼ぶ。`ingest_confirmed_pending` を通る D22 / D22-b の試験は 3482251 に無い
- 影響: 「置く → 確認待ち → confirm → `ingest_confirmed_pending`」の順で、消した時間帯に印が付くことを固定した試験が無い。
- kind: technical
- 提案: R82 の試験と兼ねて、確認待ちを経た順で印が付く試験を足す。
- 処置: fixed D22 — `archive_erased_myactivity_window_after_confirming_a_pending_archive`（確認待ちを経た順。直す前から緑 = 隙間の補填）（9119e7a）

## R85. `myactivity_located_sql` は 1 行につき原文を最大 3 回 jsonb として読み、助言ロックを握ったまま広い範囲を評価しうる
- 成果物: `crates/server/src/stay_store.rs:688-699`
- 根拠: `stay_store.rs:688-699` の SQL は `pg_input_is_valid(raw,'jsonb')`・`raw::jsonb->'locationInfos'` を 2 回と、1 行につき 3 回解析する式を持つ（実測はしていない）
- 影響: 全期間の Takeout では、範囲に含まれる全マイアクティビティ行に jsonb の解析が掛かりうる。ただし実測はしていない。
- kind: technical
- 提案: `raw LIKE '%"locationInfos"%'` を安い前置きの条件として置く、または `raw::jsonb` を 1 回だけ取る形にする。
- 処置: fixed D22 — `myactivity_located_sql` と `carries_location` の両方に `"locationInfos"` の文字列の前置きを置いた（`\u` でエスケープした欄名は偽。Takeout は書かない形で、R86 の試験に入れた）（9119e7a）

## R86. `carries_location`（Rust）と `myactivity_located_sql`（SQL）の一致を固定する試験が無い（Task 16 F2）
- 成果物: `crates/server/src/stay_store.rs:266-286` / `:688-699`
- 根拠: `stay_store.rs:266-286`（`carries_location`）と `:688-699`（`myactivity_located_sql`）が同じ判定を別々に持ち、両者を同じ入力で比べる試験は 3482251 に無い（Task 16 F2）。実際の入力で一致することは reviewer が DB で確かめた
- 影響: 実際の入力では一致する（reviewer が DB で確かめた）。ただし片方だけを直すと、範囲と印付けが黙ってずれる。
- kind: technical
- 提案: `[]` / `null` / 配列でない値 / 最上位が配列 の 4 つについて、両者がどちらも false になる試験を置く。
- 処置: fixed D22 — `archive_myactivity_location_rust_and_sql_agree`（12 入力で Rust と SQL を期待値と比べる）（9119e7a）

## R87. 2 つの消去が重なるときの `already_deleted` + `located_e` の経路に試験が無い（Task 16 F3）
- 成果物: `crates/server/src/deletion.rs:221-226`
- 根拠: `deletion.rs:221-226` の `already_deleted` に `located_e` を足した経路を、重なる 2 つの消去で通す試験は 3482251 に無い（Task 16 F3）
- 影響: 重なる 2 つの滞在を消して片方だけ戻したとき、位置を持つ項目が隠れたままになることが担保されていない。
- kind: technical
- 提案: 重なる 2 つの滞在を消し、片方だけを戻して、項目が隠れたままであることを見る試験を足す。
- 処置: fixed D22 — `archive_erased_myactivity_overlapping_erasures_restore_one_keeps_it_hidden`（`already_deleted` からマイアクティビティを外すと落ちる）（9119e7a）

## R88. design.md の D14「移行は 1 本」と Migration Plan が、2 本目の移行と食い違う
- 成果物: `openspec/changes/st12-archive-ingestion/design.md:42` / `:491`（実際は `202609181600_archive_ingestion` と `202610042315_archive_pending_file` の 2 本）
- 根拠: `design.md:42`（「移行は 1 本」）/ `:491`（「移行 1 本（D14）」）と、`migrations/` の `202609181600_archive_ingestion` と `202610042315_archive_pending_file` の 2 本を突き合わせた
- kind: technical
- 提案: D14 と Migration Plan を 2 本に直す。
- 処置: fixed D14 — D14 の行・見出し・本文と Migration Plan を 3 本と戻す順に（05d6d6d）

## R89. 印付けの失敗も `consecutive_failures` に数えるようになり、数を戻す経路が無い件の届く範囲が広がった
- 成果物: `crates/server/src/archive/scan.rs:84-90`（UPSERT は数を戻さない）/ `docs/handoff/ST13.md:65`（R37〜R46 の M7 として申し送り済み）
- 根拠: R80 の処置で `record_store_failure` に印付けの失敗が入った。`scan.rs:84-90` の UPSERT は `consecutive_failures` を戻さない
- 影響: ダウンロードのフォルダで、同じパスに中身の違う書庫を置き直すと、前の書庫の回数と `retry_after` を引き継ぐ。
- kind: technical
- 処置: deferred ST13 — `docs/handoff/ST13.md` の M7 に R89 として届く範囲の広がりを書き足した（ST13 は `tasks.md` をまだ持たない。R63 と同じ扱い）

## scoped re-review（c05a039..05d6d6d）: R82〜R88 すべて ADDRESSED。新しい Critical / Important なし（R89 は deferred で対象外）
- 3 本目の移行は許容（書き換えてよい観測表を 1 つ足すだけで、追記のみの表に触らない。`.down.sql` と `MIGRATIONS` 21→22。tasks.md の前置きは凍結された上流の文面）
- 全体の試験: `scripts/quiet-run final -- … cargo test --workspace` rc=0（server lib 532 passed ほか、failed 0）

# code-verify 第 5 回（`724de38..218077d`。Task 16 / design D22-b と R79〜R88 の処置・3 本目の移行を厚く）— 2026-10-05

対象: `feat/st12-archive-ingestion` の HEAD `218077d`（PR #10 の head はまだ `f9a3ad3` で、push 前）。**実装は触っていない。**
第 4 回の後のコードの差分は `crates/server/src/{archive/mod.rs, archive/worker.rs, deletion.rs, lib.rs, stay_store.rs, archive_flow_tests.rs}` と移行 `202610051730_archive_reread_failure`（`git diff --stat 724de38..HEAD`）。画面（`web/`）は触っていない。第 4 回までに確かめた手は、コードが触っていない範囲では繰り返していない。
変異試験と probe は作業ツリーの外の複製（`git archive HEAD` を `~/.cache/st12-cv5/` に展開。`CARGO_TARGET_DIR` も別）で行い、終わってから消した。

> 作業の副作用（報告）: (1) 最初に `docker compose up -d db` を素で叩いて既定の 55432 で DB を立ててしまい、試験が繋がらなかった（この worktree の試験は `tools/db.sh` の 55512）。そのコンテナは落とし、`tools/db.sh up -d --wait db` と `tools/db-roles.sh` で立て直した。
> (2) `.env` の `DATABASE_URL` / `DATABASE_OWNER_URL` は 55432 のままなので、`testdb::app_pool()`（`DATABASE_URL` をそのまま使う）と `tests/server_startup.rs` の 3 本 + 2 本は、素の `.env` だと「接続できない」で落ちる。55512 に差し替えて走らせた（ST12 の差分ではない。worktree の port 割り当ての既知の件）。
> (3) 共有の試験用 DB で、tasks の `CT` の一巡と複製の変異試験を同時に走らせたところ、`stay_tests` の 3〜7 本と `CT archive_copy` / `archive_ledger` / `archive_parse_legacy` が落ちた。どれも単独で走らせ直すと緑（下の表）。2 つの `cargo test` を同じ DB に並べると干渉する（ST12 の差分ではない）。
> (4) probe の試験は共有の試験用 DB に固有の利用者で行を書いた（消していない）。

## 申告: tasks の `[x]` は 51/52（13.1（人間）だけ未了）・R82〜R88 はすべて ADDRESSED。独立に実行した検証コマンド

| # | 申告 | 実行したもの |
|---|---|---|
| 1 | 15.3 / 14.3 / 12.1: fmt / clippy / `cargo test --workspace` が緑 | `set -a; . ./.env; set +a; export DATABASE_URL=…55512 DATABASE_OWNER_URL=…55512; scripts/quiet-run full -- bash -c 'cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace'`、`cargo test -p ashiato-server --test server_startup` |
| 2 | 12.1 の画面 | `cd web && npm run test -- --run`、`npm run lint` |
| 3 | 15.1 / 15.2 ほか tasks の `CT` 全 39 本 | tasks.md の `CT <名前>` の定義のまま（`tee /tmp/ct.log` + `grep -Eq 'test result: ok\. [1-9][0-9]* passed'`）を 1 本ずつ |
| 4 | 12.2 / 12.3 | `python3 scripts/check_scenarios.py . st12-archive-ingestion`、`openspec validate st12-archive-ingestion --strict`、`python3 scripts/check_chain.py .`、`tools/check-{migrations,immutable,openapi,boundaries,licenses,private,panic-log}.sh` |
| 5 | 0.1 / 12.4 / 12.5 | 本文のコマンドをそのまま（`test ! -d …st04…`・`tools/st12_delta_diff.py`・`gh pr view --json body`） |
| 6 | Task 16 と R82 の処置を試験が固定している | 複製で 9 つの変異（下の表）を入れて `cargo test -p ashiato-server --lib -- archive_ stay_ deletion location_sources` |
| 7 | 移行の検査 | 複製で 3 本目の `.down.sql` を外す / `MIGRATIONS` から外して `tools/check-migrations.sh` |
| 8 | 隙間 | 複製に probe を 1 本足して観測（位置の欄を持つ YouTube の視聴の項目） |

## 実測（一致 / 不一致）

| 申告 | 実測 | 判定 |
|---|---|---|
| 15.3 cargo | fmt・clippy rc=0。collector-windows 133 passed / server lib 532 passed / server_startup 7 passed、failed 0（port を合わせた後） | 一致 |
| 12.1 web | `Test Files 27 passed / Tests 195 passed` rc=0、lint rc=0 | 一致 |
| 15.1 `CT archive_erased_myactivity_window` | rc=0・2 passed | 一致 |
| 15.2 `CT archive_erased_myactivity_cascade` | rc=0・1 passed | 一致 |
| tasks の `CT` 全 39 本 | 39 本すべて rc=0（3 本は同時実行の干渉で一度落ち、単独で 3 / 3 / 2 passed） | 一致 |
| 12.2 check_scenarios | rc=0（`Scenario 765 / 印 960 / 担保あり 765 / 人間の確認待ち 0`。spec に無い名前を指す印の warn 6 件は前回と同じ） | 一致 |
| 12.3 validate / chain / check-*.sh | すべて rc=0（`check-panic-log.sh` は `.env` を読ませて rc=0） | 一致 |
| 0.1 / 12.4 / 12.5 | どちらも rc=0 | 一致（ただし PR 本文は `f9a3ad3` 時点のもので「Task 16 はこの本文を書いた時点で未実装」「48/52」「写しからの読み直しは数えない（D22-a）」と書いてある。push と finish の前なので指摘にしない） |
| 手 1: マイアクティビティのソース名の固定値 | python の `hashlib.sha256` で独立に計算: `検索` → `c03-myactivity-u1b6b1a6f8931`、`マップ` → `c03-myactivity-u097022418c48`。`archive_tests.rs:1142-1145` の固定値と一致 | 一致 |
| 変異 M0（変異なし・単独） | 216 passed | 基準 |
| M1: `mark_archive_arrivals` の範囲から位置を持つマイアクティビティを外す | `archive_erased_myactivity_window` ほか 3 本 FAILED | 一致（固定されている） |
| M2: `deletion::erase` の連鎖からマイアクティビティのソースを外す | `archive_erased_myactivity_cascade` / `…_overlapping_erasures_…` FAILED | 一致 |
| M3: `mark_late_arrivals` からマイアクティビティのソースを外す | 3 本 FAILED | 一致 |
| M9: 「空でない配列」を `>= 0` にする | `archive_myactivity_location_rust_and_sql_agree` FAILED | 一致 |
| **M4: `reread_archive` の格納の失敗を数えない** | **216 passed** | **不一致（R90）** |
| **M5: 読み直しに成功しても数を消さない** | **216 passed** | **不一致（R90）** |
| **M6: `reparse_older_versions` の 1 時間の待ちを外す** | **216 passed** | **不一致（R90）** |
| **M7: 読み直しの閾値 3 回 → 4 回** | **216 passed** | **不一致（R90）** |
| **M8: 読み直しの待ち 1 時間 → 8 秒** | **216 passed** | **不一致（R90）** |
| 移行の `.down.sql` を外す | `check-migrations.sh` rc=1（`戻し手順 … が無い`） | 一致 |
| 移行を `MIGRATIONS` から外す | `check-migrations.sh` rc=0（検査は配列との一致を見ない）。ただし表が無い新しい DB では R82 の試験が `store_failed` を待って落ちる経路なので、CI（毎回新しい DB）が止める | 指摘にしない |
| design D22-b「欄の有無だけで判定し…中身の形が違っても印は付く」 | 試験 `archive_flow_tests.rs:3553` / `:3556` が「`locationInfos` が配列でなければ印を付けない」を固定している | **不一致（R91）** |

---

## R90. 読み直しの経路の失敗を数える処置（R82 / D22-a）は 5 つの部品のうち 4 つを、変えても全試験が緑のまま通す
- 成果物: `crates/server/src/archive/worker.rs:1531-1537`（格納の失敗を数える）/ `:1593`（成功したら数を消す）/ `:1708-1711`（`reparse_older_versions` の待ち）/ `:187`（`failures < 3`）/ `:191`（`interval '1 hour'`）/ `crates/server/src/archive_flow_tests.rs:3421-3484`（R82 の試験は 1 本だけ）
- 根拠: 複製で 1 つずつ変異を入れ、`cargo test -p ashiato-server --lib -- archive_ stay_ deletion location_sources` を走らせた（変異なしの基準は 216 passed）:
  - M4 `reread_archive` の格納の失敗の枝から `count_reread_failure` を外す → `ok. 216 passed`
  - M5 成功したときの `DELETE FROM core.archive_reread_failure` を何もしない文にする → `ok. 216 passed`
  - M6 `reparse_older_versions` の `reread_throttled` の確認を外す → `ok. 216 passed`
  - M7 `record_reread_failure` の閾値を `failures < 4` にする → `ok. 216 passed`（試験の assert は `fired >= 3` という下限だけ。`:3465`）
  - M8 待ちを `interval '8 seconds'` にする → `ok. 216 passed`（試験が待つのは 5 秒だけ。`:3456`）
  固定されているのは「確認待ちの読み直しで、印付けが落ち続けたら `store_failed` が 1 行書かれ、5 秒は読み直さない」だけ。`reparse_older_versions` を通る失敗の試験は無い（`grep -n reparse_older_versions *_tests.rs` は `:2960` の成功の経路 1 件だけ）。
- 影響: D22-a が「数えるなら両方まとめて」「読み直しに成功したら数を消す」「`ingest_confirmed_pending` と `reparse_older_versions` の両方が待ちを見る」と書いている部品が、片方ずつ消えても止まらない。M5 が入ると、一時的な失敗が 3 回あった書庫は、成功した後でも次の 1 回の失敗で `store_failed` と 1 時間の待ちになる。M6 が入ると、版を上げた後に印付けが落ち続ける書庫は R80 / R82 と同じく走査のたびに写しを全件読み直す（消した場面の位置が生きた記録のまま、画面は何も出さない）。値はどれも D22-a（仮）で、本人の決定ではない。
- kind: technical
- 提案: R82 の試験の形（fault を外さず走査を回す）で、(1) 格納の失敗（`FailingSink` か trigger）で数える、(2) 解析器の版の読み直し（`reparse_older_versions`）でも `store_failed` と待ちになる、(3) 2 回落ちた後に成功すると `core.archive_reread_failure` の行が消える、の 3 本を足す。閾値は `fired == 3`（`store_failed` が書かれた時点の回数）で固定する。1 時間は `retry_after - now()` を 59〜61 分で見る。
- 処置: fixed D22 — `archive_reparse_persistent_store_failure_is_ledgered_and_throttled`（格納の失敗・ちょうど 3 回・59〜61 分・版の読み直しの待ち）と `archive_reparse_success_clears_reread_failures`（成功で数の行が消える）を足し、確認待ちの経路の試験を `fired == 3` と待ちの分で見るようにした。M4〜M8 を入れ直してそれぞれ 1〜2 本が落ちることを確かめた（5749da8）

## R91. design D22-b は「欄の有無だけで判定し、中身の形が違っても印は付く（C: 厳しい側）」と書くが、実装と試験は「`locationInfos` が空でない配列のときだけ印を付ける」
- 成果物: `openspec/changes/st12-archive-ingestion/design.md:484` / `crates/server/src/stay_store.rs:682-690`（`carries_location`。`as_array()` で配列でなければ false）/ `:702-713`（`myactivity_located_sql`。`jsonb_typeof(...) = 'array'` でなければ false）/ `crates/server/src/archive_flow_tests.rs:3553`（`"locationInfos":{"name":"この付近"}` → false）/ `:3556`（`"locationInfos":"この付近"` → false）/ `tasks.md` 15.1（「空でない配列で入っている」）
- 根拠: `archive_myactivity_location_rust_and_sql_agree` は M0 で緑（rc=0）。12 入力の期待値のうち、欄はあるが配列でない 2 件（オブジェクト・文字列）を「位置を持たない」として固定している。design:484 の文言（欄の有無だけで判定・中身は解析しない・中身の形が違っても印は付く）と、tasks 15.1 / 実装 / 試験（空でない配列だけ）が食い違う。
- 影響: `locationInfos` の形は design 自身が「検証者の知識で、この repo の材料に実物は無い」と書いている。実物で欄が配列でない形（1 件だけのときにオブジェクトになる、など）だった場合、design が約束した厳しい側の既定（C）は効かず、その項目は消した時間帯でも座標を原文に持ったまま生きて入る（Q14 の loss: exported）。13.1 で `field_names` を見ても、欄の名前があることしか分からず、型までは分からない。
- kind: conflict
- 提案: どちらかに揃える。厳しい側（design の C）に揃えるなら、判定を「欄があり、`null` でも空の配列・空のオブジェクト・空文字でもない」にして、Rust と SQL と試験の期待値を同時に直す。配列だけでよいとするなら、design:484 の「中身の形が違っても印は付く」を消し、反転条件（実物が配列でなかったら）を書く。
- 処置: fixed D22 仮 — （D22-c）厳しい側（design の C）に揃えた。`locationInfos` の値が null・空の配列・空のオブジェクト・空文字でなければ印を付ける（`carries_location` と `myactivity_located_sql` を同時に）。`archive_myactivity_location_rust_and_sql_agree` の期待値を直し 3 入力を足した。反転条件は design D22-c（5749da8）

## R92. YouTube の視聴・検索の項目も同じ形の JSON で、`locationInfos` を持っていれば消した滞在の時間帯でも座標を原文に持ったまま生きて入る。13.1 はマイアクティビティの `field_names` しか見ない
- 成果物: `crates/server/src/stay_store.rs:737-742`（`mark_archive_arrivals` の範囲は `LOCATION_SOURCES` と `c03-myactivity-` だけ）/ `crates/server/src/archive/worker.rs:767-803`（YouTube の項目はマイアクティビティと同じ配列の形。原文は項目の切り出し）/ `:1159-1172`（形の出力の `field_names` は YouTube の書庫でも出る）/ `tasks.md` 13.1（「マイアクティビティの形の `field_names` に `locationInfos` があるかを見る」）/ `deep.md` Q14
- 根拠: 複製に probe を足した。滞在（03:00〜04:00Z）を消した後、`locationInfos`（`center=35.658,139.745`）を持つ YouTube の視聴の項目（03:30Z）を `requests_for_file(KnownKind::YouTubeWatch, "Takeout/YouTube and YouTube Music/history/watch-history.json", …)` → `store_requests` → `mark_archive_arrivals` の順で格納した:
  ```
  PROBE youtube_after_erase: [("c03-youtube-watch", None, true)]
  ```
  `deleted_by` は `None`（生きた記録）、原文に座標の文字列が残る（`true`）。
  **YouTube の視聴・検索の履歴が実物で `locationInfos` を持つかは確かめていない**（R81 と同じく検証者の知識の範囲。この repo の材料にも無い）。言えるのは、Takeout の YouTube の履歴はマイアクティビティと同じ形の JSON（`header` / `title` / `titleUrl` / `time` / `products`）で、Q14 の答えは対象をマイアクティビティのソースに限っている、ということまで。
- 影響: 実物の YouTube の項目にその欄があれば、Q14 が避けた型（消した場面の「この付近」の位置が生きた記録として読み出しと書き出しへ流れる。loss: exported）が、YouTube のソースからそのまま起きる。Takeout の中身は形の確認の印を置くまで入らない（第 2 回 Q10）ので、いまの時点で外へ出たものは無い。13.1 の手順のままだと、YouTube の形の `field_names` は見落とされる。
- kind: premise
- 提案: 13.1 の見るものを「マイアクティビティ・YouTube の視聴・YouTube の検索の形の `field_names` に `locationInfos` があるか」に広げる。YouTube にあれば、印の対象を「Takeout の項目のうち `locationInfos` を持つもの」へ広げるかを本人へ問う（Q14 と同じ loss: exported）。印を置く前に分かるので、問うのは 13.1 の後でよい。
- 処置: escalated — `deep.md` 第 6 回 Q15（premise / loss: exported・(未回答)）。問いは `deep-questions-r6.json`、HTML は `docs/briefs/ST12-deep-r6.html`。**答え（2026-10-05）**: 推奨のまま「位置を持つ Takeout の項目（YouTube の視聴・検索も含む）は印を付けて入れる（後から消したときも同じ）」→ spec の Requirement「本人が滞在を消した時間帯の書庫の位置は、削除済みの印を付けて入る」に対象を足した（Scenario 4 本）・design D22-d（ソースの名前でなく項目が位置を持つかで決める）・tasks Task 17（16.1〜16.3）・13.1 に YouTube の形の `field_names` を見ることを足した。実装はグラフの Task ループが回す

## 手ごとの結果

- **手 1（固定値を独立に再計算する）**: マイアクティビティのソース名の固定値 2 つを python の `hashlib` で計算し直し、一致した。Q14 の試験の材料の時刻（項目は 03:30Z / 03:40Z、消す滞在は 03:00〜04:00Z、重なる 2 つ目は 03:15〜04:15Z）は読み比べて、コメントの主張（中・重なる）と合う。
- **手 2（ガードをわざと壊す）**: Q14 の判定（M1 / M2 / M3 / M9）は外すと落ちる。**R82 の処置の部品 5 つのうち 4 つと値 2 つは、外しても全部緑**（R90）。`check-migrations.sh` は `.down.sql` を外すと rc=1。`MIGRATIONS` から外しても rc=0 だが、新しい DB の CI で R82 の試験が落ちる経路なので指摘にしない。新しい表への付与は `app_role_privileges_reach_every_table` が緑（全表を見る試験）。
- **手 3（Scenario と試験を突き合わせる）**: rc=0。Q14 の 4 本を 1 本ずつ読んだ。格納の直後（`user:late`）・消すとき（`user:cascade`。位置を持たない項目に印が付かないことまで見る）・戻すとき（`deletion::restore` の後に両方 `None`）・確認待ちを経た順（R84）で、主張と試験の階層は合う。`格納に続けて失敗した書庫は台帳と画面に出る` に足された R82 の試験は `/archives/status` の `latest_archive.outcome` まで見ている。
- **手 4（本人の決定を試験が固定しているか）**: 第 5 回 Q14 の「位置を持つ項目は印を付けて入れる（後から消したときも同じ）」は M1 / M2 / M3 で落ちる。「位置を持たない項目は印を付けない」は件数つきの assert（`("位置なし", None)`）が持つ。D22-a の 3 回・1 時間は仮で本人の決定ではないが、値を変えても全部通る（R90）。ほかの本人の決定（Q1〜Q13）を持つコードは第 4 回の後に触られていない。
- **手 5（tasks の `[x]` と実体）**: 15.1 / 15.2 の `CT` は実在し rc=0（`archive_erased_myactivity_window` 2 本・`archive_erased_myactivity_cascade` 1 本）。tasks の `CT` 全 39 本、`VT` を含む 12.1 の画面、12.3 の検査、0.1 / 12.4 / 12.5 もすべて rc=0。`cargo test --test` 型の検証は tasks に無い。
- **手 6（隙間）**: 「外へ出たものは戻らない」型は R92（YouTube の項目の位置。前提は実物で未確認）と R91（欄の形が配列でないとき）。
  確かめて隙間でなかったもの: (1) 新しい表 `core.archive_reread_failure` は DB に残るので、プロセスを再起動しても数と待ちは消えない（R80 の前のメモリだけの状態とは違う）。(2) 台帳の一意の索引は `(user_id, sha256, parser_version, outcome)`（移行 1 本目:48-49）なので、読み直しの `store_failed` の行が後の `read` の行を塞ぐことはない。(3) 新しい表もアプリの役割に付与される（`grants.sql` は schema の全表。役割の試験が緑）。


## scoped re-review（218077d..5749da8）: R90・R91 は ADDRESSED。新しい Critical / Important なし（R92 は escalated で対象外）
- R90: 版の読み直しを直に呼ぶ 2 本と確認待ちの試験の締め付けで、M4〜M8 のそれぞれが落ちる assert を持つ
- R91: Rust（`carries_location`）と SQL（`myactivity_located_sql`）が同じ厳しい側の判定に揃い、一致の試験の期待値も揃った。D22-c（仮）に反転条件
- Minor（指摘にしない）: SQL の `NOT IN` は jsonb の等価比較に依る（`[ ]` も正規化されて等価）

# final review 第 4 回（94c427c..4ecd3b9。218077d..4ecd3b9 = R90・R91 の処置・深掘り第 6 回 Q15・Task 17 を厚く）— 2026-10-05

席: final reviewer（SDD の `code-reviewer.md`。ブランチ全体の review package。本文は `.superpowers/sdd/st12-task-final/final-review-4.md`）。判定は **Ready to merge: With fixes**（Critical 0 / Important 1 / Minor 3）。
入口 2 回目の申し送り: `docs/handoff/ST12.md` は前回（最終更新 2026-10-01）から増えていない。st25 R3 は PR 本文の「未処置の申し送り」に載っている。
試験: reviewer が `cargo test -p ashiato-server --lib -- archive_erased_youtube archive_myactivity_location_rust_and_sql_agree archive_reparse_ archive_erased_myactivity`（11 passed）・`openspec validate --strict`・`check_scenarios.py`（769 / 769）・`check_chain.py` を走らせ、すべて rc=0。
ledger の仕分け: parked は全件 park のまま（第 2 回 re-review Minor 1 の前提は D22-d で範囲が広がっても成り立つ）。Task 16 の F2 / F3 と、Task 16 / 17 の ⚠️（validate）は解決済み。Task 17 の ⚠️（16.3 の証跡）は、該当の試験 11 本が緑。全体は fix の後に走らせる。

## R93. 「ソースの名前でなく項目で決める」（Q15 / D22-d）が、実装では固定名の許可リストになっていて、足し忘れを止める試験が無い
- 成果物: `crates/server/src/archive/mod.rs:32-36`（`ITEM_SOURCES`）/ `crates/server/src/stay_store.rs:670-685`（`item_sources` / `is_item_source`）/ `crates/server/src/archive/worker.rs:798-832`（論理ソースの名前を別の文字列で持つ）
- 根拠: spec の Requirement は「位置の 7 本以外で書庫が項目を入れる論理ソースは**すべて**同じ判定に掛ける」と書き、deep.md Q15 は「次に同じ形のソースが見つかっても同じ問いを立て直さないため」に名前で決めないと読んだ。実装は `c03-myactivity-` の前置きと固定の 3 本の和で、`KnownKind` に種類を足して `ITEM_SOURCES` に足し忘れても、どの試験も落ちない
- 影響: いまの集合は一致しており、バグではない。足し忘れたときに起きるのは Q13〜Q15 が避けた loss: exported（消した場面の座標を原文に持ったまま生きて入る）
- kind: technical
- 提案: `KnownKind` の全種類について要求を作り、出てくる `logical_source` が `LOCATION_SOURCES ∪ ITEM_SOURCES ∪ c03-myactivity-*` に入ることを確かめる見張りの試験を足す
- 処置: fixed D22 — `archive/classify.rs` の `every_archive_logical_source_is_classified_as_location_or_item`（種類は `_` の無い網羅の `match` で持つので、種類を足すとコンパイルで落ちる。逆向きに、分類側の名前がどれも材料から出ることも見る）。`ITEM_SOURCES` から `c03-chrome-history` を外すと FAIL、戻すと PASS（3f8e113）

## R94. Chrome の履歴は、格納の直後・連鎖・戻しの振る舞いの試験が無い（Task 17 F2）
- 成果物: `crates/server/src/archive/mod.rs:35` / `crates/server/src/archive_flow_tests.rs`（`archive_erased_youtube_*` は YouTube だけ）
- 根拠: SQL と Rust の一致の試験には Chrome が出るが、印付けの 3 経路の試験は YouTube だけ
- 影響: Chrome だけ経路から外れても、一致の試験しか落ちない
- kind: technical
- 提案: `archive_erased_youtube_cascade` に Chrome の 1 件を足す
- 処置: fixed D22 — `archive_erased_chrome_history_cascade`（消す → 位置ありだけに `user:cascade`・戻すと `locations == 1`）と `archive_erased_chrome_history_late_mark`（消した後に格納 → 位置ありだけに `user:late`）。同じ変異で 2 本とも FAIL（3f8e113）

## R95. 範囲が広がった後も、名前と文言がマイアクティビティのまま
- 成果物: `crates/server/src/stay_store.rs`（`myactivity_located_sql`）/ `crates/server/src/archive_flow_tests.rs:3898`（「マイアクティビティでない行を落とした」。Task 17 F1）/ `docs/briefs/ST12-pr.md:46`（D22-c（仮）の行が「マイアクティビティの `locationInfos`」）
- 根拠: D22-d で判定は YouTube・Chrome にも掛かる
- 影響: 反転条件を読む人が範囲を狭く読む
- kind: technical
- 提案: 関数名を範囲に合わせる。assert の文言を直す。PR 本文の D22-c に「D22-d の範囲も同じ」と足す
- 処置: fixed D22 — `myactivity_located_sql` → `item_located_sql`（呼び出し 3 箇所・doc・design.md）、assert の文言、PR 本文の D22-c の行（3f8e113）。凍結した tasks.md 16.2 の本文には旧名が残る

## R96. 作り直しのたびに、後着の印の候補に Chrome の履歴が入る
- 成果物: `crates/server/src/stay_store.rs:804`（`rebuild_day` → `mark_late_arrivals`）
- 根拠: 候補に、これまで外れていた Chrome の履歴（1 日に数百〜数千行になりうる）が入る。`located` は `strpos` の安い前置きを持ち、消した滞在との結合で絞られる（reviewer は「いまは問題にならない」と見た。実測はしていない）
- 影響: 本物の量で日ごとの作り直しが遅くなる可能性
- kind: technical
- 提案: 本物の量（13.1）で遅いと分かったら、候補の引き方を変える
- 処置: rejected: st12 の DB で、消した滞在 1 件・1 日分の Chrome の履歴 5000 行（50 行に 1 行が位置あり）・位置 1440 行を入れて `mark_late_arrivals` の候補の SQL に `EXPLAIN (ANALYZE, BUFFERS)` を当てた（ROLLBACK）。消した滞在を索引で引き、滞在ごとに `event_by_user_time_live` の Index Scan、Seq Scan なし、3532 行を読んで位置の無い行は `strpos` の前置きで落ち、4.6 ms。費用は「消した滞在の数 × 日の始まりから滞在の終わりまでの行数」に比例し、いまの範囲では問題にならない。遅ければ内側の下限を「滞在の始まり − 区間の最大の長さ」に縮める（`.superpowers/sdd/st12-task-final/fix-4-report.md`）

## scoped re-review（d3ef5b0..3f8e113）: R93〜R96 すべて ADDRESSED。新しい Critical / Important なし
- 本番コードの変更は名前の置換だけで、判定の中身は変わらない
- 全体の試験: fixer が `scripts/quiet-run final4 -- cargo test --workspace` rc=0（679 passed / failed 0）。`.env` の `DATABASE_URL` / `DATABASE_OWNER_URL` が 55432 を指したままで、st12 の DB（55512）へ置き換えて走らせた（`testdb::url()` は port を差し替えるが、`app_pool()` と `tests/server_startup.rs` は env をそのまま使う）。fmt / clippy / `check_scenarios.py` / `openspec validate --strict` も rc=0
- 範囲外の観測（park。ledger に ruling）: st12 の DB に `st12_fault_*_tg` の trigger が 4 本残る（第 2 回で park した R72 の注意と同じもの）/ 上の `.env` の port

# code-verify 第 6 回（`218077d..b3f0d59`。Task 17 / design D22-d・R90〜R96 の処置を厚く）— 2026-10-05

対象: `feat/st12-archive-ingestion` の HEAD `b3f0d59`（PR #10 の head はまだ `1920209` で、push 前）。**実装は触っていない。**
第 5 回の後のコードの差分は `crates/server/src/{archive/classify.rs, archive/mod.rs, archive_flow_tests.rs, deletion.rs, stay_store.rs}` だけ（`git diff --stat 218077d..HEAD`）。移行と画面（`migrations/` `web/`）は触っていないので、第 5 回までに確かめた手は繰り返していない。
変異試験は作業ツリーの外の複製（`git archive HEAD` を `~/.cache/st12-cv6/` に展開。`CARGO_TARGET_DIR` も別）で 1 つずつ入れて戻した。

> 作業の副作用（報告）: (1) 1 回目の `cargo test --workspace` は server の lib の途中で外から `SIGTERM` を受けて止まった（同じ時刻に別の worktree（st08）のセッションが試験を走らせていた。原因は特定していない）。走らせ直すと緑。
> (2) 変異試験の 1 回目は背景の上限（10 分）で殺され、複製に変異が 1 つ残ったまま次の回が走った。複製を作業ツリーと突き合わせて（`diff -rq` で差 0）戻してから、全部をやり直した。下の表はやり直した後のもの。
> (3) 複製の試験は、DB のポートを**ディレクトリ名から**決める（`testdb::db_port`）ので、`~/.cache/st12-cv6` では存在しないポートへ繋いで全部が `pool timed out` になった。`ASHIATO_DB_PORT=55512` を渡して直した（ST12 の差分ではない）。
> (4) 殺した試験の残りで、試験用 DB の `st12_fault_*_tg` の trigger が 4 本から 7 本に増えた（利用者ごとの名前なので他の試験には効かない。第 4 回の re-review で park したものと同じ）。消していない。

## 申告: tasks の `[x]` は 54/55（13.1（人間）だけ未了）・R93〜R96 はすべて処置済み（R96 は rejected）。独立に実行した検証コマンド

| # | 申告 | 実行したもの |
|---|---|---|
| 1 | 16.3: fmt / clippy / `cargo test --workspace` が緑 | `set -a; . ./.env; set +a; export DATABASE_URL=…55512 DATABASE_OWNER_URL=…55512; scripts/quiet-run cv6full -- bash -c 'cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace'`（1 回目は SIGTERM。2 回目は `cargo test --workspace` だけ） |
| 2 | 16.1 / 16.2 の `CT` | tasks.md の `CT` の定義のまま（`tee /tmp/ct.log` + `grep -Eq 'test result: ok\. [1-9][0-9]* passed'`）で `archive_erased_youtube_window` / `archive_erased_youtube_cascade` / `archive_myactivity_location_rust_and_sql_agree` |
| 3 | 16.3 / 12.2 / 12.3 | `python3 scripts/check_scenarios.py . st12-archive-ingestion`、`openspec validate st12-archive-ingestion --strict`、`python3 scripts/check_chain.py .`、`tools/check-{migrations,boundaries,openapi,private,licenses}.sh` |
| 4 | 0.1 / 12.4 / 12.5 / 2.x の差分なし | 本文のコマンドをそのまま。`git diff --exit-code origin/main -- crates/server/src/dedup_tests.rs crates/server/src/registry_tests.rs` |
| 5 | Q15 / D22-d と R93・R94 の処置を試験が固定している | 複製で 10 の変異（下の表）を入れて `cargo test -p ashiato-server --lib -- archive_ stay_ deletion location_sources` |
| 6 | 試験の材料の時刻 | Chrome の `time_usec` を python の `datetime`（1601-01-01 起点）で計算し直す |

## 実測（一致 / 不一致）

| 申告 | 実測 | 判定 |
|---|---|---|
| 16.3 cargo | fmt・clippy rc=0。collector-windows 133 passed / server lib 539 passed / server_startup 7 passed、failed 0（2 回目） | 一致 |
| 16.1 `CT archive_erased_youtube_window` | rc=0・1 passed | 一致 |
| 16.2 `CT archive_erased_youtube_cascade` / `CT archive_myactivity_location_rust_and_sql_agree` | どちらも rc=0・1 passed | 一致 |
| check_scenarios | rc=0（`scenarios: OK`。spec に無い名前を指す印の warn は `tools/st12_delta_diff.py` の 2 件で、前回と同じもの） | 一致 |
| validate / chain / check-*.sh（DB を作り直す `check-immutable.sh` と smoke は、移行が変わっていないので走らせていない） | すべて rc=0 | 一致 |
| 0.1 / 12.4 / 12.5 / dedup・registry の試験が無変更 | すべて rc=0 | 一致（ただし手元の PR 本文の下書き `docs/briefs/ST12-pr.md:9` / `:65` / `:103` はまだ「Task 17 は未実装」「51/55」「`check_scenarios.py` は Task 17 まで落ちる」と書いている。finish の前なので指摘にしない） |
| 手 1: 試験の材料の時刻 | `13433657520000000` → `2026-09-12 03:32:00Z`、`13433658120000000` → `03:42:00Z`（消す滞在 03:00〜04:00Z の中。コメントと一致）。見張りの材料の `13222310400000000` → `2020-01-01`（コメントは無く、分類の材料なので時刻は効かない） | 一致 |
| 変異 M0（変異なし） | 223 passed | 基準 |
| M1: `ITEM_SOURCES` から `c03-chrome-history` を外す | 見張り・Chrome の 2 本・一致の試験の計 4 本 FAILED | 一致（固定されている） |
| M2: `is_item_source` を接頭辞だけにする（格納の直後の範囲） | `archive_erased_youtube_window` / `archive_erased_chrome_history_late_mark` FAILED | 一致 |
| M3: `item_located_sql` の固定名の枝を常に偽にする（SQL の判定） | YouTube と Chrome の連鎖・一致の試験の 3 本 FAILED（位置を持たない項目にまで印が付く側も止まる） | 一致 |
| M4: `deletion::erase` の連鎖から固定名を外す | `archive_erased_youtube_cascade` / `archive_erased_chrome_history_cascade` FAILED | 一致 |
| M5: `mark_late_arrivals` から固定名を外す | 2 本 FAILED | 一致 |
| M6: `mark_late_arrivals` から項目のソースを全部外す | 5 本 FAILED | 一致 |
| M7: `ITEM_SOURCES` から `c03-youtube-search` を外す | 4 本 FAILED | 一致 |
| M8 / M9: 読み手の出す論理ソースの名前を変える（Chrome / YouTube の検索） | どちらも見張りを含む 3 本 FAILED | 一致（R93 の処置は名前の足し忘れを止める） |
| **M10: 見張りの種類の連なり（`next`）を `SemanticHistory => None` で切り、切った先の Chrome の名前を変えて分類から外す** | **見張り `every_archive_logical_source_is_classified_as_location_or_item` は緑のまま**。落ちたのは Chrome の振る舞いの 2 本と一致の試験だけ | **不一致（R97）** |

---

## R97. R93 の見張りは、種類を足したときに「連なりへ繋ぐ」ことを強制しない。繋ぎ忘れた種類は、見張りが緑のまま分類から漏れる
- 成果物: `crates/server/src/archive/classify.rs:114-125`（`next`。種類の全列挙を、手で繋いだ連なりで作る）/ `:169-172`（`kinds` は `YouTubeWatch` から `next` を辿った分だけ）/ `:203-210`（逆向きの確認は `LOCATION_SOURCES ∪ ITEM_SOURCES` の名前だけを見る）/ R93 の処置（「種類は `_` の無い網羅の `match` で持つので、種類を足すとコンパイルで落ちる」）
- 根拠: 複製で M10 を入れた —— `next` の `KnownKind::SemanticHistory => Some(KnownKind::ChromeHistory)` を `=> None` にし（ChromeHistory を連なりの外に置く）、`ITEM_SOURCES` から `c03-chrome-history` を外し、読み手が出す Chrome の名前を `c03-chrome-visits` に変えた。`cargo test -p ashiato-server --lib -- archive_ stay_ deletion location_sources` は `220 passed; 3 failed`。落ちたのは `archive_erased_chrome_history_cascade` / `archive_erased_chrome_history_late_mark` / `archive_myactivity_location_rust_and_sql_agree` で、**見張りは緑**（連なりに無い種類の材料は読まれず、名前の確認も分類側の名前しか見ない）。名前だけを変えた M8 では見張りが落ちるので、見張りが効かないのは「連なりへの繋ぎ忘れ」の 1 点だけ。
  網羅の `match` が強制するのは、新しい種類の腕を `next` と `fixture` に**書くこと**まで。新しい種類 X に `X => None` と書き、いまの末尾（`ChromeHistory => None`）を直さなくてもコンパイルは通る。コードのコメント（`:115`「繋がない種類は見張りから漏れる」）がこの限界を書いているが、R93 の処置の文は「種類を足すとコンパイルで落ちる」と書いている。
- 影響: いまの 7 種類はすべて繋がっていて、バグではない。M10 で止めたのは Chrome 専用の振る舞いの試験で、**新しい種類には振る舞いの試験が無い**ので、繋ぎ忘れと分類への足し忘れが重なると、その種類の位置の欄を持つ項目は消した時間帯でも座標を原文に持ったまま生きて入る（Q13〜Q15 が避けた loss: exported）。Takeout の中身は形の確認の印を置くまで入らない（第 2 回 Q10）ので、起きるのは新しい種類を足した後に印を置いたときから。
- kind: technical
- 提案: 種類の全列挙を手で繋がずに導く（`strum::EnumIter` の derive など。手の網羅の `match` だけでは、腕を書かせても連なりへ繋ぐことは強制できない）。足さない場合は、R93 の処置の「コンパイルで落ちる」を「コンパイルが新しい種類の腕を書かせるところまで。連なりへ繋ぐのは手」に直す。

## 手ごとの結果

- **手 1（固定値を独立に再計算する）**: Task 17 の試験の材料の Chrome の時刻 2 つを python で計算し直し、消す滞在の中（03:32Z / 03:42Z）であることを確かめた。YouTube の材料の時刻（03:30Z〜03:41Z）は ISO の文字列なので読み比べだけ。この範囲で増えたハッシュの固定値は無い。
- **手 2（ガードをわざと壊す）**: Q15 / D22-d の判定の部品（固定名の集合・Rust の範囲・SQL の判定・消すときの連鎖・後着の印）は、1 つずつ外すとどれも 2〜5 本が落ちる（M1〜M7）。R93 の見張りは名前の足し忘れ（M8 / M9）では落ち、**連なりへの繋ぎ忘れ（M10）では落ちない**（R97）。
- **手 3（Scenario と試験を突き合わせる）**: rc=0。Q15 の 4 本を 1 本ずつ読んだ。`消した滞在の時間帯の位置を持つ YouTube の履歴の項目は削除済みになる` と `位置を持たない YouTube の履歴の項目は消した時間帯でも生きた記録として入る` は、置き場に zip を置いて形の確認の印を経て取り込み器に読ませ（WHEN の「書庫を置く」と同じ階層）、視聴と検索の両方で `user:late`・台帳 2 行・原文に欄が残ることまで見る。消すとき・戻すときの 2 本は格納を `store_requests` で直に行うが、主張は消す・戻すの振る舞いなので階層は合う。THEN の「生きた記録としては読み出されない」は `deleted_at` / `deleted_by` で見ている（第 4 回・第 5 回の位置とマイアクティビティの Scenario と同じ観測。ST03 の削除済みの門）。
- **手 4（本人の決定を試験が固定しているか）**: 第 6 回 Q15 の「位置を持つ YouTube の視聴・検索の項目は印を付けて入れる（後から消したときも同じ）」は M2 / M4 / M5 / M7 で、「位置を持たない項目は印を付けない」は M3（印が付きすぎる側）で落ちる。戻すときは `restore` の後の `locations == 2` と全 4 行が `None` で見ている。D22-d が本人の答えの外から足した Chrome の履歴も M1 / M4 で落ちる。D22-c（仮）の判定は一致の試験の 15 入力で固定されている（前回の確認から変わっていない）。
- **手 5（tasks の `[x]` と実体）**: 16.1 / 16.2 / 16.3 の検証は実在し rc=0。`CT` に書かれた名前は、ファイルの名前で絞る 3 本（`api_tests` / `dedup_tests` / `registry_tests`。どれも `crates/server/src/` に実在）を除いて、すべて関数として実在する。`cargo test --test` 型の検証は tasks に無い。16.2 の本文の `myactivity_located_sql` は R95 で `item_located_sql` に名前が変わり、もう無い（凍結した文面。R95 の処置が書いている。検証のコマンドは別の名前なので rc=0）。
- **手 6（隙間）**: 「外へ出たものは戻らない」型は R97（新しい種類の繋ぎ忘れ）だけ。
  確かめて隙間でなかったもの: (1) 原文が切り出せないとき（`aligned` が `None`）は項目そのものを `serde_json::to_string` で書き戻す（`archive/worker.rs:888-892`）ので、位置の欄は原文に残り、Rust と SQL の判定は同じ原文を見る。(2) `item_sources` は固定名を `core.source` の登録に依らずに足すので、登録の前に入った YouTube・Chrome の行も連鎖と後着の印の対象になる。(3) `mark_archive_arrivals` は `mark_late_arrivals` を通るので、格納の直後の印付けは後着の印と同じ集合・同じ判定を使う（M5 で `archive_erased_youtube_window` が落ちることで確かめた）。
