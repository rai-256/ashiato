# ST56 深掘りの問いの独立レビュー（deep-review）

対象: `openspec/changes/st56-collector-connection-setup/deep-questions.json`（第 1 回。Q1〜Q6 は `open`、Q7 は `irreversible`・`loss: exported`）・`premises.md`（5 行、どれも確かさ 2）と、呼び出し元が示した聞かない既定（C: 携帯の設定をバックアップと端末の移し替えから外す・`-P` の値は保存した設定が無いときの初期値・利用者は設定の文字列に入れ既定は nil の UUID・http は loopback だけ・送り先は画面を開いている口から作る）。
手順 1〜5 は要件・申し送り・走っている change（ST54・ST55）・コードだけで先にやり、そのあとで問いの一覧と突き合わせた。問いの JSON は触っていない。
走らせたのは grep と `scripts/upstream_misses.py report` だけ。エミュレータ・Windows の上では何も走らせていない。

## 手順ごとの確かめ

- **手順 1（要件どうしの衝突）**: FR-128（requirements.md:1100-1109）・FR-133（:1126-1129）・FR-106 の ★ 2026-11-03（:976-982）・FR-8 / FR-9（:184-201）・FR-115（:1034-1035）・FR-92（:390-391）・PERM-10（:1217-1238）・FR-109 / FR-110 / FR-111（:1003-1019）を読んだ。
  (a) FR-92「利用者識別子は送られてきた内容ではなく資格情報から決める」と、聞かない既定「利用者を設定の文字列に入れる」は同じ対象（記録の `user_id`）について逆を言う → R2。
  (b) FR-128 ★ 2026-11-02 の「締め出すたびに … C-01 の未送信が FR-8 の上限を越えた分は捨てる」は本人が受け入れた帰結で、FR-8 と衝突しない（呼び出し元の言うとおり問い直さない）。
  (c) FR-106 は利用者に見せる文「送り先が開発版／正式版のサーバです」を本文で決めている（:982）→ R10
- **手順 2（扉の幅）**: ST56 の doors は空（docs/stories/ST56.md:5・:24）。該当なし（確かめた範囲: ST56.md の「触れる扉」と、requirements.md §5 の関与要件に FR-128 が無いこと）
- **手順 3（新たに立つ一方通行）**:
  (a) 鍵の計算に使う入力: 記録の `user_id` は端末が積んだ時点で行に凍り（`FixCollector.kt:21・53`、`IngestRequest.kt:54`）、サーバは中身の `user_id` をそのまま一意の鍵に使う（`lib.rs:1676・1688` の `ON CONFLICT (user_id, …)`）→ R1・R2。
  (b) 外に出すもの: 送り先を変えたときの未送信（Q7）。ただし記録の置き場以外の経路（原本・生存信号・破棄の報告・途絶の問い合わせ）が Q7 の範囲に入っていない → R5。バックアップから戻した端末で Q7 の守りが効かない → R6。
  (c) 捨てるもの: 配る APK で設定の前に積んだ記録は送った時点で恒久の拒否として捨てる → R1。送り先の誤り（届かない）を 401 と違って知らせない → R7。
  (d) 取っていないもの: PC は設定が無いと起動しない → R4。PC の置き場の既定を持たない → R3
- **手順 4（日常に影響する選択）**: 毎日の気分の通知を押しても画面が開かない（配る APK の `WEB_URL` が空）→ R8。断られ続ける知らせ（Q5）・未設定の知らせ（Q3）は一覧にある。電池・容量は該当なし（確かめた範囲: 設定の確かめは貼った時の 1 回と 401 の知らせだけで、新しい常駐・周期の問い合わせを足す問いが無いこと）
- **手順 5（既存コードが要件を満たしていない箇所）**: FR-128 を `Config.kt`・`LocationService.kt`・`MainActivity.kt`・`build.gradle.kts`・`tools/release/build.sh`・`config.rs`・`main.rs`・`autostart.rs` と突き合わせた。
  (a) 「設定の前に積んだ記録は設定してから送られる（捨てない）」（`Config.kt:7-8`）は配る APK では成り立たない → R1。
  (b) 設定が揃わないとき送る仕組みを一度も立てず（`LocationService.kt:926-931`）、送り先と資格情報は立てた時の値で固まる（`:241`・`:254`・`:258`）ので、走っている間に設定しても次にサービスが起き直すまで送らない → R1 の提案に含めた。
  (c) PC の「揃っていなければ送信を始めないが、取得は続ける」（`config.rs:5-6`）は、`main.rs:14` の `Config::from_env()?` で起動ごと終わるので成り立たない → R4。
  (d) PC の自動起動の台本は合言葉を平文で書く（`autostart.rs:27-39`）。既にある台本の扱いが Q2 に無い → R9
