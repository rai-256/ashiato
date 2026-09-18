# 独立レビュー（spec） — st06-app-usage

対象: `openspec/changes/st06-app-usage/` 一式（`deep.md` / `deep-questions.json` / `proposal.md` /
`specs/device-collection/spec.md` / `design.md` / `tasks.md` / `review/deep.md`）、
`docs/stories/ST06.md`・`INDEX.md`・`docs/requirements.md`、正典 `openspec/specs/`、
`docs/collector-contract.md`、`collector-android/`、`crates/server/`、`docs/handoff/ST11.md` / `ST14.md`。
**書いた者の意図は知らない。成果物どうしの整合だけを見た。**

## 機械の検査（2026-09-18 実行。人間の目より先に）

```
$ openspec validate st06-app-usage --strict
Change 'st06-app-usage' is valid

$ python3 scripts/check_chain.py .
  要件 117 件 / Story 36 本 / 扉 26 項
  [ok]   どれかの Story に拾われた要件: 115/117 件
  [ok]   INDEX の「Story の対象外」: NFR-11, NFR-8
chain: OK (0 件 / 未回収 0 件 / warn 0 件)

$ python3 scripts/check_scenarios.py . st06-app-usage
  Scenario 464 件 / 印 514 個 / 担保あり 433 / 人間の確認待ち 0
  [FAIL] 担保の無い Scenario: 31 件（すべて st06-app-usage の delta）
scenarios: FAIL (担保なし 31 件 / 名無しの確認待ち 0 件)

$ python3 scripts/review_triage.py . st06-app-usage
  指摘 12 件 / 仮決め 0 件 / 要件へ戻すもの 0 件
triage: OK
```

`check_scenarios` の FAIL は**上流では原理的に出る**（`scripts/merge_gate.sh:104-106`「上流では tasks と
check_scenarios が原理的に満たせない」）。31 件は ADDED 30 + MODIFIED の新設 1 で、
既存の 2 本（`到達できる間は 1 時間以内に届く` / `溜まった分を…`）は
`tools/smoke.sh:259`・`IntervalTest.kt:40`・`RetentionInstrumentedTest.kt:125` に印がある。
**tasks.md の「delta は 33 本」という自己申告は実測と一致する**（`grep -c '^#### Scenario:'` = 33）。
spec の 33 本のうち tasks の `Scenario:` に挙がっていないのは上の既存 2 本だけで、
**tasks に書かれた Scenario 名は spec の `#### Scenario:` と一字一句一致している**（機械照合）。

---

## R1. Q4（取りこぼしを記録 1 件で残す）の決定を受ける要件が無い。PC 側には FR-82 という前例がある

- 成果物: `docs/requirements.md` / `openspec/changes/st06-app-usage/specs/device-collection/spec.md:100-113` /
  `docs/stories/ST06.md`
- 根拠:
  - spec は `### Requirement: 取りに行って取得元に無かった期間を記録として残す` の導出元を
    **「FR-2。扉 #14。深掘り Q4」**と書くが、FR-2（`docs/requirements.md:81-88`）にも
    FR-84（同 `:89-98`）にも「取れなかった期間を記録として残す」は 1 文字も無い。
  - **同じ扉・同じ機構が PC 側では要件になっている** ——
    `docs/requirements.md:177-185` の FR-82（「WHEN C-02 が起動する … 前回の停止から今回の起動までの期間を
    『PC が止まっていた』として記録する」★ 2026-09-12 追加。ST07 の深掘り Q1 の決定）。
    ST06 は `design.md:81` で「C-02 の `powered-off` に前例がある形を選んだ」と自ら書いている。
  - 扉 #14 の関与要件（`docs/requirements.md:979`）は `FR-33, FR-34, FR-9, FR-54, FR-35, FR-61, FR-80, FR-81, FR-82`。
    **FR-2 / FR-84 は入っていない。** ST07 は同じときに `★ 2026-09-12 補足（関与要件に FR-81 / FR-82 を追加）`
    をこの行に足している（`docs/requirements.md:975`）。
  - `docs/stories/ST06.md` の front matter は `doors: []`、本文も
    「（satisfies する要件は、扉リストのどの項の関与要件にも現れない）」。
    **人間はブリーフで扉 #14 を見ないまま Q4 に答えた**ことになる。
  - 結果、`check_chain.py` は緑のまま（FR が無いので鎖が張れない）。Story を `make_story.py` で
    再生成しても「完了の判定」に取りこぼしの項は出ず、実際 ST06.md の 4 項に gap は無い。
