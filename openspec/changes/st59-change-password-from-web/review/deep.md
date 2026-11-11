# ST59 深掘りの問いの独立レビュー（deep-review）

対象: `openspec/changes/st59-change-password-from-web/deep-questions.json`（Q1〜Q6。どれも `open` で `loss` 無し）と `premises.md`（4 行。どれも確かさ 2）。
手順 1〜5 は、問いの一覧を見る前に要件・申し送り（`docs/handoff/ST59.md`）・正典の spec・既存コードだけでやった。そのあとで問いの一覧と突き合わせた。問いの JSON は触っていない。
走らせたのは grep と `scripts/upstream_misses.py report` だけ。cargo / gradle / Windows の上では何も走らせていない。

## 手順ごとの確かめ

- **手順 1（要件どうしの衝突）**: FR-133（requirements.md:1126-1129）・FR-128 と ★ 2026-11-02（:1100-1108）・FR-109 / FR-110 / FR-111（:1006-1019）・FR-8（:184-193）と、正典の data-sensitivity（:109-150・:201-273・:577-602）・distribution（:1581-1640・:1844-1860）を読んだ。
  (a) data-sensitivity:113 は「合言葉が変わった**後の起動で**、それまでの印を無効にする」と書いている。FR-133 で起動中に変えるなら、いつ効くかの文が要る → R2。
  (b) FR-128 ★ 2026-11-02 は「共有の 1 本・発行と取り消しの画面は作らない・締め出すには値を変えて全部に設定し直す・設定し直すまで全部の端末が送れない」を本文で決めている。Q3・Q5 はこれで決まっている → R7・R8。
  (c) data-sensitivity:113・:119 は「合言葉を変えると全部のログインを無効にする」を決めている。Q1 の選択肢 3 はこれと逆 → R9。
  (d) data-sensitivity:203 の「読み出しの記録に残すもの」の一覧に、合言葉と共有の値を変える求めが無い → R1
- **手順 2（扉の幅）**: 該当なし（確かめた範囲: docs/stories/ST59.md:5 の `doors: []` と、「触れる扉」の節が空であること）
- **手順 3（新たに立つ一方通行）**:
  (a) 外に出すもの: 変えた後の新しい共有の値は画面に出る（ST56 と同じ口）。落とした携帯のログインから発行された衛星の資格情報は、締め出した後も残る → R5。
  (b) 取っていないもの: 変えた・変えようとして断られたことの記録 → R1。猶予の間に古い値で届いた記録の出所 → R8。
  (c) 保存の形と書き方: 変えた値をどこへ・どの順で書くか。書きかけで落ちた設定のファイルには、インストーラからの出口が無い → R3。設定の 1 か所（`server.json`）をインストーラとサーバの 2 者が書くことになる → R4
- **手順 4（日常に影響する選択）**: 変えた後に全部の端末でログインし直すこと（Q1）・変えたあと全部の収集アプリに貼り直すこと（Q2・Q4）は一覧にある。電池・容量・常時通知は該当なし（確かめた範囲: この Story は新しい常駐・周期の問い合わせ・通知を足さない。古い値で送り続ける収集アプリの 401 は今どおり 1 求め 1 行で読み出しの記録に残る（`access_log.rs:158-213`）が、ST44 から変わらない振る舞いなので数えない）
- **手順 5（既存コードが要件を満たしていない箇所）**: FR-133 を `web_session.rs`・`lib.rs`・`service.rs`・`main.rs`（サーバ）・インストーラの `secrets.rs`・`main.rs`・`commands.rs`・`state.rs`・`password.rs` と突き合わせた。
  (a) 変える口も画面も無い（`lib.rs:4423` の `/collector-setup` が最も近い口。`grep -rn "web-password\|collector-token" crates/server/src web/src` で該当なし）。サーバが `server.json` を書くコードも無い（`service.rs` は `load_settings`（:218）で読むだけ）。
  (b) 共有の値と照らす値は `App` が値で持ち、起動の時に 1 回だけ組む（`lib.rs:470-474`・`:625-627`・`:4676-4695`）。差し替える道が無い → R2。
  (c) 設定のファイルが壊れていると、サーバは起きず、インストーラの走らせ直し・`password`・`repair` のどれも通らない → R3。
  (d) 照らす形では `WebLogin.password` が空（`web_session.rs:360`）。新しい値を今の値と比べる検査は、照らす値を通さないと必ず外れる → R6
