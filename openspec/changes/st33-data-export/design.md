# ST33 設計 — 全記録を外部の道具で読める形に書き出す

## Context

動機は `proposal.md` の Why。本人が決めた 5 件（Q1〜Q5）と、聞かずに決めた既定 C1〜C18 は `deep.md`。
観測可能な振る舞いは `specs/data-durability/spec.md` に置いた。**ここで決めるのは、それを実装に落とすときの技術判断だけ。**

読む前提になる既存の形（main・ST22 の archive 後）:

| 場所 | いまの形 |
|---|---|
| `GET /events`（`lib.rs` の `events`） | `core.event_live` の 6 列を `fetch_all` で全件返す。**変えない**（C13。ST01 の読み出し経路） |
| `core.event_live` | `SELECT * FROM core.event WHERE deleted_at IS NULL`（最新は `202609120941_event_columns.sql`）。**作った時点で列が固まる** |
| `core.event_version_live` | 親が生きている版だけ。`core.event_version` の列のうち `txid` を持たず、親の `sensitivity` / `origin` / `external_id` を足す |
| `core.stay_erased`（ST22 D12） | 削除済みの滞在の `id` / `user_id` / `start_at` / `end_at` / `deleted_at` / `deleted_by`。緯度経度と `raw` を持たない |
| `stay_store::mark_late_arrivals` | **非公開**。`rebuild_day` が錠（`LOCK_KEY`）の中から 1 日の範囲で呼ぶだけ |
| 認可 | 各ハンドラが先頭で `authorize(&app, &headers)`（API の合言葉か画面のログイン）。読み出しの記録は `access_log::middleware` が新しい口にも自動で書く |
| 背景の仕事 | **サーバに 1 つも無い**（`tokio::spawn` は 0 件。`POST /stays/rebuild` も同期）。画面にもポーリングは無い |
| 環境変数 | `DATABASE_URL` / `API_TOKEN` / `WEB_PASSWORD` / `WEB_SESSION_MAX_AGE_DAYS` / `BIND`、`ASHIATO_SELFTEST_PANIC`（自己試験の口） |
| 権限 | `src/grants.sql` が core の全表に SELECT/INSERT/UPDATE/DELETE と default privileges を当てる（新しい表にも届く） |
| 画面 | `Root.tsx` の hash で行き先が決まる（`#/master` / `#/day/…` / それ以外は S-1）。S-7 はまだ無い |

**まだ main に無いもの**: ST12（`c03-*`・`core.archive_*`）、ST21（`core.place`・`s01-place`）、ST25。
下流が始まる時点で入っていれば、D8 の分類の検査が名前を挙げて落ちるので、そこで分類を足す（分類の規則は D8 にある）。

## Goals / Non-Goals

**Goals**

- 本人が消した場面を、印の有無に頼らず**時間と緯度経度**で除いて書き出す（Q5）
- 書き出しに入るもの・入らないものを**機械が持つ分類**で決め、表や列が増えたら黙って入る・黙って落ちるのではなく止まる（C4 / C6）
- 1 回の書き出しを、1 つの時点の写し・1 つのフォルダ・台帳の 2 行として閉じる（C10 / C17 / Q4）

**Non-Goals**

- **画面や端末へ渡す経路**（Q1）。ダウンロードの口・暗号化して受け取る口は作らない
- **定期の書き出し**（バックアップは ST30）、**拠点外へ送ること**（ST31。data-sensitivity「サーバは拠点外へ書き出す部品を持たない」を変えない）
- **期間・ソースで絞る口**（C9）、**読み込む口**（書き出したものを取り込み直す）
- **D-02 の原本**（C12）
- **古い書き出しを消すこと**（Q4。本人の手）
- `GET /events` の変更（C13）、`record-deletion` の要件の変更（読むだけ）

## Decisions

### D1. 口は 3 本。`POST /export`・`GET /export`・`GET /export/summary`

