# ST48 design — 古い版が新しい DB を書き換えない

## Context

動機は `proposal.md` の Why。いまの形（`premises.md` で repo の実物に当てて確かめた）:

- `migrate()`（`crates/server/src/lib.rs`）は `MIGRATIONS`（`build.rs` が `migrations/ORDER` から組む）の全部を毎回当て、最後に `GRANTS` を当てる。当てた版は DB に残らない
- `run_migrate()` は、接続より前に退避（`migrate_backup.rs`）を取る。失敗は `anyhow` の Err で、終了コード 1
- `run()` の起動の拒否は `exit(2)` と標準エラーの `error: kind=…`（`net_guard::refuse`・`check_db_role`）
- 試験の足場（`testdb.rs`）は `MIGRATIONS` と `GRANTS` を所有者の取引の中で直に当てる（助言ロック `4820251`）。CH04 はこれを回ごとのテンプレートの database に作り替える
- `grants.sql` は `core` の全部の表と、後から作る表への既定の権限で、アプリの役割に読み書きを許す
- 書き出し（`export.rs`）は `core` の表だけを分類と突き合わせる
- `migrations/ORDER` は時刻順でない（`merge=union` で merge の順に積まれる）。自分で `BEGIN` / `COMMIT` を持つ版が 2 つある
- 起動の台本（`stack.sh`・`dev.sh`・`smoke.sh`）は `db-roles.sh` → `migrate` → サーバの順。確認バッチの `run.sh` は `STACK_RESET` を付けずに `stack.sh up` を呼び、DB（55499）は回をまたいで残る
- いまの組み立ての版番号はどれも `0.1.0`

## Goals / Non-Goals

**Goals**: specs の 7 つの Requirement。版の記録の形を、最初に配る版から変えずに済む形で決める。

**Non-Goals**:
- ST48 より前の組み立てが ST48 以後の DB に当たることを止める（その組み立ては配らない）
- 版番号の出所を 1 つにする・リリースのタグを付ける（ST52）。更新の自動の戻し（ST58。終了コード 3 を読む側）
- 前の回の確認バッチの DB を片付ける道具
- `tools/check-immutable.sh` を版の記録に載せる（使い捨ての DB に psql で直に当てる検査で、全部の版を 2 回当てても壊れないことを見続ける。下の D12）

## Decisions

### D1. 版の記録の置き場と形 —— `core` の外の `migration_ledger`、ORDER の外の bootstrap で作る

```sql
CREATE SCHEMA IF NOT EXISTS migration_ledger;
CREATE TABLE IF NOT EXISTS migration_ledger.form (
  only_row boolean PRIMARY KEY DEFAULT true CHECK (only_row),
  form     integer NOT NULL               -- 記録の形の版。いまは 1
);
CREATE TABLE IF NOT EXISTS migration_ledger.applied (
  name           text PRIMARY KEY,         -- 版の名前（ORDER の 1 行）
  sha256         text NOT NULL,            -- 前進側の .sql の中身の sha256（16 進の小文字）
  applied_at     timestamptz NOT NULL DEFAULT now(),
  build_version  text NOT NULL,            -- 当てた組み立ての版番号（CARGO_PKG_VERSION）
  build_commit   text NOT NULL,            -- 当てた組み立ての commit（D8）
  before_ledger  boolean NOT NULL DEFAULT false   -- 版の記録より前から当たっていた（D3）
);
INSERT INTO migration_ledger.form (form) VALUES (1) ON CONFLICT DO NOTHING;
GRANT USAGE ON SCHEMA migration_ledger TO ashiato_app;
GRANT SELECT ON ALL TABLES IN SCHEMA migration_ledger TO ashiato_app;
```