- **手順 6（過去の取りこぼし）**: `upstream_misses.py report` の「拾えたはずの観点」49 件を 1 つずつ当てた。当てはまったのは 5 つ。
  「『送らずに残すので失わない』を根拠にするときは、携帯の 90 日・2 GB を挙げて、上限の前に本人へ原因が届くかを確かめる」（ST50 Q10）→ Q3 の why の「どちらも記録は失わず」→ R8。
  「秘密をファイルに置く決定では、鍵ごとに読む者と書く者を表にする」（ST55 Q11）→ `server.json` を書く者が 2 者になる → R4。
  「止まる側の design に『出口が無い』が出るなら P にしない」（ST55 Q8）→ 書きかけの `server.json` → R3。
  「照らす形では login.password が空のまま残り、合言葉そのものと比べる所が必ず外れる」（CH15）→ R6。
  「2 つの口の照合を 1 つの道にしたことで、依頼の外で変わる振る舞い」（CH15）→ 3 つ目の口を足すと、照合の 1 つずつの印と失敗の数えを共有する口が増える。差し替えの競り合いに含めた → R2。
  時計の族・取り込みの量・外部 DB の欠けた行・検証コマンドの族は、この Story に時刻で状態を動かす決定も取り込みも無いので該当なし
- **手順 7（族の網羅）**: Q3（古い共有の値をいつまで受けるか）の兄弟は「古い画面の合言葉・古い印をいつまで受けるか」（Q1 が受けている）と「変えている最中に走っている照合・ログイン」（一覧に無い → R2）。Q6（設定のファイルが無い形）の兄弟は「設定のファイルはあるが書けない・書きかけで落ちた・ほかのプロセスが掴んでいて名前を付け替えられない」（一覧に無い → R3）と「Linux の配る形（FR-131。サービスの形がまだ無い）」（R11 に含めた）
- **手順 8（前提の確かさ）**: premises.md は「`loss` のある決定はこの Story に無い」として 4 行とも確かさ 2 のまま。ところが R1・R8・R9 で、失う側を持つ問いが 3 つある。推奨（失わない側）は 3 行目と 4 行目の前提に乗るので、4 に上げる → R10。インストーラが、サーバが書き直した `server.json` を書けるかは前提の表に無い → R4

---

## R1. 合言葉・共有の値を変えた（変えようとして断られた）ことを、読み出しの記録に残すかが問いに無い
- 成果物: openspec/changes/st59-change-password-from-web/deep-questions.json
- 根拠: openspec/specs/data-sensitivity/spec.md:203（残すものの一覧は「読み出し・ログイン・ログアウト・資格情報の無い求め・衛星・API の合言葉で断った求め」で、変える求めが無い）/ crates/server/src/access_log.rs:158-167・:178-186（`/healthz` `/session` `/collector-setup` 以外の口は、ハンドラの**前**に `ok`・200 の行を書く。新しい口をここに通すと、今の合言葉が違って 403 で断った求めも `ok` の行で残る）/ crates/server/src/web_session.rs:529-543・:636-676（ST56 の `/collector-setup` は素通しにして、ハンドラが成功・合言葉違い・回数の断りを結果どおりに 1 行ずつ書いている）
- kind: irreversible
- loss: uncaptured
- 提案: 問いを 1 つ足すか、失わない側（変える口ごとに口の名前を分け、成功・今の合言葉の違い・回数の断りを `/collector-setup` と同じ形で 1 求め 1 行。値は持たない）を P として台帳に載せる。残さなかった期間に、拾った人が合言葉を試した・変えた跡は後から足せない。data-sensitivity:203 の一覧に足す MODIFIED が要ることも書く
- 処置: escalated Q2 —— Q2 の context に「変えた求めと断った求めは読み出しの記録に 1 求め 1 行で残す（値は残さない）」と書き、推奨の前提として本人に見せる。失わない側を C1 に置いた。spec の段は data-sensitivity の「残すもの」の一覧を MODIFIED で足す

