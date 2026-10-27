# CH06 design — 検証のたびにテスト用 DB を作り直す

> 第 1 回は推奨の答え（Q1〜Q3）で先に書いた。本人の答えが推奨と違えば `spec --answers` が写し直す。
> 前提は `premises.md`（P1〜P15）、聞かずに決めたものは同じファイルの末尾（C1〜C6）。spec-review（`review/spec.md` R1〜R14）を受けて D1・D4〜D8 を直した。

## Context

- `scripts/verify-run` は、どの検証コマンドの前にも `tools/verify-env.sh` を `HARNESS_VERIFY=1` で呼び、非 0 なら BLOCKED_INFRA にして検証を走らせない（`scripts/evidence.py:374-388`。P3）。
  印は検証コマンドにも渡る。ashiato2 でこの印を見ているのは `tools/stack.sh`（`down -v`）だけで、`verify-env.sh` は DB を起動して役割を整えるだけ
- worktree の DB は compose project（ディレクトリ名）ごとに分かれている（P1）。DB の中身は postgres の image の匿名 volume にある（`docker-compose.yml` は volume を宣言しない。P11）
- 作り直し 1 回は約 5 秒（P9）。10/17〜10/24 の検証は 602 回
- この機械では Docker の既定のアドレスの空きが尽きていて、新しい compose project の network が作れない（P4）。明示の subnet を持つ project は作れる（P10）。同じ subnet の network は 2 つ作れない（`Pool overlaps`。review/spec.md R3 の実測）
- ハーネスは agent と `verify-run` に、agent の起動時の `.env` を `set -a` で読んだ環境を渡す（`tools/agent-env.sh:47-50`）。`.env` に置いた値は、後で `.env` を直しても古いまま環境変数として compose に勝つ（review/spec.md R5）
- CH04（下流の Task 10）は、試験を回ごとの database に書き、`ashiato` には積まない（P5）。CH06 と CH04 は同じ症状を別の手で止め、重ねても互いの前提を崩さない（Q1 の推奨）

## Goals / Non-Goals

Goals:
- 印があれば、検証の証跡がいつも空の DB から取られる（依頼の完了の判定 1・2）
- 溜まった DB から印つきの準備を呼ぶと `core.event` が 0 行になることを、機械が確かめる（判定 3）
- アドレスの空きが尽きた機械でも、worktree（確認バッチの作業ツリーを含む）の DB を立てられる（Q3 の推奨）

Non-Goals:
- 消えた worktree の compose project（network 15 個・動いたままの DB 4 個）と宙に浮いた volume（66 個）の片付け（Q3 の推奨の「触らない」）。
  例外は D5 の 1 つだけ: これから立てる network と同じ subnet を持ち、コンテナの繋がっていない network（今ある 15 個は既定の空きの subnet なので当たらない）
- `stack.sh`・`smoke.sh`・`check-immutable.sh`・`check-panic-log.sh` の `down -v` を変えること（C2。明示の subnet があれば、手放しても次の `up` で取り直せる —— D6）
- `tools/dev.sh`・`tools/record-env.sh`・`tools/db-roles.sh`・試験のコード（`crates/`）・移行・`docker-compose.yml` を変えること
- 作り直しの回数を減らすこと（C5）

## Decisions

### D1（仮）. 作り直しの手順 —— volume を名前で控え、コンテナを消し、その volume を消して立て直す（C2）

`HARNESS_VERIFY=1` で、D2 の判定が `reset` のとき、`verify-env.sh` は次の順に行う:

1. `. tools/ports.sh` → `tools/env-init.sh`（D7。subnet のファイルと `COMPOSE_FILE` を先に入れる —— 立て直しの `up` が新しい network の設定で立つ）
2. `docker compose ps -aq db` で DB のコンテナを探す。無ければ `切った接続: 0（DB のコンテナが無い）` を出して 5. へ
3. `docker inspect` でそのコンテナの volume（`Type` が `volume` の mount）の名前を控える。D4 の確かめをする
4. D3 の 1 行を出す。`docker compose rm -s -f db`（**`-v` を付けない**）でコンテナを消し、控えた volume を `docker volume rm` で 1 つずつ消す
5. D5 の network の確かめ → `docker compose up -d --wait db` → 今までの「繋げるまで待つ」→ `tools/db-roles.sh`

