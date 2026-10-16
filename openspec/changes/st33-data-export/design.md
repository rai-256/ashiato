# ST33 設計 — 全記録を外部の道具で読める形に書き出す

## Context

動機は `proposal.md` の Why。本人が決めた 9 件（第 1 回 Q1〜Q5・第 2 回 Q6〜Q8・第 3 回 Q9。**Q3 と Q4 は第 2 回の貼り戻しの答えがいまの答え**）と、聞かずに決めた既定 C1〜C25 は `deep.md`。
要件は NFR-16（★ 2026-09-29 訂正・★ 2026-10-12 追加。**Q9: 押すときに本人が入れる合言葉で暗号化した 1 つのファイル。合言葉は保存しない**）。
観測可能な振る舞いは `specs/data-durability/spec.md` に置いた。**ここで決めるのは、それを実装に落とすときの技術判断だけ。**

読む前提になる既存の形（main・ST22 と ST12 の archive 後。2026-10-13 の `a8d13ebb`）:

| 場所 | いまの形 |
|---|---|
| `GET /events`（`lib.rs` の `events`） | `core.event_live` の 6 列を `fetch_all` で全件返す。**変えない**（C13。ST01 の読み出し経路） |
| `core.event_live` | `SELECT * FROM core.event WHERE deleted_at IS NULL`（最新は `202609120941_event_columns.sql`）。**作った時点で列が固まる** |
| `core.event_version_live` | 親が生きている版だけ。`core.event_version` の列のうち `txid` を持たず、親の `sensitivity` / `origin` / `external_id` を足す |
| `core.stay_erased`（ST22 D12） | 削除済みの滞在の `id` / `user_id` / `start_at` / `end_at` / `deleted_at` / `deleted_by`。緯度経度と `raw` を持たない |
| `stay_store::mark_late_arrivals` | **非公開**。`mark_late_arrivals(tx, user, sources: &[String], (from, to))`。`rebuild_day` が錠（`LOCK_KEY`）の中から 1 日の範囲で、`with_archive_sources(&c.sources)`（基準のソースに書庫の位置 7 本を足した並び）を渡して呼ぶ。中で位置の欄を持つ書庫の項目（`item_sources`）も足す |
| 削除の連鎖（ST22 + ST12） | `deletion::erase` は基準のソース・書庫の位置 7 本（`archive::LOCATION_SOURCES`。区間の 4 本は `INTERVAL_SOURCES` で終わりまで見る）・位置の欄（`locationInfos`）を持つ書庫の項目に印を付ける。**写真の位置・URL にだけ座標を持つ書庫の項目・ウィンドウとブラウザの履歴には付かない**（FR-50 ★ 2026-10-13） |
| 書庫の終わりの読み方 | `stay_store::event_end_sql`: `end_time` を `core.try_timestamptz` で読み、数字だけ（`^[0-9]{1,15}$`）ならミリ秒の UNIX 時刻（移行前の訪問・移動の `endTimestampMs`） |
| `zip` crate | `zip = "2"`（2.4.2）が既にある（ST12 が書庫を開くのに使う）。既定の機能に `aes-crypto` があり、`FileOptions::with_aes_encryption(AesMode::Aes256, …)` で書ける |
| 認可 | 各ハンドラが先頭で `authorize(&app, &headers)`（API の合言葉か画面のログイン）。読み出しの記録は `access_log::middleware` が新しい口にも自動で書く |
| 背景の仕事 | **サーバに 1 つも無い**（`tokio::spawn` は 0 件。`POST /stays/rebuild` も同期）。画面にもポーリングは無い |
| 環境変数 | `DATABASE_URL` / `API_TOKEN` / `WEB_PASSWORD` / `WEB_SESSION_MAX_AGE_DAYS` / `BIND`、`ASHIATO_SELFTEST_PANIC`（自己試験の口） |
| 権限 | `src/grants.sql` が core の全表に SELECT/INSERT/UPDATE/DELETE と default privileges を当てる（新しい表にも届く） |
| 画面 | `Root.tsx` の hash で行き先が決まる（`#/master` / `#/day/…` / それ以外は S-1）。S-7 はまだ無い |

**main に入ったもの**: ST12（`c03-*` の論理ソースと `core.archive_*` の 8 表。2026-10-13 に archive）。分類は D8 に足した。
**まだ main に無いもの**: ST21（`core.place`・`s01-place`）、ST17（`s01-feeling`）、ST25。
下流が始まる時点で入っていれば、D8 の分類の検査が名前を挙げて落ちるので、そこで分類を足す（分類の規則は D8 にある。`s01-feeling` の扱いは `docs/handoff/ST33.md` の C1）。
★ 2026-10-16（下流・merge gate の rebase）: ST21 が main に入り、分類の検査が `core.place` と `s01-place` を名前で挙げて落ちた。`core.place` は `Include`（`places.jsonl`。D7）、`s01-place` は緯度経度を持つ側（D5a。payload に `lat` / `lon`）・幅は既定の (c)（D5b。1 点と確かめていない）に置いた。
★ 2026-10-17（下流・merge gate の rebase）: ST17・ST14 が main に入り、分類の検査が `core.feeling_target_kind`・`core.feeling_reminder`・`core.outage_ledger` と `core.source` の 3 列（`outage_target`・`alert_after_days`・`started_on_from`）を名前で挙げて落ちた。`feeling_target_kind`（紐づけ先の種類の登録簿）は `attribute_kind` と同じく、`feeling_reminder`（本人が決めた通知の時刻）は `stay_criteria` と同じく `Include`（D7 の `feeling_target_kinds.jsonl`・`feeling_reminders.jsonl`）、`outage_ledger`（途切れの判定と通知の台帳。稼働の信号からの導出）は `coverage` と同じく `Exclude`、`source` の 3 列は `sources.jsonl` に出す。`s01-feeling` は handoff C1 のとおり緯度経度を持たない側（D5a。内容は尺度・紐づけ先・ひとことだけ）に置いた。幅は紐づけ先で読む規則を足さず既定の (c)（D5b）のまま —— 緯度経度を持たない側なので幅が効くのはひとことに座標の並びを書いた行だけで、(c) は除き過ぎの側（C21。書き出し直せば戻る）。

## Goals / Non-Goals

**Goals**

- 本人が消した場面を、印の有無に頼らず**時間と緯度経度**で除いて書き出す（Q5。時間は幅で重ねる Q6、利用者を問わない Q7、URL や題名の中の座標も数える Q8）
- 書き出しに入るもの・入らないものを**機械が持つ分類**で決め、表や列が増えたら黙って入る・黙って落ちるのではなく止まる（C4 / C6）
- 1 回の書き出しを、1 つの時点の写し・**合言葉で暗号化した 1 つの zip**・台帳の 2 行として閉じ、書き終えたら前のファイルを消して置き場に 1 つだけ残す（C10 / C17 / Q4 / Q9）
- 合言葉を、書き出しの仕事が終わるまでのメモリの他のどこにも置かない（Q9）。**このシステムが書くファイル**（置き場と一時フォルダ）に、暗号化しない記録の値を書かない（C24。DB の一時ファイルは Risks）

**Non-Goals**

- **画面や端末へ渡す経路**（Q1）。ダウンロードの口は作らない（暗号化した zip も PC の置き場にだけ置く）
- **合言葉を覚えておくこと・思い出させること**（Q9。忘れたら書き出し直す）。合言葉の強さを測ること（下限の文字数だけを見る。C23）
- **定期の書き出し**（バックアップは ST30）、**拠点外へ送ること**（ST31。data-sensitivity「サーバは拠点外へ書き出す部品を持たない」を変えない）
- **期間・ソースで絞る口**（C9）、**読み込む口**（書き出したものを取り込み直す）
- **D-02 の原本**（C12）
- **本人が置いた・写した・置き換えたファイルを消すこと**（消すのは台帳が「このシステムが書いた」と記録したパスで、zip の注記の `export_id` が台帳と合うものだけ。D16）
- `GET /events` の変更（C13）、`record-deletion` の要件の変更（読むだけ）

## Decisions

### D1. 口は 3 本。`POST /export`・`GET /export`・`GET /export/summary`

| 口 | 応答 |
|---|---|
| `POST /export` | 本体 `{"password": "<合言葉>"}`。`202 {"export_id": "<uuid>"}`。走っていれば `409 {"error": "export_running"}`。合言葉が無い・12 文字未満は `400 {"error": "password_missing" \| "password_too_short"}`、`password` 以外の鍵か問い合わせの引数があれば `400 {"error": "unexpected_field"}` |
| `GET /export` | `200 {"running": {"export_id", "started_at", "written", "total"} \| null, "last": {"finished_at", "rows", "bytes", "file", "erased_after"} \| null, "last_failure": {"at", "kind"} \| null, "interrupted": {"started_at", "file"?} \| null, "leftover": ["<file>", …]}` |
| `GET /export/summary` | `200 {"sources": [{"logical_source", "display_name", "count", "first", "last"}, …], "estimate": {"rows", "bytes", "seconds"}}`（件数 0 のソースは出さない。見込みは D18） |