- 中身は `crates/server/src/migration_ledger.sql` に置き、`include_str!` で持つ。**`migrations/ORDER` の版にしない** —— 版の記録を読む前に要り、自分自身を記録できない
- 置き場は `core` の外（C4）。`grants.sql` の `core` の既定の権限が届かないので、アプリの役割への `USAGE` と `SELECT` は bootstrap の末尾に置く（`grants.sql` に置かない —— `grants.sql` は `tools/check-immutable.sh` が記録の schema の無い DB に psql の `ON_ERROR_STOP` で当てる。review/spec.md R11）
- 書き出しの分類（`export.rs` は `core` だけを見る）に掛からず、書き出しにも入らない（D-01 の版の記録は本人の記録ではない）
- **形を凍結する**（C1。形の正本は spec の Requirement「DB に当てた移行の版を、DB の中に記録する」の表。上の DDL はそれを写したもの）: 試験が `information_schema.columns` から 2 つの表の列の名前・型・NULL 可否を読み、上の形と一字一句合うことを見る（`st48_ledger_shape_is_frozen`）。
  後の版が形を変えるときは `form` を 2 に上げる —— 古い版は知らない形として止まる（D5）。列を足すだけでも `form` を上げる（古い版が読み飛ばしてよいかを古い版は知らない）
- 所有は移行を当てる所有者の役割（`ashiato_owner`）。`db-roles.sh` は触らない（D15）

退けた案: `core` に置く —— アプリの役割が既定の権限で書けてしまう（review/deep.md R8）。sqlx の `_sqlx_migrations` —— 名前ではなく連番で持ち、ORDER の union と合わない。

### D2. 比べ方 —— 名前の集合

- 自分の知る版 `K` = `MIGRATIONS` の名前の集合。DB の版 `L` = `migration_ledger.applied.name` の集合
- **先にある（ahead）** = `L − K` が空でない。**遅れ（behind）** = `K − L` が空でない
- 並びの位置・名前の時刻では比べない（ORDER は時刻順でない。`premises.md` 1 行目）。正式版の枝の緊急修正の移行は同じ名前で幹へ入れれば、幹の版は先へ進める（C2）

### D3. 記録の無い DB の 3 分岐

| DB の様子 | 判定 | migrate がすること |
|---|---|---|
| `migration_ledger` が無く、`core` も無い | 空の DB | bootstrap → 全部を当て、`before_ledger = false` で 1 版ずつ記録 |
| `migration_ledger` が無く、`core` がある | ST48 より前の DB（Q2） | bootstrap → 全部を当て（前からある版は当て直せる）、`before_ledger = true` で 1 版ずつ記録 |
| `migration_ledger` がある | 記録のある DB | 形を読む（D5）→ ahead なら止める（D4）→ `K − L` だけを当てる |

- bootstrap は記録を読んだ後、当てる前に流す（何度流しても同じ）。ahead・読めないで止まる回は bootstrap も流さない（DB に何も書かない）
- 1 つ目の分岐と 2 つ目の分岐の間で、`before_ledger` だけが違う。印は「この行の版がいつ当たったかは記録より前で分からない」の意味で、振る舞いは変えない（診断の材料）

### D4. `run_migrate()` の順 —— 検査を退避の前に

1. 接続（所有者の URL。loopback の検査はいまのまま先）
2. 錠を取る（D11）
3. 版の記録を読む（D3・D5）
4. ahead・読めない → 錠を外し、**退避を取らず・何も書かずに**終了コード 3（D6）
5. 退避（`backup_before_migrate`。いまのまま。失敗なら終了コード 1 で当てない）
6. bootstrap → `K − L` を ORDER の順に 1 版ずつ当てて記録（D11）→ `GRANTS`
7. 錠を外す

いまは退避が接続より前にある。止める回に退避を取ると、ST58 は「この回の退避」で戻しうる（Q4 の context）ので、検査を退避より前に動かす。

### D5（仮）. 版の記録が読めないときは止める側に倒す

- 読めない = `migration_ledger` はあるのに `form` の表か行が無い・`form` が 1 でない・`applied` の列が D1 と違う（列の名前と型だけを見る。行の中身は見ない）
- 読む役割に schema の `USAGE` か表の `SELECT` が無いときは、形を見る前に「読む権限が無い（付与の抜け）」として読めないにする。`information_schema.columns` は読めない列を隠すので、付与の抜けが「表が無い」に見え、作り直しの案内へ導く（review/code.md R30）
- migrate も起動も、終了コード 3 で `kind=ledger_unreadable`
- 反転条件: 後の版が `form` を上げずに列を足す必要が出たら（D1 の凍結の試験が先に落ちて分かる）、「知らない列は読み飛ばす」に替える前に人間へ聞く（配った版の読み方が変わる）

