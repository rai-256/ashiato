# ST17 の上流成果物の独立レビュー（spec-review）

対象: `openspec/changes/st17-daily-feeling/`（proposal.md / specs/subjective-log / specs/browsing-views / design.md / tasks.md）と
`docs/stories/ST17.md`・`docs/requirements.md`（FR-36〜43・FR-57 の ★ 2026-10-07）・`docs/stories/INDEX.md`。
deep.md（Q1〜Q13・C1〜C10）は正として読み、成果物がそれを写しているかだけを見た。

## 機械の検査（最初に写す）

| コマンド | 結果 |
|---|---|
| `openspec validate st17-daily-feeling --strict` | `Change 'st17-daily-feeling' is valid`（rc=0） |
| `python3 scripts/check_chain.py .` | `chain: OK (0 件 / 未回収 0 件 / warn 0 件)`（rc=0。観点 8 の再生成との一致も通る） |
| `python3 scripts/check_scenarios.py . st17-daily-feeling` | `scenarios: FAIL (担保なし 136 件 / 名無しの確認待ち 0 件)`（rc=1）。実装前なので 136 本すべて担保なし。Scenario の数は tasks の Global Constraints の「136 本（129 / 7）」と一致し、spec の全 Scenario 名が tasks.md のどこかに逐語で出ることを grep で確かめた（欠け 0） |
| `python3 scripts/review_triage.py . st17-daily-feeling` | `triage: OK`（rc=0。この時点で数えたのは `review/deep.md` の 13 件だけ） |

## 確かめた範囲（指摘にならなかったもの）

- deep の決定の写り: Q2（ひとこと 1 欄・2,000 字）/ Q3（`energy` 版 1）/ Q4（滞在の口）/ Q5（未来の日・前後は機械）/ Q7（21:00）/ Q8（書いた日は鳴らさない・確かめられなければ鳴らす）/
  Q9（夜中も暦の今日。Scenario `日付の無いアドレスは夜中も暦の今日を開く`）/ Q10 / Q12（`event_time` = 対象の時刻。Scenario 2 本と拒否 1 本）/ Q13 / C1 / C2（錠・乱数）/ C4 / C6 / C8 / C9 / C10 は、
  対応する Scenario が存在し、答えと同じことを言っている。C3 の当初案（記入日時を `event_time` に入れる）は Scenario に残っていない
- 要件へ戻すもの 7 件（FR-36 / 39 / 40 / 41 / 42 / 43 / 57）は `docs/requirements.md` に `★ 2026-10-07` つきで入っている（441・449・453・457・460・464・587 行）。`ST17.md` はその逐語と一致する
- `ST17.md` の「完了の判定」4 つは deep の決定と食い違わない
- `specs/` に関数名・crate 名・列名は無い（`grep "core\.\|\.rs\|\.tsx\|\.kt\|payload\|event_time\|s01-feeling"` が 0 件）
- proposal の Capabilities（`subjective-log` 新規 / `browsing-views` ADDED 1 本）と `specs/` のディレクトリは一致し、`INDEX.md` の capability 表と 298〜304 行の訂正とも一致する
- ST17 の `browsing-views` の要件の名前は、正典と ST25 の delta のどれとも重ならない
- tasks の道具は実在する: `scripts/quiet-run` / `tools/android-emulator.sh` / `tools/check-migrations.sh` / `tools/check-immutable.sh` / `tools/check-openapi.sh` / `tools/check-log-private.sh` /
  `tools/seed.sh` / `tools/smoke.sh` / `crates/server/src/grants.sql` / `crates/server/src/deletion.rs` / パッケージ名 `ashiato-server` / `npm run lint`。
  `CT` / `VT` / `ET` / `GT` は件数つきで、0 本では通らない形になっている
- 既存コードの参照: `authorize()`（`lib.rs:468`）・`default_sensitivity()`（`lib.rs:576`）・`ingest_one`（`lib.rs:696`。ST19 の `parse_claim` の差し込み方）・
  `coverage::must_sources()`（`lib.rs:1999`）・`Root.tsx` の経路（`#/master` / `#/day/…` / ルート）・`POST_NOTIFICATIONS`（AndroidManifest.xml:30）は design の記述どおり

---

## R1. ST16 の深掘り Q4 で本人が決めた「S-2 の滞在の行の中に主観の本文を出す」が、持ち主の無いまま落ちている
- 成果物: openspec/changes/st17-daily-feeling/design.md（Non-Goals）/ deep.md（Q1 の帰結）
- 根拠: `docs/ui-direction.md:197-202`（★ 2026-09-13「S-2 の 1 行は、主観と人物の本文を行の中に出す」「**置き場を作るのは ST17 / ST20**」）/
  `openspec/changes/archive/2026-09-15-st16-stay-derivation/deep.md:103`（本人が proto で動かした軸。ST17 / ST20 に渡す）/
  `docs/ui-direction.md:229-230`（★ 2026-10-07「★ 2026-09-13 の『S-2 の行の中に主観の本文を出す』は**出す場所の話**で、書く口は S-3 に置く」——表示は残ると読める）/
  design.md:32-33（「S-2 の滞在の行の中に主観の本文を出すこと」を Non-Goals に置き、「載せるかはまだどの Story も持たない」）/
  deep.md:96（「滞在の気分は S-3 に置いたので、S-2 の滞在の行には手を入れない」—— 本人の出力（deep-answers-1.txt:8）は「**書く**」置き場を選んだだけで、表示については言っていない）