- 資格情報は `authorize` をそのまま使う（C11。`GET /events` と同じ扱い）。読み出しの記録は既存の層が書く
- `POST /export` が受けるのは本体の `password` 1 つだけ。**問い合わせの引数か、本体に `password` 以外の鍵があれば 400**（C9。範囲を指定したつもりで全期間が出るのを防ぐ。spec）。
  文字数は Unicode の符号位置（スカラー値）で数える（`chars().count()`。画面も同じ数え方 —— D13）。前後の空白を削らない（本人が打ったものがそのまま鍵になる）
- **合言葉は応答・ログ・台帳・読み出しの記録に出さない**（Q9）。`access_log` は本体を読まない（`access_log.rs` の規律のまま）。本体を読む抽出器の拒否の文言に値を入れない（axum の `Json` の既定の拒否は本体の断片を出しうるので、自前で読んで種別だけを返す）
- **応答に記録の値（原文・内容・緯度経度・本文）を載せない**（spec。ソース名・表示名・件数・時刻・`export_id`・種別は記録の値ではない）。`file` は zip の絶対パス（文字列）、`last_failure.kind` は種別だけ（`unclassified_table` / `missing_column` / `io` / `db` / `late_mark`）。
  分類の無い表や列の**名前**はログと台帳の `reason` に出す（識別子であって記録の値ではない）
- `last_failure` は「最後に書き終えた書き出しより新しい失敗」があるときだけ。`interrupted` と `leftover` は D11、`erased_after`（後に消した記録の件数）は D17
- **件数を返すのは `erased_after` だけ**（Q3 第 2 回）。写しの時点で除いた行の件数は返さない（C8。spec）

> 採らなかった案: `POST /export` を同期にして、終わるまで応答を返さない。1 年目で約 1 分、3 年・上限で 20 分近く（proto の見積もり）で、
> 画面を離れると続かない（Q3「画面を離れても続く」に反する）。

### D2. 背景の仕事はサーバの中の 1 本の `tokio::spawn`。状態は `App` が持つ

`App` に `exports: ExportJobs`（`Arc<Mutex<Option<Running>>>`。`Running` は `export_id`・`started_at`・書いた行の数と書く行の数の `AtomicU64`）を足す。

- `POST /export` は錠の中で `Running` が無いことを確かめて置き、`tokio::spawn` で本体（D4〜D10）を走らせて 202 を返す。終わったら（成否どちらでも）`None` に戻す
- **合言葉は spawn した本体の引数にだけ渡す**（`Running` にも `App` にも置かない）。`zeroize::Zeroizing<String>` に包み、本体が終わると上書きして捨てる（`zeroize` は既に依存の木にある。直接の依存に足す。拠点外へ送る部品ではないので `tools/check-offsite.sh` の一覧に当たらない）
- **同時に 1 本だけ**（spec の 409）
- テストの差し替え口: `App::for_test` に置き場（一時ディレクトリ）と、本体の段ごとに失敗を差し込む口を持たせる（`StayRebuilder::from_fn` と同じ形）。**表の権限を剥がす・名前を変えるやり方で失敗させない**（ST16 / ST22 と同じ規律）
- 自己試験の遅延: `ASHIATO_EXPORT_SELFTEST_DELAY_MS`（`ASHIATO_SELFTEST_PANIC` と同じ扱い。未設定なら 0）を、zip の中のファイル 1 つを書くごとに待つ。
  **e2e が「書き出し中」と「離れて戻る」を観測するためだけ**に `tools/stack.sh` の e2e の起動が渡す

> **（仮）**: 同時に 1 本だけ（2 本目は 409）。**反転条件** —— 押し直したい場面（走っているものを捨てて最初から）が実際に出たら、取り消しの口を足す。

### D3. 置き場は `ASHIATO_EXPORT_DIR`。既定は `%LOCALAPPDATA%\ashiato\export`（C16）。1 回 1 つの zip

- 決め方: `ASHIATO_EXPORT_DIR` → `%LOCALAPPDATA%\ashiato\export`（Windows）→ `$XDG_DATA_HOME/ashiato/export` → `$HOME/.local/share/ashiato/export`。
  どれも無ければ書き出しを失敗（`io`）にする（起動は止めない —— 書き出しを押すまで要らない）
- **ドキュメント・デスクトップ・OneDrive の下を既定にしない**（C16。同期されれば書いた時点で外へ出る）。`ASHIATO_EXPORT_DIR` で本人が選ぶのは止めない
- 1 回ごとのファイル名は **`ashiato-export-YYYYMMDD-HHMMSS.zip`**（Asia/Tokyo の、台帳の始まりの行を書く時点。写しの時点はその後に決まるので名前には使わない —— spec-review R46。proto の `ashiato-export-20260929-1412` に秒を足した）。同じ名前があれば `-2`, `-3` … を付ける。
  `std::fs::rename` は同じ名前のファイルを置き換えるので、`rename` の直前に同じ名前が無いことを確かめ直し、あれば次の番号の名前にする。実際に付けた名前は終わりの行の `file` に書く（D10。始まりの行と違ってよい）
- 書いている間は **`.partial-ashiato-export-….zip`** に書き、README と目録まで書いて zip を閉じてから `rename` で本来の名前にする（spec「書き終えていないファイルは書き出しの名前を持たない」）。
  実装の名前は **`.partial-ashiato-export-<YYYYMMDD-HHMMSS>-<8 桁の 16 進>.zip`**（16 進は書き出しごとの乱数。同じ秒に始まった 2 つの書きかけが同じ名前を取り合わないため。final review R8）
  `rename` の後で前の書き出しのファイルを消す（D16）。失敗したら `.partial-…` を消し、前のファイルには触らない。サーバが途中で止まったときの `.partial-…` は残る（D11。暗号化されているので中身は読めない）
- **権限**（C25。`docs/handoff/ST33.md` の C3）: Unix では、置き場が無ければ `DirBuilder::mode(0o700)` で作り、zip は `OpenOptions::mode(0o600)` で作る（`migrate_backup.rs` の退避と同じ形）。
  `ASHIATO_EXPORT_DIR` で本人が指した**既にある**フォルダの権限は変えない（本人の持ち物）。ファイルは常に 0600。Windows は利用者のプロファイルの既定の ACL に任せる（`%LOCALAPPDATA%`）

> 置き場の既定と環境での指定は spec にもある（spec-review R7）。ここに残すのは決める順と Linux の代わりの場所。

> **（仮）**: ファイル名の形と、失敗した `.partial-…` を消すこと。**反転条件** —— 名前の形は、本人が置き場を開いて読み分けにくいと言えば変える。
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
  zip = ZipWriter::new(.partial-….zip を 0600 で開いた File)
  表ごとに zip.start_file(<名前>, AES-256・Deflate・large_file) → SELECT … を fetch（stream）して 1 行ずつ書く → written を進める
  README.txt と manifest.json も同じ形で書く
