# ST25 深掘りの独立レビュー（deep-questions.json / deep.md / proto.html）

schema の手順 1〜5 を、問いの一覧を見る前に自分でやり直してから、Q1〜Q6 と C1〜C8 に突き合わせた。**問いの JSON・deep.md・proto.html は触っていない。**

確かめた範囲: `docs/stories/ST25.md`・`docs/stories/INDEX.md`（:85-175 の訂正の節）・`docs/requirements.md`（FR-1〜FR-2 / FR-7〜FR-13 / FR-16 / FR-18〜FR-25 / FR-30〜FR-31 / FR-33〜FR-35 / FR-50〜FR-57 / FR-76〜FR-85 / PERM-1〜PERM-10 / NFR-1〜NFR-23 / 扉 #1〜#15）、
`docs/ui-direction.md`（全文）、`docs/screens.md`、`openspec/specs/browsing-views/spec.md`（全文）・`desktop-collection`・`device-collection` の Requirement 見出し、
走っている change（`st05` / `st06` / `st08` / `st12` / `st22` / `st28`）の deep・design・specs のうち ST25 か「読む側」「閲覧」を名指ししている箇所、
archive の `st01` / `st02` / `st03` / `st16` / `st19` のうち ST25 を名指ししている箇所（`grep -rn "ST25\|閲覧の Story\|読む側の Story\|S-2" openspec/changes docs/handoff openspec/specs`）、
`crates/server/src/{lib.rs,stay_store.rs,stay.rs,ingest.rs}`・`crates/collector-windows/src/{lib.rs,runtime.rs}`・`migrations/*.sql`（`core.event` / `event_live` / `event_folded` / 索引 / 登録簿）、
`web/src/{Root,App,DayView,MasterView}.tsx`・`tokens.ts`、`scripts/ask_wizard.py`（proto への注入）。

**実行した確認**:

- proto を headless Chromium（`/home/yosis/dev/ashiato2/web/node_modules/playwright`）で開き、5 つの preset をダーク・ライトの両方で切り替えて `#mRows` `#mHeight` `#mFirst` と出力（`#prompt`）を読んだ。
  Q1 の context の値（13 行・1,421 px / 27 行・1,701 px / 1,840 px / 24 行・2,797 px / 2,857 行）は**全部一致**した（全記録 1 列は 77,913 px）。明暗を変えても値は変わらない
- 同じ page で、preset「横に揃える（3 列）」の画面の文字に「離席」「停止」が何回出るか（0 / 0）、preset「全部を 1 列」に「記録なし」「滞在」「移動」が何回出るか（0 / 0 / 0）、
  3 列の塊どうしの重なり（同じ列で上下が重なる組 2 つ）を数えた（R6 / R7）
- `grep -rn "matchMedia\|SCHEMES.dark" web/src/*.tsx`（R9）、`grep -ln "NFR-17" docs/stories/*.md`（INDEX.md と ST25.md だけ。R4）
- `python3 scripts/board.py`: ST14 / ST15 は `collection-coverage` を ST12 が触っているため衝突待ち（Q5 の context と一致）

手順ごとの結果:

- 手順 1（要件どうしの衝突）: FR-56 と ui-direction の「1 行の単位＝滞在」（Q1。deep.md と一致）。
  加えて、INDEX:164 と ST02 の proposal:128 が「S-1 の画面の下限を誰が持つか」で逆を言っている（R4）。FR-50 の「1 日の一覧では消した時間を『消した』と出す」を proto の 1 つの骨格が落とす（R6）
- 手順 2（扉の幅）: 扉 #14（欠損の意味）が、この Story で初めて**位置以外のソース**の上に効く。除外・gap・停止・まだ届いていないが一覧にも C にも無い（R1）。
  位置の「記録なし」も稼働状況を読まず、導入前・止めていた期間まで「取れていなかった」と出す（R2）。扉 #6（地域の時刻）は R12