### D6. 終了コードと出力の形 —— 3 は「版の食い違いで止めた。DB に何も書いていない」

| 終了コード | 意味 | 誰が |
|---|---|---|
| 0 | 当てた / 起動した | — |
| 1 | 移行の失敗（版の中身が当たらない・退避の失敗・接続できない） | migrate（いまのまま） |
| 2 | 起動の拒否（役割・待ち受け・合言葉。いまのまま） | サーバ |
| **3** | **版の食い違いで止めた。DB に何も書いていない**（`db_ahead` / `db_behind` / `ledger_unreadable`） | migrate とサーバ |

標準エラー（ST58 が読む。配った版に残るので、行の頭の形を変えない）:

```
error: kind=db_ahead
  db: 202611010900_new_thing (0.2.0 1a2b3c4d…)
  db: 202611050000_other (0.2.0 1a2b3c4d…)
  self: 0.1.0 9f8e7d6c…
  この DB には、この組み立ての知らない移行が当たっている。この組み立てでは当てない・起動しない
```

21 件以上なら、20 件の `db:` の後に件数の行が 1 つ入る（例は 23 件）:

```
error: kind=db_ahead
  db: …（20 行）
  more: 3
  self: 0.1.0 9f8e7d6c…
  この DB には、この組み立ての知らない移行が当たっている。この組み立てでは当てない・起動しない
```

- この形（行の頭と 20 件の上限）は spec の Requirement「知らない版が当たった DB には…」が持つ
- `kind=` の行は既存の `net_guard::refuse` と同じ形の 1 行目。続く行は 2 字下げで `db:` / `more:` / `self:` / 説明
- 版の名前は 20 件まで並べ、超えたら件数だけを `  more: <N>`（N は並べなかった版の数。数字だけ）の 1 行で足し、その後に `self:` の行が来る。`  db: ` の頭は 1 版 1 行（`<名前> (<版番号> <commit>)`）にだけ使う（ST58 が機械で読むので、件数の行に同じ頭を使わない）
- `db_behind` は `self:` と「まだ当たっていない版が N 個。先に `ashiato-server migrate` を」を出す（名前は並べない。新しい版のサーバで migrate を忘れただけで、名前は ORDER にある）
- `tracing::error!(kind = …)` にも同じ種別を出す（値として版の名前を載せてよい。私的データではない）
- サーバの 3 は `db_role` の検査の後・待ち受けを開く前（Requirement の「待ち受けを開く前」）

退けた案: 起動の版の食い違いを既存の 2 に入れる —— ST58 が「DB に触っていないので戻さなくてよい」を 1 つの数で読めなくなる（Q4 の推奨は区別する）。

### D7（仮）. 指紋が違う版は当て直さず、警告だけ

- 記録にある版の `sha256` が自分の中身と違う → 当て直さない・止めない・`tracing::warn!` で版の名前を出す（起動も同じく警告だけ）
- 理由: 配った版の中身は D9 の検査が凍結するので、違いが出るのは印より前の版を直した開発・確認の DB だけ（Q1 の推奨は印より前を直してよいとする）。止めると、ST23・ST41 の書き換えが入った後の worktree の DB が全部止まる
- 反転条件: 配った印のある版で違いが見つかった（凍結の検査の抜け）、または違う中身のまま進んで門の定義が食い違った DB が見つかったら、止める側（`kind=ledger_mismatch`・終了コード 3）に替える

### D8（仮）. 組み立ての印 —— 版番号と commit