commit
zip.set_comment("ashiato-export <export_id>") → zip.finish() → fsync → rename
```

- **全表を同じトランザクションで読む**（C10）。取り込みは止まらない（読み取りのトランザクションは書き込みを待たせない）
- `fetch_all` を使わない（1 年で約 120 万行。`GET /events` が抱えている形を繰り返さない）
- **行は暗号化の層に直に流す**（C24）。表ごとの平文の一時ファイルを作らない。zip の項目の大きさは閉じるときに書くので、行の数を先に知る必要は無い（目録の `rows` は書いた数）
- 台帳の始まりの行（D10）は**最初に**（D6 より前に）別のトランザクションでコミットする。どの段で落ちても始まりと終わりの 2 行が残る（spec-review R13）。写しの時点は始まりの行に書けないので、始まりの行の `snapshot_at` は「写しを読み始める直前の時刻」とし、
  正確な写しの時点は目録（D7）と終わりの行に書く

### D5. 除く範囲は 1 つの SQL の述語にまとめ、書き出しと件数の両方が同じものを使う（Q5〜Q8 / C1 / C19 / C21）

「本人が消した滞在」は `core.stay_erased` のうち `deleted_by IS NULL OR deleted_by NOT LIKE 'rebuild:%'`（`stay_store::Mark::of` と同じ読み方）。
**利用者を問わず**集める（Q7。`user_id` は収集側ごとの設定で、サーバは一致を確かめない。書き出しは全利用者を 1 つのファイルに書く —— D12）。

**消した時間 `erased_spans`**（C19。GO 資料の未解決 6）: 本人が消した滞在 `s` ごとに

- `[s.start_at, s.end_at]`（いまの版）と、
- `core.event_version` のうち `event_id = s.id` の各版 `v` の `[v.event_time, coalesce(core.try_timestamptz(v.payload->>'end'), v.event_time)]`（前の版。作り直しで縮む前の広い時間）

を合わせたもの。読むのは時刻の項目だけで、消した滞在の緯度経度と `raw` は読まない（ST22 D12 の規律）。素の `core.event_version` を読むのはこの時刻の 2 つだけ（親が消えた版は `core.event_version_live` に無いため）。

記録 `e` を**出さない**のは:

1. `core.event_live` に無い（削除の印。C1）
2. または、次の 2 つを満たす（Q5）
   - 行の時刻の幅 `[event_time, 終わり]`（D5b）が、`erased_spans` のどれかと**重なる**（端を含む。Q6）
   - 行が**緯度経度を持つ**（D5a）

前の版 `v` を出さないのは:

1. `core.event_version_live` に無い（親に削除の印）
2. または、**親 `e` が上の 2 で出ない**（spec-review R3。親が除かれた前の版は、前の版の時刻が外でも出さない —— 「親が入るものだけ」）
3. または、`v` 自身が上の 2 の条件を満たす（生きている滞在の前の版が消した時間を含む実例。レビュー R2）。`v` の幅は D5b を `v` の `payload` と `event_time` で読む。
   (c) の次の記録は「`v.event_time` より後で、親と同じ `user_id`・`logical_source`・`device_id`（`core.event_version` は `device_id` を持たないので親から取る）を持つ `core.event_live` の最初の記録の時刻。無ければ写しの時点」（spec-review R33）

この述語は Rust の 1 か所（`export::excluded_sql()` の類）が SQL の断片として持ち、本体（D4）・`GET /export/summary`・後に消した記録の件数（D17）が同じ断片を使う。
**件数を別の式で数えない** —— spec「書き出す前の件数は書き出した行の数と一致する」を、式が 1 つであることで担保する。

- `core.stay_erased` を使うので、除く判定のために消した滞在の緯度経度を読まない（ST22 D12 の規律）
- 消した滞在の数は数十件の桁で、書き出しの記録は約 120 万行。`erased_spans` は各問い合わせの `WITH erased_spans AS MATERIALIZED (…)` に置き、`EXISTS` を行ごとに当てる。
  **一時表にしない** —— 写し（D4）は `READ ONLY` のトランザクションで、`CREATE TEMP TABLE` は `cannot execute CREATE TABLE in a read-only transaction` で落ちる（spec-review R22）。
  (c) の次の記録は `core.event_live` の上で、同じ `user_id`・`logical_source`・`device_id` の中で `(event_time, id)` が次の行を引く（削除済みの行を間に挟まない —— 幅が長くなる側。C21）。
  **実装は `lead()` の窓関数ではなく、行ごとの相関副問い合わせ（`ORDER BY event_time, id LIMIT 1` の LATERAL）**（final review R8 / Task 3 F3）。並びと同じ時刻の決め方は `lead(event_time) OVER (PARTITION BY user_id, logical_source, device_id ORDER BY event_time, id)` と同じで、結果は変わらない。
  替えた理由: `lead()` は窓を作るために `core.event_live` の全行を並べ替えてから絞ることになるが、相関副問い合わせなら「消した時間の終わりより前に始まり、緯度経度を持つ行」に先に絞ってから、その行にだけ次の記録を引ける。
  前の版の次の記録も `core.event_live` から引く（前の版どうしの窓ではない）ので、同じ形で書ける。
  その代わり、消した滞在が最近だと絞り込みがほぼ全行を通し、座標を持つ行の数だけ索引を引く（Task 5 の測りで座標つき 1 行約 8ms。量は R4 の 35 万行の測りを見る）。
  かかる時間は Task 5 の量のテスト（1 年分の偽データ）で測る

#### D5b. 時刻の幅の読み方（spec の (a)〜(d)。Q6 / C21）

`export::SPAN_RULES`（ソースごとの読み方。Rust の定数）と、どのソースにも当てる鍵の一覧で決める。

| 順 | 読み方 | 鍵・ソース |
|---|---|---|
| (a) 終わりの時刻 | 内容の鍵のうち最初に読めるもの。`core.try_timestamptz` で読み、`end_time` が数字だけ（`^[0-9]{1,15}$`）ならミリ秒の UNIX 時刻として読む（`stay_store::event_end_sql` と同じ。移行前の書庫の訪問・移動）。**読めた終わりが `event_time` より前なら読めないものとして (b) へ** | `export::END_KEYS` = `end` / `end_time` / `endTime` / `endTimestamp` / `end_at` / `range_end`（`c02-window` の `idle` と `powered-off`。`crates/collector-windows/src/contract.rs` の `range_end`） |
| (b) 長さ | 内容の数の鍵 × 単位を `event_time` に足す | `export::DURATION_KEYS` = `visit_duration_us`（µs。ST08 の Chromium の訪問）/ `duration_ms`（ms）/ `duration_s`（s）。**`elapsed_ms` は入れない**（携帯の時計の記録では起動からの経過で、続いた時間ではない） |
| (c) 次の記録まで | 同じ `user_id`・`logical_source`・`device_id` の次の記録の `event_time`。無ければ写しの時点 | `SPAN_RULES` で「時刻 1 点」と決まっていない**すべてのソース**（既定）。`c02-window` の前景・`c01-app-usage`（前景の切り替わりで、次の変化まで続く。spec-review R34）・Firefox の訪問（長さを持たない）・登録簿に新しく足したソース |
| (d) 時刻 1 点 | `event_time` だけ | `SPAN_RULES` の `Point`: `c01-location` / `c01-photo` / `c01-clock` / `s01-attribute`（本人が書いた時点） |

- **既定は (c)**（C21。既定は厳しい側）。「時刻 1 点」へ移すのは、そのソースの記録が出来事 1 点であることを収集側の型で確かめた Story（理由を定数の隣にコメントで書く）。
  (d) に入れたソースも (a)(b) の鍵を持てば (a)(b) が先に効く（順は (a) → (b) → (c)/(d)）
- `c01-app-usage-rollup` と `s01-stay` は `end` を持つので (a)。ST12 の区間のソース（`archive::INTERVAL_SOURCES` の 4 本）は `end_time` で (a)（ミリ秒の数字も読む。整合の確認 2026-10-13 の K1）。
  それ以外の書庫のソース（位置の点 3 本・項目）は既定の (c)。ST12 は archive 済みで `SPAN_RULES` に足す係がいないので、**(d) に移すかは ST33 の下流が決めてよい** ——
  ST12 の解析器の型（`archive/timeline.rs` ほか）で出来事 1 点と確かめたソースだけを `Point` に足し、理由を定数の隣に書く。足さなければ (c) のまま（除き過ぎの側）
- 幅を持つソースを足す Story は、終わりや長さの鍵が一覧に無ければ足す。足し忘れても (c) で次の記録までの幅になり、**点として抜けることは無い**（Risks の除き過ぎ）
- (c) と決めたソースも `export::SPAN_NEXT_SOURCES` に名前で置く（SQL は読まない —— 一覧に無いソースは (c) の既定のまま）。登録簿の全ソースが `SPAN_RULES` の `Point` か `SPAN_NEXT_SOURCES` のちょうど一方にあることを、
  全移行を当てたまっさらな DB の `core.source` と突き合わせる試験が見る（final review R7 / handoff C1）。ソースを足す Story はどちらかに足さないとその試験で落ちる。書庫が実行時に足す `c03-myactivity-<製品>` は前置きで置く
- 終わりが始まりより前の値（読み違え・壊れた値）を点にしないのは、点にすると幅が縮んで消した時間と重ならなくなる側に倒れるため（既定は厳しい側）

#### D5a. 「緯度経度を持つ」の判定（spec の (1)〜(3)。Q8 / C20）

(1) **緯度経度を持たないと決まったソースの一覧**: `c01-app-usage` / `c01-app-usage-rollup` / `c01-clock` / `c02-window` / `c02-browser-history` / `s01-attribute`。
**一覧に無いソースは緯度経度を持つとみなす**（既定は厳しい側）。`c01-location` / `s01-stay` / `c01-photo` / `c03-*`（書庫の位置と項目。マイアクティビティ・YouTube・Chrome の項目は URL にだけ座標を持つことがある —— FR-50 ★ 2026-10-13）/ これから入る `s01-place` は一覧に入れない。
一覧は Rust の定数（`export::NO_COORD_SOURCES`）。足すのは、そのソースの payload に座標の項目が無いことを収集側の型で確かめた Story（理由を定数の隣にコメントで書く）。
持つ側も `export::COORD_SOURCES` に名前で置く（SQL は読まない —— 既定は持つ側のまま）。登録簿の全ソースがどちらか一方にだけあることを D5b の同じ試験が見る（final review R7 / handoff C1）。

(2) **内容の項目名**: オブジェクトに絞ってから `keyvalue()` を当てる（K1。スカラーや配列に `keyvalue()` を当てると DB がエラーにする）:

```sql
jsonb_path_exists(payload,
  'strict $.** ? (@.type() == "object").keyvalue() ? (@.key like_regex "^(lat|lon|lng|latitude|longitude)(e7)?$" flag "i")')
