# 整合の確認 — サーバの DB を同梱の PostgreSQL にし、Linux でも配る改訂（2026-11-03）

- 基準: `615fc4e7a26490677a050a91a9fb03b6e9dd7c1a`..`42a4dd67d60afe46faea574ebeebc11fb20c9360`

## 機械の入力（写し）

`python3 scripts/req_impact.py . --base 615fc4e7a26490677a050a91a9fb03b6e9dd7c1a`:

- FR-108（改訂）: `docs/production-prep.md:48`・`docs/stories/INDEX.md:78`・`docs/stories/stories.json:1305`
- FR-109（改訂）: `docs/stories/INDEX.md:78`・`docs/stories/stories.json:1306`
- FR-131（新規）・FR-132（新規）: 要件の外で名指ししている箇所は無い

`python3 scripts/check_chain.py .`（chain: FAIL 3 件 / 未回収 2 件 / warn 0 件）:

- [FAIL] どの Story にも拾われていない要件 2 件: FR-131, FR-132
- [FAIL] ST55: 判断の日（2026-10-25）より新しい訂正が満たす要件にある（FR-108, FR-109。最新 ★ 2026-11-03）
- [FAIL] 整合の確認: 整合の確認の後に要件が変わった: FR-108, FR-109, FR-131, FR-132（この監査がそれに当たる）
- [FAIL] ST55: stories.json + requirements.md から再生成した結果と一致しない

## 1. 改訂後の要件と、ほかの文書

## C1. ST55 の価値・完了の判定・壊しうるものが、消えた前提（Docker・実行環境の導入・ログオン時の起動・実行環境の外のフォルダ）を固定している
- 重大度: 高
- 文書: docs/stories/stories.json:1315
- 要件: FR-108, FR-109
- 根拠: stories.json:1315「Windows でサーバのインストールを走らせるだけで、DB の実行環境が入り」/ :1322「Docker の無い Windows でサーバのインストールを走らせると、実行環境が入り、ログオン時に自動で起動する（仮想化が無効なら手順が示されて止まる）」/ :1323「すべて Windows のフォルダにあり、実行環境を消して入れ直してもデータが残る」/ :1317「いまの DB のコンテナの構成（`docker-compose.yml`）と、worktree ごとの DB」。改訂後の requirements.md:980「配布物に同梱した PostgreSQL（D-01）と S-01 を OS のサービスとして登録し、PC の起動時から動かす」/ :995「サービスのアカウントだけが書ける PC 全体の置き場（Windows では `ProgramData` の下）」。docs/stories/ST55.md:18-30・:45・:55 も旧い本文の引用のまま
- 直すところ: requirements の後の /stories（ST55 の価値・done・breaks を書き直し、judged を 2026-11-03 に。ST55.md は make_story.py で貼り直す）
- 機械: 捕まえられた（check_chain.py の「判断の日より新しい訂正」と「再生成が一致しない」）
- 行き先: stories（`stories.json` の ST55 の価値・完了の判定・壊しうるものを書き直し、judged を 2026-11-03 にして再生成した）

## C2. 走っている ST55 の上流（ashiato2-up-st55、答え待ち）の深掘りの問いと聞かない既定が、Docker Desktop とログオン時の起動を前提にしている（印のみ）
- 重大度: 高
- 文書: /home/yosis/dev/ashiato2-up-st55/openspec/changes/st55-server-installer/deep-questions.json（Q1・Q2）・premises.md（P3〜P5・P11・P12、聞かない既定 C3・C4・C6・C7・C8・C12・C13・C15）
- 要件: FR-108, FR-109
- 根拠: deep-questions.json Q1「Docker Desktop で、データベースの中身を Windows のフォルダに置けるか（実物）」、Q2 の選択肢はどれも Docker か専用の WSL。premises.md C13「昇格（管理者の権限）は Docker Desktop と WSL を入れる子のプロセスだけ。データのフォルダ・設定・ログオン時の起動は、昇格しない本人の側で作る」、C12「継承を切って本人と SYSTEM だけが読み書きできるようにする。`%LOCALAPPDATA%` の中でも同じにする」、C15「ログオン時に起こす係（`ashiato-setup` の起動の口）が … DB のコンテナとサーバを起こす」。改訂後の requirements.md:985「データベースもサーバも Windows のサービスにする」・:986「インストールのときに管理者の権限を 1 回求める」
- 失われるもの: rewrite-all（第 1 回の問いを立て直すことになる。答えはまだ無いので、本人の答えは失われない。サービスに関わらない前提 P1・P2・P6〜P8・P10・P13・P14・P16 は使い回せる）
- 直すところ: その文書（走っている change の印だけを付ける。差し戻しは書かない）
- 機械: できない（worktree の中の未 merge の change は、main の check_chain.py の対象の外）
- 行き先: leave: ST55 の上流は答えずに止めてある。/stories の後に深掘りからやり直す（失うのは問いだけで、本人の答えはまだ無い）。使い回せる下調べは `docs/handoff/ST55.md` に書いた

