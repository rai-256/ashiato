# 整合の確認（2026-10-13・ST14 の深掘り第 1 回を要件へ戻した改訂）

- 基準: `3907220e6001eab37dd7a019f9be6fb918b0b82a`..`d7994a766b6057e9b60f3c8b74b21235865a9baf`

対象は ST14（`openspec/changes/st14-outage-alert/`）の深掘り第 1 回の答え（Q1〜Q9）を要件へ戻した改訂 —— FR-35（★ 2026-10-13。Q1 / Q2 / Q3 / Q4 / Q5 / Q9）、
FR-54（★ 2026-10-13。Q7 / C9）、FR-79（★ 2026-10-13。Q8）と、`docs/stories/stories.json` の ST14 の `done` に足した 2 行（再生成した `ST14.md`）。
文書は head（`d7994a76`）の内容で読んだ。

機械の入力:
- `req_impact.py --base 3907220e`: 改訂 3 件・扉 0 項。名指しは `docs/handoff/ST14.md`・`docs/stories/INDEX.md`・`stories.json`・`docs/ui-direction.md`・`docs/collector-contract.md`・`docs/production-prep.md`・`docs/openapi.json`・
  走っている st08 / st17 / st25 と ST14 自身の文書・正典 `collection-coverage` `desktop-collection` `device-collection` `external-ingestion` `record-deletion` `record-envelope`
- `check_chain.py`: 未回収 0・warn 0。FAIL 1 件は「整合の確認の後に要件が変わった: FR-35, FR-54, FR-79」（この監査が受けるもの）

## C1. 収集アプリの資格情報を「送るだけ」に締める ST44 が、FR-35 ★ の「収集アプリがサーバの判定を問い合わせる」を塞ぐ
- 重大度: 高
- 文書: docs/stories/stories.json:1047
- 要件: FR-35, PERM-10
- 根拠: 改訂後の FR-35（requirements.md:394）「**通知は携帯と PC の両方の収集アプリが、サーバの判定を問い合わせて OS の通知として出す**」。ST44 の完了の判定（stories.json:1047）「収集アプリの資格情報では、送ることだけができて記録は読めない」。ST44 への申し送り（`docs/handoff/ST44.md:9`）は「収集アプリの資格情報に残すのは `/feelings/reminder` の読みだけ」と**「だけ」で閉じている**。ST14 の判定の口（ソースの表示名と最後に届いた日時を返す。deep C5）はこの例外に入っていない。ST44 が申し送りどおりに締めると、問い合わせが 401 になり、携帯と PC のどちらからも途絶の通知が出なくなる。止まった収集アプリは自分では知らせられないので、黙って何も鳴らない状態になる —— FR-35 ★ 2026-09-10 が言う「切られた通知は 1 年の途中で気づく手段を失わせる」と同じ結果になる。ST44（`device-collection` / `desktop-collection` / `data-sensitivity`）と ST14（`collection-coverage`）は capability が重ならないので、admission は 2 本を並べて走らせる（`board.py` では ST44 は `衝突待ち`。理由は ST08 だけ）
- 直すところ: 凍結した Story への申し送り（`docs/handoff/ST44.md` の「残すのは `/feelings/reminder` だけ」に、途絶の判定の口を足す）か stories（ST44 の完了の判定に例外を書く）
- 機械: 安く捕まえられる（ST14 の結合テストで、判定の口を収集アプリの資格情報で呼んで 200 になることを確かめる試験を置く。ST44 が締めた時点でその試験が落ちる）
- 行き先: handoff ST44（例外を `/feelings/reminder` と途絶の判定の口の 2 つにする。ST14 の deep.md の「spec の段へ渡すもの」にも試験を書いた）