| 口 | 応答 |
|---|---|
| `POST /export` | `202 {"export_id": "<uuid>"}`。走っていれば `409 {"error": "export_running"}` |
| `GET /export` | `200 {"running": {"export_id", "started_at", "written", "total"} \| null, "last": {"finished_at", "rows", "bytes", "folder"} \| null, "last_failure": {"at", "kind"} \| null, "interrupted": {"started_at"} \| null}` |
| `GET /export/summary` | `200 {"sources": [{"logical_source", "display_name", "count", "first", "last"}, …]}`（件数 0 のソースは出さない） |

- 資格情報は `authorize` をそのまま使う（C11。`GET /events` と同じ扱い）。読み出しの記録は既存の層が書く
- `POST /export` は本体も問い合わせの引数も受けない。**引数（クエリ）か空でない本体が付いていれば 400**（C9。範囲を指定したつもりで全期間が出るのを防ぐ。spec）
- **応答に記録の値（原文・内容・緯度経度・本文）を載せない**（spec。ソース名・表示名・件数・時刻・`export_id`・種別は記録の値ではない）。`folder` は置き場の絶対パス（文字列）、`last_failure.kind` は種別だけ（`unclassified_table` / `missing_column` / `io` / `db` / `late_mark`）。
  分類の無い表や列の**名前**はログと台帳の `reason` に出す（識別子であって記録の値ではない）
- `last_failure` は「最後に書き終えた書き出しより新しい失敗」があるときだけ。`interrupted` は D11

> 採らなかった案: `POST /export` を同期にして、終わるまで応答を返さない。1 年目で約 1 分、3 年・上限で 20 分近く（proto の見積もり）で、
> 画面を離れると続かない（Q3「画面を離れても続く」に反する）。

### D2. 背景の仕事はサーバの中の 1 本の `tokio::spawn`。状態は `App` が持つ

`App` に `exports: ExportJobs`（`Arc<Mutex<Option<Running>>>`。`Running` は `export_id`・`started_at`・書いた行の数と書く行の数の `AtomicU64`）を足す。

- `POST /export` は錠の中で `Running` が無いことを確かめて置き、`tokio::spawn` で本体（D4〜D10）を走らせて 202 を返す。終わったら（成否どちらでも）`None` に戻す
- **同時に 1 本だけ**（spec の 409）
- テストの差し替え口: `App::for_test` に置き場（一時ディレクトリ）と、本体の段ごとに失敗を差し込む口を持たせる（`StayRebuilder::from_fn` と同じ形）。**表の権限を剥がす・名前を変えるやり方で失敗させない**（ST16 / ST22 と同じ規律）
- 自己試験の遅延: `ASHIATO_EXPORT_SELFTEST_DELAY_MS`（`ASHIATO_SELFTEST_PANIC` と同じ扱い。未設定なら 0）を、ファイル 1 つを書くごとに待つ。
  **e2e が「書き出し中」と「離れて戻る」を観測するためだけ**に `tools/stack.sh` の e2e の起動が渡す

> **（仮）**: 同時に 1 本だけ（2 本目は 409）。**反転条件** —— 押し直したい場面（走っているものを捨てて最初から）が実際に出たら、取り消しの口を足す。

### D3. 置き場は `ASHIATO_EXPORT_DIR`。既定は `%LOCALAPPDATA%\ashiato\export`（C16）

- 決め方: `ASHIATO_EXPORT_DIR` → `%LOCALAPPDATA%\ashiato\export`（Windows）→ `$XDG_DATA_HOME/ashiato/export` → `$HOME/.local/share/ashiato/export`。
  どれも無ければ書き出しを失敗（`io`）にする（起動は止めない —— 書き出しを押すまで要らない）