- **手順 6（過去の取りこぼし）**: `upstream_misses.py report` の「拾えたはずの観点」を 1 つずつ当てた。当てはまったのは 4 つ。
  「『送らずに残すので失わない』を根拠にするときは携帯の 90 日・2 GB を挙げ、上限の前に本人へ原因が届くかを確かめる」→ Q5 は 401 だけを見て、届かない送り先の場合が無い（R7）。
  「手元に状態を持って保証する決定には、保存場所が丸ごと消えた・入れ直した場合を問う」→ Q7 の「前の送り先」の覚えがバックアップから戻らない（R6）。
  「送り先の系統を取り違える」「他 Story の決定の波及」→ ST54 D2・ST55 D18 / D27 / 11.1 との重なり（R3・R11）。
  時計の族の観点: Q5 の「1 時間続いたら」は経過で数える規則だが、時刻を入力にして状態を一方向に動かす決定ではない（知らせを出すだけで、時計が狂っても記録は失われない）。該当なしと判断した
- **手順 7（族の網羅）**: Q3（携帯の設定の前の取得）の兄弟の PC → R4。Q5（401 で断られ続ける）の兄弟の「届かない」「平文が遮られる」→ R7。Q7（送り先を変えた）の兄弟の「バックアップから戻した」「同じサーバを別の URL で開いた」と、記録の置き場以外の経路 → R5・R6
- **手順 8（前提の確かさ）**: premises.md は Q7 を「外の形式に依らない」と書くが、Q7 の守りは Android の自動バックアップと端末の移し替えが未送信（`filesDir`）を写し、設定（C で外す）を写さない、という外の振る舞いに乗る → R6。
  5 行目（401 で 1 件も取り除かない）は確かさ 2 だが、`SenderTest.kt`・`St50SenderTest.kt`・`sender.rs:1079` の試験があるので 4 まで安く上がる（低く付けているだけで、loss のある決定を誤らせる向きではない）

## R1. 配る APK で設定の前に積んだ記録は、設定の後に送ると恒久の拒否として捨てられる（Q3 の推奨の「飛ばしても記録は積まれ、設定した後に送られる」が成り立たない）
- 成果物: openspec/changes/st56-collector-connection-setup/deep-questions.json
- 根拠: `tools/release/build.sh:151`（配る APK の `USER_ID` が `""` であることを検査）/ `collector-android/.../Config.kt:21`（`userId` は `BuildConfig.USER_ID`）/ `LocationService.kt:373-387`（`onCreate` で `FixCollector(userId = Config.userId)` を作る。設定の有無を見ない）・`MainActivity.kt:101`（健康の権限の記録も同じ）・`Heartbeat.kt:35` / `IngestRequest.kt:54`（`user_id` は文字列のまま未送信の行に凍る）/ `crates/server/src/ingest.rs:18`（`user_id: uuid::Uuid`）・`lib.rs:1272-1278`（読めない項目は `malformed`）/ `Sender.kt` の `PERMANENT_ERRORS` に `"malformed"`・`LocationService.kt:938` の `dropPermanentlyRejected = true`。あわせて `LocationService.kt:926-931`（未設定なら送る仕組みを立てずに戻り、後から立て直す経路が無い）・`:241`（送り先と合言葉は立てた時の値で固まる）
- kind: premise
- loss: discarded
- 提案: Q3 の context に「いまは積んだ時点の利用者が行に凍り、配る APK では空なので、そのままだと設定の後に送った時点で全部捨てる」と書く。聞かない既定の「既定は nil の UUID」を「設定の前に積む行の利用者も nil の UUID に固定する（設定の文字列の値で書き換えない）」と明記して C にするか、それが決まるまでは Q3 を `irreversible`・`loss: discarded` に上げる。前提は Robolectric の試験（未設定で積んだ 1 件を設定後に送り、捨てないこと）で確かさ 4 にする。設定の後に送る仕組みを立て直すこと（サービスの起こし直しか、立てた後に設定を読み直す）も design に要ると context に一言添える