## C3. 本人の環境を配った版へ移す手順（ST53 の確認の後）が、移さないと決めた Docker の DB の上に本人の記録を積む
- 重大度: 高
- 文書: docs/release.md:78
- 要件: FR-108, FR-109
- 根拠: release.md:80「ST53 の確認後、正式版の公開が完了してから本人が行う。まず現在のサーバを止め、DB のバックアップを取り、新しい正式版のサーバを配置する。新しい版で `ashiato-server migrate` を実行し」（いまの DB = Docker の DB）。stories.json:1252（ST52 の done）「本人がサーバをその版へ移す手順が `docs/release.md` にある（切り替えるのは ST53 の確認の後…）」。ST55 は ST53 と同じ layer 5 で、ST53 の後に来るとは限らない。改訂後の requirements.md:994「いまの Docker の DB の記録は配る形へ移さない（Q5 … 本人の答え: 移したい記録は無い）。Docker の DB は開発と試験のためだけに残る」。Q5（`req-native-postgres-r1.html`）の推奨は「切り替えた後、Docker のデータベースを片付けると、そこにしか無い記録は戻りません」と書く。ST53 の後・ST55 の前に §7 で切り替えると、配った版の本人の記録が Docker の DB に溜まり、ST55 の新しい DB へ移す口がどこにも無い
- 直すところ: requirements（本人の環境を切り替える日を ST55 の後にするか、Docker の DB から移す口を ST55 に入れるか）→ docs/release.md §7 と、凍結した ST52 への申し送り（done の「移す手順」）
- 機械: できない
- 行き先: requirements（FR-108 の ★ 2026-11-03 補足に「本人の環境の切り替えは ST55 の後、インストーラで入れて」。r3 Q4）と docs（`docs/release.md` §7 を同じに直した）

## C4. FR-109 は「書ける」だけを絞り、読む側を絞らない。ProgramData の下は既定で同じ PC のほかのアカウントから読める
- 重大度: 高
- 文書: docs/requirements.md:995
- 要件: FR-109, FR-110
- 根拠: requirements.md:995「S-01 と D-01 のサービスのアカウントだけが書ける PC 全体の置き場（Windows では `ProgramData` の下）に置く」。置くものは「D-01・D-02・移行の前の退避・拠点内のバックアップ・書き出し・DB の合言葉・設定」。改訂前の置き場（`%LOCALAPPDATA%`）は本人のプロファイルの中で、ST55 の聞かない既定 C12 は「本人と SYSTEM だけが読み書きできるようにする」、正典 data-durability は Unix で 0700 / 0600（spec.md:28・:121-124）と、読む側も絞っていた。ProgramData は継承の既定で `Users` に読みを許す（未確認。下の「未確認」）。FR-110「DB の合言葉を … その PC の中にだけ置く」も、同じ PC のどのアカウントからでも読める形を許してしまう
- 直すところ: requirements（FR-109 に「読めるのもサービスのアカウントと（書き出し・原本を開く）本人だけ」を足す）
- 機械: 安く捕まえられる（ST55 の試験で、作った置き場の ACL に `Users` / `Authenticated Users` の読みが無いことを見る）
- 行き先: requirements（FR-109 の本文に「それとインストールした本人だけが読める」を足した）。ACL の試験は `docs/handoff/ST55.md` の K2

