# distribution の書いた時点の正典（spec_merge.py snapshot。archive の 3-way の base。手で直さない）

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
