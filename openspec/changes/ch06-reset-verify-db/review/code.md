# CH06 コードのレビュー（ブランチ全体）

## final review（2b924da..63bd736）

席: `superpowers:requesting-code-review` の `code-reviewer.md`（review package `.superpowers/sdd/tasks/review-2b924da4..63bd7367.diff`）。
判定: **With fixes**（Critical なし・Important 2・Minor 11）。reviewer が手元で走らせたもの: `tools/check-verify-env-reset.sh --self-test` rc=0（CI に似せた環境でも rc=0）/
`openspec validate ch06-reset-verify-db --strict` valid / `python3 scripts/check_scenarios.py . ch06-reset-verify-db` OK / `tools/check-private.sh` rc=0。

Task ごとの ⚠️ の振り分け: T1・T2・T3①・T4 は解消（コード・証跡・P12 で確かめた）。T3②・T5・T6 は CI の結果待ち（R1）。T7① は R9、T7② は R2。

## R1. CI での本体の結果が 1 つも無い（CI だけで通る Scenario と 90 秒の反転条件が未確認）
- 成果物: .github/workflows/ci.yml:87-88 / tools/check-verify-env-reset.sh:95-100
- 根拠: 完了の判定 1 は「CI の rust job で走る」、Global Constraints は「CI と verify-run の証跡を合わせて 24 本が通る」。Scenario「CI でない main の checkout では、印つきで呼んでも行が残る」は worktree では SKIP。branch は未 push で CI の run が無い。reviewer はコードで CI の流れ（`ci-db-env.sh` → env-init 規則 1 → `CI=true` で reset → `env -u CI` で keep → `db-roles.sh` → `as_role`）を追い、破綻は見つけていない
- kind: technical
- 処置: rejected: コードの不具合ではない。reviewer が tools/check-verify-env-reset.sh:95-100 と .github/workflows/ci.yml:87-88 を追って破綻なし、`tools/check-verify-env-reset.sh --self-test` を CI に似せた環境で rc=0。CI の rust job の緑はグラフの publish → gate が見る関門で、本体の時間（D8 の 90 秒）とあわせて PR 本文に挙げる

## R2. 7.1 が 5 回中 1 回しか通っていない
- 成果物: openspec/changes/ch06-reset-verify-db/design.md:187-192（計測の記録）
- 根拠: 落ちたのは `stay_rebuild_chain_of_short_moves_settles`（3 秒の上限）ほかの時間・負荷に依る試験と、`stay_tests::fresh_db` の `DROP DATABASE … WITH (FORCE)` の権限。`crates/` は変えていない（7.2）。CH06 が入るとどの verify-run も空の DB から始まり、下流がこの揺れを踏み続ける
- kind: technical
- 処置: rejected: CH06 の不具合ではない。完了の判定 2 の証跡は evidence.jsonl の 7.1 PASS（木 4c1f3ac6、rc=0、754 秒）。落ちた試験は design.md:189-191 のとおり単独で通り、`crates/` は 7.2 の `git diff origin/main...HEAD -- crates …` が空。揺れる試験は CH06 の外の不具合として、PR 本文の「未処置の申し送り」に別の change の起票を挙げる

## R3. `cut_line` の `psql -F ' '` は application_name の空白・カンマで出所の表示がずれる
- 成果物: tools/verify-env.sh（`cut_line`）
- 根拠: 空白区切りなので `PostgreSQL JDBC Driver` などで `$1`/`$2` がずれる（合計は正しい）。T4 F3
- kind: technical
- 処置: fixed 4.1 —— tools/verify-env.sh:41,48-49 を psql `-F $'\t'` と awk `-F'\t'` にし、`-F', '` の 2 段目をやめた

## R4. `grep '切った接続: 1'` が「10」にも当たる
- 成果物: tools/check-verify-env-reset.sh:147 / :175 / :189
- 根拠: 境界が無い。`切った接続: 0` も同じ。T5 F3
- kind: technical
- 処置: fixed 5.1 —— tools/check-verify-env-reset.sh:153 を `grep -E '切った接続: 1([^0-9]|$)'`、0 の側は helper `cut0`（:56-58）。表示をわざと `切った接続: 10）` にすると本体 rc=1 を確かめて戻した

## R5. cleanup が `pg_sleep(600)` の backend を残す
- 成果物: tools/check-verify-env-reset.sh:35
- 根拠: kill は `docker compose exec` の CLI だけを止める。次の作り直しで消えるので害は小さい。T5 F4
- kind: technical
- 処置: fixed 5.1 —— tools/check-verify-env-reset.sh:36-38 の cleanup で `application_name='ch06-holder'` の backend を `pg_terminate_backend`（`|| true`）

## R6. self-test の `init` が env-init の stderr を捨て、落ちたとき場合の名前を出さない
- 成果物: tools/check-verify-env-reset.sh:265
- 根拠: `set -e` で NG の行も理由も無く終わる。各失敗が場合を名指しする要求に反する。T2 F3
- kind: technical
- 処置: fixed 2.2 —— tools/check-verify-env-reset.sh:271-277 で env-init の stderr を取り、直前の Scenario の名前つきで `self-test NG` を出す。env-init に `exit 3` を差して rc=1 と場合の名前が出るのを確かめて戻した

