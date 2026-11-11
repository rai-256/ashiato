# distribution の書いた時点の正典（spec_merge.py snapshot。archive の 3-way の base。手で直さない）

### Requirement: サーバのインストールは、管理者の確認を 1 回だけ出し、同梱の PostgreSQL とサーバを自動で起きるサービスにする

WHEN 利用者がサーバの Windows の配布物を展開してインストーラ（`ashiato-setup.exe`）を開く THE SYSTEM SHALL 管理者の確認（UAC）を 1 回だけ出し、それ以上の確認を出さずにインストールを終える。
THE SYSTEM SHALL プログラム（サーバ・インストーラ・同梱の PostgreSQL）をプログラムの置き場（`C:\Program Files\ashiato`）に、状態をすべて状態の置き場（`C:\ProgramData\ashiato`）に置く。
THE SYSTEM SHALL 同梱の PostgreSQL 17 を DB のサービス（`ashiato-db`）として、サーバをサーバのサービス（`ashiato-server`）として登録し、どちらの開始の種類も「自動」にする。
THE SYSTEM SHALL 2 つのサービスを、利用者でなく、合言葉を持たないサービス専用のアカウント（`NT SERVICE\ashiato-db`・`NT SERVICE\ashiato-server`）で動かし、利用者がログオンしていなくても動かす。
IF サービス専用のアカウントでサービスを登録できない THEN THE SYSTEM SHALL ほかのサービスと共有のアカウント（`LocalSystem`・`NetworkService`・`LocalService`）へ倒さず、登録できなかったことを示して止まる。
THE SYSTEM SHALL サーバのサービスを DB のサービスに依らせ（DB が起きてから起きる）、どちらも異常に終わったときは Windows のサービスの回復で起こし直す。
THE SYSTEM SHALL インストールで PC の収集アプリを入れず、Docker Desktop・WSL・ほかの実行環境を求めない。
THE SYSTEM SHALL インストールの間、ネットワークに出ない。

導出元: FR-108。深掘り Q1（推奨: サーバとデータベースだけ）・Q2（推奨: `ashiato-setup.exe` をサーバの zip に入れる）/ C2 / C11 / C12 / review/deep.md R8。
サービスの名前と置き場の並びは更新係（ST57）と更新（ST58）が乗る（design D14）。共有のアカウントへ倒すと、同じアカウントで動くほかのサービスが状態の置き場を読める（FR-109 が崩れる。design D3（仮））。

#### Scenario: 配る zip からのインストールで、2 つのサービスが自動で起きる形で入る

- **WHEN** サーバの Windows の配布物を展開してインストーラを管理者の権限で走らせ、最後に画面の合言葉を決める
- **THEN** サービス `ashiato-db` と `ashiato-server` が登録され、どちらも開始の種類が「自動」で実行中であり、`ashiato-server` は `ashiato-db` に依る

#### Scenario: インストールは Docker も WSL も使わない

- **WHEN** インストーラが起こした命令の一覧（試験の組み立ての記録）を、インストールの始めから終わりまで読む
- **THEN** 一覧に `docker`・`wsl`・`wsl.exe` の命令が無い

#### Scenario: 上がっていない状態から開くと、管理者に上がる起こし直しは 1 回だけ

- **WHEN** 管理者に上がっていない状態のインストーラが、インストールのために組む命令を読む
- **THEN** 自分を管理者として起こし直す命令はちょうど 1 つで、上がった後の段は起こし直しの命令を組まない

#### Scenario: サービスは利用者でなくサービス専用のアカウントで動く

- **WHEN** インストールした後に、2 つのサービスが動いているアカウントを読む
- **THEN** `ashiato-db` は `NT SERVICE\ashiato-db`、`ashiato-server` は `NT SERVICE\ashiato-server` で動き、インストールした利用者のアカウントでも `LocalSystem` でもない

#### Scenario: サービス専用のアカウントで登録できなければ、共有のアカウントへ倒さず止まる

- **WHEN** サービス専用のアカウントでの登録が断られる状態（試験の組み立ての差し込みで登録を失敗させる）でインストーラを走らせる
- **THEN** インストーラは rc≠0 で登録できなかったことを示して止まり、`LocalSystem`・`NetworkService`・`LocalService` で動くサービスは 1 つも登録されていない

