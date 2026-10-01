# ST06 final-review 修正波（2026-09-26）

STATUS: DONE

## 処置

- R17 / 3.3: Q9=c を spec・D4/D8・実装・試験に反映。保存済みの窓からの `AgeClock` の経過（数えなかった前進を補正）が 10 日以内なら停止を保ち、10 日を超えれば見込みの下限から再開する。保存済みの終わりから下限までの正の期間を gap とし、gap とイベントの全件を永続化した後にだけ窓を進める。通常の取得は切り詰めない。再起動・再開後の通常取得・gap の永続化失敗と再試行を検証した。
- R18 / 2.1: `getExtras()` のガードを API 35 に変更。API 34 と 35 の Robolectric で本番の `queryEvents` → snapshot 変換を実行し、34 では追加欄なし、35 では action/category を保存することを検証した。
- R19 / D2: `QUERY_ALL_PACKAGES` を宣言。merged manifest の権限を PackageManager 経由で検査し、起動入口のない第三者パッケージの表示名を本番の `PackageManagerAppLabels` で取得する回帰テストを追加した。
- R20 / 4.2: `pg_constraint` で `core.source` の全参照を確認。event / heartbeat / coverage / coverage_span / drop_report / source.succeeds のいずれかが残れば登録簿を残す。並行追加・将来の外部キーにも DELETE 時の例外処理で対応する。各参照だけが存在する fixture と未使用ソースの削除を実 DB の transaction 内で検証した。
- R21 / 1.1: `Collected.enqueued` は登録を試みた記録であり、永続化件数ではないと文書化した。
- R22 / 4.3: 初回完了後の DAILY の書込み失敗では `usage_rollup_not_persisted` のみを出し、未取り込みの粒度を数える `notImported` を増やさない。保存済みの印が保持される回帰テストを追加した。

`review/code.md` の R17〜R22 は上記の task / D 番号で処置済みに更新した。指摘の根拠本文は保持した。
`tasks.md` と `deep.md` は変更していない。Q9=c の Scenario を 1 本追加したため change の Scenario 総数は従来の 58 本から 59 本になる（tasks の説明数はこの実装波では変更していない）。

## TDD の RED

- Android: `.superpowers/sdd/logs/20260926-214025-final-red-android.log`。自動再開 2 件・表示名の可視性・rollup のログが期待どおり失敗。API 34/35 は jar 未配置だったため、別途下記で RED を取り直した。
- API 34: `.superpowers/sdd/logs/20260926-214317-final-red-api.log`。本番 snapshot 変換の `getExtras()` で `NoSuchMethodError`。14 件中 1 件失敗（API 35 は通過）。
- down: `.superpowers/sdd/logs/20260926-214119-final-red-migration.log`。heartbeat のみの状態で外部キー違反。未使用ソースの削除は通過。

最初の Android 実行にはテストの nullable 型のコンパイルエラー、最初の DB 実行には未起動 DB による接続失敗があった。どちらも RED の根拠には数えていない。

## 検証

すべて repository root の `scripts/quiet-run` 経由で実行し、以下は rc=0。

| 入口 | 結果 |
|---|---|
| `scripts/verify-run 1.1` | Android 全単体 43 suites / 341 tests、skipped=0 / failures=0 / errors=0。LocationFix の差分なし |
| `scripts/verify-run 2.1` | UsageSourceTest 14 件、API 34/35 の本番変換を含む |
| `scripts/verify-run 3.3` | UsageWindowClockTest 10 件 |
| `scripts/verify-run 4.1` | UsageGapTest PASS |
| `scripts/verify-run 4.2` | Rust 6 件 PASS（参照6種の fixture を含む）と migration 検査 PASS |
| `scripts/verify-run 4.3` | RollupImportTest 16 件 |
| `scripts/verify-run 8.1` | Scenario 470 件 / 担保あり 470 / 人間の確認待ち 0 |
| `scripts/verify-run 8.3` | review triage PASS（指摘45件） |
| `git diff --check` | PASS |

全単体の結果ログ: `.superpowers/sdd/logs/20260926-214951-final-verify-1.1.log`。
個別の `verify-run` のコマンド・コード指紋・rc・PASS は `openspec/changes/st06-app-usage/evidence.jsonl` に記録した。

## 環境と限界

- 設定済み Robolectric 置き場には API 36 の jar しかなかった。失敗が示した API 34/35 の jar を Maven Central から `/tmp/st06-robolectric-jars` に取得し、既存 API 36 jar と合わせた。上記検証では `ASHIATO_ROBOLECTRIC_JARS=/tmp/st06-robolectric-jars HX_CHANGE=st06-app-usage` を指定した。再実行時も同じ指定、または不足 jar の配置が必要。
- DB が停止していたので `docker compose up -d --wait db` で起動した。DB テストは各 fixture を rollback し、既存データを削除していない。
- この修正波では Android 計測テスト・smoke・Rust 全 workspace の再実行は行っていない。対象の Android 全単体と DB 回帰テストを実行した。
- 未解決の R17〜R22 はない。独立の最終確認は controller が行う。
