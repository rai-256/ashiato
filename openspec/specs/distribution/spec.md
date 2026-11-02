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

### Requirement: 印（タグ）の名前で、3 つのチャンネルに分けて配る

WHEN 印 `v<MAJOR>.<MINOR>.<PATCH>` が配布元に push される THE SYSTEM SHALL その版を通常のリリース（正式版）として公開する。
WHEN 印 `v<MAJOR>.<MINOR>.<PATCH>-beta.<N>` が push される THE SYSTEM SHALL その版を先行公開（pre-release）のリリース（ベータ版）として公開する。
THE SYSTEM SHALL 開発版を、印 `nightly` の付いた 1 つの先行公開のリリースとして持ち、作り直すたびに印をその commit へ動かし、リリースの配布物を入れ替える（開発版のリリースは 1 つだけ）。
THE SYSTEM SHALL 1 つのリリースに、サーバ（画面を含む。Windows x86_64 向けと Linux x86_64 向けの 2 つ）・携帯の収集アプリの APK・PC の収集アプリ（Windows x86_64 向け）を、同じ版番号で並べる。ただし開発版のリリースには、ST54 まで携帯の収集アプリの APK を並べず、残りの 3 つ（サーバの 2 つと PC の収集アプリ）を同じ版番号で並べる。
THE SYSTEM SHALL 配布物の名前を `ashiato-server-<版番号>-windows-x86_64.zip`・`ashiato-server-<版番号>-linux-x86_64.tar.gz`・`ashiato-collector-<版番号>.apk`（開発版は `ashiato-collector-dev-<版番号>.apk`。開発版に並べるのは ST54 から）・`ashiato-collector-windows-<版番号>-x86_64.zip` にし、版をまたいで変えない（後の Story が版の確認で名前を読む）。
THE SYSTEM SHALL 印の版番号（`v` を除いたもの）の MAJOR.MINOR.PATCH が、印の commit の出所の版と違えば公開しない。
THE SYSTEM SHALL 配布物が 1 つでも組めなければ公開しない（正式版とベータ版はリリースを作らない。開発版は前の版のリリースを残す）。開発版は ST54 まで、携帯の APK を除く 3 つを数え、携帯の APK が置き場にあれば公開しない。
THE SYSTEM SHALL 印 `v*` を、main の祖先の commit に付いたときだけ公開する。例外は、緊急の修正の枝 `release/<MAJOR>.<MINOR>` の上の正式版の印で、その規則は「緊急の修正の枝は、配った版から切り、移行と封筒の形を変えない」が持つ。
THE SYSTEM SHALL 配った移行の凍結と送信契約の版の検査が数える「配った印」を `v[0-9]*` の印に限り、開発版の印 `nightly` を数えない。
IF 印の判定を走らせる木が浅い（履歴が途中で切れていて、印の commit が main の祖先かを決められない）THEN THE SYSTEM SHALL 祖先を見る前に判定を通さず、出力に「履歴が浅い」を示す（公開しない）。
THE SYSTEM SHALL チャンネルごとの枝を持たない（正式版もベータ版も開発版も main から作る）。

導出元: FR-104（★ 2026-11-01。開発版のリリースには ST54 まで C-01 を並べない）, FR-105。深掘り Q7・Q10（推奨）・Q4（本人の答え: ST54 まで、開発版に携帯の APK を入れない）。premises の C2・C7・C9・P17。review/deep.md R8。D23（仮）・D24（仮。review/spec.md R29）。

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

#### Scenario: 開発版の配布物の検査は、携帯の APK を除く 3 つで通る
- **GIVEN** 版番号 `0.4.0-dev.20261102` を渡して組み立てた、サーバの 2 つと PC の収集アプリの配布物の置き場（署名の指紋の期待値は空）
- **WHEN** 開発版の配布物の検査を走らせる
- **THEN** 検査の終了コードは 0

