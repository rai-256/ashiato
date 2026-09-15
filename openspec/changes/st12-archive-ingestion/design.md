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
| D2 | 論理ソースの名前と登録 | Q1（本人）/ C1 / C13 / Q7（本人）/ C21 | マイアクティビティの名前の付け方と移行前の退役だけ仮 |
| D3 | 中身の見分け方（形 + 書庫の中のパス） | Q1 / C4 | 仮 |
| D4 | 格納の関門を切り出す | C10 / Q6（本人） | —— |
| D5 | 原文の切り出しと地域 | C2 / C15 | 大きなファイルの読み方だけ仮 |
| D6 | 各解析器が作る記録 | Q1 | 解析済みの欄は仮 |
| D7 | 書庫の台帳と書庫の中のファイルの表（追記のみ） | C16 / C4 / C5 / spec R7 / R13 | 場所の上限と失敗の回数だけ仮 |
| D8 | 読んだ書庫の覚え方と「置き場で見たファイル」 | C5 / R9 | —— |
| D9 | 写しと本人のファイル | Q2（本人。補足の設定）/ 第 2 回 Q11（本人）/ C14 / spec-r2 R7 | 写しの置き場の既定と確認待ちの写しの後始末だけ仮 |
| D10 | 取り込み器の生存信号は取り込み器の論理ソース 1 本に | C17 → C19 / C20 / deep R4 / spec R8 / R9 / deep-r2 R1 / R3 | 仮 |
| D11 | 取り込み済みの最終日は台帳の列から導く | Q4（本人）/ C12 / spec R13 | —— |
| D12 | 稼働状況の API と画面 | Q5（本人）/ 第 2 回 Q9（本人）/ C22 | 箱の高さの上限と取り込み器の止まりを出す閾値だけ仮 |
| D13 | 変えないもの・`collection-coverage` に触るもの | Step 3 / spec R2 / 第 2 回 Q9（本人） | —— |
| D14 | 移行は 1 本 | —— | —— |
| D16 | 形の確認の印（Takeout の書庫の中身） | 第 2 回 Q10（本人）/ 第 3 回 Q12（本人）/ spec-r2 R4 / R5 / R10 | —— |
| D15 | 完了の判定を機械で確かめる | 完了の判定 1〜5 | —— |

---

## D1（仮）. 取り込み器は S-01 の中の背景の仕事。設定は環境変数

`run()` が HTTP の待ち受けと並べて `tokio::spawn` で取り込み器を起こす。**HTTP を経由しない**（D4）。
取り込み器は **2 つの仕事**に分かれる —— **走査**（一覧・安定の確認・ハッシュ・読む列への追加）と**読み手**（列の先頭から 1 冊ずつ読む）。
**読み手が大きな書庫を読んでいる間も走査は続く**ので、見つけた書庫は見つけた順に並ぶ（spec の「読み終えたら続けて読む」）。

| 環境変数 | 意味 | 既定 |
|---|---|---|
| `ASHIATO_INBOX_DIR` | 専用のフォルダ | `%USERPROFILE%\Documents\ashiato\取り込み待ち`（本人の例。Q3） |
| `ASHIATO_DOWNLOADS_DIR` | ブラウザのダウンロードのフォルダ | `%USERPROFILE%\Downloads` |
| `ASHIATO_ARCHIVE_COPY_DIR` | 写しの置き場（D9） | `%LOCALAPPDATA%\ashiato\archive-copies` |
| `ASHIATO_ARCHIVE_KEEP_COPIES` | 写しを残すか（本人の補足。第 2 回 Q11 で「写しを作らない・既存は残す」と決まった） | `true`（`false` だけを「残さない」と読む） |
| `ASHIATO_ARCHIVE_USER_ID` | 書庫の記録の利用者識別子（FR-29） | 無し |
| `ASHIATO_ARCHIVE_SCAN_SEC` | 走査の間隔 | `120`（試験だけが縮める） |

- **走査は 2 分おき**。見つけた走査と、大きさ・更新時刻が同じと確かめる次の走査の 2 回で読む列に入るので、読み手が空いていれば**置いてから最悪 4 分余りで読み始める**（spec の 10 分以内。既定の値のまま時計を差し替えて確かめる。tasks 3.2）
- **起動の振る舞い（仮）**: `ASHIATO_ARCHIVE_USER_ID` が無ければ取り込み器を起こさず、起動のログに 1 行出す（HTTP は動く）。`ASHIATO_ARCHIVE_KEEP_COPIES` が `true` / `false` 以外なら**起動を止める**（綴り違いで写しの有無が黙って変わるのを止める）。
  反転条件: 本人が起動の失敗を運用上の負担と感じたとき（既定の `true` に倒してログに出す）
