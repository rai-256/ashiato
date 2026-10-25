## ADDED Requirements

### Requirement: 試験を途中で殺しても、次の回の後に残りが無いことを、台本と CI が確かめる

THE SYSTEM SHALL `tools/check-test-db-killed.sh` で、体積の行（利用者 1 人に記録 20,001 件）を回の database と試験ごとの database に入れる試験を、
(a) 体積の行を入れる文の途中（時点の名前 `insert`）と、(b) 体積の行を確定した後・試験が終わる前（`committed`）、の 2 つの時点で外から SIGKILL で殺し、それぞれの後に server の試験の回を 1 つ走らせて、
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
