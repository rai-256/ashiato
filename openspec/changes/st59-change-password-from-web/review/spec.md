# ST59 spec の独立レビュー（`spec --draft`。深掘り第 1 回の推奨を答えとして読んだ）

対象: `openspec/changes/st59-change-password-from-web/` の proposal.md・specs/data-sensitivity/spec.md・design.md・tasks.md・premises.md・deep.md・deep-questions.json・review/deep.md（C1〜C7）、
`docs/stories/ST59.md`・`docs/stories/INDEX.md`・`docs/requirements.md`（FR-128・FR-133）、正典 `openspec/specs/`（data-sensitivity・distribution）。
コードは `crates/server/src/{web_session,lib,service}.rs`・`crates/installer/src/{main,commands,secrets,password}.rs`・`web/src/SettingsView.tsx`・`web/e2e/global-setup.ts`・`web/playwright.config.ts`・`tools/{ports,agent-env,stack}.sh`・`tools/release/install-check.sh` を読んだ。
成果物には触っていない。

## 機械の検査

- `openspec validate st59-change-password-from-web --strict` → `Change 'st59-change-password-from-web' is valid`
- `python3 scripts/check_chain.py .` → `chain: OK (0 件 / 未回収 0 件 / warn 0 件)`（観点 8 の再生成との一致を含む）
- `python3 scripts/check_scenarios.py . st59-change-password-from-web` → `scenarios: FAIL (担保なし 24 件 / 名無しの確認待ち 0 件)`。24 件は全部この change で足した Scenario（ADDED 22・MODIFIED で足した 2）。実装の前なので担保が無いのは想定どおり（Task 9.1 が閉じる）。指摘にはしない。
  spec の `#### Scenario:` の各名を `grep -cF` で tasks.md に当て、足した 24 本がどれもどこかの Task に名指されていることを確かめた（漏れ 0）。MODIFIED の写しの 16 本は ST28・ST29・ST44 の印のまま（tasks.md:17）
- `python3 scripts/review_triage.py . st59-change-password-from-web` → `triage: FAIL (4 件)`: `deep.md R1・R5・R8・R9: escalated なのに deep.md に R<n> が無い` → R1
- `python3 scripts/spec_overlap.py candidates . st59-change-password-from-web` → `[]`（0 件）
- MODIFIED の 2 本を正典（openspec/specs/data-sensitivity/spec.md:109・:201）と `diff` した。違いは ST59 の足し分（本文 1 文・★ 1 節・Scenario 1 本ずつ）だけで、正典の文と Scenario を落としていない

## R1. deep.md に、escalated した review/deep.md の R1・R5・R8・R9 の出所が無く、triage が落ちている
- 成果物: openspec/changes/st59-change-password-from-web/deep.md
- 根拠: `python3 scripts/review_triage.py . st59-change-password-from-web` → `[FAIL] deep.md R1 / R5 / R8 / R9: escalated なのに deep.md に R<n> が無い` / review/deep.md:43（R1 → escalated Q2）・:72（R5 → Q2）・:94（R8 → Q2）・:102（R9 → Q1）/ deep.md:5-18 は問いと答えだけで `出所` の行が無い / 前例: openspec/changes/archive/2026-11-09-st56-collector-connection-setup/review/spec.md の R1（同じ FAIL を各 Q に `- 出所: review/deep.md R<n>` を書いて閉じた）
- kind: technical
- 提案: deep.md の各 Q に `- 出所: review/deep.md R<n>` を書く（Q1 ← R2・R9、Q2 ← R1・R4・R5・R8・R13、Q3 ← R11）。record が答えで書き換えても消えない位置に置き、triage を走らせ直す
- 処置: fixed deep.md —— Q1〜Q3 に `- 出所: review/deep.md R<n>` を書いた（Q1 ← R2・R9、Q2 ← R1・R4・R5・R8・R13、Q3 ← R11）

