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

THE SYSTEM SHALL 「当てた組み立ての版番号」を、その組み立ての印と同じ版番号（「3 つの部品の版番号は、1 つの出所から決まる」）にする。
導出元: FR-118。ST52: FR-104（版の出所は 1 つ）・review/spec.md R1。

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
- **THEN** どの行も、指紋がその版のファイルの中身の sha256 の 16 進と一致し、当てた時刻が空でなく、組み立ての版番号がその組み立ての版番号（組み立ての時に渡した版番号。渡さなければ出所の版。「3 つの部品の版番号は、1 つの出所から決まる」）と一致し、commit が組み立てに渡した `ASHIATO_BUILD_COMMIT` の値と一致する

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
THE SYSTEM SHALL 検査する配った印を、印 `v[0-9]*` のうち今の木の HEAD の祖先にあるものに限る（緊急の修正の枝の印は、その枝を merge していない main の検査では数えない。開発版の印 `nightly` は数えない）。
WHILE 数える配った印が 1 つも無い THE SYSTEM SHALL 何も凍結せず、検査を通す。
IF 配った印の版を読めない（印はあるが、その版の中身を取り出せない）THEN THE SYSTEM SHALL 検査を通さない。
IF 検査する木が浅い（履歴が途中で切れていて、どの印が祖先かを決められない）THEN THE SYSTEM SHALL 印を数える前に検査を通さず、履歴が浅いことを示す（配った印が 0 個に見えても通さない）。
導出元: FR-129。ST52: FR-105（緊急の修正の枝）・D1。深掘り Q12（祖先の印だけを数え、CI はその job を完全な履歴で取る）。

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

#### Scenario: 今の木の祖先でない印は、配った移行の検査で数えない
- **GIVEN** main の上の印 `v0.3.0` と、`v0.3.0` から切った枝の上の印 `v0.3.1`（その枝にだけある移行の中身を、main では別の中身で持つ）
- **WHEN** main の HEAD で配った移行の検査を走らせる
- **THEN** 終了コードは 0 で、数えた印は `v0.3.0` だけ

#### Scenario: 修正の枝の上では、枝の祖先の印を数える
- **GIVEN** main の上の印 `v0.3.0` と、その後に main で打った印 `v0.4.0-beta.1`（`v0.3.0` に無い移行を持つ）と、`v0.3.0` から切った枝 `release/0.3`
- **WHEN** 枝 `release/0.3` の HEAD で配った移行の検査を走らせる
- **THEN** 終了コードは 0 で、`v0.4.0-beta.1` の移行が今の木に無いことで落ちない

#### Scenario: 浅い木では、配った移行の検査は通らない
- **GIVEN** 印 `v0.1.0` を HEAD の 1 つ前の commit に持つ木を、深さ 1 で clone し、印だけを取り寄せた木（印は見えるが、祖先かどうかを辿れない）
- **WHEN** 配った移行の検査を走らせる
- **THEN** 終了コードは 0 でなく、出力に「履歴が浅い」を含む（印が 0 個として通らない）

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

### Requirement: 3 つの部品の版番号は、1 つの出所から決まる

THE SYSTEM SHALL サーバ（画面を含む）・携帯の収集アプリ・PC の収集アプリの版番号を、根の `Cargo.toml` の `[workspace.package]` の `version`（以下「出所の版」）だけから決める。
携帯の収集アプリの `versionName` と画面の版番号に、出所の版の写しを手で書かない。
THE SYSTEM SHALL 出所の版を `MAJOR.MINOR.PATCH`（数字 3 つ。後ろに何も付けない）の形に限り、MAJOR が 0 でなければ版の検査を落とす（一般向けの配布までは 0.x。1.0 にする変更は、この検査の値を同じ commit で変える）。
THE SYSTEM SHALL 組み立ての時に版番号が渡されたときだけ、出所の版の代わりにそれを版番号にする。渡せる版番号は `<出所の版>`・`<出所の版>-beta.<N>`（N は 1〜98）・`<出所の版>-dev.<YYYYMMDD>` の 3 つの形に限り、
それ以外の値が渡されたら、どの部品の組み立ても版番号を作らずに失敗する。
THE SYSTEM SHALL 版番号を渡すのに、出所の版を持つファイル（`Cargo.toml`・`Cargo.lock`・携帯の組み立ての設定・画面の木）を書き換えない。
THE SYSTEM SHALL 組み立ての印・版の記録の「当てた組み立ての版番号」・更新の履歴の版・`db_ahead` の行に示す自分の版番号に、同じ組み立ての同じ版番号を使う。
THE SYSTEM SHALL API の契約（`docs/openapi.json`）の版（`info.version`）を、渡した版番号でなく出所の版にする（契約の写しは checked-in で、組み立てごとに変えない）。
組み立ての印を持つのは、サーバ・画面・携帯の収集アプリ・PC の収集アプリの 4 つ（画面はサーバが配るが、印は画面自身が持つ。ST49）で、この Requirement の「部品」はこの 4 つを指す。
THE SYSTEM SHALL 3 部品と画面の版番号の出所を見る検査を持ち、手書きの版番号が残っている・部品ごとの版番号が食い違うときに CI を落とす。

導出元: FR-104（版の出所は 1 つにする —— K3）, NFR-25。深掘り Q6（推奨）。premises P10。review/deep.md R12。
書き換えずに渡すのは、書き換えると組み立ての印が `.dirty` になり、配った版が書いた追記のみの記録の印が、手元で直して組んだものと区別できなくなるため。

#### Scenario: 版番号を渡さない組み立てでは、4 つの部品の版番号が出所の版と同じ
- **GIVEN** 出所の版が `0.4.0` の木
- **WHEN** 版番号を渡さずに、サーバ・PC の収集アプリ・携帯の収集アプリ・画面を組み立てる
- **THEN** 4 つの組み立ての印の版番号はどれも `0.4.0`

#### Scenario: 渡した版番号は、4 つの部品の印に同じ値で入り、印は .dirty を持たない
- **GIVEN** 出所の版が `0.4.0` の、未 commit の変更の無い木
- **WHEN** 版番号 `0.4.0-beta.2` を渡して、4 つの部品を release で組み立てる
- **THEN** 4 つの組み立ての印はどれも `0.4.0-beta.2+<HEAD の 40 桁>` で、`.dirty` も `.debug` も持たない
- **AND** 組み立ての前と後で、木の `git status --porcelain` は空のまま

#### Scenario: 出所の版と違う版番号を渡すと、組み立てが失敗する
- **GIVEN** 出所の版が `0.4.0` の木
- **WHEN** 版番号 `0.5.0-beta.1` を渡して組み立てる
- **THEN** 組み立ては 0 でない終了コードで終わり、出所の版と渡した版番号の両方が示される

#### Scenario: 決めた形でない版番号を渡すと、組み立てが失敗する
- **GIVEN** 出所の版が `0.4.0` の木
- **WHEN** 版番号 `0.4.0-rc.1`・`0.4.0-beta.99`・`0.4.0-beta.0`・`0.4.0-dev.2026111` のどれかを渡して組み立てる
- **THEN** どれも組み立ては 0 でない終了コードで終わる

#### Scenario: 渡した版番号で組んだサーバの版の記録は、渡した版番号を持つ
- **GIVEN** 版番号 `0.4.0-beta.2` を渡して組み立てたサーバ
- **WHEN** 空の DB に `ashiato-server migrate` を走らせる
- **THEN** 版の記録のどの行も、組み立ての版番号が `0.4.0-beta.2`

#### Scenario: 渡した版番号で組んだサーバが知らない版で止まると、self の行は渡した版番号を持つ
- **GIVEN** 版番号 `0.4.0-beta.2` を渡して組み立てたサーバと、そのサーバの知らない版の記録を 1 行持つ DB
- **WHEN** `ashiato-server migrate` を走らせる
- **THEN** 終了コードは 3 で、標準エラーの `  self: ` の行の版番号は `0.4.0-beta.2`

#### Scenario: 渡した版番号で組んだサーバの更新の履歴の今の版は、渡した版番号で始まる
- **GIVEN** 版番号 `0.4.0-beta.2` を渡して組み立てたサーバ
- **WHEN** 空の DB に `ashiato-server migrate` を走らせる
- **THEN** 更新の履歴の最後の行の今の版は `0.4.0-beta.2+` で始まる

#### Scenario: API の契約の版は、渡した版番号に依らず出所の版
- **GIVEN** 出所の版が `0.4.0` の木
- **WHEN** 版番号 `0.4.0-beta.2` を渡して API の契約を生成する
- **THEN** 契約の `info.version` は `0.4.0`

#### Scenario: 携帯の組み立ての設定に版番号を手で書くと、版の検査が落ちる
- **WHEN** 携帯の組み立ての設定に `versionName = "0.1.0"` の文字の版番号を書いて版の検査を走らせる
- **THEN** 検査は 0 でない終了コードで終わり、手書きの版番号の場所が示される

#### Scenario: 画面の package.json に版番号を書くと、版の検査が落ちる
- **WHEN** `web/package.json` に `version` の欄を書いて版の検査を走らせる
- **THEN** 検査は 0 でない終了コードで終わる

#### Scenario: 出所の版の MAJOR が 0 でないと、版の検査が落ちる
- **WHEN** 出所の版を `1.0.0` にして、検査の値を変えずに版の検査を走らせる
- **THEN** 検査は 0 でない終了コードで終わる

#### Scenario: 出所の版が MAJOR.MINOR.PATCH の形でないと、版の検査が落ちる
- **WHEN** 出所の版を `0.4.0-beta.1` にして版の検査を走らせる
- **THEN** 検査は 0 でない終了コードで終わる

### Requirement: 携帯の versionCode は版番号から計算する

THE SYSTEM SHALL 携帯の収集アプリの versionCode を、版番号から `MAJOR×1,000,000 + MINOR×10,000 + PATCH×100 + 末尾` で計算し、手で付けない。
末尾は、ベータ版 `-beta.<N>` なら N、正式版（後ろに何も付かない版番号）なら 99、開発版 `-dev.<YYYYMMDD>` なら 0 とする。
THE SYSTEM SHALL MINOR と PATCH が 99 を超える版番号では、携帯の組み立てを失敗させる（versionCode の桁があふれて順が崩れる）。
THE SYSTEM SHALL この式を、配った携帯の APK の versionCode の付け方として、版をまたいで変えない（Android は小さい versionCode の上書きを拒むので、一度配った付け方より小さい数になる付け方へは戻れない）。

導出元: FR-104, FR-105。深掘り Q3（推奨）。premises P5・P13。review/deep.md R4。

#### Scenario: versionCode は版番号から決まり、ベータ < 正式 < 次の版の順になる
- **WHEN** 版番号 `0.3.0-beta.2`・`0.3.0`・`0.3.1`・`1.0.0`・`0.4.0-dev.20261101` で携帯の versionCode を計算する
- **THEN** それぞれ 30002・30099・30199・1000099・40000 になる

#### Scenario: 同じ版の開発版は、毎日同じ versionCode になる
- **WHEN** 版番号 `0.4.0-dev.20261101` と `0.4.0-dev.20261102` で携帯の versionCode を計算する
- **THEN** どちらも 40000

#### Scenario: 桁を超える版番号では、versionCode の計算が失敗する
- **WHEN** 版番号 `0.100.0` か `0.1.100` で携帯の versionCode を計算する
- **THEN** 計算は 0 でない終了コードで終わり、超えた部分が示される

#### Scenario: 桁を超える版番号では、携帯の組み立てが失敗する
- **GIVEN** 出所の版を `0.100.0` にした木
- **WHEN** 携帯の収集アプリを組み立てる
- **THEN** 組み立ては 0 でない終了コードで終わる

#### Scenario: 組み立てた APK の versionCode は、渡した版番号から計算した値
- **WHEN** 版番号 `0.4.0-beta.2` を渡して携帯の収集アプリを組み立てる
- **THEN** できた APK の versionCode は 40002 で、versionName は `0.4.0-beta.2`

### Requirement: 配る組み立ては、綺麗な木の release で、携帯の APK は接続先と資格情報を持たない

THE SYSTEM SHALL 公開する配布物のどれかの組み立ての印が `.dirty` か `.debug` を持てば、そのリリースを公開しない。
THE SYSTEM SHALL 公開する携帯の APK に焼き込んだ接続先・資格情報・本人の識別子（`BASE_URL`・`API_TOKEN`・`USER_ID`）のどれかが空でなければ、そのリリースを公開しない。
THE SYSTEM SHALL 公開しなかったときに、どの配布物のどの検査に当たったかを示し、資格情報の値そのものは示さない。

導出元: FR-104。premises の C1・C8・P15。review/deep.md R1・R12。
空にする側だけが、公開のリリースから資格情報を外へ出さない（一度公開した配布物は、取り下げても写しが残りうる）。

#### Scenario: .dirty の印を持つ配布物があれば公開しない
- **GIVEN** 4 つの配布物のうち 1 つの組み立ての印が `.dirty` を持つ置き場
- **WHEN** 配布物の検査を走らせる
- **THEN** 検査は 0 でない終了コードで終わり、その配布物の名前と `.dirty` が示される

#### Scenario: .debug の印を持つ配布物があれば公開しない
- **GIVEN** debug で組み立てた携帯の APK を含む置き場
- **WHEN** 配布物の検査を走らせる
- **THEN** 検査は 0 でない終了コードで終わる

#### Scenario: 資格情報を焼き込んだ APK があれば公開せず、値を示さない
- **GIVEN** 資格情報 `secret-token-for-test` を焼き込んで組み立てた携帯の APK を含む置き場
- **WHEN** 配布物の検査を走らせる
- **THEN** 検査は 0 でない終了コードで終わり、空でない項目の名前（`API_TOKEN`）が示され、出力のどこにも `secret-token-for-test` が無い

#### Scenario: 綺麗な release で資格情報の無い配布物は、検査を通る
- **GIVEN** 版番号を渡して綺麗な木から release で組み立てた 4 つの配布物
- **WHEN** 配布物の検査を走らせる
- **THEN** 検査の終了コードは 0

### Requirement: 携帯の APK は、チャンネルごとに決めた識別子と鍵で署名する

THE SYSTEM SHALL 正式版とベータ版の携帯の APK を、アプリ識別子 `dev.ashiato.collector` と正式な鍵で署名する。
THE SYSTEM SHALL 開発版の携帯の APK を、アプリ識別子 `dev.ashiato.collector.dev` と、正式な鍵と別の開発版の鍵で署名する（扉 34）。
THE SYSTEM SHALL 正式な鍵を、印 `v*` の push の実行にだけ開いた置き場（配布用の Environment）から、開発版の鍵を開発版の予定の実行と main からの実行にだけ開いた置き場（開発版の Environment）から読み、PR の実行（同じリポジトリの枝からの PR を含む）には渡さない。正式な鍵は開発版の作り直しの実行にも渡さない。
THE SYSTEM SHALL PR の組み立ての携帯の APK を debug の鍵で署名し、正式な鍵も開発版の鍵も使わない。
IF 署名する鍵が置き場に無い THEN THE SYSTEM SHALL 署名しない APK を公開せず、そのリリースを公開しない（開発版は前の版のリリースを残し、落ちた理由に開発版の鍵が無いことを示す）。
IF 開発版の APK に署名するときに、開発版の鍵の指紋の期待値（開発版の Environment の変数）が渡されないか、渡した鍵の指紋と違う THEN THE SYSTEM SHALL 署名せずに失敗する。
THE SYSTEM SHALL 鍵を作る台本で、正式な鍵と開発版の鍵を RSA 4096 bit・有効期限 25 年以上で作り、作った鍵の種類・長さ・有効期限を読み直して確かめ、満たさなければ失敗する。

THE SYSTEM SHALL 開発版の識別子 `dev.ashiato.collector.dev` の APK に正式な鍵を、そうでない APK に開発版の鍵を当てようとしたら、署名せずに失敗する。
導出元: 扉 34（識別子と署名鍵）。FR-106（開発版は別のアプリ）。ST52 の深掘り Q2・Q11（本人の答え。推奨と同じ）。ST52 の premises P2・P8・P12・review/deep.md R2・R3・R13・review/spec.md R10・review/code.md R26・D3（仮）・D21（仮）。
ST54 の premises の C4・C5・P15。鍵の置き場は扉 34 の「手元の USB と拠点外の 2 か所」に GitHub の Environment を足す（ST52 Q2。扉 34 ★ 2026-11-01。Environment の値は GitHub から取り出せないので、控えの代わりにはならない）。

#### Scenario: 正式版の APK は、正式な識別子と正式な鍵で署名されている
- **GIVEN** 使い捨ての正式な鍵
- **WHEN** 版番号 `0.4.0` で配る形に組み立てた携帯の APK に、署名する手順を当てる
- **THEN** APK のアプリ識別子は `dev.ashiato.collector` で、署名の証明書の指紋は使い捨ての正式な鍵の指紋と一致する

#### Scenario: 開発版の APK は、.dev の識別子と開発版の鍵で署名されている
- **GIVEN** 使い捨ての開発版の鍵
- **WHEN** 版番号 `0.4.0-dev.20261101` で配る形に組み立てた携帯の APK に、署名する手順を当てる
- **THEN** APK のアプリ識別子は `dev.ashiato.collector.dev` で、署名の証明書の指紋は使い捨ての開発版の鍵の指紋と一致し、正式な鍵の指紋と一致しない

#### Scenario: PR の組み立ての APK は debug の鍵で署名される
- **WHEN** PR の組み立てで携帯の APK を組む
- **THEN** APK の署名の証明書は debug の鍵のもの

#### Scenario: 識別子とチャンネルの鍵を取り違えると、署名しない
- **WHEN** 開発版の識別子の APK に正式な鍵を、正式な識別子の APK に開発版の鍵を当てて、署名する手順を走らせる
- **THEN** どちらも手順は 0 でない終了コードで終わり、署名された APK はできない

#### Scenario: 開発版の鍵の指紋の期待値が無ければ、開発版の APK に署名しない
- **GIVEN** 使い捨ての開発版の鍵と、版番号 `0.4.0-dev.20261102` で組み立てた携帯の APK
- **WHEN** 開発版の鍵の指紋の期待値を渡さずに、また別の回に別の鍵の指紋を渡して、署名する手順を走らせる
- **THEN** どちらも手順は 0 でない終了コードで終わり、署名された APK はできない

#### Scenario: 鍵の置き場は、印と main のそれぞれにだけ開いている
- **WHEN** 鍵の置き場の検査を配布元に向けて走らせる
- **THEN** 配布用の Environment の配る対象は印 `v*` だけで、開発版の Environment の配る対象は枝 `main` だけ。そうでなければ検査は 0 でない終了コードで終わる

#### Scenario: 鍵が置き場に無ければ、署名しない APK を公開しない
- **GIVEN** 正式な鍵の置き場が空の公開の手順
- **WHEN** 印 `v0.4.0` で携帯の APK に署名する手順を走らせる
- **THEN** 手順は 0 でない終了コードで終わり、署名された APK はできず、公開の手順に渡す配布物が無い

#### Scenario: 開発版の鍵が置き場に無ければ、開発版は前の版のまま残る
- **GIVEN** 開発版の鍵の置き場が空で、前の開発版のリリースがある配布元を模した公開の手順（偽の `gh`）
- **WHEN** 開発版の公開の手順を走らせる
- **THEN** 手順は 0 でない終了コードで終わり、出力に開発版の鍵が無いことを示し、リリースの配布物を入れ替える要求も印 `nightly` を動かす要求も出ない

#### Scenario: 鍵を作る台本は、決めた種類・長さ・有効期限の鍵だけを作る
- **WHEN** 鍵を作る台本を、使い捨ての置き場に向けて走らせる
- **THEN** できた鍵は RSA 4096 bit で、有効期限は作った日から 25 年以上先

#### Scenario: 鍵を作る台本は、決めた長さに満たない鍵を確かめで落とす
- **WHEN** 鍵を作る台本の確かめを、RSA 2048 bit の鍵に当てる
- **THEN** 確かめは 0 でない終了コードで終わる

### Requirement: 手で起こした確かめの組み立ては、下書きのリリースにだけ置き、印を作らない

THE SYSTEM SHALL 本人が手で起こした確かめの組み立て（PR の組み立ての配布物を使う）を、下書き（draft）のリリースにだけ置く。下書きは公開せず、印を作らない。
THE SYSTEM SHALL 下書きのリリースに、本物のリリースと同じ 4 つの配布物と同じリリースの種類（正式版・先行公開）を置く。
THE SYSTEM SHALL 下書きのリリースを作った前と後で、配布元の印 `v*` の数を変えない。
THE SYSTEM SHALL 開発版の確かめの組み立て（手で起こした開発版の作り直しの、公開しない run の配布物を使う）では、携帯の APK を、渡された使い捨ての開発版の鍵で署名してから開発版の配布物の検査を通し、説明文に開発版の決まった文を使った先行公開の下書きにする。印 `nightly` を作らず、動かさない。

導出元: FR-104, FR-105, FR-106。ST52 の深掘り Q5（推奨: ST52 は下書きで確かめ、最初の本物の印は ST53 の後）・premises P16・review/deep.md R7。ST54 の深掘り Q9（推奨: 下書きのリリースと使い捨ての鍵で確かめる）・D10。

#### Scenario: 確かめの組み立ては、印を作らずに下書きのリリースになる
- **GIVEN** PR の組み立てでできた 4 つの配布物
- **WHEN** 確かめの組み立ての手順を、偽の配布元に向けて版番号 `0.1.0-beta.1` で走らせる
- **THEN** 手順は下書きで先行公開のリリースを 1 つ作る要求だけを出し、印を作る要求も公開する要求も出さない

#### Scenario: 確かめの組み立ても、配布物が欠ければ下書きを作らない
- **GIVEN** 4 つの配布物のうち携帯の APK が無い置き場
- **WHEN** 確かめの組み立ての手順を、偽の配布元に向けて走らせる
- **THEN** 手順は 0 でない終了コードで終わり、配布元への要求を 1 つも出さない

#### Scenario: 開発版の確かめの組み立ては、開発版の鍵で署名した下書きになり、印を作らない
- **GIVEN** 手で起こした開発版の作り直しでできた、署名していない携帯の APK を含む 4 つの配布物と、使い捨ての開発版の鍵
- **WHEN** 開発版の確かめの組み立ての手順を、偽の配布元に向けて版番号 `0.4.0-dev.20261102` で走らせる
- **THEN** 手順は、署名の証明書の指紋が使い捨ての開発版の鍵のものの `ashiato-collector-dev-0.4.0-dev.20261102.apk` を含む 4 つで、説明文に開発版の決まった文を使う先行公開の下書きを 1 つ作る要求だけを出し、印を作る要求も動かす要求も公開する要求も出さない

### Requirement: サーバは、組み込んだ画面を同じ番号で配る

THE SYSTEM SHALL 配るサーバの実行ファイルに、画面の組み立て済みのファイルを組み込み、サーバが待ち受ける同じ番号で、`/` に画面の入口、画面の組み立て済みのファイルをその道に配る。
THE SYSTEM SHALL `/api/<道>` を、今の API の `/<道>` と同じ認可・同じ応答で受ける（画面が今使う道のまま）。収集アプリの送り先の道（`/ingest`・`/heartbeat` など）は変えない。
THE SYSTEM SHALL 画面のファイルの応答に `Content-Security-Policy: default-src 'self'; frame-ancestors 'none'` と `Cache-Control: no-store` を付ける（今の `vite preview` と同じ）。
THE SYSTEM SHALL 画面を組み込まずに組み立てたサーバ（開発と試験の組み立て）では、`/` に画面が無いことを示す 404 を返し、API は今のまま動く。
THE SYSTEM SHALL 配る組み立てでは、画面を組み込まずにサーバを組み立てたら失敗させる。
THE SYSTEM SHALL 開発の起動と画面の e2e が使う `vite preview` の画面の配り方を変えない。

導出元: FR-104（★ 2026-10-25。V-01 は S-01 が配る。配るのに Node の実行環境を求めない）。深掘り Q8（推奨）。

#### Scenario: 配ったサーバだけで、Node を入れずに画面が開く
- **GIVEN** 画面を組み込んで組み立てたサーバを DB に繋いで起動した（画面を配る Node のプロセス `vite preview` を介さない。サーバの実行ファイルは画面のファイルを外から読まない）
- **WHEN** 本物のブラウザでサーバの番号の `/` を開き、合言葉でログインする
- **THEN** 画面の入口が開き、ログインの後に画面が `/api/…` から読んだ稼働状況を表示する

#### Scenario: 画面のファイルには、今と同じ守りの見出しが付く
- **GIVEN** 画面を組み込んで組み立てたサーバ
- **WHEN** `/` と、画面の組み立て済みのファイルの 1 つを取る
- **THEN** どちらの応答も `Content-Security-Policy: default-src 'self'; frame-ancestors 'none'` と `Cache-Control: no-store` を持つ

#### Scenario: /api の道は、今の API と同じ応答と認可になる
- **GIVEN** 画面を組み込んで組み立てたサーバ
- **WHEN** `/api/healthz` と `/healthz`、資格情報の無い `/api/events` と `/events` を取る
- **THEN** `/api/healthz` と `/healthz` は同じ本文で、資格情報の無い 2 つはどちらも 401

#### Scenario: /api の道で読んだ画面の読み出しも、今と同じ読み出しの記録が残る
- **GIVEN** 画面を組み込んで組み立てたサーバと、ログインした画面の印
- **WHEN** その印で `/api/events` を読み、続けて `/events` を読む
- **THEN** 読み出しの記録には、2 回とも同じ形の 1 行ずつが残る

#### Scenario: 収集アプリの送り先の道は、今のまま受ける
- **GIVEN** 画面を組み込んで組み立てたサーバ
- **WHEN** 収集アプリの資格情報で `/heartbeat` に生存信号を 1 件送る
- **THEN** 今と同じく受け付けられる

#### Scenario: 画面を組み込まない組み立てでは、/ は 404 で API は動く
- **GIVEN** 画面を組み込まずに組み立てたサーバ
- **WHEN** `/` と `/healthz` を取る
- **THEN** `/` は 404、`/healthz` は今と同じ本文

#### Scenario: 配る組み立てで画面が無ければ、サーバの組み立てが失敗する
- **WHEN** 画面の組み立て済みのファイルの置き場を渡さずに、配る形でサーバを組み立てる
- **THEN** 組み立ては 0 でない終了コードで終わる

### Requirement: コードを触った PR ごとに、配る形の 3 部品を公開せずに組む

WHEN コード（`docs/`・`openspec/`・`*.md` 以外）を触った PR が CI を走らせる THE SYSTEM SHALL サーバ（画面を組み込む。Windows 向けと Linux 向け）・携帯の APK・PC の収集アプリを、配るときと同じ手順で、出所の版を版番号にして組み立てる。
THE SYSTEM SHALL その組み立てで、配布物が 1 つでも組めない・配布物の版番号が食い違う・配る組み立ての検査に当たるときに CI を落とす。ただし携帯の APK は debug の鍵で署名し、`.debug` の印でない release の組み立てにし、`.dirty` の検査は綺麗な checkout の上で行う。
THE SYSTEM SHALL その組み立ての配布物をどこにも公開しない（CI の実行の成果物としてだけ置く）。
WHEN PR が `docs/`・`openspec/`・`*.md` だけを触る THE SYSTEM SHALL この組み立てを飛ばす。

導出元: Story の価値（配れる形を壊した Story が、その PR の時点で落ちる）。深掘り Q9（推奨）。premises P6・P9。

#### Scenario: コードを触った PR では、配る形の組み立てが走る
- **WHEN** `crates/server/src/lib.rs` を触った PR の CI の出し分けを判定する
- **THEN** 配る形の組み立ての job は走る

#### Scenario: 文書だけの PR では、配る形の組み立てを飛ばす
- **WHEN** `docs/` と `openspec/` だけを触った PR の CI の出し分けを判定する
- **THEN** 配る形の組み立ての job は飛ばされる

#### Scenario: 配る形を壊した PR は、その PR の CI で落ちる
- **WHEN** 配布物の 1 つの組み立てが失敗する木で、配る形の組み立ての手順を走らせる
- **THEN** 手順は 0 でない終了コードで終わる

### Requirement: 配った印 v* は、消せず、動かせない

THE SYSTEM SHALL 配布元に、印 `v*` の削除と、印 `v*` が指す commit の付け替えを拒む規則を持たせる。
THE SYSTEM SHALL 正式版とベータ版を公開する前に、その規則が配布元で有効になっていることを確かめ、有効でなければ公開しない。
THE SYSTEM SHALL 壊れた版を出したときは、印を消さずに次の版を出して直す手順を、配る手順の文書に持つ。

導出元: FR-105。FR-124（配った契約の版は受け付け続ける）・FR-129（配った移行は変えない）を印の側で守る。D2（仮）。review/deep.md R10。
印を消すと、配った移行の凍結と送信契約の版の比べが、その版について黙って外れる。

#### Scenario: 印の規則が入っていなければ、公開しない
- **GIVEN** 印 `v*` の削除と付け替えを拒む規則を持たない偽の配布元
- **WHEN** 公開の前の確かめを走らせる
- **THEN** 確かめは 0 でない終了コードで終わり、規則が無いことが示される

#### Scenario: 印の規則が入っていれば、公開の前の確かめを通る
- **GIVEN** 印 `v*` の削除と付け替えを拒む規則が有効な偽の配布元
- **WHEN** 公開の前の確かめを走らせる
- **THEN** 確かめの終了コードは 0

### Requirement: 緊急の修正の枝は、配った版から切り、移行と封筒の形を変えない

THE SYSTEM SHALL 正式版だけに緊急の修正を当てるときに限り、配った正式版の印 `v<MAJOR>.<MINOR>.<PATCH>` から枝 `release/<MAJOR>.<MINOR>` を切ることを許し、その枝の上では、同じ MAJOR.MINOR で PATCH を上げた正式版の印だけを公開する（ベータ版と開発版はその枝から出さない）。
THE SYSTEM SHALL 修正の枝の正式版の印を、枝を切った元の印と `migrations/ORDER` と封筒の形の写し（`docs/contract/envelope.json`）が同じときだけ公開する。
THE SYSTEM SHALL 修正の枝の正式版を公開した後に、その枝を消す手順を、配る手順の文書に持つ。

導出元: FR-105（正式版にだけ緊急の修正を当てるときに限り、その版の枝を一時的に切る）。premises の C9。D6（仮）。review/deep.md R9。
修正の枝で移行を足すと、その移行は main に無く、main の後の版が同じ DB に当たったときに「知らない版」で止まる。封筒の形を変えると、送信契約の版が main と枝で別の意味になる。

#### Scenario: 修正の枝の正式版の印は、移行と封筒の形が元の印と同じなら公開できる
- **GIVEN** 印 `v0.3.0` から切った枝 `release/0.3` に、移行と封筒の形に触れない commit を 1 つ足し、出所の版を `0.3.1` にした木
- **WHEN** その commit の印 `v0.3.1` で配るときの判定を走らせる
- **THEN** 判定の終了コードは 0 で、リリースの種類は通常のリリース

