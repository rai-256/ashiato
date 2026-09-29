## final review（e99c457..5630e31）

### Strengths

- 削除・復元を利用者単位の advisory lock と単一トランザクションに結び、台帳への追記と状態変更を一体で扱っている。
- day view は削除済み区間を専用ビュー経由の `erased` 行として表し、復元 UI へ識別子を渡している。
- サーバ・Web・e2e の Scenario テスト、OpenAPI、immutable 検査を更新している。

## R1. 重なる削除操作の片方を戻すと、なお削除中の位置が復元される
- 成果物: `crates/server/src/deletion.rs`
- 根拠: `crates/server/src/deletion.rs:122-172,258-269`。A が連鎖削除した位置を重なる B が削除しても B 原因の台帳行を足さず、A を復元すると B が削除中でも位置を戻す。
- kind: technical
- 処置: fixed Task 2 / design D2・D6。重複済み位置にも後の削除原因を台帳へ追記し、A削除→重なるB削除→A復元でB原因の位置が残る回帰テストを追加。

## R2. 不正な `payload.end` を持つ削除済み滞在で一覧・作り直しが失敗し得る
- 成果物: `migrations/202609271716_deletion_ledger.sql`, `crates/server/src/stay_store.rs`
- 根拠: `migrations/202609271716_deletion_ledger.sql:46` と `crates/server/src/stay_store.rs:651` が `payload->>'end'` を直接 `timestamptz` にキャストする。一方 `span_of` は壊れた end を安全に扱う。
- kind: technical
- 処置: fixed Task 1・Task 3 / design D12。`core.try_timestamptz` で不正な end を NULL に畳み、ビューと rebuild の範囲判定で開始時刻へフォールバックするよう統一。malformed end の一覧・rebuild 回帰テストを追加。

## R3. ST22 の移行が計画上限の 1 本を超えている
- 成果物: `migrations/202609291151_detail_counts_index.sql`, `crates/server/src/lib.rs`
- 根拠: `migrations/202609291151_detail_counts_index.sql:1-5` と `crates/server/src/lib.rs:127-134`。Task 1 / design D10 の「移行は 1 本だけ」に対し、deletion ledger と detail counts index の 2 本を足している。
- kind: technical
- 処置: fixed Task 1 / design D10。detail counts のライブ索引を deletion ledger 移行へ統合し、2 本目の移行と MIGRATIONS 登録を除去。

## R4. 破壊操作のボタンがライトテーマでもダーク配色に固定される
- 成果物: `web/src/DayView.tsx`
- 根拠: `web/src/DayView.tsx:326-333` が `SCHEMES.dark` を固定で渡すため、OS の明暗設定に追従しない。
- kind: technical
- 処置: fixed Task 8 / design D9。DayView の破壊操作へ現在の scheme を渡し、ライトテーマでライトの操作面を使う UI 回帰テストを追加。
