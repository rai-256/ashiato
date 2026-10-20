# ST13 深掘りの問いの独立レビュー（deep-review）

対象: `openspec/changes/st13-scheduled-account-fetch/deep-questions.json`（Q1〜Q3）・`premises.md`・`proto.html`
入力: `docs/stories/ST13.md`（FR-15 / NFR-3 / NFR-12。doors なし）、`docs/requirements.md`、`docs/handoff/ST13.md`、`docs/briefs/ST13.html`、
`openspec/changes/archive/2026-10-13-st12-archive-ingestion/{deep.md,design.md}`、ST47 の design、
`crates/server/src/archive/{config.rs,scan.rs,worker.rs,classify.rs}`、`crates/server/src/lib.rs`、`migrations/*.sql`（`core.source` の INSERT を ORDER の順で数えた）、
`web/src/{LatestArchive.tsx,messages.ts}`、`collector-android/app/src/main/kotlin/dev/ashiato/collector/*`、`docs/ui-direction.md`、`scripts/upstream_misses.py report`

## 独立にやり直した手順の記録（問いの一覧を見る前）

- 手順 1（衝突）:
  - FR-15（C-03 が登録簿の間隔で取る）と NFR-12（Google 系の手作業は 2 か月に 1 回まで）が、EXT-A（テストの間は 7 日で失効）を通してぶつかる。テストのままの Google の API は毎週の許可し直しになる
  - NFR-12 の 2 文目「他のすべてのソースは手作業を要さない」。INDEX:36 / :213 を見ると、NFR-12 を満たす Story は ST13 だけ。ところが Google 以外の収集にも手作業がある（R3）
  - FR-14 の「形の確認の印を置くまで格納しない」（requirements.md:300）と、完了の判定 2「2 か月に 1 回の手作業以外に操作が要らない」（R4）
  - NFR-3 と FR-55 は同じことを言っていて衝突しない。ST12 が既に満たしている（`web/src/LatestArchive.tsx`、`web/src/__tests__/archive-heading.test.tsx`）
- 手順 2（扉）: doors は空で、扉には該当しない。ただし FR-15 の「取得間隔」と FR-35 の「想定間隔」（`core.source.expected_gap_sec`）が同じ列なのか別の列なのか、読み方が分かれる（R5）
- 手順 3（一方通行）: Q1 の推奨の側では、新しく捨てる・変換する・外へ出すものは無い。足すのは登録簿の列と、環境変数を登録簿へ移すことだけで、どちらも戻せる。
  `raw` は既に `text`（`migrations/202609111111_coverage_rebuild.sql:64` ほか）で、鍵は `logical_source + event_time + raw`（`crates/server/src/ingest.rs:171`）。どちらも ST13 は変えない。
  外へ出る候補が立つのは、Takeout の届け先をクラウドのドライブにする道を採る場合だけ（R1）。Google 以外のサービスを足す場合（Q1 の選択肢 2）は、資格情報の置き場・鍵・初回にどこまで遡るかが第 2 回の論点になる
- 手順 4（日常）: 許可し直しの頻度（EXT-A）、2 か月ごとの手作業の中身（ダウンロード・タイムラインの書き出し・年 1 回の予約のかけ直し・形の確認）、Android の権限が自動で剥がされたときの手作業（R3）、設定の画面に何が毎回見えるか（Q3）
- 手順 5（既存コード）:
  - 登録簿に取得間隔の列が無い（`migrations/202609081618_envelope.sql:9` は `expected_gap_sec` だけ）
  - 見回りの間隔は環境変数（`crates/server/src/archive/config.rs:28-31`、既定 120 秒）
  - 定期取得の枠は無い。`core.source` に定期取得のソースは 0 行（静的に 23 行、ほかに製品ごとの `c03-myactivity-u*` が動的に増える。`worker.rs:987`）
  - 申し送りの R25 / R34 / M7 / M8 は記述どおり残っている（`worker.rs` に `begin()` が 0 件、`scan.rs:55-56` の `?`、`worker.rs:137-141`、`open.rs:49,61`）。R28 は ST47 で閉じている（`classify.rs:80-112`）
  - 形の確認は CLI だけ（`tools/archive-shape.sh`、`web/src/messages.ts:140`）
  - Android の権限の自動剥奪は検出して報告するだけで、防ぐ手が無い（`AndroidCapability.kt:19`、`LocationService.kt:312-313`。`grep -rniE "hibernat|unused|revoke"` は説明のコメント 2 件だけ）