#### Scenario: 修正の枝で移行を足した印は公開しない
- **GIVEN** 印 `v0.3.0` から切った枝 `release/0.3` に移行を 1 つ足した木
- **WHEN** その commit の印 `v0.3.1` で配るときの判定を走らせる
- **THEN** 判定は 0 でない終了コードで終わり、`migrations/ORDER` が元の印と違うことが示される

#### Scenario: 修正の枝で封筒の形を変えた印は公開しない
- **GIVEN** 印 `v0.3.0` から切った枝 `release/0.3` で封筒の形の写しを変えた木
- **WHEN** その commit の印 `v0.3.1` で配るときの判定を走らせる
- **THEN** 判定は 0 でない終了コードで終わる

#### Scenario: 修正の枝からベータ版は出さない
- **GIVEN** 印 `v0.3.0` から切った枝 `release/0.3`
- **WHEN** その枝の commit の印 `v0.3.1-beta.1` で配るときの判定を走らせる
- **THEN** 判定は 0 でない終了コードで終わる

### Requirement: 印（タグ）の名前で、3 つのチャンネルに分けて配り、どのチャンネルも 4 つの配布物を並べる

WHEN 印 `v<MAJOR>.<MINOR>.<PATCH>` が配布元に push される THE SYSTEM SHALL その版を通常のリリース（正式版）として公開する。
WHEN 印 `v<MAJOR>.<MINOR>.<PATCH>-beta.<N>` が push される THE SYSTEM SHALL その版を先行公開（pre-release）のリリース（ベータ版）として公開する。
THE SYSTEM SHALL 開発版を、印 `nightly` の付いた 1 つの先行公開のリリースとして持ち、作り直すたびに印をその commit へ動かし、リリースの配布物を入れ替える（開発版のリリースは 1 つだけ）。
THE SYSTEM SHALL 1 つのリリースに、サーバ（画面を含む。Windows x86_64 向けと Linux x86_64 向けの 2 つ）・携帯の収集アプリの APK・PC の収集アプリ（Windows x86_64 向け）を、同じ版番号で並べる（正式版・ベータ版・開発版のどれも 4 つ）。
THE SYSTEM SHALL 配布物の名前を `ashiato-server-<版番号>-windows-x86_64.zip`・`ashiato-server-<版番号>-linux-x86_64.tar.gz`・`ashiato-collector-<版番号>.apk`（開発版は `ashiato-collector-dev-<版番号>.apk`）・`ashiato-collector-windows-<版番号>-x86_64.zip` にし、版をまたいで変えない（後の Story が版の確認で名前を読む）。
THE SYSTEM SHALL 印の版番号（`v` を除いたもの）の MAJOR.MINOR.PATCH が、印の commit の出所の版と違えば公開しない。
THE SYSTEM SHALL 配布物が 1 つでも組めなければ公開しない（正式版とベータ版はリリースを作らない。開発版は前の版のリリースを残す）。開発版の置き場に正式版の名前の APK（`ashiato-collector-<版番号>.apk`）があれば公開しない。
THE SYSTEM SHALL 印 `v*` を、main の祖先の commit に付いたときだけ公開する。例外は、緊急の修正の枝 `release/<MAJOR>.<MINOR>` の上の正式版の印で、その規則は「緊急の修正の枝は、配った版から切り、移行と封筒の形を変えない」が持つ。
THE SYSTEM SHALL 配った移行の凍結と送信契約の版の検査が数える「配った印」を `v[0-9]*` の印に限り、開発版の印 `nightly` を数えない。
IF 印の判定を走らせる木が浅い（履歴が途中で切れていて、印の commit が main の祖先かを決められない）THEN THE SYSTEM SHALL 祖先を見る前に判定を通さず、出力に「履歴が浅い」を示す（公開しない）。
THE SYSTEM SHALL チャンネルごとの枝を持たない（正式版もベータ版も開発版も main から作る）。

導出元: FR-104, FR-105, FR-106。ST52 の深掘り Q7・Q10（推奨）・Q4（本人の答え: ST54 まで開発版に携帯の APK を入れない。ST54 で期限が来て、開発版も 4 つにした）。ST52 の premises の C2・C7・C9・P17・review/deep.md R8・D23（仮）・D24（仮）。
ST54 の premises の C3（開発版のリリースは携帯の APK を含めて 4 つ。ST52 の申し送り）。

#### Scenario: 浅い木では、印の判定は通らない
- **GIVEN** 出所の版が `0.4.0` の main の commit に印 `v0.4.0` を付けた木を、深さ 1 で clone した木
- **WHEN** その印を配るときの判定を走らせる
- **THEN** 終了コードは 0 でなく、出力に「履歴が浅い」を含む（main の祖先でないとも、公開してよいとも判定しない）

#### Scenario: 正式版の印は、通常のリリースと判定される
- **GIVEN** 出所の版が `0.4.0` の main の commit
- **WHEN** その commit に印 `v0.4.0` を付けて配るときの判定を走らせる
- **THEN** リリースの種類は通常のリリース（先行公開でない）で、版番号は `0.4.0`

#### Scenario: 1 つのリリースの配布物は、決めた名前の 4 つで、同じ版番号を持つ
- **GIVEN** 版番号 `0.4.0` を渡して組み立てた配布物の置き場
- **WHEN** 配布物の検査を走らせる
- **THEN** 配布物は `ashiato-server-0.4.0-windows-x86_64.zip`・`ashiato-server-0.4.0-linux-x86_64.tar.gz`・`ashiato-collector-0.4.0.apk`・`ashiato-collector-windows-0.4.0-x86_64.zip` の 4 つで、どの組み立ての印の版番号も `0.4.0`

#### Scenario: 開発版のリリースの配布物は、開発版の APK を含む 4 つで、同じ版番号を持つ
- **GIVEN** 版番号 `0.4.0-dev.20261102` を渡して組み立て、開発版の鍵で署名した配布物の置き場
- **WHEN** 開発版の配布物の検査を走らせる
- **THEN** 検査の終了コードは 0 で、配布物は `ashiato-server-0.4.0-dev.20261102-windows-x86_64.zip`・`ashiato-server-0.4.0-dev.20261102-linux-x86_64.tar.gz`・`ashiato-collector-dev-0.4.0-dev.20261102.apk`・`ashiato-collector-windows-0.4.0-dev.20261102-x86_64.zip` の 4 つ

#### Scenario: 開発版の置き場に携帯の APK が無ければ、検査は落ちる
- **GIVEN** 上の 4 つから `ashiato-collector-dev-0.4.0-dev.20261102.apk` を除いた置き場
- **WHEN** 開発版の配布物の検査を走らせる
- **THEN** 検査は 0 でない終了コードで終わり、欠けた `ashiato-collector-dev-0.4.0-dev.20261102.apk` の名前が示される

#### Scenario: 開発版の置き場に正式版の名前の APK があれば、検査は落ちてその名前を示す
- **GIVEN** 開発版の 4 つに `ashiato-collector-0.4.0-dev.20261102.apk` を足した置き場
- **WHEN** 開発版の配布物の検査を走らせる
- **THEN** 検査は 0 でない終了コードで終わり、足した APK の名前が示される

#### Scenario: ベータ版の印で、先行公開のリリースになる
- **GIVEN** 出所の版が `0.4.0` の main の commit
- **WHEN** 印 `v0.4.0-beta.2` で配るときの判定を走らせる
- **THEN** リリースの種類は先行公開で、版番号は `0.4.0-beta.2`

#### Scenario: 印の版と出所の版が違えば公開しない
- **GIVEN** 出所の版が `0.4.0` の main の commit
- **WHEN** 印 `v0.5.0-beta.1` で配るときの判定を走らせる
- **THEN** 判定は 0 でない終了コードで終わり、リリースは作られない

#### Scenario: main の祖先でない commit の印は公開しない
- **GIVEN** main の祖先でない枝（`release/*` でない）の commit
- **WHEN** その commit の印 `v0.4.0-beta.3` で配るときの判定を走らせる
- **THEN** 判定は 0 でない終了コードで終わり、main の祖先でないことが示される

#### Scenario: 配布物が 1 つでも欠ければ公開しない
- **GIVEN** 4 つの配布物のうち PC の収集アプリが無い置き場
- **WHEN** 配布物をそろえて公開する手順を走らせる
- **THEN** 手順は 0 でない終了コードで終わり、欠けた配布物の名前が示され、リリースは作られない

#### Scenario: 決めた形でない印は、どのチャンネルにも配らない
- **WHEN** 印 `v0.4.0-rc.1`・`v0.4`・`release-0.4.0` で配るときの判定を走らせる
- **THEN** どれも判定は 0 でない終了コードで終わる

#### Scenario: 開発版の印は、配った印に数えない
- **GIVEN** 配った印 `v*` が 0 個で、開発版の印 `nightly` だけがある木
- **WHEN** 配った移行の検査と送信契約の版の検査を走らせる
- **THEN** どちらも配った印が 0 個として通る

### Requirement: 開発版は、幹にその日の変更があった日だけ、携帯の APK を含めて作り直す

THE SYSTEM SHALL 開発版の作り直しを、日本時間の毎日 0:00 に起こす。
WHEN 開発版の作り直しが起きる THE SYSTEM SHALL 印 `nightly` の commit と main の HEAD が同じなら何も組まずに終わり、違えば main の HEAD から開発版の 3 部品（サーバ・携帯の収集アプリ・PC の収集アプリ）を組んで開発版を作り直す。
THE SYSTEM SHALL 開発版の版番号を `<出所の版>-dev.<作り直した日の日本時間の YYYYMMDD>` にする。
WHILE 本人が開発版の公開の設定を入れていない THE SYSTEM SHALL 予定の実行で何も組まず、何も公開せず、印 `nightly` も動かさない（手で起こした作り直しだけが組み、そのときも公開しない）。
IF 出所の版の正式版の印 `v<出所の版>` が main の祖先にある THEN THE SYSTEM SHALL 開発版を組まずに終え、出所の版を次の版に上げる必要があることを示す（開発版が、出したばかりの正式版より古い版番号を名乗らないように。深掘り Q6 の推奨）。
THE SYSTEM SHALL 予定の実行が 1 回落ちても、次の回に `nightly` と main の HEAD の違いで同じ変更を拾う（日付の窓で「その日の変更」を決めない）。
IF 作り直しの判定を走らせる木が浅い（正式版の印 `v<出所の版>` が main の祖先かを決められない）THEN THE SYSTEM SHALL 判定を通さず、`build=` の行を出さず（何も組まず、何も公開しない）、出力に「履歴が浅い」を示す（祖先が見えないことを「印が無い」と読んで組まない）。
THE SYSTEM SHALL 開発版の携帯の APK を、組み立ての job では署名せずに組み、開発版の Environment の付いた公開の job の中でだけ開発版の鍵で署名する。

導出元: FR-107, FR-106。ST52 の深掘り Q5（推奨。開発版の公開は ST53 の後に本人が始める）・premises の C6・P7・P17・review/deep.md R16・D24（仮。review/spec.md R29）。ST54 の premises の C3・C4。

#### Scenario: 浅い木では、開発版の作り直しの判定は通らない
- **GIVEN** 開発版の公開の設定が入った配布元で、出所の版が `0.4.0` のまま main の祖先に印 `v0.4.0` があり、その後に main へ commit が足された木を、深さ 1 で clone した木
- **WHEN** 開発版の作り直しの判定を走らせる
- **THEN** 終了コードは 0 でなく、出力に「履歴が浅い」を含み、`build=` の行は無い

#### Scenario: nightly と main の HEAD が同じ日は、開発版を作らない
- **GIVEN** 開発版の公開の設定が入った配布元で、印 `nightly` が main の HEAD を指す木
- **WHEN** 開発版の作り直しの判定を走らせる
- **THEN** 判定は「組まない」を返す

#### Scenario: main が進んだ日は、開発版を作り直す
- **GIVEN** 開発版の公開の設定が入った配布元で、印 `nightly` の commit の後に、main に commit が 1 つ足された木
- **WHEN** 日本時間 2026-11-02 0:00 に開発版の作り直しの判定を走らせる
- **THEN** 判定は「組む」と、出所の版が `0.4.0` なら版番号 `0.4.0-dev.20261102` を返す

#### Scenario: 予定の実行が落ちた次の回に、同じ変更を拾う
- **GIVEN** 開発版の公開の設定が入った配布元で、印 `nightly` の後に main へ commit が足され、その日の作り直しが落ちて `nightly` が動いていない木
- **WHEN** 次の日に開発版の作り直しの判定を走らせる
- **THEN** 判定は「組む」を返す

#### Scenario: 印 nightly がまだ無ければ、作り直す
- **GIVEN** 開発版の公開の設定が入った配布元で、印 `nightly` が無い木
- **WHEN** 開発版の作り直しの判定を走らせる
- **THEN** 判定は「組む」を返す

#### Scenario: 公開の設定が入っていなければ、予定の実行は組まずに終える
- **GIVEN** 開発版の公開の設定が入っていない配布元で、印 `nightly` が無い木
- **WHEN** 予定の実行として開発版の作り直しの判定を走らせる
- **THEN** 判定は「組まない」と「公開しない」を返す

#### Scenario: 公開の設定が入っていなければ、手で起こした作り直しも公開しない
- **GIVEN** 開発版の公開の設定が入っていない配布元
- **WHEN** 手で起こした開発版の作り直しの判定を走らせる
- **THEN** 判定は「組む」と「公開しない」を返す

#### Scenario: 公開の設定が無ければ、印 nightly を動かす手順は走らない
- **WHEN** 開発版の作り直しの workflow を読む
- **THEN** 印 `nightly` を動かしリリースを入れ替える job は、公開の設定が `on` のときだけ走る条件を持つ

#### Scenario: 出所の版の正式版を出した後は、出所の版を上げるまで開発版を組まない
- **GIVEN** 開発版の公開の設定が入った配布元で、出所の版が `0.4.0` のまま、main の祖先に印 `v0.4.0` があり、その後に main へ commit が足された木
- **WHEN** 開発版の作り直しの判定を走らせる
- **THEN** 判定は「組まない」を返し、出所の版を上げる必要があることが示される

#### Scenario: 開発版の作り直しの workflow は、携帯の APK を組み、公開の job の中でだけ開発版の鍵で署名する
- **WHEN** 開発版の作り直しの workflow を読む
- **THEN** 携帯の APK を組む手順が組み立ての job にあり、APK に署名する手順は開発版の Environment の付いた公開の job にだけあり、その手順は開発版の鍵の指紋の期待値を開発版の Environment の変数から受け取る。どれかが欠ければ、workflow の検査は 0 でない終了コードで終わる

#### Scenario: 開発版の作り直しは、日本時間の 0:00 に起きる
- **WHEN** 開発版の作り直しの予定を読む
- **THEN** 予定は毎日 UTC 15:00（日本時間 0:00）

### Requirement: 開発版かどうかは、組み立てのときの版番号で決まる

THE SYSTEM SHALL 3 部品（サーバ・携帯の収集アプリ・PC の収集アプリ）のそれぞれが、自分が開発版か正式版か（以下「系統」）を、組み立てのときの版番号だけから決める。版番号が `-dev.` を含めば開発版、含まなければ正式版（ベータ版を含む）とする。
THE SYSTEM SHALL 版番号を渡さずに手で組んだ部品（版番号は出所の版のまま）を正式版として振る舞わせる。
THE SYSTEM SHALL 系統を、実行時の設定（環境変数・設定のファイル）で変える口を持たない。

導出元: FR-106。深掘り Q1（推奨: 組み立てのときの版番号で決める）。D1。

#### Scenario: 版番号に -dev. を持つ組み立ては開発版になる
- **WHEN** 版番号 `0.4.0-dev.20261102`・`0.4.0`・`0.4.0-beta.2` の系統を決める
- **THEN** 1 つ目は開発版、2 つ目と 3 つ目は正式版

#### Scenario: 版番号を渡さずに組んだ部品は正式版として振る舞う
- **GIVEN** 版番号を渡さずに組んだ PC の収集アプリ
- **WHEN** 自動起動の項目の名前と通知の差出人を読む
- **THEN** 自動起動の項目は `ashiato-collector.cmd`、通知の差出人は `dev.ashiato.collector`

#### Scenario: 系統の名前の環境変数を渡しても、系統は変わらない
- **GIVEN** 版番号 `0.4.0-dev.20261102` で組んだ PC の収集アプリとサーバ
- **WHEN** 環境変数 `ASHIATO_CHANNEL=official` を渡して、自動起動の項目の名前と、サーバの契約の問い合わせの見出し `Ashiato-Channel` を読む
- **THEN** 自動起動の項目は `ashiato-collector-dev.cmd`、見出しは `dev`

### Requirement: 携帯の開発版は、正式版と別のアプリとして並べて入り、名前で見分けられる

THE SYSTEM SHALL 携帯の収集アプリの開発版を、正式版と同じ端末に並べて入れられるようにし、開発版を入れても正式版のアプリと端末内の記録（未送信を含む）をそのまま残す。
THE SYSTEM SHALL 開発版のアプリの名前を「あしあと。開発版」にし、正式版の名前「あしあと。」を変えない。
THE SYSTEM SHALL 開発版に、正式版と同じ収集を全部持たせる（動かす収集を開発版だけ減らさない）。

導出元: FR-106・扉 34。深掘り Q7（推奨: 名前に「開発版」を付ける）・Q10（推奨: 全部を動かす）。premises の P1・P2・P3・C1。

#### Scenario: 開発版を入れても、正式版の端末内の記録は残る
- **GIVEN** 正式版の携帯の収集アプリが入り、端末内に未送信の記録を 1 件以上持つエミュレータ
- **WHEN** 同じエミュレータに開発版の携帯の収集アプリを入れる
- **THEN** 両方のアプリ（`dev.ashiato.collector` と `dev.ashiato.collector.dev`）が入っていて、正式版の未送信の件数は入れる前と同じ

#### Scenario: 開発版のアプリの名前は「あしあと。開発版」で、正式版は「あしあと。」のまま
- **WHEN** 版番号 `0.4.0-dev.20261102` と `0.4.0` で組み立てた携帯の APK のアプリの名前を読む
- **THEN** 開発版は「あしあと。開発版」、正式版は「あしあと。」

#### Scenario: 開発版の携帯の収集アプリは、正式版と同じ権限と常駐の口を持つ
- **WHEN** 版番号 `0.4.0-dev.20261102` と `0.4.0` で組み立てた携帯の APK の、求める権限・前景サービス・受け手の一覧を比べる
- **THEN** アプリ識別子から作られる名前を除いて、2 つの一覧は同じ

### Requirement: PC の収集アプリの開発版は、自動起動の項目・通知の差出人・実行ファイルの名前を正式版と分ける

WHERE PC の収集アプリが開発版である THE SYSTEM SHALL 自動起動の項目を `ashiato-collector-dev.cmd`、通知の差出人を `dev.ashiato.collector.dev`（表示名「ashiato 開発版」）にする。
WHERE PC の収集アプリが正式版である THE SYSTEM SHALL 今の名前（自動起動の項目 `ashiato-collector.cmd`、通知の差出人 `dev.ashiato.collector`、表示名「ashiato」）を変えない。
THE SYSTEM SHALL 開発版の PC の収集アプリの配布物の中の実行ファイルを `ashiato-collector-windows-dev.exe` にし、正式版の `ashiato-collector-windows.exe` と同じフォルダに置いても上書きしない。
THE SYSTEM SHALL 開発版の自動起動を仕込む操作で、正式版の自動起動の項目と通知の差出人の登録を書き換えない（逆も同じ）。

導出元: FR-106。深掘り Q1（推奨）・Q7（推奨）。premises の P4・C6。review/deep.md R10。深掘り第 2 回 Q14（本人の答え: 系統で選び、値はこのまま）。D1・D8。

#### Scenario: 開発版の PC の収集アプリは、開発版の自動起動の項目と差出人を使う
- **GIVEN** 版番号 `0.4.0-dev.20261102` で組んだ PC の収集アプリ
- **WHEN** 自動起動の項目の名前と、通知の差出人とその表示名を読む
- **THEN** 自動起動の項目は `ashiato-collector-dev.cmd`、通知の差出人は `dev.ashiato.collector.dev`、表示名は「ashiato 開発版」

#### Scenario: 同じ PC に並べると、正式版と開発版の自動起動の項目と差出人が両方残る
- **GIVEN** Windows の上で、正式版の自動起動を仕込み、正式版の差出人を登録した使い捨ての利用者の置き場
- **WHEN** 開発版の自動起動を仕込み、開発版の差出人を登録し、続けて正式版の自動起動をもう一度仕込む
- **THEN** スタートアップに `ashiato-collector.cmd` と `ashiato-collector-dev.cmd` が両方あり、開発版を仕込んだ後の `ashiato-collector.cmd` の中身は仕込む前と同じで、正式版を仕込み直した後の `ashiato-collector-dev.cmd` の中身も前と同じで、登録簿に `dev.ashiato.collector`（表示名「ashiato」）と `dev.ashiato.collector.dev`（表示名「ashiato 開発版」）が両方ある

#### Scenario: 開発版の PC の収集アプリの配布物の実行ファイルは、-dev の名前を持つ
- **WHEN** 版番号 `0.4.0-dev.20261102` と `0.4.0` で PC の収集アプリの配布物を組み、zip の中身を読む
- **THEN** 開発版の zip の実行ファイルは `ashiato-collector-windows-dev.exe`、正式版の zip の実行ファイルは `ashiato-collector-windows.exe`

### Requirement: PC の収集アプリの開発版は、置き場と端末識別子を自分で分け、除外の登録は渡された置き場のものを読む

WHERE PC の収集アプリが開発版である THE SYSTEM SHALL 渡された置き場（`ASHIATO_STATE_DIR`）の下の `dev` を自分の置き場（未送信・送った印・履歴の取得の印・二重起動の印）にし、端末識別子を、渡された値（`ASHIATO_DEVICE_ID`）の末尾に `-dev` を付けた値にする（渡された値がすでに `-dev` で終わるなら付け足さない）。
WHERE PC の収集アプリが開発版である THE SYSTEM SHALL 自動起動の項目に、渡された置き場と端末識別子を（`dev` を掘る前・`-dev` を付ける前の値のまま）書く（次のログオンで同じ規則がもう一度当たり、二重に掘らない）。
WHERE PC の収集アプリが開発版である THE SYSTEM SHALL 除外の登録を、前景の窓の記録でもブラウザの履歴の取り込みでも、自分の置き場ではなく渡された置き場の `exclusions.json` から読む（正式版と同じ置き場を渡せば、正式版で登録した除外がそのまま効く）。
THE SYSTEM SHALL 正式版の PC の収集アプリの置き場・端末識別子・除外の読み方を変えない。
THE SYSTEM SHALL 正式版と開発版に同じ置き場を渡しても、両方が起動して収集する（二重起動の止めで片方が黙って終わらない）。

導出元: FR-106・FR-83（除外）。深掘り Q2（推奨）。premises の P5。ST51 premises 9 の申し送り。review/deep.md R2・R3。D2。影響の判断 IMP-01（第 1 回の合意で「仕様を直す」。ブラウザの履歴の取り込みも同じ置き場から読む）。

#### Scenario: 開発版は、渡された置き場の下の dev を置き場にする
- **GIVEN** 版番号 `0.4.0-dev.20261102` の PC の収集アプリに、置き場 `<使い捨ての置き場>` と端末識別子 `pc-01` を渡す
- **WHEN** 設定を読む
- **THEN** 自分の置き場は `<使い捨ての置き場>/dev`、端末識別子は `pc-01-dev`

#### Scenario: 開発版の自動起動の項目は、渡された置き場と端末識別子をそのまま書く
- **GIVEN** 版番号 `0.4.0-dev.20261102` の PC の収集アプリに、置き場 `<使い捨ての置き場>` と端末識別子 `pc-01` を渡す
- **WHEN** 自動起動の項目の中身を作る
- **THEN** `ashiato-collector-dev.cmd` の中の置き場は `<使い捨ての置き場>`、端末識別子は `pc-01`（`dev` も `-dev` も付いていない）

#### Scenario: すでに -dev で終わる端末識別子には、重ねて付けない
- **GIVEN** 版番号 `0.4.0-dev.20261102` の PC の収集アプリに、端末識別子 `pc-01-dev` を渡す
- **WHEN** 設定を読む
- **THEN** 端末識別子は `pc-01-dev`

#### Scenario: 正式版は、渡された置き場と端末識別子をそのまま使う
- **GIVEN** 版番号 `0.4.0` の PC の収集アプリに、置き場 `<使い捨ての置き場>` と端末識別子 `pc-01` を渡す
- **WHEN** 設定を読む
- **THEN** 自分の置き場は `<使い捨ての置き場>`、端末識別子は `pc-01`

#### Scenario: 開発版は、渡された置き場の除外の登録を読む
- **GIVEN** 渡された置き場の `exclusions.json` に題名 `パスワード管理` の除外があり、`<置き場>/dev` には除外の登録が無い
- **WHEN** 開発版の PC の収集アプリが、題名 `パスワード管理` の前景の窓を記録する
- **THEN** その窓の本文（題名）は記録されない

#### Scenario: 開発版は、ブラウザの履歴の取り込みでも渡された置き場の除外の登録を読む
- **GIVEN** 渡された置き場の `exclusions.json` に URL `example.test/1` を含む訪問を除く登録があり、`<置き場>/dev` には除外の登録が無く、ブラウザの履歴に `example.test/1` と `example.test/2` の訪問がある
- **WHEN** 開発版の PC の収集アプリが、初めてブラウザの履歴を取り込んで送る
- **THEN** 送る本文に `example.test/1` の訪問は無く、`example.test/2` の訪問はある

#### Scenario: 正式版と同じ置き場を渡しても、開発版と正式版は両方起動する
- **GIVEN** 同じ置き場と端末識別子を渡した正式版と開発版の PC の収集アプリ
- **WHEN** 正式版を起動したまま開発版を起動する
- **THEN** 開発版は二重起動の止めに当たらず、二重起動の印は `<置き場>` と `<置き場>/dev` に 1 つずつあり、2 つの未送信の道は別

### Requirement: 収集アプリは、自分と同じ系統のサーバにだけ送る

THE SYSTEM SHALL サーバの契約の問い合わせ（`GET /contract`）の応答に、見出し `Ashiato-Channel` でサーバの系統（開発版は `dev`、それ以外（正式版・ベータ版）は `official`）を載せる。答えの本文（`{"contract_version": <整数>}`）は変えない。
WHEN 収集アプリが送る契機の頭で、契約の版を読めた答え（200 で版を読めた本文）を得る THE SYSTEM SHALL 応答の系統が自分の系統と違えば、その契機では記録・生存信号・破棄の報告・原本の欠け・原本のどれも送らず、未送信を手元に残し、送らなかった理由（相手の系統と自分の系統）をログに残す。
THE SYSTEM SHALL 系統を、契約の版を読めた答えの見出しからだけ読み、見出し `Ashiato-Channel` を持たない読めた答え（系統を答えない古いサーバ）を正式版のサーバの答えとみなす。404・200 で読めない本文・401 などの答えは系統の判定に使わず、今どおりに扱う（404 は「サーバを更新してください」、読めない 200 は 24 時間の待ち、401 などは一時的な失敗）。
THE SYSTEM SHALL 系統が違うことを、契約の版で止めていること（「サーバを更新してください」の通知の条件）に数えない。
WHILE 系統の違いで送らずにいる THE SYSTEM SHALL 本人に「送り先が開発版のサーバです。送り先を確かめてください」（相手が `dev`）か「送り先が正式版のサーバです。送り先を確かめてください」（相手が `official`）と知らせる。携帯は画面の行と通知、PC は通知で、どちらも状態に入った契機で 1 回、続く間は前に出してから 24 時間ごとに出し直す。携帯の上限の 7 日前の知らせにも同じ文を添える。前に出した時刻は手元に持ち、収集アプリを起こし直しても 24 時間経つまで出し直さない。系統の違いの知らせは「サーバを更新してください」の知らせと別に出し、片方を消してももう片方は消さない。
WHEN 系統の違いで送らずにいた収集アプリに、同じ系統のサーバが答える THE SYSTEM SHALL 手元に残した未送信を今どおり送り、系統の違いの知らせを消す。
THE SYSTEM SHALL 系統が同じサーバには、今どおり送る。系統の違うサーバからも、読む口（携帯と PC の途絶の判定の問い合わせ・携帯の気分の通知の判断の問い合わせ）は今どおり引く。
導出元: FR-106（★ 2026-11-03 を含む）・FR-30（収集した記録は書き換えない）。深掘り Q6（推奨のまま: 開発版のサーバにだけ送る）。深掘り第 2 回 Q12（本人の答え: どちらの向きも止める）・Q13（本人の答え: ST50 の知らせと同じ形で知らせる）。深掘り第 3 回 Q15（本人の答え: 系統の違うサーバからも読む口は止めない）。review/deep.md R1。D7・D12（仮）・D13・D14（仮）。
系統を本文の欄でなく見出しで運ぶのは、送信契約の問い合わせの答えの形（正典 record-envelope「S-01 は受け付ける送信契約の版を、問い合わせの口で答える」）を変えないため。

#### Scenario: 開発版のサーバは、契約の問い合わせに系統 dev を答える
- **GIVEN** 開発版として振る舞うサーバ
- **WHEN** API の合言葉で `GET /contract` を引く
- **THEN** 200 で、見出し `Ashiato-Channel` は `dev`、本文は `{"contract_version": <今と同じ値>}` だけ

#### Scenario: 正式版のサーバは、契約の問い合わせに系統 official を答える
- **GIVEN** 正式版として振る舞うサーバ
- **WHEN** API の合言葉で `GET /contract` を引く
- **THEN** 200 で、見出し `Ashiato-Channel` は `official`、本文は `{"contract_version": <今と同じ値>}` だけ

#### Scenario: 開発版の PC の収集アプリは、正式版のサーバに何も送らず手元に残す
- **GIVEN** 未送信に記録が 3 件ある開発版の PC の収集アプリと、契約の問い合わせに見出し `Ashiato-Channel: official` を付けて答える偽のサーバ
- **WHEN** 送る契機が 1 回来る
- **THEN** 偽のサーバに届いた要求は契約の問い合わせだけで、未送信は 3 件のまま、ログに相手の系統 `official` と自分の系統 `dev` がある

#### Scenario: 正式版の PC の収集アプリは、開発版のサーバに何も送らず手元に残す
- **GIVEN** 未送信に記録が 3 件ある正式版の PC の収集アプリと、契約の問い合わせに見出し `Ashiato-Channel: dev` を付けて答える偽のサーバ
- **WHEN** 送る契機が 1 回来る
- **THEN** 偽のサーバに届いた要求は契約の問い合わせだけで、未送信は 3 件のまま、ログに相手の系統 `dev` と自分の系統 `official` がある

#### Scenario: 開発版の PC の収集アプリは、開発版のサーバには今どおり送る
- **GIVEN** 未送信に記録が 3 件ある開発版の PC の収集アプリと、契約の問い合わせに見出し `Ashiato-Channel: dev` を付けて答える偽のサーバ
- **WHEN** 送る契機が 1 回来る
- **THEN** 3 件が取り込み口へ送られ、未送信は 0 件になる

