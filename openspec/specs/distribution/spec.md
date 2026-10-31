# distribution Specification

## Purpose
配布と更新。リリースと 3 つのチャンネルへの配布、インストール、更新の確認と適用と戻し、DB の版の守り、更新の履歴、許諾の同梱を持つ。
版が前後しても本人の DB と記録が失われず、古い版が新しい DB を書き換えないことを守る。

## Requirements

### Requirement: DB に当てた移行の版を、DB の中に記録する

THE SYSTEM SHALL D-01 に当てた移行の版を、版ごとに 1 行ずつ D-01 の中に記録する。1 行は、版の名前・中身の指紋（sha256）・当てた時刻・当てた組み立ての版番号と commit・
「版の記録より前から当たっていた」の印を持つ。
THE SYSTEM SHALL 版の記録を記録の schema（`core`）の外に置き、S-01 の実行時の役割には読むことだけを許す。
THE SYSTEM SHALL 移行を当てる経路（`ashiato-server migrate` と試験の足場）のどれで当てても、同じ記録を書く。
WHEN 移行が途中で落ちる THE SYSTEM SHALL 当て終えた版だけを記録に残し、落ちた版とその後の版を記録しない。
WHEN 版の記録の無い D-01（記録の schema `core` はある。版の記録を持たない組み立てで当ててきた）に移行を当てる THE SYSTEM SHALL 自分の知る全部の版を当て、
全部を「版の記録より前から当たっていた」の印を付けて記録する。
THE SYSTEM SHALL 版の記録を次の置き場と列の形で持ち、版をまたいで変えない。形を変えるときは記録の中の「形の版」を上げ、置き場を消さない・移さない。

| 置き場 | 列（名前・型・NULL の可否） |
|---|---|
| schema `migration_ledger`、表 `form`（1 行だけ） | `only_row` boolean NOT NULL（主キー）/ `form` integer NOT NULL（形の版。いまは 1） |
| schema `migration_ledger`、表 `applied`（版ごとに 1 行） | `name` text NOT NULL（主キー）/ `sha256` text NOT NULL / `applied_at` timestamp with time zone NOT NULL / `build_version` text NOT NULL / `build_commit` text NOT NULL / `before_ledger` boolean NOT NULL |

導出元: FR-118

#### Scenario: 版の記録の置き場と列の形は、決めた形のまま変わらない
- **WHEN** 空の DB に版の記録の置き場を作る
- **THEN** 版の記録の schema・表の名前と、各列の名前・型・NULL の可否はこの Requirement の表と一字一句同じで、形の版は 1

#### Scenario: 空の DB に移行を当てると、当てた版がすべて記録される
- **GIVEN** 何も当たっていない空の DB
- **WHEN** `ashiato-server migrate` を走らせる
- **THEN** 版の記録の行の名前の集合は `migrations/ORDER` の版の名前の集合と一致する
- **AND** どの行も「版の記録より前から当たっていた」の印を持たない

#### Scenario: 版の記録の 1 行は、名前・指紋・時刻・組み立ての版番号と commit を持つ
- **WHEN** 空の DB に `ashiato-server migrate` を走らせる
- **THEN** どの行も、指紋がその版のファイルの中身の sha256 の 16 進と一致し、当てた時刻が空でなく、組み立ての版番号が `crates/server` の版番号と一致し、commit が組み立てに渡した `ASHIATO_BUILD_COMMIT` の値と一致する

#### Scenario: 版の記録の無い古い DB には一度だけ全部を当て、印を付けて記録する
- **GIVEN** 版の記録を持たない組み立てで全部の版を当て、本人の記録（`core.event` の行）を 1 件入れた DB（版の記録の schema が無い）
- **WHEN** `ashiato-server migrate` を走らせる
- **THEN** 終了コードは 0 で、本人の記録の 1 件はそのまま残る
- **AND** 版の記録の行の名前の集合は `migrations/ORDER` と一致し、どの行も「版の記録より前から当たっていた」の印を持つ

#### Scenario: 移行が途中で落ちたら、当て終えた版だけが記録される
- **GIVEN** 3 つの版を並べ、2 つ目の中身が必ず落ちる試験用の並び
- **WHEN** その並びを空の DB に当てる
- **THEN** 当てる処理は失敗を返し、版の記録には 1 つ目の版だけがある

#### Scenario: S-01 の実行時の役割は版の記録を読めるが、書き換えられない
- **GIVEN** 移行を当てた DB
- **WHEN** S-01 の実行時の役割で、版の記録を読み、行を足し、行を書き換え、行を消す
- **THEN** 読むことだけが通り、足す・書き換える・消すはどれも権限が無いとして断られる

#### Scenario: 試験の足場が当てた DB にも、同じ版の記録がある
- **WHEN** 試験の足場の DB に繋ぐ
- **THEN** 版の記録の行の名前の集合は `migrations/ORDER` と一致する

#### Scenario: 版の記録があっても、全記録の書き出しは始まる
- **GIVEN** 移行を当てた DB（版の記録の表がある）
- **WHEN** 全記録の書き出しを始める
- **THEN** 書き出しは「分類の無い表がある」で断られずに始まり、書き出したファイルの一覧に版の記録は無い

### Requirement: migrate はまだ当たっていない版だけを当てる

