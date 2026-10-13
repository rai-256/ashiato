-- 場所の器と、場所の記録の錠（ST21 / FR-46。design D1 / D5 / D16）
-- 前進のみ。戻し手順は migrations/202610020030_places.down.sql に置く。
--
-- **場所の記録そのものは `core.event` に入る**（`origin='authored'` /
-- `logical_source='s01-place'`）。この版が足すのは 3 つだけ ——
--   1. 場所の器の表（記録を束ねる識別子。記録ではないので `core.event` に入れない）
--   2. 場所の記録の行の錠（即時の錠・COMMIT 時の門・行の削除の拒否）
--   3. 登録簿の 1 行
--
-- **ST03 / ST19 の関数（`core.reject_collected_rewrite` / `core.reject_claim_rewrite` など）は
-- 書き換えない**（design D5）—— 変えると両者の `.down.sql` と `tools/check-immutable.sh` の
-- 前提が動く。関数は場所のために新しく作る。
--
-- **当て直せる形にする**（`migrate()` は起動のたびに全版を当てる）。

-- ------------------------------------------------------------------ 場所の器（D1）
--
-- 識別子は画面が乱数で決めて `POST /places` で渡す（C1）。サーバは名前・座標から計算しない。
-- **並びは `seq`（単調増加）で決める。`created_at` では決められない**（ST19 D14）——
-- `now()` はトランザクションの開始時刻なので、同じまとまりで作った器は全部同時刻になる。
CREATE TABLE IF NOT EXISTS core.place (
  id         uuid PRIMARY KEY,
  seq        bigint GENERATED ALWAYS AS IDENTITY,
  user_id    uuid NOT NULL,
  created_at timestamptz NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX IF NOT EXISTS place_id_user ON core.place (id, user_id);
CREATE INDEX IF NOT EXISTS place_by_user ON core.place (user_id, seq);

-- 器は追記のみ。開ける必要のある操作が 1 つも無い。
-- `TRUNCATE` は行トリガで撃たれないので、文トリガを別に置く。
--
-- **ST03 の `core.reject_truncate()` を使わない** —— 使うと、その関数を落とす
-- `202609120944_gates.down.sql` が依存で当たらなくなる（ST16 / ST19 と同じ理由）。
CREATE OR REPLACE FUNCTION core.reject_place_change() RETURNS trigger AS $fn$
BEGIN
  RAISE EXCEPTION '場所の器（%）は追記のみ。書き換えも削除も切り詰めもできない（FR-46）', TG_TABLE_NAME;
END;
$fn$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS place_append_only ON core.place;
CREATE TRIGGER place_append_only
  BEFORE UPDATE OR DELETE ON core.place
  FOR EACH ROW EXECUTE FUNCTION core.reject_place_change();

DROP TRIGGER IF EXISTS place_no_truncate ON core.place;
CREATE TRIGGER place_no_truncate
  BEFORE TRUNCATE ON core.place
  FOR EACH STATEMENT EXECUTE FUNCTION core.reject_place_change();

-- ------------------------------------------------------------------ 場所の記録の即時の錠（D5 の 1）
--
-- 通すのは **削除の印（`deleted_at` / `deleted_by`）と感度**、そして下の門が見る
-- **台帳つきの消去**だけ（場所の記録も記録なので FR-50 / FR-51 が及ぶ）。
CREATE OR REPLACE FUNCTION core.reject_place_record_rewrite() RETURNS trigger AS $fn$
BEGIN
  -- **場所の記録へ付け替える向きを塞ぐ**（ST03 の R114 / ST19 と同じ型）。
  IF NEW.logical_source = 's01-place'
     AND OLD.logical_source IS DISTINCT FROM 's01-place' THEN
    RAISE EXCEPTION '他の記録を場所の記録へ付け替えることはできない（FR-46）';
  END IF;

  IF OLD.logical_source = 's01-place' THEN
    IF NEW.logical_source IS DISTINCT FROM OLD.logical_source THEN
      RAISE EXCEPTION '場所の記録の論理ソースは書き換えられない（FR-46 / FR-22）';
    END IF;
    IF NEW.origin IS DISTINCT FROM OLD.origin THEN
      RAISE EXCEPTION '場所の記録の由来は書き換えられない（FR-46 / FR-25）';
    END IF;
    IF NEW.id IS DISTINCT FROM OLD.id THEN
      RAISE EXCEPTION '場所の記録の識別子は書き換えられない（FR-21 / FR-46）';
    END IF;
    IF NEW.user_id IS DISTINCT FROM OLD.user_id THEN
      RAISE EXCEPTION '場所の記録の利用者は書き換えられない（FR-46）';
    END IF;
    IF NEW.event_time IS DISTINCT FROM OLD.event_time THEN
      RAISE EXCEPTION '場所の記録を書いた日時は書き換えられない（FR-46）';
    END IF;
    IF NEW.tz_offset_min IS DISTINCT FROM OLD.tz_offset_min
       OR NEW.tz_id IS DISTINCT FROM OLD.tz_id THEN
      RAISE EXCEPTION '場所の記録を書いた日時の地域は書き換えられない（FR-46）';
    END IF;
    IF NEW.ingest_time IS DISTINCT FROM OLD.ingest_time THEN
      RAISE EXCEPTION '場所の記録が D-01 に入った時刻は書き換えられない（FR-19 / FR-46）';
    END IF;
    IF NEW.external_id IS DISTINCT FROM OLD.external_id
       OR NEW.external_ref IS DISTINCT FROM OLD.external_ref THEN
      RAISE EXCEPTION '場所の記録の外部識別子は書き換えられない（FR-23 / FR-46）';
    END IF;
    IF NEW.device_id IS DISTINCT FROM OLD.device_id THEN
      RAISE EXCEPTION '場所の記録の端末識別子は書き換えられない（FR-46）';
    END IF;
    IF NEW.schema_version IS DISTINCT FROM OLD.schema_version
       OR NEW.unit_system IS DISTINCT FROM OLD.unit_system
       OR NEW.crs IS DISTINCT FROM OLD.crs
       OR NEW.source_updated_at IS DISTINCT FROM OLD.source_updated_at THEN
      RAISE EXCEPTION '場所の記録のエンベロープは書き換えられない（FR-46 / FR-28）';
    END IF;
    -- **`raw` / `payload` / `content_hash` はここでは見ない。** 座標も名前も原文と解析済みの
    -- 中にあり、守るのは下の門（消去の形と台帳を見る）—— FR-51 の消去を通すため、
    -- 即時の錠で閉じ切ることはできない。
  END IF;
  RETURN NEW;
END;
$fn$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS event_place_immutable ON core.event;
CREATE TRIGGER event_place_immutable
  BEFORE UPDATE ON core.event
  FOR EACH ROW EXECUTE FUNCTION core.reject_place_record_rewrite();

-- ------------------------------------------------------------------ 場所の記録の門（COMMIT 時。D5 の 2）
--
-- 本文（原文・解析済み・内容の鍵）が動く書き換えは、**消去の形で、かつ同じまとまりに
-- その記録の台帳の行があるときだけ**通す。
-- **「その記録の」を見る**（`event_id = NEW.id`）—— 照合しないと台帳 1 行で何件でも消去できる。
-- **消去の形で絞る**（ST03 の R51 / R95）—— 台帳があるだけで通すと、
-- 原文（唯一の復元元）を消しながらもっともらしい解析済みを植えられる。
CREATE OR REPLACE FUNCTION core.require_place_erasure_ledger() RETURNS trigger AS $fn$
DECLARE
  content_changed boolean;
  is_erasure      boolean;
BEGIN
  IF OLD.logical_source <> 's01-place' THEN
    RETURN NULL;   -- 場所の記録以外はこの門の対象外（ST03 / ST19 の門が別に見る）
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
    RAISE EXCEPTION '場所の記録は書き換えられない。通るのは本文の消去だけ（FR-46 / FR-51）';
  END IF;

  IF NOT EXISTS (
    SELECT 1 FROM core.erasure_ledger l
     WHERE l.event_id = NEW.id AND l.scope = 'event'
       AND l.txid = pg_current_xact_id()
  ) THEN
    RAISE EXCEPTION '場所の記録の消去は、同じまとまりにその記録の消去の台帳の行があるときだけ通る（FR-51）';
  END IF;
  RETURN NULL;
END;
$fn$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS event_place_requires_ledger ON core.event;
CREATE CONSTRAINT TRIGGER event_place_requires_ledger
  AFTER UPDATE ON core.event
  DEFERRABLE INITIALLY DEFERRED
  FOR EACH ROW EXECUTE FUNCTION core.require_place_erasure_ledger();

-- ------------------------------------------------------------------ 場所の記録の行の削除（D5 の 3）
--
-- **表の切り詰めは ST03 の文トリガ（`event_no_truncate`）が既に拒む**ので、
-- ここで足すのは行の削除だけ。
CREATE OR REPLACE FUNCTION core.reject_place_record_delete() RETURNS trigger AS $fn$
BEGIN
  IF OLD.logical_source = 's01-place' THEN
    RAISE EXCEPTION '場所の記録は行ごと消せない（FR-46）。消すなら削除の印か台帳つきの消去';
  END IF;
  RETURN OLD;
END;
$fn$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS event_place_no_delete ON core.event;
CREATE TRIGGER event_place_no_delete
  BEFORE DELETE ON core.event
  FOR EACH ROW EXECUTE FUNCTION core.reject_place_record_delete();

-- ------------------------------------------------------------------ 場所の記録の論理ソース
--
-- **`external_id_kind = 'none'`** —— 場所の記録は外部識別子を持たず、内容の鍵（乱数入りの原文）で
-- 畳まれる。`expected_gap_sec` は稼働状況に使われない（`coverage::must_sources()` は
-- NFR-13 の 5 ソースだけ）が、列が `NOT NULL` なので 1 日を入れる。
INSERT INTO core.source (logical_source, display_name, expected_gap_sec, external_id_kind)
VALUES ('s01-place', '場所', 86400, 'none')
ON CONFLICT (logical_source) DO NOTHING;
