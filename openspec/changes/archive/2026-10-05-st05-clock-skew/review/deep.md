# ST05 深掘りの独立レビュー（deep-questions.json）

**やり方**: `openspec/schemas/ashiato/schema.yaml` の `deep` の手順 1〜5 を、
問いの一覧を**開かずに**先に自分でやり直し、終わってから
`openspec/changes/st05-clock-skew/deep-questions.json`（2 問 / A 2・B 0）と、
呼び出し元が示した既定 C1〜C6 に突き合わせた。問いの JSON は編集していない。

## 手順ごとの結果

- **手順 1（satisfies の要件どうしの衝突）** —— satisfies は FR-7 の 1 件だけなので、同じ対象を語る
  周辺の要件と突き合わせた。FR-7 × NFR-13（端末が主語 = 記録が 1 件以上ある日。docs/requirements.md:567）は
  C1 が受けている。**C1 が新しい論理ソースを立てると FR-78（生存信号）/ FR-80（途絶）/ FR-35（通知）の
  対象になる点が、問いにも C にも無い（R3）。** C6 で FR-7 を C-02 に及ぼすと、C2 / C5 が既存の PC 実装と食い違う（R1）。
  FR-1 の「端末時刻」と実装の `Location.getTime()` の食い違いは Q2 が受けている。
- **手順 2（決定済の扉の幅）** —— 扉 #5（docs/requirements.md:821-824）。
  「基準時刻」が何か → Q1。「C-01 だけか」→ C6。**「同じ PC 構成で PC が自分の時計と比べている」ときに
  扉 #5 の「破れたことを後から知る」が PC について成り立たない点が無い（R2）。**
  「測定時刻」をどの時計で持つか → C5 は項目を並べるだけで、出来事時刻に何を置くかが無い（R8）。
- **手順 3（新たに立つ一方通行）** —— 取っていないもの（uncaptured）: 基準の種類 → Q1、位置の時刻の出どころ → Q2。
  **起動の識別と HTTP 基準の往復時間が C5 / Q2 に無い（R4）。** 鍵の入力: 冪等キーは
  `logical_source` + `event_time` + `raw`（crates/server/src/ingest.rs:176-179）—— `logical_source` は C1、
  `event_time` は R8。外に出るもの: Q1 の選択肢 2（外部の時刻サーバ）。捨てるもの: C2 が「取れなくても 1 件」に倒している。
  端末内の破棄（FR-9）は測定の記録にも掛かる —— ST04 に関わる。
- **手順 4（日常に影響する選択）** —— **該当なし**（確かめた範囲: 画面 —— `docs/stories/ST05.md` に面が無く、
  C1 のとおり稼働状況の画面は Must 5 ソースしか出さない（crates/server/src/lib.rs:1048 が `must_sources()` だけを引く）/
  常時通知 —— 前景サービスの通知は既存（LocationService.kt:208-219）で増えない / 容量 —— 1 時間 1 件で年 8,760 件 /
  電池 —— 送信は既に 5 分ごと（LocationFix.kt `SEND_INTERVAL_MS`）で、1 時間に 1 回の測定は桁が小さい）。
  通知が増えうるのは R3（新しいソースが ST14 の通知の対象になる）だけ。
  **確かめられなかったこと**: 端末が眠っている間に 1 時間の刻みがどれだけ伸びるか（実機が要る。ST01 の R46 は Doze を受け入れ済み）。
- **手順 5（既存コードが要件を満たしていない箇所）** —— Android に測定は 1 行も無い
  （`grep -rniE "skew|ntp|currentNetworkTime|currentGnssTime|elapsedRealtime" collector-android/app/src/main` → 該当なし）。
  これは未実装で、「動いているが満たしていない」ではない。`HttpTransport.kt` は応答の見出しを読まない（`Date` を取るなら変更が要る）。
  **動いているが満たしていないのは PC 側**: 壁時計を見回りの先頭で取り、窓の観測と置き場への保存の後で HTTP を叩いている
  （R4）。登録簿の既定で新しいソースが全件断られる（R3）。