- kind: technical
- 処置: fixed deep.md —— **kind を conflict から technical に直した**。要件の側に条項が欠けていた「抜け」で、
  選択肢のあいだの矛盾ではない（本人の答えは Q4 で確定している）。FR-82 と同じ手順で機械的に閉じた ——
  `docs/requirements.md` に **FR-85 を ★ 付きで新設**し、扉 #14 の関与要件に FR-2 / FR-84 / FR-85 を足し、
  `stories.json` の ST06 に FR-85 と完了の判定 1 行を加えて再生成した（`doors: [14]` が自動で付いた）。
  spec の gap の Requirement の導出元も FR-85 へ直した。deep.md の「要件へ戻したもの」の表に FR-85 を足してある
- 提案: deep の「効く先」に**要件へ戻すもの**を 1 行足す —— FR-85（または FR-2 の第 2 文）として
  「WHEN 取得の窓の始まりが取得元の保持の下限より前にある THE SYSTEM SHALL その期間を記録として残す」を
  ★ 付きで新設し、扉 #14 の関与要件に足す。`stories.json` の ST06 に `doors: [14]` と完了の判定 1 行を
  加えて再生成する（FR-82 と同じ手順）。

## R2. gap の記録の「出来事の時刻」が spec に無く、受け手はその日を①「記録あり」に塗る（いまは達成日も 1 日増える）

- 成果物: `openspec/changes/st06-app-usage/specs/device-collection/spec.md:100-140`
- 根拠:
  - spec は gap の記録に「期間の**始まりと終わり**と、取れなかった理由」を含めよと書くが、
    **`event_time` に何を入れるかを 1 文字も書いていない**。`/ingest` は `event_time` を必須にし
    （`docs/collector-contract.md:27`）、冪等キーも `logical_source + event_time + raw` から作る（同 `:138`）。
  - 受け手は `event_time` の `Asia/Tokyo` の日で稼働記録を積む
    （`crates/server/src/lib.rs:783-786`: `VALUES ($1, $2, ($3 AT TIME ZONE 'Asia/Tokyo')::date, $4)`）。
    そして `crates/server/src/coverage.rs:960-962` は `if facts.event_count > 0 { return DayState::Recorded }`。
    **「取れなかった」を主張する 1 件が、その日を①「記録あり」に塗る。**
  - さらに `crates/server/src/coverage.rs:28` は `DEVICE_SUBJECT = ["c01-location", "c01-app-usage"]` のままで、
    達成日は `Subject::Device => d.event_count > 0`（同 `:1122`）で数える。
    **Q2 の訂正（NFR-13 の主語移動）は要件にだけ入り、正典と定数は ST14 へ申し送られている**ので、
    ST14 が着地するまでの間、gap の記録は NFR-13 の達成日を水増しする。
  - `docs/handoff/ST14.md` の R5 は「格子にどう出すか未定」とは書いているが、
    **「何も決めないと①に塗られる・達成日が増える」という既定の挙動には触れていない**。
- kind: technical
- 処置: fixed 4.1 —— **kind を conflict から technical に直した**（spec に欄が欠けていた抜けで、要件どうしの矛盾ではない）。
  提案どおり spec に「**出来事の時刻を、その期間の終わり**とする」の 1 文と
  Scenario `gap の記録の出来事の時刻は期間の終わりである` を足し、始まりに置かない理由（収集開始日より前へ落ちうる）を
  design D4 に書いた。`docs/handoff/ST14.md` の R5 に「畳むまでの現状」として、①に塗られることと
  達成日が 1 日増えることを明記した
