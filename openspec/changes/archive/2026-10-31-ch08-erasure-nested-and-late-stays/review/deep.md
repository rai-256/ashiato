# deep-review — CH08（ch08-erasure-nested-and-late-stays）

検査対象: `deep-questions.json`（Q1〜Q5）・`defaults.md`（C1〜C6）・`premises.md`。
読んだもの: `docs/changes/CH08.md`、`docs/requirements.md` FR-50〜FR-52（:650-735）、`docs/audit/2026-10-24-st23-q8-consistency.md`、
ST23 の deep.md 第 2・3 回 / design.md D2・D6・D12・D13・D17 / spec（record-deletion）/ review/code.md R15・R25、
ST23 のコード（`feat/st23-record-erasure` 7e2396fc。対象の関数は origin の 5ed2c25d と `event_end_sql` の 3 行しか違わない）。

**実行で確かめたこと**（使い捨ての worktree に origin/feat/st23-record-erasure 5ed2c25d を出し、探りの試験を 1 本足して
`cargo test -p ashiato-server --lib ch08_probe` を走らせた。ST23 の st23 DB の回の database を使い、worktree と target は消した。試験の本文は commit していない）:

| 探り | 場面 | 結果 |
|---|---|---|
| A1 | P 10:00–11:00（位置 5 分おき）・B 10:15–10:45。B を消す → P を消す → P を本文ごと消す → B を戻す → 戻した日の作り直し | 戻した滞在 1・位置 0。B は `rebuild:absorbed`・本文あり・生きた滞在 0 |
| A2 | 同じ P・B 10:40–11:05（はみ出しの位置 11:02 / 11:05） | 戻した滞在 1・位置 2。**B は `rebuild:absorbed`・本文あり・生きた滞在 0** |
| A3 | 同じ P・B 10:40–11:30（はみ出しの位置 11:05〜11:30 の 6 件） | 戻した滞在 1・位置 6。B は印なし（一覧に出る） |
| B1 | P を本文ごと消した後、重なる L 10:30–11:30 を `rebuild:absorbed` にし、`purge_late_arrival` で本文を消す（印は変えない・範囲の行なし）→ 15:00–15:30 に位置を入れて作り直す | 15:00 の新しい滞在が `rebuild:erased-range` で隠れ、1 日の一覧に「消した」の行（15:00–15:30）が出る |
| B2 | B1 と同じで、L の時間 10:30–11:30 を範囲の行に書く → 11:10–11:30 に位置を入れて作り直す | 11:10 の新しい滞在が `rebuild:erased-range` で隠れ、「消した」の行（11:10–11:30）が出る |

（ローカルの 7e2396fc は `crates/server/src/testdb.rs:130` と `:563` に `with_database` を 2 つ持ち、試験がコンパイルできない。
ST23 の worktree に未 commit の直しがあるので rebase の途中と読んだ。CH08 の判定「ST23 の既存の試験はそのまま通る」の基点は merge した後の main で見ること）

## 手順ごとの記録

- **手順 1（要件どうしの衝突）**: FR-50 ★ 2026-10-23（:698-702）「本文ごと消した時間帯は『戻す』の対象にならない」と、ST23 Q7 の答え（すっぽり入る B の行に「戻す」を出す）が食い違う → R8。
  FR-52 の確認の件数と Q8 の確認の外の消去は C5 で要件に戻り済み（Q4 が文面を受ける）。ST23 spec の「端が触れるだけは重なる滞在に含めない」（spec.md:148）と C1 は、
  整合の確認 C2 が「別の操作の規則で食い違わない」とした（leave）。ただし C1 は失う側を既定にしている → R5
- **手順 2（扉の幅）**: CH08 は doors を持たない。Q8 の「重なる」「隠れたら」の幅（整合の確認 C2・C3）は C1・C2 が受けた。
  C2 の「P の時点で既にあった」は、P の前に隠れていた端の滞在を区別しない → R4
