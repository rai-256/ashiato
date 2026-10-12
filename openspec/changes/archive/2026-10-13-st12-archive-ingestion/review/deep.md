# ST12 深掘りの独立レビュー

対象: `openspec/changes/st12-archive-ingestion/deep-questions.json`（Q1〜Q5。A 2 / B 3）と、
その下書き `deep.md`（C1〜C14・「確かめたが問わなかったこと」）、Q5 の `proto.html`。
schema の `deep` 手順 1〜5 を、一覧と下書きを開く前に独立にやり直し、突き合わせた差分だけを書く。
**問いの JSON は触っていない。**

実施日: 2026-09-15。ブランチ `docs/st12-upstream`（clean）。

## 実測・確認の方法

| | やったこと | 結果 |
|---|---|---|
| grep | `crates/server/src/lib.rs:770-779`（`apply_external_update`） | 届いた更新時刻が保存済みより古いと `SkippedStale` を返すだけで、**本表にも `core.event_version` にも書かない** |
| grep | `crates/server/src/ingest.rs:161-191`（`content_hash`） | 鍵は `logical_source` + `event_time` + `raw` の**文字列そのまま**。169 行のコメントが「収集側の直列化は決まった形なので実害は無い」を前提にしている |
| grep | `migrations/202609120944_gates.sql:43,50,59` | 収集した行の `origin` / `external_id` / `logical_source` の書き換えを拒む。`tz_id` / `tz_offset_min` / `device_id` は**凍結しない**と 66-70 行が明記 |
| grep | `tools/check-immutable.sh:108` / `:378-384` | `origin` の付け替えと `external_id` / `external_ref` の書き換えが拒まれることを検査している |
| grep | `migrations/202609120942_dedup_indexes.sql` | 重複の索引はどちらも `(user_id, logical_source, …)`。**ソースをまたいだ重複は検出しない** |
| grep | `crates/server/src/coverage.rs:28-30` / `lib.rs:1365` | 稼働状況の API は `must_sources()` の 5 本だけを返す（下書きの手順 5 と一致） |
| grep | `crates/server/src/ingest.rs:138` | `origin = collected` は `device_id` が空だと断る |
| grep | `archive` `takeout` `inbox` `zip` を `crates/ web/src migrations tools` で | 書庫を扱うコードは無い（ヒットは無関係の `zip()` と Takeout に触れたコメントだけ） |
| 実行 | 共有 DB（`ashiato2-db-1`）で凍結列を UPDATE して確かめようとした | **実行していない**（権限で拒否。共有 DB に書く手順だったので妥当）。上の gates.sql と check-immutable.sh の読みで代えた |

## 手順ごとの記録（一覧を見る前）

- **手順 1（要件どうしの衝突）**: FR-14 / FR-16 / FR-17 / FR-55 に、全記録に掛かる FR-18 / FR-21〜FR-25 / FR-30 / FR-50 / FR-78 / NFR-3 / NFR-12 を突き合わせた。
  見つけた衝突: (a) FR-17（過去のエクスポートを置いたら取り込む）× FR-22（古い到着では書き換えない）× FR-18（原文をそのまま保存）——
  古い書庫の原文が**どこにも残らない**（→ R2）。
  (b) FR-17「既存の記録との重複」× FR-22「ソースごとに判定」× 同じ期間を持つ 2 形式（移行前のロケーション履歴と Timeline.json）（→ R10）。
  (c) FR-78（収集が有効なソースは想定間隔ごとに稼働を残す）× 書庫のソースに生存信号の担い手が無い（→ R4）。
  (d) FR-16「最終日」の読み（→ Q4 にある）
- **手順 2（扉の幅）**: Story の doors は空だが、全記録に掛かる扉のうち幅が残るもの ——
  扉 #8（由来。Google が推定した訪問は「収集した」か → R5）、扉 #12（外部サービス上の識別子。**サービスが持たない識別子を作って入れてよいか** → R1）、
  扉 #7（原文をそのまま。書庫の 1 項目の「そのまま」は何か → R3）、扉 #14（書庫のソースの「動いていた / 壊れていた」→ R4）、扉 #6（地域 → R11）
- **手順 3（新たに立つ一方通行）**: 捨てる —— 書庫そのもの（→ Q2 にある）、古い書庫の原文（→ R2）、読まなかった製品（→ R6）。
  変換する —— 1 項目の原文の切り出し方（→ R3）。鍵の入力 —— 外部識別子の作り方（→ R1）、論理ソースの粒度（→ R10）、由来（→ R5）。
  取らない —— 取り込みの結果と書庫の素性（→ R4）、Google の推定した訪問の継続分（→ R7）。
  外に出す —— 端末から PC へ運ぶ経路（→ R8）
