# ST28 成果物の独立レビュー（proposal / specs / design / tasks / ST28.md）

書いた文脈を持たない目で、`deep.md` の決定（Q1〜Q6・C1〜C7）→ `specs/` → `design.md` → `tasks.md` の写りと、正典・要件・既存コード・並走中の change との整合だけを見た。
**成果物は 1 つも触っていない。** 網の名前・網のアドレスはこのファイルに書いていない。

確かめた範囲: `openspec/changes/st28-private-network-only/**`（proposal / deep / deep-answers-1.txt / review/deep.md / specs/data-sensitivity/spec.md / design / tasks）、
`openspec/specs/record-envelope/spec.md`（:221-238 資格情報、:328-341 記録の門）、
`docs/requirements.md`（§1.4 / PERM-7 / PERM-10 / NFR-15 / §5 技術的制約）、`docs/stories/ST28.md` / `INDEX.md` / `ST01.md` / `ST29.md`、`docs/handoff/ST29.md`、
`crates/server/src/{lib.rs,testdb.rs}`・`crates/server/Cargo.toml`・`Cargo.toml`、`crates/collector-windows/src/config.rs`、
`web/{vite.config.ts,playwright.config.ts,package.json,e2e/}`、`collector-android/app/build.gradle.kts`、
`tools/{stack,dev,smoke,verify-prep,check-immutable,check-private,android-emulator}.sh`、`docker-compose.yml`、`.env.example`、`.github/workflows/ci.yml`、`scripts/merge_gate.sh`、
並走中の `feat/st06-app-usage` / `feat/st08-browser-history` / `feat/st12-archive-ingestion` / `feat/st22-record-deletion` の `main...` との差分、
vite 7.3.6 の preview の middleware の順（`web/node_modules/vite/dist/node/chunks/config.js`。別の作業ツリーの同じ版で読んだ）。

## 機械の検査（人間の目より先に）

```
$ openspec validate st28-private-network-only --strict
Change 'st28-private-network-only' is valid                              rc=0

$ python3 scripts/check_chain.py .
要件 118 件 / Story 36 本 / 扉 26 項
[ok] どれかの Story に拾われた要件: 116/118 件
chain: OK (0 件 / 未回収 0 件 / warn 0 件)                                rc=0   ← 観点 8（stories.json からの再生成との一致）も通っている

$ python3 scripts/check_scenarios.py . st28-private-network-only
Scenario 477 件 / 印 516 個 / 担保あり 433
scenarios: FAIL (担保なし 44 件 / 名無しの確認待ち 0 件)                   rc=1

$ python3 scripts/review_triage.py . st28-private-network-only
指摘 12 件 / 仮決め 0 件 / 要件へ戻すもの 3 件
triage: OK                                                               rc=0
```

`check_scenarios` の 44 件は**上流では正常**（`scripts/merge_gate.sh:99-127` が上流では見ない。テストがまだ 1 本も無い）。
44 = この change の Scenario 数（`grep -c '#### Scenario:' specs/data-sensitivity/spec.md`）で、`tasks.md:19` の自己申告と一致した。
44 本すべての名前が `tasks.md` のどこかに一字一句で現れることも機械で確かめた（取りこぼし 0。2 か所に現れるものが 3 本 —— R10）。

---

## R1. C1 の「許した網の内側なら起動する」経路が、Q2 の「網の内側から届く口は暗号化された接続だけ」「サーバと画面は 127.0.0.1 から動かない」と食い違い、その経路は C2 の検査も通る
- 分類: 決定どうしの食い違い（deep → specs）
- 成果物: openspec/changes/st28-private-network-only/specs/data-sensitivity/spec.md:9-44・:46-75
- 根拠: deep-answers-1.txt:3（Q2「サーバと画面は 127.0.0.1 から動かない」）/ deep.md:69（Q2 の効く先 specs「網の内側から届く口は暗号化された接続だけ … サーバと画面は loopback で待ち受ける（C1）」）/
  spec.md:12・:41-44（`許した網の内側のアドレスなら起動する` —— 網のアドレスに平文の http でサーバが直に口を開けることを**正しい振る舞い**として固定している）/
  spec.md:49（検査が落ちるのは「loopback でも許した網の内側でもない」口だけ）・:51（平文で落ちるのは「**網の手段が**」平文で出しているときだけ）→ 網のアドレスに直に開けた平文の口は、(a) にも (b) にも当たらず**検査が通る** /
  design.md:140-142（D11 の (a) は「loopback か `ALLOWED_NETS` の内側か」、(b) は `tailscale serve` の設定だけ）/
  tools/stack.sh:37 のコメント「本番のサーバが Tailscale の IP:18787 で動いている」—— まさにこの形が既にあった（いまは serve 経由。review/deep.md:18）/
  spec.md:113（印は「暗号化された接続でだけ送られる」）→ 直の平文の口では画面のログインが成り立たず、API の合言葉だけが平文で網へ出る。
  spec.md:84 の理由（「網の手段を差し替えたときに記録が平文で流れる」）が、収集側には当てているのにサーバ側の口には当たっていない
- kind: conflict
- 提案: どちらかに揃える。(a) Q2 を正とし、「網の内側から届く本システムの口は暗号化された接続だけ」を Requirement にして、網のアドレスで直に待ち受ける平文の口を C2 の検査で落とす Scenario を足す（C1 の `ALLOWED_NETS` は、暗号化を自前で持つ口のためだけに残すか、消す）。(b) C1 を正とするなら、直の平文の口が Q2 の答えと違う理由を deep.md に書き、本人に見せる。
- 処置: fixed D7 仮 —— 厳しい側（本人が Q2 で選んだ「127.0.0.1 から動かない」「網の口は暗号化だけ」）に揃えた。`ALLOWED_NETS` を消し、サーバは loopback 以外で起動しない。spec の最初の Requirement を「loopback でしか待ち受けない」に書き直し（Scenario 3 本）、C2 の検査は loopback 以外の口をすべて落とす。反転条件は D7（loopback へ暗号化して中継できない網の手段を使うことになったら）。proposal の What Changes に C1 から変えたことを書いた