#### Scenario: サービスを止めて起こし直すと、サーバは DB の後に起きて答える

- **WHEN** インストールした後に、2 つのサービスを止め、`ashiato-server` だけを開始する
- **THEN** `ashiato-db` が先に起き、`ashiato-server` が起きて `127.0.0.1:18700` の `/healthz` に 200 で答える

#### Scenario: サーバのサービスが異常に終わると起こし直される

- **WHEN** 動いている `ashiato-server` のプロセスを外から終わらせる
- **THEN** 1 分以内に `ashiato-server` がもう一度実行中になり、`/healthz` に 200 で答える

#### Scenario: DB のサービスが異常に終わると起こし直される

- **WHEN** 動いている `ashiato-db` の PostgreSQL の主のプロセスを外から終わらせる
- **THEN** 1 分以内に `ashiato-db` がもう一度実行中になり、`127.0.0.1:55400` が接続を受ける

#### Scenario: インストールの間、ネットワークに出ない

- **WHEN** 外への接続を拒む設定（試験の網の制限）でインストーラを走らせる
- **THEN** インストールは最後まで終わり、2 つのサービスが実行中である

### Requirement: インストールを走らせ直すと足りないものだけを足し、既にある DB・秘密・設定を上書きしない

WHEN インストーラをもう一度走らせる THE SYSTEM SHALL 既にある DB・秘密・設定・画面の合言葉を上書きせず、足りないもの（サービス・置き場・権限・プログラム）だけを足す。
THE SYSTEM SHALL 秘密を設定に書き終えてから DB を作る（途中で落ちても「秘密はあるのに DB が無い」側に倒れ、走らせ直しで DB を作れる）。
IF DB はあるのに秘密が無い THEN THE SYSTEM SHALL 何も書き換えず、DB と秘密が食い違っていることと状態の置き場の場所と、出口のインストーラの `repair` の口を示して止まる。
WHEN 前のインストールが画面の合言葉を決める前に終わっていた THE SYSTEM SHALL 走らせ直しで、画面の合言葉を決める段からやり直す。

導出元: FR-108・FR-110・FR-111。C10 / review/deep.md R5。
上書きすると DB に繋げなくなり、収集アプリの資格情報も切れる。DB があるのに秘密が無いまま止まっている間、携帯の未送信は上限（90 日・2 GB。FR-9）に向かって溜まるので、記録を残したまま戻る出口を `repair` の口に置く（design D10・D27。深掘り Q8）。

#### Scenario: 走らせ直しても秘密と DB は変わらない

- **WHEN** インストールした後に、設定のファイルの指紋と記録の件数を控え、インストーラをもう一度走らせる
- **THEN** 設定のファイルの指紋と記録の件数は控えと同じで、2 つのサービスは実行中である

#### Scenario: DB を作る途中で落ちても、走らせ直しで DB ができる

- **WHEN** DB を作る段で止まる差し込み（試験の組み立ての差し込み）でインストーラを走らせ、差し込みを外してもう一度走らせる
- **THEN** 2 回目で DB ができ、1 回目に作った秘密でサーバが起きる

#### Scenario: DB はあるのに秘密が無いと、何も書き換えずに止まる

- **WHEN** インストールした後に設定のファイルを退け、インストーラをもう一度走らせる
- **THEN** インストーラは rc≠0 で DB と秘密の食い違いと状態の置き場の場所と `repair` の口を示して止まり、DB の置き場の中身と記録の件数は変わらない

#### Scenario: 合言葉を決める前に閉じたら、走らせ直しで合言葉の段からやり直す

- **WHEN** 画面の合言葉を決める段でインストーラを閉じ、もう一度走らせる
- **THEN** 2 回目は DB と秘密を作り直さずに、画面の合言葉を決める段を出す

### Requirement: サーバは Windows の「アプリと機能」から消せ、消しても記録の置き場は残る

WHEN インストールが終わる THE SYSTEM SHALL Windows の「アプリと機能」にサーバを、版番号と一緒に載せる。
WHEN 利用者が「アプリと機能」かインストーラの `uninstall` の口からサーバを消す THE SYSTEM SHALL 管理者の確認を 1 回出し、2 つのサービスを止めて登録から消し、プログラムの置き場と「アプリと機能」の項目と、ダウンロードと取り込み待ちに足した権限を消す。
THE SYSTEM SHALL サーバを消しても、状態の置き場（記録・原本・退避・書き出し・設定）と取り込み待ちの中身を消さず、その場所と、手で消すときの手順の場所（`docs/install.md`）を示す。
WHEN 消した後にもう一度インストールする THE SYSTEM SHALL 残った状態の置き場の DB と秘密の上で、走らせ直しと同じに続ける。