- kind: conflict
- 提案: 本人が決めた表示（ST16 Q4）を誰が持つかを決める。ST17 の下流は ST25 の archive を待つので、ST17 の `browsing-views` に「滞在の行に、その滞在の最新の気分を出す」を ADDED で足せる。
  足さないなら design の Non-Goals の「本人は … S-2 の行には手を入れないと選んだ」を「書く口の選択であって表示は未割当」に直し、`ui-direction.md` と handoff（ST20 / ST25 の後続）に持ち主の無いことを書く
- 処置: fixed D20 仮 —— 足す側を採った。browsing-views に ADDED「1 日の画面の滞在の行に、その滞在の気分が出る」（Scenario 4 本）を置き、design の Non-Goals から外した。deep.md の Q1 の帰結に「書く口の話」と訂正を書き、INDEX の 2026-10-07 の追記にも残した。ひとことは行に出さない（行の高さ）。tasks 10.1 / 10.2

## R2. S-2 の頭に気分の欄を足すと、ST25 の「360×640 で 6 行以上見える」が落ちうる。ST17 の spec にも tasks にもそれを守る手段が無い
- 成果物: openspec/changes/st17-daily-feeling/specs/browsing-views/spec.md / tasks.md（Task 9）
- 根拠: `openspec/changes/st25-day-timeline/specs/browsing-views/spec.md:495`（「開いてすぐの高さ 640 CSS px に … 行が 6 行以上見えるようにする」）/
  `openspec/changes/st25-day-timeline/design.md:220`（本人の測った日で、頭に箱を置くと開いてすぐ見える行が 4 になった実測）/
  ST17 の browsing-views spec.md:5-12 は欄を「見出しの下、並びより上」に置くが高さの上限が無い / tasks.md:151-153（9.2 の e2e は欄の位置と破線だけを測り、ST25 の e2e を走らせない）。
  あわせて spec.md:12 の「その日の並び（滞在・移動・記録なし・消した の行）」は ST25 が足す「取れていない時間の行」（ST25 spec.md:493）を落としている
- kind: conflict
- 提案: browsing-views の要件に「気分の欄を足しても、ST25 の『6 行以上』を保つ（欄の高さは N px 以下）」を Scenario つきで足す。
  Task 9.2 の検証に ST25 の 1 日の画面の e2e（6 行・200 px・横溢れ）を足す。並びの列挙は ST25 の 5 種に揃えるか、「その日の並びの行」とだけ書く
- 処置: fixed D14 仮 —— 気分の欄を 1 行・高さ 40 px 以下にする SHALL と Scenario `気分の欄は 1 行で 40 px 以下` を置き、並びの列挙を「その日の並びの行」に直した。Task 10.2 で ST25 の 1 日の画面の e2e（`day-timeline.spec.ts`）と ST16 の `day-stays.spec.ts` も走らせる

## R3. 日をまたぐ滞在（たいてい毎朝の「自宅で寝ていた」滞在）は S-3 の今日の行に並ぶのに、そこに書いた記録は前の日に数えられる
- 成果物: openspec/changes/st17-daily-feeling/specs/subjective-log/spec.md（「主観の無い日を数える」「その日の滞在ごとの気分を書く」）/ design.md（D13）
- 根拠: `crates/server/src/stay_store.rs:918`（「滞在は実際の始まりと終わり（**日をまたいでも切らない**）」）/ `stay_store.rs:1024-1025`（その日に**重なる**滞在を両方の日に並べる）/
  design.md:196（S-3 の滞在の並びは `GET /stays?date=` の `kind: "stay"`）/ spec.md:378（記録が紐づく日は始まりの日）/ spec.md:432-433（Scenario は数え方だけ）。
  結果として、10/8 の S-3 で 23:10–07:30 の滞在の行を押すと、記録は 10/7 に数えられ、10/8 は「未記入」のまま・通知も鳴る。
  さらに design.md:197 は行の最新を `GET /feelings` の `stays[].stay` で引くと書くが、その記録は 10/7 の `days[]` に入るので、10/8 の行には押された状態が出ないことがある。どれも Scenario が無い