- **画面の問い / 既定の無い仮の問い** —— 該当なし（面が無い / B の問いが 0 件）。

---

## R1. C6 で FR-7 を C-02 に及ぼすと、C2・C5 が既存の PC 実装と食い違う。どちらに揃えるかが無い

- 成果物: openspec/changes/st05-clock-skew/deep-questions.json（と、JSON 外の C2 / C5 / C6）
- 根拠: C2「基準が 1 つも取れない契機でも、取れなかった印を付けて 1 件残す」に対し、
  openspec/changes/archive/2026-09-14-st07-active-window/design.md:271「**取れなかった契機は推測で埋めない**（記録を作らず、1 分後に測り直す）」、
  crates/collector-windows/src/runtime.rs:402-411（`Err` のときはログ `clock_skew_unavailable` を出して `failed()` するだけで、記録を積まない）。
  C5「起動からの経過時間」「各基準の時刻と差」に対し、crates/collector-windows/src/clock.rs:82-87 の `measure` が載せるのは
  `skew_ms` と `skew_reference` の 2 項目だけで、基準は 1 つ（HTTP の `date`）、単調時計の値は載らない。
  正典 openspec/specs/desktop-collection/spec.md:288-291 は「1 時間経過 → 1 件残っている」で、取り込み口が止まっている間の PC はこれを満たさない。
  さらに PC の測定は `c02-window` に入っている（crates/collector-windows/src/contract.rs:249、`lib.rs:39`）ので、C1 の「別の論理ソース」とも揃わない。
- kind: conflict
- loss: uncaptured
- 提案: C6 を「FR-7 の本文を C-02 に及ぼす」だけで閉じず、改訂後の FR-7 に C2 / C5 を含めるなら PC 側も直す（desktop-collection を触る。R7）、
  含めないなら C2 / C5 を「C-01 のみ」と明記する —— の 2 択を 1 問にする。PC で失敗の印と経過時間を載せていない期間は後から作れないので A。
  PC の測定を `c02-window` に置いたままにするか（過去の行は凍結されていて移せない）も同じ問いの context に書く。
- 処置: escalated （Q3 を新設した。deep.md の Q3）

## R2. 同じ PC 構成では PC 側の測定が自分の時計と比べた 0 になり、S-01 / PC の時計のずれを誰も測らない

- 成果物: openspec/changes/st05-clock-skew/deep-questions.json
- 根拠: openspec/changes/archive/2026-09-14-st07-active-window/design.md:273-276「同じ PC 構成では基準がその PC の時計そのものになり
  `skew_ms` が 0 付近に張り付く」、同 :122「要件は C-02 が S-01 と同じ PC かどうかは定めない」。
  crates/collector-windows/src/clock.rs:118-126 は基準を `base_url` の `/healthz` にしか取らない。
  S-01 の時計は「D-01 に入った時刻」（FR-19。docs/requirements.md:170）と生存信号の受信時刻（FR-78。同 :287）を刻むが、
  `grep -rn "skew\|ntp" crates/server/src` に測定は無い。Q1 の context は「携帯端末でネットワーク時刻と自宅 PC の日付を並べると
  自宅 PC の時計のずれも後から見える」と書くが、それは選択肢 1 を選び、かつ端末が圏内で Android 13 以上のときに限られる。
- kind: irreversible
- loss: uncaptured
- 提案: C6 で FR-7 を C-02 に及ぼすなら、「PC（= S-01 と同居しうる）の測定に、取り込み口とは独立な基準を足すか」を 1 問立てる
  （候補: Windows の時刻同期の最終同期時刻とずれ / 外部の時刻サーバ。後者は Q1 の選択肢 2 と同じく外に出る）。
  Q1 の context の「自宅 PC の時計のずれも見える」は成り立つ条件を併記する。