導出元: FR-108・FR-109。深掘り Q4（推奨: 作る。記録の置き場は残し、場所を示す）。
記録を消す口は作らない —— 状態の置き場は本人も書けない（FR-109）ので、消すには管理者として所有権を取ってから消す（review/deep.md R10）。

#### Scenario: インストールすると「アプリと機能」に載る

- **WHEN** インストールした後に、Windows のアンインストールの登録簿（`HKLM\Software\Microsoft\Windows\CurrentVersion\Uninstall\ashiato`）を読む
- **THEN** 表示名と版番号と、インストーラの `uninstall` の口を指す消す命令がある

#### Scenario: 消すとサービスとプログラムと登録が消える

- **WHEN** インストールした後に、インストーラの `uninstall` の口で消す
- **THEN** サービス `ashiato-db` と `ashiato-server`、プログラムの置き場、アンインストールの登録簿の項目は無い

#### Scenario: 消しても記録の置き場は残り、その場所を示す

- **WHEN** インストールして記録を 1 件入れた後に、インストーラの `uninstall` の口で消す
- **THEN** 状態の置き場の DB と設定と取り込み待ちの中身は残り、出力に状態の置き場の場所と手で消す手順の場所がある

#### Scenario: 消した後に入れ直すと、前の記録の上で続く

- **WHEN** 記録を 1 件入れてから消し、同じ版でもう一度インストールする
- **THEN** 入れ直しは画面の合言葉を聞かずに終わり、前の合言葉でログインでき、前の記録が読み出しに出る

#### Scenario: 消すとダウンロードと取り込み待ちに足した権限が外れる

- **WHEN** インストールした後に消し、本人の「ダウンロード」と取り込み待ちの権限を読む
- **THEN** どちらにもサーバのサービスのアカウントの項目が無い

### Requirement: サーバの Windows の配布物は、インストーラと、版と指紋を固定した PostgreSQL の要るものだけを持つ

THE SYSTEM SHALL サーバの Windows の配布物（`ashiato-server-<版番号>-windows-x86_64.zip`）に、サーバ・インストーラ（`ashiato-setup.exe`）・同梱の PostgreSQL 17 を入れ、インストーラとサーバの組み立ての印の版番号をそろえる。
THE SYSTEM SHALL 同梱の PostgreSQL を、版と内容の指紋（SHA-256）を固定した公式の Windows 向けの zip から取り、指紋が違えば配布物を組まない。
THE SYSTEM SHALL 同梱の PostgreSQL から、動かすのに要るもの（サーバ・`initdb`・`pg_ctl`・`psql`・`pg_dump` と、その DLL・`lib`・`share`）と許諾の文だけを入れ、管理画面（pgAdmin）・StackBuilder・開発用の見出しと文書を入れない。
THE SYSTEM SHALL Linux の配布物の中身を変えない。
THE SYSTEM SHALL 試験のための差し込みと、起こした命令の記録を、試験の組み立てでだけ読み、配る組み立てのインストーラとサーバには持たせない。

導出元: FR-108（同梱するのは要るものだけ・版が配布物で固定される）・FR-132。C12 / C13。premises P5（EDB の zip は展開で 889 MB、うち 706 MB が pgAdmin）。
許諾の一覧（FR-127）への統合は ST53（`docs/handoff/ST53.md`）。

#### Scenario: Windows の配布物にインストーラと PostgreSQL が入り、管理画面は入らない

- **WHEN** 配る形でサーバの Windows の配布物を組み、zip の目録を読む
- **THEN** `ashiato-server.exe`・`ashiato-setup.exe`・`pgsql\bin\postgres.exe`・`pgsql\bin\initdb.exe`・`pgsql\bin\pg_ctl.exe`・`pgsql\bin\psql.exe`・`pgsql\bin\pg_dump.exe` と PostgreSQL の許諾の文があり、`pgAdmin 4`・`StackBuilder`・`include`・`doc` の下のものと PC の収集アプリ（`ashiato-collector`）は無い

