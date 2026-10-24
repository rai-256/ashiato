# CH06 実装タスク — 検証のたびにテスト用 DB を作り直す

読む順: `deep.md`（**本人の答え Q1〜Q3**。第 1 回は推奨で先に書いてある）→ このファイル → `specs/verify-environment/spec.md` → `design.md` →
`premises.md`（P1〜P15 と末尾の C1〜C6）→ `docs/changes/CH06.md` → `tools/verify-env.sh`・`tools/env-init.sh`・`tools/ports.sh`・`tools/check-ports.sh`・`tools/verify-prep.sh` → `CLAUDE.md`。

検証は各タスクの本文に書いてある。「動いた」ではなくコマンドと終了コードで判定する。
`scripts/verify-run` は検証コマンドの前に毎回 `tools/verify-env.sh` を `HARNESS_VERIFY=1` で呼ぶ（Task 4 の後は、検証のたびにこの worktree の DB が空になる）。

## Global Constraints

- **テストには `Scenario: <名前>` の印を置く。** bash は `echo "Scenario: …"`（その場合を確かめる直前に出す）。
  `scripts/check_scenarios.py` が spec の全 Scenario と突き合わせ、印の無い Scenario を FAIL にする。印の名前は spec の `#### Scenario:` と**一字一句合わせる**
- **この change の Scenario は 24 本**（`verify-environment` の ADDED 6 つ: 5・5・3・1・4・6 本）。置き場は 3 つ:
  `tools/check-ports.sh --self-test` に 1 本、`tools/check-verify-env-reset.sh --self-test` に 10 本、`tools/check-verify-env-reset.sh`（本体）に 13 本。**24 本とも、このファイルのどこかの Task が名指しする**。
  本体のうち 1 本は CI（worktree でない木）だけ、2 本は subnet のファイルのある木（worktree）だけで通る（design D8）。CI と `verify-run` の証跡を合わせて全部が通る
- **追跡するファイルに点区切りの私設 IP の 4 つ組を書かない**（`tools/check-private.sh` が落とす。P14・design D5）。subnet は `printf '10.%d.%d.0/24' …` で組み、期待値も同じく組む。生成物の `docker-compose.subnet.yml` は `.gitignore` に入れる
- **subnet の値を環境変数に載せない・環境変数から読まない**（design D5・D6。ハーネスが渡す古い `.env` の値が勝つ）
- **作り直しに `docker compose down -v`・`up -V`・`--force-recreate`・`rm -v` を使わない**（design D1）。消すのは控えた名前の volume だけ
- **変えないもの**: `crates/`（試験のコードを含む）・`migrations/`・`tools/stack.sh`・`tools/smoke.sh`・`tools/check-immutable.sh`・`tools/check-panic-log.sh`・`tools/dev.sh`・`tools/record-env.sh`・`tools/db-roles.sh`・`docker-compose.yml`
- **既にある `.env` の合言葉の行と権限を変えない**（design D7）。CI（3 つの合言葉が環境変数にあり `.env` が無い）で `.env` を作らない
- **本人の決定（下流は変えない）**: Q1 依頼どおり volume ごと毎回作り直す（CH04 と重ねる）/ Q2 構わず作り直し、切った接続の数と出所を 1 行出す /
  Q3 worktree ごとの subnet を被らない規則（port を通した台帳の番号）で割り当てる。消えた worktree の残りは片付けない（例外は design D5 の、これから立てる network を塞ぐ空の network だけ）。
  （第 1 回は推奨で先に書いた。本人の答えが違えば上流がここを書き直してから下流へ渡す）
- 走っている Story・change のブランチには触らない（proposal の Impact）

## Task 1: subnet の規則と、compose への届け方

この Task が入るまで、この worktree の `verify-run` は DB を起動できず BLOCKED_INFRA になりうる（Docker の既定のアドレスの空きが尽きている。P4）。
`verify-env.sh` は検証している木の `env-init.sh` を呼ぶので、1.3 が入った後の検証から DB が立つ。

