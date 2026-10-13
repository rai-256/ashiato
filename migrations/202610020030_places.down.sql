-- 戻し手順（202610020030_places）。**一部だけ戻る。不可逆な部分がある** ——
--
-- 1. 場所の記録の錠の関数とトリガは落とす。**場所の記録が書き換えられる状態へ戻る**
--    （FR-46 が実装者の善意でしか守られない状態。この版を当てる前と同じ）。
-- 2. **場所の記録の行（`core.event` の `s01-place`）は消さない。** 消す手段を持たせない
--    のがこの版の目的なので、戻しでそれを迂回しない。
-- 3. **場所の記録か器の行が 1 つでも残っていれば、登録簿の行も器の表も残す**（design D16 /
--    ST19 D12 と同じ）。本人が名前を付けた事実は作り直せない。器の表は追記のみで、
--    記録は器を**識別子で**指すので、表を落とすと「その識別子が何だったか」が戻しで永久に失われる。
--    器の表が残るときは、その錠の関数（`core.reject_place_change()`）も残す。
--
-- 器の表と登録簿の行が落ちるのは、**場所の記録も器も 1 つも無いとき**だけ（＝当てたが使わなかった場合）。

DROP TRIGGER IF EXISTS event_place_no_delete ON core.event;
DROP TRIGGER IF EXISTS event_place_requires_ledger ON core.event;
DROP TRIGGER IF EXISTS event_place_immutable ON core.event;
DROP FUNCTION IF EXISTS core.reject_place_record_delete();
DROP FUNCTION IF EXISTS core.require_place_erasure_ledger();
DROP FUNCTION IF EXISTS core.reject_place_record_rewrite();

DO $do$
DECLARE
  has_rows boolean := false;
BEGIN
  -- 2 回目の戻しでは器の表がもう無い。表が在るときだけ中を数える
  IF to_regclass('core.place') IS NOT NULL THEN
    EXECUTE 'SELECT EXISTS (SELECT 1 FROM core.place)' INTO has_rows;
  END IF;
  IF has_rows
     OR EXISTS (SELECT 1 FROM core.event WHERE logical_source = 's01-place') THEN
    RAISE NOTICE '場所の記録か器が残っているので、器の表と登録簿の行は残す（design D16）';
    RETURN;
  END IF;

  -- 器の錠は自分で外してから落とす（DELETE / TRUNCATE を拒むトリガが付いている）
  DROP TRIGGER IF EXISTS place_no_truncate ON core.place;
  DROP TRIGGER IF EXISTS place_append_only ON core.place;
  DROP TABLE IF EXISTS core.place;
  DROP FUNCTION IF EXISTS core.reject_place_change();
  DELETE FROM core.source WHERE logical_source = 's01-place';
END;
$do$;
