# 整合の確認 — FR-112 に「入れ替えた収集アプリは切り方を示すまで問い合わせない」を足した改訂（ST57 の深掘り Q11。2026-11-11）

- 基準: `342d558af8858747a09b4bfb3babde637a926bd4`..`e90c5d51e3f387f8c5369a42fc9e4eaa07a0ddad`

## 機械の入力（写し）

`python3 scripts/req_impact.py . --base 342d558af`（改訂した要件 1 件・扉 0 項）:

- FR-112（改訂）: `docs/handoff/ST55.md:21`・`docs/stories/INDEX.md:81,529`・`docs/stories/stories.json:1365`・
  `openspec/changes/st57-update-notice/deep-questions-r2.json:16`・`deep-questions.json:11,12,24,57`・`deep.md:84`・`defaults.md:6`・`design.md:183`・
  `premises.md:11,12`・`proposal.md:5,14,16`・`provisional-review.md:3`・`review/deep.md:9,10,12,25,84,103,105,126`・`review/spec.md:150`・
  `specs/distribution/spec.md:9,107,196,255,291,639,666,813`

`python3 scripts/check_chain.py .`（chain: FAIL 1 件 / 未回収 0 件 / warn 0 件）:

- [FAIL] 整合の確認: 整合の確認の後に要件が変わった（この監査を名指ししない改訂）: FR-112（この監査がそれに当たる。FR-112 の ★ 2026-11-11 はこのファイル名を名指ししている）
- 検査 10（判断の日より新しい訂正）は ST57 を出していない —— ST57 は走っている change を持つので検査の対象の外。下の C1 はその穴に当たる（FR-113 の確認の C1 と同じ穴）

走っている change（`openspec/changes/` の archive 以外）は `st57-update-notice` の 1 本だけ。C3 はこの change の中の文書への印で、差し戻しは書かない。本人の答え（`deep-answers-2.txt`・`deep.md` の Q11）は残っており、失われるもの: なし。

## 1. 改訂後の要件と、ほかの文書

## C1. ST57 の完了の判定は「3 つの部品が 1 日 1 回確かめる」のままで、FR-112 の ★ 2026-11-11 の例外（入れ替えた収集アプリは示すまで確かめない）を受けていない。ST57.md を作り直してもこの ★ は引用に載らない
- 重大度: 中
- 文書: docs/stories/stories.json:1383
- 要件: FR-112
- 根拠: stories.json:1383「サーバの更新係・携帯・PC が 1 日 1 回自分のチャンネルを確かめ、新しい版があると…」（ST57.md:54 も同じ）。requirements.md:1027「**更新の確認を持たない前の版から入れ替えた C-01・C-02 は、「更新の確認: 有効」と切り方を本人に示すまで配布元に問い合わせない**」・:1028「どれでも示せない間は確認が止まったままになる（1 日 1 回の例外）」。deep.md:84「★ 2026-11-11 補足で書き、ST57.md を作り直す」とあるが、commit e90c5d51 は stories.json と ST57.md に触れていない。さらに `scripts/make_story.py:19` の `quote(lines, limit=6)` は要件の引用を 6 行で切るので（ST57.md:24「…（続きは requirements.md）」）、FR-112 の 8・9 行目にある ★ 2026-11-11 は作り直しても ST57.md に出ない —— Story の文書だけを読む下流は例外を知らない
- 直すところ: stories.json の ST57 の done に「確認の無い前の版から入れ替えた収集アプリは、切り方を示すまで問い合わせない（示せない間は止まったまま）」を足し → make_story.py で ST57.md を作り直す。deep.md:84 の「ST57.md を作り直す」は done を直す形に読み替える
- 機械: できない（check_chain.py の検査 10 は change を持つ Story を見ない。make_story.py の 6 行の切り詰めで引用からも漏れる）。安く捕まえられる: 検査 10 を change のある Story に warn で広げる
- 行き先: stories（stories.json の ST57 の done に入れ替えの例外を足し、make_story.py で ST57.md を作り直した）

