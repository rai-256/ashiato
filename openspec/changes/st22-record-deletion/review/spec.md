# ST22 成果物の独立レビュー（proposal / specs / design / tasks）

書いた文脈を持たない目で、`deep.md` の決定 → `specs/` → `design.md` → `tasks.md` の写りと、正典・既存コード・並走 Story との整合だけを見た。
**成果物は 1 つも触っていない。**

確かめた範囲: `openspec/changes/st22-record-deletion/**`（proposal / deep / deep-questions.json / review/deep.md / specs 2 本 / design / tasks）、
`openspec/specs/{browsing-views,derived-records,record-envelope,collection-coverage}/spec.md`、
`docs/requirements.md`（FR-31 / FR-44 / FR-45 / FR-50〜FR-54 / FR-76 / NFR-13 / NFR-17〜NFR-23）、
`docs/stories/ST22.md` / `INDEX.md` / `stories.json`、`docs/handoff/{ST22,ST23}.md`、`docs/ui-direction.md`、`docs/production-prep.md` A-3、
`crates/server/src/{lib.rs,stay_store.rs,coverage.rs,stay.rs}`、`migrations/*.sql`、`tools/{check-immutable.sh,check-migrations.sh,seed.sh,stack.sh}`、
`web/{package.json,playwright.config.ts,src/stays.ts,src/__tests__/}`、`.github/workflows/ci.yml`、`scripts/board.py` の出力、
並走中の `../ashiato2-up-st05/openspec/changes/st05-clock-skew/`（specs はまだ無い）。

## 機械の検査（人間の目より先に）

```
$ openspec validate st22-record-deletion --strict
Change 'st22-record-deletion' is valid                                   rc=0

$ python3 scripts/check_chain.py .
要件 118 件 / Story 36 本 / 扉 26 項
[ok] どれかの Story に拾われた要件: 116/118 件
chain: OK (0 件 / 未回収 0 件 / warn 0 件)                                rc=0

$ python3 scripts/check_scenarios.py . st22-record-deletion
Scenario 52 件（record-deletion 34 / browsing-views 18）
scenarios: FAIL (担保なし 42 件 / 名無しの確認待ち 0 件)                   rc=1

$ python3 scripts/review_triage.py . st22-record-deletion
指摘 13 件 / 仮決め 0 件 / 要件へ戻すもの 1 件
triage: OK                                                               rc=0
```

`check_scenarios` の 42 件は**上流では正常**（`merge_gate.sh:104-132` が「上流では tasks と check_scenarios は原理的に満たせない」として飛ばす。テストがまだ 1 本も無い）。
担保なし 42 件 = この change の 52 件 − ST16 が既に印を持つ 10 件で、`tasks.md:20-23` の自己申告（52 本 / 据え置き 10 本 / 意味が変わった 2 本）と一致した。
52 本すべてが `tasks.md` のどこかに名前で現れることも機械的に確かめた（0 件の取りこぼし）。
MODIFIED 3 本が正典の当該 Requirement の SHALL と Scenario を落としていないことも機械的に確かめた
（見出し 3 本とも一字一句一致。落ちた Scenario 0 件。書き換えた SHALL は
`削除済みの滞在と、作り直しで吸収された滞在を一覧に出さない` → `…を、滞在の行として一覧に出さない` の 1 行だけで、これは意図した改訂）。

---

## R1. 「個人属性の主張を消す操作は ST22」と INDEX が書いているのに、design が Non-Goal にしている。どの Story も拾わない条項になる
- 分類: 他 Story との食い違い
- 成果物: openspec/changes/st22-record-deletion/design.md:35-36（Non-Goals）
- 根拠: docs/stories/INDEX.md:162「| FR-50（削除） | ST22 | 主張の行の錠が削除の印を通し、読み出しが削除の印を効かせる（Q1）。**消す操作は ST22**。`docs/handoff/ST22.md` |」（ST19 の上流工程の訂正） /
  docs/requirements.md:435-437 FR-44 ★ 2026-09-15「主張は『本人が書いた』記録で、**消したことにする（FR-50）**と本文の消去（FR-51）は主張にも及ぶ」（**本人の決定**。ST19 深掘り Q1） /
  docs/handoff/ST22.md:46-53「消す操作（画面・API）は ST22 まで存在しない。ST19 は錠を開けておくだけ」 /
  design.md:35-36 は「滞在以外を 1 件ずつ消す口（C3）…は、それを画面に見せる Story が足す（個人属性は ST19 が錠を開けて待っている）」と書くが、**その Story を名指ししていない**。
  `docs/handoff/` に ST25 宛て（INDEX:164 が個人属性の画面 S-6 を ST25 に割り当てている）のファイルは無い（`ls docs/handoff/` = README, ST02, ST11, ST12, ST14, ST22, ST23） /
  `check_chain.py` は FR-50 → ST22 の 1 対 1 しか見ないので、ST22 が archive された時点でこの条項は**誰にも拾われないまま緑になる**
