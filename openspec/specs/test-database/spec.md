# test-database Specification

## Purpose
server の試験が DB に入れたデータを、後の試験（同じ回の他の試験も、次の回も）が読まず、前の回の database を次の回が片付け、詰まった問い合わせで試験が数十分止まらないこと。並走する Story と change が同じ機械の上で試験を回し続けられるための、開発の道具の約束。

## Requirements

### Requirement: 試験は回ごとの database に書き、共有の database に記録を積まない

THE SYSTEM SHALL server の試験の過程（`cargo test` が起こす試験の実行ファイル 1 つ）ごとに、移行を当てたテンプレートの database と、それを写した回の database を作り、
試験の足場（`testdb` の `pool()`・`app_pool()`・`url()`）が繋ぐ先をその回の database にする。worktree の DB の `ashiato` database には、試験は記録を書かない。
回の database は、消せない表のトリガ（行の DELETE と表の TRUNCATE を断るもの）を `ashiato` と同じに持つ。試験の足場は、トリガを外さず、無効にせず、`session_replication_role` を変えない。
導出元: CH04

#### Scenario: 試験の足場の繋ぎ先は、その回の database である
- **WHEN** 試験が `testdb::pool()` と `testdb::app_pool()` で繋ぎ、`current_database()` を読む
- **THEN** どちらも同じ回の database の名前で、`ashiato` ではない
- **AND** `testdb::url()` の database の部分も同じ名前である

#### Scenario: 試験が置いた記録は、ashiato database に入らない
- **WHEN** 試験が `testdb::pool()` で `core.event` に記録を 1 件置く
- **THEN** その記録は回の database にあり、`ashiato` database の `core.event` には無い

#### Scenario: 回の database でも、消せない表は行の削除と TRUNCATE を断る
- **WHEN** 試験が回の database で `core.event` の行を `DELETE` し、`core.event` を `TRUNCATE` する
- **THEN** どちらもトリガが断って失敗する
- **AND** 回の database の `core.event` の書き換え禁止のトリガは有効のままである

### Requirement: 書き出しの試験は、試験ごとの database で走り、試験の終わりに消える

THE SYSTEM SHALL 書き出しの試験（`export_*tests.rs` の試験）に、回のテンプレートを写した試験ごとの database を渡し、同じ回の他の試験が置いた記録と消した滞在をその試験の書き出しが判定しないようにする。
試験ごとの database は、その試験が通ったときも、assert が外れた・panic したときも、試験が終わるまでに消す。
試験の過程が外から殺されたときの残りは、次の回の始まりの片付けが消す。
導出元: CH04

#### Scenario: 書き出しの試験は、同じ回の他の試験が消した滞在を判定しない
- **WHEN** 回の database に消した滞在と、その時間に入る緯度経度つきの記録がある状態で、書き出しの試験が試験ごとの database を受け取って件数を数える
- **THEN** 試験ごとの database の記録と消した滞在の数は 0 件である（移行が置く行のほかに何も無い）

#### Scenario: 試験ごとの database は、試験が通れば終わりに消える
- **WHEN** 試験が試験ごとの database を受け取り、その名前を控えて、試験を終える
- **THEN** 試験の後、その名前の database は無い

#### Scenario: 試験ごとの database は、試験が panic しても消える
- **WHEN** 試験ごとの database を受け取って記録を置いた後に panic する処理を走らせ、panic を受け止める
- **THEN** panic が伝わった後、その database は無い

### Requirement: 試験が作る使い捨ての database は、回の印を持つ名前で、試験の足場だけが作る

THE SYSTEM SHALL 試験が移行の当て方などを見るために作る空の database を、試験の足場の関数（`testdb`）だけが、回の印を持つ名前で作る。
`crates/server` の試験のコードが、試験の足場の外で `CREATE DATABASE` を書いたとき、または書き出しの試験のファイル（`crates/server/src/export*_tests.rs`）が回の database（`testdb::pool()`）を取ったとき、
検査（`tools/check-test-db-growth.sh --lint`）がファイルと行を挙げて rc=1 で落ち、何を使えばよいかを示す。行の同じ行か 1 行上に理由つきの `db-ok: <理由>` があれば通す。
導出元: CH04

