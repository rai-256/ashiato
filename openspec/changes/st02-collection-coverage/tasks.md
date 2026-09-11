## 1. 表の作り直し（他より先。ここが動くと全部が動く）

- [x] 1.1 `migrations/0005_coverage_rebuild.sql` を書く。`core.coverage` を
      `(user_id, logical_source, day, event_count)` に作り直し、`state` 列を落とす（design D2）。
      戻し手順を `0005_coverage_rebuild.down.sql` に置き、**列の削除を含むので `不可逆` と明記する**。
      検証: `./tools/check-migrations.sh` が rc=0（明記が無いと落ちる）
- [x] 1.2 同じ版で `core.heartbeat` を作る（design D3）。`raw` は **`text`**、
      `received_at` は `timestamptz`、`(logical_source, content_hash)` に一意索引。
      検証: `docker compose up -d --wait db && cargo test -p ashiato-server` が rc=0
- [x] 1.3 同じ版で `core.coverage_span` を作る（design D8）。`kind IN ('stopped','dropped')`、
      `ended_at` は `NULL` 可、`started_at < ended_at` の CHECK を入れる。
      検証: 逆順の範囲を INSERT すると失敗することを `#[test] span_rejects_reversed_range` で確認し、
      `cargo test -p ashiato-server span_rejects_reversed_range` が rc=0
- [x] 1.4 `core.source` に `user_id uuid` と `collection_started_on date` を足す（design D5）。
      **主キーは `logical_source` のまま変えない**（`core.event` の FK が壊れる）。
      既存行には `core.event` の `min(event_time)` を `Asia/Tokyo` の日で当てる（第 6 回 Q24）。
      **記録が 1 件も無いソースは `NULL` のままにする**（第 5 回 Q22。
      登録簿にあるだけで計測を始めない）。検証: `./tools/smoke.sh` が rc=0
- [x] 1.4b **収集開始日を受け口の側で埋める** —— 記録または生存信号を受けたとき、
      そのソースの `collection_started_on` を、受けたものの出来事時刻の日との
      **`least()` で更新する**（`NULL` なら埋める）。**受信時刻ではなく、記録が作られた時刻**（第 6 回 Q24）。
      **後から古い記録が届いたら開始日は遡る**（第 7 回 Q26。圏外の保持がこれを起こす）。
      **遡ったら状態と達成の判定も引き直せる**こと（行に焼いていないので自然に満たす）。
      検証: `cargo test -p ashiato-server sets_started_on_first_arrival started_on_moves_back` が rc=0
- [x] 1.4c **1 件も届いていないソースが「導入前」のままで「途絶」にならない**ことを確認する
      （第 5 回 Q22。ウィンドウとブラウザ履歴は C-02 がまだ無い）。
      検証: `cargo test -p ashiato-server never_started_is_not_outage` が rc=0
- [x] 1.5 **稼働記録系の全表に `user_id` があることを検査で固定する**（FR-29 / 扉 #9）。
      `information_schema.columns` を引いて `core.coverage` / `core.heartbeat` /
      `core.coverage_span` / `core.source` の 4 表すべてに列があることを確認するテストを書く。
      検証: `cargo test -p ashiato-server user_id_on_all_coverage_tables` が rc=0
- [x] 1.6 **写真・ウィンドウ・ブラウザ履歴の想定間隔を登録簿に入れる**（FR-35 の改訂 = 深掘り 第 4 回 Q12。
      写真 6 時間 / ウィンドウ 6 時間 / ブラウザ履歴 24 時間）。
      **これが無いと途絶の判定も NFR-13 の利用主語 3 ソースも成立しない**（`review/spec.md` の R4）。
      検証: `./tools/seed.sh` の後に `cargo test -p ashiato-server expected_gap_seeded` が rc=0

## 2. 日境界を `Asia/Tokyo` へ（既存の欠陥 1 を直す）

- [x] 2.1 `crates/server/src/lib.rs` の `($2 AT TIME ZONE 'UTC')::date` を
      `'Asia/Tokyo'` に替える。定数 `DAY_TZ` を 1 か所に置く（design D1）。
      検証: `cargo build -p ashiato-server` と `cargo clippy -p ashiato-server -- -D warnings` が rc=0
- [x] 2.2 **境界のテストを固定する** —— `2026-03-01T14:59:59Z` の記録が `2026-03-01` に、
      `2026-03-01T15:00:01Z` の記録が `2026-03-02` の稼働記録に入ることを結合テストで確認する。
      検証: `cargo test -p ashiato-server day_boundary_jst` が rc=0
