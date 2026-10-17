# CH01 の深掘りの独立レビュー（deep-review）

対象: `openspec/changes/ch01-st17-review-fixes/deep-questions.json`（8 問）/ `premises.md`（10 行）

schema の手順 1〜5 と手順 6〜8 は、問いの一覧を開く前にやった。一覧と重なったものは書いていない。
CH01 は `satisfies: []` なので、手順 1 の「要件」は、この change が触る正典の `openspec/specs/subjective-log/spec.md`
（と、Q7 が触る `personal-entities`）に置き換えた。

- 手順 1（衝突）: 依頼の 5（滞在の行の印を戻す）と spec:745（日の欄は「送れなかったとき、選んだ値…を消さずに」）・spec:867（滞在の行は「最新の記録の値を押された状態で」）は、
  日と滞在で逆の規則を言っている → R4。依頼の 6 と spec:748（「開いたとき、…いま書けば付く前/後」）→ R5
- 手順 2（扉）: CH01 には扉が無い。ST17 の扉 #1（事前/事後の区別）と #2（紐づけ先の多相参照）を見た。
  前/後を時刻から導く形（spec:255-257）は Q5 のどの選択肢でも変わらない。紐づけ先の種類の登録簿にも触れない。幅は無い
- 手順 3（一方通行）: 主観の行は消せない（spec:322）ので、Q3 で受け付ける範囲を広げるとその行は残り続ける（Q3 の irreversible に書いてある）。
  Q7 の理由の種別は API の契約（Q7 に書いてある）。指摘 3 のトークンが追跡ファイルに残ると、commit・push で外に出る経路がある → R7
- 手順 4（日常）: 夜の通知（既定 21 時）から開いたまま 0 時をまたぐのが、日常で起きる場面（Q5）。Q6 で 1 回にまとめると、読み出しが失敗したときに日の欄と暦の両方が「読めなかった」になる（Q6 に書いてある）。抜けは無い
- 手順 5（既存コード）: 4〜9 をコードに当て、9 は台本でも測った。合わなかったのは R1（1 の移し替え）・R2（9 の索引）・R3（4 の再現の筋）
- 手順 6（過去の取りこぼし）: 当てはまったのは次の 4 つ。
  「DB を共有する連鎖で、前段が残す行に後段が依るか」（ST33 Q10）→ R1。
  「見積もり・根拠を実物で測っていない」→ R2。
  「既存関数の出力に乗せる Scenario は、例を通して確かめる」→ R3。
  「他 Story の決定の波及」→ R6。
  時計の観点（ST02 Q29・ST08 Q7）は R5 で Q5 に当てた
- 手順 7（族）: Q5（開いたまま 0 時をまたぐ）の兄弟は次の 4 つ。
  (a) 端末の時計がずれている・飛ぶ。画面もサーバも記入の日時（端末の時計。FeelView.tsx:288,304）で前/後を決めるので、食い違わない。
  (b) 日付の無いアドレスで開いたまま日をまたぐ。spec:735 が「開いた時点の今日」と決めている。
  (c) 暦の「今日」と「これから」は、読み出した時点のサーバの日付で決まる（spec:485）。
  (d) 続いている滞在に書くと、終わりが読んだ時点の値なので「後」になる（spec:258 の決定どおり）。
  4 つとも決まっている。決まっていないのは Q5 の選択肢 1 のタイマーが遅れる場合だけ → R5
- 手順 8（前提の確かさ）: `loss` がある決定は Q3（discarded）だけ。Q3 は前提 4 に依るが、前提 4 は走らせておらず確かさが 3 → R3。
  前提 9（確かさ 2）の結論は、測ると逆になった → R2。外の形式に依る決定は無く、premises.md の冒頭の「外の形式に依る決定なし」と合っている

---