- 手順 3（新たに立つ一方通行）: 該当なし（確かめた範囲: 新しい読み出しは読むだけで行を書かない。端末のブラウザに記録が残る経路は ST28 の C4（全応答に `Cache-Control: no-store`。`openspec/changes/st28-private-network-only/deep.md:140`）が塞ぐ。
  列の型・保存形式・鍵の計算の入力を変える案は Q1〜Q6・C1〜C8 のどれにも無い。滞在・主張は既に `core.event` の行で、この Story は形を変えない）。
  **ただし Q6 の選択肢 2 を選ぶと `discarded` が立つ**（R10）
- 手順 4（日常に影響する選択）: Q1 / Q4 のほか、毎日目に入る「PC の行が無い」の読み方（R1）と、前の日へ移るたびに出る「記録なし 0:00 – 24:00」（R2）。電池・通知・容量には効かない
- 手順 5（既存コードが要件を満たしていない箇所）: deep.md の 5 件に加えて、S-2 の経路の暦に無い日付の画面がダーク固定（R9）、
  `event_folded` が 1 日を読む形になっていない（R11）、位置の記録なしが稼働記録を読まない（R2）

**指摘しなかった C**: C1（NFR-23 と ui-direction の宿題 1 から決まる。ST02 Q15 と同じ解で、ラベルの集合は R5 / R8 で触れる）/
C2（出来事の時刻のまま並べ、ずれの補正は表示の計算。入力が残るので戻る）/ C5（画面は ST28 の画面ログインの経路。感度は外へ出す経路の話）/
C7（いま消せるのは滞在と位置だけで、その表示は ST22 の「消した」の行が持つ）/ C8（集計は時刻を持たない。R5 で他の種別の扱いを足す提案をした）。

**不要な問いは見つからなかった。** Q4 は ST16 の design D8（仮）の反転条件（`openspec/changes/archive/2026-09-15-st16-stay-derivation/design.md:270`）が、
Q6 は ST22 の C3 が、Q2 は ST12 が ST25 を名指しして残したもので、どれも要件や扉で幅が閉じていない。

---

## R1. 位置以外のソースの「無い」の意味（扉 #14）が問いにも C にも無い。proto は離席と PC の停止しか描かない
- 分類: 抜け
- 成果物: openspec/changes/st25-day-timeline/deep-questions.json（Q1 の proto の軸 4）/ deep.md:32-33（手順 2）
- 根拠: docs/requirements.md:218-225 FR-83「**除外した事実を残さないと、その時間帯の「記録が無い」が「PC を触っていなかった」のか「除外された」のかを永久に区別できない**」/
  docs/requirements.md:106 FR-85（取得元に残っていなかった期間を記録する。ST06 の `gap`）/ docs/requirements.md:184 FR-13（ブラウザ履歴は 24 時間間隔。今日の訪問は翌日まで届かない）/ docs/requirements.md:244 FR-16（取り込み済みの最終日）/
  openspec/specs/desktop-collection/spec.md:145-157（除外は本文を残さず事実と件数だけを残す）/ crates/collector-windows/src/lib.rs:39 `LOGICAL_SOURCE = "c02-window"` と runtime.rs:734 / :1003（`powered-off` / `excluded` が同じソースの記録として送られる）/
  openspec/changes/st25-day-timeline/proto.html:174-177 `IDLES` は「要約に文字で添える」「10 分以上の離席と PC の停止を行にする」の 2 つだけで、除外・gap・まだ届いていないは描く手段が無い /
  deep.md:33「扉 #14（データが無い日の意味）は『記録なし』の行（ST16 の決着）と Q3」—— 位置の骨格にしか当てていない。
  nest の骨格では、PC の要約が無い滞在の行は「PC を使っていなかった」と読める（除外された時間・ブラウザ履歴がまだ届いていない今日も同じ顔になる）