## C2. ST14 の capability は `collection-coverage` だけ。Q1 で 2 つの収集アプリのコードに、Q9 で書庫の登録簿に手が入るのに、admission が見る表に載っていない
- 重大度: 中
- 文書: docs/stories/INDEX.md:84
- 要件: FR-35
- 根拠: INDEX:84「`collection-coverage` | 収集の稼働状況・通知・停止 | … ST14 …」。ST14 が載っているのはこの 1 行だけ（:80-91）。改訂後の FR-35 は (1) 携帯と PC の両方の収集アプリに OS の通知を出させる（deep.md:88「PC の収集アプリに Windows の通知の口を作る」。deep.md:39「PC の収集アプリは OS の通知の口を持たない」）、(2) 書庫のソースに「通知までの日数」の列を登録簿に足す（Q9）。並走しているものは次のとおり。`crates/collector-windows` は ST08（下流。`desktop-collection`）が走らせている。`collector-android` の通知は ST17（下流。`subjective-log` / `browsing-views`。proposal.md:79「`collector-android`: 1 日 1 回の通知の予約…`RECEIVE_BOOT_COMPLETED` を足す」）が同じ時期に足している。`board.py` は ST09 / ST11（`device-collection`）を「着手可」にしている。正典 `external-ingestion` には「書庫のソースは想定間隔 60 日で登録簿に入る」（spec.md:789）と :697「60 日を過ぎたことを知らせるのは ST14」があり、`board.py` は ST47 を `external-ingestion` の上流として数えている。spec を `collection-coverage` に置けば admission は通るが、コードと他の capability の条項は重なる
- 直すところ: stories（INDEX の capability の表で ST14 を `device-collection` / `desktop-collection` に載せるか、spec を `collection-coverage` に閉じて `external-ingestion` の条項は ST47 の後に回すと INDEX の訂正に書く）
- 機械: 安く捕まえられる（`board.py` の admission で、proposal の Impact に書かれたパスと、走っている Story の Impact のパスの重なりも見る）
- 行き先: leave: spec は `collection-coverage` に閉じる（INDEX:84 の担当に「通知」がある）。capability の表は spec の置き場で、コードの重なりは admission の規則の外。重なるコードは ST14 の proposal の Impact に書く（deep.md に渡した）

## C3. ST14 宛ての申し送り（★ 2026-10-12。本人が書いたソースを分類で外す）を、深掘りも改訂後の FR-35 も受けていない
- 重大度: 中
- 文書: docs/handoff/ST14.md:130
- 要件: FR-35, FR-25
- 根拠: handoff/ST14.md:130-132「★ 2026-10-12（docs/audit/2026-10-12-st17-consistency.md C7）: …場所（`s01-place`。ST21 が `origin='authored'` で登録）と衛星の『本人が書いた』記録…ST14 の上流で、除外を『本人が書いた（FR-25）記録のソース』と分類で決めるか…を一緒に決める」。ST14 の deep.md:10-21「この Story に宛てられたもの」の表にこの項が無い。C1（deep.md:51）が外すのは「本人が書く `s01-feeling`・`s01-attribute` / 導出 `s01-stay` / 衛星 FR-99」だけで、`s01-place` が入っていない。改訂後の FR-35（requirements.md:389）も括弧は「主観 FR-36・個人属性 FR-44」のまま。ST21（走っている）の design.md:86 は `('s01-place', '場所', 86400, 'none')` で登録する。C1 の既定は「対象」なので、ST14 が作ったとおりに動くと、場所を 3 日登録しなかっただけで途絶として鳴る
- 直すところ: その文書（ST14 の deep.md の「宛てられたもの」に足し、C1 の外す印に `s01-place` を足すか `origin = authored` で絞る）と requirements（FR-35 の括弧を例に格下げし、分類で書く）
- 機械: 安く捕まえられる（handoff/ST<NN>.md の `★` 行と `## ` 見出しの数を、deep.md の「宛てられたもの」の表の行と突き合わせる。登録簿の全ソースについて「`origin` が authored のソースは判定の対象外」を試験で回す）
- 行き先: requirements（FR-35 の除外を FR-25 の「本人が書いた」の分類にした ★ 2026-10-13 補強）と ST14 の deep.md（宛てられたものの表・C1 に `s01-place` と分類）

