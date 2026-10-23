# ST23 の深掘り（第 1 回）の独立検査

対象: `openspec/changes/st23-record-erasure/deep-questions.json`（Q1〜Q6）・`premises.md`・`defaults.md`（C1〜C12）・`proto.html`（= `docs/briefs/ST23-proto.html`。`cmp` で同一）。
読んだもの: `docs/stories/ST23.md`、`docs/briefs/ST23.html`（`docs/handoff/ST23.md` と各 change の deferred）、`docs/requirements.md` の FR-44 / FR-50 / FR-51 / FR-52 / NFR-16 / NFR-20、
ST22 の archive（deep.md・design.md・specs）、正典 `openspec/specs/`、migrations（envelope・dedup・version_and_ledger・gates・stays・deletion_ledger・subjective_log・drop_reports・archive）、
`crates/server/src/{deletion,stay_store,lib,ingest,feelings,day_records}.rs`、`export/excluded.rs`、`archive/worker.rs`、`grants.sql`、`web/src/DayView.tsx`、走っている st09 / st13、
ST03 / ST12 / ST17 / ST19 / ST21 / ST33 の archive の「ST23」への言及（`grep -rn ST23 openspec/changes/archive`）、`python3 scripts/upstream_misses.py report`。

## 手順ごとの記録（問いの一覧を見る前に当てた結果）

- 手順 1（要件の衝突）: FR-51「本文を消去」と、FR-50 ★（消した時間帯の後着の印・「消した」の行・作り直しで戻さない）/ NFR-16（消した滞在の時間に入る緯度経度を持つ行を除く）/ FR-50 ★ 2026-10-17（衛星と AI に撮影位置を渡さない）が、**同じ「滞在の時間の範囲」を滞在の本文から読んでいる** → R1。
  FR-52「実行前に件数を提示」と Q3 の推奨（後着を見せずに消す）→ R7。FR-51「本文」と書庫の写し → R10。FR-51 と「消したものが再流入しない」は C1 が鍵を残す側で解いている（同意。前提の段は R17）
- 手順 2（扉の幅）: 扉 3 は Q1 が受けている。扉 7「台帳のある消去だけ、DB が強制」は、派生（滞在）の行に及んでいない（C5 が受ける。ただし前の版の強制が抜けている → R14）
- 手順 3（一方通行）: 本文の消去（Q2 / Q3）、写し（Q4）、**滞在の範囲の消失**（R1）、新しい追記のみの台帳の形（R6）、鍵を残す（C1）
- 手順 4（日常）: Q6 の表の書き直し（R15）、Q5 の行。ほかに毎日目に入るもの・通知・電池・容量の新しい選択は見つからない（確かめた範囲: Q5 の proto の行、C6 の稼働状況、ST22 の 1 日の一覧）
- 手順 5（既存コード）: R1・R8・R12・R13・R14・R15・R18
- 手順 6（過去の取りこぼし）: 時計（R3）/ 後処理の失敗（R8・R15）/ 申し送りの見出し単位の突き合わせ（R4・R5・R13）/ 見積もりを実物で測っていない（R15）/ 書き直せない記録の形（R6）/ 既存コードの振る舞いを確かめずに置いた（R12・R13）。
  検証コマンド系の 11 件は tasks がまだ無いので該当なし（確かめた範囲: change に tasks.md が無い）
- 手順 7（族）: Q3 の族に時計の兄弟が無い（R3）。Q2 の族に気分が無い（R2）。Q4 の族（DB の外のファイル: 写し・本人のファイル・D-02 の原本・書き出し・バックアップ・確認待ちの写し・WAL）は揃っている
- 手順 8（前提）: R17・R15・R16
- proto の初期表示と Q5 の draft: **一致**（`proto.html:177` の `DEFAULTS` = 置き場 `erased` / 件数 `left` / 確認 `button` / 後 `row` / 範囲 `cascade` が、draft の「消した行に戻すと並べる・ソースごと＋前の版＋残るもの・本文ごと消す / やめる・本文ごと消した行が残り戻すは出ない」と 4 軸とも同じ。入力の範囲も Q2 の推奨）。
  ただし draft と proto の「本文ごと消した 8:31 – 12:04」の行は、いまのデータの形では描けない（R1）