## R1. Q1 の「検査の中身は変えない移し替え」は成り立たない。`check-immutable.d/` の節は、schema を作り直して版を戻し終えた後に読まれる
- 成果物: openspec/changes/ch01-st17-review-fixes/deep-questions.json（Q1 の why / context）/ premises.md（前提 1）
- 根拠: tools/check-immutable.sh:1352-1353（「戻し手順を逆順に当てる」が `DROP SCHEMA core CASCADE` をしてから全版を当て直す）/
  tools/check-immutable.sh:1367-1374（`subjective_log` は down → up → down の順に当てるので、**戻した状態で終わる**。ST21・ST22・ST19・ST12・ST04・ST16 も同じで、当て直すのは ST03 の 5 本だけ: :1444-1447）/
  tools/check-immutable.sh:1507-1512（`.d/*.sh` を読む口は、その後ろにある）/
  ST17 の節は前の節が作った状態に依っている: `$AU` は ST19 の節で決めている（tools/check-immutable.sh:619）、最後の Scenario は `immutable-check` の authored の行を書き換える（:1320-1326。行は :63-128 で作っている）/
  `tools/check-immutable.d/` には README.md しか無い（`ls tools/check-immutable.d/`）。この型で動いている節はまだ 1 本も無い /
  読む口の周り（末尾）を動かすと、`feat/ch*` では tools/check-shared-appends.sh:14,46 が落とす
- kind: premise
- 提案: Q1 の context を直す。移すと、ST17 の節が読む DB の状態が変わる（主観の表が無い・`$AU` が無い・`immutable-check` の行が無い）ことを書く。
  選択肢 1 と 2 には「節の頭で、要る版を当て直し、変数と行を自分で用意する（節が長くなる）」を書き足す。
  読む口を戻しの前へ動かすのは本体の末尾に触れて CI が落とすので、案に入れない。ST28 の節（:1466-1500）も本体に残っていることを列挙に足す
- 処置: fixed deep-questions.json — Q1 の why と context を直した（読む口が戻しの後にあること・前の節の状態に依ること・`*.d/` の実績が無いこと・読む口の移動を check-shared-appends が落とすこと）。選択肢を「読む口を動かして自立した 1 本で移す（推奨）/ 本体に残す / 全部移す」に組み直した

## R2. 前提 9 と Q8 の推奨は誤り。`(logical_source, event_time)` の索引は既にあり、測ると最初の主観へ 4 ページで着く
- 成果物: openspec/changes/ch01-st17-review-fixes/premises.md（前提 9）/ deep-questions.json（Q8 の why と推奨）
- 根拠: migrations/202609112113_source_lifecycle.sql:71-72 `CREATE INDEX event_by_source_time ON core.event (logical_source, event_time)`（`migrations/ORDER` の 12 行目。:12 の down 以外に DROP は無い。`grep -rn event_by_source_time migrations/`）/
  crates/server/src/feelings.rs:624-626（`WHERE user_id = $1 AND logical_source = $2 AND raw <> ''` の `min(event_time)`。先頭の列が等号で決まるので、この索引を前から 1 行読めば済む）/
  使い捨ての postgres:17 で `(user_id, logical_source, event_time, raw, deleted_at)` の表を作った。位置 100 万行・主観 500 行に索引 2 つ（`event_by_source_time` と `event_by_user_time_live`）を張り、ANALYZE してから `EXPLAIN (ANALYZE, BUFFERS)` を取った →
  `Index Scan using event_by_source_time`・`Buffers: shared hit=1 read=3`・`Execution Time: 0.079 ms`。
  premises.md の「近いのは event_by_user_time_live で…位置などの行も読む」とは逆になった（ashiato2-verify-db の 14 行では `event_by_user_time_live` を選んだ。統計で変わる）
- kind: premise
- 提案: 前提 9 を「索引はある。EXPLAIN で最初の主観に着くまでの読みは 4 ページ（確かさ 4。条件は実データでなく合成の 100 万行）」と直す。
  依頼の規則（再現しなければ直さずに理由を残す）に当たるので、Q8 は問いから外し、「直さない。理由は `event_by_source_time`」と C に回す。
  残すなら、推奨を選択肢 3（測って遅くなければ直さない）に移す。利用者が複数いて他人の主観が古い場合だけ読みが増える、と 1 行添える
- 処置: fixed premises.md — 前提 9 を「既存の索引 event_by_source_time に乗る。合成データで 0.079 ms」に直し、Q8 を問いから外した。9 は再現しないので直さず理由を残す（C。HTML の冒頭に記載）

