# 整合の確認（2026-10-23・ST23 の深掘り第 1 回を FR-50 / FR-51 / FR-52 へ戻した改訂）

- 基準: `e1140839d7b9a9ddb80baa27c1b8822fa3ef01a3`..`ae77268d8e6ca1eaaa0e7cf86814c048c530fe70`

対象は ST23（`openspec/changes/st23-record-erasure/`）の深掘り第 1 回の答え（Q1〜Q6 すべて推奨。`deep-answers-1.txt`）を要件へ戻した改訂。
FR-50 の ★ 2026-10-23（訂正と補足の 2 つ）・FR-51 の ★ 2026-10-23・FR-52 の ★ 2026-10-23、`docs/stories/INDEX.md` 末尾の訂正（2026-10-23）、
再生成した `docs/stories/ST23.md` を見た。文書は head（`ae77268d`）の内容で読んだ。

機械の入力:
- `req_impact.py --base e1140839`: 改訂 3 件（FR-50 / FR-51 / FR-52）・扉 0 項。名指しは `docs/handoff/ST12.md` `ST22.md` `ST23.md` `ST33.md` `ST45.md` `ST47.md`・
  `docs/openapi.json`・`docs/production-prep.md`・`docs/stories/INDEX.md`・`docs/stories/stories.json`・`docs/ui-direction.md`・正典 `openspec/specs/` の 8 capability・
  走っている `st09-media-capture` と `st23-record-erasure` の文書
- `check_chain.py`: 要件 142 / Story 47 / 拾われた要件 140/142・未回収 0・warn 0。FAIL 1 件は「整合の確認の後に要件が変わった: FR-50, FR-51, FR-52」（この監査が受けるもの）

走っている change は `st09-media-capture`・`st13-scheduled-account-fetch`・`st23-record-erasure`（この改訂の出どころ）。
st09 と st23 の食い違いは**印を付けるだけ**にした（C4・C8）。失われるもの（uncaptured / discarded / exported / rewrite-all）を伴う印は無い。st13 はこの改訂の影響を受けない（末尾）。

## C1. FR-50 / FR-52 の ★ は「書庫から入った後着も入れた同じトランザクションで本文を消す」と書くが、本人が選んだ Q3 の選択肢と合意した spec は書庫の分を「格納の後の別のまとまり」で消す
- 重大度: 高
- 文書: docs/requirements.md:693
- 要件: FR-50, FR-52
- 根拠: FR-50 ★（:693-694）「本文ごと消した（FR-51）滞在の時間帯に後から届いた位置の記録（端末の位置・**書庫の位置・位置を持つ書庫の項目**）は、削除済みとして入れるだけでなく、**入れた同じトランザクションで本文も消し**」。
  FR-52 ★（:712）「後から届いた分は確認をはさまず、**入れた同じトランザクションで本文を消す**」。
  一方、本人が選んだ Q3 の選択肢の説明（`openspec/changes/st23-record-erasure/deep-questions.json` の Q3 options[0].detail）「**書庫から入るものは格納の後の別のまとまりで消す**（いまの印付けと同じ。落ちたら次の見回りで消し直す。**その間は本文が DB にある**）」、
  合意した spec（`openspec/changes/st23-record-erasure/specs/record-deletion/spec.md:347-348`）「取り込みの口から届いた記録は、保存と同じまとまりで本文を消す。」「書庫から入った記録は、格納の後のまとまりで本文を消し、消せなかったら次の見回りで消し直す。」、
  design D9（仮。`design.md:183-188`）・proposal（`proposal.md:34`）も同じ。要件は「本文が DB に一度も残らない」守りを書庫の分にも言っているが、選ばれた形と spec にはその守りが無い
  （spec の Scenario「端末から届いた位置は本文を持ったまま確定しない」は端末の分だけ）