## C4. C9（観測でない記録を「最後の記録」に数えない）が FR-35 に戻っていない。FR-54 には戻した
- 重大度: 中
- 文書: docs/requirements.md:386
- 要件: FR-35, FR-54, FR-7, FR-85
- 根拠: FR-35:386「**最後の記録**または最後の生存信号（FR-78）からの経過時間が…」。改訂で足したのは生存信号の条件（:390「生存信号は『取得できる状態』のものだけを『生きている』と数える」）だけで、記録の側には条件が無い。FR-54 ★（:646）は「観測ではない記録は『記録あり』に数えない」を格子についてだけ書く。deep.md:59 の C9 は「`clock-skew` / `idle` / `powered-off` / `excluded` と…`gap` は『最後の記録』に数えない」で、理由は「毎時の時計の測定が、窓が読めなくなった PC を『生きている』にし続ける（R2）」。要件の逐語どおりに作ると、`c02-window` には 1 時間ごとに `clock-skew` の記録が届く（FR-7。`crates/collector-windows/src/contract.rs:34`）。そのため、Q2 で取得できない状態の生存信号を外しても、PC の窓が読めなくなった件は鳴らない。deep.md「要件へ戻すもの」（:102）も FR-35 の欄に C9 を入れていない
- 直すところ: requirements（FR-35 に「観測でない記録は最後の記録に数えない」を足す。FR-54 ★ と同じ語で）
- 機械: 安く捕まえられる（ST14 の試験に「`clock-skew` だけが届き、生存信号が取得できない状態のまま 18 時間 → 鳴る」を 1 本置く）
- 行き先: requirements（FR-35 に「観測ではない記録は最後の記録に数えない」を足した）

## C5. 書庫のソースについて、通知は「最後の書庫」で、格子の⑥は「最後の記録」で測ることになった。正典は 60 日・最後の記録のまま、2 つが揃っている前提で書かれている
- 重大度: 中
- 文書: openspec/specs/external-ingestion/spec.md:697
- 要件: FR-35, FR-80, FR-54
- 根拠: 正典 external-ingestion:697「60 日を過ぎたことを知らせるのは ST14 で、この画面は見えるだけ（格子では、記録の間が 60 日を超えた日が『途絶』として見える）」。:588「書庫の各ソースに信号を送ると…FR-35 の通知も鳴らない —— 60 日の想定間隔（第 1 回 Q7）が効かなくなる」。Scenario `書き出しを忘れると書庫のソースは途絶になる`（:620「視聴履歴の最後の記録の日から 61 日たった日」→「途絶」）。改訂後の FR-35（:391-393）は、書庫のソースを「最後の書庫から 75 日」で測る。★ の理由（:412）は「書庫は出来事の時刻で測ると**その製品を使わなかった期間が途切れに見える**」。FR-80 は改訂されていないので、格子は書庫のソースを今後も最後の記録から 60 日で⑥にする。その結果、2 か月ごとに書き出していても使わなかった製品は格子で⑥になり（通知は鳴らない）、逆に書き出しを忘れても⑥は 60 日目、通知は 75 日目と、ずれて出る。深掘り Q3 の推奨の detail（deep-questions.json の Q3）は「格子の⑥と通知が同じ時刻を見る」で、これは携帯・PC のソースについてだけ成り立つ
- 直すところ: requirements（FR-80 / FR-54 で書庫のソースをどちらで測るかを決めるか、格子と通知がずれることを ★ に書く）と、その文書（external-ingestion:697 / :588 の 60 日と ST14 の記述。ST47 が `external-ingestion` を走らせている間は ST14 の MODIFIED にできない —— C2）
- 機械: できない（2 つの文書の理由どうしのずれ）
- 行き先: requirements（FR-35 ★ に格子と通知の測り方の違いを書いた）。正典 external-ingestion の 60 日の文は ST14 の spec で扱う（deep.md に渡した）