- kind: conflict
- 提案: 3 つのどれかを選ぶ。(a) INDEX.md に ST22 の訂正として「FR-50 の主張の条項は ST22 では作らず、S-6 を作る Story（ST25）が足す」を書き、`docs/handoff/ST25.md` を作って ST19 の Q1（消した主張の取り消しの扱いまで）を丸ごと移す。(b) C3 を広げて主張も消せる口を ST22 で作る。(c) Non-Goals に「ST25 が足す」と Story 番号を書く。いずれにせよ **Non-Goals に書くだけでは鎖が切れる** —— 申し送りは受け取り側のファイルにしか残らない。
- 処置: deferred ST23 —— ST22 の口は C3 で滞在だけに狭めた（本人に見せ、異論なし）。`docs/handoff/ST23.md` に根拠ごと置き、`docs/stories/INDEX.md` の ST22 の訂正で条項の宛先を ST19 の訂正（:162）から ST23 へ直した。design の Non-Goals も宛先を名指しに直した

## R2. Q1（A / loss: exported）を成り立たせているのは「書き出しが削除済みを出さないこと」なのに、ST33 への申し送りが無い。ST33 はいま着手可
- 分類: 抜け（不可逆の担保）
- 成果物: openspec/changes/st22-record-deletion/proposal.md:84（Impact の ST33 の行）
- 根拠: deep.md:31-39 Q1 は `kind: irreversible / loss: exported` に**上げられた**問い（review/deep.md R5）で、根拠は「滞在だけを消すと、その時間の位置が書き出しで外へ出て戻らない」 /
  proposal.md:84「書き出しが削除済みを出さないことは **ST33 が決める**。ST22 は『滞在を消すと位置も削除済み』にしたので、削除済みを除けば場面は外に出ない」 ——
  つまり **loss: exported を閉じる条件の半分を別 Story に預けている** /
  `python3 scripts/board.py` → 「[着手可] ST33 全記録を外部の道具で読める形に書き出す data-durability requires: ST01」（いつでも上流が始まる） /
  `ls docs/handoff/` に ST33.md は無い / review/deep.md R5 の提案も「`docs/handoff/ST33.md` に申し送る」を挙げていた
- kind: irreversible
- loss: exported
- 提案: `docs/handoff/ST33.md` を作り、「ST22 は Q1 で『滞在を消すと位置も削除済み』を本人の決定にした。書き出しが `core.event`（素のテーブル）を読むと、本人が消した場面の緯度経度がそのまま外へ出る。読むのは `core.event_live`（削除済みを出さない）にするか、出すなら本人に問う」を根拠つきで置く。Non-Goals の「書き出しの側の除外（ST33）」（design.md:43）はこの申し送りを指すこと。
- 処置: escalated —— deep.md の「specs の独立レビューが人間へ返したもの」に spec R2 として記録し、`docs/handoff/ST33.md` を作って根拠ごと置いた（ST33 の深掘りで本人に問われる）。proposal の Impact にも書いた。**ST22 の側で決められることは無い**（書き出しの経路がまだ 1 本も無い）

## R3. 「消した」区間を**移動**の計算から差し引くことが specs に無い。正典の「滞在と滞在の間は移動の行として出る」を MODIFIED していないので、archive すると正典が実装と食い違う
- 分類: 置き場（design に観測可能な振る舞い）
- 成果物: openspec/changes/st22-record-deletion/specs/browsing-views/spec.md（`滞在と滞在の間は移動の行として出る` が MODIFIED に無い）・design.md:155
- 根拠: openspec/specs/browsing-views/spec.md:59-64 の正典は
  「THE SYSTEM SHALL 一覧で、滞在と滞在の間の時間を『移動』の行として…出す。THE SYSTEM SHALL **ただし位置の記録が無い時間は移動の行に含めない**。」——
  例外は「位置の記録が無い時間」の 1 つだけで、**消した時間は例外に入っていない** /
  design.md:155「『消した』の区間は、記録なし・**移動**の計算から先に差し引く」（specs にあるのは記録なし側だけ ——
  change の browsing-views spec.md:76 は「本人が消した時間を『記録なし』の行にしない」しか書いていない） /
  crates/server/src/stay_store.rs:1007 `covered.extend(hidden);`（いまも移動から外している。**spec ではなくコードだけが持っている**） /
  docs/handoff/ST22.md:41-43 R35「空白を埋める規則のままだと、とどまっていた時間が『移動 1 時間 1 分』と出た（code-verify の実測）」——
  この実測こそが移動側の規則を正典に書く理由で、ST16 は書かずに ST22 へ渡した /
  `openspec archive` は main specs しか更新しないので、design.md:155 は正典に残らない
- kind: technical
- 提案: `滞在と滞在の間は移動の行として出る` を MODIFIED に足し、「本人が消した時間（と重なって隠された滞在の時間）も移動の行に含めない」を SHALL にして、Scenario を 1 本置く（例: 前後に位置がある日で 10:00–11:00 の滞在を消す → 10:00–11:00 に「移動」の行が無い）。いま「消した」の行を出す規則が 2 本の Requirement（記録なし・日付を指定すると…）に分かれていて、移動だけが抜けている。
- 処置: fixed specs/browsing-views/spec.md —— 「滞在と滞在の間は移動の行として出る」を MODIFIED に足し（正典の全文＋「本人が消した時間も移動の行に含めない」）、Scenario `消した時間は移動の行にならない` を置いた。tasks 5.2 に足した