- 直すところ: requirements（FR-50 ★ と FR-52 ★ の「入れた同じトランザクション」を端末の分に限り、書庫の分は「格納の後のまとまりで消し、消せなければ次の見回りで消し直す（その間は本文が DB にある）」と書く）
- 機械: できない（散文どうしの食い違い。`review_triage.py` は ★ に FR があるかしか見ない）
- 行き先: requirements（FR-50 ★ と FR-52 ★ に、同じトランザクションは端末の分だけ・書庫の分は格納の後で消し次の見回りで消し直す、を補足した）

## C2. `docs/handoff/ST23.md` の st19 Q1 / st22 R1 / st21 C13 は表題と本文で「ST23 が足す」と言ったまま。宛先の「新しい Story」には番号も申し送りの置き場も無い
- 重大度: 高
- 文書: docs/handoff/ST23.md:42
- 要件: FR-50, FR-51, FR-44
- 根拠: `docs/handoff/ST23.md:42`「## st22-record-deletion R1 — 個人属性の主張を消す操作は **ST23 が S-6 に足す**」、:74「## st21-place-registry C13 — 場所を消す口は **ST23 が S-6 の場所のカードに足す**」、
  :51-53「ST23 は同じ主張について**本文の消去**（FR-51…）も作る。…主張の消去は「先に消したことにしてから消去する」のか「直接消去してよいのか」を **ST23 の深掘りで本人に問う**」、:85「**ST23 まで重複は S-6 に残る**」。
  改訂後: FR-50 ★（:696-697）「主張・場所の記録・主観の記録を消す口…は、まだどの Story にも無い」「新しい Story として /requirements → /stories から足す」、INDEX:418「`docs/handoff/ST23.md` の st19 Q1 / st22 R1 / st21 C13 の項は、その新しい Story へ持ち越す」。
  ところが handoff の本文は直っておらず、持ち越す先のファイルも無い（`docs/handoff/` に番号の無い Story の置き場は無い）。`docs/handoff/README.md` の規則 5 は「宛先の Story が読む経路が無かった（名前はあるのに受け皿が無い工程は消える）」と書く ——
  新しい Story ができても `story_brief.py ST<新>` はこの 3 項を出さず、st22 R1 の「先に消したことにしてから消去するか」の問いが落ちる。
  逆に ST23 の下流は `tasks.md:4` で「`docs/handoff/ST23.md`（開始時と PR 前の 2 回）」を読むので、表題どおり S-6 の口を作りに行く誤読の入口が残る（`tasks.md` に 3 項を対象外とする行は無い）
- 直すところ: その文書（3 項に「宛先は ST23 ではなく新しい Story（FR-50 ★ 2026-10-23）」の印を付ける）＋ 新しい Story（/stories で番号を起こし、3 項を `docs/handoff/ST<新>.md` へ移す）
- 機械: 安く捕まえられる（`check_chain.py` か `story_brief.py` が、handoff の節の表題「ST<NN> が…」と INDEX の訂正で宛先を外した節を突き合わせる。いまは捕まらない）
- 行き先: docs（handoff ST23 の 3 項に宛先の変更の印を付けた。新しい Story の番号と handoff の移しは /requirements → /stories で起こすときに行う —— 本人の Q1 の答え）

## C3. 「主張・場所・主観を消す口はまだどの Story にも無い」は鎖に現れない。ST23 が FR-51 を丸ごと satisfies しているので、ST23 が終わると FR-51 は満たされたことになる
- 重大度: 中
- 文書: docs/requirements.md:696
- 要件: FR-50, FR-51
- 根拠: FR-50 ★（:696）「**主張（FR-44）・場所の記録・主観の記録を消す口（この FR-50 と FR-51）は、まだどの Story にも無い。**」。
  `docs/stories/stories.json:532-537` の ST23 は `"satisfies": ["FR-51", "FR-52"]` のまま（deep.md:48「Story の表題・価値・完了の判定は変わらない（`stories.json` は直さない）」）。
  `check_chain.py` は「どれかの Story に拾われた要件: 140/142」「未回収 0 件」で、FR-51 の主張・場所・主観の部分が拾われていないことを出さない。
  同じ形の先例: FR-50 ★ 2026-10-17（:687-689）「写真の記録を 1 件消す口と、写真を見る画面は、まだどの Story にも無い」「新しい Story として /requirements → /stories から足す」は、6 日後の今も `stories.json` に Story が無い