## C2. FR-113 の「7 日確かめられなければ知らせる」が、FR-112 の Q11 で意図して止めている間を除いておらず、入れ替えた収集アプリは初めて示した瞬間に「更新を 7 日確かめられていません（最後に確かめた —）」を一緒に出しうる。ST57 への申し送り C2 の直し方もこの改訂で古くなった
- 重大度: 中
- 文書: docs/requirements.md:1032
- 要件: FR-113, FR-112
- 根拠: requirements.md:1032「**更新の確認が有効な部品で 7 日確かめられなければ（網に繋がらない・更新係が止まっている など）、… 「更新を確かめられていない」を出す**」。requirements.md:1028「どれでも示せない間は確認が止まったままになる」—— 止まっている間も確認は「有効」のまま（FR-115 の既定）なので、文のままだと 7 日の数えに入る。spec.md:560「最後に確かめられた時（確かめられたことが無ければ確認を入れた時か送り先を設定した時）から 7 日が過ぎた THEN …」で、入れ替えた収集アプリの送り先の設定は数か月前でありうる。携帯は最初の画面（`MainActivity`。tasks.md:112）を開くことが示す口の 1 つで、同じ画面に「更新を確かめられていない」の行も出る（spec.md:560）ので、示した瞬間に一度も問い合わせていないのに「確かめられていません（最後に確かめた —）」が並ぶ。理由の欄（spec.md:563 の「更新係が動いていない」・「配布元に届かない」）に「まだ示していない」に当たる語は無い。`docs/handoff/ST57.md:13` の C2 は「起点を 1 つに決め（入れ替えた部品は入れ替えの後に初めて起きた時など）」としているが、Q11 の後は入れ替えの後に起きても示せなければ確かめないので、起点はその時ではなく「示した時」になる
- 直すところ: requirements（FR-113 の ★ に「FR-112 の Q11 で示すまで止めている間は数えない」を足す）。あるいはその文書（spec の 7 日の起点を「入れ替えた部品は示した時」にし、Scenario を足す）と、`docs/handoff/ST57.md` の C2 の例示を「示した時」に書き換える
- 機械: 安く捕まえられる（PC の試験に「送り先を 30 日前に設定・確認の無い版から入れ替え・トーストが `Err` を 3 回返してから `Ok` → `update-stale` の通知は出ない」、携帯の Robolectric に同じ形の Scenario）
- 行き先: handoff ST57（`docs/handoff/ST57.md` の「FR-112 の改訂の整合の確認」に C2 として明示し、FR-113 の確認の C2 の例示を「示した時」に書き換えた）

## C3. 走っている change の spec・design・tasks・proposal・go.json・PR の本文は、D23 を「通知で 1 回示す」のまま持ち、Q11 の示す口（携帯の画面・PC のトーストの成功）と示せない間の止まり方が無い（印のみ。spec 段で写す）
- 重大度: 中
- 文書: openspec/changes/st57-update-notice/specs/distribution/spec.md:194
- 要件: FR-112
- 根拠: spec.md:194「WHEN 送り先を設定済みの収集アプリが、…入れ替わって初めて起きる THE SYSTEM SHALL 「更新の確認: 有効」と切り方を通知で 1 回示し、最初の問い合わせをその後の機会にする」・:235-245 の 2 本の Scenario は通知を出せる場合だけ。:196 の導出元は FR-112 の ★ 2026-11-11 を書かず「design D23（通知を出せないときは Q11 で問い中）」。design.md:183「deep.md Q11（未回答）で本人に問う」・:201（Migration Plan）「新しい版に入れ替えた後、既に送り先を設定していれば次の機会に確かめる」—— Q11 の後は示すまで確かめない。proposal.md:25・tasks.md:21,101,114・go.json:642,1528「切り方を通知で 1 回示してから」・docs/briefs/ST57-pr.md:26「D23 … 失わない側で閉じた。`provisional.jsonl` に載せた」（台帳では resolved）。requirements.md:1028「C-01 は通知を出せたとき・本人がアプリの画面を開いて画面に出したとき、C-02 はトーストが成功を返したとき」。spec がまだ決めていないもの: 携帯の「画面」がどれか（切り方のスイッチは `SetupActivity`（spec.md:482）、「更新あり」の行は `MainActivity`（tasks.md:112））、PC の出し直しの時（deep.md の Q11 の意味「次の起動で出し直し」—— 常駐のアプリでは次のログオンまで止まりうる）
- 直すところ: その文書（次の spec 段 `spec --answers Q11` で D23 を Q11 の答えに書き直し、示せない場合と画面で示す場合の Scenario を足し、Migration Plan・proposal・tasks・go.json を合わせる）。走っている change なので差し戻しは書かない。失われるもの: なし
- 機械: 捕まえられた（req_impact.py が spec.md:196・design.md:183 を名指しした）。示す口の Scenario の欠けは安く捕まえられる（`check_scenarios.py` は印の有無だけを見るので、Q11 の Scenario を足してから印を数える）
- 行き先: leave: 走っている change の中の食い違いで、次の spec 段（`spec --answers Q11`）が D23 を Q11 の答えに書き直すときに写す（deep.md の「要件へ戻すもの（第 2 回）」に名指しした）。失われるもの: なし

