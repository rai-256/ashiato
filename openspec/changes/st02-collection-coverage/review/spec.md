# ST02 spec レビュー（独立。書いた者の意図を知らない目で、成果物どうしの整合だけを見た）

対象: `openspec/changes/st02-collection-coverage/`（specs/ 2 本・design.md・tasks.md）。
正典は `openspec/specs/`、要件は `docs/requirements.md`、Story は `docs/stories/ST02.md`。

> **id の付け方**: 呼び出し元は `S<n>` を求めたが、`scripts/review_triage.py` は
> `^## R<n>\.` しか指摘として数えない（`ENTRY` 正規表現）。`S<n>` で書くと
> **19 件の指摘が機械から見えなくなる**ので、見出しは `R<n>`、`- id:` に `S<n>` を併記した。
> `kind` も triage の語彙（technical / conflict / irreversible / daily / premise / defer）に合わせ、
> 呼び出し元が指定した細目（missing / unverifiable / misplaced / nit）は `- 種別:` に置いた。

## 機械の検査（2026-09-11 実行、そのまま写す）

```
$ openspec validate st02-collection-coverage --strict
Change 'st02-collection-coverage' is valid                                  # rc=0

$ python3 scripts/check_chain.py .
  要件 112 件 / Story 36 本 / 扉 26 項
  [ok]   どれかの Story に拾われた要件: 110/112 件
  [ok]   INDEX の「Story の対象外」: NFR-11, NFR-8
chain: OK (0 件 / 未回収 0 件 / warn 0 件)                                    # rc=0

$ python3 scripts/check_scenarios.py . st02-collection-coverage
  Scenario 63 件 / 印 33 個 / 担保あり 30 / 人間の確認待ち 0
  [FAIL] 担保の無い Scenario: 33 件（ST02 が新設した Scenario のうち 33 件すべて）
scenarios: FAIL (担保なし 33 件)                                             # rc=1

$ python3 scripts/review_triage.py . st02-collection-coverage
  指摘 12 件 / 要件へ戻すもの 7 件
triage: OK                                                                   # rc=0
```

`check_scenarios` の FAIL 33 件は**実装前なので当然**（上流の成果物しかない）。
ただし **33 件でなく 34 件出るべきだった** —— R1 を見ること。
`review_triage` の「要件へ戻すもの 7 件」は `docs/requirements.md` の本文で
FR-35 / FR-54 / FR-78 / FR-79 / FR-80 / NFR-13 の ★ 印を確認済み（各要件の本文に
`★ 2026-09-10` がある）。**要件への戻しは漏れていない。**

---

## 観点ごとの確認結果

### 観点 1: deep の決定が正典に写っているか

**16 件中 15 件は specs / requirements に写っている。**（Q1・Q2・Q3・Q5・Q6 は
第 4 回で上書きされた分を含めて確認し、**古い側が Scenario に残っていないこと**も見た ——
Q4 の「写真だけを切り出す」は delta spec:216-225 に残っておらず、
Q6 の「セルを選ぶと」は delta spec:270「行を選ぶ」に置き換わっている。）

数値を持つ決定は Scenario に数値が入っている: `Asia/Tokyo` の境界（delta spec:29-30 に
`14:59:59Z` / `15:00:01Z`）、350（同:259-262 に 360/355/352/351/349）、
24 × 24 CSS px と 360 px（同:303-307）。**入っていないのは明度の段差だけ**（R9）。

写っていない 1 件:

- **Q12（FR-35 を「最後の記録 または 最後の生存信号」で測る／想定間隔に写真とウィンドウを足す）**
  —— 要件へは戻っている（`docs/requirements.md:165-174` に ★ 印つき）が、
  ST02 の specs / tasks には**写真・ウィンドウ・ブラウザ履歴の想定間隔を登録簿に入れる作業が無い**。
  FR-35 の通知は ST14 の担当なので通知そのものは範囲外だが、
  **途絶の判定（delta spec:130）も NFR-13 の利用主語 3 ソース（同:218-221）も
  これらのソースの `expected_gap_sec` が入っていることを前提にしている**。
  tasks 7.1（tasks.md:91-93）は「位置・写真とも 6 時間」までしか触れていない。
  → R4 と同じ場所の穴なので、R4 の提案に含めた

なお **Q10（Doze は ST01 の R46 に従う）** は device-collection の根拠ブロック
（delta device-collection/spec.md:19-21）と tasks 7.1 で受けており、
「ST02 で決め直さない」という決定なので Scenario が無いことは正しいと読んだ。

### 観点 2: Scenario が検証可能か