- 直すところ: 新しい Story（/stories で主張・場所・主観を消す Story を起こす。写真を消す Story と合わせて扱うかは /stories で）
- 機械: 安く捕まえられる（要件の「まだどの Story にも無い」に機械が読める印を置き、`check_chain.py` が未回収として数える）
- 行き先: leave: 主張・場所・主観を消す新しい Story は /requirements → /stories で起こす（本人の Q1 の答え。写真を消す Story と同じ扱い）。FR-50 ★ 2026-10-23 に「まだどの Story にも無い」を書いた。record の段は Story を起こさない

## C4. 走っている st09 の design は「本文の消去を D-02 に及ぼすのは ST23」と書くが、ST23 は写真の 4 点を「写真の新しい Story」へ送り、その置き場が無い（印のみ）
- 重大度: 中
- 文書: openspec/changes/st09-media-capture/design.md:30
- 要件: FR-51, FR-50
- 根拠: st09 `design.md:30`「**本文の消去（FR-51）を D-02 に及ぼす** のは ST23（deep.md の R8）」、st09 `deep.md:263`「**ST23（R8）** —— 本文の消去（FR-51）を D-02 の原本へどう及ぼすか…」。
  st23 `defaults.md:16`（C8）「4 点は、写真を見て 1 件消す新しい Story（FR-50 ★ 2026-10-17）へ送る」、`proposal.md:108`「写真の新しい Story へ（ST09 review R8 の 4 点。C8）」。
  ところが st23 `tasks.md` Task 10.1 が書く申し送りは `docs/handoff/ST30.md` の C9 だけで、R8 の 4 点を置く行は無い（写真の Story は番号も無い）。Q1 の答え（ST23 は滞在とその連鎖だけ）でこの宛先は確定した。
  走っている change なので差し戻さない。写真の消去の口がまだ無いので、いま失われるものは無い
- 直すところ: 新しい Story（写真を消す Story を起こすときに R8 の 4 点を受ける）。st09 の design は印のみ
- 機械: できない（宛先の無い申し送りを置く場所が無い）
- 行き先: leave: 写真を消す新しい Story（FR-50 ★ 2026-10-17）を起こすときに ST09 R8 の 4 点を受ける。走っている st09 は印のみ

## C5. INDEX の ST03 の表は「実際に消す操作は ST23」のまま。2026-10-23 の訂正が挙げる行に入っていない
- 重大度: 低
- 文書: docs/stories/INDEX.md:137
- 要件: FR-51
- 根拠: INDEX:137「| FR-51（本文の物理削除） | ST23 | **台帳と門だけ**。実際に消す操作は ST23 |」。訂正（:415-416）が宛先を変えた行として挙げるのは「ST19 の FR-50 / FR-51、ST21 の FR-50 / FR-51、ST17 の FR-51、ST22 の訂正の個人属性の主張の条項」だけ。
  ST03 の門は収集した記録全体に掛かるので、改訂後は「消す操作は ST23（滞在とその連鎖）・それ以外は新しい Story」と読む必要がある
- 直すところ: その文書（訂正の節に ST03 の行も含める）
- 機械: できない
- 行き先: docs（INDEX の 2026-10-23 の訂正に ST03 の表の行を含めた）

