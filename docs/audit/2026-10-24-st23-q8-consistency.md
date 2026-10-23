# 整合の確認（2026-10-24・ST23 の深掘り第 2 回 Q8 を FR-50 へ戻した改訂）

- 基準: `84c63132da479c47dea4584bb645ee6c90f1190a`..`942c60f1ad0013f84d2d31e0b2ccdfb236f83bde`

対象は commit b9c85a38 が FR-50 に足した 1 段落（`docs/requirements.md:699-702`。「本文ごと消した滞在の時間帯に重なる滞在が、後の作り直し（吸収・時間帯で隠す）で隠れたら、
隠した同じまとまりで本文も消し、消去の台帳に『後から』として載せる」「一覧に出ている（隠れていない）間の滞在は消さない」）。文書は head（`942c60f1`）の内容で読んだ。

機械の入力:
- `req_impact.py --base 84c63132`: 改訂 1 件（FR-50）・扉 0 項。名指しは `docs/handoff/ST12.md` `ST22.md` `ST23.md` `ST33.md` `ST45.md` `ST47.md`・`docs/openapi.json`・
  `docs/production-prep.md`・`docs/stories/INDEX.md`・`docs/stories/stories.json`・`docs/ui-direction.md`・正典 `openspec/specs/` の 9 capability・
  走っている `ch02-export-next-record-window` `st09-media-capture` `st23-record-erasure` `st29-satellite-credentials` `st41-event-end-time` の文書
- `check_chain.py`: 要件 142 / Story 47 / 拾われた要件 140/142・未回収 0・warn 0。FAIL 1 件は「整合の確認の後に要件が変わった: FR-50, FR-51, FR-52」（この監査が受けるもの）

走っている change のうち、この改訂を受けるのは `st23-record-erasure`（出どころ）だけ。st23 への指摘は**印を付けるだけ**にした（C1・C6）。
失われるもの（uncaptured / discarded / exported / rewrite-all）を伴う印は無い（決定は deep.md・FR-50・go.json に書かれていて、仕様に写っていないだけ）。

## C1. FR-50 の Q8 の段落を満たす Requirement が、合意した st23 の spec・design・tasks に無い（印のみ）
- 重大度: 高
- 文書: openspec/changes/st23-record-erasure/specs/record-deletion/spec.md:139
- 要件: FR-50
- 根拠: FR-50（`docs/requirements.md:699`）「本文ごと消した滞在の時間帯に重なる滞在が、後の作り直し（吸収・時間帯で隠す）で隠れたら、隠した同じまとまりで本文も消し、消去の台帳に『後から』として載せる」。
  st23 の spec は範囲の Requirement（spec.md:139-148）に「削除の印の無い滞在（一覧に出ている滞在）の本文を、時間が重なっていても消さない」と Scenario「一覧に出ている滞在の本文は消えない」（spec.md:193）を持つが、
  その滞在が後で隠れたときに本文を消す Requirement・Scenario はどこにも無い（`後から届いた記録は、入れた後に本文も消す` の Requirement（spec.md:343-351）は位置の記録だけ）。どの Requirement の導出元も FR-50 の ★ 2026-10-23 を挙げていない。
  design.md:47「D17 … 未決（Q8。仮は否）」、design.md:267「**Q8 待ち** … この節と spec を書き直す」、tasks.md:26「D2・D6・D8・D9・D10〜D17 は（仮）決め」、tasks.md:58「本人が別に消した滞在と印の無い滞在は入れ」ない（Task 2 に Q8 の作業が無い）。
  一方、deep.md:84「spec に Requirement を足す（次の `spec`）」、go.json の未解決「この 2 つの答えは、まだ specs・design・tasks・GO 用 proto に写っていない（答えが推奨と同じで、**グラフが仕様を書く段を通らなかった**）」。
  合意（`agreement.json` `"status": "accepted"`、dd345639）はこの未解決を見せたうえで通っており、仕様を書く段が次に来る経路は文書のどこにも無い。
  また FR-50 の同じ段落の「本人がその滞在を消して本文ごと消せる」は、その滞在が P の時間にすっぽり入るとき Q7 の答え（B の行を P の行の下に残す）に依るが、Q7 も spec に写っていない（go.json の未解決。D12 と 1 日の一覧の Scenario）
