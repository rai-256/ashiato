-- 滞在を消した・戻した操作の追記専用台帳（ST22 / design D1・D12）。
-- 前進のみ。戻し手順は migrations/202609271716_deletion_ledger.down.sql に置く。
-- migrate() は起動のたびに全版を当てるため、すべて当て直せる形にする。

CREATE TABLE IF NOT EXISTS core.deletion_ledger (
  seq            bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  event_id       uuid NOT NULL,
  user_id        uuid NOT NULL,
  logical_source text NOT NULL,
  action         text NOT NULL CHECK (action IN ('erase', 'restore')),
  cause_event_id uuid NOT NULL,
  mark           text NOT NULL,
  at             timestamptz NOT NULL DEFAULT now(),
  txid           xid8 NOT NULL DEFAULT pg_current_xact_id()
);

CREATE INDEX IF NOT EXISTS deletion_ledger_by_event
  ON core.deletion_ledger (event_id, seq);
CREATE INDEX IF NOT EXISTS deletion_ledger_by_cause
  ON core.deletion_ledger (cause_event_id, seq);

-- この台帳を開ける操作は無い。TRUNCATE は行トリガでは拒めないため、文トリガも置く。
-- 他の台帳の関数を共用すると、その関数を落とす古い down 移行が依存で当たらなくなる。
CREATE OR REPLACE FUNCTION core.reject_deletion_ledger_change() RETURNS trigger AS $fn$
BEGIN
  RAISE EXCEPTION '削除の台帳は追記のみ。書き換えも削除も切り詰めもできない（ST22 / design D1）';
END;
$fn$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS deletion_ledger_append_only ON core.deletion_ledger;
CREATE TRIGGER deletion_ledger_append_only
  BEFORE UPDATE OR DELETE ON core.deletion_ledger
  FOR EACH ROW EXECUTE FUNCTION core.reject_deletion_ledger_change();

DROP TRIGGER IF EXISTS deletion_ledger_no_truncate ON core.deletion_ledger;
CREATE TRIGGER deletion_ledger_no_truncate
  BEFORE TRUNCATE ON core.deletion_ledger
  FOR EACH STATEMENT EXECUTE FUNCTION core.reject_deletion_ledger_change();

-- 削除済み滞在を読む経路には、識別子・時刻範囲・削除印だけを載せる。
-- 座標を含む payload と原文 raw は、このビューから読み出せない。
CREATE OR REPLACE VIEW core.stay_erased AS
  SELECT id,
         user_id,
         event_time AS start_at,
         coalesce((payload->>'end')::timestamptz, event_time) AS end_at,
         deleted_at,
         deleted_by
    FROM core.event
   WHERE logical_source = 's01-stay'
     AND origin = 'derived'
     AND deleted_at IS NOT NULL;
