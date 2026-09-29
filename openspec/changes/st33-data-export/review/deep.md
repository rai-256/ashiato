# ST33 深掘りの独立レビュー（deep-questions.json / deep.md / proto.html）

schema の手順 1〜5 を、問いの一覧を見る前に自分でやり直した。そのあとで問いの一覧（Q1〜Q4）と C1〜C14・X1 に突き合わせた。**問いの JSON・deep.md・proto は触っていない。**

確かめた範囲: `docs/stories/ST33.md` / `docs/handoff/ST33.md` / `docs/handoff/ST12.md`、`docs/requirements.md`（NFR-5 / NFR-7 / NFR-15 / NFR-16 / NFR-17〜23 / FR-8 / FR-12 / FR-18〜FR-31 / FR-44 / FR-50 / FR-51 / FR-54 / PERM-2〜10 / 扉 #7 #15 #23）、
`docs/ui-direction.md`（S-7・件数の表）、`migrations/*.sql` 全部、`crates/server/src/lib.rs`（`events` / ルータ）、
`crates/server/src/stay.rs` と `stay_store.rs`（main と `../ashiato2-st22`）、`collector-android/.../Heartbeat.kt` `LocationFix.kt`、`crates/collector-windows/src/heartbeat.rs` `lib.rs`、
`web/src/App.tsx`、走っている change: `../ashiato2-st22`（st22 の design D1〜D7 と移行）、`../ashiato2-st12`（st12 の deep / design / 移行）、`../ashiato2-st06` `../ashiato2-st08`（移行）、
並走中の上流 `../ashiato2-up-st28`（st28 の deep.md / deep-questions.json）、`openspec/changes/archive/2026-09-14-st07-active-window/`、`python3 scripts/board.py` の出力。

**実行した確認**:

- ST22 の worktree の DB（`ashiato2-st22-db-1`。ST22 のテストが書いた行）で、`core.event_version_live` のうち、**本人が消した滞在（`deleted_by='user'`）の時間に重なる、生きている滞在の前の版**を数えた:
  `docker exec ashiato2-st22-db-1 psql -U ashiato -d ashiato -Atc "select count(*) from core.event_version_live v join core.event d on d.user_id=v.user_id and d.logical_source='s01-stay' and d.deleted_by='user' where v.logical_source='s01-stay' and (v.payload->>'start')::timestamptz <= (d.payload->>'end')::timestamptz and (v.payload->>'end')::timestamptz >= d.event_time"` → **4**。
  1 件の例: 親の滞在は生きている（いまの版 23:00–00:50）が、前の版 2 は `start=23:00 end=03:00 lat=35.6812 lon=139.7671` で、消した滞在（01:00–02:00、同じ緯度経度）を丸ごと含む（R2）
- 生存信号の間隔: `collector-android/.../Heartbeat.kt:19` `HEARTBEAT_INTERVAL_MS = 6 * 60 * 60 * 1000L`、`crates/collector-windows/src/lib.rs:47` `EXPECTED_GAP_SEC = 21_600`、登録簿の想定間隔は `c02-browser-history` だけ 86400（`migrations/202609111111_coverage_rebuild.sql:128-132`）（R3 / R7）
- proto の量: `PER_DAY` の和 = 3,069 件/日 → 1 年 1,120,203 行 × 0.8 KB = 875 MB。うち「稼働の信号」が 1,450 件/日（47 %）（R7）
- `grep -n "R6\|ST22\|消した" ../ashiato2-st12/openspec/changes/st12-archive-ingestion/{deep,design}.md` → 申し送り R6（書庫から入る同じ場面）への決定は **0 件**（R1）

## 手順ごとの結果

- **手順 1（要件どうしの衝突）**: NFR-16 と NFR-15 は Q1 が扱っている。ただし NFR-15 の読み方は ST28 の Q3 が同時に問うている（R4）。
  **NFR-16「全記録」と FR-50 / ST22 Q1（消した場面の緯度経度を外へ出さない）**が、削除の印の無い行（別のソース・前の版・印付け前の遅着）でぶつかる。一覧に無い（R1 / R2）。
  FR-22「1 件として読む形…適用は…書き出しの各 Story」と C3 が逆を向いている（R9）