- 提案: spec に 1 文と 1 Scenario を足す ——「gap の記録の出来事の時刻を、**その期間の終わり**（＝保持の下限）とする」
  と「#### Scenario: gap の記録の出来事の時刻は期間の終わりである」。そのうえで handoff ST14 R5 に
  「受け手が畳むまでは、その日が①と数えられ、NFR-13 の達成日にも入る」を**現状として**明記する
  （申し送り先が「何が壊れているか」を知らずに着手するのを防ぐ）。

## R3. 保持の下限を `now - 10 日` の決め打ちにすると、spec の「取得元が保持している範囲」とずれ、取れたイベントを飛ばす

- 成果物: `openspec/changes/st06-app-usage/tasks.md`（2.2）/ `specs/device-collection/spec.md:61,107`
- 根拠:
  - spec は 2 か所とも**実際の下限**を言っている ——「遡る先を、取得元が**その時点で保持している範囲**までとする」（:61）、
    「窓の始まりを、生成した後に**保持の下限**まで進める」（:107）。
  - tasks 2.2 は実装を「保持の下限を `now - 10 日`（イベント）と粒度ごとの値 … で持ち、
    **1 か所（`UsageRetention`）にまとめる**」とし、検証は
    `grep -c 'UsageStatsDatabase' …/UsageRetention.kt` が 1 以上。
    **つまり「定数を置いてコメントを書いた」ことしか確かめない。**
  - この 10 日は AOSP の実装から読んだ値で、**API からは読めない**（`design.md:14-21` の表の出所は
    `UsageStatsDatabase.prune()`）。OEM 改変・OS 版差・prune の起動タイミングでずれうる。
  - ずれの向きが悪い側に出たとき、spec の「窓の始まりを保持の下限まで進める」が
    **まだ取得元に残っているイベントを飛ばす**（しかも飛ばした期間には gap の記録が積まれ、
    「取れなかった」という嘘が正典の形式で残る）。飛ばしたイベントは 10 日で消える。
- kind: technical
- 処置: fixed 2.2 —— **kind を premise から technical に直し、loss を外した。理由: 直し方が失われるものを消すため。**
  「下限で窓を切り詰める」のをやめた（spec から「窓の始まりを保持の下限まで進める」を削り、
  「**窓の始まりを切り詰めない**」を入れた）ので、見込みが外れても**取れるものは全部取る** ——
  取り損ねる経路そのものが無くなり、選ぶ余地も消えたので人間に返す問いにならない。
  gap の範囲は `[窓の始まり, min(見込みの下限, 返った最古のイベントの時刻))` で閉じ、
  長さが 0 なら積まない（提案の「取得元の応答から決める」を、下限の推定ではなく範囲の側で実現した）。
  Scenario `見込みより古いイベントが返ったときは gap が積まれない` を新設、tasks 2.2 の検証もそれに合わせた
- 提案: 下限を**取得元の応答から決める**（窓の始まりのまま問い合わせ、返った最古のイベント時刻／
  空なら粒度の箱の境界で下限を推定する）。定数は「見込み」としてしか使わない。
  spec 側にも「保持の下限は取得元の応答から決める」を 1 文入れて、実装の当て推量を禁じる。

## R4. 集計ソースの数値（取得契機 24 時間 / 生存信号 6 時間）が design にしかなく、しかも 2 つが噛み合っていない