- kind: daily
- 提案: S-3 の滞在の行の Scenario を足す（日をまたぐ滞在の行に書いたとき、その行に押された状態が出るか / 対象の日の状態が何になるか）。
  数え方（D7（仮））はそのままでも、行を「前の日の滞在」と分かる形で出す・行の値は日をまたいで引く、のどちらかを spec に置く。本人の毎朝の操作に効くので、仮決めなら PR 本文に挙げる
- 処置: fixed D13 仮 —— 数え方（始まりの日）は D7 のまま、前の日から続く滞在の行に「前の日から」を添え、行の値は読んだ 30 日の全部から滞在の識別子で引く形にした。Scenario `前の日から続く滞在の行は前の日からと出る` / `前の日から続く滞在に書いた気分はその行に出る`（今日の状態は変わらないことも THEN に置いた）。反転条件（両方の日に数える）を D13 に書いた。PR 本文の仮決めに挙げる。tasks 9.1

## R4. C7（主観を途絶に数えない）は FR-35 の本文と食い違うが、要件へ戻されず、FR-35 を作る ST14 への申し送りも無い
- 成果物: openspec/changes/st17-daily-feeling/deep.md（C7）/ specs/subjective-log/spec.md（「主観を書かない日は途絶として扱わない」）/ design.md（D1 の登録簿の行）
- 根拠: `docs/requirements.md:353-355`（FR-35「**あるソース**の最後の記録 … が、そのソースに登録された想定間隔の 3 倍を超える THEN 通知する」。除外は退役だけ）/
  design.md:66（登録簿の行 `('s01-feeling', '主観', 86400, 'none')`。FR-35 の逐語どおりに作ると 3 日書かなければ途絶）/
  `grep -n "expected_gap\|主観" docs/handoff/ST14.md` は 0 件。spec の Scenario（spec.md:370-373）は稼働状況の口を見るだけで、まだ無い ST14 の途絶の判定は撃てない
- kind: conflict
- 提案: FR-35 に「本人が書くソース（主観・個人属性）は対象にしない」を ★ で足す（C7 は聞かずに決めた C なので、要件の改訂として deep.md の「要件へ戻すもの」に足す）。
  `docs/handoff/ST14.md` に「`s01-feeling`（と `s01-attribute`）を途絶の判定から外す」を書く
- 処置: fixed D19 仮 —— C7（本人の異論なし）を要件に戻した: FR-35 に ★ 2026-10-07「本人が書くソース（主観・個人属性）は対象にしない」、deep.md の「要件へ戻すもの」に FR-35 を足し、`make_story.py` で ST14.md を再生成した（`check_chain.py` OK）。`docs/handoff/ST14.md` に `st17-daily-feeling R4` を書いた。spec の要件は ST17 で撃てる「稼働状況のソースに含めない」に絞り、途絶の通知は ST14 へ渡した

## R5. 「通知の判断の口は収集側の合言葉でも読めるので中身を返さない」の前提が事実と違う。同じ合言葉で `/events` と新しい `/feelings` が中身を全部返す
- 成果物: openspec/changes/st17-daily-feeling/specs/subjective-log/spec.md:527 / design.md（D9・Risks）/ proposal.md:80
- 根拠: `crates/server/src/lib.rs:2146-2166`（`/events` は `authorize()` だけで `core.event_live` の全行の `raw` を返す。ソースも感度も絞らない）/
  design.md:12（どの口も Bearer でも session でも通る）/ spec.md:442（`GET /feelings` も「資格情報を持つ呼び出し元」に尺度・ひとことを返す）/
  design.md:245（Risks は「『その日に書いたか』は漏れる」とだけ書く）/ `docs/handoff/ST29.md`（申し送りは `/events` を挙げるだけで、ST17 が足す `/feelings` を知らない）
- kind: technical
- 提案: spec.md:527 と design の Risks を「収集側の合言葉は ST29 まで主観の中身も読める（`/events` と `/feelings`）。判断の口を真偽だけにするのは、ST29 で範囲を絞ったときに収集側に残す口を最小にするため」に直す。
  `docs/handoff/ST29.md` に `/feelings` と `/feelings/reminder` を足す
- 処置: fixed D9 —— **kind を premise から technical に直す**（レビューは premise）。理由: 誤っていたのは本人の決定の根拠ではなく、私が spec / design に書いた理由づけ。収集側の合言葉で `/events` が全記録を読めることは ST28 Q4 で本人が承知して「ST29 まで全読み」を選んでおり（`docs/handoff/ST29.md`）、Q8 の context も「範囲は ST29 で絞る予定」と書いていた。Q8 の選択（書いた日は鳴らさない）はこの事実で変わらない。spec の理由の文と design D9 / Risks を「ST29 で絞ったとき収集側に残す口を最小にするため」に直し、`docs/handoff/ST29.md` に `/feelings` と `/feelings/reminder` を足した

