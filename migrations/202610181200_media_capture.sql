-- SPDX-License-Identifier: AGPL-3.0-only
-- ST09: 写真と動画の取り込み（design D7 / D13 / D15 / D17）。
-- 前進のみ。戻し手順は migrations/202610181200_media_capture.down.sql に置く。
-- migrate() は起動のたびに全版を当てるため、すべて当て直せる形にする。

-- ------------------------------------------------------------------ 1. 登録簿に 3 つのソースを足す
-- 想定間隔・判定の対象かどうか・警告の日数は c01-photo の行と同じ値（design D15 の 1）。
-- 既にある行は動かさない（本人が変えた値を戻さない）。
INSERT INTO core.source (logical_source, display_name, expected_gap_sec, external_id_kind,
                         outage_target, alert_after_days)
SELECT v.logical_source, v.display_name, p.expected_gap_sec, 'record', p.outage_target, p.alert_after_days
  FROM core.source p
  CROSS JOIN (VALUES
    ('c01-screenshot', '端末に保存されたスクリーンショット'),
    ('c01-image',      '端末に保存された画像'),
    ('c01-video',      '端末に保存された動画')
  ) AS v (logical_source, display_name)
 WHERE p.logical_source = 'c01-photo'
    ON CONFLICT (logical_source) DO NOTHING;

-- ------------------------------------------------------------------ 2. 4 つのソースは外部識別子を要る
-- **そのソースの記録が 0 件のときだけ**（ST08 D7 と同じ）。記録のある行を後から変えると、
-- 識別子なしで入った行と畳めない。
UPDATE core.source s
   SET external_id_kind = 'record'
 WHERE s.logical_source IN ('c01-photo', 'c01-screenshot', 'c01-image', 'c01-video')
   AND s.external_id_kind <> 'record'
   AND NOT EXISTS (SELECT 1 FROM core.event e WHERE e.logical_source = s.logical_source);

-- ------------------------------------------------------------------ 3. 収集開始日の印と、生存信号の接続の欄
-- 印は**列を足した回だけ**当てる。毎回当てると、本人が `record` へ戻した（D13 の反転）のを
-- 起動のたびに `heartbeat` へ撃ち戻す。
DO $do$
BEGIN
  IF NOT EXISTS (SELECT 1 FROM information_schema.columns
                  WHERE table_schema = 'core' AND table_name = 'source'
                    AND column_name = 'started_on_basis') THEN
    ALTER TABLE core.source ADD COLUMN started_on_basis text NOT NULL DEFAULT 'record';
    ALTER TABLE core.source ADD CONSTRAINT source_started_on_basis_known
      CHECK (started_on_basis IN ('record', 'heartbeat'));
    UPDATE core.source SET started_on_basis = 'heartbeat'
     WHERE logical_source IN ('c01-photo', 'c01-screenshot', 'c01-image', 'c01-video');
  END IF;
END
$do$;

-- 送る契機の時点で接続があったか。無い要求の行は NULL（分からない。D7）。
-- 書き換え禁止の門はそのまま（列を足すだけで、既存の行は書き換えない）。
ALTER TABLE core.heartbeat ADD COLUMN IF NOT EXISTS network boolean NULL;

-- ------------------------------------------------------------------ 4. 3 つの表（D11 / D10 / D17）
-- 画像の原本を受け取った行。**ファイルの場所を持たない**（C2）。
CREATE TABLE IF NOT EXISTS core.media_original (
  user_id        uuid        NOT NULL,
  sha256         text        NOT NULL,
  logical_source text        NOT NULL REFERENCES core.source (logical_source),
  size_bytes     bigint      NOT NULL CHECK (size_bytes >= 0),
  received_at    timestamptz NOT NULL DEFAULT now(),
  PRIMARY KEY (user_id, sha256)
);

-- 送る前に失った原本の欠け。冪等キーは id（収集側で採番）。
CREATE TABLE IF NOT EXISTS core.media_missing (
  id             uuid        PRIMARY KEY,
  user_id        uuid        NOT NULL,
  device_id      text,
  logical_source text        NOT NULL REFERENCES core.source (logical_source),
  sha256         text        NOT NULL,
  reason         text        NOT NULL CHECK (reason IN (
                   'vanished', 'changed', 'unreadable', 'too_large',
                   'record_dropped', 'record_rejected', 'record_missing')),
  noticed_at     timestamptz NOT NULL,
  received_at    timestamptz NOT NULL DEFAULT now()
);

