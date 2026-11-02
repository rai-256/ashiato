# ST54 design — 開発版を正式版と並べて入れられる

## Context

動機は `proposal.md` の Why。いまの状態（premises の P1〜P17）:

- 携帯: `build.gradle.kts` は版番号が `-dev.` のときだけ `applicationIdSuffix = ".dev"` を付ける（ST52）。アプリの名前は `AndroidManifest.xml:73` の `android:label="あしあと。"` の 1 つ（P3）。配布物の APK は接続先と資格情報を持たず、ST56 まで送らない（P14）
- PC の収集アプリ: 自動起動の項目 `autostart::ENTRY_NAME = "ashiato-collector.cmd"`、通知の差出人 `toast::AUMID = "dev.ashiato.collector"`・表示名 `ashiato`（P4）。置き場と端末識別子は環境変数で渡し既定を持たない（P5）。
  二重起動の止め（`main.rs:63-77`）と除外の登録（`main.rs:81`・`exclusion.rs:112`）は置き場の中のファイル。送る契機の頭で `GET /contract` を 1 回引き、`ContractGate` が答えで送るかを決める（`runtime.rs:810-`）
- サーバ: 版の記録（ST48）の行は当てた組み立ての版番号を持つ。知らない版で終了コード 3（P6）、中身の違う版は警告だけ（P7）。既定の待ち受け `127.0.0.1:18787`（P12）、クッキー `ashiato_session`（`web_session.rs:28`）、退避の既定 `backups/pre-migrate`（`migrate_backup.rs:30`）。DB の繋ぎ先 `DATABASE_URL` は既定なし（P13）
- 配る道具（ST52）: `nightly.yml` はサーバの 2 つと PC の収集アプリだけを組み、`check-artifacts.sh --channel nightly` は APK があれば落とし、`check-guards.sh workflows` は `nightly.yml` に APK の手順があれば落とす。開発版の説明文は最初の公開で 1 回だけ書く（P11）。
  `sign-apk.sh <apk> dev <出力>` は `ASHIATO_EXPECT_DEV_CERT_SHA256` が無いか違えば署名しない（ST52 code_verify R26）。`make-key.sh dev` は RSA 4096・10000 日の鍵を作る（P15）

## Goals / Non-Goals

**Goals**: 同じ携帯・同じ PC・同じ機械に開発版と正式版を並べても、記録・自動起動・通知・DB・送り先を取り合わないこと。開発版のリリースに携帯の APK を載せる仕組みを、本物の鍵と本物の公開なしに（下書きと使い捨ての鍵で）通すこと。

**Non-Goals**:
- 本物の開発版の鍵を作ること・`nightly` の Environment に置くこと・開発版の公開を始めること（Q9 の推奨。ST53 の後に本人が `docs/release.md` の手順で行う）
- 収集アプリの接続先と資格情報を入れる口（ST56）。開発版と正式版は別のアプリなので、ST56 の設定も別に持つ（premises の「他の Story へ回すもの」）
- サーバのインストーラ（ST55）。開発版の DB の器は手順書で立てる（D8（仮））
- 開発版のサーバの実行ファイルの名前を分けること。サーバの zip を同じフォルダに展開する置き方は ST55 が決める（下の Risks）

## Decisions

### D1. 系統（開発版か正式版か）は、組み立てのときの版番号だけで決め、`crates/build-stamp` の 1 つの関数が判定する

> 深掘り Q1（答え待ち・推奨: 組み立てのときの版番号で決める）

`ashiato-build-stamp` に `channel(version: &str) -> Channel`（`Channel::Dev` / `Channel::Official`）を足す。版番号に `-dev.` を含めば `Dev`。サーバと PC の収集アプリは `channel(env!("ASHIATO_VERSION"))` を起動時に 1 回求め、
`App`（サーバ）と `Runtime`（PC の収集アプリ）の持ち物にする。試験だけがこの持ち物を差し替える（ST50 の `contract_version` と同じ形。実行時の設定の口は作らない）。
携帯は ST52 の `resolvedBuildVersion.contains("-dev.")` をそのまま使い、`BuildConfig.CHANNEL`（`"dev"` / `"official"`）を足して `ContractGate` が読む。
- 選ばなかった案（Q1 の選択肢 2）: 起動のときの環境変数。設定を忘れると正式版の名前で上書きする
- 手で組んだ版（版番号を渡さない。worktree・縦串・確認バッチの組み立て）は正式版として振る舞う。今の開発と確認の流れはどれも正式版の名前のまま動く