## R4. 「消した」区間を差し引いた**残りの断片**をどう扱うかが決まっていない。deep が「specs で決める」と名指しした 3 件のうちの 1 件
- 分類: 抜け（検証不能）
- 成果物: openspec/changes/st22-record-deletion/specs/browsing-views/spec.md:102-105（Scenario `位置ごと消した時間は記録なしにならない`）・design.md:155
- 根拠: deep.md:86-87「playground が描かなかったもの（**specs で決める**）: …/ **消した行と前後の移動・記録なしの境目**（proto は近似）」 /
  crates/server/src/stay_store.rs:960-991 —— 「記録なし」は**生きている位置の隣り合う間隔**から作る。10:00–11:00 の滞在と位置を消すと、生きた位置の間隔は
  「10:00 直前の点 〜 11:00 直後の点」（例 09:59–11:01）になり、消した区間 [10:00, 11:00] を差し引くと **09:59–10:00 と 11:00–11:01 の 2 断片**が残る /
  stay_store.rs:978 `if b - a < gap { continue; }` —— 正典（openspec/specs/browsing-views/spec.md:73-74）の「記録なし」は
  「隣り合う位置の記録の**間隔が、記録が無いとみなす間隔以上ある区間**」なので、**1 分の「記録なし」の行は正典に反する** /
  いまの Scenario の THEN は「10:00 – 11:00 は『消した』の行として出て、『記録なし』の行にはならない」だけで、
  断片が行になっても**真のまま通る**（断片を出す実装と出さない実装のどちらも緑）
- kind: technical
- 提案: どちらかを SHALL にして Scenario を足す。(a) 差し引いた残りは**改めて「記録が無いとみなす間隔」で測り直す**（断片は行にしない）、(b) 残りは「消した」の区間に**呑ませる**（消した行を前後の点まで広げる）。(a) を採るなら Scenario は「前後に位置が続く日で滞在を消す → 『記録なし』の行が 1 つも増えない」、(b) なら「消した行の始まり・終わりが消した滞在の始まり・終わりと一致する」のように、断片の有無で真偽が変わる形にする。
- 処置: fixed specs/browsing-views/spec.md —— 提案 (a) を採り、「差し引いた残りをあらためて『記録が無いとみなす間隔』で測り直す」を SHALL にし、Scenario `消した時間の端に短い記録なしの行が生えない`（行の数が増えないことで真偽が分かれる）を置いた。design D7 にも 1 行

## R5. D7 の「重なる / 隣り合う区間はつないで 1 行」は観測可能な振る舞いなのに design にしかない。archive で正典から落ちる
- 分類: 置き場
- 成果物: openspec/changes/st22-record-deletion/design.md:147-159
- 根拠: design.md:152-153「重なる / 隣り合う区間は**つないで 1 行**にする。理由: `rebuild:erased-range` の滞在は消した範囲より広いことがあり…別々に出すと同じ時間に 2 行が重なる」——
  これは応答の**行数**と**行の時刻の範囲**を決めるので、画面から数えられる /
  specs 側の痕跡は browsing-views spec.md:7「『消した』の区間に、その区間を戻すための滞在の識別子（本人が消した滞在。**1 件以上**）を添える」の「1 件以上」だけで、
  **2 件の消した滞在が別々の行になる実装でも真** /
  design.md:158-159 は「（仮）: つないで 1 行にする粒度」と反転条件を付けているが、CLAUDE.md の（仮）は
  「後から答えを変えたら何が失われるか」が無いもの（B）を **design に置いてよい**という規範であって、**観測可能な振る舞いを specs から外してよい**という規範ではない
  （`openspec archive` は main specs しか更新しない）
- kind: technical
- 提案: 「隣り合う / 重なる『消した』の区間を 1 行にまとめる」を browsing-views の SHALL に置き、Scenario を 1 本（10:00–11:00 と 11:00–11:30 を別々に消した日 → 「消した」の行は 1 つで 10:00–11:30、識別子は 2 件）。（仮）の扱いは design.md:158-159 の反転条件をそのまま残せばよい —— 変えるときに spec も変える、が（仮）の意味。
- 処置: fixed specs/browsing-views/spec.md —— 「重なる / 隣り合う『消した』の時間を 1 つの行にまとめ、まとめた滞在の識別子をすべて持たせる」を SHALL にし、Scenario `隣り合う消した時間は 1 つの行にまとまる` を置いた。design D7 は反転条件を残したまま「振る舞いは specs にある」と書いた

## R6. 「作り直しと同時に走らせない」（C6 / 申し送り R44）に Scenario が 1 本も無い。同じ capability 群の正典には先例がある
- 分類: 検証不能（Requirement 本文だけが言っている）
- 成果物: openspec/changes/st22-record-deletion/specs/record-deletion/spec.md:201-213
- 根拠: spec.md:205「THE SYSTEM SHALL 印を書くまとまりを、**派生の作り直しと同時に走らせない**。」に対し、この Requirement の Scenario は
  `台帳に書けないと滞在の印も付かない`（spec.md:210-213）の 1 本だけで、**錠を取らない実装でも緑になる** /
  openspec/specs/derived-records/spec.md:172「THE SYSTEM SHALL 同じ利用者の作り直しを同時に 1 つしか走らせない。」には
  :193-195「#### Scenario: 同じ日の作り直しが同時に 2 回走っても滞在は二重にならない」が**ある**（ST16 の先例） /
  docs/handoff/ST22.md:31-37 R44 は ST22 宛ての申し送りの中でいちばん具体的（`pg_advisory_xact_lock(4816016, hashtext(<user_id>::text))`）で、
  tasks.md:61-64（2.4）も錠を書いているが、割り当てられた Scenario は台帳のものだけ /
  crates/server/src/stay_store.rs:15 `LOCK_KEY: i32 = 4_816_016` / :167（`rebuild_day` が同じ錠を取る）
