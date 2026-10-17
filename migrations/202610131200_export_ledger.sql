-- 書き出しの台帳（ST33 / design D10）。1 回の書き出しにつき 2 行（始まりと終わり）、追記のみ。
-- 前進のみ。戻し手順は migrations/202610131200_export_ledger.down.sql に置く。
-- migrate() は起動のたびに全版を当てるため、すべて当て直せる形にする。
-- 合言葉も、合言葉から作った値（ハッシュ・鍵）も持たない（Q9）。user_id も持たない（全利用者で 1 つ。D12）。

CREATE TABLE IF NOT EXISTS core.export_ledger (
  seq          bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  export_id    uuid NOT NULL,
  kind         text NOT NULL CHECK (kind IN ('start', 'finish')),
  at           timestamptz NOT NULL DEFAULT now(),
  route        text NOT NULL CHECK (route IN ('pc-encrypted-zip')),
  file         text NOT NULL,
  snapshot_at  timestamptz,
  result       text CHECK (result IN ('done', 'failed')),
  rows         jsonb,
  bytes        bigint,
  written      bytea,
  digest       text,
  reason       text,
  CHECK ((kind = 'start') = (result IS NULL))
);

CREATE INDEX IF NOT EXISTS export_ledger_by_export
  ON core.export_ledger (export_id, seq);

-- この台帳を開ける操作は無い。TRUNCATE は行トリガでは拒めないため、文トリガも置く。
-- 他の台帳の関数を共用すると、その関数を落とす古い down 移行が依存で当たらなくなる。
CREATE OR REPLACE FUNCTION core.reject_export_ledger_change() RETURNS trigger AS $fn$
BEGIN
  RAISE EXCEPTION '書き出しの台帳は追記のみ。書き換えも削除も切り詰めもできない（ST33 / design D10）';
END;
$fn$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS export_ledger_append_only ON core.export_ledger;
CREATE TRIGGER export_ledger_append_only
  BEFORE UPDATE OR DELETE ON core.export_ledger
  FOR EACH ROW EXECUTE FUNCTION core.reject_export_ledger_change();

DROP TRIGGER IF EXISTS export_ledger_no_truncate ON core.export_ledger;
CREATE TRIGGER export_ledger_no_truncate
  BEFORE TRUNCATE ON core.export_ledger
  FOR EACH STATEMENT EXECUTE FUNCTION core.reject_export_ledger_change();