- **手順 3（新たに立つ一方通行）**: Q8 と手当ての本文の消去（discarded）。消した後に L の時間を取れるのは範囲の行だけ（本文は空・`payload` は `{}`）で、
  `purge_late_arrival` は範囲の行を書かない（`crates/server/src/deletion.rs:1073`）→ R3（uncaptured）。手当ての対象の決め方 → R4（discarded）
- **手順 4（日常に影響する選択）**: Q7 の行の出し方（C5）・「後から N 件」（Q3）・確認の文面（Q4）。L の印の選び方しだいで、消していない滞在が毎日の一覧に「消した」として出る（探り B1・B2）→ R3。
  入れ子の B の行の兄弟 → R6
- **手順 5（既存コードが要件を満たしていない箇所）**: Q7・Q8 が作られていないこと自体は R15 が実行で確かめ済み。
  加えて、ST23 にすでにある一部重なりの「消した」行の「戻す」（`web/src/DayView.tsx:413`）は、はみ出しが短いと B を吸収させる（探り A2）。Q8 が入るとこの「戻す」は確認なしの本文の消去になる → R1
- **手順 6（過去の取りこぼし）**: 「X が出る条件を区間の境界（一部重なり・すっぽり入る・端が触れる）ごとに確かめる」（ST23 Q7）→ R1・R6。
  「その間に走る仕事（作り直し・後着・夜の書き直し）を通しても X が残るか」（ST23 Q8）→ R3（作り直しが L の窓を読み直す）。
  「時刻を入力にして状態を一方向に動かす決定」（ST02 Q29 ほか）と ST23 R18 / R22（`now()` はトランザクションの始まり）→ R4。
  「後処理で安全性を保証する設計では、後処理が落ちた場合を上流で決める」（ST22 Q6）→ R9。ほかの観点（検証コマンド・外の形式・時計の進み・写しへの削除）は CH08 に当たる箇所が無い
- **手順 7（族の網羅）**: Q2（入れ子の B を戻す）の兄弟 —— はみ出しが短い B・吸収される B → R1。Q7（すっぽり入る B）の兄弟 —— 複数の P の和に覆われる B・日の境で空になる B・B を本文ごと消した後 → R6。
  Q8 の兄弟 —— 消した後の L の印と窓 → R3、対象の見分け方（P の前に隠れた端の滞在・作り直しで隣から重なりに変わる滞在・初めから隠して作る滞在）→ R4
- **手順 8（前提の確かさ）**: premises.md は Q2（`loss: discarded`）の前提を確かさ 3（走らせていない）で置いている。入れ子は探り A1 で成り立ち、一部重なりの主張は探り A2 で崩れた → R7

## R1. 「一部だけ重なる B を戻すときは問題が無い」が誤り。はみ出しが短い B は戻すと吸収され、Q8 で本文が確認なしに消える
- 成果物: openspec/changes/ch08-erasure-nested-and-late-stays/deep-questions.json
- 根拠: Q2 の context「一部だけ重なる B を戻すときは、はみ出した分の位置が戻って B は一覧に出るので、この問いには入りません」、選択肢 1 の detail「一部だけ重なる B の行はいまのまま（『戻す』も出る）」。
  探り A2: B 10:40–11:05 を戻すと、はみ出しの位置 2 件は戻るが 5 分しかなく（`min_minutes` の既定 10。`crates/server/src/stay.rs:29`）新しい滞在ができず、`assign` が B を吸収先なしで吸収する（`crates/server/src/stay_store.rs:557-571`）。B は `rebuild:absorbed`・本文あり。
  はみ出しで新しい滞在ができても、代表点が半径の 2 倍より離れていれば引き継がず B は吸収される（`stay_store.rs:531-536`）。
  この「戻す」は ST23 の一覧にすでにある（`web/src/DayView.tsx:413`。一部重なりの B は差し引いた断片として行が残る）ので、Q7 の新しい行に限らない
