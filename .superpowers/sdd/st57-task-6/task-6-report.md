# Task 6: 配布物に更新係を入れる

## 実装内容

- `build.sh server-windows` が `ashiato-updater.exe` をサーバ・インストーラと同じ `ASHIATO_RELEASE` で組み立て、Windows zip の根へ同梱するようにした。
- Windows の依存一覧をサーバと更新係の依存木から作り、`rustls` を含むことを `notices.sh` と `check-artifacts.sh` で検査するようにした。
- `check-artifacts.sh` に更新係の存在、版印、試験差し込み、Linux 配布物への非同梱の検査を追加した。
- `package.sh` / `notices.sh` の self-test に更新係と `rustls` の成功・欠落ケースを追加した。
- `webpki-root-certs` を 1.0.12 へ更新し、脆弱性検査を通した。
- 許諾 self-test の失敗ケースで一時ファイルが残らないよう EXIT trap を修正した。

## TDD Evidence

- RED: `tools/release/package.sh --self-test` は `NG: Windows の build が走っていない`、更新係を要求する check-artifacts の期待は更新係の無い偽 zip で失敗した。これは未実装の更新係を検出する期待どおりの失敗だった。
- GREEN: `scripts/verify-run 6.1` が rc=0。`check-artifacts.sh --self-test`、`package.sh --self-test`、`notices.sh --self-test`、`check-licenses.sh`、脆弱性検査がすべて PASS。

## 変更ファイル

- `tools/release/build.sh`
- `tools/release/package.sh`
- `tools/release/notices.sh`
- `tools/release/check-artifacts.sh`
- `Cargo.lock`
- `openspec/changes/st57-update-notice/evidence.jsonl`

## Self-review

指定された範囲だけを変更し、既存 Scenario の印は維持した。clean commit 上で正規検証を再実行し、`evidence.jsonl` に 5 件すべて PASS の証跡を記録した。一時ファイルの残留も解消済み。

## 懸念

なし。