- **ドキュメント・デスクトップ・OneDrive の下を既定にしない**（C16。同期されれば書いた時点で外へ出る）。`ASHIATO_EXPORT_DIR` で本人が選ぶのは止めない
- 1 回ごとのフォルダ名は **`ashiato-export-YYYYMMDD-HHMMSS`**（Asia/Tokyo の写しの時点。proto の `ashiato-export-20260929-1412` に秒を足した）。同じ名前があれば `-2`, `-3` … を付ける
- 書いている間は **`.partial-ashiato-export-…`** に書き、README と目録まで書き終えてから `rename` で本来の名前にする（spec「書き終えていないフォルダは書き出しの名前を持たない」）。
  失敗したら `.partial-…` を消す（書き終えた書き出しではないので Q4 の「システムは消さない」に当たらない）。サーバが途中で止まったときの `.partial-…` は残る（D11）

> 置き場の既定と環境での指定は spec にもある（spec-review R7）。ここに残すのは決める順と Linux の代わりの場所。

> **（仮）**: フォルダ名の形と、失敗した `.partial-…` を消すこと。**反転条件** —— 名前の形は、本人が置き場を開いて読み分けにくいと言えば変える。
> 消すことは、失敗の原因を中身から調べたい場面が出たら、消さずに残して台帳の `reason` と並べる。

### D4. 写しは 1 つの `REPEATABLE READ READ ONLY` トランザクション。行は 1 行ずつ流す

```
台帳の始まりの行をコミット（D10）
遅着の印（D6。利用者ごとに別のトランザクションでコミット）—— 落ちたら late_mark で終わりの行
tx = begin isolation level repeatable read, read only
  SET LOCAL TimeZone = 'UTC'
  snapshot_at = transaction_timestamp()
  分類の検査（D8）と列の突き合わせ（D9）—— 落ちたらここで失敗
  表ごとに件数を数える → total（進み具合の分母）
  表ごとに SELECT … を fetch（stream）して、1 行ずつ BufWriter に書く → written を進める
commit
```

- **全表を同じトランザクションで読む**（C10）。取り込みは止まらない（読み取りのトランザクションは書き込みを待たせない）
- `fetch_all` を使わない（1 年で約 120 万行。`GET /events` が抱えている形を繰り返さない）
- 台帳の始まりの行（D10）は**最初に**（D6 より前に）別のトランザクションでコミットする。どの段で落ちても始まりと終わりの 2 行が残る（spec-review R13）。写しの時点は始まりの行に書けないので、始まりの行の `snapshot_at` は「写しを読み始める直前の時刻」とし、
  正確な写しの時点は目録（D7）と終わりの行に書く

### D5. 除く範囲は 1 つの SQL の述語にまとめ、書き出しと件数の両方が同じものを使う（Q5 / C1）

「本人が消した滞在」は `core.stay_erased` のうち `deleted_by IS NULL OR deleted_by NOT LIKE 'rebuild:%'`（`stay_store::Mark::of` と同じ読み方）。
**利用者を問わず**集める（spec-review R2。`user_id` は収集側ごとの設定で、サーバは一致を確かめない。書き出しは全利用者を 1 つのフォルダに書く —— D12）。

記録 `e` を**出さない**のは:

1. `core.event_live` に無い（削除の印。C1）
2. または、次の 2 つを満たす（Q5）
   - 行の時刻の幅 `[event_time, 終わり]` が、どれかの本人が消した滞在 `[s.start_at, s.end_at]` と**重なる**（端を含む）。
     終わりは内容の `end` / `end_time` / `endTime` / `endTimestamp` / `end_at` のうち最初に時刻として読めるもの（`core.try_timestamptz`）、どれも無ければ `event_time`（点）
   - 行が**緯度経度を持つ**（D5a）

前の版 `v` を出さないのは:

1. `core.event_version_live` に無い（親に削除の印）
2. または、**親 `e` が上の 2 で出ない**（spec-review R3。親が除かれた前の版は、前の版の時刻が外でも出さない —— 「親が入るものだけ」）
3. または、`v` 自身が上の 2 の条件を満たす（生きている滞在の前の版が消した時間を含む実例。レビュー R2）