- 直すところ: その文書（走っている st23 の specs/record-deletion・design D17・tasks Task 2。印のみ）
- 行き先: harness —— 深掘り第 2 回の答え（Q7・Q8）が推奨と同じだったので、グラフが record の後の `spec` 段を通らず GO と合意へ進んだ。Q7・Q8 は spec を書いた後に仮の否から生まれた問いで、推奨はまだ spec に無い（design D2・D17 は「Q7/Q8 待ち」、deep.md の写す先は「次の `spec`」）。st23 の spec 段を回して D2・D12・D17・spec・tasks に写す必要がある。gate の直しの席では spec 段をやらない（人間に報告した）
- 機械: 安く捕まえられる（deep.md の「要件へ戻すもの」が挙げた FR の ★ の日付を、change の specs の「導出元:」のどれかが名指すかを見る検査。いまは無い）

## C2. Q8 の段落の「重なる」が、作り直しが隠す条件（触れるだけでも重なり）と、st23 の範囲の条件（端が触れるだけは重ならない）のどちらかを決めていない
- 重大度: 中
- 文書: docs/requirements.md:699
- 要件: FR-50
- 根拠: FR-50（:699）「本文ごと消した滞在の時間帯に**重なる**滞在が、後の作り直し…で隠れたら」。作り直しが隠す側は、FR-50 ★ 2026-09-13（:657）「消した時間帯に**少しでも**重なる派生は、丸ごと削除済みにする」、
  正典 `openspec/specs/derived-records/spec.md:272`「少しでも時間が重なる滞在を、削除済みとして作る」、コード `crates/server/src/stay_store.rs:540-543`「触れるだけでも重なりとする（Q12…）」`f.start <= *e && *s <= f.end`。
  st23 の範囲の側は、spec.md:148「端が触れるだけで時間が重ならない滞在を、重なる滞在に含めない」、design.md:72「`a.start < b.end AND b.start < a.end`。端が触れるだけは重なりにしない」。
  端だけ触れる滞在は作り直しで隠れる（`rebuild:erased-range`）のに、st23 の定義で読むと Q8 の対象にならず、本文ごと消す口は作り直しが隠しただけの滞在を 409 で断る（spec.md:7-8）—— Q8 が無くそうとした「どの口からも本文を消せない」状態がその分だけ残る。
  連鎖は時間帯の両端を含めて位置を消す（`stay_store.rs:722` `event_time >= {start} AND event_time <= {end}`）ので、P を消す前に作られて P の端に触れる滞在が消した位置と同じ時刻の位置を持つときは、その座標が本文（と前の版）に残る（時刻の一致に依る）
- 直すところ: requirements（Q8 の段落の「重なる」を、作り直しが隠す条件と同じ（触れるだけを含む）か、st23 の範囲と同じ（含まない）かを書く）
- 行き先: leave: Q8 の段落の引き金は「作り直しで隠れたら」で、隠すかどうかは作り直しの規則（触れるだけでも重なり）が決める。st23 の「端が触れるだけは含めない」は本文ごと消す時点の範囲（別の操作）の規則で食い違わない。Q8 を spec に写す段（C1）がこの読み方で Requirement と Scenario を書く
- 機械: できない（散文の語の定義。`touches_erased` と spec の文の食い違いは試験の Scenario を置けば捕まる）

## C3. Q8 の段落の「隠れたら」が、作り直しが初めから隠して作る滞在を含むかを決めていない
- 重大度: 中
- 文書: docs/requirements.md:699
- 要件: FR-50
- 根拠: FR-50（:699）「後の作り直し（吸収・時間帯で隠す）で**隠れたら**」「一覧に出ている（隠れていない）間の滞在は消さない」—— 一覧に出ていた滞在が隠れる場合しか書いていない。
  作り直しは、消した時間帯に触れる新しい滞在を**隠したまま新しい行として作る**（`crates/server/src/stay_store.rs:584` `let hide = touches_erased(f, &erased);`、:605-607 `insert_stay(…, hide)`、:1009 `CASE WHEN $13 THEN now() END, CASE WHEN $13 THEN $14 END`。
  正典 `derived-records/spec.md:272`「削除済みとして作る」）。この行は一度も一覧に出ていない。
  含めると: 本文を消した滞在は次の作り直しで一致の候補にならない（st23 spec.md:237）ので、作り直しのたびに新しい隠れた行ができて本文を消し、行と「後から」の台帳の行と「後から届いた N 件」が作り直しの回数だけ増える。
  含めないと: その行は本文を持ったまま隠れ、本文ごと消す口は 409 で断る（spec.md:7-8）。起きるのは、P の前後の生きた位置が `gap_minutes`（既定 10 分。`crates/server/src/stay.rs:30`）より近く 1 つの滞在にまとまるとき
