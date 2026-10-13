# ST47 実装タスク — YouTube のマイアクティビティを視聴・検索・その他に分けて取り込む

読む順: `deep.md`（**最優先。本人が決めたこと**）→ このファイル → `specs/external-ingestion/spec.md` → `design.md`
→ `docs/stories/ST47.md` → `docs/handoff/`（開始時と PR 前の 2 回）→ `CLAUDE.md`。

触るのは `crates/server/src/archive/`（`classify.rs` / `worker.rs` / `youtube.rs` / `mod.rs`）・移行 1 本・試験（`archive_tests.rs` / `archive_flow_tests.rs`）・`tools/smoke.sh`・`docs/archive-inbox.md`。
**画面（`web/`）と `/ingest`・`/archives/status`・`/coverage` の応答の形には触らない**（proposal の Impact）。
検証は各タスクの本文に書いてある。「動いた」ではなくコマンドと終了コードで判定する。DB を使う検査は `docker compose up -d db` が前提。

## Global Constraints（規律。**最初に読む**）

- **テストには `Scenario: <名前>` の印を置く。** Rust はコメント（`// Scenario: 広告は視聴に入らない`）、bash は `echo`。
  `scripts/check_scenarios.py` が spec の全 Scenario と突き合わせ、**印の無い Scenario を FAIL にする**。印の名前は spec の `#### Scenario:` と**一字一句合わせる**
- **この change が足す Scenario は 22 本**（ADDED の Requirement の 16 本 + MODIFIED の Requirement に足した 6 本）。MODIFIED で写した既存の 12 本は既存の印を生かす（名前を変えない）
- **件数つき検証**: `cargo test <絞り込み>` は一致するテストが 0 本でも rc=0 になる。このファイルで **`CT <絞り込み> (≥N)`** と書いたものは、
  `bash -o pipefail -c 'cargo test -p ashiato-server <絞り込み> 2>&1 | tee /tmp/ct.log' && grep -Eq 'test result: ok\. [0-9]+ passed' /tmp/ct.log && test "$(cargo test -p ashiato-server <絞り込み> -- --list 2>/dev/null | grep -c ': test$')" -ge N`
  が rc=0 になることを指す（N 本以上あり、すべて通る）。走らせるのは repo の根から `scripts/quiet-run`（AGENTS.md の「長いコマンド」）
- **本人の決定（下流は変えない）**: マイアクティビティの YouTube の視聴・検索は製品側と同じ `c03-youtube-watch` / `c03-youtube-search`、製品側にも同じ分け方（Q1）/
  視聴でも検索でもない項目は `c03-myactivity-youtube`（Q2）/ 広告（`details` の欄を持つ）はその他、評価の判定より先（Q3）/
  評価は日本語・英語の決まり文句の表で視聴から外し、表が 1 件も当たらない言語のファイルは確認待ちに戻す（Q4）/ YouTube のファイルは製品の名前で見分け、形のハッシュに製品の名前を入れる（Q6）
- **聞かずに決めたこと（C1〜C6）と D の対応**: C1 → D11 / C2 → D8 / C3 → D7 / C4 → D9 / C5 → D10 / C6 → D5。
  **D2 / D4 / D6 は（仮）決め。** 反転条件は `design.md`。変えたらその D 番号を書き直す
- **YouTube 以外の種類の形のハッシュを 1 バイトも変えない**（design D5）。`hash_shape` が `MyActivity` / `ChromeHistory` などに作る同一性の JSON は、いまと同じ `{"kind","products"}` のバイト列のまま。変えると本人が置いた印がすべて効かなくなる
- **ログに出すのは件数・論理ソースの名前・書庫の中のファイルの種類・所要時間・失敗の種別だけ**（ST12 の最後の Requirement）。題名・URL・検索語は出さない。形の一覧（`destinations` / `languages`）にも値を入れない
- **本人の書庫を commit しない。** 試験の材料は合成で、本物の形（`header` / `title` / `titleUrl` / `subtitles` / `time` / `products` / `activityControls`、広告は `details`）に合わせて手で書く
- **Takeout の書庫の中身を格納する試験では、`core.archive_shape_confirmation` に合成の書庫の形を入れておく**（`archive_flow_tests.rs` の `confirm`。入れないと 0 件になる）
- **移行の名前は作成時刻 `YYYYMMDDHHMM_<slug>.sql`**（と `.down.sql`）。`crates/server/src/lib.rs` の `MIGRATIONS` の末尾へ