#### Scenario: 系統を答えない古いサーバは、正式版のサーバとみなす
- **GIVEN** 契約の問い合わせに見出し `Ashiato-Channel` を付けず `{"contract_version":1}` だけを答える偽のサーバ
- **WHEN** 正式版と開発版の PC の収集アプリに、それぞれ送る契機が 1 回来る
- **THEN** 正式版は未送信を送り、開発版は何も送らずに手元に残す

#### Scenario: 系統が違って送らない間は、サーバを更新してくださいの通知を出さない
- **GIVEN** 開発版の PC の収集アプリと、見出し `Ashiato-Channel: official` を付けて答える偽のサーバ
- **WHEN** 送る契機が 3 日にわたって何度も来る
- **THEN** 「サーバを更新してください」の通知は出ず、契約の版で止めている状態にならない

#### Scenario: 開発版の携帯の収集アプリは、正式版のサーバに何も送らず手元に残す
- **GIVEN** 未送信に記録がある開発版の携帯の収集アプリと、契約の問い合わせに見出し `Ashiato-Channel: official` を付けて答える偽のサーバ
- **WHEN** 送る契機が 1 回来る
- **THEN** 記録・生存信号・破棄の報告・原本の欠け・原本のどれも送られず、未送信の件数は変わらず、ログに相手の系統 `official` と自分の系統 `dev` がある

#### Scenario: 正式版の携帯の収集アプリは、開発版のサーバに何も送らず手元に残す
- **GIVEN** 未送信に記録がある正式版の携帯の収集アプリと、契約の問い合わせに見出し `Ashiato-Channel: dev` を付けて答える偽のサーバ
- **WHEN** 送る契機が 1 回来る
- **THEN** 記録・生存信号・破棄の報告・原本の欠け・原本のどれも送られず、未送信の件数は変わらず、ログに相手の系統 `dev` と自分の系統 `official` がある

#### Scenario: 開発版の携帯の収集アプリは、開発版のサーバには今どおり送る
- **GIVEN** 未送信に記録がある開発版の携帯の収集アプリと、契約の問い合わせに見出し `Ashiato-Channel: dev` を付けて答える偽のサーバ
- **WHEN** 送る契機が 1 回来る
- **THEN** 未送信の記録が取り込み口へ送られる

#### Scenario: 系統が違って送らない間、PC の収集アプリは送り先を確かめるよう知らせる
- **GIVEN** 開発版の PC の収集アプリと、契約の問い合わせに見出し `Ashiato-Channel: official` を付けて答える偽のサーバと、偽の通知の口
- **WHEN** 送る契機が 1 回来る
- **THEN** 札 `channel` の通知が 1 回出て、文に「送り先が正式版のサーバです。送り先を確かめてください」があり、「サーバを更新してください」は無い

#### Scenario: 系統の違いの知らせは、続く間 24 時間ごとに出し直す
- **GIVEN** 系統の違いで送らずにいて、札 `channel` の通知を出した PC の収集アプリ
- **WHEN** 送る契機が 23 時間後と 24 時間後に来る（どちらも系統の違うサーバが答える）
- **THEN** 23 時間後には出し直さず、24 時間後に 1 回出し直す

#### Scenario: 送り先を直すと、PC の収集アプリは手元に残した記録を送り、系統の違いの知らせを消す
- **GIVEN** 系統の違いで送らずに未送信が 3 件あり、札 `channel` の通知を出した開発版の PC の収集アプリ
- **WHEN** 送り先を、見出し `Ashiato-Channel: dev` を付けて答える偽のサーバに替えて、送る契機が 1 回来る
- **THEN** 3 件が取り込み口へ送られて未送信は 0 件になり、札 `channel` の通知が消える

#### Scenario: 契約の問い合わせが 404 なら、系統の違いの知らせでなく更新を促す
- **GIVEN** 開発版の PC の収集アプリと、`GET /contract` に 404 を返す偽のサーバ
- **WHEN** 送る契機が 1 回来る
- **THEN** 札 `contract` の「サーバを更新してください」の通知が出て、札 `channel` の通知は出ない

#### Scenario: 契約の問い合わせが読めない 200 なら、系統の違いの知らせを出さない
- **GIVEN** 開発版の PC の収集アプリと、`GET /contract` に 200 で HTML の本文（見出し `Ashiato-Channel` なし）を返す偽のサーバ
- **WHEN** 送る契機が 1 回来る
- **THEN** 札 `channel` の通知も札 `contract` の通知も出ず、ログに `channel_mismatch` は無く、契約の版の待ちの理由に「読めない答え」が記録される

#### Scenario: 契約の問い合わせが 401 なら、系統の違いの知らせを出さない
- **GIVEN** 開発版の PC の収集アプリと、`GET /contract` に 401 を返す偽のサーバ
- **WHEN** 送る契機が 1 回来る
- **THEN** 札 `channel` の通知も札 `contract` の通知も出ず、ログに `channel_mismatch` は無く、契約の版の待ちの状態に入らない

#### Scenario: 系統の違いの知らせの時刻は、収集アプリを起こし直しても引き継ぐ
- **GIVEN** 系統の違いで送らずにいて、札 `channel` の通知を出した PC の収集アプリ
- **WHEN** 1 時間後に収集アプリを起こし直し、系統の違うサーバが答える送る契機が来る
- **THEN** 札 `channel` の通知は出し直さない

#### Scenario: 系統の違いの知らせを消しても、サーバを更新してくださいの知らせは消えない
- **GIVEN** 札 `contract` の「サーバを更新してください」の通知と、札 `channel` の系統の違いの通知の両方を出した PC の収集アプリ
- **WHEN** 系統の違いの状態だけが抜ける（同じ系統のサーバが答え、契約の版の待ちは続く）
- **THEN** 札 `channel` の通知は消え、札 `contract` の通知は残る

#### Scenario: 系統が違って送らない間、携帯の収集アプリは画面の行と通知で送り先を確かめるよう知らせる
- **GIVEN** 正式版の携帯の収集アプリと、契約の問い合わせに見出し `Ashiato-Channel: dev` を付けて答える偽のサーバ
- **WHEN** 送る契機が 1 回来る
- **THEN** 画面の行と通知に「送り先が開発版のサーバです。送り先を確かめてください」が出て、「サーバを更新してください」の通知は出ない

#### Scenario: 送り先を直すと、携帯の収集アプリは手元に残した記録を送り、系統の違いの知らせを消す
- **GIVEN** 系統の違いで送らずに未送信が残り、系統の違いの通知を出した正式版の携帯の収集アプリ
- **WHEN** 見出し `Ashiato-Channel: official` を付けて答える偽のサーバに替えて、送る契機が 1 回来る
- **THEN** 未送信の記録が取り込み口へ送られ、系統の違いの通知と画面の行が消える

#### Scenario: 系統の違いの知らせは、携帯でも続く間 24 時間ごとに出し直す
- **GIVEN** 系統の違いで送らずにいて、系統の違いの通知を出した携帯の収集アプリ
- **WHEN** 送る契機が 23 時間後と 24 時間後に来る（どちらも系統の違うサーバが答える）
- **THEN** 23 時間後には出し直さず、24 時間後に 1 回出し直す

#### Scenario: 契約の問い合わせが 404 なら、携帯の収集アプリは系統の違いの知らせでなく更新を促す
- **GIVEN** 開発版の携帯の収集アプリと、`GET /contract` に 404 を返す偽のサーバ
- **WHEN** 送る契機が 1 回来る
- **THEN** 系統の違いの通知と画面の行は出ず、`Log.w` に相手の系統は無く、「サーバを更新してください」の通知が出る

#### Scenario: 契約の問い合わせが読めない 200 なら、携帯の収集アプリは系統の違いの知らせを出さない
- **GIVEN** 開発版の携帯の収集アプリと、`GET /contract` に 200 で HTML の本文（見出し `Ashiato-Channel` なし）を返す偽のサーバ
- **WHEN** 送る契機が 1 回来る
- **THEN** 系統の違いの通知と画面の行も「サーバを更新してください」の通知も出ず、`Log.w` に相手の系統は無く、契約の版の待ちの理由に「読めない答え」が記録される

#### Scenario: 携帯の上限の 7 日前の知らせに、系統の違いの案内を添える
- **GIVEN** 開発版の携帯の収集アプリと、見出し `Ashiato-Channel: official` で答える偽のサーバで、系統の違いで送らずにいて、最も古い未送信を積んでから 83 日経った
- **WHEN** 上限の 7 日前の知らせが出る
- **THEN** 知らせの文に「送り先が正式版のサーバです。送り先を確かめてください」が添えられる

#### Scenario: 系統の違うサーバからも、携帯の収集アプリは途絶の判定を今どおり引く
- **GIVEN** 送る契機で、契約の問い合わせに見出し `Ashiato-Channel: official` を付けて答える偽のサーバから系統の違いを知り、送らずにいる（系統の違いの知らせを出した）開発版の携帯の収集アプリ
- **WHEN** 途絶の判定を問い合わせる契機が来る
- **THEN** 同じ偽のサーバに `GET /outages` が引かれる

#### Scenario: 系統の違うサーバからも、PC の収集アプリは途絶の判定を今どおり引く
- **GIVEN** 送る契機で、契約の問い合わせに見出し `Ashiato-Channel: official` を付けて答える偽のサーバから系統の違いを知り、送らずにいる（系統の違いの知らせを出した）開発版の PC の収集アプリ
- **WHEN** 途絶の判定を問い合わせる契機が来る
- **THEN** 同じ偽のサーバに `GET /outages` が引かれる

### Requirement: DB は最初に移行を当てた系統のもので、違う系統のサーバと migrate は何も書かずに止まる

THE SYSTEM SHALL D-01 の系統を、版の記録のうち最初に当てた行（当てた時刻が最も古い行。同じ時刻なら名前の順で最初）の組み立ての版番号で決める（`-dev.` を含めば開発版の DB、含まなければ正式版の DB）。
THE SYSTEM SHALL 版の記録の schema が無い D-01 のうち、ashiato の schema（`core`）を持つもの（版の記録より前の D-01）を正式版の D-01 とみなし、`core` を持たない空の D-01 と、版の記録の行が 1 つも無い D-01（最初の移行が落ちた D-01）を系統の無い D-01 とする。
IF サーバか `migrate` が、自分と違う系統の D-01 に繋がる THEN THE SYSTEM SHALL 退避も版の適用も付与も更新の履歴の行も書かずに、終了コード 4 で終わり、標準エラーの 1 行目に `error: kind=channel_mismatch`、続く行に D-01 の系統と自分の系統を示す。
THE SYSTEM SHALL 系統の検査を、版の食い違いの検査（知らない版・まだ当たっていない版）より先に、サーバでは待ち受けを開く前に行う。
WHEN 系統の無い D-01 に `migrate` が版を当てる THE SYSTEM SHALL その D-01 を、当てた組み立ての系統の D-01 にする（版の記録の行が当てた組み立ての版番号を持つことによる。別の印の表を足さない）。
THE SYSTEM SHALL 系統の検査を外す設定を持たない。

導出元: FR-106（開発版のサーバは正式版と別の D-01 に繋ぐ）・FR-117。深掘り Q3（推奨: DB に系統の印を残し、違う側は何も書かずに止まる）。premises の P6・P13。review/deep.md R12。D3。

#### Scenario: 開発版の migrate は、正式版の DB に 1 つも当てずに止まる
- **GIVEN** 正式版の組み立てで全部の版を当てた DB
- **WHEN** 開発版の組み立てで `ashiato-server migrate` を走らせる
- **THEN** 終了コードは 4 で、標準エラーの 1 行目は `error: kind=channel_mismatch`、続く行に DB の系統 `official` と自分の系統 `dev` があり、版の記録・更新の履歴の行の数は走らせる前と同じで、退避の置き場にファイルは増えない

#### Scenario: 開発版のサーバは、正式版の DB では起動しない
- **GIVEN** 正式版の組み立てで全部の版を当てた DB
- **WHEN** 開発版の組み立てでサーバを起動する
- **THEN** 待ち受けを開かずに終了コード 4 で終わり、標準エラーの 1 行目は `error: kind=channel_mismatch`

#### Scenario: 正式版の migrate とサーバは、開発版の DB では止まる
- **GIVEN** 空の DB に開発版の組み立てで全部の版を当てた DB
- **WHEN** 正式版の組み立てで `ashiato-server migrate` を走らせ、別の回にサーバを起動する
- **THEN** どちらも終了コード 4 で終わり、標準エラーの 1 行目は `error: kind=channel_mismatch`、版の記録の行の数は変わらない

#### Scenario: 空の DB に開発版の migrate を当てると、開発版の DB になる
- **GIVEN** 表を 1 つも持たない空の DB
- **WHEN** 開発版の組み立てで `ashiato-server migrate` を走らせ、続けて開発版の組み立てでサーバを起動する
- **THEN** migrate は終了コード 0 で、版の記録の全部の行の組み立ての版番号が `-dev.` を含み、サーバの `/healthz` が 200 を返す

#### Scenario: 版の記録の無い古い DB には、開発版の migrate は当てずに止まる
- **GIVEN** 版の記録を持たない組み立てで全部の版を当てた DB（版の記録の schema が無く、ashiato の表はある）
- **WHEN** 開発版の組み立てで `ashiato-server migrate` を走らせる
- **THEN** 終了コードは 4 で、標準エラーの 1 行目は `error: kind=channel_mismatch`、版の記録の schema は作られない

#### Scenario: 版の記録の行が 1 つも無い DB には、どちらの系統の migrate も当てる
- **GIVEN** 版の記録の schema はあるが行が 1 つも無い DB
- **WHEN** 開発版の組み立てで `ashiato-server migrate` を走らせる
- **THEN** 系統の食い違いで止まらず（終了コードは 4 でない）、当てた版の行の組み立ての版番号は `-dev.` を含む

#### Scenario: 系統が同じなら、今どおり当てて起動する
- **GIVEN** 正式版の組み立てで全部の版を当てた DB
- **WHEN** 同じ版の正式版の組み立てで `ashiato-server migrate` を走らせ、サーバを起動する
- **THEN** migrate は終了コード 0、サーバの `/healthz` は 200

#### Scenario: 系統が違い、知らない版もある DB では、系統の食い違いを示す
- **GIVEN** 正式版の組み立てで全部の版を当て、版の記録に開発版の知らない版の行を 1 つ足した DB
- **WHEN** 開発版の組み立てでサーバを起動する
- **THEN** 終了コードは 4（3 でない）で、標準エラーの 1 行目は `error: kind=channel_mismatch`

### Requirement: 移行の名前を変えた・消したときは、旧名の対応を残せば、旧名で当てた DB でもサーバが動く

THE SYSTEM SHALL 移行の旧名の対応を `migrations/LINEAGE` に置き、1 行に `renamed <旧名> <今の名前>`・`removed <旧名>`・`rewritten <名前> <直しの移行の名前>` のどれか 1 つを書く（`#` で始まる行と空の行は読まない）。
THE SYSTEM SHALL サーバと `migrate` が版の記録と自分の知る版を比べるときに、`renamed` の旧名の行を今の名前の版が当たっているものとして読み（今の名前の版をもう一度当てない）、`removed` の旧名の行を知っている版として読む（知らない版として止まらない）。
THE SYSTEM SHALL 旧名の対応が無い名前は今どおり知らない版として扱い、止まるときにその名前と、その行を当てた組み立ての版番号と commit を示す（今の `error: kind=db_ahead` の形）。
THE SYSTEM SHALL `migrations/LINEAGE` の形が決めた形でない行・`renamed` の今の名前と `rewritten` の 2 つの名前が `migrations/ORDER` に無い行・`rewritten` の直しの移行が `ORDER` で直す移行より前にある行を、移行の形の検査で落とす。
THE SYSTEM SHALL 旧名の対応を読むことで、版の記録の行を書き換えない・消さない。

導出元: FR-106・FR-117。深掘り Q4（推奨: 止まったら原因を示し、DB は書き換えない。main 側で直す）。premises の P6・P7・P8。ST52 review/deep.md R11 の申し送り。D4。

#### Scenario: 名前を変えた移行は、旧名の対応があれば、旧名で当てた DB でも起動し、当て直さない
- **GIVEN** 版 `A` を当てた DB と、`A` を `B` へ名前だけ変え、`migrations/LINEAGE` に `renamed A B` を書いた組み立て
- **WHEN** その組み立てで `ashiato-server migrate` を走らせ、サーバを起動する
- **THEN** migrate は `B` を当てずに終了コード 0 で終わり、版の記録の行の数は変わらず（`A` の行はそのまま）、サーバの `/healthz` は 200

#### Scenario: 消した移行は、消したことの対応があれば、それを当てた DB でも起動する
- **GIVEN** 版 `A` を当てた DB と、`A` を `migrations/ORDER` から消し、`migrations/LINEAGE` に `removed A` を書いた組み立て
- **WHEN** その組み立てでサーバを起動する
- **THEN** `/healthz` は 200 で、版の記録の `A` の行は残っている

#### Scenario: 旧名の対応が無ければ、今どおり知らない版で止まり、その名前と当てた組み立てを示す
- **GIVEN** 版 `A` を当てた DB と、`A` を `B` へ名前を変え、`migrations/LINEAGE` に何も書かない組み立て
- **WHEN** その組み立てでサーバを起動する
- **THEN** 終了コードは 3 で、標準エラーの 1 行目は `error: kind=db_ahead`、続く行に `  db: A (<その行の版番号> <その行の commit>)` がある

#### Scenario: 旧名の対応の形が違えば、移行の形の検査が落ちる
- **GIVEN** `migrations/LINEAGE` に `moved A B`・`renamed A`・`renamed A <ORDER に無い名前>`・`rewritten C D`（`D` が `ORDER` で `C` より前）のどれか 1 行を書いた木
- **WHEN** 移行の形の検査を走らせる
- **THEN** どの場合も検査は 0 でない終了コードで終わり、その行が示される

### Requirement: 開発版に載った移行を、旧名の対応なしに変えた変更は CI で落ちる

THE SYSTEM SHALL 印 `nightly` の版にある移行について、`migrations/ORDER` から消すこと・名前を変えることを、`migrations/LINEAGE` に `removed` か `renamed` の対応が無ければ、CI の検査で落とす。
THE SYSTEM SHALL `renamed <旧名> <今の名前>` で名前を変えた移行は、今の名前の中身を印 `nightly` の版の旧名の中身と比べる（名前を変えて中身も変えたなら `rewritten` も求める）。
IF 検査する木が浅い（履歴が途中で切れている）THEN THE SYSTEM SHALL 印を見る前に検査を通さず、履歴が浅いことを示す。
THE SYSTEM SHALL 印 `nightly` の版にある移行の中身（前進側の `.sql`）を変えることを、`migrations/LINEAGE` に、その移行を直す `rewritten <名前> <直しの移行の名前>` が無ければ、CI の検査で落とす。
THE SYSTEM SHALL 印 `nightly` の後に足した移行と、戻し手順（`.down.sql`）の変更は落とさない。
WHILE 印 `nightly` が無い THE SYSTEM SHALL 何も見ずに検査を通す。
IF 印 `nightly` はあるが、その版の移行を取り出せない THEN THE SYSTEM SHALL 検査を通さない。
THE SYSTEM SHALL この検査を、配った印 `v*` の凍結（「配った移行は、名前も中身も変えられない」）と別に持ち、配った印の検査が数える印に `nightly` を足さない。

導出元: FR-106。深掘り Q4（推奨: main 側で直す）。深掘り第 2 回 Q11（本人の答え: 動く印 nightly 1 つを基準に比べる。Q5 の答えに乗る）。premises の P7・P8・P9。ST52 Q10（本人の答え: 開発版の印は配った印に数えない）。D5。

#### Scenario: 開発版に載った移行の名前を変えて旧名の対応が無ければ、検査が落ちる
- **GIVEN** 印 `nightly` の版に移行 `A` があり、今の木で `A` を `B` へ名前を変え、`migrations/LINEAGE` に何も書かない木
- **WHEN** 開発版に載った移行の検査を走らせる
- **THEN** 検査は 0 でない終了コードで終わり、`A` の名前が示される

#### Scenario: 開発版に載った移行の名前を変えても、旧名の対応があれば通る
- **GIVEN** 上の木の `migrations/LINEAGE` に `renamed A B` を書いた木
- **WHEN** 開発版に載った移行の検査を走らせる
- **THEN** 検査の終了コードは 0

#### Scenario: 名前を変えて中身も変えた移行は、旧名の対応だけでは落ちる
- **GIVEN** 印 `nightly` の版に移行 `A` があり、今の木で `A` を `B` へ名前を変えて中身も変え、`migrations/LINEAGE` に `renamed A B` だけを書いた木
- **WHEN** 開発版に載った移行の検査を走らせる
- **THEN** 検査は 0 でない終了コードで終わり、`B` の名前と、直しの移行を足して `rewritten` を書くことが示される

#### Scenario: 浅い木では、開発版に載った移行の検査は通らない
- **GIVEN** 印 `nightly` のある木を深さ 1 で clone した木
- **WHEN** 開発版に載った移行の検査を走らせる
- **THEN** 検査は 0 でない終了コードで終わり、出力に「履歴が浅い」を含む

#### Scenario: 開発版に載った移行の中身を変えて直しの対応が無ければ、検査が落ちる
- **GIVEN** 印 `nightly` の版に移行 `A` があり、今の木で `A` の中身を変え、`migrations/LINEAGE` に何も書かない木
- **WHEN** 開発版に載った移行の検査を走らせる
- **THEN** 検査は 0 でない終了コードで終わり、`A` の名前と、直しの移行を足して `rewritten` を書くことが示される

#### Scenario: 開発版に載った移行の中身を変えても、直しの移行の対応があれば通る
- **GIVEN** 上の木に直しの移行 `C` を `ORDER` の末尾に足し、`migrations/LINEAGE` に `rewritten A C` を書いた木
- **WHEN** 開発版に載った移行の検査を走らせる
- **THEN** 検査の終了コードは 0

#### Scenario: 印 nightly が無ければ、開発版に載った移行の検査は通る
- **GIVEN** 印 `nightly` が無い木
- **WHEN** 開発版に載った移行の検査を走らせる
- **THEN** 検査の終了コードは 0

#### Scenario: 印 nightly の後に足した移行は、変えても通る
- **GIVEN** 印 `nightly` の後に移行 `N` を足し、さらに `N` の中身を変えた木
- **WHEN** 開発版に載った移行の検査を走らせる
- **THEN** 検査の終了コードは 0

#### Scenario: 開発版に載った移行の戻し手順は、変えても通る
- **GIVEN** 印 `nightly` の版にある移行 `A` の `.down.sql` だけを変えた木
- **WHEN** 開発版に載った移行の検査を走らせる
- **THEN** 検査の終了コードは 0

#### Scenario: 印 nightly の版の移行を取り出せなければ、検査は通らない
- **GIVEN** 印 `nightly` が、`migrations/ORDER` を持たない commit を指す木
- **WHEN** 開発版に載った移行の検査を走らせる
- **THEN** 検査は 0 でない終了コードで終わる

### Requirement: 開発版のサーバは、正式版と取り合わない既定の待ち受け・ログインの名前・退避の置き場を持つ

WHERE サーバが開発版である THE SYSTEM SHALL 既定の待ち受けを `127.0.0.1:18788`、画面のログインのクッキーの名前を `ashiato_session_dev`、移行の前の退避の既定の置き場を `backups/pre-migrate-dev` にする。
WHERE サーバが正式版である THE SYSTEM SHALL 今の既定（待ち受け `127.0.0.1:18787`・クッキー `ashiato_session`・退避 `backups/pre-migrate`）を変えない。
THE SYSTEM SHALL 待ち受けと退避の置き場を明示の設定（`BIND`・`MIGRATE_BACKUP_DIR`）で渡したときは、系統に依らずその値を使う。

導出元: FR-106。深掘り第 2 回 Q14（本人の答え: 系統（D1 の組み立てのときの版番号）で選び、値はこのまま）。premises の C2・C6・C7・C8・P12・P17。review/deep.md R10。D8。

#### Scenario: 開発版のサーバの既定は、正式版と別の待ち受け・クッキー・退避の置き場
- **WHEN** 開発版として振る舞うサーバの、既定の待ち受け・画面のログインで返すクッキーの名前・移行の前の退避の既定の置き場を読む
- **THEN** 待ち受けは `127.0.0.1:18788`、クッキーは `ashiato_session_dev`、退避の置き場は `backups/pre-migrate-dev`

#### Scenario: 正式版のサーバの既定は、今のまま
- **WHEN** 正式版として振る舞うサーバの、既定の待ち受け・画面のログインで返すクッキーの名前・移行の前の退避の既定の置き場を読む
- **THEN** 待ち受けは `127.0.0.1:18787`、クッキーは `ashiato_session`、退避の置き場は `backups/pre-migrate`

#### Scenario: 開発版のサーバの画面のログインは、開発版の名前のクッキーだけを使う
- **GIVEN** 開発版として振る舞うサーバに画面のログインをして得た値
- **WHEN** その値を `ashiato_session_dev` の名前で付けて、また別の回に `ashiato_session` の名前で付けて、ログインの要る画面の API を引く
- **THEN** `ashiato_session_dev` では 200、`ashiato_session` では 401。ログインの応答の `Set-Cookie` は `ashiato_session_dev` だけを書き、`ashiato_session` を書かない

#### Scenario: 待ち受けと退避の置き場を明示で渡せば、開発版でもその値を使う
- **GIVEN** 開発版として振る舞うサーバに、`BIND=127.0.0.1:19999` と `MIGRATE_BACKUP_DIR=<使い捨ての置き場>` を渡す
- **WHEN** 待ち受けと移行の前の退避の置き場を読む
- **THEN** 待ち受けは `127.0.0.1:19999`、退避の置き場は `<使い捨ての置き場>`

### Requirement: 開発版のリリースには、サポートしないことを説明文の先頭に書く

THE SYSTEM SHALL 開発版を公開するたびに、開発版のリリースの説明文を、リポジトリに置いた決まった文で書き直す。決まった文の 1 行目は「この開発版はサポートしません。」で始まる（2 行目以降は文書として直してよい）。
IF 書き直した後に読んだ説明文の 1 行目が「この開発版はサポートしません。」で始まらない THEN THE SYSTEM SHALL 公開の手順を 0 でない終了コードで終える。
THE SYSTEM SHALL 印 `nightly` を動かす手順を、説明文を書き直して確かめた後に置く。

導出元: FR-106（サポートしないことを配布元に明記する）。深掘り Q8（推奨: 説明文の先頭に書き、公開のたびに書き直す）。premises の P11。D9。

#### Scenario: 開発版を公開するたびに、説明文は決まった文で書き直される
- **GIVEN** 説明文が「開発版（main の毎日の組み立て）。携帯の APK は ST54 まで含まない」の開発版のリリースがある配布元を模した偽の `gh`
- **WHEN** 開発版の公開の手順を走らせる
- **THEN** リリースの説明文を決まった文のファイルで書き直す要求が出て、書き直した後の説明文の 1 行目は「この開発版はサポートしません。」で始まる

#### Scenario: 説明文の先頭にサポートしない旨が無ければ、公開の手順は落ちる
- **GIVEN** 説明文を書き直す要求を受けても、説明文を前のまま返す偽の `gh`
- **WHEN** 開発版の公開の手順を走らせる
- **THEN** 手順は 0 でない終了コードで終わり、印 `nightly` を動かす要求は出ない

### Requirement: サーバのインストールは、管理者の確認を 1 回だけ出し、同梱の PostgreSQL とサーバを自動で起きるサービスにする

WHEN 利用者がサーバの Windows の配布物を展開してインストーラ（`ashiato-setup.exe`）を開く THE SYSTEM SHALL 管理者の確認（UAC）を 1 回だけ出し、それ以上の確認を出さずにインストールを終える。
THE SYSTEM SHALL プログラム（サーバ・更新係・インストーラ・同梱の PostgreSQL）をプログラムの置き場（`C:\Program Files\ashiato`）に、状態をすべて状態の置き場（`C:\ProgramData\ashiato`）に置く。
THE SYSTEM SHALL 同梱の PostgreSQL 17 を DB のサービス（`ashiato-db`）として、サーバをサーバのサービス（`ashiato-server`）として登録し、どちらの開始の種類も「自動」にする。
THE SYSTEM SHALL 更新係（`ashiato-updater.exe`）を更新係のサービス（`ashiato-updater`）として、サービス専用のアカウント（`NT SERVICE\ashiato-updater`）で、開始の種類を「自動」にして登録する（DB とサーバのサービスには依らせない）。
THE SYSTEM SHALL 2 つのサービスを、利用者でなく、合言葉を持たないサービス専用のアカウント（`NT SERVICE\ashiato-db`・`NT SERVICE\ashiato-server`）で動かし、利用者がログオンしていなくても動かす。
IF サービス専用のアカウントでサービス（更新係を含む）を登録できない THEN THE SYSTEM SHALL ほかのサービスと共有のアカウント（`LocalSystem`・`NetworkService`・`LocalService`）へ倒さず、登録できなかったことを示して止まる。
THE SYSTEM SHALL サーバのサービスを DB のサービスに依らせ（DB が起きてから起きる）、どちらも異常に終わったときは Windows のサービスの回復で起こし直す。
THE SYSTEM SHALL インストールで PC の収集アプリを入れず、Docker Desktop・WSL・ほかの実行環境を求めない。
THE SYSTEM SHALL インストーラの段（インストーラが起こす命令と、更新係のサービスを開始する前までのサーバと DB）は、ネットワークに出ない。開始した後の更新係の問い合わせは、この範囲の外とする。
WHEN インストールが最後に達する THE SYSTEM SHALL 「更新の確認: 有効（切るには `ashiato-setup.exe update-check off`）」を示し、その後に更新係のサービスを開始する（更新係の最初の問い合わせは、示した後にだけ出る）。
THE SYSTEM SHALL 更新係のサービスが開始できない・配布元に届かないことで、インストールを失敗にしない。

導出元: FR-108。深掘り Q1（推奨: サーバとデータベースだけ）・Q2（推奨: `ashiato-setup.exe` をサーバの zip に入れる）/ C2 / C11 / C12 / review/deep.md R8。
サービスの名前と置き場の並びは更新係（ST57）と更新（ST58）が乗る（design D14）。
★ ST57: 3 つめのサービス（更新係）を足した（FR-112・深掘り Q1・Q3・聞かない既定 C5・C6）。更新係の置き場と権限は「サーバの更新係は、DB にも状態の置き場にも触れない…」が持つ。共有のアカウントへ倒すと、同じアカウントで動くほかのサービスが状態の置き場を読める（FR-109 が崩れる。design D3（仮））。

#### Scenario: 配る zip からのインストールで、2 つのサービスが自動で起きる形で入る

- **WHEN** サーバの Windows の配布物を展開してインストーラを管理者の権限で走らせ、最後に画面の合言葉を決める
- **THEN** サービス `ashiato-db` と `ashiato-server` が登録され、どちらも開始の種類が「自動」で実行中であり、`ashiato-server` は `ashiato-db` に依る。サービス `ashiato-updater` も開始の種類が「自動」で登録されている

#### Scenario: インストールは Docker も WSL も使わない

- **WHEN** インストーラが起こした命令の一覧（試験の組み立ての記録）を、インストールの始めから終わりまで読む
- **THEN** 一覧に `docker`・`wsl`・`wsl.exe` の命令が無い

