# ST57 —— 新しい版が出たら知らせる

change: `openspec/changes/st57-update-notice/`

## deep.md に積んだ未決（A）

なし

## 仮決め（B）と Ruling

design の（仮）:
- D11（仮）. 名前
- D12（仮）. TLS は `rustls` と OS の根の確かめ（`rustls-platform-verifier`）
- D13（仮）. 見出しの集合は 5 つの名前の内側
- D14（仮）. サーバの読みの口 `GET /update-status` の形
- D15（仮）. サーバは更新係の時刻が 48 時間より古ければ「更新係が動いていない」とする
- D16（仮）. 携帯の機会は `LocationService` の起動と生存信号の定期の時
- D17（仮）. PC は `setup` の最後に聞き、`update-check off|on` の口でいつでも切り替える
- D18（仮）. Windows の CI は更新係から外への接続を塞いでからインストールする
- D19（仮）. 確かめた記録は 1 MB で古い半分を退ける
- D20（仮）. 文
- D21（仮）. 開発版も SemVer で比べる
- D26（仮）. 入れ替えた収集アプリの 7 日の起点は、切り方を示した時
- D29（仮）. 携帯で、別の画面や確認の窓が前に出ている間は印を立てず、それが退いて行が見える状態で組み直した時か窓を閉じた時に立てる
- D28（仮）. PC で、最後の入力からの経過を読めない定期の時は、切り方のトーストを試みない
- D30（仮）. 更新の知らせは既存の通知を置き換えず、NFR-18 の本文コントラストにそろえる

全 Task の ledger の `Ruling:`（逐語）:
- 全 Task の ledger の `Ruling:`（逐語）:
- 全 Task の ledger に `Ruling:` の行は無い。

## Task

10 Task のうち、埋めた Task は 10 本。

残した項目:
- 11.1（人間の確認待ち（確認バッチが持つ））: 11.1 **本物の配布元で 1 回確かめる**: 配布元に最初のリリース（ベータ版か開発版）が出た後、本人の PC のサーバ（走らせ直しで更新係が足された形）・携帯・PC の収集アプリで「更新あり」か「新しい版なし」が出ることを見る。**暗号化した接続が OS の根で通ること（`NT SERVICE\ashiato-updater` を含む）を最初に見るのはここ**（試験と CI は loopback の平文の受け手で、本物の TLS を通らない。D12（仮）の反転条件。review/spec.md R12）。いまは配布元のリリースが 0 件（P1）なので、出るまで待つ
- 11.2（人間の確認待ち（確認バッチが持つ））: 11.2 **本人の PC の走らせ直し**: ST55 で入れた本人の PC で新しい版の zip のインストーラを走らせ直し、更新係が足され、DB と設定はそのままで、最後に「更新の確認: 有効」と切り方が出ることを見る

## 通した検証（`evidence.jsonl` の項目とコマンドごとの最新）

