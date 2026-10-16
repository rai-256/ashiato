## Why

**Takeout の YouTube のマイアクティビティ（本物の書庫で 118,895 件）は、いまファイルごと「視聴」と見分けられる。**
ST12 の見分け（`URL を持つ最初の項目`）が `watch?v=` を見つけた時点でファイル全体を視聴にし、読み手は視聴でも検索でもない項目と広告まで視聴へ落とす。
印を置けば検索 5,643 件・広告 19,770 件・投票や評価などが視聴として入り、**論理ソースは内容の鍵の入力なので後から直せない**。
置かなければ 1 件も入らない（確認バッチ 20261006-0754 の ST12 13.1。`docs/verify/20261006-0754-fix.md` の (1)）。
同じ規則で、他の製品のマイアクティビティも先頭の項目が YouTube の動画なら丸ごと視聴になりうる（深掘り Q6 / review R1）。

いま直す理由は、**本人がまだ YouTube の形に印を置いていない（か、置いたかを覚えていない）うち**に見分けを正すと、凍結される行が出ないから。

## What Changes

深掘りは **第 1 回 6 問**（すべて A。5 問が rewrite-all、1 問が premise）。本人がすべて選択肢から選んだ（推奨のまま・未回答 0）。
**当初案を覆したのは Q1**（ST12 design D3 の「2 経路とも別々のソース」を、同じソース・同じ分け方へ）。

- **YouTube のファイルは製品の名前で見分ける**（Q6）。マイアクティビティの項目の最初の製品が「YouTube」のファイルを YouTube のファイルとし、URL を持つ最初の項目では決めない。
  製品側の `watch-history.json` / `search-history.json` も同じ種類として読む（Q1）
- **YouTube のファイルの項目は 1 件ずつ行き先を決める**（Q1〜Q4）。マイアクティビティ経由でも製品側でも同じ規則:
  1. `details` の欄を持つ項目（広告）→ その他（Q3）
  2. `watch?v=` の URL を持ち、題名が評価の決まり文句（日本語・英語の表）に当たる項目 → その他（Q4）
  3. `watch?v=` の URL を持つ項目 → 視聴 `c03-youtube-watch`
  4. `results?search_query=` の URL を持つ項目 → 検索 `c03-youtube-search`
  5. それ以外 → その他 `c03-myactivity-youtube`（他の製品のマイアクティビティと同じ名付け。見出し「マイアクティビティ: YouTube」。Q2）
- **形の確認**: YouTube の種類の形の同一性に**製品の名前**と**評価の決まり文句が当たった言語**を入れる（Q4・Q6）。
  表が 1 件も当たらない言語のファイルは確認待ちに戻り、その形には印を置かせない（表に言語を足して版を上げるまで入らない）。YouTube 以外の種類の形の同一性は変えない（印は効いたまま）
- **形の確認の一覧に行き先ごとの件数**（視聴 / 検索 / その他。その他のうち広告・評価）を出す。値は出さない（C3）
- **その他のソース `c03-myactivity-youtube` を移行で登録簿に足す**（想定間隔 60 日・粒度 `none`。C2）
- **解析器の版を上げる**（C4）。確認待ちの書庫は写しから形を取り直し、読み終えた書庫の新しい形に印の無いファイルは黙って飛ばさず確認待ちにする。旧い形の印では新しい分類器の読みを通さない。読み直しても YouTube 以外の論理ソースの行は増えない（同じソースの中の内容の鍵）
- **見分け方を変える前の YouTube の形（`{"kind":"YouTubeWatch","products":[]}`）に置いた印は、新しい形に引き継がない**（C6。印の表は追記のみのまま）
- **試験の材料を本物の形に合わせる**（C5。「URL を持たない視聴＝削除された動画」の材料。本物では URL の無い視聴は全部が広告）

**BREAKING**: 形の確認の一覧の `kind` の値が変わる（`YouTubeWatch` / `YouTubeSearch` → YouTube の 1 種類）。読むのは本人と `tools/smoke.sh` だけ。
`/ingest`・`/archives/status`・`/coverage` の応答の形は変えない（`/archives/status` の `sources` に `c03-myactivity-youtube` が 1 本増える）。