#### Scenario: 開発版の置き場に携帯の APK があれば、検査は落ちてその名前を示す
- **GIVEN** 上の 3 つに、`ashiato-collector-dev-0.4.0-dev.20261102.apk` か `ashiato-collector-0.4.0-dev.20261102.apk` のどちらかを足した置き場
- **WHEN** 開発版の配布物の検査を走らせる
- **THEN** どちらの場合も検査は 0 でない終了コードで終わり、足した APK の名前が示される

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
THE SYSTEM SHALL 開発版の携帯の APK に署名するときは、アプリ識別子 `dev.ashiato.collector.dev` と、正式な鍵と別の開発版の鍵で署名する（開発版のリリースに携帯の APK を並べ、開発版の鍵を作って置くのは ST54 から。扉 34）。
THE SYSTEM SHALL 正式な鍵を、印 `v*` の push の実行にだけ開いた置き場（配布用の Environment）から、開発版の鍵を開発版の予定の実行と main からの実行にだけ開いた置き場（開発版の Environment）から読み、PR の実行（同じリポジトリの枝からの PR を含む）には渡さない。正式な鍵は開発版の作り直しの実行にも渡さない。
THE SYSTEM SHALL PR の組み立ての携帯の APK を debug の鍵で署名し、正式な鍵も開発版の鍵も使わない。
IF 署名する鍵が置き場に無い THEN THE SYSTEM SHALL 署名しない APK を公開せず、そのリリースを公開しない。
THE SYSTEM SHALL 鍵を作る台本で、正式な鍵と開発版の鍵を RSA 4096 bit・有効期限 25 年以上で作り、作った鍵の種類・長さ・有効期限を読み直して確かめ、満たさなければ失敗する。

THE SYSTEM SHALL 開発版の識別子 `dev.ashiato.collector.dev` の APK に正式な鍵を、そうでない APK に開発版の鍵を当てようとしたら、署名せずに失敗する。
導出元: 扉 34（識別子と署名鍵）。FR-106（開発版は別のアプリ）。深掘り Q2・Q11（本人の答え。推奨と同じ）・Q4（本人の答え: ST54 まで、開発版に携帯の APK を入れない）。premises P2・P8・P12。review/deep.md R2・R3・R13。review/spec.md R10。
D3（仮）・D21（仮）・D23（仮）。鍵の置き場は扉 34 の「手元の USB と拠点外の 2 か所」に GitHub の Environment を足す（Q2。扉 34 ★ 2026-11-01。Environment の値は GitHub から取り出せないので、控えの代わりにはならない）。

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

#### Scenario: 鍵の置き場は、印と main のそれぞれにだけ開いている
- **WHEN** 鍵の置き場の検査を配布元に向けて走らせる
- **THEN** 配布用の Environment の配る対象は印 `v*` だけで、開発版の Environment の配る対象は枝 `main` だけ。そうでなければ検査は 0 でない終了コードで終わる

#### Scenario: 鍵が置き場に無ければ、署名しない APK を公開しない
- **GIVEN** 正式な鍵の置き場が空の公開の手順
- **WHEN** 印 `v0.4.0` で携帯の APK に署名する手順を走らせる
- **THEN** 手順は 0 でない終了コードで終わり、署名された APK はできず、公開の手順に渡す配布物が無い

#### Scenario: 鍵を作る台本は、決めた種類・長さ・有効期限の鍵だけを作る
- **WHEN** 鍵を作る台本を、使い捨ての置き場に向けて走らせる
- **THEN** できた鍵は RSA 4096 bit で、有効期限は作った日から 25 年以上先

#### Scenario: 鍵を作る台本は、決めた長さに満たない鍵を確かめで落とす
- **WHEN** 鍵を作る台本の確かめを、RSA 2048 bit の鍵に当てる
- **THEN** 確かめは 0 でない終了コードで終わる

### Requirement: 開発版は、幹にその日の変更があった日だけ作り直す

