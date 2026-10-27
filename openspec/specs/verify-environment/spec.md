# verify-environment Specification

## Purpose
検証の準備（`tools/verify-env.sh`）が、ハーネスの印（`HARNESS_VERIFY=1`）を受けて worktree のテスト用 DB を空から作り直し、証跡が前の検証の残りに頼らないこと。作り直さない木・切った接続の知らせ・作り直せないときの止まり方と、worktree ごとの DB の network が被らない subnet で立つことを含む。並走する Story と change が同じ機械の上で証跡を取り続けるための、開発の道具の約束。

## Requirements

### Requirement: 印があれば、検証の準備がこの worktree の DB を空から作り直す

WHEN 検証の準備（`tools/verify-env.sh`）が `HARNESS_VERIFY=1` で呼ばれ、作り直す木（下の Requirement）であるとき、
THE SYSTEM SHALL この worktree の compose project の DB のコンテナとその volume を消して空の DB を立て直し、DB の役割（`tools/db-roles.sh`）を整えてから rc=0 で終わる。
前の DB にあった表・行・database は、立て直した DB に 1 つも残らない。移行は当てない（server の試験の初回が当てる）。
消すのは、この worktree の DB のコンテナが使っていた volume だけで、名前で 1 つずつ消す。立て直した後に前の volume は残らない。
DB の network は消さない。ほかの compose project のコンテナ・volume と、compose の外のコンテナ・volume には触らない。
導出元: CH06

#### Scenario: 溜まった DB は、印つきの準備の後に空になる
- **WHEN** worktree の DB に `core.event` の行が 3 件と、名前が `ch06_keep_` で始まる database が 1 つある状態で、`HARNESS_VERIFY=1 tools/verify-env.sh` を呼ぶ
- **THEN** rc=0 で、`core.event` は無いか 0 行である
- **AND** `ch06_keep_` で始まる database は無い

#### Scenario: 作り直した後も、所有者とアプリの役割で繋がる
- **WHEN** `HARNESS_VERIFY=1 tools/verify-env.sh` が作り直しを終えた後に、`.env`（か環境変数）の合言葉で `ashiato_owner` と `ashiato_app` が `ashiato` database に繋ぐ
- **THEN** どちらも繋がり、`ashiato` database の所有者は `ashiato_owner` である

#### Scenario: 作り直しの後、前の DB の volume は残らない
- **WHEN** 作り直しの前に DB のコンテナの volume の名前を控え、`HARNESS_VERIFY=1 tools/verify-env.sh` を 2 回続けて呼ぶ
- **THEN** どちらも rc=0 で、控えた名前の volume は `docker volume ls` に無い
- **AND** 立て直した DB のコンテナの volume はちょうど 1 つである

#### Scenario: 作り直しでは、DB の network を手放さない
- **WHEN** 作り直しの前に DB の network の id を控え、`HARNESS_VERIFY=1 tools/verify-env.sh` を呼ぶ
- **THEN** 作り直しの後の DB の network の id は控えた id と同じである

#### Scenario: ほかの compose project と compose の外のものには触らない
- **WHEN** 別の compose project（名前が `ch06-sentinel-` で始まり、service の名前が `db`）のコンテナと、compose の外の `ch06-sentinel-` で始まるコンテナとそれが使う volume を置いた状態で、`HARNESS_VERIFY=1 tools/verify-env.sh` を呼ぶ
- **THEN** どちらのコンテナと volume も、作り直しの後に同じ id で残っている

### Requirement: 作り直すのは、印があり、worktree か CI のときだけ

THE SYSTEM SHALL 検証の準備が DB を作り直すのを、`HARNESS_VERIFY=1` があり、かつ（木が git の worktree である、か `CI=true`）のときだけにする。
それ以外は今までどおり、DB を起動して繋げるまで待ち、役割を整えるだけで、DB の中身に触らない。
CI でない main の checkout（worktree でない木）で印があるときは、作り直さなかったことと理由を標準エラーに 1 行出す。
判定は `tools/verify-env.sh --plan` で、docker にも `.env` にも触らずに `reset` か `keep: <理由>` の 1 行として出せる。
導出元: CH06

#### Scenario: 印が無ければ作り直さず、溜まった行は残る
- **WHEN** worktree の DB に `core.event` の行が 3 件ある状態で、`HARNESS_VERIFY` を渡さずに `tools/verify-env.sh` を呼ぶ
- **THEN** rc=0 で、`core.event` の行は 3 件のままである