THE SYSTEM SHALL `ashiato-server migrate` で、自分の知る版のうち版の記録に無いものだけを `migrations/ORDER` の順に当て、当てた版を記録する。
版の記録にある版は当て直さない。並びの位置ではなく、版の名前の集合で比べる。
WHEN 版の記録にある版の指紋が、自分の持つその版の中身の指紋と違う THE SYSTEM SHALL その版を当て直さず、版の名前を警告として示して続ける。
THE SYSTEM SHALL 当てる版が 1 つも無い回にも、当てる前の退避を今までどおり取り、アプリの役割への付与を当てる。
WHILE 1 つの `ashiato-server migrate` が当てている THE SYSTEM SHALL 同じ DB への別の migrate と試験の足場の当て込みを、その終わりまで待たせる。
導出元: FR-118

#### Scenario: 同じ版で当て直しても、何も変わらずに通る
- **GIVEN** `ashiato-server migrate` を一度走らせた DB
- **WHEN** 同じ組み立てで `ashiato-server migrate` をもう一度走らせる
- **THEN** 終了コードは 0 で、版の記録の行の数・名前・当てた時刻は 1 回目と同じ
- **AND** 門の関数 `core.reject_collected_rewrite` の定義は 1 回目と同じ

#### Scenario: 記録にある版は、中身をもう一度走らせない
- **GIVEN** 中身が試験用の表に 1 行を足す版を含む試験用の並びを、一度当てた DB
- **WHEN** 同じ並びでもう一度当てる処理を走らせる
- **THEN** 試験用の表の行は 1 行のまま

#### Scenario: 並びの前にある未適用の版も当たる
- **GIVEN** 試験用の 3 つの版の並びのうち、1 つ目と 3 つ目だけが当たって記録された DB
- **WHEN** 3 つの版の並びを渡して当てる処理を走らせる
- **THEN** 2 つ目の版が当たって記録され、3 つ目の版の記録の当てた時刻は変わらない

#### Scenario: 指紋の違う版は当て直さず、名前を示して続ける
- **GIVEN** 版の記録の 1 行の指紋を別の値に書き換えた DB
- **WHEN** `ashiato-server migrate` を走らせる
- **THEN** 終了コードは 0 で、その版の名前を含む警告が出力され、その行の当てた時刻は変わらない

#### Scenario: 当てる版が無い回にも、当てる前の退避を取る
- **GIVEN** 全部の版が当たった DB と、退避の置き場
- **WHEN** 退避を有効にして `ashiato-server migrate` を走らせる
- **THEN** 退避の置き場に `pre-migrate-` で始まる空でないファイルが 1 つ増える

#### Scenario: 当てる版が無い回にも、アプリの役割への付与を当てる
- **GIVEN** 全部の版が当たった DB で、アプリの役割から `core.event` の SELECT の権限を外したもの
- **WHEN** `ashiato-server migrate` を走らせる
- **THEN** 終了コードは 0 で、アプリの役割は `core.event` を読める

#### Scenario: 2 つの migrate が同時に走っても、版は 1 回ずつ当たる
- **GIVEN** 空の DB
- **WHEN** 当てる処理を 2 つ同時に走らせる
- **THEN** どちらも成功し、版の記録の行は版ごとに 1 行だけある

### Requirement: 知らない版が当たった DB には、migrate が 1 つも当てずに止まる

IF `ashiato-server migrate` が、自分の知らない版の記録を持つ D-01 に当たる THEN THE SYSTEM SHALL 退避を取らず、移行も付与も 1 つも当てず、D-01 に何も書かずに、終了コード 3 で終わる。
THE SYSTEM SHALL そのとき、標準エラーの 1 行目に `error: kind=db_ahead` を、続く行に D-01 にあって自分の知らない版を 1 つにつき `  db: <版の名前> (<版番号> <commit>)` の形で 20 件まで（超えた分は件数だけを 1 行）、
最後に `  self: <自分の版番号> <自分の commit>` を示す。この行の頭の形（`error: kind=`・`  db: `・`  self: `）は、版をまたいで変えない。
IF 版の記録が読めない（知らない形の版の記録・列が欠けた記録）THEN THE SYSTEM SHALL 同じく何も書かずに終了コード 3 で終わり、`error: kind=ledger_unreadable` を示す。
THE SYSTEM SHALL 移行の失敗（版の中身が当たらない・退避に失敗した）を終了コード 1 で終え、終了コード 3 と区別する。退避に失敗したときは版を 1 つも当てない。
THE SYSTEM SHALL 版の食い違いで止める振る舞いを外す設定を持たない（退避の設定 `MIGRATE_BACKUP`・`MIGRATE_BACKUP_DIR`・`PG_DUMP` のどの値でも止まる）。
導出元: FR-129

#### Scenario: 新しい版で当てた DB に古い版の migrate を走らせると、1 つも当てずに止まる
- **GIVEN** 全部の版が当たり、版の記録に自分の知らない版の行が 1 つある DB（新しい版で当てた DB を、自分の知らない版の行で作る）
- **WHEN** `ashiato-server migrate` を走らせる
- **THEN** 終了コードは 3 で、標準エラーの 1 行目は `error: kind=db_ahead`、続く行に `  db: <その版の名前> (<その行の版番号> <その行の commit>)` と `  self: <自分の版番号> <自分の commit>` がある
- **AND** 版の記録の行と門の関数の定義は走らせる前と同じ

#### Scenario: 止まるときは退避を取らない
- **GIVEN** 自分の知らない版の記録を持つ DB と、空の退避の置き場
- **WHEN** 退避を有効にして `ashiato-server migrate` を走らせる
- **THEN** 終了コードは 3 で、退避の置き場は空のまま

#### Scenario: 版の記録が読めない DB では、何も書かずに止まる
- **GIVEN** 版の記録の形の版を自分の知らない値に書き換えた DB
- **WHEN** `ashiato-server migrate` を走らせる
- **THEN** 終了コードは 3 で、標準エラーに `error: kind=ledger_unreadable` があり、版の記録の行は走らせる前と同じ

