# ST21 深掘りの独立レビュー（deep-review）

- 対象: `openspec/changes/st21-place-registry/deep-questions.json`（Q1〜Q4）/ `deep.md`（C1〜C12・確かめたが問わなかったこと）/ `proto.html`
- 入力: `docs/stories/ST21.md`、`docs/stories/INDEX.md`、`docs/requirements.md`（§1 / FR-25〜FR-31 / FR-37 / FR-44〜FR-51 / FR-58 / FR-76 / PERM-2〜PERM-9 / §5 扉 #15 #17）、
  `docs/ui-direction.md`（S-2 / S-6）、`openspec/specs/personal-entities/spec.md`、ST16 / ST19 の archive、走っている change（st05 / st06 / st08 / st12 / st22 / st28）と
  上流の worktree（`../ashiato2-up-st25`）、`docs/handoff/ST23.md`、既存コード
- 進め方: schema の deep の手順 1〜5 を一覧を見ずに先にやり、その後で突き合わせた

## 手順ごとの記録（一覧を見る前にやったこと）

- **手順 1（要件どうしの衝突）**: FR-48 と FR-49 のあいだに衝突は無い。要件の外側と食い違うものが 3 つあった。
  (a) PERM-2（すべての記録に感度）に対して、場所の既定を決めている行が PERM-3〜PERM-6 に無い → 一覧の Q3 にある。
  (b) 滞在と場所を結ぶ要件も Story も無い（FR-37 は日と滞在、FR-47 は滞在だけを指す）。それなのに ST21 の価値（「過去の紐づけが全部外れる」）、扉 #17 の本人の言葉、QS-1 / QS-3 / QS-4 はこの結び付きを前提にしている → 一覧の Q4 にある。ただし要件へ戻す先が書かれていない（R9）。
  (c) 場所を記録として持つなら FR-50 と FR-51 が及ぶが、場所を消す操作を持つ Story が無い（R2）
- **手順 2（扉 #17 の幅）**: 「座標が変わる」が訂正を指すのか移転を指すのか。「自宅」を役割と見るか地点と見るか → Q2。
  ほかに、重複した場所をまとめる操作（片方の識別子が消える）→ C6。識別子を名前や座標から導くか → C1。どちらも C に置いてあり、置き方に異論は無い
- **手順 3（新たに立つ一方通行）**: 捨てるもの＝名前や座標を上書きした前の値 → C2（積む）。
  列の型・鍵の入力＝原文は `core.event.raw` が text（`migrations/202609092315_raw_text.sql:28`）なので受け取ったまま残り、`payload` の jsonb は数値を numeric のまま持つので座標は丸まらない（C3 / C11）。
  内容の鍵は SHA-256(ソース, 出来事の時刻, 原文) で、乱数を原文に入れる → C11。
  外に出すもの＝地図タイル → C10、外部 AI → Q3、**ブラウザの位置 → 一覧は「網の外へは出ない」と書いているが、事実と違う（R3）**
- **手順 4（日常）**: 該当なし（確かめた範囲: 端末側のコードに変更は無い。S-6 は毎日開く画面ではない。毎日目に入るのは S-2 の見出しだけで、それは Q4 にある）
- **手順 5（既存コード）**: 場所を扱うコードは無い（`grep -rniw "place\|places\|place_id"` を crates / migrations / web/src / collector-android / tools に掛けた。当たったのは `tools/seed.sh:76` の合成データの関数 `place(k)` だけ）。
  deep.md の引用 `crates/server/src/lib.rs:344`（`default_sensitivity` が論理ソースで分かれている）、`web/src/MasterView.tsx:102-113`（タブは 1 つ）、`migrations/202609160220_personal_attributes.sql`（錠は `s01-attribute` にしか掛かっていない。:99-104 / :176 / :225）は、どれもコードと一致した。
  `personal-entities` の spec には「タブは『個人属性』の 1 つだけ」という Scenario がある（`openspec/specs/personal-entities/spec.md:555`）。ST21 はこれを MODIFIED にする必要がある（deep.md は ST19 の方針として触れているので、指摘にはしない）

## proto の実測の再現（Q1 の why）

`chrome-headless-shell`（playwright 同梱の 1243。`../ashiato2/web` の `@playwright/test` で起動）で `proto.html#<hash>` を読み、`#mHeight` / `#mScreens` を DOM から取った。**6 つとも一致した。**