- kind: technical
- 提案: derived-records の先例に合わせて Scenario を 1 本置く（例: 「同じ利用者の消す操作と作り直しを同時に始める → どちらも成功し、消した印は残り、作り直しは 1 回で終わる」）。tasks 2.4 の `CT erase_is_atomic` とは別の検証（`CT erase_locks_against_rebuild`）になる。錠は観測できない実装詳細ではない —— 「取り消しが起きない」「作り直しが無駄にならない」は結果として数えられる。
- 処置: fixed specs/record-deletion/spec.md —— Scenario `消す操作と作り直しが同時に走っても消したことは残る` を置き、tasks 2.6（`CT erase_locks_against_rebuild`）を足した（`derived-records` の先例と同じ形）

## R7. Q2 の本人の決定「**捨てない**」が、Scenario からは真偽を決められない。捨てる実装でも THEN が全部真になる
- 分類: 検証不能
- 成果物: openspec/changes/st22-record-deletion/specs/record-deletion/spec.md:186-189
- 根拠: Scenario `消した時間に後から届いた位置は受理されるが読み出しに出ない` の THEN は
  「取り込みは受理として返り、**その記録は保存され**、読み出しには出ない」の 3 主張の AND /
  いまのコード（crates/server/src/lib.rs:891-912 `Update::SkippedDeleted` / `was_skipped`）と正典
  openspec/specs/record-envelope/spec.md:404「THE SYSTEM SHALL 取り込まなかった記録の結果を**受理**として返し」により、
  **行をどこにも置かない実装でも「受理として返り」と「読み出しに出ない」は真**になる。真偽を分けるのは「保存され」だけなのに、
  それが何を見れば分かるか（素のテーブルに行が 1 件ある / 戻すと出てくる）が書かれていない /
  deep.md:41-51 Q2 の答えは「削除済みの印を付けて入れる（**捨てない**）」で、捨てる側が選択肢だった /
  観点「1 Scenario に主張が 1 つ」——AND で束ねると片方だけ通っても緑になる
- kind: technical
  （**レビューの分類を呼び出し元が直した** —— 本人の決定 Q2 を変える指摘ではなく、決定を Scenario が固定できていない技術的な欠陥のため）
- 提案: Scenario を割る。(1)「消した時間帯に後から届いた位置は**行として残る**」（THEN: 戻すとその位置が読み出しに出る、または削除済みを含めた件数が 1 増える）、(2)「後から届いた位置は読み出しに出ない」、(3) 受理の返り方は `tasks.md:79-80`（3.4）が既に別扱いにしているので Scenario から外してよい。「保存され」を残すなら、何を数えるかを THEN に書く。
- 処置: fixed specs/record-deletion/spec.md —— **kind を daily から technical に直した**（本人の決定 Q2 を変える指摘ではなく、決定を Scenario が固定できていない欠陥のため）。Scenario を 3 本に割り、「捨てない」を `戻すと読み出しに出る`（行として残る）で観測できる形にし、台帳の行も見る Scenario を足した

## R8. 製造準備 A-3「論理削除はビュー越しでしか読めない形にし、素のテーブルを直接引かせない」を design が扱っていない。deep が名指しで design に渡した論点
- 分類: 抜け（deep の決定が届いていない）
- 成果物: openspec/changes/st22-record-deletion/design.md（D7 / D11。A-3 への言及が無い）
- 根拠: deep.md:90「1 日の並びの読み出しが消した滞在の識別子と時刻の範囲を返す（**R8 の A-3 —— 素のテーブルを直接引かせない形を design で決める**）」 /
  docs/production-prep.md:73「| 削除・除外の効かせ方 | **論理削除はビュー越しでしか読めない形にし、素のテーブルを直接引かせない。** 後から入れると全クエリを書き直すことになる | FR-50 | 低 |」（**可逆性 低**。ST22 は FR-50 の Story） /
  design.md:149-150 は「いま `day_view` が…引いている**素の `core.event`** の行…を、そのまま行にする」と、素の読みを**そのまま API の応答の材料に広げる** /
  crates/server/src/stay_store.rs:937（`SELECT event_time, payload FROM core.event … deleted_at IS NOT NULL`）——
  ST22 の後は、ここに加えて erase / restore / late-marking / `stay_ids` の 4 経路が素のテーブルの削除済み行を読む /
  review/deep.md R8 の提案も「その読み出しを専用のビューに寄せるか（A-3 の形）を design に書く」だった
- kind: technical
- 提案: D 番号を 1 つ足して明示的に決める。(a) 削除済みの滞在を読む専用のビュー（例 `core.stay_erased`）を移行に足し、`day_view` と erase / restore がそれ越しに読む、(b) A-3 は「読み出し（記録を返す経路）」に限る解釈を採り、削除の操作そのものは素のテーブルを引いてよいと**書く**。どちらでもよいが、**書かないと決めたことにならない** —— A-3 は「後から入れると全クエリを書き直す」と明記された可逆性 低の項目。
- 処置: fixed D12 —— 削除済みの滞在を読む専用のビュー `core.stay_erased`（識別子と時刻の範囲だけ。緯度経度と `raw` を載せない）を移行に足し、読む側をそれに寄せた。書く側（印を付ける UPDATE）は A-3 の外だと**明示して**決めた。tasks 1.1 と 5.1 に足した

