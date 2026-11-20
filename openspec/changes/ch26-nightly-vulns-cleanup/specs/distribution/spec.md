## ADDED Requirements

### Requirement: 同梱の PostgreSQL は 17.12-1 で、大きい版は 17 のまま

THE SYSTEM SHALL 配る zip に同梱する PostgreSQL の版（`tools/release/pgsql.lock` の `version`）を `17.12-1` にし、`url` はその版の EDB の Windows の zip（`postgresql-17.12-1-windows-x64-binaries.zip`）を指し、`sha256` はその zip の値にする。
THE SYSTEM SHALL lock の `dlls` を、17.12-1 の zip の実行ファイル（`executables` の一覧）から辿れる DLL にそろえ、Windows と VC の実行時の DLL の注記を 17.12-1 の zip で調べた結果にする。
大きい版（17）・実行ファイルの一覧・DB の置き場の名前（`db\17`）は変えない。同梱の版の遅れの検査（`tools/check-pg-minor.sh`）は、17 の最新 minor が 17.12 である間 rc=0 で終わる（17.13 が出れば、今までどおり遅れとして落ちる）。
導出元: CH26

#### Scenario: lock の版と URL は 17.12-1 を指し、大きい版は 17
- **WHEN** 確かめの台本が `tools/release/pgsql.lock` を読む
- **THEN** `version` は `17.12-1`、`url` は `postgresql-17.12-1-windows-x64-binaries.zip` で終わり、`version` の `.` の前は `17` で、`sha256` は 64 桁の 16 進である

#### Scenario: 同梱の版の遅れの検査が通る
- **WHEN** 確かめの台本が、17 の最新 minor を `12`・公開日を `2026-11-12` とする版の一覧の写し（`PG_VERSIONS_URL` に `file://` で渡す）に対して、今の `tools/release/pgsql.lock` で `tools/check-pg-minor.sh` を走らせる
- **THEN** rc=0 で終わる（同梱の版が最新の minor と同じ）

### Requirement: 17.6 が使っていた DB の置き場を、17.12 がそのまま使い、17.6 に戻しても使える

WHEN 同梱の PostgreSQL が 17.6 から 17.12 に替わるとき、THE SYSTEM SHALL 17.6 で作り、サーバの移行を当てて記録を入れた DB の置き場を、作り直さず・移さずに 17.12 で起こし、同じ中身を返す。
WHEN 17.12 で記録を足した後に 17.6 へ戻すとき、THE SYSTEM SHALL 同じ置き場を 17.6 で起こし、足した記録を含めて返す。
中身が同じとは、`pg_dump` の平文の出力から版の行（`-- Dumped from` / `-- Dumped by` で始まる行）と毎回変わる鍵の行（`\restrict` / `\unrestrict` で始まる行）を除いたものが、同じ pg_dump（17.12）で取って一致することをいう。
この確かめは PostgreSQL の置き場の入れ替えを Linux で走らせて行い、Windows の配布物どうしの更新は本人の PC での確かめ（確認バッチ）に回す（design D5）。
導出元: CH26

#### Scenario: 17.6 で移行を当てた置き場を、17.12 が同じ中身で使う
- **WHEN** 確かめの台本が、使い捨ての置き場で `postgres:17.6` を起こしてサーバの移行（`ashiato-server migrate`）を当て、表に行を入れてから止め、同じ置き場で `postgres:17.12` を起こす
- **THEN** 17.12 は rc=0 で起き、`pg_dump` の出力（上の除いた形）は 17.6 で起きていたときに取った出力と同じで、同じ移行をもう一度当てると rc=0 で終わり、当てた移行の数は変わらない

#### Scenario: 17.12 で足した記録を、17.6 に戻しても読める
- **WHEN** 確かめの台本が、上の 17.12 で表に 1 行足してから止め、同じ置き場で `postgres:17.6` を起こす
- **THEN** 17.6 は rc=0 で起き、`pg_dump` の出力（上の除いた形）は 17.12 で行を足した後に取った出力と同じで、足した行がある
