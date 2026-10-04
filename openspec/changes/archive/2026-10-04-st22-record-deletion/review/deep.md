# ST22 深掘りの独立レビュー（deep-questions.json / deep.md / proto.html）

schema の手順 1〜5 を自分でやり直してから、問いの一覧（Q1〜Q4）と C1〜C10 に突き合わせた。**問いの JSON・deep.md・proto は触っていない。**

確かめた範囲: `docs/requirements.md`（FR-8 / FR-50〜FR-52 / NFR-7 / NFR-13 / NFR-17〜NFR-23）、`docs/stories/ST22.md` `INDEX.md` `stories.json`、
`docs/handoff/ST22.md`、`docs/ui-direction.md`、`docs/production-prep.md` A-3、`openspec/specs/record-envelope/spec.md`、
`openspec/changes/st16-stay-derivation/{deep.md,design.md,specs/*}`、`openspec/changes/archive/2026-09-14-st02-collection-coverage/design.md` D34、
`openspec/changes/st04-offline-retention/{deep.md,tasks.md}`、並走中の上流 `../ashiato2-up-st12`（st12-archive-ingestion/deep.md）と `../ashiato2-up-st05`、
`migrations/*.sql`（envelope / event_columns / dedup_indexes / version_and_ledger / gates / stays）、
`crates/server/src/{lib.rs,ingest.rs,stay_store.rs,coverage.rs,stay_tests.rs,dedup_tests.rs,coverage/tests/states.rs}`、
`collector-android/.../LocationFix.kt`、`scripts/board.py` の出力、proto.html の JS。

**実行した確認**（HEAD を `/tmp` に展開し、使い捨ての PostgreSQL 17 に全移行を当て、テストを 2 本足して `cargo test st22_probe` を実行。作業ツリーは変えていない。終わった後に container と展開先は消した）:

- 探針 A（`stay_tests` の形）: 10:00〜11:00 と 11:01〜11:31 に滞在 → 作り直し → 1 件目を `deleted_by='user'` で消す → `day_view`
  - 滞在だけ消す: `NoRecord 00:00–10:00 / (10:00–11:00 は空白) / Move 11:00–11:01 / Stay 11:01–11:31 / …`
  - さらに 10:00〜11:00 の位置も消して作り直す（Q1 の推奨の形）: `NoRecord 00:00–11:01 / Stay 11:01–11:31 / …` —— **消した時間が「記録なし」の行に入る**
- 探針 B（`dedup_tests` の形。`external_id_kind='record'`）: `e1` に `{"v":1}` → `{"v":2}` へ更新 → 消す → `e2` で `{"v":1}` を送る
  - `accepted=true rows=2 live=1` —— **前の版と同じ内容が、生きた記録として入る**（C5 の前提は正しい）
  - 消した後に `e1` の `{"v":3}` を送る: `accepted=true`、`{"v":3}` は `core.event` にも `core.event_version` にも **0 件**（捨てられている）

手順ごとの結果:

- 手順 1（要件どうしの衝突）: R2（FR-50 と ST16 の「記録なし」の定義）/ R4（FR-50「取り込まない」と record-envelope「取り消しは残る」）/ R12（FR-50 と NFR-13 の達成日数）
- 手順 2（扉の幅）: `doors: []`。扉ではないが決定済みの文面で幅が残るもの —— FR-50「取り込まない」（捨てるか、印を付けて置くか）R4 / 「同じ内容」の範囲（論理ソースをまたぐか）R6
- 手順 3（新たに立つ一方通行）: R1（削除と取り消しの判断を上書きの列 2 本で持つ）/ R3（C5 が捨てる範囲を広げる）/ R4（削除中の更新を捨てる）/ R5（Q1 の選択肢 2 と書き出し）
- 手順 4（日常に影響する選択）: R6（遅れて届く記録の漏れ）/ R12（稼働状況と成功の定義）/ R13（行ごとの 44 px）。常時通知・電池・端末の容量には新しい判断は無い（端末側のコードは変わらない。受理を返すので未送信にも残らない —— `Sender.kt` は `accepted` を見て取り除く）
- 手順 5（既存コードが要件を満たしていない箇所）: R2（探針 A）/ R3（探針 B）/ R4（探針 B）/ R8 / R10

