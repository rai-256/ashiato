# Task 7 レポート

## 実装内容

- Rust のロックファイルを互換範囲内で更新した。
- Web のロックファイルを `npm update` で互換範囲内に更新した。
- Android のロックファイルを `--write-locks` で再生成し、全構成のロック状態を反映した。
- 互換範囲内で解消できない勧告は、`osv-scanner.toml` に 90 日以内の期限と定型理由を付けて記録した。
- 依存名・版の一覧はこのレポートに記載していない。

## テストと検証結果

- `XDG_CACHE_HOME=/tmp/ashiato2-vuln-cache tools/check-vulns.sh` — rc=0
- `XDG_CACHE_HOME=/tmp/ashiato2-vuln-cache tools/check-vulns.sh --base "$(git merge-base HEAD origin/main)"` — rc=0、新しく持ち込んだ件数 0
- `scripts/quiet-run ch11-cargo-test -- bash -c './tools/db.sh up -d --wait db && ./tools/db-roles.sh && ./tools/tmp-leak-check.sh rust'` — rc=0、試験 2455 本、一時ファイル残りなし
- `npm_config_cache=/tmp/ashiato2-npm-cache bash -c 'cd web && npm ci && npm test -- --run'` — rc=0、52 files / 513 tests passed
- `scripts/quiet-run ch11-android-unit -- ./collector-android/gradlew -p collector-android :app:testDebugUnitTest` — rc=0、BUILD SUCCESSFUL

## TDD Evidence

この Task は新しい実行時振る舞いや Scenario を追加せず、依存ロックと脆弱性例外表だけを更新するため、新規テストコードの RED/GREEN サイクルは該当しない。

- RED 相当: 更新前の脆弱性検査は rc=1 で、未処置の勧告が報告された。
- GREEN 相当: 更新後に脆弱性検査と base 比較がともに rc=0 になり、既存試験もすべて通過した。

## 変更ファイル

- `Cargo.lock`
- `web/package-lock.json`
- `collector-android/app/gradle.lockfile`
- `osv-scanner.toml`
- `.superpowers/sdd/ch11-task-7/task-7-report.md`

## 自己レビュー

- `git diff --check` は問題なし。
- アプリケーションコード、テストの期待値、`tasks.md` は変更していない。
- Web の最初の検証は既定 npm キャッシュの読み取り専用エラーで失敗したが、許可された一時キャッシュを指定して同一コマンドを再実行し、rc=0 を確認した。

## 懸念

- 脆弱性検査のデータベース上、互換範囲外または推移的依存に由来する残存勧告は例外表で管理している。期限はすべて 90 日以内で、次回更新時に再評価する。

## Task gate 指摘への追検証

- `XDG_CACHE_HOME=/tmp/ashiato2-vuln-cache scripts/verify-run 7.1` — 2 コマンドとも PASS、`evidence.jsonl` に記録済み。
- `XDG_CACHE_HOME=/tmp/ashiato2-vuln-cache npm_config_cache=/tmp/ashiato2-npm-cache scripts/verify-run 7.2` — 3 コマンドとも PASS、`evidence.jsonl` に記録済み。
- Web 検証は 52 files / 513 tests passed。ただしハーネスの一時領域に `node-compile-cache` 1 件が残った注意が証跡に記録された。

## Fix report（review 1 / F1）

- `osv-scanner.toml` の該当行を、製品に届かない理由ではなく互換範囲内に修正版がないため更新予定として `上げる予定: 互換範囲内に修正版なし` に修正した。
- `scripts/verify-run 7.1` は 2 コマンドとも検証環境の `mkdir: Read-only file system` により rc=2（FAIL）となった。この結果を証跡に記録し、別の結果で置き換えていない。
- 環境制約を切り分けるため、`scripts/quiet-run ch11-vulns-current -- bash -c 'XDG_CACHE_HOME=/tmp/ashiato2-ch11-vuln-cache tools/check-vulns.sh'` は rc=0、`scripts/quiet-run ch11-vulns-base -- bash -c 'XDG_CACHE_HOME=/tmp/ashiato2-ch11-vuln-cache tools/check-vulns.sh --base "$(git merge-base HEAD origin/main)"'` も rc=0 だった。

## Fix report（review 1 / F1 再修正）

- `osv-scanner.toml` の該当行を、現時点で修正版が無いという事実ではなく、修正版公開後に互換範囲内へ更新する計画として `上げる予定: 修正版公開後に互換範囲内へ更新` に修正した。
- `scripts/verify-run 7.1` は 2 コマンドとも検証環境の `mkdir: Read-only file system` により rc=2（FAIL）。
- `scripts/verify-run 7.2` は Rust と Android が PASS、Web は `npm ci` の `node_modules` 展開・削除エラーで rc=226（FAIL）。
- 切り分けとして `scripts/quiet-run ch11-vulns-current-fix-cache -- env XDG_CACHE_HOME=/tmp/ashiato2-ch11-vuln-cache-fix tools/check-vulns.sh` は rc=0、`scripts/quiet-run ch11-vulns-base-fix-cache2 -- bash -c 'XDG_CACHE_HOME=/tmp/ashiato2-ch11-vuln-cache-fix-base tools/check-vulns.sh --base "$(git merge-base HEAD origin/main)"'` も rc=0。検証証跡の FAIL/PASS は置き換えていない。
