-- 0004 「収集した」からの離脱と、記録の来歴の書き換えも拒む（FR-30 / design D21）
-- 前進のみ。戻し手順は migrations/202609100000_immutable_origin.down.sql に置く。
--
-- **0002 は 3 手で迂回できた。** 独立検証で実測（review R2）:
--   UPDATE core.event SET raw='{"tampered":1}' …;  → 拒否
--   UPDATE core.event SET origin='authored'    …;  → 通る   ← トリガが origin を見ていない
--   UPDATE core.event SET raw='{"tampered":1}' …;  → 通る   ← OLD.origin が 'authored'
--   UPDATE core.event SET origin='collected'   …;  → 通る
--   → collected | {"tampered":1}
--
-- 0002 のコメントは「同じ PC の第三者製プラグイン（PERM-8）や psql を直に叩く運用が
-- 素通りする」ことを根拠に DB 側へ置いたと書いている。**その素通りする主体が、
-- まさにこの 3 手を打てる。** 分類を動かせる限り、原文の不変は成り立たない。
--
-- 併せて `content_hash` と `ingest_time` も凍結する。どちらも記録の中身ではなく
-- **来歴**（重複の判定と、いつ届いたか）で、書き換える正当な理由が無い。
-- **`tz_id` / `tz_offset_min` は凍結しない** —— 収集側の設定ミスで誤った地域が入ったとき、
-- 直せる余地を残す（原文が残っているので、正しい値は原文から引き直せる）。

CREATE OR REPLACE FUNCTION core.reject_collected_rewrite() RETURNS trigger AS $fn$
BEGIN
  -- 論理削除（deleted_at / deleted_by）は通す。FR-50 が認めている。
  IF OLD.origin = 'collected' AND (
       NEW.raw          IS DISTINCT FROM OLD.raw
    OR NEW.payload      IS DISTINCT FROM OLD.payload
    OR NEW.event_time   IS DISTINCT FROM OLD.event_time
    OR NEW.content_hash IS DISTINCT FROM OLD.content_hash
    OR NEW.ingest_time  IS DISTINCT FROM OLD.ingest_time
  ) THEN
    RAISE EXCEPTION '収集した記録の原文・解析済み・出来事の時刻・冪等キー・格納の時刻は書き換えられない（FR-30）';
  END IF;

  -- **分類そのものを動かせないようにする。** ここが開いていると、いったん
  -- 'authored' へ移してから原文を書き換え、'collected' へ戻せてしまう。
  IF OLD.origin = 'collected' AND NEW.origin IS DISTINCT FROM OLD.origin THEN
    RAISE EXCEPTION '収集した記録の由来は変えられない（FR-25 / FR-30）';
  END IF;

  RETURN NEW;
END;
$fn$ LANGUAGE plpgsql;