## R9. 導出元に FR-52 を引いている。ST22 の `satisfies` は FR-50 だけで、FR-52 は INDEX が ST23 に割り当てている
- 分類: 前倒しの手続き
- 成果物: openspec/changes/st22-record-deletion/specs/record-deletion/spec.md:239
- 根拠: spec.md:239「導出元: FR-50, **FR-52**, NFR-20（…）。深掘り Q4」 /
  docs/stories/ST22.md:4 `satisfies: [FR-50]` /
  docs/stories/INDEX.md:45「| ST23 | 本文を本当に消せる | 2 | **FR-51, FR-52** | ST22 |」・:107「| FR-51（本文の物理削除） | ST23 | **台帳と門だけ**。実際に消す操作は ST23 |」 /
  docs/requirements.md:483-484 FR-52「WHEN 利用者が**本文の物理削除**を指示する THE SYSTEM SHALL 実行前に対象件数を提示して確認を求める。」——
  ST22 の確認は**論理削除**のもので、FR-52 は要求していない（Q4 で本人が選んだから置く） /
  INDEX.md:194-204 の ST22 の訂正は `browsing-views` の前倒ししか書いておらず、FR-52 の前倒しには触れていない（ST16 / ST19 の訂正は要件の前倒しを表で明示している。:124-128 / :158-164）
- kind: conflict
- 提案: 導出元から FR-52 を外し、「FR-50 / 深掘り Q4（本人が proto で選んだ）」にする。FR-52 の水準（件数を出して確認）を論理削除にも前倒しで当てたつもりなら、INDEX.md に ST16 / ST19 と同じ形の表で「FR-52 のうち『件数を出して確認する』を ST22 が論理削除について満たす。本体は ST23」と書く。**いまは ST23 の上流が「FR-52 はもう半分できている」と読む余地がある。**
- 処置: fixed D13 仮 —— 導出元から FR-52 を外し、「ST22 の確認は FR-52 の前倒しではない（対象は戻せる削除。Q4 による）」を design D13（仮）に置いた。反転条件は「ST23 が FR-52 を実装するとき文面と形を揃える」。`docs/stories/INDEX.md` の ST22 の訂正にも書いた

## R10. 44 px の導出元にまた NFR-20 を引いている。deep の独立レビュー R13 が「出所が違う」として直した前提が戻っている
- 分類: premise
- 成果物: openspec/changes/st22-record-deletion/specs/record-deletion/spec.md:234 / :239
- 根拠: docs/requirements.md:815-820 NFR-20「**取り返しの付かない操作**（FR-51 の本文の物理削除、FR-53 の収集の停止）の対象は 44 × 44 CSS px 以上とする。…全画面に AAA を課さない理由は…全行に 44 px を課すと 1 画面に収まらなくなるため」 ——
  ST22 の削除は**取り返しが付く**（同じ change が `POST /stays/restore` と「消した」行からの戻しを作る。spec.md:266-277） /
  openspec/changes/st22-record-deletion/review/deep.md:142-148 R13（kind: premise）「NFR-20 は論理削除を含まず、行ごとに 44 px を置くことを理由付きで避けている」→ 処置 fixed（出所を ui-direction に直した） /
  docs/ui-direction.md:118「削除・停止だけ 44×44px 以上（NFR-20）」・:159「削除操作 44×44」（ui-direction 自身も NFR-20 と書いているが、確定値としてはこちらが出所）
- kind: premise
- 提案: 導出元を「NFR-19（24 px の下限）＋ `docs/ui-direction.md`:118 の確定値（削除・停止だけ 44×44 px）」に直し、理由の行に「ST22 の削除は戻せるが、誤操作の代償が非対称なので厳しい側を採る（Q4 の proto で本人が見て選んだ形）」と書く。44 px そのものは動かさなくてよい —— 直すのは**出所**。
- 処置: escalated —— deep.md の「specs の独立レビューが人間へ返したもの」に spec R10 として記録し、導出元を NFR-19 ＋ `docs/ui-direction.md` の確定値に直した。**44 px そのものは動かさない**（本人が proto の実寸を見て選んだ値で、決定は変わっていない）

## R11. 1 つの画面の要件が 2 つの capability に割れている。ST25 が `browsing-views` を MODIFIED するとき、詳細の中の消す操作が視界に入らない
- 分類: 置き場
- 成果物: openspec/changes/st22-record-deletion/specs/record-deletion/spec.md:232-277
- 根拠: 同じ 1 行の中の話が 2 つに分かれている ——
  「行を選ぶと**その場で詳細が開く**・詳細に件数が出る・24 px・キーボード」は browsing-views の ADDED（spec.md:109-118）、
  「その**詳細の末尾**に 44 px の消す操作・確認・『消した』行の戻す」は record-deletion の ADDED（spec.md:232-277） /
  docs/stories/INDEX.md:78「| `browsing-views` | 閲覧と検索 | **ST16**, **ST22**, ST25, ST26, ST36 |」・
  `python3 scripts/board.py` →「[衝突待ち] ST25 1 日を時刻順に見る browsing-views … `browsing-views` を ST22 が触っている」——
  ST22 の archive 後に ST25 が動き、ST25 は `browsing-views` の spec しか MODIFIED しない /
  正典 openspec/specs/browsing-views/spec.md:153-157「一覧は UI の下限を満たす」（指で触れる対象 24 px・記録なしの行を色だけで区別しない）も
  ST22 では MODIFIED されておらず、新しい「消した」の行と 44 px の操作はこの Requirement の外にいる
