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

## 5. 8 状態の導出

- [x] 5.1 `GET /coverage?from=&to=` を足す。決定順序は design D7 のとおり
      （導入前 → 破棄 → 停止 → 記録あり → 生存信号 → 途絶）。**順序は specs の
      Requirement 本文にも列挙してある**ので、そちらと食い違わせない。
      検証: `cargo test -p ashiato-server coverage_states` が rc=0（8 状態それぞれが出る入力）
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
- [x] 8.4 **週を選ぶと、その 7 日ぶんが 8 状態の名前で文字で出る**（第 5 回 Q20 / Q21）。
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

## 11. 独立レビューの処置（`review/code.md`）

**4 系統の独立レビュー**（`code-verify` と `pr-review-toolkit` の 3 agent）が 54 件を出した。
うち 2 件は人間の領分（`deep.md` の第 8 回）で、残りをここで処置した。
**指摘 1 件ごとの処置は `review/code.md` にある。**

- [x] 11.1 **サーバ側** —— 記録の格納・稼働記録の加算・収集開始日を 1 トランザクションに（R2）/
      `blockers` を読み出し口から返す（R39。spec の Scenario の後半が未実装だった）/
      区間ごとの取得率を返す（R9）/ 記録側の CTE を集約する（R21。利用者が複数だと分母が行数になる）/
      登録簿に無いソースを黙って落とさない（R20）/ 冪等索引に利用者識別子（R13）/
      生存信号の削除も拒む（R22）/ 0005 の作り直し条件を 0001 の形に絞る（R23）/
      401 と DB の失敗に操作名を残す（R26 / R27）/
      主語の割り当て・破棄の分母・混在する取得可否・範囲の端・想定間隔の境界・
      利用者の分離・重複だけの日・決定順序の表を検査で固定（R6 / R7 / R12 / R32 / R33 / R43 / R45 / R46 / R47 / R48 / R49 / R50 / R52 / R53）。
      検証: `cargo test --workspace` が rc=0（87 件）
- [x] 11.2 **画面** —— 日を `Asia/Tokyo` で切る（R10。UTC だと毎日 9 時間 今日が消えていた）/
      読み込み中と取得失敗をデータが無いことと分ける（R19）/
      選択を面の明るさで表さない（R34。いちばん暗い段との比が 1.422:1 に落ちていた）/
      `role="row"` をボタンから外す（R35）/ 畳み戻しで詳細も閉じる（R37）/
      セルに実際に塗られた色を見る検査（R3）/ 24 px と 53 週を定数の自己参照から外す（R4 / R5）/
      `App` の検査を起こす（R10 / R19）。
      検証: `cd web && npx tsc -b && npm run lint && npm run test && npm run build` がいずれも rc=0（38 件）
- [x] 11.3 **収集側** —— `ACCESS_NETWORK_STATE` を宣言する（R31。**実機でクラッシュループになる**）/
      端末を読む口を落とさない（R31）/ 起動時の信号も守る（R36）/
      刻みの例外を構造で受け止める（R30）/ 数えを端末の保存領域に置く（R16）/
      積めてから数えを戻す（R24）/ 読めなかった未送信を上書きで消さない（R17）/
      恒久的に断られた 1 件が先頭を塞がない（R18）/
      周期 emit・送り先・想定間隔・数えの永続を検査で固定（R40 / R41 / R42 / R51）/
      README の桁を直す（R38）。
      検証: `cd collector-android && ./gradlew :app:assembleDebug :app:testDebugUnitTest` が rc=0（102 件）
- [x] 11.4 **検査の側** —— ライセンス検査を `web` job へ移し、空振りを NG にする（R8。
      **CI の緑とローカルの緑が別物だった**）/ `smoke.sh` の前提を冒頭で作る（R14）/
      `HeartbeatResult` の欄名を突き合わせる（R25）/
      `check-immutable.sh` の「判定していない」節を直す（R22 / I10）。
      検証: `./tools/check-licenses.sh` / `./tools/smoke.sh` / `./tools/check-openapi.sh` /
      `./tools/check-immutable.sh` がいずれも rc=0

## 12. 第 8 回 Q29 —— 収集開始日を端末の時計のずれから守る

**本人の答え**: 受けるが、収集開始日の計算から外す。記録も信号も捨てない。
閾値は**登録簿に行ができた日（`core.source.registered_at`）より前**。
第 7 回 Q26（収集開始日は常にいちばん古い記録の日）は変えない。

> **なぜ「無視するだけ」では足りないか**: 収集開始日は `least()` でしか動かない
> （前にしか動かない）ので、**一度 1999 年に落ちると正しい日を送り直しても戻らない**。
> 外す条件を足すだけでは、既に汚れた行が残る。**引き直す移行が要る。**