- **手順 4（日常）**: 2 か月ごとの手数と置き場（→ Q3 にある）、端末から PC へ運ぶ手順（→ R8）、
  マップのタイムラインを今後も書き出すか（→ R7）、ダウンロードのフォルダの再走査（→ R9）、書庫の写しの容量（→ Q2 にある）、画面（→ Q5 にある）
- **手順 5（既存コードと要件）**: 書庫を扱うコードは無い（grep）。FR-16 の列も無い。
  **動いているが要件を満たさない**もの —— 古い到着の原文を捨てる（`lib.rs:778-779` → R2）、
  内容の鍵が原文の表記に依存し、Google 側の書き出しの表記は本システムが決められない（`ingest.rs:169` → R3）、
  重複の索引がソースをまたがない（→ R10）、稼働状況の API が 5 本しか返さない（→ Q5 にある）

---

## R1. C3（時刻 + 対象から記録ごとの鍵を作る）は ST03 の本人が退けた案そのもので、しかも「後から作り直せる」の根拠が凍結で崩れている
- 成果物: openspec/changes/st12-archive-ingestion/deep.md
- 根拠: openspec/changes/archive/2026-09-14-st03-idempotent-ingest/deep.md:493-502（Q25 の答えは「追わない」。**選ばれなかった「対象の識別子 + 出来事の時刻を鍵にする」は、同じ対象・同じ時刻の別々の記録が畳まれて消える不可逆を持っていた**。同 500 行「書庫の重複の扱いは ST12 が実物を見てから決める」）/ migrations/202609120944_gates.sql:50（収集した行の `external_id` は書き換え不可）/ tools/check-immutable.sh:378-384 / docs/requirements.md:953（「重複判定のキーの作り方は扉ではない」は 2026-09-07 の記述で、`external_id` を凍結した ST03 より前）/ docs/requirements.md:212（FR-23 は「外部サービス上の識別子」を持たせる。サービスが持たない値を作って入れることは書いていない）/ docs/requirements.md:385-387（FR-50: 削除済みは内容の鍵で止める）
- kind: irreversible
- loss: rewrite-all
- 提案: C から外し A の問いに立てる。選択肢は「作った鍵を外部識別子に入れる（C3）」「内容の鍵だけで判定する（ST03 Q25 の既定。書き出しの表記が変わると行が増える）」など。各選択肢に、(1) 鍵の作り方を後から変えると凍結した `external_id` を全行書き直すか、書き直さずに同じ出来事が 2 行並ぶ、(2) 変えた後の到着は FR-50 の削除の印を素通りしうる、(3) Timeline の訪問は Google が区切りや場所を直すと時刻・場所の識別子が動くので C3 の「行は増えず前の版が履歴に残る」が成り立たない、を書く。「書庫の作られた時刻」を Timeline.json でどこから取るか（ファイルの更新時刻は写すと動く）も併記する。
- 処置: escalated — 指摘を容れて C3 を問いから外し、Q6（A / rewrite-all）として問いに上げた。選択肢に「内容の鍵だけ（ST03 の既定）」「作った鍵 + 書庫の作られた時刻」「作った鍵 + 届いた順」を置き、鍵の凍結・古い書庫の版の喪失・タイムラインの訪問の区切りが動くことを書いた。deep.md の Q6 と C3 に R1 を記録