## R2. 変えた後に読み出しの記録の行を書けないと、変えたことが記録に残らない（D9（仮））。失う側を AI が選んでいる
- 成果物: openspec/changes/st59-change-password-from-web/design.md
- 根拠: design.md:105-106（D9: 変えた行は差し替えの**後**に書き、書けなければ取り消さず、変えた結果を返し、サーバのログに 1 行だけ残す。「行が 1 本欠ける」と自分で書いている）/ review/deep.md:41-43（R1 は `loss: uncaptured`。「残さなかった期間に、拾った人が合言葉を試した・変えた跡は後から足せない」）/ specs/data-sensitivity/spec.md:58・:78（Requirement の本文は例外なしの「1 件ごとに 1 行残す」、★ は「後から足せない」）/ openspec/specs/data-sensitivity/spec.md:201 の Requirement の「WHEN 読み出しの記録に行を書けない THE SYSTEM SHALL その読み出しの求めに記録を返さず、500 で断る」/ ST56 の `/collector-setup` は `ok` の行を書けなければ値を返さない（crates/server/src/web_session.rs:665-676 の `.await?` が値を返す前にある）。`/collector-token` は新しい値を返すので、D9 のままだと「値を渡したのに行が無い」が起きる / deep-questions.json の Q2 の context は本人に「変えた求めと断った求めは 1 求め 1 行で残す」とだけ見せている
- kind: irreversible
- loss: uncaptured
- 提案: 失わない側に倒す。`LOGIN_CHECK` を持ったまま、書き戻しの**前**に `ok` の行を書く。書けなければ何も変えずに 500（正典と ST56 と同じ）。書き戻しが後で失敗したら、`forbidden`・500 の行を足す（打ち消しの行。D9 の反転条件に既にある形）。
  P の見立て: 失わない側は順を入れ替えるだけで、移行も新しい語も要らない（安い）。範囲は ST59 の 2 つの口の中に収まる。要件は変わらない（spec:58 の本文どおりになる）。行が多い側（成功しなかった `ok` の後に 500 の行）に倒れるだけで、失うものは無い。P の条件を満たすので人間に問わず、D9 を直して台帳（provisional.jsonl。source は `review/spec.md#R2`）に載せ、`fixed D9 仮` で閉じればよい（review_triage.py:20-23 の「loss がある + 台帳に載った仮」の閉じ方）。サーバのログの 1 行は、ログの保存期間・回転があり追記のみでもないので、読み出しの記録の代わりにならない
- 処置: fixed D9 仮 —— 変えた行（ok）を書き戻しの前に書き、書けなければ何も変えずに 500、書き戻しで落ちたら打ち消しの行を足す形にした。spec の「誰がいつ読んだかを…」の本文に写し、provisional.jsonl に載せた（source: review/spec.md#R2）

## R3. 変える最中に飛んでいた求めの応答が、寿命の出し直しで古い印を付け直し、変えた画面がログアウトしうる
- 成果物: openspec/changes/st59-change-password-from-web/design.md
- 根拠: crates/server/src/web_session.rs:465-481（`refresh_cookie` は求めの**最初**に `decide` で印を認め、応答に `Set-Cookie` が無ければ、その求めが持ってきた**同じ印の値**を寿命を延ばして付け直す）/ design.md:36-37（D1: 1 つの求めは組を最初に 1 回だけ取る。古い組で認めた求めは古い組のまま終わる）/ design.md:131-133（D12 は変えた求めの応答の付け直しだけを扱い、ほかの求めを扱わない）/ web/src/SettingsView.tsx:27・:87-101（書き出しが走っている間、設定の画面は 1 秒ごとに状態を読む。同じブラウザのほかのタブも同じ印を送る）/ specs/data-sensitivity/spec.md:170-173（変えた画面はログインの画面に戻らない）
- kind: technical
- 提案: 差し替えより前に認められ、差し替えの後に返る応答は、変えた応答の新しい印を古い印で上書きする。次の求めは 401 になり、画面はログアウトする（Q1 の推奨に反する。失うものは無いが、Scenario は運しだいで落ちる）。
  D1 か D12 に「寿命の出し直しは、応答を返す時点で今の組の `tag` に合う印にだけ付ける」（または変えた後の古い印には付けない）を書き、Task 1.1 の試験に「差し替えをまたいだ読み出しの応答が古い印を付け直さない」を足す