## C5. 書庫の取り込み待ち（本人のダウンロード・ドキュメントのフォルダ）を、サービスのアカウントで動く S-01 は既定では見に行けない
- 重大度: 高
- 文書: openspec/specs/external-ingestion/spec.md:12
- 要件: FR-14, FR-17, FR-108, FR-109
- 根拠: external-ingestion/spec.md:12「取り込み待ち置き場を、**専用のフォルダ**と、**ブラウザのダウンロードのフォルダのうち名前が `takeout-` で始まり `.zip` で終わるファイル**の 2 つとする」、:27「ダウンロードのフォルダを見るのは、Takeout を**ダウンロードするだけで入る**ようにするため（本人の決定）」。取り込み器は S-01 の中で起きる（crates/server/src/lib.rs:4430 `archive::config::from_env()`）。既定は `USERPROFILE` から作る（crates/server/src/archive/config.rs:36-64: `Documents\ashiato\取り込み待ち`・`Downloads`）。サービスのアカウントの `USERPROFILE` は本人のものでないので、本人が落とした Takeout は既定のままでは読まれない。本人のフォルダを明示しても、FR-109 の「サービスのアカウントだけ」の形では本人のプロファイルを読む権限をどう渡すかが要件に無い。専用のフォルダの読み終えた書庫を `取り込み済み` へ動かす書き込みも要る（docs/archive-inbox.md:47）
- 直すところ: requirements（FR-108 / FR-109 に取り込み待ち置き場の扱いを足す。本人のフォルダを読む権限、または置き場を PC 全体の側へ移すか）。ST55 の深掘りで決めるなら ST55 の done に入れる
- 機械: できない（本物の Windows のサービスのアカウントで走らせる試験が要る）
- 行き先: requirements（FR-108 の ★ 2026-11-03 補足。r3 Q1: インストーラが本人のダウンロードと取り込み待ちのフォルダにサービスのアカウントの読みを足す）。実装は handoff ST55

## C6. 正典の既定の置き場（書き出し・原本）が「サーバを動かしている利用者の %LOCALAPPDATA%」のままで、FR-109 の ProgramData と食い違う
- 重大度: 中
- 文書: openspec/specs/data-durability/spec.md:26
- 要件: FR-109
- 根拠: data-durability/spec.md:26「置き場を、サーバを動かしている利用者のローカルのアプリのデータのフォルダの下に置き」・:114「（Windows なら `%LOCALAPPDATA%`）の下の `ashiato\export`」。device-collection/spec.md:1867「設定が無ければ `%LOCALAPPDATA%\ashiato\media`」・:1896。どちらもテストの印がある（crates/server/src/export_tests.rs:73・media_tests.rs:211）。サービスのアカウントで動くと「利用者」はサービスのアカウントになり、既定はそのプロファイルの下（ProgramData ではない）に落ちる。requirements.md:995 は「PC 全体の置き場（Windows では `ProgramData` の下）」
- 直すところ: ST55 の spec（正典の 2 つの Requirement を MODIFIED で ProgramData の既定へ。export_tests.rs / media_tests.rs の印つきの試験も一緒に）
- 機械: 安く捕まえられる（ST55 の spec の段で、MODIFIED の無い正典の Scenario と FR-109 の語の突き合わせ。いまは無い）
- 行き先: handoff ST55（`docs/handoff/ST55.md` に正典の 2 つの Requirement と印つきの試験を書いた）