C のうち**指摘しなかったもの**: C1・C6・C7（申し送り R14 / R44 / R37 の写しで、コードと一致した）、C9（record-envelope の正典どおり）、
C8（ST16 の作り直しは、本人が消した滞在の `[start, end]` から隠す範囲を毎回計算し直す（design D4 / `stay_store.rs` の D4 の実装）ので、掛け方を変えても作り直せば戻る。申し送り R19 の「消す操作ができる前に」は必要以上に強いが、問い直す必要は無い）。

---

## R1. 削除と取り消しの判断を、上書きされる列 2 本だけで持とうとしている。取り消し（Q4 の軸 3）や C4 で、本人がした判断の記録が消える
- 分類: 抜け
- 成果物: openspec/changes/st22-record-deletion/deep.md（C2 / C4）・proto.html（軸 3「直後 10 秒だけ戻せる」「いつでも戻せる」）
- 根拠: migrations/202609081618_envelope.sql:33-34（`deleted_at` / `deleted_by` の 2 列だけ）/ migrations/202609120944_gates.sql:93（削除の列だけの書き換えは門を素通し = 何度でも書き換えられる）/ openspec/changes/st16-stay-derivation/deep.md:41-44（ST16 Q2 は「**本人が消したという判断だけは再計算できない**」として A / discarded にした）/ docs/handoff/ST22.md:20（戻す操作は `rebuild:erased-range` も外す必要がある）/ proto.html の `restore()`（戻すと印が消えるだけ）
- kind: technical
- loss: uncaptured
- 提案: 「消す・戻す」の操作を 1 回 1 行で追記のみの台帳に残す（対象・操作・時刻・原因になった滞在）を C に足す（既定「台帳は追記のみ」）。これが無いまま取り消しを作ると、取り消した削除の時刻と「一度消した」事実が残らない。C4 で 2 回目の削除（カスケードの後の個別の削除）が記録されず、Q1 の答えを戻したときにその位置まで生き返る。C2 の「`deleted_by` に識別子を埋める」もこの台帳に置き換えられる。C にしないなら A で問う。
- 処置: escalated — 問いにはしない（持つ側は何も失わず費用が小さい = 「台帳は追記のみ」の既定）。deep.md の C2 を追記のみの削除の台帳に直し、R1 つきで記録。本人には C の一覧として見せ、異論は番号で受ける

## R2. Q1 の推奨（位置も消す）を選ぶと、消した時間は 1 日の一覧で「空白」ではなく「記録なし」の行になる。Q4 の軸 4「空白（いまの形）」と、proto の「Q1 は確認の文面にだけ効く」は事実と違う
- 分類: premise
- 成果物: openspec/changes/st22-record-deletion/deep-questions.json（Q1 / Q4 の context）・proto.html（軸 4 と「入力」の注記）
- 根拠: 探針 A の実行結果（上記。位置を消すと `NoRecord 00:00–11:01` になる）/ crates/server/src/stay_store.rs:960-991（記録なしは `event_live` の位置の間隔だけで作る）と :1007（隠した時間 `hidden` を足すのは記録なしを作った後、移動を埋めるときだけ）/ openspec/changes/st16-stay-derivation/specs/browsing-views/spec.md:70-78（「記録なし」は「居なかったのか取れていなかったのか」を分けるための行）/ docs/handoff/ST22.md:41（R35 は位置が残っている前提で「空白」と書いている）
- kind: premise
- 提案: Q4 の context と proto の軸 4 に「Q1 で位置も消すと、いまの実装ではその時間が『記録なし』になる」を書き、proto は「入力」の答えで描き分ける（「記録なし」に吸われる形を描く）。軸 4 の選択肢は「消した時間を記録なしと区別するか」まで含めて描く。Q1 の選択肢 1・3 の detail にも同じことを書く。
- 処置: fixed deep-questions.json — Q1 / Q4 の context と Q1 の選択肢に「位置も消すと記録なしに入る」を書き、proto.html の軸 4 と「入力」を描き分けた（deep.md の前提にも追記）

