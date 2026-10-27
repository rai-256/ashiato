# CH06 の前提（questions の段、2026-10-25）

この change の決定が依る、外の振る舞い（docker compose・ハーネスの `verify-run`・走っている CH04）の前提。

| # | 前提 | 何が依るか | 確かめ方 | 結果 | 出所 | 確かさ |
|---|---|---|---|---|---|---|
| P1 | compose の project 名は worktree のディレクトリ名になり、DB のコンテナと volume は worktree ごとに分かれる | 「ほかの worktree の DB には触らない」 | `docker ps` の名前を見る | `ashiato2-st23-db-1`・`ashiato2-ch04-db-1`・`ashiato2-verify-db-1` …（11 個）。`.env`・`docker-compose.yml` に `COMPOSE_PROJECT_NAME` / `name:` は無い | 実物 | 5 |
| P2 | `HARNESS_VERIFY=1` での DB の作り直しは、すでに `tools/stack.sh` が `docker compose down -v` でやっていて動いている | 作り直しの形の前例 | `tools/stack.sh:24`・`:75-76` を読む | 有り。e2e（ST33 10.2 の実測で入った）で使われている | 既存のテスト材料 | 2 |
| P3 | `verify-run` は**どの検証コマンドの前にも**（DB を使わない gradle・vitest の前にも）`tools/verify-env.sh` を `HARNESS_VERIFY=1` で呼ぶ | 作り直しの回数と、1 回あたりの時間が効く範囲 | `scripts/evidence.py:374-382` を読む | 有れば毎回呼ぶ。中身は repo が決める | 既存のテスト材料 | 2 |
| P4 | `docker compose down -v` は DB のコンテナ・volume と一緒に **project のネットワークも消す**。検証として走る `tools/stack.sh`・`smoke.sh:31,38`・`check-immutable.sh:10`・`check-panic-log.sh:15` は毎回 `down -v` を打つ。いまの機械は Docker の既定のアドレスの空きが尽きている | 作り直しの手段（ネットワークを手放すか） | この worktree で `tools/verify-env.sh` を走らせる・`docker network ls` | `failed to create network ashiato2-up-ch06_default: all predefined address pools have been fully subnetted`。ネットワークは 32 個（うち ashiato2 の 25）。`down` で手放すと、次の `up` までに他の worktree が取れば、その worktree の検証は DB を起動できず BLOCKED_INFRA になる | 実物 | 5（尽きていること）/ 4（`down -v` はネットワークを消し、`rm -s -f -v db` は残す。使い捨ての project で実測。review/deep.md R3） |
| P5 | CH04（下流が走っている。Task 10 まで済み）が入ると、`cargo test -p ashiato-server` は回ごとの database（`ashiato_run_…`）と書き出しの試験ごとの database に書き、共有の `ashiato` database には試験の記録を積まない。前の回の分は次の回の始まりに `DROP DATABASE` で消える | CH06 の範囲（Q1）。依頼の「溜まり続ける」がどこまで残るか | `openspec/changes/ch04-test-db-growth/design.md`（D1・D6）・`proposal.md`、実装 `ashiato2-ch04/crates/server/src/testdb.rs:12-18`（回の id と database）・`:212`（`_tmp_`）を読む | 設計と実装がそうなっている。`tests/server_startup.rs`・縦串・開発用の起動は `ashiato` のまま（CH04 の C7）。CH04 の Q4 は「今ある `ashiato` の 14〜18 万行には何もしない」 | 既存のテスト材料 | 2 |
| P6 | CH04 は**同じ worktree で回が並んで走る**ことを想定している（助言ロックで走っている回を見分け、消さない） | 作り直しが、並んで走っている試験を切るか（Q2） | `ch04-test-db-growth/design.md` D1・D6、実装 `ashiato2-ch04/crates/server/src/testdb.rs:155`（回の助言ロック）・`:196`（片付けの `pg_try_advisory_lock`）を読む | 想定している。volume ごと作り直すと、並んだ回の database も消える | 既存のテスト材料 | 3 |
| P7 | main の checkout（`ashiato2`）の DB は開発用の起動（`tools/dev.sh`）が使い、移行の前に退避（`migrate_backup.rs`）を取るほど中身を守っている | どの worktree で作り直してよいか（C1） | `tools/dev.sh:19-23`・`crates/server/src/migrate_backup.rs:1` を読む | 守っている。`tools/ports.sh` は main の checkout と CI に同じ 55432 を割り当てる | 既存のテスト材料 | 2 |
| P8 | CI の rust job は DB を compose で立てる（`services` ではない） | 完了の判定 3 の検査を CI で走らせられるか（C4） | `.github/workflows/ci.yml:62-68` を読む | compose で立てる | 既存のテスト材料 | 2 |
| P9 | 作り直し 1 回（コンテナと volume を消して `up --wait`）は約 5 秒。10/17〜10/24 の検証は 602 回 | Q1 の費用・C5 | 同じ image・`max_connections=300`・同じ healthcheck の使い捨ての project（`network_mode: none`）で `rm -s -f -v db` → `up -d --wait db`、`ashiato2*/openspec/changes/*/evidence.jsonl` を数える（review/deep.md R8） | 5.1 / 5.5 / 5.5 秒（役割の整えと port の公開は含まない）。毎回作り直すと 1 週間で約 1 時間 | 実物 | 3 |
| P10 | `ashiato2*_default` のネットワーク 25 個のうち 15 個は、もう無い worktree のもの（うち 4 個は DB のコンテナも動いたまま）。明示の subnet を持つ project は空きが尽きた今でもネットワークを作れる | Q3 | `docker network ls`・`docker ps -a` と `git worktree list` の突き合わせ、使い捨ての project に `ipam.config.subnet` に私設の /24 を 1 つ（review/deep.md R1） | そのとおり | 実物 | 5 |
| P11 | 一度作り直された（`--force-recreate`）コンテナは、`rm -s -f -v` でも `down -v` でも匿名 volume が残る。`up -V` も古い volume を残す。今の宙に浮いた volume は 66 個 | C2 の手段 | 使い捨ての project で実測・`docker volume ls -qf dangling=true`（review/deep.md R7） | そのとおり（`docker-compose.yml` は volume を宣言せず、image の `VOLUME` の匿名 volume） | 実物 | 4 |
| P12 | compose の file で network の ipam（subnet）が変わると、次の `up` が network を作り直し、DB のコンテナは作り直さずに繋ぎ直す（中身は残る） | 既にある worktree・main の checkout へ subnet を入れるときに DB の中身が残るか（D6） | 使い捨ての project（同じ compose 5.5.0）で subnet を替えて `up`、コンテナの id と volume の中のファイルを比べる | network は作り直され、コンテナの id は同じで、ファイルは残った | 実物 | 4 |
| P13 | `.env` の `COMPOSE_FILE`（base と subnet の 2 枚）と `ASHIATO_DB_SUBNET` は、呼び出し側が何も export しなくても `docker compose config` / `exec` / `up` に効き、既定のアドレスの空きが尽きた今の機械でも `down -v` → `up` で同じ subnet の network を取り直せる | subnet を全部の呼び出し口に届ける手段（D6） | 使い捨ての project（`.env` だけを置き、環境変数を渡さない） | `config` の ipam に subnet が出る・`up` / `exec` が rc=0・`down -v` の後の `up` も同じ subnet で立った | 実物 | 4 |
| P14 | `tools/check-private.sh` は追跡ファイルの中の点区切りの私設 IP（10 / 172.16–31 / 192.168 で始まる 4 つ組）を落とす | subnet を台本・試験・文書にどう書くか（D5） | `tools/check-private.sh:63-66` を読む | 落とす。`printf` の書式（`%d` を含む形）や `<x>` の雛形は 4 つ組にならないので通る | 既存のテスト材料 | 2 |
| P15 | CI の rust job は `.env` を持たず、合言葉を `$GITHUB_ENV` で渡す。`tools/db-roles.sh` は `.env` があればそれを読んで環境の値より優先する | CI で検査が `.env` を作ると、同じ job の後ろの段の合言葉が食い違う（D7） | `.github/workflows/ci.yml:66-69`・`tools/ci-db-env.sh`・`tools/db-roles.sh:22-25`・`tools/env-init.sh:12` を読む | そのとおり（review/deep.md R2） | 既存のテスト材料 | 2 |