## R1. 滞在を本文ごと消すと、その滞在の終わりの時刻が消え、「消した時間帯」に依る 5 つの規則が一斉に外れる（問いが無い）
- 分類: 抜け（要件の衝突）
- 成果物: openspec/changes/st23-record-erasure/deep-questions.json（Q2・Q3・Q5）/ defaults.md（C3）/ premises.md
- 根拠: 消した滞在の範囲は滞在の本文から読む —— `migrations/202609271716_deletion_ledger.sql:60`（`core.stay_erased.end_at = coalesce(payload->>'end', event_time + 1µs)`）。
  使う側: 書き出しの除外 `crates/server/src/export/excluded.rs:197-205`（NFR-16。前の版の `payload->>'end'` も読む）/ 1 日の一覧の「消した」行 `crates/server/src/stay_store.rs:1292` / 後着の印 `stay_store.rs:877`・`:633-641`（FR-50 ★ 2026-09-29）/ 作り直しの既存の滞在 `stay_store.rs:313-345`（FR-50 ★ 2026-09-13「消した時間帯に派生を作らない」）。
  **実測**（使い捨ての `postgres:17-alpine` に `core.stay_erased` の定義をそのまま当てた）: 消去の形（`raw=''` / `payload='{}'`）にした後、8:31–12:04 の滞在の `end_at - start_at` が `00:00:00.000001` になった。
  写真は連鎖に入らず本文も残る（C8・FR-50 ★ 2026-10-17）ので、その時間の撮影位置が書き出し（NFR-16）・衛星と AI（FR-60 / FR-63 / FR-100）の除外から外れる。消した時間帯の後着の位置は印が付かず生きた記録になる（Q3 のどの選択肢も、範囲が引けなければ成り立たない）。「消した」の行は 1µs になり、位置も消えたその時間は「記録なし」に吸われる（FR-50「記録なしではなく消したと出す」に反する）。
  C3 は台帳の行に「滞在の識別子と時間帯」を持つと書くが、**上の読み手を台帳へ切り替えるとはどこにも書いていない**。Q3 の族の「作り直しで滞在ができる → 作らない（FR-50 ★）」の決着も、この前提では成り立たない
- kind: conflict
- loss: exported
- 提案: 問いを 1 つ足す（A）。「本文ごと消した滞在の始まりと終わりを、どこにどの細かさで残すか」—— (a) 台帳（または本文の外の列）に残し、上の 5 つの読み手をそこへ切り替える（範囲は追記のみの台帳に永久に残る）/ (b) 残さない（撮影位置が書き出しと衛星・AI へ出る、後着の位置が生きて入る、行が記録なしになる）。
  選んだ側を Q3 の context と Q5 の draft の前提に書き、premises.md に「消去の後に範囲が引けるか」を実測つきで足す
- 処置: escalated —— 問いにはしない。FR-51 が「範囲を台帳に残す」と決めているので残さない側は要件を満たさない（失わない側が要件どおりで費用も小さい）。範囲を台帳から読む読み手の切り替えを C13 に、Q3 の context に範囲の前提を、premises.md に実測を足した

## R2. 消した滞在に紐づけた気分（主観）が、Q2 の「一緒に消すもの」にも C にも無い
- 分類: 抜け（族: Q2 の「その場面を持つ他の記録」）
- 成果物: openspec/changes/st23-record-erasure/deep-questions.json（Q2）/ defaults.md
- 根拠: 気分は滞在を指して書ける（`migrations/202610141000_subjective_log.sql` の `feeling_target_kind` に `stay`）。原文に滞在の識別子と**始まり・終わり**を持つ（`crates/server/src/feelings.rs:209`）。主観の錠は台帳つきの消去を待っている（同 `.sql:139-176`）が、Q1 の推奨では気分を消す口を作らない
- kind: technical
- 提案: Q2 の context に「その滞在を指す気分は残る（本文に始まり・終わりと気分の文を持つ）」を書き、扱いを C に 1 行足す（残す側なら失うものは無い）。一緒に消す側を選択肢に入れるなら、その選択肢の irreversible に「気分の文が戻らない」と書く
- 処置: fixed defaults.md C14 —— 気分は残す（座標を持たない本人の記録）。Q2 の context に書いた