## R2. Q6「期限なし（ログアウトするまで）」を成り立たせる cookie の寿命（D18）が design にしかない。ブラウザを閉じると消える印でも全 Scenario が通る
- 分類: 置き場（design に観測可能な振る舞い）/ 決定が Scenario に写っていない
- 成果物: openspec/changes/st28-private-network-only/design.md:183-187（D18（仮））・specs/data-sensitivity/spec.md:179-201
- 根拠: deep.md:122-126（Q6 の答えと効く先「cookie の寿命」）/ spec.md:188-191（`既定では日が経ってもログインは切れない` は**サーバ側の判定**だけを撃つ。tasks.md:109 も `App::at()` で時刻を差し込む単体）/
  spec.md のどこにも「印がブラウザを閉じても残る」「使うたびに延びる」が無い（`grep -n "400\|閉じ\|延" spec.md` → 190 行の「400 日後」だけ）/
  `Max-Age` を付けない（session cookie）実装は、spec の全 Scenario を通したまま、**ブラウザを閉じるたびにログインを失う** —— 本人が推奨（30 日）を覆して選んだ日常の形が、正典に残らない。
  `openspec archive` は main specs しか更新しないので、D18 の「400 日・使うたびに延ばす」も正典から落ちる
- kind: technical
- 提案: spec の Q6 の Requirement に「ログインの印は、ブラウザを閉じても残る寿命で覚えさせ、印で認めた応答のたびに寿命を延ばす」を SHALL で置き、Scenario を 1 本（ログイン応答と、印で認めた読み出しの応答の `Set-Cookie` が持続の寿命を持つ / ブラウザの文脈を作り直しても印が残る）。400 日という上限は D18（仮）に残してよい。
- 処置: fixed specs/data-sensitivity/spec.md —— 「ブラウザを閉じても残る寿命で覚えさせ、印で認めた応答のたびに延ばす」を SHALL にし、Scenario `ログインの印はブラウザを閉じても残り、使うたびに延びる`（2 つの応答の `Max-Age` が 1 日以上）を置いた。400 日の上限は D18（仮）に残した。tasks 4.4

## R3. 「ログインの口は資格情報なしで受ける」が specs に無く、正典の record-envelope「すべての API 要求に資格情報を要求する」とぶつかる。ログインの口の応答の形も design にしかない
- 分類: 置き場 / 正典との食い違い
- 成果物: openspec/changes/st28-private-network-only/proposal.md:65-69（Modified Capabilities なし）・design.md:44-54（D2）・specs/data-sensitivity/spec.md:107-117
- 根拠: openspec/specs/record-envelope/spec.md:223-224「THE SYSTEM SHALL **すべての API 要求に**、呼び出し元を識別できる資格情報を要求する。… 欠くか一致しない要求を 401 で拒否する」/
  design.md:53「`POST /session` は `authorize()` を通さない唯一の口」—— ログインの口は資格情報を欠いた要求を 401 にせず、一致すれば 204 を返す。archive 後の正典は record-envelope と data-sensitivity で**反対のことを言う**（どちらを読んだ実装者も「相手は例外」と読めない）/
  proposal.md:67-69 は「record-envelope の文面を変えない」とする /
  design.md:48-50 の応答（`204` + `Set-Cookie` / 不一致は `401`・本文 `unauthorized` / `GET /session` の `200 {"credential": …}`）は画面が依存する観測可能な契約（design.md:52）だが spec に無い
- kind: conflict
- 提案: data-sensitivity に「画面の合言葉でログインする口だけは、資格情報の無い求めを受ける（ほかの口は record-envelope のとおり）」を SHALL で置き、record-envelope の当該 Requirement を MODIFIED にして例外を名指すか、少なくとも data-sensitivity 側から例外であることを明記する。`GET /session` の応答の形（どの資格情報で認めたか）を画面が使うなら Scenario を 1 本置く。
- 処置: rejected: ログインの口は資格情報を欠いた求めを受ける口ではなく、**画面の合言葉そのものを資格情報として要求する口**で、欠落・不一致は 401 になる —— openspec/specs/record-envelope/spec.md:223-224 の「資格情報を要求し、欠くか一致しない要求を 401 で拒否する」の例外ではない。取り違えが起きないよう spec の SHALL（「ログインの口では、画面の合言葉そのものを資格情報として受け」）と注記と Scenario `合言葉を付けないログインの求めは断られる` を足した。応答の形は OpenAPI（コードから生成。製造準備 A-1）が契約で、design D2 はそれを指す

## R4. 台帳の書き込みに失敗しても読み出しを返す（D8）・取り込みは台帳に書かない（D8）が design にしかない。前者は C5 の uncaptured を黙って作る側
- 分類: 置き場（design に観測可能な振る舞い）
- 成果物: openspec/changes/st28-private-network-only/design.md:113-117（D8）・specs/data-sensitivity/spec.md:239-268
- 根拠: design.md:117「書き込みの失敗は応答を変えない（読み出しは返す）」—— 台帳に残らない読み出しが**成功として返る**。deep.md:140-141（C5 は loss: uncaptured —— 記録しなかった期間の読み出しは後から足せない）と CLAUDE.md の「既定は厳しい側」とは逆向きで、それが spec に書かれていないので、実装がどちらに倒しても spec は落ちない /
  design.md:113「POST の取り込みで通ったものは書かない」—— spec.md:241 は「記録の読み出しと、ログイン・ログアウトと、資格情報の無いか一致しない求め」を書く側しか列挙せず、**画面のログインで通った書き込み（D1 により `/ingest` も通る。design.md:37-41）を書くか**が spec から読めない
- kind: technical
- 提案: 台帳に書けなかったときの読み出しの扱い（返す / 断る）を spec の SHALL と Scenario にする（C5 の趣旨なら「断る」が厳しい側）。書かない求め（取り込みの成功）も spec に列挙する。
- 処置: fixed specs/data-sensitivity/spec.md —— 厳しい側（C5 の uncaptured を黙って作らない）を採り、「書けないときは記録を返さず 500」を SHALL と Scenario `読み出しの記録に書けないときは記録を返さない` にした。書かない求め（認められた取り込み）も SHALL と Scenario `取り込みは読み出しの記録に残らない` にした。design D8 に時機（ハンドラの前）と試験の差し替え口（`AccessSink`）を書いた。tasks 5.1