#### Scenario: CI でない main の checkout では、印があっても作り直さない
- **WHEN** worktree でない木で、`CI` を渡さず `HARNESS_VERIFY=1` で `tools/verify-env.sh --plan` を呼ぶ
- **THEN** 出力は `keep:` で始まり、main の checkout であることを理由に挙げる

#### Scenario: CI でない main の checkout では、印つきで呼んでも行が残る
- **WHEN** worktree でない木の DB に `core.event` の行が 3 件ある状態で、`CI` を渡さず `HARNESS_VERIFY=1` で `tools/verify-env.sh` を呼ぶ
- **THEN** rc=0 で、`core.event` の行は 3 件のままである
- **AND** 標準エラーに `keep:` で始まる 1 行がある

#### Scenario: CI の main の checkout と、worktree では、印があれば作り直す
- **WHEN** worktree でない木で `CI=true HARNESS_VERIFY=1`、`<main>-st<NN>`・`-verify`・`-ch<NN>` の worktree で `HARNESS_VERIFY=1` を渡して `tools/verify-env.sh --plan` を呼ぶ
- **THEN** どれも出力は `reset` である

#### Scenario: 印が無ければ、どの木でも作り直さない判定になる
- **WHEN** worktree でない木（`CI=true` を含む）と、`<main>-st<NN>`・`-verify`・`-ch<NN>` の worktree で、`HARNESS_VERIFY` を渡さずに `tools/verify-env.sh --plan` を呼ぶ
- **THEN** どれも出力は `keep:` で始まる

### Requirement: 作り直しは、繋いでいたものを切り、その数と出所を 1 行出す

WHEN 作り直すとき、
THE SYSTEM SHALL 待たずに作り直して DB への接続を切り、作り直しの前に数えた接続の数と、接続ごとの出所（`application_name` と接続元。接続元が無ければ `local`）を、`切った接続: <数>` を含む 1 行で出す。
繋いでいるものが無いとき、DB が止まっていたとき、DB のコンテナが無かったときは、`切った接続: 0` を含む 1 行を出す。
導出元: CH06

#### Scenario: 繋いでいるものがあれば、切った数と出所を 1 行出す
- **WHEN** `application_name` が `ch06-holder` の接続が worktree の DB で `pg_sleep` を走らせている間に、`HARNESS_VERIFY=1 tools/verify-env.sh` を呼ぶ
- **THEN** rc=0 で、出力の 1 行に `切った接続: 1` と `ch06-holder` がある
- **AND** `ch06-holder` の接続を持っていた psql は非 0 で終わっている

#### Scenario: 繋いでいるものが無ければ、切った接続は 0 と出す
- **WHEN** 準備の外から誰も繋いでいない worktree の DB で、`HARNESS_VERIFY=1 tools/verify-env.sh` を呼ぶ
- **THEN** rc=0 で、出力の 1 行に `切った接続: 0` がある

#### Scenario: DB のコンテナが無くても、切った接続は 0 と出して DB を立てる
- **WHEN** worktree の DB のコンテナが無い状態で、`HARNESS_VERIFY=1 tools/verify-env.sh` を呼ぶ
- **THEN** rc=0 で、出力の 1 行に `切った接続: 0` があり、DB のコンテナが立っている

### Requirement: 作り直せなければ、何も消さずに非 0 で終わる

IF 作り直しの前に、DB の volume をこの worktree の DB のコンテナのほかのコンテナが使っていると分かったら、
THEN THE SYSTEM SHALL コンテナも volume も消さずに、その volume と使っているコンテナの名前を出して非 0 で終わる。
コンテナや volume を消す・立て直す途中で docker が失敗したときも非 0 で終わる。どちらのときも、検証の準備は溜まった DB のまま rc=0 を返さない。
導出元: CH06

#### Scenario: DB の volume をほかのコンテナが使っていれば、何も消さずに非 0 で終わる
- **WHEN** DB のコンテナの volume を、compose の外の `ch06-sentinel-` で始まるコンテナにも付けた状態で、`HARNESS_VERIFY=1 tools/verify-env.sh` を呼ぶ
- **THEN** rc は 0 でなく、出力にその volume の名前と `ch06-sentinel-` のコンテナの名前がある
- **AND** DB のコンテナは同じ id で残り、`core.event` の行は呼ぶ前と同じ件数である