## R3. Q3 の族に「時計の狂った端末が、本文ごと消した時間帯の時刻で記録を送る」が無い
- 分類: 抜け（族・過去の取りこぼし「時刻を入力にして状態を一方向に動かす決定」）
- 成果物: openspec/changes/st23-record-erasure/deep-questions.json（Q3 の `family.siblings`）
- 根拠: `python3 scripts/upstream_misses.py report` の第 1 項（ST02 Q29 / ST08 Q7・Q11 / ST06 Q9 で同型 4 件）。端末の時計のずれは測定記録を持つ（`migrations/202609291230_clock_source.sql`。ST05）。
  Q3 の推奨は「出来事の時刻が消した時間帯に入る後着」の本文を見せずに消す（`deep-questions.json:95`）ので、時計が進んで / 戻って**別の時間の位置が消した時間帯の時刻で届く**と、関係の無い記録の本文が戻らない。逆向き（消した場面の位置が時間帯の外の時刻で届く）は生きた記録として入る
- kind: irreversible
- loss: discarded
- 提案: 兄弟を 2 行足す（「時計が進んだ / 戻った端末の位置が、消した時間帯の時刻で届く」「消した場面の位置が、時間帯の外の時刻で届く」）。Q3 の選択肢の irreversible に前者の帰結を書く（ST05 の測定で外せるかは前提として premises.md に置く）
- 処置: escalated —— family に時計の兄弟 2 行、推奨の irreversible に時計の帰結を書いた

## R4. ST33 design D17 の申し送り（後に消した記録の数え方は ST23 が決める）が、問いにも C にも無い
- 分類: 抜け（申し送りの取りこぼし）
- 成果物: openspec/changes/st23-record-erasure/defaults.md
- 根拠: `openspec/changes/archive/2026-10-17-st33-data-export/design.md:457`「ST23 が D-01 から記録を本当に消すと、その記録はいま除かれる記録の一覧から消え、数に入らなくなる → 消した id を同じ数え方に足すかを ST23 が決める」。
  同 D17（`:426-450`）は「少なく数えて残っていないと思わせる経路が無い」を不変にしている。C2（印を付ける）なら本文を消した行は数に残るが、写真のように D5 の時間帯で除く行は R1 の範囲の消失で除かれなくなり、数が減る
- kind: defer
- 提案: C に 1 行足す（本文を消した記録は削除の印で `erased_after` の数に残る / 時間帯で除く行は R1 の答えに従う）。`docs/handoff/ST33.md` か design の Non-Goals に、D17 の鍵（`export::ErasedKey`）に消去の台帳を足すかを書く
- 処置: fixed defaults.md C15

## R5. ST22 design D13 の反転条件（FR-52 の確認と論理削除の確認の形を揃えるか）を受けていない
- 分類: 抜け（申し送りの取りこぼし）
- 成果物: openspec/changes/st23-record-erasure/defaults.md
- 根拠: `openspec/changes/archive/2026-10-04-st22-record-deletion/design.md:235-236`「ST23 が FR-52 を実装するとき、論理削除の確認と文面・操作の形を揃える（揃えるなら MODIFIED）。揃えないなら、2 つの確認が並ぶ理由を ST23 の design に書く」。Q5 の draft は ST22 の確認（位置の件数だけ）と違う中身を足す
- kind: defer
- 提案: C に「ST22 の確認は変えない（戻せる削除）。2 つが並ぶ理由は design に書く」か、揃える側を Q5 の軸に入れるかを決めて書く（どちらも計算し直せる表示で失うものは無い）
- 処置: fixed defaults.md C16

## R6. C3 が足す追記のみの台帳は、消去の後に範囲を知る唯一の場所になるのに、形の凍結と読めない形の扱いが決まっていない
- 分類: 抜け（過去の取りこぼし「書き直せない記録の形」）
- 成果物: openspec/changes/st23-record-erasure/defaults.md（C3）
- 根拠: `upstream_misses.py report`「書き直せない記録（追記のみの台帳・凍結列）の形を新しく決めるときは、上流で形の凍結と読めない形の出し方を問いにする」（ST33 Q12。変異で 1,451 件 → 0 件）。
  消去は本文と台帳の行を同じまとまりで書く（C4）ので、台帳が取らなかった値（時間帯の端の扱い・時刻の地域・ソースごとの件数の分け方）は後から作れない。既存の `core.erasure_ledger` は記録ごとの行だけ（`migrations/202609120943_version_and_ledger.sql:59-69`）で件数も範囲も持たない
