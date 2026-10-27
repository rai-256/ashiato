# CH05 実装タスク — 試験の DB が打ち切った run の残りで膨れないようにする

読む順: `deep.md`（**本人の答え Q1〜Q8**）→ このファイル → `specs/test-database/spec.md` → `specs/verify-environment/spec.md` → `design.md` → `premises.md` → `docs/changes/CH05.md` →
CH04 の `openspec/changes/ch04-test-db-growth/design.md`（D1・D2・D4・D6・D7・D8）→ CH06 の `openspec/changes/ch06-reset-verify-db/design.md`（D1・D2・D5・D8）→ `crates/server/src/testdb.rs`（CH04 の後の形）→ `tools/verify-env.sh`（CH06 の後の形）→ `CLAUDE.md`。

**CH04 と CH06 の実装の上に積む**（Q1・Q8）。`testdb.rs`・`tools/check-test-db-growth.sh`・CH06 の作り直しの規則（`--plan`・`reset_db`・`clear_blocking_network`・`tools/env-init.sh`・`tools/ports.sh`）・消せない表のトリガ・試験の期待値は変えない。
書くのは、消えた作業ツリーの片付けの台本と検証の準備の 1 行（Q7）・殺した後の検査と探りの試験（Q6）・CI の 3 行・文書だけ。
膨れた `ashiato` の作り直し・見送り・作業ツリーの種類は作らない（Q8。CH06 が検証のたびに空から作り直す）。

検証は各タスクの本文に書いてある。「動いた」ではなくコマンドと終了コードで判定する。DB を使う検査は `HARNESS_VERIFY=1 tools/verify-env.sh` が前提（`scripts/verify-run` が毎回呼ぶ）。
**Task 2 を DB を使う Task より先にやる**（DB の要らない self-test で片付けを確かめ、外すものの一覧を写してから、Task 3 で検証の準備に組み込む）。

## Global Constraints

- **テストには `Scenario: <名前>` の印を置く。** Rust はコメント（`// Scenario: …`）、bash は `echo "Scenario: …"`。
  `scripts/check_scenarios.py` が spec の全 Scenario と突き合わせ、印の無い Scenario を FAIL にする。印の名前は spec の `#### Scenario:` と**一字一句合わせる**
- **この change の Scenario は 31 本**（`test-database` の ADDED 10 本・`verify-environment` の ADDED 15 本・MODIFIED 6 本）。**CH05 で新しく足したのは 26 本**で、全部このファイルのどこかの Task が名指しする。
  残る 5 本は CH06 の Requirement を MODIFIED で写した既存分で、印は CH06 の `tools/check-verify-env-reset.sh` のまま（触らない）
- **件数つき検証**: このファイルで
  - **`EV <命令>`** は `bash -c 'set -a; [ -f .env ] && . ./.env; set +a; <命令>'` rc=0（`.env` の合言葉と URL を読んで走らせる）
  - **`NL <名前と下限>`** は `bash -c 'set -- <名前と下限>; [ "$(cargo test -p ashiato-server --lib "$1" -- --list --include-ignored 2>/dev/null | grep -c ": test$")" -ge "$2" ]'` rc=0（`cargo test` は 0 本でも rc=0 になるので、本数の下限を別に見る。`#[ignore]` の試験も数える）
  （`cargo test` に絞り込みを 2 つ渡すと `unexpected argument` で落ちる。1 つずつ書く）