#### Scenario: 上がっていない状態から開くと、管理者に上がる起こし直しは 1 回だけ

- **WHEN** 管理者に上がっていない状態のインストーラが、インストールのために組む命令を読む
- **THEN** 自分を管理者として起こし直す命令はちょうど 1 つで、上がった後の段は起こし直しの命令を組まない

#### Scenario: サービスは利用者でなくサービス専用のアカウントで動く

- **WHEN** インストールした後に、2 つのサービスが動いているアカウントを読む
- **THEN** `ashiato-db` は `NT SERVICE\ashiato-db`、`ashiato-server` は `NT SERVICE\ashiato-server` で動き、インストールした利用者のアカウントでも `LocalSystem` でもない

#### Scenario: サービス専用のアカウントで登録できなければ、共有のアカウントへ倒さず止まる

- **WHEN** サービス専用のアカウントでの登録が断られる状態（試験の組み立ての差し込みで登録を失敗させる）でインストーラを走らせる
- **THEN** インストーラは rc≠0 で登録できなかったことを示して止まり、`LocalSystem`・`NetworkService`・`LocalService` で動くサービスは 1 つも登録されていない

#### Scenario: サービスを止めて起こし直すと、サーバは DB の後に起きて答える

- **WHEN** インストールした後に、2 つのサービスを止め、`ashiato-server` だけを開始する
- **THEN** `ashiato-db` が先に起き、`ashiato-server` が起きて `127.0.0.1:18700` の `/healthz` に 200 で答える

#### Scenario: サーバのサービスが異常に終わると起こし直される

- **WHEN** 動いている `ashiato-server` のプロセスを外から終わらせる
- **THEN** 1 分以内に `ashiato-server` がもう一度実行中になり、`/healthz` に 200 で答える

#### Scenario: DB のサービスが異常に終わると起こし直される

- **WHEN** 動いている `ashiato-db` の PostgreSQL の主のプロセスを外から終わらせる
- **THEN** 1 分以内に `ashiato-db` がもう一度実行中になり、`127.0.0.1:55400` が接続を受ける

#### Scenario: インストールの間、ネットワークに出ない

- **WHEN** 外への接続を拒む設定（試験の網の制限）でインストーラを走らせる
- **THEN** インストールは最後まで終わり、2 つのサービスが実行中である

#### Scenario: インストールの最後に更新の確認が有効なことと切り方を示し、その後に更新係が起きる

- **WHEN** インストールを終え、出力と、インストーラが起こした命令の一覧（試験の組み立ての記録）を読む
- **THEN** 出力に「更新の確認: 有効」と `ashiato-setup.exe update-check off` があり、命令の一覧で更新係のサービスを開始する命令は、その表示より後にある

#### Scenario: 配布元に届かなくてもインストールは終わる

- **WHEN** 更新係から外への接続を拒む設定（試験の網の制限）でインストーラを走らせる
- **THEN** インストールは最後まで終わり、サービス `ashiato-db` と `ashiato-server` は実行中で、更新係の確かめた記録に「配布元に届かない」の行がある

### Requirement: サーバは Windows のサービスとして動き、設定と秘密を状態の置き場のファイルから読む

THE SYSTEM SHALL サーバの実行ファイルに、Windows のサービスとして動く口（`ashiato-server service`）を持たせ、サービスの開始と停止の知らせを受ける。
WHEN サービスの停止を知らされる THE SYSTEM SHALL 新しい求めの受け付けを止め、今の取り込みと書き出しを書き終えるか打ち切りの印を残してから終わる。
WHILE サービスとして動く THE SYSTEM SHALL 設定と秘密を、状態の置き場の設定のファイル（`settings\server.json`）から読み、サービスの登録簿（環境変数）から秘密を読まない。
THE SYSTEM SHALL 表の所有者（移行を当て、削除の門のトリガを外せる役割）の合言葉を `settings\server.json` に置かず、親から受け継がず管理者と SYSTEM だけが読み書きできる `settings\owner.json` に置き、サーバのサービスのプロセスに渡さない。移行の段・`repair` はそこから読む。
THE SYSTEM SHALL インストーラが、サービスの登録簿（サービスの環境変数と起動の引数）に秘密を書かない。
IF 設定のファイルが無い・読めない・形が違う THEN THE SYSTEM SHALL サーバを起動せず、どれに当たるかを理由にログに書いて終わり、秘密の値をログに出さない。
WHILE サービスとして起動する THE SYSTEM SHALL DB が接続を受けるまで 120 秒まで待ってから起動の検査（DB の版・役割・合言葉）に入り、待つ間は起動を失敗にしない。
IF 120 秒待っても DB に繋げない THEN THE SYSTEM SHALL DB に繋げないことを理由にログに書いて終わる（サービスの回復が起こし直す）。
THE SYSTEM SHALL サービスとして動かない（開発の台本で起こす）ときの、環境変数から設定を読む今の振る舞いを変えない。

導出元: FR-108・FR-109・FR-110。C2 / C4 / C11 / review/deep.md R8・deep.md Q11（review/code.md R48）。
サービスの登録簿は同じ PC のほかのアカウントが読める（premises P7）ので、秘密を置かない。サーバは起動の時に DB へ 1 度だけ繋いで繋げなければ終わる（`lib.rs`）ので、サービスでは DB の起動を待つ。

#### Scenario: サービスの登録簿に秘密が無い

- **WHEN** インストールした後に、2 つのサービスの登録簿の鍵（`HKLM\SYSTEM\CurrentControlSet\Services\ashiato-*`）の値をすべて読む
- **THEN** どの値にも、DB の合言葉・API の合言葉・ログインの印の鍵・画面の合言葉の照らすための値が含まれない

#### Scenario: 表の所有者の合言葉は、サーバのサービスが読めないファイルにだけある

- **WHEN** インストールした後に、`settings\server.json` の鍵と、`settings\owner.json` の中身と権限（ACL）を読み、別の利用者として `owner.json` を開く
- **THEN** `server.json` に表の所有者の接続（`DATABASE_OWNER_URL`）が無く、`owner.json` にだけあり、`owner.json` は親から受け継がず、権限の主体は `Administrators` と `SYSTEM` だけで、別の利用者は開けない

#### Scenario: 設定のファイルが無いと、理由を書いて起動しない

- **WHEN** 設定のファイルを退けて `ashiato-server` を開始する
- **THEN** サービスは実行中にならず、状態の置き場のサーバのログに設定のファイルが無いことが書かれ、ログのどこにも秘密の値が無い

#### Scenario: DB の起動が遅れても、サーバのサービスは待ってから起きる

- **WHEN** DB が接続を受け始めるまでの間（試験の組み立ての差し込みで 20 秒遅らせる）に `ashiato-server` を開始する
- **THEN** サーバは異常に終わらず、DB が接続を受けた後に `/healthz` に 200 で答える

#### Scenario: DB が 120 秒開かなければ、理由を書いて終わる

- **WHEN** DB が接続を受けない状態（試験の組み立ての差し込みで、DB の番号を開かない）で `ashiato-server` を開始する
- **THEN** 120 秒を過ぎた後にサーバは終わり、`logs\server` の今日のファイルに DB に繋げないことが書かれている

#### Scenario: サービスの停止を受けると、書き途中の書き出しを残さずに終わる

- **WHEN** 書き出しが走っている間に `ashiato-server` を止める
- **THEN** サービスは停止し、書き出しの置き場に `.` で始まる書き途中のファイルが残らず、次に起きたときに書き出しの状態が「走っていない」である

#### Scenario: 開発の台本で起こすサーバは、今までどおり環境変数を読む

- **WHEN** 設定のファイルを置かずに、`.env` の環境変数で `ashiato-server` を（サービスでなく）起こす
- **THEN** サーバは環境変数の設定で起き、`/healthz` に 200 で答える

### Requirement: 配る DB は、OS に依らない文字の並べ方と合言葉の認証で作り、手元でだけ待ち受ける

WHEN インストーラが DB を初めて作る THE SYSTEM SHALL 同梱の PostgreSQL 17 で、文字の並べ方を OS に依らない形（`builtin` の `C.UTF-8`）、符号化を `UTF8` にし、データのチェックサムを付けて作る。
THE SYSTEM SHALL DB のデータを、PostgreSQL のメジャー版ごとに分けた状態の置き場の中の場所（`db\17`）に置く。
THE SYSTEM SHALL 配る DB のどの接続（手元の接続を含む）にも合言葉の認証（`scram-sha-256`）を求め、合言葉なしで通す設定（`trust`）を残さない。
THE SYSTEM SHALL 配る DB を `127.0.0.1` の 55400 番だけで待ち受けさせる。
WHILE インストーラの `repair` の口が DB の役割の合言葉を替えている THE SYSTEM SHALL 例外として、上がった利用者の Windows のアカウントだけが合言葉なしで入れる一時の入口を `127.0.0.1` に開き、`repair` の終わりに閉じる（一時の接続の規則と番号を残さない）。
THE SYSTEM SHALL 配る DB を製造準備の既定値で動かす: 文の記録をしない（`log_statement=none`）・遅い文の記録をしない（`log_min_duration_statement=-1`）・エラーの詳しさを短くする（`log_error_verbosity=terse`）・エラーになった文の字面を書かない（`log_min_error_statement=panic`）。
THE SYSTEM SHALL 配る DB の PostgreSQL のメジャー版を、開発と試験の DB と同じ 17 にする。

導出元: FR-108・FR-132。C1 / C5 / C6 / review/deep.md R9。
文字の並べ方は DB を作るときに決まり、変えるには全部を書き出して入れ直すことになる。OS の地域の設定に任せると、開発の DB（符号の順）と並びが変わり、Windows の更新で並べ方の表が変わると文字の索引が黙って壊れうる（design D1）。
版ごとの置き場は、メジャー版を上げるときの移し方（FR-132 の未決）の扉を開けておく。

#### Scenario: 配る DB は OS に依らない並べ方で作られ、符号の順に並ぶ

- **WHEN** 地域の設定が日本語の Windows でインストールし、記録を 1 件も入れる前に DB の設定を読む
- **THEN** 記録の DB の `datlocprovider` は `b`、`datlocale` は `C.UTF-8`、符号化は `UTF8` で、`select 'a' < 'B'` は偽を返し、データのチェックサムが有効である

#### Scenario: 配る DB は合言葉なしの接続を通さない

- **WHEN** インストールした後に、DB の接続の規則（`pg_hba.conf`）を読み、合言葉を付けずに `127.0.0.1:55400` へ接続する
- **THEN** 接続の規則に `trust` の行が無く、合言葉の無い接続は断られる

#### Scenario: 配る DB は手元の 55400 番だけで待ち受ける

- **WHEN** インストールした後に、DB の待ち受けの設定と、PC の待ち受けの一覧を読む
- **THEN** `listen_addresses` は `127.0.0.1`、`port` は `55400` で、DB の待ち受けは `127.0.0.1:55400` だけにある

#### Scenario: 配る DB はログに文と値を書かない

- **WHEN** インストールした後に、DB に `SHOW log_statement`・`SHOW log_min_duration_statement`・`SHOW log_error_verbosity`・`SHOW log_min_error_statement` を問い、目印の文字の並びを含む構文の誤った文を 1 つ送る
- **THEN** 答えは `none`・`-1`・`terse`・`panic` で、`logs\db` のどのファイルにも目印の文字の並びが無い

#### Scenario: 配る DB と開発の DB は同じメジャー版

- **WHEN** 同梱の PostgreSQL の `postgres --version` と、開発と試験の DB の像の版を読む
- **THEN** どちらもメジャー版が 17 である

### Requirement: 状態の置き場は、2 つのサービスのアカウントだけが書け、それとインストールした本人だけが読める

THE SYSTEM SHALL サーバが持ち続ける状態のすべて（DB・写真の原本・移行の前の退避・バックアップ・書き出し・書庫の写し・ログ・設定と秘密）を状態の置き場（`C:\ProgramData\ashiato`）の下に置く。
THE SYSTEM SHALL 状態の置き場の権限を、親のフォルダから受け継がず、2 つのサービスのアカウントに読み書き、インストールした本人に読みだけを付け、`Users`・`Authenticated Users`・`Everyone` に何も付けない（表の所有者の設定 `settings\owner.json` だけは、サービスのアカウントにも本人にも付けず、管理者と SYSTEM だけにする）。
THE SYSTEM SHALL DB の置き場（`db`）と DB のログの置き場（`logs\db`）には DB のサービスのアカウントだけが書け、ほかの置き場にはサーバのサービスのアカウントだけが書けるようにする。
THE SYSTEM SHALL 「インストールした本人」を、管理者に上がる前のインストーラを開いた利用者とし、別の管理者の資格情報で上がっても、その管理者を本人にしない。
WHEN インストール・合言葉の決め直し・`repair`・消すが終わる THE SYSTEM SHALL その間だけ足した書きの権限を外す。
THE SYSTEM SHALL 走らせ直しで、前のインストールの本人を変えない。

導出元: FR-109。C3 / C14 / review/deep.md R8・R10。
`ProgramData` の下に作ったフォルダは、既定で同じ PC のどの利用者にも読みを受け継ぐ（premises P6。実物で確かめた）。設定のファイルには DB の合言葉も入るので、受け継ぎを切る。
管理者は所有権を取れば読める（ほかの管理者からは守れない）。本人も書けないので、記録を手で消すには管理者として所有権を取る（design D3（仮）・`docs/install.md`）。

#### Scenario: 状態の置き場はほかのアカウントから読めない

- **WHEN** インストールした後に、状態の置き場とその下の全部のフォルダ・ファイルの権限（ACL）を読む
- **THEN** どれも親から受け継がず、読みを持つのは 2 つのサービスのアカウントとインストールした本人だけ（表の所有者の設定 `settings\owner.json` は管理者と SYSTEM だけ）で、`Users`・`Authenticated Users`・`Everyone` の項目が無い

#### Scenario: 本人は状態の置き場を読めるが書けない

- **WHEN** インストールした本人が、管理者に上がらずに書き出しの置き場のファイルを開き、状態の置き場にファイルを作ろうとする
- **THEN** ファイルは読めて、ファイルを作ることは断られる

#### Scenario: 別の利用者は状態の置き場を読めない

- **WHEN** 同じ PC の、管理者でない別の利用者が、状態の置き場の設定のファイルを開く
- **THEN** 開くことは断られる

#### Scenario: DB の置き場にはサーバのアカウントが書けない

- **WHEN** インストールした後に、DB の置き場（`db`）の権限を読む
- **THEN** 書きを持つのは `NT SERVICE\ashiato-db` だけで、`NT SERVICE\ashiato-server` は書きを持たない

#### Scenario: 上がった方が別の管理者でも、本人は上がる前に控えた利用者になる

- **WHEN** 上がる前の段が控えた利用者 A を渡し、上がった方を別の管理者 B で動かしてインストールする
- **THEN** 状態の置き場の読みを持つのは A で、B の項目は無い

#### Scenario: 上がる前の段は、開いた利用者を本人として控える

- **WHEN** 管理者に上がっていない利用者 A がインストーラを開き、上がった方へ渡す引数を読む
- **THEN** 渡す本人は A の SID である

#### Scenario: 別の利用者が走らせ直しても、本人は変わらない

- **WHEN** 利用者 A を本人にしてインストールした後に、別の管理者 B がインストーラを走らせ直す
- **THEN** 状態の置き場の読みを持つのは A のままで、B の項目は無い

#### Scenario: インストールが終わると、インストールの間の書きの権限が残らない

- **WHEN** インストールを最後まで終えた後に、状態の置き場の権限を読む
- **THEN** 2 つのサービスのアカウントのほかに書きを持つ項目が無い

### Requirement: 秘密はインストーラが自動で作り、状態の置き場の設定にだけ置き、人に見せない

WHEN インストーラが DB を初めて作る THE SYSTEM SHALL DB の 3 つの役割（管理者・表の所有者 `ashiato_owner`・実行時 `ashiato_app`）の合言葉と、API の合言葉と、ログインの印の鍵を、OS の暗号の乱数で作る。
THE SYSTEM SHALL 作った秘密を状態の置き場の設定にだけ置き、画面・端末の出力・ログ・記録・状態の置き場の外のファイルに出さない。
THE SYSTEM SHALL 秘密に固定の値（配布物に入った値・前のインストールと同じ値）を使わない。
THE SYSTEM SHALL 作った秘密が、それぞれの検査（API の合言葉・ログインの印の鍵は 16 文字以上で互いに違う）を通る長さと形を持つようにする。

導出元: FR-110・FR-109。C4 / C7。ST28 design D19（固定値を持たない）・ST44 design D8（仮）（ログインの印の鍵）。
DB の合言葉は人が覚える必要が無い。

#### Scenario: インストールごとに秘密が違う

- **WHEN** インストールして設定のファイルの秘密を控え、消して状態の置き場も消してから、もう一度インストールして比べる
- **THEN** DB の 3 つの合言葉・API の合言葉・ログインの印の鍵は、どれも 2 つの間で違う

#### Scenario: インストーラの出力とログに秘密が出ない

- **WHEN** インストーラの端末の出力と、状態の置き場のログを全部読み、設定のファイルの秘密の値を探す
- **THEN** どの秘密の値も見つからない

#### Scenario: DB を作るときの一時の合言葉のファイルが残らない

- **WHEN** インストールした後に、状態の置き場と一時の置き場（`%TEMP%`）の下のファイルを、DB の管理者の合言葉の文字の並びで探す
- **THEN** 見つかるのは設定のファイルだけである

#### Scenario: 作った秘密でサーバの起動の検査が通る

- **WHEN** インストールした後に、サーバのサービスが起きたかを見る
- **THEN** サーバは実行時の役割で DB に繋いで起き、API の合言葉とログインの印の鍵の検査で止まっていない

### Requirement: インストーラは DB の 3 つの役割を作り、インストールのときだけ移行を当てる

WHEN インストーラが DB を初めて作る THE SYSTEM SHALL 管理者・表の所有者・実行時の 3 つの役割を作り、表の所有を表の所有者へ移し、実行時の役割から管理者と所有者の権限を外す（開発の DB の役割と同じ形）。
WHEN インストーラが走る THE SYSTEM SHALL サーバの移行（`ashiato-server migrate`）を表の所有者で当て、移行の前の退避を状態の置き場の `backups\pre-migrate` に、同梱の `pg_dump` で取る。
IF 移行が失敗する THEN THE SYSTEM SHALL サーバのサービスを開始せず、移行の失敗とログの場所を示して止まる。
THE SYSTEM SHALL サーバのサービスが起きるたびには移行を当てない（移行を当てるのはインストールと更新のときだけ）。
THE SYSTEM SHALL 移行の前の退避の置き場と `pg_dump` を、作業の場所や PATH に頼らず、絶対の場所で渡す。

導出元: FR-108・FR-109。C7 / C11 / review/deep.md R7。申し送り K1（`migrate_backup.rs` の既定は作業の場所からの相対で、サービスの作業の場所は `C:\Windows\System32`）。
版の食い違いは、サーバが起動の時に止める（ST48。正典 distribution）。

#### Scenario: 配る形に移行を当ててサーバが起きる

- **WHEN** 配る zip からインストールする
- **THEN** 記録の DB の版の記録に、サーバが知っている全部の移行の版が並び、サーバは実行時の役割の検査を通って起き、`/healthz` に 200 で答える

#### Scenario: 実行時の役割は管理者でも表の所有者でもない

- **WHEN** インストールした後に、実行時の役割 `ashiato_app` の属性と、記録の表の所有者を読む
- **THEN** `ashiato_app` は管理者でなく、どの役割も継がず、記録の表の所有者は `ashiato_owner` である

#### Scenario: 移行の前の退避は状態の置き場に、同梱の pg_dump で取られる

- **WHEN** 移行の前の退避の置き場の指定と PATH の `pg_dump` を持たない環境でインストールする
- **THEN** 退避のファイルが `C:\ProgramData\ashiato\backups\pre-migrate` の下にでき、`C:\Windows\System32` の下には何もできない

#### Scenario: 移行が失敗すると、サーバのサービスを開始せずに止まる

- **WHEN** 当たらない移行を差し込んだ配布物（試験の組み立ての差し込み）でインストールする
- **THEN** インストーラは rc≠0 で移行の失敗とログの場所を示して止まり、`ashiato-server` は実行中でない

#### Scenario: サービスの起動では移行を当てない

- **WHEN** インストールした後に、版の記録の行の数を数え、`ashiato-server` を止めて開始し、もう一度数える
- **THEN** 2 つの数は同じである

### Requirement: インストールを走らせ直すと足りないものだけを足し、既にある DB・秘密・設定を上書きしない

WHEN インストーラをもう一度走らせる THE SYSTEM SHALL 既にある DB・秘密・設定・画面の合言葉を上書きせず、足りないもの（サービス・置き場・権限・プログラム）だけを足す。
WHEN 更新係の無い前のインストールの上で走らせ直す THE SYSTEM SHALL 更新係のサービスと置き場と権限を足し、更新の確認を有効にする。
THE SYSTEM SHALL 走らせ直しで、更新係の切った印（`enabled: false`）と確かめた記録を上書きせず、切ってある確認を入れ直さない。
THE SYSTEM SHALL 秘密を設定に書き終えてから DB を作る（途中で落ちても「秘密はあるのに DB が無い」側に倒れ、走らせ直しで DB を作れる）。
IF DB はあるのに秘密が無い THEN THE SYSTEM SHALL 何も書き換えず、DB と秘密が食い違っていることと状態の置き場の場所と、出口のインストーラの `repair` の口を示して止まる。
WHEN 前のインストールが画面の合言葉を決める前に終わっていた THE SYSTEM SHALL 走らせ直しで、画面の合言葉を決める段からやり直す。

導出元: FR-108・FR-110・FR-111。C10 / review/deep.md R5。
★ ST57: 更新係を足す・切った確認を戻さない（聞かない既定 C7）。
上書きすると DB に繋げなくなり、収集アプリの資格情報も切れる。DB があるのに秘密が無いまま止まっている間、携帯の未送信は上限（90 日・2 GB。FR-9）に向かって溜まるので、記録を残したまま戻る出口を `repair` の口に置く（design D10・D27。深掘り Q8）。

#### Scenario: 走らせ直しても秘密と DB は変わらない

- **WHEN** インストールした後に、設定のファイルの指紋と記録の件数を控え、インストーラをもう一度走らせる
- **THEN** 設定のファイルの指紋と記録の件数は控えと同じで、2 つのサービスは実行中である

#### Scenario: DB を作る途中で落ちても、走らせ直しで DB ができる

- **WHEN** DB を作る段で止まる差し込み（試験の組み立ての差し込み）でインストーラを走らせ、差し込みを外してもう一度走らせる
- **THEN** 2 回目で DB ができ、1 回目に作った秘密でサーバが起きる

#### Scenario: DB はあるのに秘密が無いと、何も書き換えずに止まる

- **WHEN** インストールした後に設定のファイルを退け、インストーラをもう一度走らせる
- **THEN** インストーラは rc≠0 で DB と秘密の食い違いと状態の置き場の場所と `repair` の口を示して止まり、DB の置き場の中身と記録の件数は変わらない

#### Scenario: 合言葉を決める前に閉じたら、走らせ直しで合言葉の段からやり直す

- **WHEN** 画面の合言葉を決める段でインストーラを閉じ、もう一度走らせる
- **THEN** 2 回目は DB と秘密を作り直さずに、画面の合言葉を決める段を出す

#### Scenario: 更新係の無いインストールを走らせ直すと、更新係が足される

- **GIVEN** 更新係のサービスと置き場の無い、前の版のインストール（試験の組み立ての差し込みで更新係を消した形）
- **WHEN** インストーラをもう一度走らせる
- **THEN** サービス `ashiato-updater` が開始の種類「自動」で登録され、更新係の置き場ができ、DB と設定のファイルの指紋は控えと同じである

#### Scenario: 切った更新の確認は、走らせ直しでも切ったまま

- **GIVEN** `update-check off` で切ったインストール
- **WHEN** インストーラをもう一度走らせる
- **THEN** 切った印は `enabled: false` のままで、サービス `ashiato-updater` の開始の種類は「無効」のままである

### Requirement: DB はあるのに秘密が無くて止まったら、インストーラの `repair` の口で記録を残したまま動かし直せる

WHEN 利用者がインストーラを `repair` の口で開く THE SYSTEM SHALL 管理者の確認を 1 回出す。
WHILE DB の置き場に DB があり、設定のファイルが無い THE SYSTEM SHALL `repair` の口で、2 つのサービスを止め、秘密（DB の 3 つの役割の合言葉・API の合言葉・ログインの印の鍵）を作り直し、DB の 3 つの役割の合言葉を新しい秘密に替えてから設定に書き、2 つのサービスを開始し、インストールの最後と同じ検査で画面の合言葉を決めさせる。
THE SYSTEM SHALL `repair` の口で、記録の行・移行の記録・原本・書き出しを書き換えない。
WHEN `repair` の口が終わる THE SYSTEM SHALL API の合言葉が変わったので収集アプリの繋ぎ直しが要ることと、その手順の場所（`docs/install.md`）を、秘密の値を出さずに示す。
IF `repair` の口を開いたときに設定のファイルがあるか DB が無い THEN THE SYSTEM SHALL 何も書き換えずに rc≠0 で終わり、走らせ直しか `password` の口を示す。
THE SYSTEM SHALL 管理者の確認を通らない利用者に、`repair` をさせない。
THE SYSTEM SHALL `repair` で、前のインストールの本人を変えない（設定のファイルが無いので、本人は状態の置き場の権限から読む）。

導出元: FR-108・FR-110・FR-111。深掘り Q8（本人の選択: この Story で出口を作る）。design D27。
出口が無いと、元に戻す手は DB の置き場を消して入れ直すこと（記録を全部捨てる）しかない。役割の合言葉を替え終えてから設定を置くので、途中で落ちても「DB はあるのに秘密が無い」のまま、もう一度 `repair` できる。

#### Scenario: 設定を退けて止まった後に repair すると、記録を残したままログインできる

- **WHEN** インストールして記録を 1 件入れ、記録の件数と中身の指紋を控え、設定のファイルを退けて走らせ直しが止まった後に、インストーラを `repair` の口で開いて新しい画面の合言葉を決める
- **THEN** 2 つのサービスは実行中で、新しい合言葉で画面にログインでき、記録の件数と中身の指紋は控えと同じで、前の記録が読み出しに出る

#### Scenario: repair の後は前の API の合言葉では送れず、繋ぎ直しの案内が出る

- **WHEN** `repair` の前の API の合言葉を控え、`repair` を終えてから、控えた合言葉と新しい設定の合言葉でそれぞれ記録の送信を求める
- **THEN** 控えた合言葉の求めは 401 で断られ、新しい合言葉の求めは通り、`repair` の出力に収集アプリの繋ぎ直しが要ることと `docs/install.md` があり、秘密の値は無い

#### Scenario: 止まった状態でなければ repair は何も変えずに終わる

- **WHEN** インストールを最後まで終えた後に（設定のファイルがある）、インストーラを `repair` の口で開く
- **THEN** インストーラは rc≠0 で `repair` が要らないことと走らせ直しか `password` の口を示して終わり、設定のファイルの指紋と記録の件数は変わらず、前の合言葉でログインできる

#### Scenario: repair の後に合言葉なしの入口と一時の番号が残らない

- **WHEN** 設定のファイルを退けて `repair` を終えた後に、DB の接続の規則と待ち受けの番号を読む
- **THEN** 接続の規則に `scram-sha-256` でない行（`sspi`・`trust`）が無く、一時の接続の規則のファイルが無く、DB の待ち受けは `127.0.0.1:55400` だけである

#### Scenario: 別の管理者が repair しても、本人は変わらない

- **WHEN** 利用者 A を本人にしてインストールし、設定のファイルを退けた後に、別の管理者 B が `repair` を走らせる
- **THEN** 状態の置き場の読みを持つのは A のままで B の項目は無く、作り直した設定の本人は A である

#### Scenario: repair の途中で落ちても、もう一度 repair できる

- **WHEN** 設定のファイルを退けた後に、役割の合言葉を替える段で止まる差し込み（試験の組み立ての差し込み）で `repair` を走らせ、差し込みを外してもう一度 `repair` を走らせる
- **THEN** 1 回目の後の走らせ直しは DB と秘密の食い違いを示して止まり、2 回目の `repair` の後は 2 つのサービスが実行中で、記録の件数は変わらない

### Requirement: インストールの最後に、端末で画面の合言葉を決め、そのままログインできる

WHEN インストールが最後に達する THE SYSTEM SHALL インストーラの端末で画面の合言葉を 2 回入れさせ、2 回が一致したときだけ決める。
THE SYSTEM SHALL 画面の合言葉を、16 文字より短い・`change-me` で始まる・API の合言葉と同じ・ログインの印の鍵と同じ・2 回が一致しない、のどれかなら受け付けず、理由を示して入れ直させる。
THE SYSTEM SHALL 入れた合言葉を端末に表示せず、元の値を設定にもログにも残さない。
WHEN 画面の合言葉を決め終える THE SYSTEM SHALL サーバのサービスを（起こし直して）その合言葉で動かし、既定のブラウザで画面（`http://127.0.0.1:18700/`）を開く。
WHILE 端末でない（標準入力が端末につながっていない）THE SYSTEM SHALL 標準入力の 2 行を、2 回入れた合言葉として読む。
THE SYSTEM SHALL 合言葉を聞くとき（インストールの最後・`password` の口・`repair` の口のどれも）、最初の問いの前に 1 回だけ、受け付ける条件（16 文字以上・`change-me` で始めない・API の合言葉とログインの印の鍵と同じにしない）を示し、断るたびには示し直さない。端末でないときも示す。
WHILE 標準入力が端末につながっている THE SYSTEM SHALL 条件に、入れた文字は表示されないことを添える。
THE SYSTEM SHALL 短い・雛形の理由で断るときは、理由に条件を添える（「短い（16 文字以上）」「雛形（change-me で始めない）」）。
WHILE 標準入力が端末（Windows の窓）につながっている THE SYSTEM SHALL 合言葉の問いの文と条件を、窓のコード頁に依らず読める文字で、標準出力・標準エラーの向け替えに依らず窓へ出す。

導出元: FR-111。深掘り Q3（推奨: インストーラの端末で 2 回打たせて決める）/ C8 / C10。CH14（深掘り Q1・Q3・Q4・Q5。design D1〜D6）。
画面の側に「最初に合言葉を決める頁」を出すと、決めるまでの間、同じ PC のほかのプログラムが先に決めて全部の記録を読める（Q3 の案 3）。端末でないときの読み方は、Windows の CI でインストールを試すため（design D23）。
日本語の Windows（コード頁 932）の窓では、問いの文を UTF-8 のバイトのまま窓へ書くと化けて読めない（CH14。2026-11-09 本人の実地）。条件を先に示すのは、「短い」とだけ断られても何文字要るか分からなかったため。

#### Scenario: 決めた合言葉で、そのまま画面にログインできる

- **WHEN** インストールの最後に、20 文字の合言葉を 2 回入れる
- **THEN** 画面 `http://127.0.0.1:18700/` のログインの口にその合言葉で求めると、ログインの印が発行され、記録の読み出しが通る