- [x] 2.3 **記録のタイムゾーンが日境界に効かないことをテストで固定する** ——
      `tz_id = 'America/New_York'` で `2026-03-01T15:00:01Z` を送り、`2026-03-02` に入ることを確認する
      （これが無いと後から「記録のタイムゾーンで切るほうが自然」と戻される。NFR-13 の分母が壊れる）。
      検証: `cargo test -p ashiato-server day_boundary_ignores_record_tz` が rc=0

## 3. 生存信号の受け口

- [x] 3.1 `POST /heartbeat` を足す（design D9）。`/ingest` と同じ認証、**複数件をまとめて受け、
      1 件ごとの結果を返す**。1 件だけの裸の要求も受ける。
      検証: `cargo test -p ashiato-server heartbeat_batch` が rc=0
- [x] 3.2 登録簿に無い論理ソースの生存信号を拒否する。**一部が不正でも正しい分は受け付ける。**
      検証: `cargo test -p ashiato-server heartbeat_partial_reject` が rc=0
- [x] 3.3 **1 件も受け付けなかったときだけ 400** を返し、**拒否の応答に受け取った値を含めない**
      （ST01 の 2.5 と同じ向き）。
      検証: `cargo test -p ashiato-server heartbeat_400_only_when_none heartbeat_no_echo` が rc=0
- [x] 3.3b `attempts`（前回の信号からの取得の試行回数）と `successes` を受け取って保存する
      （第 5 回 Q17）。**想定間隔より細かい空きは、この比でしか残らない。**
      **回数を持たない信号と、成功が試行を超える信号は受け付けない**（2 巡目 R7）。
      検証: `cargo test -p ashiato-server heartbeat_attempt_counts heartbeat_rejects_bad_counts` が rc=0
- [x] 3.4 `capturable` と `blockers` を受け取って保存する。
      **取得できない状態で `blockers` が空なら受け付けない** —— 理由の無い「取れない」は
      状態③の証拠にならない（FR-78）。
      検証: `cargo test -p ashiato-server heartbeat_rejects_blockerless` が rc=0
- [x] 3.5 同じ `content_hash` の生存信号を 2 回送ると行が 1 つのままであることを確認する
      （Q13。ST01 の Outbox は部分失敗の後で再送するので、重複の到着は常態）。
      検証: `cargo test -p ashiato-server heartbeat_idempotent` が rc=0
- [x] 3.6 **`raw` が素通しであることをテストで固定する** —— 重複キーとキー順を含む原文を送り、
      保存された原文がバイト単位で一致することを確認する（0003 と同じ理由。`jsonb` にすると壊れる）。
      検証: `cargo test -p ashiato-server heartbeat_raw_passthrough` が rc=0
- [x] 3.7 `received_at` が日に丸められず `timestamptz` のまま残ることを確認する（Q14）。
      検証: `cargo test -p ashiato-server heartbeat_received_at_is_timestamptz` が rc=0

## 4. 生存信号の保護（DB 側で強制する）

- [x] 4.1 `migrations/0006_immutable_heartbeat.sql` で
      `core.reject_heartbeat_rewrite()` を書き、`BEFORE UPDATE ON core.heartbeat` に置く。
      **全列の更新を拒む**（design D4。論理削除の例外を作らない）。
      検証: `./tools/check-migrations.sh` が rc=0
- [x] 4.2 `./tools/check-immutable.sh` に生存信号の項を足す。**わざと `UPDATE` を投げて
      拒否されることを確認する**（0004 が 3 手の迂回を実測で見つけている。
      アプリ層のテストだけでは `psql` を直に叩く経路が素通りする）。
      検証: `./tools/check-immutable.sh` が rc=0
- [x] 4.3 **迂回路が無いことを確かめる** —— `core.heartbeat` に分類列を持たせていないこと、
      および `raw` / `content_hash` / `received_at` / `capturable` / `blockers` の
      どれを更新しようとしても拒否されることを、列ごとに確認する。
      検証: `./tools/check-immutable.sh` が rc=0（列ごとの `UPDATE` を全部投げる）

## 5. 7 状態の導出

