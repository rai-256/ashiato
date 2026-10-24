# CH05 design — 試験の DB が打ち切った run の残りで膨れないようにする

> 第 1 回の深掘りの推奨の答え（Q1〜Q7）で先に書いた。本人の答えが推奨と違えば、その Q を引く D を書き直す（各 D の反転条件）。
> CH04 の決定は `feat/ch04-test-db-growth` の `e27a521a` の `openspec/changes/ch04-test-db-growth/design.md` で読んだ。CH04 の D 番号で引く。

## Context

動機は `proposal.md` の Why。いまの形（`premises.md`。2026-10-24）:

- `tools/verify-env.sh` は、ハーネスが検証のコマンドの前に毎回呼ぶ（`HARNESS_VERIFY=1`。`.harness2/scripts/evidence.py`）。この worktree の compose の DB を起動し、`tools/db-roles.sh` で役割を整える。
  ハーネスの取り決めは「repo はここで溜まった状態を作り直してよい。何を作り直すかは repo が決める」
- CH04 の後（`requires`）、試験は回ごとの database（`ashiato_run_<12 桁>` と `_tpl`・`_x_…`・`_tmp_…`）に書き、`ashiato` には書かない。前の回の database は次の回の初めに片付く（CH04 D6）。
  回が走っている印は `postgres` database で取る session の助言ロック `pg_advisory_lock(4820253, hashtext('<id>'))`（CH04 D1）、回の作成と片付けの直列化は `pg_advisory_lock(4820252)`（CH04 D2）
- `ashiato` を使い続けるのは試験の外の 3 つ（`tests/server_startup.rs`・縦串・`tools/dev.sh`）で、どれも使う前に自分で移行を当てる（`ashiato-server migrate`）。移行は拡張を作らない（`CREATE EXTENSION` は 0 件）
- 各 worktree の `ashiato` には、CH04 の前に積もった行が残る（ch03 214 MB など）。CH04 はこれに触らない（CH04 Q4）
- `client_connection_check_interval` は既定の 0（ch04 の DB で `SHOW`）。1 文の INSERT は、client が殺されても backend が文を終えて確定することがある
- docker の既定のネットワークの割り当てが尽きていて、新しい compose の project が DB を立てられない（`premises.md` の最後の行）

## Goals / Non-Goals

**Goals:**
- 膨れた `ashiato` を、人も agent も気づかずに、検証の準備が作り直す（`test-database` の ADDED 1 つ目）。使っている途中では作り直さない（2 つ目）
- 試験を途中で殺しても次の回の後に残りが無いことを、台本と CI が確かめる（3 つ目）。CH04 D1 の「殺されるとロックが外れ、次の回が片付ける」を初めて実物で固定する

**Non-Goals:**
- 使われていない Docker の資源（浮いた volume・作業ツリーが消えたのに動いているコンテナ・コンテナの無いネットワーク）の片付け（Q7。別の change として申し送る）
- 既存の `down -v` の台本（smoke・stack・check-immutable・check-panic-log）に見送りを当てること（Q7）。ST23 の lock timeout・権限のエラーの原因調べ（Q7）
- 下流の Task の開始前の作り直し（Q2 の 2 つ目の選択肢。harness2 の差し込み口が要る）。agent が自分で走らせる `cargo test` は検証の準備を通らないが、CH04 の後は `ashiato` に頼らないので遅さに効かない
- CH04 の回の database・試験ごとの database の大きさ（CH04 の片付けが持つ）。`ashiato` 以外の database は作り直さない
- 消せない表のトリガ・試験の足場（`testdb`）・試験の期待値（変えない）

## Decisions

### D1（仮）. 閾値は `ashiato` database の大きさ 64 MiB

