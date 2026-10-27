-- 本文ごと消す操作の台帳・範囲・派生の門・ビュー・書き直しの記録を戻す（ST23）。
-- 不可逆: 消去の操作の台帳と範囲の表の行（消したことの事実・件数・範囲）は失われる。
-- 先に止める: 操作の行があるなら戻さない。本文を消した後に戻すと、範囲の表が無くなって
-- core.stay_erased の範囲が本文の読み（1µs）に縮み、消した時間帯が分からなくなる。
-- 表が既に無い（戻した後の 2 回目）ときは見ない。IF を入れ子にして、表の無い問い合わせを組ませない。
DO $$
BEGIN
  IF to_regclass('core.erasure_operation') IS NOT NULL THEN
    IF EXISTS (SELECT 1 FROM core.erasure_operation) THEN
      RAISE EXCEPTION '消去の操作の台帳に行がある。戻すと消した時間帯の範囲が失われるので戻せない（ST23 / design Migration Plan）';
    END IF;
  END IF;
END
$$;

DO $$
BEGIN
  IF EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'ashiato_app') THEN
    REVOKE MAINTAIN ON core.event, core.event_version FROM ashiato_app;
  END IF;
END
$$;

DROP INDEX IF EXISTS core.purge_compaction_by_table;
DROP TABLE IF EXISTS core.purge_compaction;

DROP TRIGGER IF EXISTS event_requires_derived_ledger ON core.event;
DROP FUNCTION IF EXISTS core.require_derived_erasure_ledger();

-- ST41（202610231200_event_end_time）の定義へ戻す（core.stay_erased_purge の写しをやめるので作り直す）。
DROP VIEW IF EXISTS core.stay_erased;
DROP VIEW IF EXISTS core.stay_erased_purge;
CREATE VIEW core.stay_erased AS
  SELECT id,
         user_id,
         event_time AS start_at,
         coalesce(CASE WHEN end_time > event_time THEN end_time END,
                  core.try_timestamptz(payload->>'end'), event_time + interval '1 microsecond') AS end_at,
         deleted_at,
         deleted_by
    FROM core.event
   WHERE logical_source = 's01-stay'
     AND origin = 'derived'
     AND deleted_at IS NOT NULL;

ALTER TABLE core.erasure_ledger DROP COLUMN IF EXISTS operation_id;

DROP TRIGGER IF EXISTS erasure_span_no_truncate ON core.erasure_span;
DROP TRIGGER IF EXISTS erasure_span_append_only ON core.erasure_span;
DROP TABLE IF EXISTS core.erasure_span;
DROP FUNCTION IF EXISTS core.reject_erasure_span_change();

DROP TRIGGER IF EXISTS erasure_operation_no_truncate ON core.erasure_operation;
DROP TRIGGER IF EXISTS erasure_operation_append_only ON core.erasure_operation;
DROP TABLE IF EXISTS core.erasure_operation;
DROP FUNCTION IF EXISTS core.reject_erasure_operation_change();
