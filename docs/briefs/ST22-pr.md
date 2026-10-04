# ST22 記録を消したことにできる

## 未決（A）

なし。Q6 は `deep.md` に回答済み。後着位置の取り込み直後照合は ST23 へ申し送った。

## 仮決めと Ruling

- 上流の D7 / D8 / D9 / D10 は既存の仮決めを維持した。
- R1/R5: 復元対象の原因ごとの最新台帳行を確認し、要求外に有効な erase 原因が残る位置は戻さないと決めた。複数原因を台帳で保持する分、復元 SQL は複雑になる。
- R6: 重複消去の回帰テストは no-op rebuilder で前提状態を直接作ると決めた。実際の作り直しでは吸収されるため、実フロー fixture だけでは検証できない。
- R8: 取り込み直後の時間帯照合という Q6 の答えは ST23 へ送った。ST22 の既存 tasks を増やさず、外部出力の担保を次 Story で実装する。
- R11: 隣接する空白日の coverage 回帰は ST23 へ送った。現行の `active_days` は削除済みを数える `core.event` の実装を維持する。

## 実装・Task

- 10 Task / 34 項目を完了。
- R1/R5〜R7、R9、R10、R12、R13 を処置済み。
- R8 と R11 は ST23 に申し送り。いずれも `docs/handoff/ST23.md` に根拠と対象を記録した。

## 検証

- `python3 scripts/review_triage.py . st22-record-deletion` — rc=0
- `cargo check -p ashiato-server --tests` — rc=0
- `cd web && npm run lint && npm run test -- --run` — rc=0（22 files / 152 tests）
- `cargo fmt --all --check` — rc=0
- `cargo test -p ashiato-server deletion_tests` — BLOCKED_INFRA（共有 PostgreSQL 55432 へ接続できず pool timeout）
- 既存 evidence の統合検証は final review 前の commit のものが含まれるため、上記の最新検証と区別した。

## 未処置の申し送り

- ST23: 失敗した過去日の作り直し後に届く位置を、取り込み直後に削除済み時間帯と照合して隠す。
- ST23: 削除日の隣の空白日の coverage 状態が変わらない回帰テストを追加する。
