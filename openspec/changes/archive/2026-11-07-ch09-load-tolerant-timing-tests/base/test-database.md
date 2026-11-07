# test-database の書いた時点の正典（spec_merge.py snapshot。archive の 3-way の base。手で直さない）

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
