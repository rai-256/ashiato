# CH07 下流のコードレビュー

## final review（1954021..d81a260）

席: final reviewer（`superpowers:requesting-code-review` の `code-reviewer.md`）。review package は
`.superpowers/sdd/tasks/review-19540213..d81a2601.diff`（11 commit）。全 Task の ledger（`progress.md` は無く、deferred-minor / parked / Ruling の行は 0）と、
全 Task の指摘の記録の Minor / ⚠️ を指した。判定は **Yes**（Critical 0 / Important 0 / Minor 3）。

申し送り（`docs/handoff/CH07.md`）は無い。

⚠️ の確かめ（final reviewer）: Task 1 の CH06 C4 は hunk の外で不変 —— OK。Task 3 の証跡の head 7d252409 —— 3.1 の tree の
`check-verify-env-reset.sh` の blob が e66fa37b と一致し、4.3（7b9db6d1）の tree の 2 本の blob は d81a2601 と一致 —— OK。
Task 4 の D5 の注記（`tools/verify-env.sh:4`）—— OK。4.2 の `.harness2` の SHA `aaecd2c6…` は log と `evidence.jsonl` にあり、いまの `.harness2` の HEAD と一致 —— OK。
Task の Minor（Task 1 F2・Task 2 F2・Task 3 F1 / F2）はそのまま残す（理由は final reviewer の triage: 冪等・3./4./10./11. の段が壊し方を弁別する・tasks 3.1 の明示）。

## R1. `--self-test` の --plan の段のラベルが、印があると「印ありHARNESS_VERIFY=1」に展開される
- 成果物: tools/check-verify-env-reset.sh:471（d81a2601）
- 根拠: `${mark:+印あり}${mark:-印なし}` —— `${mark:-…}` は mark が空でなければ mark の値になる。`--self-test` の出力で確認（Task 2 F1 と同じ）。判定には影響しない
- kind: technical
- 処置: fixed 2.1 —— `$([[ -n $mark ]] && echo 印あり || echo 印なし)` に（5a5bb003。`--self-test` rc=0、出力が「demo（CI なし・印あり）…」）

## R2. 本体の注記が「design D8 の 1.〜8.」のままで、ch07 の 9.〜11. と合っていない
- 成果物: tools/check-verify-env-reset.sh:16・:5（d81a2601）
- 根拠: body() の上の注記。9.〜11. は ch07 で足した（Task 4 F2 の積み残し）。:5 の「CI と手元で同じ結果」は :9 の例外と並べないと読み違える
- kind: technical
- 処置: fixed 3.2 —— 「1.〜11.。1.〜8. は design D8、9.〜11. は ch07」と「（例外は下）」（5a5bb003）

## R3. 10. の途中で abort すると、`$tmp` が消えて tmpfs の DB のコンテナが残る
- 成果物: tools/check-verify-env-reset.sh の 10.（volume を持たない DB のコンテナ）
- 根拠: 後始末は trap の `$tmp` の削除だけ。残ったコンテナは次の準備が作り直す（design D3）ので収束する
- kind: technical
- 処置: rejected: 次の `tools/verify-env.sh` が宣言なしで作り直して収束する（design D3）。害は残らないので直さない

## scoped re-review（d81a260..5a5bb00）

R1・R2 とも ADDRESSED、fix の差分に新しい破損なし（`.superpowers/sdd/tasks/review-d81a2601..5a5bb003.diff`、報告 `.superpowers/sdd/ch07-final-fix-report.md`）。

## code_verify（9038f601。独立検証）

### 申告と実測

申告: tasks 8/8 `[x]`（1.1・2.1・2.2・3.1・3.2・4.1・4.2・4.3）。検証コマンドは 21 本。