## R7. body が `return 1` すると ERR trap が誤解を招く 1 行を出す
- 成果物: tools/check-verify-env-reset.sh:232
- 根拠: `"") body; exit $?` の位置で ERR trap が鳴り「NG [終わりの DB]: 232 行目のコマンドが失敗した」が余分に出る
- kind: technical
- 処置: fixed 5.1 —— fix round 1 の `body || exit $?` は `||` の左で body の errexit と ERR trap を殺していた（re-review の Important。`bash -c 'set -Eeuo pipefail; f(){ trap … ERR; false; echo continued; }; f || exit $?'` で `continued` を再現）。呼び出しを `body; exit $?` に戻し、tools/check-verify-env-reset.sh:238 の `return 1` の前で `trap - ERR` にした。本体 rc=0（52 秒）

## R8. env-init の awk が末尾に改行の無い `.env` に改行を 1 つ足す
- 成果物: tools/env-init.sh:41-45
- 根拠: 「ほかの行は 1 文字も変えない」の字面から 1 byte ずれる。2 回目以降は冪等。T1 F2
- kind: technical
- 処置: fixed 2.1 —— tools/env-init.sh:41-46 の awk を、元の末尾が改行のときだけ末尾に改行を付ける形にした（`COMPOSE_FILE` を末尾に足すときは区切りの改行が付く）

## R9. 7.3 の `openspec validate --strict` の証跡が evidence.jsonl に無い
- 成果物: openspec/changes/ch06-reset-verify-db/evidence.jsonl
- 根拠: reviewer の手元では valid。記録だけが抜けている。T7 F2
- kind: technical
- 処置: rejected: 記録の道具の制約。`scripts/verify-run 7.3` は rc=0 で、check_scenarios・check-private・check-ports・check-verify-env-reset の self-test の 4 本が evidence.jsonl に PASS で入った。`openspec validate` は harness2 の scripts/evidence.py:63 の `CMD_LIKE` に openspec が無く、本文の検証コマンドとして拾われない（`--command` でも rc=2）。手で `openspec validate ch06-reset-verify-db --strict` は rc=0

## R10. `clear_blocking_network` は自分の network が無いときだけ動く（移行の時期の端の場合）
- 成果物: tools/verify-env.sh（`clear_blocking_network`）
- 根拠: 古い既定の subnet の自分の network がある木で、移る先の subnet を消えた木の空の network が塞いでいると、up が自分の network を消した後 `Pool overlaps` で 1 回 BLOCKED_INFRA。次の呼び出しで直る
- kind: technical
- 処置: rejected: 挙動は直さず、注記で受け入れる。移行の時期に一度だけ起きる端の場合で、次の呼び出しでは自分の network が無いので確かめが動いて直る（tools/verify-env.sh:74-76 に注記を足した）。直すには up の前に自分の network を消す経路が要り、D5 の「塞いでいる空の network だけを消す」の範囲を広げる

## R11. `cut_line` は psql がどんな理由で落ちても「DB は止まっていた」と出す
- 成果物: tools/verify-env.sh（`cut_line`）
- 根拠: 役割が無い場合なども同じ表示になり、実際と違うことがある
- kind: technical
- 処置: rejected: 表示は design.md:68 と tasks.md:76 の書式 `切った接続: 0（DB は止まっていた）`（D3。「止まっていた・問えなかった」の両方をこの文言で出すと決めている）のとおり。fix round 1 で文言を変えたが、計画との食い違い（re-review の Minor）になるので戻した。tools/verify-env.sh:45 に「見分けない」旨の注記だけを残した

## R12. docs/flow-gates.md の走る場所の欄が「手元では self-test しか走らない」と読める
- 成果物: docs/flow-gates.md:15
- 根拠: 「ローカル（`--self-test` は本体を除く）」。本体は手元でも走る。T6 F1
- kind: technical
- 処置: fixed 6.2 —— docs/flow-gates.md:15 を「CI の `rust`、手元（本体と `--self-test`。`--self-test` は docker の daemon に触らない）」にした

## R13. check-ports.sh の比べる木の並びが生きている木の全部でない
- 成果物: tools/check-ports.sh:49
- 根拠: demo-ch01 の代わりに demo-ch09 を並べ、生きている demo-st06-t7 が漏れる。subnet は port の単射なので実害は無い。T1 F1
- kind: technical
- 処置: rejected: specs/verify-environment/spec.md:107-109 の Scenario が 7 つの木を名指ししていて、demo-st06-t7 を足すと Scenario とずれる。subnet は port の単射なので被らない。tools/check-ports.sh:48-49 に demo-ch09 が demo-ch01 の番号を継ぐ旨の注記を足した

## final review の fix（67477ca..）と scoped re-review

fix は 1 回（77534268、R3〜R13 を 1 本の dispatch で）。scoped re-review（`.superpowers/sdd/tasks/review-67477cae..77534268.diff`）の判定:
R3・R4・R5・R6・R8・R11・R12 は ADDRESSED、R9・R10・R13 は注記・記録の制約止まり（上の rejected）。
新しい壊れ: Important 1（R7 の直し方が body の `set -e` と ERR trap を殺す）・Minor 1（R11 の文言が design D3 / tasks 4.1 と食い違う）。
第 2 の fix wave は出さず、controller が R7 を `trap - ERR` の形に直し、R11 の文言を計画の書式に戻した（上の処置）。
その後: `tools/check-verify-env-reset.sh --self-test` rc=0 / `scripts/quiet-run reset-final2 -- tools/check-verify-env-reset.sh` rc=0（52 秒）。