- 処置: fixed D1 —— 寿命の出し直しは返す時点の組の tag に合う印にだけ付ける、を D1 と spec の本文に書き、Scenario「変える前に始まった求めの応答は、変えた画面の新しい印を上書きしない」と Task 1.1 の試験を足した

## R4. 読み出しの記録に残すと決めた断りのうち、Scenario が見るのは 403 と成功だけ。429・401・400・409・500 の行と、その結果の語は design にしか無い
- 成果物: openspec/changes/st59-change-password-from-web/specs/data-sensitivity/spec.md
- 根拠: review/deep.md:136（C1: 成功・今の合言葉の違い・回数の断り・ログインが無い・ログインでない資格情報を 1 求め 1 行）/ spec:58（本文は「変えたもの・断ったもの」）/ spec:131-135（唯一の Scenario は「違う今の合言葉で 1 回」と「正しい今の合言葉で 1 回」だけ）/ design.md:103-104（D9: 401 → `unauthorized`、429 → `login_throttled`、403・400・409・500・415 → `forbidden` と実際の状態）/ tasks.md:73（Task 4.2 も 403 と成功の行だけを見る）
- kind: technical
- 提案: 拾った人の総当たりの跡は 429 の行に出る（design.md:140）。少なくとも「回数の制限を超えた求めは `login_throttled`・429 の行で残る」と「ログインの印の無い求めは 401 の行で残る」を Scenario にする。400・409・500 は「状態の列が実際に返した状態と一致する」を 1 本でまとめてよい。結果の語の対応（D9）は観測できる振る舞いなので、spec の本文に写す（正典の「実際に返した応答のとおりに書く」と同じ書き方）。「変えた行（成功）」は語（`ok`）か状態（204 / 200）で書く
- 処置: fixed specs/data-sensitivity/spec.md —— 結果の語と状態の対応を「誰がいつ読んだかを…」の本文に写し、Scenario「合言葉と収集アプリの値を変える求めの断りは、返した状態のとおりに残る」を足した。成功の行は `ok`・204 / 200 と書いた（Task 4.2）

## R5. 観測できる振る舞いが design にしか無い（archive で正典から落ちる）
- 成果物: openspec/changes/st59-change-password-from-web/design.md
- 根拠:
  - design.md:40-41（D1）: 変えた画面の印の行を書けなければ、合言葉は変えたまま `Set-Cookie` なしで 204。spec:148 は例外なしに「新しい合言葉に結び付いた印を渡す」と言っている
  - design.md:34（D1・C2）: 失敗の数え（`limiter`）は差し替えで数え直さない。spec にも Scenario にも無く、Task 1.1 の試験（tasks.md:36）だけが持つ
  - design.md:68（D4）: 検査の順。特に「今の合言葉が合わない求めには、新しい合言葉の検査の結果を返さない」と「開発の形は合言葉を照らす前に 409」。spec:144 は 400 の理由を並べるだけで、403 との優先を言っていない（Scenario:175-178 は正しい今の合言葉でしか送らない）
  - design.md:82（D6・C6）: 案内の「携帯に入れる文字列は、携帯から開いた画面で『収集アプリの設定』から出してください」。spec:226 と Scenario:247 は「全部の収集アプリに貼り直す案内」だけ。review/deep.md:122 の R12 が足させた注意で、`127.0.0.1` で開いた画面の文字列は携帯が断る（ST56 D5・D7）
  - design.md:114-115（D10（仮））: 本文が JSON と名乗らない求めは 415（よそのサイトの単純な送信を受けない）。spec に無い