- 処置: escalated （Q3 の ③ と選択肢 1 に入れた。Q1 の context に成り立つ条件を併記した）

## R3. C1 の新しい論理ソースを登録簿に足すと、既定のままでは全件が断られ、FR-78 / FR-80 / FR-35 の対象にもなる

- 成果物: openspec/changes/st05-clock-skew/deep-questions.json（JSON 外の C1）
- 根拠: migrations/202609120940_source_columns.sql —— `external_id_kind` の既定は `'record'` で、`'none'` に倒すのは
  「列が生まれた回に居た行だけ」。後から INSERT した行は `'record'` のまま →
  crates/server/src/lib.rs:343-350 が `external_id` の無い記録を `MissingExternalId` で断る。
  collector-android/app/src/main/kotlin/dev/ashiato/collector/Sender.kt:67-68 / 79-86 は `missing_external_id` を恒久扱いしないので
  **捨てずに未送信に残し続ける**（ST04 の 90 日の破棄に至れば消える —— ST04 に関わる）。
  migrations/202609081618_envelope.sql:9 は `expected_gap_sec integer NOT NULL` で、値を決めないと行が作れない。
  FR-78（docs/requirements.md:272-273）は「あるソースの収集が有効である間、そのソースの想定間隔ごとに生存信号」、
  FR-35（同 :258-260）は想定間隔の 3 倍で通知。C1 はこのソースに生存信号を送るか・想定間隔をいくつにするか・ST14 の通知の対象かを書いていない。
- kind: technical （元は conflict。登録簿の 1 行の値で、要件どうしの衝突ではない）
- 提案: 問いにはせず C1 に 3 行足す —— 移行で `external_id_kind = 'none'` を明示する / 想定間隔の値（仮）と反転条件 /
  FR-78 の生存信号を送らない（または送る）ことと、その根拠（1 時間ごとの測定記録そのものが稼働を示す、など）。
  いずれも登録簿の 1 行と導出の規則なので後から戻る。移行が `'none'` を書く試験を tasks に置く。
- 処置: fixed deep.md （C1 に `external_id_kind = 'none'`・生存信号を送らない・想定間隔（仮）を足した。想定間隔と生存信号の有無は design で D（仮）にする）

## R4. C5 と Q2 の並べる項目に「起動の識別」と「HTTP 基準の往復時間」が無く、突き合わせと誤差の幅が後から作れない

- 成果物: openspec/changes/st05-clock-skew/deep-questions.json（Q2 の選択肢 1 / JSON 外の C5）
- 根拠: 「起動からの経過時間」（C5 / Q2 の `getElapsedRealtimeNanos()`）は**再起動で 0 に戻る**ので、起動をまたいで
  位置の記録とずれの測定を突き合わせるには起動の識別が要る。ST04 の上流は同じ理由で端末に「壁時計・単調時計・起動回数」を持たせている
  （origin/docs/st04-upstream の openspec/changes/st04-offline-retention/design.md:42 / :56-58。端末内だけで、送る 1 行には載らない —— ST04 に関わる）。
  HTTP の `date` は秒で切り捨て（archive/2026-09-14-st07-active-window/design.md:267-269）で、往復時間が残らないと誤差の幅が再計算できない。
  PC の実装は壁時計を見回りの先頭で取り（crates/collector-windows/src/runtime.rs:211 → `tick_at`）、窓の観測・状態の保存（:238-256）の**後**に
  `maybe_measure_skew(wall, …)`（:268）で HTTP を叩く。要求の上限は 30 秒（crates/collector-windows/src/sender.rs:182）で、
  その間の遅れが `skew_ms` に負の向きで混ざり、どれだけ混ざったかは記録に残らない。