### D2. PC の収集アプリの開発版は、渡された置き場の下の `dev` と、末尾に `-dev` を付けた端末識別子を使い、除外は渡された置き場から読む

> 深掘り Q2（答え待ち・推奨: 開発版は自分で分ける。除外の登録は正式版の置き場のものを読む）

`Config::from_lookup` に系統を渡す。開発版のときだけ: `state_dir = <ASHIATO_STATE_DIR>/dev`、`device_id = <ASHIATO_DEVICE_ID>` に `-dev` を付ける（すでに `-dev` で終われば付けない）、
除外の登録の置き場 `exclusions_path = <ASHIATO_STATE_DIR>/exclusions.json`（正式版は今どおり `state_dir/exclusions.json`）。除外の登録を読む口は 2 つあり、**どちらも**この `exclusions_path` を読む: 前景の窓の除外（`main.rs:81` の読み込み）と、ブラウザの履歴の取り込み（今は `HistoryCollector::new` が `state_dir.join("exclusions.json")` を自分で作る。`history/collect.rs:315`）。`HistoryCollector::new` は除外の登録の置き場を引数で受け、`Runtime::with_history` が設定の `exclusions_path` を渡す（履歴の写し・前回の成功・送った版の置き場は今どおり `state_dir/browser-history/`）。
二重起動の印・未送信・送った印・履歴の取得の印は `state_dir` の下なので、置き場を分ければ正式版と開発版が両方起動する。
- 自動起動の `.cmd` に書く変数は、渡された値（`dev` を掘る前の置き場・`-dev` を付ける前の端末識別子）を書く（次のログオンで同じ規則がもう一度当たる。二重に掘らない）
- 開発版は初回に履歴の取得の印が無いので、ブラウザの履歴を全件取り込む（正式版の初回と同じ量。Q2 の context）
- Q2 の推奨の「正式版の置き場」は「開発版に渡した置き場」と読む: 開発版は正式版の置き場を自分で知る手段を持たない（P5。置き場は本人が渡し既定が無い）。正式版と同じ `ASHIATO_STATE_DIR` を渡せば正式版の除外がそのまま効き、別の値を渡すと効かない。この読み替えは Q2 の context に書いて本人に渡した（review/spec.md R5）
- 除外の登録は読むだけで、PC の収集アプリは書かない（`exclusion.rs` に書く口は無い）。正式版の除外を開発版が書き換えることは無い
- 第 1 回の合意で IMP-01 を「仕様を直す」と答えた（`deep.md`「仕様への違和感」）。前の版は直す口を `main.rs` の 1 つだけ挙げており、そのとおりに作ると開発版の履歴の除外は常に空になり、除外したサイトの閲覧履歴（初回は過去の全件）が開発版のサーバへ送られた。前景の窓の Scenario だけでは試験がそれを見逃すので、履歴の取り込みの Scenario「開発版は、ブラウザの履歴の取り込みでも渡された置き場の除外の登録を読む」を足した

### D3. DB の系統は、版の記録の最初の行の組み立ての版番号で決め、違う系統は終了コード 4 で止める

> 深掘り Q3（答え待ち・推奨: DB に開発版か正式版かの印を残し、違う側のサーバは何も書かずに止まる）