- kind: technical
- 提案: C3 に「範囲は始まりと終わりを `timestamptz` で持ち、R1 の読み手が使う」「行の形を固定の値で試験する」「読めない行は 0 件・範囲なしと読まず、厳しい側（除く・数えられない）に倒す」を足す
- 処置: fixed defaults.md C3 —— 範囲は timestamptz のまま・形の固定の試験・読めない行は厳しい側

## R7. Q3 の推奨（後着を見せずに本文ごと消す）が、C7 の理由と FR-52 と FR-50 の「捨てない・戻せる」に反する
- 分類: 分類違い（要件の衝突を書いていない）
- 成果物: openspec/changes/st23-record-erasure/deep-questions.json（Q3）/ defaults.md（C7）
- 根拠: C7「戻せない操作で、見ていない件数を消さない」（`defaults.md:15`）。FR-52「実行前に対象件数を提示して確認を求める」（`docs/requirements.md:700-701`）—— 後着は確認の後に届くので件数は一度も見せていない。
  FR-50「消した時間帯に後から届いた位置の記録も、削除済みとして入れる（捨てない）。本人は消したものを戻せる」（`docs/requirements.md:659-660`）
- kind: conflict
- loss: discarded
- 提案: Q3 の context に、推奨が C7 と FR-52 の読み方（後着は本人の 1 回の指示の範囲に入るか）と FR-50 の ★ を変えることを書き、選んだら「要件へ戻すもの」に載ることを明示する。C7 の理由文は Q3 の答えに合わせて直す
- 処置: escalated —— Q3 の context に「推奨は FR-50 ★ と FR-52 の読み方を変え、要件へ戻す」を書き、C7 の理由を後着と分けて直した

## R8. Q3 の推奨「入れた同じまとまりで本文も消す」は、書庫の取り込みでは成り立たない（格納と印付けが別のまとまり）
- 分類: 前提が誤り
- 成果物: openspec/changes/st23-record-erasure/deep-questions.json（Q3 の選択肢 1 の detail「本文は DB に一度も残らない」）
- 根拠: 書庫は格納を commit してから、別のまとまりで消した時間帯の印を付け、落ちたら次の走査で付け直す（`crates/server/src/archive/worker.rs:2458-2477`、`stay_store.rs:824-852` の `mark_archive_arrivals`）。
  端末の取り込みの印も作り直しの中（`stay_store.rs:568`）と書き出しの前（`export/job.rs:230`）だけ。Q3 の context は「どの選択肢でも取り込みの直後に移す」と書くが、書庫の経路は「直後」でも同じまとまりではない
- kind: premise
- 提案: 選択肢 1 の detail を「端末の取り込みでは同じまとまり。書庫は格納の後のまとまりで消し、落ちたら次の走査で消し直す（その間は本文が DB にある）」に直すか、書庫の格納を項目ごとに同じまとまりへ寄せることを費用として書く
- 処置: fixed deep-questions.json —— 選択肢 1 の detail を、端末は同じまとまり・書庫は格納の後の別のまとまり（落ちたら次の見回りで消し直す）に直した

## R9. Q4 の推奨の「irreversible: なし」は、写しがバックアップ（ST30）に入ると成り立たない
- 分類: 不可逆の記述が誤り
- 成果物: openspec/changes/st23-record-erasure/deep-questions.json（Q4 の選択肢 1）/ defaults.md（C9）
- 根拠: ST12 は「写しはバックアップ（ST30）の対象に入れる」と決めている（`openspec/changes/archive/2026-10-13-st12-archive-ingestion/design.md:234`）。拠点外への写し（FR-68 / ST31）は暗号化されても中身を持つ（ST33 deep X1 と同じ型。`openspec/changes/archive/2026-10-17-st33-data-export/deep.md:189`）。
  C9 の ST30 への申し送りは「本文を消した記録」だけで写しを挙げていない。`docs/handoff/ST30.md` に写しの項は無い（`grep -n 写し docs/handoff/ST30.md` 0 件）