## R6. 「端末の再起動後も前景サービスが立つ」は事実と違う。再起動の後は、アプリを開くまで通知が出ない
- 成果物: openspec/changes/st17-daily-feeling/design.md（D10 の予約の契機）
- 根拠: design.md:165（「予約の契機: `LocationService` の起動時（端末の再起動後も前景サービスが立つときに一緒に立つ）」）/
  `collector-android/app/src/main/AndroidManifest.xml` に `<receiver>` も `RECEIVE_BOOT_COMPLETED` も無い（`grep -n "BOOT\|receiver"` が 0 件）。前景サービスを起こすのは `MainActivity` だけで、
  `AlarmManager` の予約は再起動で消える。spec の Scenario `通知の時刻の後に動き出した端末はその日のうちに通知する`（spec.md:599-602）の「動き出した」が再起動を含むなら、いまの形では成り立たない
- kind: daily
- 提案: 再起動の後に予約し直す経路（`BOOT_COMPLETED` の受け手。権限が 1 つ増える）を持つか、持たない（アプリを開くまで通知が出ない）かを決め、spec の「動き出した」を観測できる言葉（「アプリを起動した」/「端末を再起動した」）に直す。
  権限を増やすかは C8 と同じ型の判断なので、design の D で決めて PR 本文に挙げる
- 処置: fixed D10 仮 —— **kind を premise から daily（B）に直す**（レビューは premise）。理由: 「再起動後も前景サービスが立つ」は私が design に書いた誤りで、本人の決定（Q6: 携帯の収集アプリが通知する）の根拠ではない（Q6 の context に再起動の話は無い）。直し方は権限を 1 つ足すかの B の選択なので、扉を開けたままにする側（再起動の後も鳴る）を仮で採った: `BOOT_COMPLETED` の受け手と `RECEIVE_BOOT_COMPLETED`（実行時の確認の要らない権限）を足し、spec の「動き出した」を「再起動」に直した（Scenario `通知の時刻の後に再起動した端末はその日のうちに通知する`）。反転条件（アプリを開くまで鳴らない側へ）を D10 に書いた。PR 本文の仮決めに挙げる。tasks 11.2 / 11.3

## R7. 通知の時刻の幅に値が無い。design の 10 分の窓と Doze の遅れを、spec のどの Scenario も落とせない
- 成果物: openspec/changes/st17-daily-feeling/specs/subjective-log/spec.md（「携帯の収集アプリが 1 日 1 回 …」）
- 根拠: spec.md:566（「通知の時刻を過ぎてから**数分の幅**のうちに」）/ spec.md:604-607（Scenario `通知の時刻はサーバの設定に従う` は「22:30 を過ぎてから出て、21:00 には出ない」だけで上限が無い）/
  design.md:161（`setWindow(RTC_WAKEUP, at, 10 分, …)`。「数分」と合わない）/ design.md:167（`setAndAllowWhileIdle` でない予約は Doze の間は保守の窓まで遅れる）。
  あわせて、端末が新しい時刻を知るのは古い時刻に起きて問い合わせたときだけなので、時刻を早めた日（21:00 → 20:00）は 21:00 に鳴る（spec.md:534-537 の `次の判断から効く` はサーバ側だけを撃つ）
- kind: technical
- 提案: 幅を値で書く（例: 「通知の時刻から 15 分以内」）か、「数分」を消して下限（時刻より前には出ない）だけを約束する形に直す。値を書くなら Task 10.1 の `decide` と 10.3 の計測テストで撃つ。
  時刻を早めた日の振る舞いを 1 行足す
- 処置: fixed D10 仮 —— 窓を値にした（15 分。定数 `REMINDER_WINDOW`）。spec は「時刻より前に出さず、OS に 15 分の窓で起こすよう求める」と書き、深い眠りでの遅れは約束しない。時刻を早めた日は毎日 12:00 に読み直す形にして Scenario `午前に早めた通知の時刻はその日から効く` を置いた。Scenario `通知の時刻より前には通知しない` / `通知は 15 分の窓で予約される`。tasks 11.1 / 11.2

## R8. 主観の行は消せないのに、取り込み口が `event_time` の元（滞在の始まり）と `tz_offset_min` の範囲を確かめない
- 成果物: openspec/changes/st17-daily-feeling/specs/subjective-log/spec.md（「形の合わない主観の記録は受け付けない」の表）/ design.md（D1・D5）
- 根拠: design.md:97-98（滞在の `start` / `end` がいまの滞在と一致することは求めない。`start <= end` と `event_time = start` だけ）—— 任意の `start`（例: 2020 年）を送ると、
  その記録は 2020 年に数えられ、「使い始める前」（spec.md:381）が全期間で動く。行は錠で消せず（spec.md:251）、主観を消す画面も無い（design.md:31）/
  design.md:48（`tz_offset_min` / `tz_id` は 540 / `Asia/Tokyo`）は spec の表（spec.md:95-102）に無い。
  ST19 は同じ穴を `lib.rs:748-757`（review/code.md R3「範囲外の値は受理された顔をして格納され、読み出しから永久に消える」「主張の行は DB が削除を拒む」）で塞いでいる