- **置き場が無いときは作らない**（本人のフォルダを勝手に作らない）。読めない状態として取り込み器の生存信号に載せ、画面の箱に出す（D10 / D12）
- 移行前のロケーション履歴（百万件級）は 1 件 1 トランザクション（D4）で**数十分〜1 時間**かかる見込み。
  反転条件: 実測で 1 冊が 3 時間を超えるとき（まとめて格納する関門の形を足す。判定は変えない）
- ダウンロードのフォルダのパスは Windows の既知フォルダの変更を追わない。環境変数で指す。反転条件: 本人の環境で既定が外れていたとき

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
- **取り込み器そのものの論理ソース `s01-archive-inbox`**（`external_id_kind = 'none'`・`expected_gap_sec = 86400`・`display_name` は「取り込み器」）。記録は入らず、生存信号だけが入る（D10）。
  名前が `c03-` で始まらないので、稼働状況の格子にも書庫のソースの並びにも出ない
- 移行で `INSERT … ON CONFLICT (logical_source) DO NOTHING`（**本人が変えた想定間隔を戻さない**。ST02 の移行と同じ）
- **移行前のロケーション履歴の 3 本は、読み終えたら取り込み器が登録簿の `retired_on` を「その論理ソースの最後の出来事の日（`Asia/Tokyo`）の翌日」に置く**（C21。deep-r2 R2。仮）。二度と記録が来ないので、置かないと取り込んだ日から永久に「途絶」で、ST14 の通知が鳴り続ける。
  `retired_on` が既にあってより前なら延ばす（より後なら動かさない）。**格納は `retired_on` を見ていない**ので、退役した後に古いファイルを置いても入る。
  反転条件: 本人が移行前のファイルを分けて何度も置き、退役した格子が一時的に「途絶」で見えるのが紛らわしいと分かったとき
- `rawSignals` を捨てずに `c03-timeline-signal` に入れるのは、端末の中の短期の生の信号で、書き出しの後に端末から消えるため（捨てるより入れる）

**マイアクティビティの名前（仮。第 2 回 Q10 で本人が「形の確認の印を置くまで格納しない」を選んだので、この規則は印の前に `archive-shape` の出力で確かめられる。D16）**: 中の製品は書庫によって増える（検索・Discover・Play・マップ・アシスタント…）ので、固定の表にしない。
項目の `products[0]` を小文字の ASCII に畳んだもの（英数字以外は `-`）を `<製品>` にする。ASCII に畳めない名前は `u` + その名前の SHA-256 の先頭 12 桁。
**登録簿に無い `<製品>` が出てきたら、取り込み器が登録簿に 1 行足してから格納する**（FR-61 の「1 行足すだけ」を取り込み器が行う。想定間隔 60 日・粒度 `none`・`display_name` は元の名前）。
反転条件: 印を置く前の `archive-shape` の出力で `products` が無い・言語で変わると分かったとき（**印を置く前なので、何も凍結されていない**。規則を直して解析器の版を上げてから印を置く）。

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
  HTTP の `IngestResult` はここから作る（**応答の形と判定は変えない**。既存の `dedup_tests` / `registry_tests` がそのまま通ることで確かめる）
- **`DuplicateOfDeleted` を分ける**のは台帳の「削除済みで入れなかった」を数えるため。`external_id` を持たない記録は一意索引が削除済みの行も含めて弾くので、
  `ON CONFLICT DO NOTHING` の後に既存の行の `deleted_at` を 1 回引いて分ける
- **取り込み器は格納を trait 越しに呼ぶ**: `trait RecordSink { async fn store(&self, IngestRequest) -> Result<StoreOutcome, StoreError>; }`。本番は `store_one` を包む `PgSink`、
  試験は N 件目で `Err` を返す `FailingSink`（tasks 6.2 の継ぎ目。spec R15）。生存信号も同じ形（`HeartbeatSink`）
- 取り込み器が作る `IngestRequest`: `id` は**記録ごとに毎回新しい uuid**（FR-21）、`device_id = "s01-c03"`（C18）、`origin = "collected"`（C11）、
  `schema_version` は 1（解析器の版とは別）、`unit_system` / `crs` は既定、`source_updated_at` と `external_id` / `external_ref` は持たない（本人の決定 Q6）
