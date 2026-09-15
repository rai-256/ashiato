# ST12 design — 書庫を置くだけで過去のデータが入る

読む順: `deep.md`（本人が決めたこと）→ `specs/external-ingestion/spec.md` → この文書 → `tasks.md`。

**観測できる振る舞いは specs に置いた。** この文書は「どう作るか」と、聞かずに決めた既定（C）・仮決め（B）の置き場。
**（仮）の付いた D は、反転条件が成り立ったら変えてよい。（仮）の無い D のうち「本人の決定」と書いたものは下流が変えない。**

## Context

- S-01（`crates/server`）は Windows の自宅 PC で**ネイティブに動く** axum のサーバ。設定は環境変数（`DATABASE_URL` / `API_TOKEN` / `BIND`）
- 格納の判定は `crates/server/src/lib.rs` の `ingest_one`（HTTP の JSON 1 件 → 検査 → 登録簿 → 重複・削除済み・更新 → 稼働記録の加算 → 収集開始日）にあり、**HTTP を経ずに呼べない**
- 稼働状況の API（`coverage_get`）は `coverage::must_sources()` の 5 本の名前だけを `of_sources` に渡す。画面（`web/src/App.tsx`）は返った全部を `retiredLast` の順で格子に積む
- 取り込み器（C-03）・書庫・台帳のコードは無い（`deep.md` 手順 5）

## Goals / Non-Goals

**Goals**: specs の 13 の Requirement。
**Non-Goals**:
- 書庫の記録から滞在を作る（ST16 は `c01-location` だけ）。同じ出来事が 2 経路で入る組の読み方（ST25 / ST27 へ）
- 途絶の通知（ST14）・感度の操作（ST24）・写しの消去（ST23）
- 端末から PC へ運ぶ仕組み（本人の決定 Q8）。Data Portability API での定期取得（ST13）
- Takeout の `.tgz` を読む（読めなかった書庫として台帳に残すだけ。反転条件: 本人が `.tgz` で書き出していると分かったとき）

## 決定の出所

| D | 何を決めたか | 出所 | 仮 |
|---|---|---|---|
| D1 | 取り込み器は S-01 の中の背景の仕事。設定は環境変数 | C8 / Q3（本人） | 間隔と値だけ仮 |
| D2 | 論理ソースの名前と登録 | Q1（本人）/ C1 / C13 / Q7（本人） | マイアクティビティの名前の付け方だけ仮 |
| D3 | 中身の見分け方（形 + 書庫の中のパス） | Q1 / C4 | 仮 |
| D4 | 格納の関門を切り出す | C10 / Q6（本人） | —— |
| D5 | 原文の切り出しと地域 | C2 / C15 | 大きなファイルの読み方だけ仮 |
| D6 | 各解析器が作る記録 | Q1 | 解析済みの欄は仮 |
| D7 | 書庫の台帳と書庫の中のファイルの表（追記のみ） | C16 / C4 / C5 | 場所の上限だけ仮 |
| D8 | 読んだ書庫の覚え方と「置き場で見たファイル」 | C5 / R9 | —— |
| D9 | 写しと本人のファイル | Q2（本人。補足の設定）/ C14 | 写しの置き場の既定だけ仮 |
| D10 | 取り込み器の生存信号 | C17 / R4 | 仮 |
| D11 | 取り込み済みの最終日は読むときに導く | Q4（本人）/ C12 | 仮 |
| D12 | 稼働状況の API と画面 | Q5（本人） | —— |
| D13 | 変えないもの（capability の前倒しは無い） | Step 3 | —— |
| D14 | 移行は 1 本 | —— | —— |
| D15 | 完了の判定を機械で確かめる | 完了の判定 1〜5 | —— |

---

## D1（仮）. 取り込み器は S-01 の中の背景の仕事。設定は環境変数

`run()` が HTTP の待ち受けと並べて `tokio::spawn` で取り込み器を起こす。**HTTP を経由しない**（D4）。

