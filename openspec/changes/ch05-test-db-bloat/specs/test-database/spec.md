## ADDED Requirements

### Requirement: 検証の準備は、膨れた試験用 DB の database を作り直す

THE SYSTEM SHALL 検証の準備（`tools/verify-env.sh`）のたびに、作業ツリーのテスト用 DB の `ashiato` database の大きさを測り、閾値を超えていれば、
その database を消して空の database として作り直す。閾値の既定は 64 MiB（67,108,864 バイト）で、ちょうどその大きさは作り直さない。環境変数で閾値を替えられる。
作り直すのは database だけで、DB のコンテナ・ネットワーク・volume と、同じ DB にある他の database には触らない。
作り直した database の所有者は所有者の役割（`ashiato_owner`）で、移行は当てない（使う側 —— `tests/server_startup.rs`・縦串・開発用の起動 —— が今までどおり自分で当てる）。
対象の database が無ければ、大きさを測る前に空の database として作る（消した後に作れなかった検証の準備の次の回で、無いまま残らない）。
作り直したときは、作り直したこと・database の名前・作り直す前の大きさを標準エラーに 1 行出す。閾値以下なら何もしない。
作り直してよいのは、Story・change の作業ツリー（名前が `<main の名前>-st<NN>`・`-ch<NN>`・`-up-<…>`）だけとする。main の checkout・確認バッチの作業ツリー（`-verify`）・それ以外の作業ツリーでは、
閾値を超えていても大きさを標準エラーに出すだけで作り直さない。作業ツリーの種類の規則は、port の規則と同じ 1 か所（`tools/ports.sh`）に置き、port もその種類から決める。
作り直しの判定が DB に問えない・見送り以外の理由で失敗したときは、理由を標準エラーに出して rc≠0 で終わり、検証の準備も rc≠0 で終わる。
導出元: CH05

#### Scenario: 閾値を超えた database は、検証の準備で空の database に作り直される
- **WHEN** Story の作業ツリーで、対象の database に閾値を超える行が入り、同じ DB の別の database にも行が入っている状態で作り直しの判定を走らせる
- **THEN** 判定は rc=0 で終わり、対象の database は残っていて、入っていた行は無い
- **AND** 対象の database の所有者は `ashiato_owner` である
- **AND** 別の database の行はそのまま残る
- **AND** 標準エラーに、作り直したことと database の名前と作り直す前の大きさが出る

#### Scenario: 閾値の既定は 64 MiB で、ちょうどは作り直さず、超えれば作り直す
- **WHEN** 閾値の環境変数を渡さずに、大きさ 67,108,864 バイトと 67,108,865 バイトを作り直しの判定の決め方に渡す
- **THEN** 前は作り直さない、後は作り直す、と決まる

#### Scenario: 閾値以下の database は作り直さない
- **WHEN** Story の作業ツリーで、対象の database が閾値以下の状態で作り直しの判定を走らせる
- **THEN** 判定は rc=0 で終わり、入っていた行はそのまま残る

#### Scenario: main の checkout と確認バッチの作業ツリーでは、閾値を超えても大きさを出すだけで作り直さない
- **WHEN** 作業ツリーの種類が main の checkout か確認バッチで、対象の database が閾値を超えている状態で作り直しの判定を走らせる
- **THEN** 判定は rc=0 で終わり、入っていた行はそのまま残る
- **AND** 標準エラーに、作り直さなかったことと database の大きさが出る

#### Scenario: 対象の database が無ければ、判定が空の database を作る
- **WHEN** Story の作業ツリーで、対象の database が無い状態で作り直しの判定を走らせる
- **THEN** 判定は rc=0 で終わり、対象の database がある

#### Scenario: 作業ツリーの名前から、作り直してよいかが決まる
- **WHEN** main の名前が `ashiato2` のとき、作業ツリーの種類の規則に `ashiato2-st05`・`ashiato2-ch05`・`ashiato2-up-ch05`・`ashiato2-up-st23`・`ashiato2`（main の checkout）・`ashiato2-verify`・`ashiato2-rec-st22`・`ashiato2-tools` を渡す
- **THEN** 前の 4 つは作り直してよい種類で、後の 4 つは作り直さない種類である

#### Scenario: 作り直した database に、移行を当て直せる
- **WHEN** 作り直した後の対象の database に、所有者の接続でサーバの `migrate` を走らせる
- **THEN** `migrate` は rc=0 で終わり、その database の消せない表は行の DELETE を断る

#### Scenario: 検証の準備は、膨れた ashiato を作り直してから終わる
- **WHEN** Story の作業ツリーで、`ashiato` に印の表を作り、`ashiato` に接続も走っている回も無い状態で、閾値を 0 MiB にして `tools/verify-env.sh` を走らせる
- **THEN** `tools/verify-env.sh` は rc=0 で終わり、標準エラーに `ashiato` を作り直した行が出る
- **AND** `ashiato` に印の表は無く、所有者の接続で `ashiato` に繋げる