- 処置: escalated Q3 —— context に書き、推奨の前提として本人に見せる。spec の段が design の D と試験に写す

## R2. 利用者を設定の文字列に入れる既定は FR-92 と逆向きで、nil 以外が入ると画面に出ず鍵が割れる
- 成果物: openspec/changes/st56-collector-connection-setup/deep-questions.json（Q1 の選択肢 1 の detail「送り先・資格情報・利用者をまとめたもの」と、聞かない既定）
- 根拠: docs/requirements.md:390（FR-92「利用者識別子を、送られてきた内容ではなく資格情報から決める」）/ `crates/server/src/lib.rs:709-716`（収集アプリの合言葉 `Caller::ApiToken` は利用者を持たない）・`lib.rs:1676・1688`（中身の `user_id` が一意の鍵の一部）/ `lib.rs:2642・2705・3008`（読み出しは省けば `unwrap_or_default()` = nil）・`web/src/feelings.ts:251`（画面は nil で書く）・ST55 design D16（仮。取り込みも nil）
- kind: conflict
- loss: rewrite-all
- 提案: 文字列に利用者を入れない（携帯・PC とも nil に固定し、FR-92 が入ったらサーバが決める）を聞かない既定に書き直すか、入れるなら「nil 以外の利用者を貼ると、その記録は画面に出ず、後で揃えるには全行の利用者を書き直す」を選択肢に書いた問いを足す。Q1 の detail から「利用者」を外すかどうかもそれに合わせる

- 処置: escalated Q1 —— 推奨の detail に「利用者は入れない」と書いた

## R3. 配る PC の収集アプリの置き場と端末識別子の既定が一覧に無い（置き場を移すと除外の登録が外れ、除外したものが送られる）
- 成果物: openspec/changes/st56-collector-connection-setup/deep-questions.json（Q2）
- 根拠: `crates/collector-windows/src/config.rs:104-127`（`ASHIATO_STATE_DIR`・`ASHIATO_DEVICE_ID` は既定を持たない。R27: 置き場が黙って移ると「未送信・印・除外の登録が置き去りになり、除外が外れる」）/ `openspec/changes/st54-dev-side-by-side/design.md:36-45`（ST54 D2: 開発版は渡された置き場の下の `dev`・端末識別子に `-dev`・除外は渡された置き場から読む。自動起動の `.cmd` に書いた値に同じ規則がもう一度当たる）・`:47`（除外の登録が読めないと除外したサイトの閲覧履歴が過去の全件まで送られた）/ Q2 の推奨「置き場のファイルに置く」は、その置き場自体が設定の値で、`.cmd` に実行ファイルの場所だけを書くと次のログオンで置き場が渡らない
- kind: irreversible
- loss: exported
- 提案: 問いを足す（または Q2 に含める）: 配る形で設定と未送信を置く場所（例: 利用者のプロファイルの下の固定の場所）を誰が決めるか・端末識別子を誰が作るか（自動で作るなら入れ直しで別の端末になる）・ST54 D2 の `dev` と `-dev` の規則をその既定にどう当てるか。後から場所を変える選択肢には「除外の登録と未送信が置き去りになり、除外が外れて送られる」を不可逆として書く

- 処置: escalated Q8 —— irreversible・exported として足した