- 1.1 `bash -c 'scripts/quiet-run ct -- cargo test -p ashiato-update-check st57_choose_ && test "$(cargo test -p ashiato-update-check st57_choose_ -- --list 2>/dev/null | grep -c ": test$")" -ge 14'` → rc=0（PASS・head de47650）
- 1.1 `bash -c 'test "$(python3 -c "import json;d=json.load(open(\"docs/contracts/update-check-vectors.json\"));print(min(len(d[\"cases\"])//3,len(d[\"timing\"])))")" -ge 10'` → rc=0（PASS・head de47650）
- 2.1 `bash -c 'scripts/quiet-run ct -- cargo test -p ashiato-updater st57_ && test "$(cargo test -p ashiato-updater st57_ -- --list 2>/dev/null | grep -c ": test$")" -ge 12'` → rc=0（PASS・head 3bf2abe）
- 2.1 `cargo clippy -p ashiato-updater -p ashiato-winsvc --all-targets --target x86_64-pc-windows-gnu -- -D warnings` → rc=0（PASS・head 3bf2abe）
- 2.2 `tools/check-updater-deps.sh` → rc=0（PASS・head 3bf2abe）
- 2.2 `tools/check-updater-deps.sh --self-test` → rc=0（PASS・head 3bf2abe）
- 2.2 `tools/check-offsite.sh` → rc=0（PASS・head 3bf2abe）
- 2.2 `bash -c 'grep -q "check-updater-deps.sh" .github/workflows/ci.yml'` → rc=0（PASS・head 3bf2abe）
- 3.1 `bash -c 'set -a; [ -f .env ] && . ./.env; set +a; scripts/quiet-run ct -- cargo test -p ashiato-server st57_ && test "$(cargo test -p ashiato-server st57_ -- --list 2>/dev/null | grep -c ": test$")" -ge 14'` → rc=0（PASS・head 1c2fe19）
- 3.1 `./tools/check-openapi.sh` → rc=0（PASS・head 1c2fe19）
- 3.1 `tools/check-offsite.sh` → rc=0（PASS・head 1c2fe19）
- 4.1 `scripts/quiet-run e2e -- bash -c 'cd web && npm run test:e2e -- st57-update-notice.spec.ts'` → rc=0（PASS・head 49092ee）
- 4.1 `bash -c 'cd web && npx tsc --noEmit && npx vitest run'` → rc=0（PASS・head 49092ee）
- 5.1 `bash -c 'scripts/quiet-run ct -- cargo test -p ashiato-setup st57_ && test "$(cargo test -p ashiato-setup st57_ -- --list 2>/dev/null | grep -c ": test$")" -ge 10'` → rc=0（PASS・head 3848a75）
- 5.1 `cargo clippy -p ashiato-setup --all-targets --target x86_64-pc-windows-gnu -- -D warnings` → rc=0（PASS・head 3848a75）
- 6.1 `tools/release/check-artifacts.sh --self-test` → rc=0（PASS・head 95e898f）
- 6.1 `tools/release/package.sh --self-test` → rc=0（PASS・head 95e898f）
- 6.1 `tools/release/notices.sh --self-test` → rc=0（PASS・head 95e898f）
- 6.1 `tools/check-licenses.sh` → rc=0（PASS・head 95e898f）
- 6.1 `bash -c 'tools/check-vulns.sh --base "$(git merge-base HEAD origin/main)"'` → rc=0（PASS・head 95e898f）
- 7.1 `tools/release/update-check-install.sh --self-test` → rc=0（PASS・head 913f0e5）
- 7.1 `tools/release/install-check.sh --self-test` → rc=0（PASS・head 913f0e5）
- 7.1 `bash -c 'f="$1"; actionlint "$f" || docker run --rm -v "$PWD:/repo" -w /repo rhysd/actionlint:1.7.7 "$f"' _ .github/workflows/ci.yml` → rc=0（PASS・head 913f0e5）
- 8.1 `bash -c 'scripts/quiet-run ct -- cargo test -p ashiato-collector-windows st57_pc_ && test "$(cargo test -p ashiato-collector-windows st57_pc_ -- --list 2>/dev/null | grep -c ": test$")" -ge 23'` → rc=0（PASS・head 5475521）
- 8.1 `bash -c 'scripts/quiet-run ct -- cargo test -p ashiato-collector-windows'` → rc=0（PASS・head 5475521）
- 8.1 `cargo clippy -p ashiato-collector-windows --all-targets --target x86_64-pc-windows-gnu -- -D warnings` → rc=0（PASS・head 5475521）
- 8.1 `tools/check-no-time-server.sh` → rc=0（PASS・head 5475521）
- 8.1 `tools/check-licenses.sh` → rc=0（PASS・head 5475521）
- 8.1 `tools/check-updater-deps.sh` → rc=0（PASS・head 5475521）
- 8.2 `tools/release/check-artifacts.sh --self-test` → rc=0（PASS・head c9fde1a）
- 9.1 `bash -c 'scripts/quiet-run unit -- ./collector-android/gradlew -p collector-android :app:testDebugUnitTest --tests "*St57*" && test "$(cat collector-android/app/build/test-results/testDebugUnitTest/TEST-*St57*.xml | grep -c "<testcase ")" -ge 23'` → rc=0（PASS・head 92fee9e）
- 9.1 `tools/check-android-backup-rules.sh` → rc=0（PASS・head 92fee9e）
- 9.1 `tools/check-android-backup-rules.sh --self-test` → rc=0（PASS・head 92fee9e）
- 9.1 `tools/check-no-time-server.sh` → rc=0（PASS・head 92fee9e）
- 9.2 `scripts/quiet-run emu -- tools/android-emulator.sh` → rc=0（PASS・head 9f16c9f）
- 9.2 `bash -c 'test "$(cat collector-android/app/build/outputs/androidTest-results/connected/*/TEST-*.xml collector-android/app/build/outputs/androidTest-results/connected/TEST-*.xml 2>/dev/null | grep -c "<testcase [^>]*St57UpdateQueryTest")" -ge 2'` → rc=0（PASS・head 9f16c9f）
- 9.3 `tools/release/check-artifacts.sh --self-test` → rc=0（PASS・head 9f16c9f）
- 9.3 `bash -c 'tools/release/build.sh --self-test'` → rc=0（PASS・head 9f16c9f）
- 10.1 `bash -c 'for h in "update-check off" "ashiato-updater" "版の一覧"; do grep -q -- "$h" docs/install.md || exit 1; done && grep -q "update-check" docs/collector-setup.md && grep -q "update-status" docs/handoff/ST58.md && grep -q "更新係" docs/handoff/ST60.md'` → rc=0（PASS・head 7f3776b）
- 10.2 `python3 scripts/check_scenarios.py . st57-update-notice` → rc=0（PASS・head 0446620）
- 10.2 `bash -c 'set -a; [ -f .env ] && . ./.env; set +a; cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && scripts/quiet-run all -- cargo test --workspace'` → rc=0（PASS・head 0446620）
- 10.2 `bash -c 'tools/check-offsite.sh && tools/check-updater-deps.sh && tools/check-contract-version.sh && tools/check-licenses.sh && tools/check-openapi.sh'` → rc=0（PASS・head 0446620）

## レビュー（`review/code.md`）

- 指摘 45 件（fixed 45）
- 処置の re-review: 13 回。最後の判定が NOT ADDRESSED のもの: なし

## 未処置の申し送り

なし