## R2. 変えた値がいつ効くか（起動中に差し替えるか、サービスを起こし直すか）と、差し替えの最中の照合の扱いが問いに無い
- 成果物: openspec/changes/st59-change-password-from-web/deep-questions.json
- 根拠: crates/server/src/lib.rs:470-474・:625-627・:4676-4695（共有の値 `token` と `WebLogin`（照らす値・印の材料 `tag`・失敗の数え `limiter`）を `App` が値で持ち、起動で 1 回だけ組む）/ openspec/specs/data-sensitivity/spec.md:113（「変わった後の**起動で**」無効にする）/ crates/installer/src/commands.rs:800-818（回復は異常終了と `failureflag 1` で 10 秒後に起こし直すだけで、サーバが自分を起こし直す口は無い）/ crates/server/src/web_session.rs:786・:817-822（ログインは照合の後に `app.login.tag` を読んで印の行を書く。差し替えを共有の可変の置き場にすると、古い合言葉で照合が通った後・行を書く前に差し替わったとき、新しい `tag` の有効な印を古い合言葉の人に渡しうる）
- kind: technical
- 提案: Q1 の「拾われた携帯は次の操作で戻る」と Q3 の「変えた瞬間から断る」は、起動中の差し替えを黙って前提にしている。前提として context に書くか、B の問い（推奨: 設定のファイルに書けた後にだけ差し替える。照合と印の発行は同じ版の (照らす値, tag) を 1 つ取って使う。`limiter` は差し替えで数え直さない）を足す。data-sensitivity:113 の MODIFIED（「変わったときに」）が要ることも書く
- 処置: fixed deep-questions.json —— Q1 の context に「設定のファイルに書けた後に、止まらずにその場で差し替える」と書き、差し替えの規則を C2 にした。data-sensitivity の「変わった後の起動で」の MODIFIED は spec の段で書く

## R3. 設定のファイルへの書き戻しが途中で落ちた・書けなかったときの扱いが問いに無く、壊れた `server.json` にはインストーラからの出口が無い
- 成果物: openspec/changes/st59-change-password-from-web/deep-questions.json
- 根拠: crates/server/src/service.rs:218-235・crates/server/src/main.rs:20-24（読めない・JSON でない `server.json` ではサーバが起きず、終了コード 0 で止まったまま）/ crates/installer/src/main.rs:632-635・:1059-1060・:1420（走らせ直しと `password` は `read_settings` で落ちる）/ crates/installer/src/state.rs:54-56・main.rs:1453-1466（`repair` は設定のファイルが**ある**と「要りません」で断る）/ crates/installer/src/main.rs:637-645（今の書き方は一時のファイル → 名前の付け替えで、書いた中身をディスクへ流す段が無い）/ crates/server/src/service.rs:265-282（サービスは `server.json` に無い 3 つの置き場を環境に補う。環境から書き戻すと、ファイルに無かった鍵が増える）
- kind: technical
- 提案: B の問い（または C の既定）を足す。推奨の候補: 今のファイルを読み直して 2 つの鍵だけを置き換える（`secrets::replace_web_password_verifier` と同じ形）・一時のファイルを書いてディスクへ流してから付け替える・書けなければ値を変えずに断る（差し替えはしない）。書きかけで残ったときに、インストーラの `password` か `repair` が出口になるか（`repair_allowed` の条件）も決める。止まっている間に携帯の未送信が 90 日に向かって積み上がる（FR-8）ので、出口が無いままにしない
- 処置: fixed review/deep.md —— C3 にした（今のファイルを読み直して 2 つの鍵だけ置き換え・一時のファイルをディスクへ流してから付け替え・書けなければ値を変えずに断り差し替えない）。付け替えなので書きかけの server.json は残らない。残った一時のファイルを `repair` / `password` がどう扱うかは spec の段で D に書く

