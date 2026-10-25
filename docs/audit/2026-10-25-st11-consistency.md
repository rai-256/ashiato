# 整合の確認（2026-10-25・ST11 の深掘り第 1 回 Q1 を FR-11・FR-73・扉 #21 の理由の訂正へ戻した改訂）

- 基準: `2e45447fdb6324fa563e931f5f413ab5afaccc7f`..`2a2916de4316d126b6e8c3b85639ac46bb91178f`

対象は ST11（`openspec/changes/st11-health-history-permission/`）の深掘り第 1 回 Q1 の答え（推奨のまま: 初回起動で求める。要件の理由だけを訂正する）を要件へ戻した改訂。
FR-11・FR-73・扉 #21 の理由「宣言していない期間の 30 日より前は永久に読めない」を ★ 2026-10-25 で「履歴の権限は後から与えても全履歴が読める。履歴の権限が無い端末で読める範囲の起点（種類を最初に許した日の 30 日前）を早めるため初回起動で求める」に直し、
`docs/stories/stories.json` の ST11 の value と judged、ST34 の judged を直し、`docs/stories/ST11.md`・`docs/stories/ST34.md` を再生成した。振る舞い（初回起動で求める）は変えていない。
文書は head（`f5d4b7a2`。main への rebase 後は `2a2916de`。要件と Story の差分は同じ）の内容で読んだ。

機械の入力:
- `req_impact.py --base dba20fc8（rebase 後は 2e45447f）`: 改訂 2 件（FR-11・FR-73）・扉 1 項（#21）。名指しは `docs/production-prep.md:24`・`docs/stories/INDEX.md:34`・`:57`・`docs/stories/stories.json:262`・`:799` と、走っている st11 自身の文書
  （`deep-questions.json` `design.md` `go.json` `premises.md` `proposal.md` `review/deep.md` `review/spec.md` `specs/device-collection/spec.md` `tasks.md`）
- `check_chain.py`: 未回収 0・warn 0。FAIL 1 件は「整合の確認の後に要件が変わった: FR-11, FR-73, 扉#21」（この監査が受けるもの）
- `check_scenarios.py`: 担保なし 35 件はすべて走っている st11 の delta spec の Scenario（実装前）。正典（`openspec/specs/`）の Scenario に担保の無いものは 0

走っている change は `st11-health-history-permission`（この改訂の出どころ）・`st23-record-erasure`・`st29-satellite-credentials`・`st41-event-end-time`・`ch03-test-tmp-cleanup`・`ch04-test-db-growth`。
st11 以外の 5 つは FR-11・FR-73・扉 #21・Health Connect・健康データの権限を名指ししていない（`grep -rni "health\|健康\|FR-11\|FR-73\|EXT-C"` で該当なし。`/healthz` の語だけ）。失われるもの（uncaptured / discarded / exported / rewrite-all）を伴うものは無い。
st11 の delta spec（`specs/device-collection/spec.md:3-23`）・design D13・tasks Task 2.1 はすでに訂正後の理由で書かれている。古い理由の写しのうち `collector-android/app/src/main/AndroidManifest.xml:54` と `collector-android/README.md:100`・`:103` は tasks 2.1（design D13）が直す予定で、ここでは指摘にしない（末尾の「触らなくてよい」）。

## C1. FR-11 の SHALL は「履歴読み取り権限を要求する」だけだが、訂正後の理由の効き目は種類の読み取り権限を初回起動で許すことから来る。履歴だけを求めると画面は出ない
- 重大度: 中
- 文書: docs/requirements.md:209
- 要件: FR-11, FR-73
- 根拠: FR-11（requirements.md:209）「WHEN C-01 が初回起動する THE SYSTEM SHALL 健康データの**履歴読み取り権限**を要求する」。訂正後の理由（:211）「履歴の読み取り権限を与えない端末（断った・版が対応していない）では、読める範囲の起点が『**種類の読み取りを最初に許した日**の 30 日前』になる…初回起動で求めると、その起点が早まる」。
  履歴の権限を与えない端末で起点を早めるのは**種類の**読み取りの許可で、履歴の権限ではない（`premises.md` P13「読める範囲の起点は**アプリごと**（どれかの種類を最初に許した時）」）。
  しかも履歴だけを求めると画面が出ない（P4「履歴の権限だけを求めても、ダイアログは出ずに即座に拒否で返る」）。SHALL を字義どおりに作ると、どちらの端末でも理由の効き目が出ない。
  ST11 の delta spec（`specs/device-collection/spec.md:5-6`）は「睡眠・心拍・歩数・運動の読み取りと、履歴の読み取りと、背景での読み取りを**まとめて**求める」で「導出元: FR-11」と書き、深掘り Q3・Q9 の答えも同じ。
  要件の SHALL は種類の読み取り（Q3）と背景での読み取り（Q9。FR-73 の 24 時間ごとの取得が要る）を受けていない。ST11 の表題（stories.json:260）「健康データの**履歴権限**を初回起動で要求する」も同じ狭さ