## 深掘りで本人が決めたこと（2026-10-13。1 回・6 問）

全文は `deep.md`。**下流はこれを勝手に変えない。**

| # | 決めたこと | 段 |
|---|---|---|
| Q1 | マイアクティビティの YouTube の視聴・検索は **製品側と同じ `c03-youtube-watch` / `c03-youtube-search`** に入れ、**製品側のファイルにも同じ分け方を当てる**（ST12 D3 を覆す） | A |
| Q2 | 視聴でも検索でもない項目は **他の製品と同じ規則で `c03-myactivity-youtube`** に入れる | A |
| Q3 | 広告（`details` を持つ項目）は **視聴から外し、その他** に入れる。Q4 より先に当てる | A |
| Q4 | 評価は **決まり文句の表（日本語・英語）で視聴から外し**、**表が 1 件も当たらない言語のファイルは確認待ちに戻す** | A |
| Q5 | YouTube の形に印を置いたかは **覚えていない**（確認バッチの DB を確かめきれていない。答えに依らず決まることは C6） | A（premise） |
| Q6 | YouTube のファイルかどうかは **製品の名前で決め、形のハッシュにも製品の名前を入れる** | A |

## Capabilities

### New Capabilities

（なし）

### Modified Capabilities

- `external-ingestion`: YouTube の項目の行き先の規則を足す（ADDED）。形の確認の印の Requirement を、YouTube の形の同一性（製品の名前・言語）と一覧の行き先ごとの件数を含むように書き換える（MODIFIED）

**capability の照合**: `docs/stories/INDEX.md` は `external-ingestion` を ST12 / ST13 / ST47 に割り当てている。この Story が書く振る舞いはすべて書庫の取り込みで、表の割当と合う。前倒しは無い。
稼働状況の格子（`collection-coverage`）の判定・並び・高さは変えない（書庫のソースの格子は ST12 が `external-ingestion` に置いた条項のまま。行が 1 本増えるだけ）。

## Impact

- **コード**: `crates/server/src/archive/classify.rs`（種類の見分け）・`archive/worker.rs`（`takeout_items` の行き先・`shape_for_file` / `hash_shape`）・`archive/youtube.rs`（決まり文句の表）・`archive/mod.rs`（`PARSER_VERSION`）
- **移行**: 1 本（`c03-myactivity-youtube` の登録。名前は作成時刻）
- **試験と道具**: `archive_tests.rs` / `archive_flow_tests.rs` の YouTube の材料、`tools/archive-shape.sh` の `--confirm`（言語の当たらない YouTube の形を断る）、`tools/smoke.sh` の形の一覧の照合（`YouTubeWatch` の文字列）と材料の題名、`docs/archive-inbox.md` の手順
- **走っている Story との重なり**（差し戻さない）:
  - **ST25**（`st25-day-timeline`。下流）: design と spec の Scenario（`書庫の YouTube の視聴履歴とマイアクティビティの YouTube に、同じ時刻の同じ動画の記録がある日の記録を求める`）は、2 経路が別々のソースに入る ST12 D3 を前提にしている。
    この Story の後は、マイアクティビティの YouTube の視聴も `c03-youtube-watch` に入る。原文が違えば同じソースの中に 2 行になり、ST25 の「論理ソースが違えば 2 件」の規則には当たらない。
    ST25 の振る舞い（論理ソースごとに返す）は変わらず、ST25 の試験は合成の行で書かれているので壊れない。2 経路の畳み方は読む側（ST25 の後）の話のまま
  - **ST08**（`desktop-collection`）・**ST17**・**ST21**: 触る表・capability は重ならない
- **申し送り**: `docs/handoff/ST13.md` の `st12-archive-ingestion R28`（マイアクティビティの見分けが先頭 1 件の `titleUrl` に依存）と同じ所を、この Story が作り直す
  （製品の名前を持つ項目の集合で決めるので、並びに依らなくなる。design D1）。ST13 の申し送りの文書は書き換えない（宛先は ST13。ST13 の上流が読んで閉じる）
