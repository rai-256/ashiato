-- 戻し手順（202609120941_event_columns）。**不可逆** ——
-- `source_updated_at` を落とすと、外部サービス側の更新の順序が復元できなくなる
-- （以後、古い版が届いても止められない）。`external_ref` を落とすと、
-- 原文に対象の識別子を含まないソースの記録では、その対象が永久に分からなくなる。
DROP VIEW IF EXISTS core.event_live;
ALTER TABLE core.event DROP COLUMN IF EXISTS external_ref;
ALTER TABLE core.event DROP COLUMN IF EXISTS source_updated_at;
CREATE VIEW core.event_live AS SELECT * FROM core.event WHERE deleted_at IS NULL;
