# ST14 深掘りの問いの独立レビュー（deep-review）

対象: `openspec/changes/st14-outage-alert/deep-questions.json`（Q1〜Q10、note の C1〜C10）
入力: `docs/stories/ST14.md`（FR-35 / 扉 14）、`docs/requirements.md`、`docs/handoff/ST14.md`、
`crates/server/src/{coverage.rs,lib.rs,archive/worker.rs}`、`crates/collector-windows/src/{contract.rs,marker.rs,runtime.rs}`、
`collector-android/app/src/main/kotlin/dev/ashiato/collector/*`、`migrations/*.sql`、
`openspec/changes/archive/2026-09-14-st07-active-window/{deep.md,deep-questions.json}`

## 独立にやり直した手順 1〜5（問いの一覧を見る前）の記録

- 手順 1（衝突）: FR-35 × FR-82 / NFR-13（PC を閉じた時間）、FR-35 × FR-78（取れない状態の信号）、FR-35 × FR-34（意図的な停止）、
  FR-35 の Takeout 初期値（60 日 × 3 = 180 日）× EXT-E / FR-13（Chrome は 90 日で消す）・NFR-12（書き出さなかった期間は消えれば戻らない）、
  FR-35 × FR-99（衛星）・FR-36 / FR-44（本人が書く）・FR-78 ★ 2026-09-15（書庫の各ソースに信号を送らない）を挙げた
- 手順 2（扉 14 の幅）: 「最後の記録」に何を数えるか（観測でない記録 = PC の時計の測定・離席・止まっていた期間・取りこぼしの印）、
  どの時刻で測るか（出来事の時刻 / 端末の発信時刻 / 届いた時刻 / 書庫の台帳）に幅が残る
- 手順 3（一方通行）: 外への push（exported）、登録簿に足す「判定の対象」の列の既定値、通知の台帳（C6）。
  Takeout 系の閾値が取得元の保持より長いこと（取り逃した期間が uncaptured になる）
- 手順 4（日常）: 通知の経路・回数・まとめ方・PC を閉じた日・問い合わせの間隔（電池）・稼働状況の画面に出る通知の置き場
- 手順 5（既存コード）: `coverage.rs` の `active_days`（`event_time` / `emitted_at` で測る）、`decide()`、`touch_started_on`、
  `lib.rs` の取り込み（重複でも開始日を当てる・`ingest_time` は挿入時刻）、`archive_ledger_source`（ソースごとの重複件数を持つ）、
  C-02 の記録の種類（`clock-skew` / `idle` / `powered-off` / `excluded` がどれも `c02-window`）、Android の `SourceCadence`（集計も生存信号を送る）を確かめた

---

## R1. Takeout 系の通知は 180 日後で、Chrome の履歴の保持（90 日）より遅い。書き出しを忘れた期間の一部は、通知が鳴った時点でもう取り戻せない（一覧に無い）
- 成果物: openspec/changes/st14-outage-alert/deep-questions.json
- 根拠: docs/requirements.md:386-397（FR-35。想定間隔の 3 倍 / Takeout 系 = 60 日）/ migrations/202609181600_archive_ingestion.sql:148-159（`c03-chrome-history` ほか c03-* は `5184000` 秒 = 60 日 → 通知は 180 日）/ crates/server/src/archive/worker.rs:943-955（マイアクティビティの製品ごとのソースも 60 日で足す）/ docs/requirements.md:1218（EXT-E: Chrome の履歴は直近 90 日）/ docs/requirements.md:214（FR-13: Chrome 系は 90 日を過ぎた履歴を消す）/ docs/requirements.md:1214（EXT-B: 予約エクスポートは 1 年で止まる）/ docs/requirements.md:951-952（NFR-12: 書き出さなかった期間は端末から消えれば戻らない）
- kind: conflict
- loss: uncaptured
- 提案: 「書庫のソースは何日で知らせるか」を A の問いとして足す。いまの初期値では、予約が 1 年で切れた後、180 日後に通知が鳴る時点で、Chrome の履歴のうち 90 日を過ぎた分（約 3 か月ぶん）は取得元に残っていない。選択肢の例: (a) 3 倍のまま（180 日）、(b) 書庫のソースだけ倍率を下げるか、ソースごとに通知までの日数を登録簿に持つ（例: Chrome は 90 日より前）、(c) 予約エクスポートが切れる時期（最初の書庫から 1 年）に別途知らせる。それぞれの選択肢に、取得元の保持期間を過ぎて失われる期間を書く
- 処置: escalated — A の問い Q9（loss: uncaptured。推奨: 書庫のソースは最後の書庫から 75 日）を立てた。選択肢ごとに取得元の保持を過ぎて失われる期間を書いた。予約が 1 年で止まる件は 75 日の通知が拾うので別の選択肢にしない