### Requirement: worktree ごとの DB の network は、木ごとに被らない subnet で立つ

THE SYSTEM SHALL worktree ごとの DB の network の subnet（`/24`）を、その木の DB の port（`tools/ports.sh` の規則で、生きている木どうしで被らない）から決める。
`tools/env-init.sh` は、その subnet を書いた compose のファイル（`docker-compose.subnet.yml`。生成物で、追跡しない）を毎回作り直し、`.env` に `COMPOSE_FILE` を書く。呼び出し側が何も渡さない `docker compose` でも、DB の network はその subnet で立つ。
WHEN DB の network がまだ無いとき、THE SYSTEM SHALL 検証の準備が立てる前に、ほかの network が同じ subnet を持っていないかを確かめる。持っている network にコンテナが 1 つも繋がっていなければ（消えた worktree の残り）、その network だけを消して立てる。
コンテナが繋がっていれば、何も消さずに、その network とコンテナの名前を出して非 0 で終わる。
導出元: CH06

#### Scenario: どの 2 つの木も、DB の subnet が被らない
- **WHEN** 一時の repo の main の checkout と、`<main>-st<NN>`・`-verify`・`-ch<NN>` 2 つ・`-up-st<NN>`・`-rec-st<NN>` の worktree で、`tools/ports.sh` の subnet を読む
- **THEN** 7 つの木の subnet は全部違い、どれも `/24` である

#### Scenario: env-init の後は、compose の DB の network に木の subnet が入る
- **WHEN** 一時の木で `tools/env-init.sh` を呼んだ後に、環境変数を渡さずに `docker compose config` を読む
- **THEN** default の network の ipam の subnet は、その木の `tools/ports.sh` の subnet と同じである

#### Scenario: 同じ subnet を持つ、コンテナの繋がっていない network は消して立てる
- **WHEN** worktree の DB の network が無く、同じ subnet を持つ別の名前の network（`ch06-sentinel-` で始まる。コンテナは繋がっていない）がある状態で、`HARNESS_VERIFY=1 tools/verify-env.sh` を呼ぶ
- **THEN** rc=0 で、`ch06-sentinel-` の network は無く、worktree の DB の network がその subnet で立っている

#### Scenario: 同じ subnet の network にコンテナが繋がっていれば、何も消さずに非 0 で終わる
- **WHEN** worktree の DB の network が無く、同じ subnet を持つ別の名前の network に `ch06-sentinel-` で始まるコンテナが繋がっている状態で、`HARNESS_VERIFY=1 tools/verify-env.sh` を呼ぶ
- **THEN** rc は 0 でなく、出力にその network とコンテナの名前がある
- **AND** その network とコンテナは同じ id で残っている

### Requirement: env-init は、既にある .env の合言葉を変えずに、subnet の設定だけを保つ

THE SYSTEM SHALL `tools/env-init.sh` に、`.env` が既にあるときも `COMPOSE_FILE` の行を今の木の値（`docker-compose.yml` と `docker-compose.subnet.yml`、木に `docker-compose.override.yml` があればそれも）にそろえさせ、ほかの行（合言葉・URL など）は 1 文字も変えさせず、`.env` の権限も変えさせない。何度呼んでも結果は同じである。
subnet は環境変数からも `.env` からも読まず、木の port から決める（別の木から写した `.env` でも、その木の値になる）。
`POSTGRES_PASSWORD`・`OWNER_DB_PASSWORD`・`APP_DB_PASSWORD` がどれも環境変数で渡っていて `.env` が無いとき（CI）は、`.env` も compose のファイルも作らない。
`.env` が `COMPOSE_PROJECT_NAME` の行を持つとき（録画の環境が書いたもの）は、その `.env` に触らない。
確認バッチの作業ツリーの準備（`tools/verify-prep.sh`）は、`.env` を写した後か作った後に `tools/env-init.sh` を呼ぶ。
導出元: CH06

#### Scenario: 既にある .env に、合言葉を変えずに COMPOSE_FILE を足す
- **WHEN** `COMPOSE_FILE` の無い `.env` がある一時の木で `tools/env-init.sh` を呼ぶ
- **THEN** `.env` に `COMPOSE_FILE` の行が 1 つあり、ほかの行は呼ぶ前と同じである