- kind: premise
- loss: discarded
- 提案: Q2 の対象を「すっぽり入る B」から「戻しても一覧に戻らない B（入れ子・はみ出しが滞在にならない・吸収される）」に広げ、一部重なりの行の「戻す」も同じ選択肢で扱う。
  context の「一部だけ重なる B は入らない」の文を消し、探り A2 の結果を書く
- 処置: escalated —— 問いを直して本人に返した（deep.md の「問いの検査の指摘の返し先」）。deep-questions.json Q2 —— 対象を「戻しても一覧に戻らない B」に広げ、探り A2 を context に書いた。一部だけ重なる B の「戻す」も同じ選択肢で扱う

## R2. Q2 の推奨（「戻す」を出さない）は画面だけで、`POST /stays/restore` は入れ子の B を受け付けたまま —— 失わない側が成り立っていない
- 成果物: openspec/changes/ch08-erasure-nested-and-late-stays/deep-questions.json
- 根拠: `crates/server/src/deletion.rs:167` は「本文を消した記録で、別の原因で消えたままでないもの」がある時だけ 409 にする。入れ子の B の位置は本文が消えているが別の原因（P）で塞がれる（`blocked_by_other_cause`）ので数えられず、B は戻る（探り A1: 戻した滞在 1）。
  戻す口の直後に作り直しが走る（`crates/server/src/lib.rs:2142`）ので、Q8 が入ると口を叩いた時点で B は吸収され本文が消える。選択肢 1 の detail は画面のボタンしか書いていない
- kind: premise
- loss: discarded
- 提案: 選択肢 1 に「サーバも、戻しても一覧に戻らない B の戻す求めを 409 で断る（ST23 の『本文を消した記録は戻せず』の Requirement の断り方を広げる）」を書き、
  ST23 の振る舞いを変える箇所として Q1 の範囲（R10）に載せる。断り方の判定（R1 の吸収されるか）を戻す口で前もって決められるかは design に送る
- 処置: escalated —— 問いを直して本人に返した（deep.md の「問いの検査の指摘の返し先」）。deep-questions.json Q2 —— 推奨を「画面も口も戻さない（戻す口も断る）」にした。断り方の判定は design へ

## R3. Q8 で本文を消した L の印と範囲の行が決まっていない。印を変えずに後着の処理を使うと、その日の消していない滞在が「消した」に隠れる
- 成果物: openspec/changes/ch08-erasure-nested-and-late-stays/defaults.md
- 根拠: `core.stay_erased` は `user:purged` 以外の消えた滞在を全部出し、範囲の行が無い本文を消した滞在はその日全体を範囲にする（`migrations/202610240500_record_purge.sql:87-89` `:111`）。
  作り直しは本文を消した滞在を印を問わず「本人が消した時間帯」として足す（`crates/server/src/stay_store.rs:347` は `purged` だけで絞り `deleted_by` を見ない・`:381`）。
  `purge_late_arrival` は印を変えず範囲の行を書かない（`deletion.rs:1073`）、`purge_late_body` は `s01-stay` を位置の組に数える（`stay_store.rs:1006`）。
  探り B1: L を `rebuild:absorbed` のまま本文を消すと、4 時間離れた 15:00–15:30 の新しい滞在が `rebuild:erased-range` で隠れ、一覧に戻せない「消した」の行が出た。
  探り B2: L の時間を範囲の行に書くと、P の外で L の中の 11:10–11:30 の滞在が隠れた（ST23 D13「本文ごと消した時間帯は指定した滞在の時間だけ」に反する）。
  `rebuild:erased-range` のまま本文を消すと、`day_view` は L を別の「本文ごと消した」の行として出す（`stay_store.rs:1538` は吸収だけを除く、`:1581`）。
  一方、L の時間と前の版の時間は本文を消すと本文から読めなくなる（ST23 は一緒に消した滞在の時間を範囲の行に残す。spec「本文を消した滞在の時間が範囲の行に残る」）。整合の確認の「未確認」3 つめがこの印の問い
