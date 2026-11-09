# 合意済みテストの書き方変更

- Scenario: 設定の書き出しの節 — 収集アプリの設定を追加したため、書き出しの合言葉が消えたことの確認対象を画面全体から `export-section` 内へ限定した。期待する書き出し節の挙動は変更していない。
- `HealthManifestTest.backupRulesExcludeHealthPrefsAndKeepCollector` — Task 5 で `collector_setup.xml` を cloud-backup と device-transfer の両方から外すため、既存の `<exclude>` 件数の期待を health の2個から health＋collector_setup の4個へ更新。