## R2. 古い書庫を後から置くと（FR-17）、その書庫の原文は D-01 のどこにも残らない。Q2 の前提「どれを選んでも D-01 に残る」は誤り
- 成果物: openspec/changes/st12-archive-ingestion/deep-questions.json
- 根拠: crates/server/src/lib.rs:770-779（`incoming < known` で `SkippedStale` を返し、履歴にも本表にも書かない）/ deep.md の C3 (b)「過去の古い書庫を後から置いても新しい内容を巻き戻さない」（書庫の作られた時刻を更新時刻にするので、古い書庫の到着は全部 `stale` になる）/ deep-questions.json:52（Q2 の context「記録 1 件ごとの原文（その項目の JSON 片）は、どれを選んでも D-01 に残る（FR-18）」）/ docs/requirements.md:183-187（FR-18）/ docs/requirements.md:204（FR-22「古い到着では既存行を書き換えない」—— 書き換えないとは言うが、捨てるとは言っていない）
- kind: conflict
- loss: discarded
- 提案: FR-18 × FR-22 の衝突として A の問いに立てる（例: 古い到着の原文を履歴に「過去の版」として足す / 捨てる / 書庫の写しにだけ残る）。Q2 の context の一文を「古い書庫の中身と、題名が変わる前の内容は、写しを残さない選択肢では失われる」に直し、Q2 の選択肢 3 / 4 の「何が不可逆か」にも足す。
- 処置: escalated — Q2 の context の誤った一文を直し（古い書庫の内容の違う版は、Q6 で更新時刻を持つ側を選ぶと履歴にも残らない）、選択肢 3 / 4 の不可逆に足した。FR-18 × FR-22 の選び方そのものは Q6 の選択肢 2 / 3 の違いとして問う。deep.md の Q2 / Q6 に R2 を記録

## R3. 書庫の 1 項目の「原文そのまま」をどう切り出すかが問われていない。切り出し方は原文のバイト列と鍵の入力を同時に決める
- 成果物: openspec/changes/st12-archive-ingestion/deep-questions.json
- 根拠: crates/server/src/ingest.rs:165-169（鍵は原文の文字列そのまま。「同じ内容でも表記が違えば別の鍵になるが、収集側の直列化は決まった形なので実害は無い」—— 書庫の表記は Google 側が決めるので前提が成り立たない）/ migrations/202609092315_raw_text.sql（`raw` を `jsonb` から `text` へ直した経緯）/ docs/requirements.md:851（扉 #7）/ deep-questions.json:52（Q2 の context が「その項目の JSON 片」を暗に前提にしている）/ deep-questions.json の Q2 context「予約エクスポートは毎回全期間を出す」
- kind: irreversible
- loss: discarded
- 提案: A の問いを足す（または C として根拠つきで明記する）。選択肢は「ファイルのバイト列から項目の範囲をそのまま切り出す」「解析して直列化し直した JSON」「ファイル丸ごとを 1 行」など。各選択肢に (1) 直列化し直すと元のバイト列は書庫を消した時点で戻らない、(2) `raw` は鍵の入力なので後から切り出し方を変えると同じ項目が別の鍵になる（凍結した行は直せない）、(3) 書き出しの表記が 1 字でも変わると、全期間を出す予約エクスポートのたびに**全行ぶんの行か版が積む**（外部識別子を持つなら履歴、持たないなら新しい行）、を書く。
- 処置: escalated — 問いにはしない（切り出したバイト列は後から直列化し直せるが逆はできない = 扉を開けたままにする既定）。deep.md の C15 に R3 つきで記録し、表記の変化で鍵が変わる代償は Q6 の選択肢 1 の detail に書いた。本人へは C の一覧で見せる

## R4. 書庫のソースの「稼働記録」にあたる取り込みの台帳を残すかが無い。Q5 の context「書庫から入るソースは生存信号を送らない」は FR-78 と衝突する
- 成果物: openspec/changes/st12-archive-ingestion/deep-questions.json
- 根拠: docs/requirements.md:290-291（FR-78「WHILE あるソースの収集が有効である THE SYSTEM SHALL そのソースに登録された想定間隔ごとに…稼働記録に残す」—— 書庫のソースを除いていない）/ docs/requirements.md:879-881（扉 #14 の区別）/ deep-questions.json:136（Q5 の context）/ deep.md の C4「読めなかった項目は件数と場所を取り込みの結果に残し」・C5「読んだかをファイルのハッシュで覚えない」・C12（最終日は書庫の作られた時刻を持つ）—— 結果をどこに・書き換え禁止で残すかは書いていない / crates/server/src/ingest.rs:138（収集した行は `device_id` を要求するが、書庫の行に何を入れるか・どの書庫から来たか（FR-24 の provenance）が決まっていない）
- kind: conflict
- loss: uncaptured
- 提案: FR-78 × 書庫のソースの衝突を問いに立てるか、C として「置かれた書庫ごとに、書庫の素性（ファイルのハッシュ・名前・作られた時刻）・読んだ時刻・入った / 既にあった / 古かった / 読めなかった件数を、追記のみの台帳に残し、それを書庫のソースの稼働記録とする」を足す。残さなかった取り込みの結果は後から作れない（書庫を消せば、どの行がどの書庫から来たかも、読めなかった項目があったことも分からない）。Q5 の context はこの答えに合わせて直す。
- 処置: escalated — 問いにはしない（台帳は追記のみ / 取らないと後から作れない）。deep.md の C16（書庫ごとの台帳）と C17（取り込み器が 1 日 1 回、書庫の論理ソースごとに生存信号を送る）と C18（端末識別子の欄）に R4 つきで記録し、Q5 の context を直した。本人へは C の一覧で見せる