| 項目 | 検証コマンド | 実測 rc | 一致 |
|---|---|---|---|
| 1.1 | 印つきの準備の標準出力に `HARNESS_RESET: testdb` が 1 行 | 0 | 一致 |
| 1.1 | 印なしの準備の出力に `HARNESS_RESET:` が無い | 0 | 一致 |
| 1.1 | `HARNESS_VERIFY=1 tools/verify-env.sh --plan` = `reset` | 0 | 一致 |
| 1.1 | 注記でない行の `echo "HARNESS_RESET: testdb"` が 1 つ | 0 | 一致 |
| 2.1 | `tools/check-verify-env-reset.sh --self-test` | 0 | 一致 |
| 2.1 | CI と同じ環境変数で `--self-test` | 0 | 一致 |
| 2.1 | 印 2 つの grep | 0 | 一致 |
| 2.2 | `--self-test` の出力に `^SKIP` が無い（worktree） | 0 | 一致 |
| 2.2 | 印の grep | 0 | 一致 |
| 3.1 | `echo "Scenario: ` が 32 本以上（実数 35） | 0 | 一致 |
| 3.2 | 同じく 35 本以上（実数 35）/ 印 3 つの grep | 0 / 0 | 一致 |
| 3.1・3.2・4.3 | `tools/check-verify-env-reset.sh`（本体。78 秒） | 0 | 一致 |
| 4.1 | flow-gates.md の行に `HARNESS_RESET` / 冒頭 20 行に `ch07` | 0 / 0 | 一致 |
| 4.2 | `init.py --check`（`.harness2` HEAD `aaecd2c6305059209f6fbfb28211975de3610f1c`）の `[人間]` 0 行・`[ok]` 1 行 | 0 | 一致 |
| 4.3 | 変えないものの `git diff origin/main...HEAD` が空 | 0 | 一致 |
| 4.3 | `openspec validate --strict` / `check_scenarios.py` / `check-private.sh` / `check-ports.sh --self-test` | 0 / 0 / 0 / 0 | 一致 |

全部一致。`[x]` のまま実体の無いコマンド・テストは無かった（`cargo test --test` の形は使っていない）。

### 手 1: 該当なし
期待値を直に書いた固定値のテストは無い（確かめた範囲: `tools/check-verify-env-reset.sh` の差分。比べているのは件数と、spec の字そのもの `HARNESS_RESET: testdb` だけ）。

### 手 2: ガードをわざと壊す
`tools/verify-env.sh` を作業ツリーで 1 か所ずつ壊して走らせ、毎回 `git checkout` で戻した。戻した後の `git status --porcelain` と `git diff` の sha1 が、壊す前と同じであることを毎回確かめた。

| # | 壊したもの | 走らせたもの | rc | 落ちた段 |
|---|---|---|---|---|
| M1 | `cut_line` の直後で `reset_done=1`（volume が 0 個でも宣言する） | 本体 | 1 | 10. volume を持たない DB（宣言 1・期待 0） |
| M2 | `db-roles.sh` より前で宣言する | 本体 | 1 | 11. 途中で落ちる（宣言 1・期待 0） |
| M3 | 宣言を `>&2` へ出す | self-test / 本体 | 1 / 1 | 宣言の字の守り / 1.・5.・6.・7.・9. |
| M4 | コンテナが無いときに `reset_done=1` を立てない | 本体 | 1 | 7.・10. の戻し・8. |
| M5 | `--plan` でも宣言を出す | self-test | 1 | 判定だけを出すときの段（--plan の全部の組み合わせ） |
| M6 | 宣言の `echo` の行を消す（:134） | self-test / `init.py --check` | 1 / `[ok]` のまま | 宣言の字の守りだけが落ちた → R4 |
| M7 | keep の経路で `HARNESS_RESET: none` を出す | 本体 | 1 | 3. 印なし（out・err の行 1・期待 0） |

`check-verify-env-reset.sh` の中のガードは、壊したもの全部を見分けた。外側で 1 件見つかった（R4）。

### 手 3: Scenario と test の突き合わせ
`python3 scripts/check_scenarios.py . ch07-declare-verify-reset` → `scenarios: OK` rc=0（この change の 12 本は全部に印がある）。印の先を 1 本ずつ読んだ:

- 本体: 印なし→3.、CI でない main の checkout→3. の CI だけの段、作り直せずに非 0→4.、溜まった DB→5.（2. の 3 行が残った状態。標準エラーの 0 行も見る）、2 回続けて→6.、コンテナが無い→7.、止まっていた→9.、volume を持たない→10.、途中で落ちる→11.。
  どれも `decl` で標準出力の `-x` 一致と、out・err の `^HARNESS_RESET:` を数えている。spec の主張（標準出力に字のとおりの 1 行・標準エラーに 0 行）と同じ階層で見ている
- self-test: --plan→4 つの木 × CI × 印の 16 通り、git の木でない→`GIT_CEILING_DIRECTORIES` つきの写し、ハーネスの点検→`init.py --check`（R4 を参照）
- 片方の環境で SKIP になる段も、その段が走る環境で実測した:
  - 「CI でない main の checkout では、印つきでも宣言を出さない」: worktree では SKIP。`git clone` した worktree でない木（`/tmp/ch07sim.*/ch07mainsim`、`CI=true ASHIATO_DB_PORT=55988`）で本体を走らせ、rc=0・この段の `decl … 0` が ok。**実際の Actions の run はまだ観測していない**（`gh run list --branch feat/ch07-declare-verify-reset` が空。push 前）
  - 「ハーネスの点検…」: `.harness2` の無い clone で CI と同じ環境変数の `--self-test` を走らせ、rc=0・`SKIP ハーネスの点検（.harness2 が無い木…）` の 1 行・`OK`