- 直すところ: requirements（FR-11 の SHALL を「健康データの種類の読み取り権限と、履歴の読み取り権限（と背景での読み取り権限）をまとめて要求する」にし、理由の文はそのまま。ST11 の表題は stories.json で揃える）
- 機械: できない（SHALL の範囲と理由の文の食い違いは読み比べ。delta spec の「導出元: FR-11」が要件より広いことを機械は見ない）
- 行き先: requirements（FR-11 の SHALL を種類・履歴・背景をまとめて要求する形に広げ、★ に Q3・Q9 を書いた。ST11 の表題は change 名・issue と繋がるので変えない）

## C2. 扉 #21 は「一方通行の扉（後から変更できない決定）」に置かれたままだが、訂正後の本文自身が「履歴の権限は後から与えても全履歴が読める」と言う。不可逆に残るのは履歴の権限ではなく、種類の許可の時期
- 重大度: 中
- 文書: docs/requirements.md:1535
- 要件: FR-11, FR-73
- 根拠: 節の見出し（:1419）「### 一方通行の扉（後から変更できない決定）」、末尾（:1601）「> 可逆なものは扉に載せない」。「扉ではないもの」（:1583-1585）は `ACCESS_MEDIA_LOCATION` を「宣言が遅れても後から読み直せる」ので扉にしないと書く。
  扉 #21（:1535-1537）「**健康データの履歴読み取り権限を宣言するか** — 決めるもの: 記録の有無 …履歴の権限が無い端末では、読める範囲の起点が種類を最初に許した日の 30 日前になる = EXT-C。**履歴の権限は後から与えても全履歴が読める**」。
  扉の主題（履歴の権限）は、訂正後の本文によれば `ACCESS_MEDIA_LOCATION` と同じく後から取り返せる側に移った。後から取り返せないのは「履歴の権限が無い端末で、種類の読み取りを最初に許す時期」（P1・P13）で、扉の主題がそれを名指ししていない。
  `premises.md` の「失われるもの」の節も「Q2 の答えが『Android 15 以上で履歴を許す』なら、Q1 には失うものが残らない」と書き、本人の端末は Android 16（`deep.md` Q2 の補足「AQUOS sense9 Android16」）
- 直すところ: requirements（扉 #21 の主題を「健康データの読み取り権限（種類）を初回起動で求めるか」に書き換え、不可逆に残る部分 = 履歴の権限が無い端末での起点を決着状態に書く。履歴の権限の時期は「扉ではないもの」へ 1 行移す）
- 機械: できない（扉の可逆性の判定は読み比べ）
- 行き先: requirements（扉 21 の主題を「種類を初回起動で求めるか」に書き換え、履歴の権限の時期を「扉ではないもの」へ 1 行足した）

## C3. 訂正の根拠「履歴の権限は後から与えても全履歴が読める」は外部世界の主張なのに §5 の表（EXT-*）に行が無い。EXT-C を直すと言っていた計画も実行されず、根拠は archive で動く change のパスだけ
- 重大度: 中
- 文書: docs/requirements.md:1399
- 要件: FR-11, FR-73
- 根拠: EXT-C（:1399）の主張は「Health Connect は既定で権限付与時点の 30 日前までしか読めず、再インストールで起点が戻る」だけで、逐語も read-data の 2 文だけ。履歴の権限が全履歴を読ませること（`premises.md` P2 の出所「公式の定義も "Allows an application to read the entire history of health data (of any type)" https://developer.android.com/reference/android/health/connect/HealthPermissions」）は表のどの行にも無く、確認日・再確認期限も無い。
  改訂後の FR-11（:211-214）・FR-73（:954）・扉 #21（:1537-1538）はどれもこの主張に依り、根拠として `openspec/changes/st11-health-history-permission/premises.md` を引く。`scripts/archive.sh` はパスを書き換えないので、archive の後は `openspec/changes/archive/<日付>-st11-health-history-permission/premises.md` に動き、要件の ★ の参照が切れる（要件の中で `openspec/changes/` を引くのはこの 1 か所だけ。`grep -n "openspec/changes/" docs/requirements.md`）。
  計画側は EXT-C も直すと書いていた: `proposal.md:32`「要件の理由の訂正は record の段が入れる（…FR-11・扉 21・**EXT-C** と…）」、`deep-questions.json` Q1 の推奨の detail「FR-11・扉 21・**EXT-C** の理由を…に ★ で直す」。差分（`git show f5d4b7a2 -- docs/requirements.md`）に EXT-C の行は無い