## C7. 製造準備の「プラットフォーム既定値の棚卸し」（DB のログ・待ち受け）と、DB の合言葉の前提が、配る D-01 に運ばれない
- 重大度: 高
- 文書: docs/production-prep.md:235
- 要件: FR-108, FR-110
- 根拠: production-prep.md:235「PostgreSQL を `log_statement=none` / `log_min_duration_statement=-1` / `log_error_verbosity=terse` で起動（**私的データが SQL ごとログに落ちるのを止める**）。**DB の待ち受けを `127.0.0.1` に固定**」。実装は docker-compose.yml:9（`127.0.0.1:${ASHIATO_DB_PORT:-55432}:5432`）と :11-15（`command:` の 3 つ）だけで、改訂後の FR-108 の配る D-01 はこのファイルを通らない。requirements.md:980-994 も ST55 の done（stories.json:1322-1324）も、この 3 つの設定と待ち受けを配る D-01 へ持っていくと書いていない。さらに、zip の `initdb` は `-A` を渡さなければ手元の接続を合言葉なし（trust）で通すので、FR-110 の DB の合言葉が効かない（未確認。下の「未確認」）。開発の `tools/db-roles.sh:7` も「コンテナの中の psql で入るので合言葉は要らない」に依っている
- 直すところ: requirements（FR-108 に「配る D-01 も製造準備の既定値（ログの 3 つ・loopback の待ち受け・合言葉の認証）で作る」を足す）と production-prep.md:235 の適用先の列
- 機械: 安く捕まえられる（FR-108 の「配る形で移行を当ててサーバを起こす Windows の CI の試験」で、`SHOW log_statement` などと `pg_hba.conf` に trust が無いことを見る）
- 行き先: requirements（FR-108 の ★ 2026-11-03 補足に「配る D-01 も製造準備の既定値で作る」）。試験は handoff ST55

## C8. 「管理者の権限を 1 回」と、更新・自動の戻しがサービスを止めて起こし直すことが合わない
- 重大度: 中
- 文書: docs/requirements.md:986
- 要件: FR-108, FR-112, FR-114, FR-119
- 根拠: requirements.md:986「インストールのときに管理者の権限を 1 回求める」。FR-114（:1013）「S-01 の更新の手順を開く … **入れるのは本人**」、FR-119（:1027）「移行の前の退避から D-01 を戻し、前の版の S-01 で起動し直す」、docs/handoff/ST58.md「**更新の手順は、新しい版の migrate の前に古い S-01 を止める**」。サービスの停止・起動と Program Files の実行ファイルの入れ替えは、既定では管理者の権限が要る。FR-112（:1006）の更新係（S-01 と別のプロセス）がどのアカウントで動き、どこへ結果を書くか（FR-109 の「S-01 と D-01 のサービスのアカウントだけが書ける」置き場に、更新係は書けない）も決まっていない。ST57（stories.json の done「サーバの更新係・携帯・PC が 1 日 1 回」）と ST58（「サーバの更新で移行が失敗すると、退避から DB が戻り、前の版で起動し直して」）は、この食い違いを前提にしたまま
- 直すところ: requirements（FR-108 / FR-112 / FR-114 に更新のときの昇格と、更新係のアカウント・書く場所を足す。または「1 回」を「インストールと更新のたびに」へ）
- 機械: できない
- 行き先: requirements（FR-108 の補足に更新のたびの UAC（r3 Q2）、FR-112 の ★ 2026-11-03 に更新係のアカウントと置き場）

## C9. 合言葉を変えて締め出す手順が `.env` を書き換える形のままで、サービスの形では本人が書けない
- 重大度: 中
- 文書: docs/network.md:40
- 要件: FR-109, FR-111, FR-128
- 根拠: network.md:40「`.env` の `API_TOKEN` を変えて、APK と C-02 を入れ直す」・:41「`.env` の `WEB_PASSWORD`（画面の合言葉）を変えて、サーバを起動し直す」。docs/screens.md:15「合言葉（`.env` の `WEB_PASSWORD`）」。FR-128 の ★ 2026-11-02（requirements.md:1087）「落とした端末を締め出すには、値を変えて全部の収集アプリに設定し直す」。改訂後の FR-109 は設定と合言葉を「サービスのアカウントだけが書ける」置き場に置くので、本人が手で書き換える経路が無い。FR-111 はインストールの最後に決める口だけで、後から変える口はどの要件・Story にも無い
- 直すところ: requirements（画面の合言葉と収集アプリの共有の値を後から変える口を FR-111 か FR-128 に足す）→ network.md §4
- 機械: できない
- 行き先: requirements（FR-133 を足した。r3 Q3: 画面のログインの後の設定から変える。`docs/network.md` §4 の `.env` の手順は、FR-133 の Story が入るまでいまの開発の形として正しいので残す）