- kind: technical
- 提案: 画面の Requirement 2 本（`1 日の一覧の詳細から滞在を消せる` / `「消した」の行から戻せる`）を browsing-views 側へ移すか、移さないなら browsing-views の該当 Requirement の理由の行に「この一覧には `record-deletion` にも要件がある（消す操作・戻す操作）」と相互参照を 1 行入れる。あわせて「一覧は UI の下限を満たす」を MODIFIED して「『消した』の行を色だけで他の行と区別しない」を足すかどうかを決める（いまは D9 の「記録なしと同じ濃さ＋文字『消した』」だけが根拠）。
- 処置: fixed specs/browsing-views/spec.md —— 「一覧は UI の下限を満たす」を MODIFIED に足し（「消した」の行を色だけで区別しない / 消す操作だけ 44×44 px）、一覧の Requirement に `record-deletion` への相互参照を 1 行入れた。Scenario `消した行は文字で区別される` を置き、tasks 8.4 で固定する

## R12. tasks の検証に 2 つの穴 —— 新しい台帳の錠と down 移行が `tools/check-immutable.sh` の台本に入らない / 10.3 の「まとめて緑」に e2e が無い
- 分類: 検証の抜け
- 成果物: openspec/changes/st22-record-deletion/tasks.md:38-43（1.1 / 1.2）・:154-155（10.3）
- 根拠: tools/check-immutable.sh は**手で維持している台本**で、台帳ができるたびに足されてきた ——
  :360-369（ST03 の `erasure_ledger` の UPDATE / DELETE / TRUNCATE を psql から直に殴る）、:747-784（ST19 の主張）、
  :911-955（**戻し手順を逆順に当てる**。ST19 の 1 本・ST04 の 1 本・ST16 の 1 本・ST03 の 5 本を名前で列挙し、:956 で「OK ST19 の 1 本・ST04 の 1 本・ST16 の 1 本・ST03 の 5 本とも当たる」と印字）。
  ST22 の `core.deletion_ledger` と `YYYYMMDDHHMM_deletion_ledger.down.sql` を足す task がどこにも無いので、**新しい down は 1 度も当たらないまま緑**になる
  （design.md:65-66 自身が「ST03 / ST16 の関数を使い回すと、それを落とす down 移行が依存で当たらなくなる（ST16 が `tools/check-immutable.sh` で実測）」と、この工具を根拠に設計している） /
  :919 `for m in "${MIGS[@]}"` の `MIGS` は :19 で lib.rs から機械的に引くので**前進側は自動**。手当てが要るのは戻し側と錠の台本だけ /
  tasks.md:154-155（10.3）は `cargo test --workspace` / `npm run test` / `check-immutable.sh` / `smoke.sh` を並べるが **`npm run test:e2e` が無い**。
  9 章の 2 本（44 px の実寸・消して戻す）は .github/workflows/ci.yml:148-164 の `e2e` job が拾うので merge までには落ちるが、**手元の「まとめて緑」では落ちない**
- kind: technical
- 提案: 1 章に task を 1 つ足す ——「`tools/check-immutable.sh` に (a) `core.deletion_ledger` への UPDATE / DELETE / TRUNCATE が psql から拒まれること、(b) `…_deletion_ledger.down.sql` を逆順の先頭で当てて戻し、当て直せること、を足す。検証: `tools/check-immutable.sh` rc=0 かつ出力に `deletion_ledger` の行があること」。10.3 の並びに `cd web && npm run test:e2e` を足す。
- 処置: fixed 1.3 —— `tools/check-immutable.sh` に (a) 新しい台帳の錠 (b) down 移行の当て直し を足す task を置き（出力に `deletion_ledger` があることまで検証に含めた）、10.3 の「まとめて緑」に `npm run test:e2e` を足した

## R13. D3 が滞在の期間を `payload.start` から読むと書いているが、既存の唯一の読み手 `span_of` は `event_time` を使い、壊れた payload を許容している
- 分類: 前提（コードとの突き合わせ）
- 成果物: openspec/changes/st22-record-deletion/design.md:93-96（D3）
- 根拠: design.md:95「消す滞在の **`payload.start`** 〜 `payload.end`（端を含む）に `event_time` が入り…」 /
  crates/server/src/stay_store.rs:291-305 `span_of(event_time, payload)` ——「**取り込みの口から入った滞在は形が崩れていることがある**ので、読めない終わりは始まりと同じと読む」。
  始まりは常に `event_time`、終わりだけ `payload->>'end'` を読み、読めなければ `event_time` に落とす。:336 / :907 / :951 の 3 か所すべてがこの関数を通す /
  stay_store.rs:705 で派生の滞在は `event_time = f.start` として挿入されるので**正常な行では一致する**が、
  spec.md:14（「この口で滞在（派生させた滞在の記録）以外の記録を受け付けず」）は `logical_source` で絞るだけなので、
  **取り込みの口から入った `s01-stay`**（`origin` が `derived` のもの）も識別子で指せる。`payload.start` が無い / 壊れている行で範囲が決まらない