- [x] 5.1 `GET /coverage?from=&to=` を足す。決定順序は design D7 のとおり
      （導入前 → 破棄 → 停止 → 記録あり → 生存信号 → 途絶）。**順序は specs の
      Requirement 本文にも列挙してある**ので、そちらと食い違わせない。
      検証: `cargo test -p ashiato-server coverage_states` が rc=0（7 状態それぞれが出る入力）
- [x] 5.1b **停止・破棄が記録より優先されることを確認する**（第 5 回 Q19）——
      1 日を丸ごと覆う停止がある日に記録が 1 件以上あっても④を返し、
      破棄と停止が重なれば⑤を返す。
      検証: `cargo test -p ashiato-server span_outranks_records` が rc=0
- [x] 5.2 **同じ日に複数の条件が重なる入力**（停止 + 記録 / 破棄 + 停止 / 生存信号 + 記録）で、
      返る状態が 1 つに決まり、2 回評価しても同じであることを確認する。
      検証: `cargo test -p ashiato-server coverage_state_is_deterministic` が rc=0
- [x] 5.3 途絶を**行に焼かず導出する**（design D6）。**想定間隔を条件に持たせる** ——
      想定間隔 60 日のソースの 1 日の空白は途絶にならない（`review/spec.md` の R4）。
      **前後を想定間隔以内に挟まれた空白の日は②**、前後に何も無ければ⑥
      （2 巡目 R6。証拠の無い②を立てないため）。
      検証: `cargo test -p ashiato-server outage_respects_expected_gap sandwiched_gap_is_alive` が rc=0
- [x] 5.4 **`expected_gap_sec` を変えると過去の判定も変わる**ことを確認する
      （バッチで書いていたら変わらない。導出であることの検査になる）。
      検証: `cargo test -p ashiato-server outage_reevaluates_on_gap_change` が rc=0
- [x] 5.5 収集開始日より前が⑦になり、停止中の日は⑥にならないことを確認する。
      検証: `cargo test -p ashiato-server coverage_before_start coverage_stopped_not_outage` が rc=0
- [x] 5.6 **丸ごと覆わない停止は状態を決めない**ことを確認する ——
      半日だけ止めた日に記録があれば①になる（design D7 / Q3 と同じ粒度）。
      検証: `cargo test -p ashiato-server partial_stop_does_not_decide_state` が rc=0

## 6. 達成日数と合否

- [x] 6.1 `GET /coverage/achievement` を足す（**期間は取らない**）。主語の割り当てを
      サーバ側の定数に置く（design D11。DB の列にしない）。
      **判定は「分母 × 95 % 以上」**（第 5 回 Q18。絶対値の 350 ではない）。
      達成日数と分母の両方を返す。
      **窓はソースごとに「収集開始日から 365 日」**（第 6 回 Q23）。
      **`?from=&to=` は取らない** —— 窓が仕様で決まったので、呼び出し側に委ねると合否が動く。
      **合否は 5 本すべての窓が閉じた日にのみ確定**し、それまでは暫定として返す（第 7 回 Q27）。
      5 本すべてが開始していれば**確定する日と残り日数**を返し、
      1 本でも開始していなければ**どちらも返さない**。
      検証: `cargo test -p ashiato-server achievement_endpoint` が rc=0
- [x] 6.2 端末が主語の 2 ソースが「記録が 1 件以上ある日」で数えられることを確認する。
      検証: `cargo test -p ashiato-server achievement_device_subject` が rc=0
- [x] 6.3 利用が主語の 3 ソースが「**取得できる状態の**生存信号があった日」で数えられ、
      `capturable = false` しか無い日が達成に**入らない**ことを確認する
      （第 4 回 Q7。ここが抜けると権限が剥がれたまま 1 年で 365/365 になる）。
      検証: `cargo test -p ashiato-server achievement_usage_subject achievement_excludes_uncapturable` が rc=0
- [x] 6.4 **1 日を丸ごと覆う停止だけが分母から抜ける**ことを確認する ——
      半日の停止の日は分母に残る（Q3）。
      検証: `cargo test -p ashiato-server achievement_denominator_full_day_stop_only` が rc=0
- [x] 6.4b **分母から抜けた日は達成日にも数えない**ことを確認する ——
      丸ごと停止の日に記録があっても、分母にも分子にも入らない。
      **割合判定にしたことで新たに要るようになった**（達成日数 > 分母 を防ぐ。2 巡目 R4）。
      検証: `cargo test -p ashiato-server achievement_numerator_never_exceeds` が rc=0