`docker compose down -v` は使わない（network を手放す。P4）。`up -V`・`--force-recreate` は使わない（古い volume が残る。P11）。
`rm` に `-v` を付けないのは、作り直しを経ていないコンテナでは `rm -v` が匿名 volume を先に消し、名指しの `volume rm` が「無い」で落ちるため（review/spec.md R2）。
一度作り直されたコンテナでは `rm -v` が volume を残す（P11）ので、`-v` に頼らず名前で消す形にそろえる。移行は当てない（C3）。どの段で docker が失敗しても非 0 で終わる（D4）。
反転条件: compose か docker の版が上がって `rm` が匿名 volume を消す・残すの扱いが変わり、本体の「2 回続けて作り直す」段（D8 の 5. と 6.）が落ちたら、手順を実測し直す。

### D2（仮）. 作り直すかの判定と `--plan`（C1）

判定は印・木・CI の 3 つだけで決める:

| `HARNESS_VERIFY` | 木 | `CI` | 判定 |
|---|---|---|---|
| `1` 以外 | どれでも | どれでも | `keep: 印が無い` |
| `1` | worktree（git-dir と共通ディレクトリが違う） | どれでも | `reset` |
| `1` | worktree でない木（main の checkout・CI の checkout） | `true` | `reset` |
| `1` | worktree でない木 | `true` 以外 | `keep: main の checkout（開発用の起動の DB）は作り直さない` |

worktree かどうかの見分け方は `tools/ports.sh` と同じ（`git rev-parse --git-dir --git-common-dir`）。git の木でなければ worktree でない木と読む（作り直さない側）。
`tools/verify-env.sh --plan` は判定の 1 行を出して rc=0 で終わり、docker にも `.env` にも触らない（`env-init.sh` も呼ばない）。
`keep` のときは今までどおり起動して待ち、役割を整える。印があって `keep` のときだけ、判定の 1 行を標準エラーに出す。
確認バッチの作業ツリー（`-verify`）は worktree なので作り直す（`verify-run` を呼ぶ係は無い。C1）。
反転条件: CI でない経路で `CI=true` が立つ（手元の道具が `CI` を立てる）と分かったら、CI の見分けを `GITHUB_ACTIONS=true` に替える。

### D3. 切った接続の 1 行（Q2 の推奨）

作り直す前に、動いている DB の `pg_stat_activity` から、準備の psql 自身を除いた `client backend` を数え、
`verify-env: DB を作り直す（切った接続: <数>。<application_name>@<接続元>, …）` を 1 行出す。`application_name` が空なら `-`、接続元が無ければ（unix socket）`local`。
同じ出所は `×<数>` でまとめ、10 種を超えたら残りを `…` にする。DB が止まっていた・問えなかったときは `切った接続: 0（DB は止まっていた）`、コンテナが無いときは `切った接続: 0（DB のコンテナが無い）`。
`切った接続: <数>` の形と、0 でも 1 行出すことは spec が持つ。待たない・ロックで並ばない（Q2 の推奨。グラフは 1 つの worktree の中で並列にしない）。

### D4. 作り直せないとき —— 消す前に確かめ、だめなら何も消さずに非 0（C6）

D1 の 3. で、控えた volume を `docker ps -aq --filter volume=<名前>` で引き、この DB のコンテナのほかに使っているコンテナがあれば、
volume とそのコンテナの名前を標準エラーに出し、**何も消さずに** rc=1 で終わる（消した後で `volume rm` が落ちると、コンテナが無く volume が宙に浮いたまま次の `up` が新しい volume で立ち、残りを片付ける手がかりが消える）。
4. と 5. で docker が失敗したときも rc=1。どちらも溜まった DB のまま rc=0 を返さない（ハーネスの約束「証跡は溜まった状態に頼らない」）。

### D5（仮）. subnet は DB の port から決め、塞いでいる空の network だけを消す

