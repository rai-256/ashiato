## ADDED Requirements

### Requirement: 同梱の PostgreSQL の小さい版を上げるときは、lock の版・URL・指紋をそろえ、大きい版と置き場は変えない

WHEN 同梱の PostgreSQL の小さい版を上げるとき、THE SYSTEM SHALL `tools/release/pgsql.lock` の `version`・`url`（その版の EDB の Windows の zip）・`sha256`（その zip の値）をそろえて替え、
大きい版・`executables`・`license`・`license_destination` は変えない（DB の置き場の名前 `db\<大きい版>` は大きい版から決まるので変わらない）。
THE SYSTEM SHALL lock の `dlls` を、その版の zip の `executables` から辿れる同梱の DLL の集まりにそろえる。
（CH26 では 17.6-1 を 17.12-1 にした。版の時点の事実は `docs/changes/CH26.md` の完了の判定が持つ）
導出元: CH26

#### Scenario: lock の url は lock の version の zip を指す
- **WHEN** 確かめの台本が `tools/release/pgsql.lock` を読む
- **THEN** `url` は `postgresql-<version>-windows-x64-binaries.zip` で終わる

#### Scenario: 小さい版を上げても、大きい版と実行ファイルと許諾の置き場は変わらない
- **WHEN** 確かめの台本が、比べる元の版と今の `tools/release/pgsql.lock` を読む
- **THEN** `version` の `.` の前・`executables`・`license`・`license_destination` は元の版と同じ

#### Scenario: lock の DLL の一覧は、zip の実行ファイルから辿れる同梱の DLL と一致する
- **WHEN** 確かめの台本が、lock の zip を落として展開し、`executables` の実行ファイルから `x86_64-w64-mingw32-objdump -p` の `DLL Name:` を辿って、zip の `bin` にある DLL の集まりを作る（Windows と VC の実行時の DLL は数えない）
- **THEN** その集まりは lock の `dlls` と同じである

#### Scenario: 同梱の版が最新の minor なら、遅れの検査は通る
- **WHEN** 確かめの台本が、大きい版の最新 minor を lock の小さい版とする版の一覧の写し（`PG_VERSIONS_URL`）と、EDB の写し（`PG_EDB_URL_BASE`。どちらも `file://`）で `tools/check-pg-minor.sh` を走らせる
- **THEN** rc=0 で終わる

### Requirement: 小さい版を上げる前に使っていた DB の置き場を、上げた版がそのまま使い、前の版に戻しても使える

WHEN 同梱の PostgreSQL が同じ大きい版の中で小さい版を上げるとき、THE SYSTEM SHALL 前の小さい版で作り、サーバの移行を当てて記録を入れた DB の置き場を、作り直さず・移さずに新しい小さい版で起こし、同じ中身を返す。
WHEN 新しい小さい版で記録を足した後に前の小さい版へ戻すとき、THE SYSTEM SHALL 同じ置き場を前の小さい版で起こし、足した記録を含めて返す。
WHEN 新しい小さい版の `pg_dump` で取った退避を、前の小さい版の `pg_restore` で空の DB に戻すとき（インストーラの戻しの道）、THE SYSTEM SHALL 退避を取ったときと同じ中身を返す。
導出元: CH26

#### Scenario: 前の小さい版で移行を当てた置き場を、新しい小さい版が同じ中身で使う
- **WHEN** 確かめの台本が、使い捨ての置き場を前の小さい版で作ってサーバの移行を当て、表に行を入れてから止め、同じ置き場を新しい小さい版で起こす
- **THEN** 新しい小さい版は起き、DB の中身は前の小さい版で起きていたときと同じである

#### Scenario: 新しい小さい版で移行をもう一度当てても、足す移行は無い
- **WHEN** 確かめの台本が、上の新しい小さい版の DB にサーバの移行をもう一度当てる
- **THEN** rc=0 で終わり、当てた移行の数は変わらない

#### Scenario: 新しい小さい版で足した記録を、前の小さい版に戻しても読める
- **WHEN** 確かめの台本が、新しい小さい版で表に 1 行足してから止め、同じ置き場を前の小さい版で起こす
- **THEN** 前の小さい版は起き、DB の中身は新しい小さい版で行を足した後と同じで、足した行がある

#### Scenario: 新しい小さい版で取った退避を、前の小さい版が戻せる
- **WHEN** 確かめの台本が、新しい小さい版の `pg_dump` で（インストーラの退避と同じ形式で）退避を取り、前の小さい版の空の DB に前の小さい版の `pg_restore` で戻す
- **THEN** `pg_restore` は rc=0 で終わり、戻した DB の中身は退避を取ったときと同じである