63 件のうち ST02 が新設したのは 34 件。判定不能・曖昧なものは R9 / R10 / R13 に挙げた。
AND で 2 つの主張を束ねているものが 5 件あるが（delta spec:57-61, 63-67, 287-291,
303-307, 309-313）、いずれも片方が他方の前提になっており**片方だけ通って緑になる形ではない**ので
指摘にしていない。

Requirement の本文が言っているのに Scenario が言っていないもの:

- 「生存信号に利用者識別子を持たせる」（delta spec:80）→ 末尾の Requirement
  （同:315-329）が表の列として受けているので重複と見た。指摘にしていない
- 「登録簿に無いソースからの生存信号を拒否する」（同:46）→ Scenario あり（同:69-72）
- 「取得できない状態を報告する生存信号について、何が満たされていないかを残す」（同:44）
  → Scenario あり（同:63-67）。ただし**理由が空の信号を断る**側は tasks にしかない（R8）

### 観点 3: 置き場の誤り

design の D番号のうち観測可能な振る舞いを述べているもの: **D6（R7）・D7（R4 / R5 / R6）・
D9（R8）・D10（R9）・D12（R10 / R11）**。D1〜D5・D8・D11 は表の形・列・定数の置き場で、
design に置いて正しいと読んだ。実装の名前が specs に入っている件は R14。
proposal の Capabilities（proposal.md:150-181）と `specs/` の 2 ディレクトリ、
および `docs/stories/INDEX.md:72` の capability 表は**一致している**
（`collection-coverage` の積む Story に ST01 が入っている点も反映済み）。

### 観点 4: tasks が検証を持つか

コマンドと終了条件の欠けは R19。**依存順は妥当**（1 の表の作り直し → 2 の日境界 →
3/4 の受け口と保護 → 5 の導出 → 6 の集計 → 8 の画面、の順で、
鍵になる `core.coverage` の作り直しが最初に来ている）。
人間の確認待ちは 10.4 に書かれているが機械から見えない（R2）。
未決の 11.1 が「他は先に進める」と明記されているのは正しい形だと読んだ。

### 観点 5: Story が要件の現在の本文と一致しているか

`check_chain.py` の再生成との一致は **[ok]**（上の実行結果）。
`docs/stories/ST02.md:78-84` の完了の判定 5 項目は、FR-54 の 7 状態・
NFR-13 の「5 本すべてが 350 以上」・NFR-19（セルは表示専用）と**矛盾していない**
（第 4 回 Q9 で「単数で書かれていたのを直す」とした箇所も反映済み）。
`satisfies` の外に出ている要件は R15。

**1 件だけ、完了の判定が実際には見えない可能性がある** —— NFR-13 の利用主語 3 ソースのうち
ウィンドウとブラウザ履歴の生存信号は C-02 が送るもので、proposal.md:168-171 が
**意図的に ST07 / ST08 に残している**。したがって ST02 完成時点の画面では
この 2 ソースの達成日数が常に 0 になる。判断としては妥当だが、
specs にも tasks にも Story にも但し書きが無く、**完了の判定を見た人が「壊れている」と読む**。
R15 の提案に添えて 1 行残すのがよい。

### 該当が無かったもの

- **deep の「要件へ戻すもの」の戻し漏れ**: 該当なし（第 1〜3 回の 2 件と第 4 回の 6 件を
  `docs/requirements.md` の本文で確認。FR-35:169 / FR-54:248 / FR-78:178,186 /
  FR-79:193 / FR-80:200 / NFR-13:382,391 に ★ 印がある）
- **ST01 の申し送りの取りこぼし**: 「生存信号は Doze より細かくできない（実測 14.2 分）」は
  delta device-collection/spec.md:19-21 が受けている。「日境界は ST02 の担当」は Q2 で閉じている。
  「重複再送の `event_count`」は ST01 の下流が直し、正典に固定済み
  （`openspec/specs/collection-coverage/spec.md:10-26`）。
  MODIFIED の見出しは正典の見出しと**文字列として一致**しており、
  ST01 が固定した 3 つの SHALL 行も落ちていない。残る 1 件が R18

---

## 指摘

## R1. 同名の Scenario が正典に既にあり、生存信号の冪等性が「担保あり」に化けている

