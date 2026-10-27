# CH05 design — 試験の DB が打ち切った run の残りで膨れないようにする

> 第 1 回の深掘りの答え（Q1〜Q7。Q7 だけ推奨と違う）と第 2 回の答え Q8（推奨と同じ: 作り直し・見送りを外し CH06 に任せる）の後に、CH06 の gate 済みの spec を読んで書き直した（2026-10-25）。
> CH04 は `9fe8c5e`（PR #156）、CH06 は `c666673`（PR #164）の design・spec を読んだ（`deps.json`。書き出しは `docs/briefs/deps/`）。CH04・CH06 の決定は「CH04 D<n>」「CH06 D<n>」で引き、本文を写さない。
> **D 番号は振り直した**。Q8 の前の版（`837e79be`）からの対応: 旧 D6 → D2・旧 D7 → D3・旧 D9 → D4 と D5・旧 D8 → D7。旧 D1〜D5（閾値・作り直し・作業ツリーの種類・置き場・作り直しの self-test）は Q8 で外した。
> proposal と deep.md の Q8 の節は旧番号（D6・D9）で引いている。

## Context

動機は `proposal.md` の Why。CH04 と CH06 の後（`requires: [CH04, CH06]`）の形:

- 試験は回ごとの database（`ashiato_run_<12 桁>` と `_tpl`・`_x_…`・`_tmp_…`）に書き、`ashiato` には書かない。前の回の database は次の回の初めに片付く（CH04 D1・D6）。
  回が走っている印は `postgres` database で取る session の助言ロック `pg_advisory_lock(4820253, hashtext('<id>'))`、回の作成と片付けの直列化は `pg_advisory_lock(4820252)`。試験ごとの database は `testdb::isolated()`（CH04 D4）。
  1 本の問い合わせの上限は 60 秒で、`ASHIATO_TEST_STATEMENT_TIMEOUT`（`0` で上限なし）で替える（CH04 D7）。どれも `9fe8c5e` の `crates/server/src/testdb.rs` にある
- `tools/verify-env.sh` は、ハーネスが検証のコマンドの前に毎回 `HARNESS_VERIFY=1` で呼ぶ。CH06 の後は、worktree か `CI=true` なら毎回この worktree の DB のコンテナと volume を消して空から立て直す（CH06 D1・D2）。
  順は `--plan` の判定 → `. tools/ports.sh` → `tools/env-init.sh` →（`reset` なら）`reset_db` → `clear_blocking_network` → `docker compose up -d --wait db` → 役割を整える（`c666673` の `tools/verify-env.sh`）
- CH06 は worktree ごとの DB の network を、port から決めた被らない subnet で立てる（CH06 D5・D6）。台帳の番号は消えた worktree のものを使い回すので、番号を継いだ木は、消えた worktree の network と同じ subnet を取りに行く。
  その network にコンテナが繋がっていれば（消えた worktree の DB が残っている）、CH06 は何も消さずに rc=1 で止め、外し方を出して人に返す（CH06 D5。CH06 Q3 の「消えた worktree の残りは片付けない」）
- 消えた作業ツリーのコンテナは、2026-10-25 にこの機械で 15 個（動いたままのものが 4 つ）。作業ツリーが消えるたびに溜まる（`premises.md:16`）。どれも compose の project と作業ツリーの path の印を持つ。volume は名前の無い volume（`premises.md:17`）
- `client_connection_check_interval` は既定の 0。1 文の INSERT は、client が殺されても backend が文を終えて確定することがある

## Goals / Non-Goals

**Goals:**
- 試験を途中で殺しても次の回の後に残りが無いことを、台本と CI が確かめる（`test-database` の ADDED）。CH04 D1 の「殺されるとロックが外れ、次の回が片付ける」を初めて実物で固定する
- 作業ツリーが消えたのに残っている docker のコンテナと、コンテナの無いネットワークを、作り直す検証の準備が片付ける（`verify-environment` の ADDED。Q7）。volume は消さない
- CH06 の「ほかの compose project には触らない」を、片付けを除く形に書き直し、両方が archive された後の正典が食い違わないようにする（`verify-environment` の MODIFIED。Q8・review/spec.md R21）