- [ ] 1.1 `tools/ports.sh` に関数 `ashiato_db_subnet` を足す（design D5）: `ASHIATO_DB_PORT` から `off = port − 55400`、`0 ≤ off < 512` のときだけ `printf '10.%d.%d.0/24' $((200 + off / 256)) $((off % 256))`、範囲の外なら何も出さない。
  変数は export しない。冒頭の規則の注記に 1 段落足す（4 つ組を書かない）。末尾の `unset -f` に入れない（呼び出し側が使う）。
  検証: `bash -c '. tools/ports.sh; [[ $(ashiato_db_subnet) == 10.20[01].*.0/24 ]]'` rc=0 / `bash -c '. tools/ports.sh; [ -z "$(ASHIATO_DB_PORT=1 ashiato_db_subnet)" ]'` rc=0 / `tools/check-private.sh` rc=0
- [ ] 1.2 `tools/check-ports.sh --self-test` に subnet を足す（いまの `ports()` の関数で `ashiato_db_subnet` も出す）:
  `echo "Scenario: どの 2 つの木も、DB の subnet が被らない"` —— いまある 7 つの木（demo・demo-st05・demo-verify・demo-ch01・demo-ch02・demo-up-st05・demo-rec-st22）の subnet が全部違い、どれも `/24` で終わる。
  main の checkout の値は `printf` で組んだ期待値（off = 32）と同じ。
  検証: `tools/check-ports.sh --self-test` rc=0 / `bash -c '[ "$(grep -c "Scenario: " tools/check-ports.sh)" -ge 1 ]'` rc=0
- [ ] 1.3 `tools/env-init.sh` に design D7 の 3.〜5. を書く: 新しく作る `.env` に `COMPOSE_FILE` も書く / ある `.env` の `COMPOSE_FILE` の行を今の木の値にそろえる（木に `docker-compose.override.yml` があれば 3 つ目に並べる。ほかの行は変えない。
  一時ファイルに書いて `cat tmp > .env` で中身だけを入れ替え、権限を保つ）/ `ashiato_db_subnet` の値（環境変数も `.env` も見ない）で `docker-compose.subnet.yml`（default の network の `ipam.config` の `subnet` だけ。冒頭に生成物である注記）を作り直す。
  値が出なければ subnet のファイルを作らず `COMPOSE_FILE` の行を消す。`.gitignore` に `docker-compose.subnet.yml` を足す。冒頭の注記の「`.env` があれば何もしない（上書きしない）」を「合言葉の行は上書きしない。`COMPOSE_FILE` と subnet のファイルだけを保つ」に直す。
  検証: `bash -c 'grep -qE "^COMPOSE_FILE=docker-compose.yml:docker-compose.subnet.yml(:docker-compose.override.yml)?$" .env && git check-ignore -q docker-compose.subnet.yml'` rc=0 /
  `bash -c '. tools/ports.sh; [ "$(docker network inspect -f "{{range .IPAM.Config}}{{.Subnet}}{{end}}" "$(basename "$PWD")_default")" = "$(ashiato_db_subnet)" ]'` rc=0 / `tools/check-private.sh` rc=0

## Task 2: env-init の残りの規則・確認バッチの準備・自己検査の台本

- [ ] 2.1 `tools/env-init.sh` に design D7 の 1.（`.env` が無く 3 つの合言葉が環境変数にあれば何もしない）と 2.（`.env` が `COMPOSE_PROJECT_NAME` の行を持てば何もしない）を、3. の前に書く。
  `tools/verify-prep.sh` の `.env` を写す・作る段の後に `tools/env-init.sh` を呼ぶ 1 行を足す（design D7。確認バッチの作業ツリーの subnet をその木の値にする）。
  検証: `bash -c 'd=$(mktemp -d); mkdir -p "$d/tools" && cp tools/env-init.sh tools/ports.sh "$d/tools/" && POSTGRES_PASSWORD=a OWNER_DB_PASSWORD=b APP_DB_PASSWORD=c "$d/tools/env-init.sh" && [ ! -e "$d/.env" ] && [ ! -e "$d/docker-compose.subnet.yml" ]; r=$?; rm -rf "$d"; exit $r'` rc=0 /
  `bash -c 'grep -n "env-init.sh" tools/verify-prep.sh | grep -q .'` rc=0