- kind: technical
- 提案: D3 の表記を `span_of` に合わせる（「滞在の行の `event_time` 〜 `span_of` が読む終わり」）。合わせないなら、`payload.start` が読めないときに何をするか（404 で断る / `event_time` に落とす）を D3 に書く。いま tasks.md:55（2.2）も「消す滞在の `start`〜`end`」としか書いていないので、実装者が素直に読むと 2 通りに分かれる。
- 処置: fixed D3 —— 期間の読み方を `stay_store::span_of`（始まりは `event_time`、終わりは `payload->>'end'`。読めなければ始まりと同じ）に合わせ、`payload.start` を読まない理由を書いた

## R14. proposal が「止めた理由（proposal で止める）」のまま残っていて、並走の記述も古い。読む人が現在地を誤る
- 分類: 内部の食い違い
- 成果物: openspec/changes/st22-record-deletion/proposal.md:56 / :65-68 / :78-82
- 根拠: proposal.md:65-68「## 止めた理由（proposal で止める） **ST16 は merge 済みだが archive 待ち。**…ST16 の archive 後に specs → design → tasks → PR と issue へ進む」——
  しかし `ls openspec/changes/archive/` に `2026-09-15-st16-stay-derivation` があり、`openspec/specs/browsing-views/spec.md` は存在し、specs / design / tasks も**既に書かれている** /
  proposal.md:78「**ST16**（archive 待ち）」/ :80「**ST04**（下流）」（`openspec/changes/archive/2026-09-17-st04-offline-retention` で archive 済み）/
  :82「**ST12**（上流）」（`scripts/board.py` では「[下流] ST12 … #51 merged」） /
  proposal.md:56「specs はそこで止める（下の『止めた理由』）」も同じ
- kind: technical
- 提案: 「止めた理由」の節を「止めていた理由と、いつ解けたか（ST16 の archive: 2026-09-15）」に書き換えるか、Impact の並走の表に日付を入れて現況へ直す。**内容の誤りではなく時点の誤り**だが、この節は「なぜ specs が無いか」を説明する節なので、specs がある状態で残ると後から読んだ人が別の change を探す。
- 処置: fixed proposal.md —— 「止めた理由」を「止めていた理由と、解けた時点（ST16 の archive 2026-09-15）」に書き換え、Impact の並走（ST04 archive 済み / ST12 下流 / ST33 の申し送り / ST23 への引き渡し）を現況に直した

## R15. Scenario の無い SHALL が 3 つ残っている（戻す口の 404 / 台帳を読み出しに出さない / 後から届いた位置の台帳の行）
- 分類: 検証不能（小）
- 成果物: openspec/changes/st22-record-deletion/specs/record-deletion/spec.md:142 / :99 / :178
- 根拠: (a) spec.md:142「THE SYSTEM SHALL 資格情報の無い求めを 401 で、**存在しない識別子を 404 で**断る。」——
  401 には Scenario（:171-174）があるが 404 には無い。消す口の側には両方ある（:37-40 / :47-50）ので、**戻す口だけ非対称** /
  (b) spec.md:99「THE SYSTEM SHALL 台帳の行を、その記録の読み出しには出さない（台帳は記録ではない）。」—— Scenario 無し。
  docs/requirements.md:461-462 FR-50「**履歴に残した前の版と、消去の台帳は「記録」ではない**ので、それ自身の削除の印を持たず、親の記録の削除に従う」に対応する SHALL なので、要件に根がある /
  (c) spec.md:178「…削除済みの印と削除時刻を付け、**台帳に「消す」の行を積む**。」—— 後から届いた位置の台帳の行を見る Scenario は無い
  （:191-194 が戻ることは見るが、台帳経由で戻るのか範囲の再計算で戻るのかを分けない。design.md:143 は「戻すときは**台帳の原因で引く**」と決めている）
- kind: technical
- 提案: (a) は消す口と対称に 1 本足す。(b) は `GET /events` に台帳の行が出ないことを 1 本（または SHALL を落とす —— 別の表なので自明なら書かない方が正直）。(c) は「後から届いて印が付いた位置が台帳に『消す』の行を持ち、原因が消した滞在である」を足すと、tasks 4.2（`CT restore_includes_late_arrivals`）が**台帳経由**であることまで固定できる。
- 処置: fixed specs/record-deletion/spec.md —— (a) `知らない識別子を戻そうとすると断られる`、(b) `台帳の行は記録の読み出しに出ない`、(c) `後から届いて印が付いた位置は台帳に消した行を持つ` の 3 本を足し、tasks 1.2 / 3.3 / 4.1 に割り当てた

## R16. spec の Scenario に実装の literal（`rebuild:`）が入っている。同じことを言う正典は「削除した者の欄」という言い方を採っている
- 分類: 置き場（specs に実装の名前）
- 成果物: openspec/changes/st22-record-deletion/specs/record-deletion/spec.md:13 / :30
- 根拠: spec.md:13「…派生の作り直しが付ける印（**`rebuild:` で始まる値**）と区別できる値で付ける」・
  :30「**THEN** 消した滞在は戻らず、その滞在の行の**削除した者の欄は `rebuild:` で始まっていない**」 /
  正典 openspec/specs/derived-records/spec.md:269-270 は同じ規則を
  「THE SYSTEM SHALL 削除済みの印を持つ滞在のうち、**作り直しが付けた印（吸収・消した時間帯に重なる）でないもの**を、本人が消した滞在として扱う。**削除した者の欄が空の削除**も本人が消したものとして扱う。」と、実装の literal 無しで書いている /
  `grep -rn "rebuild:\|deleted_by\|deleted_at" openspec/specs/` の結果は 0 件（正典は列名も接頭辞も持っていない）