## C6. 正典 `collection-coverage` の Scenario が、改訂前の「記録が 1 件でもあれば①」と「古い記録が届けば開始日が遡る」を例外なしで固定している
- 重大度: 中
- 文書: openspec/specs/collection-coverage/spec.md:395
- 要件: FR-54, FR-79, FR-85
- 根拠: 正典:395「(5) 記録が 1 件以上あれば『記録あり』」、:402「『記録が 1 件以上ある』を、**その日に出来事の時刻が属する記録があること**として判定する」、Scenario `丸ごと覆う停止も破棄も無い日に記録があれば記録あり`（:442）。改訂後の FR-54 ★（requirements.md:644-647）「取得元に残っていなかった期間（FR-85）の日は『動いていたが取れない状態だった』」「観測ではない記録は『記録あり』に数えない」。正典:182「収集開始日より古い記録が後から届いたとき、収集開始日をその日まで遡らせる」、Scenario `後から古い記録が届くと開始日が遡る`（:251）と `登録より前の時刻の記録は開始日を動かす`（:275）。改訂後の FR-79 ★（:474）は集計（`c01-app-usage-rollup`）だけを「同じ携帯のアプリ利用の収集開始日」にそろえる。コードも改訂前のままで、`crates/server/src/coverage.rs:255-256` は記録の当て直しの gate が逐語で `"true"`（ソースを問わない）、`crates/server/src/lib.rs:1181-1189` は重複にも当てる。いずれも ST14 の specs と下流が直す予定のもの（deep C9 / C10 / Q7 / Q8）。ただし正典の (5) は (7)「生存信号があれば…③」より先に評価されるので、FR-85 の期間の日を③にするには評価の順そのものを変える必要がある
- 直すところ: その文書（ST14 の specs で `collection-coverage` の 2 つの Requirement を MODIFIED にする。評価の順の変更も含める）
- 機械: 捕まえられた（正典の Scenario は印つきの試験が守っている。771 件すべてに印がある。MODIFIED にしないまま実装すると、その試験が落ちる）
- 行き先: docs（ST14 の specs で collection-coverage の日の状態と開始日の Requirement を MODIFIED にする。deep.md の「spec の段へ渡すもの」）

## C7. ST14 は稼働状況（S-1）を変えるのに、UI の方向では「面を持たない」に数えられていて、proto が作られない
- 重大度: 中
- 文書: docs/ui-direction.md:26
- 要件: FR-54, FR-79, NFR-17
- 根拠: ui-direction.md:17「S-1 稼働状況 | … | ST02, ST04, ST10, ST12, ST15」、:26-28「**面を持たない Story（16 本）**: … ST14, ST32（OS 通知）」。`python3 scripts/story_brief.py ST14 --screens` は `[]` を返す。deep.md:5「面は無い…ので proto は作らない」。ところが改訂後の FR-54 ★（:644-645）は「週を選んだときの文字に理由…を分けて期間と一緒に出す」と、S-1 の文字を変える。Q8 は集計（`c01-app-usage-rollup`）を格子に「出す」と決めたが、正典 external-ingestion:676 / :712 の並び「Must の 5 ソース・書庫のソース・退役したソース」に集計の行の置き場が無い（INDEX:202「`collection-coverage` の画面の Requirement を後から書き換える Story（ST14 / ST15 など）は、`external-ingestion` の並びの条項も見る」）。C10 では S-1 のライト（NFR-17。handoff st25 Q5）も ST14 が引き受けている。画面の Scenario は `web/e2e` が担保する（AGENTS.md）
- 直すところ: その文書（ui-direction の S-1 の出所 Story に ST14 を足し、「面を持たない」から外す。集計の行をどこに置くかは `kind: visual` の問いにする）
- 機械: 安く捕まえられる（Story の deep.md / requirements の ★ が FR-54・FR-55・NFR-17〜23 を変えているのに `story_brief.py --screens` が空、という組を、関門で 1 行の検査にする）
- 行き先: docs（ui-direction の S-1 に ST14 を足し「面を持たない」から外した。集計の行の置き場と週の文字は構造を変えないので D（仮）にし、GO 用 proto は reading で作る —— deep.md）