| hash | 縦 px | 画面 | Q1 の値 |
|---|---|---|---|
| `vol=one` | 2,895 | 4.5 | 2,895（4.5） |
| `vol=one&cmax=all` | 5,392 | 8.4 | 5,392（8.4） |
| `vol=five` | 5,665 | 8.9 | 5,665（8.9） |
| `vol=five&cmax=all` | 18,741 | 29.3 | 18,741（29.3） |
| `vol=one&skel=regonly` | 1,315 | 2.1 | 1,315 |
| `vol=five&skel=regonly` | 4,085 | 6.4 | 4,085 |

---

## R1. Q4 の context「ST25 はまだ上流が始まっていない」は事実と違う。ST25 は S-2 の見出しを時刻の範囲のまま動かさない前提で深掘りを書いており、推奨の「ST25 へ申し送る」と組み合わせると、2 本とも相手に任せたことになる
- 成果物: openspec/changes/st21-place-registry/deep-questions.json
- 根拠: deep-questions.json:81（「1 日の一覧を作り直す ST25（FR-56。まだ上流が始まっていない）なら重ならない」）/ `git branch -a` に `docs/st25-upstream`、worktree `../ashiato2-up-st25` に `openspec/changes/st25-day-timeline/`（deep-questions.json / deep.md / proto.html。2026-09-30 20:46）/ `../ashiato2-up-st25/openspec/changes/st25-day-timeline/deep-questions.json:17`（「決着済みで動かさないもの: 行の見出しは時刻の範囲（場所の名前は ST21 まで無い）」）/ deep.md:102-104（走っている Story の一覧に ST25 が無い）
- kind: premise
- 提案: Q4 の context を直す（ST25 は上流の深掘りを書いている最中で、見出しを動かさないことにしている）。ST21 が推奨を採るなら、ST25 の答えが記録される前に S-2 の見出しの担当をどちらが持つかを揃える（ST25 への申し送りを今置く、またはどちらかの問いに入れる）。第 2 選択肢は ST22 だけでなく ST25 とも `browsing-views` で重なる、と書き足す。deep.md の「走っている Story との重なり」に ST25 を足す
- 処置: fixed deep-questions.json — Q4 の context を直した（ST25 は上流の深掘り中で、見出しを時刻の範囲のまま置く前提）。推奨を「ST21 も ST25 も見出しを触らず、ST22・ST25 の archive 後に fix/ で場所の名前にする」に直し、第 2 選択肢に ST25 との重なりを書いた。deep.md の走っている Story に ST25 を足した

## R2. 「場所は ST22 / ST23 の削除に乗せる」「重複は片方を消したことにする（ST22）」には受け手が無い。ST22 が消せるのは滞在だけで、ST23 への申し送りも個人属性の主張しか扱っていない
- 成果物: openspec/changes/st21-place-registry/deep-questions.json
- 根拠: deep-questions.json:13（Q1 の context「場所の名前・座標も『本人が書いた』記録として ST22 / ST23 の削除に乗せる」）/ deep.md:90（C6「片方を消したことにする（ST22）」）/ openspec/changes/st22-record-deletion/deep.md:102（C3。消す口は滞在と連鎖だけ）/ docs/handoff/ST23.md（場所の項が無い。`grep -n "場所\|ST21" docs/handoff/*.md` は 0 件）/ docs/stories/INDEX.md:216
- kind: premise
- 提案: C6 と Q1 の context を事実に合わせる。場所を消す操作（FR-50）と本文の消去（FR-51）を持つ Story を決める。ST21 が S-6 に消す操作を足すのか、`docs/handoff/ST23.md` に場所の項を置いて申し送るのか。申し送るなら、C6 の「重複は消す」は ST23 まで使えない（その間、重複と打ち間違えた自宅の座標が S-6 に残る）と明記する
- 処置: fixed deep.md C13 — 場所を消す口は ST23 へ申し送る（deferred ST23。spec の段で docs/handoff/ST23.md に書く）。C6 と Q1 の context に「ST23 まで重複と打ち間違えた座標は残る」と書いた