`tools/ports.sh` に関数 `ashiato_db_subnet` を足す。`off = ASHIATO_DB_PORT − 55400` として、`0 ≤ off < 512` のとき `10.<200+⌊off/256⌋>.<off mod 256>.0/24` を `printf` で組んで出し、範囲の外なら何も出さない。
変数としては export しない（`.env` や環境から古い値が回り込まない。D6）。
port は `ports.sh` の規則で生きている木どうしで被らない（main 55432・`-verify` 55499・`<main>-st<NN>` 55500+NN・台帳の木 55600+n）ので、subnet も被らない。
Q3 の推奨の「台帳の番号から割り当てる」は、port を通して実現する: 台帳の木は台帳の番号 n から port が決まり、そこから subnet が決まる。台帳を持たない木（main・`-verify`・`<main>-st<NN>`）は port の固定の規則から決まる。subnet だけの台帳は別に持たない（被らない割り当てを 2 つ持つと食い違う）。

**番号を継いだ木**: 台帳は消えた worktree の番号を使い回し（`ports.sh:27`）、消えた worktree の network は残る（Non-Goals）。番号を継いだ次の木は、同じ subnet の新しい network を作ろうとして `Pool overlaps` で落ちる（review/spec.md R3）。
そこで `verify-env.sh` は、subnet のファイルがあり、自分の project の network がまだ無いとき、`up` の前に `docker network ls` と `docker network inspect` で同じ subnet を持つ別の network を探す:
- コンテナが 1 つも繋がっていない（`docker ps -aq --filter network=<名前>` が空）→ その network だけを消して立てる（何も失わない。生きている木は port が違うので同じ subnet を持たない）
- コンテナが繋がっている → 何も消さずに、network とコンテナの名前を出して rc=1（消えた worktree の DB が動いたままなど。人が止める）。
  繋がったコンテナの compose の `working_dir` の印が無いディレクトリを指すなら、消えた worktree のものと出し、外し方（`docker rm -f … && docker network rm …`。volume は残る）を添える（review/code.md R20。
  `archive.sh` は `git worktree remove` の前に DB を止めないので、止まった DB を残して消えた木の番号を継ぐと、この形になる。CH06 の前は止まったコンテナは port を持たず通っていた）
- 同じでなく重なるだけの範囲（subnet を含む /16・/23 など。VPN や別の道具）の network → ほかの持ち物なので消さず、network と範囲を出して rc=1（R23。下の反転条件に当たったことを名前つきで見せる）

network の範囲は `docker network ls -q | xargs docker network inspect` の 1 回で全部引く（R29）。自分の network の名前は `docker compose config --format json` の `networks.default.name` から（R25）。

これは Q3 の推奨の「消えた worktree の残りは片付けない」の例外で、これから立てる network を塞いでいる空の network 1 つに限る。今ある 15 個（既定の空きの subnet）には当たらない。
`10.200` と `10.201` を選んだのは、Docker の既定の空き（172.17〜172.31 の /16 と 192.168 の /20）と、この機械の経路（WSL の eth0・Tailscale の CGNAT）のどれとも重ならないため。
**追跡するファイル（台本・試験・文書）に点区切りの 4 つ組を書かない**（`tools/check-private.sh` が落とす。P14）。期待値も `printf` で組む。生成物の `docker-compose.subnet.yml` は追跡しない（D6）。
反転条件: 機械に `10.200` か `10.201` で始まる範囲と重なる経路（VPN など）が入った、port の規則が 512 個の幅を超えた、別の repo が同じ範囲を使い始めた（`up` が `Pool overlaps` で落ち、塞いでいる network にコンテナがある）なら、範囲を替える。
本人の第 1 回の答えが「片付けない」を例外なく求めるなら、空の network も消さず rc=1 にする。

### D6（仮）. subnet は生成した compose のファイルと `.env` の `COMPOSE_FILE` で届ける

`tools/env-init.sh` が、木の port から決めた subnet（D5）を書いた `docker-compose.subnet.yml`（default の network の `ipam.config[0].subnet` だけ）を毎回作り直し、`.env` に
`COMPOSE_FILE=docker-compose.yml:docker-compose.subnet.yml`（木に `docker-compose.override.yml` があれば `:docker-compose.override.yml` を足す）を置く。`docker-compose.subnet.yml` は `.gitignore` に足す（生成物で、点区切りの値を持つ）。
compose は project のディレクトリの `.env` を呼び出し側の協力なしに読むので、`ports.sh` を読まない呼び出し口（`db-roles.sh`・`check-db-secret.sh`・`check-exposure.sh` の `compose config`・`archive-shape.sh`・`pg_dump-docker.sh`・人が打つ `docker compose`）にも効く（P13）。
ハーネスが agent の起動時に `.env` を環境に渡す `tools/agent-env.sh` は、`COMPOSE_FILE` だけを渡さない（`unset`）。値の形は木によらず同じでも、環境の値は `.env` に勝つので、
渡すと録画の木（`.env` に `COMPOSE_FILE` が無い）の compose が無い subnet のファイルを探して落ち、override のある木では override が外れる（review/code.md R15。compose は project の `.env` を自分で読むので、渡さなくてよい）。subnet の値は環境変数に載せない（review/spec.md R5: 環境変数は `.env` に勝ち、空の環境変数でも勝つ）。
`.env` の無い CI は base の `docker-compose.yml` だけで立ち、subnet を明示しない（今までどおり）。