- 手順 6（過去の取りこぼし）: 当てはまったのは次の 4 つ
  - 「最終成功時刻・間隔で動く判定には前回が未来の場合を問う」→ R6
  - 「申し送りの全項目を見出し単位で突き合わせる」→ Q2 で R25 / R28 / R32 / R34 / R35 / M1〜M10 / R89 がすべて受けられている。抜けは無い
  - 「外の形式を実物で」→ R2
  - 「既存コードの振る舞いを確かめずに置いた」→ R7・R9
  - ほかの観点（検証コマンド・寸法・コントラスト・e2e）は tasks と spec の段のもので、問いの段には当たらない
- 手順 7（族の網羅）: 取る口の族、手作業の族、時計の族を見た
  - 取る口の族: Q1 は「Data Portability」「テストの OAuth」「API 自体が無い」を挙げている。「本番で未審査（個人利用）の OAuth」と「Takeout の届け先をドライブにして取る」が抜けている（R1 / R2）
  - 手作業の族: Q2 が扱うのは申し送りの分だけ。形の確認（R4）と Google 以外の手作業（R3）が抜けている
  - 時計の族: 定期取得の枠について 1 つも問うていない（R6）
- 手順 8（前提の確かさ）: Q1（loss あり）が載っている前提は確かさ 1〜2 だけで、行も 1 つ足りない（R2）

---

## R1. Takeout の予約エクスポートの届け先をドライブにし、C-03 がそこから定期取得する道が、Q1 の選択肢にも context にも無い（Google 系で唯一、手作業を減らせる道）
- 成果物: openspec/changes/st13-scheduled-account-fetch/deep-questions.json
- 根拠: docs/requirements.md:1351（EXT-B: 予約エクスポートは 2 か月ごと、書庫は約 7 日で失効）/ docs/requirements.md:1074-1078（NFR-12 ★: クラウドのドライブを経由しないのは「位置の全履歴の写しが外へ出るため」。理由はタイムラインについて書かれている）/ openspec/changes/archive/2026-10-13-st12-archive-ingestion/deep.md:332 / :358（Q14・Q15: マイアクティビティと YouTube の項目は位置を持ちうる）/ Q1 の context「定期取得に移せるものはいま無い」/ premises.md にドライブの行が無い
- kind: conflict
- loss: exported
- 提案: Q1 に選択肢を 1 つ足す（あるいは A の問いとして別に立てる）。中身は「Takeout の予約エクスポートの届け先を Google ドライブにし、C-03 がドライブの API で新しい書庫を取りに行く（ダウンロードと 7 日の期限が消え、残る手作業はタイムラインの書き出しだけになる）」。その選択肢の不可逆の欄には次の 2 つを書く。(a) 位置を含みうる書庫の写しがドライブに残る。NFR-12 の ★ がタイムラインについて避けた理由と同じ型で、要件の文を変える。(b) ドライブを読める資格情報を自宅に置く。ドライブを読む scope の区分（restricted か）と、許可の寿命（R2）は premises.md に行を足して確かめてから書く
- 処置: fixed Q1 —— 選択肢「Takeout の届け先をドライブにし、ドライブから定期取得する」を足した（loss の中身は irreversible の欄。Q1 は loss: uncaptured のまま A）。premises.md に Takeout の Add to Drive・`drive.readonly` が restricted・`drive.file` の範囲の行を足し、選んだら spec の前に第 2 回で実物を求めると書いた