#### Scenario: 使い捨ての database の名前は、回の database の名前で始まる
- **WHEN** 試験が試験の足場から空の使い捨ての database を受け取る
- **THEN** その名前は回の database の名前で始まる

#### Scenario: 足場の外で database を作る試験は、検査が落とし、足場の関数を示す
- **WHEN** `crates/server/src` の試験のファイルに `CREATE DATABASE` の行がある木で `tools/check-test-db-growth.sh --lint` を走らせる
- **THEN** rc=1 で、そのファイルと行番号を挙げる
- **AND** 文言に、使う試験の足場の関数の名前が出る

#### Scenario: 書き出しの試験が回の database を取ると、検査が落とし、試験ごとの database を示す
- **WHEN** `crates/server/src/export_x_tests.rs` に `testdb::pool()` の行がある木で `tools/check-test-db-growth.sh --lint` を走らせる
- **THEN** rc=1 で、そのファイルと行番号を挙げる
- **AND** 文言に `testdb::isolated()` が出る

#### Scenario: CH04 の後の木は、database を作る書き方の検査を通る
- **WHEN** CH04 を当てた木で `tools/check-test-db-growth.sh --lint` を走らせる
- **THEN** rc=0 である

### Requirement: 前の回の database は、次の回の始まりに消え、走っている回のものは消えない

THE SYSTEM SHALL 回の初めに、走っていない回の database（テンプレート・回・試験ごと・使い捨て）と、回の印を持たない使い捨ての database の残り（`*_tmp_*`）を `DROP DATABASE … WITH (FORCE)` で消す。
回が走っているかは、回の間ずっと持つ助言ロックで判定し、繋がっている接続の数では判定しない。走っている回の database は消さない。
片付けはトリガを撃たない（database ごと消す）。`ashiato` と、回の印も `_tmp_` も持たない database は消さない。
導出元: CH04

#### Scenario: 持ち主のいない回の database は、次の回の始まりに消える
- **WHEN** 助言ロックを誰も持っていない回の名前で、テンプレート・回・試験ごと・使い捨ての database を作っておき、新しい回の片付けを走らせる
- **THEN** その 4 つの database はどれも無い

#### Scenario: 走っている回の database は、接続が 0 本でも消えない
- **WHEN** 別の接続がある回の助言ロックを持ち、その回の database には接続が 1 本も無い状態で、新しい回の片付けを走らせる
- **THEN** その回の database は残っている

#### Scenario: 回の印の無い使い捨ての database の残りは消え、ashiato と他の database は残る
- **WHEN** `x_tmp_` を含む名前の database と、回の印も `_tmp_` も持たない名前の database がある状態で、新しい回の片付けを走らせる
- **THEN** `_tmp_` を含む方は無く、もう一方と `ashiato` は残っている

### Requirement: 試験の 1 本の問い合わせは、時間の上限を超えたら止まって試験が落ちる

THE SYSTEM SHALL 試験の足場が作る接続（`pool()`・`app_pool()`・試験ごとと使い捨ての database）に `statement_timeout` を置き、上限を超えた問い合わせを PostgreSQL に止めさせ、その試験を失敗させる。
上限の既定は 60 秒とする。環境変数 `ASHIATO_TEST_STATEMENT_TIMEOUT` があればその値（PostgreSQL の時間の書き方）にし、`0` なら上限を置かない（量を測るときに使う）。
導出元: CH04

#### Scenario: 上限を超える問い合わせは、止まって失敗する
- **WHEN** 上限を 1 秒にした試験の足場の接続で `SELECT pg_sleep(5)` を走らせる
- **THEN** 問い合わせは 5 秒を待たずに失敗し、エラーは statement timeout による取り消し（SQLSTATE `57014`）である