採らなかったもの: `.env` に `ASHIATO_DB_SUBNET` を置いて補間する（古い値が環境変数として勝つ。R5）/ base の `docker-compose.yml` に `${ASHIATO_DB_SUBNET:?}` を書く（`ports.sh` を読まない呼び出し口と CI の素の `docker compose up` が補間で落ちる。実測）/
`docker-compose.override.yml` に書く（`tools/record-env.sh` や手で置く。取り合う）/ 全部の呼び出し口に `. tools/ports.sh` を足す（13 か所と人の手。漏れが見えない）。
`COMPOSE_FILE` を置くと compose は `docker-compose.override.yml` を自動では読まなくなるので、木にあれば並べる（`ashiato2-integ`・`ashiato2-rec-st22` に手で置いたものがある。review/spec.md R10）。
既にある木では、`COMPOSE_FILE` が入った後の最初の `up` で network が作り直され、DB のコンテナは作り直されずに繋ぎ直す（中身は残る。P12）。
`docker-compose.subnet.yml` が消えた（`git clean` など）木で `docker compose` を打つと「ファイルが無い」で落ちる。次の `verify-env.sh` か `env-init.sh` が作り直す。
同じく、env-init の後に手で置いた override は、次の `verify-env.sh` か `env-init.sh` までは読まれない（`stack.sh`・`smoke.sh` などは env-init を呼ばない。review/code.md R22。手で置くのは `ashiato2-integ` などで、録画の木は `.env` に `COMPOSE_FILE` が無いので自動で読む）。
`stack.sh` などの `down -v` が network を手放しても、明示の subnet は既定の空きに依らないので、次の `up` で同じ subnet を取り直せる（P4 の危うさを閉じる）。
反転条件: compose が `.env` の `COMPOSE_FILE` を読まなくなったら、呼び出し口に `ports.sh` を足す形に替える。

### D7（仮）. env-init の新しい規則 —— `COMPOSE_FILE` と subnet のファイルだけを保つ・CI で作らない・録画の `.env` に触らない

`tools/env-init.sh` は次の順に決める:

1. `.env` が無く、`POSTGRES_PASSWORD`・`OWNER_DB_PASSWORD`・`APP_DB_PASSWORD` がどれも環境変数にある → 何もしない（CI。`.env` を作ると `db-roles.sh` がそれを読み、`$GITHUB_ENV` と違う合言葉に揃えて同じ job の後ろの段が認証で落ちる。P15・review/deep.md R2）
2. `.env` が `COMPOSE_PROJECT_NAME` の行を持つ → 何もしない（`tools/record-env.sh` が書いた録画の `.env`）
3. `.env` が無い → 今までどおり乱数の合言葉で作り、`COMPOSE_FILE` も書く
4. `.env` がある → `COMPOSE_FILE` の行を今の木の値にそろえる（無ければ末尾に足し、違えば置き換える）。ほかの行は 1 文字も変えない
5. subnet（D5。環境変数も `.env` も見ない）を書いた `docker-compose.subnet.yml` を作り直す（中身が同じなら書かない）。subnet が出ない（範囲の外の port）ときは `docker-compose.subnet.yml` を作らず、`COMPOSE_FILE` の行を消す（今までどおり既定の空きから取る）