## R5. ログインの失敗の遅延と 429（D17（仮））は観測可能な振る舞いなのに、spec にも tasks の検証にも無い
- 分類: 置き場 / 検証の無い実装項目
- 成果物: openspec/changes/st28-private-network-only/design.md:178-181（D17（仮））・tasks.md:103-108（4.3）
- 根拠: design.md:178「失敗は 1 回ごとに 1 秒待たせ、1 分に 10 回を超えたら 429」/ tasks.md:104 は実装を指示するが、4.3 の Scenario の列挙（:105-107）にも検証（:108 `CT session_`）にも 429 を撃つものが無い（しかも `CT session_` は既存の試験で通る —— R15）/
  spec.md:109-117 は「一致しない求めを 401 で断る」だけ。429 を返す実装も返さない実装も spec を満たす。(仮) の反転条件（design.md:181「台帳に 1 日 100 回を超える `login_failed`」）は台帳が D17 の結果（429）をどう書くかにも依存するが、D8 の `outcome` の列挙（design.md:112）に 429 の値が無い
- kind: technical
- 提案: 仮決めのままでよいが、数値ごと spec の Scenario に置く（「1 分に 11 回目の失敗は 429 になる」）か、少なくとも tasks 4.3 に 429 を撃つ試験名と検証を足す。台帳の `outcome` に総当たりの断りを足す。
- 処置: fixed D17 仮 —— 数値（1 分 10 回・429）を spec の SHALL と Scenario `失敗を重ねたログインは一時的に断られる` に置き、読み出しの記録の `outcome` に `login_throttled` を足した。D17 の反転条件はそのまま。tasks 4.3

## R6. 画面の応答の CSP と `no-store` を `vite preview` だけに付け、`vite dev` を外す（D10）のは spec の「画面を配る側のすべての応答」と食い違う。dev の口は網へ出ており、C2 の検査も見ない
- 分類: 置き場 / spec と design の食い違い
- 成果物: openspec/changes/st28-private-network-only/design.md:128-129（D10）・design.md:140（D11 (a)）・specs/data-sensitivity/spec.md:205・:224
- 根拠: spec.md:205「画面を配る側の**すべての**応答に … 写しを保存させない指示」/ spec.md:224「画面を配る側の応答に … 禁じさせる指示」—— dev の例外が書かれていない /
  design.md:129「`vite dev` は HMR が inline script を使うので CSP を付けない」/ tools/dev.sh:20（`npm run dev -- --host 127.0.0.1`。port は vite の既定）/
  review/deep.md:18（網のアドレス上の 5173 —— vite dev の既定の port —— は tailscaled の serve が持つ口。**dev の画面は網から届いている**）/
  design.md:140（D11 (a) が見る port は `BIND` の port・`WEB_PORT`・55432 だけ。dev の port は入らない）
- kind: technical
- 提案: spec に dev の扱いを書く（「開発用の配り方は網へ出さない」を C2 の検査の対象に入れる、または dev にも `no-store` と、HMR を許す範囲の CSP を付ける）。例外を design に置くなら、spec 側の「すべての」を狭める。
- 処置: fixed specs/data-sensitivity/spec.md —— 開発用の画面は応答の指示を持たない代わりに網へ出さない、を spec の網の外の検査の Requirement に SHALL と Scenario `開発用の画面を網へ出していると検査が落ちる` で置いた。D10 / D11 に dev の port（`DEV_WEB_PORT`、既定 5173）を足した。手順書（D12）で serve の dev の口を消す。tasks 8.1

## R7. 「写しを保存させない指示」「読み込みを禁じる指示」は何を測れば真偽が決まるかが spec に無い。C4 / C6 が決めた値（`no-store` / `default-src 'self'`）が Scenario に無い
- 分類: 検証不能（数値・値が Scenario に無い）
- 成果物: openspec/changes/st28-private-network-only/specs/data-sensitivity/spec.md:203-237
- 根拠: deep.md:139（C4「`Cache-Control: no-store` を付け」）・:142（C6「`Content-Security-Policy: default-src 'self'` を付け」）/
  spec.md:214「どの応答も、写しを保存させない指示を持つ」・:237「同じ出所以外の資源の読み込みを禁じる指示を持ち」—— `Cache-Control: no-cache`（保存は許し再検証だけを求める）や `max-age=0` でも「指示」と読める。
  HTTP の応答ヘッダは protocol 上の観測値で、関数名・crate 名とは違って spec に書いてよい種類のもの
- kind: technical
- 提案: THEN を観測値で書く（「応答の `Cache-Control` に `no-store` がある」「応答の `Content-Security-Policy` の既定の出所が自分の出所だけである」）。
- 処置: fixed specs/data-sensitivity/spec.md —— THEN を観測値にした（`Cache-Control` に `no-store` がある / `Content-Security-Policy` の `default-src` は `'self'` だけ）。SHALL にもヘッダの値を書いた

## R8. 拠点外に置かない（NFR-15）の Scenario は、突き合わせる一覧が design にしかなく、しかも今あるサーバの部品だけで外へ接続を張れる。Requirement 本文の「口を足しても外へ書き出せない形」を検査が担保しない
- 分類: 検証不能 / Requirement 本文だけが言っていること
- 成果物: openspec/changes/st28-private-network-only/specs/data-sensitivity/spec.md:323-337・design.md:162-167（D14）
- 根拠: spec.md:336「外へ接続を張る部品（HTTP の送り手・メールの送り手・外部の保管庫の手）の一覧と突き合わせる」—— 一覧の中身は design.md:164 にだけあり、**空の一覧でも THEN（0 件）が真になる** /
  spec.md:328「口を足しても外へ書き出せない形にする」に対し、crates/server/src/lib.rs:1816 は `tokio::net::TcpListener` を使っており tokio の `net` は有効、crates/server/Cargo.toml の `sqlx`（`runtime-tokio-rustls`）は**任意の host へ TCP + TLS を張る部品そのもの**（`DATABASE_URL` の向き先）。一覧に無い部品で外へ書き出す route は今日足せる /
  `DATABASE_URL` を拠点外の DB に向ければ記録の写しがまるごと拠点外に置かれるが、これを落とす Scenario も無い