印のための新しい表や移行を足さない。版の記録（ST48）の行はもう当てた組み立ての版番号を持ち、書き換えられない（「版の記録は…決めた形のまま変わらない」）。
最初に当てた行（`applied_at` が最も古い。同じなら名前の順で最初）の版番号が `-dev.` を含めば開発版の DB。版の記録の schema が無く `core` の schema がある DB（版の記録より前の DB。今のコードの `BeforeLedger`）は正式版の DB、`core` が無い DB（`Empty`）と版の記録の行が 0 の DB（最初の移行が落ちた）は系統なし。
- 検査の置き場: `migration_ledger` の、記録を読んだ直後・`Verdict` を出す前（サーバの起動の検査と `migrate` の `apply_locked` の両方が通る所）。系統が違えば `MigrateError::ChannelMismatch { db, me }` を返し、退避（`migrate_backup`）・`BOOTSTRAP`・付与・更新の履歴（ST51）より前に止まる。`record_result`（`migration_ledger.rs:421`）の更新の履歴を書かない早い返りにも `ChannelMismatch` を足す
- 終了コード 4（新しく足す）。3（版の食い違い）と分けるのは、縦串と確認バッチの台本が 3 を「作り直さずに手順を示す」（正典「開発と確認の DB が版の食い違いで止まっても…」）に使っており、系統の取り違えはその手順で直らないため
- 「印の無い今の DB は正式版の DB」は、今ある DB の版の記録の行が全部、版番号を渡さない組み立て（`0.1.0` など）で当たっていることによる。版の記録より前の DB を開発版の `migrate` が `BOOTSTRAP` で開発版の DB にしてしまう道は、表があれば正式版とみなすことで塞ぐ
- 選ばなかった案: 新しい移行で印の表を足す。移行が 1 つ増え、印と版の記録が食い違う場合（印だけ消えた・書き換えた）の扱いが要る。版の記録は既に書き換えられない
- 選ばなかった案（Q3 の選択肢 2）: 手順書と既定の名前で分けるだけ。取り違えると正式版の DB が先に進み、正式版のサーバが起動しなくなる
- 更新の失敗からの戻し（ST58。FR-119）への申し送り: `docs/handoff/ST58.md` は今、`migrate` の終了コードを 1（移行の失敗。その回の退避がある）と 3（退避を取らずに止まった）だけで分け、戻すのは 1 の回だけと書く。終了コード 4 もその回の退避を取らないので、`rc≠0` を一律に移行の失敗と扱うと前の回の退避（それ以後の記録を含まない）を戻しうる。この change が申し送りに「終了コード 4（`kind=channel_mismatch`。系統が違うので退避も何も書かずに止まった）の回は退避から戻さず、終了コード 3 の回と同じく前の版のまま起動し直す。DB には何も書いていない」を書き足す（tasks 3.2）。第 1 回の合意で IMP-02 を「仕様を直す」と答えた（`deep.md`「仕様への違和感」）

### D4. 旧名の対応 `migrations/LINEAGE` を正本にし、サーバは `renamed` と `removed` を読み、検査が形を見る

> 深掘り Q4（答え待ち・推奨: 止まったら原因を示し、DB は書き換えない。main 側で直す）

開発版の DB が main とずれたとき、DB を書き換えずに次の版で直す道を、main の側のファイル 1 つで持つ:
- `renamed <旧名> <今の名前>`: 版の記録の旧名の行を、今の名前の版が当たっているものとして読む（今の名前を当て直さない）。指紋は今の名前の中身と比べ、違えば今どおり警告
- `removed <旧名>`: 旧名の行を知っている版として読む（`Ahead` に数えない）
- `rewritten <名前> <直しの移行>`: 開発版に載った移行の中身を直したとき、直しの移行を足したことを示す。サーバの比べ方は変えない（直しの移行は普通の移行として当たる）。D5（仮）の検査が読む
`crates/server/build.rs` が `migrations/ORDER` と同じように LINEAGE を組み込む（`LINEAGE: &[(Kind, &str, &str)]`）。`compare`（`migration_ledger.rs:174-`）は、比べる前に DB の名前の集合を LINEAGE で写す（旧名 → 今の名前・`removed` は外す）。版の記録の行は書き換えない。
`tools/check-migrations.sh` が LINEAGE の形（3 つの種類・名前の数・今の名前が ORDER にある・`rewritten` の直しの移行が直す移行より後）を見る。
- 直しの移行は、新しい中身で当てた DB にも当たるので、どちらの DB でも同じ結果になる書き方（`IF NOT EXISTS`・`CREATE OR REPLACE` など）にする。これは手順書（`docs/release.md` の「開発版の DB が止まったら」）に書く規約で、機械は見ない（Risks）
- 正式版の DB には影響しない: 配った印 `v*` の版の移行は名前も中身も変えられない（ST48）ので、正式版の DB の行が LINEAGE の旧名に当たることは無い
- 止まったときの原因は、今の `error: kind=db_ahead` の `  db: <名前> (<版番号> <commit>)` が示す（Q4 の「原因を示し」）
- 選ばなかった案（Q4 の選択肢 2）: 開発版の DB は作り直してよいものとし、作り直す。開発版にしか無い記録が失われる（Q4 の loss）
- 選ばなかった案（Q4 の選択肢 3）: 開発版に載った移行も凍結する。ST52 Q10 の本人の答え（`nightly` は配った印に数えない）と正典を変える