`.env` の書き換えは、同じディレクトリの一時ファイルに `cat` で書いてから中身だけを入れ替える（`cat tmp > .env`。inode と権限を保つ）。何度呼んでも同じ結果になる。
「ある `.env` には触らない」（今の注記。合言葉を変えると既にある volume に繋がらなくなるため）は、合言葉の行に限って残る。
確認バッチの作業ツリーの準備（`tools/verify-prep.sh`）は main の `.env` を写すか作った後に `tools/env-init.sh` を呼ぶ（写した `COMPOSE_FILE` は同じ形で、subnet のファイルはその木の値で作られる。review/spec.md R4）。
反転条件: `.env` に `COMPOSE_FILE` を書かれたくない木が出てきたら（main の checkout の開発用の起動など）、その木を 2. と同じく外す印を決める。

### D8（仮）. 検査 `tools/check-verify-env-reset.sh` —— 本体と `--self-test`（C4）

**`--self-test`**（docker の daemon に触らない。CI の rust job と手元）: 一時の repo と worktree（`tools/check-ports.sh` と同じ作り方）に `tools/verify-env.sh`・`ports.sh`・`env-init.sh` と `docker-compose.yml` を写す。
一時の木の道具は `env -u` で `POSTGRES_PASSWORD` `OWNER_DB_PASSWORD` `APP_DB_PASSWORD` `ASHIATO_DB_SUBNET` `ASHIATO_DB_PORT` `ASHIATO_HTTP_PORT` `COMPOSE_FILE` `CI` `HARNESS_VERIFY` を落として呼び、場合ごとに要るものだけ足す（CI と手元で同じ結果になる。review/spec.md R6）。
D2 の表を `--plan` で全部確かめる（Requirement「作り直すのは…」の `--plan` の 3 本）。D7 の場合を一時の木で確かめる（Requirement「env-init は…」の 6 本）。
`env-init.sh` の後に `docker compose config --format json` の subnet が `ashiato_db_subnet` の値になることを見る（`config` は network を作らない）。

**本体**（この木の DB を作り直す。worktree か `CI=true` でだけ走り、それ以外は理由を出して rc=2）: 呼び出し側の `HARNESS_VERIFY` に依らず、各段で `env -u HARNESS_VERIFY` / `HARNESS_VERIFY=1` を明示する。

1. `HARNESS_VERIFY=1 tools/verify-env.sh` で空にする（前の状態に依らない出発点）
2. 溜まった状態を作る: `core` schema と代わりの `core.event`（1 列）に 3 行、`ch06_keep_<乱数>` database。空の DB には移行が当たっていないので、本物の `core.event` ではなく代わりの表で「行のある DB」を作る
3. 印なしで呼び、3 行が残る。**worktree でない木（CI）では**、続けて `env -u CI HARNESS_VERIFY=1` で呼び、3 行が残り標準エラーに `keep:` の 1 行がある（worktree では通れないので、そう出して飛ばす。CI が通す）
4. compose の外に `ch06-sentinel-<乱数>` のコンテナ（`docker create`。動かさない）を DB の volume を付けて置き、印つきで呼ぶ → rc≠0・名前が出る・DB のコンテナの id と 3 行が同じ。そのコンテナを消す
5. 別の compose project `ch06-sentinel-<乱数>`（一時の場所の compose のファイル。service `db`・`network_mode: none`・`sleep`）と、compose の外の `ch06-sentinel-<乱数>` のコンテナと volume（DB の volume とは別）を置く。volume の名前と network の id を控え、`pg_sleep(600)` の psql を 12 本（`PGAPPNAME=ch06-holder` が 2 本、`ch06-holder-01`〜`10` が 1 本ずつ）走らせ、`pg_stat_activity` に出るまで待ってから印つきで呼ぶ →
   `切った接続: 12。ch06-holder@local ×2, ch06-holder-01@local, …, ch06-holder-09@local, …` の 1 行（D3 の `×<数>`・10 種で切る・`…` を固定する。review/code.md R16）、12 本とも psql が非 0、psql が非 0、`core.event` が無いか 0 行、`ch06_keep_` が無い、前の volume が無く volume がちょうど 1 つ、network の id が同じ、sentinel が全部同じ id で残る、`ashiato_owner` と `ashiato_app` で繋がり所有者が `ashiato_owner`