- `pg_database_size('ashiato')` が 64 MiB（67,108,864 バイト）を超えたら作り直す。作ったばかりは 9.6 MB（ch04）、移行を当てた後も 10 MB 前後、膨れた DB は 110〜214 MB（`premises.md`）
- 環境変数 `TEST_DB_REBUILD_LIMIT_MB` で替えられる（`0` なら毎回作り直す。spec の「検証の準備は、膨れた ashiato を作り直してから終わる」がこれを使う）
- 比べ方は「閾値より大きいときだけ作り直す」（ちょうどは作り直さない）。決め方は DB を使わない口 `tools/test-db-rebuild.sh --decide <バイト数>`（`rebuild` / `keep` を出す）に切り出し、既定の境界を self-test が確かめる
- deep の Q2 の推奨の detail は「64 MB」と書いた。ここでは 64 MiB（2 進）として読む（review/spec.md R11。record で答えの文にも揃える）
- 退けた案: 毎回作り直す（Q2 の 3 つ目）—— 検証 1 回ごとに数秒で、コマンドの間の状態も毎回消える。行の数で見る —— 消せない表のほかに書き出しの台帳（89 MB）など、大きさを決める表が 1 つでない

反転条件: CH04 の後に、縦串・開発用の起動だけで 64 MiB を超える worktree が出る（作り直しが 1 日に何度も起きる）なら、閾値を上げるか、縦串の後の大きさを測ってここに書く。

### D2（仮）. 作り直しは database だけ。同じ session の助言ロックの中で判定し、接続を切らずに消す

`tools/test-db-rebuild.sh` は、DB のコンテナの中の psql（管理者 = `POSTGRES_USER`。`tools/db.sh exec -T db psql -d postgres`。**`postgres` database に繋ぐ** —— 助言ロックは database ごとに分かれ、CH04 も `postgres` で取る）で、
**1 つの session** の中で次を順に行う（`DROP DATABASE` は取引の中で打てないので、取引ではなく session の助言ロックで囲む）。
組み方: 台本を標準入力から 1 つの psql に流し（各文が自分の取引）、結果は `\gset` で受け、分岐は `\if`。DROP の前後だけ `\set ON_ERROR_STOP 0` / `1` にして `:LAST_ERROR_SQLSTATE` で `55006` を見る。
見送りの理由と作り直した行は `\warn` で標準エラーへ。`application_name` は `test-db-rebuild`（self-test が止まっている判定を見つける印）。
psql を起こす命令は `TEST_DB_REBUILD_PSQL`（既定は上の `tools/db.sh exec …`）で差し替えられる（self-test が「DB に問えない」を作る）:

1. 大きさを測る。閾値以下なら何もしないで終える
2. 作業ツリーの種類（D3）が作り直してよい種類でなければ、大きさを標準エラーに出して終える
3. `pg_try_advisory_lock(4820252)`（CH04 D2 の直列化と同じ鍵）。取れなければ「回を作っている・片付けている途中」で見送る。**待たない**（検証の準備を止めない）。
   持っている間、新しい回は database を作り始める前（CH04 D2 の手順 1）で待つ。回は `ashiato` に繋がないが、CH04 の片付けと作り直しを同じ DB で並べない
4. `pg_locks` に `locktype = 'advisory' AND classid = 4820253 AND objsubid = 2`（2 つの int4 の鍵の形）が 1 行でもあれば「走っている回がある」で見送る
5. `pg_stat_activity` に `datname = '<対象>' AND backend_type = 'client backend' AND pid <> pg_backend_pid()` の接続が 1 本でもあれば「接続がある」で見送る（autovacuum などクライアントでない backend は数えない。review/spec.md R9）。
   `TEST_DB_REBUILD_PAUSE_S`（self-test だけが使う）があれば、ここで鍵を持ったまま `pg_sleep` する（spec の「判定の後に繋いだ接続」「作り直しの間は…鍵を取れない」を作る）
6. `DROP DATABASE <対象>`（**`WITH (FORCE)` を付けない**）。5 の後に繋いだ接続があれば PostgreSQL が `55006`（object_in_use）で断るので、それを「接続がある」の見送りとして扱う。それ以外のエラーは rc≠0
7. `CREATE DATABASE <対象>`。所有者の役割 `ashiato_owner` があれば `OWNER ashiato_owner` を付ける
8. `pg_advisory_unlock(4820252)`。標準エラーに `test-db-rebuild: <対象> を作り直した（<前の大きさ>）` の 1 行

