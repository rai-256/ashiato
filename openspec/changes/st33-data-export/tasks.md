# ST33 実装タスク — 全記録を外部の道具で読める形に書き出す

読む順: `deep.md`（**最優先。本人が決めた 9 件（Q1〜Q9。Q3 と Q4 は第 2 回の貼り戻しの答え）と、聞かずに決めた C1〜C25**）→ このファイル →
`specs/data-durability/spec.md` → `design.md` → `docs/stories/ST33.md` → `docs/handoff/ST33.md`（開始時と PR 前の 2 回）→ `CLAUDE.md`。

**移行は 1 本だけ足す**（design D10）。**名前は作成時刻 `YYYYMMDDHHMM_export_ledger.sql`**（連番にしない）で、
`crates/server/src/lib.rs` の `MIGRATIONS` 配列の末尾に足す（配列の長さも書き換える）。
**`GET /events`・`record-deletion` の要件・`rebuild_day` の振る舞い・`core.event_live` / `core.event_version_live` の定義には触らない**（design D14）。

検証は各タスクの本文に書いてある。「動いた」ではなくコマンドと終了コードで判定する。
DB を使う検査は `docker compose up -d db` が前提。

## Global Constraints（規律。**最初に読む**）

- **テストには `Scenario: <名前>` の印を置く。** Rust / TypeScript はコメント（`// Scenario: 削除の印の付いた記録は出ない`）、
  bash は `echo`。`scripts/check_scenarios.py` が spec の全 Scenario と突き合わせ、**印の無い Scenario を FAIL にする**
- 印の名前は spec の `#### Scenario:` と**一字一句合わせる**（空白は無視される）
- **この change が持つ Scenario は 98 本**（すべて `data-durability`。サーバ 77 本は Rust のテスト、画面 21 本は `web/e2e`）
- **件数つき検証**: `cargo test <絞り込み>` は一致するテストが 0 本でも rc=0 になる。このファイルで
  **`CT <絞り込み> <N>`** と書いたものは、次のコマンドが rc=0 になることを指す（`<N>` はその項目が持つテストの本数の下限）:
  `bash -o pipefail -c 'cargo test -p ashiato-server <絞り込み> 2>&1 | tee /tmp/ct.log' && n=$(grep -Eo 'test result: ok\. [0-9]+ passed' /tmp/ct.log | awk '{s+=$4} END{print s+0}') && [ "$n" -ge <N> ]`
  （`cargo test` に絞り込みを 2 つ渡すと `unexpected argument` で落ちる。1 つずつ書く。**Scenario 1 本にテスト 1 本以上**を置くので、`<N>` はその項目の Scenario の数以上）
- **e2e も件数で確かめる**: `ET <grep> <N>` は `cd web && npx playwright test settings-export --grep '<grep>' --list` の件数が `<N>` 以上で、かつ `npm run test:e2e -- settings-export --grep '<grep>'` が rc=0
- **書き出しのテストは直列にする**（spec-review R16）。書き出しの台帳・状態・置き場は全利用者で 1 つで、`cargo test` はテストを並列に走らせる。
  `export_tests.rs` の書き出しを走らせるテストは、static の `tokio::sync::Mutex`（`EXPORT_SERIAL`）を先頭で取る
- **テストは利用者で隔離する**（`testdb::user()` で毎回新しい利用者を作る）。書き出しは全利用者を読むので、
  **ファイルの中身を確かめるときは、そのテストの利用者の行だけを見る**（他のテストの行が混ざる前提で書く）。
  **表の件数の比較は、書き出しと同じ写しで数えた値（目録の `rows`）と比べる** —— テストの外の表の件数と比べない。置き場はテストごとの一時ディレクトリ
- **テストは時刻でも隔離する**（spec-review R27）。除く判定は**利用者を問わない**（Q7）ので、別のテスト（ST22 のテストを含む）が消した滞在の時間に入る行は、利用者が違っても除かれる。
  書き出しのテストの記録と消す滞在は、**テストごとに他のテストが使わない日**（`testdb` に `unique_day()` を足す。2000 年代の前半から、テストごとに違う日を返す）に置く。
  「出る」側の Scenario と件数の比較は、その日の行だけで確かめる
- **除く範囲の SQL は 1 か所だけ**（design D5）。書き出し・`GET /export/summary`・後に消した記録の件数（D17）が同じ断片を使う。件数を別の式で数えない
- **書き出しの分類の正本は `export::CATALOG` 1 つ**（design D8）。説明書の列の意味もここから引く。2 か所で持たない
- **記録の値を口の応答に載せない**（Q1）。ダウンロード・保存の口も画面の操作も作らない
- **書き出しは合言葉で暗号化した 1 つの zip**（Q9。design D3 / D7 / D19）。表ごとのファイル・説明書・目録はすべて zip の中で AES-256。zip の外に平文を置かない。項目の名前と注記に記録の値を書かない（C22）
- **合言葉はどこにも残さない**（Q9。design D1 / D2 / D10 / D15）—— DB・置き場・ログ・台帳・応答・読み出しの記録・画面の保存領域・エラーの文言。合言葉から作ったハッシュも残さない。ブラウザに保存を促す指定（`autocomplete="new-password"` ほか）を付けない（D13）。
  下限は 12 文字で、口も画面も Unicode の符号位置で数える（C23。**上げるのはよいが、下げるのは本人に返す**）
