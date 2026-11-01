# Task 3 実装報告

## 実装内容

- Android の版番号・versionCode・開発版 suffix・release 署名切り替えを根の `Cargo.toml` と `ASHIATO_BUILD_VERSION` に統一した。
- `version-table.tsv` を Gradle task と `version.sh --self-test` の双方で検査し、APK 版番号検査の入口を追加した。
- Web の build stamp を根の `Cargo.toml` と `ASHIATO_BUILD_VERSION` から解決し、`Cargo.toml` を印の source path に追加した。`package.json` の版番号を削除した。
- `tools/check-version.sh`、CI の chain/build-stamp 配線、release build-stamp 検査を追加・更新した。
- 画面の build-stamp テストと ST49 e2e の期待版を根の出所へ切り替えた。

## テスト結果

- `tools/release/version.sh --self-test`: PASS
- `tools/check-version.sh --self-test` / `tools/check-version.sh`: PASS
- `:app:verifyVersionCodeTable`: PASS
- `tools/release/check-artifacts.sh --self-test-gradle-range`: PASS
- 指定 APK assemble + `check-artifacts.sh apk-version`: PASS（`0.1.0-beta.2`, `10002`）
- `web/src/__tests__/build-stamp.test.ts`: 11/11 PASS
- Web 全体: 52 files / 522 tests PASS
- Android `testDebugUnitTest`: BUILD SUCCESSFUL
- `tools/check-build-stamp.sh release`: PASS
- `tools/check-build-stamp.sh android`: PASS

## TDD Evidence

- RED: `npx vitest run src/__tests__/buildStamp.test.ts` は、実装前に npm cache の EROFS で停止した。後に依存を一時 cache へ導入し、指定入口で 11/11 PASS を確認した。
- GREEN: `scripts/quiet-run web-test -- bash -c 'cd web && npx vitest run src/__tests__/build-stamp.test.ts'` は 11/11 PASS。

## 変更ファイル

`.github/workflows/ci.yml`、`collector-android/app/build.gradle.kts`、`tools/check-build-stamp.sh`、`tools/check-version.sh`、`tools/release/check-artifacts.sh`、`tools/release/version-table.tsv`、`web/build-stamp.ts`、`web/vite.config.ts`、`web/vitest.config.ts`、`web/package.json`、`web/package-lock.json`、`web/src/vite-env.d.ts`、Web の stamp テスト/e2e。

## 自己レビュー・懸念

- Kotlin daemon は sandbox 外の `/home/yosis/.local/share/kotlin` へ一時ファイルを書けず警告を出したが、Gradle は fallback compiler で成功した。
- npm は既定 cache が read-only のため、検証時のみ repo 内の一時 cache を使った。生成した依存物は追跡対象外である。