## R3. C5 は「扉を開けたままにする既定」ではない。判定を前の版に広げると、別の外部識別子で届いた記録を捨てる範囲が広がる
- 分類: 分類違い
- 成果物: openspec/changes/st22-record-deletion/deep.md（C5）
- 根拠: 探針 B（いまは前の版の内容が `rows=2 live=1` で生きて入る）/ crates/server/src/lib.rs:474-490（判定は `core.event.content_hash` だけを見る。前の版は見ない）/ lib.rs:524-526（止めた記録は挿入しない。外部識別子もどこにも残らない）/ openspec/specs/record-envelope/spec.md:398-403（「格納しない」は ST03 の正典の文言）/ CLAUDE.md の C の既定「捨てるより印を付けて入れる」
- kind: conflict
- loss: discarded
- 提案: C5 から外す。選択肢を「前の版と同じ内容は捨てる（正典の形を前の版へ広げる）」「削除済みの印を付けて入れる（record-envelope の MODIFIED が要る）」「いまのまま（生きて入る）」で A として問うか、C の既定どおり「印を付けて入れる」を採って record-envelope を触ることを R9 の capability に書く。「前の版の本文は履歴に残っているので失われない」は本文だけの話で、新しく届いた外部識別子と届いた事実は残らない。
- 処置: escalated — C5 から外し、deep.md の「本人に見せるが問わないもの」に R3 として記録（外部識別子で更新が届くソースはいま無く、ST22 の操作からは届かない）。docs/handoff/ST12.md へ申し送り

## R4. 消していた間に届いた外部サービスの更新は捨てられる。ST22 が消す操作と（軸 3 の）取り消しを作ると、戻した記録にその更新が無い。問いにも C にも無い
- 分類: 抜け
- 成果物: openspec/changes/st22-record-deletion/deep-questions.json（該当なし）・deep.md（C3 / C9）
- 根拠: 探針 B（消した後の `{"v":3}` は `accepted=true` で、どの表にも 0 件）/ crates/server/src/lib.rs:769-773（`SkippedDeleted` で返り、書くのはログだけ）/ openspec/specs/record-envelope/spec.md:410-415（「本人が自分で行う論理削除の取り消しは残る」を利点として書いている）/ docs/requirements.md:387-389（FR-50「取り込まない」—— 生きた記録に反映しないのか、どこにも置かないのか、文面には幅がある）/ 受理を返すので端末側も未送信から消す（spec.md:422-425）
- kind: conflict
- loss: discarded
- 提案: A として立てる（「削除中の更新は捨てる（いまのまま）」/「削除済みの記録の履歴に印付きで積み、生きた記録には反映しない」）。いま消せるのが滞在と位置だけ（どちらも外部の更新が来ない）なら、C3 で口を外部識別子のソース（`external_id_kind='record'`）に開けないことで、この問いを ST12 / ST13 が入るまで遅らせられる。その場合は R11 と合わせて C3 を書き直す。
- 処置: escalated — deep.md の「本人に見せるが問わないもの」に R4 として記録（R3 と同じ理由で ST22 の操作からは届かない。C3 で口を狭めた）。docs/handoff/ST12.md へ申し送り