```

**`silent => true` で黙らせない**（黙らせるとエラーの行が NULL ＝「持たない」になり、漏れる側に倒れる）。
PostgreSQL 17 で確かめた（2026-10-12）: `{"a":1,"b":{"LatitudeE7":2}}` → t、`[1,{"lng":2}]` → t、`{"a":1,"b":[{"x":1}]}` / `5` / `"s"` / `{"latlng":1}` → f。

(3) **原文の中の座標の並び**: 原文を小文字にした `l` と、`l` の URL の符号化を戻した `n`（`%2c` → `,`、`%20` と `+` → 空白、`%c2%b0` → `°`）を作り、次のどれかに当たれば持つとみなす:

| 形 | 当てる先と正規表現（`~`） | 例 |
|---|---|---|
| 数の組 | `n ~ '(^|[^0-9.])-?\d{1,3}\.\d+\s*°?\s*[,/~ ]\s*-?\d{1,3}\.\d+'` | `@35.6812,139.7671` / `ll=35.68,139.76` / `q=35.68%2C139.76` / `q=35.6+139.7` / `35.6812°, 139.7671°` / `geo:-33.86,151.2` / 経度が先の `139.7671,35.6812` / OSM の `#map=15/35.68/139.76` / Bing の `cp=35.68~139.76` |
| 地図の埋め込み | `n ~ '!3d-?\d+(\.\d+)?!4d-?\d+(\.\d+)?'` と `n ~ '!2d-?\d+(\.\d+)?!3d-?\d+(\.\d+)?'` | `!3d35.6812!4d139.7671` / `!2d139.7671!3d35.6812` |
| 度分秒 | `n ~ '\d{1,3}°\s*\d{1,2}[''′]'` | `35°40'52.3"N 139°46'00.2"E` |
| 鍵と値 | `n ~ '(^|[^a-z])(lat|latitude|lon|lng|longitude)"?\s*[=:]\s*"?-?\d{1,3}(\.\d+)?'` | `?lat=35.68&lon=139.76` / 原文の `"latitude": "35.68"` |
| plus code | **`l`**（符号化を戻す前）に `'(^|[^0-9a-z])[23456789cfghjmpqrvwx]{4,8}\+[23456789cfghjmpqrvwx]{2,3}($|[^0-9a-z])'` | `MQ7X+2V 千代田区` |

plus code だけ `l` に当てるのは、`+` を空白に戻すと plus code の `+` が消えて必ず外れるため（spec-review R20）。
PostgreSQL 17 で確かめた（2026-10-12）: 上の例はすべて t、`Chrome 128.0.6613.84`・`price 1,234.50 yen`・`v1.2 notes`・`?id=12345`・`2026/09/07 12:30` は f。

- (2)(3) は (1) の一覧のソースのためにある（Q8。ウィンドウの題名・アドレスバーと、ブラウザの履歴の URL は、地図の URL として座標を文字列で持つ）
- **(3) は C（聞かない）。既定は厳しい側**（C20。整合の確認 C4 / K2）: 小数の桁を問わず、区切りの符号化を戻して読む。座標でない数の組（`1.5, 2.3` のような題名）も、消した時間の中なら除かれる ——
  除き過ぎは書き出し直せば戻るが、読み落としは外へ出て戻らない。**形を足す（拾う側へ広げる）のは下流で決めてよい。形を狭める（いま座標と読むものを読まなくする）のは、外へ出すものを増やすので本人に返す（A / loss: exported）**

### D6. 遅れて届いた位置の印（C18）は、写しの前に利用者ごとに 1 回。ST22 の判定をそのまま呼ぶ

`stay_store` に `pub async fn mark_late_for_user(pool, user) -> sqlx::Result<()>` を足す。
中身は `rebuild_day` と同じ順（トランザクション → `lock(LOCK_KEY, user)` → 基準を読む）で、
**既存の `mark_late_arrivals` を、`rebuild_day` と同じ並び `with_archive_sources(&c.sources)` を渡して、本人が消した滞在の最小の始まりの日（Asia/Tokyo）の始まり〜（最大の終わり＋1 マイクロ秒）の範囲で 1 回呼ぶ**だけ（判定を書き写さない）。
下端を日の始まりにするのは、`mark_late_arrivals` が候補を始まりの時刻で絞り、重なりを終わりで見るため —— 消した滞在より前に始まり消した時間にかかる書庫の区間（訪問・移動）を、`rebuild_day`（1 日の範囲を渡す）と同じく候補に入れる（spec-review R42）。
並びを `c.sources` だけにすると書庫の位置に `user:late` が付かず、spec の「作り直しと同じ判定」とずれる（整合の確認 2026-10-13 の K2。D5 の述語が時間で除くので外へは出ないが、印と読み出しが食い違う）。
位置の欄を持つ書庫の項目は `mark_late_arrivals` の中で足される（`item_sources`）。
上端に 1 マイクロ秒を足すのは、`mark_late_arrivals` が候補を `event_time < 上端` で絞る（終わりを含まない）ため —— 最後の消した滞在のちょうど終わりの位置にも印を付ける（spec-review R12）。
本人が消した滞在のある利用者ごとに、写し（D4）の前に走らせる。

- 失敗したら書き出しを失敗（`late_mark`）にする。D5 の述語は印が無くても位置を除くので、ファイルに漏れることは無いが、
  spec が「印を付ける」と約束しているので黙って進まない
- `rebuild_day` の振る舞いは変えない（`record-deletion` の要件に触れない）
- 印を付ける範囲は ST22 の判定のまま（いまの版の時間）で、前の版の広い時間（C19）には広げない。広い時間の中の行は D5 の述語が印なしで除く

### D7. 形式: 暗号化した zip の中に表ごとの JSON Lines（Q2。仮 / Q9）。1 行は SQL の `json_build_object` で作る

| ファイル | 元 | 出さない列 |
|---|---|---|
| `events.jsonl` | `core.event_live`（D5 で除いた残り） | なし（全列。C4） |
| `event_versions.jsonl` | `core.event_version_live`（D5 で除いた残り） | `txid`（DB の内部の値。ビューにも無い）。ビューが足す親の `sensitivity` / `origin` / `external_id` も出さない（表の列ではない。親は `events.jsonl` にある。spec-review R10） |
| `sources.jsonl` | `core.source` | なし |
| `attribute_kinds.jsonl` / `attribute_kind_names.jsonl` | `core.attribute_kind` / `core.attribute_kind_name` | なし |
| `places.jsonl` | `core.place`（場所の器。★ 2026-10-16 下流: main に入った ST21 の表を D8 の規則で足した。`attribute_kind` と同じく記録を束ねる器で、座標と名前は `events.jsonl` の `s01-place` にある） | なし |
| `feeling_target_kinds.jsonl` / `feeling_reminders.jsonl` | `core.feeling_target_kind` / `core.feeling_reminder`（★ 2026-10-17 下流: main に入った ST17 の表を D8 の規則で足した。主観の記録そのものは `events.jsonl` の `s01-feeling`） | なし |
| `stay_criteria.jsonl` | `core.stay_criteria` | なし |
| `heartbeats.jsonl` | `core.heartbeat` | `attempts` / `successes` / `raw` / `content_hash`（C15。`content_hash` は回数を含む原文のハッシュなので一緒に外す） |
| `coverage_spans.jsonl` | `core.coverage_span` | なし |
| `drop_reports.jsonl` / `drop_report_hours.jsonl` | `core.drop_report` / `core.drop_report_hour` | なし |
| `README.txt` | 説明書（D9） | —— |
| `manifest.json` | 目録（D9） | —— |

上の 15 個の項目（★ 2026-10-16: ST21 の `places.jsonl` で 12 → 13。★ 2026-10-17: ST17 の `feeling_target_kinds.jsonl`・`feeling_reminders.jsonl` で 13 → 15）は**すべて zip の中**に置き、すべてを同じ合言葉で AES-256（WinZip AES。`zip::AesMode::Aes256`。`zip` crate は 20 バイト以上の項目を AE-1、それ未満を AE-2 で書く）・Deflate で暗号化する（C22）。zip の外（置き場）に平文のファイルを置かない。
項目の名前は上の固定の名前だけ（記録の値を含まない）。zip の注記は `ashiato-export <export_id>` だけ（D16 が前のファイルを見分けるのに読む。暗号化されない）。
`events.jsonl` は 4 GiB を超えうる（3 年・上限で 17 GB 前後。deep.md 手順 4）ので、項目は `large_file(true)`（ZIP64）で書く。

- 1 行は `SELECT json_build_object('id', id, 'raw', raw, 'event_time', <時刻>, …)::text` のように、**列の一覧（D8 の分類）から SQL を組み立てる**。
  `json_build_object` は鍵の順を保ち、`raw`（`text`）は JSON の**文字列**として逐語で入る（C4。`::jsonb` に通さない）。`payload`（`jsonb`）は JSON の値として入る