**Non-Goals:**
- 膨れた `ashiato` の作り直しと、使っている途中の見送り（Q8。CH06 が検証のたびに空から作り直す）。`tools/ports.sh` の作業ツリーの種類（Q8）
- docker の volume の片付け（Q7。浮いた volume は名前の無い volume で、どの作業ツリーのものか区別できない。片付けたコンテナの volume も残る）。印の無いコンテナ（`st04-testdb` など）と、別の repo の compose の資源
- 既存の `down -v` の台本（smoke・stack・check-immutable・check-panic-log）に手を入れること（Q7）。ST23 の lock timeout・権限のエラーの原因調べ（Q7）
- CH06 の作り直しの規則（`--plan` の判定・`reset_db`・`clear_blocking_network`・`tools/env-init.sh`・subnet）を変えること。CH04 の試験の足場（`testdb.rs`・`tools/check-test-db-growth.sh`）を変えること
- 消せない表のトリガ・試験の期待値（変えない）

## Decisions

### D1（仮）. 置き場 —— 殺した後の検査は `test-database`、片付けは `verify-environment`

- 殺した後の検査は試験の足場（CH04 の `test-database`）の約束なので、`test-database` に ADDED で 1 つ足す。`## Purpose` は持たず、CH04 の Requirement を MODIFIED しない
- 片付けは検証の準備（`tools/verify-env.sh`）の振る舞いで、CH06 の「ほかの project には触らない」と同じ Requirement の並びで読めないと、正典の読み手が矛盾に見える。そこで `verify-environment` に ADDED で置き、同じ capability で CH06 の Requirement 1 を MODIFIED する（D6）。
  proposal の初めの版（Q8 の後）は片付けも `test-database` に置くと書いていた。CH06 の gate 済みの spec を読んで、この形に直した（proposal の Capabilities も直した）
- archive は CH04 と CH06 の後（`verify-environment` も `test-database` もまだ正典に無い。`openspec validate` は MODIFIED が当たる先の無いことを情報として出す）

MODIFIED の塊は正典に無い capability に当たるので、`spec_merge.py` の 3-way の base ができない（archive は塊を丸ごと置き換える）。CH06 が c666673 の後で Requirement 1 の本文か Scenario を直すと、CH05 の archive がそれを戻してしまう。tasks 8.1 が、足したもの以外が CH06 のいまの塊と同じことを `diff` で見る。

反転条件: CH06 が archive の前に `verify-environment` の名前か Requirement 1 の名前・本文・Scenario を変えたら、MODIFIED を写し直して 1 文と Scenario を足し直す（merge gate の「依存の spec:」と tasks 8.1 で見える）。CH06 の後に片付けが検証の準備の外（下流の Task の開始前など）に移るなら、`test-database` か新しい capability へ移す change を立てる。

### D2（仮）. 殺した後の検査 —— 探りの試験を印で同期して SIGKILL する

`tools/check-test-db-killed.sh`（引数なし / `--self-test`）:

- 探りの試験 `ch05_killed_probe`（`crates/server/src/killed_probe_tests.rs`。`#[ignore]`。通常の `cargo test` では走らない）は、`CH05_PROBE_AT`（`insert` / `committed`）と `CH05_PROBE_MARK`（印のファイル）を読み:
  - 回の database（`testdb::pool()`）と試験ごとの database（`testdb::isolated()`）を取り、その名前を印のファイルに書く
  - `insert`: 回の database に、`deletion_tests.rs` の体積の試験（`stays_detail_query_uses_user_time_index`）と同じ形の 20,001 件の INSERT を、`pg_sleep(600)` を前に置いた 1 文で走らせる（文が走っている時間を作る）。文に `/* ch05_probe */` の印を入れる。印のファイルに `inserting` を書いてから文を打つ
  - `committed`: 回の database と試験ごとの database の両方に 20,001 件を入れて確定し、印のファイルに `committed` を書いてから 600 秒眠る
  - 環境変数が無ければ何もせずに通る（`--include-ignored` で走っても落ちない）