- 成果物: `openspec/changes/st06-app-usage/design.md:63-71,90-91` / `specs/device-collection/spec.md:142-152,199-200`
- 根拠:
  - **spec に集計側の間隔が無い。** Requirement「収集を始めた時点の過去の利用の集計を取り込む」は
    「収集を始めた後も、**日ごとの粒度**について集計の取り込みを続ける」（:148）とだけ書き、
    Scenario も「日ごとの集計の**取得契機に達する**」（:186）で数を持たない。
    値は design D5（`design.md:91`「位置 60 秒 / アプリ利用 30 分 / **集計 24 時間**」）と
    D3（`design.md:70`「生存信号を **6 時間ごと**に出す」・`:66` `expected_gap_sec = 21600`）にしかない。
    `openspec archive` は main specs しか更新しないので、**この 2 つの数は正典から落ちる**。
  - 数どうしが噛み合っていない。spec:199-200 は「取得の試行と成功の数えを、ソースごとに別に持ち、
    **そのソースの取得契機の間隔を満点の刻みとする**」。集計は取得契機 24 時間・生存信号 6 時間なので、
    1 区間の試行は 6h ÷ 24h = **0.25（＝ 0 回）**。そこで 1 回でも取り込みが成功すると
    `successes(1) > attempts(0)` になり、契約は `invalid_counts` で断る
    （`docs/collector-contract.md:258`「`attempts` を超える（`invalid_counts`）」、
    同 `:272-274`「収集側は `attempts` を `successes` より小さくしない（そうしないと未送信に居座る）」）。
    **6 時間ごとに 1 件ずつ、恒久的に断られる生存信号が端末に積まれる**形になる。
- kind: technical
- 処置: fixed 4.3 —— 集計の取得契機を 24 時間から **6 時間**（生存信号の区間と同じ）へ変え、spec に上げた。`初回の後も日ごとの集計が 6 時間ごとに取り込まれる` と `集計の取得率は 6 時間を刻みとして数えられる` を新設し、「**生存信号の 1 区間に取得契機が 1 回以上入る**」「成功の数を試行の数より大きくしない」を不変条件として spec に書いた。design D3 / D5 と tasks 1.3 も合わせた
- 提案: 集計の取得契機と生存信号の刻みを spec に上げる（例:「#### Scenario: 集計の取得率は 24 時間を刻みとして数えられる」
  で、24 時間に 1 回・6 時間の信号でどう数えるかを固定する）。数え方は
  「契機が区間に入らないソースは試行 0 / 成功 0 とし、成功は次に契機を含む区間で数える」など、
  `successes <= attempts` を壊さない形を 1 つ選んで書く。

## R5. tasks 1.2 の検証コマンドは、作業前のいま既に 0 件で通る

- 成果物: `openspec/changes/st06-app-usage/tasks.md`（1.2）
- 根拠:
  - 検証は `grep -rn 'source=\" *+ *LOGICAL_SOURCE' collector-android/app/src/main` が 0 件。
  - 実測（2026-09-18）: このパターンは**いま既に 0 件**（rc=1）。実際のコードは連結ではなく
    `collector-android/app/src/main/kotlin/dev/ashiato/collector/Telemetry.kt:19`
    `append(" source=").append(LOGICAL_SOURCE)`。
  - つまり 1.2 は「何もしなくても検証が通る」タスク。ST01 の 6.2（「テストで確認する」だけで
    テストが 0 本）と同じ型で、`check_scenarios.py` の docstring がその前例を名指ししている。
- kind: technical
- 処置: fixed 1.2 —— 検証を `grep -c LOGICAL_SOURCE …/Telemetry.kt` が **0**（いまは 1）へ直し、実測値を本文に残した。1.1 の「`git diff --stat` に振る舞いの変更が無い」も `git diff --numstat -- LocationFix.kt` が 0 行へ直した
- 提案: 実体に当たるパターンに直す（例:
  `grep -rn 'LOGICAL_SOURCE' collector-android/app/src/main/kotlin/dev/ashiato/collector/Telemetry.kt` が 0 件）。
  あわせて 1.1 の「`git diff --stat` に `LocationFix.kt` の振る舞いの変更が無い」も終了条件が人間の判断なので、
  「`LocationFix.kt` の diff が 0 行」など機械が決められる形にする。

## R6. tasks 7.2 は、この repo が一度踏んだ「smoke では遅延を測れない」を踏み直している