#### Scenario: インストーラとサーバの版番号はそろう

- **WHEN** 版番号 `0.4.0-beta.1` を渡して Windows の配布物を組み、`ashiato-setup.exe` と `ashiato-server.exe` の組み立ての印を読む
- **THEN** どちらの印も `0.4.0-beta.1+` で始まる

#### Scenario: 同梱する PostgreSQL の指紋が違えば組まない

- **WHEN** 固定した指紋と違う中身の PostgreSQL の zip（試験の組み立ての差し込み）で Windows の配布物を組む
- **THEN** 組み立ては rc≠0 で指紋が違うことを示して落ち、配布物はできない

#### Scenario: 配る組み立ては試験の差し込みを持たない

- **WHEN** 配る形でサーバの Windows の配布物を組み、`ashiato-setup.exe` と `ashiato-server.exe` の中の文字列を読む
- **THEN** 試験の差し込みの名前（`ASHIATO_SETUP_TEST_`・`ASHIATO_SERVICE_TEST_`）が無い

#### Scenario: Linux の配布物は PostgreSQL を持たない

- **WHEN** 配る形でサーバの Linux の配布物を組み、目録を読む
- **THEN** `pgsql` も `ashiato-setup` も無い

### Requirement: 配布物は、本体の許諾・依存の許諾の一覧・対応するソースの場所を持つ

THE SYSTEM SHALL サーバの Linux 版・Windows 版、PC の収集アプリ、携帯の APK の 4 つの配布物に、次の 3 つのファイルを入れる。tar / zip では根に、APK では `assets/licenses/` に置く。
- `LICENSE`: 木の `LICENSE` と同じ中身（追加の許可を含む）
- `THIRD-PARTY-NOTICES.txt`: その配布物に入る依存の一覧。部品ごとに `== <名前> <版>` の見出し・`License: <式>` の行・許諾の全文と著作権の表示を書く
- `SOURCE.txt`: 対応するソースの場所
THE SYSTEM SHALL `THIRD-PARTY-NOTICES.txt` の依存に、次のものを入れる。
- サーバ: 実行ファイルに入る Rust の部品、組み込んだ画面に入る Node の部品（開発だけの部品を除く）、Rust の標準ライブラリ（`rust-std`）
- PC の収集アプリ: 実行ファイルに入る Rust の部品と、Rust の標準ライブラリ（`rust-std`）
- 携帯の APK: `releaseRuntimeClasspath` の部品と、名指しの Google のライブラリが中に持つ第三者の一覧（各 aar の `third_party_licenses.txt` の中身。見出し `(gms の中の第三者)`）
IF ある部品の許諾の全文も著作権の表示も見つからない THEN THE SYSTEM SHALL 依存の一覧を組まずに、組み立てを落とす。
THE SYSTEM SHALL `SOURCE.txt` に、リポジトリの場所を書く。正式版・ベータ版の公開の組み立てでは、それに加えて印 `v<版番号>` を書く。それ以外の組み立てでは、組み立てた木の 40 桁の commit を書く。開発版と main への push の組み立てでは main の commit、PR の組み立てでは PR の head の commit になる。
THE SYSTEM SHALL PR の組み立てを、PR の head の commit そのものから組む（GitHub が作る、head とその時の main を合わせた merge の commit からは組まない）。`SOURCE.txt` の commit・組み立ての印・組んだソースが同じ commit を指す。
THE SYSTEM SHALL 配布物の検査で、期待する版番号と、配布物の `LICENSE` と比べる `LICENSE` を、配布物を組んだのと同じ commit の中身から取る。PR の CI の配布物を検査する job は、ほかの job と同じく head とその時の main を合わせた木で走り、期待値だけを PR の head の commit（main への push では push した commit）から取る。
IF 公開の組み立てに渡された印が `v<版番号>` と違う THEN THE SYSTEM SHALL 配布物を作らずに組み立てを落とす。
THE SYSTEM SHALL 3 つのファイルを組み立ての台本の中で入れる。正式版・ベータ版だけでなく、開発版・main への push・PR の組み立てにも同じ手順で入れる。
IF 配布物のどれかに 3 つのファイルのどれかが無い、`LICENSE` が期待する `LICENSE`（配布物を組んだ commit の `LICENSE`）と違う、サーバと PC の収集アプリの一覧に `rust-std` の項が無い、または携帯の APK の一覧に Google のライブラリの第三者の一覧が無い THEN THE SYSTEM SHALL 配布物の検査を落とし、その配布物を公開しない。
THE SYSTEM SHALL 配布物の検査で、`SOURCE.txt` を次の決まりで見る。合わなければ検査を落とす。
- 正式版・ベータ版として公開する配布物の検査（debug の鍵を許さない、チャンネルが `release` か `beta` の検査）: `v<期待する版番号>`
- 開発版の検査（チャンネルが `nightly` の検査）と、debug の鍵を許す検査（PR・main への push の組み立てと下書きの配布物の検査）: 組み立ての印と同じ 40 桁の commit