- 時刻（`timestamptz`）は `to_char(col AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS.US"Z"')`（C14。マイクロ秒まで、`Z` で終える）。`date` は `YYYY-MM-DD`
- 並びは `event_time, id`（記録・前の版）、それ以外は主キー。読む側が `head` で見て古い順に並ぶ
- 行の終わりは `\n`、文字コードは UTF-8（BOM 無し）

> **（仮）**: JSON Lines（Q2。本人が推奨を選んだ B）。**反転条件** —— 外部の道具（表計算など）で開けないことが実際に困ったら、
> 同じ分類から CSV を足す（JSON Lines は残す。原文を逐語で持てるのは JSON の文字列のほう）。

### D8. 書き出しの分類の正本は Rust の定数 1 つ。表と列の両方を持つ

`export::CATALOG`: core の**すべての表**について `Include { file, view, excluded_columns, columns: [(name, 意味)] }` か `Exclude { reason }` のどちらか。

- `Exclude` にするもの: `coverage`（削除済みも数える）/ `erasure_ledger` / `deletion_ledger` / `export_ledger`（D10）/ `access_log` / `web_session` / `stay_absorbed`（C6）/ `outage_ledger`（★ 2026-10-17。ST14 の途切れの判定と通知の台帳。稼働の信号からの導出）/
  ST12 の書庫の内部の 8 表 `archive_ledger` / `archive_ledger_source` / `archive_file` / `archive_shape_confirmation` / `archive_sighting` / `archive_scan_counter` / `archive_pending_shape` / `archive_reread_failure`（取り込みの内部。書庫から入れた記録は `core.event` にあり、写しのファイルは DB の外。C6）
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

**目録 `manifest.json`**: `{"format": 1, "export_id", "snapshot_at", "exported_at", "schema_version", "files": [{"name", "source", "rows", "bytes", "columns": [{"name", "meaning"}]}], "not_included": [...], "fold_rule": "..."}`。
`rows` は書いた行を数えた値（数え直しではなく書いた数）。目録は zip の中にあり暗号化されるので、D16 が前のファイルを見分けるのには使えない（それは zip の注記の `export_id`。D16）。
説明書の 1 に「このファイルは書き出すときに入れた合言葉で暗号化してある。7-Zip などの AES に対応した道具で開く」を書く（開けた人が読む文。開く前の案内は画面の確認が持つ。D13）。

### D10. 書き出しの台帳 `core.export_ledger`。1 回 2 行（始まりと終わり）、追記のみ

```sql
CREATE TABLE core.export_ledger (
  seq          bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  export_id    uuid NOT NULL,
  kind         text NOT NULL CHECK (kind IN ('start', 'finish')),
  at           timestamptz NOT NULL DEFAULT now(),
  route        text NOT NULL CHECK (route IN ('pc-encrypted-zip')),  -- Q1 / Q9 の経路
  file         text NOT NULL,                                  -- 書き終えた zip の絶対パス（start の時点で決めた名前）
  snapshot_at  timestamptz,                                    -- start: 読み始める直前 / finish: 写しの時点
  result       text CHECK (result IN ('done', 'failed')),      -- finish だけ
  rows         jsonb,                                          -- finish: {"events.jsonl": n, …}
  bytes        bigint,                                         -- finish: zip ファイルの大きさ
  written      bytea,                                          -- finish（done）: 記録のファイルに書いた id のブルームフィルタ（D17）
  digest       text,                                           -- finish（done）: zip ファイル（暗号文）の SHA-256。D16 が「書いたその物か」を見分ける
  reason       text,                                           -- failed: 種別と、分類の無い表・欠けた列の名前（記録の値は持たない）
  CHECK ((kind = 'start') = (result IS NULL))
);
CREATE INDEX export_ledger_by_export ON core.export_ledger (export_id, seq);
```

UPDATE / DELETE / TRUNCATE を拒むトリガを**この移行専用の関数**（`core.reject_export_ledger_change()`）で置く（ST22 D1 と同じ理由）。

- **C17 は「1 回 1 行」と書いたが、2 行にした。** 始まりの行を先に書かないと、書き出しの途中でサーバが止まったときに「どの時点の写しがどこに書かれかけたか」が残らない（C17 の理由そのもの）。
  1 行で持つには行を書き換えることになり、追記のみと両立しない
- `user_id` を持たない —— 書き出しは全利用者の D-01 を 1 つにまとめる（D12）
- `written` は記録の `id`（uuid）から作るブルームフィルタで、記録の値も `id` そのものも取り出せない（C17「中身は残さない」に当たらない）。偽陽性 10⁻⁶ で 1 年目 約 4 MB
- **合言葉も、合言葉から作った値（ハッシュ・鍵）も持たない**（Q9。ハッシュがあると同じ PC の上で合言葉を総当たりする手がかりになる）。`digest` は暗号文のハッシュで、合言葉にも平文にも依らない（同じ PC の衛星はファイルそのものを読めるので、新しく渡すものは無い）
- 終わりの行の `file` は実際に付けた名前（D3。始まりの行と違うことがある）。読む側は終わりの行があればその `file` を、無ければ始まりの行の `file` を使う
- 前のファイルを消したことは台帳に書かない（消したかはファイルがあるかで分かる。D16）。台帳の `file` と `snapshot_at` は消した後も残り、「どの写しがどこに書かれたか」は追える
- 稼働状況・読み出しの口のどこにも出さない（D8 で `Exclude`）

### D11. 状態（`GET /export`）は「走っているもの」をメモリから、それ以外を台帳から読む

- `running`: `App.exports` にあれば
- `last`: 台帳の最後の `finish` かつ `result = 'done'` の行
- `last_failure`: `last` より新しい `finish` かつ `failed` の行（種別だけ）
- `interrupted`: `finish` が無く、`running` でもない `start`（サーバが途中で止まった）のうち新しいもの 1 つ。`{"started_at", "file"}` で、`file` はその書き出しの名前のファイルがあるときだけ（`rename` の後に止まったもの）。新しい書き出しはそのまま受け付ける（spec）。
  最後の `start` なら名前のファイルが無くても返す。**それより前に止まったものは、名前のファイルが残っている間だけ返す** —— 次の書き出しが書き終えても、置き場に残る止まったファイルを状態から消さない（code-verify R18。前は最後の `start` しか見ず、3 回目を書き終えると止まった zip がどこにも出なくなった）。止まったものが 2 つ以上残っていても返すのは新しい 1 つ（片付ければ次が出る）。`.partial-…` は状態に出さない（R18 の残り。PR 本文の申し送り）
  残った `.partial-…` も、書き出しの名前のファイルも消さない（書き終えたことを台帳で確かめられない。D16。`.partial-…` は書き出しの名前ではない。どちらも暗号化されている）
- `last.erased_after`: 後に消した記録の件数（D17）。`last` があるときだけ数える。台帳の書いた記録が読めない形なら `null`（数えられない。deep Q12）
- `leftover`: `last` の `start` より前の、`finish`（`done`）を持つ書き出しのファイルのうち、D16 の消す条件に合い、まだ残っているもの（D16 で消せなかったもの）。`last` より後の止まった書き出しのファイルは入れない（`interrupted` の側。spec-review R41）。無ければ空の配列

### D12. 全利用者を 1 つのファイルに書く

D-01 の全記録（NFR-16）なので利用者で分けない（1 つの zip）。各行が `user_id` を持つ。除く判定（D5）は**利用者を問わない**（消した時間に入る緯度経度つきの行は誰の識別子でも除く）。遅着の印（D6）は ST22 の判定が利用者ごとなので利用者ごと。
件数（`GET /export/summary`）も全利用者の和をソースごとに返す。

### D13. 画面は `#/settings` の `SettingsView.tsx`。押すとその場に確認、走っている間は 1 秒ごとに状態を読む（Q3 第 2 回）

- 行き先: `Root.tsx` に `#/settings` を足す。入口は**S-1 の見出しの行**（`#/day/` と `#/master` と並べて「設定」）。
  ui-direction の地図は S-2 → S-7 だが、入口を決め直すのは ST25 なので、ST25 が入口を付け替えるときに S-7 へのリンクも運ぶ（`docs/screens.md` に 1 行足す）
- 節の並び: 見出し「設定」と「稼働状況へ」→ 節「全記録の書き出し」（説明 1 行・ソースごとの件数と期間・**入らないもの**の一覧・「全記録を書き出す」）→ 節「前回の書き出し」→ 後続の節の予告 1 行。**proto の軸 1〜3 で本人が選んだ形**（Q3 第 2 回: `inout` / `confirm` / `count`）
- 入らないもの: 「本人が消した場面（削除の印の付いた記録と、消した滞在の時間に入る緯度経度を持つ記録。前の版を含む）」「削除と消去の台帳」「写真・動画の原本（D-02。記録のメタは入る）」の 3 行。**件数を付けない**（C8）。
  本人の proto の出力は 1 行目を「削除の印の付いた記録」と書いていたが、Q5 で除く範囲が広がったので、spec の除く範囲に合わせて 1 行目を言い直した（GO 資料の「未解決」1 の後半。deep.md Q3）