- 成果物: `openspec/changes/st06-app-usage/tasks.md`（7.2）/ `specs/device-collection/spec.md:315-318`
- 根拠:
  - 7.2 は Scenario `アプリ利用は出来事の時刻から数えても 1 時間以内に届く` を
    `tools/smoke.sh` の psql（`ingest_time - event_time`）に置く。
  - `tools/smoke.sh:259-265` の同型の検査には、既にこう書いてある ——
    `collector-android/app/src/test/kotlin/dev/ashiato/collector/IntervalTest.kt:40-50`:
    「**遅延の本体はここ**（review R8）。smoke 手順 18 は**台本自身が `event_time` に「いま」を入れて即 POST**
    しているので、サーバ側の即時性しか測っていない。生成から格納までの上限を決めているのは、
    この送信間隔と再送の周期」。
    ST06 の THEN は「**イベントの時刻から** 1 時間以内」で、上限を決めるのは
    取得契機 30 分 + 送信 5 分＝35 分。smoke では**どんな実装でも緑になる**。
  - 併せて、**本人が決めた 30 分という数を固定する試験がどのタスクにも無い**。
    `IntervalTest.kt` は `FIX_INTERVAL_MS` / `SEND_INTERVAL_MS` を定数で止めているが、
    アプリ利用の 30 分に対応するものが tasks に現れない（1.3 の `HeartbeatCountersTest` は刻みを引数で受ける試験）。
- kind: technical
- 処置: fixed 7.2 —— 遅延の Scenario の印を `IntervalTest` 型の単体（`USAGE_INTERVAL_MS == 1_800_000` と `USAGE_INTERVAL_MS + SEND_INTERVAL_MS < 3_600_000`）へ移し、smoke は 7.2b として「取り込み口を通る／2 回送っても増えない」だけにした。`tools/smoke.sh` の `registered_at` を揃える対象に `c01-app-usage-rollup` を足すことも 7.2b に書いた
- 提案: この Scenario の印を `IntervalTest` 型の単体試験へ移す
  （`USAGE_INTERVAL_MS == 1_800_000` と `USAGE_INTERVAL_MS + SEND_INTERVAL_MS < 3_600_000` を assert）。
  smoke 側は「取り込み口を通る／2 回送っても増えない」だけを担う。
  ついでに `tools/smoke.sh:47-50` の `registered_at` を揃える対象に `c01-app-usage-rollup` を足す
  （足さないと集計の 1 件が「登録より前」になる）。

## R7. 多ソースでの破棄の範囲（tasks 6.2）に Scenario が 1 本も無い。正典は直った後も同じ嘘を許す

- 成果物: `openspec/changes/st06-app-usage/specs/device-collection/spec.md`（MODIFIED が無い）/ `tasks.md`（6.2）
- 根拠:
  - 正典 `openspec/specs/device-collection/spec.md:359-360` は
    「範囲の終わりを、**同じソースで**破棄せずに残った最も古い記録の出来事の時刻…」と書いている。
  - 実装は置き場全体の先頭を使う（`collector-android/app/src/main/kotlin/dev/ashiato/collector/Retention.kt:87`
    `records.oldest()?.item?.eventTime`）。deep.md の「確かめたが問わなかったこと」R11 のとおり。
  - しかし正典の Scenario「続けて捨てた範囲は残った記録の時刻で途切れない」（同 spec 内）は
    **ソースを 1 本しか登場させない**ので、直す前も直した後も同じく緑。
    tasks 6.2 は「2 ソースを混ぜた置き場で…試験が 1 本ある」と書くが、
    **対応する `#### Scenario:` が無いので `check_scenarios.py` には 1 行も見えない**（印を置く先が無い）。
  - この capability は `device-collection` で、走っているのは ST12（`collection-coverage` / `external-ingestion`）。
    **ST06 の側で閉じられる。** ST12 への差し戻しにはならない。
- kind: technical
- 処置: fixed 6.2 —— `## MODIFIED Requirements` に「端末から失われた記録を破棄として報告する」を全文写し、Scenario `破棄の範囲は別のソースの記録で閉じない` を 1 本新設した（本文は変えていない。既存の Scenario がソースを 1 本しか登場させないので緑のままだった、という経緯を補足に残した）。tasks 6.2 がその印を持つ
- 提案: `## MODIFIED Requirements` に「端末から失われた記録を破棄として報告する」を全文写して
  Scenario を 1 本足す ——「#### Scenario: 破棄の範囲は別のソースの記録で閉じない」
  （WHEN 位置とアプリ利用が同じ置き場にあり、位置だけを捨てる / THEN 報告の範囲の終わりは
  残った位置の記録の時刻であり、アプリ利用の記録の時刻ではない）。