## R2. 「審査なしで使う形 = 同意画面がテスト = 7 日で切れる」と言い切っているが、本番のまま未審査で個人が使う形を確かめていない。Q1（loss あり）が載る前提はどれも確かさ 2 以下
- 成果物: openspec/changes/st13-scheduled-account-fetch/premises.md / deep-questions.json の Q1 の context の (2)
- 根拠: premises.md の表の 4 行目。EXT-A（docs/requirements.md:1350）の逐語は "Authorizations by a **test user** will expire seven days" で、テストのときの寿命しか述べていない。公開の状態が「本番」で未審査のまま（警告の画面と利用者数の上限つきで）使う形の寿命は、premises.md のどの行も扱っていない。YouTube の 2016 年の行は確かさ 1（旧プロジェクトの孫引き）、Data Portability の 3 行は確かさ 2。もう 1 つ、premises.md 末尾の「この前提が外れても失われるものは無い」は、Takeout 側が古い分を保持し続けることが前提になっている。ところが FR-35 の ★ C8（docs/requirements.md:471）は「Takeout が書き出す Chrome の履歴が何日ぶんを持つかの一次情報はまだ無い」と書き、マイアクティビティには自動削除の設定がある
- kind: premise
- 提案: premises.md に 2 行足し、Q1 の context の (2) を書き直す。1 行目は「本番・未審査・個人利用の OAuth で refresh token が 7 日で切れるか（scope の区分ごと）」で、確かめ方は公式の文書と、本人の資格で同意画面を 1 つ作ってみること（外へ出る操作なので、本人に問う形でよい）。2 行目は「Takeout の届け先をドライブにしたとき、取るのに要る scope」。確かさ 4 に届かないなら、Q1 の推奨が「定期取得できる口はいま無い」という事実に依っていないこと（依っているのは本人の手元の事実だけであること）を context に明記する。「外れても失われない」には、Takeout 側の保持が分からないという但し書きを付ける
- 処置: fixed premises.md / Q1 —— OAuth の Refresh token expiration（公式文書）と未審査の利用者数の上限の行を足し、Q1 の context の (2) を「テストなら 7 日・本番の未審査なら 7 日では切れない」に書き直した。YouTube の 2016 年は「確かめ直していない」と明記。「外れても失われない」に Takeout 側の保持が分からない但し書きを付けた

## R3. NFR-12 の後半「他のすべてのソースは手作業を要さない」を満たす Story は ST13 だけなのに、Google 以外の手作業を 1 問も見ていない。Android の権限が自動で剥がされると、本人が許可し直すまで位置が取れない
- 成果物: openspec/changes/st13-scheduled-account-fetch/deep-questions.json
- 根拠: docs/stories/INDEX.md:36 / :213（NFR-12 の本体の Story は ST13 だけ）/ docs/requirements.md:1072-1073 / docs/requirements.md:480（FR-78 ★「Android は長期間使っていないアプリの権限を自動で剥がし」）/ collector-android/app/src/main/kotlin/dev/ashiato/collector/AndroidCapability.kt:19 と LocationService.kt:312-313（剥がれたことは生存信号で報告するだけ）/ `grep -rniE "hibernat|unused|revoke" collector-android/app/src/main` で防ぐ手は 0 件（説明のコメントだけ）
- kind: conflict
- loss: uncaptured
- 提案: A の問いを 1 つ足す。問うのは、ST13 が NFR-12 の後半（Google 以外の手作業ゼロ）まで持つかどうか。選択肢は次の 2 つ。(a) 持つ。手作業を棚卸しし、権限の自動剥奪を初回に 1 回だけ止める手（端末の「使っていないアプリを一時停止」の除外を案内する。収集アプリの側の作業）を足す。(b) 持たない。NFR-12 を満たす Story を /stories で付け直す。不可逆の欄には「剥がれてから本人が許可し直すまでの位置・アプリ利用は取れない（FR-35 の通知は 3 倍の間隔で鳴る）」を書く。収集アプリが長く操作されないと剥がされるのか（前景サービスが動いていても剥がされるのか）は、premises.md に行を足して確かめる
- 処置: fixed Q4 —— A の問い（kind: premise / loss: uncaptured）として足した。休止の前提は Android の公式文書で確かめて premises.md に足した（前景サービスや予約の仕事は「使った」に数えない）

## R4. 形の確認の印を置くのは CLI（`tools/archive-shape.sh`。`DATABASE_URL` と psql か docker が要る）だけなのに、完了の判定 2 の棚卸しに入っていない
- 成果物: openspec/changes/st13-scheduled-account-fetch/deep-questions.json の Q2
- 根拠: docs/requirements.md:300（FR-14: 印を置くまで格納しない）/ crates/server/src/archive/worker.rs:7-14（YouTube・マイアクティビティ・Chrome は確認が要る）/ worker.rs:1510-1530（製品の名前・言語が増えるとハッシュが変わり、また確認待ちになる）/ web/src/messages.ts:140（画面は「tools/archive-shape.sh で形を見て印を置く」と案内する）/ tools/archive-shape.sh:6-7（`DATABASE_URL` と `ASHIATO_ARCHIVE_USER_ID` が要る）/ ST12 deep.md 第 3 回 Q12（本人は「また確認待ち」を選んだ。頻度は「新しいサービスを使い始めた回・言語を変えた回」の見込み）
- kind: daily
- 提案: Q2 に選択肢を足すか、B の問いを 1 つ足して、形の確認を「2 か月に 1 回の手作業」の一部と数えてよいか、端末の上で済む形（画面に印を置く口を作る）にするかを問う。推奨（既定）も書く。確認待ちの中身は台帳と写しに残る（FR-14）ので、どちらを選んでも失うものは無い。完了の判定 2 を「確認の印は手作業に含む」と読むなら、そのことを deep.md の「確かめたが問わなかったこと」に残す
- 処置: fixed Q5 —— B の問い（推奨: 手作業に含める・CLI のまま。反転条件付き）として足した

