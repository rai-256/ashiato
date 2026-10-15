-- 主観の記録の錠と、紐づけ先の種類・通知の時刻の台帳（ST17 / FR-48。design D2 / D9 / D15 / D16）
-- 前進のみ。戻し手順は migrations/202610141000_subjective_log.down.sql に置く。
--
-- **主観の記録そのものは `core.event` に入る**（`origin='authored'` /
-- `logical_source='s01-feeling'`）。この版が足すのは 4 つだけ ——
--   1. 紐づけ先の種類の登録簿（追記のみ。D15）
--   2. 通知の時刻の台帳（追記のみ。D9）
--   3. 主観の行の 3 つの錠（即時の錠・COMMIT 時の門・行の削除の拒否。D2）
--   4. 論理ソースの登録簿の 1 行
--
-- **ST03 / ST19 / ST21 の関数は書き換えない**（design D2）—— 変えると各 `.down.sql` と
-- `tools/check-immutable.sh` の前提が動く。関数は主観のために新しく作る。
--
-- **当て直せる形にする**（`migrate()` は起動のたびに全版を当てる）。

-- ------------------------------------------------------------------ 紐づけ先の種類（D15）
--
-- 種類の名前は原文の `target.kind` と同じ文字列。**主観の行から外部キーで縛らない**
-- （鍵ではなく索引。C5）—— 種類を足すときに既存の行に触れない（FR-38）。
CREATE TABLE IF NOT EXISTS core.feeling_target_kind (
  kind       text PRIMARY KEY CHECK (kind ~ '^[a-z][a-z0-9-]*$'),
  label      text NOT NULL,
  created_at timestamptz NOT NULL DEFAULT now()
);

-- 追記のみ。`TRUNCATE` は行トリガで撃たれないので、文トリガを別に置く。
-- **ST03 の `core.reject_truncate()` を使わない**（使うと `202609120944_gates.down.sql` が
-- 依存で当たらなくなる。ST16 / ST19 / ST21 と同じ理由）。
CREATE OR REPLACE FUNCTION core.reject_feeling_ledger_change() RETURNS trigger AS $fn$
BEGIN
  RAISE EXCEPTION '主観の台帳（%）は追記のみ。書き換えも削除も切り詰めもできない（FR-48）', TG_TABLE_NAME;
END;
$fn$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS feeling_target_kind_append_only ON core.feeling_target_kind;
CREATE TRIGGER feeling_target_kind_append_only
  BEFORE UPDATE OR DELETE ON core.feeling_target_kind
  FOR EACH ROW EXECUTE FUNCTION core.reject_feeling_ledger_change();

DROP TRIGGER IF EXISTS feeling_target_kind_no_truncate ON core.feeling_target_kind;
CREATE TRIGGER feeling_target_kind_no_truncate
  BEFORE TRUNCATE ON core.feeling_target_kind
  FOR EACH STATEMENT EXECUTE FUNCTION core.reject_feeling_ledger_change();

INSERT INTO core.feeling_target_kind (kind, label)
VALUES ('day', '日'), ('stay', '滞在')
ON CONFLICT (kind) DO NOTHING;