- **手順 2（扉の幅）**: `doors: []`。扉 #15（緩く始めて締めると戻らない）の性質は Q1 が持っている。C5（感度で絞らない）は Q1 の答えで効き方が変わる（R4）
- **手順 3（新たに立つ一方通行）**: 外に出すもの = 書き出したファイル。C1 / C2 / C7 のそれぞれに、消した場面か消したことそのものが外へ出る経路がある（R1 / R2 / R3）。
  置き場のフォルダの既定が決まっていない（R5）。書き出した事実の記録を残す既定が無い（R6）。列の型: 原文は `text` のまま出す（C4）でよい —— 探したが問題は無かった。ただし `core.event_live` は `SELECT *` を作った時点で固まるビューである（R12）
- **手順 4（日常に影響する選択）**: 押したときだけ走るので、毎日目に入るものは無い。容量・時間・端末に落ちる大きさの見積もりの内訳が要件と合わない（R7）
- **手順 5（既存コードが要件を満たしていない箇所）**: NFR-16 を満たす口は無い（deep.md のとおり。`GET /events` は 6 列で `fetch_all` → `crates/server/src/lib.rs:1701-1740`）。
  `core.event_live` と `core.event_version_live` を読めば ST22 の削除が全部除かれる、という前提は成り立たない（R1 / R2）。生存信号は位置の取得の成功回数を 6 時間ごとに持つ（R3）

**C のうち指摘しなかったもの**: C4（`raw` は `text`。`migrations/202609092315_raw_text.sql`・`202609120943_version_and_ledger.sql:34`）、C6（台帳と `core.coverage` を出さない。
ただし理由は R3 と食い違う）、C9、C10、C12（NFR-16 は「D-01 の全記録」）、C13（`GET /events` は ST01 の経路）、C14（FR-20）、X1（FR-66〜68 のとおり）。
**不要な問いは見つからなかった。** Q2 / Q4 は `loss` の無い B で推奨があり、Q3 は画面の構造を proto で問うている。

---

## R1. C1「`core.event_live` だけを読む」では ST22 Q1 の担保にならない。消した場面の緯度経度は、削除の印の無い行から書き出しに出る
- 分類: 抜け（前提の誤りも含む）
- 成果物: openspec/changes/st33-data-export/deep.md（C1・手順 5 の「ビューを読めば全部除かれる」46 行）/ deep-questions.json（Q1 の context「消した記録は、どの選択肢でも出さない（…の担保）」）/ proto.html（軸 1「入らないもの: 消した記録」）
- 根拠: docs/handoff/ST12.md:22-28（st22 R6: 書庫から入る同じ場面は ST22 の削除に当たらない。内容の鍵が論理ソースを含むため）/ ../ashiato2-st12/openspec/changes/st12-archive-ingestion/design.md:76-77,157-160（`c03-timeline-visit` / `-path` / `-signal`・`c03-legacy-location` が `lat` / `lng` を持つ）/ `grep -n "R6\|ST22\|消した" ../ashiato2-st12/.../{deep,design}.md` で R6 への決定は 0 件 / ../ashiato2-st22/openspec/changes/st22-record-deletion/design.md:106（連鎖で消すのは基準のソースだけ。「位置以外は触らない」）・:143-151（遅れて届いた位置に印を付けるのは作り直しの中で、「**残る穴**: 作り直しが失敗した間だけ、その位置は読み出しに出る」）/ ../ashiato2-st22/crates/server/src/stay_store.rs:626-690（`mark_late_arrivals` は `sources` = 基準のソースだけを見る）/ docs/requirements.md:844（NFR-16「全記録」）と :472-475（FR-50 の ST22 Q1 補足。理由は「書き出しができると外へ出る」）
- kind: conflict
- loss: exported
- 提案: A の問いを 1 つ足す。問いは「書き出しは、削除の印の付いた行だけを除くか。それとも、**本人が消した滞在の時間に入る、緯度経度を持つ行**（別のソースの位置・書庫の訪問と経路・これから入る写真の位置）も除くか」。後者は NFR-16「全記録」を狭める訂正が要る。どちらにするかで、書庫（ST12）と写真（ST09）の行が外へ出るかが決まる。Q1 の context の「どの選択肢でも出さない」と deep.md 46 行、proto 軸 1 の「入らないもの」は、答えが出るまで「削除の印の付いた記録」に直す。作り直しの失敗による穴は、書き出しの前に `mark_late_arrivals` と同じ判定を 1 回走らせれば機械で塞げる（C で足してよい）
- 処置: escalated — A の問い Q5 を足した（印の無い行を除くか。推奨: 緯度経度を持つ行も除く）。Q1 の context・deep.md の手順 5・proto の「入らないもの」を「削除の印の付いた記録」に直した。作り直しの失敗の穴は C18（書き出しの前に遅着の判定を 1 回走らせる）

