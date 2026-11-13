# ST57 の処置の fix report（settle が追記する）

## 2026-11-13 — R44・R45 の再処置

- R44/R45: cleanup commit（02f263217）で削除された fix report を復元した。R43 のうち PC 更新確認と更新係インストール台本の修正範囲を現在の HEAD で再検証し、検査名・出力・rc を残す。
- 検査: `scripts/quiet-run st57-r44-r45-pc -- cargo test -p ashiato-collector-windows st57_pc_`
  - 出力: `test result: ok. 25 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.17s`（rc=0、ログ: `.superpowers/sdd/logs/20261113-143235-st57-r44-r45-pc.log`）
- 検査: `tools/release/update-check-install.sh --self-test`
  - 出力: `OK: update-check-install の self-test`（rc=0）

## 2026-11-13 — R46 の処置

- R46: e340a72 の `prepare_repair_fixture` と自己検査の追加を含む現在の HEAD で、Task 7.1 を `scripts/verify-run 7.1` で再実行した。更新係の自己検査の PASS 記録を evidence に追加した。
- 検査: `tools/release/update-check-install.sh --self-test`
  - 出力: `OK: update-check-install の self-test`（rc=0、ログ: `.superpowers/sdd/logs/20261113-160556-evidence-7.1-0-f93d10-vkq1bu3lo.log`）
- 併走した Task 7.1 の検査: `tools/release/install-check.sh --self-test`、CI workflow の actionlint
  - 出力: いずれも rc=0（対応する PASS 記録を `evidence.jsonl` に追記）
