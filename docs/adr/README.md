# ADR（Architecture Decision Record）

## ADR とは

ashiato2 の設計上の判断を、1 件 1 ファイルで残す記録である。
何が起きて（背景）、何を決め（決定）、何を退け、どうなったら見直すかを書く。
ハーネスの判断はここに書かない。harness2 の `docs/adr/` にある。

## いつ書くか

Story をまたぐ判断だけを書く。製品の前提・コードの構成・道具の規則などである。
1 つの Story（change）の中で閉じる判断は、その change の `design.md` の `D<n>` に書く。

決定が後で変わったら、元の ADR は消さない。状態を「置き換え済み（→ 後継のファイル名）」にし、新しい ADR を足す。

## 形

ファイル名は `YYYYMMDDHHMM-<英小文字の slug>.md`。時刻は判断が決まった日で、同じ日の時分は `0000` から順に振る。

```
# <題>
- 日付: YYYY-MM-DD
- 状態: 採用 | 置き換え済み（→ <後継のファイル名>）
- 出所: <PR #n・issue・メモの名前>

## 背景
（実測した事実。何が起きたか）

## 決定

## 退けた案
（分かるものだけ。分からなければ「記録なし」）

## 反転の条件
（どうなったら見直すか。無ければ「記録なし」）

## 影響
```

## 一覧（新しい順）

- [2026-10-11 lint を一段厳しくする（Rust は pedantic、Web は strictTypeChecked、Kotlin は baseline）](202610110002-stricter-lint.md)
- [2026-10-11 画面の作り直しは見た目（表面）だけにし、骨格は変えない](202610110001-ui-surface-only.md)
- [2026-10-11 純粋な計算は I/O を持たない crate に分け、コンパイラで止める](202610110000-pure-crate-split.md)
- [2026-10-08 ashiato は衛星アプリの基盤が主で、AI は衛星の 1 つとする](202610080000-foundation-for-satellites.md)
- [2026-10-03 worktree の待ち受けのポートは、どの worktree でも被らないよう機械が割り当てる](202610030000-worktree-port-allocation.md)