## R5. FR-15 の「取得間隔」と FR-35 の「想定間隔」（`expected_gap_sec`）を同じ列にするか別の列にするかを決めていない。Q3 の「その場で変える」の意味がこれで変わる
- 成果物: openspec/changes/st13-scheduled-account-fetch/deep-questions.json の Q1 の推奨・Q3
- 根拠: docs/requirements.md:308-309（FR-15「取得間隔はソースごとに登録簿が持つ」）/ :431-432（FR-35「そのソースに登録された想定間隔」）/ migrations/202609081618_envelope.sql:9（列は `expected_gap_sec` だけ）/ s01-archive-inbox は想定間隔が 86400 秒で、見回りは 120 秒（migrations/202609181600_archive_ingestion.sql:159、config.rs:31）/ Q1 の推奨は「取得の間隔」を別の列で持たせる書き方
- kind: technical
- 提案: B の問いを足す（推奨も付ける）か、Q1 の推奨の本文に書く。書くことは次の 2 つ。(a) 別の列にするなら、取得間隔が想定間隔を超えたときに断るか（超えると FR-35 / FR-80 が偽の途絶を出す）。(b) 同じ列にするなら、画面から変えると過去の格子の判定が新しい値で引き直されること。どちらも判定の閾値なので、記録が残っていれば戻る（loss は付けない）
- 処置: fixed Q1 —— 「聞かずに決める既定」に、取得の間隔は想定間隔と別の列・想定間隔より長い登録は断る、と書いた（C。spec の design に D で残す）

## R6. Q1 の推奨にある「間隔が来たら走る枠」について、時計の族（前回が未来にある・止まっている間に契機を過ぎた・起動の直後）を 1 つも問うていない
- 成果物: openspec/changes/st13-scheduled-account-fetch/deep-questions.json の Q1
- 根拠: `scripts/upstream_misses.py report` の「最終成功時刻・間隔で動く判定には『前回が未来にある』場合を必ず問う」（ST08 Q7。ST02 Q29・ST05・ST06 Q9 も同じ型）/ worker.rs:2137（いまの見回りはプロセスの中で寝るだけで、前回の時刻を持たない）
- kind: technical
- 提案: B の問いを 1 つ足すか、Q1 の推奨の本文に既定として書く。既定の例は「前回の時刻を DB に持たず、起動した直後に 1 回回し、その後は単調な時計で間隔を待つ」。いまは取るソースが 0 本なので失うものは無い。ただ、後の Story が登録簿に 1 行足すだけで乗る設計なので、時計が戻ったときと止まっていた間の扱いは、ここで決めた既定がそのまま引き継がれる。その旨を書く
- 処置: fixed Q1 —— 「聞かずに決める既定」に、前回の時刻を DB に持たず起動の直後に 1 回、その後は単調な時計で待つ、と書いた。後の Story が引き継ぐことも明記

## R7. proto の前提がコードと合わない: 見回りは「1 分ごと」と描いてあるが既定は 120 秒。「登録簿の 23 本を実物の数で」も、製品ごとに増えるマイアクティビティの行を数えていない
- 成果物: openspec/changes/st13-scheduled-account-fetch/proto.html / deep-questions.json の Q3 の context
- 根拠: proto.html:104（`"1 分ごと"`）と crates/server/src/archive/config.rs:28-31（`unwrap_or(120)`）、openspec/changes/archive/2026-10-13-st12-archive-ingestion/design.md:61（既定 120）/ worker.rs:985-1003（`ensure_myactivity_source` が、印を通った製品ごとに `c03-myactivity-u*` を足す）/ ST12 deep.md Q1 の答え（本人はマイアクティビティを全部選んだ。補足では製品の例を 9 つ挙げている）
- kind: premise
- 提案: proto の値を「2 分ごと」に直す。書庫の節にマイアクティビティの製品の行を例として数本（例: 検索・マップ・Discover・Play ストア）足し、360 px で一覧の長さが分かるようにする。Q3 の context の「実物の数で」は「静的な 23 本と、製品ごとに増える行」に直す
- 処置: fixed proto.html / Q3 —— 見回りを「2 分ごと」に直し、マイアクティビティの製品ごとの行を 4 本足した。Q3 の context を「静的な 23 本と、製品ごとに増える行」に直した