## C6. `docs/ui-direction.md` は ST23 を S-5（記録の詳細）に置くが、Q5 で決まった ST23 の口は S-2 の 1 日の一覧の「消した」行。主張・場所を消す口が載る S-6 の行は FR-50 / FR-51 を持たない
- 重大度: 低
- 文書: docs/ui-direction.md:21
- 要件: FR-50, FR-51, FR-52
- 根拠: ui-direction:21「| S-5 記録の詳細 | 1 件を見る・消す・… | ST22, ST23, ST24 | FR-50, FR-51, FR-52, …」、:18「| S-2 主表現（1 日を見る） | … | ST25, ST16, ST17 |」、:22「| S-6 マスタ管理 | … | ST19, ST20, ST21 | FR-44〜FR-49 |」。
  st23 `deep.md:35`（Q5 の推奨の形）「1 日の一覧の「消した 10:05 – 11:48」の行に「戻す」と並べて「本文ごと消す」（44×44 px 以上）を置く」。handoff ST23:45・:77 は主張・場所を消す口を S-6 に置く
- 直すところ: その文書（ST23 の口を S-2 に、S-6 の触れる要件に FR-50 / FR-51 を足すのは新しい Story を起こすとき）
- 機械: 安く捕まえられる（`story_brief.py --screens` の表と、change の spec の画面の Requirement が触る画面の突き合わせ）
- 行き先: docs（ui-direction の S-2 に ST23 と FR-51 / FR-52 を足した。S-6 の要件は新しい Story を起こすときに足す）

## C7. FR-51 ★ は確認に「書庫の写し N 個にも**入っている**」と出すと書くが、合意した spec は記録の入っていない写しも数える
- 重大度: 低
- 文書: docs/requirements.md:707
- 要件: FR-51, FR-52
- 根拠: FR-51 ★（:707）「確認（FR-52）に「書庫の写し N 個にも入っている」と出す」。st23 spec（`specs/record-deletion/spec.md:66`）「範囲の記録が入っている**可能性のある**、残している書庫の写しの数」、
  Scenario「記録の入っていない写しも同じソースなら数える」（:90-93）「応答の残る書庫の写しの数は 2 である（1 と数えない）」。design の文面（`design.md:251`）は「消えずに残るもの: 書庫の写し N 個」
- 直すところ: requirements（「入っているかもしれない写し N 個」に）
- 機械: できない
- 行き先: requirements（FR-51 ★ を「入っているかもしれない」に直し、数え方を補足した）

## C8. 走っている st23 の proposal・design・spec に「答えはまだ無い」「推奨が採られれば」の文が残る（印のみ）
- 重大度: 低
- 文書: openspec/changes/st23-record-erasure/proposal.md:16
- 要件: FR-50, FR-51, FR-52
- 根拠: `proposal.md:16`「**深掘りの第 1 回（6 問）の答えはまだ無い。推奨の答えで先に書いた**」、:41「**推奨が採られたときに要件へ戻すもの**（★ は record の段が足す。この段では要件を書き換えない）」、:62「推奨を採ったことにはしていない」、
  `design.md:3`「**深掘り第 1 回の答えはまだ無く、推奨の答えで書いた**」、`specs/record-deletion/spec.md:353`「推奨が採られれば record の段で FR-50 ★ と FR-52 の読み方に ★ を足す」。
  `deep.md:3-5` は答え（2026-10-23）と合意を記録済みで、要件の ★ も入った。走っている change なので差し戻さない。失われるものは無い
- 直すところ: leave（走っている change。st23 の下流が読むときの誤読だけ）
- 機械: できない
- 行き先: leave: 走っている st23 の合意済みの仕様の文。record の段は仕様を書き換えない（合意の digest が変わる）。意味は deep.md の答えと一致している