- 台本は、`cargo test -p ashiato-server --lib --no-run --message-format=json` で試験の実行ファイルの path を取り、`setsid` で直接起こす（`--ignored --exact killed_probe_tests::ch05_killed_probe --nocapture`。`ASHIATO_TEST_STATEMENT_TIMEOUT=0` —— CH04 D7 の 60 秒の上限で探りが先に落ちないように）。
  `insert` は印の `inserting` と、回の database の `pg_stat_activity` に `ch05_probe` の印のある `active` の文が見えるのを待つ。`committed` は印の `committed` を待つ。待ちの上限は 120 秒（`TEST_DB_KILLED_WAIT_S` で替えられる。越えたら「探りの印が来なかった（探りが走らなかった・0 本の絞り込み）」と時点の名前を出して rc=1）。その後、プロセスグループごと `kill -KILL`
- **探りを起こす前に**、一覧 S0（`pg_database` の名前）・`ashiato` の `core.event` の行数（表が無ければ 0 —— CH06 の後の `ashiato` は空から立ち、移行は server の試験の初回まで当たらない）・`pg_database_size('ashiato')` を読み、殺した後に一覧 S1 を読む
  （殺す直前に S0 を読むと、探りの database が S0 に入って差が空になる）。殺した回の database = S1 にあって S0 に無いもの（印のファイルの名前もそこに入っていることを見る）。1 つも無ければ「殺した回の database が見えない（検査が空振り）」で rc=1
- 次の回は `cargo test -p ashiato-server --lib ch05_next_run`（同じファイルの、`testdb::pool()` で繋いで `SELECT 1` するだけの 1 本。CH04 の片付けを起こす）。秒数を測って標準出力に出す
- 判定（順に。最初に外れたもので落ちる）: 次の回の rc → 殺した回の database のうち、次の回の後の一覧 S2 に残るもの（名前を挙げる）→ `ashiato` の行数が殺す前と違う（前と後を挙げる）→ `ashiato` の大きさが殺す前から 1 MiB を超えて増えた（前と後を挙げる）
- 2 つの時点を順に行う（`insert` → `committed`）。どちらかが落ちれば、その時点の名前を出して落ちる
- 差し替え（`--self-test`）: `TEST_DB_KILLED_PROBE_CMD`（探り）・`TEST_DB_KILLED_NEXT_CMD`（次の回）・`TEST_DB_KILLED_LIST_CMD`（一覧）・`TEST_DB_KILLED_STAT_CMD`（`ashiato` の行数と大きさ）。CH04 D8 の `TEST_DB_CMD`・`TEST_DB_LIST_CMD` と同じ形
- **同じ worktree で別の `cargo test` と並べて走らせない**（並んだ回の database が S1 と S0 の差に入る）。冒頭のコメントに書く。検証の準備（`HARNESS_VERIFY=1 tools/verify-env.sh`）とも並べない（CH06 が DB ごと作り直す）
- 大きさの許し（1 MiB）は、触らない database でも統計や空き領域の地図で大きさが少し動くことがあるため。行数は厳密に見る
- `insert` の時点で殺すのは、行を 1 つも作る前の走っている文（`pg_sleep(600)` が FROM の外側にあり、眠り終えるまで `generate_series` の行を作らない。review/code.md R19 の実測で `pg_relation_size('core.event')` = 0）。
  書きかけの未確定の行がある状態は見ない。見るなら別の時点として足し、この D2 を変える
- 「INSERT の途中で殺せば巻き戻って行は残らない」とは書かない（`client_connection_check_interval` が 0 なので、文は終わって確定しうる）。見るのは、確定したかに依らず次の回の後に残らないことだけ
- Q8 の前の版にあった「`committed` の眠りの間に作り直しの判定が見送る」の確かめは外した（作り直しの判定を作らない）
- 退けた案: 本物の体積の試験（`stays_detail_query_uses_user_time_index` など）を時間で殺す —— 文が 1 秒未満で終わり、殺す時点を選べない。探りの試験に眠りを入れて印で同期すれば、時点が揺れない。
  CH04 の `tools/check-test-db-growth.sh` に足す —— CH04 の台本は CH04 の持ち物で、CH05 は変えない（壊してはいけないもの）