- id: S1
- 成果物: openspec/changes/st02-collection-coverage/specs/device-collection/spec.md
- 種別: unverifiable
- 根拠: 新設 `#### Scenario: 再送しても重複しない`（delta device-collection/spec.md:39-42、
  中身は「同じ**生存信号**を 2 回送る → S-01 に残る生存信号は 1 件」）が、
  正典の同名 Scenario（`openspec/specs/device-collection/spec.md:72-75`、中身は**記録**の重複）と
  **文字列として同一**。`check_scenarios.py:44-48` は Scenario 名を正規化して `setdefault` で
  辞書に入れるので後勝ちが消え、`tools/smoke.sh:48` の印（`/ingest` の重複判定）1 つで
  両方が担保済みになる。実測: FAIL 33 件の一覧にこの名前だけ出てこない
- kind: technical
- 提案: delta 側の名前を「同じ生存信号を再送しても重複しない」等に変える。
  併せて、この Scenario は**サーバ側に残る行数**を主張しているので device-collection ではなく
  collection-coverage の「同じ生存信号を 2 回送っても 1 行」（delta:91-94）と重複している ——
  片方を消すか、端末側は「再送する」だけを主張する形に切り分ける
- 処置: fixed specs/device-collection/spec.md —— 同名 Scenario を削除し、端末側は「再送する／同じ冪等キーを持つ」だけを主張する形に切り分けた。サーバ側に残る行数は collection-coverage の「同じ生存信号を 2 回送っても 1 行」が持つ

## R2. tasks.md に「人間の確認待ち」の見出しが無く、実機判定の 2 件が永久に FAIL する

- id: S2
- 成果物: openspec/changes/st02-collection-coverage/tasks.md
- 種別: missing
- 根拠: `check_scenarios.py:32,53-62` は `^##+ .*人間の確認待ち` という**見出し**を探し、
  その節の中の `Scenario: <名前>` だけを除外する。tasks.md の該当は `## 10. 通し` の中の
  箇条書き `- [ ] 10.4 **人間の確認待ち**`（tasks.md:134-136）で見出しではなく、
  Scenario 名も書かれていない。実測でも `人間の確認待ち 0`
- kind: technical
- 提案: `## 11.` の前に `## 人間の確認待ち` の節を立て、そこに
  `Scenario: グレースケールでも 7 状態が区別できる` のように、**実機・実目でしか判定できない
  Scenario 名を列挙**する（10.4 の本文だけでは機械に届かない）
- 処置: fixed 10.3 —— tasks.md に `## 人間の確認待ち` の**見出し**を立て、実機でしか判定できない 2 件（グレースケールで 7 状態 / 1 年ぶんの格子）を Scenario 名で列挙した。実測で `人間の確認待ち 2` になった

## R3. 分母から日を除く一方で閾値が絶対値 350 のままで、達成の判定式が閉じていない

- id: S3
- 成果物: openspec/changes/st02-collection-coverage/specs/collection-coverage/spec.md
- 種別: conflict
- 根拠: delta spec:110「収集開始日より前の日を、NFR-13 の分母に数えない」、
  delta spec:223「分母から、…**1 日を丸ごと覆うもの**だけを除く」、
  delta spec:225「合否を **5 本すべてが 350 以上**として返す」。
  `docs/requirements.md:374`（NFR-13）も「365 日中 350 日以上（95 %）」と
  「分母から除く」を並べている。**分母を減らしても閾値が 350 の絶対値なら、
  除外は達成を近づけず遠ざけるだけ**（分子も同時に減る）。導入から 1 年未満の期間では
  分母が 365 に満たず、350 は原理的に到達不能。「95 %」と「350 日」のどちらが判定かも定まらない。
  Scenario「1 ソースでも 350 未満なら未達」（delta spec:259-262）は分母に触れないので、
  この曖昧さを固定しない。加えて**どの 365 日を数えるか**（起点・終点）が spec に無く、
  design D9 は `from` / `to` を呼び出し側に委ねている（design.md:176-177）
- kind: conflict
- 提案: 「分母から除く」の結果を判定に反映する式を 1 つに決めて Scenario にする
  （例: 閾値 = `ceil(分母 × 0.95)`、または閾値は 350 固定で除外は分子だけに効く）。
  併せて 365 日窓の起点を spec に固定する。**判定式は 1 年の計測が始まったら変えられない**
  （deep.md:237 が自らそう書いている）ので、本人へ返す側だと読んだ
- 処置: escalated —— 第 5 回 Q18。判定式は 1 年の計測が始まったら変えられないので本人の領分

## R4. 途絶の判定が想定間隔を見ていない（design D7 の最後の行）