- kind: technical
- 提案: Scenario の主張を検査できる範囲に下げる（「サーバの依存に、名指しした外への送り手の部品が 0 件」と一覧を spec に書く）か、本文の「口を足しても外へ書き出せない」を削る。DB の接続先が loopback（か許した網の内側）であることを起動時に見る Scenario を足すかを検討する。
- 処置: fixed D14 —— Scenario に「一覧は検査の側が持ち 1 つ以上の名前を持つ」を入れ、台本が空の一覧で走らないことを D14 と tasks 8.2 に書いた。本文の「口を足しても外へ書き出せない」は外した（限界を D14 に明記）。DB の接続先が loopback でなければサーバも移行も起動しない、を SHALL と Scenario `DB の接続先が loopback でなければ起動しない` にした（tasks 3.3）

## R9. 「台帳」が 2 つの意味で使われ、切り詰めの Scenario がどの表を指すか決まらない。tasks 2.4 は消去の台帳を撃たない
- 分類: 検証不能（語の衝突）
- 成果物: openspec/changes/st28-private-network-only/specs/data-sensitivity/spec.md:293-296・tasks.md:67-70（2.4）
- 根拠: spec.md:295「記録・履歴・台帳の表の切り詰めを求める」—— この Requirement（:270-281）は record-envelope の記録の門から導出していて、そこでの「台帳」は**消去の台帳**（openspec/specs/record-envelope/spec.md:333-338）。同じ spec の :239-268 では「台帳」は**読み出しの台帳**を指す /
  tasks.md:68 は `core.event`・履歴・`core.access_log` だけを撃ち、**消去の台帳を撃たない**。並走中の st22 は `core.deletion_ledger`、st12 は書庫の台帳を足す（`git diff main...feat/st22-record-deletion` / `feat/st12-archive-ingestion` の TRUNCATE トリガ）
- kind: technical
- 提案: 読み出しの台帳を「読み出しの記録」など別の語にするか、Scenario を「記録の schema の**すべての表**について切り詰めが拒まれる」（`移行が足したどの表にも…` と同じく表を列挙しない形）にする。
- 処置: fixed specs/data-sensitivity/spec.md —— 読み出しの台帳を「読み出しの記録」に改名し、「台帳」は record-envelope の消去の台帳だけを指すようにした。切り詰めの Scenario は `アプリの接続からはどの表も切り詰められない`（記録の schema のすべての表を DB から引く。並走中の change の台帳も自動で入る）にした。tasks 2.4

## R10. 1 つの Scenario に主張を束ねたものがあり、1 つの Scenario を 2 つの試験に分けて同じ印を置く計画になっている。片方だけで緑になる
- 分類: 検証可能性（1 Scenario 1 主張）
- 成果物: openspec/changes/st28-private-network-only/specs/data-sensitivity/spec.md:123-126・:148-152・:234-237、tasks.md:105-106・:133-140
- 根拠: spec.md:148-152 `ログインの印はスクリプトから読めず、別のサイトからの求めには付かない` —— THEN に 3 属性 + AND（スクリプトから読めない）の 4 主張 /
  spec.md:126 —— 401 / 記録 0 件 / 入力欄が出る、の 3 主張 / spec.md:237 —— 指示を持つ / 違反の報告が出ない、の 2 主張 /
  tasks.md:106 と :140 が同じ Scenario 名の印を別々の試験（サーバの `Set-Cookie` と、e2e の `document.cookie`）に置く。`違う合言葉ではログインできない`（:105 と :138）・`全インタフェースでは起動しない` も 2 か所。`check_scenarios.py` は印が 1 つあれば担保ありと数えるので、片方の試験が消えても緑のまま
- kind: technical
- 提案: 束ねた Scenario を分ける（例: 「ログインの印はスクリプトから読めない」「ログインの印は暗号化された接続でだけ送られる」「ログインの印は別のサイトから始まった求めには付かない」）。2 つの層で撃つものは Scenario を層ごとに分ける。
- 処置: fixed specs/data-sensitivity/spec.md —— 束ねた Scenario を割った（ログインの印 3 本・未ログイン 2 本・違う合言葉はサーバと画面の 2 本・CSP は指示と報告の 2 本）。tasks の Global Constraints に「1 つの Scenario の印は 1 つの試験にだけ」を置き、`bind_allowed` の単体には印を置かないと書いた。59 本がそれぞれ tasks に 1 回だけ現れることを機械で確かめた

## R11. Requirement の本文だけが言っていて、Scenario が 1 本も撃たない SHALL がある
- 分類: 検証の抜け（Requirement 本文 ⊃ Scenario）
- 成果物: openspec/changes/st28-private-network-only/specs/data-sensitivity/spec.md
- 根拠: 本文だけが言っていて Scenario が撃たないもの ——
  - :11「サーバ**と画面**の待ち受けの既定を loopback」—— Scenario（:21-44）はサーバだけ。画面の待ち受けは D7（design.md:106）が台本の引数で固定するだけ
  - :115「合言葉の比較を、一致した長さから内容が推測されない方法で」—— Scenario なし（tasks 4.3 が `token_matches` を名指すだけ）
  - :206「端末の上で記録を見る仕組み（オフラインの写し）を持たない」—— Scenario なし（C4 の後半。deep.md:139）
  - :245「台帳の行を記録の読み出しに出さない」—— Scenario なし
  - :275「表の定義の変更を拒む」—— Scenario はトリガの無効化（:288-291）だけ