### D5（仮）. 開発版に載った移行を、旧名の対応なしに変えた PR は、CI の検査で落とす

D4 は「直すための道」を用意するだけで、中身だけを変えた移行は警告だけで止まらない（P7）—— 誰も気づかなければ開発版の DB の形は黙って main とずれる。そこで、ずれを生む変更を PR の時点で機械が止める:
`tools/check-nightly-migrations.sh`（`--self-test` つき）が、印 `nightly` の版の `migrations/ORDER` と各版の前進側の `.sql` を今の木と比べ、
消した・名前を変えた版に `removed` / `renamed` が無い、中身を変えた版（`renamed` で名前を変えた版は、今の名前の中身を `nightly` の旧名の中身と比べる）に `rewritten` が無いとき落とす。浅い木では印を見る前に落とす（ST52 D1 と同じ止め）。印 `nightly` が無ければ通す。CI の `rust` の job（完全な履歴。ST52 D1）で `check-released-migrations.sh` の隣に走らせ、
取り寄せは `git fetch --force --no-tags origin '+refs/tags/nightly:refs/tags/nightly'`（無ければ何もしない）。
- 失わない側: 止めがあれば、開発版の DB の形が黙ってずれることは起きない（直しの移行を足せば通る）。止めが無いと、ずれた DB に開発版の記録が書かれ続ける
- 費用: 台本 1 本と CI の 1 手順。移行を直す PR に、直しの移行と LINEAGE の 1 行が要る（今の頻度で週に 1 回ほど。premises P8）。比べは中身のバイト列で、コメントや空白だけの直しでも `rewritten` を求める（配った印の凍結（ST48）と同じ粒度）
- 反転条件: 開発版の公開を始めた後、この検査で止まる PR が多く、直しの移行の書き方の手間が開発を止めると分かったら、検査を警告にする（開発版の DB の作り直しを選び直すなら Q4 を本人に聞き直す）
- 台帳: `provisional.jsonl` の D5（source: deep.md#Q4。Q4 に依る）

### D6. 開発版の作り直しは携帯の APK を署名せずに組み、公開の job が開発版の鍵で署名して確かめてから入れ替える

> premises の C3・C4・C5（正典と ST52 の申し送りから導いた。聞かずに決める）

- `nightly.yml` の `package-linux` に `tools/release/build.sh apk` を足す（`ASHIATO_APK_UNSIGNED=1`。`release.yml` の組み立てと同じ。ST52 D18）
- `publish` の手順は `tools/release/publish-nightly.sh <置き場> <版番号>` に移す（YAML に論理を書かない。ST52 D8）。順は: 開発版の鍵を `$RUNNER_TEMP` に戻す → `sign-apk.sh <apk> dev <出力>`（`ASHIATO_EXPECT_DEV_CERT_SHA256` は `nightly` の変数 `ASHIATO_CERT_SHA256` から）→ 署名していない APK を置き場から外す →
  `check-artifacts.sh --channel nightly`（`ASHIATO_EXPECT_CERT_SHA256` も同じ変数）→ 配布物の入れ替え（ST52 D15）→ 説明文の書き直しと確かめ（D9）→ 印 `nightly` を動かす。鍵は job の終わりに消す