- id: S4
- 成果物: openspec/changes/st02-collection-coverage/design.md
- 種別: conflict
- 根拠: design.md:139 の D7 は「7 | 上のどれでもない | ⑥ 途絶」で、
  **`expected_gap_sec` を条件に持たない**。一方 FR-80（`docs/requirements.md:197`）と
  delta spec:130 は「**そのソースに登録された想定間隔を超えて**記録も生存信号も届かない」を
  条件にしている。`FR-35`（同 165-168）が定める想定間隔は
  ブラウザ履歴 = 24 時間 / Takeout 系 = 60 日なので、D7 のとおり実装すると
  **ブラウザ履歴や Takeout 系の正常な空白日がすべて⑥途絶になる**。
  tasks 5.3（tasks.md:68-70）は「想定間隔を超えて…無い日が⑥になる」と gap 込みで書いており、
  design と tasks も食い違っている
- kind: technical
- kind の訂正（呼び出し元、2026-09-11）: レビューは `conflict` としたが、**人間に問う幅が無い**。
  design と tasks が食い違っていたのは事実だが、**正解は FR-80 の逐語**
  （「そのソースに登録された想定間隔を超えて記録も生存信号も届かない」）で既に決まっており、
  design 側が条件を落としていただけだった。要件どうしが争っているわけではないので `technical` に直す。
  **指摘の中身は 1 つも否定していない** —— 挙がった帰結（ブラウザ履歴 24 時間や Takeout 系 60 日の
  正常な空白日がすべて⑥途絶になる）はそのとおりで、そのまま直した。
- 提案: D7 の 7 行目に想定間隔の条件を入れ、
  **「想定間隔を超えない空白日は途絶にならない」**（例: 想定間隔 24 時間のソースで
  1 日だけ空き、翌日に記録がある）を Scenario として specs に足す。
  いまの specs にはこの否定側の Scenario が 1 つも無い
- 処置: fixed D7 —— 決定順序の 7 行目に想定間隔の条件を入れ、8 行目（超えていなければ②）を足した。specs に否定側 Scenario「想定間隔を超えない空白の日は途絶にならない」を新設。tasks 1.6 で写真・ウィンドウ・ブラウザ履歴の想定間隔を登録簿に入れる作業を立てた（Q12 の写っていなかった半分）

## R5. 「記録があれば記録あり」が無条件で、D7 の順序と矛盾する

- id: S5
- 成果物: openspec/changes/st02-collection-coverage/specs/collection-coverage/spec.md
- 種別: conflict
- 根拠: delta spec:196-199「**WHEN** ある日にそのソースの記録が 1 件以上ある
  **THEN** その日の状態は「記録あり」である」（条件なし）。
  design.md:132-136 の D7 は 破棄(2) → 停止(3) → 記録あり(4) の順なので、
  **1 日を丸ごと覆う停止や破棄がある日は、記録が 1 件以上あっても④/⑤を返す**。
  design.md:144 は「丸ごと覆わない停止・破棄は状態を決めない。半日だけ止めた日は記録があれば①」と
  書いており、丸ごと覆う場合は①にならないことを前提にしている
- kind: conflict
- 提案: どちらが正かを決め、spec の Scenario 側に条件を書く
  （「丸ごと覆う停止・破棄が無い日に記録が 1 件以上あれば①」など）。
  この選択は**画面で「その日にデータがあること」が見えるかどうか**を決めるので、
  技術判断ではなく本人の判断に見える
- 処置: escalated —— 第 5 回 Q19。指摘のとおり技術判断ではなかった

## R6. 7 状態の決定順序が design にしか無く、archive で正典から落ちる

- id: S6
- 成果物: openspec/changes/st02-collection-coverage/design.md
- 種別: misplaced
- 根拠: delta spec:185 は「複数の条件が同時に成り立つ日について、状態を**一意に決める順序を定める**」
  としか言っておらず、順序そのものは design.md:127-145（D7）にしかない。
  `openspec archive` は main specs しか更新しないので、**D7 は正典に残らない**。
  とくに design.md:141「**破棄を停止より先に見る**」は deep.md に対応する問いが無く
  （Q1〜Q16 のどれも順序を決めていない）、**AI が独断で決めた観測可能な振る舞い**にあたる。
  Scenario「同じ入力から同じ状態が決まる」（delta spec:211-214）は 2 回の評価が一致することしか
  主張しないので、順序が入れ替わっても緑のまま
- kind: technical
- 提案: 7 段の順序を Requirement 本文に列挙し、
  少なくとも「破棄と停止が重なった日は破棄」「停止と記録が重なった日は◯◯」を
  Scenario として固定する。順序の**選択そのもの**は R5 と同じ理由で本人に返す余地がある
- 処置: escalated —— 第 5 回 Q19（R5 と同じ問い）。順序の選択が決まってから specs の Requirement 本文に列挙する。design D7 には「答えが出るまでの仮」であることと、実装は順序を 1 か所に閉じ込めること（tasks 5.1）を明記した