- kind: technical
- loss: uncaptured
- 提案: defaults に C7 を足す —— L は ST23 D6 と同じ `user:purged` に付け替えて窓から外し（削除の台帳に原因 = P の行）、L と前の版の時間は `kind = late` の操作の範囲の行に残し、操作の行は `stays = 1` で数える。
  探り B1・B2 の結果を理由に書く（範囲の行を書かないと、L の時間は後から取れない）
- 処置: escalated —— 問いを直して本人に返した（deep.md の「問いの検査の指摘の返し先」）。defaults.md C7 —— L を `user:purged` に付け替え、L と前の版の時間を `kind = late` の操作の範囲の行に残し、`stays = 1` で数える

## R4. Q8 と手当ての対象の見分け方が、P の前に隠れていた端の滞在と、隣から重なりに変わる滞在を区別しない
- 成果物: openspec/changes/ch08-erasure-nested-and-late-stays/defaults.md
- 根拠: C2「P を本文ごと消した時点で既にあった滞在が後で隠れたものだけ。見分け方（行の作られた時刻と操作の時刻を比べる、など）は design」。Q5 の対象「P と重なり、`rebuild:absorbed` か `rebuild:erased-range` で隠れて本文が残っている滞在（C1・C2 の範囲）」。
  (a) P の前から作り直しの印で隠れていて、端が触れるだけで P の範囲から外れた滞在は、ST23 が残すと決めている（試験 `purge_scope_edge_touching_rebuild_stay_is_not_included`、`crates/server/src/purge_tests.rs:1122`。確認の件数に入れない）。この行は「P の時点で既にあった」「P と重なる（C1 で端を含む）」「隠れて本文が残る」を満たすので、手当てが確認なしに消す。
  (b) 作り直しは、隣にあって P と重ならなかった一覧の滞在を、P をまたぐ新しい滞在に引き継いで書き換えてから隠しうる（`stay_store.rs:627` → `update_stay`）。書き換えた後の時間で「重なる」を見ると、P の座標を持ったことの無い滞在と前の版を消す。
  (c) 時刻の比べ方は、`now()` がトランザクションの始まりなので錠を待った順と食い違う（ST23 code.md R18 / R22 で夜の書き直しの窓が同じ形で崩れた）。
  作り直しの中の判定には時刻が要らない（作り直しの前に一覧に出ていた行を、この作り直しが隠したか）。初めから隠して作る行は `insert_stay` が `deleted_at` を作った時と同じ `now()` にする（`stay_store.rs:1228`、`ingest_time` の既定 `now()`）
- kind: technical
- loss: discarded
- 提案: C2 を「この作り直しが隠した、作り直しの前に一覧に出ていた行で、作り直しの前の時間が P と重なるもの」に書き直す。
  Q5 の対象に「P の本文ごと消した操作の後に隠れたもの（P の前に隠れていた端の滞在は入れない）」を足し、その見分けに使う列と、時刻で比べるなら錠の順との食い違いを context に書く
- 処置: escalated —— 問いを直して本人に返した（deep.md の「問いの検査の指摘の返し先」）。defaults.md C2・deep-questions.json Q5 —— 見分け方を「作り直しの前に一覧に出ていた・作り直しの前の時間が P と重なる・この作り直しが隠した」にし、時刻で比べない。Q5 の対象から P の前に隠れていた端の滞在を外した