## Task 1: その他のソースを登録する移行（design D8）

- [ ] 1.1 移行 1 本で `core.source` に `('c03-myactivity-youtube', 'マイアクティビティ: YouTube', 5184000, 'none')` を `ON CONFLICT (logical_source) DO NOTHING` で足し、`MIGRATIONS` の末尾に載せる。
  down は `c03-myactivity-youtube` の記録が 1 件も無いときだけ登録を消す。見出しの文字列は `worker::ensure_myactivity_source` が作るものと同じにする。
  Scenario: `その他のソースは 60 日で登録されている`。
  検証: `tools/check-migrations.sh` rc=0、`CT archive_youtube_other_source_registered (≥1)`（移行を当てた DB で書庫を置かずに登録簿を読み、見出し・`expected_gap_sec = 5184000`・`external_id_kind = 'none'` を見る。
  想定間隔を 30 日に変えてから移行を当て直しても 30 日のままであることも見る）

## Task 2: YouTube のファイルの見分けと、項目の行き先（design D1 / D2 / D3 / D4 / D10）

- [ ] 2.1 `KnownKind::YouTubeWatch` / `YouTubeSearch` を `KnownKind::YouTube` の 1 つにし、`classify.rs` の `kind_of` を design D1 の順（最初の製品の名前の集合がちょうど `{"YouTube"}` → `YouTube` / 製品の名前を持つ項目があれば `MyActivity` / 製品の名前を持つ項目が無く視聴・検索の URL があれば `YouTube`）にする。
  `requires_shape_confirmation` と見張り（`every_archive_logical_source_is_classified_as_location_or_item`）の材料を新しい種類に合わせ、見張りの材料が 3 つの行き先（視聴・検索・`c03-myactivity-youtube`）をすべて通るようにする（design D11）。
  検証: `CT classify (≥2)`（見張りが通ること。加えて `kind_of` の単体試験: 先頭が `watch?v=` の製品「検索」のファイルが `MyActivity`、製品「YouTube」だけのファイルが `YouTube`、製品が混ざったファイルが `MyActivity`、製品の名前を持たず `watch?v=` を持つファイルが `YouTube`）
- [ ] 2.2 `archive/youtube.rs` に決まり文句の表（design D4（仮）: 日本語は末尾、英語は先頭の一致。評価と、言語の見分けだけに使う視聴・検索）と、項目の題名から「評価か」「当たった言語」を返す関数を置く。
  `worker.rs` の `takeout_items` の `YouTube` の枝を、項目ごとに spec の順（`details` の欄の有無 → `watch?v=` かつ評価 → `watch?v=` → `results?search_query=` → その他）で行き先を決める形に書き直す。
  その他は `c03-myactivity-youtube` で、payload は ST12 のマイアクティビティと同じ欄（`product` / `title` / `url` / `details`）。視聴・検索の payload は ST12 のまま（design D3）。
  `archive_tests.rs` の「URL を持たない＝削除された動画」を視聴に入れる材料と試験を、本物の形（URL の無い項目はその他）に書き直す（design D10 / C5）。
  Scenario: `マイアクティビティの YouTube の視聴は視聴のソースに入る` / `マイアクティビティの YouTube の検索は検索のソースに入る` / `視聴でも検索でもない YouTube の項目はマイアクティビティの YouTube に入る` /
  `広告は視聴に入らない` / `URL の無い広告もその他に入る` / `高く評価した動画は視聴に入らない` / `英語の書き出しの評価も視聴に入らない` / `URL の無い YouTube の項目はその他に入る` /
  `製品側の視聴履歴にも同じ分け方が当たる` / `2 経路の同じ視聴は同じ視聴のソースに入る` / `先頭が YouTube の動画の他の製品のファイルは YouTube に入らない` / `YouTube と他の製品が混ざったファイルは製品ごとのソースに入る`。
  検証: `CT archive_youtube_route (≥12)`（`archive_flow_tests.rs` の流れで、形の印を置いてから合成の Takeout の書庫を置き、`core.event` の論理ソースごとの件数を見る。Scenario 1 本に試験 1 本以上。
  「入らない」側の論理ソースの件数が 0 であることも見る）、`CT youtube (≥3)`（決まり文句の表の単体試験: 日本語の評価 2 種・英語の評価 2 種が当たり、動画名の途中に「を高く評価しました」を含む視聴の題名は当たらない）、
  `CT archive_ (≥1)` で ST12 の既存の書庫の試験がすべて通る