- [ ] 2.2 `tools/check-verify-env-reset.sh`（新しいファイル。`set -euo pipefail`、使い方の注記、`--self-test` と本体の 2 つの入口）の `--self-test` を書く（design D8）。
  一時の repo と worktree を `tools/check-ports.sh` と同じ作り方で作り、`tools/verify-env.sh`・`tools/ports.sh`・`tools/env-init.sh`・`docker-compose.yml` を写す。一時の場所は `mktemp -d`（`$TMPDIR` の下）で、`trap` で消す。
  一時の木の道具は design D8 の 9 つの変数を `env -u` で落として呼ぶ。この段では次の 7 本:
  `echo "Scenario: 既にある .env に、合言葉を変えずに COMPOSE_FILE を足す"` / `echo "Scenario: 別の木から写した .env でも、その木の subnet になり、何度呼んでも同じである"`（demo-ch01 の `.env` を demo-ch02 に写し、`ASHIATO_DB_SUBNET` に demo-ch01 の値を入れて 2 回呼ぶ。2 回目の後の `.env` と subnet のファイルが 1 回目の後と `cmp` で同じ）/
  `echo "Scenario: env-init が書き換えた .env の権限は、書き換える前と同じである"`（`stat -c %a` が 600）/ `echo "Scenario: 木に override のファイルがあれば、COMPOSE_FILE に並べる"` /
  `echo "Scenario: 合言葉が環境変数で渡っていれば、.env を作らない"` / `echo "Scenario: compose project の名前を持つ .env には触らない"`（呼ぶ前と `cmp` で同じ）/
  `echo "Scenario: env-init の後は、compose の DB の network に木の subnet が入る"`（`env -i PATH="$PATH" HOME="$HOME" docker compose config --format json` の `networks.default.ipam.config[0].subnet` が一時の木の `ashiato_db_subnet`）。
  失敗はどの場合かを出して最後に rc=1、全部通れば `OK` の 1 行で rc=0。
  検証: `tools/check-verify-env-reset.sh --self-test` rc=0 / `bash -c 'POSTGRES_PASSWORD=a OWNER_DB_PASSWORD=b APP_DB_PASSWORD=c ASHIATO_DB_SUBNET=x CI=true tools/check-verify-env-reset.sh --self-test'` rc=0（CI と同じ環境）/
  `bash -c '[ "$(grep -c "echo \"Scenario: " tools/check-verify-env-reset.sh)" -ge 7 ]'` rc=0 / `tools/check-private.sh` rc=0

## Task 3: 作り直すかの判定と `--plan`

- [ ] 3.1 `tools/verify-env.sh` に design D2 の判定を書く（印・worktree かどうか・`CI`）。`--plan` は判定の 1 行（`reset` / `keep: <理由>`）を出して rc=0 で終わり、docker にも `.env` にも触らない（`env-init.sh` も呼ばない）。
  印があって `keep` のときは、判定の 1 行を標準エラーに出してから今までどおり起動して待つ。冒頭の注記を design D11 のとおり直す。
  検証: `bash -c '[ "$(HARNESS_VERIFY=1 tools/verify-env.sh --plan)" = reset ]'` rc=0（worktree で走る）/ `bash -c 'env -u HARNESS_VERIFY tools/verify-env.sh --plan | grep -q "^keep:"'` rc=0
- [ ] 3.2 `tools/check-verify-env-reset.sh --self-test` に 3 本を足す（2.2 の一時の repo と worktree を使う）:
  `echo "Scenario: CI でない main の checkout では、印があっても作り直さない"`（worktree でない木で `HARNESS_VERIFY=1 … --plan` が `keep:` で始まり `main` を含む）/
  `echo "Scenario: CI の main の checkout と、worktree では、印があれば作り直す"`（worktree でない木の `CI=true HARNESS_VERIFY=1`、`demo-st05`・`demo-verify`・`demo-ch01` の `HARNESS_VERIFY=1` が `reset`）/
  `echo "Scenario: 印が無ければ、どの木でも作り直さない判定になる"`（worktree でない木（`CI=true` と無し）と 3 つの worktree で、印なしが `keep:`）。
  検証: `tools/check-verify-env-reset.sh --self-test` rc=0 / `bash -c '[ "$(grep -c "echo \"Scenario: " tools/check-verify-env-reset.sh)" -ge 10 ]'` rc=0