## R2. C2 の前の版（`core.event_version_live`）は、生きている滞在の古い版として、消した場面の時刻と緯度経度を出す
- 分類: 抜け（C2 に loss がある）
- 成果物: openspec/changes/st33-data-export/deep.md（C2）
- 根拠: migrations/202609120943_version_and_ledger.sql:78-91（`event_version_live` が除くのは**親が消えた**版だけ）/ ../ashiato2-st22/crates/server/src/stay_store.rs:599,696-712（作り直しで識別子を継いだ滞在は、前の版を `core.event_version` に積む）/ crates/server/src/stay.rs:195（滞在の原文は `start` / `end` / `lat` / `lon` を持つ）/ docs/requirements.md:294-300（FR-31: 識別子を継ぐのは重なりがいちばん大きい 1 件）/ 冒頭の DB 確認（4 件。生きている滞在の版 2 が、消した滞在 01:00–02:00 を含む 23:00–03:00 と同じ緯度経度を持つ）
- kind: conflict
- loss: exported
- 提案: R1 の問いに「前の版」を含める（選択肢ごとに、生きている記録の前の版のうち、消した滞在の時間に重なる版を出すかを書く）。少なくとも、C2 の理由「親が消えれば出ない（C1）」は、親が生きていて前の版だけが消した時間を含む場合を覆っていないことを書く。FR-50「重なる派生は丸ごと削除済み」は、前の版には掛かっていない（ST22 の判定はいまの版の区間だけを見る）
- 処置: escalated — Q5 に前の版を含めた（選択肢 1 が「生きている記録の前の版のうち消した時間に重なるもの」を除く）。deep.md の C2 に「親が生きていて前の版だけが消した時間を含む場合は C1 では除かれない」と書いた

## R3. C7 で出す生存信号の `successes` から、6 時間単位で「ここで消した」が読める。C6 と C8 の理由と食い違う
- 分類: 抜け（C7 に loss がある）
- 成果物: openspec/changes/st33-data-export/deep.md（C6 / C7 / C8）
- 根拠: collector-android/app/src/main/kotlin/dev/ashiato/collector/Heartbeat.kt:19（6 時間ごと）・:95-180（`successes` は位置の取得が成功した回数で、1 回の成功が位置の記録 1 件）・:272-276（原文に `attempts` / `successes` を持つ）/ migrations/202609111111_coverage_rebuild.sql:52-66（`core.heartbeat` の列）/ deep.md の C6「日ごとの件数は…記録の数との差から『その日に消した』ことが読める → 入れない」、C8「削除済みの件数は、消したことそのものを外へ出す」
- kind: conflict
- loss: exported
- 提案: 6 時間の窓で「信号の `successes` − 書き出した位置の件数」を取れば、消した件数と、消した時間帯のおおよその位置（時刻）が出る。日ごとの件数（C6）より細かい。どちらかに揃える: (a) 「消したことそのもの（件数と時間帯）を外へ出すか」を R1 と同じ問いの選択肢として本人に問う。(b) C6 / C8 の理由から「消したことが読める」を外す（出してよいとする）。(c) 生存信号から `attempts` / `successes` を除いて出す。この場合は FR-54 の「動いていたが取れない状態だった」の材料が書き出しから減ることを書く。AI が C で決めてよいのは (c) の「出さない側」だけで、その代償を C7 に書く
- 処置: escalated — 問いにはしない（出さない側 (c) は扉を開けたままにする既定）。deep.md の C15 に「生存信号の取得の回数と原文は出さない」と代償（取得率が書き出しから読めない）を R3 つきで記録。本人には C の一覧として見せ、異論は番号で受ける