- 版番号は `env!("CARGO_PKG_VERSION")`。commit は `build.rs` が決めて `cargo:rustc-env=ASHIATO_BUILD_COMMIT=…` で渡す: 環境変数 `ASHIATO_BUILD_COMMIT` があればそれ、無ければ `git rev-parse HEAD`（失敗したら `unknown`）
- `build.rs` は `cargo:rerun-if-env-changed=ASHIATO_BUILD_COMMIT` を出す。git の HEAD の変化では作り直さない（差分のビルドでは commit が古いまま残りうる。診断の材料なので許す）
- 試験は、組み立てに渡した値を別の名前（`ASHIATO_EXPECT_BUILD_COMMIT`）で受けて行の値と比べる（cargo は build script の `rustc-env` を試験の実行時の環境にも入れるので、`ASHIATO_BUILD_COMMIT` を実行時に読むと組み立て済みの値が見える）。CI は同じ乱数を両方に渡して 1 本だけ走らせる（review/code.md R20）
- 反転条件: ST52 が版番号の出所を 1 つにしたら、そちらから読む（commit は残す）。commit が古いままで食い違いを読み違えた事例が出たら、`.git/HEAD` と参照先を `rerun-if-changed` に足す

### D9（仮）. 配った印 = タグ `v*`、凍結の検査は `tools/check-released-migrations.sh`

- 印の型: `git tag --list 'v[0-9]*'`。ST52 がタグを付け始める。別の型にするなら、この台本の型を一緒に変える（`docs/handoff/ST52.md`）
- 検査: 印ごとに `git show <tag>:migrations/ORDER` の版の名前を取り、今の木の `migrations/ORDER` に全部あること、`git diff --quiet <tag> HEAD -- migrations/<名前>.sql` が 0 であることを見る。`.down.sql` は見ない
- 印が 0 個なら「配った印が 0 個」と出して rc=0。印の commit や `ORDER` を取り出せなければ rc≠0（印があるのに読めないのは CI の取り寄せの不足）
- CI の取り寄せは `git ls-remote` の失敗を「印が 0 個」と区別して落とす（読めないまま検査を緑にしない。review/code.md R18）
- CI: rust の job で `git fetch --force --no-tags --depth=1 origin '+refs/tags/v*:refs/tags/v*'` の後に走らせる（checkout は浅いのでタグが無い）。`--self-test`（使い捨ての git の木でタグを付け、中身の変更・消去・名前の変更が落ち、印の後の版と `.down.sql` の変更が通ることを見る）も同じ job で走らせる
- 反転条件: 正式版の枝の緊急修正（FR-105）で、印の版の移行を直す必要が出たら、直さずに新しい版を足す。それができない事例が出たら人間へ聞く

### D10（仮）. これから足す版の規則 —— 名前の時刻 `202610250000` 以後

- `tools/check-migrations.sh` に足す: 名前の時刻（頭の 12 桁）が `202610250000` 以上の版は、(a) 前進側に `BEGIN` / `START TRANSACTION` / `COMMIT` / `ROLLBACK` / `END` を文として持たない（`$$` の中の関数本体の `BEGIN … END` は数えない。行頭の語と `;` で判定し、`DO $$` / `CREATE FUNCTION` の本体の中は除く）、
  (b) `.down.sql` に `DELETE FROM migration_ledger.applied WHERE name = '<版の名前>'` を持つ
- 境を名前の時刻にするのは、走っている ST23・ST41・ST29 の版（どれも 2026-10-25 より前の時刻）に、後から規則を課さないため（差し戻さない）
- `--self-test` を足す（使い捨てのフォルダに規則を破る版と守る版を置いて、落ちる・通るを見る）
- `tools/check-immutable.sh` は psql で当てる DB にも版の記録の置き場（bootstrap）だけを作る（行は書かない）。(b) の `DELETE` が戻しの検査で落ちないため（review/code.md R23）
- 反転条件: 境より前の時刻を名前に付けた新しい版が入った（時刻を手で書き違えた）と分かったら、境を「この版より後に ORDER に足した版」に替える

### D11. 1 版 1 取引と錠