## R5. Google が推定した訪問を「収集した」にする C11 は扉 #8 の幅で、由来は凍結されるので問いに上げる
- 成果物: openspec/changes/st12-archive-ingestion/deep.md
- 根拠: docs/requirements.md:857-859（扉 #8「混ざると AI が推定した値と本人が書いた値を区別できず、成功条件 2 を壊す」）/ docs/requirements.md:222（FR-25 は 3 分類のみで、外部サービスが推定した値の置き場を定めていない）/ migrations/202609120944_gates.sql:43（収集した行の由来は変えられない）/ tools/check-immutable.sh:108 / deep-questions.json の Q1 option 1 の detail「Google が推定した訪問を含む」/ docs/requirements.md:256（FR-76 の滞在は本システムが導く派生で、同じ時間帯に Google の訪問と並ぶ）
- kind: irreversible
- loss: rewrite-all
- 提案: A の問いを足す（「収集した」/「派生させた」/ 論理ソースを分けたうえで「収集した」、など）。各選択肢に「由来は収集した行では DB が書き換えを拒むので、後から変えるには全行の書き直しが要る」「『派生させた』にすると FR-30 の凍結の外に出る」「AI（A-01）が本システムの滞在と Google の訪問を同じ重みで読む」を書く。Q1 でマップのタイムラインを選ばなければ要らない問いなので、Q1 の答えに条件づけてよい。
- 処置: escalated — 問いにはしない。「派生させた」は FR-31 が本システムが原文から作り直せるものと定めていて、Google の推定は作り直せないので選択肢として成り立たない。扉 #8 の区別（本人が書いた値との区別）は崩れず、Google の推定であることは論理ソースの名前（C1）で分かる。理由を deep.md の C11 に R5 つきで書き、本人へは C の一覧で見せて異論を番号で受ける

## R6. Q1 を B（仮でよい）にした理由は Q2 の非推奨の選択肢に依存していて、推奨どおりに答えると選ばなかった製品は失われる
- 成果物: openspec/changes/st12-archive-ingestion/deep-questions.json
- 根拠: deep-questions.json:11（Q1 の why「選ばなくても、書庫の写しを残す（Q2）なら後から読み直せるので仮でよい」）/ deep-questions.json:57 以下（Q2 の推奨は「**読んだ製品のファイルだけ**写しを残す」、irreversible「読まなかった製品は、本人が書庫を消すと戻らない」）/ docs/requirements.md:809（EXT-B: 書庫は約 7 日で失効、ダウンロードは 5 回まで）
- kind: irreversible
- loss: discarded
- 提案: Q1 に `loss: discarded` を付けて A にするか、Q1 の why を「Q2 で『丸ごと写す』を選んだときだけ仮でよい。それ以外では、選ばなかった製品は書庫を消した時点で戻らない」に直し、各選択肢の「何が不可逆か」を書く。
- 処置: escalated — Q1 に loss: discarded を付けて A にし、why を「Q2 で丸ごと写すときだけ後から読める」に直し、各選択肢に不可逆を書いた。deep.md の Q1 に R6 を記録

## R7. 「マップの履歴は過去分を 1 回入れる用途」を本人に問わず事実として置いている。Google の推定した訪問は C-01 からは作れない
- 成果物: openspec/changes/st12-archive-ingestion/deep-questions.json
- 根拠: deep-questions.json:12（Q1 の context「これから先の位置は C-01 が集めるので、マップの履歴の取り込みは過去分を 1 回入れる用途になる」）/ docs/requirements.md:30（§1.2「現状は Google マップ履歴のみを月 2 回程度」）/ docs/requirements.md:256 と :380（本システムの滞在は半径と時間だけで作り、場所は本人が登録する —— Google の場所の同定や移動手段の推定は作らない）/ reference/requirements.md:721（「端末買い替えで取れなくなる」「移行期間中に Takeout していなかった利用者が過去データを失った」）/ deep.md の C13（端末から書き出すタイムラインも想定間隔 60 日で登録）× docs/requirements.md:276-280（FR-35 は退役していないソースを 3 倍で通知する）
- kind: daily
- loss: uncaptured
- 提案: B / daily の問いを足す（「今後もタイムラインを定期的に書き出す（手作業が増える）」「過去分を 1 回だけ（それ以降の Google の訪問・移動手段は取らない。端末を替えると戻らない）」）。「1 回だけ」なら C13 の 60 日を置くと ST14 で通知が鳴り続けるので、取り込み後に退役させる等を C13 に足す。
- 処置: escalated — Q7（A / daily / uncaptured。推奨: 2 か月ごと）として問いに足し、Q1 の context の「過去分を 1 回入れる用途」を削った。C13 を Q7 の答えに従う形に直した