## C9. FR-50 ★ は「各 Story が置いた錠と台帳の門はそのまま使える」と書くが、ST23 が足す 1 操作 1 行の台帳は滞在の形で、主張・場所・主観の消去の「件数・範囲」は載らない
- 重大度: 低
- 文書: docs/requirements.md:697
- 要件: FR-51
- 根拠: FR-50 ★（:697）「各 Story が置いた錠と台帳の門（ST19 / ST21 / ST17）はそのまま使える」。FR-51（:699-700）は「削除した事実・時刻・件数・範囲を**台帳に**残す」。
  st23 `design.md:108`「`core.erasure_operation` | …`kind text CHECK IN ('user','late')`…`stay_id uuid`（指定した滞在）・`start_at`・`end_at`・`stays`・`locations`・`archive_locations`・`archive_items`・`versions`」——
  記録ごとの門（`core.erasure_ledger`。`design.md:110`「ST19 / ST21 / ST17 の門は `operation_id` を見ない」）はそのまま使えるが、FR-51 の「件数・範囲」を残す操作の台帳は新しい Story が別に要る
- 直すところ: leave（新しい Story の上流が決める。要件の文は記録ごとの門については正しい）
- 機械: できない
- 行き先: requirements（FR-50 ★ の台帳の文を記録ごとの門に狭め、1 操作の台帳の載せ方は新しい Story が決めると書いた）

## K1. FR-51 ★ の「写しを丸ごと消すのは、本人が選ぶ別の操作」—— その操作はコードにも正典にも無く、正典は既にある写しを消さないと決めている
- 重大度: 中
- 文書: docs/requirements.md:707
- 要件: FR-51, FR-14
- 根拠: FR-51 ★（:707）「写しを丸ごと消すのは、本人が選ぶ別の操作。」、st23 `proposal.md:44`「写しの丸ごとの削除は本人が設定で選ぶ別の操作」。
  正典 `openspec/specs/external-ingestion/spec.md:503`「WHEN 設定が「残さない」である THE SYSTEM SHALL 写しを作らない。**既に作った写しは消さない。**」。
  コード: 写しの設定は環境変数（`crates/server/src/archive/config.rs:20` `ASHIATO_ARCHIVE_KEEP_COPIES`）で画面の設定ではなく、写しを消す経路は確認待ちのために作った写しの後始末（`crates/server/src/archive/worker.rs:1813-1830`）だけ。
  router に写しを消す口は無い（`crates/server/src/lib.rs` の書庫の route は `/archives/status`:3429 だけ）。本文ごと消した場面の座標は写しに残り続け、本人が消す手段は ashiato の中に無い（置き場のファイルを手で消す以外）
- 直すところ: requirements（「別の操作」がまだどの Story にも無いことを書くか、新しい Story にする）
- 機械: できない
- 行き先: requirements（FR-51 ★ に、写しを丸ごと消す操作はまだどの Story にも無いと補足した）

## K2. `docs/handoff/ST23.md` の st12 R23 は「写しを消すコードが無い」と言うが、コードは ST12 の R56 で直っている
- 重大度: 低
- 文書: docs/handoff/ST23.md:90
- 要件: FR-14, FR-51
- 根拠: handoff ST23:90-92「`crates/server/src/archive/` に写しを消す経路が 1 つも無い」「ST23 で 1 本にまとめる」。
  コード: `crates/server/src/archive/worker.rs:1810-1830`（`keep_copies` が偽のとき、読み直した確認待ちの写しを `std::fs::remove_file(stored_path)` で消す）。
  st23 `review/deep.md:136-143`（R13）「処置: … docs/handoff/ST23.md の済みの印は record の段で付ける」—— この record の段の commit（`ae77268d`）では handoff は変わっていない。`tasks.md:153`（10.1）が持ち越している
- 直すところ: その文書（済みの印。st23 の tasks 10.1 が既に持つ）
- 機械: できない
- 行き先: docs（handoff ST23 の st12 R23 に済みの印を付けた。試験の確かめは st23 の tasks 10.1）

## 未確認

