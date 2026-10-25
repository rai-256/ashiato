# CH07 実装タスク — 検証の準備が作り直した DB を宣言する

読む順: `deep.md`（問い 0 件。C1〜C9 の要約）→ このファイル → `specs/verify-environment/spec.md` → `design.md` → `premises.md`（P1〜P9 と C1〜C9）→ `docs/changes/CH07.md` →
CH06 の `tools/verify-env.sh`・`tools/check-verify-env-reset.sh`（CH06 の実装の上に積む。`requires: [CH06]`）→ `CLAUDE.md`。

検証は各タスクの本文に書いてある。「動いた」ではなくコマンドと終了コードで判定する。
`scripts/verify-run` は検証コマンドの前に毎回 `tools/verify-env.sh` を `HARNESS_VERIFY=1` で呼ぶ（この worktree の DB は検証のたびに空になる。CH06）。

## Global Constraints

- **テストには `Scenario: <名前>` の印を置く。** bash は `echo "Scenario: …"`（その場合を確かめる直前に出す）。
  `scripts/check_scenarios.py` が spec の全 Scenario と突き合わせ、印の無い Scenario を FAIL にする。印の名前は spec の `#### Scenario:` と**一字一句合わせる**
- **この change の Scenario は 12 本**（`verify-environment` の ADDED 2 つ: 5・7 本）。置き場は `tools/check-verify-env-reset.sh` の 2 つの入口:
  `--self-test` に 3 本（「判定だけを出すときは、宣言を出さない」「git の木でなければ、印つきでも作り直さない判定になる」「ハーネスの点検が、作り直して宣言する準備と認める」）、本体に 9 本。**12 本とも、このファイルのどこかの Task が名指しする**。
  本体のうち 1 本（CI でない main の checkout）は CI だけ、`--self-test` のうち 1 本（ハーネスの点検）は `.harness2` のある木（worktree）だけで通る（design D3・D4）。CI と `verify-run` の証跡を合わせて全部が通る
- **宣言の行は `HARNESS_RESET: testdb` そのもの。** `tools/verify-env.sh` 自身の `echo` で、成功の出口（待ちのループの `exit 0`）の直前に 1 回だけ、標準出力に出す。
  出すのは `reset_db` が「DB のコンテナが無かった」か「控えた volume を 1 つ以上消した」ときだけ（design D2）。`HARNESS_RESET: none` は出さない（C3）
- **CH06 の振る舞いを変えない**（C4。「壊してはいけないもの」）: 判定（`plan`）・`--plan` の出力・作り直しの手順と対象・切った接続の 1 行の形と出し先（標準エラー）・作り直せないときの rc・`keep:` の標準エラーの 1 行。
  CH06 の 24 本の Scenario の確かめ（`tools/check-verify-env-reset.sh` の既存の段と `tools/check-ports.sh --self-test`）を消さない・弱めない
- **変えないもの**: `crates/`（試験のコードを含む）・`migrations/`・`tools/stack.sh`・`tools/smoke.sh`・`tools/check-immutable.sh`・`tools/check-panic-log.sh`・`tools/dev.sh`・`tools/record-env.sh`・`tools/db-roles.sh`・`tools/env-init.sh`・`tools/ports.sh`・`docker-compose.yml`・`.github/workflows/`
- **追跡するファイルに点区切りの私設 IP の 4 つ組を書かない**（`tools/check-private.sh` が落とす。CH06 D5）
- 走っている Story・change のブランチには触らない。CH06 の tasks へ差し戻さない（C6）

## Task 1: 宣言の行

- [ ] 1.1 `tools/verify-env.sh` の `reset_db` に、DB のコンテナが無くて戻るときと、控えた volume を 1 つ以上消し終えたときにだけ立つ変数（例: `reset_done=1`）を足す（volume が 0 個なら立てない）。
  待ちのループの中で `tools/db-roles.sh` が通った後、`exit 0` の直前に、変数が立っていれば `echo "HARNESS_RESET: testdb"` を 1 回出す（標準出力。`>/dev/null`・`>&2`・`$(…)` の中に置かない）。
  冒頭の注記に「作り直したら `HARNESS_RESET: testdb` を標準出力に 1 行（ハーネスの約束。ch07 design D2）」を足す。CH06 が既に宣言の行を足していたら、その行をこの条件に結び直し、字は 1 つにする（design D1）。
  検証: `bash -c 'out=$(HARNESS_VERIFY=1 tools/verify-env.sh 2>/dev/null) && [ "$(grep -cx "HARNESS_RESET: testdb" <<<"$out")" = 1 ]'` rc=0（worktree で走る。標準エラーに出ないことは 3.1 の 5. が見る）/
  `bash -c '[ -z "$(env -u HARNESS_VERIFY tools/verify-env.sh 2>&1 | grep "HARNESS_RESET:")" ]'` rc=0 /
  `bash -c '[ "$(HARNESS_VERIFY=1 tools/verify-env.sh --plan)" = reset ]'` rc=0 /
  `bash -c '[ "$(grep -v "^\s*#" tools/verify-env.sh | grep -c "echo \"HARNESS_RESET: testdb\"")" = 1 ]'` rc=0