- 模擬に使った複製は `docker compose down -v` と `rm -rf` で片付けた

### 手 4: 本人の決定が test で固定されているか
deep.md の問いは 0 件で、決めたのは C1〜C9（数値の決定は無い）。それぞれを壊したときに落ちるテストがあるかを手 2 で確かめた。
C1 の字そのもの（`grep -cx`。M3・M6）、C2 の「volume ≥1 か、コンテナが無い」（M1・M4）、C2 の「役割まで整えた後」（M2）、C2 の標準出力（M3）、C3 の none を出さない（M7）、C4・C5 の `--plan` が宣言しない（M5）。
C4 の CH06 の振る舞いは、変えないものの diff が空（4.3）で、CH06 の段も本体の rc=0 で通った。値を変えても全部通る決定は無かった。

### 手 5: tasks の `[x]` と実体
冒頭の表のとおり。8 件・21 本の全部が実在し、rc=0。

### 手 6: 隙間
- **コンテナだけが外で消され、volume が残った状態**を作って確かめた: 行を入れる → `docker compose rm -s -f db`（`-v` なし）→ `HARNESS_VERIFY=1 tools/verify-env.sh`。
  rc=0・標準出力 `HARNESS_RESET: testdb`・新しい DB に表は無かった（`relation … does not exist`）ので、宣言は事実どおり。
  ただ前の匿名 volume `ace11acf…` は残った（確かめた後に手で消した）→ R5
- プロセスの再起動・権限の拒否・時計: この change は準備の出力の 1 行だけを足し、記録を持たない。捨てる経路は CH06 の作り直し（判定は変えていない。4.3 で固定）だけ
- 10. の途中で止まると tmpfs のコンテナが残る件は、final review の R3（rejected）で扱っている。重ねて出さない

### 手 7: 該当なし
`python3 scripts/plan_fix.py pending openspec/changes/ch07-declare-verify-reset` → `[]` rc=0。

## R4. Scenario「ハーネスの点検が、作り直して宣言する準備と認める」は注記の字だけで通る。この Scenario の印の段は振る舞いを観測していない
- 成果物: tools/check-verify-env-reset.sh（ハーネスの点検の段）/ tools/verify-env.sh:4・:134 / .harness2/scripts/init.py:369
- 根拠: M6 で `tools/verify-env.sh:134` の `echo` の行を消した。`python3 .harness2/scripts/init.py --check` は変わらず `[ok] tools/verify-env.sh（HARNESS_VERIFY=1 で作り直して HARNESS_RESET を宣言する）` を出した。`init.py:369` は `"HARNESS_RESET:" not in text` の部分一致で、`:4` の注記がこれを満たす。
  `--self-test` は rc=1 で落ちた。ただ落としたのは印の無い「宣言の字」の守り（`echo "HARNESS_RESET: testdb" の行がちょうど 1 つ（0。期待は 1）`）で、この Scenario の段ではない。CH07 の中の守りは効いている。完了の判定 3（`[ok]` がちょうど 1 つ）は、実装の有無と関係なく成り立つ
- kind: defer
- 提案: harness2 の `check_verify_env` が注記の行を除いて字を探すようにする（ashiato2 の Story・change の外。harness2 側で直す）。CH07 では、この Scenario の段に「宣言の字」の守りも並べるか、完了の判定 3 は字面の点検だけを見ていると design D4 に書き添える
- 処置: fixed 2.2 —— CH07 の中で直した。`tools/verify-env.sh:4` の注記から宣言の字（`HARNESS_RESET` とコロン）を外し、`--self-test` の宣言の字の守りに「注記の行に `HARNESS_RESET:` が無い」を足した。M6 と同じく echo の行を消すと `init.py --check` が `[人間] tools/verify-env.sh が作り直したものを宣言していない` を出し、この Scenario の段（`[ok] … ちょうど 1 つ`・`[人間] … 無い`）が NG になることを実測（`tools/check-verify-env-reset.sh --self-test` rc=1）。harness2 の `check_verify_env` が注記を除いて読む直しは harness2 側の話で、PR 本文の申し送りに残す