- [x] 6.5 合否が「**5 本すべてが分母の 95 % 以上**」で返り、落ちたソースが分かることを
      **固定値**で確認する（分母 365 で 360, 355, 352, 351, 340 → 線は 346.75 なので
      未達 + 340 のソース名）。
      検証: `cargo test -p ashiato-server achievement_verdict_all_five` が rc=0
- [x] 6.5b **分母が短いソースは線も下がる**ことを固定値で確認する
      （分母 200 日・達成 195 日 → 線は 190 日なので達成）。
      **ここが絶対値の 350 では落ちていた** —— 導入 1 年未満のソースが原理的に到達不能だった。
      検証: `cargo test -p ashiato-server achievement_threshold_scales` が rc=0
- [x] 6.6 導入前の日が分母に入らないことを確認する（FR-79）。
      検証: `cargo test -p ashiato-server achievement_excludes_before_start` が rc=0
- [x] 6.7 **窓がソースごとに別の日に始まる**ことを確認する（第 6 回 Q23）——
      位置の開始日が `2026-04-01`、ブラウザ履歴が `2026-07-01` なら、窓もそれぞれから 365 日。
      検証: `cargo test -p ashiato-server achievement_window_per_source` が rc=0
- [x] 6.8 **365 日が経つ前でも途中経過が出る**ことを確認する（第 6 回 Q23）——
      開始日から 200 日なら分母 200 日で同じ式。**その合否は暫定**（第 7 回 Q27）。
      検証: `cargo test -p ashiato-server achievement_partial_window` が rc=0
- [x] 6.9 **確定と暫定を区別する**ことを確認する（第 7 回 Q27）——
      4 本の窓が閉じて 1 本が 300 日目なら暫定 + 残り 65 日、
      1 本が未開始なら暫定 + 確定日なし、5 本すべて閉じたら確定。
      **これが無いと収集開始 10 日目のソースが 100 % で「達成」に見える**（3 巡目 R3）。
      検証: `cargo test -p ashiato-server achievement_provisional_until_all_windows_close` が rc=0

## 7. 端末からの生存信号（C-01）

- [x] 7.1 `collector-android` に生存信号の送出を足す。間隔は登録簿の想定間隔
      （位置・写真とも 6 時間）に合わせる。
      **送出方式は ST01 の R46（Doze の除外を要求しない）に従い、ここで決め直さない**。
      検証: `./gradlew :app:assembleDebug` が rc=0
- [x] 7.2 権限・センサ・接続の状態を読み、`capturable` と `blockers` に載せる。
      **権限が無い状態で `capturable = false` と `blockers` が埋まる**ことを確認する。
      検証: `./gradlew :app:testDebugUnitTest --tests '*Heartbeat*'` が rc=0
- [x] 7.2b **前回の信号からの取得の試行回数と成功回数を数えて載せる**（第 5 回 Q17）。
      信号を送るたびに数えを戻す。**これが ST01 の R46（Doze）が渡した宿題の答え** ——
      信号が来ている＝生きていた / 取得率が低い＝眠っていた / 信号が来ない＝死んでいた。
      検証: `./gradlew :app:testDebugUnitTest --tests '*HeartbeatCounters*'` が rc=0
- [x] 7.3 生存信号を**記録と同じ未送信の仕組みに乗せる**（ST01 の Outbox）。
      送信失敗後に再送されること、停止と再開をまたいで残ること、
      **再送が同じ冪等キーを持つ**ことを確認する。
      検証: `./gradlew :app:testDebugUnitTest --tests '*HeartbeatOutbox*'` が rc=0
- [x] 7.4 **記録が 1 件も生成されない期間でも生存信号が出る**ことを確認する
      （これが FR-78 の主目的。記録の生成に相乗りさせると意味が消える）。
      検証: `./gradlew :app:testDebugUnitTest --tests '*HeartbeatWithoutRecords*'` が rc=0
- [x] 7.5 `collector-android/README.md` に、生存信号が Doze の維持時間帯に乗ること
      （実測の最長空き 14.2 分 << 想定間隔 6 時間）を書く。
      検証: `./gradlew :app:assembleDebug` が rc=0（文書のみなので影響が無いことの確認）

## 8. 稼働状況の画面（S-1）

- [x] 8.0 **web にテストの走らせ方を用意する**（現状 `package.json` に `test` が無い）。
      `vitest` を入れ、`npm run test` を足す。
      検証: `cd web && npm run test` が rc=0（テスト 0 件でも走ること）