## R3. proto の軸 4 は「この端末のいまの位置」を「ブラウザの位置（網の外へは出ない）」と説明しているが、ブラウザの位置取得は位置の提供者（OS かブラウザの事業者）へ問い合わせる。しかもいまの http の画面では使えない
- 成果物: openspec/changes/st21-place-registry/proto.html
- 根拠: openspec/changes/st21-place-registry/proto.html:268（`d:"ブラウザの位置（網の外へは出ない）"`）/ deep.md:95（C10 が座標の入れ方の 1 つに「この端末のいまの位置」を挙げている）/ collector-android/app/src/main/kotlin/dev/ashiato/collector/FixSource.kt:28（C-01 は既に `FusedLocationProviderClient` = Google Play 開発者サービスを使っている）/ openspec/changes/st28-private-network-only/design.md:165（画面が https になるのは ST28 から。Geolocation API は安全な文脈でしか動かない）
- kind: premise
- loss: exported
- 提案: 軸 4 の説明を直す。PC のブラウザでは、周りの Wi-Fi などを Google か Microsoft の位置サービスへ送って位置を得る。スマホでは C-01 と同じ OS の位置の経路を通る。何が、登録するその時に外へ出るのかを書く。ST28 が入るまで「いまの位置」は動かないことも書く。これを選択肢に残すかどうかは本人に任せる（聞くなら `irreversible` の欄に「登録した時点の位置の問い合わせが提供者へ出る」と書く）
- 処置: escalated — 本人に Q1 の軸 4 で選ばせる（deep.md の R3 / Q1）。proto.html の軸 4 の「いまの位置」の説明を、押したときに位置の提供者へ問い合わせること・ST28 まで動かないことに直した。Q1 の選択肢の irreversible にも書いた。選択肢は残し、本人に任せる

## R4. 「地図は本体の範囲外（§1）」は §1 の読み過ぎ。§1 が範囲外にしているのは地図の衛星アプリで、本体の画面に地図を置くことは ui-direction で一度は候補になっている
- 成果物: openspec/changes/st21-place-registry/deep-questions.json
- 根拠: deep-questions.json:12（Q1 の why「地図は本体の範囲外 §1」）/ proto.html:138 / :165 / :372 / deep.md:94（C10）/ docs/requirements.md:49（「衛星アプリ（旅行実績・ランキング・分析・地図）は本システムの範囲外」）/ docs/ui-direction.md:139（主表現の軸に「地図」）/ :212（presets「地図から」）/ :243（見本に地図のピンがあった）
- kind: premise
- 提案: 地図を使わない理由を「外のタイルを取ると、見ている座標がその事業者へ出る」の 1 本にし、§1 を根拠から外す（C10 は「使わない側が扉を開けたまま」なので、C のままでよい）。Q1 の why にある「地図を出せないので、滞在から見分ける」という前提は、タイルを手元に置く地図（外へ出ない）が選択肢に無いことを明記した上で書く
- 処置: fixed deep.md C10 — 地図を使わない理由をタイルの事業者へ座標が出る 1 本にした。Q1 の why と proto の文面からも §1 を外し、手元にタイルを置く地図を選択肢にしていないことを書いた

## R5. Q2 の不可逆の根拠「移転の日は記録のどこにも無い」は事実と違う。位置の記録が残っている期間は、移転の日を後から計算できる。第 3 選択肢の不可逆は、要件にまだ無い紐づけを前提にしている
- 成果物: openspec/changes/st21-place-registry/deep-questions.json
- 根拠: deep-questions.json:29（「後から『あれは移転だった』と分かっても、いつ移ったかは記録のどこにも無い」）/ :30（C2 により前の座標は書いた時刻つきの行として残る）/ docs/requirements.md:315（FR-31: 滞在は原文から作り直せる）/ proto.html:246（proto の職場の移転は、前の座標の滞在と後の座標の滞在として描かれている。境目がデータから読める）/ deep-questions.json:47（第 3 選択肢の不可逆「主観・人物・問い合わせの答え」）に対し、docs/requirements.md:440 / :472（FR-37 / FR-47 が指すのは日と滞在だけ）、deep.md:92（C8: 手で付ける紐づけは作らない）、deep-questions.json:86（Q4 の推奨: 照合は保存しない）
- kind: irreversible
- loss: uncaptured
- 提案: `loss` に書くものを、実際に取れないものへ絞る。訂正か移転かという本人のその時の区別と、位置の記録が無い期間（記録を取る前・行っていない場所）の移転の日だけが対象になる。それ以外の移転の日は、前後の座標での滞在の境目から計算し直せる、と context に書く。第 3 選択肢の不可逆は「場所の識別子を指す紐づけが後から足されたら」という条件つきであることを書く。絞った後で失われるものが本人の判断だけになるなら、A のまま問うのか、B（推奨を既定）に下げるのかをもう一度判定する
- 処置: escalated — 残る uncaptured（本人のその時の区別と、位置の記録が無い期間の移転の日）を Q2 で本人に問う（deep.md の R5 / Q2）。Q2 を A（irreversible / uncaptured）から B（open。推奨は変えるたびに聞く）へ下げた。why に「位置の記録がある期間は滞在の境目から計算し直せる」を、第 3 選択肢の不可逆に「場所を指す紐づけが後から足されたら」の条件を書いた