- 1 の前に、対象の database が無ければ `CREATE DATABASE`（7 と同じ所有者の付け方）する（spec の「対象の database が無ければ」）
- 見送りは rc=0 で、標準エラーに `test-db-rebuild: <対象> の作り直しを見送った（<理由>。<大きさ>）`。検証はいまの database のまま続く（Q3）
- コンテナ・ネットワーク・volume には触らない（Q4）。依頼の手順（`down -v` → `up` → `db-roles`）を覆したことは `docs/changes/CH05.md` に書く。ネットワークの割り当てが尽きた機械で `down -v` すると取り直せないことがある（Context）
- `WITH (FORCE)` を付けないのが CH04 D6 との違い: CH04 の片付けは持ち主のいない回の database を消すので切ってよいが、`ashiato` に繋いでいるのは起きているサーバや `tests/server_startup.rs` で、切ると使っている途中で壊す
- 対象の database の名前は `TEST_DB_REBUILD_DB`（既定 `ashiato`）。`--self-test` はこれを使い捨ての名前に替えて本物の DB で確かめる（D5）
- 退けた案: 接続の数だけで見る（Q3 の 2 つ目）—— CH04 の回はプールを試験ごとに作って捨てるので、走っている回でも接続が 0 本の間がある（CH04 の premises C6）

反転条件: CH04 の gate・fix で鍵の番号か回の名前が変わったら、3・4 を CH04 に合わせる（`story_deps.py read-spec` の差分で見る）。見送りが続いて作り直せない worktree が出る（起きっぱなしの開発用のサーバなど）なら、見送りの回数を数えて知らせる形を足す。

### D3（仮）. 作業ツリーの種類は `tools/ports.sh` が出し、作り直してよいのは `st`・`ch`・`up` の 3 種

- `tools/ports.sh` に、種類を返す関数 `ashiato_worktree_kind <根の名前> <main の名前> [main]`（3 つ目に `main` を渡すと、worktree でない木 = main の checkout）を足し、source した側に `ASHIATO_WORKTREE_KIND` を置く。
  種類は `main`（worktree でない木。CI もここ。いまと同じく git dir と共通 dir が同じかで見分け、その答えを 3 つ目の引数で渡す）・`st`（`<main>-st<NN>`）・`ch`（`<main>-ch<NN>`）・`up`（`<main>-up-<…>`）・`verify`（`-verify` で終わる）・`other`（それ以外。録画の `-rec-st<NN>`・道具の作業など）
- **port の分岐を、この種類で分ける形に書き換える**（規則を 1 つにする。review/spec.md R15）。port の値は変えない: `main` → 55432 / 18787、`st` → 55500+NN / 19000+2×NN、`verify` → 55499 / 18787、`ch`・`up`・`other` → 今までどおり台帳。
  変えていないことは既存の `tools/check-ports.sh --self-test` が見る（tasks 1.2）
- 作り直してよいのは `st`・`ch`・`up`（Q5）。`main`・`verify`・`other` は測って出すだけ。`TEST_DB_REBUILD_KIND` で種類を上書きできる（`--self-test` と CI —— CI の木は `main` なので、上書きしないと作り直しを確かめられない）
- 退けた案: 種類の正規表現を `tools/test-db-rebuild.sh` に書く —— 規則が 2 か所になる（`tools/ports.sh` の冒頭「規則はここだけ」）

反転条件: 確認バッチの作業ツリー（`-verify`。いま 49 MB）が閾値を超え、確認バッチが遅くなる例が出たら、Q5 の 2 つ目（`-verify` も含める。人間が画面を見ている間はサーバの接続で見送られる）を本人に返す。

### D4（仮）. 検証の準備の中の置き場 —— 起動を待った後、役割を整える前

`tools/verify-env.sh` の順を「起動 → 受け付けを待つ → `tools/test-db-rebuild.sh` → `tools/db-roles.sh`」にする。

- 役割を整える前に置くのは、`tools/db-roles.sh` が `ashiato` に繋いで所有を整えるから（作り直した直後の `ashiato` の所有も、ここで `ashiato_owner` に揃う）
- `tools/test-db-rebuild.sh` が rc≠0（測れない・見送り以外のエラー）なら、検証の準備も rc≠0 で終わる（ハーネスは BLOCKED_INFRA）。消した後に作れなかったときもここに入る。
  次の検証の準備は、対象の database が無ければ作る（大きさを測る前に `CREATE DATABASE` する。手順 7 と同じ所有者の付け方）ので、無いまま残らない