## R8. tasks 7.1 に終了条件となるコマンドが無く、下流の 1 セッションでは原理的に終わらない

- 成果物: `openspec/changes/st06-app-usage/tasks.md`（7.1）
- 根拠:
  - 本文は「1 日あたりのイベントの件数とバイト数を実機（またはエミュレータ）で **24 時間ぶん**数え、
    90 日ぶんの見積りを出す」。検証は「数えた結果が `deep.md` に書かれていること /
    `python3 scripts/check_chain.py .` rc=0」。
  - `scripts/check_chain.py` は要件 → Story の鎖と `stories.json` からの再生成しか見ない
    （実測: 出力は「要件 117 件 / Story 36 本 / 扉 26 項」）。**`deep.md` に数が書かれたかは見ない。**
    前半は自己申告で、コマンドが無い。
  - 24 時間の実測は機械が再現できない「時間そのもの」に当たるが、
    tasks 0 は「**『人間の確認待ち』に逃がせる Scenario は 1 本も無い**」と宣言しており、
    `> 物理: time` の印も無い。どちらの扱いなのかが成果物から決まらない。
  - 併せて 7.3 の検証 `python3 scripts/check_scenarios.py .` は
    `docs/collector-contract.md` を 1 行も読まない（`TEST_DIRS` は `crates` / `collector-android/app/src/test` /
    `androidTest` / `tools` / `web/src` / `web/e2e` / `tests`）。契約に表を足したことの検査になっていない。
- kind: technical
- 処置: fixed 7.1 —— 24 時間の実測をやめ、`tools/usage-volume.sh`（1 件あたりのバイト数 × 実測の件数から 90 日ぶんを出し、1 GB を超えたら rc=1）に置き換えた。7.3 の検証も「契約の表から固定試験の期待値を読み、表を 1 行変えると試験が落ちる」形へ直した（`check_scenarios.py` は `docs/` を読まないため）
- 提案: 7.1 を「エミュレータで N 時間計測して外挿する台本」にしてコマンドと閾値で終える
  （`tools/…sh` が件数・バイト数を出力し、90 日換算が閾値を超えたら rc=1）。
  それが無理なら `## 人間の確認待ち` 節へ `> 物理: time` 付きで移す。
  7.3 の検証は「契約の表の欄名と 3.4 / 4.4 の固定試験の期待値を突き合わせる」スクリプト、
  あるいは固定試験の期待値を契約から読む形にする。

## R9. tasks 5.1 が書き直すと言う handoff ST11 の箇所がずれている。矛盾するのは「叩き台の文面」

- 成果物: `openspec/changes/st06-app-usage/tasks.md`（5.1）/ `docs/handoff/ST11.md`
- 根拠:
  - 5.1 は「`docs/handoff/ST11.md` の **4 点のうち 2 と 4** を書き直し、ST11 へ渡す形を更新する」。
  - handoff の 4 点は 1) `kind=permission_denied` が logcat に残る 2) 前景サービスの通知が出ない
    3) crash バッファにアプリが出ない 4) 権限は拒否のまま。
    ST06 の Q3 / Q7 で変わるのは **2 だけ**で、**4（権限は拒否のまま）は変わらない**。
  - 一方、同じ handoff の**「文面の叩き台」**は
    「#### Scenario: 位置の権限を拒否してもアプリは落ちず、収集を始めない / THEN アプリは終了し、**収集は始まらず**」。
    これは ST06 の spec:193-203（「収集の開始を、ソースの取得条件が満たされているかに依らず行う」）と
    **正面から矛盾する**。ST11 がこの叩き台を採ると、正典の中で 2 つの Requirement が食い違う。