- kind: technical
- 提案: 表の `invalid_feeling_target` に「滞在の `start` / `end` が、その滞在のいまの範囲と重ならない」（一致は求めない。作り直しの揺れは許す）を、
  `malformed_feeling` に「地域のずれが -1439〜1439 分でない」を足し、それぞれ Scenario を 1 本ずつ置く
- 処置: fixed D5 —— 表の `invalid_feeling_target` に「区間がその滞在のいまの区間と重ならない」（一致は求めない）、`malformed_feeling` に「記録の地域が Asia/Tokyo でない」を足し、Scenario `滞在の区間と重ならない始まりと終わりは受け付けない` / `作り直しで少し動いた滞在の区間でも受け付ける` / `記録の地域が東京でない主観は受け付けない` を置いた。tasks 3.1 / 3.2

## R9. 観測できる振る舞いが design にだけある
- 成果物: openspec/changes/st17-daily-feeling/design.md（D5・D6・D8・D14）
- 根拠:
  - D14（design.md:203）「記入ありの日は『日』の最新（**後があれば後、無ければ前**）」—— S-3 の暦の要件（spec.md:771）は「その日の『日』の最新」とだけ書き、前と後が両方ある日にどちらを出すかを決めていない（S-2 の頭の要件 browsing-views spec.md:6 は書いている）
  - D5（design.md:96）ひとことの上限の数え方「Unicode のスカラー値の数」—— spec.md:101 / 140-146 は「2,000 文字」だけ。画面の `maxLength` は UTF-16 の単位で数えるので、絵文字を含むと画面と取り込み口の境界が食い違う
  - D5（design.md:93）検査の順 —— 2 つ以上に当たる記録でどの種別が返るかは観測できる（ST19 は `lib.rs:714-725` のコメントで順を理由つきで持つ）
  - D8（design.md:139）「`stays` は記録のある滞在だけ」—— spec.md:444 の「その日の各滞在ごとに」と読み違えうる
- kind: technical
- 提案: 上の 4 つを spec の要件の本文と Scenario へ移す（前後が両方ある日の暦の値・上限の数え方の単位・2 つ当たるときの種別・`stays` の範囲）。design には理由だけを残す
- 処置: fixed D5 —— 4 つとも spec へ移した: 暦の値は「後があれば後、無ければ前」（Scenario `前と後がある日の暦は後の値を出す`）/ ひとことの上限はコードポイント（Scenario `ひとことの上限はコードポイントで数える`）/ 検査の順は「表の上から」（Scenario `2 つに当たる記録は表の上の種別で断られる`）/ `stays` は記録のある滞在だけ（読み出しの要件の本文）。design には理由だけを残した

## R10. 「気分 滞在ごとに N 件」の N が何の数か決まっていない。消した滞在にだけ書いた日は、S-2 が「記入あり」なのに S-3 に何も出ない
- 成果物: openspec/changes/st17-daily-feeling/specs/browsing-views/spec.md / specs/subjective-log/spec.md
- 根拠: browsing-views spec.md:7（「気分 滞在ごとに N 件」）/ spec.md:32-35（Scenario「滞在にだけ記録が 2 件ある日」—— 1 つの滞在に 2 回書いた日と、2 つの滞在に 1 回ずつ書いた日が区別されない。書き直しを数えるかも不明）/
  design.md:112（消した滞在・たどり着かない吸収の記録も「記録の数（日の状態）には数える」）と subjective-log spec.md:448（滞在ごとの欄には返さない）を合わせると、
  消した滞在にだけ書いた日は S-2 に「気分 滞在ごとに N 件」・暦に「滞」が出るが、S-3 の滞在の行には何も出ない
- kind: daily
- 提案: N を「記録のある滞在の数（書き直しは 1 と数える）」のように決めて Scenario の WHEN を 2 通りに分ける。
  消した滞在にだけ書いた日の出し方（数えるが出さない / 数えない）を 1 本の Scenario で決める（Q10 / Q13 の読み方の範囲で、仮でよい）
- 処置: fixed D6 仮 —— N は「記録のある滞在の数（書き直しは 1、消した滞在も数える）」と spec に書き、Scenario を `滞在にだけ書いた日は未記入と出ない`（2 滞在 × 1 件）と `同じ滞在の書き直しは 1 と数える` に分けた。消した滞在にだけ書いた日は「記入あり」に数え、読み出しは件数（表示しない滞在の記録の数）を返し、S-3 は「消した滞在の気分 N 件（ここには出しません）」を出す（Scenario `消した滞在の気分は数だけが返る` / `消した滞在の気分は数だけ出る`）。反転条件を D6 に書いた