#### Scenario: 短い・既定の雛形・一致しない合言葉は受け付けない

- **WHEN** インストールの最後に、15 文字の値・`change-me` で始まる 20 文字の値・1 回目と違う 2 回目を、それぞれ入れる
- **THEN** どれも受け付けられず、短い・雛形・一致しない、をそれぞれ理由に示し、合言葉を入れ直す段に戻る

#### Scenario: API の合言葉やログインの印の鍵と同じ値は受け付けない

- **WHEN** インストールの最後に、設定のファイルの API の合言葉と同じ値と、ログインの印の鍵と同じ値を、それぞれ 2 回入れる（試験は設定から読んだ値を渡す）
- **THEN** どちらも受け付けられず、API の合言葉と同じ・ログインの印の鍵と同じ、をそれぞれ理由に示す

#### Scenario: 合言葉を決め終えると、既定のブラウザで画面を開く

- **WHEN** インストールの最後に合言葉を決め、インストーラが起こした命令の一覧（試験の組み立ての記録）を読む
- **THEN** 一覧の最後に、既定のブラウザで `http://127.0.0.1:18700/` を開く命令がある

#### Scenario: 合言葉の元の値は、どこにも残らない

- **WHEN** 合言葉を決めた後に、状態の置き場の全部のファイルと、インストーラの端末の出力と、サービスの登録簿を、決めた合言葉の文字の並びで探す
- **THEN** どこにも見つからない

#### Scenario: 端末でないときも、断りより前に合言葉の条件を 1 回だけ示す

- **WHEN** 標準入力に合言葉の行を流して（端末でない）、インストールの最後・`password` の口・`repair` の口のそれぞれで合言葉を決め（`password` の口では、断られる値を 3 組入れてから受け付けられる値を入れる）、出力を読む
- **THEN** どの口の出力にも、16 文字以上・change-me で始めない・API の合言葉・ログインの印の鍵、の語を含む条件の行がちょうど 1 つあり、断りの行があれば条件の行はその最初より前にあり、「表示されません」の語は無い

#### Scenario: 短い・雛形で断るときは、理由に条件を添える

- **WHEN** 15 文字の値と、`change-me` で始まる 20 文字の値を、それぞれ 2 回入れる
- **THEN** 断りの行に「短い（16 文字以上）」と「雛形（change-me で始めない）」がある

#### Scenario: コード頁が UTF-8 でない窓でも、合言葉の問いと条件が読める

- **WHEN** コード頁が UTF-8（65001）でない窓（疑似コンソール）でインストーラを `password` の口で開き、20 文字の合言葉を 2 回打ち、制御の並びを除いた窓の文字を読む
- **THEN** 窓の文字に「画面の合言葉: 」「画面の合言葉（もう一度）: 」と、16 文字以上・change-me で始めない・API の合言葉・ログインの印の鍵・表示されません、の語を含む条件の行があり、条件の行は最初の「画面の合言葉: 」より前にある

#### Scenario: 窓で打った合言葉で決め直せる

- **WHEN** 窓（疑似コンソール）でインストーラを `password` の口で開き、20 文字の合言葉を 2 回打つ
- **THEN** インストーラは rc=0 で終わり、サーバが答えるようになった後に、その合言葉で画面にログインできる

#### Scenario: 窓で打った合言葉は、窓に出ない

- **WHEN** 窓（疑似コンソール）でインストーラを `password` の口で開き、合言葉を 2 回打って、窓に出た文字を読む
- **THEN** 打った合言葉の文字の並びが、窓に出た文字のどこにも無い

#### Scenario: 標準出力と標準エラーを向け替えても、合言葉の問いは窓に出る

- **WHEN** 窓（疑似コンソール）で、標準出力と標準エラーをファイルへ向け替えてインストーラを `password` の口で開き、合言葉を 2 回打つ
- **THEN** 窓に出た文字に「画面の合言葉: 」と条件の行があり、向け替えたファイルに「画面の合言葉: 」が無い

### Requirement: 画面の合言葉を忘れたら、インストーラで管理者として決め直せる

WHEN 利用者がインストーラを `password` の口で開く THE SYSTEM SHALL 管理者の確認を 1 回出し、インストールの最後と同じ検査で新しい画面の合言葉を決めさせる。
WHEN 新しい画面の合言葉を決める THE SYSTEM SHALL サーバのサービスを起こし直し、それまでのログインの印をすべて無効にする。
THE SYSTEM SHALL 合言葉を決め直しても、DB・記録・ほかの秘密（DB の合言葉・API の合言葉・ログインの印の鍵）を変えない。
THE SYSTEM SHALL 管理者の確認を通らない利用者に、合言葉を決め直させない。

導出元: FR-111。深掘り Q3（推奨: `ashiato-setup.exe password` で決め直す）。
覚えている合言葉を画面から変える口は ST59（FR-133）。忘れたときは画面にログインできず、設定はサービスだけが書ける置き場にある（FR-109）ので、本人が手でファイルを直せない。

#### Scenario: 決め直した合言葉でログインでき、前の合言葉ではできない

- **WHEN** インストールした後に、インストーラを `password` の口で開いて新しい合言葉を決める
- **THEN** 新しい合言葉でのログインは通り、前の合言葉でのログインは 401 で断られる

#### Scenario: 決め直すと、それまでのログインは切れる

- **WHEN** 前の合言葉でログインした印を控えてから合言葉を決め直し、控えた印で記録の読み出しを求める
- **THEN** 求めは 401 で断られる

#### Scenario: 決め直しても、ほかの秘密と記録は変わらない

- **WHEN** 合言葉を決め直す前と後で、記録の件数と、DB の合言葉・API の合言葉・ログインの印の鍵を比べる
- **THEN** どれも同じである

#### Scenario: 管理者に上がらなければ決め直せない

- **WHEN** 管理者でない利用者が `password` の口を開き、管理者の確認を断る（試験の組み立ての差し込みで、確認を断ったときと同じ戻り値を返す）
- **THEN** インストーラは rc≠0 で終わり、設定のファイルは変わらず、前の合言葉でログインできる

### Requirement: インストールした形でも、Takeout をダウンロードするだけで取り込まれる

WHEN インストーラが走る THE SYSTEM SHALL インストールした本人の「ダウンロード」（`shell:Downloads` の実の場所）にサーバのサービスのアカウントの読みを足し、本人のプロファイルの直下の取り込み待ち（`%USERPROFILE%\ashiato\取り込み待ち`）を作って読み書きを足す。
THE SYSTEM SHALL 2 つの場所と書庫の写しの置き場（状態の置き場の `archive-copies`）を、絶対の場所でサーバに渡す。
THE SYSTEM SHALL 取り込みの利用者に、収集アプリと画面と同じ既定の利用者を渡し、インストールごとに作り直さない。
THE SYSTEM SHALL 取り込み待ちを、同期されるフォルダ（「ドキュメント」・「デスクトップ」・OneDrive の下）に作らない。
THE SYSTEM SHALL 取り込み待ちの親（`%USERPROFILE%\ashiato`）にサーバのサービスのアカウントのフォルダを作る権限（そのフォルダだけ）を足し、取り込み待ちが消されてもサーバが作り直せるようにする。
THE SYSTEM SHALL ダウンロードと取り込み待ちとその親のほかの、本人のフォルダに権限を足さない。

導出元: FR-108 の ★ 2026-11-03 補足（`req-native-postgres-r3.html` の Q1）。C9 / C16 / review/deep.md R1。申し送り C5 / K3。
サービスのアカウントで動くと、取り込み待ちとダウンロードの既定は本人のフォルダでなくサービスのプロファイルを指す（premises P11）。取り込みの利用者を渡し忘れると Takeout が黙って入らず、別の値だと記録が画面に出ないまま書き換えられない（design D16（仮））。

#### Scenario: ダウンロードに置いた Takeout が、インストールした形で取り込まれる

- **WHEN** インストールした後に、形の確認の印を置き、本人の「ダウンロード」に `takeout-20261103T000000Z-001.zip` を置いて 10 分待つ
- **THEN** その書庫の記録が、画面にログインして読む既定の利用者の読み出しに出て、サーバのログに取り込み器が止まっている旨（`archive_disabled`）が無い

#### Scenario: 取り込み待ちは本人のプロファイルの直下にでき、サーバが読み書きできる

- **WHEN** インストールした後に、`%USERPROFILE%\ashiato\取り込み待ち` の有無と権限を読む
- **THEN** フォルダがあり、`NT SERVICE\ashiato-server` が読み書きを持ち、場所は「ドキュメント」・「デスクトップ」・OneDrive の下でない

#### Scenario: 取り込み待ちを消しても、サーバが作り直す

- **WHEN** インストールした後に本人が取り込み待ちのフォルダを消し、`ashiato-server` を止めて開始する
- **THEN** 取り込み待ちのフォルダがもう一度あり、`NT SERVICE\ashiato-server` が読み書きを持つ

#### Scenario: ダウンロードにはサーバのアカウントの読みだけが足される

- **WHEN** インストールした後に、本人の「ダウンロード」の権限を読む
- **THEN** `NT SERVICE\ashiato-server` の項目は読みだけで、書きを持たない

### Requirement: 配るサーバと DB は、開発の台本と被らない固定の番号で待ち受け、塞がっていたら止まる

THE SYSTEM SHALL 配る正式版とベータ版のサーバを `127.0.0.1:18700`、DB を `127.0.0.1:55400` で待ち受けさせる。
THE SYSTEM SHALL 2 つの番号を、PostgreSQL の既定（5432）と開発の台本が使う番号の割り当てのどれとも被らないものにする。
IF インストールのときに 2 つの番号のどちらかが別のプログラムに塞がれている THEN THE SYSTEM SHALL サービスを登録せず、塞がっている番号と塞いでいるプログラムを示して止まる。

導出元: FR-108。C5。本人の PC では 5432 が別のプロジェクトの DB に塞がっている（premises P8）。被らない割り当てで防ぐ。
開発版の番号（DB 55401・サーバ 18701）と名前は ST54（FR-106）が確かめる（design D14）。

#### Scenario: 配る番号は開発の台本の割り当てと被らない

- **WHEN** 開発の台本が使う番号の割り当ての範囲と既定の番号を全部挙げ、18700 と 55400 と比べる
- **THEN** どちらも割り当ての範囲にも 5432 にも入らない

#### Scenario: 番号が塞がっていると、登録せずに止まる

- **WHEN** 別のプログラムが `127.0.0.1:55400` で待ち受けている状態でインストーラを走らせる
- **THEN** インストーラは rc≠0 で 55400 が塞がっていることと塞いでいるプログラムを示して止まり、サービス `ashiato-db` も `ashiato-server` も登録されていない

### Requirement: サービスにしたサーバと DB のログは状態の置き場のファイルに書き、送った文と秘密はイベントログに出さない

THE SYSTEM SHALL DB のログを状態の置き場の `logs\db` のファイルに書き、DB のサービスのアカウントにそのフォルダの書きを付ける。
THE SYSTEM SHALL 送った文の字面と秘密の値を Windows のイベントログに書かない（DB がログのファイルへ切り替わる前の起動の知らせと、サービスの制御の知らせは出てよい）。
THE SYSTEM SHALL サービスとして動くサーバのログ（起動を拒んだ理由を含む）を、状態の置き場の `logs\server` のファイルに書く。
THE SYSTEM SHALL ログのファイルを日ごとに分け、DB は 7 日分、サーバは 30 日分を残して古いものを消す。
THE SYSTEM SHALL ログのファイルの権限を状態の置き場の権限の Requirement に従わせる（`logs\db` は DB のサービスのアカウントが、`logs\server` はサーバのサービスのアカウントが書き、ほかのアカウントからは読めない）。

導出元: FR-109。C17 / review/deep.md R2。製造準備は「一度出たログは消せない」とする（`docs/production-prep.md`）。イベントログの読み手は確かめていないので、文の字面と秘密は出さない。
サービスとして動く PostgreSQL は、ログのファイルへ切り替わる前の出力と `pg_ctl` の知らせをイベントログへ書く。これは受け入れる（深掘り Q9。本人の選択）。
サービスの標準出力はどこにも残らないので、ファイルに書かないと起動の拒否が本人に見えない（design D17・D3）。

#### Scenario: 起動を拒んだ理由が、サーバのログのファイルに残る

- **WHEN** インストールした後に、設定のファイルのログインの印の鍵を 15 文字の値に差し替えて（試験の組み立ての差し込み）`ashiato-server` を開始する
- **THEN** サービスは実行中にならず、`logs\server` の今日のファイルに鍵が短いことが書かれている

#### Scenario: DB のサービスは logs の db に書け、DB のログのファイルができる

- **WHEN** インストールした後に、`logs\db` の権限を読み、`ashiato-db` を止めて開始する
- **THEN** `logs\db` の書きを持つのは `NT SERVICE\ashiato-db` だけで、`ashiato-db` は実行中になり、`logs\db` に今日の曜日の名前のファイルがある

#### Scenario: 送った文と秘密はイベントログに出ない

- **WHEN** インストールして、DB に構文の誤った文を 1 つ送り、2 つのサービスを止めて開始した後に、Windows のアプリケーションのイベントログの項目を全部読む
- **THEN** どの項目にも、送った文の文字の並びと、設定のファイルの秘密の値が無い

#### Scenario: ログは DB が 7 日分、サーバが 30 日分残る

- **WHEN** 日付をずらしたサーバのログのファイル（31 日前と 29 日前）を置いて `ashiato-server` を開始し、DB のログの設定を読む
- **THEN** 31 日前のファイルは消え、29 日前のファイルは残り、DB のログのファイルの名前は曜日で、日が替わると前の週の同じ曜日のファイルを上書きする

#### Scenario: ログのファイルはほかのアカウントから読めない

- **WHEN** インストールした後に、`logs` の下のフォルダとファイルの権限を読む
- **THEN** 読みを持つのは 2 つのサービスのアカウントとインストールした本人だけである

### Requirement: サーバは Windows の「アプリと機能」から消せ、消しても記録の置き場は残る

WHEN インストールが終わる THE SYSTEM SHALL Windows の「アプリと機能」にサーバを、版番号と一緒に載せる。
WHEN 利用者が「アプリと機能」かインストーラの `uninstall` の口からサーバを消す THE SYSTEM SHALL 管理者の確認を 1 回出し、DB とサーバの 2 つのサービスと更新係のサービスを止めて登録から消し、プログラムの置き場と「アプリと機能」の項目と、ダウンロードと取り込み待ちに足した権限を消す。
THE SYSTEM SHALL サーバを消しても、状態の置き場（記録・原本・退避・書き出し・設定）と更新係の置き場と取り込み待ちの中身を消さず、状態の置き場と更新係の置き場の場所と、手で消すときの手順の場所（`docs/install.md`）を示す。
WHEN 消した後にもう一度インストールする THE SYSTEM SHALL 残った状態の置き場の DB と秘密の上で、走らせ直しと同じに続ける。

導出元: FR-108・FR-109。深掘り Q4（推奨: 作る。記録の置き場は残し、場所を示す）。
★ ST57: 更新係のサービスも消し、更新係の置き場は残す（聞かない既定 C7）。`repair` は更新係に触れない。
記録を消す口は作らない —— 状態の置き場は本人も書けない（FR-109）ので、消すには管理者として所有権を取ってから消す（review/deep.md R10）。

#### Scenario: インストールすると「アプリと機能」に載る

- **WHEN** インストールした後に、Windows のアンインストールの登録簿（`HKLM\Software\Microsoft\Windows\CurrentVersion\Uninstall\ashiato`）を読む
- **THEN** 表示名と版番号と、インストーラの `uninstall` の口を指す消す命令がある

#### Scenario: 消すとサービスとプログラムと登録が消える

- **WHEN** インストールした後に、インストーラの `uninstall` の口で消す
- **THEN** サービス `ashiato-db` と `ashiato-server` と `ashiato-updater`、プログラムの置き場、アンインストールの登録簿の項目は無い

#### Scenario: 消しても記録の置き場は残り、その場所を示す

- **WHEN** インストールして記録を 1 件入れた後に、インストーラの `uninstall` の口で消す
- **THEN** 状態の置き場の DB と設定と取り込み待ちの中身と、更新係の置き場の確かめた記録は残り、出力に状態の置き場と更新係の置き場の場所と手で消す手順の場所がある

#### Scenario: 消した後に入れ直すと、前の記録の上で続く

- **WHEN** 記録を 1 件入れてから消し、同じ版でもう一度インストールする
- **THEN** 入れ直しは画面の合言葉を聞かずに終わり、前の合言葉でログインでき、前の記録が読み出しに出る

#### Scenario: 消すとダウンロードと取り込み待ちに足した権限が外れる

- **WHEN** インストールした後に消し、本人の「ダウンロード」と取り込み待ちの権限を読む
- **THEN** どちらにもサーバのサービスのアカウントの項目が無い

### Requirement: インストールの最後に、携帯から届く経路と置き場の空きを知らせる

WHEN インストールが最後に達する THE SYSTEM SHALL Tailscale の設定を書き換えず、ログオンしていない間も携帯から届くための設定（「無人で動かす」）の入れ方と、`tailscale serve` の転送先を `http://127.0.0.1:18700` にするコマンドを示す。
WHEN インストールが始まる THE SYSTEM SHALL 状態の置き場のドライブの空きを見て、初年度の見積もり（40 GB）より少なければ、空きと見積もりを示して知らせ、インストールは止めない。
THE SYSTEM SHALL 状態の置き場を `C:\ProgramData\ashiato` に固定し、インストールのときに別の場所を選ばせない。

導出元: FR-108・FR-109。深掘り Q6（推奨: Tailscale に触らず示す）・Q7（推奨: `C:\ProgramData\ashiato` に固定し、空きを知らせる）/ review/deep.md R3・R6。
Tailscale の設定を書き換えると、本人の網の設定に ashiato が手を入れる（要件の範囲の外）。空きが尽きると DB が書けなくなり、携帯は上限（90 日・2 GB。FR-9）を越えた分を捨てる。

#### Scenario: 最後に Tailscale の転送先を示し、Tailscale の設定は変えない

- **WHEN** インストールを終え、出力と、インストーラが起こした命令の一覧（試験の組み立ての記録）を読む
- **THEN** 出力に「無人で動かす」の入れ方と `tailscale serve` の転送先 `http://127.0.0.1:18700` のコマンドがあり、命令の一覧に `tailscale` の命令が無い

#### Scenario: インストーラは状態の置き場を選ぶ口を持たない

- **WHEN** インストーラに状態の置き場を選ぶ引数（`--state-dir D:\ashiato`）を渡して走らせる
- **THEN** インストーラは知らない引数として rc≠0 で止まり、何も作らない

#### Scenario: 空きが見積もりより少なければ知らせ、インストールは続ける

- **WHEN** 状態の置き場のドライブの空きを 10 GB と見せる差し込み（試験の組み立ての差し込み）でインストールする
- **THEN** 出力に空き 10 GB と見積もり 40 GB が示され、インストールは最後まで終わる

### Requirement: サーバの Windows の配布物は、インストーラと、版と指紋を固定した PostgreSQL の要るものだけを持つ

THE SYSTEM SHALL サーバの Windows の配布物（`ashiato-server-<版番号>-windows-x86_64.zip`）に、サーバ・更新係（`ashiato-updater.exe`）・インストーラ（`ashiato-setup.exe`）・同梱の PostgreSQL 17 を入れ、インストーラと更新係とサーバの組み立ての印の版番号をそろえる。
THE SYSTEM SHALL 同梱の PostgreSQL を、版と内容の指紋（SHA-256）を固定した公式の Windows 向けの zip から取り、指紋が違えば配布物を組まない。
THE SYSTEM SHALL 同梱の PostgreSQL から、動かすのに要るもの（サーバ・`initdb`・`pg_ctl`・`psql`・`pg_dump` と、その DLL・`lib`・`share`）と許諾の文だけを入れ、管理画面（pgAdmin）・StackBuilder・開発用の見出しと文書を入れない。
THE SYSTEM SHALL Linux の配布物の中身を変えない。
THE SYSTEM SHALL 試験のための差し込みと、起こした命令の記録を、試験の組み立てでだけ読み、配る組み立てのインストーラと更新係とサーバには持たせない。

導出元: FR-108（同梱するのは要るものだけ・版が配布物で固定される）・FR-132。C12 / C13。premises P5（EDB の zip は展開で 889 MB、うち 706 MB が pgAdmin）。
★ ST57: 更新係の実行ファイルを足した（FR-112）。Linux の配布物は変えない（聞かない既定 C10）。
許諾の一覧（FR-127）への統合は ST53（`docs/handoff/ST53.md`）。

#### Scenario: Windows の配布物にインストーラと PostgreSQL が入り、管理画面は入らない

- **WHEN** 配る形でサーバの Windows の配布物を組み、zip の目録を読む
- **THEN** `ashiato-server.exe`・`ashiato-updater.exe`・`ashiato-setup.exe`・`pgsql\bin\postgres.exe`・`pgsql\bin\initdb.exe`・`pgsql\bin\pg_ctl.exe`・`pgsql\bin\psql.exe`・`pgsql\bin\pg_dump.exe` と PostgreSQL の許諾の文があり、`pgAdmin 4`・`StackBuilder`・`include`・`doc` の下のものと PC の収集アプリ（`ashiato-collector`）は無い

#### Scenario: インストーラとサーバの版番号はそろう

- **WHEN** 版番号 `0.4.0-beta.1` を渡して Windows の配布物を組み、`ashiato-setup.exe` と `ashiato-updater.exe` と `ashiato-server.exe` の組み立ての印を読む
- **THEN** どの印も `0.4.0-beta.1+` で始まる

#### Scenario: 同梱する PostgreSQL の指紋が違えば組まない

- **WHEN** 固定した指紋と違う中身の PostgreSQL の zip（試験の組み立ての差し込み）で Windows の配布物を組む
- **THEN** 組み立ては rc≠0 で指紋が違うことを示して落ち、配布物はできない

#### Scenario: 配る組み立ては試験の差し込みを持たない

- **WHEN** 配る形でサーバの Windows の配布物を組み、`ashiato-setup.exe` と `ashiato-updater.exe` と `ashiato-server.exe` の中の文字列を読む
- **THEN** 試験の差し込みの名前（`ASHIATO_SETUP_TEST_`・`ASHIATO_SERVICE_TEST_`・`ASHIATO_UPDATER_TEST_`）が無い

#### Scenario: Linux の配布物は PostgreSQL を持たない

- **WHEN** 配る形でサーバの Linux の配布物を組み、目録を読む
- **THEN** `pgsql` も `ashiato-setup` も `ashiato-updater` も無い

### Requirement: 本体の許諾に、名指しした Google Play 開発者サービスのライブラリと組み合わせて配る追加の許可を付ける

THE SYSTEM SHALL 本体の許諾を、GNU AGPL 第 3 版に第 7 条の追加の許可を 1 つ足したものにする。許可の文の正本は英文で、`LICENSES/AdditionRef-ashiato-play-services.txt` の 1 ファイルに置く。
THE SYSTEM SHALL 正本の `Listed libraries:` より前の行を、次の英文と 1 字も違わないものにする（段落の間は空行 1 つ）。

```
Additional permission under GNU AGPL version 3 section 7

If you modify this Program, or any covered work, by linking or combining it with the Google Play services libraries listed below (any version of them, but not a modified version), containing parts covered by the terms of the Android Software Development Kit License, the licensors of this Program grant you additional permission to convey the resulting work. Corresponding Source for a non-source form of such a combination shall not include the source code for the parts of those libraries used.
```

THE SYSTEM SHALL 正本の `Listed libraries:` の節に、名指しのライブラリを `- <group>:<name>` の形で 1 行ずつ書く。中身は `com.google.android.gms:play-services-location`・`com.google.android.gms:play-services-base`・`com.google.android.gms:play-services-basement`・`com.google.android.gms:play-services-tasks` の 4 つ。
THE SYSTEM SHALL `LICENSE` の先頭に正本のファイルの中身をそのまま置き、その後に AGPL-3.0 の全文を変えずに置く。
IF `LICENSE` の先頭が正本のファイルと 1 字でも違う、または AGPL-3.0 の全文が今の `LICENSE` の全文と違う THEN THE SYSTEM SHALL 許諾の文の検査を落とし、CI を落とす。
THE SYSTEM SHALL `README.md` のライセンスの節に、追加の許可が付いていること・正本は英文であること・日本語の参考訳を置く。`CONTRIBUTING.md` には、貢献も同じ許諾（追加の許可を含む）の下で配ることを書き足し、CLA の受け入れ条件は変えない。

導出元: FR-127, NFR-26, 扉 35。深掘り Q1・Q2（第 1 回の推奨）。premises P1・P10。review/deep.md R2・R3。review/spec.md R7・R8。§5「配布の方針」（許可の範囲は Google Play 開発者サービスのライブラリに限る）。
許可の文は配った版に付いたまま出回り、取り消せない（扉 35）。名指しを狭く持てば、必要になったときに次の版で広げられる。

#### Scenario: 追加の許可の正本は、深掘りで決めた英文と同じ
- **WHEN** 許諾の文の検査を今の木に当てる
- **THEN** 正本の `Listed libraries:` より前の行は、この Requirement の英文と 1 字も違わない

#### Scenario: LICENSE の先頭は追加の許可の正本と同じ
- **WHEN** 許諾の文の検査を今の木に当てる
- **THEN** `LICENSE` の先頭の行は `LICENSES/AdditionRef-ashiato-play-services.txt` の全行と同じで、検査の終了コードは 0

#### Scenario: LICENSE の許可の文を正本と変えると、許諾の文の検査が落ちる
- **GIVEN** `LICENSE` の追加の許可の文から `but not a modified version` を消した使い捨ての木
- **WHEN** 許諾の文の検査を当てる
- **THEN** 検査は 0 でない終了コードで終わり、`LICENSE` と正本が違うことが示される

#### Scenario: LICENSE の AGPL の全文を変えると、許諾の文の検査が落ちる
- **GIVEN** `LICENSE` の AGPL-3.0 の全文の 1 行を書き換えた使い捨ての木
- **WHEN** 許諾の文の検査を当てる
- **THEN** 検査は 0 でない終了コードで終わり、AGPL の全文が違うことが示される

#### Scenario: 許可の文の名指しは、位置のライブラリとそれが引く 3 つだけ
- **WHEN** 正本のファイルの `Listed libraries:` の節を読む
- **THEN** 名指しは `play-services-location`・`play-services-base`・`play-services-basement`・`play-services-tasks` の 4 つ（どれも `com.google.android.gms`）で、`play-services-ads` も `play-services-measurement` も無い

#### Scenario: README のライセンスの節は、追加の許可と正本の言葉を示す
- **WHEN** 許諾の文の検査を今の木に当てる
- **THEN** `README.md` のライセンスの節は `AdditionRef-ashiato-play-services` と、正本が英文で日本語は参考訳であることを持つ。持たなければ検査は 0 でない終了コードで終わる

### Requirement: ソースと部品の許諾の表示は、追加の許可を含む 1 つの式にそろえる

THE SYSTEM SHALL 本体のソースの先頭の許諾の表示と、部品の許諾の欄（根の `Cargo.toml` の workspace の `license`・`web/package.json` の `license`・`web/package-lock.json` の根の `license`）を、`AGPL-3.0-only WITH AdditionRef-ashiato-play-services` にする。
API の契約の写し（`docs/openapi.json`）の許諾の欄も同じ式にする。
THE SYSTEM SHALL 次の 2 種類は、表示を書き換えない。
- 移行のファイル（`migrations/` の 3 つ。中身の指紋を DB に記録しているため）は `AGPL-3.0-only` のまま。最初の印から後は、前進側の 2 つ（`.sql`）の表示を直せない（配った移行の凍結の検査が中身の差を落とす）
- 第三者のファイル（gradle の wrapper の `collector-android/gradlew`・`gradlew.bat`。Apache-2.0）はそのまま
THE SYSTEM SHALL ソースの許諾の表示の検査を持つ。検査は、追跡しているファイル（symlink は辿らない）の先頭 25 行にある `SPDX-License-Identifier:` を読む。決めた式でもなく、上の例外の一覧（パスと許す値の組）にも合わない表示があれば、CI を落とす。

導出元: FR-127 ★ 2026-10-25（追加の許可を足す Story は、ソースの SPDX を一緒に直す）。深掘り Q5（第 1 回の推奨）。premises P4・P5・P6。review/deep.md R8・R11。review/spec.md R3・R16。深掘り Q10・Q11（第 2 回）。D4・D5。§5 ★ 2026-11-03。
§5 の「他人の GPL / AGPL のコードを取り込むと、その部分に許可が及ばない」について。repo に写したコードは依存の検査に現れない。写したコードが先頭 25 行に自分の許諾の表示を持っていれば、この検査が止める（表示を持たないものは止められない。D5・Q11）。

#### Scenario: 決めた許諾の表示でないソースがあれば、表示の検査が落ちる
- **GIVEN** 先頭に `// SPDX-License-Identifier: GPL-3.0-only` を持つ Rust のファイルを 1 つ足した使い捨ての木
- **WHEN** ソースの許諾の表示の検査を当てる
- **THEN** 検査は 0 でない終了コードで終わり、そのファイルの名前が示される

#### Scenario: 例外の一覧のファイルは、元の表示のままで表示の検査を通る
- **WHEN** ソースの許諾の表示の検査を今の木に当てる
- **THEN** 検査の終了コードは 0 で、例外として通したものに `collector-android/gradlew`・`collector-android/gradlew.bat`（`Apache-2.0`）と移行の 3 つのファイル（`AGPL-3.0-only`）の 5 つが示される

#### Scenario: 例外の一覧のファイルでも、許す値と違う表示なら表示の検査が落ちる
- **GIVEN** 移行の例外のファイルの表示を `GPL-3.0-only` に書き換えた使い捨ての木
- **WHEN** ソースの許諾の表示の検査を当てる
- **THEN** 検査は 0 でない終了コードで終わり、そのファイルの名前が示される

#### Scenario: 表示の検査は symlink の先を読まない
- **GIVEN** 木の外の `GPL-3.0-only` の表示を持つファイルを指す symlink を 1 つ足した使い捨ての木
- **WHEN** ソースの許諾の表示の検査を当てる
- **THEN** 検査の終了コードは 0 で、symlink の先のファイルは読まれない

#### Scenario: API の契約の写しは、部品と同じ許諾の式を持つ
- **WHEN** API の契約を作り直して、checked-in の写しと比べる
- **THEN** 写しの `info.license.identifier` は `AGPL-3.0-only WITH AdditionRef-ashiato-play-services` で、作り直したものとの差は無い

### Requirement: 配布物に入る依存の許諾を、携帯の依存も含めて検査する