- 鍵の Secrets が空なら `sign-apk.sh` が落ち、それより後の手順は走らない。開発版のリリースは前の版のまま残り、出力に「開発版の鍵が無い」を出す（正典「配布物が 1 つでも組めなければ公開しない（…開発版は前の版のリリースを残す）」と FR-104 から導く。review/deep.md R9）
- `check-artifacts.sh --channel nightly` は開発版も 4 つを期待する。開発版の APK の名前は `ashiato-collector-dev-<版番号>.apk`（ST52 D17）で、正式版の名前の APK があれば落とす（取り違えの止め）。識別子が `.dev`・指紋が開発版の鍵のものかは今の検査が見る
- `check-guards.sh workflows` の nightly の検査を反転する: 組み立ての job に `build.sh apk` があり、`sign-apk.sh`（`publish-nightly.sh` の中）は `environment: nightly` の付いた `publish` からだけ呼ばれ、`ASHIATO_EXPECT_DEV_CERT_SHA256` を `vars.ASHIATO_CERT_SHA256` から渡す
- `publish-nightly.sh --self-test` は偽の `gh` と使い捨ての鍵で、鍵がある・無い・説明文が書き直されない、を走らせる

### D7（仮）. サーバは `GET /contract` の応答の見出しに系統を載せ、収集アプリは系統が違うサーバへ何も送らない（正式版の収集アプリも開発版のサーバへ送らない）

> 深掘り Q6（答え待ち・推奨: 開発版のサーバにだけ送る。確かめ方は spec で決める）に、逆向きを足した

- サーバ: `GET /contract` の応答に見出し `Ashiato-Channel: dev | official` を足す（`docs/openapi.json` の応答の見出しの記述を作り直す）。本文 `{"contract_version": <整数>}` は変えない —— 答えの形は正典 record-envelope が最初の配った印の後は変えないと決めていて、ST50 の試験 `st50_contract_answers_version` は本文の完全一致を見る（review/spec.md R1）。収集アプリが送る封筒の形と送信契約の版も変えない
- PC の収集アプリ: `/contract` の応答の見出し `Ashiato-Channel` を読み（`Transport::get` の返す `Reply` に見出しを足す）、無ければ `official`。`ContractGate` の前に系統を比べ、違えばその契機は何も送らず（記録・生存信号）、`warn` のログに `channel_mismatch server=<相手> self=<自分>` を出す。違う系統の答えは、契約の版の待ち（ST50 の「サーバを更新してください」の通知）に数えない
- 携帯の収集アプリ: `ContractGate.kt` の答え `Version` に応答の見出しの系統を持たせ、違えば新しい答え `ChannelMismatch` にして、5 つの送り手（記録・生存信号・破棄の報告・原本の欠け・原本）が何も送らない。ログは `Log.w`。系統の違いは「サーバを更新してください」の条件に数えない（spec に Scenario）
- **逆向き（正式版の収集アプリ → 開発版のサーバ）も止める**: Q6 は開発版の収集アプリだけを聞いた。逆向きに送ると、正式版の未送信が開発版の DB に入って正式版の手元から消え、正式版の DB には届かない（正式版の記録が開発版の DB にしか無くなる）。開発版の DB を作り直す・止める日に失われうる
- 失わない側: 送らなければ未送信は手元に残り、送り先を直せばそのまま送られる。止めないと、正式版の記録が開発版の DB に入る（取り違えに気づくまで）
- 費用: 答えの欄 1 つと、2 つの収集アプリのゲートに比べを 1 つずつ。系統を答えない古いサーバは正式版とみなすので、今の正式版の組み合わせは何も変わらない
- 反転条件: 開発版のサーバで正式版の収集アプリの記録を試したい（正式版の記録を開発版の DB に入れたい）と本人が言ったら、逆向きの止めを外す。通知が無いために送り先の取り違えに何日も気づかなかったと分かったら、ST50 の通知と同じ形で知らせる
- 台帳: `provisional.jsonl` の D7（source: deep.md#Q6。Q6 に依る）

### D8（仮）. 開発版のサーバの既定は、待ち受け `127.0.0.1:18788`・クッキー `ashiato_session_dev`・退避 `backups/pre-migrate-dev`、開発版の DB の器はポート 55433