- kind: technical
- 提案: 上の 5 つを spec の本文に写し、少なくとも D4 の「今の合言葉が違えば、新しい合言葉が検査に当たっても 403」と D6 の案内の文を Scenario にする（前者はサーバの試験、後者は Task 6.1 の e2e で見られる）。D1 の 204 は R2 と同じ考えで「印を書けなければ何も変えずに 500」に倒すかを決めてから写す
- 処置: fixed specs/data-sensitivity/spec.md —— 印の行を書けなければ何も変えずに 500（D1 を失わない側に直した）・limiter を数え直さない・断る順・415・携帯から開いた画面の案内を本文に写し、Scenario「今の合言葉が違えば、新しい合言葉が検査に当たっても…」「合言葉を変えても、失敗の数えは数え直されない」「本文が JSON でない求めでは、何も変わらない」を足し、案内の文を Scenario の THEN に入れた

## R6. 「失敗が回数の制限を超えると…」は回数が書かれておらず 2 通りに読め、「収集アプリの設定」の口との共有を確かめていない
- 成果物: openspec/changes/st59-change-password-from-web/specs/data-sensitivity/spec.md
- 根拠: spec:143（本文: 失敗の数えをログインの口・収集アプリの設定の文字列を出す口・収集アプリの値を変える口と共有）/ spec:184-187（GIVEN「ログインの口で 1 回、2 つの変える口で合わせて回数の制限まで」。「合わせて制限まで」が 2 つの口だけで 10 回なら、ログインの口を数えに入れない実装でも 429 になる。合計で 10 回（1 + 9）なら共有を確かめている。どちらかが書かれていない）/ 前例の試験は 1 + 9 で読んでいる: crates/server/src/st56_tests.rs:216-237・web_session_tests.rs:536-549（`for _ in 0..9` の後にログインで 1 回）/ openspec/specs/data-sensitivity/spec.md:373（制限は 1 分 10 回）/ `/collector-setup` は Scenario に出てこない
- kind: technical
- 提案: GIVEN を数で書く（「ログインの口で 1 回、`/collector-setup` で 1 回、2 つの変える口で合わせて 8 回、違う合言葉で失敗した（合計 10 回）」）。これで spec:143 が挙げる 4 つの口の共有を 1 本で確かめられる
- 処置: fixed specs/data-sensitivity/spec.md —— GIVEN を「1 分のうちに、ログイン 1・収集アプリの設定 1・合言葉を変える 4・収集アプリの値を変える 4（合わせて 10 回）」に書き直した（Task 4.2）

## R7. 検証できない・空で通る Scenario がある
- 成果物: openspec/changes/st59-change-password-from-web/specs/data-sensitivity/spec.md
- 根拠:
  - spec:204-207「変えている最中に前の合言葉で照合が通ったログインの印は、変えた後は使えない」: THEN が「ログインが印を受け取ったなら」の条件付き。ログインが印を受け取らない（例: 500 や 401）実装でも空で真になる。そのときログインに何が返るかを言っていない
  - spec:199-202「画面の合言葉を変えると、設定のファイルは照らす値だけが変わる」: WHEN に変える操作が無い（「変える前と後の設定のファイルを比べる」だけ）。対の spec:273-276 は「収集アプリの値を変える前と後」と操作を書いている
  - spec:131-135: 4 行（2 つの口 × 断り・成功）とログに値が無いことを 1 本に束ねている。「値が無い」を見ない試験でも、行の数だけで緑になりうる
- kind: technical
- 提案: 1 本目は THEN を「ログインは印を受け取らずに断られる（状態は X）か、受け取った印での読み出しが 401 になる。どちらでも、新しい合言葉に結び付かない印は通らない」と 2 つの場合を名指すか、D1 の順（照合した組の `tag` で書く）から結果を 1 つに決める。2 本目は WHEN に「画面の合言葉を変える」を足す。3 本目は「値が残らない」を別の Scenario に分ける
- 処置: fixed specs/data-sensitivity/spec.md —— 1 本目の THEN を D1 の順から「ログインは 204 で印を受け取り、その印での読み出しは 401」に決め、2 本目の WHEN に「画面の合言葉を変え」を足し、3 本目を行の Scenario と値の Scenario に分けた