#### Scenario: 退避に失敗すると、終了コード 1 で何も当てない
- **GIVEN** 版の記録から 1 行を消した DB と、退避の道具が失敗する設定
- **WHEN** 退避を有効にして `ashiato-server migrate` を走らせる
- **THEN** 終了コードは 1 で、標準エラーに `kind=db_ahead` も `kind=ledger_unreadable` も無い
- **AND** 消した 1 行は記録に戻っていない（その版は当たっていない）

#### Scenario: 版の中身が当たらないと、終了コード 1 で終わる
- **GIVEN** 版の記録から 1 行を消し、その版の中身が当たらないように DB の中の物を書き換えた DB
- **WHEN** `ashiato-server migrate` を走らせる
- **THEN** 終了コードは 1 で、標準エラーに `kind=db_ahead` も `kind=ledger_unreadable` も無く、その版の記録の行は無い

#### Scenario: 退避の設定を変えても、知らない版の当たった DB には当たらない
- **GIVEN** 自分の知らない版の記録を持つ DB
- **WHEN** `MIGRATE_BACKUP=off` を付けて、また別の回に `MIGRATE_BACKUP_DIR` と `PG_DUMP` を変えて `ashiato-server migrate` を走らせる
- **THEN** どの回も終了コードは 3 で、版の記録の行は走らせる前と同じ

### Requirement: サーバは DB と版が食い違えば起動しない

IF S-01 が、自分の知らない版の記録を持つ D-01 に繋がる THEN THE SYSTEM SHALL 起動せず、終了コード 3 で終わり、migrate と同じ形（`error: kind=db_ahead`・`  db: …`・`  self: …`）で示す。
IF S-01 が、自分の知る版のうち記録に無いものがある D-01（版の記録が無い D-01 を含む）に繋がる THEN THE SYSTEM SHALL 起動せず、終了コード 3 で終わり、標準エラーに `error: kind=db_behind` と、まだ当たっていない版の数と「先に `ashiato-server migrate` を」の案内を示す。
IF 版の記録が読めない THEN THE SYSTEM SHALL 起動せず、終了コード 3 で終わり、`error: kind=ledger_unreadable` を示す。
WHEN 記録にある版の指紋が自分の持つ中身の指紋と違う THE SYSTEM SHALL 起動し、その版の名前を警告として示す。
THE SYSTEM SHALL 版の検査を、待ち受けを開く前に行う。
導出元: FR-117

#### Scenario: 新しい版で当てた DB に古い版のサーバを起動すると、起動しない
- **GIVEN** 全部の版が当たり、版の記録に自分の知らない版の行が 1 つある DB
- **WHEN** S-01 の実行時の役割でサーバを起動する
- **THEN** 待ち受けを開かずに終了コード 3 で終わり、標準エラーの 1 行目は `error: kind=db_ahead`、続く行に `  db: <その版の名前> (<その行の版番号> <その行の commit>)` と `  self: <自分の版番号> <自分の commit>` がある

#### Scenario: まだ当たっていない版のある DB では、サーバは起動しない
- **GIVEN** 全部の版を当てた後、版の記録から 2 行を消した DB
- **WHEN** サーバを起動する
- **THEN** 待ち受けを開かずに終了コード 3 で終わり、標準エラーに `error: kind=db_behind` と、まだ当たっていない版の数 2 と、`ashiato-server migrate` の案内がある

#### Scenario: 版の記録の無い DB では、サーバは起動しない
- **GIVEN** 版の記録を持たない組み立てで全部の版を当てた DB（版の記録の schema が無い）
- **WHEN** サーバを起動する
- **THEN** 終了コードは 3 で、標準エラーに `error: kind=db_behind` がある

#### Scenario: 版が揃っていれば、サーバは起動する
- **GIVEN** 同じ組み立てで `ashiato-server migrate` を走らせた DB
- **WHEN** サーバを起動する
- **THEN** `/healthz` が 200 を返す

#### Scenario: 指紋の違う版があっても、サーバは警告を出して起動する
- **GIVEN** 同じ組み立てで migrate を走らせた後、版の記録の 1 行の指紋を別の値に書き換えた DB
- **WHEN** サーバを起動する
- **THEN** `/healthz` が 200 を返し、出力にその版の名前を含む警告がある

#### Scenario: 版の記録が読めない DB では、サーバは起動しない
- **GIVEN** 版の記録の形の版を自分の知らない値に書き換えた DB
- **WHEN** サーバを起動する
- **THEN** 終了コードは 3 で、標準エラーに `error: kind=ledger_unreadable` がある

### Requirement: 配った移行は、名前も中身も変えられない

THE SYSTEM SHALL 配った印（リリースのタグ）の付いた版にある移行について、その名前を `migrations/ORDER` から消すこと・名前を変えること・中身（前進側の `.sql`）を変えることを、CI の検査で落とす。
THE SYSTEM SHALL 配った印の後に足した移行と、戻し手順（`.down.sql`）の変更は落とさない。
WHILE 配った印が 1 つも無い THE SYSTEM SHALL 何も凍結せず、検査を通す。
IF 配った印の版を読めない（印はあるが、その版の中身を取り出せない）THEN THE SYSTEM SHALL 検査を通さない。
導出元: FR-129

#### Scenario: 配った版にある移行の中身を変えると、検査が落ちる
- **GIVEN** 印 `v0.1.0` の付いた版と、その版にある移行の中身を 1 行変えた今の木
- **WHEN** 配った移行の検査を走らせる
- **THEN** 終了コードは 0 でなく、変えた移行の名前と印の名前が示される

