-- 携帯端末の健康データの権限の記録の論理ソース（ST11 / design D1）。
--
-- **`external_id_kind = 'none'`** —— 権限の状態は外部サービスの記録ではない。既定の `'record'` のままだと
-- 識別子を持たない記録が全件 `missing_external_id` で断られ、端末の未送信に残り続ける（ST05 R3）。
-- **`outage_target = false`** —— 状態が変わったときだけ届くので、既定の対象のままだと想定間隔の 3 倍で途切れの知らせが鳴る。
--
-- `expected_gap_sec` は稼働状況にも途切れの判定にも使われない（`coverage::must_sources()` に入れない。生存信号も送らない）が、
-- 列が `NOT NULL` なので 6 時間（`c01-clock` と同じ扱い）を入れる。
-- `acquisition` / `fetch_trigger` は列を指定しない —— `c01-` の前置きで ST13 の trigger が
-- `acquisition = 'device'`・`fetch_trigger = 'on_change'` を入れる（`fetch_interval_sec` は NULL）。
--
-- **種類（FR-102）は「稼働状況」。** 登録簿にはまだ種類の列が無いので、ここに注記だけ残す
-- （種類の列を作る Story がこの行を「稼働状況」で入れる）。
INSERT INTO core.source (logical_source, display_name, expected_gap_sec, external_id_kind, outage_target)
VALUES ('c01-health-permission', '携帯端末の健康データの権限', 21600, 'none', false)
ON CONFLICT (logical_source) DO NOTHING;