この述語は Rust の 1 か所（`export::excluded_sql()` の類）が SQL の断片として持ち、本体（D4）と `GET /export/summary` が同じ断片を使う。
**件数を別の式で数えない** —— spec「書き出す前の件数は書き出した行の数と一致する」を、式が 1 つであることで担保する。

- **幅で重ねる**のは、前の版の滞在（23:00〜03:00）が消した 01:00〜02:00 を丸ごと含む実例（レビュー R2）と、書庫の訪問（`end_time`。ST12 design）を点の判定では拾えないため（spec-review R1）
- 終わりの鍵の一覧は、幅を持つソースを足す Story が自分の鍵を足す（`export::END_KEYS`）。足し忘れても点として扱われ、**始まりが消した時間に入れば除かれる**（残るのは「消した時間の前に始まり、中を通って終わる」形だけ。Risks）
- `core.stay_erased` を使うので、除く判定のために消した滞在の緯度経度を読まない（ST22 D12 の規律）
- 消した滞在の数は数十件の桁で、書き出しの記録は約 120 万行。`EXISTS (… core.stay_erased …)` を行ごとに当てる。`event_time` の索引で十分かは Task 5 の量のテスト（1 年分の偽データ）で測り、遅ければ消した区間を一時表に置く

#### D5a. 「緯度経度を持つ」の判定（spec の (1)〜(3)）

(1) **緯度経度を持たないと決まったソースの一覧**: `c01-app-usage` / `c01-app-usage-rollup` / `c01-clock` / `c02-window` / `c02-browser-history` / `s01-attribute`。
**一覧に無いソースは緯度経度を持つとみなす**（既定は厳しい側）。`c01-location` / `s01-stay` / `c01-photo` / これから入る `c03-*`・`s01-place` は一覧に入れない。
一覧は Rust の定数（`export::NO_COORD_SOURCES`）。足すのは、そのソースの payload に座標の項目が無いことを収集側の型で確かめた Story（理由を定数の隣にコメントで書く）。

(2) **内容の項目名**: `jsonb_path_exists(payload, 'strict $.**.keyvalue() ? (@.key like_regex "^(lat|lon|lng|latitude|longitude)(e7)?$" flag "i")')` の形で、**どの深さでも**見る（入れ子の経路の点を含む）。

(3) **原文の中の座標の並び**: `raw ~ '-?\d{1,2}\.\d{3,}\s*,\s*-?\d{1,3}\.\d{3,}'`（`@35.6812,139.7671`・`ll=35.68,139.76`・`q=35.68,139.76`）または
`raw ~* '!3d-?\d+\.\d+!4d-?\d+\.\d+'`（Google マップの埋め込みの形）。小数 3 桁以上を要るのは、版番号や金額の並びを拾い過ぎないため。

- (2)(3) は (1) の一覧のソースのためにある（spec-review R4。ウィンドウの題名・アドレスバーと、ブラウザの履歴の URL は、地図の URL として座標を文字列で持つ）
- **読み違えると除き過ぎる側に倒れる**（座標でない数の並びを座標と読む）。除き過ぎは書き出し直せば戻る。読み落とし（新しい座標の書き方）は戻らないので、見つけたら (3) に形を足す

> **（仮）**: (3) の並びの形（小数 3 桁以上の組と `!3d…!4d…`）。**反転条件** —— 消した時間の中のウィンドウやブラウザの行が、座標でないのに大量に除かれていると分かったら（書き出したファイルを読んで、その時間のウィンドウが欠けている）、形を狭める。
> 座標の別の書き方（度分秒・plus code など）が原文に出ると分かったら、形を足す。

### D6. 遅れて届いた位置の印（C18）は、写しの前に利用者ごとに 1 回。ST22 の判定をそのまま呼ぶ