| 環境変数 | 意味 | 既定 |
|---|---|---|
| `ASHIATO_INBOX_DIR` | 専用のフォルダ | `%USERPROFILE%\Documents\ashiato\取り込み待ち`（本人の例。Q3） |
| `ASHIATO_DOWNLOADS_DIR` | ブラウザのダウンロードのフォルダ | `%USERPROFILE%\Downloads` |
| `ASHIATO_ARCHIVE_COPY_DIR` | 写しの置き場（D9） | `%LOCALAPPDATA%\ashiato\archive-copies` |
| `ASHIATO_ARCHIVE_KEEP_COPIES` | 写しを残すか（本人の補足） | `true`（`false` だけを「残さない」と読む。**値の綴り違いは起動を止める**） |
| `ASHIATO_ARCHIVE_USER_ID` | 書庫の記録の利用者識別子（FR-29） | 無し（**未設定なら取り込み器を起こさず、起動のログに 1 行出す**。HTTP は動く） |
| `ASHIATO_ARCHIVE_SCAN_SEC` | 走査の間隔 | `300`（試験だけが縮める） |

- **走査は 5 分おき**（spec の「置かれてから 10 分以内」を、安定の確認 2 回ぶんで満たす）
- **置き場が無いときは作らない**（本人のフォルダを勝手に作らない）。読めない状態として生存信号に載せる（D10）
- **1 つの書庫を読み終えるまで次の書庫に進まない**（直列）。移行前のロケーション履歴（百万件級）は 1 件 1 トランザクション（D4）で**数十分〜1 時間**かかる見込み。
  反転条件: 実測で 1 冊が 3 時間を超えるとき（まとめて格納する関門の形を足す。判定は変えない）
- ダウンロードのフォルダのパスは Windows の既知フォルダの変更（`Downloads` を D ドライブへ移すなど）を追わない。環境変数で指す。反転条件: 本人の環境で既定が外れていたとき

## D2. 論理ソースの名前と登録

**本人の決定（Q1）**: 6 つの中身をすべて読む。**製品 × 記録の種類ごとに別の論理ソース**（C1 / R10）。

| 中身 | 論理ソース | 1 件 |
|---|---|---|
| マップのタイムライン（`Timeline.json`） | `c03-timeline-visit` / `c03-timeline-activity` / `c03-timeline-path` / `c03-timeline-signal` | `semanticSegments` の訪問 / 移動 / 経路の点 1 つ / `rawSignals` の 1 件 |
| 移行前のロケーション履歴 | `c03-legacy-location` / `c03-legacy-visit` / `c03-legacy-activity` | `Records.json` の `locations` 1 件 / Semantic Location History の `placeVisit` / `activitySegment` |
| YouTube の視聴履歴 | `c03-youtube-watch` | 視聴 1 件 |
| YouTube の検索履歴 | `c03-youtube-search` | 検索 1 件 |
| マイアクティビティ | `c03-myactivity-<製品>`（D2 の下の規則） | 操作 1 件 |
| Chrome の履歴 | `c03-chrome-history` | 訪問 1 件 |

- **すべて `external_id_kind = 'none'`**（本人の決定 Q6 = 内容の鍵だけ）、**`expected_gap_sec = 5184000`**（60 日。C13 / Q7）、`display_name` は日本語
- 移行で `INSERT … ON CONFLICT (logical_source) DO NOTHING`（**本人が変えた想定間隔を戻さない**。ST02 の移行と同じ）
- `rawSignals` を捨てずに `c03-timeline-signal` に入れるのは、端末の中の短期の生の信号で、書き出しの後に端末から消えるため（捨てるより入れる）

**マイアクティビティの名前（仮）**: 中の製品は書庫によって増える（検索・Discover・Play・マップ・アシスタント…）ので、固定の表にしない。
項目の `products[0]` を小文字の ASCII に畳んだもの（英数字以外は `-`）を `<製品>` にする。ASCII に畳めない名前は `u` + その名前の SHA-256 の先頭 12 桁。
**登録簿に無い `<製品>` が出てきたら、取り込み器が登録簿に 1 行足してから格納する**（FR-61 の「1 行足すだけ」を取り込み器が行う。想定間隔 60 日・粒度 `none`・`display_name` は元の名前）。
反転条件: 実物の書庫で `products` が無い・言語で変わると分かったとき（名前は凍結されるので、**最初の本物の書庫を入れる前に** `tools/archive-shape.sh`（D15）で確かめる）。

## D3（仮）. 中身の見分け方

Takeout のフォルダ名とファイル名は**アカウントの言語で訳される**ことがあるので、名前だけに頼らない。**形で見分け、名前で補う。**

