-- 0005 の戻し手順。
--
-- **不可逆。** 戻すと次が失われる:
--
--   1. **生存信号（core.heartbeat）が全部消える。** 「動いていた・記録なし」と
--      「動いていたが取れない状態だった」の唯一の証拠で、**遡って作れない**
--      （扉 #14 が「後から区別する唯一の手段」と呼んだ当のもの）。
--   2. **停止と破棄の期間（core.coverage_span）が全部消える。** 0001 の core.coverage は
--      期間を note の自由文でしか持てないので、時刻の範囲は移す先が無い。
--   3. **収集開始日（core.source.collection_started_on）が消える。** 記録から引き直せる値だが、
--      記録が 1 件も無いまま生存信号だけが届いていたソースの開始日は戻らない。
--   4. **稼働記録の日の区切りが UTC に戻る。** 0005 は Asia/Tokyo で数え直している（深掘り Q2）。
--
-- 以下は「元の形に戻す」だけで、上の 4 つは戻らない。

DROP TABLE IF EXISTS core.heartbeat;
DROP TABLE IF EXISTS core.coverage_span;
ALTER TABLE core.source DROP COLUMN IF EXISTS collection_started_on;
ALTER TABLE core.source DROP COLUMN IF EXISTS user_id;

DROP TABLE IF EXISTS core.coverage;
CREATE TABLE core.coverage (
  logical_source text NOT NULL REFERENCES core.source(logical_source),
  day            date NOT NULL,
  state          text NOT NULL CHECK (state IN ('alive','stopped','dropped')),
  event_count    integer NOT NULL DEFAULT 0,
  note           text,
  PRIMARY KEY (logical_source, day, state)
);