- **Rejected が出たら**その 1 件を「読めなかった」に数える（形は解析器が作るので、出るのは解析器の不具合）。**Err（DB の失敗）が出たら書庫を中断し、通常の台帳の行は書かない**（次の走査で読み直す）。
  **同じ書庫が続けて 3 回中断したら**、台帳に `store_failed` の行を 1 つ書き、以後その書庫は 1 時間に 1 回だけ読み直す（spec。3 回と 1 時間は仮。
  反転条件: DB の一時的な停止で `store_failed` が画面に出て本人が紛らわしいと感じたとき。回数は走査の間で持ち、再起動で 0 に戻る）

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
core.archive_ledger        1 行 = 1 回の読み（読めた / 読めなかった / 既に読んだ / 格納に失敗した）
  id uuid, user_id, sha256, file_name, size_bytes,
  created_at timestamptz, created_at_from ('name' | 'first_seen'),
  inbox ('dedicated' | 'downloads'), first_seen_at, started_at, finished_at,
  parser_version int, outcome ('read' | 'unreadable' | 'already_read' | 'store_failed' | 'pending_shape'), unreadable_kind text NULL,
  already_read_ledger_id uuid NULL   … already_read のとき、前に読んだ行（画面の「前に読んだ時刻」）
  skipped_files int
core.archive_ledger_source 1 行 = 1 回の読み × 1 論理ソース
  ledger_id, logical_source, inserted, duplicate, deleted_blocked, unreadable,
  max_event_at timestamptz NULL   … その書庫が運んだ（入った・既にあった・削除済みで入れなかった）項目のいちばん新しい出来事の時刻（D11）
  unreadable_at jsonb   … [{inner_path, byte_offset}] 先頭 100 件まで（仮。件数は unreadable が全数を持つ）
core.archive_file          1 行 = 読んだ製品のファイル 1 つ（写しの目録。D9）
  ledger_id, inner_path, sha256, size_bytes, copied bool