## R5. コンテナが外で消され volume が残った状態では、宣言は事実どおりだが前の DB の中身を持った volume が残り続ける
- 成果物: tools/verify-env.sh:59（`reset_db` の、コンテナが無い分岐）
- 根拠: 手 6 の実測。`docker compose rm -s -f db` の後に印つきの準備で rc=0・`HARNESS_RESET: testdb`・新しい DB は空。前の匿名 volume `ace11acfc88c…` は `docker volume inspect` で残っていた。
  CH06・CH07 のどちらの Scenario も「コンテナが無く、volume はある」を扱わない（7. はコンテナと volume の両方を消してから呼ぶ）。CH07 の主張（宣言が事実どおり）は破っておらず、記録が失われる経路でもない。溜まるのはディスクと、消したつもりの私的データの写し
- kind: defer
- 提案: CH06 の作り直しの対象（D1。`-v` を使わず、名前で控えた volume だけを消す）の範囲。コンテナが無いときに、その project のラベルを持つ宙に浮いた volume を片付けるかを CH06 の後続（change）で決める。CH07 では直さない
- 処置: followup CH06 —— `docs/handoff/CH06.md` に書いた（CH06 は archive 済み。後続の change で決める）

## security review（1954021..9038f60）

見た経路:
- `scripts/verify-run` → `tools/verify-env.sh`（`HARNESS_VERIFY=1`）→ `reset_db` が `reset_done=1` → 役割を整えた後に標準出力へ `HARNESS_RESET: testdb`（固定の字。外から来る値を含まない）（3）
- ハーネスの取り込み: `.harness2/scripts/evidence.py:404` が `RESET_MARK` を準備の出力（ログの `$ (cd … && ` より前）だけから拾う → `evidence.jsonl` の `reset`。検証コマンド（試験）の出力が宣言を偽れる経路は無い（3）
- `tools/verify-env.sh` の標準出力に出る行は `--plan` の `reset` / `keep: <固定の理由>`（18〜24 行）と 134 行の宣言だけ。`cut_line`（接続の出所）・volume 名・network 名は従来どおり標準エラーで、この差分は出し先を変えていない（2）
- 検査の本体の新しい段（9.〜11.）: tmpfs の compose ファイルと `ENV_FILE` の行き先は `mktemp -d` の `$tmp`（33 行・trap で消す）。合言葉（`owner_pw` `app_pw`）は既存の読み方のままで、新しい段は出力に出さない（`decl` / `want` が出すのは行数だけ）（3）
- `--self-test` の新しい段: `init.py --check` の出力は件数だけを `want` に渡す。git でない木の段は `$tmp/nogit` に写した 2 本だけを `GIT_CEILING_DIRECTORIES` で閉じて呼ぶ（3）
- 新しい listen・接続先・依存（crate / npm / gradle）は無い。`git diff --stat` は shell 2 本・docs・openspec のみ（2）
- 足された `evidence.jsonl` に私設 IP・`/home/…`・合言葉の字は無い（grep で 0 件。`tools/check-private.sh` rc=0）（4）

安全の決め手: 宣言の行は固定の字 `HARNESS_RESET: testdb` 1 本だけで、記録の中身・接続の出所・合言葉を運ばない —— 確かさ 4。
`scripts/quiet-run selftest -- tools/check-verify-env-reset.sh --self-test` が rc=0・`OK`（「echo "HARNESS_RESET: testdb" の行がちょうど 1 つ」「出力を捨てる形でない」「--plan は 1 行で宣言を含まない」を含む）。標準出力へ出る echo が上の 4 か所だけであることは grep で確かめた（2）。

指摘なし（消したつもりの DB の中身が宙に浮いた volume に残る経路は既出の R5 が扱う）

## /code-review（1954021..9038f60）

見た: Claude Code の組み込みの `/code-review`（9 件）。写しは scripts/code_review_record.py

## R6. The self-test step for the Scenario "ハーネスの点検が、作り直して宣言する準備と認める" only checks that `init.py --check` prints `[ok]`, and init.py does a plain substring search for `HARNESS_RESET:` that the comment on verify-env.sh:4 already satisfies. This step cannot detect a missing declaration.
- 成果物: tools/check-verify-env-reset.sh:488
- 根拠: Delete the `echo "HARNESS_RESET: testdb"` at verify-env.sh:134. `init.py --check` still prints `[ok] tools/verify-env.sh` because the line-4 comment matches, so this Scenario's step passes. Only the separate, unlabelled "宣言の字" guard fails (this was M6 in code_verify), so the Scenario's label vouches for behaviour it never observes.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: fixed 2.2 —— R4 と同じ直し。注記が `init.py` の部分一致を満たさなくなり、この Scenario の段は echo の行の有無で決まる（echo を消すと段が NG、`--self-test` rc=1 を実測）