#### Scenario: 配った版にある移行を消すと、検査が落ちる
- **GIVEN** 印 `v0.1.0` の付いた版と、その版にある移行の 1 つをファイルごと `migrations/ORDER` から消した今の木
- **WHEN** 配った移行の検査を走らせる
- **THEN** 終了コードは 0 でなく、消えた名前が示される

#### Scenario: 配った版にある移行の名前を変えると、検査が落ちる
- **GIVEN** 印 `v0.1.0` の付いた版と、その版にある移行の 1 つの名前を（ファイルと `migrations/ORDER` の行の両方で）変えた今の木
- **WHEN** 配った移行の検査を走らせる
- **THEN** 終了コードは 0 でなく、元の名前が示される

#### Scenario: 配った印が無ければ、検査は通る
- **GIVEN** 配った印の型のタグが 1 つも無い木
- **WHEN** 配った移行の検査を走らせる
- **THEN** 終了コードは 0 で、印が 0 個だったことが示される

#### Scenario: 配った印の後に足した移行は、変えても通る
- **GIVEN** 印 `v0.1.0` の後に足した移行の中身だけを変えた今の木
- **WHEN** 配った移行の検査を走らせる
- **THEN** 終了コードは 0

#### Scenario: 配った版にある移行の戻し手順は、変えても通る
- **GIVEN** 印 `v0.1.0` の版にある移行の `.down.sql` だけを変えた今の木
- **WHEN** 配った移行の検査を走らせる
- **THEN** 終了コードは 0

#### Scenario: 配った印の版を読めなければ、検査は通らない
- **GIVEN** 印の名前はあるが、その commit を取り出せない木
- **WHEN** 配った移行の検査を走らせる
- **THEN** 終了コードは 0 でない

### Requirement: これから足す移行は、版の記録と一緒に当たり、一緒に戻る

THE SYSTEM SHALL 名前の時刻が `202610250000` 以後の移行について、前進側の `.sql` が自分で取引を始める・終える文（`BEGIN` / `START TRANSACTION` / `COMMIT` / `ROLLBACK` / `END`）を持つことを、CI の検査で落とす。
THE SYSTEM SHALL 同じ移行の `.down.sql` が、その版の名前の版の記録の行を消す文を持たないことを、CI の検査で落とす。
THE SYSTEM SHALL 名前の時刻が `202610250000` より前の移行には、この 2 つを求めない。
導出元: FR-118

#### Scenario: 新しい版が自分で取引を持つと、検査が落ちる
- **GIVEN** 名前の時刻が `202610250000` 以後で、前進側に `COMMIT;` を持つ移行
- **WHEN** 移行の検査を走らせる
- **THEN** 終了コードは 0 でなく、その移行の名前が示される

#### Scenario: 新しい版の戻し手順が版の記録の行を消さないと、検査が落ちる
- **GIVEN** 名前の時刻が `202610250000` 以後で、`.down.sql` に版の記録の行を消す文が無い移行
- **WHEN** 移行の検査を走らせる
- **THEN** 終了コードは 0 でなく、その移行の名前が示される

#### Scenario: 前からある版には、この規則を求めない
- **WHEN** 今の `migrations/` に移行の検査を走らせる
- **THEN** 終了コードは 0（`202609082001_immutable_collected` と `202609112113_source_lifecycle` は自分の `BEGIN` / `COMMIT` を持つまま通る）

### Requirement: 開発と確認の DB が版の食い違いで止まっても、DB を捨てずに立て直せる

THE SYSTEM SHALL 確認バッチの起動（`dist/verify-<tag>/run.sh`）を、回ごとに別の DB（docker compose の project。DB の置き場の volume も project ごとに分かれる）に立て、前の回の DB を消さずに残す。
WHEN `tools/stack.sh` が、同じ待ち受けの port を別の project の DB が持っているのに当たる THE SYSTEM SHALL その DB を止める（消さない）だけにし、置き場を残す。
THE SYSTEM SHALL `tools/stack.sh down` を、DB の容器を止めるだけにする（容器を消さない。DB の置き場は容器に付いた無名の volume なので、容器を消すと置き場が孤立する）。
WHEN `tools/stack.sh` の移行が版の食い違い（終了コード 3）で止まる THE SYSTEM SHALL DB を作り直さずに止まり、使い捨ての DB なら作り直す手順（`STACK_RESET=1`）と、
残す DB なら別の project で立てる手順（`COMPOSE_PROJECT_NAME=<名前>`）を示す。
THE SYSTEM SHALL `STACK_RESET=1` で作り直した DB（縦串・e2e・確認の試験）には、今までどおり退避を取らずに全部の版を当てて立てる。
導出元: FR-129

#### Scenario: 確認バッチの起動は、回ごとの project に立つ
- **WHEN** 確認バッチの準備が `dist/verify-<tag>/run.sh` を書く
- **THEN** その `run.sh` は、回の名前を含む project の名前を `COMPOSE_PROJECT_NAME` として `tools/stack.sh` に渡し、回の名前が違えば project の名前も違う

#### Scenario: 別の project で立てると、前の DB は残る
- **GIVEN** project A で立てて記録を 1 件入れ、`tools/stack.sh down` で止めた DB
- **WHEN** `COMPOSE_PROJECT_NAME` に別の名前 B を渡して `tools/stack.sh up --check-only` を走らせる
- **THEN** B の DB に全部の版が当たってサーバが `/healthz` に答える
- **AND** project A の DB をもう一度起こすと、記録の 1 件はそのまま残っている