## R7. 「想定間隔を変えると過去の判定も変わる」が tasks にしか無い

- id: S7
- 成果物: openspec/changes/st02-collection-coverage/specs/collection-coverage/spec.md
- 種別: misplaced
- 根拠: design.md:108-125（D6）が「途絶は行に焼かず導出する」と決め、
  その**観測可能な帰結**を tasks.md:68-70（5.3）が
  「`expected_gap_sec` を変えると過去の判定も変わることをテストで確認する」と書いている。
  specs 側にこの主張は無い。tasks は archive で消える成果物なので、
  実装がバッチで途絶を書き込む形に戻しても、正典は落ちない
- kind: technical
- 提案: 「想定間隔を後から変えると、過去の日の途絶の判定も変わる」を Scenario にする
  （D6 の 3 つの理由のうち、外から観測できるのはこれだけ）
- 処置: fixed 5.4 —— specs に Scenario「想定間隔を変えると過去の日の判定も変わる」を新設した（D6 の観測できる唯一の帰結）

## R8. 受け口の応答の形（まとめ受け・400 の条件・拒否時に値を返さない）が specs に無い

- id: S8
- 成果物: openspec/changes/st02-collection-coverage/specs/collection-coverage/spec.md
- 種別: misplaced
- 根拠: design.md:175（D9）「複数件をまとめて受け、1 件ごとの結果を返す」、
  tasks.md:36「1 件も受け付けなかったときだけ 400」、tasks.md:38「拒否の応答に受け取った値を含めない」、
  tasks.md:41-42「取得できない状態のとき `blockers` が空なら 400」——
  いずれも**応答の形と状態符号**であって観測可能な振る舞いだが、specs にどれも無い。
  ST01 は同型を正典に置いている（`openspec/specs/record-envelope/spec.md:145-181`
  「取り込み口は複数件をまとめて受け取り、1 件ごとの結果を返す」／
  同:80「応答に内部の詳細（表名・接続先・受け取った値）が含まれない」）。
  **同じ種類の振る舞いが ST01 では正典、ST02 では design と tasks** という不揃いになっている
- kind: technical
- 提案: 生存信号の受け口についても record-envelope と同じ粒度の Requirement を specs に立てる
  （まとめ受けと 1 件ごとの結果／1 件も受け付けなかったときだけ拒否／拒否の応答に受け取った値を含めない／
  取れない状態で理由が空の信号は受け付けない）
- 処置: fixed specs/collection-coverage/spec.md —— Requirement「生存信号の受け口は複数件をまとめて受け取り、1 件ごとの結果を返す」を新設し、まとめ受け／一部不正／1 件も受け付けないときだけ拒否／応答に受け取った値を含めない／理由の無い「取れない」を断る、の 5 つを Scenario にした

## R9. 「明度が異なる」に数値が無く、ui-direction が 1.009:1 で通ったのと同じ穴が空いている

- id: S9
- 成果物: openspec/changes/st02-collection-coverage/specs/collection-coverage/spec.md
- 種別: unverifiable
- 根拠: delta spec:293-296「**THEN** どの 2 状態の組み合わせも**明度が異なる**」——
  差が 0.001 でも真になる。tasks.md:110-112（8.3）も
  「隣接する 2 段の差が**判別可能な下限**を超えることをテストで確認する」で、
  その下限の値がどこにも無い（design.md:186-207 の D10 も順序だけで数値が無い）。
  同じ書き方で ui-direction の 6 色が輝度差 **1.009:1** のまま目視で通っており
  （deep.md:105-109）、この Story はその反省から出発している
- kind: technical
- 提案: 隣接 2 段の下限を数値で決めて Scenario に書く（例: 相対輝度比 1.5:1 以上、
  または L\* の差 8 以上）。**7 段を 12 % 〜 100 % に単調に並べる**（D10）だけでは
  段差の大きさが決まらない
- 処置: escalated —— 第 5 回 Q21。検算したところ**明度だけでは原理的に成り立たない**（隣接 3:1 を 6 区間で 3^6 = 729:1、sRGB の最大は 21:1、21:1 を 6 等分しても 1.661:1）。数値を決める前に「何を併せるか」が要る

## R10. 「行」が何を指すかが spec と design で食い違い、「行を選ぶ」の範囲が決まらない