6. もう一度印つきで呼び、`切った接続: 0` の 1 行・rc=0・前の volume が無く volume がちょうど 1 つ（2 回続けて作り直せる）
7. `docker compose rm -s -f db` と volume の削除で DB のコンテナを無くしてから印つきで呼び、`切った接続: 0` の 1 行と、DB のコンテナが立っていること
8. **subnet のファイルがある木（worktree）では**: DB の network を手放す（`down` は使わない —— 匿名 volume が宙に浮く。P11。D1 と同じく `docker compose rm -s -f db` → 控えた volume を `docker volume rm` → `docker network rm <project>_default`）→ 同じ subnet の `ch06-sentinel-<乱数>` network（コンテナ無し）を作って印つきで呼ぶ → rc=0・sentinel の network が無い・DB の network がその subnet。
   もう一度同じ手順で手放す → 同じ subnet の sentinel の network に `docker create --network` で sentinel のコンテナ（compose の `working_dir` の印が無いディレクトリを指す）を繋いで呼ぶ → rc≠0・名前と「消えた worktree」の外し方が出る・network とコンテナが同じ id。
   sentinel を消し、subnet を含む /23 の sentinel の network を作って呼ぶ → rc≠0・名前が出る・network が同じ id（R23）。それを消してもう一度印つきで呼び、DB を立て直す。CI（subnet のファイルが無い）ではそう出して飛ばす

sentinel は `trap` で必ず消す（コンテナは `docker rm -f -v`。postgres の image の `VOLUME` が作る名前の無い volume も残さない。名指しで付けた DB の volume は `-v` でも消えない。review/code.md R14）。終わった DB は空で役割が整っている（後ろの段の試験がそのまま使える）。
CI では rust job の末尾（`check-ports.sh --self-test` の後）に `--self-test` と本体の 2 行を置く。`CI=true` は GitHub が渡す。
本体の 3. の CI だけの段と 8. の worktree だけの段は、CI と `verify-run`（worktree）の証跡を合わせて Scenario の全部を通す。
反転条件: 本体が CI で 90 秒を超えるなら、6. と 7. を 1 回にまとめる。

### D9. CH04 との関係

CH06 は `testdb.rs` に触らない。CH04 の回ごとの database は volume ごと消えるので、同じ worktree で並んで走っている回は切られる（Q2 の推奨。Risks）。
CH04 は空の DB からテンプレートを作るので、作り直しの後の初回も通る。どちらが先に merge されても互いの前提は崩れない。CH06 を CH04 に依らせない（`requires: []`）。

### D10（仮）. capability は新しい `verify-environment`。`docs/changes/CH06.md` の `capability` は空のまま

CH04 の `test-database` はまだ正典に無く、同じ新しい capability に `## Purpose` を持つ ADDED を 2 つの change が積むと、archive の後の側の Purpose の扱いが決まっていない（CH04 の D13 と同じ理由）。
検証の準備（ハーネスとの約束）と試験の中の database の持ち方は、壊れ方も直し方も別。front matter の `capability` は CH04 の D12 と同じく空のまま（触る change が無い。失うものは無い）。
反転条件: 両方が archive された後に 1 つにまとめたくなったら、Requirement を移す change を立てる（OpenSpec は capability の改名を許さない）。

### D11. 文書

`tools/verify-env.sh` の冒頭の注記（「何度呼んでも同じ」を「印が無ければ何度呼んでも同じ。印があれば毎回空から」に）、`tools/env-init.sh` の注記（D7）、`tools/ports.sh` の規則の節（D5）、
`docs/testing.md` の CI の表の rust 行（検査の数）、`docs/flow-gates.md` の検査の表に `tools/check-verify-env-reset.sh` の 1 行。
`docs/changes/CH06.md` は CH03・CH04 と同じ節の並び（`## 価値` `## 依頼（逐語）` `## 壊してはいけないもの` `## 完了の判定` を 1 つずつ）にした（`hx change` の雛形の重複は消した。機械は最初の `## 完了の判定` を読む。review/spec.md R7）。

### D12（仮）. 追跡する文書の subnet は点区切りで書かない

review/code.md・evidence.jsonl などの追跡する文書に subnet を書くときは `(第 2 組, 第 3 組)/接頭長` の形にする（台本の `printf '10.%d.%d.0/24'` と同じく、点区切りの 4 つ組を書かない。`tools/check-private.sh` が落とす）。
値は port の規則から決まる docker の subnet で手元の網の実値ではないが、`HARNESS_ALLOW_PRIVATE_NAMES=1` で通す例外は作らない（review/code.md R19）。
反転条件: `tools/check-private.sh` が docker の subnet の範囲（D5 の 2 つ）を私設 IP から外すようになったら、点区切りで書いてよい。

