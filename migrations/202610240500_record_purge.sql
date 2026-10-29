-- 本文ごと消す操作の台帳・範囲・派生の門・ビュー・書き直しの記録（ST23 / design D4 / D5 / D7 / D11）。
-- 前進のみ。戻し手順は migrations/202610240500_record_purge.down.sql に置く。
-- migrate() は起動のたびに全版を当てるため、すべて当て直せる形にする。

-- ------------------------------------------------------------------ 消去の操作の台帳（D4）
-- 1 操作 1 行。件数と範囲は操作の単位の事実で、記録ごとの台帳（core.erasure_ledger）には複製しない。
CREATE TABLE IF NOT EXISTS core.erasure_operation (
  id                uuid PRIMARY KEY,
  user_id           uuid NOT NULL,
  kind              text NOT NULL CHECK (kind IN ('user', 'late')),
  parent_id         uuid,                                  -- kind = 'late' のとき元の操作
  stay_id           uuid NOT NULL,                         -- 指定した滞在
  start_at          timestamptz NOT NULL,                  -- 指定した滞在の時間（D13）
  end_at            timestamptz NOT NULL,
  stays             integer NOT NULL CHECK (stays >= 0),
  locations         integer NOT NULL CHECK (locations >= 0),
  archive_locations integer NOT NULL CHECK (archive_locations >= 0),
  archive_items     integer NOT NULL CHECK (archive_items >= 0),
  versions          integer NOT NULL CHECK (versions >= 0),
  at                timestamptz NOT NULL DEFAULT now(),
  txid              xid8 NOT NULL DEFAULT pg_current_xact_id()
);

CREATE INDEX IF NOT EXISTS erasure_operation_by_parent ON core.erasure_operation (parent_id);
CREATE INDEX IF NOT EXISTS erasure_operation_by_stay ON core.erasure_operation (stay_id);

-- 本文を消した滞在と前の版の時間。時刻は丸めない（C3）。
CREATE TABLE IF NOT EXISTS core.erasure_span (
  operation_id uuid NOT NULL,
  event_id     uuid NOT NULL,                              -- FK を張らない（消去は記録より後まで残る）
  scope        text NOT NULL CHECK (scope IN ('event', 'version')),
  version_no   integer,
  start_at     timestamptz NOT NULL,
  end_at       timestamptz NOT NULL CHECK (end_at >= start_at)
);

CREATE INDEX IF NOT EXISTS erasure_span_by_event ON core.erasure_span (event_id, scope);
CREATE INDEX IF NOT EXISTS erasure_span_by_operation ON core.erasure_span (operation_id);

-- 記録ごとの台帳を操作に結ぶ。既存の行は NULL のまま（ST19 / ST21 / ST17 の門は operation_id を見ない）。
ALTER TABLE core.erasure_ledger ADD COLUMN IF NOT EXISTS operation_id uuid;

-- 追記のみ。TRUNCATE は行トリガでは拒めないため文トリガも置く。
-- 関数は表ごとに別にする（共用すると、その関数を落とす古い down 移行が依存で当たらなくなる）。
CREATE OR REPLACE FUNCTION core.reject_erasure_operation_change() RETURNS trigger AS $fn$
BEGIN
  RAISE EXCEPTION '消去の操作の台帳は追記のみ。書き換えも削除も切り詰めもできない（ST23 / design D4）';
END;
$fn$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS erasure_operation_append_only ON core.erasure_operation;
CREATE TRIGGER erasure_operation_append_only
  BEFORE UPDATE OR DELETE ON core.erasure_operation
  FOR EACH ROW EXECUTE FUNCTION core.reject_erasure_operation_change();

DROP TRIGGER IF EXISTS erasure_operation_no_truncate ON core.erasure_operation;
CREATE TRIGGER erasure_operation_no_truncate
  BEFORE TRUNCATE ON core.erasure_operation
  FOR EACH STATEMENT EXECUTE FUNCTION core.reject_erasure_operation_change();