導出元: FR-127（本体の許諾と依存の許諾の一覧を同梱する。対応するソースは同じ印のリポジトリで示す）。深掘り Q6・Q7（第 1 回の推奨）。review/deep.md R5・R6・R7・R10 の C。review/spec.md R1・R2・R5・R11・R16。D8・D9・D16・D18。深掘り Q13・Q14・Q15（第 2 回）・Q16（第 3 回）・Q17（第 4 回）。FR-127 ★ 2026-11-02・★ 2026-11-03・★ 2026-11-03（第 3 回）。
動く印 `nightly` は翌日に別の commit を指すので、開発版の写しには動かない commit を書く（Q7）。配った写しに書いた場所は、後から直せない。

#### Scenario: 4 つの配布物は、許諾・依存の一覧・ソースの場所を持つ
- **WHEN** 配る形の 4 つの配布物の置き場に、配布物の検査を当てる
- **THEN** 4 つのどれにも `LICENSE`・`THIRD-PARTY-NOTICES.txt`・`SOURCE.txt` があり（携帯の APK は `assets/licenses/` の下）、検査の終了コードは 0

#### Scenario: 配布物の LICENSE が木の LICENSE と違えば、公開しない
- **GIVEN** サーバの Linux 版の tar の `LICENSE` を、AGPL の全文だけのものに差し替えた配布物の置き場
- **WHEN** 配布物の検査を当てる
- **THEN** 検査は 0 でない終了コードで終わり、`LICENSE` が違う配布物の名前が示される

#### Scenario: 許諾のファイルが欠けた配布物は、公開しない
- **GIVEN** サーバの Linux 版の tar から `THIRD-PARTY-NOTICES.txt` を抜いた配布物の置き場
- **WHEN** 配布物の検査を当てる
- **THEN** 検査は 0 でない終了コードで終わり、欠けたファイルと配布物の名前が示される

#### Scenario: Rust の実行ファイルの依存の一覧は、標準ライブラリを含む
- **WHEN** サーバと PC の収集アプリの配布物の `THIRD-PARTY-NOTICES.txt` を読む
- **THEN** どちらにも `== rust-std ` の見出しの項（`License: MIT OR Apache-2.0`）がある

#### Scenario: サーバの依存の一覧は、画面に入る Node の部品を含み、開発だけの部品を含まない
- **WHEN** サーバの配布物の `THIRD-PARTY-NOTICES.txt` を読む
- **THEN** `react` の項があり、`vitest` の項は無い

#### Scenario: 許諾の全文も著作権の表示も見つからない部品があれば、組み立てが落ちる
- **GIVEN** 許諾のファイルも作者の欄も持たず、許諾の式の全文の写しも無い部品を 1 つ持つ、偽の依存の一覧
- **WHEN** 依存の一覧を組む
- **THEN** 組む手順は 0 でない終了コードで終わり、その部品の名前が示され、依存の一覧のファイルはできない

#### Scenario: 携帯の APK の依存の一覧は、Google のライブラリの中の第三者を含む
- **WHEN** 携帯の APK の `assets/licenses/THIRD-PARTY-NOTICES.txt` を読む
- **THEN** `play-services-location` の aar の `third_party_licenses.txt` から取り出した、`(gms の中の第三者)` の見出しの項がある

#### Scenario: 正式版の配布物のソースの場所は、その版の印
- **GIVEN** 印 `v0.4.0` の公開の組み立て
- **WHEN** 配布物の `SOURCE.txt` を読む
- **THEN** `SOURCE.txt` はリポジトリの場所と印 `v0.4.0` を持つ