## R11. Q11 の読み方「履歴は対象の日（**滞在**）ごとに開ける」が、滞在の行に写っていない。滞在の行の「最新」は前/後を分けない
- 成果物: openspec/changes/st17-daily-feeling/specs/subjective-log/spec.md（「その日の滞在ごとの気分を書く」）
- 根拠: deep.md:165（Q11 の読み方「履歴は S-3 で対象の日（滞在）ごとに開ける」「前/後が違う記録は別に数える」）/
  spec.md:638（「書き直した履歴」は「その日の『日』の記録」だけ）/ spec.md:736（滞在の行は「その滞在の最新の記録」—— 読み出し（spec.md:444）は前/後ごとの最新を返すが、行がどちらを出すか書いていない）
- kind: daily
- 提案: 滞在の行にも「書き直した履歴」を置くか、置かないことを Q11 の読み方の訂正として deep.md に書く。行に出す最新は前/後のどちらか（日の頭と同じ「後があれば後」など）を 1 行と Scenario で決める
- 処置: fixed D13 仮 —— 滞在の行にも「書き直した履歴 N 件」を置き（Scenario `滞在の書き直した履歴を開ける`）、行に出す最新を「後があれば後、無ければ前」と spec に書いた（日の頭・暦・S-2 の行と同じ規則）。tasks 9.1

## R12. 要件の本文だけが言い、Scenario が言っていないこと
- 成果物: openspec/changes/st17-daily-feeling/specs/subjective-log/spec.md / specs/browsing-views/spec.md
- 根拠: どれも対応する Scenario が無い（下の各行）
  - spec.md:574「通知の仕組みが位置の収集と送信を止めない」（Task 10.2 は既存の単体が緑のままを見るが、Scenario の印が無い）
  - spec.md:569「得られないときは最後に得た時刻を使う」
  - spec.md:383 / 449「本文を消去した記録を数えない / 返さない」（Scenario は削除の印だけ。消去は錠が通すので、`raw = ''` の行は作れる）
  - spec.md:106「作り直しで吸収された滞在を紐づけ先にする記録を断らない」（Scenario は消した滞在だけ）
  - spec.md:217「前/後を記録の本文に持たせない」
  - spec.md:738「滞在の読み出しに失敗したときはそれと区別できる形で出す」/ spec.md:801「通知の時刻を保存できなかったとき入れた値を消さずに出す」/ spec.md:635「送れたとき … 対象の日を出す」
  - spec.md:772 と browsing-views spec.md:9「『これから』の日に未記入を出さない」（暦にも S-2 の頭にも Scenario が無い）
  - 本人が範囲で消した滞在（`deleted_by = 'rebuild:erased-range'`。`stay_store.rs:22`）の気分 —— Scenario `消した滞在の気分は滞在ごとの欄に出ない` は 1 件を消す形だけ。
    design.md:110-111 は `rebuild:` で始まる印を吸収として扱い、範囲で消した滞在を「本人が消した」に数えていない（たどり着かないので結果は隠れるが、根拠の分類が違う）
  - 暦に無い日付のアドレス（`#/feel/2026-13-45`）で S-3 を開いたときの振る舞い（ST25 の「解釈できないアドレスは今日の 1 日の画面」と、`Root.tsx:25-38` の 1 日の画面の扱いのどちらに倣うか）
- kind: technical
- 提案: 上の各行に Scenario を 1 本ずつ足すか、要件の本文から外す。範囲で消した滞在は design D6 の分類を `stay_store::Mark` の 4 種に合わせて書き直す
- 処置: fixed D6 —— 各行に Scenario を置いた: `通知の仕組みは位置の送信を止めない` / `届かないときは最後に得た通知の時刻を使う` / `本文を消去した記録しか無い日は未記入` / `本文を消去した主観は読み出しに出ない` / `吸収された滞在を紐づけ先にできる` / `前後は格納した記録に持たない` / `滞在の読み出しの失敗は滞在が無いと出ない` / `通知の時刻を保存できなかったとき入れた値が残る` / `保存したら対象の日が出る` / `これからの日は暦で未記入と出ない` / `これからの日の 1 日の画面に未記入と出ない` / `範囲を消して隠れた滞在の気分は滞在ごとの欄に出ない` / `暦に無い日付のアドレスは書く操作を出さない`（暦に無い日付は書く操作を出さず今日への入口を出す。SHALL も足した）。D6 の滞在の分類を `stay_store::Mark` の 4 種の表に書き直した

## R13. 1 つの Scenario に 2 つ以上の主張を束ねたもの
- 成果物: openspec/changes/st17-daily-feeling/specs/subjective-log/spec.md
- 根拠: `日の記録の出来事の時刻はその日の 0 時`（spec.md:41-44。出来事の時刻 と 記入の日時が別に残る）/ `その日の滞在が並び、押すとその滞在に保存される`（743-746。並びの順 と 保存の中身）/
  `画面で通知の時刻を変えられる`（805-808。画面の表示 と API の応答）/ `快不快のボタンは 64 px 以上で 1 列に並ぶ`（828-831。大きさ・上端・横スクロール）/
  `主観を何日も書かなくても途絶にならない`（370-373。ソースに含まれない と 途絶でない）。片方だけ通っても緑になる