## R3. Q3 の再現の筋が、製品のどの経路でも起きない。「本文を消した滞在」は製品のコードからは作られず、終わりの無い滞在の扱いは ST16 の決まりに沿っている
- 成果物: openspec/changes/ch01-st17-review-fixes/deep-questions.json（Q3 の why / context / 選択肢）/ premises.md（前提 4）
- 根拠: 滞在の本文を `payload='{}'` にするサーバのコードは無い（`grep -rn "raw *= *''" crates --include=*.rs` で出るのはコメントと test だけ。`erasure_ledger` を書くのも export.rs:464 の読み出しだけ）/
  消す操作は印だけを付ける: crates/server/src/deletion.rs:282,290（`SET deleted_at, deleted_by`）/
  作り直しは同じ id の本文を新しい始まり・終わりで書き換える: crates/server/src/stay_store.rs:971-984（`update_stay` が `payload` と `event_time` を入れ替える）。
  Q3 の context の「作り直しで滞在が 9:10 からになり、本文が消される」は起きない（9:10–11:40 の本文になり、9:12–11:40 は重なるので受け付ける）/
  終わりが読めない滞在は、取り込みの口から入った形の崩れた行として起きうる。その読み方は「始まりと同じと読む」と決まっている（crates/server/src/stay_store.rs:293-307 `span_of`。ST16 の design D2）。feelings.rs:294-298 はこれと同じ読み方をしている /
  why の「断られた気分は画面から消え」は、指摘 5 と前提 5 に合わない（web/src/FeelView.tsx:306 で印は残る）/
  前提 4 の確かさは 3（辿っただけで走らせていない）なのに、Q3 は loss のある A
- kind: premise
- loss: discarded
- 提案: A の問いとして出す前に、前提 4 を走らせて確かめる（どの経路で `payload` に `end` の無い `s01-stay` ができるか。確かさ 4）。
  製品の経路で起きないなら、依頼の規則どおり「直さず、コメントの『消した滞在も通す』を『削除の印の付いた滞在を通す。終わりの読めない滞在は span_of と同じく始まりの一点』に直す」を C にする。
  起きる経路があるなら、context をその経路に書き直す。「ST16 の span_of と違う読み方になる」を選択肢 1 と 2 の detail に足す
- 処置: escalated — Q3。Q3 を A（loss: discarded）から B に直した。why に「製品に滞在の本文を消す経路が無い」、context に ST16 の span_of と同じ形であることを書き、推奨を「直さない・コメントを正確にし、本文の消去を作る Story へ申し送る」にした。premises.md の 4 も直した。record の段で deep.md の Q の項にこの R 番号を書く

## R4. Q4 は spec の 2 つの要件の食い違いを、表示だけの話として仮で問うている。選択肢 1 と 2 は、日の欄の要件（spec:745）を書き換える
- 成果物: openspec/changes/ch01-st17-review-fixes/deep-questions.json（Q4）
- 根拠: openspec/specs/subjective-log/spec.md:745（日の欄: 「送れなかったとき、選んだ値と入れたひとことを消さずに、断られたのか届かなかったのかを区別して出す」。断られたときも含む）/
  spec:867（滞在の行: 「その滞在の最新の記録…の値を押された状態で出す」。指摘 5 は、いまの実装 web/src/FeelView.tsx:306 がこちらに合っていないということ）/
  Q4 の context は「日の欄も戻さない（同じ形）」と書くだけで、それが spec:745 の決めた振る舞いだとは書いていない
- kind: conflict
- 提案: context に 2 つの要件を並べて引く。日の欄は spec が「消さない」と決め、滞在の行は spec が「最新の記録」と決めていて、いまのコードは日の欄では spec どおり、滞在の行では spec に合っていない、と書く。
  選択肢 1 と 2 には「spec:745 を MODIFIED にする（日の欄の Scenario『届かなかったとき選んだ値とひとことが残る』と、断られたときの扱いが変わる）」と書く。
  選択肢 3 は「spec どおりに直すだけ」と書く
- 処置: escalated — Q4。Q4 の why に spec の日の欄の決定（断られても消さない）を書き、推奨を「滞在の行だけ直す。日の欄は spec どおり」にした。日の欄も変える選択肢には spec の書き換えであることを明記した。record の段で deep.md の Q の項にこの R 番号を書く