- kind: technical
- loss: uncaptured
- 提案: 問いにはせず C5（と Q2 の選択肢 1 の文面）に「起動の識別（起動回数）」「各基準を読む直前・直後の単調時計の値」を足す
  （列を持つ既定。費用はほぼ 0）。PC 側を直すかは R1 の答えに従う。
- 処置: escalated （問いにはせず C6 と Q2 の選択肢 1 に起動の識別と前後の単調時計を足し、本人に C として見せる。PC 側の遅れは Q3 の ②）

## R5. Q1 の選択肢 1 の「不可逆」が失うものを書いていない。選択肢 3 は選択肢 1 に支配されていて、実質の幅は「外に問い合わせるか」だけ

- 成果物: openspec/changes/st05-clock-skew/deep-questions.json（Q1）
- 根拠: 選択肢 1 の `irreversible` は「Android 12 以下ではネットワーク時刻が取れない」—— これは端末の制約で、
  **選択肢 1 を選んだ（2 を選ばなかった）ことで失うもの**ではない。選択肢 1 で失うのは「OS が一度も / 長く同期していない間の、外部の時刻との比較」（uncaptured）。
  `SystemClock.currentGnssTimeClock()` は API 29、`currentNetworkTimeClock()` は API 33
  （`~/Android/Sdk/platforms/android-36/data/api-versions.xml:50383-50384`）、minSdk は 30
  （collector-android/app/build.gradle.kts:13）なので衛星の時刻はどの対応端末でも呼べる。
  選択肢 3 は選択肢 1 の部分集合で、選択肢 1 は通信先を増やさず費用も小さい ——
  CLAUDE.md の C「片方の選択肢が扉を開けたままにし、費用が小さいもの」に当たる。
- kind: irreversible
- loss: uncaptured
- 提案: 選択肢 1（取れる基準は全部並べる）を C に移し、Q1 を「アプリが外部の時刻サーバに自分で問い合わせるか」の 2 択に絞る。
  問い合わせる側の不可逆は `exported`（端末の IP と時刻の問い合わせが外に出る）、問い合わせない側は `uncaptured`（上記）と書く。
- 処置: escalated （Q1 を外部の時刻サーバへの問い合わせの有無に絞り、全部並べるのを C5 に移した）

## R6. Q2 の context が、融合プロバイダを使っていることを書かず、未確認の外部の主張を事実として置いている

- 成果物: openspec/changes/st05-clock-skew/deep-questions.json（Q2）
- 根拠: 位置は `LocationServices.getFusedLocationProviderClient` から `PRIORITY_HIGH_ACCURACY` で取っている
  （collector-android/app/src/main/kotlin/dev/ashiato/collector/FixSource.kt:28-31）。GPS の位置プロバイダを直接使っていない。
  Q2 の context「衛星の測位（GPS）から来た位置は**衛星の時計**の時刻を持ち」「衛星由来の位置の時刻は正しいまま入っていて、直すと逆に 5 分狂う」は、
  融合プロバイダの位置にも当てはまるかを示していない。要件は外部の主張を URL と逐語つきで EXT 表に置く決まり
  （docs/requirements.md:785-791）だが、この主張には出所も逐語も無い。
  コードの事実（出来事時刻 = `Location.time`：FixCollector.kt:42、原文の `device_time` も同じ値：LocationFix.kt:62）は context のとおり。
- kind: premise
- 提案: context を「融合プロバイダの位置の時刻がどの時計から来るかは保証が見当たらない（未確認）」に直し、出所の逐語を足す。
  選択肢 3 の不可逆「衛星の正しい時刻を出来事時刻として持たなくなる」も同じ前提に乗っているので書き直す。
  確かめる手順（エミュレータか実機で時計を 5 分ずらし、`Location.time` と `System.currentTimeMillis()` を並べる）を完了の判定の近くに置く。
- 処置: fixed deep-questions.json （Q2 の context を融合プロバイダと逐語の出所で直し、未確認であることと確かめる手順を書いた）