- kind: technical
- 提案: 主張ごとに Scenario を分ける。分けないなら、Task の試験が THEN の各句を別の assert で撃つことを tasks に書く
- 処置: fixed 2.1 —— 5 本とも分けた: `日の記録の出来事の時刻はその日の 0 時` と `記入の日時は出来事の時刻と別に残る`（2.1）/ `その日の滞在が始まりの順に並ぶ` と `滞在の行を押すとその滞在に保存される`（9.1）/ `画面で通知の時刻を変えられる` と `画面で変えた通知の時刻が通知の判断に効く`（9.1）/ `快不快のボタンは高さ 64 px 以上`・`快不快のボタンは 1 列に並ぶ`・`幅 360 px で横のスクロールが出ない`（12.1。あわせて 64 px を高さだけにした —— proto の 64 px は高さで、幅 360 px を 5 つで分けると 64 px の幅は入らない）/ 途絶の Scenario は、ST17 で撃てる「稼働状況に含まれない」1 本に絞った（R4）

## R14. tasks が自分の Global Constraints（画面の文字の Scenario は jsdom と e2e の両方）を守っていない
- 成果物: openspec/changes/st17-daily-feeling/tasks.md
- 根拠: tasks.md:17（「画面の文字を言う Scenario は jsdom と e2e の両方に印を置く」）/ AGENTS.md（「画面の Scenario は `web/e2e`（本物のブラウザ）で担保する」）/
  jsdom だけに置かれた画面の文字の Scenario: 7.1 の `快不快のボタンは数値と言葉を持つ`・`もう一方の前後の最新は 1 行で添えられる`・`書き直した履歴を開くと全部が出る`・`断られたとき理由の文が出る`・`届かなかったとき選んだ値とひとことが残る`（tasks.md:119-123）/
  8.1 の `暦に記入ありの日の値が出る`・`使い始める前の日は未記入と出ない`・`滞在の無い日は滞在が無いと出る`・`吸収された滞在の気分は吸収先の行に出る`（134-138）/
  9.1 の `滞在にだけ書いた日は未記入と出ない`・`使い始める前の日の 1 日の画面に未記入と出ない`・`気分の読み出しの失敗は未記入と出ない`（147-150）
- kind: technical
- 提案: 上の Scenario を 7.2 / 8.2 / 9.2 の e2e の Scenario の列に足すか、Global Constraints を `docs/testing.md` §4 の線引き（指定と勘定は jsdom、実寸・経路・往復は e2e）に合わせて書き直し、どの Scenario がどちらかを明記する
- 処置: fixed 8.2 —— Global Constraints を「画面の Scenario は全部に e2e の印。jsdom は追加」に書き直し、8.2 / 9.2 / 10.2 の e2e が同じ Task の jsdom の項目の Scenario の全部に印を置くことにした（AGENTS.md の「画面の Scenario は web/e2e で担保する」に合わせた）

## R15. e2e の Task が後の Task の偽データに依り、消せない主観の行が共有の DB に積み上がる
- 成果物: openspec/changes/st17-daily-feeling/tasks.md（Task 8.2 / 9.2 / 7.2 / 11.1）
- 根拠: tasks.md:139（8.2 は「偽データ（Task 11）の日で」）—— Task 11 は後にある（依存順が逆）/
  `tools/seed.sh:10`（`normal` の位置は 2026-09-07 の 1 日だけ）—— 「30 日のうち 9 日が未記入」「吸収された滞在」（tasks.md:175）には、日を広げた位置と作り直しの手順が要るが書かれていない /
  主観の行は錠で消せない（spec.md:251）のに、7.2 / 8.2 / 9.2 は今日・昨日・明日・偽データの日に書き込む。手元は `reuseExistingServer`（`web/playwright.config.ts`）で DB を作り直さないので、
  2 回目の実行では `ひとことを足すと記録がもう 1 件積まれる`（「その日の記録は 2 件になる」）や「未記入の日」が前の実行の行で崩れる。画面は既定の利用者だけを使うので、利用者で分けることもできない
- kind: technical
- 提案: 偽データの Task（11.1）を Task 7 の前へ移す。e2e は件数を差分で数える・書き込む日を実行ごとに別の日にする・「未記入の日」は偽データが固定で持つ、のどれかを tasks に書く。
  `seed.sh` を 2 回流しても主観が増えない形（原文を固定する）にすることも書く
- 処置: fixed 6.1 —— 偽データを Task 6 へ移した（画面の Task より前）。原文を固定して 2 回流しても増えないことを seed 自身が数える。e2e は実行ごとに乱数で選んだ 2030 年以降の日を `page.clock` の今日にして書き、件数は差分で数え、見え方の assert は偽データの固定の日にだけ行う（Global Constraints）。通知の時刻は最後に 21:00 へ戻す（9.2）