## Task 4: 作り直しの手順・切った接続の 1 行・作り直せないとき・塞いでいる network

- [ ] 4.1 `tools/verify-env.sh` に design D1 の手順を書く（判定が `reset` のとき。`env-init.sh` の後、`up` の前）: `docker compose ps -aq db`（無ければ `切った接続: 0（DB のコンテナが無い）`）→ `docker inspect` で volume の名前を控える →
  D4 の確かめ（`docker ps -aq --filter volume=<名前>` にこのコンテナのほかがあれば、volume とコンテナの名前を標準エラーに出し、何も消さずに `exit 1`）→
  D3 の 1 行（`pg_stat_activity` の `client backend`、自分の接続を除く。`application_name@接続元`、無ければ `-` / `local`、同じ出所は `×<数>`、10 種を超えたら `…`。問えなければ `切った接続: 0（DB は止まっていた）`）→
  `docker compose rm -s -f db`（`-v` なし）→ 控えた volume を `docker volume rm`（落ちたら `exit 1`）。
  検証: `bash -c 'HARNESS_VERIFY=1 tools/verify-env.sh 2>&1 | grep -q "切った接続: "'` rc=0
- [ ] 4.2 `tools/verify-env.sh` に design D5 の network の確かめを書く（判定に依らず、`docker-compose.subnet.yml` があり、自分の project の network が無いとき、`up` の前）: 同じ subnet を持つ別の network を探し、
  コンテナが繋がっていなければその network だけを消す。繋がっていれば network とコンテナの名前を標準エラーに出し、何も消さずに `exit 1`。その後に今までの `up -d --wait db`・待ち・`db-roles.sh`。どの docker の失敗も非 0 にする。
  検証: `bash -c 'HARNESS_VERIFY=1 tools/verify-env.sh && HARNESS_VERIFY=1 tools/verify-env.sh'` rc=0（2 回続けて作り直せる）/
  `bash -c '! grep -vE "^\s*#" tools/verify-env.sh | grep -nE "down -v|up .*-V|force-recreate|rm -[a-z]*v"'` rc=0
- [ ] 4.3 作り直し 1 回の時間を 3 回測り、design の「計測の記録」に書く（10 秒を超えたら、どの段が遅いかも書く）。
  検証: `bash -c 'for i in 1 2 3; do s=$(date +%s); HARNESS_VERIFY=1 tools/verify-env.sh >/dev/null 2>&1 || exit 1; echo "reset $(( $(date +%s) - s ))s"; done'` rc=0

## Task 5: 検査の本体 —— 溜まった DB から作り直して確かめる

- [ ] 5.1 `tools/check-verify-env-reset.sh` の本体（引数なし）を design D8 の 1.〜8. のとおりに書く。worktree でも `CI=true` でもない木では、理由を出して rc=2（作り直さない木で走らせない）。
  各段で `HARNESS_VERIFY` を明示する（`env -u HARNESS_VERIFY` / `HARNESS_VERIFY=1`。verify-run が渡す印に依らない）。DB への問い合わせは `docker compose exec -T db psql -X -qtA -U ashiato`。
  sentinel（`ch06-sentinel-<乱数>` のコンテナ・volume・network・compose project）は `trap` で必ず消す。印は次の 13 本を、その場合を確かめる直前に出す:
  3. で `echo "Scenario: 印が無ければ作り直さず、溜まった行は残る"`・（worktree でない木だけ）`echo "Scenario: CI でない main の checkout では、印つきで呼んでも行が残る"` /
  4. で `echo "Scenario: DB の volume をほかのコンテナが使っていれば、何も消さずに非 0 で終わる"` /
  5. で `echo "Scenario: 繋いでいるものがあれば、切った数と出所を 1 行出す"`・`echo "Scenario: 溜まった DB は、印つきの準備の後に空になる"`・`echo "Scenario: 作り直しでは、DB の network を手放さない"`・
  `echo "Scenario: ほかの compose project と compose の外のものには触らない"`・`echo "Scenario: 作り直した後も、所有者とアプリの役割で繋がる"`（`.env` か環境変数の合言葉で、コンテナの中の `psql -h 127.0.0.1` で繋ぐ）/
  6. で `echo "Scenario: 繋いでいるものが無ければ、切った接続は 0 と出す"`・`echo "Scenario: 作り直しの後、前の DB の volume は残らない"`（5. で控えた名前が無く、5. と 6. の後とも volume がちょうど 1 つ）/
  7. で `echo "Scenario: DB のコンテナが無くても、切った接続は 0 と出して DB を立てる"` /
  8.（subnet のファイルがある木だけ）で `echo "Scenario: 同じ subnet を持つ、コンテナの繋がっていない network は消して立てる"`・`echo "Scenario: 同じ subnet の network にコンテナが繋がっていれば、何も消さずに非 0 で終わる"`。
  飛ばす段は `SKIP <理由>` を出す。失敗はどの段かを出して rc=1。終わった DB は空で役割が整っている。
  検証: `tools/check-verify-env-reset.sh` rc=0 / `bash -c '[ "$(grep -c "echo \"Scenario: " tools/check-verify-env-reset.sh)" -ge 23 ]'` rc=0 /
  `bash -c '[ -z "$(docker ps -aq --filter name=ch06-sentinel-)" ] && [ -z "$(docker volume ls -q --filter name=ch06-sentinel-)" ] && [ -z "$(docker network ls -q --filter name=ch06-sentinel-)" ]'` rc=0