- kind: daily
- 提案: Q1 の proto の軸 4 を「取れていない時間（離席・PC の停止・除外・取得元に残っていなかった・まだ届いていない）」に広げて描く。広げないなら B を 1 問足す（推奨: 要約の行に文字で添える ——「PC 除外 3 件」「ブラウザは 9/13 まで取り込み済み」）。記録は全部残っていて表示の規則は戻せるので `loss` は無い。
- 処置: escalated — Q1 の proto の軸 4 を「取れていない時間（離席・PC の停止・除外・まだ届いていない）」に広げて描き（除外の記録を足し、日「今日 14:30」でブラウザ履歴が届いていない状態を出す）、Q1 の context に書いた。deep.md の手順 2 を直した

## R2. Q3 が狭い。位置の「記録なし」は稼働記録を読まないので、書庫の日だけでなく導入前・止めていた期間・破棄された時間も「取れていなかった」と出る
- 分類: 抜け / 前提が誤り（Q3 の why の範囲）
- 成果物: openspec/changes/st25-day-timeline/deep-questions.json（Q3）/ deep.md:41（手順 5）
- 根拠: crates/server/src/stay_store.rs:958-990 —— 記録なしは `load_points(pool, user, &c.sources, d0 - gap, d1 + gap)` の点の間隔だけから作り、稼働記録（停止・破棄の報告・収集開始日）を読まない /
  openspec/specs/browsing-views/spec.md:77「過ぎた日に位置の記録が 1 件も無いとき、その日全体を記録なしの行として出す」/
  docs/requirements.md:511 FR-54 の 8 状態（「意図的な停止」「破棄された期間」「途絶」「導入前」「退役」を「記録なし」と分ける）/ :397 FR-79 / :351 FR-34 / :147 FR-9 /
  Q3 の why「『記録なし』は『取れていなかった』を意味する行（扉 #14）なので、事実と違う」—— 同じ理屈が、収集開始日より前の日（「前の日」を押すたびに毎日 0:00 – 24:00）と、本人が止めた期間と、端末が破棄した時間にそのまま当たる。書庫の日はその一部
- kind: daily
- 提案: Q3 を「位置の記録なしの行に、その時間の稼働の状態（導入前・止めていた・破棄・書庫にある）を文字で添えるか」に広げる（B。推奨は文字で添え、行の判定は変えない）。
  行の中の状態の符号化なので、Q1 の proto の「記録なし」の行に添え書きの有無を 1 軸足して描くほうが速い（文字で問うなら、推奨の既定があるので B のままでよい）。
- 処置: escalated — Q3 を「位置の記録なしの行に、その時間の事情（収集を始める前・止めていた・破棄・書庫）を添えるか」に広げた（B、推奨: 文字で添え行の判定は変えない）。why に stay_store.rs の計算と FR-54 の 8 状態を書き、deep.md の手順 5 を直した

## R3. Q2 の組の列挙が申し送りと違う。「移行前のロケーション履歴 × Timeline.json」（どちらも書庫）が抜け、ST08 の「2 台の PC から届いた同じ訪問」も無い
- 分類: 前提が誤り / 抜け
- 成果物: openspec/changes/st25-day-timeline/deep-questions.json（Q2 の question・options）/ deep.md:10-23（この Story に宛てられたもの）
- 根拠: openspec/changes/st12-archive-ingestion/deep.md:326-327「**移行前のロケーション履歴と Timeline.json の重なる期間** / Chrome の履歴（Takeout）と PC のブラウザ履歴（ST08）/ YouTube の視聴履歴とマイアクティビティの YouTube」——
  Q2 の question は 1 組目を「書庫のタイムラインと手元の位置」に置き換えている。申し送りの 1 組目は**両方とも書庫**なので、選択肢 2 / 3（手元を優先 / 書庫を優先）ではどちらを隠すかが決まらない /
  openspec/changes/st08-browser-history/design.md:33「同期している複数の PC から届いた同じ訪問を 1 件に畳んで読む形（**読む側の Story**。材料は D9 で残す）」/ :188-189「2 台から届いた同じ訪問は、訪問時刻（同期で保たれる）と URL のハッシュで読む側が 1 件に畳める」/
  openspec/changes/st08-browser-history/deep.md:184 本人の答え「PC 側の番号で作る（2 台から届いた同じ訪問は別の行。**読む側で畳む**）」。ST25 の名を出していないので deep.md の「宛てられたもの」の拾い方から漏れている
