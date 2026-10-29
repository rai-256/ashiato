# spec-review — CH08（ch08-erasure-nested-and-late-stays）

検査対象: `proposal.md` / `specs/record-deletion/spec.md` / `specs/browsing-views/spec.md` / `design.md` / `tasks.md` / `docs/changes/CH08.md`。
決定の正本: `deep.md`（Q1〜Q5）・`defaults.md`（C1〜C7）・`premises.md`。突き合わせたコード: main の ST23 の後（merge-base 71c76e81）の
`crates/server/src/stay_store.rs`・`deletion.rs`・`lib.rs`・`stay.rs`、`migrations/202610240500_record_purge.sql`、`web/e2e/st23-purge.spec.ts`、`web/src/stays.ts`。

## 機械の検査

| コマンド | 結果 |
|---|---|
| `openspec validate ch08-erasure-nested-and-late-stays --strict` | valid（rc=0） |
| `python3 scripts/check_chain.py .` | chain: OK（rc=0） |
| `python3 scripts/check_scenarios.py . ch08-erasure-nested-and-late-stays` | FAIL: 担保なし 40 件（この change が足した 40 本すべて。実装前なので想定どおり）。名無しの確認待ち 0（rc=1） |
| `python3 scripts/review_triage.py . ch08-erasure-nested-and-late-stays` | FAIL 13 件。すべて `review/deep.md` の処置（R1〜R5・R8 の `loss` 付きを `fixed` で閉じている / R6 の `kind=daily` を仮にしていない）。spec の成果物の指摘ではない（rc=1） |
| `python3 scripts/spec_overlap.py candidates . ch08-erasure-nested-and-late-stays` | `[]`（rc=0） |

MODIFIED の写しの照合（正典 `openspec/specs/*/spec.md` と Requirement ごとに突き合わせ。`$TMPDIR` の使い捨ての照合で確かめた）:
正典の Scenario は 8 つの Requirement のどれからも落ちていない。本文を書き換えた既存の Scenario は「後から届いて本文を消した件数が行に出る」の THEN 1 本だけ（tasks の Global Constraints の記述どおり）。
足した Scenario は record-deletion の MODIFIED 25・ADDED 6・browsing-views の MODIFIED 9 の計 40 本で、tasks の 40 本と一致し、tasks はその 40 本をそれぞれちょうど 1 回名指している
（1.1: 11 / 2.1: 6 / 3.1: 5 / 4.1: 2 / 5.1: 3 / 6.1: 13）。

---

## R1. CH08.md に「## 完了の判定」が 2 つあり、機械が読む先頭の節が深掘りの答え（Q2・Q5）と逆のことを言っている
- 成果物: docs/changes/CH08.md
- 根拠: `docs/changes/CH08.md:36-43` に依頼の写しの「## 完了の判定」が残っている（`grep -c '^## 完了の判定' docs/changes/CH08.md` = 2。CH01〜CH07 はどれも 1）。
  その本文は「そこから B を戻す・本文ごと消すができる（e2e）」（:38。Q2 の答え「すっぽり入る B の行には『本文ごと消す』だけ」と逆）と
  「移行か作り直しの経路で消え」（:40。Q5 の答え「サーバの起動のたび」と逆）。末尾に `change_new.py` の雛形の注記「（`hx change` の依頼の逐語。…1〜3 文に整える）」（:43）も残っている。
  spec の段が書いた正しい節は :89 の 2 つめ。`scripts/issue_body.py:47` の `section()` は `re.search` で**先頭の一致**を取り（`done = section(story, r"完了の判定")` :128）、
  `scripts/stories_json.py:29` も先頭の `## 完了の判定` を取るので、下流の issue と確認バッチに古い判定が出る。:18 の「## 依頼」・:29 の「## なぜ」も同じ写しの残り
- kind: conflict
- 提案: :18〜:43 の写し（「## 依頼」「## なぜ」「## 完了の判定」と雛形の注記）を消し、「## 価値」の直後を「## 依頼（逐語）」にする。依頼の文は :45 以降の逐語の引用に残っている
- 処置: rejected: kind は conflict ではない —— 判断の衝突ではなく、`hx change` の依頼の写しが残っていた（`docs/changes/CH08.md` の旧 :36-43。答えとの食い違いは写しの文面だけ）。写しを消して直した: 依頼の写しの「## 依頼」「## なぜ」「## 完了の判定」と雛形の注記を消し、「## 完了の判定」を spec の段が書いた 1 つだけにした