THE SYSTEM SHALL 依存の許諾の検査に、Rust・画面（Node）・携帯（gradle / maven）の 3 つの節を持たせる。携帯の節は、配る携帯の APK に入る依存（`releaseRuntimeClasspath`）の部品ごとに POM の許諾を読む。
POM が許諾を持たなければ、親の POM を辿って読む。読んだ許諾の名前は、対応表で SPDX の名前に直す。1 つの部品が許諾の名前を 2 つ以上持てば、全部の名前が許可されているか名指しに当たるときだけ通す（1 つでも許可の外なら落とす）。
IF 依存の POM を取り寄せられない（ネットワークにも gradle の cache にも無い） THEN THE SYSTEM SHALL その部品を飛ばさず、検査を落とす。
THE SYSTEM SHALL オープンソースの許諾でない部品のうち、追加の許可の正本の `Listed libraries:` に名指しされた部品だけを通す。名指しの一覧は正本のファイルから読み、検査の中に写しを持たない。
IF 携帯の依存に、名指しの外のオープンソースでない部品・許諾を読めない部品・対応表に無い許諾の名前の部品のどれかがある THEN THE SYSTEM SHALL 検査を落とし、その部品の座標と許諾の名前を示す。
THE SYSTEM SHALL 許可の一覧から `AGPL-3.0-only` を外す。自分の部品（workspace の crate と画面の根）は、名前と決めた許諾の式の組で通す。他人の部品の GPL / AGPL の許諾は、どれも検査が落とす。
THE SYSTEM SHALL ほかの許可の一覧は減らさない。一覧は `MIT`・`MIT-0`・`Apache-2.0`・`BSD-2-Clause`・`BSD-3-Clause`・`ISC`・`Unicode-3.0`・`Unicode-DFS-2016`・`Zlib`・`CC0-1.0`・`MPL-2.0`・`0BSD`・`Apache-2.0 WITH LLVM-exception`・`CDLA-Permissive-2.0`・`BlueOak-1.0.0`・`CC-BY-4.0`・`Python-2.0` のまま。
IF ある節の検査する対象が見つからない（`web/node_modules` が無い・gradle が依存を解決できない） THEN THE SYSTEM SHALL その節を通さず、対象が無いことを示して検査を落とす。

導出元: NFR-26 ★ 2026-10-25（C-01 を含めて機械で確かめる。許可の外のプロプライエタリな依存が入ったら止める）。FR-127。深掘り Q2（第 1 回の推奨）。premises P1・P2・P3。review/deep.md R8 の C。review/spec.md R6・R7・R11・R16。D6（仮）・D7。深掘り Q12（第 2 回）。
壊してはいけないもの: 依存の許諾の検査の既存の許可一覧（`AGPL-3.0-only` を外すのは、自分の部品を名前で通す形に替えるためで、今の依存は 1 つも落ちない。P3）。

#### Scenario: 今の携帯の依存は、許諾の検査を通る
- **WHEN** 依存の許諾の検査の携帯の節を今の木に当てる
- **THEN** 検査の終了コードは 0

#### Scenario: 今の携帯の依存で、オープンソースでないとして通すのは名指しの 4 つだけ
- **WHEN** 依存の許諾の検査の携帯の節を今の木に当てる
- **THEN** オープンソースでない部品として通したと示されるものは、名指しの 4 つの座標だけ

#### Scenario: 名指しの外の Google のライブラリを足すと、許諾の検査が落ちる
- **GIVEN** 今の携帯の依存の一覧に、許諾が `Android Software Development Kit License` の `com.google.android.gms:play-services-ads` を 1 行足した偽の依存の一覧
- **WHEN** 依存の許諾の検査の携帯の節を当てる
- **THEN** 検査は 0 でない終了コードで終わり、`com.google.android.gms:play-services-ads` と許諾の名前が示される

#### Scenario: 対応表に無い許諾の名前の部品があれば、許諾の検査が落ちる
- **GIVEN** POM の許諾の名前が対応表に無い部品を 1 つ持つ、偽の依存の一覧
- **WHEN** 依存の許諾の検査の携帯の節を当てる
- **THEN** 検査は 0 でない終了コードで終わり、その部品の座標と許諾の名前が示される

#### Scenario: 許諾の名前を 2 つ持つ部品は、片方でも許可の外なら許諾の検査が落ちる
- **GIVEN** 許諾の名前が `The Apache Software License, Version 2.0` と `Android Software Development Kit License` の 2 つを持つ、名指しの外の部品の偽の依存の一覧
- **WHEN** 依存の許諾の検査の携帯の節を当てる
- **THEN** 検査は 0 でない終了コードで終わり、その部品の座標と `Android Software Development Kit License` が示される

#### Scenario: 許諾の名前を 2 つ持つ部品は、両方が許可されていれば通る
- **GIVEN** 許諾の名前が `The Apache Software License, Version 2.0` と `The MIT License` の 2 つを持つ部品の、偽の依存の一覧
- **WHEN** 依存の許諾の検査の携帯の節を当てる
- **THEN** 検査の終了コードは 0

#### Scenario: POM が許諾を持たない部品は、親の POM の許諾で判定する
- **GIVEN** 許諾を持たない POM と、Apache-2.0 の許諾を持つその親の POM を置いた、使い捨ての maven の置き場から解決する依存
- **WHEN** 携帯の依存の POM を書き出し、依存の許諾の検査の携帯の節を当てる
- **THEN** その部品は親から読んだ許諾として示され、Apache-2.0 として通る

#### Scenario: 名指しの一覧を正本から消すと、その部品で許諾の検査が落ちる
- **GIVEN** 正本のファイルの `Listed libraries:` から `play-services-tasks` の行を消した使い捨ての正本と、`play-services-tasks` を持つ偽の依存の一覧
- **WHEN** 依存の許諾の検査の携帯の節を当てる
- **THEN** 検査は 0 でない終了コードで終わり、`com.google.android.gms:play-services-tasks` が示される

#### Scenario: 他人の部品の AGPL の許諾は、許諾の検査が落とす
- **GIVEN** 許諾が `AGPL-3.0-only` の他人の crate を 1 つ持つ、偽の Rust の依存の一覧
- **WHEN** 依存の許諾の検査の Rust の節を当てる
- **THEN** 検査は 0 でない終了コードで終わり、その crate の名前が示される

#### Scenario: 自分の部品は、決めた許諾の式で許諾の検査を通る
- **WHEN** 依存の許諾の検査の Rust と画面の節を今の木に当てる
- **THEN** 検査の終了コードは 0 で、workspace の crate と画面の根は `AGPL-3.0-only WITH AdditionRef-ashiato-play-services` として通る

#### Scenario: 許可の一覧は、AGPL-3.0-only を外したほかは前と同じ
- **WHEN** 依存の許諾の検査の許可の一覧を読む
- **THEN** 一覧はこの Requirement に並べた 17 の値とちょうど同じで、`AGPL-3.0-only` を持たない

#### Scenario: POM を取り寄せられない部品があれば、許諾の検査が落ちる
- **GIVEN** 使い捨ての maven の置き場に、`.module` と jar はあるが `.pom` が無い部品を 1 つ足した依存（依存の解決は通る）を、ネットワークを使わずに解決する環境
- **WHEN** 携帯の依存の POM を書き出し、依存の許諾の検査の携帯の節を当てる
- **THEN** 検査は 0 でない終了コードで終わり、その部品の座標と、POM を取れないことが示される（依存の解決の失敗ではない）

#### Scenario: 検査する対象が無い節は、通さずに落ちる
- **GIVEN** gradle が依存を解決できない（携帯の組み立ての道具が無い）環境
- **WHEN** 依存の許諾の検査の携帯の節を当てる
- **THEN** 検査は 0 でない終了コードで終わり、対象が無いことが示される

#### Scenario: node_modules が無ければ、画面の節は通さずに落ちる
- **GIVEN** `web/node_modules` が無い木
- **WHEN** 依存の許諾の検査の画面の節を当てる
- **THEN** 検査は 0 でない終了コードで終わり、対象が無いことが示される

### Requirement: 配布物は、本体の許諾・依存の許諾の一覧・対応するソースの場所を持つ

THE SYSTEM SHALL サーバの Linux 版・Windows 版、PC の収集アプリ、携帯の APK の 4 つの配布物に、次の 3 つのファイルを入れる。tar / zip では根に、APK では `assets/licenses/` に置く。
- `LICENSE`: 木の `LICENSE` と同じ中身（追加の許可を含む）
- `THIRD-PARTY-NOTICES.txt`: その配布物に入る依存の一覧。部品ごとに `== <名前> <版>` の見出し・`License: <式>` の行・許諾の全文と著作権の表示を書く
- `SOURCE.txt`: 対応するソースの場所
THE SYSTEM SHALL `THIRD-PARTY-NOTICES.txt` の依存に、次のものを入れる。
- サーバ: 実行ファイル（Windows 版はサーバと更新係の `ashiato-updater.exe` の両方）に入る Rust の部品、組み込んだ画面に入る Node の部品（開発だけの部品を除く）、Rust の標準ライブラリ（`rust-std`）
- PC の収集アプリ: 実行ファイルに入る Rust の部品と、Rust の標準ライブラリ（`rust-std`）
- 携帯の APK: `releaseRuntimeClasspath` の部品と、名指しの Google のライブラリが中に持つ第三者の一覧（各 aar の `third_party_licenses.txt` の中身。見出し `(gms の中の第三者)`）
IF ある部品の許諾の全文も著作権の表示も見つからない THEN THE SYSTEM SHALL 依存の一覧を組まずに、組み立てを落とす。
THE SYSTEM SHALL `SOURCE.txt` に、リポジトリの場所を書く。正式版・ベータ版の公開の組み立てでは、それに加えて印 `v<版番号>` を書く。それ以外の組み立てでは、組み立てた木の 40 桁の commit を書く。開発版と main への push の組み立てでは main の commit、PR の組み立てでは PR の head の commit になる。
THE SYSTEM SHALL PR の組み立てを、PR の head の commit そのものから組む（GitHub が作る、head とその時の main を合わせた merge の commit からは組まない）。`SOURCE.txt` の commit・組み立ての印・組んだソースが同じ commit を指す。
THE SYSTEM SHALL 配布物の検査で、期待する版番号と、配布物の `LICENSE` と比べる `LICENSE` を、配布物を組んだのと同じ commit の中身から取る。PR の CI の配布物を検査する job は、ほかの job と同じく head とその時の main を合わせた木で走り、期待値だけを PR の head の commit（main への push では push した commit）から取る。
IF 公開の組み立てに渡された印が `v<版番号>` と違う THEN THE SYSTEM SHALL 配布物を作らずに組み立てを落とす。
THE SYSTEM SHALL 3 つのファイルを組み立ての台本の中で入れる。正式版・ベータ版だけでなく、開発版・main への push・PR の組み立てにも同じ手順で入れる。
IF 配布物のどれかに 3 つのファイルのどれかが無い、`LICENSE` が期待する `LICENSE`（配布物を組んだ commit の `LICENSE`）と違う、サーバと PC の収集アプリの一覧に `rust-std` の項が無い、または携帯の APK の一覧に Google のライブラリの第三者の一覧が無い THEN THE SYSTEM SHALL 配布物の検査を落とし、その配布物を公開しない。
THE SYSTEM SHALL 配布物の検査で、`SOURCE.txt` を次の決まりで見る。合わなければ検査を落とす。
- 正式版・ベータ版として公開する配布物の検査（debug の鍵を許さない、チャンネルが `release` か `beta` の検査）: `v<期待する版番号>`
- 開発版の検査（チャンネルが `nightly` の検査）と、debug の鍵を許す検査（PR・main への push の組み立てと下書きの配布物の検査）: 組み立ての印と同じ 40 桁の commit

導出元: FR-127（本体の許諾と依存の許諾の一覧を同梱する。対応するソースは同じ印のリポジトリで示す）。深掘り Q6・Q7（第 1 回の推奨）。review/deep.md R5・R6・R7・R10 の C。review/spec.md R1・R2・R5・R11・R16。D8・D9・D16・D18。深掘り Q13・Q14・Q15（第 2 回）・Q16（第 3 回）・Q17（第 4 回）。FR-127 ★ 2026-11-02・★ 2026-11-03・★ 2026-11-03（第 3 回）。
★ ST57: サーバの Windows 版の一覧に、更新係の実行ファイルに入る Rust の部品を足した（review/spec.md R11）。
動く印 `nightly` は翌日に別の commit を指すので、開発版の写しには動かない commit を書く（Q7）。配った写しに書いた場所は、後から直せない。

#### Scenario: 4 つの配布物は、許諾・依存の一覧・ソースの場所を持つ
- **WHEN** 配る形の 4 つの配布物の置き場に、配布物の検査を当てる
- **THEN** 4 つのどれにも `LICENSE`・`THIRD-PARTY-NOTICES.txt`・`SOURCE.txt` があり（携帯の APK は `assets/licenses/` の下）、検査の終了コードは 0

#### Scenario: 配布物の LICENSE が木の LICENSE と違えば、公開しない
- **GIVEN** サーバの Linux 版の tar の `LICENSE` を、AGPL の全文だけのものに差し替えた配布物の置き場
- **WHEN** 配布物の検査を当てる
- **THEN** 検査は 0 でない終了コードで終わり、`LICENSE` が違う配布物の名前が示される

#### Scenario: 許諾のファイルが欠けた配布物は、公開しない
- **GIVEN** サーバの Linux 版の tar から `THIRD-PARTY-NOTICES.txt` を抜いた配布物の置き場
- **WHEN** 配布物の検査を当てる
- **THEN** 検査は 0 でない終了コードで終わり、欠けたファイルと配布物の名前が示される

#### Scenario: Rust の実行ファイルの依存の一覧は、標準ライブラリを含む
- **WHEN** サーバと PC の収集アプリの配布物の `THIRD-PARTY-NOTICES.txt` を読む
- **THEN** どちらにも `== rust-std ` の見出しの項（`License: MIT OR Apache-2.0`）がある

#### Scenario: サーバの依存の一覧は、画面に入る Node の部品を含み、開発だけの部品を含まない
- **WHEN** サーバの配布物の `THIRD-PARTY-NOTICES.txt` を読む
- **THEN** `react` の項があり、`vitest` の項は無い

#### Scenario: 許諾の全文も著作権の表示も見つからない部品があれば、組み立てが落ちる
- **GIVEN** 許諾のファイルも作者の欄も持たず、許諾の式の全文の写しも無い部品を 1 つ持つ、偽の依存の一覧
- **WHEN** 依存の一覧を組む
- **THEN** 組む手順は 0 でない終了コードで終わり、その部品の名前が示され、依存の一覧のファイルはできない

#### Scenario: 携帯の APK の依存の一覧は、Google のライブラリの中の第三者を含む
- **WHEN** 携帯の APK の `assets/licenses/THIRD-PARTY-NOTICES.txt` を読む
- **THEN** `play-services-location` の aar の `third_party_licenses.txt` から取り出した、`(gms の中の第三者)` の見出しの項がある

#### Scenario: 正式版の配布物のソースの場所は、その版の印
- **GIVEN** 印 `v0.4.0` の公開の組み立て
- **WHEN** 配布物の `SOURCE.txt` を読む
- **THEN** `SOURCE.txt` はリポジトリの場所と印 `v0.4.0` を持つ

#### Scenario: 印が版番号と違う公開の組み立ては、配布物を作らずに落ちる
- **GIVEN** 版番号 `0.4.0` で、印として `v0.4.1` を渡した組み立て
- **WHEN** `SOURCE.txt` を組む
- **THEN** 手順は 0 でない終了コードで終わり、`SOURCE.txt` はできない

#### Scenario: 開発版と main への push の配布物のソースの場所は、組み立てた commit
- **GIVEN** main の commit を checkout した木で、印を渡さない組み立て（開発版か main への push の組み立て）
- **WHEN** 配布物の `SOURCE.txt` を読む
- **THEN** `SOURCE.txt` はリポジトリの場所と、組み立ての印と同じ 40 桁の commit を持ち、`nightly` の文字を持たない

#### Scenario: PR の組み立ての配布物のソースの場所は、PR の head の commit
- **GIVEN** main と合わせていない枝の commit（PR の head）を checkout した木で、印を渡さない組み立て
- **WHEN** 配布物の `SOURCE.txt` と組み立ての印を読む
- **THEN** `SOURCE.txt` はリポジトリの場所と、その head の 40 桁の commit を持ち、組み立ての印も同じ commit を持つ

#### Scenario: 組み立ての commit が 40 桁の 16 進でなければ、配布物を作らずに落ちる
- **GIVEN** 印を渡さず、組み立ての印の commit の上書き（`ASHIATO_BUILD_COMMIT`）に `main` を渡した組み立て
- **WHEN** `SOURCE.txt` を組む
- **THEN** 手順は 0 でない終了コードで終わり、`SOURCE.txt` はできない

#### Scenario: PR の配布物は、PR の head の commit から組まれ、同じ commit の期待値で検査される
- **WHEN** CI の配る形を組む job（サーバの Linux 版・携帯の APK と、Windows の 2 つ）と、その配布物を検査する job の checkout と検査の呼び出しを、workflow の検査で読む
- **THEN** 組む 2 つの job は、PR では PR の head の commit を、main への push では push した commit を checkout する
- **AND** 検査する job の作業の checkout は `ref` を持たない（合わせた木）
- **AND** 検査する job は、期待する版番号と比べる `LICENSE` を、組む 2 つの job と同じ commit を別の置き場に checkout したものから検査に渡す

#### Scenario: 配布物の LICENSE は、渡した期待する LICENSE と比べる
- **GIVEN** 台本の木の `LICENSE` と違う `LICENSE` のファイル（配布物を組んだ commit の `LICENSE` に当たる）と、その `LICENSE` を持つ配布物の置き場
- **WHEN** そのファイルを期待する `LICENSE` として渡して配布物の検査を当て、次に渡さずに当てる
- **THEN** 渡したときは検査の終了コードは 0、渡さないときは 0 でない終了コードで終わり、`LICENSE` が違うことが示される

#### Scenario: PR の組み立てと下書きの配布物の検査は、head の commit のソースの場所で通る
- **GIVEN** `SOURCE.txt` が組み立ての印と同じ 40 桁の commit（PR の head）を持つ配布物の置き場
- **WHEN** debug の鍵を許す配布物の検査（PR の組み立てと下書きの検査）を当てる
- **THEN** 検査の終了コードは 0

#### Scenario: debug の鍵を許す検査でも、組み立ての印の commit でないソースの場所の配布物は落ちる
- **GIVEN** `SOURCE.txt` に、commit でなく印 `v0.4.0` を持つ配布物の置き場と、`Commit: main` を持つ配布物の置き場と、7 桁の短い commit を持つ配布物の置き場と、組み立ての印と違う 40 桁の commit を持つ配布物の置き場
- **WHEN** それぞれに debug の鍵を許す配布物の検査を当てる
- **THEN** 4 つとも検査は 0 でない終了コードで終わり、`SOURCE.txt` が決まりと合わないことが示される

#### Scenario: ソースの場所がチャンネルの決まりと合わない配布物は、公開しない
- **GIVEN** `SOURCE.txt` に印でなく commit を持つ配布物の置き場
- **WHEN** `--channel beta --expect-version 0.4.0-beta.1` で配布物の検査を当てる
- **THEN** 検査は 0 でない終了コードで終わり、`SOURCE.txt` が決まりと合わないことが示される

#### Scenario: サーバの Windows 版の依存の一覧は、更新係だけが持つ部品を含む
- **WHEN** サーバの Windows 版の配布物の `THIRD-PARTY-NOTICES.txt` を読み、更新係の実行ファイルの依存の木と突き合わせる
- **THEN** 更新係だけが持つ暗号化した接続の部品（`rustls` など。サーバは持たない）の項がある

### Requirement: 正式な鍵を読む公開は、本人の承認を経る

THE SYSTEM SHALL 配布用の Environment `release` に、本人を必須の承認者として付け、管理者が承認を飛ばせない設定にする。承認されるまで、その Environment を使う公開の job は始まらず、正式な鍵も渡らない。
THE SYSTEM SHALL 本人が印を打って本人が承認する運用を止めない（自分の承認を禁じる設定を求めない）。
THE SYSTEM SHALL AI のエージェントが使う資格情報を、リポジトリの管理の権限と公開の承認の権限を持たないものにする。
THE SYSTEM SHALL 本人が鍵を置く手順の順を「エージェントの資格情報の切り替え → ルールセットと承認者 → 鍵」にする。
THE SYSTEM SHALL 守りの確かめの台本に、2 つの副コマンドを持たせる。どちらも、満たさなければ 0 でない終了コードで終わる。
- `approvals`: `release` に必須の承認者が付き、本人がその中にいて、管理者が承認を飛ばせないことを確かめる
- `agent-credential`: 今の資格情報がリポジトリの管理者でないことを確かめる
THE SYSTEM SHALL 正式版とベータ版の公開の job で、署名の前に `approvals` を通し、通らなければ署名も公開もしない。
THE SYSTEM SHALL 公開の job の中の確かめが守りの本体でないことを、手順の文書に書く。job の定義は印の commit の中の workflow で、書き換えられるため。守りの本体は、GitHub が job を始める前に効く承認である。

導出元: ST52 の IMP-01（印を push できる人が main を通さずに正式な鍵を読めるかもしれない。後の Story で扱う → ST53）。扉 34。深掘り Q3（第 1 回の推奨）。premises P7・P8・P11・P12。review/deep.md R4。D11（仮）。
一度読まれた鍵は、読まれなかったことにできない。鍵を替えると配った APK は上書きできなくなり、全員に入れ直しを頼むことになる（ST52 の P2）。
公開の承認の権限は API から読めないので、資格情報がそれを持たないことは確認バッチで確かめる（tasks の「人間の確認待ち」）。

#### Scenario: 承認者の無い配布用の Environment では、公開の前の確かめが落ちる
- **GIVEN** `release` に保護の規則が無い偽の配布元
- **WHEN** `approvals` の確かめを走らせる
- **THEN** 確かめは 0 でない終了コードで終わり、必須の承認者が無いことが示される

#### Scenario: 管理者が承認を飛ばせる設定では、公開の前の確かめが落ちる
- **GIVEN** `release` に本人が必須の承認者として付いているが、管理者が承認を飛ばせる偽の配布元
- **WHEN** `approvals` の確かめを走らせる
- **THEN** 確かめは 0 でない終了コードで終わり、管理者が承認を飛ばせることが示される

#### Scenario: 本人が承認者で管理者が飛ばせなければ、公開の前の確かめを通る
- **GIVEN** `release` に本人が必須の承認者として付き、管理者が承認を飛ばせず、自分の承認を禁じる設定が無い偽の配布元
- **WHEN** `approvals` の確かめを走らせる
- **THEN** 確かめの終了コードは 0

#### Scenario: 本人が承認者にいなければ、公開の前の確かめが落ちる
- **GIVEN** `release` の必須の承認者が本人でない利用者だけの偽の配布元
- **WHEN** `approvals` の確かめを走らせる
- **THEN** 確かめは 0 でない終了コードで終わる

#### Scenario: 管理者の資格情報では、エージェントの資格情報の確かめが落ちる
- **GIVEN** リポジトリの権限が `admin: true` と返る偽の配布元
- **WHEN** `agent-credential` の確かめを走らせる
- **THEN** 確かめは 0 でない終了コードで終わり、管理者の権限があることが示される

#### Scenario: 管理者でない資格情報では、エージェントの資格情報の確かめを通る
- **GIVEN** リポジトリの権限が `admin: false`・`maintain: false`・`push: true` と返る偽の配布元
- **WHEN** `agent-credential` の確かめを走らせる
- **THEN** 確かめの終了コードは 0

#### Scenario: 公開の job は、署名の前に承認者を確かめる
- **WHEN** 守りの確かめで、公開の workflow を読む
- **THEN** 公開の job の手順で、`check-guards.sh approvals` が署名の手順（`sign-apk.sh`）より前にある。そうでなければ確かめは 0 でない終了コードで終わる

### Requirement: インストーラは、子の命令の出力を窓のコード頁で読み、Windows の文字コードで表せる名前なら英字でなくても止まらない

WHEN インストーラが子の命令（`sc.exe`・`icacls`・`whoami`・`reg.exe`・`tasklist`・`netstat`・PowerShell・同梱の `psql`・`ashiato-server migrate` など）の出力を読む THE SYSTEM SHALL 出力を、子が書いた文字として読む（日本語の Windows の窓のコード頁 932 で書かれた出力も、UTF-8 で書く子の出力も、化けさせない）。
THE SYSTEM SHALL 子の命令が失敗したときに添える出力の末尾を、子が書いた文のまま示す。
THE SYSTEM SHALL 実行した利用者の名前（`repair` が DB の認証の対応表に使う）と、ダウンロードの場所（指定が無いときにレジストリから読む）を、英字でない文字を含んでも、Windows が持つ名前と同じ文字として読む。
THE SYSTEM SHALL 英字だけの出力は、いまと同じ値として読む。
THE SYSTEM SHALL `repair` の DB の認証の対応表の利用者名の行を、英字でない文字を含むときは Windows の文字コード（ANSI のコード頁）のバイト列で、文字どおりに（正規表現でなく）書く。英字だけの名前の行はいまと同じに書く。
IF 利用者名が Windows の文字コードで表せない THEN THE SYSTEM SHALL 合わない行を書かずに、表せないことを理由に示して `repair` を止める。
THE SYSTEM SHALL UTF-8 としても窓のコード頁としても読めない並びは置換の文字にして読み、それを理由に止まらない（いまと同じ）。

導出元: CH14（深掘り Q2。design D2・D8・D9）。
2026-11-09、本人の PC（コード頁 932）で、`sc.exe`・`icacls`・`tasklist`・`reg.exe`・`whoami` の管の出力が cp932 のバイト列で、UTF-8 として読むと化けることを確かめた（`premises.md`）。利用者名やフォルダ名に日本語があると、`repair` が DB に入れず、インストールがダウンロードの場所を読めずに止まる見込みだった。

#### Scenario: 窓のコード頁で書いた子の命令の失敗の理由が、化けずに読める

- **WHEN** 失敗した子の命令の出力が、コード頁 932 の窓で書いた「指定されたサービスはインストールされたサービスとして存在しません。」のバイト列で、インストーラの窓のコード頁が 932 である
- **THEN** 失敗の行に添える出力の末尾に、その文が同じ文字のまま入り、置換の文字（U+FFFD）が無い

#### Scenario: UTF-8 で書く子の出力は、窓のコード頁が 932 でもそのまま読める

- **WHEN** 子の命令（`ashiato-server migrate` など）が日本語を含む失敗の行を UTF-8 で書き、それをコード頁 932 を指して読む
- **THEN** 失敗の行に添える出力の末尾に、その行が同じ文字のまま入る

#### Scenario: どのコード頁でも読めない並びは、置換の文字にして止まらない

- **WHEN** 子の命令の失敗の出力が、後ろが欠けた cp932 の並び（2 バイトの文字の 1 バイト目 `0x82` で終わる）で、それをコード頁 932 を指して読む
- **THEN** 読みは止まらずに文字列を返し、欠けた所は置換の文字（U+FFFD）になり、その前の文字はそのまま残る

#### Scenario: 英字だけの子の出力は、いまと同じ値として読む

- **WHEN** 英字だけの `whoami` の利用者名（`pc\owner`）・`reg.exe` のダウンロードの場所・`sc.exe` の出力を、コード頁 932 と 437 のそれぞれで読む
- **THEN** 読んだ値は、UTF-8 として読んだいまの値と同じ

#### Scenario: コード頁 932 で書いた日本語の利用者名を、同じ文字として読む

- **WHEN** `whoami` の出力がコード頁 932 で書いた `pc\足跡太郎` で、それをコード頁 932 で読む
- **THEN** 利用者名は `pc\足跡太郎` として読まれ、`repair` の DB の認証の対応表の行は `足跡太郎` の文字を持つ

#### Scenario: コード頁 932 で書いた日本語のダウンロードの場所を、同じ文字として読む

- **WHEN** `reg.exe query` の出力がコード頁 932 で書いた `%USERPROFILE%\ダウンロード` の値で、それをコード頁 932 で読む
- **THEN** ダウンロードの場所は本人のプロファイルの下の `ダウンロード` として読まれる

#### Scenario: 英字でない利用者名の対応表の行は、Windows の文字コードのバイト列で、文字どおりに書く