- kind: premise
- 提案: Q2 の組を申し送りの逐語に直し（書庫 × 手元の位置は Q3 の側に残してよい）、ST08 の組を足す。選択肢に「書庫どうしの組（移行前 × Timeline.json）はどちらを出すか」を入れる。
  ST08 の組は C6（内容の鍵で畳む）では畳まれない（識別子が PC 側の番号なので原文が違う）ことを context に書く。表示の規則なので B のまま。
- 処置: escalated — Q2 の組を申し送りの逐語 3 組に直し、ST08 の「2 台の PC から届いた同じ訪問」を（4）として足した。推奨を「（4）だけ 1 件に畳み、（1）〜（3）は両方出す」にし、（4）が C6 では畳まれないことを context に書いた。deep.md の宛てられたものに ST08 を足した

## R4. Q5 の前提が片側だけ。ST02 は S-1 の画面の下限を「ST25 が代表して持つ」として持たなかった。選択肢 1 の申し送り先が名指しされていない
- 分類: 前提が誤り（conflict の片側が抜けている）
- 成果物: openspec/changes/st25-day-timeline/deep-questions.json（Q5 の why・選択肢 1）
- 根拠: openspec/changes/archive/2026-09-14-st02-collection-coverage/proposal.md:127-128「**UI 系の非機能（NFR-17〜NFR-22）はすべて ST25（S-2 主表現）が代表して持つ設計になっているため**」/
  docs/stories/INDEX.md:164「画面を持つ Story が自分の画面について満たす」（2026-09-15、ST19 の上流工程の訂正。ST02 の後に書かれた）/
  docs/stories/ST25.md:4 `satisfies` に NFR-17〜23。`grep -ln "NFR-17" docs/stories/*.md` は INDEX.md と ST25.md だけで、ST14 / ST15 の `satisfies` に NFR-17 は無い /
  openspec/changes/st22-record-deletion/review/spec.md:50-53（R1）「Non-Goals に書くだけでは鎖が切れる」「`check_chain.py` は … 誰にも拾われないまま緑になる」
- kind: conflict
- 提案: Q5 の why に ST02 proposal:128 を足し、2 つの文書が逆を言っていることを示す（これが Q5 の conflict の本体）。
  選択肢 1 の「ST14 / ST15 か、ST12 の merge 後の小さな直し」を 1 つに名指しし、`docs/handoff/ST<NN>.md` と `処置: followup ST<NN>` で拾う形まで書く。名指ししないと、ST25 の archive で NFR-17 の鎖は緑になり、S-1 はダーク固定のまま残る。B のまま。
- 処置: escalated — Q5 の why に ST02 proposal:127-128 と INDEX:164 の食い違いを書き、選択肢 1 の申し送り先を ST14（`docs/handoff/ST14.md` / `followup ST14`）と名指しした