- **このシステムが書くファイル（置き場と一時フォルダ）に、暗号化しない記録の値を書かない**（C24。design D4）。行は zip の暗号化の層に直に流す。表ごとの平文の一時ファイルを作らない。置き場に zip の他のファイルを書かない（C22）
- **`mark_late_arrivals` の判定を書き写さない**（design D6）。`stay_store` に呼ぶ口を足すだけ
- **D2 / D3 / D7 / D13 / D16 / D17 / D18 は（仮）決め。** 反転条件は `design.md` にある。変えたらその D 番号の（仮）を外すか反転条件を書き直す
- **Q1〜Q9 は本人の決定。下流は変えない** —— Q1 置き場（自宅 PC のフォルダだけ）/ Q2 形式 / **Q3 画面の形（第 2 回: 件数・期間 ＋「入らないもの」・確認をはさんでから進み具合・後に消した記録が N 件と出す）** /
  **Q4 前の書き出し（第 2 回: 書き終えたら消して 1 つだけ置く）** / Q5 除く範囲（緯度経度を持つ行も除く。前の版を含む）/ **Q6 幅で重ねて除く / Q7 利用者を問わず除く / Q8 URL や題名の中の座標も数える** /
  **Q9 押すときに本人が入れる合言葉で暗号化した 1 つのファイル（合言葉は保存しない）**
- **削除の連鎖は ST12 の後、書庫の位置と位置の欄を持つ書庫の項目にも印を付ける**（`deletion.rs` の `with_archive_sources`。design の Context）。「印の無い別のソース」を材料にするテストは、**写真の位置（`c01-photo`）か、テストで登録簿に足した連鎖の掛からないソース**を使う。書庫の位置を「印が付かない」材料にしない（整合の確認 2026-10-13 の C2）
- **除く範囲を狭める変更はしない**（deep.md の「spec で精密化したもの」）。C19（消した滞在の前の版の広い時間を数える）・C20（D5a (3) の並びは厳しい側。**形を足すのはよいが、狭めるのは本人に返す**）・C21（幅の読めないソースは次の記録まで）も同じ。
  書庫の点のソースを `SPAN_RULES` の `Point` に移すのは、ST12 の解析器の型で出来事 1 点と確かめたものだけ（design D5b）
- 記録の値（座標・時刻・URL）・合言葉・置き場のフルパスを**ログに出さない**（製造準備 A-2。design D15）。出すのは件数・大きさ・種別・`export_id`・ファイルの最後の名前だけ

## Task 1: 移行 —— 書き出しの台帳（design D10）

- [x] 1.1 移行 `migrations/YYYYMMDDHHMM_export_ledger.sql` と `.down.sql` を足す —— `core.export_ledger`（D10 の列と CHECK）、索引 1 本、
  UPDATE / DELETE を拒む行トリガと TRUNCATE を拒む文トリガ（**この移行専用の関数** `core.reject_export_ledger_change()`）。当て直せる形（`IF NOT EXISTS`）。
  `route` は `pc-encrypted-zip` だけ、パスの列は `file`（終わりの行は実際に付けた名前）、`finish`（`done`）の行は zip の SHA-256 の `digest` も持つ。`finish` の行の `written`（記録の id のブルームフィルタ。design D17）も持つ。**合言葉とそのハッシュの列を持たない**。`MIGRATIONS` 配列の末尾に足す。検証: `tools/check-migrations.sh` rc=0、`CT export_ledger_migration 1`（全版を 2 回当てて落ちないテスト `export_ledger_migration_applies_twice`）
- [x] 1.2 台帳が追記のみであることのテスト。
  Scenario: `書き出しの台帳の行は書き換えも削除もできない`。検証: `CT export_ledger_is_append_only 1`
- [x] 1.3 `tools/check-immutable.sh` に足す —— (a) `core.export_ledger` への UPDATE / DELETE / TRUNCATE が psql から拒まれること、
  (b) `…_export_ledger.down.sql` を戻しの逆順の先頭で当てて、当て直せること。
  検証: `bash -o pipefail -c 'tools/check-immutable.sh | tee /tmp/ci.log' && grep -q export_ledger /tmp/ci.log`

## Task 2: 書き出しの骨組み —— 設定の型・差し替え口・分類・権限（design D2 / D3 / D8 / D9。`crates/server/src/export.rs`）

書き出しは新しいモジュール `crates/server/src/export.rs`（分類・SQL・ファイル・本体）に置く。テストは `crates/server/src/export_tests.rs`。
**後の Task が使う差し替え口はここで作る**（spec-review R14）。

- [x] 2.1 本体が受ける設定の型（置き場・遅延・**段ごとの差し替え口**: 分類の段に渡す表と列の一覧、段の前後で呼ぶ関数、失敗の差し込み）と、
  置き場の決め方（`ASHIATO_EXPORT_DIR` → `LOCALAPPDATA` → `XDG_DATA_HOME` → `HOME`。環境を引数で受ける純関数）。
  本体はこの Task では「分類 → 列の突き合わせ → 空の zip を `.partial-….zip` に 0600 で作り `rename`」まで（中身は Task 5）。置き場が無ければ 0700 で作る（Unix。既にある置き場の権限は変えない。design D3）。
  合言葉は本体の引数（`Zeroizing<String>`。`Debug` で伏せる）として受ける。
  Scenario: `置き場の指定が無ければローカルのアプリのデータのフォルダの下に書く`（純関数に `LOCALAPPDATA` だけを持つ環境を渡し `…\ashiato\export` を返すこと、ドキュメント・デスクトップの下でないこと） /
  `置き場を環境で指定すればそこに書く`（一時フォルダを指定して本体を走らせ、その下にファイルができる） /
  `Unix では置き場と書き出しのファイルは持ち主だけが読み書きできる`（`#[cfg(unix)]`。まだ無い置き場を指して走らせ、`metadata().permissions().mode() & 0o777` が 0o700 と 0o600）。
  検証: `CT export_dir 3`