-- ------------------------------------------------------------------ 通知の時刻の台帳（D9）
--
-- いまの時刻は利用者ごとに `id` が最大の行。無ければ 21:00（コードの定数）。秒は持たない。
CREATE TABLE IF NOT EXISTS core.feeling_reminder (
  id         bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  user_id    uuid NOT NULL,
  at         time NOT NULL,
  created_at timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS feeling_reminder_by_user ON core.feeling_reminder (user_id, id);

DROP TRIGGER IF EXISTS feeling_reminder_append_only ON core.feeling_reminder;
CREATE TRIGGER feeling_reminder_append_only
  BEFORE UPDATE OR DELETE ON core.feeling_reminder
  FOR EACH ROW EXECUTE FUNCTION core.reject_feeling_ledger_change();

DROP TRIGGER IF EXISTS feeling_reminder_no_truncate ON core.feeling_reminder;
CREATE TRIGGER feeling_reminder_no_truncate
  BEFORE TRUNCATE ON core.feeling_reminder
  FOR EACH STATEMENT EXECUTE FUNCTION core.reject_feeling_ledger_change();

-- ------------------------------------------------------------------ 主観の記録の即時の錠（D2 の 1）
--
-- 通すのは **削除の印（`deleted_at` / `deleted_by`）と感度**、そして下の門が見る
-- **台帳つきの消去**だけ（主観の記録も記録なので FR-50 / FR-51 が及ぶ）。
CREATE OR REPLACE FUNCTION core.reject_feeling_rewrite() RETURNS trigger AS $fn$
BEGIN
  -- **主観の記録へ付け替える向きを塞ぐ**（ST03 の R114 / ST19 / ST21 と同じ型）。
  IF NEW.logical_source = 's01-feeling'
     AND OLD.logical_source IS DISTINCT FROM 's01-feeling' THEN
    RAISE EXCEPTION '他の記録を主観の記録へ付け替えることはできない（FR-48）';
  END IF;

  IF OLD.logical_source = 's01-feeling' THEN
    IF NEW.logical_source IS DISTINCT FROM OLD.logical_source THEN
      RAISE EXCEPTION '主観の記録の論理ソースは書き換えられない（FR-48 / FR-22）';
    END IF;
    IF NEW.origin IS DISTINCT FROM OLD.origin THEN
      RAISE EXCEPTION '主観の記録の由来は書き換えられない（FR-48 / FR-25）';
    END IF;
    IF NEW.id IS DISTINCT FROM OLD.id THEN
      RAISE EXCEPTION '主観の記録の識別子は書き換えられない（FR-21 / FR-48）';
    END IF;
    IF NEW.user_id IS DISTINCT FROM OLD.user_id THEN
      RAISE EXCEPTION '主観の記録の利用者は書き換えられない（FR-48）';
    END IF;
    IF NEW.event_time IS DISTINCT FROM OLD.event_time THEN
      RAISE EXCEPTION '主観の記録の出来事の時刻は書き換えられない（FR-48）';
    END IF;
    IF NEW.tz_offset_min IS DISTINCT FROM OLD.tz_offset_min
       OR NEW.tz_id IS DISTINCT FROM OLD.tz_id THEN
      RAISE EXCEPTION '主観の記録の出来事の時刻の地域は書き換えられない（FR-48）';
    END IF;
    IF NEW.ingest_time IS DISTINCT FROM OLD.ingest_time THEN
      RAISE EXCEPTION '主観の記録が D-01 に入った時刻は書き換えられない（FR-19 / FR-48）';
    END IF;
    IF NEW.external_id IS DISTINCT FROM OLD.external_id
       OR NEW.external_ref IS DISTINCT FROM OLD.external_ref THEN
      RAISE EXCEPTION '主観の記録の外部識別子は書き換えられない（FR-23 / FR-48）';
    END IF;
    IF NEW.device_id IS DISTINCT FROM OLD.device_id THEN
      RAISE EXCEPTION '主観の記録の端末識別子は書き換えられない（FR-48）';
    END IF;
    IF NEW.schema_version IS DISTINCT FROM OLD.schema_version
       OR NEW.unit_system IS DISTINCT FROM OLD.unit_system
       OR NEW.crs IS DISTINCT FROM OLD.crs
       OR NEW.source_updated_at IS DISTINCT FROM OLD.source_updated_at THEN
      RAISE EXCEPTION '主観の記録のエンベロープは書き換えられない（FR-48 / FR-28）';
    END IF;
    -- **`raw` / `payload` / `content_hash` はここでは見ない。** 値もひとことも原文と解析済みの
    -- 中にあり、守るのは下の門（消去の形と台帳を見る）—— FR-51 の消去を通すため、
    -- 即時の錠で閉じ切ることはできない。
  END IF;
  RETURN NEW;
END;
$fn$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS event_feeling_immutable ON core.event;
CREATE TRIGGER event_feeling_immutable
  BEFORE UPDATE ON core.event
  FOR EACH ROW EXECUTE FUNCTION core.reject_feeling_rewrite();

-- ------------------------------------------------------------------ 主観の記録の門（COMMIT 時。D2 の 2）
--
-- 本文（原文・解析済み・内容の鍵）が動く書き換えは、**消去の形で、かつ同じまとまりに
-- その記録の台帳の行があるときだけ**通す。
-- **「その記録の」を見る**（`event_id = NEW.id`）—— 照合しないと台帳 1 行で何件でも消去できる。
-- **消去の形で絞る**（ST03 の R51 / R95）—— 台帳があるだけで通すと、
-- 原文（唯一の復元元）を消しながらもっともらしい解析済みを植えられる。
CREATE OR REPLACE FUNCTION core.require_feeling_erasure_ledger() RETURNS trigger AS $fn$
DECLARE
  content_changed boolean;
  is_erasure      boolean;
BEGIN
  IF OLD.logical_source <> 's01-feeling' THEN
    RETURN NULL;   -- 主観の記録以外はこの門の対象外（ST03 / ST19 / ST21 の門が別に見る）
  END IF;

  content_changed :=
       NEW.raw          IS DISTINCT FROM OLD.raw
    OR NEW.payload      IS DISTINCT FROM OLD.payload
    OR NEW.content_hash IS DISTINCT FROM OLD.content_hash;

  IF NOT content_changed THEN
    RETURN NULL;   -- 削除の印・感度だけの書き換えは素通し（FR-50 / PERM-2）
  END IF;

  is_erasure :=
        NEW.raw = '' AND OLD.raw <> ''
    AND NEW.payload = '{}'::jsonb
    AND NEW.event_time   = OLD.event_time
    AND NEW.content_hash = OLD.content_hash;

  IF NOT is_erasure THEN
    RAISE EXCEPTION '主観の記録は書き換えられない。通るのは本文の消去だけ（FR-48 / FR-51）';
  END IF;

  IF NOT EXISTS (
    SELECT 1 FROM core.erasure_ledger l
     WHERE l.event_id = NEW.id AND l.scope = 'event'
       AND l.txid = pg_current_xact_id()
  ) THEN
    RAISE EXCEPTION '主観の記録の消去は、同じまとまりにその記録の消去の台帳の行があるときだけ通る（FR-51）';
  END IF;
  RETURN NULL;
END;
$fn$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS event_feeling_requires_ledger ON core.event;
CREATE CONSTRAINT TRIGGER event_feeling_requires_ledger
  AFTER UPDATE ON core.event
  DEFERRABLE INITIALLY DEFERRED
  FOR EACH ROW EXECUTE FUNCTION core.require_feeling_erasure_ledger();

-- ------------------------------------------------------------------ 主観の記録の行の削除（D2 の 3）
--
-- **表の切り詰めは ST03 の文トリガ（`event_no_truncate`）が既に拒む**ので、
-- ここで足すのは行の削除だけ。
CREATE OR REPLACE FUNCTION core.reject_feeling_delete() RETURNS trigger AS $fn$
BEGIN
  IF OLD.logical_source = 's01-feeling' THEN
    RAISE EXCEPTION '主観の記録は行ごと消せない（FR-48）。消すなら削除の印か台帳つきの消去';
  END IF;
  RETURN OLD;
END;
$fn$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS event_feeling_no_delete ON core.event;
CREATE TRIGGER event_feeling_no_delete
  BEFORE DELETE ON core.event
  FOR EACH ROW EXECUTE FUNCTION core.reject_feeling_delete();

-- ------------------------------------------------------------------ 主観の記録の論理ソース
--
-- **`external_id_kind = 'none'`** —— 主観の記録は外部識別子を持たず、内容の鍵で畳まれる。
-- `expected_gap_sec` は稼働状況に使われない（`coverage::must_sources()` は NFR-13 の 5 ソースだけ）
-- が、列が `NOT NULL` なので 1 日を入れる。
INSERT INTO core.source (logical_source, display_name, expected_gap_sec, external_id_kind)
VALUES ('s01-feeling', '主観', 86400, 'none')
ON CONFLICT (logical_source) DO NOTHING;
