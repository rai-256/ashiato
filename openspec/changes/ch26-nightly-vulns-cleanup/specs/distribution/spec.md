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

### Requirement: 同梱の PostgreSQL の lock を変える PR では、変える前の lock で組んだ試験の zip から今の lock の zip への更新と戻しを Windows で通す

WHEN PR の差分（PR の base との分岐点から PR の head まで）が `tools/release/pgsql.lock` を変え、分岐点の lock と PR の lock の大きい版が同じとき、THE SYSTEM SHALL CI の Windows の組み立てで、更新の試験の古い側の zip（`package-windows-test-old`）だけを分岐点の lock で組み、新しい側の zip（`package-windows-test-new`）とほかの試験の zip は PR の lock で組み、Windows のインストールの試験の job で、古い側を入れた上での新しい側への更新と、移行が落ちたときの古い側への戻しを、今の更新の試験の Scenario 群（`tools/release/update-apply-install.sh`）で通す。
IF PR の差分が lock を変えたのに、分岐点の lock が取れないか、分岐点の lock と大きい版が違う THEN THE SYSTEM SHALL 古い側を組まずに CI を落とす（大きい版を上げる道は大きい版を上げる Story が扱う）。
WHEN PR の差分が lock を変えないか（base の側だけが lock を変えた古い枝を含む）、PR でない run のとき、THE SYSTEM SHALL 分岐点の lock を読まず、試験の zip を今のまま全部 PR の lock で組む。
WHEN 古い側を分岐点の lock で組んだ run では、THE SYSTEM SHALL 組んだ後、保存する前と、前の組み立てを使い回したときの両方で、古い側の zip が同梱する PostgreSQL の版が分岐点の lock の版で、新しい側の zip の版が PR の lock の版であることを確かめ、違えば CI を落とし、版の組を出力に出す。
THE SYSTEM SHALL この判定と確かめを Windows の runner の bash（bash・git・unzip・sha256sum）で動かし、lock の改行の違い（LF と CRLF）だけでは中身が違うと判定しない。
導出元: CH26（本人の決定 R2 = C）

#### Scenario: PR の差分が lock を変えたら、変えたと判定して分岐点を示す
- **WHEN** 確かめの台本が、分岐点の後に lock の版を `17.6-1` から `17.12-1` へ変えた commit を head とする使い捨ての git の木で、PR の差分が lock を変えたかを判定する
- **THEN** 変えたと判定し、分岐点の commit を示す

#### Scenario: base の側だけが lock を変えた古い枝は、変えたと判定しない
- **WHEN** 確かめの台本が、分岐点の後に base の側だけが lock を変え、head の側は lock を変えていない使い捨ての git の木で、PR の差分が lock を変えたかを判定する
- **THEN** 変えていないと判定する

#### Scenario: 中身が違い大きい版が同じ前の lock なら、古い側を前の lock で組むと判定する
- **WHEN** 確かめの台本が、版 `17.6-1` の前の lock と版 `17.12-1` の今の lock を比べる
- **THEN** rc=0 で、古い側を前の lock で組むと判定する

#### Scenario: 改行だけが違う 2 つの lock は、同じ中身と判定する
- **WHEN** 確かめの台本が、同じ lock の LF の写しと CRLF の写しを比べる
- **THEN** rc=0 で、同じ中身と判定する

#### Scenario: 前の lock と大きい版が違えば、古い側を組まずに落ちる
- **WHEN** 確かめの台本が、版 `16.4-1` の前の lock と版 `17.12-1` の今の lock を比べる
- **THEN** rc≠0 で、出力に `16` と `17` がある

#### Scenario: 組んだ zip の同梱の版を、許諾の文から読む
- **WHEN** 確かめの台本が、`THIRD-PARTY-NOTICES.txt` の PostgreSQL の節に `version: 17.6-1` を持つ使い捨てのサーバの zip から同梱の版を読む
- **THEN** rc=0 で `17.6-1` を出す

#### Scenario: 許諾の文に PostgreSQL の版が無い zip では、版を読めずに落ちる
- **WHEN** 確かめの台本が、`THIRD-PARTY-NOTICES.txt` に PostgreSQL の節が無い使い捨てのサーバの zip から同梱の版を読む
- **THEN** rc≠0 で終わる

#### Scenario: 古い側と新しい側の zip が前と今の lock の版なら、確かめは通って版の組を出す
- **WHEN** 確かめの台本が、版 `17.6-1` の前の lock・版 `17.12-1` の今の lock・同梱の版が `17.6-1` の古い側の zip・`17.12-1` の新しい側の zip で、試験の zip の版を確かめる
- **THEN** rc=0 で、出力に `17.6-1 → 17.12-1` がある