## R4. `server.json` をサーバとインストーラの 2 者が書くことになる。インストーラの `password`・走らせ直し・`repair` との関係が、問いにも前提にも無い
- 成果物: openspec/changes/st59-change-password-from-web/premises.md
- 根拠: crates/installer/src/main.rs:1417-1442（`password` は `server.json` の照らす値だけを書き換えて起こし直す）/ main.rs:1087-1101（走らせ直しは設定が無いときだけ書くので、画面で変えた値は残る —— 確かめた）/ main.rs:1500-1505（`repair` は `new_install_secrets()` で共有の値も作り直す。全部の収集アプリに貼り直しになる）/ crates/installer/src/commands.rs:462-465・:570-608（一時の書きは「中のファイルの持ち主がインストーラ」を前提に書かれ、`takeown /A /R` で持ち主を戻す。サーバのアカウントが付け替えで作った `server.json` に、インストーラの書きが届くかは実物で見ていない）
- kind: premise
- 提案: premises.md に行を足す —— 「サーバのサービスのアカウントが書き直した `server.json` を、インストーラの `password` と走らせ直しが読み書きできる」（確かめ方: Windows の CI で、サーバに書かせた後に `password` を走らせる）。値の置き場は `server.json` 1 つに限る（DB などに別に持つと、インストーラの `password` の決め直しが黙って効かなくなる）を C として書く。`repair` が共有の値も作り直すことを Q2 か Q4 の context に足す
- 処置: fixed premises.md —— 5 行目に足した（確かさ 2。CI の Windows で 4 に上げる）。値の置き場を server.json 1 つに限ることを C4 にし、`repair` が共有の値も作り直すことを新しい Q2 の context に書いた

## R5. 落とした携帯のログインから発行された衛星の資格情報は、合言葉と共有の値を変えても残る
- 成果物: openspec/changes/st59-change-password-from-web/deep-questions.json
- 根拠: crates/server/src/satellite_admin.rs:84・:361（衛星の資格情報の発行は画面のログインだけで通り、今の合言葉を入れ直させない）/ satellite_admin.rs:4・:427（取り消しは 1 本ずつ）/ 衛星の資格情報は照らす値にも共有の値にも結び付かない（`web_session.rs:243-249` の `tag` の材料に入らない）/ data-sensitivity:201 の ★ ST29「衛星の求めは承認（ST38）までどの口でも断る」/ `ls openspec/changes/` に ST38 の change が無い（tasks.md がまだ無い）
- kind: defer
- loss: exported
- 提案: 今は衛星の求めがどれも 403 なので害は無いが、ST38 が読みを開いた後は、締め出しの後も拾った人が発行した 1 本で読める。ST38 へ `deferred ST38`（衛星の資格情報の発行に今の合言葉の入れ直しを要るか・締め出しのときに衛星の資格情報の一覧を見せるか）とするか、ST59 の変えた後の案内に「衛星の資格情報も確かめる」を足すかを決める
- 処置: escalated Q2 —— Q2 の context に「衛星の資格情報は変わらず、拾った携帯から発行された 1 本は残る。今は害が無く ST38 へ申し送る」と書いて本人に見せる。docs/handoff/ST38.md に申し送りを足した

## R6. 新しい値の検査を起動の検査と揃えないと、次の起動でサーバが起きなくなる。照らす形で今の値と比べる所は必ず外れる
- 成果物: openspec/changes/st59-change-password-from-web/deep-questions.json
- 根拠: crates/server/src/lib.rs:4514-4549（照らす形の起動は、照らす値が API の合言葉に当たる・ログインの印の鍵が画面の合言葉に当たると、起動しない）/ crates/installer/src/password.rs:44-63（インストーラの決め直しは 16 文字・`change-me`・API の合言葉・ログインの印の鍵・2 回の一致の 5 つを見る）/ crates/server/src/web_session.rs:356-369（照らす形の `WebLogin` は `password` が空。`token_matches(.., &login.password)` で比べる所は必ず外れる。CH15 で同じ型が出た）
- kind: technical
- 提案: 問いは要らない（C）。design に「新しい画面の合言葉はインストーラの 5 つと同じ検査、新しい共有の値は照らす値とログインの印の鍵に当たらないことを照らす値で確かめる。起動の検査に落ちる値は書かない」を書く。Q2 の選択肢 2 の detail（「画面の合言葉…と違うことを検査する」）も、照らす値を通す形で書く
- 処置: fixed review/deep.md —— C5 にした