#### Scenario: 版の食い違いで止まると、作り直さずに手順を示す
- **GIVEN** 記録を 1 件入れ、版の記録に自分の知らない版の行を 1 つ足した DB
- **WHEN** その DB の project で `tools/stack.sh up --check-only` を走らせる
- **THEN** 終了コードは 0 でなく、出力に `STACK_RESET=1` と `COMPOSE_PROJECT_NAME` の手順があり、その DB の記録の 1 件と版の記録の行はそのまま残る

#### Scenario: 作り直した DB には、今までどおり全部が当たる
- **WHEN** `STACK_RESET=1` を付けて `tools/stack.sh up --check-only` を走らせる
- **THEN** 終了コードは 0 で、版の記録の行の名前の集合は `migrations/ORDER` と一致し、退避の置き場に新しいファイルは無い

### Requirement: S-01 の版が変わると、更新の履歴に 1 行残る

WHEN `ashiato-server migrate` が版の検査（知らない版・読めない版の記録で止まる検査）を通った THE SYSTEM SHALL 当てる前の退避を取る前に、自分の組み立ての印が更新の履歴の直前の**通った**行の今の版と違えば、移行の結果 `attempted`（試み）の 1 行を書き、退避を取る前に commit する（退避に入るので、退避から D-01 を戻しても残る）。
WHEN `ashiato-server migrate` が終わる（当てた・当てる版が無かった・版の中身が当たらずに落ちた・退避に失敗した）THE SYSTEM SHALL 試みの行を書いた回は比べ直さずに必ず、試みの行を書けなかった回（表がまだ無かった）は自分の組み立ての印が更新の履歴の直前の**通った**行の今の版と違えば、更新の履歴に結果の 1 行を書く。表があって試みの行を書かなかった回（直前の通った行と同じ組み立て）は、結果の行も書かない。
THE SYSTEM SHALL 試みと結果の行を当てる処理の錠を持つ接続で書き、結果の行を錠を外す前に書く。
THE SYSTEM SHALL 起動の行を比べて書く取引では、当てる処理の錠を待たずに取ろうとし、取れなければ（migrate が走っている）起動の行を書かずに種別 `update_history_unwritten` の警告を示して起動を続け、取れたらその中で版の記録をもう一度読み、自分の知らない版があれば起動の行を書かない（試みと結果の間に通った行がほかの書き手から入らず、結果の行の前の版は試みの行の前の版と同じになる。版の検査の後に当たった版の上に、古い組み立ての起動の行を書かない）。
WHEN S-01 が起動し、版の検査（「サーバは DB と版が食い違えば起動しない」）を通った THE SYSTEM SHALL 待ち受けを開く前に、自分の組み立ての印が直前の通った行の今の版と違えば、更新の履歴に 1 行を書く。
THE SYSTEM SHALL 1 行に、書いた時刻（S-01 の時計）・部品（`S-01`）・きっかけ（`migrate` か `startup`）・前の版・今の版・移行の結果を持たせる。
移行の結果は `attempted`（版の検査を通り、退避を取る前に書いた試み。当てた版・落ちた版・SQLSTATE は空）・`applied`（この回に当てた版の名前を `migrations/ORDER` の順に持つ）・`nothing_to_apply`（当てる版が無かった。起動の行はいつもこれ）・`failed`（落ちた版の名前と SQLSTATE を持つ。落ちる前に当て終えた版の名前も持つ）・
`backup_failed`（当てる前の退避に失敗した。版は 1 つも当てていない）のどれかにする。
THE SYSTEM SHALL 「前の版」を、更新の履歴の中で書いた順に最後の、移行の結果が `applied` か `nothing_to_apply` の行の今の版にする（試みの行と落ちた行の今の版は D-01 を動かしていないので数えない）。そういう行が無ければ前の版を空（記録の始まり）にする。
THE SYSTEM SHALL migrate が版の中身の失敗でなく落ちた（当てた後の付与・錠・版の記録の読み書きの失敗。終了コード 1）ときも、表を書けるなら移行の結果 `failed` で 1 行を書き、落ちた版の名前を空、SQLSTATE を取れればその符号にする（D-01 が新しい版へ動いた後に落ちた回を `applied` に見せない）。
THE SYSTEM SHALL 自分の組み立ての印を、`<版番号>+unknown…` の印も含めて、そのまま今の版に書く。
WHILE 2 つの書き手（migrate と起動、2 つの起動、または 2 つの migrate）が同時に比べて書く THE SYSTEM SHALL 同じ版の変化を、migrate なら試みと結果の 1 組、起動なら 1 行だけ書く（題の「1 行」は、版の変化 1 回につき結果の行が 1 つ残ること。migrate はその前に試みの行を 1 つ持つ）。
THE SYSTEM SHALL DB の誤りの本文（行の値を含みうる）を、更新の履歴にも、S-01 のログにも、migrate の標準出力・標準エラーにも残さない。migrate が落ちたときに示すのは、種別・落ちた版の名前・SQLSTATE だけにする。
IF migrate が終了コード 3 で止まる（`db_ahead`・`ledger_unreadable`）、または起動が版の検査で断られる THEN THE SYSTEM SHALL 更新の履歴に何も書かない。
THE SYSTEM SHALL 試みの行の後に、同じ migrate の結果の行（`via` が `migrate` で移行の結果が `attempted` でない行）が、次の試みの行より前に無いことで、その回が結果を書く前に終わった（退避から D-01 を戻した・途中で止まった）と読めるようにする。
IF 更新の履歴の表がまだ無い D-01（ST51 の移行より前で落ちた・退避の失敗・ST51 の移行を当てる前の試み）THEN THE SYSTEM SHALL 行を書かずに、種別 `update_history_unwritten` の警告を示し、migrate の終了コードを変えない。
THE SYSTEM SHALL 更新の履歴を書けなかったことで、migrate の終了コードも起動の成否も変えない。