#### Scenario: 古い側の zip が前の lock の版でなければ、確かめは落ちる
- **WHEN** 確かめの台本が、上と同じ 2 つの lock と、同梱の版がどちらも `17.12-1` の古い側と新しい側の zip で、試験の zip の版を確かめる
- **THEN** rc≠0 で終わる

#### Scenario: CI の定義は、lock を変えた PR でだけ前の lock を取り、古い側だけを置き換えて組んで戻す
- **GIVEN** この repo の CI の定義
- **WHEN** 確かめの台本が、変更の判定の job と Windows の配布物の job の手順を読む
- **THEN** 前の lock を取る手順は、PR の差分が lock を変えたと判定された run でだけ走り、鍵を出す手順より前にあり、試験の zip を組む手順は古い側を組む間だけ lock を前の lock で置き換え、組んだ後に戻す

#### Scenario: CI の定義は、試験の zip の版を、保存の前と使い回した run の両方で確かめる
- **GIVEN** この repo の CI の定義
- **WHEN** 確かめの台本が、Windows の配布物の job の手順を読む
- **THEN** 試験の zip の版の確かめは、組み立ての手順の中（保存の前）と、保存の手順の後の、引けたかどうかに依らず走る手順の 2 か所にある

#### Scenario: CI の定義の形が 1 つでも崩れると、形の確かめは落ちる
- **WHEN** 確かめの台本が、上の 2 つの形のどれか 1 つを崩した CI の定義の写し（前の lock を取る手順を鍵の手順の後へ動かす・判定に依らず走らせる・古い側のほかも置き換える・置き換えを戻さない・保存の前の版の確かめを消す・使い回したときの版の確かめを引けなかったときだけにする・鍵に前の lock を渡さない）で形を確かめる
- **THEN** どの写しでも rc≠0 で終わる

## MODIFIED Requirements

### Requirement: PR の CI は、組み立ての入力が同じ Windows の配布物を使い回し、インストールの試験は全部走らせる

WHEN コードを触った PR が CI を走らせ、組み立ての鍵が同じ前の組み立ての Windows の配布物（Windows の配布物の job が上げる配布物の全部。今は配る形と試験の形の 2 つ）が CI のキャッシュに残っている THE SYSTEM SHALL Windows の配布物をビルドせずにそれを使い、インストールと更新の試験を走らせる。
THE SYSTEM SHALL Windows の配布物の job が上げる配布物の全部を、保存し、引く（一部だけを使い回さない）。
THE SYSTEM SHALL 組み立ての鍵を、次に名指しするものだけを除いた、追跡している全部のファイルの中身と、走っている CI の定義と、組み立てに使う道具の版（Rust・Node・runner の image）と、更新の試験の古い側を組んだ前の `pgsql.lock`（PR の差分が lock を変えた run だけ。変えていない run では「無い」という印）から作る。除くのは `tools/release/install-check.sh`・`tools/release/update-check-install.sh`・`tools/release/conpty-run.ps1`・`docs/` と `openspec/` の下・名前が `.md` で終わるファイルだけ。名指ししていないファイルが 1 つでも変われば、鍵は変わる。
THE SYSTEM SHALL 前の組み立ては鍵が完全に一致したときだけ使い、鍵の一部が一致するだけの組み立ては使わない。
WHEN Windows の配布物をビルドする THE SYSTEM SHALL Rust の依存のビルドを、Rust の版が同じ前のビルドから戻して使う。
WHEN 鍵を出せない THE SYSTEM SHALL 使い回さずにビルドする。
WHEN 幹（`main`）への push が CI を走らせる THE SYSTEM SHALL 使い回さずにビルドする。
THE SYSTEM SHALL 使い回した run でも、インストールと更新の試験の Scenario を全部走らせ、1 つでも走らなければ CI を落とす。

★ 2026-11-22 CH26 で、木の外から来る入力（更新の試験の古い側を組んだ前の lock）を鍵に足した（本人の決定 R2 = C。design D9（仮））。
導出元: CH17（受け皿の依頼 #308。本人の決定: 試験の台本だけが変わった push では、前に作った Windows の配布物を使い回してインストールの試験だけを走らせる。範囲は CI の Windows の配布物とインストールの試験の job だけ）。
深掘り第 1 回 Q1・Q2（本人の答え）で、Requirement「コードを触った PR ごとに、配る形の 3 部品を公開せずに組む」と「配布物は、本体の許諾・依存の許諾の一覧・対応するソースの場所を持つ」の本文を、使い回しを認め、使い回した配布物は組んだ commit を指すように改めた（下の MODIFIED。FR-127 ★ 2026-11-17）。

#### Scenario: 試験の台本と文書だけが違う 2 つの木は、同じ組み立ての鍵になる
- **GIVEN** `tools/release/install-check.sh`・`tools/release/update-check-install.sh`・`tools/release/conpty-run.ps1`・`docs/` の文書だけが違う 2 つの commit と、同じ道具の版
- **WHEN** それぞれの commit で組み立ての鍵を出す
- **THEN** 2 つの鍵は同じ