`stay_store` に `pub async fn mark_late_for_user(pool, user) -> sqlx::Result<()>` を足す。
中身は `rebuild_day` と同じ順（トランザクション → `lock(LOCK_KEY, user)` → 基準を読む）で、
**既存の `mark_late_arrivals` を、本人が消した滞在の最小の始まり〜（最大の終わり＋1 マイクロ秒）の範囲で 1 回呼ぶ**だけ（判定を書き写さない）。
上端に 1 マイクロ秒を足すのは、`mark_late_arrivals` が候補を `event_time < 上端` で絞る（終わりを含まない）ため —— 最後の消した滞在のちょうど終わりの位置にも印を付ける（spec-review R12）。
本人が消した滞在のある利用者ごとに、写し（D4）の前に走らせる。

- 失敗したら書き出しを失敗（`late_mark`）にする。D5 の述語は印が無くても位置を除くので、ファイルに漏れることは無いが、
  spec が「印を付ける」と約束しているので黙って進まない
- `rebuild_day` の振る舞いは変えない（`record-deletion` の要件に触れない）

### D7. 形式: 表ごとの JSON Lines（Q2。仮）。1 行は SQL の `json_build_object` で作る

| ファイル | 元 | 出さない列 |
|---|---|---|
| `events.jsonl` | `core.event_live`（D5 で除いた残り） | なし（全列。C4） |
| `event_versions.jsonl` | `core.event_version_live`（D5 で除いた残り） | `txid`（DB の内部の値。ビューにも無い）。ビューが足す親の `sensitivity` / `origin` / `external_id` も出さない（表の列ではない。親は `events.jsonl` にある。spec-review R10） |
| `sources.jsonl` | `core.source` | なし |
| `attribute_kinds.jsonl` / `attribute_kind_names.jsonl` | `core.attribute_kind` / `core.attribute_kind_name` | なし |
| `stay_criteria.jsonl` | `core.stay_criteria` | なし |
| `heartbeats.jsonl` | `core.heartbeat` | `attempts` / `successes` / `raw` / `content_hash`（C15。`content_hash` は回数を含む原文のハッシュなので一緒に外す） |
| `coverage_spans.jsonl` | `core.coverage_span` | なし |
| `drop_reports.jsonl` / `drop_report_hours.jsonl` | `core.drop_report` / `core.drop_report_hour` | なし |
| `README.txt` | 説明書（D9） | —— |
| `manifest.json` | 目録（D9） | —— |

- 1 行は `SELECT json_build_object('id', id, 'raw', raw, 'event_time', <時刻>, …)::text` のように、**列の一覧（D8 の分類）から SQL を組み立てる**。
  `json_build_object` は鍵の順を保ち、`raw`（`text`）は JSON の**文字列**として逐語で入る（C4。`::jsonb` に通さない）。`payload`（`jsonb`）は JSON の値として入る
- 時刻（`timestamptz`）は `to_char(col AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS.US"Z"')`（C14。マイクロ秒まで、`Z` で終える）。`date` は `YYYY-MM-DD`
- 並びは `event_time, id`（記録・前の版）、それ以外は主キー。読む側が `head` で見て古い順に並ぶ
- 行の終わりは `\n`、文字コードは UTF-8（BOM 無し）

> **（仮）**: JSON Lines（Q2。本人が推奨を選んだ B）。**反転条件** —— 外部の道具（表計算など）で開けないことが実際に困ったら、
> 同じ分類から CSV を足す（JSON Lines は残す。原文を逐語で持てるのは JSON の文字列のほう）。

### D8. 書き出しの分類の正本は Rust の定数 1 つ。表と列の両方を持つ

`export::CATALOG`: core の**すべての表**について `Include { file, view, excluded_columns, columns: [(name, 意味)] }` か `Exclude { reason }` のどちらか。