| 見分ける先 | 形（先に見る） | 名前（補う） |
|---|---|---|
| タイムライン | 最上位が `semanticSegments` を持つオブジェクト | `Timeline.json` |
| `Records.json` | 最上位が `locations` の配列を持つ | `Records.json` |
| Semantic Location History | 最上位が `timelineObjects` の配列を持つ | `YYYY_MONTH.json` |
| Chrome の履歴 | 最上位が `Browser History` の配列を持つ | `BrowserHistory.json` / `History.json` |
| YouTube の視聴 / 検索 | 項目が `header` / `time` / `titleUrl` を持つ配列で、**`titleUrl` が `watch?v=`（視聴）/ `results?search_query=`（検索）** | `watch-history.json` / `search-history.json` |
| マイアクティビティ | 項目が `header` / `time` / `products` を持つ配列で、YouTube の 2 つに当たらない | `MyActivity.json` / `マイアクティビティ.json` |

- 形でも名前でも当たらないファイルは**読まない**（台帳の「読まなかったファイル」に数える）
- HTML のマイアクティビティ・YouTube（`.html`）は「読めなかった」に数える（spec）。**同じ製品の JSON が同じ書庫にあれば、HTML は読まなかったに数える**
- マイアクティビティの中の YouTube の項目と、YouTube の製品の視聴履歴は**同じ出来事を 2 経路で持つ**。どちらも入れる（後続の読み方の Story へ）

反転条件: 実物の書庫で形が表と違うとき（解析器の版を上げ、覚えている書庫を読み直す。D8）。

## D4. 格納の関門を切り出す

`ingest_one(app, &Value)` を 2 段に分ける。

```text
ingest_one(app, &Value)        … HTTP 用。JSON → IngestRequest の解釈と「形が壊れている」の拒否だけ
  └ store_one(pool, IngestRequest) -> StoreOutcome   … 検査・登録簿・重複・削除済み・更新・稼働記録・収集開始日（いまの本体をそのまま）
```

- `StoreOutcome` は `Inserted(id)` / `Duplicate(id)` / `DuplicateOfDeleted(id)` / `Updated(id)` / `Skipped(id, reason)` / `Rejected(IngestError)`。
  HTTP の `IngestResult` はここから作る（**応答の形と判定は変えない**。既存の `api_tests` / `dedup_tests` がそのまま通ることで確かめる）
- **`DuplicateOfDeleted` を分ける**のは台帳の「削除済みで入れなかった」を数えるため。`external_id` を持たない記録は一意索引が削除済みの行も含めて弾くので、
  `ON CONFLICT DO NOTHING` の後に既存の行の `deleted_at` を 1 回引いて分ける
- 取り込み器は `store_one` を直接呼ぶ。`id` は**記録ごとに毎回新しい uuid**（FR-21）、`device_id = "s01-c03"`（C18）、`origin = "collected"`（C11）、
  `schema_version` は解析器の版と別に 1、`unit_system` / `crs` は既定、`source_updated_at` と `external_id` / `external_ref` は持たない（本人の決定 Q6）
- **Rejected が出たら**その 1 件を「読めなかった」に数える（形は解析器が作るので、出るのは解析器の不具合）。**Err（DB の失敗）が出たら書庫を中断し、台帳に書かない**（次の走査で読み直す。spec）

## D5. 原文の切り出しと地域

**原文（C15）**: 解析器は、対象の配列の中の各項目の**開始と終了のバイト位置**を字句の走査で見つけ、その範囲のバイト列を `raw` にする（`serde_json` で解釈するのはその範囲だけ）。
UTF-8 として正しくない範囲は読めなかった項目に数える（`raw` は `text`）。

**大きなファイルの読み方（仮）**: zip の中のファイルを 1 MiB ずつ読み、字句の状態（深さ・文字列の中か・エスケープ）を持ったまま項目の境目を探す。**ファイル全体をメモリに載せない**。
項目 1 件が 64 MiB を超えたら読めなかった項目に数えて次へ進む。反転条件: 64 MiB を超える正当な項目が実物にあったとき。