## Task 3: 形の同一性と、形の一覧の行き先ごとの件数（design D5 / D6 / D7）

- [ ] 3.1 `shape_for_file` が `YouTube` の種類にも `products`（最初の製品の名前の集合）を入れ、`languages`（design D4 の当たった言語の集合）と `destinations`（`watch` / `search` / `other` と、`other` のうち `ad` / `rating` の件数）を足す。
  `hash_shape` は `YouTube` の種類だけ `{"kind","products","languages"}` を同一性にし、それ以外の種類はいまと同じバイト列の `{"kind","products"}` のままにする（`destinations` はハッシュに入れない）。
  `tools/archive-shape.sh` は変えない（形の JSON をそのまま出す）。言語の当たらない形にも印を置ける（design D6（仮））。
  Scenario: `形の確認の出力に YouTube の行き先ごとの件数が出る` / `決まり文句の表が当たらない YouTube のファイルはまた確認待ちになる` / `他の製品のファイルは YouTube の形の印では格納されない` / `見分け方を変える前の YouTube の形に置いた印は効かない`。
  検証: `CT archive_youtube_shape (≥4)`（Scenario ごとに 1 本以上。旧い形は `{"kind":"YouTubeWatch","products":[]}` のハッシュを試験の中で計算して印を置く）、
  `CT archive_shape_hash_is_unchanged_for_other_kinds (≥1)`（`MyActivity`（製品「検索」）と `ChromeHistory` の形について、`hash_shape` の結果が、試験の中に文字列で書いた `{"kind":"MyActivity","products":["検索"]}` / `{"kind":"ChromeHistory","products":[]}` の sha256 と一致する）、
  `CT archive_shape_confirm (≥1)`（既存の試験に、題名「京都 旅館 を視聴しました」の YouTube の項目を含む合成の書庫で `tools/archive-shape.sh` の出力に「京都」が無く、`destinations` と `languages` があることを足す）

## Task 4: 解析器の版を上げ、読み直す（design D9）

- [ ] 4.1 `archive::PARSER_VERSION` を `"2"` にする。版が上がったとき、確認待ちのファイルの形を写しから取り直し、`core.archive_pending_shape` の旧い版の行を新しい形の行に置き換える（ST12 D8 の `read` の行の読み直しはそのまま）。
  Scenario: `解析器の版が上がると確認待ちのファイルは新しい形で一覧に出る` / `読み直しても YouTube 以外の論理ソースの行は増えない` / `YouTube 以外の形に置いた印は効いたままになる`。
  検証: `CT archive_youtube_reparse (≥3)`（旧い版の値を試験の中で台帳と確認待ちの表に入れて版の上げを再現し、確認待ちの一覧の形が新しい形だけになること /
  製品「検索」・Chrome・`Timeline.json` を含む書庫を読み終えた後の版の上げで、それらの論理ソースの件数が変わらないこと / 製品「検索」と Chrome の形の印が版の上げの後も効くこと）、
  `CT archive_flow_ (≥1)` で ST12 の流れの試験がすべて通る（`解析器の版が上がると読み直される` を含む）

## Task 5: 位置の印と稼働状況の最終日（design D11）