#### Scenario: 印が版番号と違う公開の組み立ては、配布物を作らずに落ちる
- **GIVEN** 版番号 `0.4.0` で、印として `v0.4.1` を渡した組み立て
- **WHEN** `SOURCE.txt` を組む
- **THEN** 手順は 0 でない終了コードで終わり、`SOURCE.txt` はできない

#### Scenario: 開発版と main への push の配布物のソースの場所は、組み立てた commit
- **GIVEN** main の commit を checkout した木で、印を渡さない組み立て（開発版か main への push の組み立て）
- **WHEN** 配布物の `SOURCE.txt` を読む
- **THEN** `SOURCE.txt` はリポジトリの場所と、組み立ての印と同じ 40 桁の commit を持ち、`nightly` の文字を持たない

#### Scenario: PR の組み立ての配布物のソースの場所は、PR の head の commit
- **GIVEN** main と合わせていない枝の commit（PR の head）を checkout した木で、印を渡さない組み立て
- **WHEN** 配布物の `SOURCE.txt` と組み立ての印を読む
- **THEN** `SOURCE.txt` はリポジトリの場所と、その head の 40 桁の commit を持ち、組み立ての印も同じ commit を持つ

#### Scenario: 組み立ての commit が 40 桁の 16 進でなければ、配布物を作らずに落ちる
- **GIVEN** 印を渡さず、組み立ての印の commit の上書き（`ASHIATO_BUILD_COMMIT`）に `main` を渡した組み立て
- **WHEN** `SOURCE.txt` を組む
- **THEN** 手順は 0 でない終了コードで終わり、`SOURCE.txt` はできない

#### Scenario: PR の配布物は、PR の head の commit から組まれ、同じ commit の期待値で検査される
- **WHEN** CI の配る形を組む job（サーバの Linux 版・携帯の APK と、Windows の 2 つ）と、その配布物を検査する job の checkout と検査の呼び出しを、workflow の検査で読む
- **THEN** 組む 2 つの job は、PR では PR の head の commit を、main への push では push した commit を checkout する
- **AND** 検査する job の作業の checkout は `ref` を持たない（合わせた木）
- **AND** 検査する job は、期待する版番号と比べる `LICENSE` を、組む 2 つの job と同じ commit を別の置き場に checkout したものから検査に渡す

#### Scenario: 配布物の LICENSE は、渡した期待する LICENSE と比べる
- **GIVEN** 台本の木の `LICENSE` と違う `LICENSE` のファイル（配布物を組んだ commit の `LICENSE` に当たる）と、その `LICENSE` を持つ配布物の置き場
- **WHEN** そのファイルを期待する `LICENSE` として渡して配布物の検査を当て、次に渡さずに当てる
- **THEN** 渡したときは検査の終了コードは 0、渡さないときは 0 でない終了コードで終わり、`LICENSE` が違うことが示される

#### Scenario: PR の組み立てと下書きの配布物の検査は、head の commit のソースの場所で通る
- **GIVEN** `SOURCE.txt` が組み立ての印と同じ 40 桁の commit（PR の head）を持つ配布物の置き場
- **WHEN** debug の鍵を許す配布物の検査（PR の組み立てと下書きの検査）を当てる
- **THEN** 検査の終了コードは 0

#### Scenario: debug の鍵を許す検査でも、組み立ての印の commit でないソースの場所の配布物は落ちる
- **GIVEN** `SOURCE.txt` に、commit でなく印 `v0.4.0` を持つ配布物の置き場と、`Commit: main` を持つ配布物の置き場と、7 桁の短い commit を持つ配布物の置き場と、組み立ての印と違う 40 桁の commit を持つ配布物の置き場
- **WHEN** それぞれに debug の鍵を許す配布物の検査を当てる
- **THEN** 4 つとも検査は 0 でない終了コードで終わり、`SOURCE.txt` が決まりと合わないことが示される

#### Scenario: ソースの場所がチャンネルの決まりと合わない配布物は、公開しない
- **GIVEN** `SOURCE.txt` に印でなく commit を持つ配布物の置き場
- **WHEN** `--channel beta --expect-version 0.4.0-beta.1` で配布物の検査を当てる
- **THEN** 検査は 0 でない終了コードで終わり、`SOURCE.txt` が決まりと合わないことが示される