反転条件: CI で `setsid` やプロセスグループの kill が効かない（試験の実行ファイルが残る）なら、実行ファイルの pid を直接 `kill -KILL` する形に替える。次の回の秒数が殺す前のきれいな回の 10 倍を超える例が出たら、秒数も合否に入れるかを本人に返す（Q6）。
CH04 の gate・fix で鍵の番号か回の名前が変わったら、`testdb.rs` に合わせる（tasks 1.1 が読み直す）。

### D3. 新しい試験の名前の頭は `ch05_`

`crates/server/src/killed_probe_tests.rs` の 2 本（`ch05_killed_probe`・`ch05_next_run`）。`#[ignore]` の探りは `cargo test` の既定では走らず、本数の数えは `-- --list --include-ignored` で見る。
CH04 の `--lint` は `CREATE DATABASE` と書き出しの試験のファイル（`export*_tests.rs`）の `testdb::pool()` を見るが、このファイルはどちらにも当たらない。

### D4（仮）. 片付けの台本 `tools/docker-tidy.sh`

本人の Q7 の答え（推奨と違う）: 使われていないコンテナ・ネットワークを片付ける（volume は消さない）。片付けてよいのは「作業ツリーが消えたことを機械で確かめられるもの」だけ（選択肢の detail）。

- 口は 4 つ: 引数なし（片付ける）・`--dry-run`・`--self-test`・`--verify-env-check`（D5）
- **main の名前と親のフォルダ**: `git worktree list --porcelain` の最初の `worktree` 行（main の checkout）から取る（`tools/ports.sh` の `_ports_main` と同じ答えになる。`ports.sh` は触らない —— Q8）。
  消えたかどうかは、`git worktree list --porcelain` の path の一覧に無く、かつ `[ -e <path> ]` でない、の両方で見る（`git worktree prune` の前の path も、消えた path も同じに扱う。片方だけだと、手で消したフォルダや、prune の前の残りで食い違う）
- **コンテナ**: `docker ps -a --filter label=com.docker.compose.project --format …` と `inspect` で project と working_dir の印を読み、spec の条件を全部満たすものだけを `docker rm -f <id>`（**`-v` を付けない**。名前の無い volume は残る —— `premises.md:17`）。
  project の名前を `<main>-` で始まるものに絞るのは、同じ親のフォルダの別の repo（`lifeLog2`・その `agent-*`）と、CH06 の検査が立てる compose の project（`ch06-sentinel-<乱数>`。CH06 D8 の本体 5.）を外さないため
- **ネットワーク**: `docker network ls --filter label=com.docker.compose.project` の project を読み、spec の条件（`<main>-` で始まる・繋がったコンテナが無い・その project の印のコンテナが無い・生きている作業ツリーの名前（basename を小文字にしたもの。compose の project の名前の決め方）と同じでない・`-verify` でない）を満たすものだけを `docker network rm`。
  「繋がったコンテナ」は `docker ps -aq --filter network=<名前>` で読む（止まっているコンテナも出る。CH06 の `clear_blocking_network` と同じ）。`docker network inspect` の `.Containers` は止まったコンテナを数えない（2026-10-25 実測。review/spec.md R40）ので使わない。
  外すコンテナを決めた後に、そのコンテナを除いて「繋がったコンテナが無い」を判定する（外したコンテナのネットワークも同じ回で空き、`--dry-run` も同じ一覧を出す）
- 並んだ片付け（別の作業ツリーの検証の準備が同時に走る）で、もう無いものを外そうとして `No such container` / `No such network` になったら、外せたものとして扱う
- 出すもの: 外したものごとに `docker-tidy: コンテナ <名前> を外した（作業ツリー <path> が無い）` / `docker-tidy: ネットワーク <名前> を外した`。`--dry-run` は `docker-tidy: 外す: …` の形で同じものを出す
- docker を呼ぶ命令は `TEST_DOCKER_TIDY_DOCKER`（既定 `docker`）、main の path は `TEST_DOCKER_TIDY_MAIN`、生きている作業ツリーの path の一覧は `TEST_DOCKER_TIDY_WORKTREES`（1 行 1 path のファイル）で差し替えられる（`--self-test` と `--verify-env-check` が使う）。
  `tools/verify-env.sh` から呼ばれたときも、この 3 つは環境からそのまま届く（`verify-env.sh` は何も足さない）