CREATE OR REPLACE FUNCTION core.reject_erasure_span_change() RETURNS trigger AS $fn$
BEGIN
  RAISE EXCEPTION '消去の範囲の表は追記のみ。書き換えも削除も切り詰めもできない（ST23 / design D4）';
END;
$fn$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS erasure_span_append_only ON core.erasure_span;
CREATE TRIGGER erasure_span_append_only
  BEFORE UPDATE OR DELETE ON core.erasure_span
  FOR EACH ROW EXECUTE FUNCTION core.reject_erasure_span_change();

DROP TRIGGER IF EXISTS erasure_span_no_truncate ON core.erasure_span;
CREATE TRIGGER erasure_span_no_truncate
  BEFORE TRUNCATE ON core.erasure_span
  FOR EACH STATEMENT EXECUTE FUNCTION core.reject_erasure_span_change();

-- ------------------------------------------------------------------ 消した時間帯を読む 1 か所（D5）
-- start_at / end_at は、範囲の表（scope = 'event'）があればそこから、無ければ本文から読む（終わりは列 end_time を先に。ST41）。
-- 本文を消した（raw = ''）のに範囲の行が無い滞在は、出来事の時刻の日（Asia/Tokyo）の 00:00〜24:00 とみなす（D14）。
-- 一緒に本文を消した滞在（deleted_by = 'user:purged'）は出さない —— 本文ごと消した時間帯は
-- 指定した滞在の時間だけ（D13）で、吸収された滞在の時間を窓にしない。
-- 座標を含む payload と原文 raw は載せない。
-- 本文を消した印 purged と操作 operation_id は core.stay_erased_purge にだけ載せ、core.stay_erased は
-- ST41 までの 6 列のまま、その写しにする —— 列を足すと、当て直しで ST41（202610231200_event_end_time）の
-- CREATE OR REPLACE VIEW が「cannot drop columns from view」で落ちる。
CREATE OR REPLACE VIEW core.stay_erased_purge AS
  SELECT e.id,
         e.user_id,
         coalesce(s.start_at,
                  CASE WHEN e.raw = '' THEN d.day_start ELSE e.event_time END) AS start_at,
         coalesce(s.end_at,
                  CASE WHEN e.raw = '' THEN d.day_start + interval '1 day'
                       ELSE coalesce(CASE WHEN e.end_time > e.event_time THEN e.end_time END,
                                     core.try_timestamptz(e.payload->>'end'),
                                     e.event_time + interval '1 microsecond') END) AS end_at,
         e.deleted_at,
         e.deleted_by,
         (e.raw = '') AS purged,
         s.operation_id
    FROM core.event e
    LEFT JOIN LATERAL (
           SELECT sp.start_at, sp.end_at, sp.operation_id
             FROM core.erasure_span sp
             JOIN core.erasure_operation op ON op.id = sp.operation_id
            WHERE sp.event_id = e.id AND sp.scope = 'event'
            ORDER BY op.at, op.id
            LIMIT 1
         ) s ON true
    CROSS JOIN LATERAL (
           SELECT date_trunc('day', e.event_time AT TIME ZONE 'Asia/Tokyo') AT TIME ZONE 'Asia/Tokyo' AS day_start
         ) d
   WHERE e.logical_source = 's01-stay'
     AND e.origin = 'derived'
     AND e.deleted_at IS NOT NULL
     AND e.deleted_by IS DISTINCT FROM 'user:purged';

CREATE OR REPLACE VIEW core.stay_erased AS
  SELECT id, user_id, start_at, end_at, deleted_at, deleted_by
    FROM core.stay_erased_purge;