- `HARNESS_VERIFY` の有無で分けない（検証の準備を呼ぶのは検証の前だけ）
- 同じ検証の項目の 2 つ目以降のコマンドの前でも作り直りうる。CH04 の後の試験は `ashiato` の中身に頼らないので落ちない。検証のコマンドが前のコマンドの残した `ashiato` の行に頼る書き方をしない（tasks の Global Constraints。`review/deep.md` R8）

反転条件: 作り直しの判定が検証の準備を 1 秒以上遅くする（psql を起こす費用）なら、大きさだけを先に `docker compose exec` 1 回で読み、超えたときだけ手順 2 以降を起こす形にする。

### D5（仮）. `tools/test-db-rebuild.sh --self-test` は本物の DB の使い捨ての database で確かめる

- 対象を `ch05_rebuild_probe_<乱数>` にし（`TEST_DB_REBUILD_DB`）、所有者の役割で作って行を入れ、閾値を `0` か大きな値にして判定を走らせる。終わりと中断で消す（`trap … EXIT`）。名前に `_tmp_` を入れない（CH04 D6 の片付けが、同じ worktree で並んだ回から消してしまう）
- SIGKILL で `trap` が走らなかった残りは、次の self-test の初めに消す: self-test は `postgres` で専用の助言ロック `pg_advisory_lock(4820254)` を持ち、取れたとき（並んだ self-test が無いとき）だけ `ch05_rebuild_probe_` で始まる database を `DROP DATABASE … WITH (FORCE)` で消す（この頭は CH05 の self-test と殺した後の検査だけが使う。review/spec.md R19）
- 別の session が鍵を持つ・繋がっている状態は、背景の psql（**`postgres` database に繋ぎ**、`SELECT pg_advisory_lock(…); SELECT pg_sleep(…)`）を起こして作り、`pg_locks` に鍵が見えてから判定を走らせ、確かめた後に止める
- 「判定の後に繋いだ接続」と「作り直しの間は鍵を取れない」は、`TEST_DB_REBUILD_PAUSE_S` で判定を手順 5 の後に止め、`application_name = 'test-db-rebuild'` の session が `pg_sleep` にいるのを `pg_stat_activity` で見てから、別の接続を繋ぐ / 鍵を試す
- 「DB に問えない」は `TEST_DB_REBUILD_PSQL=false` で作る
- 作業ツリーの種類の規則（`ashiato_worktree_kind`）は、DB を使わずに名前を渡して確かめる
- 移行を当て直せることは、作り直した使い捨ての database に `DATABASE_OWNER_URL` の database の部分を替えて `ashiato-server migrate`（`MIGRATE_BACKUP=off`。`tests/server_startup.rs` と同じ）を走らせ、`core.event` の `DELETE` が断られることで見る
- `ashiato` の作り直しそのものは、閾値 `0` で `tools/verify-env.sh` を走らせて見る（`--verify-env-check`。tasks 2.2。この worktree の `ashiato` を作り直す —— 試験だけの DB で、使う側が移行を当て直す）。
  見送られたときは理由を出して rc=1（作り直しを確かめられていない）、実際の種類が `st`・`ch`・`up` でなければ rc=2 で何もしない（main の checkout の `ashiato` を作り直さない）

反転条件: 背景の psql の起動の待ちで self-test が揺れる（鍵を取る前に判定が走る）なら、背景の側が鍵を取った後に印のファイルを書き、それを待ってから判定を走らせる。

### D6（仮）. 殺した後の検査 —— 探りの試験を印で同期して SIGKILL する

`tools/check-test-db-killed.sh`（引数なし / `--self-test`）:

- 探りの試験 `ch05_killed_probe`（`crates/server/src/killed_probe_tests.rs`。`#[ignore]`。通常の `cargo test` では走らない）は、`CH05_PROBE_AT`（`insert` / `committed`）と `CH05_PROBE_MARK`（印のファイル）を読み:
  - 回の database（`testdb::pool()`）と試験ごとの database（`testdb::isolated()`）を取り、その名前を印のファイルに書く
  - `insert`: 回の database に、`deletion_tests.rs` の体積の試験（`stays_detail_query_uses_user_time_index`）と同じ形の 20,001 件の INSERT を、`pg_sleep(600)` を前に置いた 1 文で走らせる（文が走っている時間を作る）。印のファイルに `inserting` を書いてから文を打つ
  - `committed`: 回の database と試験ごとの database の両方に 20,001 件を入れて確定し、印のファイルに `committed` を書いてから 600 秒眠る