失われるもの（`loss`）のある決定は、どれもこの前提に載せていない（試験の DB の中身は誰の記録でもなく、作り直せば戻る。main の checkout の DB は C1 で触らない側に倒した）。

## 聞かないで決めるもの（C）

- **C1. main の checkout（`tools/ports.sh` の「worktree でない木」）では作り直さない。ただし CI（`CI=true`）は作り直す側に倒す。** 印があっても起動して待つだけにし、作り直さなかったと 1 行出す。
  CI の checkout も「worktree でない木」なので、木だけで見分けると CI では作り直しが起きず C4 が落ちる（review/deep.md R2）。見分け方の取り違えが main の DB を消す側に倒れないことは C4 の台本で固定する。
  開発用の起動の DB で、移行の前に退避を取るほど中身を守っている（P7）。作り直すと失われる側で、作り直さない側は失わない（遅いだけ）。
  確認バッチの作業ツリー（`-verify`）は作り直す（`verify-run` を呼ぶ係は無く、`tools/stack.sh` がすでに `down -v` している）
- **C2. ネットワークは手放さない。古い volume は名指しで消す。** 作り直しは、消す前に DB のコンテナの volume の名前を `docker inspect` で取り、コンテナを消した後にその volume を `docker volume rm` で消して立て直す（`docker compose down -v` も `up -V` も使わない）。
  手放すと、空きの尽きた機械では次の `up` が他の worktree に負けて BLOCKED_INFRA になる（P4）。`-V` と作り直されたコンテナの `rm -v` は古い volume を残す（P11）。
  `stack.sh`・`smoke.sh`・`check-immutable.sh`・`check-panic-log.sh` の `down -v` はこの change では触らない（Q3 で subnet の割り当てを選ばない限り同じ危うさが残ることを proposal に書く）
