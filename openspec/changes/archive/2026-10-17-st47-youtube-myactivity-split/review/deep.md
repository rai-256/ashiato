# ST47 deep-review（deep-questions.json の独立検査）

問いの一覧を見る前に schema の手順 1〜5 をやり直し、その後で突き合わせた。
件数は本物の書庫（`マイ アクティビティ/YouTube/マイアクティビティ.json` と、同じ Takeout の他の製品のマイアクティビティ）を
**形だけ**数えたもの（URL は経路の型・題名は末尾 / 先頭の決まり文句・`details` は欄の有無と `name` だけを数え、題名・検索語・URL の値は出していない）。

## 手順ごとの記録

- 手順 1（要件どうしの衝突）: FR-14（印を置くまで格納しない）・FR-16・FR-17（内容の鍵 = 論理ソース + 時刻 + 原文。`crates/server/src/ingest.rs:178-193`）を読んだ。
  要件どうしの直接の衝突は無いが、**問いの推奨どうし・note と問いの間**に食い違いが 2 件ある（R2・R3）
- 手順 2（扉の幅）: 該当なし（確かめた範囲: `docs/stories/ST47.md` の `doors: []`、「触れる扉」の節が空であること）
- 手順 3（新たに立つ一方通行）: 論理ソースは内容の鍵の入力（`ingest.rs:189`）なので、項目の振り分けの規則はすべて rewrite-all。
  Q1〜Q4 が拾っている。抜けは「言語が変わったときに印が効かない」（R4）と「他の製品のファイルが YouTube と見分けられる」（R1）。
  原文の型（`raw` は書庫のバイト列の切り出しの `text`。`worker.rs:886-891`）と payload の欄（原文から作り直せる）は問わなくてよい
- 手順 4（日常に影響する選択）: 新しい格子の行は Q2・Q3 の選択肢の detail に書かれている。広告を除いた「その他」の項目 3,595 件は、連続する 2 件の間の最大の空きが 54 日で、
  想定間隔 60 日（note で据え置き）でも格子が「途絶」で埋まることは無い。指摘なし
- 手順 5（既存コードが要件を満たしていない箇所）: `classify.rs` の `kind_of` と `worker.rs` の `hash_shape` で R1。note の件数は検算でおおむね合う
  （全 118,895・`watch?v=` 109,334・広告 19,770・検索 5,643・URL の無い項目 303・投票 376・回答 196・広告のリンク先 220・位置 0）。合わないのは R2 の 2 点

## R1. 他の製品のマイアクティビティも、YouTube の視聴として丸ごと見分けられうる（抜け。kind_of の「URL を持つ最初の項目」の規則と、YouTube の形の同一性に製品の名前が無いこと）
- 成果物: openspec/changes/st47-youtube-myactivity-split/deep-questions.json
- 根拠: `crates/server/src/archive/classify.rs:88-98`（URL を持つ最初の項目が `watch?v=` を含めば、ファイル全体を `YouTubeWatch` にする）/ `crates/server/src/archive/worker.rs:767-775`（`YouTubeWatch` のファイルでは、視聴でも検索でもない項目が視聴へ落ちる）/ `worker.rs:1142`（`products` を形に入れるのは `MyActivity` だけ）/ `worker.rs:1402-1412`（`hash_shape` は `kind` と `products` だけ）。
  本物の書庫で `watch?v=` の URL を持つ項目の数: マイアクティビティの「動画検索」973 件中 628 件（65 %）・「検索」98,471 件中 1,404 件・「Chrome」16,424 件中 2,397 件（`search_query=` も 84 件）。今回の書庫では、どれも URL を持つ最初の項目が YouTube でなかったので `MyActivity` に分かれた。
  項目は新しい順に並ぶので、次の書き出しで直近の操作が YouTube の動画を開いたものなら、そのファイルは丸ごと `YouTubeWatch` になる。そのときの形は `{"kind":"YouTubeWatch","products":[]}` で、YouTube のマイアクティビティや製品側の `watch-history.json` と**同じハッシュ**になる。YouTube の形に一度印を置けば、確認待ちにならずに黙って `c03-youtube-watch` に入る
- kind: technical
- loss: rewrite-all
- 提案: 本人に問う形にはしない。note の「聞かないで決めること」に足し、design の D 番号に残す。YouTube かどうかは URL ではなく項目の `products[0]` で決め（本物の書庫では YouTube のファイルの 118,895 件がすべて `YouTube` で、他の製品のファイルには 1 件も無い）、YouTube の種類の形にも製品の名前を入れる。この Story の `kind_of` の作り直しで同時に直る範囲。
- 処置: escalated — Q6（kind: irreversible / loss: rewrite-all）として本人に問う。deep.md の R1 / Q6