- [x] 12.1 `migrations/0007_source_lifecycle.sql` で `collection_started_on` を**引き直す** ——
      `core.event` と `core.heartbeat` のうち `registered_at` の日以降のものだけから
      `min()` を取る。1 件も無いソースは `NULL` に戻す。
      検証: 1999 年の信号を入れてから移行を当て直し、`collection_started_on` が
      その信号の日でないこと（`cargo test --workspace` の `clock_skew_` 系が rc=0）
- [x] 12.2 `coverage::touch_started_on` に同じ閾値を足す（受け口の側で二度と汚さない）。
      検証: `cargo test --workspace` が rc=0
- [x] 12.3 `specs/collection-coverage/spec.md` の「ソースごとに収集を開始した日が残る」に
      **計算から外す条件**を足し、Scenario を 2 本足す（外れること / 記録自体は残ること）。
      検証: `openspec validate st02-collection-coverage --strict` が rc=0

## 13. 第 8 回 Q30 —— 「同時に見える」を 1 スクロール以内に緩める

**本人の答え**: 開いた直後に 2〜3 ソース、**ひとスクロールで 5 ソースすべて**。
第 7 回 Q28（開いた直後は直近 4〜5 週だけ）は変えない。NFR-19（週の帯 24 px）は割らない。

> **崩れたのは「スクロールせずに」だけ。** 根拠にした 600 px は
> `5 ソース × 5 行 × 24 px` でセルだけを積んだ勘定で、**見出し・余白・ボタン・
> 達成の表を数えていなかった**。実際に宣言されている箱を積むと約 1,491 px（実測）。

- [x] 13.1 `specs/collection-coverage/spec.md` の格子の Requirement と Scenario を
      「開いた直後に 2〜3 ソース、ひとスクロールで 5 ソースすべて」に書き直す。
      検証: `openspec validate st02-collection-coverage --strict` が rc=0
- [x] 13.2 **宣言された箱を DOM から積む検査**を書く（`web/src/__tests__/one-scroll.test.tsx`）。
      基準の画面は **360 × 640 CSS px**（NFR-19 が幅に使っている小さい端末）。
      ひとスクロール = **2 画面ぶん = 1,280 px**。
      **定数どうしを突き合わせない**（D27）—— 積むのは `element.style` に実際に入っている値で、
      突き合わせる相手は固定の予算。余白を増やせばこの検査が落ちる。
      検証: `cd web && npm run test` が rc=0
- [x] 13.3 勘定に収まるまで画面を詰める（`INITIAL_WEEKS` 5 → 4、節の余白）。
      **24 px を割らない**。検証: 同上（`target-size` も緑のまま）

## 14. 登録簿の 2 列（**ST02 が作る**。2026-09-11 の判断）

`retired_on` と `succeeds` は**読む側が稼働記録**なので ST02 が作る
（`openspec/changes/st03-idempotent-ingest/design.md` D7 の表）。
ST03 は `external_id_kind` だけを作る。

- [x] 14.1 `migrations/0007_source_lifecycle.sql` に `retired_on date` と
      `succeeds text REFERENCES core.source(logical_source)` を足す。
      **`retired_on` は真偽値にしない**（ST03 R56。真偽値だと退役より前の本物の途絶が遡って消える）。
      検証: `\d core.source` に 2 列が出て、移行を当て直しても壊れない（`cargo test --workspace`）
- [x] 14.2 `migrations/0007_source_lifecycle.down.sql`（**不可逆**と明記）。
      検証: `./tools/check-migrations.sh` が rc=0

## 15. ST03 から差し戻された 5 件 と 第 8 回 Q31

ST03 の深掘り（5 巡 26 問）が ST02 に返したもの。根拠は
`openspec/changes/st03-idempotent-ingest/review/deep-r5.md`（R55 / R56 / R57 / R63 / R64）。

- [x] 15.1 **状態を 7 → 8**（「退役」。FR-54）。評価の順は**「導入前」の直後** ——
      収集期間の外側という同じ型で、`retired_on` は `collection_started_on` の対。
      判定は `day > retired_on`（**退役した日そのものはまだ収集していた**）。
      検証: `cargo test --workspace` が rc=0（退役後が「退役」・退役日は元の状態）
- [x] 15.2 **退役した日以降を途絶の判定と NFR-13 の分母から外す**（FR-80 / R64）。
      検証: 同上（退役後の空白日が「途絶」にならず、分母も伸びない）