## C10. NFR-25 の 1.0 へ上げる合図「S-01 の配り方の選び直し」が、この改訂で済んだことになる
- 重大度: 中
- 文書: docs/requirements.md:1284
- 要件: NFR-25
- 根拠: requirements.md:1284「版番号は、一般向けの配布（§5「配布の方針」の S-01 の配り方の選び直し）までは **0.x** とし、選び直した後の最初の正式版を **1.0** とする」。改訂後の §5（:1549-1550）「一般向けの配布の前に、専用の WSL 環境か、Windows に直接入れる形かを選び直す」。その未決を「Windows に直接入れる形」で閉じた」。字面どおりなら次の正式版が 1.0 になる。「一般向けの配布」と「選び直し」は、この改訂で別の時点になった
- 直すところ: requirements（NFR-25 の括弧の中を「一般向けの配布」だけにする）
- 機械: できない
- 行き先: requirements（NFR-25 の合図を「一般向けの配布」だけにした）

## C11. 開発版を正式版と並べる要件（FR-106）が、2 つの S-01 / D-01 のサービスの名前と置き場を分けると言っていない。走っている ST54 の D8（仮）は開発版の DB を docker compose で立てる（印のみ）
- 重大度: 中
- 文書: docs/requirements.md:974
- 要件: FR-106, FR-108, FR-109
- 根拠: requirements.md:974-976「C-01 はアプリ識別子 `dev.ashiato.collector.dev`、C-02 は自動起動の項目と通知の差出人 … 開発版の S-01 は正式版と別の D-01 に繋ぐ」。サービスの形では、サービスの名前と ProgramData の下の置き場も取り合う（どちらも要件に無い）。/home/yosis/dev/ashiato2-up-st54/openspec/changes/st54-dev-side-by-side/design.md:114-124 D8（仮）「開発版の DB の器: `docs/release.md` に `ASHIATO_DB_PORT=55433 docker compose -p ashiato-nightly up -d db`」。開発版は公開される配布物（FR-106「公開されているが、サポートしない」）なので、「Docker Desktop も WSL も利用者に入れさせない」（requirements.md:1546）とぶつかる。D8 の反転条件「ST55 のインストーラが待ち受け・DB の器・実行ファイルの置き場を別の規則で決めたら、そちらに合わせる」が受ける。退避の既定 `backups/pre-migrate-dev` も相対の場所（K1 と同じ）
- 失われるもの: なし（D8 は仮で、台帳 provisional.jsonl に載っている）
- 直すところ: requirements（FR-106 にサービスの名前と状態の置き場を足す）。ST54 は印だけ
- 機械: できない
- 行き先: requirements（FR-106 の ★ 2026-11-03 補足）。ST54 は D8（仮）の反転条件が受ける。handoff ST55 にも書いた

## C12. 同梱する PostgreSQL（と zip に入る部品）の許諾を揃える宛先が ST55 と書かれ、FR-127 を持つ ST53（走っている）と食い違う。許諾の検査も同梱の部品を見ない（印のみ）
- 重大度: 中
- 文書: docs/requirements.md:1610
- 要件: FR-127, NFR-26, FR-108
- 根拠: requirements.md:1610 EXT-S の向け先「zip に入る PostgreSQL 以外の部品の許諾は ST55 が揃える」。FR-127（:1078）「配布物に入る依存の許諾の一覧を同梱する」は INDEX.md で ST53 が拾い、ST55 の satisfies（stories.json:1305-1310）に FR-127 は無い。走っている ST53（/home/yosis/dev/ashiato2-up-st53/openspec/changes/st53-license-bundle/design.md:125）の `tools/release/notices.sh <server|collector-windows|apk>` は Rust・Node・gradle の依存から組み、同梱の PostgreSQL を数えない。production-prep.md:46「AGPL-3.0 と両立しない許諾の依存を CI で落とす」も、同梱するバイナリ（zip の DLL）は `tools/check-licenses.sh` の対象の外になる
- 失われるもの: なし
- 直すところ: requirements（EXT-S の向け先を「FR-127 を持つ Story」にするか、ST55 の satisfies に FR-127 を足す）。ST53 は印だけ
- 機械: 安く捕まえられる（check-artifacts.sh で、サーバの配布物に PostgreSQL の実行ファイルがあるなら THIRD-PARTY-NOTICES.txt に PostgreSQL の見出しがあることを見る）
- 行き先: requirements（EXT-S の向け先を「同梱する ST55 が FR-127 の一覧と NFR-26 の検査に足す」にした）と handoff ST55

