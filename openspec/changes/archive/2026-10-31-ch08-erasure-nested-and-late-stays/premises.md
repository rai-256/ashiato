# CH08 の前提

外の形式に依る決定なし（他サービスの書き出し・OS や端末の振る舞いに依らない。触るのは ST23 の自前のコードと DB の行だけ）。

問いの前提にした、ST23 のいまのコード（`origin/feat/st23-record-erasure` の 5ed2c25d）の振る舞い。どれも外の形式ではないので確かさの表には載せず、
辿った場所と、走らせて確かめたものを残す。Q2（`loss: discarded`）の前提は確かさ 4（下の探り。deep-review が使い捨ての worktree と DB で試験を足して走らせた。`review/deep.md` の冒頭の表）。

| 振る舞い | 辿った場所 |
|---|---|
| P を消すと、P の時間にある、既に B で消えていた位置にも原因 P の台帳の行が足される。P を本文ごと消すと、その位置の本文も消える | `crates/server/src/deletion.rs` の `erase_using_action`（「既に別の消去原因で隠れている位置にも、この操作の原因を追記する」）・`purge_scope` の 2 |
| B を戻すと、本文を消した位置は別の原因（P）の最新の erase に塞がれて戻らない。B の滞在の行だけが戻り、その日の作り直しが走る | `deletion.rs` の `restore`（`blocked_by_other_cause`・`rebuild_days`） |
| 作り直しは、生きた位置を読む前に、消した時間帯（本文ごと消した P を含む）に入る生きた位置へ後着の印を付ける。新しく作る滞在は生きた位置だけから作る | `crates/server/src/stay_store.rs` の `rebuild_day`（`mark_late_arrivals` → `load_points`） |
| 合う新しい滞在が無い一覧の滞在は `rebuild:absorbed` で隠れる（吸収先なしでも） | `stay_store.rs` の `assign`（`absorbed`）・`absorb` |
| 一覧の滞在 L が作り直しで新しい本文に書き換わるときは、古い本文が前の版に積まれる | `stay_store.rs` の `rebuild_day`（`push_version` → `update_stay`） |
| 1 日の一覧は「本文ごと消した」の区間を「消した」の区間から差し引き、空になった「消した」の行は出さない | `stay_store.rs` の `day_view`（`subtract_spans`） |

## 走らせて確かめたもの（確かさ 4。`review/deep.md` の探り）

| 探り | 場面 | 結果 |
|---|---|---|
| A1 | P 10:00–11:00・B 10:15–10:45。B を消す → P を消す → P を本文ごと消す → B を戻す → 作り直し | 戻った滞在 1・位置 0。B は `rebuild:absorbed`・本文あり（一覧に戻らない） |
| A2 | 同じで B 10:40–11:05（はみ出し 5 分） | 戻った位置 2 だが滞在にならず、B は `rebuild:absorbed`（一覧に戻らない） |
| A3 | 同じで B 10:40–11:30（はみ出し 30 分） | B は一覧に出る |
| B1 | 重なる L を `rebuild:absorbed` のまま、範囲の行なしで本文を消す → 15:00 に位置を入れて作り直す | 4 時間離れた滞在が `rebuild:erased-range` で隠れ、「消した」の行が出た |
| B2 | B1 で L の時間を範囲の行に書く → 11:10–11:30 に位置を入れて作り直す | P の外で L の中の滞在が隠れた |