## R6. Q1 の軸 7 は Q2 と同じことを聞いている。同じ判断を 2 回させることになり、2 つの答えが食い違ったときの読み方も要る
- 成果物: openspec/changes/st21-place-registry/proto.html
- 根拠: proto.html:159（「軸 7 — 座標を変えるときに聞くこと（Q2 と同じ答えにする）」）/ deep-questions.json:4（「Q1 の軸 7 と Q2 は同じことを聞いています —— Q2 の答えを正とします」）/ proto.html:250（軸 7 の値で候補の並びが変わる）
- kind: technical
- 提案: 軸 7 をコントロールから外して、画面の状態（入力）として扱う。Q2 の選択肢を切り替えて描けるようにしておけば、Q2 を読むときの絵として使える。または出力の「指示をコピー」から軸 7 を外す。こうすれば、人間が 1 つの判断を 1 回だけすれば済む
- 処置: fixed proto.html — 軸 7 を「Q2 の選択肢ごとの見え方（入力。軸ではない）」にして、指示の出力から外した

## R7. Q3 の推奨の説明「外部 AI は滞在の座標は見えるが、そこが『自宅』だとは知らない」は守りを大きく書いている。滞在だけで自宅は当てられる
- 成果物: openspec/changes/st21-place-registry/deep-questions.json
- 根拠: deep-questions.json:62 / docs/requirements.md:635（PERM-3: 滞在は「外部 AI に出してよい」）/ proto.html の合成データ（自宅 4,710 時間。夜を通して居る所は 1 か所）
- kind: premise
- 提案: 第 1 選択肢の detail を、実際に隠れるものに直す。隠れるのは本人が付けた名前の文字列と、滞在の無い登録（まだ行っていない場所・記録を取る前の実家）の座標で、「どこに夜いるか」は滞在から外部 AI が推し量れる。Q3 は B のままでよい（感度は `personal_attributes.sql` の錠でも書き換えを通す列で、外へ出る経路は ST27 まで無い）
- 処置: fixed deep-questions.json — Q3 の第 1 選択肢の detail を、隠れるもの（本人が付けた名前と滞在の無い登録の座標）と、滞在から推し量れること（夜に居る所）に直した

## R8. C9 と Q4 の推奨は滞在（ST16 / `derived-records`）の上に作るが、ST21 の requires は ST01 だけになっている
- 成果物: openspec/changes/st21-place-registry/deep-questions.json
- 根拠: docs/stories/ST21.md:5（`requires: [ST01]`）/ docs/stories/INDEX.md:43 / deep.md:93（C9: 名前の無い所は滞在の代表点から作る）/ deep-questions.json:85（照合は滞在の代表点を使う）/ crates/server/src/stay_store.rs:176（滞在の材料の読み方）
- kind: technical
- 提案: 推奨どおりに進むなら、ST21.md と INDEX の requires に ST16 を足す（ST16 は archive 済みなので admission は変わらない）。第 3 選択肢（照合を作らない）を採り、かつ C9 も外すなら、今のままでよい
- 処置: fixed deep.md — 「要件へ戻すもの」に stories.json の ST21 の requires へ ST16 を足すことを挙げた（record の段で再生成する）

## R9. Q4 の照合（場所ごとの滞在の合計）は、FR-48 / FR-49 から導けない新しい振る舞いだが、「効く先」と「要件へ戻すもの」に requirements.md が無い
- 成果物: openspec/changes/st21-place-registry/deep-questions.json
- 根拠: deep.md:75（Q4 の効く先は specs / design / handoff だけ）/ deep.md:111（要件へ戻す候補は FR-49 と PERM だけ）/ docs/requirements.md:473-474（FR-48 / FR-49 は照合について何も言っていない）/ docs/requirements.md:805-808（QS-1 / QS-3 / QS-4 は滞在と場所の組を使う）
- kind: technical
- 提案: 照合を ST21 で作る（第 1・第 2 選択肢）なら、要件へ戻すものの候補に「滞在を場所に照らす」を足す（FR-48 に ★ を付けるか、新しい FR を立てる）。こうしないと、`personal-entities` の Requirement の導出元に FR が無くなる。第 3 選択肢なら不要
- 処置: fixed deep.md — 「要件へ戻すもの」に、Q4 で照合を作るなら FR-48 の ★ か新しい FR を挙げた
