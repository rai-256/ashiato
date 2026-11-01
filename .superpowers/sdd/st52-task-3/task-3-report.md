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

## Task gate 証跡

`scripts/verify-run 3.1`、`3.2`、`3.3`、`3.4` を最終 commit 後の木で実行し、すべて PASS。証跡は `openspec/changes/st52-release-channels/evidence.jsonl` に記録された。

- 3.1: versionCode 表、beta APK (`versionCode=10002`)、桁超過 Gradle 検査の 3 件が PASS。
- 3.2: build-stamp Vitest 11/11 と `npm run build` が PASS。
- 3.3: `check-version.sh --self-test` と通常検査が PASS。
- 3.4: release / android の build-stamp 検査が PASS。

検証環境の `node-compile-cache` が verify-run の一時領域に残ったという注意が出たが、検証の終了コードはすべて 0 だった。

## TDD Evidence

- RED: `npx vitest run src/__tests__/buildStamp.test.ts` は、実装前に npm cache の EROFS で停止した。後に依存を一時 cache へ導入し、指定入口で 11/11 PASS を確認した。
- GREEN: `scripts/quiet-run web-test -- bash -c 'cd web && npx vitest run src/__tests__/build-stamp.test.ts'` は 11/11 PASS。

## 変更ファイル

`.github/workflows/ci.yml`、`collector-android/app/build.gradle.kts`、`tools/check-build-stamp.sh`、`tools/check-version.sh`、`tools/release/check-artifacts.sh`、`tools/release/version-table.tsv`、`web/build-stamp.ts`、`web/vite.config.ts`、`web/vitest.config.ts`、`web/package.json`、`web/package-lock.json`、`web/src/vite-env.d.ts`、Web の stamp テスト/e2e。

## 自己レビュー・懸念

- Kotlin daemon は sandbox 外の `/home/yosis/.local/share/kotlin` へ一時ファイルを書けず警告を出したが、Gradle は fallback compiler で成功した。
- npm は既定 cache が read-only のため、検証時のみ repo 内の一時 cache を使った。生成した依存物は追跡対象外である。

## Fix round 1

### 対応した指摘

- F1: `check-build-stamp.sh release` が、4 部品の release 印・dirty/debug 無し・組み立て後の clean status、env 無し、出所不一致、形不正、0.9.0 の出所変更、未 commit の出所変更を実際に判定してから各 Scenario の印を出すようにした。
- F2: Gradle self-test が clone の `Cargo.toml` を `0.100.0` に変更し、env 無しで失敗することと `MINOR が上限を超えた` の理由を確認するようにした。
- F3: `apk-version` が `version.sh code` と APK の `versionCode` を比較し、2 本の Scenario 印を判定箇所に置いた。
- F4: Gradle の beta 番号を 1〜99 に揃えた。
- F5: Web の偽 git が `:/Cargo.toml` pathspec のときだけ未 commit 変更を返すようにした。

### Fix round 1 の検証

- `npx vitest run src/__tests__/buildStamp.test.ts`: 11/11 PASS。
- `scripts/quiet-run unit -- ./collector-android/gradlew -p collector-android :app:verifyVersionCodeTable`: rc=0。
- `scripts/quiet-run apk-range -- tools/release/check-artifacts.sh --self-test-gradle-range`: rc=0。
- APK assemble + `check-artifacts.sh apk-version`: rc=0、`versionCode='10002' versionName='0.1.0-beta.2'`。
- `scripts/quiet-run stamp-release-final -- tools/check-build-stamp.sh release`: rc=0。
- `scripts/quiet-run stamp-android -- tools/check-build-stamp.sh android`: rc=0。
- `tools/check-version.sh --self-test` / `tools/check-version.sh`: rc=0。
- `scripts/quiet-run web-build -- bash -c 'cd web && npm run build'`: rc=0。
- `python3 scripts/check_scenarios.py .`: 今回の Task 3 の印は認識された。全 change では下流 Task の未実装 Scenario 46 件が残るため rc=1（今回の範囲外）。

### Fix round 1 の自己レビュー

4 部品すべてについて、渡した版・env 無しの出所版・未 commit の Cargo.toml による dirty を判定していることを確認した。APK の versionCode は APK 内の versionName から `version.sh code` を通して比較する。今回の変更に関する追加の懸念はない。

## Fix round 2

### 対応した指摘

- F1: `tools/check-build-stamp.sh release` の画面・携帯の印を部分一致から一意な完全な印の抽出へ変更し、env 無し・出所変更後 commit・未 commit の各ケースでも画面と携帯の印を commit/dirty まで判定するようにした。サーバ・PC・APK の既存判定と合わせ、各 Scenario の印を判定後に出力する。
- F4/F12: Gradle の beta 番号の許容範囲を 1〜89 に戻し、`version.sh` と Web の規則および versionCode の正式版 tail=99 と揃えた。

### 検証

- `bash -n tools/check-build-stamp.sh` / `git diff --check`: PASS。
- `scripts/quiet-run unit -- ./collector-android/gradlew -p collector-android :app:verifyVersionCodeTable`: rc=0、BUILD SUCCESSFUL。
- `scripts/quiet-run apk-range -- tools/release/check-artifacts.sh --self-test-gradle-range`: rc=0。
- `tools/check-version.sh --self-test && tools/check-version.sh`: rc=0。
- `scripts/quiet-run stamp-release-fix2 -- tools/check-build-stamp.sh release`: rc=0、6つの release Scenario の判定完了。
- `scripts/quiet-run stamp-android-fix2 -- tools/check-build-stamp.sh android`: rc=0。

### TDD Evidence

既存の self-test / release 検査を回帰テストとして実行した。今回の修正は検査の網羅性と beta 境界の回帰であり、新しい Scenario の追加は不要だった。

### 自己レビュー

変更は指摘された2ファイルに限定した。CI の setup-node 指摘は final review の確認対象として変更していない。

### 追加回帰確認

- `scripts/quiet-run cargo-test-fix2 -- cargo test`: rc=101（1847 passed / 1 failed / 3 ignored）。失敗は今回の変更箇所外の `archive_reparse_persistent_store_failure_is_ledgered_and_throttled` で、PostgreSQL の deadlock (`40P01`) により `unwrap()` が失敗したもの。版検査・build-stamp の対象検証はすべて rc=0 だった。