- 台本は、`cargo test -p ashiato-server --lib --no-run --message-format=json` で試験の実行ファイルの path を取り、`setsid` で直接起こす（`--ignored --exact … --nocapture`。`ASHIATO_TEST_STATEMENT_TIMEOUT=0`。CH04 D7 の 60 秒の上限で落ちないように）。
  `--exact` に渡す名前は `killed_probe_tests::ch05_killed_probe`。
  `insert` は印の `inserting` と、回の database の `pg_stat_activity` に `ch05_probe` の印のある `active` の文が見えるのを待つ。`committed` は印の `committed` を待つ。待ちの上限は 120 秒（`TEST_DB_KILLED_WAIT_S` で替えられる。越えたら「探りの印が来なかった（探りが走らなかった・0 本の絞り込み）」と時点の名前を出して rc=1）。その後、プロセスグループごと `kill -KILL`
- **探りを起こす前に**、一覧 S0（`pg_database` の名前）・`ashiato` の `core.event` の行数（表が無ければ 0）・`pg_database_size('ashiato')` を読み、殺した後に一覧 S1 を読む（殺す直前に S0 を読むと、探りの database が S0 に入って差が空になる。review/spec.md R10）。殺した回の database = S1 にあって S0 に無いもの（印のファイルの名前もそこに入っていることを見る）。
  1 つも無ければ「殺した回の database が見えない（検査が空振り）」で rc=1
- 次の回は `cargo test -p ashiato-server ch05_next_run`（同じファイルの、`testdb::pool()` で繋いで `SELECT 1` するだけの 1 本。CH04 の片付けを起こす）。秒数を測って標準出力に出す
- 判定（順に。最初に外れたもので落ちる）: 次の回の rc → 殺した回の database のうち、次の回の後の一覧 S2 に残るもの（名前を挙げる）→ `ashiato` の行数が殺す前と違う（前と後を挙げる）→ `ashiato` の大きさが殺す前から 1 MiB を超えて増えた（前と後を挙げる）
- 2 つの時点を順に行う（`insert` → `committed`）。どちらかが落ちれば、その時点の名前を出して落ちる
- `committed` の眠りの間（探りが回の助言ロックを持っている間）に、使い捨ての `ch05_rebuild_probe_<乱数>` を作って閾値 `0`・種類 `st` で `tools/test-db-rebuild.sh` を走らせ、「走っている回」で見送ることを見る（本物の CH04 の鍵で見送ることの確かめ。review/spec.md R8）。使い捨ては見た後に消す
- 差し替え（`--self-test`）: `TEST_DB_KILLED_PROBE_CMD`（探り）・`TEST_DB_KILLED_NEXT_CMD`（次の回）・`TEST_DB_KILLED_LIST_CMD`（一覧）・`TEST_DB_KILLED_STAT_CMD`（`ashiato` の行数と大きさ）。CH04 D8 の `TEST_DB_CMD`・`TEST_DB_LIST_CMD` と同じ形
- **同じ worktree で別の `cargo test` と並べて走らせない**（並んだ回の database が S1 と S0 の差に入る）。冒頭のコメントに書く
- 大きさの許し（1 MiB）は、触らない database でも統計や空き領域の地図で大きさが少し動くことがあるため。行数は厳密に見る
- 「INSERT の途中で殺せば巻き戻って行は残らない」とは書かない（`client_connection_check_interval` が 0 なので、文は終わって確定しうる）。見るのは、確定したかに依らず次の回の後に残らないことだけ
- 退けた案: 本物の体積の試験（`stays_detail_query_uses_user_time_index` など）を時間で殺す —— 文が 1 秒未満で終わり、殺す時点を選べない。探りの試験に眠りを入れて印で同期すれば、時点が揺れない。
  CH04 の `tools/check-test-db-growth.sh` に足す —— CH04 の下流が同じファイルを触っている間に、CH05 が同じファイルを取り合う