## R7. Q5 は不要。共有の 1 本で、落とした端末は値を変えて全部に設定し直すことは FR-128 の本文で決まっている
- 成果物: openspec/changes/st59-change-password-from-web/deep-questions.json
- 根拠: docs/requirements.md:1104-1107（FR-128 ★ 2026-11-02。ST44 の深掘り Q2 で、推奨（端末ごと）を本人が覆して「共有の 1 本・発行と取り消しの画面は作らない・締め出すには値を変えて全部に設定し直す」を選んだ）/ Q5 の選択肢 2 自身が「要件 FR-128 を変える」と書いている
- kind: technical
- 提案: Q5 を外す。要件を変える選択肢は Story の中の B ではない。もし聞き直すなら /requirements の改訂として出す（ST44 で本人が答えた問いの聞き直しになることも書く）
- 処置: fixed deep-questions.json —— 旧 Q5 を外した（FR-128 ★ 2026-11-02 で本人が決めている。GO 資料の「やらないこと」に出す）

## R8. Q3 は FR-128 の本文で決まっており、猶予の側には失うものがある。why の「どちらも記録は失わず」は誤り
- 成果物: openspec/changes/st59-change-password-from-web/deep-questions.json
- 根拠: docs/requirements.md:1106-1107（「締め出すたびに全部の端末が設定し直すまで送れず」—— 猶予を置かないことは本文の帰結として本人が受け入れている。同じ行に「落とした端末の値で送られた偽の記録は後から見分けられず」）/ crates/server/src/lib.rs:774（共有の値の照合は一致するかだけで、どの値で届いたかを行に持たない）/ requirements.md:184-193（FR-8。猶予の無い側でも、貼り直すまでの未送信は 90 日の上限に掛かる）
- kind: irreversible
- loss: uncaptured
- 提案: Q3 を外す（猶予なしは FR-128 で決まっている）。残すなら、選択肢 2（24 時間受ける）に `loss: uncaptured`（猶予の間に拾った携帯から入った偽の記録は、どの値で届いたかを残さないので後から分けられない）を書き、選択肢 1 の detail に FR-8 の 90 日を書く。why の「どちらも記録は失わず」を消す
- 処置: escalated Q2 —— 旧 Q3 を外した（猶予なしは FR-128 ★ 2026-11-02 で本人が選んだ）。Q2 の context に「変えた瞬間から断る・90 日を越えた分は捨てる・偽の記録は見分けられない」を FR-128 の帰結として書き、本人に見せる。C7

## R9. Q1 の選択肢 3（ログインを残す）は、正典の spec と逆で、失うものがある
- 成果物: openspec/changes/st59-change-password-from-web/deep-questions.json
- 根拠: openspec/specs/data-sensitivity/spec.md:113（合言葉が変わったら全部の印を無効にする）・:119（「端末を落としたときに、その端末のログインを締め出す手段は…画面の合言葉を変えることだけ」）/ Q1 の選択肢 3 の detail 自身が「拾われた携帯は記録を読み続けられる」と書いている
- kind: irreversible
- loss: exported
- 提案: 選択肢 3 を外す（決まっている spec に逆らう選択肢で、選ぶと拾った人に記録が読まれ続ける）。残る 1 と 2 の違い（変えた本人が入れ直すか）は日常の B なので、`kind: open` のままでよい。note の「この回の問いはどれも…失うものが無い」も、選択肢 3 を外さない限り誤り
- 処置: escalated Q1 —— Q1 の選択肢 3（ログインを残す）を外し、context に「合言葉を変えたら全部のログインを無効にすることは今の仕様で決まっている」と書いて本人に見せる