## 2. 文書とコード

## K1. 携帯で「通知を出せた」と判断する既存の口 `SetupNotifier.canPost()` は送り先の設定のチャンネルの重要度を見ており、design D23 が名指すまま使うとチャンネル `update` だけを切った人を「示せた」と数える
- 重大度: 中
- 文書: openspec/changes/st57-update-notice/design.md:183
- 要件: FR-112
- 根拠: design.md:183「通知を出せない端末（携帯は `SetupNotifier.kt` の `canPost()` が false —— Android 13 以降の通知の許可が無い・通知が切ってある・チャンネルの重要度が NONE …）」。`collector-android/app/src/main/kotlin/dev/ashiato/collector/SetupNotifier.kt:106`「`private fun canPost(): Boolean =`」・:112「`manager.getNotificationChannel(CHANNEL)?.importance != NotificationManager.IMPORTANCE_NONE`」・:115「`const val CHANNEL = LocationService.SETUP_CHANNEL`」。案内はチャンネル `update` に出す（design.md:182）。Android はチャンネルを切ってあっても `notify()` を黙って捨てるので、`update` を切って `setup` を残した人は、この判断では「出せた」になり印が立ち、案内を見ないまま問い合わせが出る —— requirements.md:1027 が防ぐとした「本人が切る口を見ないまま、問い合わせの元の IP と時刻が配布元に残る」がそのまま起きる。`canPost()` は private で、ほかの通知の判断（`AndroidRetentionAlerts` など）も同じ形で個別に持っている
- 直すところ: その文書（spec 段で「示せた」の判断を、許可・`areNotificationsEnabled()`・**チャンネル `update` の**重要度で見ると書く）。コード（判断をチャンネルを引数に取る 1 か所の関数にする）
- 機械: 安く捕まえられる（Robolectric でチャンネル `update` だけを `IMPORTANCE_NONE` にして、問い合わせが 0 件の Scenario）
- 行き先: handoff ST57（`docs/handoff/ST57.md` の「FR-112 の改訂の整合の確認」に K1 として明示した）