- **新しく足す試験の名前の頭は `ch05_`**（design D3）。**commit の題は `feat(CH05): …` / `fix(CH05): …` / `chore(CH05): …`**（tasks 8.2 は題に依らず、`origin/main` と取り込んだ CH04・CH06 の SHA（`deps.json`）の先の commit を拾って、触ったファイルを見る）
- **CH04 の鍵の番号と回の名前は、CH04 の後の `crates/server/src/testdb.rs` から読む**（design の `4820252`・`4820253`・`ashiato_run_`・`isolated()` は CH04 の `9fe8c5e` の値。違えば `testdb.rs` に合わせ、design の D2 に書き足す）
- **CH06 の `tools/verify-env.sh` は行を足すだけ**（片付けの 1 行と注記。既存の行を消さない・書き換えない。tasks 8.2 が見る）。片付けは判定が `reset` のときだけ、DB を消す前に呼ぶ（design D5）
- `down` と、volume を消す命令（`docker volume rm`・`volume prune`・`system prune`・`rm -v`）を新しく書かない（Q7）。
  コンテナとネットワークを外す命令（`docker rm -f`・`docker network rm`）は `tools/docker-tidy.sh` の片付けの本体にだけ書く（design D4）。
  例外は 1 つ: `tools/docker-tidy.sh` の `--self-test`・`--verify-env-check` が自分で作ったもの（印 `dev.ashiato.ch05-tidy-probe=1` のコンテナ・ネットワークと控えた id の volume）を消す後始末（`rm -fv`・`network rm`・`volume rm`。片付けの本体からは呼ばない。design D4）
- **助言ロックと database の一覧は `postgres` database で読む**（database ごとに分かれる。design D2）
- **一時ファイルは `mktemp -d` の下に作る**（`/tmp` の固定名に書かない。CI は `TMPDIR` を置かない）
- **本人の決定（下流は変えない）**: Q1 CH04 の上に積み、CH04 が持たないものだけをやる / Q6 量で見る（秒数は出すだけ）。殺す時点は 2 つ /
  Q7 **使われていないコンテナ・ネットワークを片付ける（volume は消さない）**。片付けてよいのは作業ツリーが消えたことを機械で確かめられるものだけ。既存の `down -v` の台本への見送りと、lock timeout・権限のエラーの調べは入れない /
  Q8 **作り直しと見送り（Q2〜Q5）は作らず CH06 に任せる**。CH05 は殺した後の検査と片付けだけ
- 走っている Story・change のブランチには触らない（proposal の Impact）

## Task 1: 前提を確かめる

- [x] 1.1 この作業ツリーの DB が立ち、CH04 と CH06 の実装が取り込まれている（`docs/changes/CH05.md` の `requires: [CH04, CH06]` と `story_deps.py included`）。
  `testdb.rs` の回の助言ロック・直列化のロックの番号と回の名前の頭・`isolated()` が design D2 と同じ。違えば design D2 を書き直してから進む。
  `tools/verify-env.sh` が CH06 の形（`--plan`・作り直し・塞いでいる network の確かめ）で、design D5 の置き場（`reset` のとき、DB を消す前・network を確かめる前）がある。違えば design D5 を書き直してから進む。
  検証: `bash -c 'HARNESS_VERIFY=1 tools/verify-env.sh'` rc=0 / `python3 scripts/story_deps.py included CH05 CH04` rc=0 / `python3 scripts/story_deps.py included CH05 CH06` rc=0 /
  `bash -c 'f=crates/server/src/testdb.rs; grep -q 4820252 "$f" && grep -q 4820253 "$f" && grep -q ashiato_run_ "$f" && grep -q "pub async fn isolated" "$f"'` rc=0 /
  `bash -c 'f=tools/verify-env.sh; grep -q -- "--plan" "$f" && grep -q "reset_db" "$f" && grep -q "clear_blocking_network" "$f"'` rc=0

## Task 2: 消えた作業ツリーのコンテナとネットワークの片付けの台本（Q7。DB を使う Task より先）