## R5. Q1 を B にした根拠「外へ出す経路はまだ無い」は盤面と合わない。書き出し（ST33）はいま着手可で、選択肢 2 を選んでから答えを変えると、その間に書き出した位置は戻らない
- 分類: 分類違い
- 成果物: openspec/changes/st22-record-deletion/deep-questions.json（Q1 の why / context）・deep.md（Q1「なぜ仮でよいか」）
- 根拠: `python3 scripts/board.py` の出力「[着手可] ST33 全記録を外部の道具で読める形に書き出す data-durability」（record-deletion と capability が重ならないので並走できる）/ Q1 context 自身が「滞在だけを消す側を選んだまま書き出しが先にできると、その時間の位置は…出る」と書いている / 付け直しの根拠 C2 は、C4（2 回目の削除を記録しない）と組むと、カスケードの後に本人が個別に消した位置を区別できない（R1）
- kind: conflict
- loss: exported
- 提案: 選択肢 2（滞在だけ）に「ST33 / ST27 が先にできると、その時間の位置は外へ出る（戻らない）」と書き、Q1 を A（loss: exported）に上げる。B のままにするなら、反転条件として「ST33 の上流が始まる前に確定する」を design に書き、`docs/handoff/ST33.md` に申し送る。R1 の台帳を入れない限り「付け直せる」は言えない。
- 処置: escalated — Q1 を A（kind irreversible / loss exported）に上げ、選択肢 2 に書き出しで外に出ることを書いた。deep.md Q1 に R5

## R6. Q2 の推奨を選んでも、消した場面の位置は 2 つの経路で生きた記録に戻る。選択肢の説明「遅れて届いた分も隠れたまま」は言い過ぎ
- 分類: 抜け
- 成果物: openspec/changes/st22-record-deletion/deep-questions.json（Q2 の options[0].detail / context）
- 根拠:
  - (a) 滞在の終わりは消した時点までに届いた位置で決まる。後から届いた位置が消した滞在の `[start, end]` の外側を延ばすと、作り直した滞在は Q12 で丸ごと隠れる（openspec/changes/st16-stay-derivation/design.md:136-137）が、**範囲の外の位置は印が付かない**（Q2 の規則は「その時間帯」だけ）
  - (b) 同じ場面が別の論理ソースで入る。内容の鍵は論理ソースを含む（crates/server/src/ingest.rs:170-193）ので FR-50 の判定に当たらない。並走中の ST12 の上流はマップのタイムラインを `c03-timeline-*` として入れる（../ashiato2-up-st12/openspec/changes/st12-archive-ingestion/deep.md:144-148）
  - (c) Q1 で「全ソース」を選ぶと、PC の未送信（上限なし。docs/requirements.md:571-573）も遅れて届くが、Q2 は位置しか問うていない
- kind: daily
- 提案: 選択肢 1 の detail を「消した滞在の時間帯に入る位置だけ隠れる。時間帯の外側へ延びた分と、書庫から入る別のソースは隠れない」に直す。「隠れた滞在（`rebuild:erased-range`）を作った位置にも印を付ける」を選択肢として足すか検討する。(b) は `docs/handoff/ST12.md` に申し送る。Q2 の対象を「Q1 で消すことにしたソース」に揃える。
- 処置: escalated — Q2 の説明と context を直し（隠れるのは時間帯に入る分だけ / 対象は Q1 で消すソース）、本人に問う。(b) は docs/handoff/ST12.md へ。deep.md Q2 に R6

## R7. Q2 の context「外出先で滞在を消すと、その時間の位置はまだ端末にあることが多い」には根拠が無い
- 分類: premise
- 成果物: openspec/changes/st22-record-deletion/deep-questions.json（Q2 の context）
- 根拠: collector-android/app/src/main/kotlin/dev/ashiato/collector/LocationFix.kt:16（`SEND_INTERVAL_MS = 300_000`）—— 画面を開けるなら同じサーバの `/ingest` にも届くので、未送信は普通 5 分ぶんまで / docs/requirements.md:95-96（FR-8 は「送れていない記録」「端末に積んでから 90 日」に訂正済み。context の「自宅 PC に届かない間」は訂正前の文面）
- kind: premise
- 提案: 起こりやすいのは「端末が圏外か送れない間に、PC の画面でその日の滞在を消す」だと書き直す（FR-8 の現行の文面で）。問いの重さ自体は変わらない。
- 処置: fixed deep-questions.json — Q2 の context を FR-8 の現行の文面と「端末が送れない間に PC で消す」に直した