- `Exclude` にするもの: `coverage`（削除済みも数える）/ `erasure_ledger` / `deletion_ledger` / `export_ledger`（D10）/ `access_log` / `web_session` / `stay_absorbed`（C6）
- 書き出しの始めに `information_schema.tables`（core・BASE TABLE）と `information_schema.columns` を読み、
  **分類の無い表・`Include` の表で `columns` にも `excluded_columns` にも無い列**があれば失敗（spec）
- 列の意味（`columns` の 2 つ目）が README の列の説明になる（D9）。**分類と説明書を 2 か所で持たない**
- 後から表や列を足す Story は、ここに 1 行足さないと書き出しのテスト（Task 2）が落ちる —— それが狙い（「黙って入る / 黙って落ちる」を止める）
- ビューは分類の対象外（`events` / `event_versions` の `view` として名前で持つ）

### D9. 列の突き合わせ（C4 / R12）と説明書

**突き合わせ**: `events` は `core.event` の列のうち出す列が `core.event_live` に全部あるか、`event_versions` は `core.event_version` の出す列が `core.event_version_live` に全部あるかを、
D8 と同じトランザクションで `information_schema.columns` から確かめる。欠けていたら失敗（`missing_column`。理由に列の名前）。

**説明書 `README.txt`**（日本語。外の道具なしで読める）:

1. これは何か・写しの時点・書き出した時刻・記録の形の版（`MIGRATIONS` の最後の名前）
2. ファイルごとに: 名前・元・行の数・列（名前と意味。D8）
3. 入らないもの: 本人が消した場面（削除の印の付いた記録と、消した滞在の時間に入る緯度経度を持つ行。前の版を含む）・D8 の `Exclude` の表・D7 の出さない列・D-02 の原本。
   **除いた件数は書かない**（C8）
4. 同じ内容の行を 1 件として読む規則（C3）: `core.event_folded`（`202609120943_version_and_ledger.sql`）と同じ ——
   `user_id`・`logical_source`・`content_hash` が同じ行を 1 件とみなし、時刻はいちばん早い `event_time`、原文・内容・由来は `ingest_time, id` の順で最初の行、
   感度はいちばん厳しい（大きい）値、外部の識別子は全部を並べて持つ。**文面はビューの定義から起こし、ビューが変わったら説明書の文面を合わせる**（Task 5 のテストが両方を突き合わせる）
5. 時刻の書き方（UTC・`Z`）と、ずれ・タイムゾーンの列の読み方

**目録 `manifest.json`**: `{"format": 1, "snapshot_at", "exported_at", "schema_version", "files": [{"name", "source", "rows", "bytes", "columns": [{"name", "meaning"}]}], "not_included": [...], "fold_rule": "..."}`。
`rows` は書いた行を数えた値（数え直しではなく書いた数）。

### D10. 書き出しの台帳 `core.export_ledger`。1 回 2 行（始まりと終わり）、追記のみ

```sql
CREATE TABLE core.export_ledger (
  seq          bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  export_id    uuid NOT NULL,
  kind         text NOT NULL CHECK (kind IN ('start', 'finish')),
  at           timestamptz NOT NULL DEFAULT now(),
  route        text NOT NULL CHECK (route IN ('pc-folder')),   -- Q1 の経路
  folder       text NOT NULL,                                  -- 書き終えた名前（start の時点で決めた名前）
  snapshot_at  timestamptz,                                    -- start: 読み始める直前 / finish: 写しの時点
  result       text CHECK (result IN ('done', 'failed')),      -- finish だけ
  rows         jsonb,                                          -- finish: {"events.jsonl": n, …}
  bytes        bigint,                                         -- finish: フォルダの中のファイルの大きさの和
  reason       text,                                           -- failed: 種別と、分類の無い表・欠けた列の名前（記録の値は持たない）
  CHECK ((kind = 'start') = (result IS NULL))
);
CREATE INDEX export_ledger_by_export ON core.export_ledger (export_id, seq);
```

