# CH02 — 書き出しの「次の記録」の引きを行数の 2 乗で遅くならないようにする

> 第 1 回の深掘りの答えを待たずに、推奨の答え（`deep.md` の `（答え待ち・推奨: …）`）で先に書いた（2026-10-20 の流れ）。
> 本人の答えが推奨と違えば `spec --answers` が写し直す。

## Why

書き出しから除く範囲（`crates/server/src/export/excluded.rs` の `overlap_sql`）は、幅の (c)「次の記録まで」を行ごとの相関副問い合わせで引いていて、
行数に対して 2 乗で伸びる。2026-10-20 の実測で、7.8 万行の DB で書き出し 1 回の 3 本の問い合わせがそれぞれ約 84 秒、export の試験 142 本が 2 時間を超えた
（`docs/changes/CH02.md`）。本物の写しは約 120 万行で、合成 120 万行ではいまの引きが 1 行 42 ms（全部なら約 14 時間）かかる（`premises.md`）。
書き出しの本体・設定画面を開くたびの件数の数え（`GET /export/summary`）・後に消した記録の件数（ST33 D17）が同じ断片を通るので、どれも同じ理由で遅い。

## What Changes

- 次の記録の引きを、ソース・端末ごとの**窓関数の一度読み**に替える（相関副問い合わせをやめる）。記録と前の版の次の記録を、`core.event_live` の同じ並びから 1 回で決める
- **同じ時刻に並んだ記録の「次の記録」を、時刻が後の最初の記録にする**（深掘り Q1 の推奨。答え待ち）。いまは `(event_time, id)` の順で隣の同じ時刻の記録になり、
  片方が幅 0 の点として消した時間の中でも書き出しに出る。前の版の側はすでに「時刻が後の最初の記録」で引いていて、記録の側をそれに揃える。
  除く行は**増える側にだけ**変わる（除き過ぎは書き出し直せば戻る。C21 の既定は厳しい側）。依頼の「結果を変えない」を越えるのはこの 1 点だけ
- それ以外は、どの記録と前の版が除かれるかを**変えない**。消した滞在が 0 件なら窓を回さず、消した時間の終わりより後の行は各組の最初の 1 行だけを窓に入れる絞り込み（ST33 D5 が相関副問い合わせを選んだ理由）を残す（Q4 の推奨）
- **ST33 design D5 の「`lead()` の窓関数ではなく相関副問い合わせ」の判断を覆す。** D5 は「窓は全行を並べ替えてから絞る」ことを理由にしたが、絞り込みは窓の前に時刻だけで置け、
  いまの形は絞った行の数 × 組の行の数を読む（84 秒の実測）。D5 の「並びと同じ時刻の決め方は `lead()` と同じ」も、同じ時刻の扱いが変わるので書き直す（design D1・D2）
- 変わらないことを、いまの SQL を試験の中に残してまっさらな DB の偽データで新旧を突き合わせて固定する（Q2 の推奨）。速さは、読んだ量を行数 N と 4N で比べる試験で固定し、
  量の試験で時間を記録する（Q3 の推奨）
- 移行は足さない（索引も足さない。`premises.md`: 索引 + 相関副問い合わせは 120 万行で約 14 分）

## Capabilities

### New Capabilities

（無し）

### Modified Capabilities

- `data-durability`: 「書き出しは本人が消した場面を出さない」の (c) に、同じ時刻の記録を飛ばすことを書き足し、いまの読み方（端末を持たない記録・緯度経度を持たない記録も次の記録になる・前の版の (c)・消した滞在が無いとき）を Scenario で固定する。
  除く範囲の判定が行数の 2 乗で伸びないことを Requirement として足す

## Impact

- コード: `crates/server/src/export/excluded.rs`（`overlap_sql` と `excluded_sql` の CTE。公開の形 `ExcludedSql` / `EXCLUDED_EVENT` / `EXCLUDED_VERSION` / `event_kept` / `version_kept` は変えない）。
  呼び出し側（`dump.rs:138`・`summary.rs:92`・`ledger.rs:431`）は触らない。試験は `crates/server/src/export_excluded_tests.rs` の並びに新しいファイル `export_window_tests.rs` を足す
- API・ファイルの形・台帳（`written` の形。D17）・移行: 変わらない。書き出しの結果は Q1 の同じ時刻の組だけが「除く」側へ変わる
- 走っている Story との重なり: **ST09**（`st09-media-capture`。下流が走っている）が同じ `excluded.rs` の `SPAN_RULES` / `NO_COORD_SOURCES` / `COORD_SOURCES` に
  `c01-screenshot` / `c01-image` / `c01-video` を足し、`data-durability` に ADDED の Requirement（「写真と画像と動画のソースと、ST09 が足す欄と表は書き出しの分類に載る」）を足す。
  CH02 が触るのは `overlap_sql` と `excluded_sql` の本体と、別の Requirement（MODIFIED）で、行も Requirement も重ならない。gate の rebase で後から入る側が追従する。差し戻さない。
  ST09 の export の試験は ST09 のブランチにあり、CH02 の新しい断片は ST09 のソースも一覧を読むだけなので、どちらが先に入っても同じ試験が通る
- `docs/changes/CH02.md` の front matter の `capability: []` はそのままにする（admission は済み。`data-durability` を書くと走っている ST09 と衝突待ちになるが、上の通り行も Requirement も重ならない）
- ST33 の D13 の反転条件（件数の数え 1 回が 3 秒以上なら替える。2026-10-14 に 35 万行 6.5 秒で当たったまま）: この change の計測の値を渡す。替えるかは D13 の持ち主（本人）が決め、この change では替えない