- [x] 15.3 **第 8 回 Q31 —— 収集開始日を引き継ぎの鎖から引く。**
      新しい名前の収集開始日は**鎖の根（いちばん古い引き継ぎ元）の収集開始日**。
      検証: 同上（`succeeds` で繋いだ 2 世代で、窓の起点が根の日になる）
- [x] 15.4 **Must の 5 本を鎖の先端に解決する**（Q31「古い名前を分母から外し、新しい名前が窓を引き継ぐ」）。
      定数が指す名前が退役していれば、その後継を数える。
      検証: 同上（退役した名前が `failing` にも `not_started` にも出ない）
- [x] 15.5 **「記録あり」と件数を `core.event` から引く**（R57）。
      いまは `core.coverage.event_count > 0`。ST03 が更新経路を開けると出来事の時刻が
      別の日へ動き、**記録の無い日が「記録あり」・記録のある日が「途絶」**になる（ST03 の実測）。
      検証: 同上（記録の時刻を動かすと状態も動く）
- [x] 15.6 **退役したソースの格子は末尾に置き、既定で畳む**（R63）——
      Must の 5 本が 1 画面から押し出されるため。
      検証: `cd web && npm run test` が rc=0（退役が末尾・既定で行を出さない）
- [x] 15.7 `specs/collection-coverage/spec.md` / `design.md` を 8 状態・引き継ぎ・
      「記録ありの出どころ」に合わせる。`docs/openapi.json` も引き直す。
      検証: `openspec validate --strict` / `./tools/check-openapi.sh` / `scripts/check_scenarios.py` が rc=0

## 16. 通し（第 8 回の後）

- [x] 16.1 検証コマンド一式を通す（`cargo fmt` / `clippy` / `test` / gradle / web / `tools/*.sh` /
      `scripts/check_scenarios.py` / `check_chain.py` / `review_triage.py` / `openspec validate --strict`）
- [x] 16.2 **独立レビューを掛け直す**（`code-verify` と `pr-review-toolkit`）。
      指摘 1 件ごとに `review/code.md` へ `処置:` を書く。検証: `scripts/review_triage.py . st02-collection-coverage` が rc=0

## 17. 第 2 巡の独立レビューの処置（`review/code-r2.md` ほか 3 系統）

第 8 回の答えを実装したあと、**4 系統の独立レビュー**が 36 件を出した。
**3 件は成功条件 1 が黙って 0 % に落ちる型**で、私が書いたコメントが
「そうならないように」と主張している当の落ち方だった。

- [x] 17.1 **引き継ぎの鎖ごと数える**（R1 / R2 / R3）—— 先端の名前だけで数えていたので、
      分母は鎖の根から数えるのに分子は切り替え後しか拾わなかった。
      `resolve_chain` を Rust 側に置き、**退役しているあいだだけ後継へ進む**（生きている
      ソースから乗り換えない）。`GET /coverage` も同じ解決を通す（格子と達成が別の 5 本を
      見ていた）。検証: `cargo test --workspace` が rc=0
- [x] 17.2 **正典に従って「退役した日以降」**（I1）—— 実装は `day > retired_on` だったが、
      `docs/requirements.md` の FR-54 / FR-80 はどちらも「以降」。
      **理由が筋の通ったものでも、実装が要件の逐語を独断で変えてよいことにはならない。**
      検証: 同上（退役日そのものが⑧・前日は本来の状態）
- [x] 17.3 **記録を引く索引と期間の絞り**（R9 / I4）—— 出どころを `core.coverage`（1 日 1 行）から
      `core.event`（記録ごとに 1 行）へ移したのに、索引も期間の絞りも足していなかった
      （`EXPLAIN` が `Seq Scan`。位置は年 50 万行超）。検証: 同上
- [x] 17.4 **移行が正規の収集開始日を後ろへ動かさない**（C-3 / G4）—— `migrate()` は
      起動のたびに当て直すので、記録を破棄した後の再起動で⑤が⑦に化けた。
      **汚れているときだけ引き直す**条件に絞る。検証: `repair_is_idempotent_and_never_moves_a_clean_date_forward`
- [x] 17.5 **輪と枝分かれ**（R12 / H-4）—— `succeeds` に一意部分索引（作れなくても起動は止めない）、
      たどる側は通った名前を覚えて輪を検出し warn。検証: `a_cycle_in_the_chain_terminates`
- [x] 17.6 **黙って空振りする経路に印を残す**（H-1 / H-2）—— 閾値で弾いたこと、
      退役しているのに後継が無いこと、鎖が深すぎることを `tracing::warn!` に。検証: `cargo clippy` が rc=0