> premises の C2・C6・C7・C8（review/deep.md R10。聞かずに決める）

- 待ち受け: `net_guard::default_bind(channel)`。正式版は `127.0.0.1:18787` のまま。`18788` は `tools/ports.sh` の割り当て（18787・19000〜19399）の外
- クッキー: `web_session::cookie_name(channel)`。正式版は `ashiato_session` のまま。クッキーはポートで分かれない（P17。RFC 6265 §8.5）ので、同じ host で開いた 2 つの画面が同じ名前を上書きし合わないように名前を分ける
- 退避: `MIGRATE_BACKUP_DIR` が無いときの既定だけを分ける。同じ作業ディレクトリから 2 つを起こしても、退避が 1 つの置き場に混ざらない
- 開発版の DB の器: `docs/release.md` に `ASHIATO_DB_PORT=55433 docker compose -p ashiato-nightly up -d db`（別の project なので volume も別）。`55433` は ports.sh の DB の割り当て（55432・55499・55500〜55699）の外
- PC の収集アプリの実行ファイル: 開発版の zip の中の名前を `ashiato-collector-windows-dev.exe` にする（`tools/release/build.sh` が版番号で決める）。同じフォルダに展開しても正式版の実行ファイルを上書きしない。自動起動の `.cmd` は `current_exe()` を書くので、それぞれ自分の実行ファイルを起こす
- 失わない側: 名前を分ければ、2 つを並べても片方のログイン・退避・実行ファイルを上書きしない。どれも後から名前を変えれば戻る
- 反転条件: ST55 のインストーラが待ち受け・DB の器・実行ファイルの置き場を別の規則で決めたら、そちらに合わせる。`tools/ports.sh` の割り当ての範囲が 18788 か 55433 を含むように広がったら、ここを動かす
- 台帳: `provisional.jsonl` の D8（source: review/deep.md#R10）

### D9. 開発版の説明文は `tools/release/nightly-notes.md` に置き、公開のたびに書き直して 1 行目を確かめる

> 深掘り Q8（答え待ち・推奨: 説明文の先頭に書き、公開のたびに書き直す）

`publish-nightly.sh` が、配布物を入れ替えた後に `gh release edit nightly --notes-file tools/release/nightly-notes.md` を出し、`gh release view nightly --json body` の 1 行目が「この開発版はサポートしません。」で始まるかを見る。
始まらなければ 0 でない終了コードで終わり、印 `nightly` を動かさない（配布物は入れ替わった後だが、印が前の commit のままなので次の回に作り直す）。
最初の公開の `gh release create` も同じファイルを `--notes-file` で渡す。spec が固定するのは 1 行目だけで、2 行目以降（並べて入れる・別の DB に繋ぐ・手順書へのリンク）は文書として直してよい。

### D10. ST54 の完了の判定は、下書きのリリースと使い捨ての鍵で確かめた形で見る

> 深掘り Q9（答え待ち・推奨: 下書きのリリースと使い捨ての鍵で確かめる。完了の判定にこの 3 つを足す）

Q9 の推奨のもとでは、本物の開発版の鍵はまだ作らず、開発版の公開も始めない（ST52 Q5: ST53 の後に本人が始める）。そこで完了の判定（`docs/stories/ST54.md`）を次のように読む:
- 「携帯に開発版を入れても、正式版はそのまま動き、端末内の記録も残る」→ エミュレータに 2 つを並べて入れる台本（`tools/android-side-by-side.sh`）が通る（P2 を 4 に上げる）
- 「PC に開発版を入れても、正式版の自動起動と通知は別のまま残る」→ Windows の runner の実行時の試験が通る
- 「開発版のリリースに『サポートしない』と書いてある」→ 下書きのリリース（印を作らない）の説明文の 1 行目で見る
- Q9 で足す 3 つ（開発版のリリースに携帯の APK が並ぶ・開発版の鍵で署名されている・「サポートしない」の文）→ 手で起こした `nightly.yml`（公開の設定なし。組むだけ）の配布物を `tools/release/draft.sh --channel nightly` が落とし、使い捨ての開発版の鍵で署名し、`check-artifacts.sh --channel nightly` を通して下書きのリリースにする（正典の下書きの Requirement を MODIFIED にした。review/spec.md R12）。本人が下書きの並び・APK の署名の指紋・説明文を見て消す
- 完了の判定の文面を `stories.json` に写すのは、本人が Q9 に答えた後の record の段（推奨と違えば読み替えを外す）
- 見直す条件: Q9 が「本物の開発版を公開して確かめる」なら、この読み替えを外し、ST54 の確認を ST53 と本人の鍵の用意の後にする