- kind: technical
- 提案: 1 本ずつ Scenario を足す（例: 画面を配る側が loopback 以外に口を開けていない / 画面の配布物に service worker と端末内の保存が無い / 台帳の行が記録の読み出しの応答に 0 件）。撃たないなら本文から外す。
- 処置: fixed specs/data-sensitivity/spec.md —— 画面の待ち受けは網の外の検査の Scenario `loopback 以外で待ち受ける口があると検査が落ちる` が画面の口も見る（本文の口の一覧に画面を名指した）/ 比較の方法は record-envelope の同じ SHALL に委ねて本文から外した（design D1）/ オフラインの写しは Scenario `画面は端末に記録の写しを置かない` / 記録の読み出しに出さないは Scenario `読み出しの記録は記録の読み出しに出ない` / 表の定義の変更は Scenario `アプリの接続からは表の定義を変えられない`（トリガの無効化で撃つ）にした

## R12. 正典に残る spec に Story 番号・「この Story の時点で」・環境変数名が入っている
- 分類: 置き場（正典に一時的な語 / 実装の名前）
- 成果物: openspec/changes/st28-private-network-only/specs/data-sensitivity/spec.md:5・:19・:121・:325・:328・:332
- 根拠: spec.md:328「THE SYSTEM SHALL **この Story の時点で**、サーバに拠点外の宛先へ接続を張る部品を持たせない」—— archive 後の正典で「この Story」は指す先が無く、SHALL の効く期間が読めない /
  :325・:332（ST31）・:121（ST29）・:5（ST24 / ST29）/ :19 の `BIND`（環境変数名。実装の名前）
- kind: technical
- 提案: SHALL から「この Story の時点で」を外す（ST31 で MODIFIED する前提なら、今の条文を無条件で書く）。Story 番号と `BIND` は導出の注記（design か deep）へ寄せる。
- 処置: fixed specs/data-sensitivity/spec.md —— SHALL から「この Story の時点で」・Story 番号・`BIND` を外した。拠点外のバックアップは「この Requirement を MODIFIED にしてその部品だけを開ける」と注記に書いた

## R13. 所有者の役割は自分の表のトリガを外せるのに、その合言葉の扱いを spec が見ていない。stack.sh はその合言葉をサーバのプロセスに渡す
- 分類: 前提の穴（R103 の開口部が所有者へ移る）
- 成果物: openspec/changes/st28-private-network-only/specs/data-sensitivity/spec.md:270-321・design.md:87-92（D5）・:189-195（D19）・tasks.md:42-46（1.2）
- 根拠: 表の所有者は `ALTER TABLE … DISABLE TRIGGER` を実行できる（design.md:83 の自己検査が所有者を拒むのもそのため）。つまり**所有者の合言葉を持つ者は記録の門を外せる** /
  spec.md:277・:318-321 は「DB の**管理者**の合言葉を配布物に固定値として持たない」だけで、所有者の合言葉は対象外 /
  design.md:191（D19）は `testdb::pool()` の既定を `postgres://ashiato_owner:…@127.0.0.1:55432/ashiato` とする —— 既存の `testdb.rs:15` と同じ形なら、所有者の合言葉の固定値が追跡ファイルに入る /
  tasks.md:44 は `.env.example` の値を `change-me-…` の雛形にする（公開の値。そのまま使っても何も止めない）/
  design.md:91 は「サーバの実行時のプロセスに所有者の合言葉を持たせない」を D5 の理由にするが、tools/stack.sh:21 は `set -a; . ./.env; set +a` で **`.env` の全部を export してからサーバを起動する**（:63）ので、`DATABASE_OWNER_URL` / `OWNER_DB_PASSWORD` / `POSTGRES_PASSWORD` がサーバの環境に入る。tasks にこれを外す項目が無い
- kind: technical
- 提案: spec の「固定値を持たない」を所有者（とアプリ）の合言葉にも広げ、`tools/db-roles.sh` か起動時の検査が雛形の値（`change-me-`）を拒む Scenario を置く。tasks 2.2 に「サーバを起動する台本は所有者・管理者の合言葉をサーバの環境から外す」と、その検証（起動したサーバの `/proc/<pid>/environ` に `DATABASE_OWNER_URL` が無い 等）を足す。
- 処置: fixed D5 —— 所有者とアプリの合言葉も管理者と同じ扱いにした（spec の SHALL と Scenario `配布物に DB の合言葉の固定値が無い` / `雛形の合言葉のままでは役割を作らない` / `サーバの実行時の環境に所有者と管理者の合言葉が無い`）。D5 でサーバを `env -u` で起動し、D19 で `testdb.rs` の固定の URL を消した。tasks 1.1 / 1.2 / 2.2

## R14. 収集アプリの 2 つの Scenario は、いまのコードのままで通る。平文の例外を baseUrl の host から作る D26 の形が戻っても落ちない
- 分類: 検証が変更を見分けない
- 成果物: openspec/changes/st28-private-network-only/specs/data-sensitivity/spec.md:87-95・tasks.md:153-155（7.2）
- 根拠: collector-android/app/build.gradle.kts:77-93 はいま `base-config cleartextTrafficPermitted="false"` と **baseUrl の host だけ**の平文の例外を出す /
  計測テストは CI（.github/workflows/ci.yml:223・:226）でも手元（tools/android-emulator.sh:53・:57）でも `-Pashiato.baseUrl=http://127.0.0.1:18787` で組み立てるので、いまのコードでも
  `isCleartextTrafficPermitted("example.invalid")` は偽・`("127.0.0.1")` は真 —— tasks.md:159 の試験は**変更前から緑**。
  取り除くべき形（`https://<網のホスト名>` で組み立てても、その host への平文を許す）は、baseUrl が loopback の組み立てでは現れない
- kind: technical
- 提案: 撃つ宛先を「組み立てに使った接続先の host」にする（`BuildConfig` の baseUrl の host が loopback でない組み立て —— 例: `-Pashiato.baseUrl=https://example.invalid` —— の network security config にその host の平文の許可が無いことを、生成物の XML か単体で見る）。計測テストの 2 本はその上に残す。
- 処置: fixed D13 —— 撃つ宛先を組み立てに使った接続先の host にした（`-Pashiato.baseUrl=https://example.invalid:1` で生成された XML にその host の平文の許可が無い。いまのコードでは落ちる）。Scenario を `収集アプリは接続先の宛先にも平文を許さない` に直し、印を `tools/check-apk-cleartext.sh` に置く。計測テストは loopback の 1 本だけ。tasks 7.1 / 7.2