- **WHEN** 利用者名 `pc\足跡表ソ`（`表`・`ソ` は cp932 の 2 バイト目が `\`）の対応表の行を、Windows の文字コード 932 で組む
- **THEN** 行は `repair "足跡表ソ@pc" ashiato` を cp932 で符号化したバイト列そのもので、正規表現の行（`/(?i)`）ではない

#### Scenario: Windows の文字コードで表せない利用者名なら、repair は理由を示して止まる

- **WHEN** 利用者名 `pc\足跡` の対応表の行を、Windows の文字コード 1252 で組む
- **THEN** 行は組まれず、表せないことと文字コードの番号 1252 を理由に示して止まる

#### Scenario: 英字でない名前の管理者でも repair でき、決めた合言葉でログインできる

- **WHEN** 名前に英字でない文字を含む管理者が、設定を退けて止まったインストールに `repair` を走らせ、合言葉を決める
- **THEN** `repair` は rc=0 で終わり、決めた合言葉で画面にログインできる

### Requirement: サーバの更新係・携帯・PC の収集アプリが、それぞれ配布元に直に新しい版を確かめる

WHILE その部品の更新の確認が有効である THE SYSTEM SHALL サーバの更新係・携帯の収集アプリ・PC の収集アプリのそれぞれに、自分の版の新しい版があるかを配布元（`api.github.com` のリポジトリ `rai-256/ashiato` のリリース）へ自分で問い合わせさせる。
THE SYSTEM SHALL 携帯と PC の収集アプリに、新しい版があるかをサーバに問い合わせさせない（サーバが止まっていても、収集アプリは自分の新しい版を知る）。
THE SYSTEM SHALL 1 つの部品の確認を切っても、ほかの部品の確認を止めない。

導出元: FR-112・FR-115。深掘り Q1（推奨: 要件どおり、3 つがそれぞれ直に聞く）。
配布元からは、問い合わせの元の IP と時刻が見える（携帯の IP は繋いでいる網で変わる）。中身は版の一覧の取得だけで、記録は出ない（下の「更新を確かめる問い合わせは…」）。

#### Scenario: PC の収集アプリは新しい版をサーバに問い合わせない

- **GIVEN** サーバが止まっていて、手元の受け手（試験の配布元）が新しい版 `0.5.0` のリリースを返す
- **WHEN** 版 `0.4.0` の PC の収集アプリが確かめる時を迎える
- **THEN** 受け手に 1 回問い合わせて「更新あり 0.5.0」を出し、サーバの宛先への問い合わせは 0 件である

#### Scenario: 携帯の収集アプリは新しい版をサーバに問い合わせない

- **GIVEN** サーバが止まっていて、手元の受け手（試験の配布元）が新しい版 `0.5.0` のリリースを返す
- **WHEN** 版 `0.4.0` の携帯の収集アプリが確かめる時を迎える
- **THEN** 受け手に 1 回問い合わせて「更新あり 0.5.0」を出し、サーバの宛先への問い合わせは 0 件である

#### Scenario: サーバの確認を切っても、収集アプリは確かめる

- **GIVEN** サーバの更新の確認を切ってある
- **WHEN** PC の収集アプリが確かめる時を迎える
- **THEN** PC の収集アプリは受け手に 1 回問い合わせる

### Requirement: 更新を確かめる問い合わせは、版の一覧を取る決まった形だけを配布元へ送る

THE SYSTEM SHALL 3 つの部品の更新の問い合わせを、方法 `GET`・本文なしにし、道を次の 2 つに限る: 開発版でない部品は `/repos/rai-256/ashiato/releases?per_page=20`、開発版の部品は `/repos/rai-256/ashiato/releases/tags/nightly`。
THE SYSTEM SHALL 問い合わせの見出しを、`Host`・`User-Agent`・`Accept`・`Accept-Encoding`・`Connection` の 5 つの名前の内側に限り、`User-Agent` の値を `ashiato-update-check` に、`Accept` の値を `application/vnd.github+json` に固定する。
THE SYSTEM SHALL 問い合わせのどこ（道・見出し・本文）にも、自分の版番号・端末の機種・OS の版・端末や利用者の識別子・資格情報・記録を入れない。
THE SYSTEM SHALL 転送（3xx）は同じ `api.github.com` の中のものだけを追い、ほかの宛先への転送は「確かめられない」として扱う。
THE SYSTEM SHALL 暗号化した接続の証明書を OS の持つ根で確かめ、更新係と PC の収集アプリに根の一覧の部品を同梱しない。
THE SYSTEM SHALL 宛先を組み立てのときの定数にし、配る組み立て（サーバの更新係・PC の収集アプリの配る zip、携帯の配る APK）には宛先を実行時に変える口を持たせない。
THE SYSTEM SHALL 宛先を差し替える口を試験の組み立てにだけ持たせ、試験の組み立ては差し替えの宛先が無ければ確かめない（自動の試験は本物の配布元に問い合わせない。配る組み立てを動かす CI は、外への接続を塞いで走らせる）。

導出元: FR-116（機械の検査で固定する。要件の Q7）。聞かない既定 C2・C8。premises P1〜P3・P10。
GitHub の API は `User-Agent` の無い問い合わせを断る（P3）ので、見出しを 1 つは送る。`Connection` は Android の通信の部品が自分で足すことがある（design D13（仮）。聞かない既定 C2 の 4 つに足した）。
自分の版は送らず、取った一覧を手元で選ぶ。配布元から見分けられるのは開発版かどうか（道の違い）だけで、開発版の印 `nightly` は公開されている。

#### Scenario: 更新係の問い合わせは版の一覧の取得だけである

- **GIVEN** 宛先を手元の受け手に差し替えた試験の組み立ての更新係（版 `0.4.0`）
- **WHEN** 更新係が 1 回確かめ、受け手が受けた問い合わせを全部読む
- **THEN** 問い合わせは 1 件で、方法は `GET`、道は `/repos/rai-256/ashiato/releases?per_page=20`、本文は 0 バイト、見出しの名前は許した 5 つの内側、`User-Agent` は `ashiato-update-check`、`Accept` は `application/vnd.github+json` で、どの見出しの値にも `0.4.0` が無い

#### Scenario: PC の収集アプリの問い合わせは版の一覧の取得だけである

- **GIVEN** 宛先を手元の受け手に差し替えた試験の組み立ての PC の収集アプリ（版 `0.4.0`・端末の識別子 `pc-0123456789ab`）
- **WHEN** PC の収集アプリが 1 回確かめ、受け手が受けた問い合わせを全部読む
- **THEN** 問い合わせは 1 件で、方法・道・本文・見出しの名前と値は更新係と同じ決まりのとおりで、どの見出しの値にも `0.4.0` と `pc-0123456789ab` と資格情報が無い

#### Scenario: 携帯の問い合わせは版の一覧の取得だけである

- **GIVEN** 宛先を端末の中の受け手に差し替えた試験の組み立ての携帯の収集アプリ（本物の Android の上）
- **WHEN** 携帯の収集アプリが 1 回確かめ、受け手が受けた問い合わせを全部読む
- **THEN** 問い合わせは 1 件で、方法・道・本文・見出しの名前と値は更新係と同じ決まりのとおりで、どの見出しの値にも端末の機種名（`Build.MODEL`）・OS の版（`Build.VERSION.RELEASE`）・アプリの版・資格情報が無い

#### Scenario: 更新係と PC の収集アプリは根の一覧を同梱しない

- **WHEN** 更新係と PC の収集アプリを組み立てる部品の一覧を、同梱の根の一覧の部品（一覧は検査の側が持つ）と突き合わせる
- **THEN** 一致する部品は 0 件である

#### Scenario: 開発版の部品は印 nightly の 1 件だけを取る

- **GIVEN** 宛先を手元の受け手に差し替えた、版 `0.4.0-dev.20261102` の更新係
- **WHEN** 更新係が 1 回確かめる
- **THEN** 受け手が受けた道は `/repos/rai-256/ashiato/releases/tags/nightly` だけである

#### Scenario: 別の宛先への転送は追わない

- **GIVEN** 手元の受け手が `301` で `api.github.com` でない宛先へ転送を返す
- **WHEN** 更新係が確かめる
- **THEN** 更新係は転送先へ問い合わせず、その回を「確かめられない」（理由: 読めない答え）として記録する

#### Scenario: 配る組み立ては宛先を変える口を持たない

- **WHEN** 配る形でサーバの Windows の配布物と PC の収集アプリの配布物を組み、`ashiato-updater.exe` と `ashiato-collector-windows.exe` の中の文字列を読む
- **THEN** 宛先を差し替える口の名前（`ASHIATO_UPDATER_TEST_`・`ASHIATO_COLLECTOR_TEST_UPDATE_URL`）が無く、宛先 `https://api.github.com/repos/rai-256/ashiato/releases` がある

#### Scenario: 携帯の配る APK の宛先は配布元である

- **WHEN** 配る形で携帯の APK を組み、組み立ての定数の更新の宛先を読む
- **THEN** 宛先は `https://api.github.com/repos/rai-256/ashiato/releases` で、配る組み立てに宛先の差し替え（`-P`）を渡すと組み立ては 0 でない終了コードで落ちる

#### Scenario: 試験の組み立ては差し替えの宛先が無ければ確かめない

- **GIVEN** 宛先を差し替えていない試験の組み立ての PC の収集アプリ
- **WHEN** 確かめる時を迎える
- **THEN** 外への問い合わせは 0 件で、確かめた記録に「宛先なし」が 1 行ある

### Requirement: 新しい版かどうかは、入っている版のチャンネルで決める

THE SYSTEM SHALL 配布元の一覧から、次の規則で「見つけた最新の版」を選ぶ:
正式版（`<MAJOR>.<MINOR>.<PATCH>`）の部品は、印が `v<MAJOR>.<MINOR>.<PATCH>` のリリースだけを数える。
ベータ版（`<X.Y.Z>-beta.<N>`）の部品は、土台の版によらず、印が `v<A.B.C>` か `v<A.B.C>-beta.<M>` の形のリリースを数える。
開発版（`<版>-dev.<日付>`）の部品は、印 `nightly` のリリースだけを数える。
THE SYSTEM SHALL ベータ版を、先行公開の旗でなく印の形 `v<版>-beta.<N>` で見分ける（開発版のリリースも先行公開で作られる）。
THE SYSTEM SHALL 下書きのリリース（`draft: true`）を数えない。
THE SYSTEM SHALL 版を、自分の部品の配布物の名前（`ashiato-server-<版>-windows-x86_64.zip`・`ashiato-collector-windows-<版>-x86_64.zip`・`ashiato-collector-<版>.apk`、開発版の携帯は `ashiato-collector-dev-<版>.apk`）から読み、自分の部品の配布物が無いリリースを数えない。
THE SYSTEM SHALL 数えたリリースのうち版番号の決まり（SemVer。開発版も土台の版から比べる）で最も新しい版を「見つけた最新の版」とし、それが自分の版より新しいときだけ「更新あり」にする（自分の版が一覧のどれより新しければ「更新なし」）。
THE SYSTEM SHALL 空の一覧と、数えるリリースが 1 つも無い一覧を「確かめた。新しい版なし」として扱う。

導出元: FR-112（「利用者のチャンネル」）。深掘り Q2（推奨: 入っている版のチャンネルで決める）。premises P4〜P7。
ベータ版の人が正式版を入れると、以後はベータ版を知らせない。開発版は配布物を入れ替える途中で 2 つの版の配布物が並ぶことがある（`tools/release/publish-nightly.sh`）ので、自分の配布物の名前の版だけを読み、並んでいれば新しい方を取る。
選び方は 3 つの部品で同じで、共有の材料（`docs/contracts/update-check-vectors.json`）で試す（design D3）。

#### Scenario: 正式版の部品は、より新しい正式版だけを更新ありにする

- **GIVEN** 一覧に `v0.5.0-beta.1`（先行公開）と `v0.4.1`（正式版）があり、どちらも 4 つの配布物を持つ
- **WHEN** 版 `0.4.0` の部品が一覧から選ぶ
- **THEN** 見つけた最新の版は `0.4.1` で、「更新あり 0.4.1」になる

#### Scenario: ベータ版の部品は、より新しいベータ版と正式版を更新ありにする

- **GIVEN** 一覧に `v0.4.0-beta.3` と `v0.4.0` がある
- **WHEN** 版 `0.4.0-beta.2` の部品が一覧から選ぶ
- **THEN** 見つけた最新の版は `0.4.0` で、「更新あり 0.4.0」になる

#### Scenario: ベータ版の部品は、土台の版の上がったベータ版も更新ありにする

- **GIVEN** 一覧で最も新しいのが `v0.5.0-beta.1` で、ほかは `v0.4.0` 以下
- **WHEN** 版 `0.4.0-beta.2` の部品が一覧から選ぶ
- **THEN** 見つけた最新の版は `0.5.0-beta.1` で、「更新あり 0.5.0-beta.1」になる

#### Scenario: 正式版を入れた後は、ベータ版を知らせない

- **GIVEN** 一覧に `v0.5.0-beta.1` だけが新しい
- **WHEN** 版 `0.4.0` の部品が一覧から選ぶ
- **THEN** 「更新なし」になる

#### Scenario: 開発版の部品は、印 nightly の配布物の版だけを見る

- **GIVEN** 印 `nightly` のリリースが配布物 `ashiato-server-0.4.0-dev.20261110-windows-x86_64.zip` を持ち、一覧に正式版 `v0.5.0` もある
- **WHEN** 版 `0.4.0-dev.20261102` の更新係が選ぶ
- **THEN** 見つけた最新の版は `0.4.0-dev.20261110` で、正式版 `0.5.0` を数えない

#### Scenario: 開発版は土台の版が上がった版を新しいとみなす

- **GIVEN** 印 `nightly` のリリースが `ashiato-server-0.5.0-dev.20261101-windows-x86_64.zip` を持つ
- **WHEN** 版 `0.4.0-dev.20261110` の更新係が選ぶ
- **THEN** 見つけた最新の版は `0.5.0-dev.20261101` で、「更新あり」になる

#### Scenario: 開発版の入れ替えの途中で 2 つの版が並べば、新しい方を取る

- **GIVEN** 印 `nightly` のリリースが `ashiato-collector-dev-0.4.0-dev.20261102.apk` と `ashiato-collector-dev-0.4.0-dev.20261110.apk` を持つ
- **WHEN** 版 `0.4.0-dev.20261102` の携帯の収集アプリが選ぶ
- **THEN** 見つけた最新の版は `0.4.0-dev.20261110` である

#### Scenario: 下書きのリリースは数えない

- **GIVEN** 一覧に `draft: true` の `v0.5.0` がある
- **WHEN** 版 `0.4.0` の部品が一覧から選ぶ
- **THEN** 「更新なし」になる

#### Scenario: 開発版の先行公開をベータ版に数えない

- **GIVEN** 一覧に先行公開の印 `nightly` のリリース（配布物の版 `0.5.0-dev.20261110`）だけがある
- **WHEN** 版 `0.4.0-beta.2` の部品が一覧から選ぶ
- **THEN** 「更新なし」になる

#### Scenario: 自分の部品の配布物が無いリリースは数えない

- **GIVEN** 一覧の `v0.5.0` が PC の収集アプリの配布物を持たない
- **WHEN** 版 `0.4.0` の PC の収集アプリが一覧から選ぶ
- **THEN** 「更新なし」になる

#### Scenario: 自分の版が一覧より新しければ更新なし

- **GIVEN** 一覧で最も新しい正式版が `v0.4.0`
- **WHEN** 手で組んだ版 `0.5.0` の部品が一覧から選ぶ
- **THEN** 「更新なし」になる

#### Scenario: 空の一覧は確かめた・新しい版なしになる

- **GIVEN** 配布元が `200` で `[]` を返す
- **WHEN** 部品が確かめる
- **THEN** その回は「確かめた」として記録され、「更新なし」になる

#### Scenario: 3 つの部品は同じ材料で同じ版を選ぶ

- **WHEN** 共有の材料の全件（選び方と時期の判定）を、更新係と PC の収集アプリの選び方と、携帯の収集アプリの選び方に通す
- **THEN** どの件も、材料に書いた「見つけた最新の版」と「更新あり／なし」に一致する

### Requirement: 確かめるのは最後に確かめてから 24 時間が過ぎた後の最初の機会で、最初の問い合わせは切る口に触れた後にする

THE SYSTEM SHALL 各部品に、最後に確かめた時（成功でも失敗でも）から 24 時間が過ぎた後の最初の機会（起動した時か、部品の定期の時）に 1 回だけ確かめさせる。
IF 最後に確かめた時刻が今より未来である THEN THE SYSTEM SHALL 期限が来たとみなして確かめる。
THE SYSTEM SHALL 失敗した回も「確かめた」に数え、同じ 24 時間の中で問い合わせを繰り返さない。
THE SYSTEM SHALL 最初の問い合わせを、その部品の確認を切る口に本人が触れられるようになった後にだけ出す: サーバの更新係はインストールの最後に「更新の確認: 有効」と切り方を示した後、携帯と PC の収集アプリは送り先の設定（設定の文字列）を保存した後（組み立ての時に送り先を渡した試験の組み立ても、設定済みに数える）。
WHEN 送り先を設定済みの収集アプリが、更新の確認を持たない前の版から入れ替わって初めて起きる THE SYSTEM SHALL 「更新の確認: 有効」と切り方を本人に 1 回示し、示した印を立て、最初の問い合わせを示した後の機会にする。
THE SYSTEM SHALL 入れ替えた収集アプリが「示した」とみなすのを次の時だけにする: 携帯は、チャンネル `update` に通知を出せた時（通知の許可があり、アプリの通知が切られておらず、チャンネル `update` の重要度が「なし」でない。切り方の通知は、後の「新しい版」の通知で置き換えずに残す）か、本人がアプリの画面を開いて「更新の確認: 有効」と切り方が画面に出た時（最初の画面がその行をほかの行より上に組んで画面に出した時か、設定の画面が「新しい版を確かめる」のスイッチを送り先の入力欄より上に組んで画面に出した時。権限を求めている途中で画面を組まない間や、同じ回に別の画面を前に出した間や、確認の窓（前の回から残っている窓も含む）が出ている間に組み直した時は数えず、確認の窓が出ているときは、その窓が設定を前に出さずに閉じた時に数える）。PC は、トーストを出して成功が返った時。
WHILE 入れ替えた携帯がまだ示していない THE SYSTEM SHALL 最初の画面を、ランチャー以外から開かれた時（通知を押した時など）でも、入れ替えの行を出す間は閉じずに出す。THE SYSTEM SHALL 入れ替えの行を、示した印が無いか、その画面が開いている間に既に行を出したときに出し、示した後に新しく開いた最初の画面には出さない。
WHILE 入れ替えた収集アプリがまだ示していない THE SYSTEM SHALL 配布元に問い合わせず、PC は次の起動の時と、起動し直さなくても定期の時に示し直す。ただし定期の時は、画面がロックされておらず、最後のキーボード・マウスの入力からの経過が 5 分（300 秒）以内と読めた時だけ試み、それ以外の定期の時はトーストを試みない。携帯は通知を出せるようになった後の機会（収集のサービスの起動か定期の時）か、画面を開いた時に示す。どれでも示せない間は確認が止まったままになる（1 日 1 回の例外）。

導出元: FR-112（1 日 1 回。認証なしの問い合わせは IP ごとに 1 時間 60 回まで。premises P2）。聞かない既定 C5。review/deep.md R2・R4。review/spec.md R16（入れ替えの族）。深掘り Q11（本人の答え: 示す口を広げ、どれでも示せない間は問い合わせない。FR-112 ★ 2026-11-11。design D23）。深掘り Q12（本人の答え: 行を組んで画面に出した時だけ印を立てる。行は一番上、閉じずに出す。design D24）。別の画面や窓が前に出ているとき（design D29（仮）。はじめの D27 は別の席が否とした）。深掘り Q13（本人の答え: 定期の時は、ロックしておらず直前 5 分以内に入力があった時だけ出し直す。design D25）。入力の経過を読めない定期の時（design D28（仮））。深掘り Q14（本人の答え: 切り方の通知を更新ありと並べて残す。design D30）。
収集アプリは時刻サーバに合わせない（`tools/check-no-time-server.sh`）ので、端末の時計は戻りうる。戻ったときに何年も確かめない形にしない。進みすぎても 1 日に数回聞くだけで上限に遠い。

#### Scenario: 24 時間たたないうちは確かめない

- **GIVEN** 最後に確かめたのが 23 時間前の部品
- **WHEN** 起動するか、定期の時を迎える
- **THEN** 問い合わせは 0 件である

#### Scenario: 24 時間が過ぎた後の最初の機会に 1 回確かめる

- **GIVEN** 最後に確かめたのが 25 時間前の部品
- **WHEN** 定期の時を 2 回続けて迎える
- **THEN** 問い合わせは 1 件だけである

#### Scenario: 失敗した回も 24 時間の数えに入る

- **GIVEN** 1 時間前に確かめて配布元に届かなかった部品
- **WHEN** 定期の時を迎える
- **THEN** 問い合わせは 0 件である

#### Scenario: 最後に確かめた時刻が未来なら確かめる

- **GIVEN** 最後に確かめた時刻が今より 3 年先に記録された部品（時計が戻った）
- **WHEN** 定期の時を迎える
- **THEN** 問い合わせが 1 件出て、最後に確かめた時刻が今に書き直される

#### Scenario: PC の収集アプリは送り先を設定するまで確かめない

- **GIVEN** 送り先の設定を保存していない、入れたての PC の収集アプリ
- **WHEN** 起動し、定期の時を迎える
- **THEN** 外への問い合わせは 0 件で、設定を保存した後の最初の機会に 1 件出る

#### Scenario: 携帯の収集アプリは送り先を設定するまで確かめない

- **GIVEN** 送り先の設定を保存しておらず、組み立ての時にも送り先を渡していない、入れたての携帯の収集アプリ
- **WHEN** 起動し、定期の時を迎える
- **THEN** 外への問い合わせは 0 件で、設定を保存した後の最初の機会に 1 件出る

#### Scenario: 確認の無い前の版から入れ替えた収集アプリは、切り方を示してから確かめる

- **GIVEN** 送り先を設定済みで、更新の確認を持たない前の版から入れ替えた PC の収集アプリ
- **WHEN** 入れ替えた後に起動し、定期の時を 2 回迎える
- **THEN** 起動の時に「更新の確認: 有効」と切り方（`update-check off`）の通知が 1 回出て、その起動の時には問い合わせが出ず、その後の最初の機会に 1 件出る

#### Scenario: 確認の無い前の版から入れ替えた携帯は、切り方を示してから確かめる

- **GIVEN** 送り先を設定済みで、更新の確認を持たない前の版から入れ替えた携帯の収集アプリ
- **WHEN** 入れ替えた後に収集のサービスが起き、定期の時を 2 回迎える
- **THEN** 起きた時に「更新の確認: 有効」と切り方（設定の画面のスイッチ）の通知が 1 回出て、その時には問い合わせが出ず、その後の最初の機会に 1 件出る

#### Scenario: 切り方の通知を出した後に更新ありの通知が出ても、切り方の通知は残る

- **GIVEN** 切り方を示した通知を出した、入れ替えた携帯の収集アプリ
- **WHEN** その後の最初の確認で新しい版を見つける
- **THEN** 通知の欄には「更新の確認: 有効」と切り方の通知と「新しい版 <版> が出ています」の通知がどちらも残る

#### Scenario: 入れ替えた PC の収集アプリは、トーストを出せない間は問い合わせない

- **GIVEN** 送り先を設定済みで、更新の確認を持たない前の版から入れ替えた PC の収集アプリで、トーストの出し手が失敗を返す
- **WHEN** 起動し、定期の時を 3 回迎え（24 時間ずつ時計を進める）、起動し直す
- **THEN** 外への問い合わせは 0 件で、示した印は立っておらず、起動し直した時にもう一度切り方のトーストを試みる

#### Scenario: 入れ替えた PC の収集アプリは、トーストを出せた後の機会に確かめる

- **GIVEN** 切り方のトーストを出せずに止まっている、入れ替えた PC の収集アプリ
- **WHEN** トーストの出し手が成功を返すようになってから起動し、定期の時を迎える
- **THEN** 起動の時に札 `update-intro` のトーストが 1 回出て、その時には問い合わせが出ず、その後の最初の機会に 1 件出る

#### Scenario: 入れ替えた PC の収集アプリは、起動し直さなくても定期の時に切り方のトーストを試み直す

- **GIVEN** 起動の時に切り方のトーストを出せなかった（出し手が失敗を返した）、入れ替えた PC の収集アプリ
- **WHEN** 起動し直さずに、出し手が成功を返すようになってから、画面がロックされておらず最後の入力から 60 秒の定期の時を 2 回迎える
- **THEN** 1 回目の定期の時に札 `update-intro` のトーストが 1 回出て、その時には問い合わせが出ず、2 回目の定期の時に 1 件出る

#### Scenario: 入れ替えた携帯は、通知を出せず画面も開かなければ問い合わせない

- **GIVEN** 送り先を設定済みで、更新の確認を持たない前の版から入れ替えた携帯の収集アプリで、通知の許可が無い
- **WHEN** 収集のサービスが起き、定期の時を 3 回迎える（24 時間ずつ時計を進める）
- **THEN** 外への問い合わせは 0 件で、示した印は立っていない

#### Scenario: 入れ替えた携帯は、チャンネル update だけを切ってあっても示したとみなさない

- **GIVEN** 送り先を設定済みで、更新の確認を持たない前の版から入れ替えた携帯の収集アプリで、通知の許可はあり、チャンネル `update` の重要度だけが「なし」
- **WHEN** 収集のサービスが起き、定期の時を 2 回迎える
- **THEN** 外への問い合わせは 0 件で、示した印は立っていない

#### Scenario: 入れ替えた携帯は、画面に切り方を出した後の機会に確かめる

- **GIVEN** 通知を出せずに止まっている、入れ替えた携帯の収集アプリ
- **WHEN** 本人がアプリの最初の画面を開き、その後に定期の時を迎える
- **THEN** 最初の画面の一番上の行に「更新の確認: 有効」と切り方（設定の画面の「新しい版を確かめる」）が出て、示した印が立ち、その時には問い合わせが出ず、その後の最初の機会に 1 件出る

#### Scenario: 入れ替えた携帯は、設定の画面を開くと示したとみなす

- **GIVEN** 通知を出せずに止まっている、入れ替えた携帯の収集アプリ
- **WHEN** 本人が設定の画面を開き、その後に定期の時を迎える
- **THEN** 設定の画面の「新しい版を確かめる」のスイッチが送り先の入力欄より上に並び、示した印が立ち、その後の最初の機会に問い合わせが 1 件出る

#### Scenario: 入れ替えた携帯は、通知を後から許すと次の機会に切り方を通知してから確かめる

- **GIVEN** 通知の許可が無いまま止まっている、入れ替えた携帯の収集アプリ
- **WHEN** 通知を許し、定期の時を 2 回迎える
- **THEN** 1 回目の定期の時にチャンネル `update` に「更新の確認: 有効」と切り方の通知が 1 回出て、その時には問い合わせが出ず、2 回目の定期の時に 1 件出る

#### Scenario: 入れ替えた携帯は、権限を求めている途中は画面に出したとみなさない

- **GIVEN** 通知を出せずに止まっている、入れ替えた携帯の収集アプリで、まだ求めていない権限が残っている
- **WHEN** 本人が最初の画面を開き、権限を求める窓が出たまま定期の時を迎える
- **THEN** 示した印は立っておらず、外への問い合わせは 0 件である

#### Scenario: 入れ替えた携帯は、ランチャー以外から開かれて他の行が無くても入れ替えの行を閉じずに出す

- **GIVEN** 通知を出せずに止まっている、入れ替えた携帯の収集アプリで、最初の画面に出す行が入れ替えの行のほかに無い
- **WHEN** 最初の画面が、ランチャーの入口（`ACTION_MAIN`）でない Intent で開かれる
- **THEN** 最初の画面は閉じず、入れ替えの行が出ている
- **AND** 示した印が立つ

#### Scenario: 入れ替えた携帯は、同じ回に確認の窓を重ねたときは窓が閉じた時に画面に出したとみなす

- **GIVEN** 通知を出せずに止まっている、入れ替えた携帯の収集アプリで、未使用のアプリの制限の対象になっていて確認の窓をまだ出していない
- **WHEN** 本人が最初の画面を開き、確認の窓が出て、その窓を「あとで」で閉じる
- **THEN** 窓が出ている間は示した印が無く、窓を閉じた時に立つ

#### Scenario: 入れ替えた携帯は、示した後に新しく開いた最初の画面には入れ替えの行を出さない

- **GIVEN** 最初の画面に入れ替えの行を出して示した印が立った、入れ替えた携帯の収集アプリ
- **WHEN** その画面を閉じ、本人がもう一度ランチャーから最初の画面を開く
- **THEN** 新しく開いた最初の画面に入れ替えの行は無い

#### Scenario: 入れ替えた携帯は、確認の窓を「設定を開く」で閉じた回は画面に出したとみなさない

- **GIVEN** 通知を出せずに止まっている、入れ替えた携帯の収集アプリで、未使用のアプリの制限の対象になっていて確認の窓をまだ出していない
- **WHEN** 本人が最初の画面を開き、確認の窓が出て、その窓を「設定を開く」で閉じ、設定から最初の画面に戻る
- **THEN** 窓を閉じた時には示した印が無く、戻って画面を組み直した時に立つ

#### Scenario: 入れ替えた携帯は、前の回の確認の窓が出たまま画面を組み直しても画面に出したとみなさない

- **GIVEN** 通知を出せずに止まっている、入れ替えた携帯の収集アプリで、使用状況のアクセスの設定にまだ送っておらず、未使用のアプリの制限の対象になっていて確認の窓をまだ出していない
- **WHEN** 本人が最初の画面を開き、使用状況のアクセスの設定と確認の窓が同じ回に前に出て、設定から最初の画面に戻る（窓は出たまま）
- **THEN** 戻って画面を組み直した時には示した印が無い
- **AND** 窓を「あとで」で閉じた時に示した印が立つ

#### Scenario: 入れ替えた携帯は、同じ画面を組み直しても入れ替えの行を残す

- **GIVEN** 最初の画面に入れ替えの行を出して示した印が立ち、その画面が開いたままの、入れ替えた携帯の収集アプリ
- **WHEN** 同じ画面が権限の結果を受けて組み直される（続けて画面が前に出る）
- **THEN** 組み直した画面にも入れ替えの行が出ている

#### Scenario: 入れ替えた携帯は、同じ回に別の画面を前に出したときは画面に出したとみなさない

- **GIVEN** 通知を出せずに止まっている、入れ替えた携帯の収集アプリで、使用状況のアクセスの設定へまだ送っていない
- **WHEN** 本人が最初の画面を開き、同じ回に使用状況のアクセスの設定が前に出る
- **THEN** その回には示した印が立たず、設定から最初の画面に戻って画面を組み直した時に立つ

#### Scenario: 入れ替えた PC の収集アプリは、画面のロック中の定期の時には切り方のトーストを試みない

- **GIVEN** 起動の時に切り方のトーストを出せなかった、入れ替えた PC の収集アプリで、トーストの出し手は成功を返すようになっている
- **WHEN** 画面がロックされたまま、最後の入力からの経過が 60 秒と読める定期の時を 3 回迎える（24 時間ずつ時計を進める）
- **THEN** トーストの出し手が呼ばれた回数は 0 で、示した印は立っておらず、外への問い合わせは 0 件である

#### Scenario: 入れ替えた PC の収集アプリは、最後の入力から 5 分を過ぎた定期の時には切り方のトーストを試みない

- **GIVEN** 起動の時に切り方のトーストを出せなかった、入れ替えた PC の収集アプリで、トーストの出し手は成功を返すようになっている
- **WHEN** 画面はロックされておらず、最後の入力からの経過が 301 秒の定期の時を迎え、次に 300 秒の定期の時を迎える
- **THEN** 301 秒の時にはトーストの出し手が呼ばれず、300 秒の時に札 `update-intro` のトーストが 1 回出て、どちらの時にも問い合わせは出ない

#### Scenario: 入れ替えた PC の収集アプリは、入力の経過を読めない定期の時には切り方のトーストを試みない

- **GIVEN** 起動の時に切り方のトーストを出せなかった、入れ替えた PC の収集アプリで、トーストの出し手は成功を返すようになっている
- **WHEN** 画面はロックされておらず、最後の入力からの経過を読めない定期の時を 2 回迎える
- **THEN** トーストの出し手が呼ばれた回数は 0 で、示した印は立っておらず、外への問い合わせは 0 件である

### Requirement: サーバの更新係は、DB にも状態の置き場にも触れない別のプログラム・別のサービスとして動く

THE SYSTEM SHALL サーバの版を確かめる役を、サーバ（`ashiato-server`）と別の実行ファイル（`ashiato-updater.exe`）・別のサービス（`ashiato-updater`。開発版は `ashiato-updater-dev`）にし、合言葉を持たないサービス専用のアカウント（`NT SERVICE\ashiato-updater`。開発版は `NT SERVICE\ashiato-updater-dev`）で、開始の種類を「自動」にして動かす。
THE SYSTEM SHALL 更新係の実行ファイルに、DB に繋ぐ部品（一覧は検査の側が持ち、1 つ以上の名前を持つ）を持たせない。
THE SYSTEM SHALL サーバを組み立てる部品に更新係の実行ファイルの crate を含めない（サーバが外へ送る部品を持たない守り（`tools/check-offsite.sh`）を変えない）。
THE SYSTEM SHALL 更新係の置き場（`C:\ProgramData\ashiato-updater`。開発版は `C:\ProgramData\ashiato-updater-dev`）を状態の置き場（`C:\ProgramData\ashiato`）の外に作り、親のフォルダから受け継がず、更新係のアカウントに読み書き、サーバのサービスのアカウントとインストールした本人に読みだけを付け、`Users`・`Authenticated Users`・`Everyone` に何も付けない。切った印のファイル（`settings\check.json`）は管理者と SYSTEM だけが書け、更新係とサーバのアカウントと本人は読みだけにする。
THE SYSTEM SHALL 更新係のアカウントに、状態の置き場の読み書きを付けない。

導出元: FR-112（★ 2026-10-25・★ 2026-11-03）。聞かない既定 C1・C6・C9。premises P11・P12。申し送り ST55 D14（`docs/handoff/ST57.md`）。
更新係は記録を持たないので、問い合わせの部品を持っても記録は外へ出せない。サーバは更新係の置き場のファイルを読むだけで、書けない。

