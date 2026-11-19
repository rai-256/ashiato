<!-- 処置の判定: re-review -->

## /code-review（0d55f06..a0c7a13）

見た: Claude Code の組み込みの `/code-review`（9 件）。写しは scripts/code_review_record.py

## R1. The check that "appending to an open issue does not add an assignee" cannot catch a regression on the report-unfiled path. Outside the GH_FAIL_ASSIGNEE case the fake gh writes `$*` to its log without escaping newlines, so a 2-line body puts `--assignee` on the next line, which does not start with `issue comment`. Fix: escape newlines in the log on every call (as the GH_FAIL_ASSIGNEE case already does), or grep for `--assignee` on every line after the comment line.
- 成果物: tools/check-vulns.sh:571
- 根拠: Someone later adds `--assignee owner` to the `issue comment` call in report_unfiled. The log reads `issue comment 12 --body 夜: 2026-..` followed by `run: https://example.invalid/run/4 --assignee owner-x`. The check `! grep -Eq '^issue (comment|edit) .*--assignee'` does not match, so the self-test stays green while the spec's Scenario is broken.（確かさ 2）
- kind: technical
- 処置: fixed 2.3 —— 偽の `gh` が全呼び出しで改行を `\\n` として 1 行に記録するようにし、追記に `--assignee` を足す回帰を既存の `^issue (comment|edit).*--assignee` の検査が捕まえる形にした
- 提案: （/code-review は提案を持たない。settle が直し方を決める）

## R2. create_issue retries without the assignee after any non-zero exit from the first `issue create`, not only after an assignee error. Timeouts, 5xx errors and secondary rate limits also trigger the retry. Fix: limit the retry to failures whose stderr shows an assignee problem, or check by title that the issue does not exist before retrying.
- 成果物: tools/check-vulns-issues.py:125
- 根拠: The night's `gh issue create --assignee owner` creates the issue on the server, then gh exits non-zero on a network timeout. The retry without the assignee creates a second issue with the same title (and the copy has no assignee). This happens on both the file-issues and report-unfiled paths.（確かさ 2）
- kind: technical
- 処置: rejected: specs の Requirement と design D5 は「担当者つきの `issue create` が 0 でない rc で終わったら」理由を問わず担当者なしで 1 回だけ立て直す、と明記している。タイムアウトや 5xx を除外すると正典に反するため、この change では狭めない
- 提案: （/code-review は提案を持たない。settle が直し方を決める）

## R3. When file-issues hangs and hits the job's `timeout-minutes: 20`, the job is cancelled. Then `failure()` is false and the report step never runs. A hanging gh (network or GitHub outage) is a plausible way for the notification to fail, and it leaves only a red run, the same 9-night silence this change set out to fix. Fix: put a short `timeout-minutes` on the file step so it ends as a failure, or set the report step's condition to cover cancellation too.
- 成果物: .github/workflows/vulns-nightly.yml:71
- 根拠: `gh issue list` stays unanswered and the job is cancelled at 20 minutes. The step condition `failure() && steps.file.outcome == 'failure'` is false, so no notification issue is created.（確かさ 2）
- kind: technical
- 処置: fixed 2.2 —— `file-issues` の段に 5 分の `timeout-minutes` を置いた。job 全体の 20 分の打ち切りでなく段の failure になるため、既存の `failure() && steps.file.outcome == 'failure'` で知らせの段へ進む
- 提案: （/code-review は提案を持たない。settle が直し方を決める）

## R4. download-artifact has `continue-on-error: true`. If the download fails, the findings folder is missing, the file step skips file-issues and succeeds, and the report step does not run. That night's findings are lost without any notification. Fix: treat a download failure as a file-step failure too, for example by adding an `id` to the download step and a report-step condition that covers its outcome.
- 成果物: .github/workflows/vulns-nightly.yml:59
- 根拠: An artifact service outage makes the download fail while `vulns-findings` exists. file-issues is not called, report-unfiled is not called, and no issue is created.（確かさ 2）
- kind: technical
- 処置: fixed 2.2 —— artifact 取得の段に `id: download` を付け、`continue-on-error` で失敗した outcome を file 段へ渡して rc=2 にする。これにより既存の file 段の failure 条件で知らせる
- 提案: （/code-review は提案を持たない。settle が直し方を決める）

