# 整合の確認（2026-11-02・ST44 の深掘り第 1 回 Q2 の答えを FR-128 へ戻した改訂）

- 基準: `63d5480fed91b596909661c5164315451144745a`..`b2e0a295fad20df73deabddbd76c1783aa4d1c32`

対象は commit b2e0a295 が FR-128 に足した ★ 2026-11-02 補足（`docs/requirements.md:1076-1078`）:
「**収集アプリの資格情報は、携帯と PC で共有する 1 本のまま**（端末ごとには発行しない）。ここで設定させる資格情報はその共有の値で、発行・取り消しの画面は作らない。どの端末の資格情報で届いたかは持たない（端末は PERM-10 のとおり送り手の申告のまま）。落とした端末を締め出すには、値を変えて全部の収集アプリに設定し直す」。
同じ commit で `docs/stories/ST56.md:22-24` の FR-128 の写しに ★ の 2 行が載った（3 行目は「…（続きは requirements.md）」で切れる）。
答えの記録（`openspec/changes/st44-collector-credential-scope/deep.md`・`deep-answers-1.txt`・`deep-questions.json` の Q2）と突き合わせた。文書は head（`b2e0a295`）の内容で読んだ。

機械の入力:
- `req_impact.py . --base 63d5480f…`: 改訂した要件 1 件（FR-128）・扉 0 項。名指しは `docs/stories/INDEX.md:79`・`docs/stories/stories.json:1332`・st44 の `deep-questions.json:60`・`deep.md:16,26`・`review/deep.md:8,90`・`review/spec.md:98,99`・st52 の `premises.md:22`・`review/deep.md:8,23`
- `check_chain.py .`: 要件 171 / Story 58 / 扉 35・拾われた要件 169/171・未回収 0・warn 0。FAIL 2 件 ——「ST56: 判断の日（2026-10-25）より新しい訂正が満たす要件にある（FR-128。最新 ★ 2026-11-02）」（C1）、「整合の確認の後に要件が変わった（この監査を名指ししない改訂）: FR-128」（この監査が受けるもの）

走っている change は `st44-collector-credential-scope`（出どころ）と `st52-release-channels`。st44 への指摘は**印だけ**（C2・C3）。st44 の tasks は `- [x]` が 0 件で、文書の側で失われるもの（uncaptured / discarded / exported / rewrite-all）は無い。Q2 の答え自身が `loss: uncaptured`（どの端末の資格情報で届いたかをずっと持たない）を本人の選択として持つ。st52 はこの改訂を前提にしていない（下の「触らなくてよい」）。

## C1. ST56 の判断（judged 2026-10-25）が改訂の前のままで、完了の判定「資格情報が取り消されたとき」が ★ の「発行・取り消しの画面は作らない」と噛み合わない
- 行き先: stories（`stories.json` の ST56 の完了の判定の 3 つ目を「設定を間違えた・サーバの合言葉（共有の 1 本）を変えたときに、記録を捨てずに設定し直しを促す」にし、judged を 2026-11-02 にして再生成した。題・価値は共有の値の設定でも成り立つので変えない）
- 重大度: 中
- 文書: docs/stories/stories.json:1348
- 要件: FR-128
- 根拠: ST56 の完了の判定（`stories.json:1348`）「設定を間違えた・**資格情報が取り消された**ときに、記録を捨てずに設定し直しを促す」、`judged: 2026-10-25`（`:1350`）。
  改訂後の FR-128 ★（`requirements.md:1077-1078`）「ここで設定させる資格情報はその共有の値で、**発行・取り消しの画面は作らない**」「落とした端末を締め出すには、**値を変えて**全部の収集アプリに設定し直す」。
  共有の 1 本では「取り消し」の操作は無く、起きるのは本人がサーバの `API_TOKEN` を変えること（`crates/server/src/lib.rs:4331` は起動時に環境変数 1 つを読む）。判定の語のままだと、ST56 の上流は Q2 の推奨（`deep-questions.json` Q2 options[0]「画面で端末ごとの資格情報を発行・取り消しし」）を前提に「取り消された 1 本」を作るか、取り消しの口がどこにあるかを問い直す。
  `check_chain.py` が FAIL「ST56: 判断の日（2026-10-25）より新しい訂正が満たす要件にある（FR-128。最新 ★ 2026-11-02）」を出している