- 直すところ: requirements（EXT の表に「履歴の読み取り権限（`READ_HEALTH_DATA_HISTORY`）があれば、許した時期に依らず全履歴が読める。入れ直した後も同じ」の行を足し、一次情報・逐語・確認日・再確認期限・影響 FR-11, FR-73 を書く。FR-11・FR-73・扉 #21 の根拠をその EXT へ向け、change のパスは ★ の経緯としてだけ残す）
- 機械: 安く捕まえられる（`check_chain.py` か `req_impact.py` で、要件が引く `openspec/changes/<name>/` のパスが存在するかを見る。archive の後に落ちる）
- 行き先: requirements（§5 に EXT-O を足し、FR-11・FR-73・扉 21 の根拠を EXT-O へ向けた。change のパスは経緯の名前だけに）

## C4. 扉 #21 の見出しが「宣言するか」のまま。改訂は FR-11・FR-73 の本文と扉の決着状態で「宣言」を「要求」に言い換えた
- 重大度: 低
- 文書: docs/requirements.md:1535
- 要件: FR-11, FR-73
- 根拠: 改訂の差分で FR-11（:210）「権限の**宣言**はこの時点で行う」→「権限の**要求**はこの時点で行う」、FR-73（:953）「権限の宣言は後回しにできない」→「権限の要求は初回起動で行う」、扉 #21 の決着状態（:1536）「宣言は Must」→「要求は Must」。
  扉 #21 の見出し（:1535）は「健康データの履歴読み取り権限を**宣言するか**」のままで、`docs/stories/ST11.md:27`・`docs/stories/ST34.md:27` の逐語にも写っている。manifest の宣言（P4 の注記「いまの manifest は履歴の権限だけを宣言している」）はすでにあり、決めているのは実行時に求めるかなので、「宣言」は実装者に manifest の 1 行で足りると読ませる
- 直すところ: requirements（扉 #21 の見出しを「…を初回起動で求めるか」に。C2 で主題を書き換えるならそこで一緒に。ST11.md・ST34.md は再生成で揃う）
- 機械: できない（語の言い換えの取り残しは読み比べ）
- 行き先: requirements（C2 と一緒に扉 21 の見出しを「求めるか」にした。ST11.md・ST34.md は再生成）

## C5. 訂正後の文の言い回しが根拠より強い・狭い:「Android 15 の実物」はエミュレータ、「種類の読み取りを最初に許した日」は EXT-C の逐語では「どれかの権限」
- 重大度: 低
- 文書: docs/requirements.md:213
- 要件: FR-11
- 根拠: FR-11 の ★（:213）「旧い理由…は **Android 15 の実物で**成り立たなかった」。`premises.md` の確かめ方は「Android 15（API 35、`system-images;android-35;google_apis;x86_64`…）の**エミュレータ**」で、表の出所も「実物（エミュレータ Android 15）」。`deep.md` Q2 の読みも「実物の端末で確かめたのではない」。
  FR-11 の本文（:211）「種類の**読み取り**を最初に許した日の 30 日前」に対し、EXT-C の逐語（:1399）は "up to 30 days prior to when **any permission** was first granted"、P13 は「どれかの種類を最初に許した時」。読み取り以外の権限（背景での読み取りなど）を先に許しても起点が決まるかを要件の文が狭めている
- 直すところ: requirements（:213 を「Android 15 のエミュレータで」に、:211 を「どれかの権限を最初に許した日の 30 日前」に）
- 機械: できない
- 行き先: requirements（「Android 15 のエミュレータで」「どれかの権限を最初に許した日」に直した）