## R5. The check that skips file-issues when there is no material only confirms that the string `find "$RUNNER_TEMP/findings" -type f -print -quit` appears somewhere in the run body. It does not check the branch itself. Commenting out the `if` line or changing it to `if true || [ -d ...`, so file-issues is called every night, still passes. Fix: compare the whole `if [ -d ... ] && find ... | grep -q .; then` line for an exact match at the start of a line (ignoring indentation), and add a self-test mutation for it.
- 成果物: tools/check-vulns-wiring.sh:167
- 根拠: The line is rewritten to `if true || [ -d "$RUNNER_TEMP/findings" ] && find "$RUNNER_TEMP/findings" -type f -print -quit | grep -q .; then`. file-issues runs with rc=2 on nights without material, and then report-unfiled runs too, which breaks the canon's rule "issue を立てない" (create no issue on rc=2 nights). The wiring check still passes.（確かさ 2）
- kind: technical
- 処置: fixed 2.2 —— 配線検査は file 段の run 内に、材料の置き場とファイルを両方確かめる正確な guard 行があることを照合し、`if true; then` への自己試験の変異も落とす
- 提案: （/code-review は提案を持たない。settle が直し方を決める）

## R6. The body comparison takes the date from Python's `datetime.now(timezone.utc)` and later from bash's `date -u +%F`. If the self-test runs across UTC midnight (09:00 JST), the two dates differ and the test fails spuriously. Fix: let tests inject the date through an environment variable, or compare only the shape of the first line, e.g. `^夜: [0-9]{4}-[0-9]{2}-[0-9]{2}$`.
- 成果物: tools/check-vulns.sh:539
- 根拠: report-unfiled runs at 23:59:59 UTC and diff runs at 00:00:00, giving `夜: 2026-11-20` versus `夜: 2026-11-21`. diff fails and the self-test returns rc=1.（確かさ 2）
- kind: technical
- 処置: fixed 2.1 —— `CHECK_VULNS_TEST_UTC_DATE` により自己試験だけ UTC 日付を固定し、作成・追記とも同じ 2 行本文を比較する。実行時は従来どおり現在の UTC 日付を使う
- 提案: （/code-review は提案を持たない。settle が直し方を決める）

## R7. report_unfiled reads open issues across the whole repository with no label filter and an upper limit of 1000. Unlike file_issues it does not check the count, so once open issues pass 1000 (harness 依頼 and 違和感 issues included) it can miss the existing notification issue and create a duplicate every night. The design accepts this risk, but the count to watch is the whole repository's open issues, not just the 脆弱性-labelled ones. Fix: narrow the list with `--search` on the title (`in:title`), or note in the design that this is a whole-repository count.
- 成果物: tools/check-vulns-issues.py:207
- 根拠: With 1000 or more open issues, `--limit 1000` does not return the notification issue. Every failed night creates a new copy of `脆弱性: 毎晩の知らせを issue にできなかった`.（確かさ 2）
- kind: technical
- 処置: fixed 2.1 —— ラベルを使わず、決まった題と `in:title` の検索で open issue を絞ってから読むようにした。1000 件の上限は同じ題の候補だけに掛かる
- 提案: （/code-review は提案を持たない。settle が直し方を決める）

## R8. load_material runs outside the try block. A broken material file (JSONDecodeError) or a row missing a key (KeyError) ends file-issues with a Python traceback and rc=1, not rc=2. The spec says failures end with an `error:` line and rc=2, and the traceback in the run log is harder to read. Fix: catch (json.JSONDecodeError, KeyError, ValueError) in file_issues, print `error: issue を記録できなかった（…）` and return 2.
- 成果物: tools/check-vulns-issues.py:152
- 根拠: vulnerabilities.json is truncated mid-write, so json.loads raises. The script exits rc=1 with a traceback; report-unfiled does run, but the message format and rc differ from the contract.（確かさ 2）
- kind: technical
- 処置: fixed 2.1 —— 材料の読み込み・題への束ねまでを `file_issues` の例外処理に含め、壊れた JSON が `error: issue を記録できなかった` と rc=2 で終わり traceback を出さない自己試験を足した
- 提案: （/code-review は提案を持たない。settle が直し方を決める）

## R9. One commit message on this branch is in English: `8ebce8bbe fix: strengthen nightly issue retry checks`. Fix: reword it in Japanese when the branch is rebased or squashed.
- 成果物: tools/check-vulns.sh:1
- 根拠: /home/yosis/dev/ashiato2-ch23/CLAUDE.md, section 'git': 「コミットは `feat:` / `fix:` / `docs:` / `chore:` + 日本語 1 行、本文に「なぜ」」 (the type prefix plus one line in Japanese, with the why in the body). Commit 8ebce8bbe has an English subject line.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。settle が直し方を決める）