- 直すところ: requirements の次の /stories（`stories.json` の ST56 の完了の判定の 3 つ目を「設定を間違えた・サーバの合言葉を変えたときに…」の形にし、judged を 2026-11-02 にする。題・価値は共有の値の設定でもそのまま成り立つ）
- 機械: 捕まえられた（`check_chain.py` の判断の日の FAIL。判定の語が要件と食い違うことまでは見ない）

## C2. 走っている st44 の `review/spec.md` R10・R11 の `deferred ST56` / `deferred ST37` が、いま `story_brief.py` で ST56・ST37 に「端末ごとの発行」「資格情報の識別子の欄の置き場」として届いている（印のみ）
- 行き先: docs（走っている st44 の `review/spec.md` R10・R11 の処置を `deferred ST56` / `deferred ST37` から `rejected:`（Q2 で共有の 1 本を選んだので申し送る中身が無い）に改めた。`spec --answers Q2` の段は `review/answers.md` を書き `review/spec.md` を書き直さないので、record のこの段で止めた。ST37・ST56 のブリーフに届かなくなる）
- 重大度: 高
- 文書: openspec/changes/st44-collector-credential-scope/review/spec.md:105
- 要件: FR-128, FR-24, FR-92, PERM-10
- 根拠: `review/spec.md:110-111`（R10）「ST56 へ: 画面での端末ごとの発行・取り消しと入れた後の設定の口、記録の行と読み出しの記録の行にどの資格情報で届いたかを書く欄…処置: deferred ST56」、`:118-119`（R11）「ST37 へ: 書き手の列を作るとき、収集アプリの資格情報の識別子を書く欄の置き場を空けておく（ST56 が埋める）処置: deferred ST37」。
  `python3 scripts/story_brief.py ST56` / `ST37` を走らせると、どちらも「この Story に宛てられたもの」に R10 / R11 が逐語で出る（「深掘りの前に読む」節）。
  改訂後の FR-128 ★「端末ごとには発行しない」「どの端末の資格情報で届いたかは持たない」とは逆で、`deep.md` の Q2 の帰結「どの端末の資格情報で届いたかは、ずっと持たない」とも逆。
  ST37 は `requires: [ST29]` だけ（`stories.json` の ST37）で、ST29 は archive 済み（`openspec/changes/archive/2026-10-27-st29-satellite-credentials`）なので、st44 の spec の段より先に上流が始まりうる。始まると、誰も埋めない「収集アプリの資格情報の識別子」の欄の置き場を前提に書き手の列の形を決める（`docs/handoff/ST37.md:12-16` の衛星の粒度の問いと同じ表に入る）。
  ST56 は `requires: [ST52, ST44]` で st44 の archive より先には始まらない
- 直すところ: leave（走っている st44 の `spec --answers Q2` の段が直す。その段の書き直しが design・proposal・specs だけで終わると、`review/spec.md` の R9〜R11 の処置は残り、R10・R11 は配送され続ける）
- 機械: 安く捕まえられる（`review_triage.py` か `story_brief.py`: `deferred ST<NN>` の R の提案に出る深掘りの問いの選択肢の label が、`deep-answers-<n>.txt` で選ばれなかった側のものなら warn）