## R8. Requirement の本文だけが言っていて Scenario が言っていないもの
- 成果物: openspec/changes/st59-change-password-from-web/specs/data-sensitivity/spec.md
- 根拠:
  - spec:146: 書けないときの画面の文「保存できませんでした。合言葉は変わっていません」。Scenario:194-197 は口の 500 だけを見る。Task 6.1 の e2e（tasks.md:88-91）もこの文を名指さない
  - spec:152・:235: 入れた合言葉・照らす値・新旧の値を**応答**（と応答の見出し）に出さない。Scenario:135 は読み出しの記録とサーバのログだけを見る
  - spec:236: 「画面を開き直す」と消える。Scenario:283-286 は「別の画面へ移って戻る」だけ
- kind: technical
- 提案: 1 つ目は e2e の Scenario にする（書けなくするのは Task 5 の台本で設定のファイルの置き場を読みだけにすれば足りる）。2 つ目は Scenario:135 か別の Scenario に「応答の本文と見出しのどこにも、今の合言葉・新しい合言葉・照らす値が無い（`/collector-token` の成功の本文の新しい値を除く）」を足す。3 つ目は WHEN に「読み込み直す」を足す
- 処置: fixed specs/data-sensitivity/spec.md —— Scenario「設定のファイルに書けないと、画面は合言葉が変わっていないと出す」（e2e）・「…値は、記録にもログにも応答の見出しにも残らない」（応答の見出しと本文を見る）・「読み込み直すと、新しい値の設定の文字列は消える」を足し、Task 4.2・6.1 に置いた

## R9. premises 4 行目（収集アプリは 401 で記録を捨てない）を上げる Task が無く、完了の判定 2 の後半はこの Story のどこでも確かめない
- 成果物: openspec/changes/st59-change-password-from-web/premises.md
- 根拠: premises.md:6（「3 行目はサーバの試験、1・2・5 行目は CI の Windows runner で 4 に上げる」。4 行目が挙がっていない）/ premises.md:13（4 行目は確かさ 2）/ review/deep.md:106-108（R10: 4 行目は collector-android の `St56SetupNotifierTest.kt:75-76` と `crates/collector-windows/src/runtime.rs:1815` の `st56_runtime_unauthorized_keeps_pending_records` を走らせれば 4 に上がる）/ tasks.md:38-39（Task 1.2 はサーバの `st56_` だけを走らせる）/ docs/stories/ST59.md:39（完了の判定「古い値の収集アプリは送れなくなり、記録を捨てずに設定し直しを促される」）
- kind: technical
- 提案: Task 1.2（または新しい Task）に、R10 が挙げた 2 本を `-- --list` の本数つきで走らせる検証を足し、premises.md:6 に 4 行目を足す。ST59 は収集アプリを変えないので壊す見込みは低いが、完了の判定の後半の担保をどこに置くかを tasks に書く
- 処置: fixed 1.3 —— Task 1.3 に PC と携帯の 401 で記録を捨てない試験を本数つきで置き、premises 4 行目を確かさ 4 に上げる手順にした