#### Scenario: 別の木から写した .env でも、その木の subnet になり、何度呼んでも同じである
- **WHEN** 別の一時の木で作った `.env` を写した木で、別の subnet を環境変数 `ASHIATO_DB_SUBNET` に入れて `tools/env-init.sh` を 2 回呼ぶ
- **THEN** `docker-compose.subnet.yml` の subnet はその木の `tools/ports.sh` の subnet で、1 回目の後と 2 回目の後の `.env` と `docker-compose.subnet.yml` は同じである

#### Scenario: env-init が書き換えた .env の権限は、書き換える前と同じである
- **WHEN** 権限が 600 の `.env` がある一時の木で `tools/env-init.sh` を呼び、`COMPOSE_FILE` の行が足される
- **THEN** `.env` の権限は 600 のままである

#### Scenario: 木に override のファイルがあれば、COMPOSE_FILE に並べる
- **WHEN** `docker-compose.override.yml` がある一時の木で `tools/env-init.sh` を呼ぶ
- **THEN** `.env` の `COMPOSE_FILE` は `docker-compose.yml`・`docker-compose.subnet.yml`・`docker-compose.override.yml` の 3 つを並べる

#### Scenario: 合言葉が環境変数で渡っていれば、.env を作らない
- **WHEN** `.env` の無い一時の木で、3 つの合言葉を環境変数で渡して `tools/env-init.sh` を呼ぶ
- **THEN** rc=0 で、`.env` も `docker-compose.subnet.yml` も作られていない

#### Scenario: compose project の名前を持つ .env には触らない
- **WHEN** `COMPOSE_PROJECT_NAME` の行を持つ `.env` がある一時の木で `tools/env-init.sh` を呼ぶ
- **THEN** `.env` は呼ぶ前と同じである

### Requirement: 作り直したら、検証の準備は作り直したものを 1 行で宣言する

WHEN 検証の準備（`tools/verify-env.sh`）が `HARNESS_VERIFY=1` で呼ばれ、作り直す木（Requirement「作り直すのは、印があり、worktree か CI のときだけ」）であり、
この worktree の DB のコンテナが使っていた volume を 1 つ以上消したか、DB のコンテナが無くて新しく立て、DB の役割まで整えて rc=0 で終わるとき、
THE SYSTEM SHALL 標準出力に `HARNESS_RESET: testdb` だけの行を、行頭から 1 行だけ出す。DB のコンテナが止まっていた場合も同じである。
行の前に何も付けず、volume の名前や切った接続の数を足さない（それは切った接続の 1 行が持つ）。標準エラーには出さない。
宣言の行は `tools/verify-env.sh` 自身が出し、字 `HARNESS_RESET: testdb` をその本文に置く（ハーネスの点検は準備の本文の字で読む）。
作り直しの対象・手順・切った接続の 1 行・`--plan` の出力は変えない。
導出元: CH07

#### Scenario: 溜まった DB を作り直したら、宣言を 1 行出す
- **WHEN** worktree の DB に `core.event` の行が 3 件ある状態（DB のコンテナは volume を持って動いている）で、`HARNESS_VERIFY=1 tools/verify-env.sh` を呼ぶ
- **THEN** rc=0 で、標準出力に `HARNESS_RESET: testdb` だけの行がちょうど 1 つある
- **AND** 標準エラーに `HARNESS_RESET:` で始まる行は無い

#### Scenario: 2 回続けて作り直しても、どちらも宣言を 1 行出す
- **WHEN** `HARNESS_VERIFY=1 tools/verify-env.sh` が作り直しを終えた直後に、もう一度 `HARNESS_VERIFY=1 tools/verify-env.sh` を呼ぶ
- **THEN** rc=0 で、2 回目の標準出力にも `HARNESS_RESET: testdb` だけの行がちょうど 1 つある

#### Scenario: DB のコンテナが無くても、立てたら宣言を 1 行出す
- **WHEN** worktree の DB のコンテナとその volume が無い状態で、`HARNESS_VERIFY=1 tools/verify-env.sh` を呼ぶ
- **THEN** rc=0 で、DB のコンテナが立っており、標準出力に `HARNESS_RESET: testdb` だけの行がちょうど 1 つある

#### Scenario: DB のコンテナが止まっていても、作り直したら宣言を 1 行出す
- **WHEN** worktree の DB のコンテナを止めた（volume は持ったまま）状態で、`HARNESS_VERIFY=1 tools/verify-env.sh` を呼ぶ
- **THEN** rc=0 で、出力の 1 行に `切った接続: 0（DB は止まっていた）` がある
- **AND** 標準出力に `HARNESS_RESET: testdb` だけの行がちょうど 1 つある