導出元: FR-121。扉 #33。深掘り（ST51）Q1・Q6。premises の C2・C4・C5（design D1（仮）・D2（仮）・D3（仮）・D7・D8・D9・D16（仮））。review/spec.md R2・R6・R9。
試みの行は、退避から D-01 を戻すと消える結果の行の代わりに、通らなかった回を戻しの後にも 1 行残すため（FR-121 ★ 2026-10-29「退避からの戻しで消えない置き場に書く」・Q6）。落ちた版の名前と SQLSTATE は結果の行にだけあり、戻すと消える —— 戻した後に書き直すのは ST58（Q2）。
起動の行は版の検査の後に書く —— 前に書くと、古い S-01 が新しい D-01 に書くことになる（FR-117）。起動できた S-01 は自分の知る版が全部当たっている（ST48）ので、起動の行の移行の結果は `nothing_to_apply` になる。
誤りの本文を残さないのは、制約に反した誤りが行の値（`Key (x)=(値) already exists`）を含み、追記のみの行と書き出しから消す口が無いから（Q1）。SQLSTATE（5 文字の符号）は値を含まない。

#### Scenario: 新しい組み立ての migrate が版を当てると、当てた版の名前を持つ 1 行が残る

- **GIVEN** 印 `0.1.0+<commit A>` の組み立てで migrate を走らせた後、試験用の版を 1 つ並びの末尾に足した D-01
- **WHEN** 印 `0.1.0+<commit B>` の組み立てで migrate を走らせる
- **THEN** 終了コードは 0 で、更新の履歴の最後の行は部品 `S-01`・きっかけ `migrate`・前の版 `0.1.0+<commit A>`・今の版 `0.1.0+<commit B>`・移行の結果 `applied`・当てた版の名前は足した版の 1 つだけ
- **AND** その 1 つ前の行は、きっかけ `migrate`・前の版 `0.1.0+<commit A>`・今の版 `0.1.0+<commit B>`・移行の結果 `attempted`・当てた版の名前と落ちた版の名前と SQLSTATE は空

#### Scenario: 当てる版が無い入れ替えの migrate も 1 行残る

- **GIVEN** 印 `0.1.0+<commit A>` の組み立てで migrate を走らせた D-01
- **WHEN** 同じ版の並びを持つ印 `0.1.0+<commit B>` の組み立てで migrate を走らせる
- **THEN** 更新の履歴の最後の行は前の版 `0.1.0+<commit A>`・今の版 `0.1.0+<commit B>`・移行の結果 `nothing_to_apply`・当てた版の名前は空
- **AND** その 1 つ前の行は今の版 `0.1.0+<commit B>`・移行の結果 `attempted` の行

#### Scenario: 同じ組み立てで migrate を走らせ直しても、行は増えない

- **GIVEN** 印 `0.1.0+<commit A>` の組み立てで migrate を走らせた D-01
- **WHEN** 同じ組み立てで migrate をもう一度走らせる
- **THEN** 更新の履歴の行の数は走らせる前と同じ

#### Scenario: migrate を走らせずに入れ替えた S-01 は、起動のときに 1 行を書く

- **GIVEN** 印 `0.1.0+<commit A>` の組み立てで migrate を走らせた D-01
- **WHEN** 同じ版の並びを持つ印 `0.1.0+<commit B>` の組み立ての S-01 を、migrate を走らせずに起動する
- **THEN** `/healthz` が 200 を返し、更新の履歴の最後の行はきっかけ `startup`・前の版 `0.1.0+<commit A>`・今の版 `0.1.0+<commit B>`・移行の結果 `nothing_to_apply`

#### Scenario: migrate の後に同じ組み立ての S-01 を起動しても、行は増えない

- **GIVEN** 印 `0.1.0+<commit B>` の組み立てで migrate を走らせた D-01
- **WHEN** 同じ組み立ての S-01 を起動し、止めて、もう一度起動する
- **THEN** 更新の履歴の行の数は migrate の後と同じ

#### Scenario: 移行が落ちた回は、落ちた版と SQLSTATE を持つ 1 行が残り、誤りの本文は残らない

- **GIVEN** 印 `0.1.0+<commit A>` で当てた後、試験用の表の text の列に値 `st51-secret-value` の行を入れ、その列を integer に変えて必ず落ちる試験用の版を並びの末尾に足した D-01
- **WHEN** 印 `0.1.0+<commit B>` を渡して、その並びを当てる処理を走らせる
- **THEN** 当てる処理は失敗を返し、更新の履歴の最後の行は移行の結果 `failed`・落ちた版の名前はその版・SQLSTATE は `22P02`・前の版 `0.1.0+<commit A>`・今の版 `0.1.0+<commit B>`
- **AND** 更新の履歴のどの列にも `st51-secret-value` の文字列が無い

#### Scenario: 移行が落ちた migrate は終了コード 1 で終わり、標準エラーに行の値を出さない

- **GIVEN** 版の記録から 1 行を消し、その版を当て直すと、誤りの本文に行の値 `st51-secret-value` を含む誤り（型の入力の誤りなど）で落ちるように DB の中の物を書き換えた D-01
- **WHEN** `ashiato-server migrate` を走らせる
- **THEN** 終了コードは 1 で、標準出力と標準エラーのどこにも `st51-secret-value` が無く、更新の履歴の最後の行は移行の結果 `failed` で落ちた版の名前はその版

#### Scenario: 落ちた回の後の行の前の版は、最後に通った版になる