## R16. 「`ashiato.webUrl` が無いビルドを gradle の検査で止める」は、CI・計測テスト・確認バッチのビルドを全部落とす。design の移行の計画とも食い違う
- 成果物: openspec/changes/st17-daily-feeling/tasks.md:164 / design.md:256
- 根拠: `.github/workflows/ci.yml:185` / `:188`（`./gradlew :app:assembleDebug` / `:app:testDebugUnitTest` に `-P` が無い）/ `tools/android-emulator.sh:61`（`-Pashiato.baseUrl` だけ）/
  `tools/verify-prep.sh:38`（`~/.gradle/gradle.properties` の値で `assembleDebug`）。どれも `ashiato.webUrl` を渡さないので、検査を入れた時点で Task 10.2 の `GT` と 10.3 の `AT` 自身が落ちる /
  design.md:256 は「`ashiato.webUrl` が無いビルドは通知を出すが、押しても画面を開かない（ビルドの検査で止める）」と、出す と 止める を同じ文で言っている
- kind: technical
- 提案: 既存の `ashiato.baseUrl` と同じ扱い（空なら空のまま組み立て、`http://` で loopback 以外なら落とす）にするか、止めるのを release だけにする。
  無いときの振る舞い（通知は出し、押すと何も開かない / 通知を出さない）を 1 つに決めて Scenario にする
- 処置: fixed D10 —— gradle で止めない。`ashiato.webUrl` が無いビルドは通知を出し、`Intent` を付けない（押しても何も開かない）に 1 つに決め、Scenario `画面のアドレスを持たないビルドでも通知は出る` を置いた。Migration Plan 4 も同じ文に直した。tasks 11.2

## R17. 件数の無い検証が 2 つ残っている
- 成果物: openspec/changes/st17-daily-feeling/tasks.md（Global Constraints の `AT` / 11.3）
- 根拠: tasks.md:25（`AT` は `tools/android-emulator.sh` の rc=0 だけ。計測テストの全部を走らせるので、`FeelingReminderInstrumentedTest` が 0 本でも、`@NeedsPristinePermissions` の側に入って片方の段で飛ばされても通る）/
  tasks.md:182（「`--list` にこの change の試験だけが出る」に終了条件が無い）
- kind: technical
- 提案: `AT` に `grep -Eq 'tests="[1-9]' collector-android/app/build/outputs/androidTest-results/connected/**/TEST-*FeelingReminderInstrumentedTest*.xml` を足す。
  11.3 は `cargo test -p ashiato-server st17_ -- --list 2>/dev/null | grep ': test$' | grep -v '^st17_'` が空、のように rc で書く
- 処置: fixed 12.2 —— `AT <クラス>` に結果の XML の件数の検査（`tests="[1-9]`）を足し、11.3 は `AT FeelingReminderInstrumentedTest` にした。12.2 の `--list` は `! … | grep -vq st17_` の rc で書いた

## R18. 要件の前倒しと後続への申し送りが、INDEX にも handoff にも無い
- 成果物: docs/stories/INDEX.md（2026-10-07 の訂正）/ proposal.md（Impact の「後続へ」）
- 根拠: proposal.md:69（「PERM-4（ST24）/ FR-50（ST22）/ FR-51（ST23）/ NFR-17〜23（ST25）の条項だけを満たす」）—— INDEX の ST17 の訂正（298-304 行）は capability だけで、
  ST19 の前例（INDEX.md:158-166 の表）のような要件の前倒しの表が無い。R4 の FR-35 の絞り込みも同じ /
  proposal.md:79-80 の後続への 4 件（ST18: 種類を足すときは表の行とコードの検査を同じ change で / ST24: 感度の分岐 / ST26: ひとことは NFC / ST29: 収集側の口）は proposal にしか無く、
  `docs/handoff/` に ST18・ST24・ST26 のファイルは無い。`docs/handoff/README.md` の規則 5・6 は、宛先の上流が読む経路を handoff に置いている
- kind: defer — 他 Story の担当（ST18 / ST24 / ST26 / ST29 / ST14）
- 提案: INDEX の ST17 の訂正に、ST19 と同じ形の「要件 / 本体の Story / ST17 が満たす条項」の表を足す。
  `docs/handoff/ST18.md`・`ST24.md`・`ST26.md` を作り、`ST29.md`（R5）と `ST14.md`（R4）に追記する
- 処置: fixed proposal.md —— INDEX の ST17 の訂正に、ST19 と同じ形の「要件 / 本体の Story / ST17 が満たす条項」の表（PERM-4 / FR-50 / FR-51 / FR-35 / NFR-17〜23）を足した。`docs/handoff/ST18.md`・`ST24.md`・`ST26.md` を作り（`st17-daily-feeling R18`）、`ST29.md`（R5）と `ST14.md`（R4）に追記した。proposal の「後続へ」に handoff の置き場を書いた