## R2. Q4 の前提が誤り。URL の無い「〜を視聴しました」98 件は削除された動画ではなく、全件が広告で、Q3 と Q4 の推奨が逆を指す
- 成果物: openspec/changes/st47-youtube-myactivity-split/deep-questions.json
- 根拠: deep-questions.json:100（Q4 の context「削除された動画の視聴（URL が無い。98 件）がその他に入る」）、:106（推奨「URL の無い『〜を視聴しました』は視聴へ」）。
  本物の書庫で URL が無く題名が「を視聴しました」で終わる 98 件は、**98 件すべてが `details`（`name` は「Google 広告から」だけ）を持つ**。広告でない URL の無い視聴は 0 件。
  削除された動画は URL を持ったまま題名に決まり文句が無い形で、`watch?v=` の 210 件の側にいる。`crates/server/src/archive_tests.rs:2047` の「URL を持たない＝削除された動画」という材料（final review R57）は、本物の形と合っていない。
  Q4 の「高評価・低評価・保存（約 200 件）」のうち、`watch?v=` を持つのは「高く評価しました」190 件・「低く評価しました」6 件で、「保存しました」の 8 件はどれも `watch?v=` 以外の URL（URL だけの判定でも、もともとその他に入る）
- kind: premise
- 提案: Q4 の context と選択肢の detail を本物の形に直す。98 件は Q3 で行き先が決まるので、Q4 の選択肢の差は約 196 件の評価だけになる（「URL だけ」の短所として書かれている「URL の無い視聴 98 件がその他に入る」は、Q3 の推奨のもとでは短所にならない）。Q3 と Q4 の規則のどちらを先に当てるかを、どちらかの問いに書き込む。
- 処置: fixed deep-questions.json — note と Q3・Q4 の context を本物の形に直した。URL の無い 98 件は広告として Q3 に入れ、Q3 を Q4 より先に当てると Q3 に書いた。Q4 は評価 196 件だけの問いにした。`archive_tests.rs:2047` の材料は spec の段で tasks に拾う（deep.md の C5）

## R3. Q1 の推奨「経路の見分けが要らない」は、note の「製品側の watch-history.json 単体の読み方は変えない」と Q3・Q4 の推奨を一緒に採ると成り立たない（ST12 D3 を覆すことも書かれていない）
- 成果物: openspec/changes/st47-youtube-myactivity-split/deep-questions.json
- 根拠: deep-questions.json:4（note「製品側の `watch-history.json` 単体の読み方は変えない」）/ :19（Q1 の推奨の detail「書庫の中での経路の見分けが要らない」）/ Q3 の推奨（広告を視聴から外す）・Q4 の推奨（決まり文句の表で外す）/
  `worker.rs:767-775`（製品側の読み方: 視聴でも検索でもない項目は見分けた種類へ落ち、広告も視聴に入る）/ Q1 の context 自身の「両者は項目の形がほぼ同じ」/ `openspec/changes/st12-archive-ingestion/design.md:117`（D3「同じ出来事を 2 経路で持つ。どちらも入れる」）。
  製品側の読み方を変えずに Q3・Q4 をマイアクティビティだけに当てるには、2 つのファイルを書庫の中で見分ける必要がある。これは Q1 の推奨が「要らない」と書いている見分けそのもの。
  見分けずに両方へ当てるなら、note の「変えない」が誤りで、製品側の広告と評価も視聴から外れる。
  どちらも採らないと、`c03-youtube-watch` に、製品側から来た広告は入り、マイアクティビティから来た広告は入らない、という状態のまま凍結される
  （製品側の `watch-history.json` に広告が入るかは、この repo にも本物の書庫にも実物が無く確かめていない）
- kind: conflict
- loss: rewrite-all
- 提案: Q1 に「分ける規則（Q2〜Q4）を製品側の `watch-history.json` にも当てるか」を足す（選択肢: 両方に当てる＝note の「変えない」を外す / マイアクティビティだけ＝経路の見分けが要る）。あわせて、推奨が ST12 D3（経路ごとに入れる）を覆すことを context に書く。
- 処置: escalated — Q1 の選択肢を「同じソース＋製品側にも同じ分け方を当てる」/「別ソース＋製品側は変えない」に組み直し、推奨が ST12 D3 を覆すことを context に書いて本人に問う。note の「製品側の読み方は変えない」を消した。deep.md の R3 / Q1