## R8. deep.md の「読み出しは全部 `core.event_live`、`core.event` を読むのは稼働状況だけ」は誤り。1 日の一覧も削除済みの滞在を素のテーブルから読んでいる
- 分類: premise
- 成果物: openspec/changes/st22-record-deletion/deep.md（前提として確かめたこと 2 項目め）・deep-questions.json（Q1 の context）
- 根拠: crates/server/src/stay_store.rs:936-940（`day_view` が `FROM core.event … deleted_at IS NOT NULL` で隠した滞在を引く）/ stay_store.rs:319-320・:786（作り直しも `core.event` を読む）/ docs/production-prep.md:73（A-3「素のテーブルを直接引かせない」）/ `GET /stays` は位置の点を返さない（stay_store.rs:861-867 の応答は date / criteria / entries だけ。「滞在と点」の「点」は応答に無い）/ Q1 context の「位置の記録を消すと、その日の滞在は作り直され」は ST22 が作る動き（申し送り R37）で、いまのコードには無い
- kind: premise
- 提案: 前提の記述を直す。Q4 の軸 4「行を残す」と軸 3「いつでも戻せる」は削除済みの滞在の時間と識別子を画面に返す読み出しを増やすことになるので、その読み出しを専用のビューに寄せるか（A-3 の形）を design に書く。Q1 の context は「ST22 で作る」と時制を直す。
- 処置: fixed deep.md — 前提の記述を直し（素の core.event を読むのは 2 か所）、Q1 の context の時制を「ST22 で作る」に直した。A-3 のビューは design で扱う

## R9. ST22 が触る capability が盤面の 1 本（record-deletion）より多い。並走中の Story と重なっても `board.py` が検出できない
- 分類: 他 Story
- 成果物: docs/stories/INDEX.md（capability の表）・openspec/changes/st22-record-deletion/deep.md
- 根拠: `python3 scripts/board.py` —— ST22 は `record-deletion` だけ。実際に触る先:
  - `browsing-views`: Q4（ST16 の Scenario「消した滞在と吸収された滞在は一覧に出ない」openspec/changes/st16-stay-derivation/specs/browsing-views/spec.md:53-56 を変える選択肢がある）。ST16 は archive 待ちで `openspec/specs/browsing-views` がまだ無く、ST25 はこれで衝突待ち
  - `derived-records`: 滞在の削除・C6・C7（ST16 archive 待ち）
  - `record-envelope`: C5 と Q2 の推奨（取り込みの時点で印を付ける）。**ST05 の上流が同じ capability で走っている**（board「[上流] ST05 record-envelope」）
  - `collection-coverage`: Q3 で選択肢 2 を選んだとき（ST04 の下流が走っている。Q3 の context 自身が書いている）
- kind: technical
- 提案: INDEX.md に過去の訂正と同じ形で「ST22 を browsing-views / derived-records にも割り当てる。record-envelope は C5 / Q2 の答え次第」を足す。deep.md に「どの答えがどの capability を足すか」を書き、ST05 と record-envelope の順を決める。
- 処置: fixed deep.md — 「答えが触る capability」の表を足し、record-deletion に ADDED で置く方針を書いた。INDEX.md との照合は Step 3

## R10. C10「利用者はクエリで指定」を ID で消す操作にそのまま使うと、錠（R44）と作り直し（R37）が別の利用者に掛かる
- 分類: 技術の既定
- 成果物: openspec/changes/st22-record-deletion/deep.md（C10）
- 根拠: crates/server/src/lib.rs:1148（`/stays` は `q.user_id.unwrap_or_default()` —— 省くと nil UUID）/ docs/handoff/ST22.md:31-35（錠の鍵は `hashtext(<user_id>)`）/ :23-27（`rebuild_day(pool, user, day)`）
- kind: technical
- 提案: C10 を「利用者は消す行の `user_id` から取る。クエリで渡された利用者と違えば断る」に直す。クエリの値をそのまま使うと、錠が別の鍵になり作り直しも別の利用者の日に当たるが、どちらも失敗としては見えない。
- 処置: fixed deep.md — C10 を「利用者は消す行の user_id から取り、クエリと違えば断る」に直した