- id: S10
- 成果物: openspec/changes/st02-collection-coverage/specs/collection-coverage/spec.md
- 種別: unverifiable
- 根拠: delta spec:268「**ソースごとに行を分け、行頭にソース名の文字を置く**」（行 = ソース）に対し、
  design.md:229-230（D12）は「格子は 53 週 × 7 日 × 5 ソース。**ソースごとに 1 つの格子を縦に積む**」——
  1 ソースの格子の中の「行」は**曜日**になる。delta spec:270「WHEN 利用者が行を選ぶ
  THE SYSTEM SHALL **その範囲**の状態を文字で表示する」は、行 = ソース（1 年ぶん）なのか
  行 = 曜日（1 年ぶんの同じ曜日）なのか 行 = 週なのかで**出るものがまったく違う**。
  deep 第 4 回 Q16（deep.md:306）の本人の答えは「タップは行（**週 or ソース**）単位で受け」で、
  そこも 2 択のまま残っている
- kind: daily
- 提案: 「行」の定義を 1 つに固定してから Scenario を書き直す
  （何をタップすると何日ぶんの状態が文字で出るかが、毎日開く画面の使い勝手そのもの）。
  Q16 の答えが 2 択を残したままなので、本人に確かめる幅が残っている
- 処置: escalated —— 第 5 回 Q20。第 4 回 Q16 の答えが「週 or ソース」の 2 択のまま残っていた

## R11. セルの実寸（360 px 幅で約 6 px）と横スクロールの有無を下流に委ねている

- id: S11
- 成果物: openspec/changes/st02-collection-coverage/design.md
- 種別: daily
- 根拠: design.md:230-231「1 ソースぶんが 53 列なので、360 px 幅ではセルが **6 px 前後**になる。
  …**横スクロールで実寸を上げる余地は下流に委ねる**」。
  specs にはセルの最小寸法も横スクロールの有無も無い（delta spec:264-307）。
  一方 ST02 の完了の判定は「1 か月放置した後に開くと、**欠けた日が一目で分かる**」
  （`docs/stories/ST02.md:83`）で、6 px のセルでグレースケールの 7 段を見分けることが前提になる。
  NFR-19 は操作対象にしか掛からない（セルは表示専用）ので、**要件はこれを止めない**
- kind: daily
- 提案: 画面の見え方に直結する日常の判断なので、下流の実装者に委ねず
  最小セル寸法か横スクロールの方針を上流で決める（決められないなら本人へ返す）。
  R9 の明度の下限と合わせて初めて「一目で分かる」が判定可能になる
- 処置: escalated —— 第 5 回 Q21（R9 と同じ問いに含めた）。design D12 の「下流に委ねる」を取り消した

## R12. 収集開始日のバックフィル規則を AI が独断で決めており、1 年計測の起点が動く

- id: S12
- 成果物: openspec/changes/st02-collection-coverage/design.md
- 種別: irreversible
- 根拠: design.md:104-106（D5）「`collection_started_on` は登録時に埋める。
  既存の登録済みソースには migration で「**最初の記録または最初の生存信号の日**」を当て、
  どちらも無ければ **migration を当てた日**を入れる」。
  この日付は⑦導入前の境界であり（delta spec:109）、**NFR-13 の分母の起点**でもある
  （delta spec:110）。deep.md の Q1〜Q17 のどこにも対応する問いが無い。
  「どちらも無ければ当てた日」は、記録がまだ届いていないソース（ウィンドウ・ブラウザ履歴は
  C-02 がまだ存在しない）の**成功条件 1 の計測開始日を migration の実行日に固定する**ことになり、
  後から「本当はいつ始めたか」を作り直せない
- kind: irreversible
- 提案: 収集開始日の決め方（実際に登録した日か、最初のデータの日か、本人が指定するか）を
  本人に返す。少なくとも specs に Scenario を 1 本置いて、規則を正典に残す
- 処置: escalated —— 第 5 回 Q22。design D5 のバックフィル規則を取り消し、tasks 1.4 を列の追加までで止めた

## R13. 「残る」が行なのか導出なのかを Scenario が言っておらず、観測手段が定義されていない

- id: S13
- 成果物: openspec/changes/st02-collection-coverage/specs/collection-coverage/spec.md
- 種別: unverifiable
- 根拠: delta spec:57-61「**THEN** その日のそのソースの**稼働記録が残っている**」、
  同:152-155「**THEN** その期間が**途絶として残っている**」。
  design.md:36-48（D2）の `core.coverage` は取り込み時にしか行を立てないので、
  生存信号だけの日には**行が無い**。途絶も行に焼かない（D6）。
  つまりどちらの Scenario も「行を見る」では偽になり、「稼働状況を引く」では真になるが、
  **どちらで測るかが書かれていない**。design.md:124-125 が
  「観測される振る舞いは『稼働状況を引くと途絶が出る』であり、行の有無ではない」と
  補っているが、それは archive で消える文書にある