## C8. 75 日の根拠「Takeout の Chrome の履歴は取得元に 90 日だけ」が、PC のローカルの Chrome についての EXT-E を出典にしている
- 重大度: 中
- 文書: docs/requirements.md:413
- 要件: FR-35, FR-13, FR-14
- 根拠: FR-35 ★:413「Takeout 系の 60 日 × 3 = 180 日では、鳴った時点で **Chrome の履歴（取得元に直近 90 日だけ。EXT-E / FR-13）の約 3 か月ぶんが取得元から消えていて**」。EXT-E（:1242）は「Your History lists the pages you've visited on Chrome in the last 90 days」で、影響の欄は「FR-13」だけ。:1253-1255 の注記も「Chromium は**ローカルの履歴データベース**からも 90 日で削除する」（`history_backend.h`）。FR-13 は C-02 が PC の履歴 DB を読む経路の要件。`c03-chrome-history` は Takeout の `BrowserHistory.json` / `History.json`（`openspec/changes/archive/2026-10-13-st12-archive-ingestion/design.md:111`）で、Google 側にある同期の履歴の書き出し。その保持期間を述べる EXT の行は無い。Q9 は A（`uncaptured`）の問いで、この前提がそのまま「失われるもの」の見積もりになっている。また EXT-E の再確認期限（2027-03-07）が来ても、影響の欄に FR-35 が無いので、FR-35 の 75 日は見直しの対象に上がらない
- 直すところ: requirements（Takeout の Chrome の履歴の保持を EXT に立てて一次情報を引く。少なくとも EXT-E の影響に FR-35 を足す）
- 機械: 安く捕まえられる（要件の本文に出てくる `EXT-<x>` と、EXT の表の影響の欄を `check_chain.py` で突き合わせる）
- 行き先: requirements（FR-35 ★ を「PC の手元では 90 日」に直し、Takeout 側は一次情報が無いと書いた。EXT-E の影響に FR-35 を足した）

## C9. FR-35 の「そのソースを運んだ最後の書庫」と FR-16 の「それを運んだ書庫」は、同じ言葉で別の書庫を指す。既存のコードは FR-16 の側を引いている
- 重大度: 低
- 文書: docs/requirements.md:392
- 要件: FR-35, FR-16
- 根拠: FR-35:392「書庫から取り込むソースは『そのソースを運んだ最後の書庫』の時刻で測る」。FR-16:266「最終日は、入った記録のうちいちばん新しい出来事の日とする。**それを運んだ書庫の作られた時刻も併せて保持する**」。いまの口 `archives_status_for`（`crates/server/src/lib.rs:2280-2285`）は「**最終日を運んだ書庫だけを見る**（D11）」で、`ORDER BY ls.max_event_at DESC, l.created_at DESC` の先頭、つまり最も新しい出来事を運んだ書庫の時刻を返す。後の書き出しに古い出来事が入っていなかった場合（期間を絞った書き出しや、取得元で消えた後の書き出し）には、最後の書庫とは別の書庫になる。深掘り Q3 の context は「新しい行が 0 件でも分かる」なので、FR-35 の意味は台帳の最後の書庫のほうになる。時刻の列も決まっていない（台帳は `created_at`（書庫の名前から。無ければ見つけた時刻。`worker.rs:357-377`）・`discovered_at`・`finished_at` を持つ）。75 日と 90 日の間は 15 日しかない
- 直すところ: requirements（FR-35 に「書庫の作られた時刻」と書き、FR-16 の値とは別物だと書く）
- 機械: 安く捕まえられる（ST14 の試験に「後の書庫が古い出来事を運ばなかった」を 1 本置く）
- 行き先: requirements（FR-35 に「作られた時刻」と、FR-16 の書庫とは別物だと書いた）

## C10. FR-35 の測り方は「端末から来るソース」と「書庫から取り込むソース」の 2 つしか書いておらず、取り込み器とアカウント系の定期取得が入らない
- 重大度: 低
- 文書: docs/requirements.md:391
- 要件: FR-35, FR-15, FR-78
- 根拠: FR-35:391-392「経過時間は、端末から来るソースは端末で作られた時刻…で、書庫から取り込むソースは…最後の書庫の時刻で測る」。取り込み器 `s01-archive-inbox` の生存信号は S-01 が出す（正典 external-ingestion:575。deep C3「生存信号で測る（3 日で鳴る）」）。C-03 の定期取得（FR-15。ST13）は書庫でも端末でもない。どちらも C3 や ST13 が決めれば足りるが、FR-35 の逐語からはどちらの規則で測るのか読めない
- 直すところ: requirements（「それ以外（S-01 が作るもの）は生存信号の発信時刻」と 1 文足す）
- 機械: できない
- 行き先: requirements（FR-35 に「それ以外（S-01 が作るもの）は生存信号の発信時刻」を足した）