#### Scenario: 上限の既定は 60 秒で、試験の足場の接続に入っている
- **WHEN** 環境変数を渡さずに `testdb::pool()` で繋ぎ、`SHOW statement_timeout` を読む
- **THEN** `1min` である

#### Scenario: 環境変数の値が上限になり、0 なら上限を置かない
- **WHEN** 上限を決める足場の関数に、環境変数が無いとき・`1s` のとき・`0` のときの値を渡す
- **THEN** それぞれ 60 秒・1 秒・上限なしになる

### Requirement: 試験を 2 回続けて走らせても前の回の database が残らないことを、台本と CI が確かめる

THE SYSTEM SHALL `tools/check-test-db-growth.sh` で、与えた `cargo test` の引数で試験を 2 回続けて走らせ、各回の前後の database の一覧を比べて、
1 回目に現れた database が 2 回目の後に 1 つも無いこと・2 回目の後に残るのはその回のテンプレートと回の database だけで、試験ごとと使い捨ての database が無いこと・
`ashiato` を消していないことを見る。どれかが外れれば、残った database の名前を挙げて rc=1 で落ちる。試験が落ちればその終了コードで落ちる。
`--self-test` は偽の試験と偽の一覧で、この判定そのものを確かめる。CI の rust job は、`--self-test` と、書き出しの試験と使い捨ての database を作る試験を含む絞り込みでこの台本を走らせる。
導出元: CH04

#### Scenario: 前の回の database が 2 回目の後に残っていれば、名前を挙げて落ちる
- **WHEN** `--self-test` の偽の試験が 1 回目の回の database を 2 回目の後にも残す
- **THEN** 台本は rc=1 で、その database の名前を挙げる

#### Scenario: 2 回目の試験ごとの database が残っていれば、名前を挙げて落ちる
- **WHEN** `--self-test` の偽の試験が 2 回目に試験ごとの database を 1 つ残す
- **THEN** 台本は rc=1 で、その database の名前を挙げる

#### Scenario: ashiato database が消えていれば落ちる
- **WHEN** `--self-test` の偽の一覧で、2 回目の後に `ashiato` が無い
- **THEN** 台本は rc=1 である

#### Scenario: 試験が落ちれば、台本も試験の終了コードで落ちる
- **WHEN** `--self-test` の偽の試験が rc=101 で終わる
- **THEN** 台本は rc=101 で終わる

#### Scenario: 前の回が片付き、最後の回の分だけが残れば、台本は通る
- **WHEN** `--self-test` の偽の試験が、1 回目の分を 2 回目の始まりに消し、2 回目のテンプレートと回の database だけを残す
- **THEN** 台本は rc=0 である

#### Scenario: CI の rust job は台本を走らせる
- **WHEN** `.github/workflows/ci.yml` の rust job を読む
- **THEN** `tools/check-test-db-growth.sh --self-test` と `tools/check-test-db-growth.sh --lint` と、`cargo test` の引数を渡した `tools/check-test-db-growth.sh` を走らせる

### Requirement: 試験を途中で殺しても、次の回の後に残りが無いことを、台本と CI が確かめる

THE SYSTEM SHALL `tools/check-test-db-killed.sh` で、体積の行（利用者 1 人に記録 20,001 件）を回の database と試験ごとの database に入れる試験を、
(a) 体積の行を入れる文の途中（時点の名前 `insert`。文が走り始め、行を 1 つも書く前）と、(b) 体積の行を確定した後・試験が終わる前（`committed`）、の 2 つの時点で外から SIGKILL で殺し、それぞれの後に server の試験の回を 1 つ走らせて、
次を順に見て、最初に外れたもので落ちる: 次の回の試験の rc（0 でなければその rc）/ 殺した回が作った database（回・テンプレート・試験ごと）が次の回の後に 1 つも無い（残れば名前を挙げて rc=1）/
`ashiato` database の `core.event` の行数（表が無ければ 0）が殺す前と同じ（違えば前と後を挙げて rc=1）/ `ashiato` database の大きさが殺す前から 1 MiB を超えて増えていない（超えれば前と後を挙げて rc=1）。
殺した回の database は、探りの試験を起こす前と殺した後の一覧の差で決め、差が空なら（検査が空振り）rc=1 で落ちる。探りの試験がその時点に着いた印を 120 秒待っても来なければ、時点の名前を挙げて rc=1 で落ちる。
落ちたときは、どの時点で落ちたかを出す。次の回にかかった秒数は標準出力に出すが、合否には使わない。
`--self-test` は偽の探り・偽の次の回・偽の一覧と統計で、この判定そのものを確かめる。CI の rust job は `--self-test` と、本物の試験での検査を走らせる。
導出元: CH05