## R4. Q1 の NFR-15 の読み方は、並走中の ST28 の Q3 が同じ文面を問うている。Q1 の前提（「NFR-15 を訂正せずに満たせる」「携帯に入れば文面を割る」）は ST28 の答えで変わる
- 分類: 前提の誤り
- 成果物: openspec/changes/st33-data-export/deep-questions.json（Q1 の why / context / 選択肢の detail）/ deep.md（手順 1・Q1・「答えが触る capability」）
- 根拠: ../ashiato2-up-st28/openspec/changes/st28-private-network-only/deep.md:64-68（Q3「NFR-15 の『拠点外に出る』の読み方」B / conflict。推奨「本システムが記録の写しを拠点外に置くこと」。もう 1 つの選択肢は「本人の端末は自宅の延長とみなす」）・:106（C4 `Cache-Control: no-store`「端末のブラウザに記録を残さない」）/ 同 deep-questions.json の Q3 context「収集アプリは送る前の記録を本人の端末に最長 90 日ためている（NFR-7）」/ docs/requirements.md:117-118（FR-8: 携帯は未送信を 90 日・2 GB まで平文で持つ）・:843（NFR-15）
- kind: premise
- 提案: Q1 の context に ST28 の Q3 と C4 を書く。ST28 で「本人の端末は自宅の延長」と読めば、選択肢 2 は NFR-15 に当たらなくなる。推奨の読みなら、端末に落とすことが「本システムが写しを拠点外に置く」に当たるかが、また未決になる。**Q1 の本体は「NFR-15 の文面」ではなく「押しただけで全記録（全感度。C5）が家の外の端末に入ってよいか」**だと書き直す。NFR-15 の ★ 訂正は ST28 と ST33 のどちらか一方の record で 1 回だけ行う、と deep.md の「要件へ戻すもの」に書く。選択肢 2 / 3 の irreversible には「PERM-5（人物: AI に出さない）と PERM-6 の段も含む全段が入る」（C5）を足す
- 処置: fixed deep-questions.json — Q1 の本体を「押しただけで全記録（全感度）が家の外の端末に入ってよいか」に書き直し、ST28 の Q3 / C4 と FR-8 を context に置いた。選択肢の不可逆に全感度を足した。deep.md の Q1 と「答えが触る capability」に、NFR-15 の訂正は ST28 と ST33 のどちらか一方で 1 回だけと書いた