## C11. 「途絶の通知（FR-35）は S-01 が出す」が、FR-9 の理由の文と正典 `device-collection` に残っている
- 重大度: 低
- 文書: docs/requirements.md:176
- 要件: FR-9, FR-35
- 根拠: requirements.md:176「途絶の通知（FR-35）は S-01 が出すので、自宅 PC が止まっている間は鳴らない」。正典 device-collection:549 も同じ文。改訂後の FR-35:394 は「通知は携帯と PC の両方の収集アプリが…OS の通知として出す」。判定はサーバにあるので（deep C4）「S-01 が止まっている間は鳴らない」という結論は変わらないが、「出す」主体が変わった
- 直すところ: requirements（「S-01 が判定するので」に直す）。正典は次に `device-collection` を MODIFIED にする Story が直す
- 機械: できない
- 行き先: requirements（FR-9 を「S-01 が判定する」に直した）。正典 device-collection:549 は次に device-collection を MODIFIED にする Story が直す

## C12. FR-54 ★ は 2 つの理由をどちらも「取得元に残っていなかった期間（FR-85）」に入れている。送信契約と ST25 は、時計の食い違いの理由をそう呼ばない
- 重大度: 低
- 文書: docs/requirements.md:644
- 要件: FR-54, FR-85
- 根拠: FR-54 ★:644-645「**取得元に残っていなかった期間（FR-85）の日は『動いていたが取れない状態だった』**とし…理由（取得元に無かった / 時計の食い違いで諦めた）を分けて」。FR-85:127 は「取得の窓の始まりが、取得元がその時点で保持している範囲より前にある」ときだけを言う。送信契約（`docs/collector-contract.md:221`）は `clock_skew_abandoned` について「その期間を取得元に問い合わせたか」を「**していない**（取得元にはまだ残っていたかもしれない）」とする。走っている ST25 の spec（`openspec/changes/st25-day-timeline/specs/browsing-views/spec.md:411`）は「『スマホ 時計のずれで取れていない 6:00 – 9:00』の行があり、『取得元に残っていない』とは書かれていない」。状態を③にそろえる決定（Q7）とは矛盾しない。食い違っているのは見出しの語だけ
- 直すところ: requirements（「端末が取りこぼしとして残した期間（`kind: gap`。理由は 2 つ）」のように、FR-85 の名前を両方にかぶせない）
- 機械: できない
- 行き先: requirements（FR-54 ★ を「端末が取りこぼしとして残した期間（理由 2 つ）」に直した）

## C13. FR-54 ★ の「観測ではない記録」は分類で書かれていて、C9 の列挙（格子からは `clock-skew` と `gap` だけを除く）と範囲が違う
- 重大度: 低
- 文書: docs/requirements.md:646
- 要件: FR-54, FR-82, FR-81, FR-83
- 根拠: FR-54 ★:646「その期間を主張する記録や、PC の時計の測定記録の**ような**観測ではない記録は『記録あり』に数えない」。deep.md:59 の C9 は「格子の日の状態の数えからも `clock-skew` と `gap` を除く」。一方で「最後の記録」の側では `idle` / `powered-off` / `excluded` も観測でないとしている。`powered-off`（FR-82）は「PC が止まっていた期間」を主張する記録（`contract.rs:29-30`）で、分類の文面には当てはまるが、列挙には入っていない。`powered-off` の出来事の時刻は区間の始まり（`crates/collector-windows/src/marker.rs:108`）、つまりまだ動いていた時刻なので、実害は小さい
- 直すところ: requirements（「のような」をやめて列挙するか、C9 の列挙を FR-54 の分類に合わせる）
- 機械: できない
- 行き先: requirements（FR-54 ★ の「のような」をやめ、取りこぼしの期間の記録と PC の時計の測定記録に列挙した —— C9 の格子の列挙と同じ）