-- 文字抽出の結果。エンジンとその世代の列を持つ。この change では行を入れる口を作らない。
CREATE TABLE IF NOT EXISTS core.text_extraction (
  id             uuid        PRIMARY KEY,
  user_id        uuid        NOT NULL,
  event_id       uuid        NOT NULL REFERENCES core.event (id),
  engine         text        NOT NULL,
  engine_version text        NOT NULL,
  extracted_at   timestamptz NOT NULL,
  text           text        NOT NULL,
  recorded_at    timestamptz NOT NULL DEFAULT now()
);

-- 3 つとも追記のみ。TRUNCATE は行トリガでは拒めないため、文トリガも置く。
-- 他の台帳の関数を共用すると、その関数を落とす古い down 移行が依存で当たらなくなるので、この版の専用にする。
CREATE OR REPLACE FUNCTION core.reject_media_change() RETURNS trigger AS $fn$
BEGIN
  RAISE EXCEPTION '% は追記のみ。書き換えも削除も切り詰めもできない（ST09 / design D15）', TG_TABLE_NAME;
END;
$fn$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS media_original_append_only ON core.media_original;
CREATE TRIGGER media_original_append_only
  BEFORE UPDATE OR DELETE ON core.media_original
  FOR EACH ROW EXECUTE FUNCTION core.reject_media_change();
DROP TRIGGER IF EXISTS media_original_no_truncate ON core.media_original;
CREATE TRIGGER media_original_no_truncate
  BEFORE TRUNCATE ON core.media_original
  FOR EACH STATEMENT EXECUTE FUNCTION core.reject_media_change();

DROP TRIGGER IF EXISTS media_missing_append_only ON core.media_missing;
CREATE TRIGGER media_missing_append_only
  BEFORE UPDATE OR DELETE ON core.media_missing
  FOR EACH ROW EXECUTE FUNCTION core.reject_media_change();
DROP TRIGGER IF EXISTS media_missing_no_truncate ON core.media_missing;
CREATE TRIGGER media_missing_no_truncate
  BEFORE TRUNCATE ON core.media_missing
  FOR EACH STATEMENT EXECUTE FUNCTION core.reject_media_change();

DROP TRIGGER IF EXISTS text_extraction_append_only ON core.text_extraction;
CREATE TRIGGER text_extraction_append_only
  BEFORE UPDATE OR DELETE ON core.text_extraction
  FOR EACH ROW EXECUTE FUNCTION core.reject_media_change();
DROP TRIGGER IF EXISTS text_extraction_no_truncate ON core.text_extraction;
CREATE TRIGGER text_extraction_no_truncate
  BEFORE TRUNCATE ON core.text_extraction
  FOR EACH STATEMENT EXECUTE FUNCTION core.reject_media_change();

-- アプリの役割への権限は crates/server/src/grants.sql が持つ（毎回当てる。全表に UPDATE / DELETE も付く）。
-- 3 表の書き換え禁止は上のトリガだけが持つ（権限では絞っていない。D15 の 4 の扱いは人間の判断待ち）。

-- ------------------------------------------------------------------ 5. 収集開始日の当て直し（D13）
-- 印が `heartbeat` のソースは、登録日以降でいちばん古い生存信号の日、無ければ NULL。
-- **記録は 1 件も変えない**（収集開始日の列だけを書く）。写真の記録の撮影時刻は何年も前になりうるので、
-- 記録から引くと窓が撮影日まで遡る（Q13）。印が `record` に戻されたソースは対象にしない。
UPDATE core.source s
   SET collection_started_on = (
         SELECT min((h.emitted_at AT TIME ZONE 'Asia/Tokyo')::date)
           FROM core.heartbeat h
          WHERE h.logical_source = s.logical_source
            AND (h.emitted_at AT TIME ZONE 'Asia/Tokyo')::date
                >= (s.registered_at AT TIME ZONE 'Asia/Tokyo')::date)
 WHERE s.started_on_basis = 'heartbeat'
   AND s.collection_started_on IS DISTINCT FROM (
         SELECT min((h.emitted_at AT TIME ZONE 'Asia/Tokyo')::date)
           FROM core.heartbeat h
          WHERE h.logical_source = s.logical_source
            AND (h.emitted_at AT TIME ZONE 'Asia/Tokyo')::date
                >= (s.registered_at AT TIME ZONE 'Asia/Tokyo')::date);
