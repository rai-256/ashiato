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

## R4. `grep '切った接続: 1'` が「10」にも当たる
- 成果物: tools/check-verify-env-reset.sh:147 / :175 / :189
- 根拠: 境界が無い。`切った接続: 0` も同じ。T5 F3
- kind: technical

## R5. cleanup が `pg_sleep(600)` の backend を残す
- 成果物: tools/check-verify-env-reset.sh:35
- 根拠: kill は `docker compose exec` の CLI だけを止める。次の作り直しで消えるので害は小さい。T5 F4
- kind: technical

## R6. self-test の `init` が env-init の stderr を捨て、落ちたとき場合の名前を出さない
- 成果物: tools/check-verify-env-reset.sh:265
- 根拠: `set -e` で NG の行も理由も無く終わる。各失敗が場合を名指しする要求に反する。T2 F3
- kind: technical

## R7. body が `return 1` すると ERR trap が誤解を招く 1 行を出す
- 成果物: tools/check-verify-env-reset.sh:232
- 根拠: `"") body; exit $?` の位置で ERR trap が鳴り「NG [終わりの DB]: 232 行目のコマンドが失敗した」が余分に出る
- kind: technical

## R8. env-init の awk が末尾に改行の無い `.env` に改行を 1 つ足す
- 成果物: tools/env-init.sh:41-45
- 根拠: 「ほかの行は 1 文字も変えない」の字面から 1 byte ずれる。2 回目以降は冪等。T1 F2
- kind: technical

## R9. 7.3 の `openspec validate --strict` の証跡が evidence.jsonl に無い
- 成果物: openspec/changes/ch06-reset-verify-db/evidence.jsonl
- 根拠: reviewer の手元では valid。記録だけが抜けている。T7 F2
- kind: technical

## R10. `clear_blocking_network` は自分の network が無いときだけ動く（移行の時期の端の場合）
- 成果物: tools/verify-env.sh（`clear_blocking_network`）
- 根拠: 古い既定の subnet の自分の network がある木で、移る先の subnet を消えた木の空の network が塞いでいると、up が自分の network を消した後 `Pool overlaps` で 1 回 BLOCKED_INFRA。次の呼び出しで直る
- kind: technical

## R11. `cut_line` は psql がどんな理由で落ちても「DB は止まっていた」と出す
- 成果物: tools/verify-env.sh（`cut_line`）
- 根拠: 役割が無い場合なども同じ表示になり、実際と違うことがある
- kind: technical

## R12. docs/flow-gates.md の走る場所の欄が「手元では self-test しか走らない」と読める
- 成果物: docs/flow-gates.md:15
- 根拠: 「ローカル（`--self-test` は本体を除く）」。本体は手元でも走る。T6 F1
- kind: technical

## R13. check-ports.sh の比べる木の並びが生きている木の全部でない
- 成果物: tools/check-ports.sh:49
- 根拠: demo-ch01 の代わりに demo-ch09 を並べ、生きている demo-st06-t7 が漏れる。subnet は port の単射なので実害は無い。T1 F1
- kind: technical
