# Task 6 実装報告

## 実装内容

- `tools/release/build.sh` が `LICENSE`・`THIRD-PARTY-NOTICES.txt`・`SOURCE.txt` を作り、Linux tar / Windows zip のルートへ同梱するようにした。
- APK 用に一時 bundle を作り、`-Pashiato.licenseAssets` で Gradle に渡すようにした。
- `collector-android/app/build.gradle.kts` は property がある場合だけ release assets に bundle を追加するようにした。
- `check-artifacts.sh` に `--expect-license` を追加し、全配布物の LICENSE を指定ファイルと比較するようにした。
- `release.yml` に `ASHIATO_SOURCE_REF` を4つの配布 build 手順へ追加した。
- `ci.yml` の配布 build job を PR head / push commit checkout にし、`package-artifacts` に `built` checkout と head 由来の版番号・LICENSE 検査を追加した。
- `check-guards.sh` に checkout と期待値の構造検査、および指定 Scenario の5種類の偽 workflow self-test を追加した。

## TDD Evidence

### RED

`tools/release/check-guards.sh --self-test` を guard 実装前に実行し、現行 workflow が head checkout を持たないため `NG: ci の package-linux が配布物を組む commit を checkout しない` で rc=1 になった。

### GREEN

実装後、`tools/release/check-guards.sh --self-test` は指定 Scenario を含め `OK: rules・Environment・workflow の守りを検査した` で rc=0。`tools/release/notices.sh --self-test` も rc=0。

## 検証

- `bash -n tools/release/build.sh tools/release/check-artifacts.sh tools/release/check-guards.sh tools/release/notices.sh` — PASS
- `tools/release/check-guards.sh --self-test` — PASS
- `tools/release/check-guards.sh workflows` — PASS
- APK の実ビルド — writable な Kotlin daemon 保存先を指定した環境で PASS。APK に `assets/licenses/LICENSE`・`SOURCE.txt`・`THIRD-PARTY-NOTICES.txt` の3点を確認。
- `scripts/verify-run 6.1` — workflow/guard/self-test の5項目は PASS。build-apk は `/home/yosis/.local/share/kotlin/daemon` が read-only のため FAIL（実装後に writable な一時保存先を指定した直接ビルドでは PASS）。
- `tools/release/check-artifacts.sh --self-test` — 既存の debug APK 経路が同じ Kotlin daemon の read-only 制約で FAIL。

## Fix report

- 変更: `build.sh apk` の Gradle 実行へ writable な `$TMPDIR/ashiato-kotlin-daemon` を `kotlin.daemon.storageDir` として渡した。これにより検証環境の read-only な既定 Kotlin daemon 保存先に依存しない。
- 検証: `PATH=/usr/bin:$PATH scripts/verify-run 6.1`（`scripts/quiet-run verify-6.1-system-unzip -- bash -c ...` 経由） — rc=0、6項目すべて PASS。
- 環境注記: `/home/yosis/bin/unzip` は `-l` を無視して展開する shim だったため、指定された APK 検証コマンドは標準 `unzip` を優先して実行した。APK は `assets/licenses/` の3点を確認済み。

## 変更ファイル

- `.github/workflows/ci.yml`
- `.github/workflows/release.yml`
- `collector-android/app/build.gradle.kts`
- `tools/release/build.sh`
- `tools/release/check-artifacts.sh`
- `tools/release/check-guards.sh`
- `openspec/changes/st53-license-bundle/evidence.jsonl`

## 自己レビュー

要求された Task 6 の範囲に限定し、nightly の checkout / `ASHIATO_SOURCE_REF` は変更していない。検証中に APK の一時展開物が作業木へ残ったため削除し、最終作業木には追跡外生成物がない。Kotlin daemon の read-only 環境制約は未解決の懸念として残る。