## C6. 正典の device-collection は、ST11 が「その場では取れない権限」の型を引き継ぐと書くが、ST11 の前提では健康データの権限はその場のダイアログで取れる
- 重大度: 低
- 文書: openspec/specs/device-collection/spec.md:959
- 要件: FR-11, FR-2
- 根拠: 正典 `### Requirement: 利用状況へのアクセスを初回起動で求める` の本文（spec.md:955-960）「この特別なアクセスは、位置のような**その場のダイアログでは取れない** …**ここで決めた形を ST11（健康データの履歴権限を初回起動で要求する）が引き継ぐ。** 同じ capability で、同じ『その場では取れない権限』の型にあたる」。
  `premises.md` P4・P14 は、種類と一緒に求めれば Health Connect の画面（種類を選ぶ → 追加のアクセス）がその場で出ると確かめた。st11 の delta spec は「1 度だけ自動で求め、以後は常駐の通知から」の形（Q4）を引き継いだが、理由の「その場では取れない」は当たらない。
  st11 の delta spec の MODIFIED は `ソースごとに独立して収集する`（spec.md:254）だけで、この Requirement の本文は archive の後も残る。この改訂で生まれた食い違いではないが、ID を名指ししない意味の衝突として載せる
- 直すところ: その文書（正典の :959-960 を「1 度だけ自動で求め、以後は常駐の通知からたどる形を ST11 が引き継ぐ」に。走っている st11 の delta に MODIFIED で入れるかは st11 の判断なので、ここでは印だけ）
- 機械: できない
- 行き先: handoff ST11（正典の文の直しを ST11 の spec の段の判断として申し送った）

## 未確認

- Android 14 以下・Android 16 の端末で、履歴の権限を後から与えて全履歴が読めるか（P2・P3 は Android 15 のエミュレータだけ。本人の端末は Android 16 で、版の上では当たる見込みと `deep.md` Q2 の読みが書くが、実物では確かめていない）。C2・C3 の結論は Android 15 の結果に依る
- 背景での読み取りの権限（`READ_HEALTH_DATA_IN_BACKGROUND`）を先に許した場合も、EXT-C の "any permission" として起点が決まるか（C5 は逐語どうしの食い違いだけを書いた。実物では確かめていない）

## 触らなくてよいと判断したもの

- `collector-android/app/src/main/AndroidManifest.xml:54` の注記「宣言しない期間の 30 日より前は永久に読めない」と `collector-android/README.md:100`（「5 つの権限は、いずれも宣言しない期間が取り返せないか」。manifest の `uses-permission` は 17 行で数も合わない）・`:103`: 走っている st11 の tasks 2.1（`tasks.md:71-72`）と design D13（`design.md:227-228`）が訂正後の理由で書き直すと決めている
- st11 自身の `go.json:35`・`:48`・`:51`、`deep-questions.json:6`・`:16`、`docs/briefs/ST11-pr.md:7`（「答え待ち」の見出しのまま）: 走っている change の資料で、古い理由は「誤りだった」として引いている（`go.json:51` は理解の確認の誤答の選択肢）。印だけ
- `docs/briefs/ST11.html`・`ST11-spec.html`・`ST11-deep.html` に古い理由の文が残る: `.gitignore:34` の `docs/briefs/` で追跡されない生成物（`story_brief.py` などが作り直す）
- `docs/stories/ST11.md`・`docs/stories/ST34.md` の扉 #21 の逐語が「…（続きは requirements.md）」で切れ、「関与要件: FR-11, FR-73」の行が落ちた: `scripts/make_story.py:19-23` の `quote(limit=6)` の定型の切り方。stories.json から作り直すと同じものになる（一時の木で再生成して `diff -q` が一致）
- `docs/production-prep.md:24`（Health Connect は native API なので Kotlin）と `docs/stories/INDEX.md:34`・`:57`: 理由を引いておらず、改訂と食い違わない
- `docs/handoff/ST11.md`: 位置の権限を拒否したときの Scenario の申し送りだけで、健康データの権限の理由に依らない（st11 の Q10 と delta spec が受けている）
- ST34 の完了の判定「権限付与から 30 日以上前のデータも、履歴権限があれば取れる」（stories.json:814）: 訂正後の理由（履歴の権限があれば全履歴）と食い違わない
- コードとの突き合わせ（K）: 改訂は理由の文だけで、口・列・既定値・検査を述べていない。確かめたのは manifest が健康データの権限を `READ_HEALTH_DATA_HISTORY` の 1 つだけ宣言し（`AndroidManifest.xml:55`）、`collector-android/app/src/main/kotlin` に健康データの権限を求めるコードが無いこと（`grep -rni "health\|HISTORY"` 0 件）で、`premises.md` P4・P15 と proposal の「宣言しているだけで、求めるコードが無い」に一致する。FR-11 の振る舞いが未実装なのは ST11 がこれから作るもので、文書が実装済みと言う箇所は無い
- `check_chain.py` の FAIL「整合の確認の後に要件が変わった: FR-11, FR-73, 扉#21」: この監査が受けるもの
