-- 0003 の戻し。**戻すと原文が失われる** —— jsonb はキー順・重複キー・数値表記を
-- 正規化するので、この方向は不可逆になる。
--
-- **原文が JSON でない記録があると落ちる。** 0003 以降の原文は「取得元から受け取った
-- 文字列そのもの」であって JSON である保証が無い。戻す前に確かめる:
--   SELECT count(*) FROM core.event WHERE raw IS NOT NULL AND raw::jsonb IS NULL;
DROP VIEW IF EXISTS core.event_live;
ALTER TABLE core.event ALTER COLUMN raw TYPE jsonb USING raw::jsonb;
CREATE VIEW core.event_live AS SELECT * FROM core.event WHERE deleted_at IS NULL;