```

- 3 表とも **UPDATE / DELETE / TRUNCATE を拒むトリガ**（`core.erasure_ledger` と同じ作り。`tools/check-immutable.sh` に足す）
- **読み終えてから 1 回で INSERT する**（途中の行を作らない）。読んでいる間の表示は**第 2 回 Q9 で本人に問うている**（台帳ではなく読み手のメモリの状態から出す形になる。台帳は追記のみのまま）
- `file_name` は置き場の中の名前だけ（フォルダのパスは持たない）。**記録の本文は持たない**（spec）
- 書庫の作られた時刻: Takeout の名前 `takeout-YYYYMMDDTHHMMSSZ-NNN` から取れれば `name`、取れなければ置き場で最初に見つけた時刻を `first_seen`
- `unreadable_kind`: `unsupported_format`（`.tgz` など）/ `broken_zip` / `html_only` / `no_known_content`（形にも名前にも当たるファイルが 1 つも無い）

## D8. 読んだ書庫の覚え方と「置き場で見たファイル」

- **覚えている** = `core.archive_ledger` に同じ `user_id` / `sha256` / `parser_version` で `outcome IN ('read','unreadable')` の行がある（`store_failed` は覚えない。D4）
- 走査のたびに全ファイルのハッシュを取ると数十 GB を読み続けるので、**`core.archive_sighting`（置き場で見たファイル。書き換えてよいキャッシュ）**に
  `(user_id, inbox, file_name) → size_bytes, mtime, sha256, first_seen_at, last_seen_scan` を持ち、**大きさと更新時刻が同じならハッシュを取り直さない**
- **一覧に無くなったファイルの sighting の行は、その走査で消す**（spec R5）。取り込み済みへ移したファイルも一覧から消えるので行が消え、**同じ名前で置き直すと新しいファイルとして見える**
- **安定の確認**: 前回の走査と大きさ・更新時刻が同じで、かつ一時ファイルの名前でないときだけ読む列に入れる
- **置き直し**: sighting に新しく立った行のハッシュが覚えている書庫と同じなら、中身を読まずに `already_read` の行を 1 つ足す（専用のフォルダなら取り込み済みへ移す）。
  ダウンロードのフォルダに残り続けるファイルは sighting の行が残るので、走査のたびに行が増えない
- **解析器の版**: `archive::PARSER_VERSION`（整数）。上げると、覚えている書庫（`read` の行）を**写し（`core.archive_file` の目録）から**、写しが無ければ置き場のファイルから読み直す。
  どちらにも無ければ読み直さず、ログに件数だけ出す
- 取り込み済みへ移した後の書庫（`取り込み済み` の中）は走査しない

## D9. 写しと本人のファイル

**本人の決定（Q2）**: 読んだ製品のファイルだけ写しを残し、専用のフォルダの書庫は「取り込み済み」へ移す。**写しを残すかは設定で変えられ、既定は残す**（補足）。
**補足の読み（第 2 回 Q11 で本人が決めた）**: 設定が効くのは**システムの写し**で、「残さない」は写しを作らないこと（既に作った写しは残す）。本人のファイルは設定に関わらず消さない。
**例外: 形の確認を待っている書庫（D16）は、設定に関わらず写しを作る**（第 2 回 Q10 の選択肢の文。写しが無いと印を置いた後に読み直せない）。
**（仮）残さない設定のとき、確認待ちのために作った写しは、印を置いて読み直し終えたら消す**（spec R7）—— 本人が第 2 回 Q11 で選んだ「写しを作らない」に戻す。
消した後にその書庫を解析器の版の上げで読み直せないのは、残さない設定の他の書庫と同じ（本人が Q11 で受け入れた代償）。反転条件: 本人が確認待ちの写しは残したいと言ったとき

- 写しは**中身のハッシュで名前を付ける**（`<ASHIATO_ARCHIVE_COPY_DIR>/<sha256 の先頭 2 桁>/<sha256>`）。同じ中身は 1 つ（spec）。目録は `core.archive_file`
- 「読んだ製品のファイル」= D3 の表で見分けたファイル（読めなかった項目を含むファイルも写す —— 解析器を直して読み直すため）。**写真などは写さない**
- 書庫そのもの（zip）は写さない（Q2 の選択肢 2 は採らなかった）
- 専用のフォルダの書庫は、台帳を INSERT した**後で** `取り込み済み\<元の名前>` へ `rename` する（同じボリュームなので移動は一瞬で、中身は変わらない）。
  同じ名前があれば `<元の名前の拡張子の前> (2)` から空いている番号にする（spec）。移動に失敗しても台帳は残る（覚えているので読み直さない）。**ログに種別だけ出す**
- ダウンロードのフォルダは**読むだけ**（開くのも読み取り専用）
- 写しの置き場の既定（仮）: `%LOCALAPPDATA%\ashiato\archive-copies`。反転条件: D-02（ST09）の置き場が決まったとき、その隣へ移す（写しは中身の名前なので移しても目録は変わらない）
- **写しはバックアップ（ST30）の対象に入れる**ことを ST30 に申し送る必要がある —— ST30 は `待ち` で tasks が無いので、この design の Non-Goals と `deep.md` の後続に書いた ST23 と同じ扱いで、PR 本文に書く

## D10（仮）. 取り込み器の生存信号は取り込み器の論理ソース 1 本に

**第 1 回の C17 から変えた**（C19。spec の独立レビュー R9）: 書庫の各論理ソースに毎日信号を送ると、ST02 の判定順で書庫のソースが「途絶」にならず、
FR-35 の「最後の記録または最後の生存信号」からの通知も鳴らないので、本人が決めた 60 日（第 1 回 Q7）が効かなくなる。

- 信号は **`s01-archive-inbox` だけ**に送る。書庫の論理ソースの稼働状況は**記録だけ**から ST02 の判定で決まる（記録の間が 60 日を超えた日が「途絶」）
- **`Asia/Tokyo` の日の最初の走査**で 1 件残す（`emitted_at` はその走査の時刻なので、その日に属する）。同じ日に既にあるか（`core.heartbeat` をその日の範囲で引く）を見てから送るので、再起動しても 2 件にならない
- `attempts` は前回の信号からの走査の回数、`successes` は 2 つの置き場をどちらも読めた走査の回数。**回数は `core.archive_scan_counter`（書き換えてよい 1 行の表）に走査ごとに足し、信号を送ったら 0 に戻す**（C20。deep-r2 R3。
  メモリだけに持つと、PC を毎晩切る運用で毎日その日の回数が消える。**載せなかった期間の取得率は後から作れない**）
- `capturable` = 直近の走査で両方の置き場が読めたか。読めなければ `blockers` に `dedicated_inbox_unreadable` / `downloads_unreadable`
- 格納は `/heartbeat` の 1 件の本体（`heartbeat_one` の検査の後ろ）を D4 と同じ形で切り出して呼ぶ。`raw` は信号の JSON、`device_id = "s01-c03"`
- 画面: `/archives/status` に直近の信号の `capturable` と `blockers` を載せ、読めない置き場を箱の中に文字で出す（D12）

## D11. 取り込み済みの最終日は台帳の列から導く

**本人の決定（Q4）**: 入った記録のうちいちばん新しい出来事の日。C12 の「列を持つ」は、**台帳の論理ソースごとの行の `max_event_at` と台帳の `created_at`** として持つ（spec R13）。

- `last_event_on` = その利用者・論理ソースの `archive_ledger_source.max_event_at` の最大（`outcome = 'read'` の行）を `Asia/Tokyo` の日にしたもの
- `last_archive_created_at` = その最大の `max_event_at` と同じ値を持つ行の書庫のうち、`created_at` がいちばん新しいもの（spec の「最終日の出来事を運んだ書庫のうち、いちばん新しく作られた書庫」）
- `max_event_at` は入った・既にあった・**削除済みで入れなかった**項目を含めて数える（取り込んだ事実は削除で消えない。spec）
- 古い書庫を後から置いても最大は動かない（spec）
- `core.event` を引かないので、記録の件数に比例して遅くならない

## D12. 稼働状況の API と画面

**本人の決定（Q5 / 第 2 回 Q9）**: 書庫のソースにも格子（開いた直後は直近 4 週）、見出しの横に「`YYYY-MM-DD` まで（N 日前）」、**「直近に置いた書庫」1 件の箱を Must の 5 本の前（達成の下）**、読んでいる間は箱に件数、まだ無いソースは「まだ無い」。

**API**:
- `GET /coverage` —— 名前の並びを **`must_sources()` の後ろに登録簿の `c03-*`（`display_name` 順）**を足して `of_sources` に渡す。**応答の形は変えない**（`SourceCoverage` の配列）。
  `achievement_get` は変えない（NFR-13 の 5 本のまま）
- `GET /archives/status`（新設。OpenAPI に載せる）:
  `{ "sources": [{ "logical_source", "last_event_on": "YYYY-MM-DD" | null, "last_archive_created_at": … | null }],
     "latest_archive": { "file_name", "first_seen_at", "outcome", "unreadable_kind", "inserted", "duplicate", "unreadable", "previously_read_at" } | null,
     "reading": { "file_name", "inner_path", "items_read", "started_at" } | null,
     "pending_shape": { "archives": n, "files": n } | null,
     "inbox": { "capturable": bool, "blockers": [...], "emitted_at" } | null }`。
  `reading` は読み手のメモリの状態（台帳は読み終えてから書くので、台帳からは出ない）。1,000 件ごとに更新する
  `latest_archive` は台帳の `finished_at` がいちばん新しい行（`already_read` / `store_failed` を含む）で、件数は論理ソースを足し合わせた値。
  `already_read` のときは件数を持たず、`previously_read_at` に前に読んだ行の `finished_at` を入れる（spec R3 の (2)）

**画面**（`web/src`）:
- `App.tsx`: `/archives/status` を**別々に**受ける（読み出しの失敗で格子を消さない。ST02 の R19 と同じ）。
  格子の並びは **Must の 5 本 → 書庫のソース（`c03-` で始まり退役していない）→ 退役**（`retiredLast` の前に書庫のソースを分ける）
- **Must の 5 本の前（`AchievementPanel` の直後）**に「直近に置いた書庫」の箱: `<file_name>`・`<見つけた時刻>`・「入った N · 既にあった N · 読めなかった N」。
  `outcome = unreadable` なら「読めなかった書庫です（<種別の日本語>）」、`store_failed` なら「格納に失敗しています（1 時間ごとに読み直します）」、
  `already_read` なら「既に読んだ書庫です（<前に読んだ時刻> に読んだものと同じ中身）」を**文字で**。台帳が空なら「まだ書庫が置かれていません」。
  `inbox.capturable = false` なら「<置き場> が読めません」を足す。**取り込み器の最後の信号が 3 日（想定間隔 1 日の 3 倍）より前か一度も無ければ**「取り込み器の最後の確認: N 日前」（一度も無ければ「取り込み器はまだ一度も動いていません」）を足す（C22。deep-r2 R4。3 日は仮。反転条件: PC を数日切る運用で本人が紛らわしいと感じたとき）
- `reading` があれば箱の先頭に「読んでいます: <file_name> <items_read> 件まで（<started_at> から）」（第 2 回 Q9）。`pending_shape` があれば「形の確認を待っている書庫が N 冊あります（`tools/archive-shape.sh` で形を見て印を置く）」（D16）
- **箱の高さは 160 CSS px 以下（仮）**: **直近の書庫が読めなかった・格納に失敗した行は省かない**（spec R14。第 1 回 Q5 の理由そのもの）。残りを優先順（読んでいる途中 → 読めない置き場・取り込み器の止まり → 形の確認待ち → 直近の書庫の読めた結果）に積み、溢れる行は省いて「ほか N 件」を 1 行出す。
  反転条件: 優先の低い行が常に省かれ、本人が見落としたと分かったとき（上限を上げ、`collection-coverage` の予算の文も合わせて直す）
- `CoverageGrid.tsx`: 見出しの横に**任意の注記**を受け取る（Must の 5 本には渡さない）。書庫のソースには「`YYYY-MM-DD` まで（N 日前）」か「まだ無い」。
  N は `todayInTz`（`Asia/Tokyo`）との差。**格子の形・3 段・週の選択・既定の 4 週は変えない**
- 表面は `tokens.ts` のまま。箱の境界線は `TEXT.muted`（proto と同じ）

**ひとスクロール（第 2 回 Q9 で本人が変えた）**: 箱が Must の 5 本の前に入るので、Must の最後の格子の下端は箱の高さぶん下がる（第 2 回の proto の実測: 箱あり 1,379 px / 2 本目の直近 4 週 754 px）。
`collection-coverage` の予算の文を「箱の高さを除いて数える」に MODIFIED で直し（D13）、**既存の `one-scroll.test.tsx` の勘定から `min(箱の宣言の高さ, 160)` を引く**（予算の定数 `VIEWPORT_H_PX = 640` / `ONE_SCROLL_PX = VIEWPORT_H_PX * 2` は変えない。箱が伸びても予算は 160 px ぶんまでしか伸びない。spec-r2 R13）。
`declaredHeight` は `web/src/__tests__/` の共有の helper に出し、箱の 160 px の試験（tasks 10.3）も同じ勘定で測る。
書庫のソースの格子を足した応答（固定の 10 本 + マイアクティビティ 2 本 = 12 本）と高さ 160 px の箱の場合の試験を足す（tasks 10.4）。

## D13. 変えないもの・`collection-coverage` に触るもの

- **`record-envelope` の要件と判定**: `store_one` への切り出しは形だけ（D4）。`/ingest` の応答と拒否の理由は変わらない
- **`collection-coverage` の判定は変えない**: 8 状態の判定順・3 段・格子の形・達成の数え方・`/coverage` の応答の形。書庫のソースは同じ判定で出る
- **`collection-coverage` の「稼働状況は 1 年を週に畳んだ格子で見える」を MODIFIED で 1 か所だけ変える** —— 開いた直後の 2 ソースとひとスクロールの高さを、**Must の前に置く箱の高さを除いて数える**（第 2 回 Q9。本人が「予算を変える判断になる」を読んで選んだ）。
  **この Requirement は ST04 の下流（PR #49）も MODIFIED で書き換えている**ので、ST12 の delta は **ST04 の delta の文を写して予算の文だけを変えた**。
  **写した元は PR #49 の head（8abafb5）**（spec-r2 R1。main の上流の版より 1 文と Scenario 1 本多い）。
  **ST12 の `requires` に ST04 を足した**（layer 3）。**ただし `requires` は盤面の表示に効くだけで、下流の起動を機械では止めない**（spec-r2 R2: `board.py` は tasks.md があると requires を見ず、`merge_gate.sh` は上流の merge 後に `story.sh` を出し、`story.sh` は requires を見ない）。
  **止めるのは tasks 0.1 の終了条件**（ST04 の change が archive されていなければ rc=1）と、PR 本文・issue の冒頭の文。0.1 は正典（ST04 の archive 後）と ST12 の delta を突き合わせ、許した差のほかに差があれば rc=1。ST04 へは差し戻さない
  （harness2 側の穴として報告する: 上流の lane の gate が requires の未 archive を見ない）
- **第 1 回の D13（仮）の反転条件が成り立った** —— 第 1 回は画面の要件を `external-ingestion` だけに置き、`collection-coverage` に割り当てなかった。予算の文を書き換える本人の決定で、割り当てた（`docs/stories/INDEX.md` の 2026-09-15 の訂正）。
  書庫のソースの並び（Must → 書庫 → 退役）・見出しの注記・箱の中身は `external-ingestion` に置いたまま
- **同じファイルは触る**: `lib.rs`（`MIGRATIONS`・route・`coverage_get` の名前の並び）・`App.tsx`・`CoverageGrid.tsx`・`docs/openapi.json` と既存の試験 3 本（tasks 1.1 / 9.2 / 10.4）
- `collector-android` / `collector-windows` は触らない（Q8）

## D14. 移行は 1 本

`migrations/YYYYMMDDHHMM_archive_ingestion.sql`（名前は作成時刻）と `.down.sql`:
- `core.archive_ledger` / `core.archive_ledger_source` / `core.archive_file`（D7。`user_id` あり。追記のみのトリガ）
- `core.archive_sighting`（D8。書き換えてよい。`user_id` あり）/ `core.archive_scan_counter`（D10。書き換えてよい 1 行。`user_id` あり）/ `core.archive_pending_shape`（D16。書き換えてよい）
- `core.archive_shape_confirmation`（D16。**追記のみ**のトリガ。`user_id` あり）
- 索引: `archive_ledger (user_id, sha256, parser_version)` / `archive_ledger (user_id, finished_at DESC)` / `archive_ledger_source (logical_source, max_event_at DESC)`
- 登録簿の行: D2 の固定の 10 本と `s01-archive-inbox`（計 11 本。`c03-myactivity-*` は取り込み器が足す）
- **当て直せる形**（`IF NOT EXISTS` / `ON CONFLICT DO NOTHING`）。`MIGRATIONS` 配列の末尾に足す

## D15. 完了の判定を機械で確かめる

| 完了の判定 | 確かめ方 |
|---|---|
| 1 書庫を置くと、しばらくして中身が D-01 に入る（Takeout は印を置いてから） | 結合テスト: 一時ディレクトリを置き場にして走査の間隔を 1 秒にし、合成の形の印を入れた DB で合成の Takeout の書庫を置いて記録の件数を待つ。印の無い DB では 0 件のまま（D16） |
| 2 同じ書庫をもう一度置いても行が増えない | 同じ書庫を別名で置き直し、件数が変わらず台帳に `already_read` が 1 行 |
| 3 画面に最終日が出る | vitest: `/archives/status` の固定値で「2026-09-12 まで（3 日前）」を描く。`tools/smoke.sh` に書庫を 1 冊置いて `/archives/status` の `last_event_on` を見る段を足す |
| 4 Timeline.json を置いても入る | 結合テスト: 合成の `Timeline.json`（訪問・移動・経路の点・生の信号）を置き、4 つの論理ソースに入る |
| 5 読めなかった書庫が画面に出る | vitest: `outcome = unreadable` の固定値で箱の文字を見る。結合テスト: `.tgz` を置いて台帳に `unreadable` |

- **合成の書庫**（`crates/server/tests/fixtures/archive/`）は公開されている形から手で作る。**本人の書庫を commit しない**
- **`tools/archive-shape.sh`**: 形の確認を待っている書庫の**形だけ**を写しから出し、`--confirm` で印を置く（D16）
- **人間の確認待ち**（機械が再現できないもの）: 本物の Takeout の書庫と端末から書き出した `Timeline.json` を置き、画面の「直近に置いた書庫」で読めなかった 0 と出るか
  （本物の Google の書き出しは合成で置き換えられない）

## D16. 形の確認の印（Takeout の書庫の中身）

**本人の決定（第 2 回 Q10）**: 形の確認の印を置くまで、Takeout の書庫の中身は格納しない（台帳と写しには残す。印を置いたら写しから読み直す）。

- **形**（shape）: Takeout の書庫の中の、読む対象のファイルごとに **論理ソースの名前を決めるものだけ** —— `(見分けた種類, マイアクティビティなら各項目の products[0] から D2 の規則で作った製品の名前の集合)`（spec-r2 R5 / deep-r3 R1 で狭めた）。
  **比べ方は集合の一致ではなく「印を置いた名前・種類に無いものが含まれるか」**。`products` の 2 つ目以降の値が増減しても、作る名前が同じなら確認待ちにしない
  項目の欄の名前・書庫の中のパスの名前は、論理ソースの名前を決めないので形に入れない（Google が欄を 1 つ足しただけで確認待ちにしない）。
  **確認の出力には**、判断の材料として見分けた種類・パスの型・最上位の鍵・欄の名前・件数・`products` の値も出す。**値（題名・URL・検索語・座標・時刻）は出さない**
- **確認待ち**: 読み手は Takeout の書庫を開いて見分け、写しを作り（設定に関わらず）、形を取り、**印を置いた形の集合に無い形のファイル**を格納せずに `core.archive_pending_shape`（書き換えてよい）に積み、
  台帳に `outcome = 'pending_shape'` の行を **1 回の読みに 1 つ**書く（D7 の粒度。ファイルごとの確認待ちは `archive_pending_shape` の側に持つ。spec R10）。**印を置いた形のファイルは同じ書庫の中でも格納する**（spec）。
  確認待ちの書庫は「覚えている」（D8）に入れて走査のたびに読み直さない。**印を置いたら、同じ中身・同じ解析器の版でも読み直し、台帳に `read` の行を 1 つ足す**
- **印**: `core.archive_shape_confirmation`（追記のみ。`user_id`・形のハッシュ・形の JSON・印を置いた時刻）。**置く手段は `tools/archive-shape.sh`**:
  引数なしで確認待ちの形を一覧（値を出さない。spec）、`--confirm <形のハッシュ>…` で印を置く。印を置くと読み手が次の走査で `archive_pending_shape` の書庫を写しから読み直す
- **印のときに無かった中身・製品の名前が出たら、そのファイルだけまた確認待ち**（第 3 回 Q12 の本人の決定）。また確認待ちになるのは次の型（本人への答えの逐語は deep.md の Q12）:
  (1) マイアクティビティの製品が増えた（「Discover」「マップ」「アシスタント」…）(2) 初めて出る中身（最初の書庫に無かった Chrome の履歴・分割書庫の `-002.zip` の別の製品）
  (3) 書き出しの言語が変わって `products` の値が「検索」→ `Search` になった (4) Google が製品の名前を変えた。
  **ならないもの**: 項目の欄が増えた / フォルダ・ファイルの名前が訳された / `products` の 2 つ目以降の値の増減 / 件数の変化
- **Timeline.json と移行前のロケーション履歴は確認を待たない**（最上位の形が一意で、名前が固定。第 2 回 Q10 の選択肢の文）
- 画面: `/archives/status` の `pending_shape` から箱に「形の確認を待っている書庫が N 冊あります」（D12）
- 本人の手順書（`docs/archive-inbox.md`）: 最初の Takeout の書庫を置く → 箱に確認待ちが出る → `tools/archive-shape.sh` の出力を見る（値は出ない）→ `--confirm`

## Risks / Trade-offs

- **[実物の形が公開の形と違う]** → 形で見分け（D3）、読めないものは台帳と画面に出す。`tools/archive-shape.sh` で入れる前に確かめる。解析器の版を上げれば写しから読み直せる（D8）
- **[マイアクティビティの名前・見分け方が凍結される]** → 形の確認の印を置くまで Takeout の書庫の中身は格納しない（D16。本人の決定）。人間の確認待ち H.1 で本物の書庫の形を見て印を置く
- **[Google が表記を変えた回に全期間ぶんの行が増える]** → 本人が代償を読んで Q6 で受け入れた。増えた行は内容が残るので読む側で畳める
- **[百万件級の書庫が遅い]** → 読み手は 1 冊ずつ、HTTP の取り込みと DB の接続を取り合わない（接続の上限 5 のうち 1 本だけ使う）。読んでいる間も走査は続く。反転条件は D1
- **[ST04 の下流と同じファイルを触る]** → `coverage.rs` は触らず `lib.rs` の `coverage_get` の名前の並びだけ、`CoverageGrid.tsx` は任意の注記を足すだけ。移行の名前は作成時刻。後から merge する側が追従する
- **[既存の試験 2 本が「末尾の移行」「`/coverage` はちょうど 5 本」を固定している]** → tasks 1.1 / 9.2 で、主張を「滞在の移行がある」「先頭の 5 本が Must の順、その後ろは `c03-` だけ」に直す（試験の意図は変えない）
- **[写しに本文が残る]** → 物理削除（FR-51）が写しに届かない。ST23 へ申し送り（`deep.md`）

## Migration Plan

移行 1 本（D14）。戻すときは `.down.sql`（3 表と sighting を落とし、登録簿の `c03-*` の行は**記録が無いときだけ**消す）。
取り込み器は `ASHIATO_ARCHIVE_USER_ID` が無ければ起きないので、移行を当てても既存の運用は変わらない。

## Open Questions

- Takeout の Chrome の履歴のファイル名（`BrowserHistory.json` か `History.json` か）—— 形で見分けるので、どちらでも specs と tasks は変わらない