## R4. PC は設定が無いと起動ごと終わり、設定するまでの間を取っていない（Q3 の PC の兄弟が無い）
- 成果物: openspec/changes/st56-collector-connection-setup/deep-questions.json（Q2・Q3）
- 根拠: `crates/collector-windows/src/main.rs:14`（`Config::from_env()?` が欠けた変数で `Err` を返して終わる）/ `config.rs:5-6`（「取得は続ける」と書いているが成り立っていない）/ Q3 は携帯だけを問い、Q2 の推奨は「黒い窓で貼らせる」だけで、窓を閉じた・貼らなかった間に取るかを書いていない
- kind: irreversible
- loss: uncaptured
- 提案: Q3 の兄弟として PC の問いを足す（または Q2 に選択肢として足す）:「設定の前から前景の窓・入力・ブラウザを取って積む（利用者は nil・端末識別子と置き場は R3 の既定）」と「設定が済むまで取らない（その間の記録は取っていない）」。取る側に倒すなら R1・R3 が前提になると context に書く

- 処置: escalated Q2 —— context に「設定が無いままでも取って積む」を書いた

## R5. Q7 の「残っている記録」は記録の置き場だけを指し、送り先に結び付くほかの経路が入っていない
- 成果物: openspec/changes/st56-collector-connection-setup/deep-questions.json（Q7）
- 根拠: `LocationService.kt:643`（生存信号は別の置き場 `heartbeat.jsonl`）・`:603`（破棄の報告の置き場）/ `LocationService.kt:234`・`OriginalUploader.kt` の `RECORD_MISSING_GIVE_UP = 7 日`（原本の送り待ちは前の送り先が受けた記録に結び付き、新しい送り先では `409 record_missing` が 7 日続くと欠けにする）/ `LocationService.kt:249-254`（途絶の問い合わせ）と `openspec/changes/st54-dev-side-by-side/design.md:178`（`GET /outages` は相手のサーバの `core.outage_ledger` に `device_id` を書き、どの読む口も相手の `core.access_log` に 1 行残す）/ `FeelingReminderReceiver.kt:90`（気分の通知の問い合わせ）
- kind: irreversible
- loss: exported
- 提案: Q7 の context に、送り先を変えたときに止める範囲（記録・生存信号・破棄の報告・原本・読む口の問い合わせ）を並べ、選択肢 1 の「押すまで送らない」がどこまでを止めるかを書く。前の送り先が受けた記録の原本が新しい送り先で 7 日後に欠けになる（`discarded`）ことも、どの選択肢で起きるかを書く

- 処置: escalated Q7 —— context に止める範囲を並べ、原本の 7 日の欠けを選択肢に書いた

## R6. バックアップから戻した携帯では「前の送り先」の覚えが無く、Q7 の守りが効かない。送り先が同じかの判定も context の中で食い違う
- 成果物: openspec/changes/st56-collector-connection-setup/deep-questions.json（Q7）・openspec/changes/st56-collector-connection-setup/premises.md
- 根拠: `collector-android/app/src/main/res/xml/health_data_extraction_rules.xml`・`health_full_backup_content.xml`（外しているのは `health_permission.xml` だけで、未送信 `filesDir/outbox.jsonl`（`LocationService.kt:633`）と置き場 `outbox/` は既定どおり写る）/ 聞かない既定 C「設定をバックアップと端末の移し替えから外す」/ Q7 の context「初めての設定のときは前の送り先が無いので … そのまま送る」・「送り先の文字列と資格情報で見分ける（同じサーバの値を変えただけなら同じ送り先）」（資格情報を判定に入れると FR-133・ST55 の `repair` で値を変えたときに別の送り先と見え、入れなければ「文字列と資格情報で」と合わない。同じサーバを `127.0.0.1` と `*.ts.net` で開いたときの扱いも無い）/ premises.md の末尾「Q7 は外の形式に依らない」
- kind: irreversible
- loss: exported
- 提案: Q7 の context に「戻した端末・移し替えた端末は未送信だけを持ち前の送り先を持たないので、初めての設定と見分けられない」を書き、守りの条件を「前の送り先が無くても未送信があれば押すまで送らない」にするか、秘密でない送り先の印を未送信と同じ範囲に置くかを選択肢に入れる。同じ送り先かの判定（URL だけか・サーバが返す印か）を 1 つに決めて書く。自動バックアップと端末の移し替えが `filesDir` を写す前提を premises.md に足し、loss のある Q7 が乗るので確かさ 4（エミュレータで `bmgr` の backup / restore を通す台本）まで上げる