- **C3. 作り直した後の役割は今までどおり `tools/db-roles.sh`。** 移行は当てない（`testdb.rs` の初回が当てる。依頼の完了の判定 2）
- **C4. 完了の判定 3 の検査は `tools/check-verify-env-reset.sh`（`--self-test` 付き）を新しく置き、CI の rust job で走らせる**（P8。CI も compose で DB を立てる）。
  溜まった DB（`core.event` に行を足し、残す database も 1 つ作る）から `HARNESS_VERIFY=1 tools/verify-env.sh` を呼び、`core.event` が 0 行・足した database が無い・役割で繋げる、を見る。
  印の無いときと、CI でない main の checkout で行が残る（作り直さない）こと、作り直しの前後でこの project の volume の数が増えないことも同じ台本で見る。
  CI では `.env` を `env-init.sh` が乱数で作ると `db-roles.sh` が `$GITHUB_ENV` と違う合言葉に揃え、同じ job の後ろの段が認証で落ちうる（review/deep.md R2）—— 検査が CI の合言葉と食い違わないことを spec の段で決める
- **C5. 検証のたびに毎回作り直す**（依頼の逐語「検証のたびに」。DB を使わない検証の前も）。1 回約 5 秒・1 週間で約 1 時間（P9）。回数を絞ると前の検証の残りの上で走る（review/deep.md R5）
- **C6. 作り直せなければ非 0 で終わる**（ハーネスは BLOCKED_INFRA として記録し、検証を走らせない）。溜まった DB のまま進めると、ハーネスの約束「証跡は溜まった状態に頼らない」（`scripts/evidence.py:28`）に反する（review/deep.md R6）