- [ ] 5.1 `c03-myactivity-youtube` が位置の印付け（`stay_store` の登録済みの `c03-myactivity-*`）の対象に入っていること、分けた 3 つの論理ソースの最終日が `/archives/status` の `sources` に出ることを試験で固定する（コードを足す必要が無ければ足さない）。
  Scenario: `消した滞在の時間帯の位置を持つ YouTube のその他の項目は削除済みになる` / `分けた論理ソースの最終日が稼働状況に出る`。
  検証: `CT archive_youtube_location (≥1)`（滞在を消してから、同じ時間帯の時刻で `locationInfos` を持つ広告と持たない広告を含む合成の YouTube のマイアクティビティを印を置いて格納し、前者だけに `deleted_by = 'user:late'`）、
  `CT archive_youtube_last_event (≥1)`（`last_event_on` が 3 つの論理ソースでそれぞれ spec の日付）

## Task 6: 縦串の道具と本人の手順書（design D10）

- [ ] 6.1 `tools/smoke.sh` の形の一覧の照合（`grep -q 'YouTubeWatch'`）を新しい種類の文字列に合わせ、smoke の書庫の材料に製品「YouTube」の視聴の項目を足して、印を置いた後に `c03-youtube-watch` の最終日が出ることを見る（いまの照合を弱めない）。
  `docs/archive-inbox.md` の手順に、YouTube の形の一覧の読み方（`products`・`languages`・`destinations`。`languages` が空なら評価を見分けられないこと）を足す。
  検証: `scripts/quiet-run smoke -- tools/smoke.sh` rc=0、`grep -q 'destinations' docs/archive-inbox.md && grep -q 'languages' docs/archive-inbox.md` rc=0

## Task 7: まとめの検査

- [ ] 7.1 検証: `cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings` rc=0、`scripts/quiet-run test -- cargo test --workspace` rc=0
- [ ] 7.2 検証: `python3 scripts/check_scenarios.py . st47-youtube-myactivity-split` rc=0（この change の全 Scenario に印）
- [ ] 7.3 検証: `python3 scripts/check_chain.py .` rc=0、`openspec validate st47-youtube-myactivity-split --strict` rc=0、
  `tools/check-migrations.sh` / `tools/check-openapi.sh` / `tools/check-boundaries.sh` / `tools/check-immutable.sh` / `tools/check-licenses.sh` がすべて rc=0
- [ ] 7.4 PR 本文に **仮決め（D2 / D4 / D6）と反転条件**、**確認バッチの DB の印を確かめる手順**（design の Risks の 1 つ目）を列挙する。
  検証: `body=$(gh pr view --json body -q .body); for d in 2 4 6; do grep -q "D${d}（仮）" <<<"$body" || { echo "D${d} が無い"; exit 1; }; done; grep -q 'f3e578ff' <<<"$body"` rc=0
- [ ] 7.5 `docs/handoff/` を読み直す（開始時と PR 前の 2 回）。
  検証: `test ! -f docs/handoff/ST47.md || { body=$(gh pr view --json body -q .body); grep -oE '^## [a-z0-9-]+ R[0-9]+' docs/handoff/ST47.md | sed 's/^## //' | sort -u | while read -r h; do grep -qF "$h" <<<"$body" || { echo "PR 本文に無い: $h"; exit 1; }; done; }` rc=0

## 人間の確認待ち

**機械で確かめられないのは「違和感」と、本物の Google の書き出しだけ**（2026-09-14 の決定）。正しさは Task 1〜6 の試験が持つ。

- [ ] 8.1 本物の Takeout の書庫（YouTube のマイアクティビティを JSON で含むもの）を専用のフォルダに置き、`tools/archive-shape.sh` の一覧で YouTube の形が
  `products: ["YouTube"]`・`languages: ["ja"]` で出て、`destinations` が視聴 約 8.9 万・検索 約 5,600・その他（うち広告 約 2 万・評価 196 前後）に分かれて見えるかを見る。
  印を置いた後、稼働状況の書庫のソースの格子に視聴・検索・「マイアクティビティ: YouTube」の 3 本の最終日が出るかを見る（design D4（仮）の反転条件の確かめ。合成の材料は Google の実物の題名の言い回しを再現できない）
  > 物理: realdata —— 本人の本物の書き出しの題名の言い回しと件数は、合成では再現できない
- 確認バッチ（`/verify`）の手順書が、この Story について「触ってみて違和感は無かったか」を 1 問だけ聞く
  （見るもの: 形の一覧の YouTube の行き先ごとの件数と、稼働状況の書庫のソースの格子の 3 本）