## R15. 検証コマンドのうち、変更前から rc=0 になるものがある（0 本で緑の型）
- 分類: tasks の検証
- 成果物: openspec/changes/st28-private-network-only/tasks.md:108（4.3）・:158（7.3）・:62（2.2）・:75（2.6）・:155（7.2）
- 根拠:
  - 4.3 `CT session_` —— 絞り込み `session_` は部分一致で、既存の `view_supersession_by_a_superseded_claim_still_counts`（crates/server/src/attributes.rs:1009）・`read_deleted_supersession_restores_the_target`（attributes_tests.rs:586）・`read_erased_claims_vanish_and_their_supersession_lifts`（:638）に当たる。**ログインの試験が 0 本でも `test result: ok. 3 passed` で CT が通る**
  - 7.3 `cargo test -p ashiato-collector-windows config_` —— 既存の `config_requires_every_var_including_state_dir`（crates/collector-windows/src/config.rs:137）に当たり、変更前から 1 passed
  - 2.2 `grep -c migrate … | grep -v ":0$" | wc -l | grep -qx 4` —— いまは 4 ファイルとも 0 で非空虚だが、振る舞いではなく語の有無を見る。`tools/verify-prep.sh` の `run.sh` は `exec ./tools/stack.sh up`（:64）なので、verify-prep に `migrate` の語を足す必要は無く、コメント 1 行で通る
  - 2.6 `grep -q ashiato_app /tmp/ci.log` —— 拒まれたことではなく、出力にその語があることだけを見る
  - 7.2 の 1 つめ `grep -rl CleartextPolicyInstrumentedTest` —— ファイルの存在だけ（2 つめの `tools/android-emulator.sh` が実体）
- kind: technical
- 提案: 絞り込みを試験名の接頭辞で一意にする（`CT web_session_`・`config_base_url_` など、既存に当たらない名前）か、`--exact` と試験名を 1 本ずつ書く。2.2 / 2.6 は出力の OK 行（`OK ashiato_app は … を拒む`）や起動の順序を見る形に。
- 処置: fixed 4.3 —— 試験の接頭辞をこの change だけのものにした（`server_startup_` / `web_session_` / `access_log_` / `app_role_` / `response_no_store_` / `base_url_cleartext_`）。Global Constraints に `-- --list` で確かめる規律を置いた。2.2 は語の有無ではなく `tools/smoke.sh` と `STACK_RESET=1` の起動で、2.6 は拒まれたときだけ出る `OK app-role` の行数で見る形にした。7.2 はファイルの存在の検査を外した

## R16. tasks の依存順が逆のものが 2 つある
- 分類: tasks の依存順
- 成果物: openspec/changes/st28-private-network-only/tasks.md:67-70（2.4）・:42-46（1.2）
- 根拠:
  - 2.4 は `TRUNCATE … core.access_log` が拒まれることを撃つが、`core.access_log` を作るのは 4.1（tasks.md:97-98）。2.4 の時点では表が無く、試験は「存在しない」で落ちる（あるいは表を外して書かれ、4.1 の後に誰も足し戻さない）
  - 1.2 の検証 `! git grep -nE "POSTGRES_PASSWORD: *[a-zA-Z0-9]" -- docker-compose.yml .github/` は、.github/workflows/ci.yml:67 の `POSTGRES_PASSWORD: ashiato` に当たって**1.2 の時点では必ず落ちる**。ci.yml を直すのは後の 1.4 で、しかも 1.4 は「CI の合言葉はその job の中だけの値」（tasks.md:52）—— 字面の値を書けば 1.2 の検証に再び当たる
- kind: technical
- 提案: 2.4 の `core.access_log` の行を 5.2 へ移す（または 4.1 を Task 2 の前へ）。1.2 の `.github/` の検証を 1.4 へ移し、CI の値の置き方（`${{ }}` で job 内に生成する等、字面の値を書かない形）を 1.4 に書く。
- 処置: fixed 1.2 —— 1.2 の検証を `docker-compose.yml` と `testdb.rs` だけに絞り、CI を含む全体の検査は 1.4 に移した。CI の合言葉は job の中で乱数から作り字面を書かない（design D19）。2.4 は `core` のすべての表を DB から引く形にしたので、4.1 の前は `core.access_log` を含まず、後は自動で含む（順序に依らない）

## R17. 起動のたびに DB を作り直す経路（CI の smoke / e2e、`STACK_RESET=1`、`smoke.sh` の `down -v`）で、役割と合言葉が無くなる。tasks は CI の「DB の job」しか直さない
- 分類: tasks の抜け（CI と台本）
- 成果物: openspec/changes/st28-private-network-only/tasks.md:51-53（1.4）・:60-62（2.2）
- 根拠: tools/smoke.sh:19-20（`docker compose down -v` → `up`）・tools/stack.sh:58-61（`STACK_RESET=1` で `down -v`。e2e は CI で `STACK_RESET: "1"`、ci.yml:164-168）—— volume を消すと `tools/db-roles.sh` が作った役割が消え、`ashiato-server migrate`（所有者）も起動（アプリ）も接続できない /
  tasks 2.2 は台本に `migrate` を足すだけで、`db-roles.sh` を呼ぶことを書いていない /
  1.2 で `docker-compose.yml` が `${POSTGRES_PASSWORD:?…}` になると、`.env` の無い CI の smoke job（ci.yml:229-240）と e2e job（:148-168）では `docker compose up` 自体が落ちる。e2e は `WEB_PASSWORD` も要る（D10。design.md:135）。tasks 1.4 は「DB の job」（rust job。ci.yml:54-84）しか直さない
