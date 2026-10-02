# docs/handoff/ を PR の前に読んだ結果（2 回目）

- 読んだ日時: 2026-10-02 13:12（JST）
- `git log -1 --format=%h -- docs/handoff`: 7438f23
- ST25 に触れる申し送り: `docs/handoff/ST25.md`・`ST12.md`・`ST14.md`

## 処置

| 申し送り | 内容 | 処置 |
|---|---|---|
| `ST25.md`（st21-place-registry Q4） | S-2 の見出しを場所の名前にするのは ST21・ST22・ST25 の archive 後の `fix/`。ST21・ST22 が archive 済みなら ST25 の PR 本文の「後続」にこの `fix/` を挙げる | **ST21・ST22 はまだ archive されていない**（`openspec/changes/archive/` に無い）ので、PR 本文の「後続」には挙げない。見出しは時刻の範囲のまま（ST25 は見出しを触らない） |
| `ST12.md`（review/spec.md R3） | 書庫の名前が design と実装で違う。ST25 は実装の名前（`c03-timeline-move` / `c03-timeline-route`）で許可リストを作った | 済み。登録簿の `c03-%` を表と突き合わせる試験（`st25_allow_registry_covers_archive`）が Task 1.1 にある。ST12 の側の直しは ST12 の merge 後の `fix/` |
| `ST14.md`（Q5） | 稼働状況の画面の明暗の追従は ST14 の担当。ST25 の後は 1 日の画面（明）→ 稼働状況（暗）と移るたびに入れ替わる | ST25 は 1 日の画面だけ（Q5）。稼働状況の画面は触っていない。ST14 の上流が拾う |