## R2. 起動時の手当て（D5）が、P の時点で本人が消していて後から戻されて吸収された B の本文を、確認なしに消す（Q2 の答えに反する）
- 成果物: openspec/changes/ch08-erasure-nested-and-late-stays/design.md
- 根拠: 対象は design D5 と specs/record-deletion/spec.md の ADDED。D5 の条件は `deleted_by IN ('rebuild:absorbed', 'rebuild:erased-range')`・`raw <> ''`・いまの時間が P と重なる・`R.ingest_time < P.at`・`R.deleted_at >= P.at`（design.md:188-192）。
  ST23 のいまのコードで、P を本文ごと消した後に B（P と重なる本人が消した滞在）を戻すと、`deletion::restore` が B の印を `NULL` に戻し（`deletion.rs:189-192`）、
  commit の後の作り直しで `absorb` が `deleted_at = now()`・`rebuild:absorbed` を付ける（`stay_store.rs:1258-1260`）。探り A1・A2 はちょうどこの状態（`rebuild:absorbed`・本文あり）を作っている（premises.md:119-120）。
  B は P より前からある（`ingest_time < P.at`）、隠れたのは P の後（`deleted_at >= P.at`）、時間は P と重なるので、D5 の 5 条件をすべて満たし本文が消える。
  一部だけ重なる B の行の「戻す」は ST23 の一覧にすでにある（`web/src/DayView.tsx:413`。review/deep.md R1）ので、ST23 の merge から CH08 の merge までに本人が押せばこの状態ができる。
  Q2 の答えの帰結は「B の本文を消すのは、本人が確認を見て『本文ごと消す』を押したときだけ」（deep.md:28）、ADDED の Requirement も対象を「その本文ごと消す操作の時点では一覧に出ていて」（spec.md:421）に限っているが、
  D5 の表（design.md:201-206）にこの場合が無く、ADDED の Scenario にも無い
- kind: conflict
- 提案: D5 の条件に「P の操作の後に、R 自身の戻す（`core.deletion_ledger` の `action = 'restore'`）が無い」を足す。順は時刻でなく `deletion_ledger.txid` と `erasure_operation.txid`（どちらも xid8）で取れる ——
  ただしそれは、戻す口（`deletion.rs:114-120`）も本文ごと消す口（`stay_row_and_lock`。`deletion.rs:856`）も、最初の書き込みより前に錠を取る間だけ成り立つ（実装でこの前提を確かめる）。
  これは戻された B を外すだけで、`core.stay_absorbed` に txid が無いので D5 の「P の後に隠れた」の時刻の比べ（R3）は残る。ADDED に Scenario「本文ごと消した時点で本人が消していて、その後に戻されて吸収された滞在は、起動しても本文が残る」を足し、D5 の表に行を足す
- 処置: fixed D5 仮 —— kind を conflict に直した（loss が無く、失わない側を選べる）。条件 3（`core.deletion_ledger` に R の行が無い）を足し、本人が消したことのある滞在を外した（失わない側）。ADDED の本文と Scenario「本文ごと消した後に戻されて作り直しで隠れた消した滞在は起動しても本文が残る」を足した。txid の順に頼らず、行の有無だけで見る

## R3. D5 が deep.md Q5 の帰結（「時刻で比べない」）を覆しているのに deep.md に残っておらず、覆した先の 1 行が失う側（消す側）を仮で選んでいる
- 成果物: openspec/changes/ch08-erasure-nested-and-late-stays/design.md
- 根拠: 対象は design D5 と deep.md の Q5。deep.md:57「P の後に隠れたかは、削除の台帳と吸収の表（`core.stay_absorbed`）の順で見分け、時刻で比べない」。本人に見せた Q5 の context も同じ（`deep-questions.json` Q5「時刻で比べると、錠を待った順と食い違うことがある」）。
  design.md:198-199 は「時刻を比べる理由」を書いて時刻で比べているが、deep.md の「当初案を覆したもの」（:79-84）に載っていない。
  さらに表の最後の行（design.md:206）は「**消す側**」を（仮）で受け入れている（:210）。消えるのは R のいまの本文と前の版すべてで、前の版は P の前の端の滞在の本文（本人が残すと決めた側。deep.md:56 `purge_scope_edge_touching_rebuild_stay_is_not_included`）。
  仮で進めてよいのは失わない側だけ（CLAUDE.md「失わない側が高くつかず…なら、AI が仮で進む」）。反転条件に書いた「前の版のどれかの時間が P と重なることも条件に足す」（:212）が失わない側で、費用は条件 1 つ