## R5. 「その日の記録」に何が入るかの C が集計（C8）しか無い。`core.event` には滞在・個人属性の主張・時計の測定・除外・停止・gap・非前景のアプリ利用イベントが同じ表に入る
- 分類: 抜け（C）
- 成果物: openspec/changes/st25-day-timeline/deep.md:44-57（C の表）/ deep.md:10-23（宛てられたもの）
- 根拠: openspec/changes/archive/2026-09-15-st16-stay-derivation/design.md:33「D1. 滞在は `core.event` の 1 行。ソースは `s01-stay`」/
  migrations/202609160220_personal_attributes.sql:4-5「主張そのものは `core.event` に入る（`origin='authored'` / `logical_source='s01-attribute'`）」/
  openspec/changes/archive/2026-09-17-st19-personal-attributes/design.md:25 Non-Goals「**主張を 1 日の一覧（S-2）や検索に出すこと（ST25 / ST26）**」—— deep.md:20 は ST19 から入口（Q4）しか拾っていない /
  openspec/changes/st05-clock-skew/design.md:16 / :189（PC の時計の測定記録は `c02-window` の `kind = clock-skew`。毎時 1 件）/
  openspec/changes/st06-app-usage/specs/device-collection/spec.md:6-7「1 イベントにつき 1 件」「種別でふるい落とさない」/
  proto.html:238「スマホのアプリ（前景に出た 1 回を 1 行。画面の入切などのイベントは数にだけ入れる）」—— どの種別を行にするかは proto の中でだけ決まっていて、C に無い
- kind: technical
- 提案: C を 1 つ足す ——「1 日の並びに出すのは活動の種別の**許可リスト**（位置・前景・訪問 …）。滞在は骨格として出し、記録として二重に出さない。測定記録は出さない。除外・停止・gap は R1 の『取れていない時間』の側で扱う。主張は出さない（出すかは後から足せる）」。
  許可リストにするのは、ST06 が種別をふるい落とさないので、新しい種別が黙って行に混ざるのを防ぐため（既定は厳しい側）。ST19 の申し送りを deep.md の「宛てられたもの」に足す。
- 処置: fixed deep.md — C9（行にする種別の許可リスト。滞在は骨格・測定記録は出さない・主張は出さない・取れていない時間は Q1 軸 4 / Q3）を足し、ST19 の申し送りを宛てられたものに足した

## R6. proto で要件を破る組み合わせを選べて、出力がそれを言わない（FR-56 / 記録なし・消したの行 / NFR-19 / ui-direction）
- 分類: 分類違い（要件で幅が既に閉じている選択肢が、開いたまま見せてある）
- 成果物: openspec/changes/st25-day-timeline/proto.html（Q1 に埋め込み）
- 根拠:
  (a) proto.html:172 軸 3「アプリ・サイトごとの合計だけ」（「時刻の並びは出さない」）は、どの骨格と組んでも、場所以外の記録が時刻順に並ぶ場所が画面から無くなる →
  docs/requirements.md:549 FR-56「その日の記録を時刻順に表示する」と docs/stories/ST25.md:72「その日の記録がソースをまたいで時刻順に並ぶ」を満たさない /
  (b) proto.html:429 `renderFlat` は滞在・移動・記録なしの行を描かない（実測: preset「全部を 1 列」で「記録なし」0・「滞在」0・「移動」0）→
  openspec/specs/browsing-views/spec.md:71-77（記録なしの行）と docs/requirements.md:494 FR-50「1 日の一覧では、消した時間を『記録なし』ではなく『消した』と出す」を落とす。出力は :513「方向の文書を直す」とだけ書く /
  (c) proto.html:445 `height: Math.max(24,(e-s)*PX-2)` —— 3 列の骨格は短い区間を 24 px に伸ばし、後から描く次の区間に重ねる。実測で 2 組（「移動 11 分」の塊が「滞在 12:15–12:58」「滞在 13:09–17:36」に覆われる）。
  11 分 × 1.2 px ≒ 13 px しか見えないので、覆われた移動の塊は docs/requirements.md:838 NFR-19（24 × 24 CSS px）を割る。proto の「動かせないもの」には NFR-19 が書いてある /
  (d) 「交互に並べる」（1 行 = PC・スマホのまとまり）と「横に 3 本の列」（リストでない）も docs/ui-direction.md:107 / :140-141（「リストで並べ、1 行の単位は『滞在』」）から外れるが、外れを書くのは :513 の flat と hour だけ