## R5. Q1 の推奨の選択肢の「不可逆: なし」は、置き場のフォルダ次第で成り立たない。置き場の既定がどこにも無い
- 分類: 抜け
- 成果物: openspec/changes/st33-data-export/deep-questions.json（Q1 選択肢 1 の irreversible）/ deep.md（C の一覧）/ proto.html:278（`C:\Users\rai\ashiato\export\…`）
- 根拠: ../ashiato2-st12/openspec/changes/st12-archive-ingestion/design.md:55,57（同じ PC の先例: 取り込み待ちは `%USERPROFILE%\Documents\ashiato\取り込み待ち`、写しは `%LOCALAPPDATA%\ashiato\archive-copies`。**ユーザーのフォルダを置き場にした前例がある**）/ ../ashiato2-up-st28/openspec/changes/st28-private-network-only/deep.md:50（「同じ PC の別プロセス」も止めるべき相手として挙げている）/ docs/requirements.md:633,635-641（PERM-8 / PERM-10: プラグインは同じ PC で動き、感度の上限は API で掛ける）
- kind: technical
- loss: exported
- 提案: C を 1 つ足す（扉を開けたままにする既定: 厳しい側）。置き場の既定は、本人のフォルダのうちクラウドへ同期されうる場所（ドキュメント・デスクトップ）を避ける（`%LOCALAPPDATA%\ashiato\export` のように ST12 の写しと同じ系統）。環境変数で変えられる。Q1 選択肢 1 の irreversible は「置き場がクラウドへ同期されるフォルダなら、書いた時点で外へ出る。平文のファイルは API の感度の上限（PERM-8）を通らず、同じ PC のどのプロセスからも読める」に直す。Q4 の「毎回残す」はこれを回数ぶん積む、と Q4 の detail に足す。同期されているかどうかは本人の PC の設定次第で、リポジトリからは確かめられない —— だから既定で避ける
- 処置: escalated — 問いにはしない（同期されない置き場を既定にするのは厳しい側の既定）。deep.md の C16 に `%LOCALAPPDATA%\ashiato\export` を R5 つきで記録。Q1 選択肢 1 の不可逆・Q4 の「毎回残す」の説明・proto の置き場のパスを直した

## R6. 書き出した事実（いつ・どの時点の写しを・何件・どこへ）を残す既定が無い。後から「どの書き出しに、後で消した場面が入っていたか」が作れない
- 分類: 抜け（C で足せる）
- 成果物: openspec/changes/st33-data-export/deep.md（C の一覧）/ proto.html（「前回の書き出し」の欄・軸 3「この書き出しの後に消した記録が N 件」）
- 根拠: proto.html:271-285（画面は前回の時刻・行数・置き場を出す。軸 3 は書き出しの時刻より後の削除を数える）/ deep.md の C10（1 つの時点の写し）/ ../ashiato2-st22/openspec/changes/st22-record-deletion/design.md:49-63（削除の台帳は追記のみ・1 操作 1 行。突き合わせる相手の時刻はこちらにある）
- kind: technical
- loss: uncaptured
- 提案: C15 として足す: 「書き出しは 1 回 1 行を追記のみの台帳に残す（開始・写しの時点・終了・表ごとの件数・置き場・Q1 の経路）。**中身は残さない**」。残さないと、軸 3 の数え方は「前回」の 1 件しか持てない。Q4 で「毎回残す」を選んだとき、消した滞在がどの書き出しに入っているかを後から出せない（ファイルは移されうるし、選択肢 2 では置き場をシステムが知らない）。台帳が書き出しに入るかは C6 と同じ扱いでよい
- 処置: escalated — 問いにはしない（「台帳は追記のみ」の既定。持つ側は何も失わず費用が小さい）。deep.md の C17 に書き出しの台帳（1 回 1 行・中身は残さない）を R6 つきで記録。C6 に「書き出しの台帳も書き出しに入れない」を足した