- kind: conflict
- 提案: 最後の行は反転条件の側（前の版の時間でも P と重なることを条件に足す）を初めから採る。時刻で比べることは deep.md の「当初案を覆したもの」に理由とともに書き足すか、R2 の txid の比べ方で「時刻で比べない」を守る
- 処置: fixed D5 仮 —— 時刻の比べをやめ（deep.md Q5 の帰結どおり）、条件を「いまの時間が重なる・削除の台帳に行が無い・ingest_time と deleted_at が等しくない」にした。消す側に倒れる行は無くなり、外れる行はどれも残す側（表に来歴ごとに書いた）

## R4. D5 の表に、P の後に「消した時間帯と重なる」として書き換わって隠れた滞在（残す側）が無く、ADDED の「時間が重なり」がどの時刻の時間かを言っていない
- 成果物: openspec/changes/ch08-erasure-nested-and-late-stays/design.md
- 根拠: 対象は design D5 と specs/record-deletion/spec.md の ADDED。ST23 の作り直しが引き継いで隠すとき、`update_stay` は R の本文と `event_time` を新しい滞在 `f` のものに書き換え、古い本文を `push_version` で前の版に積む（`stay_store.rs:641-644`・`:1189-1195`）。
  `f` は生きた位置だけから作るので P の時間を含まず（R5 の根拠）、書き換えた後の R のいまの時間は P と重ならない。D5 は「R のいまの時間」で重なりを見る（design.md:191）ので、
  P の座標を持つ前の版を残したまま R を見つけない。D2 は作り直しの中で「作り直しの前の時間」を見る（design.md:140）ので、手当てと作り直しで見る時間が違う。
  ADDED の本文（spec.md:421）は「本文ごと消した時間帯と時間が重なり」とだけ書き、どの時点の時間かを言っていない
- kind: technical
- 提案: D5 の表に「P の後に引き継がれて隠れ、書き換わった R → 残す側（前の版に P の座標が残る）」の行を足し、Risks に移すか、前の版の時間でも重なりを見る（R3 の反転条件と同じ条件で両方を拾える）。
  ADDED の本文に「本文ごと消す操作の時点の時間（前の版の時間を含む）が重なる」のように、見る時間を書く
- 処置: fixed D5 仮・specs/record-deletion ADDED —— 引き継いで隠れ書き換わった L は残す側として表と Risks に書き、前の版の時間を見る形は反転条件（`event_version` に txid を足してから）に置いた。ADDED の本文に「いまの時間（作り直しの印で隠れたときの時間）」と書いた

## R5. 「作り直しで消した時間帯と重なるとして隠れた重なる滞在の本文も消える」の場面は作れない（新しい滞在は本文ごと消した時間帯に触れられない）
- 成果物: openspec/changes/ch08-erasure-nested-and-late-stays/specs/record-deletion/spec.md
- 根拠: design.md の D2 の (b) も同じ前提。Scenario（spec.md:232-235）は「10:50 から 11:40 の新しい滞在が消した時間帯に触れて隠れる」。`rebuild_day` は位置を読む前に `mark_late_arrivals` で、本文ごと消した P の時間帯 `[start_at, end_at]`（両端を含む）に入る生きた位置へ `user:late` を付ける
  （`stay_store.rs:604-611`・`:938-955` の `e.event_time <= s.end_at AND end >= s.start_at`）。`load_points` は `core.event_live` を読む（`:182-186`）ので、10:50 の位置は判定に入らない。
  滞在の始まりと終わりは位置の時刻（`stay.rs:146-147`）なので、新しい滞在の始まりは P の終わりより後になり、`touches_erased`（`:577-579`。両端を含む）は P について真にならない。
  `f` が隠れるのは、別の本人が消した時間帯をまたぐとき（間隔が `gap_minutes` の既定 10 分（`stay.rs:30`）に満たない短い時間帯）だけ。
  D2 の (b) の理由「`f` の隠れた行（消した時間帯の『消した』の行になる時間）が次の作り直しまで無くなる」（design.md:144-145）も、P に触れる `f` を前提にしている