- kind: conflict
- 提案: (a)(b)(c) は選べなくするか、選んだとき出力に「FR-56 / browsing-views の記録なし・消した / NFR-19 を満たさない。選ぶなら要件か spec の訂正が要る」と書く。
  (d) は :88 の hint と :513 の外れの判定に inter / lanes を足す（「並べ方＝リスト」「1 行の単位＝滞在」のどちらから外れるかを書く）。
- 処置: escalated — proto を直した: (a) 軸 3「合計だけ」を「合計を先に、その下に畳んだ時刻順」に置き換え、どれを選んでも時刻順が残るようにした / (b) 全記録 1 列に滞在・移動・記録なしを区切りの行として挟んだ / (c) 3 列の塊を伸ばさず時刻どおりの高さで描き、24 px を割る塊の数を測って画面と出力に書く（NFR-19 を割ると書く）/ (d) 並べ方ごとに ui-direction のどちらから外れるかをカードと出力に書く

## R7. proto の出力が、描いていない状態を書く。preset「横に揃える（3 列）」は離席と PC の停止を画面のどこにも出さないのに、出力は「要約に文字で添える」と書く
- 分類: 文字で決めさせている画面の問い（描かれていない組み合わせが答えになる）
- 成果物: openspec/changes/st25-day-timeline/proto.html
- 根拠: proto.html:182 preset `{skel:"lanes", sum:"count", det:"fold", idle:"text"}` / :452 3 列の骨格は `state.idle==="row"` のときだけ離席と停止を描く / :490 軸 4 が無効になるのは flat のときだけ /
  実測: この preset で画面の文字に「離席」0 回・「停止」0 回、出力の 4 行目は「離席と PC の停止 — 要約に文字で添える。」。
  docs/ui-direction.md の UIR-5 / UIR-6 / UIR-29「一度も描かれていない組み合わせが prompt に書けてしまっていた」と同じ型
- kind: technical
- 提案: 3 列の骨格では軸 4 の「要約に文字で添える」を無効にして出力から落とすか、3 列の中に文字で描く。貼り戻された答えがこの組み合わせのままだと、specs を書く側は画面に無いものを要件にする。
- 処置: fixed proto.html — 3 列でも軸 4「文字で添える」を PC の塊の文字（離席 N 分・除外 N 件・→ 停止）に描くようにした。実測で 3 列の preset に「離席」が出る

## R8. proto の「実物の量」が 1 日だけ。滞在の上限の日と、Q2 の推奨（両方出す）で増える書庫の記録が描かれていない
- 分類: 文字で決めさせている画面の問い（高さの比較が 1 つの日にしか当たらない）
- 成果物: openspec/changes/st25-day-timeline/proto.html / deep-questions.json（Q1 の why・context）
- 根拠: proto の日は滞在 7 件・閉じた行 13（1,421 px）。docs/ui-direction.md:194「要件上限の 15 件（FR-76 の滞在は 1 日 5〜15 件）は、スマホ幅では 1 画面に収まらない」/
  ui-direction のレビュー UIR-32 / 46 / 47（「最長・最多ケースが 1 つも無かった」→ 文脈に「最長・最多」を足した）/
  proto.html:229-240 の `REC` は場所・PC・ブラウザ・スマホだけで、書庫のソース（`c03-*`）の記録は 0 件 —— Q2 の推奨を採ると、書庫を置いた日は要約と詳細に「書庫: Chrome」「YouTube」「タイムラインの訪問」の行が加わる /
  スマホは前景に出た回数 226 件だけ（:238）で、ST06 が集める種別すべてではない（件数は ST06 の tasks 7.1 で実測する予定: openspec/changes/st06-app-usage/design.md:135）/
  Q1 の why「約 3,000〜4,000 件（proto の日は 2,857 件）」は自分で書いた幅の下限を割っている