- [x] 2.1 `tools/docker-tidy.sh` を design D4 のとおりに書く（口は 4 つ: 引数なし = 片付ける / `--dry-run` / `--self-test` / `--verify-env-check`。`--verify-env-check` は 3.2）。冒頭のコメントに、外す条件（コンテナとネットワーク）・volume は消さないこと・差し替えの環境変数（`TEST_DOCKER_TIDY_DOCKER`・`TEST_DOCKER_TIDY_MAIN`・`TEST_DOCKER_TIDY_WORKTREES`）・検証の準備が `reset` のときだけ呼ぶこと・なぜ要るか（CH05 Q7）を書く。
  `--self-test` は偽の docker と、main を乱数の名前に差し替えた本物の docker の 1 件（design D4。この機械の本物の残りには当たらない）で、次の 12 本の Scenario を印つきで確かめる:
  `作業ツリーが消えたコンテナは、動いていても外される` / `外したコンテナの名前の無い volume は残る` / `作業ツリーが残っていれば、止まっているコンテナも外さない` / `main の checkout と確認バッチのコンテナは、path が無くても外さない` / `印の無いコンテナと、別の repo の compose のコンテナには触らない` / `コンテナの無いネットワークは、作業ツリーが残っていなければ外す` / `--dry-run は外すものを出すだけで、何も外さない` / `外すものが無ければ、片付けは何も出さない` / `もう無いものを外そうとしたら、外せたものとして扱う` / `片付けが docker に問えなければ、片付けは rc≠0 で落ちる` / `外す命令が失敗すれば、片付けは rc≠0 で落ちる` / `片付けが docker に打つ命令に、volume を消すものは無い`。
  **本物の docker の 1 件は main を差し替えるので、この機械の本物の残りは外さない。** 書けたら、この機械で `tools/docker-tidy.sh --dry-run` の出力（外すものの一覧）を design の「計測の記録」の `- 片付けたもの:` に写す（3.1 で検証の準備に組み込むと、次の検証の準備で本物が外れる）。
  検証: `bash -n tools/docker-tidy.sh` rc=0 / `tools/docker-tidy.sh --self-test` rc=0 /
  `bash -c 'f=tools/docker-tidy.sh; for s in "作業ツリーが消えたコンテナは、動いていても外される" "外したコンテナの名前の無い volume は残る" "作業ツリーが残っていれば、止まっているコンテナも外さない" "main の checkout と確認バッチのコンテナは、path が無くても外さない" "印の無いコンテナと、別の repo の compose のコンテナには触らない" "コンテナの無いネットワークは、作業ツリーが残っていなければ外す" "外すものが無ければ、片付けは何も出さない" "もう無いものを外そうとしたら、外せたものとして扱う" "片付けが docker に問えなければ、片付けは rc≠0 で落ちる" "外す命令が失敗すれば、片付けは rc≠0 で落ちる" "片付けが docker に打つ命令に、volume を消すものは無い"; do grep -qF "Scenario: $s" "$f" || { echo "印が無い: $s"; exit 1; }; done; grep -qF "Scenario: --dry-run は外すものを出すだけで、何も外さない" "$f"'` rc=0 /
  `bash -c 'grep -qE "^- 片付けたもの: .*[0-9a-z]" openspec/changes/ch05-test-db-bloat/design.md'` rc=0

## Task 3: 検証の準備に組み込む

- [x] 3.1 `tools/verify-env.sh` の、`tools/env-init.sh` の後・DB を消す前（`reset_db` の前）に、判定が `reset` のときだけ `tools/docker-tidy.sh` を呼ぶ 1 行を足す（rc≠0 なら `verify-env: 片付けに失敗した（…）。続ける` を標準エラーに出して続ける。design D5）。
  冒頭の注記に「作り直すときは、消えた作業ツリーのコンテナとネットワークを先に片付ける（CH05 Q7。volume は消さない）」を 1 行足す。既存の行は変えない。CH06 の検査がそのまま通ることを見る。
  検証: `bash -n tools/verify-env.sh` rc=0 / `bash -c 'HARNESS_VERIFY=1 tools/verify-env.sh'` rc=0 / `tools/check-verify-env-reset.sh --self-test` rc=0 / `EV scripts/quiet-run ch06 -- tools/check-verify-env-reset.sh` rc=0