**地域（C2）**:
- 時刻文字列がずれ（`+09:00`）を持てば `tz_offset_min` にそのずれ、`tz_id` に `UTC±HH:MM` ではなく **`Etc/GMT-9` 形式の IANA 名**（ずれ 0 は `UTC`）。解析済みの内容に `"tz_from_source": true`
- ずれを持たない（`Z` / `time_usec` / `timestampMs`）なら `tz_offset_min = 0`・`tz_id = "UTC"`・`"tz_from_source": false`
- `Timeline.json` の `startTimeTimezoneUtcOffsetMinutes` があればそれを優先する
- **位置や本人の居住地から推定しない**（地方時の表示は読むときに導く。後の Story）

## D6（仮）. 各解析器が作る記録

**出来事の時刻**と、解析済みの内容（`payload`）の最小の欄。欄は仮（原文があるので足せる）。全記録に `archive_sha256` / `inner_path` / `tz_from_source` / `parser_version` を持たせる。

| 論理ソース | 出来事の時刻 | payload の主な欄 |
|---|---|---|
| `c03-timeline-visit` | `startTime` | `end_time` / `place_id` / `semantic_type` / `lat` / `lng` / `probability` |
| `c03-timeline-activity` | `startTime` | `end_time` / `activity_type` / `distance_m` / 始点と終点の `lat` `lng` |
| `c03-timeline-path` | 点の `time` | `lat` / `lng`（`"35.6812°, 139.7671°"` の文字列を数に。**原文は文字列のまま**） |
| `c03-timeline-signal` | 信号の `timestamp` | 種類（`position` / `wifiScan` / `activityRecord`）と、位置なら `lat` `lng` `accuracy_m` |
| `c03-legacy-location` | `timestamp` か `timestampMs` | `lat` / `lng`（`E7` を度に）/ `accuracy_m` / `source` / `device_tag` |
| `c03-legacy-visit` | `duration.startTimestamp` | `end_time` / `place_id` / `name` / `address` / `lat` / `lng` |
| `c03-legacy-activity` | `duration.startTimestamp` | `end_time` / `activity_type` / `distance_m` |
| `c03-youtube-watch` | `time` | `title` / `url` / `channel_name` / `channel_url` |
| `c03-youtube-search` | `time` | `query`（`titleUrl` の `search_query` を復号）/ `title` |
| `c03-myactivity-*` | `time` | `product` / `title` / `url` / `details` |
| `c03-chrome-history` | `time_usec` | `title` / `url` / `page_transition` / `client_id` |

- 座標は `crs = EPSG:4326` の度（FR-28）
- **NFC は解析済みの内容だけ**（`store_one` がそのまま行う。原文は触らない）

## D7. 書庫の台帳と書庫の中のファイルの表（追記のみ）

```text
core.archive_ledger        1 行 = 1 回の読み（読めた / 読めなかった / 既に読んだ）
  id uuid, user_id, sha256, file_name, size_bytes,
  created_at timestamptz, created_at_from ('name' | 'first_seen'),
  inbox ('dedicated' | 'downloads'), first_seen_at, started_at, finished_at,
  parser_version int, outcome ('read' | 'unreadable' | 'already_read'), unreadable_kind text NULL,
  skipped_files int
core.archive_ledger_source 1 行 = 1 回の読み × 1 論理ソース
  ledger_id, logical_source, inserted, duplicate, deleted_blocked, unreadable,
  unreadable_at jsonb   … [{inner_path, byte_offset}] 先頭 100 件まで（仮。件数は unreadable が全数を持つ）
core.archive_file          1 行 = 読んだ製品のファイル 1 つ（写しの目録。D9）
  ledger_id, inner_path, sha256, size_bytes, copied bool
```

- 3 表とも **UPDATE / DELETE / TRUNCATE を拒むトリガ**（`core.erasure_ledger` と同じ作り。`tools/check-immutable.sh` に足す）
- **読み終えてから 1 回で INSERT する**（途中の行を作らない —— 「取り込み中」は画面に出さない。本人は Q5 で取り込み中の表示を選んでいない）
- `file_name` は置き場の中の名前だけ（フォルダのパスは持たない）。**記録の本文は持たない**（spec）
- 書庫の作られた時刻: Takeout の名前 `takeout-YYYYMMDDTHHMMSSZ-NNN` から取れれば `name`、取れなければ置き場で最初に見つけた時刻を `first_seen`
- `unreadable_kind`: `unsupported_format`（`.tgz` など）/ `broken_zip` / `html_only` / `no_known_content`（形にも名前にも当たるファイルが 1 つも無い）

## D8. 読んだ書庫の覚え方と「置き場で見たファイル」