#### Scenario: ハーネスの点検が、作り直して宣言する準備と認める
- **WHEN** この木で、準備の宣言を確かめる版のハーネスの点検 `python3 .harness2/scripts/init.py --check` を呼ぶ
- **THEN** 出力に `[人間] tools/verify-env.sh` を含む行が無く、`[ok] tools/verify-env.sh` を含む行がちょうど 1 つある

### Requirement: 作り直していなければ、検証の準備は宣言を出さない

THE SYSTEM SHALL 検証の準備が DB の中身を作り直さずに終わるとき、または非 0 で終わるときは、`HARNESS_RESET:` で始まる行を標準出力にも標準エラーにも出さない。
当たるのは、印が無いとき・CI でない main の checkout（worktree でない木）で印があるとき・git の木でないとき・作り直せずに（作り直しの途中で落ちたときを含む）非 0 で終わるとき・
DB のコンテナはあるが volume を 1 つも持たない（消した volume が 0 個の）ときである。`HARNESS_RESET: none` も出さない。
`tools/verify-env.sh --plan` は判定の 1 行（`reset` か `keep: <理由>`）だけを出し、宣言を出さない。
導出元: CH07

#### Scenario: 印が無ければ、宣言を出さない
- **WHEN** worktree の DB に `core.event` の行が 3 件ある状態で、`HARNESS_VERIFY` を渡さずに `tools/verify-env.sh` を呼ぶ
- **THEN** rc=0 で、`core.event` の行は 3 件のままである
- **AND** 標準出力にも標準エラーにも `HARNESS_RESET:` で始まる行は無い

#### Scenario: CI でない main の checkout では、印つきでも宣言を出さない
- **WHEN** worktree でない木の DB に `core.event` の行が 3 件ある状態で、`CI` を渡さず `HARNESS_VERIFY=1` で `tools/verify-env.sh` を呼ぶ
- **THEN** rc=0 で、標準エラーに `keep:` で始まる 1 行がある
- **AND** 標準出力にも標準エラーにも `HARNESS_RESET:` で始まる行は無い

#### Scenario: 作り直せずに非 0 で終わるときは、宣言を出さない
- **WHEN** DB のコンテナの volume を、compose の外の `ch06-sentinel-` で始まるコンテナにも付けた状態で、`HARNESS_VERIFY=1 tools/verify-env.sh` を呼ぶ
- **THEN** rc は 0 でなく、標準出力にも標準エラーにも `HARNESS_RESET:` で始まる行は無い

#### Scenario: 作り直しの途中で落ちたら、宣言を出さない
- **WHEN** 作り直しの前に DB のコンテナの volume の名前を控え、DB の役割を整える段が失敗する（`ENV_FILE` に無いファイルを指す）ようにして `HARNESS_VERIFY=1 tools/verify-env.sh` を呼ぶ
- **THEN** rc は 0 でなく、控えた名前の volume は無い（volume を消した後で落ちた）
- **AND** 標準出力にも標準エラーにも `HARNESS_RESET:` で始まる行は無い

#### Scenario: git の木でなければ、印つきでも作り直さない判定になる
- **WHEN** git の木でない一時の場所に写した `tools/verify-env.sh` を、`CI` を渡さず `HARNESS_VERIFY=1` で `--plan` で呼ぶ
- **THEN** 出力は `keep:` で始まる 1 行で、`HARNESS_RESET:` で始まる行は無い

#### Scenario: volume を持たない DB のコンテナでは、作り直しても宣言を出さない
- **WHEN** worktree の DB のコンテナが volume を 1 つも持たずに動いている状態で、`HARNESS_VERIFY=1 tools/verify-env.sh` を呼ぶ
- **THEN** rc=0 で、出力の 1 行に `切った接続: ` がある（作り直しの経路を通った）
- **AND** 標準出力にも標準エラーにも `HARNESS_RESET:` で始まる行は無い

#### Scenario: 判定だけを出すときは、宣言を出さない
- **WHEN** 一時の repo の main の checkout と worktree で、`HARNESS_VERIFY=1` を渡したときと渡さないときに `tools/verify-env.sh --plan` を呼ぶ
- **THEN** どれも出力は 1 行で、`reset` か `keep:` で始まる行であり、`HARNESS_RESET:` で始まる行は無い