- kind: technical
- 提案: 場面を作れる形に書き直す —— 例: P 10:00–11:00 と重なる一覧に出ている L 10:30–11:40、本文を消していない消した滞在 B2 11:20–11:25（10 分未満）、位置 11:05〜11:40（11:19 と 11:26 の間隔が 10 分未満）で作り直すと、`f` 11:05–11:40 が L を引き継ぎ B2 に触れて隠れる。
  THEN の「10:50 から 11:40 の滞在の本文は残っている」も新しい時刻に合わせる。作れる場面が無いと判断するなら、(b) の分岐と Scenario を落とし、D2 にその理由を書く
- 処置: fixed D2 —— specs/record-deletion の Scenario と Scenario を作れる場面（10 分未満の消した滞在 B2 をまたぐ新しい滞在）に書き直し、D2 に (b) が起きる場面と理由を書いた

## R6. 既存の server の試験 2 本も書き換えになり、「書き直す既存の試験は e2e 1 本だけ」が成り立たない
- 成果物: openspec/changes/ch08-erasure-nested-and-late-stays/design.md
- 根拠: 対象は design D1、tasks.md 7.1、docs/changes/CH08.md の完了の判定。
  (1) `crates/server/src/stay_tests.rs:2187-2197`（`stay_day_view_empty_day_is_all_no_record`）が `DayEntry { … late_purged: None }` を全項目の構造体リテラルで書いている。
  D6 が `DayEntry` に `nested: Option<Vec<NestedEntry>>` を足す（design.md:216）と、この試験はコンパイルできず書き換えが要る。
  (2) `crates/server/src/purge_late_tests.rs:466-535`（`purge_late_archive_row_hidden_by_another_stay_keeps_that_stay_restorable`。ST23 の /code-review R24）は、A 10:00–11:00（`scene_basic`。`purge_tests.rs:841-854`）を本文ごと消した後に、
  A にすっぽり入る B 10:15–10:25 を `deletion::restore` で戻し `is_ok()` を assert する（:530-531）。Q2 と D4 では B は同じまとまりの作り直しで吸収されるので `NotRestorable` になり、この試験は落ちる（Q2 が変える振る舞いだが、D1 の列挙に無い）。
  D1（design.md:132-133）・tasks 7.1（「書き直した ST23 の試験が `st23-purge.spec.ts` の 1 本だけであることを PR の本文に書く」）・CH08.md:102（「文面の変わる e2e 1 本を除いてそのまま通る」）はこれを数えていない
- kind: technical
- 提案: D1 と tasks 7.1・CH08.md の完了の判定に、`stay_tests.rs` の 1 本（フィールドを足すだけ）と `purge_late_tests.rs` の 1 本（B を一部だけ重なる・戻せる形にするか、R24 の主張を残す場面に作り直す）を書き足し、
  後者を直すタスクを Task 3 に置く
- 処置: fixed D1 —— tasks 3.1・5.1・7.1・CH08.md も合わせ、書き直す既存の試験を 3 本（e2e 1・`stay_tests.rs` 1・`purge_late_tests.rs` 1）として表にし、直す Task を置いた

## R7. 2.1 の `grep -q "purge_hidden_overlap_remedy" lib.rs` は、ログの種別の文字列だけで通り、`run()` から呼ばれていることを確かめない。起動が止まらないことも `run()` を通して確かめていない
- 成果物: openspec/changes/ch08-erasure-nested-and-late-stays/tasks.md
- 根拠: tasks.md:63 の検証は名前の grep。同じ 2.1 の本文（tasks.md:56）と design.md:195 は、`warn` の `kind = "purge_hidden_overlap_remedy"` を lib.rs に書くと決めているので、関数を呼ばなくても grep は rc=0 になる。
  Scenario「手当てが失敗しても起動は止まらず、次の起動で消える」（spec.md:455-458）の「起動は止まらず」は `run()` の振る舞いだが、試験は「`run()` が呼ぶのと同じ関数を呼ぶ」（tasks.md:59）なので、
  `run()` が `?` で伝えて起動を止める書き方でも緑になる
- kind: technical
- 提案: 起動時の処理を `run()` から呼ぶ 1 つの関数（例: 失敗を飲み込んで件数をログに出すところまで含めた関数）に切り出し、試験はそれを呼ぶ。
  grep は呼び出しの形（例: `deletion::purge_hidden_overlap_remedy(&pool)`）にするか、ログの種別を関数名と別の文字列にする