## R10. D5 は「外れても失うものが無い」と「戻り道が無くなるので P にしない」を両方書き、Task 7.1 の終わりの条件に CI の結果が無い
- 成果物: openspec/changes/st59-change-password-from-web/design.md
- 根拠: design.md:74（「外れても失うものが無い」）と :76（「直せなければ A として人間に返す（忘れたときの戻り道が無くなるので P にしない）」）/ premises.md:14（5 行目は確かさ 2。「サーバが作ったファイルに届くかは未確認」）/ crates/installer/src/commands.rs:461-465（`temporary_write_paths` のコメントは、settings を「インストーラだけが書く置き場で、中のファイルの持ち主もインストーラ」と前提している。ST59 でサーバも書くので、この前提が外れる）/ tasks.md:103（Task 7.1 の検証は `--self-test` と `--list` の本数だけで、実地は「PR の CI の `install-windows`」と括弧に書くだけ。手元の検証が通れば、CI を見ずに [x] を付けられる）
- kind: technical
- 提案: D5 の「失うものが無い」を消し、外れたときに失うもの（画面で変えた後に合言葉を忘れると、`password` の口が使えない。出口は `repair` の条件（設定のファイルが無い。openspec/specs/distribution/spec.md:1733-1736）を満たすために管理者として `server.json` を退けることだけで、そのとき共有の値と DB の合言葉も作り直しになる）を書く。Task 7.1 の終わりの条件に「PR の CI の `install-windows` が緑で、3 本の `== Scenario:` の行がログにある」を足す。`commands.rs:461-465` のコメントが ST59 の後も正しいかを Task 7.1 で直すことも書く
- 処置: fixed D5 —— 「失うものが無い」を消し、外れたときに失うもの（password が使えず、出口は server.json を退けて repair するだけ）を書いた。Task 7.1 の終わりの条件に CI の install-windows の緑とログの 3 本を足し、commands.rs のコメントを直すことを書いた

## R11. 設定のファイルを指す台本は、`.env` の `WEB_PASSWORD` を受け継ぐと起動を断られる
- 成果物: openspec/changes/st59-change-password-from-web/tasks.md
- 根拠: tasks.md:78-80（Task 5.1: 設定のファイルの鍵を環境へ入れ、`ASHIATO_SERVICE_MODE=1` を付けて起こす。ほかの環境変数を外すとは書いていない）/ crates/server/src/lib.rs:4511-4516 と web_session.rs:150-156（照らす形では `WEB_PASSWORD` が環境にあると `RawPasswordPresent` で起動しない）/ web/e2e/global-setup.ts:15-16（e2e は `WEB_PASSWORD` が環境に無いと落ちる。つまり e2e の process は必ず `WEB_PASSWORD` を持つ）/ tools/agent-env.sh:46-51（ハーネスは `.env` の値を agent の環境に渡す）/ design.md:92-95（D8（仮）も同じく書いていない）
- kind: technical
- 提案: Task 5.1 に「`up` は `WEB_PASSWORD` を外し（`env -u WEB_PASSWORD` か、設定のファイルの鍵と `PATH` などだけの `env -i`）、`--self-test` で `up` が組む環境に `WEB_PASSWORD` が無いことを確かめる」を足す
- 処置: fixed 5.1 —— 台本の `up` は `WEB_PASSWORD` を環境から外し、`--self-test` で無いことを確かめる。D8 にも書いた

## R12. tasks の検証の細部
- 成果物: openspec/changes/st59-change-password-from-web/tasks.md
- 根拠:
  - tasks.md:39（Task 1.2）: `cargo test -p ashiato-server web_session` などの 4 本に `-- --list` の本数の確かめが無い（AGENTS.md「`cargo test <名前>` 型は `-- --list` で 1 本以上あることも確かめる」。今の本数は `web_session` 16・`st55_` 47・`st56_` 7・`ch15_` 8 を grep で数えた。名前を変えると 0 本で rc=0 になる）
  - tasks.md:58（Task 3.1）: Scenario「画面から合言葉を変えると、起動し直さずにほかのログインが使えなくなる」の試験の括弧書きは「変えた求めの印は 401、応答の `Set-Cookie` の印は通る」。Scenario（spec:38-39）が見るのは **2 つ目のブラウザ**の印が 401 になることで、括弧書きはそれを名指していない