- [x] 3.2 `tools/docker-tidy.sh --verify-env-check` を足す（design D4・D5・D6）。**この worktree の DB を作り直す**ので、`HARNESS_VERIFY=1 tools/verify-env.sh --plan` が `reset` でなければ理由を出して rc=2 で何もしない。次の 3 本を印つきで確かめる:
  `印の無い検証の準備と --plan は、片付けを呼ばない`（`TEST_DOCKER_TIDY_DOCKER` を引数を書き留める偽の命令にして、まず `HARNESS_VERIFY=1 tools/verify-env.sh` で書き留めた命令が 1 個以上あること（偽の命令が効いている対照）を見て、書き留め先を空にしてから `env -u HARNESS_VERIFY tools/verify-env.sh` と `HARNESS_VERIFY=1 tools/verify-env.sh --plan` を呼び、どちらも rc=0・新しく書き留めた命令が 0 個）/
  `片付けが落ちても、検証の準備は DB を作り直して続く`（`TEST_DOCKER_TIDY_DOCKER=false` で `HARNESS_VERIFY=1 tools/verify-env.sh` が rc=0・`片付けに失敗した` と片付けの rc を含む 1 行と `切った接続:` の 1 行・DB のコンテナが立っている）/
  `消えた作業ツリーのコンテナは、印つきの準備で外れ、その volume は残る`（2.1 の本物の 1 件と同じく `mktemp -d` の下の乱数の main に差し替え、`<main>-st97` のコンテナを置いて `HARNESS_VERIFY=1 tools/verify-env.sh` を呼び、rc=0・コンテナが無い・控えた id の volume が残る・標準エラーに `切った接続:` の 1 行。後始末は 2.1 と同じ印）。
  続けて（一覧は 2.1 で写してある）`HARNESS_VERIFY=1 tools/verify-env.sh` を走らせて本当に片付け、片付けるものが無い 2 回目の `tools/docker-tidy.sh` の秒数を `- 片付けの時間:` に書く（1 秒を超えれば D5 の反転条件）。
  検証: `EV tools/docker-tidy.sh --verify-env-check` rc=0 /
  `bash -c 'f=tools/docker-tidy.sh; for s in "片付けが落ちても、検証の準備は DB を作り直して続く" "消えた作業ツリーのコンテナは、印つきの準備で外れ、その volume は残る"; do grep -qF "Scenario: $s" "$f" || { echo "印が無い: $s"; exit 1; }; done; grep -qF "Scenario: 印の無い検証の準備と --plan は、片付けを呼ばない" "$f"'` rc=0 /
  `bash -c 'HARNESS_VERIFY=1 tools/verify-env.sh 2>/dev/null; [ -z "$(tools/docker-tidy.sh --dry-run 2>&1)" ]'` rc=0 /
  `bash -c 'grep -qE "^- 片付けの時間: .*[0-9]" openspec/changes/ch05-test-db-bloat/design.md'` rc=0

## Task 4: 殺すための探りの試験

- [x] 4.1 `crates/server/src/killed_probe_tests.rs`（新しいファイル。`lib.rs` の試験のモジュールの並びに 1 行）に design D2 の 2 本を書く:
  `ch05_killed_probe`（`#[ignore]`。`CH05_PROBE_AT` が `insert` / `committed`、`CH05_PROBE_MARK` に回の database と試験ごとの database の名前と段階を書く。`insert` は `pg_sleep(600)` を前に置いた 1 文の 20,001 件の INSERT に `/* ch05_probe */` の印を入れる。
  `committed` は回の database（`testdb::pool()`）と試験ごとの database（`testdb::isolated()`）の両方に 20,001 件を確定してから 600 秒眠る。環境変数が無ければ何もせずに通る）/
  `ch05_next_run`（`testdb::pool()` で繋いで `SELECT 1`。CH04 の片付けを起こすだけ）。INSERT の形は `deletion_tests.rs` の `stays_detail_query_uses_user_time_index` と同じにする。
  検証: `cargo build -p ashiato-server --tests` rc=0 / `NL ch05_ 2` rc=0 / `EV scripts/quiet-run ct -- cargo test -p ashiato-server --lib ch05_next_run` rc=0 /
  `tools/check-test-db-growth.sh --lint` rc=0

