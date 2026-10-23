# ST41 幅のある出来事に終わりの時刻を持たせる（上流）

change: `openspec/changes/st41-event-end-time/`。触る capability は `record-envelope`（MODIFIED 3・ADDED 2）・`data-durability`（ADDED 1）・`collection-coverage`（ADDED 1）。満たす要件は FR-91（扉 #28）。

> **第 1 回の深掘りの答えを待たずに、推奨の答えで先に書いた**（2026-10-20 の流れ）。下の Q1〜Q7 は**本人の答えではなく推奨**で、`deep.md` は `（答え待ち・推奨: …）` のまま。
> 答えが推奨と違えば `spec --answers` が仕様に写し直す。

## 深掘り（第 1 回。推奨で書いた）

| Q | 段 | 推奨（仕様はこれで書いた） | どこに効くか |
|---|---|---|---|
| Q1 終わりが始まりより前 | B | 送られたまま入れる。期間では始まりの点、書き出しは内容の終わり→長さ→次の記録の順（spec レビュー R2 で本人が見る説明を合わせた） | record-envelope の時刻の要件・期間の要件、data-durability の ADDED、design D4 |
| Q2 「期間に重なる」の境界 | B | 期間も記録も「始まりを含み、終わりを含まない」。点も入れる。長さ 0・逆順は点 | 期間の要件、design D8 |
| Q3 期間で引く口 | B | `GET /events` に `from` / `to` / `source`。終わりの 3 欄は値のある行にだけ載る（終わりの無い行の鍵は 6 つのまま） | 期間の要件、design D9 |
| Q4 日の区切り | B | Asia/Tokyo のまま。件数は始まりの日に 1 件、期間で引けば重なった日すべてに出る | collection-coverage の ADDED、design D12 |
| Q5 滞在の終わりを列にも書くか | B | 書かない | design D13、tasks 7.1b |
| Q6 収集済みの中身の終わり | B | 写さない | design D14、tasks 7.1b |
| Q7 終わりだけが違う到着 | **A**（`loss: discarded`。本人の答えが要る） | 鍵は変えない。外部の識別子のある記録だけ、終わりの違いを更新にする | record-envelope の更新の要件・ADDED「終わりの時刻は、同じ記録の判定にも削除済みの止めにも使わない」、design D7 |

## 仮決め（B）と反転条件（design の（仮））

- **D4（仮）** 逆順の終わりは断らない（Q1）。反転条件: Q1 で「断る」なら `invalid_end_time` で断り CHECK を足す
- **D8（仮）** 半開区間・点も入れる（Q2）
- **D9（仮）** `GET /events` に足す（Q3）。反転条件: 新しい口・口なし
- **D12（仮）** 日は Asia/Tokyo・件数は始まりの日（Q4）。1 日の並びは ST39 へ申し送り
- **D13（仮）・D14（仮）** 滞在・収集済みの記録は列に写さない（Q5・Q6）
- **D16（仮）** 期間の求めに件数の上限を置かない（ST45 が衛星向けに使うなら足す）
- **D17（仮）** 書き出しの「列の終わりを先に読む」は ADDED のまま（CH02 が同じ要件を MODIFIED しているため）。反転条件: CH02 が archive 済みなら MODIFIED に
- **D18（仮）** 内容が同じで終わりを持たない到着は保存済みの終わりを消さない（進行中の版の再送で終わりが巻き戻らない。spec レビュー R3）

仮でない決定: D1（3 列・揃い・CHECK）・D2（取り込み口の検査と `invalid_end_time`）・D3（収集アプリは変えない）・D5（終わりの時刻は出来事の時刻と同じ錠、ずれと TZ は錠の外）・
D6（進行中→終わり）・D7（鍵は変えない。µs に切り捨てて比べる）・D10（終わりのある行だけの式の部分 GiST 索引）・D11（書き出しと消した滞在の時間は列から先に読む）・D15（移行 1 つ。CHECK は `NOT VALID`）。
聞かずに決めたことは deep.md の C1〜C6。

## capability の前倒し

なし。`collection-coverage` と `data-durability` に ADDED で足したので、`docs/stories/INDEX.md` の capability 表に ST41 を足し、訂正の節に理由を書いた。
`data-durability` は走っている CH02 が同じ要件を MODIFIED しているので MODIFIED を重ねなかった（D17（仮））。

## tasks

7 Task・18 項目。Scenario は 73 本（新しく足した 56 本は全部どこかの Task が名指しする）。画面は持たないので e2e と人間の確認待ちは無い。
収集アプリには触らない（tasks 2.3・7.1b が `git diff --quiet` で確かめる）。

## 申し送り

- `docs/handoff/ST39.md` に 3 節: 外部の識別子を持たない送り手は終わってから送る（review/deep.md R3）・始まりの側のずれと TZ の検査が無い既存の穴（R5）・1 日の並びに衛星のソースを足すときの読み方（spec レビュー R9）
- CH02 へは、ST41 が先に main に入ったときだけ下流が `docs/handoff/CH02.md` を書く（tasks 7.1）

## レビュー

- deep-review（`review/deep.md`）: 8 件 —— escalated 6（Q1・Q7 と、問いの HTML の「聞かずに決めること」に写したもの）・fixed 2
- spec-review（`review/spec.md`）: 17 件 —— fixed 14（うち D17・D18 の仮 2）・escalated 1（R2: 本人が見る Q1 の説明を spec に合わせ、Q1 で答えてもらう）・deferred ST39 1（R9）・
  分類を直して fixed 1（R14: premise → technical。CHECK の時間を実物で測った —— `NOT VALID` 1.4 ms・素の検査 357.7 ms。premises.md）
- `openspec validate --strict` rc=0 / `review_triage.py` OK / `check_chain.py` OK