- `--self-test`:
  - 判定は偽の docker（引数を書き留め、決まった一覧と印・`No such container`・失敗を返す）で確かめる: 消えた・残っている・main・`-verify`・印なし・別の repo（`ch06-sentinel-` を含む）・親のフォルダが違う、のコンテナと、空の・生きている作業ツリーの・`-verify` の・外さないコンテナの繋がった・印の無い・別の repo のネットワーク
  - 本物の docker で 1 件（`作業ツリーが消えたコンテナは、動いていても外される`・`外したコンテナの名前の無い volume は残る`）: **main を差し替える** —— `d=$(mktemp -d)`・`TEST_DOCKER_TIDY_MAIN=$d/ch05tidy<乱数>`（作る）・`TEST_DOCKER_TIDY_WORKTREES` は main だけ。
    project `ch05tidy<乱数>-st97`・working_dir `$d/ch05tidy<乱数>-st97`（作らない path）の印の動いているコンテナ（`postgres:17-alpine` を `sleep` で起こす。image の `VOLUME` で**名前の無い volume** が付く。その id を控える）と、同じ project の印のネットワーク（`docker network create --label …`）を作り、片付けを走らせ、コンテナとネットワークが無く、控えた id の volume が残ることを見る。
    main の名前が乱数なので、この機械の本物の残り（`ashiato2-*`）には当たらず、他の作業ツリーの片付けもこの 1 件を外さない。self-test が作るものには印 `dev.ashiato.ch05-tidy-probe=1` を付け、`trap … EXIT` で自分のコンテナ・ネットワークと控えた id の volume を消す。
    落ちて trap が走らなかった残りは、次の self-test の初めに印で探して消す（コンテナは `rm -fv`）。これは片付けの本体ではなく self-test の後始末で、spec の命令の Scenario が見るのは片付けの本体が打つ命令。ネットワークを作れない（割り当てが尽きた）ときは理由を出して rc=1
  - ci.yml の rust job に自分の行があることを見る（`CI の rust job は、片付けの self-test を走らせる`）
- 退けた案: 片付けを下流の Task の 1 回だけにする —— 作業ツリーが消えるたびにまた溜まる（Q7 の答えは「片付ける」で、1 回の掃除ではない）/ `docker container prune`・`docker network prune` —— 作業ツリーが残っている止まったコンテナと、別の repo のものまで消す /
  作業ツリーの path が無いだけで外す —— `git worktree list` にあって一時的に path が見えない木を外しうる

反転条件: 作業ツリーが残っているのに外された例が出たら、`git worktree list` だけで見る形に寄せる。CH06 か harness2 が作業ツリーの消え方か compose の project の名前の決め方を変えたら、条件を合わせる。
本人が volume も片付けると決めたら、外したコンテナの volume の id を記録する形を先に足す（いまは手がかりが残らない —— Risks）。

### D5（仮）. 片付けを呼ぶ場所 —— 作り直す（`reset`）ときだけ、`reset_db` の前

`tools/verify-env.sh` に 1 行（と注記）を足す。CH06 の流れの、`tools/env-init.sh` の後・`reset_db` の前に、判定が `reset` のときだけ `tools/docker-tidy.sh` を呼ぶ。
rc≠0 なら `verify-env: 片付けに失敗した（docker-tidy の rc=<n>）。続ける` を標準エラーに出して続ける（`片付けに失敗した` と rc は spec が持つ。片付け自身の行はこの字面を使わない）（片付けの対象は他の作業ツリーの残りで、この worktree の検証の成否とは別。落とすと、他の作業ツリーの残りでこの worktree が BLOCKED_INFRA になる）。