- 押すと: `POST` はまだ送らず、節の中に `role="dialog"` の確認を出す。中身は上から
  1. 「約 N 行・約 N MB・約 N 秒。この PC のフォルダに、合言葉で暗号化した 1 つのファイルとして書き出します。」
  2. 「合言葉は保存しません。忘れると開けません（書き出し直せば新しい合言葉で作れます）。7-Zip などの AES に対応した道具で開けます。」
  3. 「書き終えると前回の書き出しのファイルは消えます。」（**前回の書き出しがあるときだけ**。初めての書き出しでは消えるものが無い。spec-review R32）
  4. `<form>` の中に「合言葉」と「もう一度」の `type="password"` の入力 2 つ（`autocomplete="off"`。**`new-password` / `current-password` を付けない** —— ブラウザに生成と保存を促す。spec-review R43。12 文字未満・不一致の間は「書き出す」を `disabled` にし、理由を `aria-live="polite"` の 1 行で出す）と「書き出す」（submit）「やめる」。
     文字数は `Array.from(value).length`（符号位置。口の `chars().count()` と同じ。UTF-16 の `length` を使わない —— spec-review R44）
- 確認が出たら**合言葉の入力にフォーカスを移す**（キーボードでは打って Tab、もう一度打って Enter で書き出せる。spec の Scenario も同じ形）。
  見込みは `GET /export/summary` の `estimate`（D18）。「書き出す」で `POST /export`（本体 `{"password"}`）、「やめる」で押す前の形に戻す（入力は空にする）。
  ブラウザの `window.confirm` は使わない（文面と大きさを決められない）
- **合言葉の扱い**（Q9）: 入力の値は React の state にだけ持ち、`POST` を送ったら確認ごと消す（state を空にして入力を外す）。URL・`localStorage`・`sessionStorage` に書かない。
  `POST` の失敗（400 / 409 / 通信）でも入力は空に戻し、節の中に 1 行（「書き出しを始められませんでした（すでに書き出し中です / 合言葉が短すぎます / 通信できませんでした）」）を出して押す前の形に戻す（spec。409 のときは状態を読み直して進み具合を出す）。
  `<form>` は `action` を持たず `onSubmit` で `preventDefault` する（URL のクエリに載らず、ページを移らない —— 保管庫が「送信」として拾いにくくする）
- `running` がある間は「全記録を書き出す」を `disabled` にする（spec。別の端末やタブで始めた書き出しを含む）
- 途中で止まった書き出しの 1 行は、`interrupted.file` があればパスも出す（「前の書き出しは途中で止まりました（<始めた時刻>）。<ファイルのパス> が残っています。」）
- 開いたときに `GET /export/summary` と `GET /export` を 1 回ずつ。`running` があれば 1 秒ごとに `GET /export` を読み直し、無くなったら止める（画面を離れると止まり、戻ると開いたときの読み出しで再開する）。
  **読み直しは前の応答が返ってから 1 秒後に次を読む**（`setTimeout` の連鎖。`setInterval` は前の応答を待たずに重ねて読み、遅い応答が DB の接続を奪い合う。final review R1）
- 前回の書き出しの欄: `last.erased_after` が 1 以上なら「この書き出しの後に消した記録が N 件あります。書き出したファイルの中には残っています。」を `role="note"` で 1 行（Q3 第 2 回の proto の文面のまま）
- 失敗は**書き出しの節**（「全記録の書き出し」。「前回の書き出し」の節ではない。final review R3）の中に 1 行（「書き出しに失敗しました（<種別>）」）、途中で止まったものも 1 行（「前の書き出しは途中で止まりました（<始めた時刻>）」）、
  消せなかった前のファイルも 1 行（「前の書き出しのファイルを消せませんでした（<ファイルのパス>）。次の書き出しでもう一度消します。」）。どれも「全記録を書き出す」を押し直せる（spec）
- `MIN_TARGET_PX`（24）・ライト / ダークの `tokens.ts`・フォーカスの輪郭は既存の画面と同じものを使う（合言葉の入力 2 つも 24 px 以上）。ファイルのパスは `word-break: break-all`（360 px で横にはみ出さない）
- ダウンロード・保存・`file://` のリンクを置かない（Q1）

> **（仮）**: 1 秒ごとに読む間隔と、件数を開くたびに数える形（D1 の `GET /export/summary`）。**反転条件** —— 件数の数えに 3 秒以上かかる量になったら（書庫で百万件級を入れた後など）、
> 件数を前回の書き出しの目録から出して「いまの件数」は押したときにだけ数える。間隔は、読み出しの記録（access_log）が 1 回の書き出しで数百行増えるのが困るなら伸ばす。
> **判定（2026-10-14。final review R1 / R4）: 件数の数えの反転条件に当たる。** 使い捨ての DB に 1 年分（35 万行の `c02-window`、100 に 1 つが座標つき、消した滞在 1 件）を置いた測り
> （`export_volume_one_run_reports_time_and_memory`）で、`GET /export/summary` の数え 1 回が **6.5 秒**（書き出し全体は 42.8 秒）。3 秒を超えるので「件数を前回の書き出しの目録から出し、
> いまの件数は押したときにだけ数える」形へ替える条件を満たした。**この fix wave では替えていない**（替えるかは本人か controller が決める。開いたときに 1 回だけ読むので、繰り返しの負荷ではない）。
> 1 秒ごとの読み出し（`GET /export`）の側は D17 の鍵つきの覚えと、応答を待ってから次を読む形で、数え直しを書き込みのあったときだけにした。
> 確認・入らないもの・後に消した件数・消せなかったファイル・合言葉の案内の文面と、確かめの再入力も仮。**反転条件** —— 本人が確認バッチで読みにくいと言えば文面を変える（出す・出さないは Q3 / Q9 の答えなので変えない）。
> 再入力は、本人が二度打つのが煩わしいと言えば「表示する」の切り替えに替えてよい（打ち間違えたまま書き出しても、書き出し直せば戻る）。
> **再入力と文面は spec の Requirement と Scenario にも書いてある** —— 替えるときは spec も直す（archive の後は `MODIFIED`。spec-review R47）。

### D14. 変えないもの

| もの | なぜ |
|---|---|
| `GET /events` | C13。ST01 の読み出し経路 |
| `record-deletion` の要件・`rebuild_day` の振る舞い | 読むだけ（D6 は既存の判定を呼ぶ口を足すだけ） |
| `data-sensitivity` の要件 | 拠点外へ送る部品を足さない（ファイルはローカル）。`tools/check-offsite.sh` が見る一覧に当たる crate を足さない（`zip` は既にあり、`zeroize` は送る部品ではない） |
| `core.event_live` / `core.event_version_live` の定義 | 書き出しは読むだけ。列が欠けたら直すのは列を足した Story（D9 が落ちる） |

### D15. ログ

`kind = "export"` に `export_id`・段（`late_mark` / `catalog` / `columns` / `write` / `finish`）・件数・大きさ・種別だけ。
**記録の値・合言葉・置き場のパスの中の利用者名は出さない**（製造準備 A-2 / Q9）—— パスは応答と台帳には出すが、ログには `file` の最後の名前だけを出す。
合言葉はどの段のログにも、エラーの文言（`Debug` の出力を含む）にも出さない。本体の型は `Debug` を手で書いて合言葉を伏せる。

### D16（仮）. 置き場には書き終えた書き出しを 1 つだけ置く。書き終えてから前のものを消す（Q4 第 2 回 / Q9）

本人が第 2 回の貼り戻しで選んだ B（推奨。第 1 回は「毎回別のフォルダに残す」）。Q9 で書き出しが 1 つの zip になったので、消す単位はファイル 1 つ。

- 順: 新しい書き出しを `.partial-….zip` に書く → `rename` → 台帳の `finish`（`done`。D10）→ **前のファイルを消す**。
  書き終える前（失敗・サーバの停止）は前のファイルに触らない —— 手元の書き出しが 0 個になる時間を作らない。
  `finish` を消す前に書くのは、消している間にサーバが止まっても、書き終えた書き出しが台帳に `done` として残るため（spec-review R24）
- 消す対象: 台帳で `finish`（`done`）を持つ書き出しのうち今回のもの以外で、その `file` のパスにあるファイルが、**大きさ・SHA-256（`digest`）が終わりの行と同じで、zip の注記が `ashiato-export <その行の export_id>`** のもの。
  大きさとハッシュで見るのは、本人が前のファイルに zip の道具で項目を足す・除くと、注記は残る（道具は注記を引き継ぐ）が中身は本人の物になるため（spec-review R40。消すのは `discarded` で戻らない —— 「捨てるより残す」）。
  大きさが違えばハッシュを計算しない（1 GB 級を毎回読まない）