- kind: technical
- 処置: fixed 5.1 —— **kind を conflict から technical に直した**（宛先のずれと、まだ誰も採っていない叩き台の文面の誤りで、
  要件どうしの矛盾ではない）。tasks 5.1 の対象を「点 2 と**文面の叩き台**」に直し、点 4 は変わらないと明記した。
  あわせて `docs/handoff/ST11.md` の叩き台を
  `位置の権限を拒否しても収集は始まり、拒否したことが残る` へ**この場で書き直した**（ST11 が古い文面を採れないように）
- 提案: 5.1 の対象を「点 2 と**文面の叩き台**」に直し、叩き台を
  「位置の権限を拒否しても収集は始まり、位置は取得できない状態として生存信号に載る」へ置き換える
  （ST06 の Scenario `位置の取得条件が欠けても位置の生存信号は届く` と同じ言い方に揃える）。

## R10. handoff ST14 の R6 は、ST14 の `satisfies` に無い要件（NFR-13）を負わせるのに、INDEX に理由が無い

- 成果物: `docs/handoff/ST14.md` / `docs/stories/INDEX.md`
- 根拠:
  - NFR-13 を持つのは ST02（`docs/stories/INDEX.md:24` の satisfies に `NFR-13`）。
    ST14 は `satisfies: [FR-35]`（`docs/stories/ST14.md:4`）。
  - handoff は「ST12 の archive 後。ST14 の上流で `collection-coverage` の `MODIFIED` に畳む」と書く。
  - 同じ形（satisfies に無い条項を別の Story が満たす）は前例があり、
    そのときは **INDEX に表で理由が残されている**（`docs/stories/INDEX.md:179-186`
    「要件の側でも、satisfies に無い条項を ST12 が満たす … | 要件 | 本体の Story | ST12 が満たす条項 |」）。
    ST06 のぶんはその表にも INDEX の注にも無いので、**handoff を読む者にしか見えない**。
- kind: technical
- 処置: fixed deep.md —— `docs/stories/INDEX.md` の「satisfies に無い条項を別の Story が満たす」の注に 1 行足した（NFR-13 の主語の仕分けは本体 ST02、反映は ST14。出所は st06-app-usage の deep Q2 と `docs/handoff/ST14.md`）。ST06 の行の satisfies と扉の列も再生成に合わせた
- 提案: `INDEX.md` の同じ注に 1 行足す（「NFR-13 の主語の仕分け（本体 ST02）を ST14 が正典と定数に反映する。
  出所: st06-app-usage の deep Q2 / `docs/handoff/ST14.md`」）。`stories.json` を触らずに済む位置に置く。

## R11. D7（遡って取った記録の地域）は記録の観測できる中身なのに、design にしかない

- 成果物: `openspec/changes/st06-app-usage/design.md:105-115` / `specs/device-collection/spec.md`
- 根拠:
  - D7 は「遡って取ったイベントの `tz_id` / `tz_offset_min` は、**取得時点の端末の地域**を付ける（仮）」。
    これは送る 1 件の中身（`docs/collector-contract.md:27-35` の必須欄）で、**外から観測できる**。
  - spec の delta には `tz` に触れる文が 1 つも無い（`grep tz` で 0 件）。
    `openspec archive` は main specs しか更新しないので、この決定は正典に残らない ——
    D12 / D13 / D14 の実測と同じ置き場の誤り。
  - 反転条件（ST16 以降で引き直す）は書かれており、失われるものは無い（凍結対象外）。
    **落ちるのは「決めたこと」であって、データではない。**
- kind: technical
- 処置: fixed 3.2 —— spec の「取得の窓は取りこぼさずに進む」に「遡って取ったイベントの記録に**取得時点の端末の地域**を付ける」の 1 文と、Scenario `遡って取った記録の地域は取得時点の端末の地域である` を足した。design D7 は（仮）と反転条件を残したまま
- 提案: Requirement「取得の窓は取りこぼさずに進む」に 1 文と 1 Scenario を足す ——
  「#### Scenario: 遡って取った記録の地域は取得時点の端末の地域である」
  （WHEN 3 日前のイベントを取得する / THEN その記録の地域は取得時点の端末の地域である）。
  design には（仮）と反転条件を残したままでよい。

