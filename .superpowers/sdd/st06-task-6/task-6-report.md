# ST06 Task 6 実装レポート

## 実装内容

- `Retention.dropBy` は、破棄された各 logical source ごとに
  `records.oldest(source)` を読み、そのソースの残存最古記録だけを
  `DropLedger.endBatch` へ渡す。
- `RetentionTest` の 2 ソース回帰は、位置を破棄した後に先頭へ残るアプリ利用
  (`10:10Z`) ではなく、残存する位置 (`10:20Z`) で破棄範囲を閉じることを検証する。
  指定どおり `// Scenario: 破棄の範囲は別のソースの記録で閉じない` を置いた。
- 実装・試験は着手時点で `6c83287` に既に退避コミットされていた。作業中はこの
  正しい実装を変更せず、意図的なガード破壊による RED と復元後の GREEN を実証した。

## RED / GREEN

### RED（ガード破壊）

`Retention.kt` の `records.oldest(source)` を一時的に `records.oldest()` へ戻してから、次を実行した。

```sh
JAVA_HOME=/home/yosis/tools/jdk-17.0.20+8 ANDROID_HOME=/home/yosis/.local/opt/android-sdk PATH=/home/yosis/tools/jdk-17.0.20+8/bin:$PATH ./gradlew :app:testDebugUnitTest --tests 'dev.ashiato.collector.RetentionTest.位置の破棄の範囲は先に残ったアプリ利用ではなく残った位置で閉じる'
```

結果は rc=1。JUnit XML は `tests="1" failures="1" errors="0"` で、期待どおり
`expected: 2026-06-01T10:20:00Z but was: 2026-06-01T10:10:00Z` と失敗した。
これは置き場全体の先頭であるアプリ利用記録によって位置の範囲が誤って閉じられた証拠である。

### GREEN（復元後）

ガードを `records.oldest(source)` へ戻して、次を実行した。

```sh
JAVA_HOME=/home/yosis/tools/jdk-17.0.20+8 ANDROID_HOME=/home/yosis/.local/opt/android-sdk PATH=/home/yosis/tools/jdk-17.0.20+8/bin:$PATH ./gradlew :app:testDebugUnitTest --tests '*RetentionTest*' --console=plain
```

結果は rc=0、`BUILD SUCCESSFUL`。生成された JUnit XML は `RetentionTest` が 11 本、
`UsageRetentionTest` が 7 本で、いずれも failures=0/errors=0（計 18 本実行）だった。

## 変更ファイル

- `collector-android/app/src/main/kotlin/dev/ashiato/collector/Retention.kt`
- `collector-android/app/src/test/kotlin/dev/ashiato/collector/RetentionTest.kt`
- `.superpowers/sdd/st06-task-6/task-6-report.md`

最初の 2 ファイルの実装・試験差分は着手時点でコミット `6c83287` に含まれていた。
今回のコミットは本レポートである。

## 検証

- `git diff --check` は rc=0。
- focused Gradle 実行は rc=0、対象テスト 18 本の実行を XML で確認した。
- 意図的なガード破壊の focused 実行は rc=1、対象回帰 1 本の失敗を XML で確認した。

## 自己レビュー

- 同じ共有 outbox を分割していない（P4/C11 を維持）。
- 破棄順は `SegmentStore` の積載順のまま。範囲終端の検索だけを logical source で絞っている。
- 2 ソースを混在させ、別ソースが置き場全体の先頭となる配置なので、旧実装を確実に検出する。

## 懸念

- なし。ローカル環境では既定の `java` が未設定だったため、既存の
  `/home/yosis/tools/jdk-17.0.20+8` と `/home/yosis/.local/opt/android-sdk` を明示して検証した。