## C13. 下流の ST44 の D8（仮）が、新しい秘密（`WEB_SESSION_KEY`）を本人の `.env` へ足す形で入る（印のみ）
- 重大度: 低
- 文書: openspec/changes/st44-collector-credential-scope/design.md:88
- 要件: FR-109, FR-111
- 根拠: design.md:88「印の鍵は環境変数 `WEB_SESSION_KEY` で渡し、無い・短い・ほかと同じなら起動しない」・:162「本人の機械の `.env` に `WEB_SESSION_KEY=$(openssl rand -hex 24)` を 1 行足す」。反転条件（:94）は「更新の経路（ST53〜ST58 の自動の更新）が `.env` に行を足せず」を挙げていて、ST55 のインストーラが作る秘密の一覧（FR-110 の DB の合言葉、FR-111 の画面の合言葉）にこの鍵が入ることは、どこにも書かれていない
- 失われるもの: なし
- 直すところ: 凍結した Story への申し送り（ST44 から ST55 へ「インストーラは `WEB_SESSION_KEY` も作る」）
- 機械: 安く捕まえられる（ST55 の試験で、作った設定でサーバが起動するかを見れば、鍵の作り忘れで落ちる）
- 行き先: handoff ST55（インストーラが作る秘密に `WEB_SESSION_KEY` を入れる）

## C14. 技術的制約の「版は実装時に決める」が FR-132 の「17」と合わない
- 重大度: 低
- 文書: docs/requirements.md:1488
- 要件: FR-132
- 根拠: requirements.md:1488「D-01 は **PostgreSQL**（本人の指定）。版は実装時に決める」。:1103「配る D-01 と、開発と試験の D-01 の PostgreSQL を、同じメジャー版（17）にする」
- 直すところ: requirements（:1488 に ★ 2026-11-03 で FR-132 を指す）
- 機械: できない
- 行き先: requirements（技術的制約の行に ★ 2026-11-03 で FR-132 の 17 を指した）

## C15. ST58 の壊しうるものが、退避の置き場を旧い呼び名「Windows のフォルダ」で書いている
- 重大度: 低
- 文書: docs/stories/stories.json:1395
- 要件: FR-109, FR-119
- 根拠: stories.json:1395「移行の前の自動の退避と、その置き場（Windows のフォルダ。ST55）」（docs/stories/ST58.md:39 も同じ）。改訂後の FR-109 は「PC 全体の置き場（Windows では `ProgramData` の下）」
- 直すところ: requirements の後の /stories（ST58 の breaks）
- 機械: 安く捕まえられる（check_chain.py は ST58 の満たす要件に FR-109 が無いので拾わない。旧い本文の語を stories.json から grep する検査なら拾える）
- 行き先: stories（`stories.json` の ST58 の breaks を「PC 全体の置き場（FR-109。ST55）」に直した）

## 2. 文書とコード

## K1. 移行の前の退避の既定が作業ディレクトリからの相対で、`pg_dump` も PATH から引く。サービスで起こすと FR-109 の置き場に入らない
- 重大度: 中
- 文書: docs/requirements.md:995
- 要件: FR-109, FR-119, FR-108
- 根拠: crates/server/src/migrate_backup.rs:30-33 `MIGRATE_BACKUP_DIR` が無ければ `PathBuf::from("backups/pre-migrate")`、`PG_DUMP` が無ければ `PathBuf::from("pg_dump")`。サービスの作業ディレクトリは既定で `C:\Windows\System32` になり、同梱の PostgreSQL の `pg_dump` は PATH に無い。退避に失敗すると移行を当てない（migrate_backup.rs のテスト `failed_or_empty_backup_is_an_error`）ので、記録は失われないが、渡し忘れると最初の更新が必ず止まる。requirements.md:995 は退避を「PC 全体の置き場（Windows では `ProgramData` の下）」に置くとする
- 直すところ: コード（既定を FR-109 の置き場と同梱の `pg_dump` にする。または ST55 の design で渡すと決め、試験で固定する）
- 機械: 安く捕まえられる（FR-108 の Windows の CI の試験で、`MIGRATE_BACKUP_DIR` を渡さずに migrate を当てる）
- 行き先: handoff ST55