---

## 観点ごとの結び

- **観点 1（deep の決定が正典に写っているか）**: Q1 / Q8 / Q3 / Q5 / Q6 / Q7 は spec の Requirement と
  Scenario に写っている（貼り戻しの逐語と `deep-questions.json` の選択肢を 1 件ずつ突き合わせた。
  未回答 0 / 推奨と違う側 0 / 覆したもの 0 という deep.md の記述も一致）。
  **Q6 の数（30 分）は Requirement の題と本文、および Scenario `取得率はソースごとの刻みで数えられる`
  （6 時間で 12 回）に入っている**が、実装側の定数を止める試験が tasks に無い（R6）。
  **Q2 は要件（NFR-13 ★）まで戻っているが、正典と実装は古いまま**で、その帰結が R2 として観測できる。
  **Q4 だけは要件に戻っていない**（R1）。
- **観点 2（Scenario が検証可能か）**: 「正しく」「適切に」型の曖昧な THEN は無い。
  バイト一致・件数・窓の位置・試行回数 12 など、測れる量で書かれている。
  **「人間の確認待ち」に逃げた Scenario は 0 本**（`check_scenarios.py` の実測も 0）。
  検証可能性が実際には成立しないものが 3 件 —— R2（`event_time` が未定義なので THEN の日が決まらない）、
  R3（「取得元が保持している範囲」が実装では決め打ちに化ける）、R6（smoke では測れない）。
  AND で 2 つ束ねた Scenario が 4 本あるが（`許可しなくても収集は始まる` など）、
  どちらの半分も同じ試験で観測できるので指摘に挙げない。
- **観点 3（置き場の誤り）**: R4（集計の間隔 24 時間 / 生存信号 6 時間）と R11（tz）が design 止まり。
  spec に自分の実装の名前（関数名・crate 名・列名）は入っていない（バッククォートの中身は
  `/drops` `gap` `powered-off` `collection-coverage` と AOSP の `UsageStatsDatabase.prune()` だけ）。
  proposal の Capabilities（`device-collection` の Modified のみ）は `specs/` のディレクトリとも
  `INDEX.md:69` の capability 表（`device-collection` に ST06 あり）とも一致する。
- **観点 4（tasks が検証を持つか）**: 大半はコマンドと rc を持つ。持たない／通ってしまうのが
  R5（1.2 と 1.1）・R8（7.1 と 7.3）。依存順は成立している（1.x で口を作ってから 3.x で使い、
  移行 4.2 は記録を積む 4.3 より前）。spec の 33 本のうち tasks に拾われていないのは既存の 2 本だけで、
  それは ST01 のテストに印がある（実測）。
- **観点 5（Story と要件の一致）**: `check_chain.py` は観点 8 まで OK。
  ST06.md の「価値」「完了の判定」4 項は deep の Q3 / Q5 / Q7 と矛盾しない。
  `satisfies` は FR-2 / FR-84 で、spec の導出元もその 2 つ（＋扉 #14 ← R1）。
  前倒しは無い。
- **観点 6（走っている Story への差し戻し）**: `specs/` は `device-collection` 1 本だけで、
  `collection-coverage` / `record-envelope` の delta は無い。
  `c01-app-usage-rollup` を登録簿に足しても `/coverage` と `/coverage/achievement` は
  `must_sources()` の 5 本しか返さない（`crates/server/src/lib.rs:1551,1578`、
  `crates/server/src/api_tests.rs:659-670,733-745` がリテラルで止めている）ので、
  ST12 の走行中の振る舞いは壊れない。`expected_gap_seeded`（`api_tests.rs:592-618`）も
  5 ソースのリテラル照合なので行が 1 本増えても落ちない。**この点の ST06 の主張は実測で裏が取れた。**
  ただし R2 は「ST06 の側で閉じる手がある」（gap の `event_time` を spec で決め、
  現状の帰結を handoff に書く）ので、差し戻しにせずに処理できる。R7 も `device-collection` 内で閉じる。