## Task 6: CI と文書

- [ ] 6.1 `.github/workflows/ci.yml` の rust job の末尾（`./tools/check-ports.sh --self-test` の行の後）に `- run: ./tools/check-verify-env-reset.sh --self-test` と `- run: ./tools/check-verify-env-reset.sh` の 2 行を足す。
  `cargo test` の周りの行には触らない（CH03・CH04 が手を入れている。proposal の Impact）。
  検証: `bash -c 'grep -qE "^ +- run: ./tools/check-verify-env-reset.sh --self-test$" .github/workflows/ci.yml && grep -qE "^ +- run: ./tools/check-verify-env-reset.sh$" .github/workflows/ci.yml'` rc=0 /
  `bash -c '! git diff origin/main...HEAD -- .github/workflows/ci.yml | grep -E "^[-+][^-+]" | grep -v "check-verify-env-reset" | grep -q .'` rc=0
- [ ] 6.2 文書（design D11）: `docs/testing.md` の CI の表の rust 行に検査を足す。`docs/flow-gates.md` の検査の表に `tools/check-verify-env-reset.sh` の 1 行（見るもの: 印つきの準備で worktree の DB が空から作り直され、作り直さない木・切った接続・作り直せないときの止まり方・塞いでいる network。走る場所: CI の rust、手元）。
  検証: `bash -c 'grep -q "check-verify-env-reset" docs/testing.md && grep -q "check-verify-env-reset" docs/flow-gates.md'` rc=0

## Task 7: 完了の判定と全体の確かめ

- [ ] 7.1 作り直した直後の DB で、server の試験が通る（依頼の完了の判定 2。移行は試験の初回が当てる）。本数と時間を design の「計測の記録」に書く。
  検証: `bash -c 'set -a; [ -f .env ] && . ./.env; set +a; scripts/quiet-run ct -- cargo test -p ashiato-server'` rc=0 / `bash -c '[ "$(cargo test -p ashiato-server -- --list 2>/dev/null | grep -c ": test$")" -ge 1000 ]'` rc=0
- [ ] 7.2 変えないものに触っていない（Global Constraints）。
  検証: `bash -c '[ -z "$(git diff origin/main...HEAD -- crates migrations tools/stack.sh tools/smoke.sh tools/check-immutable.sh tools/check-panic-log.sh tools/dev.sh tools/record-env.sh tools/db-roles.sh docker-compose.yml)" ]'` rc=0
- [ ] 7.3 鎖の検査。
  検証: `openspec validate ch06-reset-verify-db --strict` rc=0 / `python3 scripts/check_scenarios.py . ch06-reset-verify-db` rc=0 / `tools/check-private.sh` rc=0 / `tools/check-ports.sh --self-test` rc=0 / `tools/check-verify-env-reset.sh --self-test` rc=0