- 当てる処理は 1 本の接続で行う。始めに `pg_advisory_lock(4820251)`（試験の足場がいま取る鍵と同じ数。セッションの錠）を取り、終わり（成功も失敗も）に外す。試験の足場はいまの外側の取引と `pg_advisory_xact_lock` を外して呼ぶ（残すと、別の接続の `apply_migrations` が自分を待ち続ける）
- 1 版ごとに取引を開き、版の中身 → `INSERT INTO migration_ledger.applied …` → commit。版が落ちたらその取引ごと戻り、その版も後の版も記録されない（C7）
- 自分で `BEGIN` / `COMMIT` を持つ前からの 2 版は、中の `COMMIT` で取引が先に閉じ、記録の行はその後に単独で入る。その間に落ちると、次の回にその版をもう一度当てる（前からの版は当て直せる。`tools/check-immutable.sh` が 2 回当てて見ている）。これから足す版は D10 で自分の取引を持たない
- `GRANTS` は全部の版の後に毎回、**錠の中で**当てる（同時の `GRANT … ON ALL TABLES` / `ALTER DEFAULT PRIVILEGES` は `tuple concurrently updated` で落ちる）。ahead・読めないなら流さない（D4）
- 錠を外せなかったら、その接続を pool に返さずに `detach` して閉じる（セッションの錠は接続が閉じるまで残る）

### D12. 当てる経路を 1 つの関数に寄せる

- `pub async fn apply_migrations(conn: PoolConnection<Postgres>, versions: &[(&str, &str)]) -> Result<(), MigrateError>` と、退避（`before_apply`）と付与の SQL（`grants`）を足した `apply_migrations_with` を `migration_ledger.rs` に置く（接続を受け取るのは、錠を外せないとき `detach` して閉じるため）。`migrate()`・試験の足場（`testdb.rs`。CH04 のテンプレートも）・試験用の並び（Scenario「途中で落ちたら」）がこれを呼ぶ
- 版の記録を読む `read_ledger(conn) -> Result<LedgerState, ReadError>` と、判定 `compare(known, ledger) -> Verdict`（純粋な関数。`Ahead` / `Behind` / `Even`）を分け、起動の検査も同じものを使う
- `tools/check-immutable.sh` は psql で直に当てるまま（使い捨ての DB を `down -v` で捨てる。版の記録は持たない）。「`run()` は起動のたびに全版を当てる」の注記だけを「前からある版は当て直せることを保つ（記録の無い古い DB に一度全部を当てるので）」に直す

### D13（仮）. 確認バッチは回ごとの compose の project に立つ

- `verify-prep.sh` が書く `run.sh` に `export COMPOSE_PROJECT_NAME="ashiato-verify-<tag を英小文字・数字・- に寄せたもの>"` を足す。compose の DB の容器と置き場（無名の volume）は project ごとに分かれる。`docker compose exec db psql -d ashiato` の書き方（`seed.sh` など）はそのまま新しい DB を指す
- 待ち受けの port（55499）は回をまたいで同じなので、`stack.sh` は DB を起こす前に、その port を公開している容器が別の project のものなら `docker stop` する（`down` しない。置き場が残る）。止めたことを 1 行出す
- 前の回の DB を見るには `COMPOSE_PROJECT_NAME=<前の名前> docker compose up -d db`
- 端末の測定は回ごとの DB に分かれて入るので、`verify-prep.sh` の `c01-clock` の数（ST05 D4 の反転条件）は、前の回の DB（`ashiato-verify-*`）と既定の project の DB を 1 つずつ起こして数えて足し、機械が判定する（止まっていた DB は数えた後に止め直す。review/code.md R27）
- 退けた案: 1 つの PostgreSQL の中で database を分ける —— `seed.sh`・`seed.d/`・`archive-shape.sh` などの `-d ashiato` を全部書き換えることになる
- 反転条件: 残る project の置き場が数十 GB を超える・本人が片付けの手間を違和感として返したら、片付けの道具を足す（別の change）

### D14. 止まったときの手順 —— `stack.sh` と `dev.sh`

- `migrate` の終了コードが 3 なら、作り直さずに次を出して止まる（終了コード 3 のまま）:
  `この DB には、この木の知らない移行が当たっている（上の db: の行）。使い捨ての DB なら STACK_RESET=1 で作り直す。残す DB なら COMPOSE_PROJECT_NAME=<別の名前> で別の DB に立てる（この DB は消えない）`
- 止める仕組みを外す環境変数は作らない（Q5。`MIGRATE_BACKUP=off` でも止まる）

### D15. `db-roles.sh` は版に依らない操作だけにする