- kind: technical
- 提案: 該当の THEN を「稼働状況を引くと、その日の状態が◯◯として返る」に書き換える
  （実装の名前を使わずに、観測の手段だけを固定する）
- 処置: fixed specs/collection-coverage/spec.md —— 「稼働記録が残っている」「途絶として残っている」を「稼働状況を引くと…返る」に書き換え、観測の手段を固定した

## R14. specs に実装の名前（表名・列名）が入っている

- id: S14
- 成果物: openspec/changes/st02-collection-coverage/specs/collection-coverage/spec.md
- 種別: nit
- 根拠: delta spec:9「記録ごとのタイムゾーン（`tz_id`）で日を区切らない」、
  同:34「`tz_id` が `America/New_York` の記録を」、
  同:41-42「想定間隔（`core.source.expected_gap_sec`）ごとに」、
  同:71「`core.source` に無い論理ソース名で」。
  正典の 3 本（`openspec/specs/*/spec.md`）は SHALL 行にも Scenario にも表名・列名を持たず、
  唯一の `jsonb` は根拠ブロックの実測の引用（record-envelope/spec.md:20）にとどまる
- kind: technical
- 提案: 「記録に付いたタイムゾーン」「ソースに登録された想定間隔」「ソース登録簿に無い論理ソース」
  のように、実装の名前を根拠ブロック側へ落とす（列名が変わると spec が嘘になる）
- 処置: fixed specs/collection-coverage/spec.md —— `tz_id` / `core.source.expected_gap_sec` / `core.source` を「記録に付いたタイムゾーン」「そのソースに登録された想定間隔」「ソース登録簿」に置き換えた

## R15. ST02 の `satisfies` に無い要件を specs が満たしており、前倒しの理由がどこにも無い

- id: S15
- 成果物: openspec/changes/st02-collection-coverage/specs/collection-coverage/spec.md
- 種別: missing
- 根拠: `docs/stories/ST02.md:4` の `satisfies` は
  `[FR-33, FR-54, FR-78, FR-79, FR-80, NFR-13]`。
  一方 delta spec は FR-34 / FR-9（同:164「導出元: FR-34, FR-9」）、
  FR-29（同:320）、FR-30（同:82）、NFR-19（同:275）を導出元に挙げている。
  `docs/stories/INDEX.md:26,37` では **FR-9 は ST04（layer 2）**、**FR-34 は ST15（layer 2）**、
  同:23 では **FR-29 / FR-30 は ST01**。INDEX の「訂正」節（同:82-95）は
  **capability の前倒し**しか記録しておらず、**要件の前倒し**の理由は書かれていない。
  proposal の「capability の照合（Step 3）」（proposal.md:198-209）も capability だけを見て
  「ずれは無い」と結論している
- kind: technical
- 提案: FR-34 / FR-9 を ST02 で前倒す理由（`core.coverage` を作り直す唯一の機会だから）を
  INDEX か ST02.md の `satisfies` に落とす。FR-29 / FR-30 は ST01 が満たしたことになっている
  要件の穴埋めなので、**ST01 の archive された正典に穴があったこと**が分かる形で残す
- 処置: fixed proposal.md —— 「要件の前倒し」の節を新設。(a) FR-34 / FR-9 は**置き場だけ**を先に作る（satisfies は動かさない。入力の手段は ST04 / ST15）、(b) FR-29 / FR-30 は**前倒しではなく ST01 の正典に開いていた穴の穴埋め**、と性質を分けて記録した

## R16. MODIFIED 要件が指す「design D13」が ST02 の design に存在しない

- id: S16
- 成果物: openspec/changes/st02-collection-coverage/specs/collection-coverage/spec.md
- 種別: nit
- 根拠: delta spec:11「導出元: FR-33, NFR-13。**design D13**。深掘り Q2。」だが、
  ST02 の design.md は D1〜D12 まで（design.md:186,208,222）。
  D13 は ST01 の設計（`openspec/changes/archive/2026-09-10-st01-location-ingest/design.md:167`）。
  archive 後、正典の読者は存在しない D 番号を追うことになる
- kind: technical
- 提案: 「ST01 design D13」と出所を明記するか、参照を落とす
- 処置: fixed specs/collection-coverage/spec.md —— 「design D13」を「ST01 の design D13」に直した

## R17. ui-direction の宿題 1 が「ST02 の深掘りで決める」のまま残っている