- [x] 8.1 `ui-direction.md` の確定値（色相 132° / 彩度 30% / 明るさ 12 / 3 段）を
      `web/src` のトークンとして 1 か所に置く（design D12）。**S-1 のためだけの色を足さない**。
      検証: `cd web && npx tsc -b && npm run lint` がいずれも rc=0
- [x] 8.2 1 年を週に畳んだ格子を出す。**ソースごとに格子を分け、ソース名の文字を添える**（Q15）。
      **格子は縦長**（1 行 = 1 週、上から下へ 53 週、**新しい週が上**。第 6 回 Q25）。
      **開いた直後は各ソースの直近 4〜5 週だけ**を出し、1 年は伸ばして見る（第 7 回 Q28）。
      検証: `cd web && npx tsc -b && npm run lint && npm run build` がいずれも rc=0
- [x] 8.3 **格子のセルは 3 段だけ**で描く（design D10。第 5 回 Q21。要件は NFR-23）——
      「記録あり」「動いていた・記録なし」「それ以外」。
      **隣り合う 2 段の相対輝度比が 3:1 以上**（WCAG SC 1.4.11）であることを検査する。
      **7 段にしない** —— 隣接 3:1 を 6 区間積むと 3^6 = 729:1 が要り、sRGB の最大 21:1 では
      原理的に成り立たない。検証: 3 段の相対輝度を計算し全ペアが 3:1 以上であることを
      `cd web && npm run test -- state-contrast` で確認し rc=0
- [x] 8.4 **週を選ぶと、その 7 日ぶんが 7 状態の名前で文字で出る**（第 5 回 Q20 / Q21）。
      **格子で「それ以外」に畳まれた日も、ここでどの状態だったかが分かる。**
      セルにイベントハンドラを付けない（Q16）。
      検証: `cd web && npm run test -- week-select` が rc=0
- [x] 8.5 **幅 360 px で操作対象が 24 × 24 CSS px を割らず、格子が横スクロールなしで
      収まることを検査する**（NFR-19）。**週の帯の高さが 24 px 以上**であること、
      セルが操作対象になっていないことも同じ検査で確認する。
      検証: `cd web && npm run test -- target-size` が rc=0
- [x] 8.5b **新しい週から古い週へ上から下に並び、直近の週が一番上にある**ことを確認する
      （第 6 回 Q25）。**「53 週が並んでいる」ことは開いた直後には求めない**（8.5c を参照）。
      検証: `cd web && npm run test -- newest-week-first` が rc=0
- [x] 8.5c **開いた直後に 5 ソースすべての直近 4 週が同時に見える**ことを確認する（第 7 回 Q28）——
      5 × 5 行 × 24 px ≒ 600 px。**これが無いと完了の判定が 1 本目のソースにしか成立しない**
      （3 巡目 R4）。伸ばすと 1 年ぶん（53 週）が読めることも確認する。
      検証: `cd web && npm run test -- initial-viewport expand-to-year` が rc=0
- [x] 8.6 5 ソースの達成日数と分母、5 本すべてが分母の 95 % 以上かの合否を数値で出し、
      **それが確定か暫定かを示す**（NFR-13 / 第 4 回 Q9 / 第 5 回 Q18 / 第 7 回 Q27）。
      **暫定のときは確定までの残り日数**を出す。確定する日が定まらないとき
      （まだ収集を開始していないソースがある）は、その理由を出す。
      検証: `cd web && npm run test -- achievement-panel` が rc=0
- [x] 8.7 各格子に添えたソース名と、週を選んだときに出る状態名の文字が **4.5:1 以上**であることを
      確認する（NFR-18）。**セルには掛からない**（本文でも文字画像でもないため）。
      検証: `cd web && npm run test -- text-contrast` が rc=0

## 9. 契約文書

- [x] 9.1 `docs/openapi.json` に `POST /heartbeat` と `GET /coverage` と
      `GET /coverage/achievement` を足す。
      検証: `./tools/check-openapi.sh` が rc=0
- [x] 9.2 `docs/collector-contract.md` に生存信号の契約を書く ——
      間隔・`capturable` と `blockers` の値域・冪等キーの作り方・再送の扱い。
      検証: `./tools/check-openapi.sh` が rc=0（契約と openapi のずれを見る）

## 10. 通し