- [x] 2.2 `export::CATALOG` —— core のすべての表を `Include { file, view, excluded_columns, columns: [(名前, 意味)] }` か `Exclude { reason }` に分ける（design D7 の表と D8 の `Exclude` の一覧）。
  `Exclude` には ST12 の `archive_*` の 8 表も入れる（design D8）。`information_schema` を読んで、分類の無い表と、`Include` の表で分類の無い列を返す。
  Scenario: `いまの記録の schema の表と列はすべて分類されている`（いまの DB で 0 個） /
  `分類の無い表があると書き出しは始まらない` / `分類の無い列があると書き出しは始まらない`（**2.1 の差し替え口で、分類の段に分類の無い表 `x` / 列 `y` を足した一覧を渡して本体を走らせ**、
  失敗で終わること・`.` で始まらないファイルが無いこと・`.partial-…` も残らないこと・失敗の理由（台帳の `reason` は Task 6 で足す。ここでは本体の返す失敗）に名前があることを確かめる。共有のテスト DB に表を足さない）。
  検証: `CT export_catalog 3`
- [x] 2.3 列の突き合わせ（D9）—— `core.event` の出す列が `core.event_live` に、`core.event_version` の出す列が `core.event_version_live` に全部あるか。欠けていれば種別 `missing_column` と列の名前。
  Scenario: `いまの記録の表の列はすべて読み出しにある`（いまの DB で 0 個） / `読み出しに欠けた列があると書き出しは失敗する`（2.2 と同じく差し替え口で読み出しの列の一覧から 1 つ抜いて本体を走らせ、THEN を全部確かめる）。
  検証: `CT export_view_columns 2`

## Task 3: 除く範囲と、書き出す前の件数と見込み（design D5 / D5a / D5b / D12 / D18。`GET /export/summary`）

- [x] 3.1 除く範囲の SQL 断片（D5）—— **Q5〜Q8 と C19〜C21 を名指しで固定する**:
  - 消した時間 `erased_spans`: `core.stay_erased` から `Mark::of` と同じ読み方で、**利用者を問わずに**（Q7）引いたいまの版の時間と、その滞在の `core.event_version` の各版の時間（C19。読むのは `event_time` と `payload->>'end'` だけ。緯度経度を読まない）
  - 時刻の幅（D5b。Q6 / C21）: `END_KEYS`（`range_end` を含む。`end_time` の数字だけの値はミリ秒の UNIX 時刻 —— `stay_store::event_end_sql` と同じ。読めた終わりが `event_time` より前なら次の順へ）→ `DURATION_KEYS`（`visit_duration_us` ほか。`elapsed_ms` は入れない）→ `SPAN_RULES` の `Point`（`c01-location` / `c01-photo` / `c01-clock` / `s01-attribute`）なら点、それ以外（`c01-app-usage` を含む）は同じ `user_id`・`logical_source`・`device_id` の次の記録まで（無ければ写しの時点まで）。前の版の次の記録は design D5 の 3 の読み方
  - `erased_spans` は各問い合わせの `WITH … AS MATERIALIZED` に置く（写しは `READ ONLY` で一時表を作れない。design D5）
  - 緯度経度（D5a。Q8 / C20）: `NO_COORD_SOURCES`（理由をコメントで添える）・(2) は design の式（オブジェクトに絞ってから `keyvalue()`。**`silent` で黙らせない**）・(3) は design の表の 5 つの形（plus code だけ符号化を戻す前の文字列に当てる）
  - 前の版は「親が除かれたら除く」と「自分が条件を満たせば除く」の両方
  - 断片は「消した滞在をいつの時点で集めるか」を引数に持つ（D17 が写しの時点 `T` で使う）

  **この Task では断片を通した件数（そのテストの利用者の行）で形を固定する**。ファイルでの確かめは 5.4。
  加えて、(2) の式が数の値・配列・文字列だけの内容でエラーにならないこと、(3) の式が design D5a の表の例すべてに当たり `Chrome 128.0.6613.84`・`price 1,234.50 yen`・`2026/09/07 12:30` に当たらないことを DB で当てる。
  加えて、終わりが始まりより前の `end_time` を持つ行が (c) の幅で読まれることを 1 本で固定する。
  検証: `CT export_excluded_sql 19`（spec の「書き出しは本人が消した場面を出さない」の遅着を除く 16 本と同じ形を件数で 1 本ずつ ＋ (2)(3) の式の 2 本 ＋ 終わりが前の 1 本）
- [x] 3.2 `GET /export/summary` —— ソースごとの件数・いちばん古い時刻・いちばん新しい時刻（3.1 の断片で除いた残り。全利用者の和。件数 0 のソースは出さない）、登録簿の表示名、見込み `estimate`（D18）。資格情報なしは 401。
  Scenario: `書き出しの見込みの行の数は件数の和と一致する` / `資格情報の無い件数と状態の求めは断られる`（件数の側。状態の側は 6.4）。検証: `CT export_summary 2`
  （`書き出す前の件数は書き出した行の数と一致する` は書き出しの本体が要るので 5.4）

## Task 4: 遅れて届いた位置の印（design D6。`stay_store`）