## R11. C3「どのソースの記録でも消せる口」は扉を開けたままにする側ではない。狭い口は後から広げられるが、広い口はソースごとの副作用を今決めてしまう
- 分類: 技術の既定
- 成果物: openspec/changes/st22-record-deletion/deep.md（C3）
- 根拠: 消した後に要る処理がソースで違う —— 滞在は R14 / R44（docs/handoff/ST22.md:5-12, 31-37）、位置は R37（:23-29）、外部識別子のソースは更新を捨てる（R4、lib.rs:769-773）/ 資格情報は収集側と共有の 1 本（crates/server/src/lib.rs:194-215）なので、収集側の不具合でも消す口を叩ける / 既に `rebuild:erased-range` で隠れた滞在を口から消すと、C4 で本人の削除が記録されない
- kind: technical
- 提案: C3 を「画面が消すもの（滞在）と、そのカスケード（Q1 の答え）だけを受ける。他のソースを消す口は、それを見せる画面を作る Story（ST25 など）が足す」に直す。広い口のままにするなら、ソースごとの副作用と R4 を design に列挙する。
- 処置: fixed deep.md — C3 を「滞在とその連鎖だけを受ける口」に狭めた

## R12. Q3 の選択肢 2 に、成功の定義（NFR-13 の達成日数）が落ちることが書かれていない。ST02 がこの形にした理由は人間の決定ではなく design の判断
- 分類: 文面
- 成果物: openspec/changes/st22-record-deletion/deep-questions.json（Q3 の why / options[1].detail）
- 根拠: crates/server/src/coverage/tests/states.rs:858-861（「`core.event_live` から引くと過去の稼働状況が遡って⑥へ変わり、**成功条件 1 の達成日数が落ちる**」）/ openspec/changes/archive/2026-09-14-st02-collection-coverage/design.md:702-704（D34。深掘りの問いではない）/ docs/requirements.md:23-25（成功の定義 1「1 年間データが途切れずに」）
- kind: daily
- 提案: 選択肢 2 の detail に「消した日は達成日数（成功の定義 1）からも落ちる。消すと成果が下がるので消すのをためらう」を足す。why の「ST02 が意図して」は「ST02 の design D34 が（人間に問わずに）」と出所を正す。B のままでよい。
- 処置: escalated — Q3 の選択肢 2 に達成日数が落ちることを足し、why の出所を ST02 design D34 に直した。B のまま本人に問う。deep.md Q3 に R12

## R13. Q4 と proto が「動かせない」とした「削除のボタンは 44×44 px（NFR-20）」の出所が違う。NFR-20 は論理削除を含まず、行ごとに 44 px を置くことを理由付きで避けている
- 分類: premise
- 成果物: openspec/changes/st22-record-deletion/deep-questions.json（Q4 の context）・proto.html（「動かせないもの」と軸 1「行の右端に常に置く」）
- 根拠: docs/requirements.md:717-721（NFR-20 の対象は「FR-51 の本文の物理削除、FR-53 の収集の停止」。理由に「全行に 44 px を課すと 1 画面に収まらなくなる」）/ docs/ui-direction.md:118（「削除・停止だけ 44×44px」—— 44 px の出所はこちら）
- kind: premise
- 提案: 出所を ui-direction に直す。軸 1 の選択肢 2 に「行ごとに 44 px のボタンが並ぶ（NFR-20 が避けた形）」と書き、proto の「1 画面に見える行」の計測を並べて見せる。proto のそのほかの下限（NFR-17 の明暗の追従、NFR-19 の 24 px、NFR-22 の `:focus-visible`、消した行を文字「消した」で区別すること）は、どの軸を選んでも満たしていた。
- 処置: fixed deep-questions.json — Q4 の context の出所を ui-direction に直し、proto.html の「動かせないもの」と軸 1 の選択肢 2 に NFR-20 が避けた形であることを書いた
