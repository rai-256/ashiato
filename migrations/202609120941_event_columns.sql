-- 記録に 2 列を足す（ST03 / 深掘り Q20 / Q24。design D8）
-- 前進のみ。戻し手順は migrations/202609120941_event_columns.down.sql に置く。
--
-- どちらも **day one で持つ**。後から足すと、持つ前に起きた更新の順序（Q20）と、
-- 原文に含まれない対象の識別子（Q24）が永久に復元できない。

-- Q20。外部サービス側の更新時刻。**時刻型で作る** —— 不透明な版（ETag・世代番号）しか
-- 返さないソースが出たときは「届いた順」に倒す（design D8）。ソース単位ではなく
-- **記録単位**で判定する（同じソースでも項目が欠ける到着がある）。
ALTER TABLE core.event ADD COLUMN IF NOT EXISTS source_updated_at timestamptz;

-- Q24。「対象ごと」の識別子（動画 ID など）。**索引を張らない** ——
-- 重複の判定には使わないので、索引を張ると「判定に使える」という誤解の足場になる。
-- 判定に使うと、同じ対象についての 2 件目が一意違反で落ちる（R42 の実測）。
ALTER TABLE core.event ADD COLUMN IF NOT EXISTS external_ref text;

-- **ビューを作り直す。** `core.event_live` は 0001 が `SELECT *` で作っており、
-- 展開はビューを作った時点で固まる —— 足した 2 列はビューに出てこない。
-- `CREATE OR REPLACE VIEW` は末尾への列の追加だけを許すので、`ALTER TABLE` が
-- 列を末尾に足すこの形なら置き換えられる。
CREATE OR REPLACE VIEW core.event_live AS
  SELECT * FROM core.event WHERE deleted_at IS NULL;