- [x] 4.1 `stay_store::mark_late_for_user(pool, user)` を足す —— トランザクション → `lock(LOCK_KEY, user)` → 基準を読む →
  既存の `mark_late_arrivals` を、**`rebuild_day` と同じ並び `with_archive_sources(&c.sources)`** を渡して、本人が消した滞在の最小の始まり〜（最大の終わり＋1 マイクロ秒）で 1 回呼ぶ（design D6。整合の確認 2026-10-13 の K2）。
  本人が消した滞在が無ければ何もしない。何度走っても印と台帳の行は増えない。
  範囲の下端は消した滞在の最小の始まりの日（Asia/Tokyo）の始まり（design D6）。
  検証: `CT mark_late_for_user 5`（消した後に印の無い位置を `testdb::put_event` で直に入れ、呼ぶと `user:late` が付く / **最後の消した滞在のちょうど終わりの時刻の位置にも付く** / **印の無い書庫の位置（`c03-timeline-visit`）にも付く** / **消した滞在より前に始まり、終わりが消した時間に入る `c03-timeline-visit` にも付く** / 2 回目で台帳が増えない）、
  `CT late_arrival_is_marked 1`（ST22 のテストがそのまま緑。`rebuild_day` の振る舞いは変えない）

## Task 5: 書き出しの中身（design D4 / D7 / D9 / D19。`export.rs`）

この Task のテストは、書き出した zip を **`zip` crate の読み手に合言葉を渡して開き**（`ZipArchive::by_name_decrypt`。このシステムのコードを通さない）、中のファイルを読む。
テストの合言葉は 12 文字以上の固定の文字列で、テストの中だけに置く。

- [x] 5.1 写し（D4）: `REPEATABLE READ READ ONLY`、`SET LOCAL TimeZone = 'UTC'`、分類と列の突き合わせ → 表ごとの件数（`total`）→ 表ごとに zip の項目を AES-256・Deflate・`large_file(true)` で開き、`fetch`（stream）して 1 行ずつ書く（`fetch_all` を使わない。平文の一時ファイルを作らない）。
  1 行は `CATALOG` の列から組んだ `json_build_object(...)::text`（`raw` は文字列のまま、時刻は `to_char(… AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS.US"Z"')`）。前の版は表の列だけ（ビューが足す親の 3 列を出さない）。
  Scenario: `記録の行は記録の表のすべての列を持つ` / `前の版の行は前の版の表の列から内部の番号を除いた列を持つ` / `原文は受け取った文字列のまま出る` /
  `時刻は UTC で書かれ、ずれとタイムゾーンは列のまま出る` / `どのファイルのどの時刻の列も UTC で書かれる`（`information_schema` で時刻の型の列を引き、全ファイルの全行の値を見る） /
  `同じ内容の行は畳まずに出る` / `どの感度の記録も出る` / `前の版は親が書き出しに入るものだけ出る` / `読むための辞書と稼働の元データが出る`（ファイルがあり、件数が目録の `rows` と一致し、そのテストの利用者が足した行が入っている） /
  `生存信号から取得の回数と原文が出ない`。検証: `CT export_file_rows 10`
- [x] 5.2 zip（D3 / D7 / D19）: `.partial-ashiato-export-YYYYMMDD-HHMMSS.zip`（始まりの行を書く時点の名前）に書き、README と目録まで書いて注記 `ashiato-export <export_id>` を置いて閉じてから `rename`。同じ名前があれば `-2` …（`rename` の直前にも確かめ直す）。失敗したら `.partial-…` を消す。
  前のファイルを消すこと（D16）は台帳の行が要るので 6.2（spec-review R35）。
  Scenario: `書き出すと置き場の下に日時つきの zip ファイルが 1 つできる`（空の置き場で書き、置き場の下のファイルがその zip だけ。合言葉で開いて項目の名前の一覧が D7 の 12 個） /
  `書き出した zip のすべての項目は AES-256 で暗号化されている`（`ZipArchive::get_aes_verification_key_and_salt(i)` が全項目で `Some` で、`aes_mode` が `AesMode::Aes256`） /
  `書き終えていないファイルは書き出しの名前を持たない`（2.1 の差し替え口で書く途中に止めて置き場の名前を読み、続けて書き終えた後にも読む） /
  `合言葉が無ければ zip の中身を読めない`（`by_index` で合言葉なしに読むと全項目が暗号化の誤りで読めない・違う合言葉で `by_name_decrypt` が誤りを返す・`file_names()` と `comment()` に、そのテストで取り込んだ原文の目印の文字列と緯度経度の値が無い） /
  `書き出しの途中で記録の値を暗号化しないまま書かない`（**一時フォルダをテストごとの空のフォルダに向けて**（`TMPDIR` を本体の設定で差し替える）、遅延の差し替え口で書く途中に止め、置き場と一時フォルダの下の全ファイルの中身のバイト列に、取り込んだ原文の目印の文字列が無いこと。
  加えて、設定を経ずに書く経路を捕まえるため、プロセスの `std::env::temp_dir()` の下で書き出しの前後に増えたファイルにも目印が無いこと（書き出しのテストは直列なので測れる）。置き場の名前は `.` で始まるものだけ）。
  検証: `CT export_zip 5`
- [x] 5.3 説明書と目録（D9）: `README.txt`（日本語。D9 の 1〜5。1 に「合言葉で暗号化してある。7-Zip などの AES に対応した道具で開く」）と `manifest.json`（`rows` は書いた数）。どちらも zip の中。除いた件数をどこにも書かない。
  畳む規則の文面が `core.event_folded` の定義（`GROUP BY` の列）と食い違わないことをテストで突き合わせる。
  Scenario: `説明書にファイルごとの列の意味と件数が書かれている` / `出さない列は説明書の入らないものに名前で挙がる` / `目録の行の数はファイルの行の数と一致する` /
  `このシステムを使わずに中身を読める`（pool を閉じてから、`zip` crate の読み手に合言葉を渡して開き、`serde_json` だけで全ファイルの全行を読み、原文の値を取り込んだ文字列と比べる）。
  検証: `CT export_readme 4`