- 直すところ: requirements（作り直しが隠して作る行を Q8 の対象に含めるかと、含めるなら作り直しのたびに増えないことを書く）
- 行き先: leave: 作り直しが初めから隠して作る行は一度も一覧に出ておらず、Q8 の段落（一覧に出ていた滞在が隠れたら）の対象かは spec に写す段（C1）が Requirement と Scenario で決める。決め方によって要件の本文は変わらない
- 機械: できない

## C4. Q8 の段落は ST23 だけが作る FR-50 の条項だが、INDEX の「satisfies に無い条項を ST23 が満たす」に載っておらず、鎖では済んだ ST22 が FR-50 を拾ったままになる
- 重大度: 中
- 文書: docs/stories/INDEX.md:424
- 要件: FR-50
- 根拠: INDEX.md:424「**要件の側でも、satisfies に無い条項を ST23 が満たす**: FR-50 ★ 2026-09-29（…）」—— 挙げるのはこの 1 つだけ。
  `docs/stories/stories.json` の ST23 は `"satisfies": ["FR-51", "FR-52"]`、FR-50 を satisfies するのは ST22（`openspec/changes/archive/2026-10-04-st22-record-deletion`。archive 済み）。
  Q8 の段落（requirements.md:699-702）を作るのは ST23 だけ（deep.md:84「spec に Requirement を足す」）で、同じ形の FR-50 ★ 2026-10-23 訂正（Q3。:693-698）も載っていない。
  `check_chain.py` は FR-50 を ST22 で「拾われた」と数えるので、st23 が Q8 を作らずに終わっても鎖は落ちない（C1 と重なると誰も作らない）
- 直すところ: その文書（`docs/stories/INDEX.md` の ST23 の訂正の節に、FR-50 ★ 2026-10-23 の訂正（Q3）と追加（Q8）を ST23 が満たすと足す）
- 行き先: stories —— `docs/stories/INDEX.md` の ST23 の 2026-10-22 の訂正の節に、FR-50 ★ 2026-10-23 訂正（Q3）と追加（Q8）を ST23 が満たすと足した
- 機械: 安く捕まえられる（★ が「ST<NN> の深掘り」を名乗り、その FR が ST<NN> の satisfies に無いとき、INDEX の訂正の節がその FR と ★ の日付を名指すかを見る。いまは `check_chain.py` に無い）

## C5. FR-52 は本文ごと消す指示が確認の後に及ぶ先を「後から届く分」とだけ書き、確認の時点で DB にあって数えなかった滞在を後で消すことを書いていない
- 重大度: 中
- 文書: docs/requirements.md:722
- 要件: FR-52, FR-50
- 根拠: FR-52（:720-723）「実行前に対象件数を提示して確認を求める。**本文ごと消す指示は、指示した時間帯に後から届く分にも及ぶ**（後から届いた分は確認をはさまず…）。確認に出す件数は、押した時点で DB にある分」、
  その理由（:724）「後から届く分の件数を前もって示せないので」。Q8 で後に本文を消す滞在 L は**押した時点で DB にあり**、確認の件数には入らない（st23 spec.md:147、design.md:263-264 D17「入れない」）。
  st23 の確認の既定は「見せていない件数を消さない」（spec.md:126「深掘り C7（既定は厳しい側。戻せない操作で、見せていない件数を消さない）」）。
  FR-52 だけを読むと、確認の後に及ぶのは後から届く記録だけで、確認で見せずに後から消える滞在があることが分からない（確認の文面に「後から届く分にも及ぶ」とだけ出す実装になりうる）
- 直すところ: requirements（FR-52 の「及ぶ」先に、後の作り直しで隠れる重なる滞在を足し、FR-50 の Q8 の段落を指す）
- 行き先: requirements —— FR-52 に、指示が後の作り直しで隠れる重なる滞在にも及ぶことを ★ 2026-10-24 補足で足した
- 機械: できない

## C6. 1 日の一覧の「後から届いた N 件も本文ごと消した」が、Q8 で数える作り直しで隠れた滞在を「届いた」と呼ぶことになる（印のみ）
- 重大度: 低
- 文書: openspec/changes/st23-record-erasure/specs/record-deletion/spec.md:533
- 要件: FR-50
- 根拠: deep.md:83 Q8 の写す先「消去の台帳に『後から』（kind = late、元の操作 = P）として載り、P の行の『**後から届いた N 件**も本文ごと消した』に数える」。
  record-deletion spec.md:533「その時間帯に**後から届いて**本文を消した記録があれば、その件数を『本文ごと消した』の行に添える」、:578「後から届いた 2 件も本文ごと消した」、design.md:218 同じ文面。
  FR-50（:699）は「消去の台帳に『後から』として載せる」で、「届いた」とは言っていない。隠れた滞在は届いたのではなく作り直しが隠したもの