- [x] 17.7 **検査が守っていなかったもの**（G1 / G2 / G3 / R6 / R7）——
      論理削除された記録が「記録あり」のまま（`core.event_live` に替えても緑だった）/
      登録日ちょうど（閾値が `>` に倒れると通常の導入手順が永久に未開始）/
      ⑧の評価位置（破棄・停止の下へ動かしても緑だった）/
      移行の閾値（消しても緑だった）。検証: `cargo test --workspace` が rc=0
- [x] 17.8 **画面の検査**（R4 / M-1 / M-2 / G5 / G7）—— 並べ替えを `App` から見る
      （`retiredLast` を外しても全緑だった）/ `retired_on` が `undefined` のとき全格子が
      畳まれる / 勘定が読めない値を黙って 0 にする / 退役が増えた形で予算を測る。
      検証: `cd web && npm run test` が rc=0
- [x] 17.9 **7 → 8 の掃き替え**（R10 / I7）と `smoke.sh` の `WHERE` 無し `UPDATE`（H-6 / I6）、
      足場が 2 表を同値で書いていた件（H-7）。検証: `check_scenarios.py` / `check-openapi.sh` が rc=0
- [x] 17.10 **主張の訂正**（R5 / G6）—— 「4 つとも必要で 1 つ戻すだけで落ちる」は false だった。
      落ちるのは 3 つで、余地は 44 px。design D35 に ★ で訂正を入れる。検証: 実測して書いた

## 18. 第 9 回 Q32 —— 閾値は生存信号だけに掛ける

**本人の答え**: 閾値（登録簿に行ができた日より前）を掛けるのは**生存信号だけ**。
記録は時刻がいくら古くても収集開始日を作れる。第 8 回 Q29 の決定は変えない ——
崩れたのは「記録にも同じ閾値を掛けてよい」という**暗黙の前提**だけ。

- [x] 18.1 `coverage::touch_started_on` が**記録か生存信号か**（`Arrival`）を受け取り、
      閾値を生存信号にだけ掛ける。検証: `cargo test --workspace` が rc=0
      （`an_old_record_starts_collection_even_before_registration` —— 登録の 2 年前に
      撮られた写真が開始日を作り、同じ日の生存信号は外れたまま）
- [x] 18.2 移行 0007 の引き直しも同じ規則に。**「汚れている」の印を狭める** ——
      「閾値より前の生存信号の日ちょうどにあり、その日に記録が 1 件も無い」。
      「登録より前」だけだと過去ぶんの取り込みを毎起動で消し、「支える記録が無い」だけだと
      破棄した次の再起動で⑤が⑦に化ける。
      検証: `repair_keeps_a_backfilled_start_date` /
      `repair_does_not_move_forward_after_records_are_dropped`（どちらも壊すと落ちる）
- [x] 18.3 `specs/collection-coverage/spec.md` の収集開始日の Requirement と Scenario、
      `design.md` の D31 に ★ 訂正。検証: `openspec validate --strict` /
      `scripts/check_scenarios.py` が rc=0

## 他 Story が書き手を持つ Scenario（`review/code.md` の R11）

**ST02 は表がその形で持てることまでを満たす。** 契機（WHEN / IF）は他 Story にある ——
`check_scenarios.py` は印しか見ないので、緑が「ST02 が契機まで持っている」と読まれないよう、
ここに明示する。

| Scenario | 契機を持つ Story |
|---|---|
| 半日の停止が時刻の範囲で残る | **ST15**（開始日と終了日で止める。`docs/stories/ST15.md`） |
| 破棄が期間と件数で残る | **ST04**（端末内バッファの上限と破棄。`docs/stories/ST04.md`） |

## 人間の確認待ち

**実機・実目でしか判定できない Scenario。** ここに挙げたものだけが `check_scenarios.py` の
除外対象になる（見出しが `人間の確認待ち` であることに意味がある。箇条書きでは機械に届かない）。

- Scenario: 1 年ぶんが一目で読める

> **「グレースケールでも格子の 3 段が区別できる」はここから外した。** この Scenario の逐語は
> 「3 段それぞれのセルの**相対輝度を取り**、隣り合う 2 段の比が 3:1 以上」で、
> **計算で閉じる**（`web/src/__tests__/state-contrast.test.ts` が描くのと同じ値から計算する）。
> 目で見て区別できるかは `docs/stories/ST02.md` の完了の判定が持つ。

### 実データを待つ判定と、その期日

`docs/stories/ST02.md` の完了の判定「1 か月放置した後に開くと、欠けた日が一目で分かる」は、
**1 か月放置した実データが要る**ので、ST02 の PR では判定できない。