- kind: irreversible
- loss: exported
- 提案: 選択肢 1 の irreversible を「いまは無い。バックアップ（ST30）ができた後は、消すまでの写しがバックアップと拠点外の写しに入る」に直し、C9 の申し送りに書庫の写しを足す
- 処置: escalated —— 推奨の irreversible をバックアップ後の帰結に直し、C9 の申し送りに書庫の写しを足した

## R10. Q4 の推奨は FR-51 の「本文」から書庫の写しを外す読み方で、要件へ戻すことを書いていない
- 分類: 分類違い
- 成果物: openspec/changes/st23-record-erasure/deep-questions.json（Q4）
- 根拠: FR-51「対象の本文を消去し」（`docs/requirements.md:693-694`）。ST12 は写しを「本文の物理削除（FR-51）が写しに届かない」問題として ST23 に渡した（`openspec/changes/archive/2026-10-13-st12-archive-ingestion/deep.md:411-412`）。
  推奨（写しを消さない）は失わない側なので、要件を変えないなら A に置く理由が無く（P）、変えるなら変えることが問いの根拠になる。どちらかが書かれていない
- kind: conflict
- 提案: Q4 の why に「推奨は FR-51 の本文に写しを含めない読み方で、FR-51 に ★ を足す」と書く（A のまま）。書かないなら P に下げて台帳に載せる
- 処置: escalated —— why に「推奨は FR-51 の本文に写しを含めない読み方で、FR-51 に ★ を足す」と書いた（A のまま）

## R11. Q2 の選択肢 2「後から位置も消せる」には、後から消す口が無い
- 分類: 不可逆の記述が誤り
- 成果物: openspec/changes/st23-record-erasure/deep-questions.json:47（Q2 の選択肢 2）
- 根拠: Q1 の推奨は口を「消した」行だけにし、Q5 の draft は本文ごと消した後の行に操作を出さない（`deep-questions.json:147`「本文ごと消した … の行が残り、戻すは出ない」）。位置を 1 件ずつ見る画面は無い（1 日の記録の読み出しは位置を返さない。`openspec/specs/browsing-views/spec.md:305`）
- kind: premise
- 提案: 選択肢 2 の detail に「後から位置を消すときは、本文ごと消した行に『残りの本文を消す』を出す」など口を書くか、irreversible を「後から消す口はこの Story に無い」に直す
- 処置: fixed deep-questions.json —— 選択肢 2 の irreversible に「後から消す口はこの Story に無い」を書いた

## R12. Q1 の context「PC のウィンドウ・ブラウザ履歴は 1 件ずつ見る画面がまだ無い」は誤り
- 分類: 前提が誤り
- 成果物: openspec/changes/st23-record-erasure/deep-questions.json:12（Q1 の context）
- 根拠: ST25（archive 済み）の正典 `openspec/specs/browsing-views/spec.md:298`「日付を指定してその日の記録を 1 件ずつ読み出せる」（PC の前景・ブラウザの訪問・スマホの前景・書庫の活動）/ `:510`「行を開くと、その時間の記録が時刻順に出て、記録 1 件まで届く」。実装は `crates/server/src/day_records.rs:210-298`、画面は `web/src/DayView.tsx:241`（`/api/day/records`）。1 件ずつ見る画面が無いのは写真だけ
- kind: premise
- 提案: context を「PC のウィンドウ・ブラウザ履歴は 1 日の画面（ST25）で 1 件ずつ見えるが、消す口は無い。写真は見る画面が無い」に直す。選択肢に PC とブラウザを消す口を足すかは Q1 の幅として書き足す（どちらも失うものは無い）
- 処置: fixed deep-questions.json —— context を ST25 の 1 日の画面の事実に直した

## R13. Q4 の context の ST12 R23（残さない設定で写しを消すコードが無い）は、ST12 の中で既に直っている
- 分類: 前提が誤り
- 成果物: openspec/changes/st23-record-erasure/deep-questions.json:117（Q4 の context の最終段落）
- 根拠: `crates/server/src/archive/worker.rs:1809-1832`（`keep_copies` が偽のとき、確認待ちのために作った写しを読み直し後に `remove_file`。2026-10-04 の d4b44b7c）。
  `openspec/changes/archive/2026-10-13-st12-archive-ingestion/review/code.md:640-644`（R56「処置: fixed D9」。試験 `archive_flow_pending_copies_are_removed_after_rereading_when_copies_are_off` が `crates/server/src/archive_flow_tests.rs:2220` にある）。ブリーフの申し送り（`docs/handoff/ST23.md` の st12 R23）が古いまま