- **`reset` のときだけにする理由**: CH06 の Requirement「作り直すのは、印があり、worktree か CI のときだけ」は、それ以外を「今までどおり、DB を起動して…役割を整える**だけ**」と言う。毎回の呼び出しで片付けると、その Requirement も書き直すことになる。
  検証は worktree で `HARNESS_VERIFY=1` で呼ばれる（`scripts/verify-run`）ので、新しい作業ツリーの初回の準備は必ず `reset` を通り、片付けの役（消えた作業ツリーの分を空ける）は果たせる。`--plan` は docker に触らない（CH06 D2）ので呼ばない
- **`clear_blocking_network` の前に置く理由**: 台帳の番号を継いだ木は、消えた作業ツリーの network と同じ subnet を取りに行く（Context）。そこに消えた作業ツリーの DB が繋がったままだと、CH06 は rc=1 で人に返す。
  片付けが先に走ると、そのコンテナとネットワークは（`<main>-` の project で、作業ツリーが消えていれば）外れていて、CH06 の確かめは残ったもの（片付けの条件に当たらないもの）だけを見る。CH06 の rc=1 の振る舞い自体は変えない
- `reset_db` の前に置くのは、片付けが落ちても DB を消す前の状態で続けられるため（後でも結果は同じ）
- CH07 が同じ場所（`reset_db` の直前）に `reset_done=` を足した後は、片付けの行を `reset_done=` の後・`reset_db` の前に置く（片付けは `reset_done` に触らず、`HARNESS_RESET` の宣言は作り直した DB だけを指す。review/code.md R22）
- CI（`CI=true` の main の checkout）でも `reset` なら走る。CI に消えた作業ツリーは無いので何も外さない
- 退けた案: 毎回（印が無いときも）片付ける —— CH06 の Requirement 2 の「だけ」を MODIFIED で書き直すことになり、開発用の起動（main の checkout）まで docker を見に行く /
  `clear_blocking_network` の中で、繋がったコンテナが消えた作業ツリーのものなら外す —— CH06 の関数の中身を書き換える（CH06 Q3 の決定を CH05 が覆す）

反転条件: 印の無い呼び出し（人が手で走らせる `tools/verify-env.sh`）だけで作業ツリーを作って DB を立てる経路が出て、そこでネットワークの割り当てが尽きる例が出たら、毎回呼ぶ形にして CH06 の Requirement 2 も MODIFIED する。
CH07 の宣言が片付けの結果も宣言の対象にすると決めたら、片付けの行を `reset_done=` の前に移す。片付けが検証の準備を 1 秒以上遅くする（`docker ps -a` と `inspect` の費用。計測の記録）なら、1 日に 1 回（印のファイルの時刻で見る）にする。

### D6. CH06 の Requirement 1 の MODIFIED は 1 文だけ

- CH06 の Requirement「印があれば、検証の準備がこの worktree の DB を空から作り直す」の本文と Scenario 5 本を、`c666673` の spec から 1 字も変えずに写し、「ほかの compose project のコンテナ・volume と、compose の外のコンテナ・volume には触らない」の後に、
  片付けの例外の 1 文（`<main の名前>-` の消えた作業ツリーのコンテナとネットワークは片付けが外す。volume は消さない）を足す。見出しは変えない（MODIFIED の同一性）。`導出元:` に CH05 を足す
- 例外の 1 文を確かめる Scenario を 1 本足す（`消えた作業ツリーのコンテナは、印つきの準備で外れ、その volume は残る`。`tools/docker-tidy.sh --verify-env-check` が本物の docker で見る）。写した 5 本の印は CH06 の `tools/check-verify-env-reset.sh` のまま
- CH06 の Requirement「worktree ごとの DB の network は、木ごとに被らない subnet で立つ」は書き直さない。片付けは `<main>-` の project に絞り、その前に走るだけで、CH06 の Scenario（`ch06-sentinel-` の network とコンテナ）はどれもそのまま成り立つ（D5）
- CH06 の Requirement「作り直すのは、印があり、worktree か CI のときだけ」も書き直さない（片付けは `reset` の枝だけ。D5）
- CH06 の「作り直せなければ、何も消さずに非 0 で終わる」と、subnet の Requirement の「何も消さずに…非 0」も書き直さない。どちらの「何も消さず」も、この worktree の DB のコンテナ・volume と塞いでいる network を指す。
  片付けはその判定の前に終わり、外したものは判定が非 0 でも戻さない —— このことは CH05 の ADDED の本文に 1 文で書いた（MODIFIED を増やすと、CH06 の直しを戻すおそれ（D1）が増える）