- **`finish` を持たない書き出し（`rename` の後・`finish` の前に止まったもの）のファイルは消さない**。書き終えたことも、ハッシュも台帳に無い。`GET /export` の `interrupted.file` と画面の 1 行で見せ、消すのは本人（spec。第 2 巡の R24 では次で消していたが、R40 の見分け方ではそれを確かめられない）
- 合わないファイル（本人が置き換えた・中身を変えた）・台帳に無いパス（本人が置いた・別の名前に写した）・`.partial-…` には触らず、`leftover` にも出さない（このシステムの書き出しではない）。
  zip として読めないファイルも合わないものとして扱う（消さない）
- 置き場を `ASHIATO_EXPORT_DIR` で変えた後でも、前の置き場の書き出しを消す（台帳の `file` は絶対パス）。「1 つだけ置く」は置き場ごとではなくこのシステムが書いたもの全体で数える
- 消せなかったら（使用中・権限）: 書き出しは `done` のまま。そのファイルは条件に合ったまま残るので `GET /export` の `leftover` に出る（D11。台帳には書かない —— `finish` はもう書いてある）。次の書き出しが書き終えたときに、もう一度消す
- 消すのはファイルシステムの削除（ごみ箱を通さない）

> フォルダ（第 2 回の形）では、本人がフォルダの中に置いたファイルを残すために、目録が挙げるファイルだけを消していた（spec-review R23）。1 つのファイルでは、
> 見分けるのは「このシステムが書いたその物か」になり、大きさ・ハッシュ・注記がすべて合うものだけを消す。どれかを本人が変えたファイルは消さない側に倒れる。

**反転条件** —— 前の書き出しを手元に残したい場面（書き出し直す前の状態と比べたいなど）が実際に出たら、本人に Q4 を聞き直す（残す側に戻すと、後に消した場面が古いファイルに積み上がる —— 第 1 回の帰結）。
消すことで困ったら（同期の道具が消したことを外へ伝えるなど）、消す前に本人に確かめる形を足す。

### D17（仮）. 後に消した記録の件数 `erased_after`（Q3 第 2 回）

**前回の書き出しの記録のファイルに入っていて、いまは本人が消した場面に当たる記録**を数える。写しの時点で「入っていたか」を後から作り直さず、**書いたものを覚えておく**:

- 書き出しは記録のファイルに 1 行書くたびに、その `id` をブルームフィルタに足し、`finish`（`done`）の行の `written` に置く（D10。偽陽性 10⁻⁶）
- 数えるときは、いま除かれる記録 —— 本人の削除の印がある（`deleted_by IS NULL OR deleted_by NOT LIKE 'rebuild:%'`。`Mark::of` と同じ読み方）か、D5 の 2 に当たる —— の `id` を流し、`last` の `written` に入るものを数える（Rust の側で当てる）
- **誤りは多く数える側にしか出ない**（ブルームフィルタは入っていたものを落とさない）。少なく数えて「残っていない」と思わせる経路が無い
  （spec-review R25: 写しの時点を `deleted_at` と比べる形だと、写しの前に始まり後にコミットした削除（`deleted_at` は始まりの時刻）を数え落とした）
- 作り直し（`POST /stays/rebuild`）が付けた印（`rebuild:`）は数えない —— 本人が消していないものを「消した」と出さない（spec）
- 数えるのは記録（`events.jsonl`）の行だけ。前の版は数えない（前の版は親と一緒に出入りする —— D5 の前の版の 2）
- 書き出し直すと `last` が新しくなり、0 に戻る（spec）
- **数えた結果は鍵つきで覚え、鍵が同じなら数え直さない**（final review R1）。「1 秒ごとの読み出しで毎回数えても軽い」は成り立たなかった ——
  数えるのは「いま除かれる記録」だが、それを決める D5 の断片は `core.event_live` の全行に座標の判定と (c) の次の記録の引きを当てるので、
  共有の開発 DB で 1 回約 10 秒かかった（Task 3）。画面は前の応答を待ってから次を読む（D13）。
  鍵（`export::ErasedKey`）は `last` の `export_id` と、`core.deletion_ledger`・`core.event_version`・`core.stay_absorbed` の**行の数**:
  - 本人の削除の印（消す・連鎖・遅着の印）と戻すは削除の台帳に、記録の内容・時刻の更新（取り込みの更新・作り直しの `push_version`）は前の版に、作り直しの吸収は吸収の台帳に、同じまとまりで 1 行を足す
  - 版を書かない内容の更新は無い（2026-10-14 に `UPDATE core.event` と `ON CONFLICT … DO UPDATE` を洗った）: `collected` は門（`require_version_or_ledger`）が同じまとまりの版を強制し、
    `derived` の滞在は作り直しが `push_version` してから書き換え（中身が同じで印だけ変えるときは版を書かないが、それは `rebuild:` の印）、`authored` の行は書き換える経路が無い。
    `ON CONFLICT … DO UPDATE` は書庫の表（`core.archive_*`）だけ
  - 3 つの表は追記のみ（書き換えと削除をトリガが断る）なので、行の数はコミットのたびに必ず増える。最後の `seq` は、先に番号を取って後にコミットした行を見落としうるので使わない
  - 鍵に無い書き込みは数を減らす側か動かさない側にしか働かない: 新しい記録の取り込み（次の記録が手前に来て幅が縮む）・作り直しの `rebuild:erased-range`（`s01-stay` の行だけで、滞在は `end` で幅を読むので他の行の幅を延ばさない）・消去（本文を空にする）。
    覚えた値はこの場合に**多く数える側にだけ**ずれ、上の「誤りは多く数える側にしか出ない」を保つ
  - 鍵は数える前に読む（数えている間に入った書き込みは次の読み出しで鍵を変える）
  - 量の目安: 同じ断片を通る `GET /export/summary` の数えが 1 年分（35 万行）で 6.5 秒（D13 の判定）。数え直しはこの桁になる
  - 前提: 消した滞在の時間は過去にある。(c) の幅の終わりの「写しの時点」は数える時刻（`now()`）なので、始まりが未来の消した滞在があると、時間が経つだけで重なりが増えうる（鍵は動かない）。
    滞在は届いた位置から作る派生なので、未来の滞在は時計の狂った端末からしか生じない
- **`written` の形は凍結する**（deep Q12。code-verify R20）。台帳は追記のみで `written` を書き直せないので、位置の取り方・ハッシュの数・大きさの決め方・先頭の 2 バイト（版・ハッシュの数）を変えると、前に書いた行の `id` が入っていないと読まれる（変異で 1,451 件 → 0 件を実測）。
  固定の 3 つの UUID を入れたバイト列を、`bloom.rs` を使わない別の実装（Python の `hashlib`）で作って期待値に直書きした試験（`export::bloom::tests::the_frozen_shape_is_kept`）が形を固定する。
  形を変えるなら `VERSION` を上げ、前の版を読む口を残す
- **読めない形の `written`（知らない版・空）は 0 件と読まない**: `last.erased_after` を `null`（数えられない）で返し、画面は「件数を数えられません。ファイルの中に後に消した記録が残っているかもしれません」と 1 行出す（deep Q12）。0 と出すと少なく数える側に倒れ、上の「少なく数えて残っていないと思わせる経路が無い」が崩れる
- 件数は `GET /export` の応答と画面にだけ出し、ファイル・目録には書かない（C8）
- **ST23 が D-01 から記録を本当に消す**（FR-51）と、その記録はいま除かれる記録の一覧から消え、数に入らなくなる（ファイルには残っている）→ ST23 が入るときに、消した `id` を同じ数え方に足すかを ST23 が決める（下流の PR の前に `docs/handoff/ST23.md` に書く）

> **反転条件** —— 偽陽性で 0 にならない実例が出たら偽陽性の率を下げる。前の版も数えたほうが分かりやすいと本人が言えば、前の版の `id` も同じ形で覚える。
> 覚えた値を使う形は、鍵に無い書き込みで数が増える経路が見つかったら（新しい種類の印や、台帳を書かずに `deleted_at` を付ける書き手を足したとき）その表を鍵に足す。
> 鍵が変わるたびに数え直す 1 回が重すぎる（書き出しの間の画面が数十秒待つ）なら、件数を書き出しの直後と削除の操作の直後にだけ数えて台帳の外（メモリ）に置く形に替える。

### D19. 暗号は zip の AES-256（WinZip AES）。合言葉は 12 文字以上（Q9 / C22 / C23）