- id: S17
- 成果物: docs/ui-direction.md
- 種別: missing
- 根拠: deep.md:299-301（第 4 回 Q15）は「**`ui-direction` の宿題 1 がこれで閉じる**」と書いているが、
  `docs/ui-direction.md:166` は
  「**色を意味の担い手にする箇所には形かラベルを併せる**こと（ST02 / ST25 の深掘りで決める）」の
  ままで、閉じた印（★ と日付、決着の内容）が無い。
  要件（`docs/requirements.md`）側は 6 件とも ★ 印が入っているのに、
  ui-direction だけ**戻しの手続きから外れている**（`review_triage.py` は
  requirements.md しか見ないので機械も止めない）
- kind: technical
- 提案: `docs/ui-direction.md` の宿題 1 に
  「★ 2026-09-10 決着（ST02 深掘り第 4 回 Q15）—— ソースごとに行を分け、行頭にソース名の文字を置く」
  を書き足す。ST25 側に残る宿題があるならそれも明記する
- 処置: fixed proposal.md —— `docs/ui-direction.md` の宿題 1 に決着（★ 2026-09-10、第 4 回 Q15）と、取り消した誤読と、ST25 側に残る宿題を書き足した。proposal にその記録を残した。指摘のとおり ui-direction は `review_triage.py` の検査対象外で、機械が止めない場所だった

## R18. R46 の B が渡した 2 件のうち 1 件を、proposal と design が別々に扱っている

- id: S18
- 成果物: openspec/changes/st02-collection-coverage/proposal.md
- 種別: premise
- 根拠: ST01 の R46 は B で決着し、ST02 へ **2 つ**の要求を渡している
  （`archive/2026-09-10-st01-location-ingest/deep.md:279`
  「『収集できなかった期間』と『その理由（静止による省電力）』を区別して残す」）。
  design.md:245 は「**ST02 の現在の設計はどちらも満たしていない**」と書き、
  proposal.md:195-197 は「**R46 の B のうち 1 件は ST02 の設計で埋まらなかった**」と書いている
  （＝もう 1 件は埋まった、と読める）。**同じ change の 2 文書が違うことを言っている。**
  実際、1（空きの期間を残す）を受ける Requirement も Scenario も specs に無く、
  第 5 回 Q17（`deep-questions-r5.json`）は**理由の側だけ**を問うている
- kind: premise
- 提案: 「1 は記録の時刻差から後から導出できるので ST02 では作らない」という判断を
  どちらか一方の文書に一本化し、**その導出が可能であること自体を Scenario にする**か、
  Q17 の選択肢に 1 の扱いも含める（いまは「何も足さない」を選ぶと 1 も落ちる）
- 処置: escalated —— 第 5 回 Q17。指摘のとおり proposal と design が食い違っていた。**正しくは「1（空きの期間）は記録の時刻差から後から導出できるので ST02 では作らない、2（理由）は作れないので問う」**で、両文書をこれに揃えた。Q17 の選択肢にも 1 の扱いを含めた

## R19. tasks 47 件のうち 34 件にコマンドと終了条件が無い（群でまとまって欠けている）

- id: S19
- 成果物: openspec/changes/st02-collection-coverage/tasks.md
- 種別: missing
- 根拠: `grep -c '^- \[ \]'` = 47。そのうち
  `rc=0` / `cargo test` / `gradlew` / `npm run` / `npx tsc` / `check-*.sh` のいずれも書かれていないのは
  **34 件**: 1.3, 2.1, 2.3, 3.2〜3.6, 4.1, 4.3, 5.2〜5.5, 6.1〜6.6, 7.1, 7.3〜7.5,
  8.1, 8.3〜8.7, 9.2, 10.3, 10.4, 11.1。
  とくに **群 6（達成日数と合否）は 6 件すべて**、**群 8（画面 S-1）は 7 件中 6 件**、
  **群 5（7 状態の導出）は 5 件中 4 件**が「テストで確認する」だけで終わっている。
  ST01 の 6.2 が「テストで確認する」のままテスト 0 本でチェックされた実測がある
  （`check_scenarios.py` の docstring）
- kind: technical
- 提案: 少なくとも群 5 / 6 / 8 の各タスクに、走らせるコマンドと rc を書く
  （群 8 は `npm run test` 等が未整備なら、先にその整備をタスクとして立てる）。
  10.3 は `python3 scripts/check_scenarios.py .` を挙げているが rc の条件が無い
- 処置: fixed 8.0 —— tasks を書き直し、群 5 / 6 / 8 を含む全タスクに走らせるコマンドと rc を入れた。web はテストの走らせ方が無かったので 8.0（vitest を入れて `npm run test` を足す）を先に立てた。10.3 にも rc の条件を書いた