## K2. FR-109 の「サービスのアカウントだけが書ける」の守りが、Windows のコードに無い（権限を付けるのは Unix だけ）
- 重大度: 中
- 文書: docs/requirements.md:995
- 要件: FR-109
- 根拠: crates/server/src/media.rs:41-43（`#[cfg(unix)] const PRIVATE_DIR_MODE: u32 = 0o700`）、crates/server/src/migrate_backup.rs:141-163（`#[cfg(unix)]` の 0700 / 0600）、`grep -rn "cfg(windows)" crates/server/src` は 0 件。Windows では置き場を継承の権限のまま作る（ST55 の premises P16 も同じことを確かめている）
- 直すところ: コード（ST55 で。置き場を作る側が ACL を付ける）
- 機械: 安く捕まえられる（ST55 の Windows の試験で、作った置き場の ACL を読む）
- 行き先: handoff ST55

## K3. 書庫の置き場の手順書は「サービスから起動しても写しを見失わない」と言うが、サービスでは既定の取り込み待ちとダウンロードが本人のものでなくなる
- 重大度: 中
- 文書: docs/archive-inbox.md:16
- 要件: FR-14, FR-108
- 根拠: archive-inbox.md:14-16「書かなければ `%USERPROFILE%\Documents\ashiato\取り込み待ち` / `%USERPROFILE%\Downloads` / `%LOCALAPPDATA%\ashiato\archive-copies` … 起動した場所に依らないので、サービスやタスクスケジューラから起動しても写しを見失わない」。crates/server/src/archive/config.rs:36-41 は `USERPROFILE` / `LOCALAPPDATA` をそのプロセスの環境から読むので、サービスのアカウントで動くとサービスのプロファイルの下になる。写しを見失わないのは正しいが、本人が落とした Takeout は読まれない（C5）
- 直すところ: その文書（サービスで動かすときの置き場の渡し方を足す）。中身は C5 の決着に従う
- 機械: できない
- 行き先: handoff ST55（`docs/archive-inbox.md` の書き直しも ST55 で。中身は C5 の決着）

## K4. FR-108 が根拠に挙げる ST55 の premises.md が main に無い
- 重大度: 低
- 文書: docs/requirements.md:984
- 要件: FR-108
- 根拠: requirements.md:984「ST55 の深掘りの下調べ（`openspec/changes/st55-server-installer/premises.md` P3）」。main の `openspec/changes/` には `st44-collector-credential-scope` と `archive` しか無い（`ls openspec/changes/st55-server-installer` → No such file or directory）。ファイルは未 merge の枝 `docs/st55-upstream`（/home/yosis/dev/ashiato2-up-st55）にだけある。C2 で書き直すと P3 の番号も変わりうる
- 直すところ: requirements（P3 の結果を逐語で本文に持つ。または枝と commit を添える）
- 機械: 安く捕まえられる（要件の本文のバックティックのパスが木にあるかの検査。いまは無い）
- 行き先: requirements（FR-108 の ★ 2026-11-03 補足に枝 `docs/st55-upstream` と commit `4f068c66` を書いた。結果の逐語は本文にある）

## 確かめた事実（食い違いなし。指摘ではない）

- FR-108 の「ST52 が閉じた（`ashiato-server-<版番号>-windows-x86_64.zip`）」: openspec/specs/distribution/spec.md:631、tools/release/build.sh:55
- FR-131 の「Linux の配布物（`ashiato-server-<版番号>-linux-x86_64.tar.gz`）は ST52 が作る」: tools/release/build.sh:57・:127。「S-01 は Linux で組み立てて試験する（CI）」: .github/workflows/ci.yml:57-60（`rust` job は ubuntu-24.04）
- FR-132 の「`docker-compose.yml`（`postgres:17-alpine`）」: docker-compose.yml:3。repo の中に別の PostgreSQL の版の指定は無い
- S-01 は Windows で直に動かし、待ち受けと DB の宛先を loopback に限る: crates/server/src/net_guard.rs:31（`DEFAULT_BIND = "127.0.0.1:18787"`）・:57-72（`db_host_is_local`）・crates/server/src/lib.rs:4388-4396。同梱の D-01 を loopback で待ち受ければこの守りはそのまま効く
- production-prep.md:48 の向け先 EXT-S は requirements.md:1610 にある

