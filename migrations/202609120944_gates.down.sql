-- 戻し手順（202609120944_gates）。**不可逆** ——
-- 門を外すと、履歴を残さない書き換えと台帳を残さない消去が通るようになる。
-- 通った後で門を戻しても、**その間に消えた原文は戻らない**（深掘り Q10 / Q17 / Q23）。
DROP TRIGGER IF EXISTS heartbeat_no_truncate ON core.heartbeat;
DROP TRIGGER IF EXISTS erasure_ledger_no_truncate ON core.erasure_ledger;
DROP TRIGGER IF EXISTS event_version_no_truncate ON core.event_version;
DROP TRIGGER IF EXISTS event_no_truncate ON core.event;
DROP TRIGGER IF EXISTS event_version_no_delete ON core.event_version;
DROP TRIGGER IF EXISTS event_no_delete ON core.event;
DROP TRIGGER IF EXISTS erasure_ledger_append_only ON core.erasure_ledger;
DROP TRIGGER IF EXISTS version_requires_ledger ON core.event_version;
DROP TRIGGER IF EXISTS event_version_immutable ON core.event_version;
DROP TRIGGER IF EXISTS event_requires_version ON core.event;
DROP FUNCTION IF EXISTS core.reject_version_delete();
DROP FUNCTION IF EXISTS core.reject_collected_delete();
DROP FUNCTION IF EXISTS core.reject_truncate();
DROP FUNCTION IF EXISTS core.reject_ledger_change();
DROP FUNCTION IF EXISTS core.require_ledger_for_erasure();
DROP FUNCTION IF EXISTS core.reject_version_rewrite();
DROP FUNCTION IF EXISTS core.require_version_or_ledger();
-- 0004 の錠へ戻す（4 列を再び凍結する）
CREATE OR REPLACE FUNCTION core.reject_collected_rewrite() RETURNS trigger AS $fn$
BEGIN
  IF OLD.origin = 'collected' AND (
       NEW.raw          IS DISTINCT FROM OLD.raw
    OR NEW.payload      IS DISTINCT FROM OLD.payload
    OR NEW.event_time   IS DISTINCT FROM OLD.event_time
    OR NEW.content_hash IS DISTINCT FROM OLD.content_hash
    OR NEW.ingest_time  IS DISTINCT FROM OLD.ingest_time
  ) THEN
    RAISE EXCEPTION '収集した記録の原文・解析済み・出来事の時刻・冪等キー・格納の時刻は書き換えられない（FR-30）';
  END IF;
  IF OLD.origin = 'collected' AND NEW.origin IS DISTINCT FROM OLD.origin THEN
    RAISE EXCEPTION '収集した記録の由来は変えられない（FR-25 / FR-30）';
  END IF;
  RETURN NEW;
END;
$fn$ LANGUAGE plpgsql;