## Task 5: 殺した後の検査の台本

- [x] 5.1 `tools/check-test-db-killed.sh` を design D2 のとおりに書く（引数なし / `--self-test`）。冒頭のコメントに、2 つの時点・判定の順・差し替えの環境変数（`TEST_DB_KILLED_PROBE_CMD`・`TEST_DB_KILLED_NEXT_CMD`・`TEST_DB_KILLED_LIST_CMD`・`TEST_DB_KILLED_STAT_CMD`・`TEST_DB_KILLED_WAIT_S`）・
  **同じ worktree で別の `cargo test` と検証の準備と並べて走らせない**こと・なぜ要るか（CH05）を書く。
  `--self-test` は偽の探り・偽の次の回・偽の一覧と統計で、次の 7 本の Scenario を印つきで確かめる:
  `殺した回の database が次の回の後に残っていれば、名前を挙げて落ちる` / `ashiato database の行数が増えていれば落ちる` / `ashiato database の大きさだけが 1 MiB を超えて増えていれば落ちる` /
  `殺した回の database が見えなければ、空振りとして落ちる` / `探りの印が来なければ、待ちの上限で時点の名前を挙げて落ちる` / `次の回の試験が落ちれば、台本も試験の終了コードで落ちる` / `殺した回の分が片付き、ashiato が増えていなければ、台本は通る`。
  検証: `bash -n tools/check-test-db-killed.sh` rc=0 / `tools/check-test-db-killed.sh --self-test` rc=0 /
  `bash -c 'f=tools/check-test-db-killed.sh; for s in "殺した回の database が次の回の後に残っていれば、名前を挙げて落ちる" "ashiato database の行数が増えていれば落ちる" "ashiato database の大きさだけが 1 MiB を超えて増えていれば落ちる" "殺した回の database が見えなければ、空振りとして落ちる" "探りの印が来なければ、待ちの上限で時点の名前を挙げて落ちる" "次の回の試験が落ちれば、台本も試験の終了コードで落ちる" "殺した回の分が片付き、ashiato が増えていなければ、台本は通る"; do grep -qF "Scenario: $s" "$f" || { echo "印が無い: $s"; exit 1; }; done'` rc=0

## Task 6: 本物の試験で殺す

- [ ] 6.1 本物の試験で走らせる。台本は 2 つの時点（`insert` → `committed`）の印を出す（design D2）:
  `echo "Scenario: 体積の行を入れる途中で殺した回の database は、次の回の後に残らない"` / `echo "Scenario: 体積の行を確定した後に殺した回の database は、次の回の後に残らない"`。
  次の回の秒数（2 回分）と、殺す前のきれいな回の秒数（`ch05_next_run` を殺す前にも 1 回走らせて測る）を design の「計測の記録」の `- 次の回の秒数:` に書く。
  検証: `EV scripts/quiet-run killed -- tools/check-test-db-killed.sh` rc=0 /
  `bash -c 'f=tools/check-test-db-killed.sh; for s in "体積の行を入れる途中で殺した回の database は、次の回の後に残らない" "体積の行を確定した後に殺した回の database は、次の回の後に残らない"; do grep -qF "Scenario: $s" "$f" || exit 1; done'` rc=0 /
  `bash -c 'grep -qE "^- 次の回の秒数: .*[0-9]" openspec/changes/ch05-test-db-bloat/design.md'` rc=0