- kind: premise
- 提案: context から「どの選択肢でも直す」を外し、「R23 は ST12 の R56 で直っている」と書く。`docs/handoff/ST23.md` の該当項に済みの印を付ける（記録の段で）
- 処置: fixed deep-questions.json —— context から R23 の直しを外し、ST12 R56 で直っていると書いた。docs/handoff/ST23.md の済みの印は record の段で付ける

## R14. Q2 の context「DB の門が親と全部の前の版を同じまとまりで消すことを求める」は、滞在には当たらない。C5 にも前の版の強制が無い
- 分類: 前提が誤り
- 成果物: openspec/changes/st23-record-erasure/deep-questions.json:34（Q2 の context の「前の版」）/ defaults.md（C5）
- 根拠: 前の版の強制は「収集した」記録の門の中だけ（`migrations/202609120944_gates.sql:82-84` で `OLD.origin <> 'collected'` は素通し、前の版の検査は `:119-125`）。滞在は `origin = 'derived'` で、前の版を持つ（ST16 R38 の実測 1,443 行）。
  C5 は「台帳の行を要る門を掛ける」だけで、「親を消すなら前の版も同じまとまりで」を書いていない
- kind: premise
- 提案: Q2 の context を「収集した記録は門が前の版も求める。滞在はいまは求めない（C5 で同じ形にする）」に直し、C5 に前の版の検査（ST03 の R40 と同じ）を足す
- 処置: fixed deep-questions.json —— 滞在は前の版をいまは求めないと直し、C5 に前の版の検査を足した

## R15. Q6 の推奨（消した直後に表を書き直す）は、サーバの DB の役割では実行されず、黙って飛ばされる。時間の見積もりも実物で測っていない
- 分類: 前提が誤り
- 成果物: openspec/changes/st23-record-erasure/deep-questions.json:155-156（Q6）/ premises.md:7
- 根拠: サーバは表の所有者でない `ashiato_app` で動く（`crates/server/src/grants.sql:5-9`、所有者の接続は移行だけ `crates/server/src/lib.rs:146`。ST28 design D5）。
  **実測**（使い捨ての `postgres:17-alpine`、所有者でない役割に `SELECT, INSERT, UPDATE, DELETE` だけ付与）: `VACUUM FULL core.event` は `WARNING: permission denied to vacuum "event", skipping it` を出して**成功で返り**、表のファイルは書き直されなかった（`relfilenode` 不変）。
  実行するには役割に `MAINTAIN`（PG17）を足すか所有者で繋ぐ必要があり、ST28 の役割の分け方を変える。premises.md の実測は 1 行の表で、「数十秒〜数分」は測っていない（`core.event` は 1 年で約 1.0 GB の見込み。`openspec/changes/archive/2026-10-17-st33-data-export/deep.md:44`）。
  `ACCESS EXCLUSIVE` は書き出しの長い読み出しの後ろで待ち、待つ間は後続のすべての読み書きが止まる。古いファイルの消去はファイルシステムの上の解放で、ディスクのブロックを上書きしない（画面の「ディスクから追い出しています」は言い過ぎ）
- kind: premise
- 提案: premises.md に役割の実測（上の WARNING）と、実物の量での書き直しの時間・必要な空き容量を足す。Q6 の推奨の detail に「ST28 の役割に MAINTAIN を足す（または所有者の接続で走る別の仕事）」を費用として書き、黙って飛ばされたことを検出する（`relfilenode` が変わったか）を C に足す
- 処置: fixed premises.md —— 役割の実測と、時間を測っていないことを premises.md に足し、Q6 の context に MAINTAIN と書き直しの検出を書いた。推奨を「毎晩」に変えた（直後は待つ時間が測れていない）。why の「追い出す」をファイルシステムの解放と書き分けた

## R16. C10 の理由「書き出しは PC の外の暗号化した zip で、ashiato の手が届かない」は誤り
- 分類: 前提が誤り
- 成果物: openspec/changes/st23-record-erasure/defaults.md:18（C10）
- 根拠: NFR-16「書き出したファイルは本システムが自宅 PC の決まったフォルダにだけ置き」（`docs/requirements.md:1268`）。ST33 は置き場の前の書き出しを台帳で見分けて**自分で消している**（`openspec/changes/archive/2026-10-17-st33-data-export/design.md:96`・`:403` の D16）。
  手が届かないのは、本人が置き場から別の場所へ写したものだけ