- kind: technical
- 提案: Task 1.2 の 4 本に、Task 1.1 と同じ形の `test "$(… -- --list | grep -c ": test$")" -ge <今の本数>` を足す。Task 3.1 の括弧書きに「2 つ目のログインの印が 401」を足す（変えた側の古い印の 401 は足したままでよい）
- 処置: fixed 1.2 —— 4 本に今の本数の下限を付けた。Task 3.1 の括弧書きに「2 つ目のログインの印での読み出しが 401」を書いた

## R13. 正典 distribution の「作った秘密を…画面…に出さない」と、画面に共有の値を出す口が食い違ったまま。ST59 はサーバが作る秘密を増やす
- 成果物: openspec/changes/st59-change-password-from-web/proposal.md
- 根拠: openspec/specs/distribution/spec.md:1636-1637（「THE SYSTEM SHALL 作った秘密を状態の置き場の設定にだけ置き、画面・端末の出力・ログ・記録・状態の置き場の外のファイルに出さない」。この行は WHEN の付かない別の SHALL）/ proposal.md:49（「インストーラが DB を初めて作るときの文で…食い違わない（共有の値を画面に出すのは ST56 から）」）/ archive の st56 は distribution に delta を持たない（`ls openspec/changes/archive/2026-11-09-st56-collector-connection-setup/specs/` → data-sensitivity・desktop-collection・device-collection）/ spec:226-227（ST59 はサーバが新しい秘密を作り、画面に出す）
- kind: technical
- 提案: 食い違いは ST56 からあり、ST59 が広げる。distribution は ST59 の割り当ての外なので、proposal の「食い違わない」を「ST56 から食い違っている。ST59 では直さない」に改め、`docs/handoff/` か INDEX の訂正の節に「distribution の秘密の Requirement に、共有の値は本人の画面にだけ出す例外を足す」を申し送る（前倒しにしない）
- 処置: fixed proposal.md —— 「食い違わない」を「ST56 から食い違い、ST59 が広げる。直さず申し送る」に改め、Task 8.2 で docs/handoff/ST60.md に例外を足す申し送りを書くことにした（前倒しはしない）

---

## 前提の実物での確かめ（premises.md）

- 1 行目（サーバのアカウントは `settings\server.json` を書ける）: crates/installer/src/commands.rs:422-440（`acl_commands`: 根に `/inheritance:r` の後 `<server>:(OI)(CI)M` と本人の `RX` だけ）で、`settings` は根から書きを継ぐ。D2 の「一時のファイルは `settings` の権限を継ぎ、ACL は今の `server.json` と同じ」もこれで成り立つ（main.rs:653-656 のコメントも同じことを言う）。実物の Windows では未確認（Task 7.1 が `icacls` を比べる）
- 2 行目（同じフォルダの付け替えで置き換わる）: Rust の文書だけ。実物では未確認
- 3 行目（照らす値を変えると印が全部通らない）: crates/server/src/web_session.rs:242-249（`secret_tag_for_verifier` は鍵と照らす値（塩を含む）の HMAC）と :441-460（`session_valid` は `secret_tag = app.login.tag` の行だけを通す）で確かめた。新しい照らす値は塩が変わるので、同じ合言葉に変えても印は全部通らなくなる。差し替えの後は `app.login.tag` を差し替え後の組から読む必要がある（D1）。寿命の出し直しの穴は R3
- 4 行目（収集アプリは 401 で記録を捨てない）: 正典と ST56 の試験の名前だけ。走らせていない（R9）
- 5 行目（インストーラの `password` がサーバの書いた `server.json` に届く）: crates/installer/src/commands.rs:461-465 のコメントの前提が ST59 で外れ、:575-604 の `takeown /A /R` と特権の ACE で届くかはコードからは決まらない（R10）

## 観点 1: deep の決定が正典に写っているか