- [x] 5.4 除く範囲をファイルで確かめる —— spec の「書き出しは本人が消した場面を出さない」の Scenario 16 本（`消した時間に後から届いて印の無い位置には、書き出しの前に印が付く` は 6.1）を、
  書き出したファイル（そのテストの利用者の行。`別の利用者の識別子で入った同じ時間の位置も出ない` は 2 人目の利用者の行）で確かめる:
  `削除の印の付いた記録は出ない` / `消した滞在の時間に入る、印の無い別のソースの位置は出ない`（写真の位置） / `消した時間をまたぐ、別のソースの幅を持つ緯度経度つきの記録は出ない`（テストで登録簿に足した連鎖の掛からないソース） / `終わりをミリ秒の数字で持つ記録も、消した時間をまたげば出ない`（同じ） / `長さで幅を持つ記録が消した時間に重なれば出ない` / `終わりを持たない記録は、次の記録までの幅で重なれば出ない` / `消した滞在の前の版が持っていた広い時間も、消した時間に数える` / `別の利用者の識別子で入った同じ時間の位置も出ない` / `生きている滞在の前の版が消した時間に重なれば出ない` / `消した滞在の時間に入っても、緯度経度を持たない記録は出る` / `消した滞在の時間の外の位置は出る` / `緯度経度を持たないと決まっていないソースは、消した時間の中では出ない` / `緯度経度を持たないソースでも、内容に緯度経度があれば消した時間の中では出ない` / `原文に地図の座標を持つウィンドウの記録は、消した時間の中では出ない` / `短い小数・符号化した区切り・度分秒・plus code の座標も、消した時間の中では出ない` / `親が消した場面の規則で除かれると、前の版も出ない`。
  `台帳と導出の表は書き出しに入らない`（zip の中の項目の名前の一覧が spec の THEN の一覧と等しい）も足す。
  `除いた行の件数はどこにも出ない`（消した滞在があるときの zip の中の全ファイルに、除いた件数の数字も「削除」の件数の欄も無い。応答の側は 6.4）。
  `書き出す前の件数は書き出した行の数と一致する`（滞在を消した状態で `GET /export/summary` を読み、続けて書き出したファイルのそのソースの行の数と比べる）。
  検証: `CT export_file_excludes 19`
- [x] 5.5 写しの時点（C10）。2.1 の差し替え口で、写しを読み始めた後・書き終える前に記録を取り込み・滞在を消す。
  Scenario: `書き出しの途中で届いた記録はその書き出しに入らない`（取り込みは受理され、次の書き出しには出る） /
  `書き出しの途中で付いた削除の印はその書き出しに反映しない`（その書き出しには出て、次の書き出しには出ない） /
  `書き出しのファイルどうしが食い違わない`（前の版の `event_id` がすべて記録のファイルにあり、記録の `logical_source` がすべて登録簿のファイルにある）。
  検証: `CT export_snapshot 3`
- [x] 5.6 量 —— 書き出しを 1 回走らせ、かかった時間と最大メモリを `kind = "export"` のログに出す。**既定（手元・CI とも）は 2 万行**。
  記録は消せない共有のテスト DB に積み上がるので（spec-review R36）、1 年分（35 万行 / 利用者）は `ASHIATO_EXPORT_VOLUME_ROWS=350000` を渡したときだけ、使い捨ての DB（`docker compose` の DB に別の名前の database を作り、終わったら落とす）で走らせる。
  **除く判定が 1 年分で 5 分を超えたら**、`erased_spans` を写しの前の別のトランザクションで作る形を考える（写しの中では一時表を作れない。design D5）。検証: `CT export_volume 1`（`#[ignore]` にしない）

## Task 6: 口と背景の仕事と台帳（design D1 / D2 / D10 / D11。`lib.rs`）

- [x] 6.1 `App` に `exports`（D2）と、2.1 の設定・差し替え口を足す。`POST /export` は本体 `{"password"}` を自前で読み（axum の `Json` の既定の拒否は本体の断片を出しうる。design D1）、202 と `export_id`、走っていれば 409、資格情報なしは 401、
  合言葉が無い・12 文字（`chars().count()`。Unicode の符号位置）未満なら 400、問い合わせの引数か `password` 以外の鍵があれば 400。合言葉は spawn する本体の引数にだけ渡す（`Running` にも `App` にも置かない）。
  `ASHIATO_EXPORT_SELFTEST_DELAY_MS`（未設定は 0）をファイル 1 つごとに待つ。本体の順は「台帳の始まりの行 → 遅着の印（Task 4。失敗なら `late_mark`）→ 写し」（D4）。
  Scenario: `範囲を指定した書き出しの求めは断られる` / `書き出し中にもう一度求めると断られる` / `資格情報の無い書き出しの求めは断られる` /
  `合言葉の無い・短い書き出しの求めは断られる`（本体なし・`{}`・11 文字のそれぞれ。12 文字ちょうどは 202 —— 境界も 1 本。基本多言語面の外の文字（絵文字）11 個は 400・12 個は 202 も 1 本） /
  `消した時間に後から届いて印の無い位置には、書き出しの前に印が付く`（基準のソースの位置と書庫の位置（`c03-timeline-visit`）を印なしで直に入れ、書き出しの後に `GET /events` を読んでどちらも出ないこと、削除の台帳にそれぞれの `user:late` の行）。
  検証: `CT export_endpoint 5`
