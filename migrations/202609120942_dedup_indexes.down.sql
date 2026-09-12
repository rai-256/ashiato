-- 戻し手順（202609120942_dedup_indexes）。**不可逆** ——
-- 内容の鍵の索引を一意へ戻すと、Q6 で入った「同じ内容・違う外部識別子」の 2 行目以降が
-- 一意違反になって索引を作れない。作れるようにするには**その行を消すほかなく**、
-- 消した記録は復元できない。
DROP INDEX IF EXISTS core.event_hash_all;
DROP INDEX IF EXISTS core.event_dedup_hash;
DROP INDEX IF EXISTS core.event_dedup_ext;
CREATE UNIQUE INDEX event_dedup_ext
  ON core.event (logical_source, external_id) WHERE external_id IS NOT NULL;
CREATE UNIQUE INDEX event_dedup_hash
  ON core.event (logical_source, content_hash);