#### Scenario: 作り直しの判定が DB に問えなければ、判定も検証の準備も rc≠0 で落ちる
- **WHEN** 作り直しの判定が DB に問う命令を、必ず失敗する命令に替えて、作り直しの判定と `tools/verify-env.sh` を走らせる
- **THEN** どちらも rc≠0 で終わり、標準エラーに理由が出る

### Requirement: 使っている途中の試験用 DB は作り直さない

THE SYSTEM SHALL 作り直しの判定で、次のどれかに当たるときは対象の database を作り直さず、見送ったことと理由を標準エラーに 1 行出して rc=0 で終える（検証は今の database のまま続く）:
走っている試験の回がある（回の間ずっと持つ助言ロックを誰かが持っている）/ 回を作っている・片付けている途中である（回の作成と片付けを直列にする助言ロックを取れない）/ 対象の database にクライアントの接続が 1 本でもある /
消そうとしたときに PostgreSQL が「使われている」と断った。見送りは待たずに決める。
判定から作り直しの終わりまで、回の作成と片付けを直列にする助言ロックを持ち、その間に新しい回が database を作り始めないようにする。対象の database を接続ごと切って消すことはしない。
導出元: CH05

#### Scenario: 走っている回の助言ロックがあれば、作り直さずに見送る
- **WHEN** 別の接続が回の助言ロックを持ち、対象の database が閾値を超えている状態で作り直しの判定を走らせる
- **THEN** 判定は rc=0 で終わり、入っていた行はそのまま残る
- **AND** 標準エラーに、見送ったことと、走っている回があるという理由が出る

#### Scenario: 回を作っている途中なら、作り直さずに見送る
- **WHEN** 別の接続が回の作成と片付けを直列にする助言ロックを持ち、対象の database が閾値を超えている状態で作り直しの判定を走らせる
- **THEN** 判定は 10 秒以内に rc=0 で終わり、入っていた行はそのまま残る
- **AND** 標準エラーに、見送ったことと理由が出る

#### Scenario: 対象の database に接続があれば、作り直さずに見送り、その接続は切られない
- **WHEN** 別の接続が対象の database に繋がったまま、対象の database が閾値を超えている状態で作り直しの判定を走らせる
- **THEN** 判定は rc=0 で終わり、入っていた行はそのまま残る
- **AND** 繋がっていた接続は、判定の後も `SELECT 1` を返す
- **AND** 標準エラーに、見送ったことと、接続があるという理由が出る

#### Scenario: 判定の後に繋いだ接続があれば、消すのを PostgreSQL が断り、見送りになる
- **WHEN** 作り直しの判定が接続の確かめを終えて消す前に止まっている間に、別の接続が対象の database に繋ぐ
- **THEN** 判定は rc=0 で終わり、入っていた行はそのまま残り、繋いだ接続は `SELECT 1` を返す
- **AND** 標準エラーに、見送ったことと、接続があるという理由が出る

#### Scenario: 作り直しの間は、別の接続が回の作成の鍵を取れない
- **WHEN** 作り直しの判定が接続の確かめを終えて消す前に止まっている間に、別の接続が回の作成と片付けを直列にする助言ロックを待たずに取ろうとする
- **THEN** 取れない
- **AND** 判定が終わった後は取れる

### Requirement: 試験を途中で殺しても、次の回の後に残りが無いことを、台本と CI が確かめる

THE SYSTEM SHALL `tools/check-test-db-killed.sh` で、体積の行（利用者 1 人に記録 20,001 件）を回の database と試験ごとの database に入れる試験を、
(a) 体積の行を入れる文の途中（時点の名前 `insert`）と、(b) 体積の行を確定した後・試験が終わる前（`committed`）、の 2 つの時点で外から SIGKILL で殺し、それぞれの後に server の試験の回を 1 つ走らせて、
次を順に見て、最初に外れたもので落ちる: 次の回の試験の rc（0 でなければその rc）/ 殺した回が作った database（回・テンプレート・試験ごと）が次の回の後に 1 つも無い（残れば名前を挙げて rc=1）/
`ashiato` database の `core.event` の行数が殺す前と同じ（違えば前と後を挙げて rc=1）/ `ashiato` database の大きさが殺す前から 1 MiB を超えて増えていない（超えれば前と後を挙げて rc=1）。
殺した回の database は、探りの試験を起こす前と殺した後の一覧の差で決め、差が空なら（検査が空振り）rc=1 で落ちる。探りの試験がその時点に着いた印を 120 秒待っても来なければ、時点の名前を挙げて rc=1 で落ちる。
落ちたときは、どの時点で落ちたかを出す。次の回にかかった秒数は標準出力に出すが、合否には使わない。
`committed` の時点で探りが眠っている間（回の助言ロックを持っている間）に作り直しの判定を走らせ、走っている回があるとして見送ることも確かめる。
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
- **AND** 探りが眠っている間に走らせた作り直しの判定は、走っている回があるとして見送っている
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

#### Scenario: CI の rust job は、殺した後の検査と作り直しの検査を走らせる
- **WHEN** `.github/workflows/ci.yml` の rust job を読む
- **THEN** `tools/check-test-db-killed.sh --self-test` と、引数なしの `tools/check-test-db-killed.sh` と、`tools/test-db-rebuild.sh --self-test` を走らせる