## R8. 端末で書き出した Timeline.json を PC の置き場まで運ぶ経路が問われていない。経路によっては位置の履歴が端末と自宅 PC の外を通る
- 成果物: openspec/changes/st12-archive-ingestion/deep-questions.json
- 根拠: 呼び出し元が確かめた一次情報 https://support.google.com/maps/answer/6258979 の逐語 "Select your preferred storage location. Tap Save."（保存先は本人が選ぶ）/ deep-questions.json:85（Q3 の context「端末から PC へ運ぶ手順は置き場を変えても残る」—— C-01 経由の選択肢を示さずに残ると決めている）/ docs/requirements.md:32（§1.2 データ所有を最優先にする理由は漏洩リスク）/ docs/requirements.md:756（S-01 / D-01 / D-02 はオンプレ、クラウドに置かない）/ docs/requirements.md:582（NFR-12 の手作業の上限）
- kind: daily
- loss: exported
- 提案: Q3 に選択肢を足すか、別の daily の問いを立てる（「本人が USB / 共有フォルダで運ぶ」「C-01 が端末上の書き出しファイルを拾って S-01 へ送る」「クラウドのドライブを経由する」）。クラウドを経由する選択肢の「何が不可逆か」に「位置の全履歴が一度外へ出る。出た分は戻らない」を書く。
- 処置: escalated — Q8（A / daily / exported。推奨: 網の外に出ない手段で本人が運ぶ）として問いに足した。C-01 から送る機能は device-collection を ST04 の下流が触っているので、選ばれても後の change にする旨を context に書いた

## R9. ダウンロードのフォルダの扱いが Q3 の context と C5 / C8 / C14 と Q2 の推奨で食い違い、推奨どおりに組むと大きな書庫を数分おきに読み直し続ける
- 成果物: openspec/changes/st12-archive-ingestion/deep-questions.json
- 根拠: deep-questions.json:85（Q3 の context「読んだかどうかはファイルの中身のハッシュで覚える」）/ deep.md の C5「ファイルのハッシュで『読んだ』を覚えて読み飛ばす形にすると…→ もう一度読む」/ C8「置き場は数分おきに見る」/ C14「ダウンロードのフォルダのファイルは動かさない」/ deep-questions.json:57（Q2 の推奨「書庫は『取り込み済み』へ移す」）/ deep-questions.json の Q2 context（写真を含む書庫は 1 回で数十 GB）
- kind: conflict
- 提案: 「読んだ書庫を覚えるか」を 1 つに決めて Q3 の context と C5 を揃える（例: 覚えて読み飛ばし、読み直しは明示の操作にする）。Q2 の推奨の「移す」がダウンロードのフォルダには効かないことを Q2 か Q3 に書く。失われるものは無いので問いは増やさなくてよい。
- 処置: escalated — 失われるものは無いので問いは増やさない。C5 を「ハッシュと解析器の版で覚え、版が上がったときだけ読み直す」に直し、Q3 の context を揃え、Q2 の context に「移すのは専用のフォルダだけ」を足した。design で D 番号（仮）にする。deep.md の Q2 / Q3 / C5 に R9 を記録