UPDATE / DELETE / TRUNCATE を拒むトリガを**この移行専用の関数**（`core.reject_export_ledger_change()`）で置く（ST22 D1 と同じ理由）。

- **C17 は「1 回 1 行」と書いたが、2 行にした。** 始まりの行を先に書かないと、書き出しの途中でサーバが止まったときに「どの時点の写しがどこに書かれかけたか」が残らない（C17 の理由そのもの）。
  1 行で持つには行を書き換えることになり、追記のみと両立しない
- `user_id` を持たない —— 書き出しは全利用者の D-01 を 1 つにまとめる（D12）
- 稼働状況・読み出しの口のどこにも出さない（D8 で `Exclude`）

### D11. 状態（`GET /export`）は「走っているもの」をメモリから、それ以外を台帳から読む

- `running`: `App.exports` にあれば
- `last`: 台帳の最後の `finish` かつ `result = 'done'` の行
- `last_failure`: `last` より新しい `finish` かつ `failed` の行（種別だけ）
- `interrupted`: 最後の `start` に対応する `finish` が無く、`running` でもない（サーバが途中で止まった）。新しい書き出しはそのまま受け付ける（spec）。
  残った `.partial-…` は消さない（中身を調べる手がかりで、書き出しの名前ではない）
- **書き出した後に消した記録の件数は数えない**（Q3「知らせない」）。台帳の `snapshot_at` と削除の台帳から後で作れる状態は保つ（C17）

### D12. 全利用者を 1 つのフォルダに書く

D-01 の全記録（NFR-16）なので利用者で分けない。各行が `user_id` を持つ。除く判定（D5）は**利用者を問わない**（消した時間に入る緯度経度つきの行は誰の識別子でも除く）。遅着の印（D6）は ST22 の判定が利用者ごとなので利用者ごと。
件数（`GET /export/summary`）も全利用者の和をソースごとに返す。

### D13. 画面は `#/settings` の `SettingsView.tsx`。押せば確認なしで始まり、走っている間は 1 秒ごとに状態を読む

- 行き先: `Root.tsx` に `#/settings` を足す。入口は**S-1 の見出しの行**（`#/day/` と `#/master` と並べて「設定」）。
  ui-direction の地図は S-2 → S-7 だが、入口を決め直すのは ST25 なので、ST25 が入口を付け替えるときに S-7 へのリンクも運ぶ（`docs/screens.md` に 1 行足す）
- 節の並び: 見出し「設定」と「稼働状況へ」→ 節「全記録の書き出し」（説明 1 行・ソースごとの件数と期間・「全記録を書き出す」）→ 節「前回の書き出し」→ 後続の節の予告 1 行。**proto の初期値の形のまま**（Q3）
- 開いたときに `GET /export/summary` と `GET /export` を 1 回ずつ。`running` があれば 1 秒ごとに `GET /export` を読み直し、無くなったら止める（画面を離れると止まり、戻ると開いたときの読み出しで再開する）
- 失敗は節の中に 1 行（「書き出しに失敗しました（<種別>）」）、途中で止まったものも 1 行（「前の書き出しは途中で止まりました（<始めた時刻>）」）。どちらも「全記録を書き出す」を押し直せる（spec）
- `MIN_TARGET_PX`（24）・ライト / ダークの `tokens.ts`・フォーカスの輪郭は既存の画面と同じものを使う。置き場のパスは `word-break: break-all`（360 px で横にはみ出さない）
- ダウンロード・保存・`file://` のリンクを置かない（Q1）

> **（仮）**: 1 秒ごとに読む間隔と、件数を開くたびに数える形（D1 の `GET /export/summary`）。**反転条件** —— 件数の数えに 3 秒以上かかる量になったら（書庫で百万件級を入れた後など）、
> 件数を前回の書き出しの目録から出して「いまの件数」は押したときにだけ数える。間隔は、読み出しの記録（access_log）が 1 回の書き出しで数百行増えるのが困るなら伸ばす。