- [x] 6.2 台帳（D10）と前のファイルを消すこと（D16。Q4 第 2 回 / Q9）: 最初に `start` の行をコミット、終わったら `finish`（`done` / `failed`・表ごとの行の数・大きさ・`reason`・`done` なら `written`）。
  `done` の `finish` を書いた後で、台帳の `finish`（`done`）を持つ書き出しのうち今回以外で、その `file` のファイルが大きさ・`digest`（SHA-256）・zip の注記 `ashiato-export <export_id>` のすべてで合うものを消す（zip として読めない・どれかが合わないもの・`finish` を持たない止まった書き出しのファイルは消さない。design D16）。
  Scenario: `書き出すと台帳に始まりと終わりの行が積まれる` / `台帳の行は経路を持ち、記録の中身を持たない` /
  `失敗した書き出しも台帳に残る`（`late_mark` の段と分類の段の 2 つで、差し替え口から失敗させる。分類の段の `reason` に表の名前） /
  `書き終えると前の書き出しのファイルは消え、1 つだけ残る` /
  `書き出しが失敗すると前の書き出しのファイルは残る`（2.1 の差し替え口で 2 回目を失敗させ、1 回目のファイルの大きさと中身のハッシュを比べる） /
  `このシステムが書き出していないファイルは消さない`（`ashiato-export-19990101-000000.zip` と `my-notes.txt` を先に置き、1 回目の書き出しを `copy-of-export.zip` に写してから 2 回目） /
  `前の書き出しのファイルを本人が置き換えていれば消さない`（1 回目のファイルを、注記の違う zip で同じ名前に置き換えてから 2 回目。`leftover` にも出ない） /
  `前の書き出しのファイルに本人が項目を足していれば消さない`（1 回目の zip を `ZipWriter::new_append` で開いて項目を 1 つ足し（注記は残る）、2 回目。`leftover` にも出ない）。
  検証: `CT export_ledger_rows 8`
- [x] 6.3 `GET /export`（D11 / D16 / D17）: `running`（メモリ）/ `last`（`erased_after` を含む）/ `last_failure` / `interrupted` / `leftover`（台帳と置き場）。台帳の行と `running` から状態を作る部分は純関数にする。
  Scenario: `書き出しの間は進み具合が行の数で読める`（遅延の差し替え口で止めて読む） / `書き終えると前回の書き出しが読める`（`rows` は zip の中の各ファイルの行の和、`bytes` は zip ファイルの大きさ） /
  `1 度も書き出していなければそう返る`（**共有のテスト DB では台帳を空にできない**（追記のみ）ので、状態を作る純関数に「行が無い」を渡して固定し、口はその関数を通すだけであることを 6.3 の他のテストが確かめる） /
  `失敗しても前回の書き出しの値は変わらない` / `途中で止まった書き出しはそう返り、次の書き出しを始められる`（1 回書き終えた後、2 回目を差し替え口で `rename` の後・`finish` の前に止め、新しい `App` で読む。`interrupted.file` にそのファイル、`leftover` に出ない、3 回目を書き終えても止まったファイルは残る） /
  `書き出した後に記録を消すと、状態に後に消した記録の件数が出る`（滞在 1 件と連鎖で印の付いた位置の数。前回の書き出しのほかの値は消す前と同じ） /
  `後に消した記録の件数は、印の無い別のソースの行も数える`（写真の位置。書庫の位置は連鎖で印が付くので材料にしない） / `作り直しで置き換わった滞在は後に消した記録に数えない` /
  `書き出し直すと後に消した記録の件数は 0 に戻る` /
  `前のファイルを消せなければ、書き出しは書き終えたまま、状態にそのファイルが出る`（2.1 の差し替え口で前のファイルを消す段を失敗させる。台帳の `finish` は `done`、`leftover` にそのファイル、次の書き出しで消える）。
  検証: `CT export_status 10`
- [x] 6.4 応答に記録の値が無いこと・合言葉が残らないこと・資格情報。
  Scenario: `書き出しの口は記録の中身を返さない`（3 つの口の応答の文字列に、取り込んだ記録の原文・`lat` の値・本文が含まれない） /
  `合言葉はどこにも残らない`（目印の合言葉で書き出し、`core.export_ledger` の全列を `::text` で・3 つの口の応答（400 の応答も）・`core.access_log` の全列・テストの `tracing` の書き手が集めたログ（`tracing_subscriber` の `MakeWriter` で受ける）・置き場のファイルの名前・zip の注記に、その文字列が無い） /
  `資格情報の無い件数と状態の求めは断られる`（状態の側）/ `除いた行の件数はどこにも出ない`（応答の側）。
  検証: `CT export_responses 4`

## Task 7: OpenAPI と画面の在り処

- [x] 7.1 3 本の口を `#[utoipa::path]` と `ApiDoc` の `paths` / `components` に足し、`docs/openapi.json` を作り直す。検証: `tools/check-openapi.sh` rc=0
- [x] 7.2 `docs/screens.md` の表に `S-7 設定 | /#/settings | S-1 の見出しの行の「設定」` を 1 行足し、末尾の「まだ実装が無い面」から S-7 を外す。
  検証: `grep -q '#/settings' docs/screens.md`

## Task 8: 画面（design D13。`web/src/`。jsdom で測れるもの）

- [x] 8.1 `export.ts` —— 3 本の口の応答の型と読み手（**形が違えば失敗として出す**。`stays.ts` と同じ形）と、`POST /export` に本体 `{"password"}` を送る関数（合言葉を URL・保存領域に載せない）。
  検証: `cd web && npm run test -- export`（正しい応答を通し、壊れた応答を弾く単体テスト）