- 処置: escalated Q7 —— 推奨を「前の送り先が分からないまま未送信がある時も押すまで送らない」にし、同じ送り先の判定をアドレスだけにした。推奨はバックアップの写り方に依らないので premises.md の末尾を直した

## R7. 送り先の誤り（届かない）は 401 にならず、Q5 の知らせが出ない。PC の画面から作った文字列は携帯で自分自身を指す
- 成果物: openspec/changes/st56-collector-connection-setup/deep-questions.json（Q4・Q5）
- 根拠: docs/stories/ST56.md の完了の判定 3「設定を間違えた … ときに … 設定し直しを促す」/ premises.md 3 行目と聞かない既定「送り先は画面を開いている口から作る」/ `collector-android/app/build.gradle.kts:162-197`（携帯の平文の許可は `localhost`・`127.0.0.1`。PC で `http://127.0.0.1:18787` の画面から作った文字列を携帯に運ぶと、携帯は自分に送り続ける）/ `ContractGate.kt:27`（届かないは一時的な失敗）/ FR-9 (3)（requirements.md:200。知らせは 83 日目の 1 回）/ Q4 の推奨は届かないときに「保存しました。繋がったら送ります」と見せる
- kind: daily
- loss: discarded
- 提案: Q5 の兄弟として「一度も繋がったことの無い送り先に届かないまま N 日続く」ときに知らせるかを B（既定つき）で足す。携帯に貼った文字列の送り先が loopback なら受け付けない（PC の画面から作った文字列を携帯に貼った誤りを貼った時点で止める）ことを聞かない既定に足すか、Q4 の選択肢 1 の detail に書く

- 処置: escalated Q5 —— 推奨に「一度も届かないまま 24 時間で知らせる」「携帯は loopback の送り先を貼った時点で断る」を足した

## R8. 配る APK では通知が開く画面のアドレスが空で、毎日の気分の通知を押しても何も開かない。設定の文字列に入っていない
- 成果物: openspec/changes/st56-collector-connection-setup/deep-questions.json（Q1）
- 根拠: `collector-android/.../Config.kt:26`（`webUrl` は `BuildConfig.WEB_URL` だけ）/ `build.gradle.kts:105-108`（空のままでも組み立てを止めない）/ `FeelingNotification.kt:52`（空なら押した先を付けない）/ premises.md 3 行目（配る形は画面と API が同じ口なので、送り先から画面のアドレスを作れる）
- kind: daily
- 提案: 問いは要らない。聞かない既定に「画面のアドレスは設定の送り先から `/api` を外して作る（`-P ashiato.webUrl` は初期値としてだけ残す）」を足し、Q1 の detail の「まとめたもの」の中身に合わせる

- 処置: escalated Q1 —— 推奨の detail に「通知が開く画面のアドレスも送り先から作る」を足した

## R9. Q2 の推奨「環境変数があれば先に使う」と、既にある自動起動の台本の扱いが無い
- 成果物: openspec/changes/st56-collector-connection-setup/deep-questions.json（Q2）
- 根拠: `crates/collector-windows/src/autostart.rs:27-39`（`--install-autostart` が 5 つの変数を合言葉ごと `.cmd` に平文で書く）・`config.rs:61-67`（読む変数）/ docs/stories/ST56.md の完了の判定 2「合言葉が平文の台本に残らない」/ FR-133（requirements.md:1126）・ST55 spec distribution:297-299（`repair` で合言葉が変わる）: 古い `.cmd` が残ると、環境変数が先なので `setup` で設定し直しても古い合言葉で送り続け、401 が止まらない（PC は未送信に上限が無いので記録は失わない。`outbox.rs:4`）
- kind: technical
- 提案: Q2 の選択肢 1 の detail に「既にある `ashiato-collector.cmd` は、新しい設定を置いた時点で実行ファイルの場所だけの形に書き換える」と「環境変数を先に使うのは開発と試験の印（明示の変数か引数）があるときだけ」を書く。問いを増やさず C にしてよい