#### Scenario: 体積の行を入れる途中で殺した回の database は、次の回の後に残らない
- **WHEN** 体積の行を入れる文が走っている間に試験の実行ファイルを SIGKILL で殺し、続けて server の試験の回を 1 つ走らせる
- **THEN** 殺した回が作った database は、どれも次の回の後に無い
- **AND** `ashiato` database の `core.event` の行数は殺す前と同じで、大きさは殺す前から 1 MiB を超えて増えていない

#### Scenario: 体積の行を確定した後に殺した回の database は、次の回の後に残らない
- **WHEN** 体積の行を回の database と試験ごとの database に確定した後、試験が終わる前に試験の実行ファイルを SIGKILL で殺し、続けて server の試験の回を 1 つ走らせる
- **THEN** 殺した回が作った database（回・テンプレート・試験ごと）は、どれも次の回の後に無い
- **AND** `ashiato` database の `core.event` の行数は殺す前と同じで、大きさは殺す前から 1 MiB を超えて増えていない
- **AND** 次の回にかかった秒数が標準出力に出る

#### Scenario: 殺した回の database が次の回の後に残っていれば、名前を挙げて落ちる
- **WHEN** `--self-test` の偽の次の回が、殺した回の database を消さずに残す
- **THEN** 台本は rc=1 で、その database の名前と時点の名前を挙げる

#### Scenario: ashiato database の行数が増えていれば落ちる
- **WHEN** `--self-test` の偽の統計で、次の回の後の `ashiato` database の `core.event` の行数が殺す前より多い
- **THEN** 台本は rc=1 で、殺す前と後の行数を挙げる

#### Scenario: ashiato database の大きさだけが 1 MiB を超えて増えていれば落ちる
- **WHEN** `--self-test` の偽の統計で、行数は殺す前と同じで、大きさが殺す前より 1 MiB と 1 バイト大きい
- **THEN** 台本は rc=1 で、殺す前と後の大きさを挙げる

#### Scenario: 殺した回の database が見えなければ、空振りとして落ちる
- **WHEN** `--self-test` の偽の探りが database を 1 つも作らずに殺される
- **THEN** 台本は rc=1 で、殺した回の database が見えないことを挙げる

#### Scenario: 探りの印が来なければ、待ちの上限で時点の名前を挙げて落ちる
- **WHEN** `--self-test` の偽の探りが印を書かずに眠り、待ちの上限を短くして台本を走らせる
- **THEN** 台本は rc=1 で、印が来なかったことと時点の名前を挙げる

#### Scenario: 次の回の試験が落ちれば、台本も試験の終了コードで落ちる
- **WHEN** `--self-test` の偽の次の回が rc=101 で終わる
- **THEN** 台本は rc=101 で終わる

#### Scenario: 殺した回の分が片付き、ashiato が増えていなければ、台本は通る
- **WHEN** `--self-test` の偽の次の回が殺した回の database を消し、`ashiato` database の行数と大きさを変えない
- **THEN** 台本は rc=0 である

#### Scenario: CI の rust job は、殺した後の検査を走らせる
- **WHEN** `.github/workflows/ci.yml` の rust job を読む
- **THEN** `tools/check-test-db-killed.sh --self-test` と、引数なしの `tools/check-test-db-killed.sh` を走らせる
