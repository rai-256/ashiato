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

## Fix report: review 1 F1〜F3

- 変更: `check-artifacts.sh` が配る updater について、`settings/test-hooks.json` が無いこと、GitHub API と releases の宛先があることを updater 本体の文字列で検査するようにした。self-test に updater の試験差し込み、宛先欠落、試験用設定ファイル、`rustls` 欠落の偽配布物を追加した。
- 変更: `package.sh --self-test` の偽 build に updater を自作させ、存在を確認する循環的な検査を削除した。updater の配布物検査は `check-artifacts.sh` に集約した。
- RED: `tools/release/check-artifacts.sh --self-test` は正常な偽 updater に宛先を入れる前に `NG: Windows の更新係に GitHub API の宛先が無い` で失敗し、新しい検査が欠落を検出することを確認した。
- GREEN: `scripts/verify-run 6.1` が rc=0。`check-artifacts.sh --self-test`、`package.sh --self-test`、`notices.sh --self-test`、`check-licenses.sh`、`check-vulns.sh` の 5 件が PASS。
- 変更ファイル: `tools/release/check-artifacts.sh`、`tools/release/package.sh`、`openspec/changes/st57-update-notice/evidence.jsonl`
- self-review: F1〜F3 の範囲以外の実装変更はなく、`git diff --check` も通過。コミット `699325179` にコード変更を記録した。

## Fix report: review 1 F1〜F2 の再検証

- 変更: 配布 zip の目録から対象の実行ファイルだけを取り出し、updater の完全な GitHub releases URL と `test-hooks.json` の不在を検査するよう整理した。
- 変更: self-test の updater 負例を、試験差し込み・宛先欠落・GitHub API だけ・試験用設定ファイルに分離し、rustls 欠落の依存一覧も rc≠0 を確認する形にした。
- RED: 変更後の最初の self-test は正常 fixture の宛先表現が検査形式と一致せず `NG: Windows の更新係に GitHub releases の宛先が無い` で失敗した。fixture を正しい完全 URL に直し、API だけの負例から完全 URL を除去した。
- GREEN: `tools/release/check-artifacts.sh --self-test` rc=0（最終実行 535 秒）。前回の `package.sh --self-test`、`notices.sh --self-test`、`tools/check-licenses.sh`、`tools/check-vulns.sh --base "$(git merge-base HEAD origin/main)"` もすべて rc=0。
- self-review: `git diff --check` を通過。F3 の package 側の循環検査除去は既存コミットで完了しており、今回も変更していない。残る CI 実配布物の確認は最終 review の対象。