## R10. 失う側を持つ問いの推奨が、確かさ 2 の前提に乗っている。premises.md の「loss のある決定は無い」は誤り
- 成果物: openspec/changes/st59-change-password-from-web/premises.md
- 根拠: premises.md 1〜4 行目（どれも確かさ 2、冒頭に「失われるもののある決定はこの Story に無い」）/ R1・R8・R9（失う側がある）/ 3 行目（照らす値を変えると全部の印が通らない）は Q1 の推奨、4 行目（収集アプリは 401 で記録を捨てない）は Q3 の推奨の土台 / 走らせれば確かめられる試験がある: collector-android/app/src/test/kotlin/dev/ashiato/collector/St56SetupNotifierTest.kt:75-76（`401 で断られた記録は取り除かない`）・crates/collector-windows/src/runtime.rs:1815（`st56_runtime_unauthorized_keeps_pending_records`）。携帯の原本の送りも 401 で行を残す（OriginalUploader.kt:340-353 の `else -> Step.Retry`）
- kind: premise
- 提案: 冒頭の文を直し、3・4 行目をその試験を走らせて確かさ 4 に上げる（3 行目は `crates/server/src/st55_tests.rs:69` の `st55_verifier_changes_session_tag_material` と `web_session_tests.rs:341` の `web_session_lifetime_password_change_revokes_all`。後者は元の値の形の試験なので、照らす形で印が全部失効する試験が無ければ足す。どちらも `cargo test <名前> -- --list` で 1 本以上あることを確かめてから走らせる）。R4 の前提の行を足す
- 処置: fixed premises.md —— 冒頭の文を直した。失う側の論点はどれも要件・正典・C で失わない側に決まっていて、人間に問う 3 問に loss は無いので、確かさ 2 のまま問いを立て、下流の試験で 4 に上げると書いた

## R11. Q6 の推奨（開発の形では断る）を選ぶと、完了の判定の画面の Scenario を `web/e2e` で担保できない。そのことが選択肢に無い
- 成果物: openspec/changes/st59-change-password-from-web/deep-questions.json
- 根拠: web/playwright.config.ts:49-50（e2e のサーバは `tools/stack.sh up`）/ tools/stack.sh:35（開発の形。`WEB_PASSWORD` の元の値）/ web/e2e/global-setup.ts:15（全部の spec が同じ `WEB_PASSWORD` でログインする。共有のサーバで変えると他の spec が落ちる）/ AGENTS.md「画面の Scenario は `web/e2e`（本物のブラウザ）で担保する」/ crates/server/src/main.rs:7-28（設定のファイルを読む `service` の口は Windows だけ）/ requirements.md の FR-131（Linux の配る形にはまだサービスの形が無い）
- kind: technical
- 提案: Q6 の context と選択肢 1 の detail に、「この形の e2e は、設定のファイルから起こした別のサーバ（または変えた値を戻す段）が要る。Linux の配る形でも画面から変えられない」を書く。開発の形でも起動中だけ差し替える選択肢 2 は、e2e で担保できる代わりに起こし直すと戻る。その違いが選ぶ理由になるので、選択肢に書く
- 処置: fixed deep-questions.json —— 新しい Q3（旧 Q6）の推奨を「設定のファイルを指して起こしたサーバでだけ変えられる」にし、context に e2e は設定のファイルを指した別のサーバで行うこと・Linux の配る形の状況を書いた

## R12. Q2 は C で決めてよい。context の「貼り直しても保留にはならない」は、画面を開いたアドレスによっては成り立たない
- 成果物: openspec/changes/st59-change-password-from-web/deep-questions.json
- 根拠: openspec/specs/distribution/spec.md:1636-1639（共有の値を含む秘密は OS の暗号の乱数で作り、固定の値を使わない）/ requirements.md:1104-1105（人が覚える値ではなく、設定の文字列で貼る）/ openspec/changes/archive/2026-11-09-st56-collector-connection-setup/design.md:73-78（D7: 設定の文字列の送り先は画面を開いているアドレスから作る。`127.0.0.1` で開けば携帯は D5 で断る）・:178-182（D19: 別のアドレスは別の送り先）・:123-135（D13: 未送信があるまま送り先が変わると押すまで止める。値を変えた後は 401 で未送信が必ず積まれている）
- kind: technical
- 提案: Q2 を外し、「サーバが乱数で作る」を C にする（選択肢 2 は今の spec の秘密の作り方から外れる）。変えた後の案内に D7 の注意（携帯に入れるなら携帯から開いているアドレスで出す）を足し、PC は別のアドレスの文字列を貼ると 1 回押すことになる、と context を直す
- 処置: fixed deep-questions.json —— 旧 Q2 を外し、C6 にした（サーバが OS の乱数で作る）。案内の注意も C6 に書いた