## R8. Q3 が「画面から間隔を変えられるか」（API と資格情報の形の問い）を画面の構造の問いに混ぜている。変えて効く行はいま 1 本しかない
- 成果物: openspec/changes/st13-scheduled-account-fetch/deep-questions.json の Q3
- 根拠: Q3 の why 自身が「表示は後から変えられるが、『画面から間隔を変えられるか』は API と権限の形を決める」と書いている / proto.html:104 で編集できる行（`true`）は s01-archive-inbox の 1 本だけ / docs/ui-direction.md:23（S-7 の「ソース・停止」は ST13 と ST15 が分け合う。ST15 は未着手で、ソースごとの停止も同じ一覧に乗る見込み）
- kind: technical
- 提案: 「画面から変えるか」は proto から外し、B の文字の問いにする（推奨: 見せるだけ）。消えるものは無く、口は後から足せる。Q3 の proto に残すのは「どこに出すか・並べ方・書庫の行に何を出すか」の構造だけにする。S-7 のソースの一覧に ST15 の停止の操作が後から乗ることを context に一言書く
- 処置: fixed Q3 / Q6 —— 「画面から変えるか」を proto から外して B の Q6（推奨: 見せるだけ）にした。Q3 の context に ST15 の停止が S-7 の一覧に乗る見込みを書いた

## R9. Q2 の推奨に入っている R25 の直しは、ST12 の design D4（本人の決定 Q6 / C10 の「1 件 1 トランザクション」の格納の関門）に触りうる。それを書かずに「仮でよい」としている
- 成果物: openspec/changes/st13-scheduled-account-fetch/deep-questions.json の Q2
- 根拠: docs/handoff/ST13.md:13（「括り直すと PgSink が持つ格納関門の境界（ST03 の本人の決定）に触る」）/ openspec/changes/archive/2026-10-13-st12-archive-ingestion/design.md:31（D4 の出所は「C10 / Q6（本人）」）・:67（移行前のロケーション履歴は 1 件 1 トランザクションで数十分〜1 時間）/ Q2 の context は R25 の中身しか書いていない
- kind: premise
- 提案: Q2 の context と推奨の detail に、R25 をどう直すかを書く。たとえば「記録は 1 件ずつのまま、`read` の行とソース別台帳だけを 1 つのトランザクションで書く（D4 に触らない）」。D4 に触る直し方（記録ごと括る）しか無いなら、本人の決定を変えることになり P の条件（要件と本人の決定を変えない）を外れるので、その選択肢を A として分ける
- 処置: fixed Q2 —— context に R25 の直し方（記録は 1 件ずつのまま、`read` の行とソース別台帳だけを 1 つのトランザクションで書く。D4 に触らない）を書き、推奨の detail にも書いた

---

## 該当なしの確かめ

- **不要**: 該当なし。Q1 は本人の手元の事実を問う問いで、Q2 は推奨のある B。Q3 は画面の構造の問いで、ST13 が S-7 を持つ（docs/ui-direction.md:23）。どれも要件や扉で既に決まった問いではない
- **既定の無い仮の問い**: 該当なし。B は Q2 だけで、推奨がある
- **不可逆の根拠が無い**: 該当なし（`kind: irreversible` の問いが無い）。Q1 の `loss: uncaptured` には、取得元が古い分を消すと取れないという中身がある
- **文字で決めさせている画面の問い**: 該当なし。画面の構造は Q3 の proto に載っている。R8 は逆向きの指摘で、構造ではない問いが proto に混ざっている
- **扉の幅（手順 2）**: doors は空。§5 の扉の「関与要件」を FR-15 / NFR-3 / NFR-12 で引くと 0 件だった（当たった 1498 行目は NFR-15）。幅は R5 だけ
