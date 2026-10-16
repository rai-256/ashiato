-- 戻し手順（202610141000_subjective_log）。**一部だけ戻る。不可逆な部分がある** ——
--
-- 1. 主観の記録の錠の関数とトリガは落とす。**主観の記録が書き換えられる状態へ戻る**
--    （FR-48 が実装者の善意でしか守られない状態。この版を当てる前と同じ）。
-- 2. **主観の記録の行（`core.event` の `s01-feeling`）は消さない。** 消す手段を持たせない
--    のがこの版の目的なので、戻しでそれを迂回しない。
-- 3. **主観の記録か通知の時刻の行が 1 つでも残っていれば、登録簿の行・紐づけ先の種類の表・
--    通知の時刻の台帳を残す**（design D16 / ST19 D12 と同じ）。本人が書いたものは作り直せない。
--    表が残るときは、その錠の関数（`core.reject_feeling_ledger_change()`）とトリガも残す。
--
-- 表と登録簿の行が落ちるのは、**主観の記録も通知の時刻も 1 つも無いとき**だけ。

DROP TRIGGER IF EXISTS event_feeling_no_delete ON core.event;
DROP TRIGGER IF EXISTS event_feeling_requires_ledger ON core.event;
DROP TRIGGER IF EXISTS event_feeling_immutable ON core.event;
DROP FUNCTION IF EXISTS core.reject_feeling_delete();
DROP FUNCTION IF EXISTS core.require_feeling_erasure_ledger();
DROP FUNCTION IF EXISTS core.reject_feeling_rewrite();

DO $do$
DECLARE
  has_rows boolean := false;
BEGIN
  -- 2 回目の戻しでは表がもう無い。表が在るときだけ中を数える
  IF to_regclass('core.feeling_reminder') IS NOT NULL THEN
    EXECUTE 'SELECT EXISTS (SELECT 1 FROM core.feeling_reminder)' INTO has_rows;
  END IF;
  IF has_rows
     OR EXISTS (SELECT 1 FROM core.event WHERE logical_source = 's01-feeling') THEN
    RAISE NOTICE '主観の記録か通知の時刻が残っているので、2 表と登録簿の行は残す（design D16）';
    RETURN;
  END IF;

  -- 台帳の錠は自分で外してから落とす（DELETE / TRUNCATE を拒むトリガが付いている）
  DROP TRIGGER IF EXISTS feeling_reminder_no_truncate ON core.feeling_reminder;
  DROP TRIGGER IF EXISTS feeling_reminder_append_only ON core.feeling_reminder;
  DROP TRIGGER IF EXISTS feeling_target_kind_no_truncate ON core.feeling_target_kind;
  DROP TRIGGER IF EXISTS feeling_target_kind_append_only ON core.feeling_target_kind;
  DROP TABLE IF EXISTS core.feeling_reminder;
  DROP TABLE IF EXISTS core.feeling_target_kind;
  DROP FUNCTION IF EXISTS core.reject_feeling_ledger_change();
  DELETE FROM core.source WHERE logical_source = 's01-feeling';
END;
$do$;