- 処置: fixed 2.1 —— design D5 も合わせ、起動時の処理を失敗を飲み込む `purge_hidden_overlap_remedy(&pool) -> usize` 1 つにまとめ、試験はそれを呼ぶ。ログの種別を `purge_remedy` にし、grep を呼び出しの形 `deletion::purge_hidden_overlap_remedy(&pool)` にした

## R8. Requirement の本文だけが言っていて Scenario が言っていないことが 2 つある
- 成果物: openspec/changes/ch08-erasure-nested-and-late-stays/specs/browsing-views/spec.md
- 根拠: 2 は specs/record-deletion/spec.md の ADDED。
  1. browsing-views「1 日の並びを読み出せる」の「その滞在の後から本文を消した件数は親の区間の件数に足す」（browsing-views/spec.md:12）。入れ子の本文ごと消した滞在の Scenario（:45-48）は印だけを見て件数を見ない。
     design D6 の 2（design.md:223）が足すと決めているが、件数を足さなくても全 Scenario は緑
  2. ADDED の「消せずに残った件数をログに出し（記録の識別子・時刻・本文は出さない）」（record-deletion/spec.md:425）。Scenario（:455-458）は件数 1 が出ることだけを見て、識別子・時刻を出さないことを見ない
- kind: technical
- 提案: 1 は Scenario を足す（例: 入れ子の滞在を本文ごと消した後にその時間帯へ位置が 1 件届くと、親の区間の後から本文を消した件数が 1 増え、別の区間ができない）。
  2 は既存の Scenario の THEN に「ログの行に滞在の識別子と時刻が無い」を別の Scenario として足すか、`tools/check-log-private.sh` が拾う形であることを tasks に書く
- 処置: fixed 5.1 —— Scenario「入れ子の滞在の後から本文を消した件数は親の区間に足される」（browsing-views）と「起動の手当てのログは件数だけを持つ」（record-deletion ADDED。tasks 2.1）を足した

## R9. design にだけある観測できる振る舞いがあり、1 つは spec と食い違う
- 成果物: openspec/changes/ch08-erasure-nested-and-late-stays/design.md
- 根拠: 対象は design D4・D6・D7 と specs/browsing-views/spec.md。
  1. D6 の 4（design.md:225-226）は「本文ごと消した B はまとめない」「端が触れるものもまとめる」。spec は「重なる入れ子の滞在は 1 つの項目にまとめ、識別子をすべて持つ」（browsing-views/spec.md:10）・
     「重なるものは 1 つの行にまとめる」（:78）で、本文ごと消した入れ子の例外も、端が触れるものを含むことも言っていない（spec のとおりに作ると、本文ごと消した入れ子 2 つが 1 行にまとまる）
  2. D6 の 2「互いに覆う（時間が同じ）ときは、操作の `at` の早い方を親」（design.md:222-223）—— どちらが「本文ごと消した」の行に出るかが変わる
  3. D7「消す口は `visible_overlaps` を比べない」（design.md:235）—— 確認の後に一覧に出ている重なる滞在が増えても 409 にならない
  4. D4 の同じまとまりの作り直しが落ちたとき（`RangeUnsettled`・`MarkChanged`）の戻す口の応答。いまは作り直しが落ちても 200（`lib.rs:2147-2157` はログだけ）だが、D4 では戻すごと巻き戻る。
     この経路は `StayRebuilder`（`lib.rs:537-557`。試験が差し替える口）も通らない
  `openspec archive` は main specs しか更新しないので、これらは正典から落ちる
- kind: technical
- 提案: 1 は spec の文を D6 に合わせる（または D6 を spec に合わせる）。2〜4 は Requirement に 1 文ずつ写し、3・4 は Scenario も足す（4 は「作り直しが落ちると戻す求めは何も戻さず 5xx」など、どちらに倒すかを書く）
- 処置: fixed D6 仮 —— design D4・D7 と specs も合わせた。1: spec を D6 に合わせた（本文を消していないものは重なるか隣り合うものをまとめ、本文ごと消したものはまとめない。Scenario を足した）。2: 時間が同じときの親を spec と Scenario に写した（D6 は txid の順）。3: 確認の口の Requirement に「消す口は比べない」と Scenario を足した。4: D4 の 4 に 503 を決め、Requirement と Scenario を足した