## R13. Q4 は画面の並び（設定の画面に何を同時に置くか）を文字で問うている
- 成果物: openspec/changes/st59-change-password-from-web/deep-questions.json
- 根拠: Q4 の 3 つの選択肢は、設定の画面（web/src/SettingsView.tsx、ST56 の web/src/CollectorSetupPanel.tsx の隣）に置く欄の数と並びの違い / Q1 の推奨と組み合わせると、「2 回操作する」の 2 回目は新しい合言葉で入れ直す（Q1 の選択肢 2 なら、間にログインし直しも挟まる）。この順は選択肢に書かれていない
- kind: daily
- 提案: `kind: visual` にして、設定の画面の 3 つの並びを playground の HTML（`proto`）で見せる。2 つを続けて変えるときに何回合言葉を入れるかを、Q1 の答えごとに選択肢の detail に書く
- 処置: rejected: 選択肢は画面の上の並び方ではなく、操作が 1 回か 2 回かの違い。ST59 には ui-direction の面が無い（`python3 scripts/story_brief.py ST59 --screens` が `[]`）。部品は今の設定の画面の節（web/src/CollectorSetupPanel.tsx:59 と同じ節の形）を足すだけ。Q1 との組み合わせ（2 回目に入れる合言葉・ログインし直し）は新しい Q2 の選択肢の detail に書いた

## 聞かない既定（C）—— 処置で足したもの。spec の段は design の D に写す

旧 Q2・Q3・Q5 を外したので、新しい問いは Q1（旧 Q1）・Q2（旧 Q4）・Q3（旧 Q6）。

- C1: 画面の合言葉を変える口と、収集アプリの値を変える口は、読み出しの記録に別々の口の名前で残す。成功・今の合言葉の違い・回数の断り・ログインが無い・ログインでない資格情報を、`/collector-setup` と同じ形で 1 求め 1 行。値も照らす値も持たない（R1）
- C2: 新しい値は、設定のファイルに書けた後にだけ、起動し直さずにその場で差し替える。照合と印の発行は、同じ版の（照らす値・印の材料）を 1 つ取って使う（古い合言葉で照合が通った求めに新しい印を渡さない）。ログインの失敗の数えは差し替えで数え直さない（R2）
- C3: 書き戻しは、今の `server.json` を読み直して 2 つの鍵（`WEB_PASSWORD_VERIFIER` / `API_TOKEN`）だけを置き換え、同じフォルダに一時のファイルを書いてディスクへ流してから名前を付け替える。書けなければ値を変えずに断り、差し替えない（R3）
- C4: 値の置き場は `server.json` の 1 つに限る（DB などに別に持たない。インストーラの `password` の決め直しが今どおり効く）（R4）
- C5: 新しい画面の合言葉は、インストーラの決め直しと同じ検査（16 文字以上・`change-me` でない・収集アプリの値とログインの印の鍵に当たらない・2 回の入力の一致）。新しい収集アプリの値は、照らす値とログインの印の鍵に当たらないことを確かめる。起動の検査に落ちる値は書かない（R6）
- C6: 新しい収集アプリの値は、サーバが OS の暗号の乱数で作る（distribution の秘密の作り方と同じ。本人は入れない）。変えた直後に「収集アプリの設定」と同じ形の設定の文字列を出し、案内に「携帯に入れる文字列は携帯から開いた画面で出す」（ST56 D7）を添える（R12）
- C7: 古い収集アプリの値は、変えた瞬間から断る（FR-128 ★ 2026-11-02）。端末ごとの値にはしない（同じ ★。要るなら /requirements から）（R7・R8）

