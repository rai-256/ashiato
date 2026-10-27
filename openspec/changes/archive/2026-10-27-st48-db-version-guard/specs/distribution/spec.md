## Purpose

配布と更新。リリースと 3 つのチャンネルへの配布、インストール、更新の確認と適用と戻し、DB の版の守り、更新の履歴、許諾の同梱を持つ。
版が前後しても本人の DB と記録が失われず、古い版が新しい DB を書き換えないことを守る。

## ADDED Requirements

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