## Task 2: `--self-test` の確かめ

- [ ] 2.1 `tools/check-verify-env-reset.sh` の `--self-test` に、`echo "Scenario: 判定だけを出すときは、宣言を出さない"` の印を置き、`demo` と `demo-st05`・`demo-verify`・`demo-ch01` で、
  印あり・なしと `CI=true` あり・なしの `--plan` の出力が、どれも 1 行で `reset` か `keep:` で始まり、`HARNESS_RESET:` で始まる行を含まないことを確かめる（design D4）。
  同じ入口で、`echo "Scenario: git の木でなければ、印つきでも作り直さない判定になる"` の印を置き、`git init` しない一時の場所（`$tmp` の下）に `tools/verify-env.sh` を写して、
  `CI` を落とし `HARNESS_VERIFY=1` で `--plan` を呼ぶと、出力が `keep:` で始まる 1 行で `HARNESS_RESET:` を含まないことを確かめる（design D4）。
  同じ入口で、`tools/verify-env.sh` の注記でない行に `echo "HARNESS_RESET: testdb"` がちょうど 1 つあり、その行が `>/dev/null`・`>&2`・`$(` を含まないことを確かめる（Scenario の印は付けない。CH06 の旗の確かめと同じ守り）。
  検証: `tools/check-verify-env-reset.sh --self-test` rc=0 /
  `bash -c 'POSTGRES_PASSWORD=a OWNER_DB_PASSWORD=b APP_DB_PASSWORD=c ASHIATO_DB_SUBNET=x CI=true tools/check-verify-env-reset.sh --self-test'` rc=0（CI と同じ環境）/
  `bash -c 'grep -qF "echo \"Scenario: 判定だけを出すときは、宣言を出さない\"" tools/check-verify-env-reset.sh && grep -qF "echo \"Scenario: git の木でなければ、印つきでも作り直さない判定になる\"" tools/check-verify-env-reset.sh'` rc=0
- [ ] 2.2 同じ `--self-test` に、`echo "Scenario: ハーネスの点検が、作り直して宣言する準備と認める"` の印を置き、`.harness2/scripts/init.py` があって `check_verify_env` を含むなら、repo の根で
  `python3 .harness2/scripts/init.py --check` を走らせ（rc は見ない）、出力に `[人間] tools/verify-env.sh` を含む行が無く、`[ok] tools/verify-env.sh` を含む行がちょうど 1 つあることを確かめる。
  無ければ `SKIP …（.harness2 が無い木。worktree の verify-run が通す）` の 1 行を出して飛ばす（design D4。CI には `.harness2` が無い）。
  検証: `bash -c 'out=$(tools/check-verify-env-reset.sh --self-test) && ! grep -q "^SKIP" <<<"$out"'` rc=0（worktree で走る。`.harness2` があるので飛ばさない）/
  `bash -c 'grep -qF "echo \"Scenario: ハーネスの点検が、作り直して宣言する準備と認める\"" tools/check-verify-env-reset.sh'` rc=0

## Task 3: 検査の本体 —— 段ごとの宣言の数と、2 つの新しい段

- [ ] 3.1 `tools/check-verify-env-reset.sh` の本体に、`call` の後の宣言の数を見る関数（`$tmp/out` の `HARNESS_RESET: testdb` だけの行の数と、`$tmp/out`・`$tmp/err` の `^HARNESS_RESET:` の行の数）を足し、
  design D3 の表のとおり既存の段に確かめを足す。印を置く: 3. に `echo "Scenario: 印が無ければ、宣言を出さない"`、3. の CI だけの段に `echo "Scenario: CI でない main の checkout では、印つきでも宣言を出さない"`、
  4. に `echo "Scenario: 作り直せずに非 0 で終わるときは、宣言を出さない"`、5. に `echo "Scenario: 溜まった DB を作り直したら、宣言を 1 行出す"`（標準エラーに 0 行も見る）、
  6. に `echo "Scenario: 2 回続けて作り直しても、どちらも宣言を 1 行出す"`、7. に `echo "Scenario: DB のコンテナが無くても、立てたら宣言を 1 行出す"`。
  1. と 8.（空の network を消して立てる呼び出しと最後の立て直しで 1 行、rc≠0 の 2 回で 0 行）にも数の確かめを足す（印は付けない）。
  検証: `tools/check-verify-env-reset.sh` rc=0（worktree で走る）/ `bash -c '[ "$(grep -c "echo \"Scenario: " tools/check-verify-env-reset.sh)" -ge 32 ]'` rc=0（CH06 の 23 本・Task 2 の 3 本・この段の 6 本）