- kind: technical
- 提案: proto に「日」の切替を足す —— いまの日 / 滞在 15 件の日 / 書庫を置いた日（Q2 の推奨どおり両方出す）。骨格ごとの高さと「開いてすぐ見える行」を 3 つの日で出し、出力にも 3 つの値を書く。
  C1 のラベルの集合（「PC」「スマホ」「ブラウザ」「場所」）に書庫のソースのラベルが無いことも、この日を描けば見える。
- 処置: fixed proto.html — 日の切替（いつもの日 / 滞在 15 件の日 / 書庫を置いた日 / 今日 14:30）を足し、出力に 4 つの日の行数・高さ・開いてすぐ見える行を書く。書庫のラベル（「書庫: Chrome」など）が出る。Q1 の why を「数千件（見積もり 約 4,120、proto のいつもの日 2,795）」に直した

## R9. 前提が誤り: 「1 日の画面は明暗に追従する」は、S-2 の経路のうち暦に無い日付の画面で成り立たない（ダーク固定）
- 分類: 前提が誤り
- 成果物: openspec/changes/st25-day-timeline/deep.md:39（手順 5）/ deep-questions.json（Q5 の why「1 日の画面とマスタ管理は追従する」）
- 根拠: web/src/Root.tsx:29-33 `data-testid="day-invalid"` の画面が `background: tone(SCHEMES.dark.ground), color: tone(SCHEMES.dark.text)` を直に使い、`useScheme` を読まない。
  `#/day/2026-13-45` を開くと出る、S-2 の経路（ST25 自身の画面）の一部。`grep -rn "matchMedia" web/src/*.tsx` は DayView.tsx だけ（MasterView は DayView の `useScheme` を import）/
  docs/requirements.md:831 NFR-17 / docs/stories/ST25.md:73「OS をライトに切り替えると画面が明るくなり」
- kind: premise
- 提案: Q5（S-1 をどうするか）とは別に、S-2 の経路の直しとして C か tasks に入れる（NFR-17 は ST25 の satisfies なので問いにしない）。deep.md の手順 5 と Q5 の why の「1 日の画面は追従する」を「暦に無い日付の画面を除いて」に直す。
- 処置: escalated — deep.md の手順 5 と Q5 の why を「暦に無い日付の画面を除いて」に直し、その画面の明暗は C10（ST25 自身の画面なので問わずに直す）にした

## R10. Q6 の選択肢 2 に、失われるもの（discarded）が書かれていない
- 分類: 不可逆の記述が無い
- 成果物: openspec/changes/st25-day-timeline/deep-questions.json（Q6 の選択肢 2）
- 根拠: openspec/changes/st22-record-deletion/review/deep.md:61-67（R4、`loss: discarded`）「消していた間に届いた外部サービスの更新は捨てられる。… 戻した記録にその更新が無い」/
  docs/requirements.md:478-481 FR-50「削除済みの記録には、同じ内容の再送も外部サービスからの更新も取り込まない」/
  Q6 の context は R4 を指しているが、選択肢 2 の detail は「ソースごとの副作用（消していた間の外部の更新・…）をこの Story で決める」で、何が戻らないかを書いていない。
  Q6 の why「足さなくても何も失われず」は選択肢 1 について正しく、推奨（足さない）が既定なので B の置き方は成り立つ
- kind: irreversible
- loss: discarded
- 提案: 選択肢 2 の detail に「外部の識別子を持つソース（ブラウザ履歴・書庫）を消すと、消していた間に届いた更新は捨てられ、戻しても戻らない」を書く。
  context に「選択肢 2 が選ばれたら、口を開けるソースごとに A の問いを第 2 回で立てる」を足す。B のままでよい。
- 処置: escalated — Q6 の選択肢 2 に irreversible（消していた間の外部の更新は捨てられ、戻しても戻らない）を書き、context に「選ばれたら第 2 回で A の問いを立てる」を足した。推奨（足さない）が既定なので B のまま

