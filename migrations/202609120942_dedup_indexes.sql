-- 重複を防ぐ索引を 2 段にする（ST03 / 深掘り Q2 / Q6 / Q15。design D1）**BREAKING**
-- 前進のみ。戻し手順は migrations/202609120942_dedup_indexes.down.sql に置く。
--
-- ============================================================================
-- **コードを先に直すこと。** この版を当てた瞬間、古いコードの
-- `ON CONFLICT (logical_source, content_hash)` は
-- `there is no unique or exclusion constraint matching the ON CONFLICT specification`
-- で**文として落ちる**（実測）。部分索引には述語を文に書かないと当たらない。
-- 順序が逆だと、その間の取り込みが**全件 500** になる。
-- `crates/server/src/lib.rs` の `MIGRATIONS` 配列は起動時に当たるので、
-- 新しい実行ファイルなら両方が同時に入る —— **psql から手で当てるときだけ順序が効く**。
-- ============================================================================
--
-- 2 段にする理由（Q6）: 外部サービス上の識別子が違えば別の記録として入れる。
-- 内容の鍵を一意のまま残すと、その 2 件目が一意違反で落ちる（実測）。
-- 内容側を **`external_id` を持たない記録に限る部分索引**へ狭めるほかない。
--
-- 利用者識別子を足す理由（Q2 / Q15）: 判定は利用者ごと。**鍵の中身には混ぜない**ので
-- `content_hash` の作り方は ST01 のまま（`hash_is_pinned` の期待値は変わらない）。

-- 外部識別子があればそれで畳む（利用者ごと・ソースごと）
DROP INDEX IF EXISTS core.event_dedup_ext;
CREATE UNIQUE INDEX IF NOT EXISTS event_dedup_ext
  ON core.event (user_id, logical_source, external_id) WHERE external_id IS NOT NULL;

-- 外部識別子が無ければ内容の鍵で畳む。**部分索引**（Q6）
DROP INDEX IF EXISTS core.event_dedup_hash;
CREATE UNIQUE INDEX IF NOT EXISTS event_dedup_hash
  ON core.event (user_id, logical_source, content_hash) WHERE external_id IS NULL;

-- **一意ではない 3 本目。** Q8 の畳み込み（内容の鍵が同じ複数行を 1 件として読む）と
-- Q19 の削除済みの判定（削除済みと内容の鍵が一致する記録を入れない）が両方これに乗る。
-- 実測: 判定は索引走査 1 回・バッファ 2 ページ・0.058 ms。
CREATE INDEX IF NOT EXISTS event_hash_all
  ON core.event (user_id, logical_source, content_hash);
