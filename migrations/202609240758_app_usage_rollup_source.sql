-- アプリ利用の**集計**の論理ソースを登録簿に足す（ST06 / FR-84 / 深掘り Q5 / design D3）。
-- 前進のみ。戻し手順は migrations/202609240758_app_usage_rollup_source.down.sql に置く。
--
-- **イベント（`c01-app-usage`）と同じソースに混ぜない**（design D3）——
-- 粒度の違うものを 1 本にすると、内容の鍵での重複の判定と稼働状況の数えが壊れる。
-- 集計は年ごとの箱に 2 年ぶん、日ごとの箱に 10 日ぶんが同時に入ってくる。
--
-- **`external_id_kind = 'none'` を明示する**（独立レビュー R7）。列の既定は
-- `'record'`（＝識別子が無ければ断る側）で、そのままだと取り込み口が
-- `missing_external_id` で断る。しかもその理由は「受け手側の設定で変わりうる」扱いなので、
-- **端末は未送信から取り除かず、6 時間ごとに 1 件ずつ永久に溜まる。**
-- 集計は外部サービス上の識別子を持たない（取得元は端末の OS）ので、内容の鍵で畳む。
--
-- **`expected_gap_sec = 21600`（6 時間）を明示する**（design D3 / 本人の決定 C8）。
-- 列は `NOT NULL` で既定を持たないうえ、この値は端末側の生存信号の区間と
-- **同じでなければならない** —— ずらすと受け手が「想定間隔を超えて何も来ない」と
-- 判定する窓とずれ、正常な運用が⑥「途絶」に見える。
--
-- **`ON CONFLICT DO NOTHING`**（`migrate()` は起動のたびに全版を当て直す）。
-- 条件なしで上書きする版にすると、**本人が変えた想定間隔が再起動のたびに初期値へ戻る**。

INSERT INTO core.source (logical_source, display_name, expected_gap_sec, external_id_kind)
VALUES ('c01-app-usage-rollup', '携帯端末のアプリ利用（集計）', 21600, 'none')
ON CONFLICT (logical_source) DO NOTHING;