- kind: technical
- 提案: 2.2 に「DB を作り直す台本（smoke.sh / stack.sh の STACK_RESET）は `db-roles.sh` → `migrate` → サーバの順」を足し、1.4 を smoke / e2e job の環境（`POSTGRES_PASSWORD` / 役割の合言葉 / `WEB_PASSWORD` を job 内で作る）まで広げる。検証は CI の 3 job が緑であること。
- 処置: fixed 2.2 —— DB を作り直す台本（`smoke.sh` の `down -v` / `stack.sh` の `STACK_RESET=1`）を含めて `db-roles` → `migrate` → サーバの順にした。1.4 を CI の rust / smoke / e2e の 3 job に広げ、`WEB_PASSWORD` も job の中で作る。design D5

## R18. 実機・本人の環境で要る手順（本物の DB の所有の移し替え・`tailscale serve` の https 化・`ashiato.baseUrl` の書き換えと APK の入れ直し・`.env` の新しい秘密）が tasks に無く、人間の手順とも書かれていない
- 分類: 人間の確認待ち・人間の作業の明示
- 成果物: openspec/changes/st28-private-network-only/tasks.md（Task 9）・design.md:211-218（Migration Plan）
- 根拠: design.md:213-217 は 5 手の移行を書くが、tasks のどの項目も本人の機械での実施を持たない（tasks.md:20「人間の確認待ちに逃がせる Scenario は 1 本も無い」は Scenario の話）/
  tools/verify-prep.sh:35 は `~/.gradle/gradle.properties` の `ashiato.baseUrl` で APK を組み立てる —— 7.1 の後、その値が `http://<網のホスト名>` のままなら**組み立てが落ち、確認バッチの APK が出なくなる** /
  確認バッチの `run.sh` は `tools/stack.sh` を呼び（verify-prep.sh:64）、stack.sh は本人の `.env` を読む（stack.sh:21）—— `.env` に `POSTGRES_PASSWORD` 等が無ければ DB が上がらない /
  `tools/db-roles.sh` は本人の**実データの入った DB** の所有を移す（design.md:199-201）。戻し方は design.md:218 にあるが、実施前の退避が書かれていない
- kind: daily
- 提案: Task 9 に「本人の機械で行う移行」を 1 項目立て、人間の作業であること・順序（`.env` → `db-roles.sh` → `migrate` → serve の https 化 → `check-exposure.sh` → baseUrl → APK）・確認バッチの手順書に載せることを書く。`db-roles.sh` の前の DB の退避の要否も書く。
- 処置: fixed D20 仮 —— 本人の機械での移行（退避 → `.env` → 役割 → 移行 → serve の https 化 → APK）を D20 に順序つきで置き、`docs/network.md`（tasks 9.1）と PR 本文に写す。確認バッチは平文の `ashiato.baseUrl` で APK を作らずに案内を出して続ける（tasks 9.2）。反転条件は D20

## R19. ST28 が PERM-10 を画面の経路と DB の役割で満たし直し、第 2 の資格情報を足したのに、PERM-10 の本文（「配り方は ST29 の深掘りで決める」）と INDEX に印が無い
- 分類: 要件・INDEX との整合（観点 5）
- 成果物: docs/requirements.md:636-642（PERM-10）・docs/stories/INDEX.md・openspec/changes/st28-private-network-only/specs/data-sensitivity/spec.md:119・:185・:247・:280
- 根拠: ST28.md:4 の satisfies は PERM-7 / NFR-15 だけ。spec の 10 Requirement のうち 4 本（:119・:185・:247・:280）の導出元が PERM-10（ST01 の satisfies。ST01.md:4）/
  requirements.md:642「資格情報の配り方（共有の合言葉 / 呼び出し元ごとの資格情報）は ST29 の深掘りで決める」—— Q1 で画面の合言葉を API の合言葉と別に持つことにしたのは、配り方の一部を ST28 で決めたことになるが、PERM-10 に ★ が無い /
  INDEX.md は過去の Story の要件側の前倒し・満たし直しを注記で残している（:84・:116・:124・:211）が、ST28 の PERM-10 には無い（`grep -n "PERM-10" docs/stories/INDEX.md` → 0 件）。deep.md:157 に書いた一文は archive で change と一緒に退く /
  C7 の文言揃えは §1.4 と §5 だけで、requirements.md:639（PERM-10 の本文）と openspec/specs/record-envelope/spec.md:228 に「PERM-7（外部からの到達を Tailscale 網内に限る）」が残る
- kind: conflict
- 提案: PERM-10 に ★ 2026-09-29 補足（「画面の経路は画面の合言葉のログインで満たす（ST28 Q1）。収集側・プラグインの配り方は ST29」）を足し、INDEX に ST28 の PERM-10 の満たし直しを 1 行注記する。PERM-10 の本文の「Tailscale 網内」も C7 に揃える。
- 処置: fixed D21 仮 —— PERM-10 に ★ 2026-09-29 補足（画面の経路は画面の合言葉のログインで満たす。収集側・プラグインは ST29）を入れ、本文の「Tailscale 網内」を PERM-7 に揃えた。INDEX に「同じ型が ST28 からも 1 件出ている」の注記を足し、deep.md の「要件へ戻すもの」に PERM-10 を足した。`make_story.py` で再生成（ST01.md の逐語が変わった）、`check_chain.py` OK。record-envelope の正典の注記（:228）の「Tailscale 網内」は導出の散文で、次に record-envelope を開く change が揃える（ここで開くと並走中の Story と capability が重なる）

## R20. proposal の「同じファイルは触る」の一覧が、並走中の change が実際に触っているファイルより狭い
- 分類: 並走 Story との重なり
- 成果物: openspec/changes/st28-private-network-only/proposal.md:84-85
- 根拠: proposal.md:84 は `lib.rs`・`docs/openapi.json`・`web/e2e/`・`tools/stack.sh` だけを挙げる。`git diff --stat main...<branch>` では:
  `tools/smoke.sh`（st06 / st08 / st12）・`crates/server/src/testdb.rs`（st12。試験ごとにプールを持つ形に変え、`docker-compose.yml` に `max_connections=300` を足す）・`docker-compose.yml`（st12）・`tools/check-immutable.sh`（st12 / st22）・`.github/workflows/ci.yml`（st06 / st08）—— どれも ST28 の tasks 1.2 / 1.3 / 1.4 / 2.2 / 2.6 / 5.2 が書き換える。
  特に testdb.rs は D19 が `pool()` の接続先を変え、st12 はその同じ関数を変えている。差し戻しは要らない（tasks 10.3 の rebase で追える）が、Impact が狭いと rebase の見積もりを誤る