- [ ] 6.2 判定が本物で効くことを、片付けを外して確かめる: CH04 の片付けを呼ぶ行（`testdb.rs` の `sweep_previous_runs(…).await;`。回を作る関数と試験から呼ぶ片付けの 2 か所）を全部一時的に外して 6.1 の台本を走らせ、rc=1 で出力に `ashiato_run_` の名前が出ることを見て、元に戻す（commit しない）。
  結果（rc と挙がった名前の数）を design の「計測の記録」の `- 片付けを外した回:` に書く。
  検証: `bash -c 'set -a; [ -f .env ] && . ./.env; set +a; f=crates/server/src/testdb.rs; trap "git checkout -- $f" EXIT; [ "$(grep -cE "^[[:space:]]*sweep_previous_runs\(.*\)\.await;" "$f")" -ge 1 ] || exit 3; sed -i -E "s|^([[:space:]]*)(sweep_previous_runs\(.*\)\.await;)|\1// ch05-mutation: \2|" "$f"; out=$(tools/check-test-db-killed.sh 2>&1); rc=$?; echo "$out" | tail -20; [ "$rc" = 1 ] && grep -q ashiato_run_ <<<"$out"'` rc=0 /
  `git diff --quiet HEAD -- crates/server/src/testdb.rs` rc=0 /
  `bash -c 'grep -qE "^- 片付けを外した回: .*[0-9]" openspec/changes/ch05-test-db-bloat/design.md'` rc=0

## Task 7: CI と文書

- [ ] 7.1 `.github/workflows/ci.yml` の rust job の、CH04 の `tools/check-test-db-growth.sh` の行と CH06 の `tools/check-verify-env-reset.sh` の行の後ろに 4 行を足す: `tools/check-test-db-killed.sh --self-test`・`tools/docker-tidy.sh --self-test`・`tools/docker-tidy.sh --verify-env-check`・`tools/check-test-db-killed.sh`（この順。作り直しを殺した後の検査の本体より前に）（design D7。CI では `CI=true` で `--plan` が `reset`）。既存の行は変えない。
  `tools/check-test-db-killed.sh --self-test` の中に、ci.yml に自分の 2 行があることを見る検査を足す（`echo "Scenario: CI の rust job は、殺した後の検査を走らせる"`）。`tools/docker-tidy.sh --self-test` の中に、ci.yml に自分の 2 行（`--self-test`・`--verify-env-check`）があることを見る検査を足す（`echo "Scenario: CI の rust job は、片付けの self-test を走らせる"`）。
  検証: `tools/check-test-db-killed.sh --self-test` rc=0 / `tools/docker-tidy.sh --self-test` rc=0 /
  `bash -c 'grep -qF "Scenario: CI の rust job は、殺した後の検査を走らせる" tools/check-test-db-killed.sh && grep -qF "Scenario: CI の rust job は、片付けの self-test を走らせる" tools/docker-tidy.sh && grep -q "tools/check-test-db-killed.sh --self-test" .github/workflows/ci.yml && grep -qE "tools/check-test-db-killed.sh *$" .github/workflows/ci.yml && grep -q "tools/docker-tidy.sh --self-test" .github/workflows/ci.yml && grep -q "tools/docker-tidy.sh --verify-env-check" .github/workflows/ci.yml'` rc=0
- [ ] 7.2 `docs/testing.md` に 1 節（殺した後の検査と、同じ worktree で別の `cargo test`・検証の準備と並べて走らせないこと・作り直す検証の準備が消えた作業ツリーのコンテナとネットワークを片付けること・volume を消さないこと）、
  `docs/flow-gates.md` の検査の表に `tools/check-test-db-killed.sh`・`tools/docker-tidy.sh --self-test` の 2 行を足す。
  検証: `bash -c 'grep -q "check-test-db-killed" docs/testing.md && grep -q "docker-tidy" docs/testing.md && grep -q "check-test-db-killed" docs/flow-gates.md && grep -q "docker-tidy" docs/flow-gates.md'` rc=0
- [ ] 7.3 CI に足した 4 行の時間を手元で測り（`time` で 4 つを順に）、design の「計測の記録」の `- CI の行の時間:` に書く。
  検証: `bash -c 'grep -qE "^- CI の行の時間: .*[0-9]" openspec/changes/ch05-test-db-bloat/design.md'` rc=0

