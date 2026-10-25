# CH06 final review fix round 1（67477cae..77534268）

fix agent の報告（要約・逐語に近い）:
- R3 tools/verify-env.sh:41,48-49: psql -F $'\t'、awk 2 つも -F'\t'。2 段目の -F', ' の awk をやめ 1 段目で最大 10 件と末尾 … まで出す
- R4 tools/check-verify-env-reset.sh:153,56-58,181,195: `grep -E '切った接続: 1([^0-9]|$)'`、0 の側は helper cut0。壊し確かめ: 表示を `切った接続: 10）`/`切った接続: 1$n。` にすると本体 rc=1（5./6. NG）。戻し済み
- R5 同:36-38: cleanup で pg_terminate_backend（application_name='ch06-holder'）|| true。本体後 ch06-holder 0
- R6 同:271-277: init が env-init の stderr を一時ファイルに取り、失敗時に直前の Scenario 名を awk で引いて `self-test NG: [<Scenario>] …` → self-test FAILED。awk は `Scenario[:]`（check_scenarios の誤検出回避）。壊し確かめ: env-init に exit 3 で rc=1、場合名つきで出た。戻し済み
- R7 同:243: `"") body || exit $?; exit 0 ;;`
- R8 tools/env-init.sh:41-46: 末尾改行の無い .env に改行を足さない形に直した（COMPOSE_FILE を末尾に足すときだけ区切りの改行が付く）。一時の木で 5 通り、2 回目以降同じ中身
- R9: scripts/verify-run 7.3 rc=0、4 本が evidence.jsonl に PASS。openspec validate は harness2 scripts/evidence.py:63 の CMD_LIKE に openspec が無く記録できない（--command でも rc=2）。手で rc=0
- R10 tools/verify-env.sh:74-76: clear_blocking_network の注記に移行時期の端の場合を追記。挙動不変
- R11 tools/verify-env.sh:45-46: `切った接続: 0（DB に繋げなかった）`、見分けない旨の注記
- R12 docs/flow-gates.md:15: 「CI の `rust`、手元（本体と `--self-test`。`--self-test` は docker の daemon に触らない）」
- R13 tools/check-ports.sh:48-49: 注記のみ（spec.md:107-109 の Scenario が 7 つの木を名指しするため並びは不変）

確かめ（最終のコード）: check-verify-env-reset.sh --self-test 0 / check-ports.sh --self-test 0 / check-private.sh 0 /
check_scenarios.py 0 / quiet-run 本体 0（64 秒、3. は worktree で SKIP）/ verify-run 7.3 0 / 変えないファイルの diff 空 0 / openspec validate --strict（手）0

## Rulings（final の controller）

- Ruling: R2 —— 7.1 の 5 回中 1 回の PASS で完了の判定 2 を満たすとみなした（落ちた試験は時間・負荷と `fresh_db` の `FORCE` の権限で、`crates/` は不変・単独で通る）。違っていたら: 揺れる試験を直す change を CH06 の前に通す必要があり、CH06 の merge が遅れる。揺れる試験の起票を PR 本文の申し送りに挙げる
- Ruling: R7 —— re-review の Important（`body || exit $?` が errexit と ERR trap を殺す）を、第 2 の fix wave でなく controller が直した（`trap - ERR` の後に `return 1`、呼び出しは `body; exit $?`）。この直しはレビューの席を通っていない（bash の再現 2 本と本体 rc=0 で確かめた。code-verify が続けて見る）。違っていたら: 本体の失敗が trap の行を出さないか、通らない回が OK になる