#### Scenario: 更新係は DB に繋ぐ部品を持たない

- **WHEN** 更新係の実行ファイルを組み立てる部品の一覧を、DB に繋ぐ部品の一覧と突き合わせる
- **THEN** 一致する部品は 0 件である

#### Scenario: サーバは更新係の部品に依らない

- **WHEN** サーバを組み立てる部品の一覧を読み、外へ送る部品の検査を走らせる
- **THEN** 一覧に更新係の実行ファイルの crate が無く、外へ送る部品の検査は今までどおり通る

#### Scenario: 更新係はサービス専用のアカウントで自動で起きる

- **WHEN** インストールした後に、サービス `ashiato-updater` の開始の種類と、動いているアカウントを読む
- **THEN** 開始の種類は「自動」で、`NT SERVICE\ashiato-updater` で動き、`LocalSystem` でもインストールした利用者でもない

#### Scenario: 更新係のアカウントは状態の置き場を読めない

- **WHEN** インストールした後に、状態の置き場とその下の全部の権限（ACL）を読む
- **THEN** どこにも `NT SERVICE\ashiato-updater` の項目が無い

#### Scenario: 更新係の置き場は、サーバが読めて書けない

- **WHEN** インストールした後に、更新係の置き場とその下の全部の権限を読む
- **THEN** どれも親から受け継がず、書きを持つのは `NT SERVICE\ashiato-updater`（`settings\check.json` は管理者と SYSTEM だけ）で、`NT SERVICE\ashiato-server` とインストールした本人は読みだけを持ち、`Users`・`Authenticated Users`・`Everyone` の項目が無い

### Requirement: 確かめた結果は「見つけた最新の版」で持ち、確かめた記録を追記で残す

THE SYSTEM SHALL 各部品に、確かめた結果を「見つけた最新の版」（と見つけた時刻）・最後に確かめた時刻・最後に確かめられた時刻で持たせ、「更新あり」かどうかは読む側がそのときの自分の版と比べて決める。
THE SYSTEM SHALL 確かめるたびに、時刻・成否・確かめられない理由・見つけた最新の版を 1 行、追記だけの記録に残す（書き換えず、古い行を消さない。1 MB を超えたら古い半分を別のファイルへ退けてよい）。
THE SYSTEM SHALL 確かめた記録の理由を、次の 6 つで分ける: 配布元に届かない（接続できない）・時間切れ（接続に 10 秒、全体で 30 秒を超えた）・問い合わせの上限（`403`・`429`）・配布元が見つからない（`404`）・読めない答え（`200` でも読めない形・別の宛先への転送・転送が 3 回を超えた・ほかの状態の番号）・宛先なし（試験の組み立てで差し替えの宛先が無い）。
THE SYSTEM SHALL サーバが更新係の結果を読むときに、記録の理由に加えて「更新係が動いていない」（下の 48 時間）を使う。
THE SYSTEM SHALL 携帯の収集アプリの確かめた記録と「知らせた版」の置き場を、クラウドのバックアップと端末の移し替えから外す。

導出元: FR-112・FR-113。聞かない既定 C3・C4。review/deep.md R3・R7。
入れた後に古い結果で「更新あり」が残らない。表示の規則（7 日など）を変えても、過去の記録から計算し直せる。携帯の確かめた記録は、携帯がいつ網に繋がっていたかの記録にもなる。

#### Scenario: 新しい版を入れた後は、古い結果でも更新ありにならない

- **GIVEN** 見つけた最新の版 `0.5.0` を持つ結果
- **WHEN** 版 `0.5.0` に入れ替えた部品がその結果を読む
- **THEN** 「更新なし」になる

#### Scenario: 確かめるたびに記録が 1 行増え、前の行は変わらない

- **GIVEN** 確かめた記録が 3 行ある部品
- **WHEN** 1 回確かめて配布元に届かない
- **THEN** 記録は 4 行になり、前の 3 行は 1 バイトも変わらず、4 行目の理由は「配布元に届かない」である

#### Scenario: 確かめられない理由を状態の番号で分ける

- **WHEN** 受け手が `403`・`429`・`404`・`500`・読めない JSON の `200` を順に返し、部品が 5 回確かめる
- **THEN** 5 行の理由は順に「問い合わせの上限」「問い合わせの上限」「配布元が見つからない」「読めない答え」「読めない答え」である

#### Scenario: 答えない配布元は時間切れになる

- **WHEN** 受け手が接続を受けたまま 31 秒答えず、部品が 1 回確かめる
- **THEN** その回は 29 秒より前に終わらず、40 秒より前に終わり、理由は「時間切れ」である

#### Scenario: 携帯の確かめた記録はバックアップから外れる

- **WHEN** 携帯の収集アプリのバックアップの規則（クラウドのバックアップと端末の移し替えの両方）と、確かめた記録のファイルの置き場を読み、バックアップの規則の検査を走らせる
- **THEN** 更新の確認の結果と知らせた版の置き場を外す行があり、その行を消した写しでは検査が落ちる。確かめた記録のファイルは OS がバックアップしない置き場にある

### Requirement: サーバの更新の確認は、インストーラの命令で切ったり入れたりする

THE SYSTEM SHALL インストーラに `update-check off` と `update-check on` の口を持たせ、管理者の確認を 1 回出して切り替える。
WHEN `update-check off` を走らせる THE SYSTEM SHALL 切った印（`settings\check.json` の `enabled: false`）を書き、更新係のサービスを止めて開始の種類を「無効」にする。
WHEN `update-check on` を走らせる THE SYSTEM SHALL 印を `enabled: true` と入れた時刻にし、更新係のサービスの開始の種類を「自動」にして開始する。
WHILE 切った印が `enabled: false` である THE SYSTEM SHALL 更新係を開始されても配布元に問い合わせさせない。
THE SYSTEM SHALL 切った確認を、インストールの走らせ直しでも新しい版への入れ替えでも入れ直さない。
THE SYSTEM SHALL サーバに、更新係のサービスの状態を問い合わせさせず、切った印と結果の時刻だけで「切ってある」「動いていない」「確かめられていない」を見分けさせる。

導出元: FR-115（部品ごとに設定で無効にできる。既定は有効）。深掘り Q3（推奨: インストーラの命令で切る・入れる）。聞かない既定 C7・C9。
サービスを止めて無効にするので、通信が無いことを OS の上で確かめやすい。画面から切る口は作らない（サーバから更新係へ書ける道を作らない）。

#### Scenario: 切ると更新係のサービスが止まり、問い合わせが無くなる

- **WHEN** インストールした後に `ashiato-setup update-check off` を走らせ、更新係のサービスを読み、更新係の確かめる時を 2 回迎えさせる（試験の組み立ての差し込みで時計を進める）
- **THEN** サービス `ashiato-updater` は停止していて開始の種類は「無効」で、受け手が受けた問い合わせは 0 件である

#### Scenario: 切った印があれば、更新係を開始しても問い合わせない

- **GIVEN** 切った印が `enabled: false` の更新係の置き場
- **WHEN** 更新係を手で開始し、確かめる時を迎えさせる
- **THEN** 受け手が受けた問い合わせは 0 件で、確かめた記録は増えない

#### Scenario: 入れ直すと更新係が起きて確かめる

- **GIVEN** `update-check off` で切った後
- **WHEN** `ashiato-setup update-check on` を走らせる
- **THEN** サービス `ashiato-updater` は開始の種類が「自動」で実行中になり、受け手が問い合わせを 1 件受ける

#### Scenario: 切り替えの口は管理者の確認を 1 回だけ出す

- **WHEN** 管理者に上がっていない状態のインストーラが、`update-check off` のために組む命令を読む
- **THEN** 自分を管理者として起こし直す命令はちょうど 1 つである

### Requirement: 画面は、サーバの「更新あり」と「確かめられていない」をどの画面でも見出しの上の帯に出し、設定に更新の確認の状態を出す

WHEN サーバの更新係が見つけた最新の版がサーバの版より新しい THE SYSTEM SHALL ログインした画面のどの画面でも、見出しの上の帯に 1 行で「更新あり <版>（<月>/<日> に見つけた）」を出す。
WHEN サーバの更新の確認が有効で 7 日以上確かめられていない THE SYSTEM SHALL 同じ帯に「更新を 7 日確かめられていない（最後に確かめた <月>/<日>・<理由>）」を出す（2 つが同時なら 2 行）。
THE SYSTEM SHALL 確認を切ってあるとき・新しい版が無いとき・まだ 7 日たたないときは帯を出さない。
THE SYSTEM SHALL 帯を押しても何もしない（新しい版を落とす口は後の Story が作る）。
THE SYSTEM SHALL 設定の画面に「更新の確認」の節を置き、状態（下の `state` の画面の語）・入っている版・見つけた最新の版と見つけた日・最後に確かめた日時と、確かめられなかったならその理由を出す。
THE SYSTEM SHALL 帯と節の中身を、サーバが更新係の置き場の結果を読んで返す口（`GET /update-status`）から取り、画面から外の宛先へ要求を出さない。
THE SYSTEM SHALL `GET /update-status` に画面のログインを求め、API の合言葉では答えない。
THE SYSTEM SHALL `GET /update-status` の応答を、`state`・`installed`（サーバの版）・`latest`・`found_at`・`checked_at`・`last_ok_at`・`reason` の欄で返し、`state` を次の 8 つのどれかにする（画面の語との対応）:
`off`（切ってある）・`not_running`（動いていない。更新係の置き場が無い）・`pending`（まだ確かめていない。一度も確かめられておらず、確認を入れてから 7 日たたない）・`latest`（新しい版なし）・`available`（新しい版あり）・`stale`（7 日確かめられていない）・`available_stale`（新しい版あり・7 日確かめられていない）・`unreadable`（結果を読めない）。
THE SYSTEM SHALL `GET /update-status` の応答に `Cache-Control: no-store` を付ける。
IF 更新係の置き場はあるのに結果のファイル（`status.json`）が無い THEN THE SYSTEM SHALL 確認を入れた時から 7 日たつまでは `pending`、たてば `stale` にする。
IF 結果のファイルが 64 KB を超える・読めない形・版や時刻の形でない値を持つ THEN THE SYSTEM SHALL その中身を画面に出さず、`state` を `unreadable` にして、帯は出さず設定の節にだけ「結果を読めない」と出す。
THE SYSTEM SHALL 帯を出したときも、360 px の幅で横に流さず、帯の文字と地のコントラストを 4.5:1 以上にする。

導出元: FR-113（画面に「更新あり」）。深掘り Q7（推奨の形: どの画面でも見出しの上の帯に 1 行・設定の画面に状態）・Q8（推奨: 何も起きない。版番号と見つけた日を出すだけ）・Q9（推奨: 7 日確かめられなければ出す）。聞かない既定 C9・C10。
画面は外部の資源を読まない（`default-src 'self'`）。帯の中身はサーバが返すので、この守りは変わらない。Linux や開発の形のサーバは更新係を持たない（C10）ので、設定に「動いていない」と出て帯は出ない。

#### Scenario: 更新ありはどの画面でも見出しの上に出る

- **GIVEN** 更新係の結果が見つけた最新の版 `0.5.0`（11/10 に見つけた）で、サーバの版が `0.4.0`
- **WHEN** ログインして稼働状況・1 日を見る・マスタ管理・設定の各画面を開く
- **THEN** どの画面でも、見出しより上に「更新あり 0.5.0（11/10 に見つけた）」の帯が 1 つある

#### Scenario: 6 日確かめられていないだけなら帯は出ない

- **GIVEN** 更新の確認が有効で、最後に確かめられたのが 6 日前で、その後は「配布元に届かない」
- **WHEN** ログインして稼働状況の画面を開く
- **THEN** 帯は無い

#### Scenario: 7 日確かめられなければ帯に出る

- **GIVEN** 更新の確認が有効で、最後に確かめられたのが 8 日前（11/03）で、最後に確かめた（届かなかった）のは 1 時間前で、その理由が「配布元に届かない」
- **WHEN** ログインして稼働状況の画面を開く
- **THEN** 帯に「更新を 7 日確かめられていない（最後に確かめた 11/03・配布元に届かない）」が出る

#### Scenario: 更新ありと確かめられていないが同時なら 2 行出る

- **GIVEN** 見つけた最新の版 `0.5.0` を持ち、その後 8 日確かめられていない結果
- **WHEN** 360 px の幅でログインして稼働状況の画面を開く
- **THEN** 帯が 2 行出て、ページは横に流れない（文書の幅が窓の幅を超えない）

#### Scenario: 帯の文字と地のコントラストは 4.5:1 以上

- **GIVEN** 「更新あり」と「7 日確かめられていない」の帯が 2 行出る結果
- **WHEN** 明るい既定と暗い既定の配色で稼働状況の画面を開き、帯の文字の色と地の色を読む
- **THEN** どの帯でも、文字と地のコントラストは 4.5:1 以上である

#### Scenario: 新しい版が無ければ帯は出ない

- **GIVEN** 見つけた最新の版がサーバの版と同じ `0.4.0` で、最後に確かめたのが 1 時間前
- **WHEN** ログインして稼働状況の画面を開く
- **THEN** 帯は無く、見出しの位置は帯の無い画面と同じである

#### Scenario: 確認を切ってあれば帯は出ず、設定に切ってあると出る

- **GIVEN** 切った印が `enabled: false` で、前の結果に見つけた最新の版 `0.5.0` が残っている
- **WHEN** ログインして稼働状況と設定の画面を開く
- **THEN** 帯は無く、設定の「更新の確認」に「切ってある」と出る

#### Scenario: 帯を押しても何も起きない

- **GIVEN** 「更新あり 0.5.0」の帯が出ている
- **WHEN** 帯を押す
- **THEN** URL は変わらず、ブラウザが出した要求は増えない

#### Scenario: 設定に更新の確認の状態が出る

- **GIVEN** 更新係の結果が見つけた最新の版 `0.5.0`（11/10 に見つけた）・最後に確かめた 11/10 09:12 で、サーバの版が `0.4.0`
- **WHEN** ログインして設定の画面を開く
- **THEN** 「更新の確認」の節に「有効・新しい版あり」・入っている版 `0.4.0`・見つけた最新の版 `0.5.0` と見つけた日・最後に確かめた日時が出る

#### Scenario: 更新係の無いサーバは設定に動いていないと出し、帯を出さない

- **GIVEN** 更新係の置き場が無いサーバ（開発の形）
- **WHEN** ログインして稼働状況と設定の画面を開く
- **THEN** 帯は無く、設定の「更新の確認」に「動いていない」と出る

#### Scenario: 帯を出しても外部への要求は 0 件である

- **GIVEN** 「更新あり 0.5.0」の帯が出る結果
- **WHEN** ログインして稼働状況・1 日を見る・マスタ管理・設定の各画面を開き、ブラウザが出した要求を数える
- **THEN** 画面を配る側と同じ出所以外への要求は 0 件である

#### Scenario: 更新の状態の口はログインが要り、API の合言葉では答えない

- **WHEN** ログインの印なしで、また API の合言葉だけを付けて `GET /update-status` を求める
- **THEN** ログインの印なしは `401`、API の合言葉だけは `403` で、どちらも版も時刻も返さない

#### Scenario: 読めない結果のファイルは画面に出さない

- **GIVEN** 更新係の置き場の結果のファイルが 65 KB ある、または見つけた最新の版が `<script>` という文字列
- **WHEN** `GET /update-status` を求める
- **THEN** 応答に結果のファイルの中身は無く、`state` は `unreadable` である

#### Scenario: 64 KB より小さい正しい結果のファイルは読む

- **GIVEN** 63 KB の、見つけた最新の版 `0.5.0` を持つ正しい結果のファイル（余白で大きくした）と、サーバの版 `0.4.0`
- **WHEN** `GET /update-status` を求める
- **THEN** `state` は `available` で、`latest` は `0.5.0` である

#### Scenario: 結果を読めないときは帯を出さず、設定にだけ出す

- **GIVEN** 読めない形の結果のファイル
- **WHEN** ログインして稼働状況と設定の画面を開く
- **THEN** 帯は無く、設定の「更新の確認」に「結果を読めない」と出る

#### Scenario: まだ確かめていない更新係は、7 日たつまで帯を出さない

- **GIVEN** 更新係の置き場があり、確認を 2 日前に入れ、結果のファイルがまだ無い
- **WHEN** ログインして稼働状況と設定の画面を開き、`GET /update-status` を求める
- **THEN** 帯は無く、設定に「まだ確かめていない」と出て、`state` は `pending` である

#### Scenario: 新しい版なしは設定にだけ出る

- **GIVEN** 見つけた最新の版がサーバの版と同じ `0.4.0` で、最後に確かめたのが 1 時間前
- **WHEN** ログインして設定の画面を開く
- **THEN** 設定の「更新の確認」に「新しい版なし」と最後に確かめた日時が出る

#### Scenario: 更新の状態の口は応答を残させない

- **WHEN** ログインして `GET /update-status` を求める
- **THEN** 応答の `Cache-Control` は `no-store` で、本文は `state`・`installed`・`latest`・`found_at`・`checked_at`・`last_ok_at`・`reason` の欄を持つ

### Requirement: 携帯と PC の収集アプリの更新の確認は、端末ごとに切れる

THE SYSTEM SHALL 携帯の収集アプリの設定の画面に「新しい版を確かめる」のスイッチ（既定は入）を置き、その値を携帯の中にだけ持つ。
THE SYSTEM SHALL PC の収集アプリに `update-check off` と `update-check on` の口を持たせ、`setup` で設定を保存した後に「新しい版を確かめますか [Y/n]」を聞き（既定は確かめる）、その値を PC の収集アプリの置き場（`%LOCALAPPDATA%\ashiato\collector` の `settings.json`）にだけ持つ。
THE SYSTEM SHALL 更新の確認の値を設定の文字列に入れない（携帯と PC を別々に切れる）。
WHILE その端末の確認が切ってある THE SYSTEM SHALL その端末から配布元へ問い合わせない。

導出元: FR-115（★ 2026-10-25: C-02 の設定の置き場は FR-128 の設定と同じ所）。深掘り Q4（推奨: 端末ごとに切る）。

#### Scenario: 携帯でスイッチを切ると問い合わせが無くなる

- **GIVEN** 送り先を設定した携帯の収集アプリ
- **WHEN** 設定の画面の「新しい版を確かめる」を切り、確かめる時を 2 回迎えさせる
- **THEN** 受け手が受けた問い合わせは 0 件である

#### Scenario: PC で update-check off にすると問い合わせが無くなる

- **GIVEN** 送り先を設定した PC の収集アプリ
- **WHEN** `ashiato-collector-windows.exe update-check off` を走らせ、確かめる時を 2 回迎えさせる
- **THEN** 受け手が受けた問い合わせは 0 件で、`settings.json` に確認を切った値がある

#### Scenario: PC の setup は確かめるかを聞き、既定は確かめる

- **WHEN** `setup` で設定の文字列を貼って保存し、「新しい版を確かめますか [Y/n]」に空の行で答える
- **THEN** `settings.json` に確かめる値があり、次の機会に問い合わせが 1 件出る

#### Scenario: 携帯で切っても PC は確かめる

- **GIVEN** 同じ設定の文字列を貼った携帯と PC の収集アプリ
- **WHEN** 携帯だけで確認を切り、PC の確かめる時を迎えさせる
- **THEN** PC の問い合わせは 1 件出る

### Requirement: 携帯の収集アプリは、新しい版を画面に出し、新しい版ごとに 1 回だけ通知する

WHEN 携帯の収集アプリが自分の版より新しい版を見つける THE SYSTEM SHALL アプリの最初の画面に「更新あり <版>（<月>/<日> に見つけた）」の行を出す。
WHEN 携帯の収集アプリがまだ知らせていない新しい版を見つける THE SYSTEM SHALL 通知のチャンネル `update` に「新しい版 <版> が出ています」を 1 回出し、同じ版では 2 度出さない。
WHEN 本人が更新の通知を押す THE SYSTEM SHALL アプリの最初の画面を開く。
THE SYSTEM SHALL 携帯と PC の「確かめられていない」の行と通知の文を「更新を 7 日確かめられていません（最後に確かめた <月>/<日>・<理由>）」にする（確かめられたことが無ければ「最後に確かめた —」）。
THE SYSTEM SHALL 携帯の更新の通知を既存のサーバ更新の通知と別のチャンネル・通知 ID で出し、後者を置き換えず文を分ける。

導出元: FR-113（★ 2026-11-11、携帯の画面に「更新あり」）。深掘り Q5（画面に出し、新しい版ごとに 1 回だけ通知する）。
携帯のアプリの画面は設定するときにしか開かないので、画面に出すだけでは気づかないことがある。知らせた版の置き場が消えた（入れ直した）ときは、同じ版をもう 1 回知らせるだけで害は無い。

#### Scenario: 携帯は新しい版を最初の画面に出す

- **GIVEN** 見つけた最新の版 `0.5.0`（11/10 に見つけた）を持つ、版 `0.4.0` の携帯の収集アプリ
- **WHEN** アプリの最初の画面を開く
- **THEN** 「更新あり 0.5.0（11/10 に見つけた）」の行が出る

#### Scenario: 携帯は新しい版ごとに 1 回だけ通知する

- **GIVEN** 版 `0.4.0` の携帯の収集アプリ
- **WHEN** `0.5.0` を見つける確かめを 2 回続け、その後 `0.5.1` を見つける
- **THEN** チャンネル `update` の通知は 2 回（`0.5.0` と `0.5.1`）出る

#### Scenario: 携帯の更新の通知を押すと最初の画面が開く

- **WHEN** 「新しい版 0.5.0 が出ています」の通知を押す
- **THEN** アプリの最初の画面が開き、「更新あり 0.5.0」の行が見える

#### Scenario: 携帯の更新の通知は既存のサーバの更新の知らせを置き換えず、文が違う

- **WHEN** 携帯の更新の通知と既存のサーバ更新の通知を出す
- **THEN** チャンネルと通知 ID は別で、更新の通知だけが「新しい版」を含む

### Requirement: PC の収集アプリは、新しい版ごとに 1 回だけ通知する

WHEN PC の収集アプリがまだ知らせていない新しい版を見つける THE SYSTEM SHALL 「新しい版 <版> が出ています」の通知（札 `update`）を 1 回出し、同じ版では 2 度出さない。
THE SYSTEM SHALL 更新の通知の文を、系統の知らせ（送り先が開発版／正式版のサーバ）や送信契約の版の知らせと分ける。

導出元: FR-113（★ 2026-10-25: C-02 は画面が無く、表に出るのは通知だけ）。深掘り Q6（推奨: 新しい版ごとに 1 回）。
見逃すと次の版まで出ない。画面（ブラウザ）にはサーバの「更新あり」が出続ける。

#### Scenario: PC は新しい版ごとに 1 回だけ通知する

- **GIVEN** 版 `0.4.0` の PC の収集アプリ
- **WHEN** `0.5.0` を見つける確かめを 2 回続け（24 時間ずつ時計を進める）、PC の収集アプリを起こし直してからもう 1 回確かめる
- **THEN** 札 `update` の通知は 1 回だけ出る

#### Scenario: PC の更新の通知はほかの知らせと文が違う

- **WHEN** 更新の通知と、系統の知らせと、送信契約の版の知らせの文を読む
- **THEN** 更新の通知の文は「新しい版」を含み、ほかの 2 つの文と同じでない

### Requirement: 7 日確かめられなければ、「更新を確かめられていない」を知らせる

WHILE その部品の更新の確認が有効である IF 最後に確かめられた時（確かめられたことが無ければ確認を入れた時か送り先を設定した時。確認の無い前の版から入れ替えた収集アプリは切り方を示した時）から 7 日が過ぎた THEN THE SYSTEM SHALL 「確かめられていない」の知らせ（画面の帯は「更新を 7 日確かめられていない（…）」、携帯と PC は「更新を 7 日確かめられていません（…）」）を、「更新あり」と同じ所に、最後に確かめられた日と最後の理由を付けて出す: 画面の帯（サーバ）・携帯の最初の画面の行と通知 1 回・PC の通知 1 回。
THE SYSTEM SHALL 携帯と PC の「確かめられていない」の通知を、確かめられない日が続く 1 回の間に 1 度だけ出し、確かめられたら次に続いたときにまた出せるようにする。
IF 最後に確かめられた時刻が今より未来である THEN THE SYSTEM SHALL 7 日が過ぎたとみなす。
IF サーバの更新係の最後に確かめた時刻（成功でも失敗でも）が 48 時間より古い THEN THE SYSTEM SHALL 最後の理由を「更新係が動いていない」にする。
WHILE その部品の確認が切ってある THE SYSTEM SHALL 「確かめられていない」を出さない。
WHILE 入れ替えた収集アプリがまだ切り方を示していない THE SYSTEM SHALL 「確かめられていない」を出さない（示すまで問い合わせないのは本人の決定で、確かめられないのではない。design D26（仮））。

導出元: FR-113（★ 2026-11-11）。深掘り Q9（7 日確かめられなければ出す）。聞かない既定 C5・C9。review/deep.md R4・R7。 入れ替えた収集アプリの起点: 深掘り Q11・design D26（仮）。
確かめられない間は「更新あり」が出ないので、黙っていると「新しい版が無い」のか「確かめられていない」のかが分からない。配った版の中の確かめ方（宛先・読み方）はその版からは直せないので、壊れたことに気づく口がここになる。

#### Scenario: 7 日確かめられなければ PC が 1 回知らせる

- **GIVEN** 最後に確かめられたのが 8 日前で、その後は配布元に届かない PC の収集アプリ
- **WHEN** 確かめる時を 3 回迎える（24 時間ずつ時計を進め、どれも届かない）
- **THEN** 「更新を 7 日確かめられていません」の通知は 1 回だけ出て、最後に確かめた日と「配布元に届かない」を含む

#### Scenario: 確かめられたら、次に続いたときにまた知らせる

- **GIVEN** 「更新を 7 日確かめられていません」を 1 回知らせた PC の収集アプリ
- **WHEN** 1 回確かめられ、その後また 8 日確かめられない
- **THEN** 「更新を 7 日確かめられていません」の通知がもう 1 回出る

#### Scenario: 携帯は 7 日確かめられなければ画面に出し、1 回通知する

- **GIVEN** 送り先を設定してから 8 日、一度も確かめられていない携帯の収集アプリ
- **WHEN** 確かめる時を 2 回迎え、最初の画面を開く
- **THEN** 「更新を 7 日確かめられていません」の通知は 1 回だけ出て、最初の画面に同じ文の行がある

#### Scenario: 6 日確かめられていないだけなら知らせない

- **GIVEN** 最後に確かめられたのが 6 日前で、その後は配布元に届かない PC の収集アプリ
- **WHEN** 確かめる時を迎える（届かない）
- **THEN** 「更新を確かめられていません」の通知は出ない

#### Scenario: 確認を切った部品は確かめられていないと知らせない

- **GIVEN** 確認を切ってから 30 日たった PC の収集アプリ
- **WHEN** 起動し、定期の時を迎える
- **THEN** 「更新を 7 日確かめられていません」の通知は出ない

#### Scenario: 最後に確かめられた時刻が未来なら、7 日が過ぎたとみなす

- **GIVEN** 最後に確かめられた時刻が今より 1 年先に記録され、今の確かめも届かない PC の収集アプリ
- **WHEN** 確かめる時を迎える
- **THEN** 「更新を 7 日確かめられていません」の通知が出る

#### Scenario: 更新係の時刻が古ければ、理由は更新係が動いていない

- **GIVEN** 更新係の結果の最後に確かめた時刻が 49 時間前で、その回は配布元に届かず、最後に確かめられたのが 9 日前で、確認は有効
- **WHEN** `GET /update-status` を求める
- **THEN** 状態は「7 日確かめられていない」で、理由は「更新係が動いていない」である

#### Scenario: 更新係の時刻が 48 時間たたなければ、理由は最後の回のまま

- **GIVEN** 更新係の結果の最後に確かめた時刻が 47 時間前で、その回は配布元に届かず、最後に確かめられたのが 9 日前で、確認は有効
- **WHEN** `GET /update-status` を求める
- **THEN** 状態は「7 日確かめられていない」で、理由は「配布元に届かない」である

#### Scenario: 入れ替えた収集アプリは、示すまでの間を 7 日に数えない

- **GIVEN** 送り先を 30 日前に設定し、更新の確認を持たない前の版から入れ替えた PC の収集アプリで、トーストの出し手が 3 回続けて失敗を返した後に成功を返す
- **WHEN** 起動と定期の時を重ねて切り方のトーストが出て、その後の機会に確かめて配布元に届かない
- **THEN** 札 `update-stale` の通知は出ない

#### Scenario: 入れ替えた携帯は、切り方を画面に出した時に確かめられていないを並べない

- **GIVEN** 送り先を 30 日前に設定し、更新の確認を持たない前の版から入れ替えて通知を出せない携帯の収集アプリ
- **WHEN** 本人がアプリの最初の画面を開く
- **THEN** 最初の画面に「更新の確認: 有効」と切り方の行が出て、「更新を 7 日確かめられていません」の行は無く、その通知も出ない

### Requirement: 携帯の収集アプリは、繋いでいる網を問わずに確かめる

THE SYSTEM SHALL 携帯の収集アプリに、網に繋がっていれば網の種類（Wi-Fi・携帯の回線）を問わずに確かめさせる。
THE SYSTEM SHALL 携帯の収集アプリの 1 回の問い合わせで、圧縮した応答を受ける（`Accept-Encoding: gzip`）。

導出元: 深掘り Q10（推奨: 網を問わない）。review/deep.md R8（1 回の通信量の見込み: 20 件・gzip で 140 KB より小さい）。

#### Scenario: 携帯は携帯の回線でも確かめる

- **GIVEN** Wi-Fi に繋がらず、携帯の回線だけに繋がっている（と通信の部品が答える）携帯の収集アプリ
- **WHEN** 確かめる時を迎える
- **THEN** 問い合わせが 1 件出る

#### Scenario: 携帯は圧縮した応答を受ける

- **WHEN** 携帯の収集アプリが本物の Android の上で 1 回確かめ、受け手が受けた見出しを読む
- **THEN** `Accept-Encoding` の値は `gzip` を含む

### Requirement: インストーラの `repair` の口は、更新係のサービスと置き場に触れない

THE SYSTEM SHALL `repair` の口で、更新係のサービスの登録・開始の種類・状態と、更新係の置き場（切った印・結果・確かめた記録）を変えない。

導出元: FR-112・FR-115。聞かない既定 C7。review/spec.md R10。
`repair` は DB と秘密の食い違いを直す口で、更新係は DB も秘密も持たない。切った確認を `repair` で入れ直さない。

#### Scenario: repair は更新係に触れない

- **GIVEN** `update-check off` で切ったインストールで、設定のファイルを退けた（DB はあるのに秘密が無い）状態
- **WHEN** `repair` の口が組む命令を読み、`repair` を走らせる
- **THEN** 命令に `ashiato-updater` の名前が出ず、走らせた後もサービス `ashiato-updater` の開始の種類は「無効」で、切った印と確かめた記録の指紋は控えと同じである
