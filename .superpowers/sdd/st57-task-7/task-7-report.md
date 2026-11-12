# Task 7 実装報告

## 実装内容

- `tools/release/update-check-install.sh` を追加した。
  - `--block-outbound` で `ashiato-updater.exe` から loopback 以外への外向き接続を遮断する規則を設定する。
  - `--self-test` で偽の `sc.exe`・`icacls.exe`・`netsh.exe` を使い、サービス・ACL・遮断規則の判定を確認する。
  - 配る zip と試験 zip の Windows 実地検査に、Task 7 の 10 Scenario の印を置いた。
  - 試験 zip では `ASHIATO_SETUP_TEST_UPDATER_URL` を loopback の受け手へ向ける。
- `install-windows` の最初のインストール前に遮断規則を設定し、`install-check.sh` の後に新しい更新係検査を同じ runner で実行するよう CI を更新した。
- job の最後に `update-check-install.sh --list` と配る zip／試験 zip の実出力を突き合わせる検査を追加した。
- `install-check.sh` の遮断対象、更新係の登録・実行・削除・記録の置き場の期待を ST57 の仕様に合わせた。

## TDD Evidence

- RED: `tools/release/update-check-install.sh --self-test` → rc=127（ファイル未存在）。未実装の self-test が実行できないことを確認した。
- GREEN: `tools/release/update-check-install.sh --self-test` → `OK: update-check-install の self-test`。

## 検証

- `bash -n tools/release/update-check-install.sh tools/release/install-check.sh` → rc=0
- `tools/release/update-check-install.sh --self-test` → rc=0
- `tools/release/install-check.sh --self-test` → rc=0
- `bash -c 'f="$1"; actionlint "$f" || docker run --rm -v "$PWD:/repo" -w /repo rhysd/actionlint:1.7.7 "$f"' _ .github/workflows/ci.yml` → rc=0（ローカルに `actionlint` が無く、fallback 経路で完了）
- `git diff --check` → rc=0

## Task gate 追加検証

- `scripts/verify-run 7.1` → 3 件すべて PASS、rc=0。
  - `tools/release/update-check-install.sh --self-test`
  - `tools/release/install-check.sh --self-test`
  - 指定の `actionlint` コマンド
- 証跡は `openspec/changes/st57-update-notice/evidence.jsonl` に記録された。

## 変更ファイル

- `.github/workflows/ci.yml`
- `tools/release/install-check.sh`
- `tools/release/update-check-install.sh`

## 自己レビュー

- ローカル環境には Windows サービス、ACL、実配布 zip が無いため、Windows の実地検査そのものは実行していない。これは仕様どおり `install-windows` CI で実行する。
- CI の遮断規則は削除 step を持たず、job の終了まで維持する。

## Fix round 1（review 1）

### 変更

- F1〜F3: ACL の主体名の固定文字列を正し、pretty JSON の空白を許す無効判定にし、問い合わせ待ちの deadline を正しい算術展開にした。
- F4: 2 本目の zip の前に、サービス停止待ち・プロセス停止・登録削除・プログラム/状態の置き場の削除を行う `reset_machine` を使うようにした。
- F5〜F6: self-test にサービス種別/アカウント、ACL の継承・書き・読み、無効設定、trace の順序の異常系を追加した。実地検査には時計の差し込み、無効化後 2 回の確かめ、再有効化後の問い合わせ件数、DB/設定の指紋、repair の前提・命令・無効状態・記録指紋を追加した。
- F7: `install-check.sh` の削除後案内で `ashiato-updater` の置き場も確認するようにした。

### Fix verification

- RED: 追加後の初回 `tools/release/update-check-install.sh --self-test` は、self-test の偽 Windows 環境に `cygpath` が無く rc=127 になった。self-test の環境構築不足を修正するための失敗として確認した。
- GREEN: `tools/release/update-check-install.sh --self-test` → `OK: update-check-install の self-test`、rc=0。
- `tools/release/install-check.sh --self-test` → `OK: installer の確かめの部品と Scenario の印の置き場を確認した`、rc=0。
- `bash -n tools/release/update-check-install.sh tools/release/install-check.sh` → rc=0。
- `scripts/verify-run 7.1` → 3 件すべて PASS、rc=0（更新係 self-test、install-check self-test、actionlint）。

### Fix round self-review

- Windows のサービス/ACL/配布 zip をこの Linux 環境では実行できないため、実地の結果は確認していない。実地検査は `install-windows` job に残している。
- F8〜F10 と final review 対象の確認事項は、今回の open findings の範囲外なので変更していない。