## R7. proto と Q1 / Q4 の量の内訳が、要件と実装の数と合わない（稼働の信号は約 100 倍多く、ウィンドウは 5〜10 分の 1）
- 分類: 前提の誤り
- 成果物: openspec/changes/st33-data-export/proto.html:106,177-180（`PER_DAY`）/ deep-questions.json（Q1 context「約 110 万行・約 0.9 GB」、Q4 context「約 0.9 GB」）/ deep.md:37
- 根拠: Heartbeat.kt:19 と crates/collector-windows/src/lib.rs:47（生存信号は 6 時間ごと = 1 ソース 1 日 約 4 件。ブラウザ履歴は 86400 秒 = 1 件。proto は「約 290 件 / ソース × 5」= 1,450 件/日で、1 年目の表示では**位置の次に大きい行が「稼働の信号 529,250 件」**になる）/ docs/requirements.md:160（ウィンドウの通常の切り替えは 1 日 1,500〜3,000 件、枠は 5,990〜15,200 件。proto は 300 件）/ docs/requirements.md:657（NFR-5: 記号データは年 3〜6 GB）/ docs/ui-direction.md:68（生の記録 1 日 約 4,120 件）/ openspec/changes/st06-app-usage/deep.md:29（アプリ利用は OS が返すイベントを全部取る。proto の 150 件は実測ではない）
- kind: premise
- 提案: `PER_DAY` を直す: 稼働の信号を 1 日 約 13 件（6 時間 × 3 ソース + ブラウザ 1）に、ウィンドウを 1,500〜3,000 件にする。量の選択肢に「NFR-5 の上限（年 6 GB 前後）」を足す。1 年目の合計は近い値のまま（約 0.8〜1.3 GB）でも、内訳を画面で見て決める問いなので、何が多いかは正しくしておく。3 年目の上限側は 10 GB を超えうるので、軸 2 の時間（1 秒 2 万行の仮定）・Q1 選択肢 2 の「携帯に 1 つの zip」・Q4 の「容量は回数ぶん」の説明を上限側でも書く。`c01-app-usage-rollup`（ST06）と書庫のソース（ST12。移行前のロケーション履歴は百万件級 = ST12 design D1）は行に無い
- 処置: fixed proto.html — `PER_DAY` を直した（生存信号 1 日 約 13 件、ウィンドウ 約 2,000 件、写真のメタ 5.5 件、1 行 0.9 KB）。量の入力に「3 年目・NFR-5 の上限」を足した。Q1 / Q4 の context と deep.md の手順 4 の量を直した

## R8. C11 の「利用者ごとの認証を決めるのは ST28」は出所と合わない。書き出しの口がどの合言葉で開くかは ST28 の Q1 / Q4 の答えで変わる
- 分類: 前提の誤り
- 成果物: openspec/changes/st33-data-export/deep.md（C11）
- 根拠: docs/requirements.md:641（PERM-10「資格情報の配り方…は ST29 の深掘りで決める」）/ ../ashiato2-up-st28/openspec/changes/st28-private-network-only/deep.md:50（Q1: 画面のログイン）・:72-75（Q4: 収集アプリの合言葉で全記録が読めるままにするか。A / exported。推奨は「送る」だけに絞り、読み出しは画面のログインか読み出し用の別の合言葉）・:36（いまは画面を配る vite の proxy が合言葉を付けるので、画面に届けば合言葉無しで全記録を読める）
- kind: premise
- 提案: C11 を「読み出し（`GET /events`）と同じ扱いにする。どの合言葉が読み出しに通るかは ST28 の Q1 / Q4 の答えに従い、呼び出し元ごとの本決めは ST29」に直す。Q1 で選択肢 2 / 3 を選ぶと、ST28 の Q1 が決まるまでは「画面に届いた人が 1 回押せば全記録が 1 ファイルで手に入る」ことを、選択肢 2 の irreversible に書く（`GET /events` でも読めるが、1 回で全表・全版がまとまるのは書き出しの口だけ）
- 処置: fixed deep.md — C11 を「読み出しと同じ扱い。どの合言葉が通るかは ST28 の Q1 / Q4、呼び出し元ごとの本決めは ST29」に直した。Q1 の context に、合言葉が決まるまで画面に届いた人が 1 回で全記録を手にできることを足した