## R5. C1 は失う側（端が触れるだけの L の本文と前の版を消す）を、聞かずに既定にしている
- 成果物: openspec/changes/ch08-erasure-nested-and-late-stays/defaults.md
- 根拠: C1「端が触れるだけも含む」。ST23 は本文ごと消す時点の範囲で端が触れるだけを外した（spec.md:148、試験 `purge_tests.rs:1122` は端の隠れた滞在の本文を残すことを固定）。
  C1 の後は、同じ端の滞在でも P の前に隠れていれば残り、後に隠れれば確認なしに消える。消えるのは L の本文と前の版（それまでの基準で作った滞在の履歴）で、本文を消した行は作り直しの候補から外れる（`stay_store.rs:333` `raw <> ''`）ので戻らない。
  失わない側（含めない）の費用は「端の位置の時刻が一致したときだけ、その座標が L の本文に残る」（整合の確認 C2）で、高くつかず範囲を越えず要件の本文も変えない
- kind: irreversible
- loss: discarded
- 提案: C1 を A の問いにするか、既定を失わない側（ST23 の範囲と同じく端が触れるだけは含めない）に倒して理由を書き直す。どちらでも、R4 の (a) と同じ規則で手当ての対象を揃える
- 処置: escalated —— 問いを直して本人に返した（deep.md の「問いの検査の指摘の返し先」）。defaults.md C1 —— 既定を失わない側（端が触れるだけは含めない。ST23 の範囲と同じ）に倒した

## R6. Q7 の兄弟が欠けている。複数の P に覆われる B・日の境で空になる B・B を本文ごと消した後の行の出方
- 成果物: openspec/changes/ch08-erasure-nested-and-late-stays/defaults.md
- 根拠: C5「P の『本文ごと消した』の行のすぐ下に…B の行を出す」は P が 1 つのときしか書いていない。`day_view` は「消した」の断片から本文ごと消した区間をすべて差し引く（`crates/server/src/stay_store.rs:1633` の `subtract_spans(start, end, &purged_spans)`）ので、
  10:00–11:00 と 11:00–12:00 の 2 つの P に跨る B 10:30–11:30 も、日の境で切った後に P に覆われる B も空になる。どちらの P の下に出すかが無い。
  B を本文ごと消すと B は `user`・本文なし・範囲の行ありになり、`day_view` は B を P と重なる別の「本文ごと消した」の行として出す（`stay_store.rs:1571-1590`）—— P の下に入れた行との関係が決まっていない
- kind: daily
- 提案: C5 の条件を「差し引いて空になる B」にし、置き場（重なりがいちばん大きい P の下、など）と、B を本文ごと消した後に B の行がどうなるか（P の下に残す・別の行にする）を足す。行の構造の判断なので GO の proto で絵にする
- 処置: fixed D6 仮 —— defaults.md C5 を spec の段が design D6（仮）と反転条件にした。—— 空になる B の置き場（重なりがいちばん大きい P の下）・日の境・本文ごと消した後の行を足した。CH08 は ui-direction の画面を持たない（`story_brief.py CH08 --screens` が空）ので相談の proto は作らず、GO 資料の段で例として示す

## R7. premises.md の確かさが 3 のまま、Q2（`loss: discarded`）の前提と一部重なりの主張を載せている
- 成果物: openspec/changes/ch08-erasure-nested-and-late-stays/premises.md
- 根拠: premises.md「確かさ 3 —— 1 歩ずつ辿った。走らせてはいない」。上の探り A1 で入れ子の前提は成り立ち、A2 で一部重なりの主張（Q2 の context）は崩れた。
  premises.md が挙げる b0268f81 はローカルの `feat/st23-record-erasure`（7e2396fc）の祖先でない（rebase 前。origin の 5ed2c25d の祖先）
- kind: premise
- 提案: premises.md に探り A1〜A3（場面・結果）を載せて入れ子の行を確かさ 4 にし、一部重なりの行を「はみ出しが滞在にならなければ吸収される」に直す。辿った commit をローカルの先端か origin の 5ed2c25d に替える
- 処置: fixed premises.md —— 探り A1〜B2 を確かさ 4 として載せ、辿った commit を origin の 5ed2c25d に替えた