## R10. 正典の「本文ごと消す範囲は…」が「一覧に出ている滞在の本文を、時間が重なっていても消さない」と無条件に言ったまま残る
- 成果物: openspec/changes/ch08-erasure-nested-and-late-stays/specs/record-deletion/spec.md
- 根拠: `openspec/specs/record-deletion/spec.md:480`「THE SYSTEM SHALL 本人が別の操作で消した滞在と、削除の印の無い滞在（一覧に出ている滞在）の本文を、時間が重なっていても消さない。」は
  「WHEN 本人が消した滞在を本文ごと消す」の下にあるが、文自体は時点を限っていない。CH08 は同じ滞在を、後で作り直しで隠れたときに消す（MODIFIED「本文を消した記録は戻せず、作り直しも戻さない」spec.md:201-202）。
  proposal.md:67 はこの Requirement を「変えない」とし、MODIFIED に入れていない。archive の後、正典に 2 つの文が並ぶ
- kind: technical
- 提案: この Requirement も MODIFIED で写し、該当の文を「この操作では消さない（作り直しで隠れた後は『本文を消した記録は戻せず、作り直しも戻さない』に従う）」と時点を限る。Scenario は変えない
- 処置: fixed proposal.md —— proposal の MODIFIED の一覧に足し、specs/record-deletion で 「本文ごと消す範囲は…」を MODIFIED で写し、該当の文を「この操作では」に限って作り直しの規則へ送った（Scenario は変えない）

## R11. 7.2 は「起動の手当てを通した縦串」と言うが、縦串に CH08 の節が無く、手当ても入れ子も通らない
- 成果物: openspec/changes/ch08-erasure-nested-and-late-stays/tasks.md
- 根拠: tasks.md:123-124 の検証は `tools/smoke.sh` rc=0 だけ。CH08 は `tools/smoke.d/` にファイルを足さない（tasks.md:7-8 の触るファイルに無い。`ls tools/smoke.d` に ST23 の `202610251200_st23-purge.sh` はある）。
  smoke.sh は CH08 の前でも rc=0 になるので、この項目はコードが無くてもチェックできる
- kind: technical
- 提案: 7.2 の文言を「既存の縦串が緑のまま」に直すか、`tools/smoke.d/<日時>_ch08-…sh` を足して起動の手当て（ログの件数）と入れ子の項目を 1 つずつ外から見る

---
- 処置: fixed 7.2 —— 文言を「既存の縦串が緑のまま」に直した（CH08 は縦串の節を足さない。振る舞いは Task 1〜6 の試験が担保する）

## 観点ごとの記録

- **観点 1（deep の決定が正典に写っているか）**: Q1 → CH08.md「壊してはいけないもの」・tasks の Global Constraints。Q2 → MODIFIED「消したものを戻せる」の 409 と 5 本の Scenario・「「消した」の行から戻せる」の 2 本。
  Q3 → 「後から N 件も本文ごと消した」（THEN の書き換え 1 本と新しい 2 本）。Q4 → MODIFIED 2 つの Requirement の文と 4 本の Scenario（文面の逐語あり）。Q5 → ADDED の Requirement と 6 本。
  C1〜C7 → D2〜D9（対応表は tasks.md:24）。数値は 44 px（2 本）と時刻の場面だけで、deep に間隔・上限の数値は無い。指摘は R2・R3（Q2・Q5 の帰結と D5）、R1（CH08.md の古い判定）
- **観点 2（検証可能か）**: R5（作れない場面）・R7（起動を通らない）・R8（本文だけの主張）。THEN に主張が複数ある Scenario は tasks.md:17 が 1 つずつ assert すると書いている
- **観点 3（置き場）**: R9・R10。specs に新しく入った実装の名前は無い（`core.erasure_ledger` は正典からの写し :280）。proposal の Capabilities・specs のディレクトリ・CH08.md の `capability` は record-deletion / browsing-views で一致。change なので INDEX.md の表には載らない
- **観点 4（tasks）**: R6・R7・R11。各項目にコマンドと rc の条件があり、`cargo test` の絞り込み（`purge_hidden_` `purge_remedy_` `purge_restore_` `purge_visible_` `purge_nested_` `purge_` `stay_` `deletion_tests` `purge_api_` `day_`）は
  パッケージ名 `ashiato-server`（`crates/server/Cargo.toml:2`）と既存の試験の名前・モジュール（`deletion_tests`・`purge_api_tests.rs`・`purge_day_tests.rs`）に当たる。新しい前置き 5 つは既存の試験と重ならない（`grep -rn "purge_hidden_\|purge_remedy_\|purge_restore_\|purge_visible_\|purge_nested_" crates/server/src` が 0 件）。
  `-- --list` の下限は Scenario の数（11・6・5・2）と 5.1 の 6（3 本 + 日の境・跨る・まとめ）に合う。`tools/check-shared-appends.sh`・`src/bin/openapi.rs`・`web/e2e/day-erase.spec.ts` は在る。
  依存順: 3.1 の `rebuild_day_in` の切り出しは 1.1 の `rebuild_day` の書き換えの後で、5.1 は 1.1 の `kind = 'late'` の行に依る —— 順に並んでいる。人間の確認待ちは無し（画面は e2e）