## R2. PC の時計の測定など「観測ではない記録」が、FR-35 の「最後の記録」を 1 時間ごとに新しくする。窓が読めなくなっても鳴らない（一覧に無い。C10 は Q7 次第で、しかも格子にしか効かない）
- 成果物: openspec/changes/st14-outage-alert/deep-questions.json
- 根拠: crates/collector-windows/src/contract.rs:30-46, 337（`clock-skew` / `idle` / `powered-off` / `excluded` はどれも `c02-window` で送る）/ crates/collector-windows/src/runtime.rs:925-950（1 日に `clock-skew` 24 件・生存信号 4 件。テスト `ticks_keep_heartbeat_and_skew_intervals_for_a_day`）/ crates/server/src/coverage.rs:897-911（`active_days` は種類を問わず `core.event` の `event_time` を数える）/ docs/handoff/ST14.md「st05-clock-skew R6」
- kind: conflict
- 提案: Q2 の推奨（取得できる状態の生存信号だけを数える）は生存信号の側しか閉じていない。前景の窓が読めなくなって生存信号が `capturable=false` になっても、毎時の `clock-skew` の記録が「最後の記録」を新しくし続けるので、Q2 でどれを選んでも鳴らない。B の問いとして「FR-35 の『最後の記録』に数えない記録の種類（`clock-skew`・`idle`・`powered-off`・`excluded`、c01-app-usage の `kind: gap`）」を足すか、Q2 に含める。C10 の除外は Q7 の答えと関係なく判定にも当てる、と書き直す
- 処置: escalated — C9 に「観測ではない記録（clock-skew / idle / powered-off / excluded / gap）は途切れの判定の『最後の記録』に数えない」を足し、Q7 の答えに関係なく判定に当てる形にした。格子の数えから除くのは clock-skew と gap（申し送りの範囲）

## R3. Q3 の推奨（書庫は届いた時刻で測る）では、Q3 自身が挙げた「YouTube を半年見なかっただけで鳴る」が直らない。携帯の側で挙げている違いも実際には出ない
- 成果物: openspec/changes/st14-outage-alert/deep-questions.json（Q3）
- 根拠: migrations/202609081618_envelope.sql:22（`ingest_time DEFAULT now()` は挿入したときの時刻）/ crates/server/src/lib.rs:1160-1190（重複した記録は行を作らない。`ingest_time` も進まない）/ migrations/202609181600_archive_ingestion.sql:53-62（`archive_ledger_source` は書庫ごと・ソースごとに `duplicate_count` を持つ ＝ 新しい記録が 0 件でも「この書庫がこのソースを運んだ」が残る）/ crates/server/src/lib.rs:2276-2287（`archives_status_for` が台帳から書庫の作られた時刻を引いている）/ crates/server/src/coverage.rs:897-911
- kind: premise
- 提案: (1) 書き出しはしたが、その製品を使っていなかった書庫では新しい行が 0 件になり、届いた時刻は進まない。推奨を選んでも、使わなかったソースは 180 日で鳴る。選択肢に「書庫のソースは、そのソースを運んだ最後の書庫（台帳）の時刻で測る」を足す。(2) context の「届いた時刻で測ると、圏外で 18 時間溜めた携帯は鳴る」は、出来事の時刻で測っても同じように鳴る（18 時間何も届いていなければ、どちらの最大値も 18 時間前）。携帯の側で 2 つが実際に分かれるのは、端末の時計がずれているとき（進んでいる時計の `emitted_at` は通知を遅らせ、遅れている時計は早く鳴らす。ST05 / FR-7 が測っている）と、サーバが止まっていた後。違いの説明をこの 2 つに直す
- 処置: escalated — Q3 の選択肢に「そのソースを運んだ最後の書庫の時刻（台帳）」を足して推奨にし、context に重複では行も届いた時刻も進まないことを書いた。why の違いの説明を「時計のずれ」と「サーバが止まっていた後」に直した