## R7. ST05 が触る capability が INDEX の割り当て（record-envelope だけ）を超え、ST04 と ST08 に重なる

- 成果物: openspec/changes/st05-clock-skew/deep-questions.json（Q2 / JSON 外の C6）
- 根拠: docs/stories/INDEX.md:68 は ST05 を `record-envelope` にだけ置く。Android での測定と Q2（位置の原文を変える）は
  `device-collection`、C6（PC にも及ぼす）と R1 は `desktop-collection` の振る舞い。
  `python3 scripts/board.py` → `ST04 … device-collection … [上流] #44 draft`、`ST11 … [衝突待ち] device-collection を ST04 が触っている`、
  `ST08 … desktop-collection … [着手可]`。CLAUDE.md は同じ capability を 2 本が同時に触ると差し戻しが起きると定める。
  端末で壁時計と単調時計の食い違いを見る処理は ST04 の design D2（`clock_jump`、1 時間）と C3（PC の R17 は 60 秒）で重なる —— ST04 に関わる。
- kind: technical （元は conflict。capability の割り当ての照合で、要件どうしの衝突ではない）
- 提案: 問いにはせず、proposal の前に INDEX の capability 表へ訂正（ST05 → device-collection / desktop-collection）を足し、
  盤面が `衝突待ち` を出すなら specs 以降を ST04 の archive まで止める旨を deep.md に記す。
  ST08 を先に始めるかどうかとの順序も同じ箇所に書く。
- 処置: fixed deep.md （`docs/stories/INDEX.md` の capability 表に ST05 → device-collection を足し訂正の節を書いた。specs 以降は ST04 の archive を待つ旨を deep.md の「走っている Story との重なり」に書いた）

## R8. 測定記録の出来事時刻（冪等キーの入力）に、端末の時計と基準のどちらを置くかが決まっていない

- 成果物: openspec/changes/st05-clock-skew/deep-questions.json（JSON 外の C4 / C5）
- 根拠: C5 は「端末の時計・各基準の時刻と差」を並べるが、`event_time` に何を置くかを書いていない。
  `event_time` は冪等キーの入力（crates/server/src/ingest.rs:176-179）で、日の割り当て（crates/server/src/coverage.rs:248-252 の
  収集開始日）と時刻順の表示（FR-56）にも使われる。PC 側は端末の時計（crates/collector-windows/src/clock.rs:83 の `local`）。
  C2 の「取れなかった」記録では基準の時刻が無いので、置けるのは端末の時計だけ。
  原文に両方の時刻が残れば、読む側で引き直せる（後から戻る）。
- kind: technical
- 提案: 問いにはせず C4 / C5 に「出来事時刻は端末の時計（C4 と同じく補正しない）。基準の時刻は原文と解析済みの項目に持つ」の既定と反転条件を書く。
- 処置: fixed deep.md （C7）

## R9. C2 の「取れなくても 1 件」が、測り直しの刻みごとか契機ごとかを決めていない

- 成果物: openspec/changes/st05-clock-skew/deep-questions.json（JSON 外の C2）
- 根拠: PC 側は失敗すると 60 秒後に測り直す（crates/collector-windows/src/clock.rs:25 `SKEW_RETRY_SEC`、:61-63）。
  C2 を同じ刻みにそのまま掛けると、圏外 1 日で 1,440 件の「取れなかった」になり、契機（1 時間）ごとに 1 件なら 24 件。
  端末内の保持は件数とバイトで上限がある（FR-8 / NFR-7。docs/requirements.md:95-98 / :550）—— ST04 に関わる。
  どちらでも、取れなかった事実は残るので失うものは無い。
- kind: technical
- 提案: C2 に「取れなかった印は契機ごとに 1 件。測り直しで取れたら別の 1 件」の既定を書き、tasks に件数を数える試験を置く。
- 処置: fixed deep.md （C2 に「契機ごとに 1 件」を足した）