- [ ] 3.2 7. の後・8. の前に段を 3 つ足す（design D3）。
  **9. 止まっていた DB**: `docker compose stop db` → `echo "Scenario: DB のコンテナが止まっていても、作り直したら宣言を 1 行出す"` → 印つきで呼び、rc=0・出力に `切った接続: 0（DB は止まっていた）`・標準出力に宣言 1 行。
  **10. volume を持たない DB**: `$tmp` に service `db` の `tmpfs: /var/lib/postgresql/data` だけを書いた compose のファイルを置き、DB のコンテナと volume を消してから、環境変数 `COMPOSE_FILE` に今の木の値（`.env` の `COMPOSE_FILE`、無ければ `docker-compose.yml`）とそのファイルを `:` で並べて `docker compose up -d --wait db` →
  DB のコンテナの volume が 0 個であることを確かめる → `echo "Scenario: volume を持たない DB のコンテナでは、作り直しても宣言を出さない"` → 同じ `COMPOSE_FILE` で印つきで呼び、rc=0・出力に `切った接続: ` の行・宣言 0 行・volume が 0 個のまま。
  戻す: 同じ `COMPOSE_FILE` で `docker compose rm -s -f db` → `COMPOSE_FILE` を渡さずに印つきで呼び、rc=0・宣言 1 行・volume がちょうど 1 つ。
  **11. 途中で落ちる**: volume の名前を控え → `echo "Scenario: 作り直しの途中で落ちたら、宣言を出さない"` → `ENV_FILE=$tmp/<無い名前>` を渡して印つきで呼び、rc≠0・宣言 0 行（標準出力にも標準エラーにも）・控えた volume が無い。
  戻す: `ENV_FILE` を渡さずに印つきで呼び、rc=0・宣言 1 行。
  `tools/db-roles.sh` が `.env` を読み直すことで 10. の印つきの呼び出しが落ちたら、design D3 の反転条件のとおり override の形に替え、`design.md` の D3 を書き直す。
  検証: `tools/check-verify-env-reset.sh` rc=0（worktree で走る）/
  `bash -c '[ "$(grep -c "echo \"Scenario: " tools/check-verify-env-reset.sh)" -ge 35 ]'` rc=0 /
  `bash -c 'grep -qF "echo \"Scenario: DB のコンテナが止まっていても、作り直したら宣言を 1 行出す\"" tools/check-verify-env-reset.sh && grep -qF "echo \"Scenario: volume を持たない DB のコンテナでは、作り直しても宣言を出さない\"" tools/check-verify-env-reset.sh && grep -qF "echo \"Scenario: 作り直しの途中で落ちたら、宣言を出さない\"" tools/check-verify-env-reset.sh'` rc=0
  （10. と 11. の戻しで DB が volume を 1 つ持つ形に戻ることは、検査の本体の中で確かめる。verify-run は検証のたびに準備で DB を作り直すので、検証コマンドからは戻しを見られない）

## Task 4: 文書と完了の判定

- [ ] 4.1 `docs/flow-gates.md` の `tools/check-verify-env-reset.sh` の行の「見るもの」に「作り直したときだけの `HARNESS_RESET` の宣言」を足し、`tools/check-verify-env-reset.sh` の冒頭の注記に ch07 の段（9.〜11.）と宣言の確かめ、`--self-test` の「CI と手元で同じ結果」の例外（ハーネスの点検の段は `.harness2` のある木だけ。CI は SKIP）を足す（design D5）。
  検証: `bash -c 'grep -F "check-verify-env-reset.sh" docs/flow-gates.md | grep -qF "HARNESS_RESET"'` rc=0 / `bash -c 'head -20 tools/check-verify-env-reset.sh | grep -q "ch07"'` rc=0
- [ ] 4.2 完了の判定 3 を確かめる（C7）: `.harness2` の SHA を証跡に残し、`init.py --check` の `tools/verify-env.sh` の項目が `[ok]` だけであること。
  検証: `bash -c 'grep -q check_verify_env .harness2/scripts/init.py || exit 1; git -C .harness2 rev-parse HEAD; o=$(python3 .harness2/scripts/init.py --check 2>&1 || true); ! grep -qF "[人間] tools/verify-env.sh" <<<"$o" && [ "$(grep -cF "[ok] tools/verify-env.sh" <<<"$o")" = 1 ]'` rc=0
- [ ] 4.3 変えないものが変わっていないこと（Global Constraints）と、全体の確かめ。
  diff の基準は `origin/main`。`tools/env-init.sh`・`tools/ports.sh`・`.github/workflows/` は CH06 の差分が混ざるので diff に入れず、CH06 の検査（`tools/check-ports.sh --self-test`・`tools/check-verify-env-reset.sh` の CH06 の段）が CH06 の振る舞いを固定する。
  検証: `bash -c '[ -z "$(git diff origin/main...HEAD -- crates migrations tools/stack.sh tools/smoke.sh tools/check-immutable.sh tools/check-panic-log.sh tools/dev.sh tools/record-env.sh tools/db-roles.sh docker-compose.yml)" ]'` rc=0 /
  `bash -c 'openspec validate ch07-declare-verify-reset --strict'` rc=0 / `python3 scripts/check_scenarios.py . ch07-declare-verify-reset` rc=0 / `tools/check-private.sh` rc=0 /
  `tools/check-ports.sh --self-test` rc=0 / `tools/check-verify-env-reset.sh --self-test` rc=0 / `tools/check-verify-env-reset.sh` rc=0