## R7. The header comment contains the literal `HARNESS_RESET: testdb`, and the harness doctor (`check_verify_env`, init.py:369) uses that same literal to decide the script declares a reset. The comment alone keeps the doctor green.
- 成果物: tools/verify-env.sh:4
- 根拠: A later edit drops or renames the echo on line 134, or moves it inside a function that never runs. The doctor keeps passing because of the comment. The only remaining guard is in the self-test, which runs only in CI or by hand, so the harness may never see `reset` in evidence.jsonl and nobody is told.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: fixed 1.1 —— `tools/verify-env.sh:4` の注記を「宣言（`HARNESS_RESET` の行。字は待ちのループの echo）」に替え、「注記に宣言の字を書かない」理由を 1 行足した。`--self-test` が注記の字を 0 で守る（R4）

## R8. `--self-test` now calls `python3 .harness2/scripts/init.py --check`, which runs the whole doctor: `gh repo view` against the network, `openspec --version`, and the GitHub and machine checks. A test documented as hermetic now makes network calls and depends on external tools.
- 成果物: tools/check-verify-env-reset.sh:487
- 根拠: In a worktree with no network, or with gh not logged in or rate-limited, `gh repo view` hangs or fails. `|| true` hides the failure, but the self-test still waits on network I/O, and the doctor's output shape is now coupled to unrelated checks. The header says the self-test gives the same result in CI and locally and does not touch the docker daemon, yet it now depends on the gh and openspec state of the host.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: rejected: 判定 3 を `init.py --check` で見るのは premises.md C7 の決定（完了の判定 3 がこの出力で定義されている）。段は `.harness2` のある木だけで走り CI は SKIP（`tools/check-verify-env-reset.sh:9` に例外として明記）。数えるのは `tools/verify-env.sh` の行だけで、gh が引けないときに足されるのは別の `todo` の行なので件数は変わらない。手元の実測 `time python3 .harness2/scripts/init.py --check` は 1.8 秒、`--self-test` 全体で 3 秒

## R9. Stage 10 replaces the DB with a tmpfs container under `set -Eeuo pipefail`. If any command between the swap and the restore (lines 252–262) fails, the EXIT trap deletes `$tmp/tmpfs-compose.yml` but leaves the tmpfs DB container running.
- 成果物: tools/check-verify-env-reset.sh:252
- 根拠: Say `docker compose up -d --wait db` with cf10 times out, or the marked call fails. The body exits and the worktree keeps a volume-less container whose compose file no longer exists. On the next `HARNESS_VERIFY=1 tools/verify-env.sh`, reset_db finds that container with 0 volumes, so reset_done stays empty and no `HARNESS_RESET: testdb` is printed even though the DB was recreated. Every verify run in that worktree records reset=false until someone removes the container by hand.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: rejected: R3 と同じ。残った tmpfs のコンテナは、次の印つきの準備で `reset_db` が消して volume を持つ形で立て直す（`tools/verify-env.sh:70`。10. の「戻す」がこの経路そのもので、`decl "volume を持つ形に戻す" 1` が次の回の宣言を確かめている）。宣言が出ないのはその 1 回だけで「毎回」ではなく、その 1 回は C2 の volume 0 個の規則（R11）どおり

## R10. When the DB container is missing but its anonymous volume still exists (the container was removed externally without -v), the script declares `HARNESS_RESET: testdb` and leaves the old volume, which still holds the previous DB's data.
- 成果物: tools/verify-env.sh:59
- 根拠: Run `docker compose rm -s -f db`, then the marked prep. It prints rc=0 and `HARNESS_RESET: testdb`, but the old volume still holds the previous run's private rows and they pile up across runs (reproduced in code_verify hand 6). The declaration claims the DB was emptied while the previous data copy still sits on disk.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: followup CH06 —— R5 と同じ。`docs/handoff/CH06.md` に書いた