- **覚えている** = `core.archive_ledger` に同じ `user_id` / `sha256` / `parser_version` で `outcome IN ('read','unreadable')` の行がある
- 走査のたびに全ファイルのハッシュを取ると数十 GB を 5 分おきに読むので、**`core.archive_sighting`（置き場で見たファイル。書き換えてよいキャッシュ）**に
  `(inbox, file_name, size_bytes, mtime) → sha256, first_seen_at` を持ち、**大きさと更新時刻が同じならハッシュを取り直さない**
- **安定の確認**: 前回の走査と大きさ・更新時刻が同じで、かつ一時ファイルの名前でないときだけ読む（spec の「大きさが変わり続けている」）。前回の値はこの表に持つ
- **置き直し**（spec）: 覚えている `sha256` のファイルが、`archive_sighting` に無い `(inbox, file_name)` で現れたときだけ `already_read` の行を 1 つ足す。
  ダウンロードのフォルダに残り続けるファイルは sighting にあるので、走査のたびに行が増えない
- **解析器の版**: `archive::PARSER_VERSION`（整数）。上げると、覚えている書庫（`read` の行）を**写し（`core.archive_file` の目録）から**、写しが無ければ置き場のファイルから読み直す。
  どちらにも無ければ読み直さず、ログに件数だけ出す
- 取り込み済みへ移した後の書庫（`取り込み済み` の中）は走査しない

## D9. 写しと本人のファイル

**本人の決定（Q2）**: 読んだ製品のファイルだけ写しを残し、専用のフォルダの書庫は「取り込み済み」へ移す。**写しを残すかは設定で変えられ、既定は残す**（補足）。
**補足の読み取り**（`deep.md` Q2）: 設定が効くのは**システムの写し**。本人のファイルは設定に関わらず消さない。

- 写しは**中身のハッシュで名前を付ける**（`<ASHIATO_ARCHIVE_COPY_DIR>/<sha256 の先頭 2 桁>/<sha256>`）。同じ中身は 1 つ（spec）。目録は `core.archive_file`
- 「読んだ製品のファイル」= D3 の表で見分けたファイル（読めなかった項目を含むファイルも写す —— 解析器を直して読み直すため）。**写真などは写さない**
- 書庫そのもの（zip）は写さない（Q2 の選択肢 2 は採らなかった）
- 専用のフォルダの書庫は、台帳を INSERT した**後で** `取り込み済み\<元の名前>` へ `rename` する（同じボリュームなので移動は一瞬で、中身は変わらない）。
  同じ名前があれば `<元の名前> (2)` にする。移動に失敗しても台帳は残る（覚えているので読み直さない）。**ログに種別だけ出す**
- ダウンロードのフォルダは**読むだけ**（開くのも読み取り専用）
- 写しの置き場の既定（仮）: `%LOCALAPPDATA%\ashiato\archive-copies`。反転条件: D-02（ST09）の置き場が決まったとき、その隣へ移す（写しは中身の名前なので移しても目録は変わらない）
- **写しはバックアップ（ST30）の対象に入れる**ことを ST30 に申し送る必要がある —— ST30 は `待ち` で tasks が無いので、この design の Non-Goals と `deep.md` の後続に書いた ST23 と同じ扱いで、PR 本文に書く

## D10（仮）. 取り込み器の生存信号

- 走査のたびに `attempts += 1`、両方の置き場の一覧が取れた走査で `successes += 1`（片方でも読めなければ成功に数えない）
- **`Asia/Tokyo` の日が変わって最初の走査**で、D2 の全論理ソース（登録簿で `c03-` で始まるもの）について 1 件ずつ生存信号を残し、回数を 0 に戻す
- `capturable` = 直近の走査で両方の置き場が読めたか。読めなければ `blockers` に `dedicated_inbox_unreadable` / `downloads_unreadable`
- 格納は `/heartbeat` の 1 件の本体（`heartbeat_one` の検査の後ろ）を D4 と同じ形で切り出して直接呼ぶ。`raw` は信号の JSON、`device_id = "s01-c03"`
- 反転条件: 1 日 1 回では置き場の止まった時間帯が粗すぎると分かったとき（想定間隔の 1/4 など、回数を増やす）

## D11（仮）. 取り込み済みの最終日は読むときに導く

**本人の決定（Q4）**: 入った記録のうちいちばん新しい出来事の日。

