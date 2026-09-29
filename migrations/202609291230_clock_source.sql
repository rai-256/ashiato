-- 端末の時計のずれの測定記録の論理ソース（ST05 / design D1）。
--
-- **`external_id_kind = 'none'`** —— 測定は外部サービスの記録ではない。既定の `'record'` のままだと
-- 識別子を持たない記録が全件 `missing_external_id` で断られ、端末の未送信に残り続ける。
--
-- `expected_gap_sec` は稼働状況に使われない（`coverage::must_sources()` に入れない。生存信号も送らない）が、
-- 列が `NOT NULL` なので 6 時間（仮。反転条件は design D1）を入れる。
INSERT INTO core.source (logical_source, display_name, expected_gap_sec, external_id_kind)
VALUES ('c01-clock', '携帯端末の時計のずれ', 21600, 'none')
ON CONFLICT (logical_source) DO NOTHING;