## R11. Whether to declare depends on whether a volume was removed, not on whether the DB container was recreated. A volume-less (tmpfs or bind-mount) DB is destroyed and recreated with no declaration.
- 成果物: tools/verify-env.sh:71
- 根拠: Use a compose override with tmpfs for /var/lib/postgresql/data, or one where the data dir is a bind mount. With the mark set, `docker compose rm -s -f db` throws away the tmpfs data and a fresh empty DB comes up, but reset_done stays empty. The harness records that nothing was reset, so evidence says the DB was not reset after a run that started from empty, which is the opposite of what happened.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: rejected: 意図した振る舞い。premises.md C2 が「コンテナがあって控えた volume が 0 個（bind mount で中身を持つなど）なら出さない —— 中身が残ったまま『作り直した』と宣言しない（厳しい側）」と決め、Scenario「volume を持たない DB のコンテナでは、作り直しても宣言を出さない」（`tools/check-verify-env-reset.sh:251`）が固定している。bind mount は中身が残るので宣言すると事実に反する。この repo の compose の DB は volume を持つ

## R12. The "宣言の字" guard rejects only three literal patterns (`>/dev/null`, `>&2`, `$(`). Other ways the declaration line can keep the marker off stdout are not caught.
- 成果物: tools/check-verify-env-reset.sh:482
- 根拠: Rewrite line 134 as `echo "HARNESS_RESET: testdb" >>"$log"`, `| tee /dev/stderr >/dev/null` in another form, `>"$tmp/x"`, or `2>&1 >&3`. The guard still reports ok, so the self-test passes while the harness never sees the marker on stdout. Only the docker-backed body stages would catch it.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: fixed 2.1 —— 3 つの字の否定をやめ、「echo の後ろが空白だけ」かつ「`$(`・バッククォートを含まない」で見る形にした。`>>"$tmp/x"` を付けた変異で `--self-test` が NG・rc=1 になることを実測

## R13. The extra stage-5 check (`標準エラーに HARNESS_RESET: の行は 0 行`) adds nothing: `decl … 1` already asserts exactly one exact line on stdout and exactly one `^HARNESS_RESET:` line across stdout and stderr together, which implies stderr has 0.
- 成果物: tools/check-verify-env-reset.sh:176
- 根拠: No failure is possible here; the cost is a duplicated assertion in only one of the stages that call decl. A reader may wrongly conclude that the other decl stages do not cover stderr.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: rejected: 重複は事実（`decl` の 2 つの数から標準エラー 0 行が導ける）だが、tasks 3.1 が 5. に「標準エラーに 0 行も見る」を名指しし、spec の主張（標準エラーに出さない）を字のまま段に残すためのもの（`tools/check-verify-env-reset.sh:176`）。落ちる経路を増やしも隠しもしない

## R14. Stages 9–11 sit between stage 7 and stage 8 and change the DB state that stage 8 (the subnet checks) depends on. Stage 8 now starts from a DB that stage 11 rebuilt with a separate restore call, and the 1.〜11. labels no longer match the order they run in.
- 成果物: tools/check-verify-env-reset.sh:228
- 根拠: If the stage-11 restore (`marked # 戻す`) fails, for example because db-roles hits a transient error, stage 8 runs against a DB with no roles or no container. Its failures are then reported under "8. 塞いでいる network" even though stage 11 caused them, which misleads triage.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: rejected: 11. の戻しが落ちると、まず `want "戻す準備は rc=0"` が `NG [11. 途中で落ちる]` を出す（`tools/check-verify-env-reset.sh:274`）ので、最初の NG の段名が原因を指す。番号は段の出所（1.〜8. は CH06 design D8、9.〜11. は ch07。`:16` に明記）で、8. は subnet のファイルがある木だけの段で network を消すので最後に置いている

## scoped re-review（9038f60..作業ツリー。R4・R6・R7・R12 の直し）

R4・R6・R7・R12 とも ADDRESSED（独立の reviewer。変異 3 つ —— echo の行を消す・注記に字を足す・echo の後ろに `>>"$log"` —— で `--self-test` が rc=1、戻した木で rc=0）。
fix の差分に新しい破損は無い。確かさの低い残り 2 件は fix 1 回の決まりで直さず park する（PR 本文の申し送りに写した）:
- echo の**前**に置いたリダイレクト（`|| >/dev/null echo "HARNESS_RESET: testdb"`）は「echo の後ろが空」の守りを抜ける（`tools/check-verify-env-reset.sh:486`）。本体の段（標準出力の宣言の数）は落とす
- 行末の注記（`exit 0  # HARNESS_RESET: x`）は「注記の行に字が無い」の守りを抜け、`init.py --check` も `[ok]` を出す（`:489`）。harness2 の `check_verify_env` が注記を除いて読む直しと同じ根