## C3. 走っている st44 の design D2・proposal・specs・tasks・go.json が、Q2 の推奨（端末ごと・ST56 が作る・PERM-10 に ★）のまま（印のみ）
- 行き先: leave: 次の `spec --answers Q2` の段が design D2・proposal・specs・tasks・go.json を写す（graph が `deep_draft.py status` の diverged Q2 で回す）。R9 の処置も同じ段で直る。tasks は未着手で失うものは無い。`deep.md` の Q2 にも「要件へ戻すもの」と、PERM-10 の★が要らなくなったことを書いた
- 重大度: 中
- 文書: openspec/changes/st44-collector-credential-scope/design.md:42
- 要件: FR-128, PERM-10
- 根拠: 改訂後の FR-128 ★「端末ごとには発行しない」。change はまだ推奨の側:
  - `design.md:27`（Non-Goals）「端末ごとの資格情報の発行・取り消しと、どの資格情報で届いたかを…書く欄（ST56。D2）」、`:42` D2 の見出し「端末ごとの発行は ST56 が作る（深掘り Q2 の推奨）」、`:45-47`（ST56・ST37 への申し送り）、`:48`「ST56 までに届いた記録は…（深掘り Q2 の推奨の irreversible）」、`:49-50`（PERM-10 の ★ を record の段が足す —— `deep.md` Q2 は「要らなくなった」と書く）、`:157`（Risks「ST56 が端末ごとの資格情報を作る（D2）」）
  - `proposal.md:23`（「ST56 の端末ごとの資格情報など」）、`:26`「端末ごとの発行・取り消しと…は ST56 が作る」、`:65-66`（申し送り ST56・ST37）
  - `specs/data-sensitivity/spec.md:254`「端末ごとの発行と取り消しは ST56 が作る」（正典に入る導出元の文）
  - `tasks.md:23`「Q2 配り方は共有の 1 本のまま。端末ごとの発行は ST56」
  - `go.json:506`・`:511`・`:524`・`:542`・`:716`・`:1820`（「端末ごとの発行は ST56 が作る」）
  - `review/spec.md:95-103` R9 の処置「推奨のまま答えが来たら record の段が PERM-10 の★…を足す」（escalated のまま）
  tasks は 13 件中 `- [x]` が 0 件で、文書の側で失われるものは無い
- 直すところ: leave（次の `spec --answers Q2` の段が写す。C2 の R10・R11 も同じ段で）
- 機械: 安く捕まえられる（`review_triage.py` の逆向き —— deep.md の「要件へ戻すもの」に挙がった要件の最新 ★ が、change の design / specs の最終更新より新しければ warn）

## C4. FR-128 の「設定のしかた（V-01 が示す符号を読み取る等）…は ST29 / ST44 の深掘りで決める」の宛先が消えた
- 行き先: requirements（FR-128 ★ 2026-11-02 に「設定のしかた（V-01 が共有の値を符号などで示すか）は決めていない —— ST56 の深掘りで決める」を足した）
- 重大度: 中
- 文書: docs/requirements.md:1075
- 要件: FR-128
- 根拠: FR-128 ★ 2026-10-25（`:1075`）「**設定のしかた**（V-01 が示す符号を読み取る等）と**資格情報の配り方**は、ST29 / ST44 の深掘り（`docs/handoff/ST29.md` の Q4）で決める」。
  ★ 2026-11-02（`:1076-1078`）は配り方だけを決め、設定のしかたの宛先を書かない。ST29 は archive 済みで、`openspec/changes/archive/2026-10-27-st29-satellite-credentials/` に FR-128・符号の語は 0 件。ST44 の `deep-questions.json` の問いは Q1〜Q3 で設定のしかたを問わず、`deep.md` の「要件へ戻すもの」も配り方だけ。`docs/handoff/ST29.md` の Q4 は「収集側の資格情報が読める範囲」で設定のしかたを持たない。
  さらに ★ の「発行・取り消しの画面は作らない」と base の「V-01 が示す符号」は、V-01 が共有の `API_TOKEN` を符号として示してよいのかを決めていない（示すなら、画面のログインで入れる人に収集アプリの合言葉が見える。示さないなら本人が値を手で写す）。ST56 の上流は「ST29 / ST44 で決まった」と読んで決まった答えを探す
- 直すところ: requirements（★ 2026-11-02 に「設定のしかた（V-01 が共有の値を示すか）は ST56 の深掘りで決める」を足す）
- 機械: 安く捕まえられる（check_chain: 要件の本文の「ST<NN> の深掘り…で決める」の ST<NN> がすべて archive 済みで、その要件の後の ★ が同じ句を名指さなければ warn）