## C14. Q2 で取得可否が通知の引き金になった。取得可否の定義は「後から引き直せる」を理由に厳しい側へ倒してあり、走っている ST08 の「新しく鳴る場面は無い」が成り立たなくなる（印）
- 重大度: 中
- 文書: openspec/changes/st08-browser-history/deep.md:150
- 要件: FR-35, FR-78, FR-13
- 根拠: 改訂後の FR-35:390「生存信号は『取得できる状態』（FR-78）のものだけを『生きている』と数える」。ST08（下流で走っている）の spec（`openspec/changes/st08-browser-history/specs/desktop-collection/spec.md:440-441`）は、ブラウザ履歴について「見つかったブラウザ・プロファイルの**1 つでも**履歴を読めなかったとき…取得できない状態」とし、理由を「読めなかった対象を載せておけば、**数え方は後から引き直せる**」（:449）と書く。正典 desktop-collection:186-187 は、ウィンドウの取得可否に「URL を読み取る経路が応答すること」を含める。どちらの定義も、取得可否が格子と NFR-13（引き直せる数え方）にしか効かなかった時期に決めたもの。改訂後は、プロファイルが 1 つ読めないだけで 72 時間後に鳴り、URL の経路が応答しないだけで 18 時間後に鳴る。ST08 の deep.md:150-152「この Story で新しく鳴る場面は無い（ST07 Q8 で本人が許容を選んでいる）」は、記録と生存信号が届くかどうかだけを見た判断だった。鳴った通知は引き直せない。鳴りすぎる通知は切られる（FR-35 ★ 2026-09-10）。失われるデータは無い（計算し直せる B）
- 直すところ: 凍結した Story への申し送り（ST08 には差し戻さない。ST14 の上流で、通知に使う取得可否を blockers の種類で絞るかを決める）
- 機械: できない
- 行き先: docs（ST14 の deep.md。通知に使う「取得できない」を blockers の種類で絞るかを design の D（仮）にする。ST08 には差し戻さない）

## C15. 「途切れ 1 回につき音は 1 回」が、知らせる側が 2 台あるときに 1 台ずつなのか全体で 1 回なのか読めない
- 重大度: 低
- 文書: docs/requirements.md:395
- 要件: FR-35
- 根拠: FR-35:394-395「通知は携帯と PC の両方の収集アプリが…出す」「途切れ 1 回につき音は 1 回で、続く間は音の無い通知を残し、戻ったら黙って消す」。deep C6「鳴らしたこと・戻ったことを追記のみの表に残す」は、サーバ側の台帳を示している。台帳を全体で 1 つにすると 2 台目が鳴らず、1 台ずつにすると 1 回の途切れで 2 回鳴る。どちらも計算し直せる（B）
- 直すところ: requirements（「端末ごとに 1 回」か「全体で 1 回」かを書く）
- 機械: できない
- 行き先: requirements（FR-35 に「知らせる端末ごとに音は 1 回」と書いた）

## C16. ST14 の satisfies は FR-35 だけで、FR-54 / FR-79 / NFR-13 / NFR-17 の条項を前倒しで満たすことが INDEX に無い
- 重大度: 低
- 文書: docs/stories/stories.json:334
- 要件: FR-54, FR-79, NFR-13, NFR-17
- 根拠: stories.json の ST14 の `satisfies` は `["FR-35"]` だけ。deep.md:107「表題・価値・`satisfies` は変えない」。改訂後の FR-54 ★ と FR-79 ★ は、どちらも「ST14 の深掘り」を出所にしている。INDEX は、他の Story の本体の要件の条項を前倒しで満たすときに表を置く（ST04 の例: INDEX:170-175「要件 | 本体の Story | ST04 が満たす条項」）。ST14 には、NFR-13 の行（INDEX:219。ST12 の節）しか無い。FR-54（Q7）・FR-79（Q8）・NFR-17（C10）の表は無い
- 直すところ: stories（INDEX に ST14 の前倒しの表を足す）
- 機械: 安く捕まえられる（`★ … ST<NN> の深掘り` を持つ要件の ID が、その Story の satisfies にも INDEX の前倒しの表にも無ければ warn にする）
- 行き先: stories（INDEX に ST14 の前倒しの表 FR-54 / FR-79 / NFR-17 を足した）

## 未確認