### D11. 携帯の開発版の名前は、組み立ての `manifestPlaceholders` で替える

> 深掘り Q7（答え待ち・推奨: 名前に「開発版」を付ける）・Q10（答え待ち・推奨: 全部を動かす）

`AndroidManifest.xml` の `android:label` を `${appLabel}` にし、`build.gradle.kts` が `-dev.` のときだけ `"あしあと。開発版"`、ほかは `"あしあと。"` を渡す。アイコンは同じ（Q7 の選択肢 2 を選ばない）。
常駐の通知の見出しはアプリの名前を使うので、並べた 2 つの常駐の通知も名前で分かれる。動かす収集は変えない（Q10）。
権限・前景サービス・受け手の宣言は識別子から名前を作る（`${applicationId}`）か、識別子に依らない。並べて入れられることは、エミュレータで 2 つを入れて確かめる（P2。下の Risks）。
PC の通知の表示名「ashiato 開発版」は D1 の系統から `toast::display_name(channel)` が決める。

## Risks / Trade-offs

- [ライブラリが足す provider の authority が識別子に依らない名前だと、2 つ目が入らない（P2 は確かさ 2）] → Task 9 のエミュレータの台本で 2 つを並べて入れ、`adb install` が通ることを見る（4 に上げる）。落ちたら、その provider の名前を `${applicationId}` から作る設定を足す（振る舞いは変わらない）
- [直しの移行を、新しい中身で当てた DB にも同じ結果になるように書くことは、機械が見ない（D4）] → `docs/release.md` に規約を書き、開発版の DB が止まったときの手順（原因の読み方・LINEAGE の書き方・直しの移行の書き方）を置く。直しを誤っても開発版の DB だけに効き、正式版の DB の移行は凍結されている（ST48）
- [系統の判定を版番号に頼るので、手で版番号に `-dev.` を渡して組めば、手元の組み立ても開発版になる] → それは意図した振る舞い（D1）。手元の DB に当てると開発版の DB になり、正式版の組み立てでは止まる（D3）。手順書に書く
- [携帯の端末識別子は、アプリごとに初回に採番して自分のデータの置き場に持つ（`DeviceId.kt` の `resolveDeviceId`。`ANDROID_ID` を使わない）] → 識別子が違えば置き場も違う（P1）ので、並べた 2 つは別の端末識別子になる。PC のように末尾を付ける規則は要らない
- [サーバの zip の実行ファイルの名前は正式版と開発版で同じ] → 同じフォルダに展開すると上書きする。ST55 のインストーラが置き場を決める（Non-Goals）。手順書は別のフォルダに展開すると書く
- [開発版の収集アプリが送らずに手元に積み続けても、本人は気づきにくい（D7）] → ログに残す。D7 の反転条件

## Migration Plan

- 正式版の名前・既定は何も変えないので、今の本人の PC の収集アプリ・サーバ・DB はそのまま動く。今の DB は版の記録の行が全部正式版の組み立てなので正式版の DB（D3）
- `migrations/LINEAGE` は空（説明の行だけ）で入る。印 `nightly` はまだ無い（公開を始めていない）ので、D5（仮）の検査は通る
- 戻すとき: 系統の判定を `Official` に固定すれば、名前・既定・送り先・DB の検査は今に戻る。`nightly.yml` の APK の手順を外せば ST52 の 3 つの開発版に戻る（そのときは `check-artifacts.sh` と `check-guards.sh` の nightly の期待も戻す）

## Open Questions

（無し。Q1〜Q10 は答え待ちで、推奨で書いた。推奨と違った答えは `spec --answers` が写す）