## R4. Q10 は ST07 の深掘り Q8 で本人が明示的に決めたことを引かずに聞き直している。その「前提」の書き方は事実と合わない
- 成果物: openspec/changes/st14-outage-alert/deep-questions.json（Q10）
- 根拠: openspec/changes/archive/2026-09-14-st07-active-window/deep.md:150-160（Q8「PC を開かない日に鳴る通知」。本人は**「許容する（想定間隔 6 時間のまま）」を明示的に選んだ**。退けた選択肢は「PC の電源が切れている間は通知の対象から外す」）/ docs/requirements.md:1148（C-02 が S-01 と同じ PC かどうかは定めない）/ crates/collector-windows/src/marker.rs:98-112（`powered-off` の記録は区間の始まりを出来事の時刻に持つ）/ deep-questions.json note の C7（起動した直後は判定を待つ）
- kind: premise
- 提案: (1) Q10 の why にある「PC を開かないことは壊れたことではない、が既に決めたこと」「FR-35 はこの前提を書いていなかった」は、ST07 Q8 の答え（鳴ることを許容した）と逆になっている。premise として立てるなら、ST07 Q8 の答えを引用したうえで、どの事実が違っていたかを名指しする。候補は 2 つ。ST07 Q8 の context「土曜の昼に必ず鳴る」は S-01 が PC の外にあることが前提で、同じ PC なら起動するまで判定は動かない。また選択肢 3 の「PC が切れていると C-02 が死んでいると外から区別できない」は、FR-82（`powered-off` と `boot_at`）の実装で、起動した後なら区別できるようになった。(2) context の「PC を開いた直後に鳴る」は、同じファイルの C7（起動直後は判定を待つ）と食い違う。同じ PC に置いた構成では、C-02 が起動時に送る記録が先に届くので、逐語どおりでも鳴らない。(3) S-01 と C-02 が同じ PC かは要件で決まっていないので、別の PC に置く構成（ST07 Q8 が想定した形）での振る舞いも選択肢に書く
- 処置: escalated — 旧 Q10 を問いから外し、C12 にした（ST07 Q8 の本人の答え「許容する」のまま、要件は変えない）。同じ PC / 別の機械の両方の振る舞いを C12 に書いた

## R5. Q7 は「どの Story で直すか」という段取りの問いで、daily ではない。申し送りは既に ST14 を宛先にしている
- 成果物: openspec/changes/st14-outage-alert/deep-questions.json（Q7）
- 根拠: docs/handoff/ST14.md（R6・st05 R6 は「ST14 の上流で collection-coverage の MODIFIED に畳む」、R5 は「ST14 で畳むのが安い」、R7 は「ST12 の archive 後」、st25 Q5 は「ST14 の上流で collection-coverage に NFR-17 を足す」と、宛先が既に書かれている）/ CLAUDE.md「分類できない問いは、たいてい人間に聞く必要が無い」「走っている Story へは差し戻さない」/ crates/server/src/coverage.rs:28（`DEVICE_SUBJECT` に `c01-app-usage` が残っている。Q7 の context の主張は正しい）
- kind: defer
- 提案: Q7 は削り、引き受けを既定にする（C 項目にして D 番号に残す）。人間が要るのは R31（本人の実機を USB で繋いで数える）だけで、これは選択ではなく物理的な作業なので、確認バッチの手順に回す。Q8・Q9 の「Q7 で引き受けない場合は使わない」と、C9・C10 の「Q7 で引き受ける場合」という条件を外す。なお「どこで直しても失うものは無い」は、R31 については言い切れない。数えて上限を超えていた場合、測るまでの間に長く圏外にいると、FR-9 の上限で捨てられる（discarded）。その一文を添える
- 処置: escalated — 旧 Q7 を問いから外し、C10（申し送りは全部引き受ける）にした。R31 は実機を繋ぐ物理の作業として確認バッチの手順へ、先延ばしにすると捨てられうる（discarded）一文を添えた。Q7（gap）・Q8（集計）と C9 の条件を外した

## R6. Q1 の推奨は「稼働状況の画面にも出す」を含むが、画面のどこに出すかの問いが無い。FR-55 の高さの予算と干渉する
- 成果物: openspec/changes/st14-outage-alert/deep-questions.json（Q1 / note「画面の構造の問いはありません」）
- 根拠: docs/requirements.md:624-631（FR-55: Must の 5 ソースの格子の前に書庫の箱を置き、「Must の 5 ソースのひとスクロールと開いた直後の 2 ソース」は箱の高さを除いて数える。上限 160 CSS px は仮）/ CLAUDE.md「画面の構造は文字で問わない」
- kind: daily
- 提案: Q1 で画面にも出すことになれば、途切れの表示（帯・箱・格子の見出しの印のどれか）が画面の構造を変える。note の「画面の構造の問いはありません」を取り下げ、`docs/briefs/ST14-proto.html` を作って `kind: visual` + `proto` の問いを 1 つ足す（書庫の箱との上下、ひとスクロールの予算に含めるか）
- 処置: escalated — Q1 の選択肢から「画面にも出す」を外し、C13（画面に新しい置き場を作らない。途切れは格子の⑥で見える）にした。画面の構造が変わらないので proto の問いは立てない