## C5. PERM-10 の本文と ★ 2026-10-11 が、収集側の資格情報の配り方を未決のまま書いている
- 行き先: requirements（PERM-10 に ★ 2026-11-02 補足「収集側の配り方は、携帯と PC で共有する 1 本に決まった（ST44 の深掘り Q2。FR-128 ★ 2026-11-02）」を足した）
- 重大度: 低
- 文書: docs/requirements.md:1194
- 要件: PERM-10, FR-128
- 根拠: PERM-10 ★ 2026-09-08（`:1180`）「資格情報の配り方（共有の合言葉 / 呼び出し元ごとの資格情報）は ST29 の深掘りで決める」、★ 2026-10-11（`:1194`）「**収集側の資格情報の配り方（従来の ST29 の Q4）の宛先は /stories で決める**」。
  決定は FR-128 ★ 2026-11-02 にしか無い。ST44 が満たすのは PERM-10 だけ（`stories.json` の ST44 の satisfies）で、ST44 の完了の判定も「（配り方は深掘りで決める。handoff/ST29.md の Q4）」のまま。PERM-10 から読むと配り方は未決に見え、決まった先（FR-128）への参照が無い。`deep.md` Q2 は「PERM-10★ 2026-10-11 の『端末は送り手の申告のまま』は変えない」としたが、同じ ★ の宛先の文は触れていない
- 直すところ: requirements（PERM-10 に ★ 2026-11-02 で「収集側の配り方は共有の 1 本（FR-128 ★ 2026-11-02。ST44 Q2）」を 1 行）
- 機械: できない（どの ★ が「後で決める」を閉じたかは散文）

## C6. FR-128 ★ が、選んだ側の帰結（締め出すたびに全部の端末が止まり、携帯は上限を越えた分を捨てる）を持たず、ST56 の「記録を捨てずに」とつながらない
- 行き先: requirements（FR-128 ★ 2026-11-02 に、選んだ側の帰結 —— 偽の記録を後から見分けられない・締め出すたびに全部の端末が設定し直すまで送れず、C-01 の未送信が FR-8 の上限を越えた分は捨てる —— を 1 文で足した）
- 重大度: 低
- 文書: docs/requirements.md:1078
- 要件: FR-128, FR-8
- 根拠: ★「落とした端末を締め出すには、値を変えて全部の収集アプリに設定し直す」。選んだ選択肢の irreversible（`deep-questions.json` Q2 options[1]）「偽の記録を後から見分けられない。締め出すたびに全部の端末が入れ直しになり、その間に溜まりすぎた分は捨てる」、context「端末は断られたことを本人に知らせず（知らせるのは ST56）、途絶の問い合わせも同じ値なので途絶の通知も出ない。気付く手がかりは画面の格子の途絶だけ」。
  ★ には帰結の 2 つ（偽の記録を見分けられない・全端末が止まり C-01 は FR-8 の上限で捨てる）が無く、`deep.md` Q2 の「帰結」行にだけある。ST56 の完了の判定「記録を捨てずに設定し直しを促す」は、値を変えた後に本人が気付くまでの間も FR-8（90 日 / 2 GB）が掛かることを持たない
- 直すところ: requirements（★ に帰結を 1 文）か leave（`deep.md` と `deep-questions.json` が持ち、ST56 の上流は FR-8 を読む）
- 機械: できない

## 未確認

- st44 の次の `spec --answers Q2` の段が `review/spec.md` を書き直すか（R9〜R11 の処置を変えるか）。段の定義（harness2 側）を読んでいない。書き直さないなら C2 の配送は archive の後も残る
- ST37 の上流がいま着手の候補になっているか（`scripts/hx status` を見ていない）。C2 の重大度はこれに依る
- `docs/briefs/ST44-pr.md:15`・`ST44-deep.html` 等の推奨の文は、グラフが段ごとに作り直す資料なので中身を確かめていない
- 正典の Scenario のテストの印は全件は見ていない（この改訂は正典の Requirement も Scenario も変えない）