- **観点 5（change の文書）**: R1（最優先）。「壊してはいけないもの」は Scenario になっている（端が触れるだけ: record-deletion「端が触れるだけの滞在は作り直しで隠れても本文が残る」ほか / 一覧に出ている L: 「一覧に出ている重なる滞在は隠れるまで本文が残る」/ 戻せる B: 「はみ出しが滞在になる消した滞在は戻せる」）。
  依頼（逐語）の範囲は越えていない（D4 の戻す口の断りは Q2 の決定による）
観点 6: 該当なし（確かめた範囲: 呼び出しは `spec --answers` ではない）

観点 7: 該当なし（spec_overlap.py candidates が 0 件）

### 独立に確かめたこと（指摘にならなかったもの）

- **D2 とコードの流れ**: `load_existing` は本文を消した滞在を `raw = ''` で候補から外し、`core.stay_erased_purge` の `purged` の時間を `Mark::UserDeleted` で足す（`stay_store.rs:329-384`）。`assign` の吸収は `Live | ErasedRange` だけ（:560）、引き継いで隠す経路は `update_stay(hide)`（:632-644）。
  D2 の 1（`Mark::Live`）・2（作り直しの前の時間）・3(a)（`plan.absorbed`）は、この流れの `assign` の後・書き込みの前で判定できる。`PurgedStay` に `operation_id` を足すのも、ビューが `operation_id` を持つ（record_purge.sql:86-113）ので成り立つ。
  `user:purged` はビューから外れる（同 :113）ので、D3 の「L の時間は窓を広げない」も成り立つ。派生の門（同 :129-160）は前の版を先に消すことと同じまとまりの `erasure_ledger` の行を求め、D3 の順（前の版 → 記録）と合う。問題は (b) の前提だけ（R5）
- **D4 の成立**: `restore` は最初の SQL で `pg_advisory_xact_lock(LOCK_KEY, hashtext(user_id::text))` を取り（`deletion.rs:114-120`）、`stay_store::lock` も同じ鍵（`stay_store.rs:169-176`）。助言錠は同じセッションで取り直せるので、`rebuild_day_in(tx, …)` を同じトランザクションで呼べる。
  巻き戻せば D2 が戻した B の本文を消しても残らない。探り A1〜A3 の場面（はみ出し 2 件 = 3 分 < `min_minutes` 10、6 件 = 25 分）は spec の 3 本の Scenario の時刻と合う。R9 の 4（作り直しが落ちたときの応答）だけが決まっていない
- **D6 と Scenario の時刻**: 跨る B 10:40–11:30 は P1 10:00–11:00 と 20 分・P2 11:00–12:00 と 30 分で P2 の下（browsing-views:144-147・record-deletion:322-325 の両方と合う）。
  日の境 23:30–翌 00:30 は翌日の `d0` で 00:00–00:30 に切られ、P 00:00–01:00 を差し引くと空になる（`day_view` の `start.max(d0)`・`subtract_spans`。`stay_store.rs:1602-1633`）。
  まとめ 10:10–10:30 と 10:20–10:40 → 10:10–10:40、並び 10:05–10:20 → 10:30–10:50 も成り立つ。入れ子の B は API の `s01-stay`（`origin: "derived"`）の取り込みで作れる（`web/e2e/st23-purge.spec.ts:190-210`）
- **D5 のずれの向き**: 表の 1〜3 行目の向き（残す側）は合っている（`insert_stay` は `ingest_time` と `deleted_at` が同じ `now()`、`update_stay` は `coalesce(deleted_at, now())`）。抜けは R2（消す側）と R4（残す側）