- 直すところ: その文書（走っている st23 の record-deletion spec の 1 日の一覧の行の Requirement・design.md:218 の文面。印のみ）
- 行き先: leave: 印のみ。走っている st23 の spec・design の文面で、Q8 を spec に写す段（C1）が「後から」の数え方と一緒に直す
- 機械: できない

## 未確認

- st23 で仕様を書く段（`spec`）がこの後に走るか。go.json は「グラフが仕様を書く段を通らなかった」と書き、合意は済んでいる。下流が合意した spec（Q7・Q8 を持たない）のまま始まるかは、グラフの状態を見ていないので確かめていない（C1 の重大度はこれに依る）
- `agreement.json` の `head`（`435f5ef9`）は HEAD の祖先ではない（rebase 前の commit。同じ題の `80f16ab4` が作り直した版）。harness2 の `verify_checklist.since_agreed` はこの head と `git diff` するので、
  その commit が gc で消えると合意からの差分を出せない（`None` で返る）。鮮度は digest でも見ているが、このリポジトリでの影響は確かめていない
- Q8 で本文を消した滞在に st23 design D6 の `user:purged` を付けるか（付けると今のコードの `Mark::of`（`stay_store.rs:258-266`）は本人の削除と読む）。spec が書かれていないので確かめられない。C3 の「含める」側の振る舞いはこれで変わる

## 触らなくてよいと判断したもの（理由つき）

- **FR-50 の段落が言う作り直しの事実**: 「吸収・時間帯で隠す」はコードの 2 つの印（`stay_store.rs:20` `rebuild:absorbed`、:22 `rebuild:erased-range`）と一致。隠す経路は `rebuild_day` だけ（呼び出しは `stay_store.rs:1109` の `rebuild_all` と `lib.rs:417`）で、
  `rebuild_day` は 1 トランザクション（`stay_store.rs:558` の doc「1 トランザクション」、:564 `pool.begin()` … :616 `tx.commit()`）なので「隠した同じまとまりで本文も消す」は置ける。滞在の判定は `core.event_live` だけを読む（`stay_store.rs:180-184` `POINTS_SQL`）ので、消した位置から作られた滞在は次の作り直しで点を失う —— FR-50 の ★ の理由は成り立つ。滞在を消す操作（`deletion.rs:240-`）は重なる滞在を直接隠さない
- **正典の Scenario の印**: `record-deletion` と `derived-records` の全 Scenario に `Scenario: <名前>` の印がテストにある（機械で全件確かめた。欠け 0）。この改訂は正典の Scenario を変えない
- **`docs/stories/stories.json` の ST22 / ST23 の表題・価値・完了の判定**: Q8 は消す範囲を広げる側で、ST23 の完了の判定（「実行前に対象件数が出て確認を求められる」「本文が DB から消え、…が残っている」）と ST22 の判定を崩さない。確認の外で消える分の書き方は C5 で要件に寄せる
- **`docs/ui-direction.md:21`・`docs/production-prep.md:74,181`・`docs/handoff/ST12/ST22/ST33/ST45/ST47.md`・`docs/openapi.json:449`**: FR-50 を名指すのは論理削除のビュー・写真・連鎖のソース・属性の主張の話で、作り直しで隠れた滞在の本文には触れない
- **走っている `st09` `ch02` `st29` `st41` と `docs/handoff/ST12.md:19`・`ST33.md:17`・`docs/openapi.json:449` の FR-50 の名指し**: 写真・書き出しの除外・衛星の書き手・外部の更新・ビュー越しの読み出し・終わりの時刻の話で、Q8 の段落を受けない。
  st41 は消した滞在の時間の読み方（`core.stay_erased` の `end_at` を列から先に読む。st41 design.md:154-159・specs/data-durability/spec.md:8）を変え、st23 の「台帳の範囲から読む」（D5・D13）と同じビューに触るが、
  これは Q8 の前からある走っている change どうしの重なりで、この改訂は増やさない（Q8 の「時間帯」は st23 の範囲の定義に乗る）
- **FR-51 / FR-52 が `check_chain.py` の FAIL に並ぶこと**: 84c63132 からの差分で変わった要件は FR-50 だけ。FR-51 / FR-52 は前の監査（`2026-10-23-st23-consistency.md`）の行き先を反映した改訂で、その中身は前の監査が見た。この監査の基準の head が最新になるので、鮮度の判定はこの監査で受ける
- **Q7 の答え**: 要件の本文を変えない（deep.md:90）ので、この監査の対象外。spec に写っていないことは C1 の根拠にだけ書いた