-- ------------------------------------------------------------------ 派生の門（D7）
-- 収集した記録の門（core.require_version_or_ledger）は変えない。派生の記録（滞在）の本文の消去だけを、
-- COMMIT の時に見る。本文を消した行を「raw = ''」の印で読む D5 / D6 を守るため、
-- 消去の形でない「原文だけ空にする」書き換えと、書き戻しも拒む。
CREATE OR REPLACE FUNCTION core.require_derived_erasure_ledger() RETURNS trigger AS $fn$
DECLARE
  is_erasure boolean;
BEGIN
  IF OLD.origin <> 'derived' THEN
    RETURN NULL;
  END IF;

  -- 原文を空のまま解析済みだけ差し替える形も書き戻し（収集した記録の門 gates.sql と同じ穴）
  IF OLD.raw = '' AND (NEW.raw <> '' OR NEW.payload <> '{}'::jsonb) THEN
    RAISE EXCEPTION '本文を消した滞在へ本文は書き戻せない（ST23 / design D7）';
  END IF;

  IF OLD.raw <> '' AND NEW.raw = '' THEN
    is_erasure :=
          NEW.payload = '{}'::jsonb
      AND NEW.event_time   = OLD.event_time
      AND NEW.content_hash = OLD.content_hash;
    IF NOT is_erasure THEN
      RAISE EXCEPTION '滞在の本文の消去は、原文と解析済みを一緒に空にする形だけが通る（ST23 / design D7）';
    END IF;
    IF NOT EXISTS (
      SELECT 1 FROM core.erasure_ledger l
       WHERE l.event_id = NEW.id AND l.scope = 'event'
         AND l.txid = pg_current_xact_id()
    ) THEN
      RAISE EXCEPTION '滞在の本文の消去は、同じまとまりに消去の台帳の行があるときだけ通る（ST23 / design D7）';
    END IF;
    IF EXISTS (
      SELECT 1 FROM core.event_version v WHERE v.event_id = NEW.id AND v.raw <> ''
    ) THEN
      RAISE EXCEPTION '滞在の本文の消去は、前の版のすべても同じまとまりで消す（ST23 / design D7）';
    END IF;
  END IF;
  RETURN NULL;
END;
$fn$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS event_requires_derived_ledger ON core.event;
CREATE CONSTRAINT TRIGGER event_requires_derived_ledger
  AFTER UPDATE ON core.event
  DEFERRABLE INITIALLY DEFERRED
  FOR EACH ROW EXECUTE FUNCTION core.require_derived_erasure_ledger();

-- ------------------------------------------------------------------ 夜の書き直しの記録（D11）
-- 記録の値を持たない（表の名前・時刻・結果・SQLSTATE と、書き直しの直前のスナップショットだけ）。
-- snapshot は次の晩の対象の窓の区切り（それから見えない消去の操作が、まだ書き直していない分）。取る前に落ちたら NULL。
CREATE TABLE IF NOT EXISTS core.purge_compaction (
  id          bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  table_name  text NOT NULL CHECK (table_name IN ('core.event', 'core.event_version')),
  started_at  timestamptz NOT NULL,
  finished_at timestamptz NOT NULL,
  outcome     text NOT NULL CHECK (outcome IN ('rewritten', 'not_rewritten', 'lock_timeout', 'failed')),
  sqlstate    text,
  snapshot    pg_snapshot
);
-- 列を足す前にこの移行を当てた DB にも足す（移行は起動のたびに全版を当てる）
ALTER TABLE core.purge_compaction ADD COLUMN IF NOT EXISTS snapshot pg_snapshot;

CREATE INDEX IF NOT EXISTS purge_compaction_by_table ON core.purge_compaction (table_name, finished_at);

-- 書き直し（VACUUM FULL）の権限。表ごとに 2 表だけ（grants.sql の ALL TABLES には広げない）。
-- 役割は db-roles.sh が作る。役割より先に当たる検査（管理者だけの DB）では付与を飛ばす。
DO $$
BEGIN
  IF EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'ashiato_app') THEN
    GRANT MAINTAIN ON core.event, core.event_version TO ashiato_app;
  END IF;
END
$$;