- kind: technical
- 提案: proposal の Impact の一覧にこの 5 ファイルを足す。tasks 10.3 の rebase の追従に「testdb.rs のプールの持ち方（st12）に `app_pool()` を揃える」を足す。
- 処置: fixed proposal.md —— Impact の「同じファイル」に `testdb.rs` / `tools/smoke.sh` / `tools/check-immutable.sh` / `docker-compose.yml` / `ci.yml` を足した。tasks 10.3 に st12 の `testdb.rs` のプールの持ち方と `max_connections` への追従を足した

## R21. ログインの印（cookie）は port で分かれないので、同じ機械名の別の port で動く他のサービスにも送られる
- 分類: 前提（Q1 の「同じ PC の別プロセスも止まる」）
- 成果物: openspec/changes/st28-private-network-only/design.md:58-61（D3）・specs/data-sensitivity/spec.md:113・:148-152
- 根拠: cookie は host で束ねられ port では分かれない（RFC 6265 §8.5「Cookies do not provide isolation by port」）。design.md:59 は `Domain` を付けない（host だけ）が、**同じ host の別 port には付く** /
  この機械は網の同じ機械名で ashiato 以外の口も出している（review/deep.md:19「https は操作盤（2024）」、:18 の 5 つの口）。その別の口のサービスは、ブラウザがそこを開くたびにログインの印を受け取る。SameSite は site 単位なので同じ host の別 port は「同じサイト」で、spec.md:113 の「別のサイトから始まった求めには付かない」はここを止めない /
  deep.md:52（Q1 の推奨の理由「同じ PC の別プロセスも止まる」）
- kind: technical
- 提案: 手順書（D12）に「画面は専用の機械名（か、ほかのサービスを同居させない host）で出す」を書くか、印の有効性を画面の出所（`Origin` / `Host`）に縛る形を design で検討する。spec には「同じ host の別の口へは印を渡さない」を主張できないので、書くなら限界を注記に置く。
- 処置: fixed D12 —— 主張できない限界として spec の注記と design の Risks に書き、手順書に「画面を出す機械名に信頼できないサービスを同居させない」を置いた（tasks 9.1 の検証が「同居」の語を見る）

## R22. `secret_tag` は画面の合言葉を鍵にした固定の文への HMAC で、DB に全行同じ値で残る。DB の写し（ST31 のバックアップを含む）から合言葉を総当たりで確かめる道具になる
- 分類: 設計の副作用
- 成果物: openspec/changes/st28-private-network-only/design.md:63-67（D3）・:174-176（D16（仮））
- 根拠: design.md:66「`secret_tag` = HMAC-SHA256（鍵 = 画面の合言葉、本文 = 固定の文字列 `ashiato-web-session-v1`）」—— 塩の無い速いハッシュが `core.web_session` の全行に入る。
  DB の写しを持つ者は、合言葉の候補ごとに HMAC 1 回で当否を確かめられる。D16（仮）の反転条件は下限を 12 文字に下げうる（design.md:176）/
  「合言葉を変えると全行が無効」（Q6）は、合言葉そのものでなく**合言葉の世代**を DB の外（`.env` から導く乱数や世代番号）で持っても成り立つ
- kind: technical
- 提案: `secret_tag` を、合言葉から遅い鍵導出（塩つき）で作るか、合言葉を変えたときに `.env` 側の世代の値を変える形にする。D3 に採らなかった案として残す。

- 処置: fixed D3 —— `secret_tag` の鍵を `API_TOKEN`（DB に無い秘密）にし、本文を固定の文字列 ‖ 画面の合言葉にした。DB の写しだけでは合言葉の候補を確かめられない。当初の案を「採らなかった案」に残した

---

## 観点ごとのまとめ

- 観点 1（deep の決定 → 正典）: R1・R2・R7。Q1 / Q3 / Q4 / Q5 は Scenario に写っている（Q4 は `API の合言葉での読み書きは変わらない`、Q5 は 8 本、Q3 は本文 :325-327）。Q6 の締め出し（合言葉の変更で全無効）と期限の設定値（30 日 / 31 日の数値つき）は Scenario にある。**覆した 2 問（Q4・Q6）について、覆す前の案（収集側の合言葉を送る経路だけに絞る / 30 日）が specs に残っていないことを確かめた**（`grep -n "30 日" spec.md` → 期限を設定したときの Scenario の 1 か所だけ）。要件へ戻すもの（NFR-15 / §1.4 / §5）は requirements.md:56・:845・:873 に ★ 2026-09-29 で入っている
- 観点 2（検証可能性）: R7・R8・R9・R10・R11
- 観点 3（置き場）: R2・R3・R4・R5・R6・R12。proposal の Capabilities（`data-sensitivity` のみ・New）と `specs/` のディレクトリは一致し、INDEX.md:77 の割当（ST24 / ST28 / ST29）とも一致する（前倒しは無い）
- 観点 4（tasks の検証）: R15・R16・R17・R18。`CT` / `ET` の略記は tasks.md:21-25 で定義されている（件数つき・pipefail つき）。問題はその絞り込みが既存の試験に当たること（R15）
- 観点 5（Story と要件）: `check_chain.py` の再生成との一致は通っている。ST28.md の価値・完了の判定は deep の決定と矛盾しない（完了の判定 4 件は Q1 / C7 / Q5 と一致）。R19（PERM-10 の満たし直しに要件側・INDEX 側の印が無い）
- 並走中の change への差し戻し: tasks・design のどこにも st06 / st08 / st12 / st22 の change のファイルを書き換える指示は無い（tasks.md:9。付与は移行の配列の外 D4、e2e は storageState D10、route は D1）。重なりの一覧の狭さだけを R20 に挙げた