## R11. C3 / C6 の技術の前提: `core.event_folded` は 1 日を読む形になっていない。ST26 が同じ読み出しを使うことも C3 に無い
- 分類: 前提が誤り（C の技術の前提）
- 成果物: openspec/changes/st25-day-timeline/deep.md:52（C3）/ :55（C6）
- 根拠: migrations/202609120943_version_and_ledger.sql:95-109 —— 列は user_id / logical_source / content_hash / folded_rows / event_time / id / raw / payload / origin / sensitivity / external_ids だけで、
  `tz_offset_min` / `tz_id`（FR-20）・`device_id`（FR-24。PC が 2 台のときの見分け。R3）・`ingest_time` が無い。`event_time` は `min(event_time)` で `GROUP BY user_id, logical_source, content_hash` の後の列なので、
  日付で絞る条件は表全体を畳んだ後に掛かり、索引 `event_by_source_time`（migrations/202609112113_source_lifecycle.sql:71）が効かない /
  docs/stories/ST26.md:35「壊してはいけないもの: ST25 のタイムライン（同じ読み出し経路を使う）」
- kind: technical
- 提案: C6 を「ST03 の代表の選び方（D16（仮））を、日付で先に絞ってから畳む形で使う。地域と端末の列を持つ」に直す（ビューの再定義で戻せる。問いにしない）。
  C3 に「ST26（検索）が同じ読み出しを使う。要約だけを返す形にしない」を足す。
- 処置: fixed deep.md — C6 を「日付で先に絞ってから畳む形・地域と端末の列を持つ」に直し、C3 に ST26 が同じ読み出しを使うこと（1 件ずつ返せる形）を書いた。手順 5 に event_folded の件を足した

## R12. C4 の「決着済み」は ST16 の design D8（仮）を spec に上げたもの。記録の地域が Asia/Tokyo でない行の時刻の出し方（FR-20 / 扉 #6）は C に無い
- 分類: 前提が誤り（C の理由）/ 抜け（C）
- 成果物: openspec/changes/st25-day-timeline/deep.md:53（C4）
- 根拠: openspec/changes/archive/2026-09-15-st16-stay-derivation/design.md:237-240「### D8（仮）. S-2 の最小形 … 一覧は `#/day/YYYY-MM-DD`（省けば今日。Asia/Tokyo）」/ :270 反転条件「ST25 が … ソースをまたいだ時刻順の表示を足すときに、ルートと行き先の置き方を決め直す」/
  openspec/specs/browsing-views/spec.md:12「その日（Asia/Tokyo）」/ docs/requirements.md:267 FR-20（UTC からのずれとタイムゾーン識別子の両方）/ :982 扉 #6（決定済: 記録の有無）。
  ソースをまたいで記録の時刻を並べて見せる画面はこの Story が最初で、旅先では PC とスマホの地域が違う日が起きうる
- kind: technical
- 提案: C4 の理由を「正典（ST16 の D8（仮）を spec に上げたもの）」に直し、C を 1 つ足す ——「日の区切りと行の時刻は Asia/Tokyo。記録の地域が違う行には、その地域の時刻を添える」。表示の規則で戻せるので問いにしない。
- 処置: fixed deep.md — C4 の理由を「ST16 の D8（仮）を spec に上げた正典」に直し、C11（地域の違う行にその地域の時刻を添える）を足した

## R13. Q4 は kind: open だが、deep.md の手順 4 は Q4 を日常の選択として挙げている
- 分類: 分類違い
- 成果物: openspec/changes/st25-day-timeline/deep-questions.json:72（Q4 の kind）
- 根拠: deep-questions.json:72 `"kind": "open"` / deep.md:36「手順 4（日常に影響する選択）: 毎日開く画面の構造（Q1）と、開いたとき最初に出るもの（Q4）」
- kind: daily
- 提案: Q4 の kind を `daily` にする（B のまま。推奨も変えない）。
- 処置: escalated — Q4 の kind を daily に直した（B のまま。推奨も変えない）