### D14. 変えないもの

| もの | なぜ |
|---|---|
| `GET /events` | C13。ST01 の読み出し経路 |
| `record-deletion` の要件・`rebuild_day` の振る舞い | 読むだけ（D6 は既存の判定を呼ぶ口を足すだけ） |
| `data-sensitivity` の要件 | 拠点外へ送る部品を足さない（ファイルはローカル）。`tools/check-offsite.sh` が見る一覧に当たる crate を足さない |
| `core.event_live` / `core.event_version_live` の定義 | 書き出しは読むだけ。列が欠けたら直すのは列を足した Story（D9 が落ちる） |

### D15. ログ

`kind = "export"` に `export_id`・段（`late_mark` / `catalog` / `columns` / `write` / `finish`）・件数・大きさ・種別だけ。
**記録の値・置き場のパスの中の利用者名は出さない**（製造準備 A-2）—— パスは応答と台帳には出すが、ログには `folder` の最後の名前だけを出す。

### D16（仮）. 前の書き出しは毎回別のフォルダに残し、システムは消さない（Q4）

本人が推奨（置き換える）を覆して選んだ B。D3 の 1 回 1 フォルダと、どこにも「古いものを消す」処理を置かないことで実現する。

**反転条件** —— 置き場の容量が実際に困ったら（上限の 3 年で 1 回 17 GB 前後）、古いものを消す口を足す（そのときも消すのは本人の操作で、自動にはしない）。

## Risks / Trade-offs

- **一覧（D5a）にあるソースが、(2)(3) で読めない形で座標を持つと、消した時間の中でも出る** → (2) はどの深さの項目名も見る。(3) の形は見つけたら足す。一覧に無いソースは除く側なので、新しいソースの既定は漏れない
- **終わりの鍵（D5）を足し忘れた幅を持つソースは、消した時間の前に始まって中を通る行が出る** → 幅を持つソースを足す Story が `END_KEYS` に足す。ST12 の `end_time` はいま足してある
- **除き過ぎ**（緯度経度を持つが場面とは関係の薄い行。例: 消した時間に登録した場所）→ 出さない側。後で範囲を狭めて書き出し直せる（ファイルは D-01 から作り直せる）
- **長い読み取りのトランザクション**（1 年目で約 1 分、上限なら 20 分近く）→ その間 VACUUM が古い行を片付けられない。取り込みは止まらない。押したときだけ走るので常時ではない
- **書き出した後に消した場面は、前のフォルダに残る**（Q4 で本人が選んだ帰結。Q3 で知らせないことも選んだ）→ 台帳の `snapshot_at` と削除の台帳から後で突き合わせられる
- **置き場が同期されるフォルダだと、書いた時点で外へ出る**（C16）→ 既定は `%LOCALAPPDATA%`。本人が `ASHIATO_EXPORT_DIR` で変えるのは止めない
- **ディスクが足りない** → `io` で失敗、`.partial-…` を消す。前の書き出しには触らない
- **サーバが止まると `.partial-…` が残る** → 状態に `interrupted` と出る。消すのは本人

## Migration Plan

1. 移行 1 本（`YYYYMMDDHHMM_export_ledger.sql` と `.down.sql`）を当てる。既存の行に触らない
2. サーバを入れ替える（口 3 本・背景の仕事・`stay_store::mark_late_for_user`）
3. 画面を入れ替える（`#/settings` と S-1 のリンク）
4. 巻き戻し: `.down.sql` で `core.export_ledger` とトリガと関数を落とす。**書き出したフォルダは残る**（システムの外にある）。台帳が消えるので、どの書き出しが何時点の写しかは各フォルダの目録で読む

## Open Questions

無し。（仮）決めは D2 / D3 / D5a / D7 / D13 / D16 の 6 件で、反転条件はそれぞれの項に書いた。