| 判定 | いつ見るか | 何を見るか |
|---|---|---|
| 1 か月放置した後に開くと、欠けた日が一目で分かる | **収集開始から 1 か月後**（ST01 の実機収集が 2026-09-10 に始まっているので **2026-10-10 以降**） | 開いた直後に 2〜3 ソースの直近 4 週が見え、**ひとスクロールで 5 ソースすべて**が見えるか（第 8 回 Q30 で緩めた）。欠けた日が格子の段の違いとして読めるか |
| グレースケールでも格子の 3 段が区別できる（目で見る側） | 実機で 1 度 | 端末の色覚補正（グレースケール）を掛けて 3 段が読めるか。**隣接 3:1 は計算で確かめてある**（`web/src/__tests__/state-contrast.test.ts`）ので、ここは目で見る側だけ |

> **期日は `docs/stories/ST02.md` には書けない**（この節に置いた理由）。
> あのファイルは `docs/stories/stories.json` からの**再生成物**で、
> 手で足すと `scripts/check_chain.py` が「再生成した結果と一致しない」で落ちる（実測）。
> Story の完了の判定は本人のものなので、**下流が書き換える場所ではない**。

## 第 5 回の深掘りで確定したこと（**未決は無い**）

独立レビュー（`review/spec.md`）が、AI が独断で決めていた一方通行の判断を 5 件見つけた。
2026-09-11 に全問回答を得て、上のタスクに反映済み。**下流は勝手に変えないこと。**

| # | 本人の答え | 反映先 |
|---|---|---|
| Q17 | 生存信号に**取得の試行回数と成功回数**を載せる | 3.3b / 7.2b |
| Q18 | 達成の判定は**分母の 95 % 以上**（絶対値の 350 ではない） | 6.1 / 6.5 / 6.5b / 8.6 |
| Q19 | 丸ごと覆う**停止・破棄を記録より優先する** | 5.1 / 5.1b |
| Q20 | 格子の**行は週**（1 行 = 7 日） | 8.4 |
| Q21 | 格子は **3 段**に畳み、8 状態は**週を選んだときの文字**で読む | 8.3 / 8.4 / 8.5 |
| Q22 | 収集開始日は**最初の記録か生存信号が届いた日**。届くまでは開始していない | 1.4 / 1.4b / 1.4c |

## 第 6 回の深掘りで確定したこと（**未決は無い**）

2 巡目の独立レビュー（`review/spec-r2.md`）が見つけた 3 件。2026-09-11 に回答を得て反映済み。
**下流は勝手に変えないこと。**

| # | 本人の答え | 反映先 |
|---|---|---|
| Q23 | 窓はソースごとに**収集開始日から 365 日**。`?from=&to=` は落とす | 6.1 / 6.7 / 6.8 |
| Q24 | 収集開始日は最初の記録が**作られた**日（届いた日ではない） | 1.4 / 1.4b |
| Q25 | 格子は**縦長**（1 行 = 1 週、新しい週が上） | 8.2 / 8.5 / 8.5b |

## 第 8 回の深掘りで確定したこと（**未決は無い**）

下流の独立レビューが出した 2 件と、ST03 の深掘りから相乗りした 1 件。
2026-09-11 に 3 問とも回答を得て反映済み（群 12 / 13 / 15）。**下流は勝手に変えないこと。**

| # | 本人の答え | 反映先 |
|---|---|---|
| Q29 | 時計が狂った信号は**受けるが、収集開始日の計算から外す**（閾値は登録簿に行ができた日） | 12.1 / 12.2 / 12.3 |
| Q30 | 「同時に見える」を**ひとスクロール以内**に緩める（第 7 回 Q28 と NFR-19 は変えない） | 13.1 / 13.2 / 13.3 |
| Q31 | **古い名前を分母から外し、新しい名前が窓を引き継ぐ** | 15.3 / 15.4 |

## 第 7 回の深掘りで確定したこと（**未決は無い**）

3 巡目の独立レビュー（`review/spec-r3.md`）が出した 3 件。2026-09-11 に回答を得て反映済み。
**下流は勝手に変えないこと。**

| # | 本人の答え | 反映先 |
|---|---|---|
| Q26 | 収集開始日は**遡って動く**（常にいちばん古い記録の日） | 1.4b |
| Q27 | 合否は **5 本すべての窓が閉じた日に確定**。それまでは暫定 | 6.1 / 6.8 / 6.9 / 8.6 |
| Q28 | 開いた直後は**各ソースの直近 4〜5 週**。1 年は伸ばして見る | 8.2 / 8.5c |