## R9. C3（畳まない）は、FR-22 が書き出しを「1 件として読む形」の適用先に挙げていることに触れていない
- 分類: 前提の誤り（失われるものは無い）
- 成果物: openspec/changes/st33-data-export/deep.md（C3）
- 根拠: docs/requirements.md:261（FR-22「内容ハッシュが同じ複数の行を**1 件として読む形**を用意する（適用は閲覧・検索・AI・**書き出し**の各 Story）」）/ migrations/202609120943_version_and_ledger.sql:95-112（`core.event_folded`。「実際に使うのは後続の Story（閲覧・検索・AI・書き出し）」）
- kind: technical
- 提案: C3 の理由に FR-22:261 を引き、「畳まずに出し、`content_hash` で畳めることを説明書に書く（`event_folded` と同じ規則を説明書に載せる）」と、要件の適用をどう満たすかを書く。畳んだ形も作り直せるので B 以下。問いにはしなくてよい
- 処置: fixed deep.md — C3 に FR-22 の適用先を引き、畳む規則（`core.event_folded` と同じ）を説明書に書くことで満たすと書いた。Q2 の context にも足した

## R10. proto と Q3 の「書き出しは取り返しの付かない操作ではない」は、Q1 の選択肢 1 を前提にしている
- 分類: 前提の誤り
- 成果物: openspec/changes/st33-data-export/proto.html:110-116（動かせないもの）/ deep-questions.json（Q3 の context）
- 根拠: deep-questions.json Q1 選択肢 2 の irreversible「携帯に落ちた全記録は…家の外へ出て戻らない」/ docs/requirements.md:815-820（NFR-20 は FR-51 / FR-53 を名指ししているので、24 px のままでも要件は割らない）/ docs/ui-direction.md:23（S-7 は ST13・ST15・ST24・ST29・ST33。Q3 の why と proto は ST13 を落としている）
- kind: premise
- 提案: 「D-01 を変えない」は正しいが、Q1 で端末に受け取る側を選ぶと、押すことそのものが Q1 の不可逆になる。proto の固定の文言を「Q1 が PC のフォルダのとき」に限る。入力で端末に受け取る側を選んだときは、軸 2 の「確認をはさむ」の説明に「押すと全記録が端末に落ちる」を出す。S-7 に後から足す Story に ST13 を足す
- 処置: fixed proto.html — 固定の文言を「D-01 を変えない。ただし Q1 で端末に受け取る側なら押すことが Q1 の不可逆」に直し、端末に受け取る入力では確認の文面に「全記録（全感度）がいま見ている端末に入る」を出すようにした。ST13 を S-7 に足す Story に加えた（Q3 の why・proto・出力の文）

## R11. Q2 の kind が daily になっている。書き出しの形式は日常の選択ではない
- 分類: 分類違い（A / B は変わらない）
- 成果物: openspec/changes/st33-data-export/deep-questions.json（Q2 の kind）/ deep.md（Q2 の見出し「B / daily」）
- 根拠: deep.md の手順 4「書き出しは本人が押したときだけ走る…毎日目に入るものは無い」
- kind: technical
- 提案: Q2 の kind を `open` にする（`loss` 無し・推奨あり。B のまま）。Q4 は容量と、消した記録が積み上がるかの問いなので daily のままでよい
- 処置: fixed deep-questions.json — Q2 の kind を open にした（B のまま）。deep.md の Q2 の見出しも直した

## R12. `core.event_live` は `SELECT *` を作った時点で列が固まる。後で `core.event` に列が足されると、C4「全列を出す」から黙って欠ける
- 分類: 既存コードの穴（いまは一致している）
- 成果物: openspec/changes/st33-data-export/deep.md（C4）
- 根拠: migrations/202609120941_event_columns.sql:19-22（「展開はビューを作った時点で固まる —— 足した 2 列はビューに出てこない」ので作り直した）/ `docker exec ashiato2-st22-db-1 psql … information_schema.columns` で `core.event` と `core.event_live` の列は、いまは 21 列で一致
- kind: technical
- 提案: C4 に「書き出す列は `core.event` の列の一覧と突き合わせる（ビューの列が欠けていたら書き出しを落とす）」を足す。機械で判定できるので問いにはしない。書き出し直せば戻るので失われるものは無い
- 処置: fixed deep.md — C4 に「書き出す列は `core.event` の列の一覧と機械で突き合わせ、ビューから欠けていたら書き出しを落とす」を足した
