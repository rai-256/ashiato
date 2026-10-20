# CH02 の前提（実物の調査）

外の振る舞いに依る前提は PostgreSQL（compose の `postgres:17-alpine`）の窓関数と計画だけ。
2026-10-20 に使い捨ての `postgres:17-alpine`（17.11）へ合成データを入れて走らせた（`docker run --rm`。repo の DB には触っていない）。
合成データ: `core.event` に似せた表 120 万行（利用者 2・ソース 5・端末 3 と NULL が 1/7・4 行ずつ同じ時刻）と、前の版に似せた行 約 4.8 万（時刻を 0 か 5 秒前へずらす）。

| 前提 | 何が依るか | 確かめ方 | 結果 | 出所 | 確かさ |
|---|---|---|---|---|---|
| いまの引き（相関副問い合わせ）は、行ごとに同じソースの約 1.5 万〜4.5 万行を読み流す | 遅さの原因（依頼の診断） | 合成 120 万行で、先頭 2,000 行の次の記録をいまの形で引く | 2,000 行で 83.9 秒（1 行 42 ms）。`Bitmap Heap Scan` が loops=2000・1 回 4.5 万行 | 実物（PG 17.11 で走らせた） | 4 |
| `lead(event_time) OVER (PARTITION BY user_id, logical_source, device_id ORDER BY event_time, id)` は、いまの `(n.event_time, n.id) > (r.event_time, r.id)` の引きと同じ値を返す（同じ時刻・端末 NULL を含む） | 「結果を変えない」 | 3,300 行（うち端末 NULL 474）を両方で引いて突き合わせ | 食い違い 0 件 | 実物 | 4 |
| `PARTITION BY` は `device_id` の NULL を 1 つの組にまとめる（`IS NOT DISTINCT FROM` と同じ） | 端末を持たない記録の幅 | 上の突き合わせに NULL を含めた | 食い違い 0 件 | 実物 | 4 |
| 120 万行全部に `lead()` を当てても 1 回 2 秒の桁 | 「線形に近い」 | 120 万行の `lead()` を数える | 1.9 秒。**payload・raw を持たない細い表の値**（本物は太い。窓には鍵の列だけを載せて id で戻す前提。本物の量の時間は下流が量の試験で測る —— Q3） | 実物 | 3 |
| 前の版の次の記録（`n.event_time > r.event_time` の厳密な「後」。記録と前の版を混ぜた並び）は、**逆向きの並びで頭を固定した枠** `min(event_time) FILTER (WHERE 記録) OVER (… ORDER BY event_time DESC RANGE BETWEEN UNBOUNDED PRECEDING AND '1 microsecond' PRECEDING)` で同じ値になる | 前の版の除外を変えない | 約 2,400 行を両方で引いて突き合わせ | 食い違い 0 件・120 万 + 4.8 万行で 1.4 秒 | 実物 | 4 |
| **順向きで尾を固定した枠**（`RANGE BETWEEN '1 microsecond' FOLLOWING AND UNBOUNDED FOLLOWING` の `min`）は 2 乗に戻る（`min` は枠の頭が動くたびに数え直す） | 下流が選ぶ書き方の罠 | 同じデータで走らせた | 9 分を超えて止めた | 実物 | 4 |
| `timestamptz` の精度はマイクロ秒なので、`'1 microsecond' PRECEDING` は「厳密に後」と同じ | 前の版の枠 | PostgreSQL の文書（Date/Time Types: "1 microsecond resolution"）。deep-review が手元の PG で `'…00.0000004Z'::timestamptz = '…00Z'` が t、1 µs 差の記録が前の版の次になることを確かめた | 成り立つ | 公式文書 https://www.postgresql.org/docs/17/datatype-datetime.html・実物 | 4 |
| `(user_id, logical_source, device_id, event_time, id)` の索引 + NULL と非 NULL の 2 本に分けた相関副問い合わせは、窓関数より遅い | 「索引で直す」案を退ける | 合成 120 万行で先頭 2 万行 | 2 万行で 14.5 秒（1 行 0.7 ms。120 万行なら約 14 分） | 実物 | 4 |
| いまの記録の引き `(n.event_time, n.id) > (r.event_time, r.id)` は、同じ時刻に並んだ記録の先の方を幅 0 にする | Q1（`loss: exported`） | deep-review が値を直に書いた SQL で再現（01:00 × 2・03:00・消した時間 01:30〜02:00 → 片方だけ出る） | 再現した | 実物 | 4 |
| 本物の写しに、(c) で緯度経度を持ちうるソースの同じ時刻の組が何件あるか | Q1 の重さ | 未確認（本物の DB は見ていない） | — | — | 1 |

失われるもの（`loss`）のある決定は Q1 だけ（同じ時刻の記録の幅。`exported`）。Q1 は「どちらに倒すか」の決定で、前提（漏れる形がある）は確かさ 4。
件数（確かさ 1）は Q1 の判断に載せない —— 1 件でも漏れる形があることが判断の根拠。
それ以外は、結果が変われば緯度経度が書き出しに出るが、それは実装の誤りで決定ではない —— 変えない側を試験で固定する（Q2）。

## record の段への入力（CH02.md の「壊してはいけないもの」。deep-review R8）

- 写し（書き出しの本体）は `READ ONLY` のトランザクションで、一時表を作れない（ST33 design D5。窓の結果を一時表に置かない）
- 次の記録は `core.event_live` から引き、削除の印の行を間に挟まない（幅が長くなる側。ST33 design D5）
- 前の版の組の鍵（`user_id`・`logical_source`・`device_id`）は親から取る（`core.event_version` は `device_id` を持たず、利用者は凍結される）
- D17 の件数の鍵の論拠「新しい記録の取り込みは幅を縮める側にだけ働く」（ST33 design D17）を崩さない —— 窓は絞った後の行でなく `core.event_live` 全体の並びで次を決める
- 除く範囲の式は `excluded.rs` の 1 か所だけ（書き出しの本体・`GET /export/summary`・D17 が同じ断片を使う）