- Q1（推奨: ほかの端末は全部切り、変えた画面だけ残す）→ spec:8（MODIFIED の本文）・:36-39・:170-173 に写っている。前の正典の「変わった後の起動で」は spec:7 に残り、手で書き換えた・`password` のときの文として ★（spec:18-19）が範囲を分けている。食い違いは無い
- Q2（推奨: 2 つの節を別々に置く）→ spec:141・:226・:153 と Scenario:219-222 に写っている
- Q3（推奨: 設定のファイルを指したサーバでだけ。開発の形は文を出して断る）→ spec:145・:230 と Scenario:189-192。断りの文は deep.md:18 と一字一句同じ
- 聞かない既定 C1〜C7 → D9・D1・D2・D3・D4・D6・D7 に写っている（proposal.md:12・tasks.md:20）。spec への写しの不足は R4（C1）・R5（C2 の数え直さない・C6 の案内）
- 数値: 本人の決定（Q1〜Q3）に数値は無い。C5 の 16 文字は Scenario:177（15 文字で 400）にある。回数の制限の数（1 分 10 回）は R6
- 要件へ戻すもの: 無い（review/deep.md・deep.md のどちらにも挙がっていない）

## 観点 2: Scenario が検証可能か

R6・R7・R8 に書いた。それ以外の Scenario は、状態符号・本文の理由・画面の文・設定のファイルの鍵ごとの比べ・読み出しの記録の行で真偽が決まる。

## 観点 3: 置き場の誤り

- design の D に置いた観測できる振る舞い: R2（D9）・R3（D1/D12）・R4（D9 の語）・R5
- specs の実装の名前: 該当なし（確かめた範囲: spec の全文を読んだ。出てくるのは口の路（`POST /web-password`・`POST /collector-token`。正典にも `POST /collector-setup` がある）・断りの本文・インストーラの `password` の口・`.env` で、どれも外から観測できる名前。関数名・crate 名・列名は無い）
- proposal の Capabilities と specs/ のディレクトリ: どちらも `data-sensitivity` だけで一致。INDEX.md:103 も ST59 を `data-sensitivity` にだけ割り当てている。前倒しは無い（distribution の食い違いは R13）

## 観点 4: tasks が検証を持つか

R9・R10・R11・R12 に書いた。依存の順（Task 1 の組 → Task 2 の書き戻し → Task 3・4 の口 → Task 5 の台本 → Task 6 の e2e → Task 7 の CI）は、鍵（組）を変えるタスクが先にあり、逆順は無い。
人間の確認待ち: tasks.md:119-121 が「無い」と書き、Windows の実物は CI の `install-windows` に置いている（その終わりの条件は R10）。

## 観点 5: Story が要件の現在の本文と一致しているか

- `check_chain.py` の観点 8（再生成との一致）は通っている
- ST59.md の価値・完了の判定（:38-40）は Q1〜Q3 の推奨・C1〜C7 と矛盾しない。完了の判定 2 の後半（記録を捨てずに促される）の担保の置き場は R9
- satisfies は FR-133 だけで、specs が満たそうとしている要件は FR-133 と、その帰結として本文が引く FR-128（ST56 の満たし方をそのまま使う）。前倒しは無い

## 観点 6: 答えが写ったか

該当なし（mode は `spec --draft`。本人の答えはまだ無い）

## 観点 7: 走っている change との前提関係

観点 7: 該当なし（spec_overlap.py candidates が 0 件）

## 失う側のある問い（loss）

- 新しく見つけたもの: R2（D9。`loss: uncaptured`）。失わない側（行を書き戻しの前に書き、書けなければ何も変えない）は順の入れ替えだけで安く、範囲は ST59 の口の中、要件は変えない（spec:58 の本文どおりになる）。**P の条件を満たす**ので人間に問わず、D9 を失わない側に直して台帳に載せればよい
- review/deep.md の R1・R8・R9（loss あり）は、要件 FR-128・正典・C1 で失わない側に決まっていて、Q1・Q2 の context で本人に見せている（R1 で、triage がその出所を読めるようにする）。R5（`loss: exported`。衛星の資格情報が残る）は今は衛星の求めがどれも断られるので害が無く、ST38 へ申し送り済み（docs/handoff/ST38.md:84-87）。spec:242 にも書いてある