## R4. Q4 の推奨の「表に無い言語では URL だけの判定に戻る」は黙って凍結される。言語が変わったら確認待ちに戻す選択肢が無い（FR-14 の印が効かない）
- 成果物: openspec/changes/st47-youtube-myactivity-split/deep-questions.json
- 根拠: deep-questions.json:106（推奨の detail「表に無い言語では URL だけの判定に戻る（約 200 件が視聴に混ざる側へ倒れる）」）/ docs/requirements.md:255-257（FR-14「形の確認の印を置くまで格納しない」。理由は「振り分けの規則が本物の書庫で外れていると、既に入った行は直せない」）/
  ST12 design D16 の第 3 回 Q12（「書き出しの言語が変わって `products` の値が『検索』→ `Search` になった」は確認待ちに戻すと本人が決めた）/ `worker.rs:1402-1412`（形の同一性は `kind` と `products` だけ）。
  YouTube の `products` は訳されない名前（本物の書庫の 118,895 件がすべて `YouTube`）なので、書き出しの言語が変わっても形のハッシュは変わらない。題名の決まり文句で振り分ける規則を入れると、言語が変わって規則が外れても印は効かず、外れた行の行き先が黙って凍結される
- kind: irreversible
- loss: rewrite-all
- 提案: Q4 に「決まり文句の表で 1 件も当たらない（表に無い言語の）ファイルは確認待ちに戻す（形に『表が当たった言語』を入れる）」という選択肢を足すか、推奨の detail を「確認待ちに戻る」に変える。Q1 の選択肢「URL だけ」を選んだ場合はこの問題が起きないことも書く。
- 処置: escalated — Q4 に「表が当たらない言語のファイルは確認待ちに戻す」を足して推奨にし、黙って URL に戻る案は別の選択肢にして本人に問う。deep.md の R4 / Q4

## R5. Q5 は Story の完了の判定で既に決まっていて、残る幅は表示だけ（不要）
- 成果物: openspec/changes/st47-youtube-myactivity-split/deep-questions.json
- 根拠: docs/stories/ST47.md:56（完了の判定「`tools/archive-shape.sh` の一覧で視聴・検索・その他が分かれて見える」＝推奨の選択肢そのもの）/ Q5 の why（表示だけで、後から変えられる）
- kind: defer
- 提案: 問いから外し、C（「行き先ごとの件数を出す。その他の内訳は出さない」を D 番号に残す）にする。人間の時間を使わない。
- 処置: fixed deep-questions.json — Q5（表示の問い）を外し、行き先ごとの件数を出すことを note の「聞かないで決めること」と deep.md の C3 に移した

## R6. 前提「この形にはまだ印を置いていない」は機械で確かめられるうえ、確認バッチの記録には印を置いた形跡がある
- 成果物: openspec/changes/st47-youtube-myactivity-split/deep-questions.json
- 根拠: deep-questions.json:4（note「前提: この形にはまだ印を置いていない…置いていたら補足に書いてください」）/ docs/verify/20261006-0754-fix.md（2026-10-08「それまでこの形には印を置かない」）の後に、docs/verify/20261006-0754-answers-2.txt（2026-10-12）で 13.1 が「通った」になった。
  13.1 の手順（`openspec/changes/st12-archive-ingestion/tasks.md:256-259`）は `--confirm` で印を置くこと。印は `core.archive_shape_confirmation` の `shape_hash` で、この形のハッシュは `sha256('{"kind":"YouTubeWatch","products":[]}')` = `f3e578ff3482bbe4ecd78fb117f501e733af26a5a654a4cd82ab0d1a11bdedc8`（`worker.rs:1402-1412` と同じ作り方を手で計算）。確認バッチの DB（`tools/stack.sh` の `ashiato2-db-1`）は今は動いていないので、引いてはいない
- kind: premise
- 提案: 問う前に確認バッチの DB でこのハッシュの印と `c03-youtube-watch` の件数を引き、結果を note に書く。印があれば「誤って視聴に入った行をどうするか」を A の問いとして立てる（印の表は追記のみで（ST12 D16）、外す道具は無い。`grep -rn "DELETE FROM core.archive_shape_confirmation" crates tools` は 0 件）。
- 処置: escalated — 動いている DB（`docker exec ashiato2-st12-db-1 psql …` と ashiato2-integ-db-1）ではハッシュ f3e578ff… の印は 0 件。確認バッチの DB は動いておらず確かめきれないので、Q5（kind: premise）として本人に聞く。deep.md の R6 / Q5