- 方式: `zip::AesMode::Aes256`。項目ごとに合言葉から鍵を作る（PBKDF2-HMAC-SHA1・1,000 回・項目ごとの salt。形式が決めている）。7-Zip・WinRAR・Python の `pyzipper` などで開ける。
  `zip` crate 2.4.2 は 20 バイト以上の項目を **AE-1** で書き（`write.rs:1083-1098`。切り替える口は無い）、**平文の CRC-32 と元の大きさが暗号化されない中央ディレクトリに残る**（spec-review R38）。
  中身を推し量れるのは、文面がほぼ決まった説明書と目録の数（行の数・大きさ）を、CRC-32 で総当たりして確かめる経路まで。記録の値（原文・座標・本文）は高い不確かさを持つので CRC-32 からは戻らない。
  量（表ごとの行の数と大きさ）は外から読めるものとして扱う（C22 の理由を「量を隠す」から外した。deep.md C22）。AE-2 で書くには crate を替えるか自前で書くことになり、得るのは量の秘匿だけなので採らない
  **Windows のエクスプローラーの「すべて展開」は AES の zip を開けない**（ZipCrypto しか読まない）—— 画面の確認と説明書に「7-Zip などの AES に対応した道具」と書く（D13 / D9）
- 採らなかった案: ZipCrypto（エクスプローラーで開けるが、既知の平文で破れる）/ 7z 形式（AES-256 で鍵の導出も強いが、標準の道具が少なく、Rust で書く crate を足すことになる）/ age・GPG（「外部の道具で開ける」が本人の手元に道具を求める）
- **合言葉の下限は 12 文字**（C23）。形式の鍵の導出は 1,000 回で軽く、同じ PC の衛星はファイルを写して合言葉を手元で総当たりできる。強さを測る規則（文字種など）は置かない（本人が覚えられる長い文を妨げない）。
  下限を**上げる**のは下流で決めてよい。**下げる**のは衛星に読まれうる側へ倒すので本人に返す（A / loss: exported）
- 文字数は Unicode のスカラー値で数え、正規化しない（同じ文字を打てば同じ鍵）。zip の AES は合言葉を UTF-8 のバイト列として使う。日本語の合言葉も通るが、開く道具によっては UTF-8 の合言葉を正しく扱わない —— 説明書と画面には書かない（本人が打てたものを止めない）。確認バッチで開けない道具が見つかれば文面を足す

### D18（仮）. 書き出しの見込み（確認に出す。Q3 第 2 回）

`GET /export/summary` の `estimate`:

- `rows`: ソースごとの件数の和（記録の行の数。前の版と辞書の行は含めない —— 確認は「約 N 行」の目安で、記録が大半を占める）
- `bytes`: **圧縮した後の zip の大きさの見込み**。前回の書き出しの `bytes / rows` × `rows`。前回が無ければ 1 行 180 バイト（deep.md 手順 4 の平文 900 バイトに、JSON の Deflate の仮の圧縮率 0.2 を掛けた値。Task 5.6 の量のテストで実測して置き換える —— spec-review R49）
- `seconds`: 前回の書き出しの（`finish.at` − `start.at`）/ `rows` × `rows`。前回が無ければ 20,000 行 / 秒（proto の見積もり）。1 未満は 1

> **反転条件** —— 確認の見込みが実際と 2 倍以上ずれると本人が言えば、前の版と辞書の行を足す・書き出しの量のテスト（Task 5.6）の実測から係数を取る。

### D20（仮）. 台帳の始まりの行は `POST /export` の求めの中で書く。書けなければ 500（code-verify R22）

- `spawn` が走っている印を置いた後、背景の仕事を起こす**前に** `insert_start` を書く。書けなければ印を外し、何も走らせず `500 {"error": "ledger_unavailable"}`（種別だけ）。画面は既存の「書き出しを始められませんでした」の 1 行に乗る
- 前は背景の仕事の中で書いていたので、書けないとログ 1 行だけで終わり、`202` を返したまま状態にも画面にも何も出なかった（spec「失敗した書き出しも台帳に残る」の外側）
- 試験の差し込み口は `Stage::Start`
- **反転条件**: 求めの中で DB を待つ時間が画面の押した感触を損なう（`POST` の応答が 1 秒を超える）なら、背景で書いたまま失敗をメモリの `ExportJobs` に 1 件持ち、`last_failure` に出す形に替える

## Risks / Trade-offs

- **一覧（D5a）にあるソースが、(2)(3) で読めない形で座標を持つと、消した時間の中でも出る** → (2) はどの深さの項目名も見る。(3) の形は見つけたら足す。一覧に無いソースは除く側なので、新しいソースの既定は漏れない
- **幅の読めないソースは次の記録までの幅になる（D5b の (c)）ので、消した時間の前の行が除かれ過ぎる**（次の記録が何時間も後なら、その間に消した滞在が 1 つあれば除かれる。次が無ければ写しの時点まで）→ 除き過ぎは書き出し直せば戻る。点のソースは `SPAN_RULES` の `Point` に足して狭める（収集側の型で確かめた Story が）
- **(3) の並びは座標でない数の組も拾う**（C20）→ 消した時間の中の `1.5, 2.3` のような題名のウィンドウも除かれる。除き過ぎの側で、狭めるのは本人に返す
- **除き過ぎ**（緯度経度を持つが場面とは関係の薄い行。例: 消した時間に登録した場所）→ 出さない側。後で範囲を狭めて書き出し直せる（ファイルは D-01 から作り直せる）
- **長い読み取りのトランザクション**（1 年目で約 1 分、上限なら 20 分近く）→ その間 VACUUM が古い行を片付けられない。取り込みは止まらない。押したときだけ走るので常時ではない
- **書き出した後に消した場面は、次に書き出すまで前のファイルに残る**（Q4 第 2 回で 1 つだけ置く。Q3 第 2 回で画面に件数を出す）→ 画面の「後に消した記録が N 件」が書き出し直しを促す。N は多く数える側にだけずれる（D17）
- **前のファイルを消せないと 2 つ残る**（使用中・権限）→ `leftover` と画面の 1 行に出し、次の書き出しで消し直す（D16）
- **合言葉を総当たりされうる**（D19）→ zip の AES の鍵の導出は軽く、同じ PC の衛星はファイルを写して手元で試せる。下限 12 文字で、推測しやすい合言葉（短い単語の組・使い回し）は守れない。
  暗号化は「権限を越えて読める衛星に、そのまま読ませない」ための層で、合言葉の強さは本人の手にある
- **開いて取り出した後のファイルは平文**（Q9 の選択肢の帰結）→ 本人の扱い。システムは取り出した先を知らない
- **zip の外から見えるもの**: 項目の名前（固定）・項目ごとの圧縮後と元の大きさ・平文の CRC-32（AE-1。D19）・zip の注記（`export_id`）・ファイルの更新時刻 → 記録の値は無い。表ごとの量（大きさ。説明書と目録の数は CRC-32 で推し量れる）は外から読める
- **DB の一時ファイル**: 記録と前の版の `event_time, id` の並べ替えと D5 の `lead` の窓は、量によって PostgreSQL の一時ファイルに溢れ、平文の行がそこに書かれうる（spec-review R48）→ D-01 そのものと同じ置き場（DB のデータの下）で、D-01 より多くを渡すものではない。spec の「一時フォルダ」はこのシステムが書くものを指す
- **合言葉を忘れると開けない** → 書き出し直す（ファイルは D-01 から作り直せる）。前の書き出しは書き終えると消えるので、忘れた合言葉のファイルも次で消える
- **置き場が同期されるフォルダだと、書いた時点で外へ出る**（C16）→ 既定は `%LOCALAPPDATA%`。本人が `ASHIATO_EXPORT_DIR` で変えるのは止めない
- **ディスクが足りない** → `io` で失敗、`.partial-…` を消す。前の書き出しには触らない（書き終える前に前のものを消さないので、置き場には前の 1 つ分と書きかけ 1 つ分の空きが要る。圧縮するので JSON Lines のままより小さい）
- **サーバが止まると `.partial-…` が残る** → 状態に `interrupted` と出る。消すのは本人

## Migration Plan

1. 移行 1 本（`YYYYMMDDHHMM_export_ledger.sql` と `.down.sql`）を当てる。既存の行に触らない
2. サーバを入れ替える（口 3 本・背景の仕事・`stay_store::mark_late_for_user`）
3. 画面を入れ替える（`#/settings` と S-1 のリンク）
4. 巻き戻し: `.down.sql` で `core.export_ledger` とトリガと関数を落とす。**書き出したファイルは残る**（システムの外にある）。台帳が消えるので、どの書き出しが何時点の写しかは各ファイルの目録（合言葉で開く）で読む

## Open Questions

無し。（仮）決めは D2 / D3 / D7 / D13 / D16 / D17 / D18 の 7 件で、反転条件はそれぞれの項に書いた。
Q9 の形（暗号の方式・項目の置き場所・合言葉の下限・平文の一時ファイル・権限）は（仮）にしない —— どれも緩めると外へ出るものが増えるので、厳しい側の既定 C22〜C25 として決めた（D19 / D7 / D4 / D3）。
D5a (3) の並びの形は（仮）から外した —— 読み落としは外へ出て戻らないので、計算し直せば戻る B ではない（整合の確認 C4）。既定は厳しい側の C20 として決め、狭める変更は本人に返す。