- kind: technical
- 提案: :30 の THEN を「作り直しの後も、その滞在は**本人が消した**ものとして扱われている（作り直しの付ける印に変わっていない）」のように振る舞いで書き、`rebuild:` という値そのものは design D4 と tasks の規律（どちらも既に持っている。design.md:103-113 / tasks.md:29-30）に残す。申し送り R14 の拘束力は design と tasks で足りる。
- 処置: fixed specs/record-deletion/spec.md —— `rebuild:` の literal を spec から外し、「作り直しが付ける印と区別できる形」「作り直しの後も本人が消したものとして扱われている」に直した（値そのものは design D4 と tasks の規律に残る）

## R17. 「稼働状況は消した記録も数える」が `record-deletion` にだけ置かれる。`collection-coverage` を次に触る Story（ST14 / ST15）からは見えない
- 分類: 置き場（小。理由は proposal にある）
- 成果物: openspec/changes/st22-record-deletion/specs/record-deletion/spec.md:215-230
- 根拠: `grep -n "削除\|消し" openspec/specs/collection-coverage/spec.md` → **0 件**。ST22 の後も、稼働状況の正典は削除について何も言わない /
  proposal.md:81 / design.md:196 は「ST04 / ST12 が並走しているので `collection-coverage` は触らない」と**理由を明示している**（判断としては妥当） /
  `python3 scripts/board.py` →「[衝突待ち] ST14 収集が途切れたら気づける collection-coverage」「[衝突待ち] ST15 …」——
  次にこの capability を触るのは ST14 / ST15 で、どちらも Q3（本人の決定）を知らないまま稼働状況の数え方を書き換えうる /
  tasks.md:118-119（7.1）が Rust のテストで固定するので**実装は落ちる**が、**spec の側は矛盾したまま緑**になる
- kind: technical
- 提案: ST22 の Requirement の理由の行に「この規則は `collection-coverage` の正典には書かない（ST04 / ST12 の並走を避けた）。稼働状況の数え方を変える Story はここを読むこと」と書き、`docs/stories/INDEX.md` の ST22 の訂正に 1 行足す（ST16 / ST19 の訂正と同じ形）。並走が解けた後に `collection-coverage` へ移す前提なら、それも書く。

---

## 観点ごとの結果

- **観点 1（deep の決定が正典に写っているか）**: Q1 → R2（担保の半分が ST33 に預けられている）/ Q2 → R7（「捨てない」が検証できない）/ Q4 → R3・R4・R5・R8（「specs で決める」と名指しされた 3 件のうち、移動の境目と断片と A-3 のビューが未処理）。
  Q3 と C1 / C3 / C4 / C7 / C8 / C9 / C10 は Scenario に写っている。C2（台帳）は Requirement と Scenario 4 本まで写っている。C6 のみ R6。
  数値（44 px / 件数 12 / 台帳 4 行）は Scenario に入っており、実装が変えれば落ちる。
- **観点 2（Scenario が検証可能か）**: R4 / R6 / R7 / R15。「正しく」「適切に」「そのまま」の類の語は 52 本のどれにも無かった。
- **観点 3（置き場）**: R3 / R5（design に観測可能な振る舞い）/ R11 / R16 / R17。`proposal.md` の Capabilities（`record-deletion` 新設・`browsing-views` MODIFIED）と `specs/` のディレクトリと `INDEX.md`:76 / :78 は一致した。
- **観点 4（tasks が検証を持つか）**: 32 タスク全部に `検証:` があり、`CT` の定義（`cargo test` が 0 本でも rc=0 になる穴を塞ぐ）も規律として置かれている。依存順（移行 → 消す口 → 作り直し → 戻す口 → 並び → 画面 → e2e）は逆転していない。「人間の確認待ち」を 0 件にし、画面の Scenario を e2e（本物のブラウザ）に置いているのは 2026-09-18 の決定どおりで、逃がしは無い。指摘は R12 の 2 件のみ。
- **観点 5（Story と要件の一致）**: `check_chain.py` の再生成との一致は rc=0。`docs/stories/ST22.md` の価値・完了の判定（「画面から消え」「次の同期で同じ記録が復活しない」）は Q1〜Q4 と矛盾しない。`satisfies` の外の要件を満たそうとしている箇所は R9（FR-52）と R1（FR-44 の条項）。
  FR-50 の ★ 2026-09-15 の 5 つの文（位置も消す / 後から届いた位置 / 戻せる / 追記のみの台帳 / 「消した」と出す / 稼働状況は数える）は、いずれも対応する Requirement を持っている。
- **並走（追加で見た範囲）**: `record-envelope` は ST05 の上流がまだ specs を持たず（`../ashiato2-up-st05/.../st05-clock-skew/` は deep と proposal のみ）、ST22 も触らないので衝突なし。
  `collection-coverage`（ST12 下流）と `desktop-collection`（ST08）と `device-collection`（ST06）への MODIFIED も無い。残る論点は R17 と R11（ST25）。
- 処置: fixed specs/record-deletion/spec.md —— Requirement の理由の行に「この規則は `collection-coverage` の正典には書かない。稼働状況の数え方を変える Story はここを読むこと」を書き、`docs/stories/INDEX.md` の ST22 の訂正に同じ 1 行を足した（ST14 / ST15 が読む）