- 別の表に持たず、**読むときに** `core.event` から `max(event_time)`（論理ソースごと・利用者ごと。**削除済みの行も含める** —— 取り込んだ事実は消えない）を引き、
  `Asia/Tokyo` の日にする。その行の `payload->>'archive_sha256'` から台帳の `created_at` を引く（C12 の「書庫の作られた時刻も併せて持つ」）
- 古い書庫を後から置いても `max` は動かない（spec）
- 反転条件: 引く時間が稼働状況の画面の応答で 200 ms を超えるとき（論理ソースごとの水位の表を足す。値は `core.event` から作り直せる）

## D12. 稼働状況の API と画面

**本人の決定（Q5）**: 書庫のソースにも格子、見出しの横に「`YYYY-MM-DD` まで（N 日前）」、格子の群の頭に「直近に置いた書庫」1 件、まだ無いソースは「まだ無い」。

**API**:
- `GET /coverage` —— 名前の並びを **`must_sources()` の後ろに登録簿の `c03-*`（`display_name` 順）**を足して `of_sources` に渡す。**応答の形は変えない**（`SourceCoverage` の配列）。
  `achievement_get` は変えない（NFR-13 の 5 本のまま）
- `GET /archives/status`（新設。OpenAPI に載せる）:
  `{ "sources": [{ "logical_source", "last_event_on": "YYYY-MM-DD" | null, "last_archive_created_at": … | null }],
     "latest_archive": { "file_name", "first_seen_at", "outcome", "unreadable_kind", "inserted", "duplicate", "unreadable" } | null }`。
  `latest_archive` は台帳の `finished_at` がいちばん新しい行（`already_read` を含む）で、件数は論理ソースを足し合わせた値

**画面**（`web/src`）:
- `App.tsx`: `/archives/status` を**別々に**受ける（読み出しの失敗で格子を消さない。ST02 の R19 と同じ）。
  格子の並びは **Must の 5 本 → 書庫のソース（`c03-` で始まり退役していない）→ 退役**（`retiredLast` の前に書庫のソースを分ける）
- 書庫のソースの格子の群の頭（Must の 5 本の直後）に「直近に置いた書庫」の箱: `<file_name>`・`<見つけた時刻>`・「入った N · 既にあった N · 読めなかった N」。
  `outcome = unreadable` なら「読めなかった書庫です（<種別の日本語>）」を**文字で**。台帳が空なら「まだ書庫が置かれていません」
- `CoverageGrid.tsx`: 見出しの横に**任意の注記**を受け取る（Must の 5 本には渡さない）。書庫のソースには「`YYYY-MM-DD` まで（N 日前）」か「まだ無い」。
  N は `todayInTz`（`Asia/Tokyo`）との差。**格子の形・3 段・週の選択・既定の 4 週は変えない**
- 表面は `tokens.ts` のまま。箱の境界線は `TEXT.muted`（proto と同じ）

**ひとスクロール**: 書庫のソースは Must の 5 本より下なので、Must の最後の格子の下端は変わらない（proto の実測 1,128 px。ST04 の印を足した後も ST04 の試験が見る）。
既存の `one-scroll` の試験に「書庫のソースを 11 本足した応答」の場合を足す（D15）。

## D13. 変えないもの（capability の前倒しは無い）

- **`record-envelope` の要件と判定**: `store_one` への切り出しは形だけ（D4）。`/ingest` の応答と拒否の理由は変わらない
- **`collection-coverage` の要件**: 8 状態の判定順・3 段・格子の形・達成の数え方。`/coverage` の応答の形。書庫のソースは同じ判定で出る（spec）
- **Step 3（capability の照合）**: FR-14 / FR-16 / FR-17 / FR-55 は INDEX の表どおり `external-ingestion`。
  画面に書庫のソースを出す振る舞い（FR-55）は稼働状況の画面に乗るが、**要件としては `external-ingestion` に置いた** ——
  `collection-coverage` に MODIFIED を書くと、下流が走っている ST04 と capability が重なる（衝突待ちの型）。書き直すものが無いので前倒しも要らない
- `collector-android` / `collector-windows` は触らない（Q8）

## D14. 移行は 1 本