- **GIVEN** 印 `0.1.0+<commit A>` で通った後、印 `0.1.0+<commit B>` の migrate が落ちた D-01
- **WHEN** 落ちる版を取り除いた印 `0.1.0+<commit C>` の組み立てで migrate を走らせる
- **THEN** 更新の履歴の最後の行は前の版 `0.1.0+<commit A>`・今の版 `0.1.0+<commit C>`・移行の結果 `nothing_to_apply`

#### Scenario: 退避に失敗した回は、版を当てていない行が残る

- **GIVEN** 別の印 `0.1.0+<commit A>` の通った行を置いた D-01 と、退避の道具が失敗する設定
- **WHEN** 退避を有効にして `ashiato-server migrate` を走らせる
- **THEN** 終了コードは 1 で、更新の履歴の最後の行は移行の結果 `backup_failed`・当てた版の名前は空・落ちた版の名前と SQLSTATE は空
- **AND** その 1 つ前の行は今の版が同じ印の、移行の結果 `attempted` の行

#### Scenario: 退避を取るときには、試みの行がもう D-01 に入っている

- **GIVEN** 別の印 `0.1.0+<commit A>` の通った行を置いた D-01
- **WHEN** 印 `0.1.0+<commit B>` を渡して当てる処理を走らせ、退避を取る所で、別の接続から更新の履歴を読む
- **THEN** 別の接続から、今の版 `0.1.0+<commit B>`・移行の結果 `attempted` の行が読める

#### Scenario: 当てる処理の途中で同じ組み立ての S-01 が起動しても、試みと結果の組は崩れない

- **GIVEN** 別の印 `0.1.0+<commit A>` の通った行を置いた D-01
- **WHEN** 印 `0.1.0+<commit B>` を渡して当てる処理を走らせ、退避を取る所で、印 `0.1.0+<commit B>` の起動の書き込みを別の接続から走らせる
- **THEN** 起動の書き込みは待たずに終わって種別 `update_history_unwritten` の警告を示し、更新の履歴の行は書いた順に、試み（前の版 `0.1.0+<commit A>`・今の版 `0.1.0+<commit B>`）・結果（前の版 `0.1.0+<commit A>`・今の版 `0.1.0+<commit B>`・移行の結果 `nothing_to_apply`）の 2 行で、起動の行は無い

#### Scenario: 版の検査の後に知らない版が当たった D-01 には、起動の行を書かない

- **GIVEN** 別の印 `0.1.0+<commit A>` の通った行を置き、版の記録に自分の知らない版の行が 1 つある D-01
- **WHEN** 印 `0.1.0+<commit C>` の起動の書き込みを、版の検査を通さずに直接走らせる
- **THEN** 更新の履歴の行は走らせる前と同じ

#### Scenario: 同じ組み立ての当てる処理を 2 つ同時に走らせても、試みと結果は 1 組だけ

- **GIVEN** 別の印 `0.1.0+<commit A>` の通った行を置いた D-01
- **WHEN** 印 `0.1.0+<commit B>` を渡して当てる処理を 2 つ同時に走らせる
- **THEN** 今の版 `0.1.0+<commit B>` の行は、試みと結果が続けて 1 組だけ

#### Scenario: 更新の履歴の表が無い D-01 に初めて当てて通った migrate は、警告を出し、結果の 1 行だけを残す

- **GIVEN** 何も当たっていない空の DB
- **WHEN** 印 `0.1.0+<commit A>` を渡して全部の版を当てる処理を走らせる
- **THEN** 当てる処理は通り、種別 `update_history_unwritten` の警告が出て、更新の履歴の行は前の版が空・今の版 `0.1.0+<commit A>`・移行の結果 `applied` の 1 行だけ

#### Scenario: 移行が落ちて退避から D-01 を戻しても、通らなかった回の試みの行が残る

- **GIVEN** 印 `0.1.0+<commit A>` で migrate を走らせた後、必ず落ちる試験用の版を並びの末尾に足した D-01
- **WHEN** 印 `0.1.0+<commit B>` の組み立てで退避を有効にして migrate を走らせ（終了コード 1）、そのとき取った退避のファイルを空の別の DB に戻す
- **THEN** 戻した DB の更新の履歴の最後の行は、前の版 `0.1.0+<commit A>`・今の版 `0.1.0+<commit B>`・移行の結果 `attempted` で、その後に結果の行は無い
- **AND** 戻した DB の版の記録に、落ちた版は無い

#### Scenario: 知らない版の当たった DB で止まった migrate は、何も書かない

- **GIVEN** 版の記録に自分の知らない版の行が 1 つある D-01
- **WHEN** 別の組み立ての印で migrate を走らせる
- **THEN** 終了コードは 3 で、更新の履歴の行は走らせる前と同じ

#### Scenario: 版の検査で断られた起動は、何も書かない

- **GIVEN** 版の記録に自分の知らない版の行が 1 つある D-01
- **WHEN** 別の組み立ての印の S-01 を起動する
- **THEN** 終了コードは 3 で、更新の履歴の行は起動する前と同じ

#### Scenario: 更新の履歴の初めの行は、前の版を空にする

- **GIVEN** 更新の履歴の表が空の D-01
- **WHEN** 印 `0.1.0+<commit A>` の組み立てで migrate を走らせる
- **THEN** 更新の履歴の行は 2 行で、どちらも前の版は空・今の版は `0.1.0+<commit A>`、移行の結果は書いた順に `attempted`・`applied`

#### Scenario: 更新の履歴の表が無い D-01 で落ちても、当てる処理は落ちた版の失敗を返し、書けなかったことを警告する

- **GIVEN** 更新の履歴の表を作る版より前の版で必ず落ちる、試験用の並び
- **WHEN** その並びを空の DB に当てる
- **THEN** 当てる処理は落ちた版の失敗を返し、種別 `update_history_unwritten` の警告が出て、DB に更新の履歴の表は無い