- [x] 10.1 `./tools/smoke.sh` に生存信号の縦串を足す ——
      **記録を 1 件も入れずに生存信号だけを送り、`GET /coverage` がその日を②で返す**ことを確認する
      （これが FR-78 の目的そのもの。記録経由でしか確かめないと穴が残る）。
      検証: `./tools/smoke.sh` が rc=0
- [x] 10.2 検証: `cargo fmt --all --check` /
      `cargo clippy --workspace --all-targets -- -D warnings` / `cargo test --workspace` /
      `./tools/check-boundaries.sh` / `./tools/check-licenses.sh` がいずれも rc=0
- [x] 10.3 全 Scenario に test の印があることを確認する。
      検証: `python3 scripts/check_scenarios.py .` が rc=0
      （「人間の確認待ち」の節に挙げた Scenario だけが除外される）

## 人間の確認待ち

**実機・実目でしか判定できない Scenario。** ここに挙げたものだけが `check_scenarios.py` の
除外対象になる（見出しが `人間の確認待ち` であることに意味がある。箇条書きでは機械に届かない）。

- Scenario: 1 年ぶんが一目で読める

> **「グレースケールでも格子の 3 段が区別できる」はここから外した。** この Scenario の逐語は
> 「3 段それぞれのセルの**相対輝度を取り**、隣り合う 2 段の比が 3:1 以上」で、
> **計算で閉じる**（`web/src/__tests__/state-contrast.test.ts` が描くのと同じ値から計算する）。
> 目で見て区別できるかは `docs/stories/ST02.md` の完了の判定が持つ。

`docs/stories/ST02.md` の完了の判定「1 か月放置した後に開くと、欠けた日が一目で分かる」は、
**1 か月放置した実データが要る**ので、ST02 の PR では判定できない。
`docs/stories/ST02.md` に判定の期日を残す。

## 第 5 回の深掘りで確定したこと（**未決は無い**）

独立レビュー（`review/spec.md`）が、AI が独断で決めていた一方通行の判断を 5 件見つけた。
2026-09-11 に全問回答を得て、上のタスクに反映済み。**下流は勝手に変えないこと。**

| # | 本人の答え | 反映先 |
|---|---|---|
| Q17 | 生存信号に**取得の試行回数と成功回数**を載せる | 3.3b / 7.2b |
| Q18 | 達成の判定は**分母の 95 % 以上**（絶対値の 350 ではない） | 6.1 / 6.5 / 6.5b / 8.6 |
| Q19 | 丸ごと覆う**停止・破棄を記録より優先する** | 5.1 / 5.1b |
| Q20 | 格子の**行は週**（1 行 = 7 日） | 8.4 |
| Q21 | 格子は **3 段**に畳み、7 状態は**週を選んだときの文字**で読む | 8.3 / 8.4 / 8.5 |
| Q22 | 収集開始日は**最初の記録か生存信号が届いた日**。届くまでは開始していない | 1.4 / 1.4b / 1.4c |

## 第 6 回の深掘りで確定したこと（**未決は無い**）

2 巡目の独立レビュー（`review/spec-r2.md`）が見つけた 3 件。2026-09-11 に回答を得て反映済み。
**下流は勝手に変えないこと。**

| # | 本人の答え | 反映先 |
|---|---|---|
| Q23 | 窓はソースごとに**収集開始日から 365 日**。`?from=&to=` は落とす | 6.1 / 6.7 / 6.8 |
| Q24 | 収集開始日は最初の記録が**作られた**日（届いた日ではない） | 1.4 / 1.4b |
| Q25 | 格子は**縦長**（1 行 = 1 週、新しい週が上） | 8.2 / 8.5 / 8.5b |

## 第 7 回の深掘りで確定したこと（**未決は無い**）

3 巡目の独立レビュー（`review/spec-r3.md`）が出した 3 件。2026-09-11 に回答を得て反映済み。
**下流は勝手に変えないこと。**

| # | 本人の答え | 反映先 |
|---|---|---|
| Q26 | 収集開始日は**遡って動く**（常にいちばん古い記録の日） | 1.4b |
| Q27 | 合否は **5 本すべての窓が閉じた日に確定**。それまでは暫定 | 6.1 / 6.8 / 6.9 / 8.6 |
| Q28 | 開いた直後は**各ソースの直近 4〜5 週**。1 年は伸ばして見る | 8.2 / 8.5c |
