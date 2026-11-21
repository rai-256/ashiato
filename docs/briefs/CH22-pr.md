# CH22 —— 配る形を外につながる状態で入れて置き、落ち続けや誤りの続きを PR の CI で捕まえる

change: `openspec/changes/ch22-soak-install-online-ci/`

## deep.md に積んだ未決（A）

なし

## 仮決め（B）と Ruling

design の（仮）:
- D1（仮）. 置く段は新しい台本 `tools/release/soak-install.sh` に置き、組み立ての鍵が変わることを受け入れる
- D2（仮）. 消すのは台本の掃除で、対象は `update-apply-install.sh` の `reset_machine` と同じものに PC の収集アプリを足す
- D4（仮）. 起こし直しは、5 秒ごとの状態とプロセスの番号と、システムの記録の 2 つから数える
- D5（仮）. 置く段の始まりは「塞ぎの規則を消す直前」、置く間は「入れ終わって healthz が 200 になってから 5 分」
- D6（仮）. 同じ誤りは、日時を除いた中身が同じ行が 1 つのログに 3 回以上で落とす。サーバの panic の行は 1 行で落とす
- D7（仮）. PC の収集アプリは、入れ終わりに起こし、1 分後に 1 回止めて起こし直す
- D8（仮）. 時間の上限は、置く段の手順に 15 分、`install-windows` の job に 100 分
- D13（仮）. 問い合わせの上限だけのときの警告は、GitHub の注記（`::warning::`）で出す
- D16（仮）. 実地の置く段は GitHub Actions の runner に限定する
- D17（仮）. 置く時間 300 秒を自己試験で固定する

全 Task の ledger の `Ruling:`（逐語）:
- 全 Task の ledger に `Ruling:` の行は無い。

## Task

4 Task のうち、埋めた Task は 4 本。
残した項目は無い。

## 通した検証（`evidence.jsonl` の項目とコマンドごとの最新）

- 1.1 `tools/release/soak-install.sh --self-test` → rc=0（PASS・head 0ce8175）
- 1.1 `tools/release/check-ng-observations.sh tools/release/soak-install.sh` → rc=0（PASS・head 0ce8175）
- 1.1 `bash -c 'test "$(grep -c "^[[:space:]]*# Scenario: " tools/release/soak-install.sh)" -ge 20 && grep -q "^SOAK_SAME_ERROR_LIMIT=3$" tools/release/soak-install.sh'` → rc=0（PASS・head 0ce8175）
- 2.1 `tools/release/soak-install.sh --self-test` → rc=0（PASS・head 184ec31）
- 2.1 `bash -c 'test "$(tools/release/soak-install.sh --list | wc -l)" -eq 2'` → rc=0（PASS・head 184ec31）
- 2.2 `tools/release/soak-install.sh --self-test` → rc=0（PASS・head 184ec31）
- 2.2 `bash -c 'f="$1"; { actionlint -ignore SC2155 "$f" || docker run --rm -v "$PWD:/repo" -w /repo rhysd/actionlint:1.7.7 -ignore SC2155 "$f"; }' _ .github/workflows/ci.yml` → rc=0（PASS・head 184ec31）
- 2.2 `bash -c 'grep -q "soak-install.sh --self-test" .github/workflows/ci.yml && grep -q "soak-install.sh --list" .github/workflows/ci.yml && grep -q -- "--block-outbound" .github/workflows/ci.yml'` → rc=0（PASS・head 184ec31）
- 3.1 `bash -c 'grep -q "soak-install.sh" docs/testing.md && grep -q "置く段" docs/testing.md'` → rc=0（PASS・head 2231311）
- 4.1 `python3 scripts/check_scenarios.py . ch22-soak-install-online-ci` → rc=0（PASS・head 7c2a39d）
- 4.1 `tools/release/soak-install.sh --self-test` → rc=0（PASS・head 7c2a39d）
- 4.1 `tools/release/install-check.sh --self-test` → rc=0（PASS・head 7c2a39d）
- 4.1 `tools/release/update-check-install.sh --self-test` → rc=0（PASS・head 7c2a39d）
- 4.1 `tools/release/update-apply-install.sh --self-test` → rc=0（PASS・head 7c2a39d）
- 4.1 `bash -c 'git diff --name-only origin/main...HEAD | grep -Ev "^(tools/release/soak-install\.sh|\.github/workflows/ci\.yml|docs/testing\.md|docs/changes/CH22\.md|openspec/changes/ch22-soak-install-online-ci/.*)$" | grep -v "^docs/briefs/" | { ! grep -q .; }'` → rc=0（PASS・head 7c2a39d）

## レビュー（`review/code.md`）

- 指摘 40 件（escalated 1・fixed 39）
- 処置の re-review: 9 回。最後の判定が NOT ADDRESSED のもの: なし

## 未処置の申し送り

なし