- st23 の design D9 の「格納の後のまとまりで消し、落ちたら次の見回りで消し直す」が、いまの取り込み器で本当に読み直しを起こすか（`mark_archive_arrivals` が Err のとき `read` の台帳の行を書かない、という design:187-188 の記述）。コードの該当箇所は読んでいない
- ST21 / ST19 / ST17 の錠（`core.require_place_erasure_ledger` `core.require_claim_erasure_ledger` `core.require_feeling_erasure_ledger`）が、ST23 の足す `core.erasure_ledger.operation_id`（NULL 可）で振る舞いを変えないこと。移行の関数の本文（`event_id` と `scope` を見る）は見たが、ST23 の移行はまだ無いので確かめられない

## 触らなくてよいと判断したもの（理由つき）

- **archive の design の「ST23 が消す」**（`openspec/changes/archive/2026-10-13-st21-place-registry/design.md:22`・:315・:344、`…/2026-09-17-st19-personal-attributes/design.md:22`、`…/2026-10-17-st17-daily-feeling/design.md:31`、`…/2026-09-14-st03-idempotent-ingest/design.md:25`、`…/2026-10-04-st22-record-deletion/design.md:34-43`）:
  archive は当時の判断の記録で書き換えない。宛先の変更は INDEX の訂正（2026-10-23）と FR-50 ★ が持つ。ST21 design:344 の「間違えて「直す」を選んだら ST23 の消す口で直す記録を消せば戻る」は、新しい Story ができるまで「移った」の記録を足して戻すことになる（同じ行の括弧書きのとおり。失うものは無い）
- **正典 `record-deletion` の Purpose「本文そのものを消す物理削除（ST23）も、この capability に積む」**（`openspec/specs/record-deletion/spec.md:6`）と :268「FR-52 は ST23 の要件」: ST23 は今も物理削除と FR-52 を持つ。主張・場所の消去をどの capability に積むかは新しい Story が決める
- **正典 `personal-entities` / `subjective-log` の「主張・場所・主観も記録なので FR-50 / FR-51 が及ぶ」**（`personal-entities/spec.md:205`・:862、`subjective-log/spec.md:305`）: 錠が消去を通す約束で、口の持ち主を書いていない。改訂と矛盾しない。
  4 capability（record-deletion 39・personal-entities 251・subjective-log 165・browsing-views 129）の Scenario はすべてテストに `Scenario: <名前>` の印がある（機械で全件見た）
- **`docs/stories/ST23.md` の扉 3（個人属性）**: Q1 で主張は ST23 の外になったが、扉は `check_chain.py` の規則 4（扉に関与する要件を satisfies する Story はその扉に触れる）が FR-51 から機械で付ける。外すなら C3 の直し（satisfies の分け方）と一緒
- **`docs/production-prep.md:73` の「物理削除は対象件数を提示して確認を求める。CLI からも同じ関門」**: FR-52 ★ の「後から届いた分は確認をはさまない」は、確認した指示が後着に及ぶという読み方で、関門（消す口が確認で見せた件数を運ぶ。st23 spec の「確認で見せた件数と消す時点の件数が違えば、消さずに断る」）はサーバの側にある
- **`docs/handoff/ST22.md:65`「ST22 / ST23 は錠を触る Story」**: R20 は st23 の C11（締めない）で受けた。宛先の変更と関係しない
- **`docs/handoff/ST45.md`・`ST47.md`・`ST33.md`・`ST12.md` の FR-50**: 「消した」の連鎖と後着の印・書き出しの除外についての項で、改訂が変えたのは本文ごと消した時間帯の後着だけ。ST33 の数え方は st23 の C15 が受けている
- **`st13-scheduled-account-fetch`**: FR-50 / FR-51 / FR-52 を名指ししない（`req_impact.py` の一覧に無い）。書庫の後着は `mark_archive_arrivals` の中で消すので、ST13 の取り込み器の台帳のトランザクションとは別の関数（st23 `proposal.md:104-105`）
- **`docs/openapi.json:449`**: `GET /events` の要約「削除されていない記録を時刻順に返す（FR-50 のビュー越し）」。本文ごと消した記録も削除の印を持つ（st23 の C2）ので、このまま成り立つ