## Task 8: 全体の確かめ

- [ ] 8.1 Scenario の印と spec の検査。CH05 の MODIFIED の塊（足した 1 文・足した Scenario・`導出元:` の行を除く）が、CH06 のいまの Requirement 1（archive の前は取り込んだ CH06 の delta、後は正典）と同じであること（design D1。違えば写し直して 1 文を足し直す）。
  検証: `python3 scripts/check_scenarios.py . ch05-test-db-bloat` rc=0 / `openspec validate ch05-test-db-bloat --strict` rc=0 /
  `bash -c 'c=openspec/specs/verify-environment/spec.md; [ -f "$c" ] || c=openspec/changes/ch06-reset-verify-db/specs/verify-environment/spec.md; [ -f "$c" ] || exit 3; f() { grep -vE "^(導出元:|ただし、作業ツリーが消えた|[[:space:]]*$)"; }; diff <(sed -n "/^### Requirement: 印があれば/,/^#### Scenario: 消えた作業ツリーのコンテナは/p" openspec/changes/ch05-test-db-bloat/specs/verify-environment/spec.md | sed "\$d" | f) <(sed -n "/^### Requirement: 印があれば/,/^### Requirement: [^印]/p" "$c" | sed "/^### Requirement: [^印]/d" | f)'` rc=0
- [ ] 8.2 変えないものが変わっていない: CH05 の commit（`origin/main` と、取り込んだ CH04・CH06 の SHA（`deps.json` の `sha`、無ければ `spec_sha`）のどれにも入っていない commit）が触ったファイルが許したものだけで、`crates/server/src/lib.rs` と `tools/verify-env.sh` の差分は行の追加だけ。
  許すもの: `tools/check-test-db-killed.sh`・`tools/docker-tidy.sh`・`tools/verify-env.sh`・`crates/server/src/killed_probe_tests.rs`・`crates/server/src/lib.rs`・`.github/workflows/ci.yml`・`docs/`・`openspec/changes/ch05-test-db-bloat/`
  （`testdb.rs`・試験の期待値・`tests/server_startup.rs`・既存の `down -v` の台本・`tools/dev.sh`・`tools/check-test-db-growth.sh`・`tools/ports.sh`・`tools/env-init.sh`・`tools/check-verify-env-reset.sh`・`migrations/` は入らない）。
  検証: `bash -c 'set -o pipefail; ex=$(python3 -c "import json; d=json.load(open(\"openspec/changes/ch05-test-db-bloat/deps.json\")); print(\" \".join(v.get(\"sha\") or v[\"spec_sha\"] for v in d.values()))"); files=$(git log --no-merges --name-only --format= HEAD --not origin/main $ex | sort -u | grep .); [ -n "$files" ] || exit 3; bad=$(grep -vE "^(tools/check-test-db-killed\.sh|tools/docker-tidy\.sh|tools/verify-env\.sh|crates/server/src/killed_probe_tests\.rs|crates/server/src/lib\.rs|\.github/workflows/ci\.yml|docs/|openspec/changes/ch05-test-db-bloat/)" <<<"$files"); [ -z "$bad" ] || { echo "触ってはいけない: $bad"; exit 1; }'` rc=0 /
  `bash -c 'set -o pipefail; ex=$(python3 -c "import json; d=json.load(open(\"openspec/changes/ch05-test-db-bloat/deps.json\")); print(\" \".join(v.get(\"sha\") or v[\"spec_sha\"] for v in d.values()))"); ! git log --no-merges -p --format= HEAD --not origin/main $ex -- crates/server/src/lib.rs tools/verify-env.sh | grep -E "^-[^-]"'` rc=0
- [ ] 8.3 server の試験の全体が、作り直した DB の上で通る（`tests/server_startup.rs` が移行を当てる）。
  検証: `bash -c 'HARNESS_VERIFY=1 tools/verify-env.sh'` rc=0 / `EV scripts/quiet-run ct -- cargo test -p ashiato-server` rc=0