### D7. CI の rust job の 4 行と、文書

- `.github/workflows/ci.yml` の rust job の、CH04 の `tools/check-test-db-growth.sh` の行と CH06 の `tools/check-verify-env-reset.sh` の行の後ろに: `tools/check-test-db-killed.sh --self-test`・`tools/docker-tidy.sh --self-test`・`tools/docker-tidy.sh --verify-env-check`・`tools/check-test-db-killed.sh`（この順。作り直しを殺した後の検査の本体より前に）。既存の行は変えない
- `--verify-env-check` も CI に置く（CH06 の本体と同じく、CI では `CI=true` で `--plan` が `reset` になり、docker もある）。置かないと、後の change が `verify-env.sh` の片付けの 1 行を消しても CI が緑のまま（review/spec.md R39）。DB を作り直すので、DB を使う後ろの段（`check-test-db-killed.sh` の本体）より前に置き、作り直した DB は役割まで整っている
- `docs/testing.md` に 1 節（殺した後の検査と並べて走らせないこと・作り直す検証の準備が消えた作業ツリーのコンテナとネットワークを片付けること・volume を消さないこと）、`docs/flow-gates.md` の検査の表に `tools/check-test-db-killed.sh`・`tools/docker-tidy.sh --self-test` の 2 行

### D8（仮）. 外すコンテナの 1 行に、残す volume の名前を出す

review/code.md R20: DB の volume は名前の無い volume で、印は `com.docker.volume.anonymous` だけ。コンテナを外すと、どの作業ツリーの volume だったかを docker から引けなくなる（Q7 は volume を残すが、結び付きは外した時点で捨てられる）。
失わない側で仮に進める: 外すと決めたコンテナごとに `docker inspect` の Mounts から volume の名前を読み、外した 1 行（と `--dry-run` の 1 行）の末尾に `残す volume: <名前…>`（無ければ `なし`）を足す。
inspect に問えないときは、そのコンテナを外さずに理由を出して rc=1（結び付きを書き残せないまま外さない）。検証の準備からの呼び出しでは、この行は `scripts/verify-run` の検証のログに残る。
spec の「1 つごとに名前と、コンテナならその作業ツリーの path を標準エラーに 1 行出す」に足すだけで、本文は変えない。`--self-test` が偽の docker と本物の docker の 1 件で、この 1 行に控えた volume の名前が出ることを見る。

反転条件: 本人が「結び付きは要らない」と決めたら、inspect と行の末尾を外す。「行では足りない（ログの残る作業ツリーも消える）」と決めたら、main の checkout の下の記録のファイルへ 1 行ずつ足す形にする（spec を MODIFIED する）。

## Risks / Trade-offs

- [CH04 の鍵の番号・回の名前・`isolated()` が gate・fix で変わる] → D2 は CH04 の D 番号で引き、反転条件に書いた。下流の入口が CH04 の gate を通った SHA を取り込み、tasks 1.1 で `testdb.rs` から読み直す
- [CH06 の `tools/verify-env.sh` の流れ（関数の名前・順）が gate・fix で変わる] → 片付けの 1 行の置き場は「`reset` のとき、DB を消す前・network を確かめる前」で決め、関数の名前に頼らない。tasks 1.1 で `c666673` の後の形を読み直す
- [殺した後の検査が CI の時間を伸ばす] → 探りの 2 回と次の回の 2 回。試験の実行ファイルは `cargo test --workspace` で組み済み。tasks 7.3 で CI の 4 行の時間を手元で測って書く
- [`ashiato` の大きさの許し 1 MiB が、本物の残りを見逃す] → 体積の行 20,001 件は数 MB で、許しより大きい。行数は厳密に見る
- [片付けたコンテナの volume は残るが、どのコンテナのものだったかの手がかりが消える（名前の無い volume）] → 本人が選んだ形（Q7。volume は消さない）。片付けの 1 行に作業ツリーの path と残す volume の名前を出す（D4・D8（仮））。ログが残らない呼び出しでは、作られた時刻で `docker volume ls` から探す
- [動いている消えた作業ツリーの DB（ch01・ch02・st13・st33）を止める] → 作業ツリーが無いので、その DB に繋ぐ試験もサーバも無い。中身は volume に残る（D4）
- [印の無い呼び出しでは片付けない] → 検証は印つきで呼ばれる。印の無い経路で枯渇する例が出たら D5 の反転条件