- Google の Takeout が書き出す Chrome の履歴（`c03-chrome-history`）が、実際に何日ぶんを持つか（C8）。repo の中に一次情報が無い。本物の書庫のいちばん古い訪問の時刻と書き出しの時刻の差を測れば決まる
- 形の確認の印を待っている書庫（FR-14 ★。台帳の `outcome = 'pending_shape'`）が、`archive_ledger_source` に行を持つか。持たないなら、書き出したのに印を置いていない間は「最後の書庫」が進まず、75 日で「書き出しを忘れている」と鳴る。`worker.rs` の読み直しの経路（D18）までは追っていない
- ST17 の下流が `collector-android` に足す通知（通知チャネル・`RECEIVE_BOOT_COMPLETED`・AlarmManager）と、ST14 が足す通知が同じファイル（`MainActivity.kt`・マニフェスト）に当たるか。ST17 の枝は読んでいない
- C-02 が Windows の通知（トースト）を、スタートアップのフォルダから起動した未パッケージの exe（`autostart.rs`）のまま出せるか（AUMID の登録が要るか）。実装の見積もりで、文書どうしの食い違いではない
- 正典 `openspec/specs/*/spec.md` の Scenario 771 件はすべて `crates` `web/src` `web/e2e` `collector-android/app/src` `tools` に印がある（空白を無視して照合。0 件欠け）。印の先の試験が Scenario を観測しているかは見ていない

## 触らなくてよいと判断したもの

- **`crates/server/src/coverage.rs:28` の `DEVICE_SUBJECT` に `c01-app-usage` が残る（NFR-13 と食い違う）** —— 改訂の前からある食い違いで、宛先は `docs/handoff/ST14.md` の st06 R6。ST14 の deep C10 が引き受けている
- **FR-35 ★ 2026-09-11 の「書庫系は 180 日後から永久に」と、想定間隔の初期値「Takeout 系 = 60 日」** —— 前者は経緯の ★ の引用。後者は FR-80（格子の⑥）の想定間隔として生きている（75 日は通知だけの列）
- **`docs/production-prep.md:241`「想定間隔の 3 倍で通知する条件（FR-35）」** —— 製造準備の時点の記述で、書庫以外には今も成り立つ。実装の担当（ST14）も変わらない
- **ST14 の完了の判定「位置の収集を 18 時間以上止めると通知が来る」** —— 携帯の問い合わせは 6 時間ごと（C11）なので、鳴るのは 18〜24 時間後になる。「以上」と読めば成り立つ。意図的な停止（FR-34）では鳴らない（C2）こととの読み分けは、2026-09-09 からある文面で、この改訂が持ち込んだものではない
- **正典 device-collection:550「常時出ている通知は増やさず（文言を足すだけ）」と、FR-35 ★ の「続く間は音の無い通知を残し」** —— 前者は FR-9（未送信の日数）の本人の決定の理由で、途絶の通知には及ばない。常駐の通知に文言として足すか別の通知にするかは、下流が決める表示の B
- **FR-79 ★ の理由「集計は過去の分をまとめて取ったもの」と、ST02 第 9 回 Q32（ブラウザ履歴の初回 90 日・Takeout の 1 年ぶんで開始日が遡るのは正しい）** —— 例外は集計の 1 ソースだけを名指ししていて、Q32 の規則は他のソースに残る。違いは、集計の記録は年・月の箱に 1 件ずつしか無く、遡った日がほとんど②/⑥になる点。★ の本文はそれを書いている（「導入前の 1〜2 年が格子に②/⑥で並ぶ」）
- **`docs/collector-contract.md:469`（生存信号の間隔は FR-35 の初期値に合わせる）** —— 改訂は想定間隔の値を変えていない
- **`openspec/specs/record-deletion/spec.md:245`（稼働状況は削除済みの記録も数える）** —— FR-54 ★ が除くのは観測でない記録で、削除とは独立している
- **走っている change への印**（差し戻しではない。どれも失われるもの無し）:
  - `st17-daily-feeling` —— spec の「主観を書かない日は途絶として扱わない」（subjective-log spec:406-412）は改訂後の FR-35 とも合う。携帯の通知のコードが重なる点は C2 と「未確認」
  - `st21-place-registry` —— `s01-place` が FR-35 の除外に入っていない（C3）。判定はまだどこにも無いので、ST21 が作るものは変わらない
  - `st25-day-timeline` —— 取りこぼしの 2 つの理由を行の文字で分けており、FR-54 ★（③に畳み、理由は分けて出す）と同じ向き。語の違いは C12
  - `st08-browser-history` —— C14 の印のみ。spec の FR-79 の名指し（初回 90 日で開始日が遡る）は、FR-79 ★ の集計の例外の影響を受けない