- 処置: fixed deep-questions.json —— Q2 の推奨の detail に、古い .cmd の書き換えと環境変数を先に使う条件を書いた

## R10. Q6（呼び名）は FR-106 が利用者に見せる文を決めていて、もう一方の選択肢は要件の本文を書き換える。人に聞かなくてよい
- 成果物: openspec/changes/st56-collector-connection-setup/deep-questions.json（Q6）
- 根拠: docs/requirements.md:982（FR-106 ★ 2026-11-03「文は『送り先が開発版／正式版のサーバです。送り先を確かめてください』の形」）/ Q6 の選択肢 2 の detail「FR-106 の文を書き換える」/ docs/handoff/ST56.md の 3 つ目の箇条（どれに揃えるかを決める、とだけ言う）
- kind: defer
- 提案: Q6 を外し、聞かない既定に「利用者に見せる語は FR-106 の『送り先』に揃える（要件の『接続先』は本文の語のまま）」を足す。申し送りの項目を処置したことは deep.md に残す

- 処置: fixed deep-questions.json —— Q6 を外した。利用者に見せる語は FR-106 の「送り先」に揃える ——下の C6

## R11. Q1 の context は ST55 の「秘密は人が見ない」を狭く読んでおり、ST55 が ST56 に預けた手順の置き場も書いていない
- 成果物: openspec/changes/st56-collector-connection-setup/deep-questions.json（Q1）・openspec/changes/st56-collector-connection-setup/premises.md（4 行目）
- 根拠: `openspec/changes/st55-server-installer/proposal.md:24`（「秘密は自動で作り、人は見ない … 画面にも記録にも出さない」。API の合言葉を含む。インストーラの画面に限っていない）/ ST55 design D18（仮。:154-158 収集アプリを繋ぐのは ST56 から）・D27（:222）・spec distribution:299・tasks 11.1（:169。`repair` の後の収集アプリの繋ぎ直しを `docs/install.md` に書く）/ Q1 の context「走っている ST55 は『秘密は画面にも記録にも出さない』と書いている（インストーラの画面のこと）」
- kind: premise
- 提案: Q1 の context を「ST55 は API の合言葉を人が見ない値として作る。選択肢 1・2 はそれを『画面にログインして合言葉を入れ直した本人には見せる』に変える（ST55 の proposal の 1 行と揃える申し送りが要る）」に直す。どの選択肢でも、ST55 の `docs/install.md` の繋ぎ直しの節を ST56 が書き足すと context に添える
- 処置: fixed deep-questions.json —— Q1 の context を直した。ST55 への申し送りと install.md の繋ぎ直し

## 聞かない既定（C）——問い直しのあとで足したもの。spec の段は design の D に写す

- C1: 記録の利用者は、設定の前も後も既定の利用者（nil の UUID）に固定する。設定の文字列に利用者を入れない。設定した後、送る仕組みを立て直す（サービスの起こし直しを待たない）
- C2: PC の収集アプリは設定が無いままでも起動し、取って積む。送るのは設定の後
- C3: 携帯の設定（送り先・資格情報）は、Google のバックアップと端末の移し替えから外す。組み立ての時の `-P` の値は、保存した設定が無いときの初期値として残す（試験と計測テストを壊さない）
- C4: 通知が開く画面のアドレスは、送り先から `/api` を外して作る（`-P ashiato.webUrl` は初期値）
- C5: http の送り先は PC は loopback だけ（今の規則）。携帯は貼った文字列の送り先が loopback なら断る（PC の画面から作った文字列を携帯に貼った誤り）
- C6: 利用者に見せる語は FR-106 の「送り先」に揃える（要件の本文の「接続先」は変えない）。申し送りの 3 つ目の問いはこれで処置した
- C7: 設定の文字列の送り先は、画面を開いている口から作る（配る形は画面と API が同じ口。premises.md 3 行目）