#### Scenario: 組み立てに入るファイルが 1 つでも違えば、鍵が変わる
- **GIVEN** `crates/server/src/lib.rs`・`migrations/ORDER`・`web/src/App.tsx`・`tools/release/build.sh`・`tools/release/pgsql.lock`・`tools/licenses/` の 1 ファイル・`LICENSE`・`Cargo.toml`・`collector-android/` の 1 ファイル・`Cargo.lock`・`.github/workflows/ci.yml` のどれか 1 つだけが違う 2 つの commit と、同じ道具の版
- **WHEN** それぞれの commit で組み立ての鍵を出す
- **THEN** どの組でも 2 つの鍵は違う

#### Scenario: 名指ししていないファイルが増えれば、鍵が変わる
- **GIVEN** 一方にだけ `tools/release/` の新しい台本が 1 つ足された 2 つの commit と、同じ道具の版
- **WHEN** それぞれの commit で組み立ての鍵を出す
- **THEN** 2 つの鍵は違う

#### Scenario: 道具の版が違えば、鍵が変わる
- **GIVEN** 同じ commit と、Rust の版の文字列だけが違う 2 つの道具の版
- **WHEN** それぞれで組み立ての鍵を出す
- **THEN** 2 つの鍵は違う

#### Scenario: 鍵を出せない木では、鍵を出さずに落ちる
- **GIVEN** git の木でない置き場（親の置き場の git の木も見ない）
- **WHEN** 組み立ての鍵を出す
- **THEN** 手順は 0 でない終了コードで終わり、標準出力に鍵を出さない

#### Scenario: 前の組み立ては、鍵が空でなく完全に一致した PR の run でだけ引く
- **GIVEN** この repo の CI の定義
- **WHEN** Windows の配布物の job の手順を読む
- **THEN** 前の組み立ての配布物を引く手順は、PR の run で鍵が空でないときだけ走り、鍵の完全な一致で引き、前方一致の引き方を持たない

#### Scenario: 引けたときだけ組み立てを飛ばし、今と同じ名前で上げる
- **GIVEN** この repo の CI の定義
- **WHEN** Windows の配布物の job の手順を読む
- **THEN** インストーラの試験と配布物の組み立ての手順は引けなかったときだけ走り、配布物を上げる手順は引けたかどうかに依らず今と同じ成果物の名前で走る

#### Scenario: 組み立てた配布物は、鍵が空でなく組み立てが成功したときだけ保存する
- **GIVEN** この repo の CI の定義
- **WHEN** Windows の配布物の job の手順を読む
- **THEN** 配布物を保存する手順は、引けず・鍵が空でなく・それまでの手順が成功したときだけ走り、引く手順と同じ鍵と置き場を持つ

#### Scenario: 上げる配布物は全部、保存と引きの置き場に入る
- **GIVEN** この repo の CI の定義
- **WHEN** Windows の配布物の job の手順を読む
- **THEN** その job が成果物として上げる置き場は 1 つ残らず、前の組み立てを引く手順と保存する手順の置き場に入っている

#### Scenario: 鍵を出す手順は、道具の版と走っている CI の定義を鍵に入れる
- **GIVEN** この repo の CI の定義
- **WHEN** Windows の配布物の job の鍵を出す手順を読む
- **THEN** その手順は、Rust の版・Node の版・runner の image の名前と版・走っている run の CI の定義を、鍵の台本に渡す

#### Scenario: 鍵を出す手順は、古い側を組んだ前の lock の指紋を鍵に入れる
- **GIVEN** この repo の CI の定義
- **WHEN** 確かめの台本が、Windows の配布物の job の鍵を出す手順を読む
- **THEN** その手順は、前の lock を取る手順が出した前の lock の指紋（取らなかった run では「無い」という印）を、道具の版と一緒に鍵の台本に渡す

#### Scenario: ビルドするときは、Rust の依存のビルドを戻す
- **GIVEN** この repo の CI の定義
- **WHEN** Windows の配布物の job の手順を読む
- **THEN** 引けなかったときだけ、Rust の依存の置き場と組み立ての置き場を、Rust の版と依存の固定の一覧の hash を持つ鍵で戻し、Rust の版が同じ前の鍵からも戻せる

#### Scenario: 使い回した run でも、インストールと更新の Scenario が 1 つ欠ければ CI が落ちる
- **GIVEN** この repo の CI の定義
- **WHEN** Windows のインストールの試験の job の手順を読む
- **THEN** その job は、配る形と試験の形の 2 つの配布物でインストールと更新の試験を走らせ、走った Scenario を台本の `--list` の全部と突き合わせ、欠けがあれば 0 でない終了コードで終わる手順を、使い回したかどうかで分けずに持つ