- [x] 8.2 `SettingsView.tsx` と `Root.tsx` の `#/settings`、`App.tsx` の見出しの行に「設定」のリンク。
  節の並びは D13（**Q3 第 2 回: 件数・期間 ＋「入らないもの」/ 確認をはさむ / 後に消した記録が N 件**）。開いたときに summary と状態を 1 回ずつ読み、`running` の間は 1 秒ごとに状態を読み直す。
  押すとその場に確認（見込み・暗号化した 1 つのファイル・合言葉は保存しない・AES の zip を開ける道具・「書き終えると前回の書き出しのファイルは消えます」、合言葉と「もう一度」の入力、「書き出す」「やめる」。design D13）。失敗・途中で止まった・消せなかったファイルは節の中に 1 行。
  ダウンロード・保存・`file://` のリンクを置かない。合言葉は state にだけ持ち、送ったら（失敗でも）空にして入力ごと外す。入力は `autocomplete="off"`、文字数は `Array.from(value).length`。
  `running` の間は「全記録を書き出す」を `disabled`。`POST` が 409 / 400 / 通信の失敗なら節に 1 行出して押す前の形に戻す。
  検証: `cd web && npm run test -- SettingsView`（件数と期間と入らないものの表示・押しただけでは `POST` しない・11 文字と不一致では「書き出す」が `disabled` で理由が出る・絵文字 11 個では `disabled`、12 個では押せる・409 で 1 行が出て入力が消える・`running` の間は押せない・12 文字の一致で「書き出す」を押すと本体に合言葉を持つ `POST` が 1 回だけ・「やめる」で `POST` しない・送った後に入力が無く `localStorage` / `sessionStorage` / `location.href` に合言葉が無い・`running` が消えたら読み直しが止まる・失敗と途中で止まったものと消せなかったファイルの 1 行・`erased_after` が 0 なら注記を出さず 1 以上なら出す）
- [x] 8.3 色は `tokens.ts` の値だけを使い、ライト / ダークのコントラスト比を `web/src/contrast.ts` で単体テストに固定する（画面の Scenario の印は 9.5 の e2e に置く）。
  検証: `cd web && npm run test -- settings-contrast`

## Task 9: e2e（本物のブラウザ。`web/e2e/settings-export.spec.ts`）

**画面の Scenario を「人間の確認待ち」へ逃がさない。** アサートするのは**数値と経路**（実寸・可視・フォーカス・URL・`scrollWidth`）。スクリーンショット比較は使わない。
e2e の合言葉は spec ファイルの中の固定の 12 文字以上の文字列（テスト専用）。
`tools/stack.sh` の起動に `ASHIATO_EXPORT_DIR`（worktree の中の gitignore された置き場。`.gitignore` に足す）と、**e2e のときだけ** `ASHIATO_EXPORT_SELFTEST_DELAY_MS=400` を渡す
（確認バッチの `run.sh` には遅延を渡さない）。消した後は同じテストの中で戻し、DB を元の状態に返す（後続のテストが同じ DB を見る）。

- [x] 9.1 `tools/stack.sh` に置き場と遅延を足す。`settings-export.spec.ts` の `beforeAll` で **遅延が効いていることを確かめ、効いていなければ理由つきで落とす**
  （合言葉を添えた `POST /api/export` の直後の `GET /api/export` に `running` が出ること。確かめた後は **`running` が消えるまで待ってから**次へ進む（9.2 が 409 に当たらないように。spec-review R45）。手元の `run.sh` の stack を `reuseExistingServer` で使い回すと遅延が 0 で、進み具合の Scenario が揺れるため。spec-review R19。
  落ちたときの文言に「run.sh を止めるか CI=1 で走らせる」を出す）。`web/e2e/offsite.spec.ts` の `VIEWS` に `/#/settings` を足す（data-sensitivity の既存の Scenario。印は動かさない）。
  検証: `cd web && npm run test:e2e -- offsite`
- [x] 9.2 入口・件数・押す・進み具合・離れて戻る・前回の書き出し。
  Scenario: `設定画面に全記録の書き出しの節がある`（S-1 のリンクから。URL が `#/settings`） / `押す前にソースごとの件数と期間が出る`（画面の件数を `GET /api/export/summary` と突き合わせる） /
  `押す前に入らないものの一覧が出る`（3 行があり、その一覧の中に数字が無い） /
  `押すとその場に確認が出て、合言葉を入れて書き出すを選ぶと進み具合が出る`（押した直後に `POST /api/export` が送られていないこと、確認の文に見込みと「この PC のフォルダ」「暗号化した 1 つのファイル」「合言葉は保存しません」「AES」「前回の書き出しのファイルは消えます」があること（前回の書き出しがある状態で）、`page.on('request')` で捕まえた `POST /api/export` が 1 回だけで本体の `password` が入れた合言葉であること、「書き出す」の後に進み具合） /
  `合言葉が短いか確かめと一致しなければ書き出すを押せない`（11 文字の一致・12 文字の不一致で「書き出す」が `toBeDisabled`、理由の 1 行が可視、`POST /api/export` が 0 回） /
  `書き出しを始めると合言葉は画面に残らない`（「書き出す」の後に `input[type=password]` が 0 個、`page.content()`・`page.url()`・`localStorage` と `sessionStorage` の全値に合言葉が無い） /
  `合言葉の入力はブラウザに保存を促さない`（2 つの入力の `autocomplete` 属性が `new-password` でも `current-password` でもない、「書き出す」の前後で `page.url()` が同じ） /
  `確認でやめると書き出しは始まらない`（`page.on('request')` で `POST /api/export` が 0 回） /
  `画面を離れて戻っても進み具合が出る`（`#/day/2026-09-07` へ移って戻る） /
  `書き終えると前回の書き出しが新しくなる`（行の数を `GET /api/export` と突き合わせる）。
  検証: `ET 'Scenario 9.2' 10`（各テストの名前の頭に `Scenario 9.2:` を付ける）