- 版の検査より前に管理者で走る（C6）。いまの中身（役割の作成と合言葉・付与の剥がし・`core` の所有の移し）は版に依らないので変えない。冒頭の注記に「版に依る操作（表・列・関数を前提にするもの）を足さない。足すなら移行の版にする」と書く

### D16. 拒否の試験の DB

- 版の記録に知らない行を書く試験は、worktree で共有する DB に書かない（その worktree の stack と試験が全部止まる）。使い捨ての database（`CREATE DATABASE st48_<乱数 12 桁>`、所有者の役割は `CREATEDB` を持つ）を作り、試験の終わりに `DROP DATABASE … WITH (FORCE)` で消す
- 単体（`crates/server/src/migration_ledger_tests.rs`）は `testdb` に足す使い捨ての口を使う（CH04 が入っていれば CH04 の `*_tmp_*` の口に載せ替える）。実行ファイルの試験（`tests/server_startup.rs`）は自分で作って消す。CH04 の `--lint` に掛かる行には `db-ok: <理由>` を付ける
- 実行ファイルの試験は `MIGRATE_BACKUP=off` か、退避の置き場を `"$TMPDIR"` の下の空のフォルダにし、`PG_DUMP` を偽の台本（呼ばれたらファイルを書く）にして「退避を取らない」を数える

### D17（仮）. サーバの起動の検査はアプリの役割で読む

- `run()` は `check_db_role` の後に `read_ledger` → `compare` を通す。読むのはアプリの役割（`grants.sql` の SELECT）
- 指紋の違いは警告だけ（D7）。`db_behind` は「記録が無い」も含む
- 反転条件: 起動の検査が遅い（版が数百を超えて目に見える）と分かったら、名前の集合でなく件数と最後の名前で先に絞る

## Risks / Trade-offs

- [ST48 より前の組み立てが ST48 以後の DB に migrate を当てる] → 止められない（その組み立ては版の記録を知らない）。配る版は全部 ST48 を含む（Non-Goals）。開発の worktree で古い枝の組み立てを走らせると起きうる —— 前からある版は当て直せるので、門の定義は古い枝のものに戻る。次に新しい組み立てで migrate を当てても、記録上は当たっているので当て直さない（**直らない**）。開発の DB は使い捨てなので `STACK_RESET=1` で作り直す
- [記録より前の DB を「ST48 より前」と読み違える（R2）] → 後の版が `migration_ledger` を消す・移すと、古い版はその DB を「記録の無い DB」と読んで全部を当てる。D1 で置き場と形を凍結し、形を変えるときは `form` を上げる（消さない）
- [指紋の違いを止めない（D7）] → 印より前の版を直した worktree の DB は、直した中身が当たらないまま進む。開発の DB だけで起き、`STACK_RESET=1` で直る
- [DB の置き場は無名の volume（`docker-compose.yml` に `volumes:` が無い）] → 容器を消す（`docker compose down`。`-v` 無しでも）と置き場が孤立し、次の `up` は空の置き場を付ける。`stack.sh down` を `docker compose stop` に替え、別の project の DB も `docker stop` だけにする（D13）。named volume に替えない —— main の checkout の DB（電話の記録が入っている）が次の `up` で空の置き場に付け替わる
- [確認バッチの前の回の DB が積もる（D13）] → 片付けは後の change。前の回の DB の置き場は `docker volume ls` で見える
- [`run.sh` の DB が回ごとに新しくなる] → 確認の間に電話が送った記録は、その回の DB にだけ入る（次の回の DB には無い）。前の回の DB は残るので失われない
- [CH04 の `testdb.rs` の作り替えと衝突する] → 当てる処理を `apply_migrations` に寄せ、`testdb.rs` の差は呼び出しの数行にする。後から入る側が rebase で解く

## Migration Plan

- 移行の版は足さない。ST48 の入った組み立てで最初に migrate を当てた回に、`migration_ledger` が作られ、ST48 より前の DB は D3 の 2 つ目の分岐を通る
- 戻すとき: ST48 より前の組み立てに戻すと、版の記録は読まれずに残る（害は無い）。`DROP SCHEMA migration_ledger CASCADE` で消せる —— 消すと、次に ST48 以後の組み立てで当てる回が「記録の無い DB」の分岐を通る