## R8. Q2 は Q7 の答えと FR-50 の本文の衝突なのに、context に FR-50 を引いていない
- 成果物: openspec/changes/ch08-erasure-nested-and-late-stays/deep-questions.json
- 根拠: `docs/requirements.md:698-699` FR-50 ★ 2026-10-23「（戻せない。本文ごと消した時間帯は『戻す』の対象にならない）」。Q7 の答え（ST23 deep.md 第 2 回）は B の行に「戻す」を出すとした。
  Q2 は「いまのコードでは」の前提としてだけ書かれ、本人の決定と要件の文がぶつかっていることが見えない
- kind: conflict
- loss: discarded
- 提案: Q2 の kind を conflict にし（loss はそのまま）、context に FR-50 の一文を引く。選択肢 1 が要件の文の側、選択肢 2 が Q7 の答えの字面の側であることを書く
- 処置: escalated —— 問いを直して本人に返した（deep.md の「問いの検査の指摘の返し先」）。deep-questions.json Q2 —— kind を conflict にし（loss はそのまま）、FR-50 の一文と、どの選択肢がどちらの側かを context に書いた

## R9. Q5 の選択肢に、手当てが落ちたときに次にいつ消し直すかが無い
- 成果物: openspec/changes/ch08-erasure-nested-and-late-stays/deep-questions.json
- 根拠: Q5 の推奨「サーバの起動のたびに…探して消す」。起動の時の 1 回が落ちる（錠の待ち・門の拒否）と、次の起動まで本文が残る。
  ST22 Q6（後着の印を作り直しの成否に依存させ、落ちた日が再試行されなかった）と ST23 D9（落ちたら見回りで消し直す）は、後処理が落ちた場合を上流で決めている
- kind: technical
- 提案: Q5 の各選択肢の detail に「落ちたら次はいつ消し直すか（次の起動・見回り・その日の作り直し）と、落ちたことがどこに出るか」を足す。起動の時に落ちたらサーバの起動を止めるかも書く
- 処置: fixed deep-questions.json Q5 —— 各選択肢に、落ちたら次にいつ消し直すかと、起動を止めるかを書いた

## R10. Q1 の推奨「ST23 のほかの形は変えない」は、Q2・Q8 が必ず変える ST23 の振る舞いを挙げていない。R10・R23 を足す選択肢は依頼の判定と食い違う
- 成果物: openspec/changes/ch08-erasure-nested-and-late-stays/deep-questions.json
- 根拠: Q2 の推奨を守るには戻す口の断り方が変わる（R2。`deletion.rs:167`）。Q8 は `rebuild_day` が本文を消すようにし（ST23 D17 の「入れない」の後に、隠れたら消す）、L の印を `user:purged` に付け替えれば `core.stay_erased`・ST33 の「後に消した記録」の数え方に入る（ST23 design D6「`user:purged` は ST33 の D17 には削除の印として入る」）。
  C1 は ST23 の範囲の規則（spec.md:148）と別の規則を Q8 に置く。どれも Q1 の 2 つの選択肢に出てこない。
  一方、選択肢 2（R10・R23）は ST23 の確認の範囲の Scenario を書き直すが、`docs/changes/CH08.md` の完了の判定は「ST23 の既存の Scenario と試験はそのまま通る」
- kind: technical
- 提案: Q1 の推奨の文を「Q7・Q8・手当てと、それが必ず変える ST23 の振る舞い（戻す口の断り方・作り直しの本文の消去・L の印）だけ」に直して一覧を context に書く。
  選択肢 2 を残すなら、依頼の完了の判定を変えることになると detail に書く
- 処置: fixed deep-questions.json Q1 —— 必ず変える ST23 の振る舞い（戻す口・作り直しの本文の消去・L の印・一覧）を context に挙げ、選択肢 2 は完了の判定を変えると書いた