反転条件: CI で `setsid` やプロセスグループの kill が効かない（試験の実行ファイルが残る）なら、実行ファイルの pid を直接 `kill -KILL` する形に替える。次の回の秒数が殺す前のきれいな回の 10 倍を超える例が出たら、秒数も合否に入れるかを本人に返す（Q6）。

### D7. 新しい試験の名前の頭は `ch05_`

`crates/server/src/killed_probe_tests.rs` の 2 本（`ch05_killed_probe`・`ch05_next_run`）。`#[ignore]` の探りは `cargo test` の既定では走らず、本数の数えは `-- --list --ignored` で見る。
CH04 の `--lint` は `CREATE DATABASE` と書き出しの試験のファイルの `testdb::pool()` を見るが、このファイルはどちらも持たない。

### D8. CI の rust job の 3 行と、文書

- `.github/workflows/ci.yml` の rust job、CH04 の `tools/check-test-db-growth.sh` の行の後ろに: `tools/test-db-rebuild.sh --self-test`・`tools/check-test-db-killed.sh --self-test`・`tools/check-test-db-killed.sh`
- CI の木の種類は `main` なので、`--self-test` は `TEST_DB_REBUILD_KIND` で種類を上書きして作り直しを確かめる（D3）。CI の `ashiato` を作り直すことはしない（`--self-test` は使い捨ての名前だけを触る）
- `docs/testing.md` に 1 節（検証の準備が `ashiato` を作り直すこと・閾値・見送り・殺した後の検査）、`docs/flow-gates.md` の検査の表に 2 行

## Risks / Trade-offs

- [CH04 の鍵の番号・回の名前が gate・fix で変わる] → D2・D6 は CH04 の D 番号で引き、反転条件に書いた。下流の入口が CH04 の gate を通った SHA を取り込み、tasks 1.1 で番号を `testdb.rs` から読み直す
- [回の鍵を持たない `ashiato` の利用者（`tests/server_startup.rs`）は、同じ作業ツリーで検証の準備と並べると、接続の切れ目で作り直されうる] → 検証の準備はハーネスが検証のコマンドの前に 1 本ずつ呼ぶので、同じ作業ツリーの試験と並ばない。並べて手で走らせたときだけ起きる（作り直された後は `migrate` が当て直す）
- [作り直しが、試験の外で `ashiato` を使っている人を壊す] → 接続があれば見送り、消すときも接続を切らない（D2 の 5・6）。接続を持たずに `ashiato` の中身に頼る使い方（同じ検証の項目の前のコマンドが置いた行を、後のコマンドが読む）は、閾値を越えた回に限って壊れうる。tasks に書かない取り決めで閉じる（D4）
- [作り直しの判定が DB の壊れで rc≠0 になり、検証が BLOCKED_INFRA で止まる] → 準備が落ちる理由は標準エラーに出る。DB が壊れていれば、どのみち試験は通らない
- [殺した後の検査が CI の時間を伸ばす] → 探りの 2 回と次の回の 2 回。試験の実行ファイルは `cargo test --workspace` で組み済み。tasks 5.3 で 3 行の時間を手元で測って書く
- [`ashiato` の大きさの許し 1 MiB が、本物の残りを見逃す] → 体積の行 20,001 件は数 MB で、許しより大きい。行数は厳密に見る

## Migration Plan

移行は足さない。merge の後、各 worktree の `ashiato` は次の検証の準備で閾値を越えていれば 1 度作り直される。戻すときは `tools/verify-env.sh` の 1 行を外せば今の振る舞いに戻る（作り直した `ashiato` は使う側が移行を当て直す）。

## 計測の記録

下流が書く（鍵の後ろに数字を書く。tasks の検証が `^- <鍵>: [0-9]` を見る）:

- 作り直しの判定:（tasks 2.2。閾値以下で判定だけにかかる秒数と、作り直したときの秒数）
- 次の回の秒数:（tasks 4.2。殺す前のきれいな回、`insert` の後、`committed` の後）
- 片付けを外した回:（tasks 4.3。rc と、挙がった database の名前の数）
- 3 行の時間:（tasks 5.3。`tools/test-db-rebuild.sh --self-test`・`tools/check-test-db-killed.sh --self-test`・`tools/check-test-db-killed.sh` を手元で走らせた秒数）
