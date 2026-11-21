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

### Requirement: 同梱の PostgreSQL の lock を変える PR では、前の lock で組んだ試験の zip から今の lock の zip への更新と戻しを Windows で通す

WHEN PR が `tools/release/pgsql.lock` の中身を変え、PR の base の lock と大きい版が同じとき、THE SYSTEM SHALL CI の Windows の組み立てで、更新の試験の古い側の zip（`package-windows-test-old`）だけを base の lock で組み、新しい側の zip は PR の lock で組み、Windows の runner で古い側から新しい側への更新と、移行が落ちたときの古い側への戻しを通す。
IF PR の base の lock と PR の lock の大きい版が違う THEN THE SYSTEM SHALL 古い側を組まずに CI を落とす（大きい版を上げる道は大きい版を上げる Story が扱う）。
WHEN PR が lock を変えないか、PR でない run のとき、THE SYSTEM SHALL 試験の zip を今のまま全部 PR の lock で組む。
THE SYSTEM SHALL 組んだ後に、試験の zip が同梱する PostgreSQL の版が上の決め方どおりであることを確かめ、違えば CI を落とす。
THE SYSTEM SHALL Windows の組み立ての使い回しの鍵に、古い側を組んだ lock を入れる（前の lock が変われば組み直す）。
導出元: CH26（本人の決定 R2 = C）

#### Scenario: lock の中身が違い大きい版が同じなら、古い側を前の lock で組む
- **WHEN** 確かめの台本が、版 `17.6-1` の前の lock と版 `17.12-1` の今の lock で `tools/release/pgsql-prev-lock.sh decide` を走らせる
- **THEN** rc=0 で `swap` を出す

#### Scenario: lock の中身が同じなら、今のまま組む
- **WHEN** 確かめの台本が、同じ中身の 2 つの lock で `tools/release/pgsql-prev-lock.sh decide` を走らせる
- **THEN** rc=0 で `same` を出す

#### Scenario: 前の lock と大きい版が違えば、古い側を組まずに落ちる
- **WHEN** 確かめの台本が、版 `16.4-1` の前の lock と版 `17.12-1` の今の lock で `tools/release/pgsql-prev-lock.sh decide` を走らせる
- **THEN** rc≠0 で、出力に `16` と `17` がある

#### Scenario: 組んだ zip の同梱の版を、許諾の文から読む
- **WHEN** 確かめの台本が、`THIRD-PARTY-NOTICES.txt` の PostgreSQL の節に `version: 17.6-1` を持つ使い捨てのサーバの zip で `tools/release/pgsql-prev-lock.sh zip-version` を走らせる
- **THEN** rc=0 で `17.6-1` を出し、その節が無い zip では rc≠0 で終わる

#### Scenario: lock を変えた PR では、前の lock の版の zip から今の lock の版の zip へ更新し、戻しも通る
- **WHEN** `tools/release/pgsql.lock` を `17.6-1` から `17.12-1` へ変える PR の CI が走る
- **THEN** `package-windows` は古い側の zip の版 `17.6-1` と新しい側の zip の版 `17.12-1` を確かめて出力に出し、`install-windows` の `tools/release/update-apply-install.sh` はその 2 つの zip で全部の Scenario を通す

#### Scenario: CI の定義は、古い側だけを前の lock で組み、版を確かめ、前の lock を鍵に入れる
- **WHEN** 確かめの台本が `tools/release/pgsql-prev-lock.sh check-workflow` で `.github/workflows/ci.yml` の `package-windows` を読む
- **THEN** rc=0 で、`decide` を呼ぶ step が鍵の step より前にあり、鍵の step の `WINDOWS_BUILD_KEY_TOOLS` が `prev-pgsql=` の項を持ち、試験の zip を組む step は `old` を組む間だけ `tools/release/pgsql.lock` を置き換えて組んだ後に戻し、`zip-version` で版を確かめる step は使い回したときも走る（`if:` に `cache-hit` を持たない）

#### Scenario: CI の定義の形が崩れると、確かめは落ちる
- **WHEN** 確かめの台本が、上の形のどれか 1 つを崩した `ci.yml` の写し（鍵の step から `prev-pgsql=` を消す・`old` 以外も置き換える・置き換えを戻さない・版の確かめに `cache-hit` の条件を付ける）で `tools/release/pgsql-prev-lock.sh check-workflow` を走らせる
- **THEN** どの写しでも rc≠0 で終わる