#### Scenario: 付与で落ちた migrate は、落ちた版の名前を持たない failed の行を残す

- **GIVEN** 別の印 `0.1.0+<commit A>` の通った行を置き、付与が必ず落ちるようにした D-01
- **WHEN** 印 `0.1.0+<commit B>` を渡して当てる処理を走らせる
- **THEN** 当てる処理は失敗を返し、更新の履歴の最後の行は移行の結果 `failed`・落ちた版の名前は空・今の版 `0.1.0+<commit B>`

#### Scenario: 同じ版の変化を 2 つ同時に書いても、行は 1 つ

- **GIVEN** 別の印 `0.1.0+<commit A>` の通った行を置いた D-01
- **WHEN** 印 `0.1.0+<commit B>` の起動の書き込みを 2 つ同時に走らせる
- **THEN** 今の版 `0.1.0+<commit B>` の行は 1 行だけ

#### Scenario: commit の取れない組み立ての印も、そのまま今の版に書く

- **GIVEN** 別の印 `0.1.0+<commit A>` の通った行を置いた D-01
- **WHEN** 印 `0.1.0+unknown.debug` を渡して起動の書き込みを走らせる
- **THEN** 更新の履歴の最後の行の今の版は `0.1.0+unknown.debug`

### Requirement: 更新の履歴は、決めた置き場と列の形で持つ

THE SYSTEM SHALL 更新の履歴を次の置き場と列の形で持ち、版をまたいで変えない。形を変えるときは列を足すか、決めた値の一覧に値を足すだけにし、今の列の名前・型・NULL の可否と今の値の意味を変えない。

| 置き場 | 列（名前・型・NULL の可否） |
|---|---|
| schema `core`、表 `update_history`（1 回の版の変化につき 1 行） | `seq` bigint NOT NULL（主キー。書いた順）/ `at` timestamp with time zone NOT NULL / `component` text NOT NULL / `via` text NOT NULL / `from_version` text NULL / `to_version` text NOT NULL / `outcome` text NOT NULL / `applied_versions` text[] NOT NULL / `failed_version` text NULL / `sqlstate` text NULL |

THE SYSTEM SHALL `component` を `S-01`、`via` を `migrate` か `startup`、`outcome` を `attempted`・`applied`・`nothing_to_apply`・`failed`・`backup_failed` のどれかに限る。
THE SYSTEM SHALL 「書いた順」を `seq` で決め、時刻で決めない（S-01 の時計は戻りうる）。

導出元: FR-121。扉 #33。premises の C6（design D4（仮））。review/deep.md R11。
凍結する行の形は後から書き直せないので、ST48 の版の記録と同じく Requirement の表で固定する。

#### Scenario: 更新の履歴の置き場と列の形は、決めた形のまま変わらない

- **WHEN** 空の DB に全部の版を当てる
- **THEN** 更新の履歴の schema・表の名前と、各列の名前・型・NULL の可否はこの Requirement の表と一字一句同じ

#### Scenario: 決めていない値の行は入らない

- **WHEN** 所有者の接続で、`via` が `upgrade` の行を更新の履歴に足す
- **THEN** 足すことは制約に反するとして断られる

#### Scenario: 時計が戻っても、前の版は書いた順で決まる

- **GIVEN** 印 `0.1.0+<commit A>` の行の後に、それより前の時刻を持つ印 `0.1.0+<commit B>` の通った行を書いた D-01
- **WHEN** 印 `0.1.0+<commit C>` の組み立てで migrate を走らせる
- **THEN** 更新の履歴の最後の行の前の版は `0.1.0+<commit B>`

### Requirement: 更新の履歴は書き換えられず、S-01 の実行時の役割は足すことだけができる

THE SYSTEM SHALL 更新の履歴の行を、どの役割からも書き換えさせず、消させない（所有者でも、書き換えの門で断る）。
THE SYSTEM SHALL S-01 の実行時の役割に、更新の履歴を読むことと足すことだけを許す。
THE SYSTEM SHALL 落ちた回の行を、落ちた版の取引を戻した後の別の取引で書く（落ちた版と一緒に戻らない）。
THE SYSTEM SHALL 試みの行を、版を当てる取引とは別の取引で、退避を取る前に commit する（当てた版と一緒に戻らず、退避に入る）。

導出元: FR-121。扉 #33（追記のみ）。深掘り（ST51）Q6。premises の C5（design D3（仮）・D10・D16（仮））。
ST48 の版の記録（実行時の役割は読むだけ）と向きが違うのは、起動の行を実行時の役割が書くから（C2）。

#### Scenario: 更新の履歴の行は書き換えられない

- **GIVEN** 更新の履歴に 1 行ある D-01
- **WHEN** 所有者の接続と S-01 の実行時の役割の接続のそれぞれで、その行の今の版を書き換え、その行を消す
- **THEN** どれも断られ、行は書き換える前と同じ

#### Scenario: S-01 の実行時の役割は、更新の履歴を読み、足せる

- **WHEN** S-01 の実行時の役割で、更新の履歴を読み、行を 1 つ足す
- **THEN** どちらも通る

#### Scenario: 落ちた回の行は、落ちた版と一緒に戻らない

- **GIVEN** 印 `0.1.0+<commit A>` で migrate を走らせた後、必ず落ちる試験用の版を並びの末尾に足した D-01
- **WHEN** 印 `0.1.0+<commit B>` の組み立てで migrate を走らせる
- **THEN** 落ちた版は版の記録に無く、更新の履歴には移行の結果 `failed` の行がある