THE SYSTEM SHALL 開発版の作り直しを、日本時間の毎日 0:00 に起こす。
WHEN 開発版の作り直しが起きる THE SYSTEM SHALL 印 `nightly` の commit と main の HEAD が同じなら何も組まずに終わり、違えば main の HEAD から開発版の部品（ST54 までは携帯の収集アプリを除く、サーバと PC の収集アプリの 2 部品。深掘り Q4）を組んで開発版を作り直す。
THE SYSTEM SHALL 開発版の版番号を `<出所の版>-dev.<作り直した日の日本時間の YYYYMMDD>` にする。
WHILE 本人が開発版の公開の設定を入れていない THE SYSTEM SHALL 予定の実行で何も組まず、何も公開せず、印 `nightly` も動かさない（手で起こした作り直しだけが組み、そのときも公開しない）。
IF 出所の版の正式版の印 `v<出所の版>` が main の祖先にある THEN THE SYSTEM SHALL 開発版を組まずに終え、出所の版を次の版に上げる必要があることを示す（開発版が、出したばかりの正式版より古い版番号を名乗らないように。深掘り Q6 の推奨）。
THE SYSTEM SHALL 予定の実行が 1 回落ちても、次の回に `nightly` と main の HEAD の違いで同じ変更を拾う（日付の窓で「その日の変更」を決めない）。
IF 作り直しの判定を走らせる木が浅い（正式版の印 `v<出所の版>` が main の祖先かを決められない）THEN THE SYSTEM SHALL 判定を通さず、`build=` の行を出さず（何も組まず、何も公開しない）、出力に「履歴が浅い」を示す（祖先が見えないことを「印が無い」と読んで組まない）。

導出元: FR-107。深掘り Q5（推奨。開発版の公開は ST53 の後に本人が始める）。premises の C6・P7・P17。review/deep.md R16。D24（仮。review/spec.md R29）。

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

#### Scenario: 開発版の作り直しの workflow は、ST54 まで携帯の APK を組む手順を持たない
- **WHEN** 開発版の作り直しの workflow を読む
- **THEN** 携帯の APK を組む手順も、APK に署名する手順も無い。どちらかがあれば、workflow の検査は 0 でない終了コードで終わる

#### Scenario: 開発版の作り直しは、日本時間の 0:00 に起きる
- **WHEN** 開発版の作り直しの予定を読む
- **THEN** 予定は毎日 UTC 15:00（日本時間 0:00）

### Requirement: 手で起こした確かめの組み立ては、下書きのリリースにだけ置き、印を作らない

THE SYSTEM SHALL 本人が手で起こした確かめの組み立て（PR の組み立ての配布物を使う）を、下書き（draft）のリリースにだけ置く。下書きは公開せず、印を作らない。
THE SYSTEM SHALL 下書きのリリースに、本物のリリースと同じ 4 つの配布物と同じリリースの種類（正式版・先行公開）を置く。
THE SYSTEM SHALL 下書きのリリースを作った前と後で、配布元の印 `v*` の数を変えない。

導出元: FR-104, FR-105。深掘り Q5（推奨: ST52 は下書きで確かめ、最初の本物の印は ST53 の後）。premises P16。review/deep.md R7。

#### Scenario: 確かめの組み立ては、印を作らずに下書きのリリースになる
- **GIVEN** PR の組み立てでできた 4 つの配布物
- **WHEN** 確かめの組み立ての手順を、偽の配布元に向けて版番号 `0.1.0-beta.1` で走らせる
- **THEN** 手順は下書きで先行公開のリリースを 1 つ作る要求だけを出し、印を作る要求も公開する要求も出さない

#### Scenario: 確かめの組み立ても、配布物が欠ければ下書きを作らない
- **GIVEN** 4 つの配布物のうち携帯の APK が無い置き場
- **WHEN** 確かめの組み立ての手順を、偽の配布元に向けて走らせる
- **THEN** 手順は 0 でない終了コードで終わり、配布元への要求を 1 つも出さない

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