`migrations/YYYYMMDDHHMM_archive_ingestion.sql`（名前は作成時刻）と `.down.sql`:
- `core.archive_ledger` / `core.archive_ledger_source` / `core.archive_file`（D7。`user_id` あり。追記のみのトリガ）
- `core.archive_sighting`（D8。書き換えてよい。`user_id` あり）
- 索引: `archive_ledger (user_id, sha256, parser_version)` / `archive_ledger (user_id, finished_at DESC)` /
  `core.event (user_id, logical_source, event_time DESC)`（**無ければ**。D11 の `max`）
- 登録簿の行: D2 の固定の 10 本（`c03-myactivity-*` は取り込み器が足す）
- **当て直せる形**（`IF NOT EXISTS` / `ON CONFLICT DO NOTHING`）。`MIGRATIONS` 配列の末尾に足す

## D15. 完了の判定を機械で確かめる

| 完了の判定 | 確かめ方 |
|---|---|
| 1 書庫を置くと、しばらくして中身が D-01 に入る | 結合テスト: 一時ディレクトリを置き場にして走査の間隔を 1 秒にし、合成の Takeout の書庫を置いて記録の件数を待つ |
| 2 同じ書庫をもう一度置いても行が増えない | 同じ書庫を別名で置き直し、件数が変わらず台帳に `already_read` が 1 行 |
| 3 画面に最終日が出る | vitest: `/archives/status` の固定値で「2026-09-12 まで（3 日前）」を描く。`tools/smoke.sh` に書庫を 1 冊置いて `/archives/status` の `last_event_on` を見る段を足す |
| 4 Timeline.json を置いても入る | 結合テスト: 合成の `Timeline.json`（訪問・移動・経路の点・生の信号）を置き、4 つの論理ソースに入る |
| 5 読めなかった書庫が画面に出る | vitest: `outcome = unreadable` の固定値で箱の文字を見る。結合テスト: `.tgz` を置いて台帳に `unreadable` |

- **合成の書庫**（`crates/server/tests/fixtures/archive/`）は公開されている形から手で作る。**本人の書庫を commit しない**
- **`tools/archive-shape.sh <書庫>`**: 本物の書庫の**形だけ**（書庫の中のパス・最上位の鍵・項目の欄の名前・件数・`products` の値の種類）を出す。値（題名・URL・座標・検索語）は出さない。
  D2 のマイアクティビティの名前と D3 の見分け方を、最初の本物の書庫を入れる前に確かめるためのもの
- **人間の確認待ち**（機械が再現できないもの）: 本物の Takeout の書庫と端末から書き出した `Timeline.json` を置き、画面の「直近に置いた書庫」で読めなかった 0 と出るか
  （本物の Google の書き出しは合成で置き換えられない）

## Risks / Trade-offs

- **[実物の形が公開の形と違う]** → 形で見分け（D3）、読めないものは台帳と画面に出す。`tools/archive-shape.sh` で入れる前に確かめる。解析器の版を上げれば写しから読み直せる（D8）
- **[マイアクティビティの名前が凍結される]** → `products[0]` から作る規則（D2）を、最初の本物の書庫の前に `archive-shape.sh` で確かめる。人間の確認待ちに入れた
- **[Google が表記を変えた回に全期間ぶんの行が増える]** → 本人が代償を読んで Q6 で受け入れた。増えた行は内容が残るので読む側で畳める
- **[百万件級の書庫が遅い]** → 直列で走らせ、HTTP の取り込みと DB の接続を取り合わない（接続の上限 5 のうち 1 本だけ使う）。反転条件は D1
- **[ST04 の下流と同じファイルを触る]** → `coverage.rs` は触らず `lib.rs` の `coverage_get` の名前の並びだけ、`CoverageGrid.tsx` は任意の注記を足すだけ。移行の名前は作成時刻。後から merge する側が追従する
- **[写しに本文が残る]** → 物理削除（FR-51）が写しに届かない。ST23 へ申し送り（`deep.md`）

## Migration Plan

移行 1 本（D14）。戻すときは `.down.sql`（3 表と sighting を落とし、登録簿の `c03-*` の行は**記録が無いときだけ**消す）。
取り込み器は `ASHIATO_ARCHIVE_USER_ID` が無ければ起きないので、移行を当てても既存の運用は変わらない。

## Open Questions

- Takeout の Chrome の履歴のファイル名（`BrowserHistory.json` か `History.json` か）—— 形で見分けるので、どちらでも specs と tasks は変わらない