## Risks / Trade-offs

- **同じ worktree で並んで走っているものは切られる**（Q2 の推奨）: agent が背後で走らせた `cargo test`・人が立てた `tools/dev.sh`・CH04 の並んだ回。CH04 の片付けの助言ロックは volume の作り直しを止めない（P6）。
  切られた側には D3 の 1 行が verify-env の出力（証跡のログ）に残る
- **検証 1 回あたり約 5 秒＋役割の整え**（P9。1 週間で約 1 時間）。DB を使わない検証の前も作り直す（C5）
- **main の checkout の `.env` にも `COMPOSE_FILE` が入る**（`verify-env.sh` がそこで走ったとき）。次の `up` で network が作り直され、開発用の DB のコンテナは繋ぎ直しのために 1 回止まって動く（中身は残る。P12）。開発用のサーバの接続は 1 回切れる
- **録画の環境（`COMPOSE_PROJECT_NAME` を持つ `.env`）は既定のアドレスの空きのまま**（D7）。空きが尽きている間は録画の DB が立たないことがある（今と同じ）
- **塞いでいる network にコンテナが繋がっていると、その木の検証は BLOCKED_INFRA のまま**（D5）。出力に network とコンテナの名前が出るので、人が止める
- **別の repo・別の道具が同じ subnet を使うと `up` が落ちる**（D5 の反転条件）。黙って取り違えはせず、BLOCKED_INFRA として見える
- **消えた worktree の残りはそのまま**（Non-Goals）。生きている木は `COMPOSE_FILE` が入った後の `up` で既定の空きの network を手放すので、空きは戻っていく

## Migration Plan

DB の移行は無い。rebase した worktree で最初に `verify-env.sh` が走ると `.env` に `COMPOSE_FILE` が入り、`docker-compose.subnet.yml` ができて、network が明示の subnet に替わる（P12）。戻すときは `.env` の `COMPOSE_FILE` の行を消す。

## 計測の記録（下流が書く）

- 作り直し 1 回の時間（`HARNESS_VERIFY=1 tools/verify-env.sh` の実時間。3 回）: 12 秒・12 秒・7 秒（2026-10-25、ch06 worktree、証跡 4.3 の run。10 秒を超えた回がある）。
  遅い段（`bash -x` の時刻差、2 回）: `docker compose rm -s -f db` 約 1.3 秒、`docker compose up -d --wait db`（healthcheck 待ち）約 5.2〜5.7 秒、残りは役割の整えと接続の確かめ（約 1〜5 秒、回で揺れる）。切る・消す・network の確かめは各 0.1〜0.2 秒。支配的なのは新しい DB の起動待ち
- 作り直した直後の `cargo test -p ashiato-server` の本数と時間（`scripts/verify-run` の証跡）:
  `--list` の本数は 1386（lib）+ 9（tests/server_startup）。`scripts/verify-run 7.1`（並列度は既定）を 2026-10-25 に 5 回走らせ、4 回 FAIL（rc=101、650〜830 秒）・1 回 PASS（木 4c1f3ac6、rc=0、754 秒）。
  落ちたのは時間・負荷に依る試験で回ごとに違う（`stay_rebuild_chain_of_short_moves_settles`（作り直し 3 秒の上限）が 4 回中 4 回、他に `stays_migration_applies_twice`、`archive_scan_interval_change_takes_effect_without_restart`（走査 7 回））。
  どれも単独では通り（2.5 秒）、`crates/` は変えていない。`--test-threads=4` は手元で 1 回 1386+9 本全部 ok（705 秒）だったが再現せず、`verify-run --candidate` で走らせた回は FAIL
  （1385 通過・1 落ち、710 秒。`stay_tests::fresh_db` の `DROP DATABASE … WITH (FORCE)` が「permission denied to terminate process」。`ashiato_owner` は `ashiato_app` の接続を切れない）。並列度の問題とは決められず、候補は採っていない。
  マシンの load average は約 10（16 コア）。試験の時間の上限が並列の負荷に負けている疑いで、作り直しの副作用ではないと見る
- `tools/check-verify-env-reset.sh` 本体の時間（手元と CI）:
