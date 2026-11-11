# 整合の確認 — FR-113 に携帯の通知と「更新を確かめられていない」を足した改訂（ST57 の深掘り Q5・Q9。2026-11-11）

- 基準: `5475ba30561be6e4f61d7066ced0ace5cfe4b3ef`..`1cc7b4e6241bb5f2c835f728e1e189971d306f8f`

## 機械の入力（写し）

`python3 scripts/req_impact.py . --base 5475ba30561be6e4f61d7066ced0ace5cfe4b3ef`（改訂した要件 1 件・扉 0 項）:

- FR-113（改訂）: `docs/stories/INDEX.md:81`・`docs/stories/stories.json:1366`・
  `openspec/changes/st57-update-notice/deep-questions.json:102,103,109,176`・`deep.md:66`・`design.md:173,175,176`・`proposal.md:5,29`・
  `review/deep.md:11,98,101`・`review/spec.md:21,23,25,26,150`・`specs/distribution/spec.md:291,371,519,544,566`

`python3 scripts/check_chain.py .`（chain: FAIL 1 件 / 未回収 0 件 / warn 0 件）:

- [FAIL] 整合の確認: 整合の確認の後に要件が変わった（この監査を名指ししない改訂）: FR-113（この監査がそれに当たる。FR-113 の ★ 2026-11-11 はこのファイル名を名指ししている）
- 検査 10（判断の日より新しい訂正）は ST57 を出していない —— ST57 は走っている change を持つので検査の対象の外（`scripts/check_chain.py:206`「まだ change の無い Story だけ」）。下の C1 はその穴に当たる

走っている change（`openspec/changes/` の archive 以外）は `st57-update-notice` の 1 本だけ。下の C の多くはこの change の中の文書への印で、差し戻しは書かない。どれも本人の答え（`deep-answers-1.txt`）を失わない（失われるもの: なし）。

## 1. 改訂後の要件と、ほかの文書

## C1. ST57 の完了の判定と判断の日が、FR-113 の ★ 2026-11-11 で足した 2 つの知らせ（携帯の通知・「更新を確かめられていない」）を受けていない
- 重大度: 中
- 文書: docs/stories/stories.json:1382
- 要件: FR-113
- 根拠: stories.json:1382「新しい版があると画面と携帯の画面、PC の通知に「更新あり」が出る」・:1386「"judged": "2026-11-03"」。requirements.md:1029「**C-01 は画面に出すのに加えて、新しい版ごとに 1 回だけ通知も出す**」・:1030「**7 日確かめられなければ … 「更新を確かめられていない」を出す**」。ST57.md（:24-26）は make_story.py で貼り直され FR-113 の引用は新しいが、:53 の完了の判定は旧いまま。壊しうるもの（stories.json:1376-1379）も、携帯と PC に通知を足すのに既存の通知（契約の版・系統・送り先の設定。下の C5・K1）を挙げていない
- 直すところ: stories.json の ST57 の done（携帯の通知・7 日の知らせ）と breaks（既存の通知を置き換えない）、judged を 2026-11-11 に → make_story.py で ST57.md を貼り直す
- 機械: できない（check_chain.py の検査 10 は走っている change を持つ Story を見ない）。安く捕まえられる: 検査 10 を change のある Story にも warn で広げる
- 行き先: stories（stories.json の ST57 の done に携帯の通知と 7 日の知らせ、breaks に既存の通知を足し、judged を 2026-11-11 にして再生成した）

## C2. 一度も確かめられていない収集アプリの「7 日」の起点が「確認を入れた時か送り先を設定した時」と 2 つあり、入れ替えた収集アプリは入れ替えの直後に「7 日確かめられていません」を出しうる
- 重大度: 中
- 文書: openspec/changes/st57-update-notice/specs/distribution/spec.md:560
- 要件: FR-113, FR-115
- 根拠: spec.md:560「最後に確かめられた時（確かめられたことが無ければ確認を入れた時か送り先を設定した時）から 7 日が過ぎた THEN …」。requirements.md:1030「**7 日確かめられなければ … 「更新を確かめられていない」を出す**」。収集アプリの結果の欄（design.md:60）は `latest`・`found_at`・`checked_at`・`last_ok_at`・`last_reason`・`notified_version`・`stale_notified` で、確認を入れた時（サーバの `check.json` の `since` に当たるもの）を持たない。送り先を設定した時は端末に残っている（携帯 `SetupNotifier.kt:121` `setup_saved_at_ms`、PC `destination_gate.rs:96` `setup_saved_at`）。前の版から入れ替えた収集アプリ（design.md:182 D23）は、送り先を設定した時が数か月前でありうるので、入れ替えの後の最初の確かめが落ちると（TLS の根を読めない など）その場で 7 日を過ぎたことになる —— 7 日待たずに知らせる
- 直すところ: その文書（spec の起点を 1 つに決める: 収集アプリにも「確認を入れた時」を持たせ、入れ替えた部品は入れ替えの後に初めて起きた時を起点にする など。Scenario を 1 本足す）
- 機械: 安く捕まえられる（Task 9.1・PC の試験に「送り先を 30 日前に設定し、確認の無い版から入れ替え、最初の確かめが届かない → 通知は出ない」の Scenario）
- 行き先: handoff ST57

## C3. 更新係を持たないサーバ（Linux の配布物・開発の形）は「確かめられていない」を帯に出さず、FR-113 の「更新係が止まっている → 出す」が ST60 まで黙って破れる。ST60 はそれを受けていない
- 重大度: 中
- 文書: openspec/changes/st57-update-notice/specs/distribution/spec.md:372
- 要件: FR-113, FR-112, FR-131
- 根拠: spec.md:372「Linux や開発の形のサーバは更新係を持たない（C10）ので、設定に「動いていない」と出て帯は出ない」・spec.md:365 `not_running`（更新係の置き場が無い）は帯の 3 つ（`available`・`stale`・`available_stale`）に入らない（tasks.md:63）。requirements.md:1030「**7 日確かめられなければ（網に繋がらない・更新係が止まっている など）、… V-01 の画面 … に「更新を確かめられていない」を出す**」「（壊れた確認が黙って続くのを防ぐ）」。Linux の配布物は既にある（正典 distribution の「Linux の配布物は PostgreSQL を持たない」）。ST60 は satisfies が FR-131 だけ（stories.json の ST60）、申し送りに書く予定は「Linux の更新係の起こし方」だけ（design.md:109・tasks.md:127）で、帯を出さないことは申し送らない。Windows でも更新係の置き場が消えれば同じく `not_running` で帯が出ない
- 直すところ: 凍結した Story への申し送り（ST60: Linux の更新係を起こしたら、`not_running` の帯の扱いも FR-113 に合わせる）。あるいは requirements（FR-113 の ★ に「更新係を持たない形は ST60 まで出さない」を明記）
- 機械: できない（要件の例外を書くかどうかの判断）
- 行き先: handoff ST60

## C4. 帯の文字のコントラストを 3:1 以上としており、NFR-18 の本文の 4.5:1 を下回ってよいことになる
- 重大度: 中
- 文書: openspec/changes/st57-update-notice/specs/distribution/spec.md:369
- 要件: NFR-18, FR-113
- 根拠: spec.md:369「帯の文字と地のコントラストを 3:1 以上にする」・:398-402 の Scenario「どの帯でも、文字と地のコントラストは 3:1 以上である」・tasks.md:64-65「`contrast.ts` で 3:1 以上」。requirements.md:1454「**NFR-18**: 本文と背景のコントラスト比は **4.5:1 以上**、大きい文字は **3:1 以上**とする」。spec は帯の文字を大きい文字と決めていない（「1 行」の本文）。正典 browsing-views は一覧と 1 日の文字に 4.5:1 を求めている（spec.md:277・:852）。改訂で帯に「更新を 7 日確かめられていない（最後に確かめた <月>/<日>・<理由>）」という長い本文が載る
- 直すところ: その文書（spec と tasks の 3:1 を 4.5:1 にするか、帯の文字を NFR-18 の大きい文字にすると書く）
- 機械: 安く捕まえられる（e2e の Scenario の閾値を 4.5 にするだけ。spec の数値と NFR-18 の照合は機械では拾っていない）
- 行き先: handoff ST57

## C5. 携帯に足す 3 種の通知（入れ替えの案内・更新あり・確かめられていない）と PC の `update-stale` を、既存の「サーバの更新」の知らせと見分けることを spec が固定していない
- 重大度: 低
- 文書: openspec/changes/st57-update-notice/specs/distribution/spec.md:553
- 要件: FR-113, FR-126
- 根拠: 見分けを固定しているのは PC の「更新あり」だけ（spec.md:542「更新の通知の文を、系統の知らせ … や送信契約の版の知らせと分ける」・:553 の Scenario）。携帯の要件（spec.md:512-537）と「確かめられていない」の要件（:558-615）には置き換え・見分けの文が無い。既存の携帯の通知チャンネル `contract` の表示名は「サーバの更新」（`ContractNotifier.kt:91`・`:155`）、文は「サーバを更新するか、つながっている網を確かめてください」（正典 device-collection:2455）。新しい「更新を 7 日確かめられていません（… 配布元に届かない）」は語が近い。携帯のチャンネル `update` には 3 種が載る（design.md:182 の D23 の案内も `update`）が、通知の ID も置き換えの有無も決めていない（tasks.md:112）。正典 distribution:1109 は系統と契約の知らせを「片方を消してももう片方は消さない」と固定しており、同じ形の固定が新しい知らせに無い
- 直すところ: その文書（携帯の 3 種と PC の `update-stale` について「ほかの知らせを置き換えず、文が違う」を Scenario にする。チャンネル `update` の表示名を「サーバの更新」と違えると書く）
- 機械: 安く捕まえられる（PC の :553 と同じ形の Scenario を携帯と `update-stale` に足す）
- 行き先: handoff ST57

## C6. 改訂の後も、spec・proposal・design が「推奨のままなら FR-113 を改訂する」「draft では要件に触らない」と未来形のまま残り、Q9 の要件は導出元に FR-113 を書かない
- 重大度: 低
- 文書: openspec/changes/st57-update-notice/specs/distribution/spec.md:566
- 要件: FR-113
- 根拠: spec.md:566「導出元: 深掘り Q9（推奨: 7 日確かめられなければ出す。**推奨のままなら FR-113 を ★ で改訂する**）」（FR-113 を名指ししない）・:519「**推奨のままなら FR-113 を ★ で改訂する**」・:371「導出元: FR-113（画面に「更新あり」）」（帯の「確かめられていない」の出所が FR-113 の ★ 2026-11-11 になった）・proposal.md:29「この draft では `docs/requirements.md` と `docs/stories/ST57.md` に触らない」・design.md:173-176（D22（仮））。改訂は commit 1cc7b4e6 で済み、ST57.md も貼り直された。deep.md:66 は「要件へ戻すもの: FR-113」で閉じている
- 直すところ: その文書（導出元を「FR-113（★ 2026-11-11）」にし、「推奨のままなら」を済んだ形に直す。D22 は仮でなくなった）
- 機械: できない（散文の時制）
- 行き先: handoff ST57

## C7. FR-113 の ★ の「Q6」が、すぐ下の FR-114 の「Q6」（要件の聞き取りの Q6）と同じ書き方で別の問いを指す
- 重大度: 低
- 文書: docs/requirements.md:1029
- 要件: FR-113, FR-114
- 根拠: requirements.md:1029「★ 2026-11-11 追加。ST57 の深掘り Q5・Q9。… （C-02 の通知も新しい版ごとに 1 回。Q6）」—— 冒頭は「Q5・Q9」と書き、Q6 は ST57 の深掘りの Q6（deep.md「### Q6. PC の通知の回数」）。requirements.md:1032（FR-114）「**入れるのは本人**（Q6）」は 2026-10-24〜25 の要件の聞き取りの Q6
- 直すところ: requirements（「ST57 の深掘り Q5・Q6・Q9」と冒頭にまとめる）
- 機械: できない
- 行き先: requirements（★ の冒頭を「ST57 の深掘り Q5・Q6・Q9」にまとめ、括弧の Q6 を外した）

## C8. FR-113 の ★ は「確認を切った部品」を除いておらず、文のままだと切った部品も 7 日で「確かめられていない」を出すことになる
- 重大度: 低
- 文書: docs/requirements.md:1030
- 要件: FR-113, FR-115, FR-112
- 根拠: requirements.md:1030「**7 日確かめられなければ（網に繋がらない・更新係が止まっている など）、… 「更新を確かめられていない」を出す**」。spec.md:564「WHILE その部品の確認が切ってある THE SYSTEM SHALL 「確かめられていない」を出さない」・:593 の Scenario（切ってから 30 日で出ない）・:410（切ってあれば帯は出ない）。FR-112（requirements.md:1020）は「WHILE 更新の確認が有効である」で確かめる側だけを条件づけ、FR-113 の知らせ側には条件が無い。spec の解釈は妥当だが、要件の文が支えていない
- 直すところ: requirements（★ の文に「確認が有効な部品で」を足す）
- 機械: できない
- 行き先: requirements（★ の文に「更新の確認が有効な部品で」を足した）

## C9. 画面と要件の対応（UI の方向）と画面の在り処が、どの画面にも出る帯と設定の「更新の確認」の節を持たない
- 重大度: 低
- 文書: docs/ui-direction.md:23
- 要件: FR-113
- 根拠: ui-direction.md:23「| S-7 設定 | ソース・停止・… 書き出し（★ 2026-10-11） | ST15, ST29, ST33, ST38 | FR-53, … NFR-16 |」に ST57・FR-113 が無く、S-1・S-2・S-6 の行にも見出しの上の帯が無い。spec.md:355「画面は、サーバの「更新あり」と「確かめられていない」をどの画面でも見出しの上の帯に出し、設定に更新の確認の状態を出す」。改訂で帯の中身に「確かめられていない」が足された。docs/screens.md は URL の表で、帯は新しい面でないので行は要らない
- 直すところ: その文書（ui-direction.md の S-7 の行に ST57・FR-113、各面の上に帯が乗ることの注記）
- 機械: できない
- 行き先: docs（ui-direction.md の S-7 の行に更新の確認の状態・帯・ST57・FR-113 を足した）

## 2. 文書とコード

## K1. 携帯の通知の ID 6 が「送り先が設定されていません」と系統の違いの知らせで重なっており、ST57 が通知を足す前に ID の割り当ての決まりが無い
- 重大度: 低
- 文書: openspec/specs/distribution/spec.md:1109
- 要件: FR-106, FR-113
- 根拠: `collector-android/app/src/main/kotlin/dev/ashiato/collector/LocationService.kt:1464`「`const val SETUP_NOTIFICATION_ID = 6`」と `ContractNotifier.kt:104`「`companion object { const val CHANNEL_NOTIFICATION_ID = 6 }`」（`ChannelMismatchNotifier`）。Android は同じ ID の通知をチャンネルに依らず置き換える。`SetupNotifier.kt:36` の `setupSaved()` は `manager.cancel(LocationService.SETUP_NOTIFICATION_ID)` で ID 6 を無条件に消すので、送り先を保存し直すと系統の違いの通知も消え、`channelNotifiedAtMs` は残るので 24 時間は出し直されない（`ContractNotifier.kt:89`）。正典 distribution:1109 は系統の違いの知らせを「続く間は前に出してから 24 時間ごとに出し直す」とする。ST57 は携帯のチャンネル `update` に 3 種の通知を足す（C5）が、ID を決めていない（tasks.md:112）
- 直すところ: コード（ID の重なりを解き、通知の ID を 1 か所の定数にまとめる）。ST57 の tasks に新しい通知の ID を書く
- 機械: 安く捕まえられる（`*_NOTIFICATION_ID` の値が重ならないことを見る JVM の単体試験か grep の検査）
- 行き先: handoff ST57

## 未確認

- 更新の確認のコードは 1 行も無いこと（design の Context の前提）は `rg` で `update-check` / `update_check` / `update-status` / `ashiato-updater` を探して 0 件を確かめた。それより先の、spec が述べる振る舞い（`GET /update-status`・帯・札 `update`・チャンネル `update`）はまだコードが無いので照合していない
- Android が `HttpURLConnection` で `Connection` 以外の見出しを足さないか（design D13（仮））は確かめていない（計測テストの対象）
- 携帯の `POST_NOTIFICATIONS` を断った端末で、改訂の「C-01 の通知」が出ないときにどう扱うか（既存の通知は許可が無ければ出さない —— `OutageNotifier.kt:55`）は spec に無いが、要件の改訂は許可の扱いを求めていないので指摘にしていない
- 正典の Scenario のテストの印は全件を機械で見ていない（改訂は正典を変えていない）

## 触らなくてよいと判断したもの

- `openspec/changes/st57-update-notice/deep-questions.json:102-176`・`review/deep.md`・`review/spec.md`・`go.json`・`proto.html` の「FR-113 に無い」等の言い回し: 問いと指摘を出した時点の記録で、改訂の前の状態を正しく述べている
- 文の揺れ（要件の「更新を確かめられていない」、帯の「更新を 7 日確かめられていない（…）」、携帯と PC の「更新を 7 日確かめられていません（…）」）: 要件は概念の名前、spec と D20（仮）は表示の文で、spec.md:560 が対応を書いている
- design.md:200（Migration Plan「既に送り先を設定していれば次の機会に確かめる」）と D23（入れ替えの後に初めて起きた時は案内だけで問い合わせない）: 「次の機会」が案内の後の機会と読めるので食い違いとまでは言えない（C2 の起点の問題とは別）
- `docs/handoff/ST58.md`: 改訂は「更新あり」を押した後（FR-114）を変えていない。携帯の通知を押すと最初の画面が開く（spec.md:534）ことは ST57 の申し送り（tasks.md:127）で ST58 に届く予定
- `docs/stories/INDEX.md:81`: satisfies・requires・layer は改訂で変わらない
- `docs/collector-contract.md:349` の「確かめられていない」: GPS の時刻の話で、この改訂の語ではない