## 未確認

- Windows の `ProgramData` の下に作ったフォルダの継承の既定で、`Users` に読みが許されるか（C4 の前提。この機械は WSL で、Windows の ACL を読んでいない）
- サービスのアカウント（LocalService・仮想アカウント `NT SERVICE\…` など）で動かしたときに `USERPROFILE` / `LOCALAPPDATA` がどこを指すか（C5・C6・K3。走らせていない）
- EDB の zip に入っている PostgreSQL 以外の部品（OpenSSL・ICU・libxml2 ほか）とその許諾、zip の大きさ（C12。落として中身を見ていない）
- zip の `initdb` を `-A` なしで走らせたときの `pg_hba.conf` が手元の接続を trust で通すか（C7。PostgreSQL の initdb の既定としての記憶で、走らせていない）
- PostgreSQL 17 の公式の支援が 2029 年 11 月までか（FR-132。versioning のページを読んでいない）
- `docs/briefs/req-native-postgres-r1.html`・`-r2.html` は git の外（`.gitignore:34` の `docs/briefs/`）。中身は手元のファイルで読んだが、ほかの機械からは辿れない（従来の `req-distribution-r1.html` と同じ扱い）

## 触らなくてよいと判断したもの

- 開発と試験の道具の Docker への依存（`tools/check-db-version-stop.sh`・`check-panic-log.sh`・`check-db-secret.sh`・`check-verify-env-reset.sh`・`verify-prep.sh`・`pg_dump-docker.sh`・`db-roles.sh`、docs/testing.md:32・:104・:146-148、openspec/specs/verify-environment・distribution/spec.md:274-281、README.md:27）: FR-108・FR-132 は開発と試験の DB を Docker のまま残すと決めている
- `tools/check-exposure.sh:169-177` が DB の port を `docker compose config` から取る: `DB_PORT` で渡せるので、配る形でもそのまま使える
- production-prep.md:137（`docker compose up -d --wait db` を 2026-09-08 に走らせた記録）と過去の監査（docs/audit/2026-10-25-consistency.md の EXT-O = Docker Desktop の行）: その日の記録で、本文の決定ではない
- requirements.md:1000（FR-109 の ★ Q4「コンテナの中のボリュームは…」）・:1544-1552 の旧い本文の引用: 経緯の ★ で、いまの本文ではない
- ST11.md:23・ST34.md:30 などの EXT-O: いまの EXT-O（Health Connect の履歴の権限）を指していて正しい。旧い Docker Desktop の EXT-O は外された
- C-02 の自動起動（ログオン時・本人の権限。crates/collector-windows/src/autostart.rs・README.md:42）と `%LOCALAPPDATA%` の使用（crates/collector-windows/src/main.rs）: FR-108 は C-02 を「従来どおりログオン時に本人の権限で起こす」とした
- S-01 をサービスとして登録する口（サービスの制御の受け手）がまだ無いこと（crates/server/src/main.rs:6-10 は引数なしと `migrate` だけ）: 要件は今あるとは言っていない。ST55 が作る
- 開発（alpine・musl）と配る形（Windows の EDB の版）の locale・文字の並び順の違い: FR-132 の Q4 の推奨で Windows の CI の試験 1 本が受けると決めた。足りるかは ST55 の深掘りで見る範囲
- FR-131 の Linux の置き場（FR-109 は「Windows では `ProgramData`」とだけ書く）と、コードの既定（`XDG_DATA_HOME` / `HOME`）: Linux の D-01 の入れ方を期限つきの未決にしてあり、Linux の配布の Story の上流で決める
- FR-131 / FR-132 がどの Story にも拾われていないこと: check_chain.py が捕まえていて、/stories で解く予定