## R10. C1「製品ごとに 1 本」は、記録の種類で分ける既定と FR-23 に反する。移行前の形式と Timeline.json が同じ期間を持つとき、FR-17 の重複はソースをまたいで検出されない
- 成果物: openspec/changes/st12-archive-ingestion/deep.md
- 根拠: docs/requirements.md:217（FR-23「識別子を返す記録と返さない記録が混ざるソースは、論理ソースを分けて登録する」）/ deep-questions.json の Q1 option 1 の detail（Timeline.json は「訪問・移動の区間・経路の点」の種類が混ざる）/ migrations/202609120944_gates.sql:59（収集した行の論理ソースは書き換え不可）/ migrations/202609120942_dedup_indexes.sql（索引は `logical_source` ごと）/ reference/requirements.md:721（タイムラインはクラウドから端末へ移行した —— 移行前の Takeout の履歴と端末の書き出しは同じ期間を含みうる）/ CLAUDE.md の C の既定「細かい粒度で持つ」
- kind: technical
- loss: rewrite-all
- 提案: C1 を「製品 × 記録の種類ごとに 1 本（訪問・移動・経路の点・生の位置）」に直す（C のままでよい。分けた行は読むときに合わせられるが、混ぜた行は凍結で分けられない）。移行前の形式と Timeline.json の重なりは、どちらを読むときに優先するかを後の閲覧の Story へ送る旨を「確かめたが問わなかったこと」に足す（両方残るので失われるものは無い）。
- 処置: escalated — 問いにはしない（細かい粒度で持つ）。deep.md の C1 を「製品 × 記録の種類ごと」に直し、移行前の形式と Timeline.json の重なりは両方残るので閲覧の Story で優先を決める旨を「確かめたが問わなかったこと」に足した。本人へは C の一覧で見せる

## R11. C2 の理由「Asia/Tokyo と仮定して書くと全行の書き直しが要る」は誤り。地域の列は凍結されていない
- 成果物: openspec/changes/st12-archive-ingestion/deep.md
- 根拠: migrations/202609120944_gates.sql:66-70（「`device_id` / `tz_id` / `tz_offset_min` は…直せる余地を残す（どれも冪等キーの入力ではない）」）/ migrations/202609100000_immutable_origin.sql:17（「`tz_id` / `tz_offset_min` は凍結しない」）/ crates/server/src/ingest.rs:178-191（鍵の入力は `logical_source` / `event_time` / `raw` だけ）
- kind: premise
- 提案: C2 の結論（推定しない・取得元のとおりに持つ）はそのままでよいが、理由を「地域は後から直せる（凍結していない）。ただし推定値と取得元の値を見分ける印が無いと、どれを直すべきか分からなくなる」に直す。**凍結される `event_time` の側**（UTC の瞬間の読み方を誤ると直せない）を不可逆の例として挙げ替える。
- 処置: fixed deep.md — C2 の結論（推定しない）はそのまま、理由を「地域の列は凍結されていないが、推定した値と取得元の値を見分ける印が無いと直す対象が分からない」に直し、凍結される側（出来事の時刻）を例に挙げ替えた

---

## 突き合わせの結果（一覧にあるもの）

| 問い | 判定 | 理由 |
|---|---|---|
| Q1 | 分類違い（R6）/ 前提を問わず置いている（R7） | 失われるものの有無が Q2 に依存する |
| Q2 | 妥当（A / discarded）。context の前提に誤り（R2 / R3）、推奨と C14 の食い違い（R9） | |
| Q3 | 妥当（B / daily、推奨あり）。context が C5 と矛盾（R9）、運ぶ経路が抜け（R8） | |
| Q4 | 妥当（B、推奨あり）。C12 で両方の日を持つので表示の規則は戻る | 不要ではない（FR-16 に 2 つの読みがある） |
| Q5 | 妥当（visual + proto）。context の「生存信号を送らない」が FR-78 と衝突（R4） | 軸 3 の「置いた書庫の結果」は R4 の台帳が無いと画面に出す材料が無い |

- **不要**: 該当なし（確かめた範囲: Q1〜Q5 を FR-14/16/17/55 と扉 #6/#7/#8/#12/#14/#15 に当てた。要件か扉で既に幅なく決まっている問いは無かった）
- **既定の無い仮の問い**: 該当なし（Q1 / Q3 / Q4 はいずれも `recommended` あり）
- **文字で決めさせている画面の問い**: 該当なし（画面の構造は Q5 が proto で問うている。Q4 は日付の意味の選択で構造ではない）

## 処置のまとめ（呼び出し元が付けた）

11 件すべてに処置を付けた。

| 処置 | 件数 | 指摘 |
|---|---|---|
| 問いに足した・A に上げた | 4 | R1（Q6）/ R6（Q1 を A へ）/ R7（Q7）/ R8（Q8） |
| 問いの文面を直した | 1 | R2（Q2 の context と不可逆。選び方は Q6 で問う） |
| 聞かないで決める既定として記録（C の一覧で本人に見せる） | 5 | R3（C15）/ R4（C16〜C18）/ R5（C11）/ R9（C5）/ R10（C1） |
| 成果物を直した | 1 | R11（C2 の理由） |