- [x] 9.3 台帳の状態で決まる表示 —— **台帳が空・失敗・途中で止まった・消せなかったファイルの状態は共有の stack では作れない**（台帳は追記のみ）ので、`page.route` で `GET /api/export` を
  `last: null` / `last_failure` あり / `interrupted` あり / `leftover` ありの応答に差し替えて、画面の文言と「全記録を書き出す」が押せることを確かめる（サーバの側は 6.3 が固定する）。
  Scenario: `まだ書き出していなければそう出る`（押して出た確認に「前回の書き出しのファイルは消えます」が無い） / `失敗した書き出しと途中で止まった書き出しは節に出て、押し直せる` /
  `前の書き出しのファイルを消せなかったら節に出る`（パスと「次の書き出しでもう一度消します」が出る） /
  `書き出しが走っている間は全記録を書き出すを押せない`（`running` ありの応答で、ボタンが `toBeDisabled` で進み具合が可視） /
  `書き出しの求めが断られたらそのことが出て押す前の形に戻る`（`page.route` で `POST /api/export` に 409 を返し、1 行が可視・`input[type=password]` が 0 個・「全記録を書き出す」が押せる）。
  `失敗した書き出しと途中で止まった書き出しは節に出て、押し直せる` の途中で止まった側は `interrupted.file` ありの応答でパスが出ることも見る。検証: `ET 'Scenario 9.3' 5`
- [x] 9.4 書き出した後に消すと件数が出る —— 合言葉を入れて書き終えた後に `#/day/2026-09-07` で滞在を 1 件消し、設定画面を開き直して「前回の書き出し」の欄に「この書き出しの後に消した記録が N 件あります。書き出したファイルの中には残っています。」が出て、
  N が `GET /api/export` の `last.erased_after` と一致し 1 以上であること。最後に戻す。
  Scenario: `書き出した後に記録を消すと、前回の書き出しの欄に件数が出る`。検証: `ET 'Scenario 9.4' 1`
- [x] 9.5 操作と下限 —— 360 px の幅で「全記録を書き出す」と確認の合言葉の入力 2 つ・「書き出す」「やめる」の `boundingBox` の幅と高さが 24 以上、Tab で移ってフォーカスの輪郭（`outline` の幅が 0 でない）、Enter で確認が出て
  フォーカスが合言葉の入力に移り（`document.activeElement` が 1 つ目の `input[type=password]`）輪郭が見え、`keyboard.type` で合言葉、Tab で確かめの入力へ移って輪郭が見え、同じ合言葉を打って Enter で進み具合が出る、
  `document.documentElement.scrollWidth <= clientWidth`（ファイルのパスが出ていて確認を開いた状態で）、操作できる要素（`a, button, [download]`）にダウンロード・保存・`file:` が無い。
  `page.emulateMedia({ colorScheme })` でライトとダークを切り替え、節の文字（見出し・件数・前回の書き出し）の計算後の文字色と背景色からコントラスト比を出して 4.5 以上。
  Scenario: `書き出すの操作は 24 px を下回らない` / `キーボードで書き出すを押せてフォーカスの位置が見える` / `設定画面は 360 px の幅で横にスクロールしない` /
  `画面から書き出したファイルを受け取る操作は無い` / `設定画面の文字はライトでもダークでも 4.5:1 を下回らない`。検証: `ET 'Scenario 9.5' 5`

## Task 10: 仕上げ

- [ ] 10.1 検査 3 本。検証: `python3 scripts/check_scenarios.py . st33-data-export` rc=0、
  `python3 scripts/review_triage.py . st33-data-export` rc=0、`python3 scripts/check_chain.py .` rc=0
- [ ] 10.2 まとめて緑。検証: `cargo fmt --all --check` / `cargo clippy --workspace --all-targets -- -D warnings` /
  `cargo test --workspace` / `cd web && npm run lint && npm run test && npm run build && npm run test:e2e` /
  `tools/check-immutable.sh` / `tools/check-offsite.sh` / `tools/check-log-private.sh` / `tools/smoke.sh` が全部 rc=0
- [ ] 10.3 `docs/handoff/` を PR の前にもう一度読む（開始時と合わせて 2 回）。`docs/handoff/ST33.md` の**全部の節**（R2・2026-10-11・2026-10-12 の整合の確認（C1 / C3 / C9 / C5））と、`docs/audit/2026-10-13-st33-merge-consistency.md`（C1 / C2 / K1 / K2 は spec・design・tasks に直接写した）を読む。ST33 宛ての st22-record-deletion R2 が、
  5.4 の Scenario（`削除の印の付いた記録は出ない` ほか）で閉じていることを確かめる。
  検証: `grep -c 'st22-record-deletion' docs/handoff/ST33.md` が 1 以上、かつ `grep -c 'Scenario: 削除の印の付いた記録は出ない' crates/server/src/export_tests.rs` が 1 以上

## 人間の確認待ち

**無し。** 画面の Scenario は 9 章の e2e（本物のブラウザ）が担保し、サーバの Scenario は Rust のテストが担保する。
書き出したファイルを「このシステムを使わずに読める」ことも、DB を閉じてから `zip` crate の読み手に合言葉を渡して開き、汎用の JSON の読み手で読むテスト（5.3）が担保する。
7-Zip などの道具で開けることは、同じ形式（WinZip AE-2）の読み手が開けることで担保する（道具そのものを CI に入れない）。
機械が再現できない物理（ロック・電池・本物の GPS・時間そのもの・実機）に当たるものが、この Story には無い。
確認バッチの手順書が Story ごとに 1 問聞く「触ってみて違和感は無かったか」だけが人間に渡る。