## R7. C1 は登録簿に「判定の対象」の列を足すが、既定値を書いていない。対象外が既定だと、新しい収集のソースが誰にも気づかれずに外れる
- 成果物: openspec/changes/st14-outage-alert/deep-questions.json（note の C1 / C2）
- 根拠: migrations/202609111111_coverage_rebuild.sql:127-133・202609240758_app_usage_rollup_source.sql:31-33・crates/server/src/archive/worker.rs:943-955（ソースはそれぞれの Story の移行や実行時の `INSERT` で足されていて、1 か所で足しているわけではない）/ docs/stories/INDEX.md（ST08・ST09・ST35 など、これから収集のソースを足す Story が残っている）/ CLAUDE.md の C の規則「既定は厳しい側」
- kind: technical
- loss: uncaptured
- 提案: C1 に「列の既定は『対象』（鳴らす側）。外すのは本人が書くもの・導出・衛星だけで、外すものに印を付ける」と書く。逆の既定にすると、印を付け忘れた新しい収集のソースは途切れても鳴らず、その間のデータは取れないまま残る。C2 の「導入前は鳴らさない」も同じ型で、一度も届かないまま壊れている収集は鳴らない。それを受け入れるなら C2 にそう書く
- 処置: escalated — C1 に「列の既定は『対象』（鳴らす側）。外すものに印を付ける」を書いた。C2 に「一度も届かないまま壊れている収集は鳴らない（格子の⑦で見える）」を受け入れると書いた

## R8. 携帯がサーバに問い合わせる間隔（電池と、気づくまでの遅れ）が問われていない
- 成果物: openspec/changes/st14-outage-alert/deep-questions.json（Q1）
- 根拠: openspec/changes/st17-daily-feeling/design.md:168-175（ST17 の D10 は 1 日 1 回の時刻で起こす仕組み）/ collector-android/app/src/main/kotlin/dev/ashiato/collector/Heartbeat.kt:19（生存信号は 6 時間ごと）/ docs/stories/ST14.md「位置の収集を 18 時間以上止めると通知が来る」
- kind: daily
- 提案: Q1 の推奨は「携帯が ST17 の仕組みに乗ってサーバに問い合わせる」だが、ST17 の起こし方は 1 日 1 回で、そのままだと 18 時間の判定から最長 24 時間遅れて鳴る。問い合わせの間隔（例: 生存信号と同じ 6 時間に相乗りする / 1 日 1 回）を B の問いとして足すか、Q1 の選択肢の detail に遅れと電池を書く
- 処置: escalated — C11（携帯は生存信号と同じ 6 時間、PC は時計の測定と同じ 1 時間に相乗り）を足し、Q1 の context に「携帯で鳴るのは止まってから 18〜24 時間後」を書いた

---

## 一覧の問いのうち、事実の主張を確かめて問題が無かったもの（記録）

- Q1 の context: PC の収集アプリは OS の通知を出さない（`crates/collector-windows/src` の「通知」は OS からの焦点の通知だけ）、ST17 の D10 は AlarmManager（`openspec/changes/st17-daily-feeling/design.md:168`）—— 合っている
- Q2 の context: 生存信号は `capturable` / `blockers` / 試行回数 / 成功回数を持ち、`decide()` は記録が 0 件で `capturable=false` の日を③にする（`coverage.rs:960-967`）—— 合っている
- Q6 の context: `c01-clock` は 1 時間ごと（`ClockSkewScheduler.kt:8`）で生存信号を送らず、登録簿の値は 21600（`202609291230_clock_source.sql`）、格子にも出していない（`lib.rs:2390-2401` は `must_sources()` と `c03-%` だけ）—— 合っている
- Q9 の context: 集計も生存信号を送る（`CollectionSource.kt` の `SourceCadence.APP_USAGE_ROLLUP`）—— 合っている。なお、開始日を 1 文の `UPDATE` で直しても、重複した記録が届くたびに開始日が当て直される（`lib.rs:1181-1190`）。ふだんは端末の指紋の台帳が古い箱を送り直さないので戻らないが、アプリを入れ直して台帳が消えると戻る。推奨を選ぶなら、`touch_started_on` の側の直しも Q9 の対象に入る
- Q10 の context: NFR-13 は PC の 2 ソースについて、生存信号が届いた日だけを分母にしている（`requirements.md:967-969`）—— 合っている（前提の誤りは R4）
- C3: 取り込み器の生存信号は 1 日 1 件（`archive/worker.rs:1297-1360`）、`c03-legacy-*` は取り込み器が退役させる（`archive/worker.rs:925-940`）—— 合っている