- kind: premise
- 提案: C10 の理由を「置き場の書き出しは消せるが、消すと本人の書き出しが失われる（失う側）。消さない側を既定にし、確認に『書き出したファイルには残る』と出す。本人が他へ写したものには届かない」に直す
- 処置: fixed defaults.md C10 —— 置き場の書き出しは消せるが消すと失う側、と理由を直した

## R17. premises.md: 失われるもののある Q3 / Q4 が確かさ 3 の前提に載っている。同じ確かめ方で段が揃っていない。範囲の前提が無い
- 分類: 前提が誤りうる（手順 8）
- 成果物: openspec/changes/st23-record-erasure/premises.md:10-12・:14-15
- 根拠: :10「本文の消去の後も `content_hash` は残り、同じ内容の再送は入らない」は確かさ 3（走らせていない）だが、Q3 の context（`deep-questions.json:88`「同じ内容の再送は、どの選択肢でも入らない」）と Q4 の context（`:117`「読み直しても、消した記録は戻らない」）が載っている。
  外れる場合が辿られていない: 鍵は出来事の時刻を入力に持つ（`crates/server/src/ingest.rs:178-195`）ので、解析器を直して読み直す（写しを残す理由そのもの）で出来事の時刻の読み方が変わる（`archive/timezone.rs`）と、消した項目が新しい鍵で生きた記録として入る。
  :10 の確かめ方は「コード」で 3、:11 と :12 も「コード」で 2 —— 同じ確かめ方で段が違う。:14-15「失われるもののある決定は外の形式に依らない」は、Q3 / Q4 が :10 に依ることと合わない。R1 の「消去の後に範囲が引けるか」は前提の表に無い
- kind: premise
- 提案: :10 を台本で 4 まで上げる（本文を消した行に同じ再送と、時刻だけ違う読み直しを当てて重複になるか）。外れる場合（時刻の読み方が変わった読み直し）を Q4 の context に書く。:10 の段を 2 に揃えるか、3 の根拠（辿った手順）を書く。R1 の前提を実測つきで足す
- 処置: fixed premises.md —— 確かさ 3 の行に辿った手順と外れる場合を書き、末尾の「外の形式には依らない」を、選択肢の失われ方が載る前提（4）と載らない帰結（3）の書き分けに直した。外れる場合（読み直しで鍵が変わる）は Q4 の context に書いた

## R18. C2 が本文を消した滞在に付ける印の値が決まっておらず、作り直しが本文を消した滞在を読み直して書き戻す経路が残る
- 分類: 前提が誤り（C の帰結が既存コードと合わない）
- 成果物: openspec/changes/st23-record-erasure/defaults.md（C2・C5）
- 根拠: Q2 の推奨は吸収された滞在（印 `rebuild:absorbed`）と消した時間帯で隠した滞在（`rebuild:erased-range`）も本文ごと消す。作り直しは印を問わずすべての滞在を読み（`crates/server/src/stay_store.rs:320-345`。範囲と座標を `payload` から読むので、本文を消した滞在は 1µs で座標なしになる）、
  一致した既存の滞在の `raw` を書き戻し、隠す条件でなければ印を外す（`stay_store.rs:971-978` の `update_stay`）。C5 は「作り直しの書き換えは今までどおり通す」なので、本文を消した滞在に新しい本文が入っても門は止めない。
  `rebuild:` の印のまま本文を消すと、本人の削除として読む判定（`stay_store.rs:678-692` の `user_erased_stay_sql`、ST33 D17）からも外れる
- kind: technical
- 提案: C2 に「本文を消した滞在には `rebuild:` でない印を付ける（または台帳の行で本文を消したと分かる）」「作り直しは本文を消した滞在を一致の候補にしない」を足し、C5 の門に「本文を消した行（`raw = ''`）への本文の書き戻しを拒む」を足す

- 処置: fixed defaults.md C2 / C5 —— 本文を消した滞在の印を本人の印へ付け替え、作り直しの一致の候補から外し、門で本文の書き戻しを拒む
