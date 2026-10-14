# ST14 下流のコードレビュー

## final review（e32f33a..078d803）

席: final reviewer（`superpowers:requesting-code-review` の `code-reviewer.md`）。review package は
`.superpowers/sdd/tasks/review-e32f33a9..078d803c.diff`（60 commit）。全 Task の ledger（deferred-minor / parked / Ruling の行は 0）と、
全 Task の指摘の記録の Minor / ⚠️ を指した。判定は **With fixes**（Critical 0 / Important 3 / Minor 6）。

申し送り（`docs/handoff/ST14.md`）は開始時から増えていない（最後の変更は 00398a42、preflight の生成より前）。
C1（`export::CATALOG`）は base にも main にも無いので ST33 の下流が受ける。C3 / K1 / K3 / R4 / R7 / R24 / R26 / st05 R6 は実装に入っている（final reviewer の突き合わせ）。

## R1. 収集開始日はあるが「最後に生きていた時刻」の材料が何も無いソースは、永久に途切れと判定されない
- 成果物: crates/server/src/outage.rs:122-134（`last_alive` が `None` → `judge` の `filter_map` で落ちる）・outage.rs:319-320 の試験
- 根拠: 取得できない状態の生存信号も `touch_started_on(Arrival::Heartbeat)` で収集開始日を立てる（`lib.rs` の `heartbeat_one`）。
  最初から権限が無い写真・ブラウザ履歴、窓を一度も読めず clock-skew しか届かない PC は、開始日を持ち判定の対象なのに判定から黙って外れる。
  deep.md Q2 は「知らせる —— 取得できる状態の生存信号だけを生きていると数える」、C2 が鳴らさないとしたのは「一度も届かない」（開始日の無い）収集だけ。
  spec の (4) は開始日の無いものだけを外し、この場合は書いていない。実装が D 番号なしで決めていた
- kind: conflict
- 提案: 収集開始日の Asia/Tokyo の 0 時を「最後に生きていた時刻」の下限に足し（`alive_floor` と同じ形）、design に D<n>（仮）と反転条件を書く

## R2. 判定の材料を台帳の助言ロックの外で読んでいるので、同時の問い合わせで途切れが開き直され、もう一度鳴る
- 成果物: crates/server/src/lib.rs:2740-2748（`outages_get`）・crates/server/src/outage_store.rs:169-177（`record` の `pg_advisory_xact_lock`）
- 根拠: `outage_store::inputs` と `outage::judge` は `record` の取引の前に走る。A が「X は途切れ」と判定 → B が「X は戻った」と判定して先にロックを取り `closed` → A がロックを取ると開いた途切れが無いので新しい `opened`。
  識別子が変わり端末で再び鳴る（Q4「途切れ 1 回につき端末ごとに 1 回」に反する）。`st14_ledger_concurrent_opens_once` は単一スレッドの `join!` で重なりを保証しない（Task 7 F1 / F2）
- kind: technical
- 提案: ロックを取った同じ取引の中で `inputs` を読み `judge` する（`inputs` が取引を受ける）

## R3. `has_non_collected` を支える索引が無く、記録が増えると問い合わせが遅くなり通知が黙って止まる
- 成果物: crates/server/src/outage_store.rs（`EXISTS (... AND e.origin <> 'collected')`）・migrations/202610141130_outage_alert.sql
- 根拠: `core.event` の索引に `origin` を含むものが無い。収集した記録しか持たないソース（位置は年に数十万行）では一致が無く、問い合わせのたびに利用者 × ソースの全行を読む。
  収集アプリは 10 秒で打ち切るので遅くなると Unreachable →「通知が変わらない」（Task 5 F1）。design D4 は「足りなければ D 番号を足して部分索引」
- kind: technical
- 提案: まだ merge していない同じ移行に `(user_id, logical_source) WHERE origin <> 'collected'` の部分索引を足し、D<n>（仮）を書く

## R4. design.md が実装に合わせて書き戻されていない（D7 の追記のみの関数・D21 の no-preference）
- 成果物: openspec/changes/st14-outage-alert/design.md（D7 / D21）
- 根拠: D7 は「既存の台帳と同じ関数」のまま。実装は申し送り K3 どおり台帳専用の `core.reject_outage_ledger_change()`（Task 1 ⚠️）。
  D21 の no-preference の読み替え（Task 10 F3）も書かれていない
- kind: technical
- 提案: D7 と D21 を実装の形に直す

## R5. Scenario の印を括弧の中に書いていて、`check_scenarios.py` が `）` まで名前として拾う
- 成果物: crates/server/src/outage_tests.rs:422
- 根拠: `（Scenario: 停止が終わった時刻から数え直す）` —— 印の名前が `停止が終わった時刻から数え直す）` になり warn
- kind: technical
- 提案: 印を独立した行の `// Scenario: <名前>` にする

## R6. 台帳の形の `CHECK` が `opened` に `last_alive_at` を求めていない
- 成果物: migrations/202610141130_outage_alert.sql（`outage_ledger_shape`）
- 根拠: design D7 は「`last_alive_at` は `opened` だけが持つ」。いまの CHECK は `opened` で NULL を許す
- kind: technical
- 提案: `opened` のとき `last_alive_at IS NOT NULL` を CHECK に足し、試験で固定する

## R7. 偽データの clock-skew の payload の鍵が契約と違い、コメントの開始日が古い
- 成果物: tools/seed.sh（`c02-window` の `kind = clock-skew`）
- 根拠: seed は `trigger` / `available` / `references` / `unavailable`、契約（`crates/server/src/contract.rs:502`）は `clock_trigger` / `clock_available` / `clock_references` / `clock_unavailable` / `skew_ms`。
  サーバは `kind` しか読まないのでいまは害が無い。コメントの「開始日（2026-09-07）」も古い（Task 10 F1）
- kind: technical
- 提案: 契約の鍵にそろえ、コメントを直す

## R8. 達成日（NFR-13）は取りこぼしの日でも取得できる状態の信号があれば数え、格子は同じ日を③に塗る
- 成果物: crates/server/src/coverage.rs（`achieve_one` は `decide` を通らない）
- 根拠: 格子は C9 / D 番号どおり取りこぼしの日を③にするが、達成の数えは生存信号の `capturable` を見るだけ。spec は達成の側の扱いを書いていない
- kind: conflict
- 提案: いまの振る舞いを D<n>（仮）として書き、反転条件（達成も格子の状態から数える）を付ける

## R9. Android の問い合わせは位置のスケジューラにだけ乗っている
- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/OutagePoller.kt・LocationService.kt
- 根拠: 位置の収集が動いていない構成では、アプリ利用の途切れも携帯に届かない
- kind: technical
- 提案: 注記のみ（final reviewer も「D12 の設計どおり」としている）

## R10. 取りこぼしの理由の文言を、端末から来た文字で `messages` のオブジェクトから直に引いている
- 成果物: web/src/CoverageGrid.tsx（`m.gapReason[g.reason]`）
- 根拠: `reason` は端末から来る値で、`constructor` などは prototype を引く（Task 10 F2）
- kind: technical
- 提案: `Object.hasOwn` で確かめ、無ければ既定の文言にする