## K2. FR-112 の ★ は「C-02 は通知を切っていてもトーストが成功を返しうるので、見ないまま問い合わせが出うる」を残る損として書くが、PC の収集アプリが既に有効にしている WinRT の機能に、通知が切ってあるかを読む口（`ToastNotifier::Setting`）がある
- 重大度: 中
- 文書: docs/requirements.md:1028
- 要件: FR-112
- 根拠: requirements.md:1028「C-02 はトーストが成功を返したとき。… C-02 は Windows の設定で通知を切っていてもトーストが成功を返しうるので、見ないまま問い合わせが出うる」。deep.md の Q11「残るもの（選択肢の「不可逆」）: PC で通知を切っている（が `Ok` になる）人は、見ないまま問い合わせが出うる」。`crates/collector-windows/Cargo.toml:54`「`windows = { version = "0.62", features = ["UI_Notifications", …] }`」で、windows 0.62.2 の `Windows/UI/Notifications/mod.rs:3382` `impl ToastNotifier` の :3397「`pub fn Setting(&self) -> windows_core::Result<NotificationSetting>`」、:1686-1687 `DisabledForApplication`・`DisabledForUser`。`crates/collector-windows/src/toast.rs:71-72`・:192-193 は `CreateToastNotifierWithId(…).and_then(|notifier| notifier.Show(&toast))` で、同じ `notifier` から `Setting()` を読める。読めば、アプリか利用者の単位で通知を切っている人を「示せない」に数えられ、要件が残す損の一部を消せる（集中モードなどは `Setting` に出ない）
- 直すところ: requirements（★ の残る損を「`Setting` が有効と返すのに見えない場合」に絞る）か、その文書（spec 段で、PC は `Show` の前に `Setting()` が `Enabled` であることも「示せた」の条件にすると書く）
- 機械: できない（本物の Windows で通知を切った状態の `Setting()` の値を見る必要がある。下の「未確認」）
- 行き先: handoff ST57（`docs/handoff/ST57.md` の「FR-112 の改訂の整合の確認」に K2 として明示した。値の確かめは本物の Windows で）

## 未確認

- `ToastNotifier::Setting()` が、未パッケージの exe と登録簿に書いた AUMID（`toast.rs` の `register_aumid`）について、Windows の設定の「通知」のアプリ単位・全体の切り替えを実際に `DisabledForApplication`・`DisabledForUser` で返すか。API の存在と、既に有効な機能で呼べることだけを確かめた（K2）
- Android で `notify()` が通知を捨てる条件のうち、`canPost()` の 3 つ以外（おやすみモード・アプリの通知の一時停止）が「出せた」に数えられるか。改訂は「通知を出せたとき」とだけ書き、見えたかは求めていない
- PC の収集アプリが常駐でなく起動し直す頻度（Q11 の「次の起動で出し直し」の間隔。C3）は、運用の実測が無い
- 正典の Scenario のテストの印は `check_scenarios.py` で全件を見た: 担保なし 90 件はすべて走っている change `st57-update-notice` の Scenario で、正典（`openspec/specs/`）の欠けは 0 件。change の Scenario はまだ実装が無いので指摘にしていない

## 触らなくてよいと判断したもの

- `openspec/changes/st57-update-notice/deep-questions.json`・`deep-questions-r2.json`・`review/deep.md`・`review/spec.md`・`provisional-review.md` の FR-112 の名指し: 問いと指摘と判定を出した時点の記録で、改訂の前の状態を正しく述べている
- `premises.md:11,12`（P1・P2）・`defaults.md:6`（C2）・spec.md:9,107,255,291,639,666,813・proposal.md:5,14,16: FR-112 の 1 日 1 回・更新係・チャンネルを引いており、Q11 の例外（入れ替えた収集アプリだけ）と食い違わない
- `defaults.md` の C5「携帯と PC は設定（FR-128）を保存した後」: 入れたての部品の話で、入れ替えた部品は design D23 が別に持つ（C3 で直す側）
- `docs/handoff/ST55.md:21`: FR-112 の ★ 2026-11-03（更新係のアカウント）の申し送りで、この改訂は触れていない
- `docs/stories/INDEX.md:81,529`: satisfies・requires・layer は変わらない。:529 は 2026-11-03 の経緯
- 合意（`agreement.json` の accepted）: Q11 は本人の答えで、仮でない決定の変更ではないので聞き直しの対象にしていない
- 語の揺れ: 改訂は概念の名前を変えていない（「入れ替えた」「示す」「切り方」は spec と deep.md と同じ語）