## R5. Q5 の選択肢 1 と 2 は「開いたとき」の要件（spec:748）を書き換えるのに、そう書いていない。選択肢 1 の「押す前から一致する」には、タイマーが遅れた場合の扱いが無い
- 成果物: openspec/changes/ch01-st17-review-fixes/deep-questions.json（Q5）
- 根拠: openspec/specs/subjective-log/spec.md:748（「**開いたとき**、…いま書けば付く前/後の最新の記録の値をボタンとひとことに出し」）/
  archive の design.md:201（「開いたときの選択」）。いまのコード（web/src/FeelView.tsx:98,123,222）は spec どおりで、指摘 6 は spec の決定が生む食い違い /
  選択肢 1 は画面が 0 時に自分で切り替わる時計（タイマー）を持つ。開いたまま携帯が眠る・タブが後ろに回ると、タイマーは遅れて発火する。
  その間に押すと、選択肢 1 でも画面は「前」・サーバは「後」になる（時計の族: ST02 Q29・ST08 Q7）
- kind: conflict
- 提案: context に「いまの形は spec:748 どおり。選択肢 1 と 2 は spec:748 を MODIFIED にする」と足す。
  選択肢 1 の detail に、押したとき（と画面が前に戻ったとき）にも前/後を判定し直すかを書く（書かないなら「眠っていた間は一致しない」と書く）。
  時計がずれる・飛ぶ場合は、画面もサーバも端末の時計で決めるので食い違わない、と 1 行添える（兄弟の場合が決まっていることを示す）
- 処置: escalated — Q5。Q5 の why に spec の「開いたとき」どおりであることと、直すならその文を書き換えることを書いた。タイマーの遅れを選択肢に書き、推奨を「押したときに判定し直す」に移した。record の段で deep.md の Q の項にこの R 番号を書く

## R6. Q7 の選択肢 1 は、CH01 の capability に無い `personal-entities`（主張・場所）の拒む順を触る
- 成果物: openspec/changes/ch01-st17-review-fixes/deep-questions.json（Q7）/ docs/changes/CH01.md:6
- 根拠: docs/changes/CH01.md:6 `capability: [subjective-log, browsing-views]` /
  crates/server/src/lib.rs:1015-1051（主張）・:1091-1104（場所）は、`openspec/specs/personal-entities/spec.md:96-97`（主張の拒む表）・:736（場所の拒む表）の順を実装している /
  `docs/stories/INDEX.md:87`（`personal-entities` は ST19・ST20・ST21・ST24・ST37・ST39 の capability）
- kind: technical
- 提案: Q7 の選択肢 1 の detail に、次の 3 点を足す。
  (1) まとめると `personal-entities` の振る舞いにも触れる。
  (2) 固定する Scenario（いまの 3 つの拒み方）は `personal-entities` の delta にも置き、CH01 の capability に `personal-entities` を足す。
  (3) 走っている Story が `personal-entities` を触っていれば着手の判定で当たる。
  選択肢 2（まとめない）の detail には「capability は増えない」と書く
- 処置: fixed deep-questions.json — Q7 の context に、まとめるなら capability に personal-entities を足し、その spec の Scenario とテストで拒み方を固定することを書いた

## R7. 指摘 3 を「直し方が 1 つしかない」として問わずに直す（C）の根拠が弱い。trap は KILL で走らず、トークンを追跡ファイルに書かない直し方もある
- 成果物: openspec/changes/ch01-st17-review-fixes/deep-questions.json（note の「3（トークンを戻す trap）は直し方が 1 つしかない」）
- 根拠: scripts/record-run:555-556（子のプロセスグループに TERM を送り、1 秒後に KILL）・:582-584（環境のプロセスに TERM、10 秒待って KILL）。
  bash の trap は前景の gradle が終わるまで走らず、KILL では走らない /
  scripts/record-run:593（片付けは worktree を `--force` で消すので、record-run 自身が止まらない限り、ファイルは残らない）/
  .gitignore:29（`collector-android/local.properties` は追跡されない）/ tools/record-env.sh:166-170
- kind: technical
- 提案: C のままでよいが、note の理由を直す。「trap は TERM・INT・EXIT で戻す。KILL の後は、片付けの `worktree remove --force` で消える。
  追跡されないファイルに書く案（`app/build.gradle.kts` の読み先を変える）は、差分が大きいので採らない」と書く。
  record-run 自身が落ちて worktree が残る場合を気にするなら、そのとき追跡ファイルに残る（commit・push で外に出る）ことを 1 行添えて、問いに上げる
- 処置: fixed deep-questions.json — note の 3 の理由を「trap だけでは KILL で戻らないので、追跡されないところからトークンを渡す形に直す」に改めた（C のまま）

