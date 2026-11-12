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

## 変更ファイル

- `.github/workflows/ci.yml`
- `tools/release/install-check.sh`
- `tools/release/update-check-install.sh`

## 自己レビュー

- ローカル環境には Windows サービス、ACL、実配布 zip が無いため、Windows の実地検査そのものは実行していない。これは仕様どおり `install-windows` CI で実行する。
- CI の遮断規則は削除 step を持たず、job の終了まで維持する。