## 触らなくてよいと判断したもの（理由つき）

- **`docs/stories/INDEX.md:79`（ST56 の行）**: 題・layer・満たす要件・前提・扉（—）で、配り方を固定していない。`device-collection`・`desktop-collection`・`data-sensitivity`・`distribution` に ST56 を置く割り当て（`:92`・`:93`・`:100`・`:105`）も、共有の値を設定させる口と取り消し後の振る舞いで成り立つ
- **`docs/stories/ST56.md:22-24` の写しの切れ**: 「落とした端末を締め出すには…」の 3 行目が「…（続きは requirements.md）」で切れるのは `make_story.py` の形どおり。2 行目までに「端末ごとには発行しない」「発行・取り消しの画面は作らない」が載る
- **ST29（archive 済み）・`docs/handoff/ST29.md`**: ST29 は衛星の資格情報の Story で、PERM-14 の「1 つの衛星に複数の資格情報を発行してよい（端末ごとなど）」（`requirements.md:1218`）は衛星の話。ST29.md の Q4 は読める範囲の申し送りで、ST44 Q1 が受けた
- **ST37 の Story と `docs/handoff/ST37.md`**: 完了の判定「書き手と利用者識別子がサーバの側で資格情報から決まり」は共有の 1 本でも成り立つ（収集アプリの書き手は「本体」。PERM-10 ★ 2026-10-11）。handoff の 2 項（ST17 の口・衛星の書き手の粒度）は収集アプリの配り方に依らない。問題は st44 から届く R11（C2）
- **ST57**: FR-112〜FR-116 と ST57 の本文に資格情報・端末の語は無い。更新の確認は配布元への通信で、サーバの合言葉を使わない
- **st52-release-channels**: `premises.md:22` P15「設定させる口は ST56（FR-128）」、`specs/distribution/spec.md:176-200`（公開する APK は資格情報を持たない）は配り方に依らない
- **`docs/network.md` §4（電話を落としたとき）**: 「`.env` の `API_TOKEN` を変えて、APK と C-02 を入れ直す」は共有の 1 本の手順と合う。古い「ST29 まで全読み」の文は st44 の tasks 7.2 が書き直す
- **正典 `openspec/specs/data-sensitivity/spec.md:124`**「呼び出し元ごとの範囲は PERM-10 の配り方として後で決める」: st44 の delta が要件ごと REMOVED する（`specs/data-sensitivity/spec.md:3`）
- **`docs/ui-direction.md:23`（S-7 の「資格情報の発行」）**: 衛星の資格情報（PERM-14・ST29）で、収集アプリの発行の画面を前提にしていない
- **`docs/production-prep.md:73`（鍵・資格情報の保管＝Windows の資格情報ストア）**: 記録先は扉 #24（バックアップ暗号鍵の保管方式。`requirements.md:1725`）で、収集アプリの合言葉の置き場を述べていない。C-02 が `.cmd` に平文で書くこと（`crates/collector-windows/src/autostart.rs:17,34`）は FR-128 ★ 2026-10-25 が既に書き、ST56 の完了の判定が受けている
- **★ の事実（コードで確かめた）**: サーバの収集アプリの資格情報は 1 つの環境変数 `API_TOKEN`（`crates/server/src/lib.rs:4331`）、呼び出し元の種類は `WebSession`・`ApiToken`・`Satellite` の 3 つで収集アプリは 1 種類（`crates/server/src/web_session.rs:41-49`）、端末は送られた欄 `device_id`（`crates/server/src/ingest.rs:21`）、読み出しの記録が持つ資格情報の識別子は衛星の分だけ（`migrations/202610241200_satellite_credentials.sql:68`）、C-01 は `buildConfigField` で焼き込む（`collector-android/app/build.gradle.kts:57-61`）。どれも ★ と食い違わないので K は無い
- **語の揺れ**: この改訂は概念の名前を変えていない（「収集アプリの資格情報」「共有の 1 本」は deep.md・deep-questions.json と同じ語）