## Migration Plan

移行は足さない。merge の後、各 worktree の次の印つきの検証の準備が、消えた作業ツリーのコンテナとネットワークを外す（初回はこの機械で 15 個前後）。
戻すときは `tools/verify-env.sh` の 1 行を外せば CH06 の振る舞いに戻る（外したコンテナは戻らないが、volume は残っている）。

## 計測の記録

下流が書く（鍵の後ろに数字を書く。tasks の検証が `^- <鍵>: [0-9a-z]` を見る）:

- 片付けたもの: 2026-10-27 の `tools/docker-tidy.sh --dry-run` が挙げたもの（本物は Task 3.1 の組み込み後の最初の準備で外れる）。コンテナ 19 個（ashiato2-ch07-db-1・ashiato2-st48-db-1・ashiato2-ch06-db-1・ashiato2-st29-db-1・ashiato2-ch04-db-1・ashiato2-ch01-db-1・ashiato2-ch02-db-1・ashiato2-st13-db-1・ashiato2-st33-db-1・ashiato2-st47-db-1・ashiato2-st08-db-1・ashiato2-st14-db-1・ashiato2-st21-db-1・ashiato2-st25-db-1・ashiato2-st12-db-1・ashiato2-st28-db-1・ashiato2-st22-db-1・ashiato2-st22-ra-db-1・ashiato2-st19-db-1）、ネットワーク 8 個（ashiato2-ch02_default・ashiato2-ch04_default・ashiato2-ch06_default・ashiato2-ch07_default・ashiato2-st13_default・ashiato2-st29_default・ashiato2-st33_default・ashiato2-st48_default）
- 片付けの時間: 0.16 秒（2026-10-27。片付けるものが無い 2 回目の `tools/docker-tidy.sh`。1 秒以内で D5 の反転条件に当たらない）
- 次の回の秒数: 殺す前のきれいな回 0.8 秒、`insert` の後 1.2 秒、`committed` の後 1.3 秒（2026-10-27。10 倍の目安に遠く、D2 の反転条件に当たらない）
- 片付けを外した回: rc=1。`insert` の時点で `ashiato_run_b6d93ce233e7`・`…_tpl`・`…_x_95503bb9d443` の 3 個が次の回の後に残ると挙がった（2026-10-27。`sweep_previous_runs` の 2 行を外した。外した後は git checkout で戻した）
- CI の行の時間: `tools/check-test-db-killed.sh --self-test` 3.6 秒・`tools/docker-tidy.sh --self-test` 2.0 秒・`tools/docker-tidy.sh --verify-env-check` 16.3 秒・`tools/check-test-db-killed.sh` 10 秒（試験の実行ファイルがビルド済み）/ 16.4 秒（`killed_probe_tests.rs` を touch した再ビルド込み）、計 32〜38 秒前後（2026-10-27、final review R14 の測り直し。self-test の 2 本は `$EPOCHREALTIME` の差、`--verify-env-check` は `.env` を読んで `time`、本体は `scripts/quiet-run killed` の表示（10 秒）と `time scripts/quiet-run killed …`（16.4 秒）。全部 rc=0。前に書いた 16.3・52.9 秒のうち本体の 52.9 秒は証跡が無く、置き換えた）
