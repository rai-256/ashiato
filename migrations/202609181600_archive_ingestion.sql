-- 書庫の台帳と置き場の状態（ST12 / design D7, D8, D14, D16）。
-- 前進のみ。戻し手順は 202609181600_archive_ingestion.down.sql に置く。

CREATE TABLE IF NOT EXISTS core.archive_ledger (
  id bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  user_id uuid NOT NULL,
  sha256 text NOT NULL,
  parser_version text NOT NULL,
  outcome text NOT NULL CHECK (outcome IN ('read','already_read','unreadable','pending_shape','store_failed')),
  inbox_kind text NOT NULL DEFAULT 'inbox',
  created_at timestamptz,
  discovered_at timestamptz NOT NULL DEFAULT now(),
  started_at timestamptz,
  finished_at timestamptz NOT NULL DEFAULT now(),
  unreadable_kind text,
  -- 置き場の中の名前だけ。フォルダのパスも記録の本文も持たない（D7）。
  file_name text,
  -- already_read のとき、前に読んだ行（画面の「前に読んだ時刻」）。
  already_read_ledger_id bigint,
  unreadable_count integer NOT NULL DEFAULT 0 CHECK (unreadable_count >= 0),
  skipped_file_count integer NOT NULL DEFAULT 0 CHECK (skipped_file_count >= 0)
);
-- D7 が要求する、画面の箱が使う 2 列。merge 前にこの移行を当てた開発 DB にも足す。
ALTER TABLE core.archive_ledger ADD COLUMN IF NOT EXISTS file_name text;
ALTER TABLE core.archive_ledger ADD COLUMN IF NOT EXISTS already_read_ledger_id bigint;
CREATE UNIQUE INDEX IF NOT EXISTS archive_ledger_once
  ON core.archive_ledger (user_id, sha256, parser_version, outcome);
CREATE INDEX IF NOT EXISTS archive_ledger_finished
  ON core.archive_ledger (user_id, finished_at DESC);

CREATE TABLE IF NOT EXISTS core.archive_ledger_source (
  ledger_id bigint NOT NULL REFERENCES core.archive_ledger(id),
  logical_source text NOT NULL REFERENCES core.source(logical_source),
  inserted_count integer NOT NULL DEFAULT 0 CHECK (inserted_count >= 0),
  duplicate_count integer NOT NULL DEFAULT 0 CHECK (duplicate_count >= 0),
  deleted_count integer NOT NULL DEFAULT 0 CHECK (deleted_count >= 0),
  unreadable_count integer NOT NULL DEFAULT 0 CHECK (unreadable_count >= 0),
  max_event_at timestamptz,
  PRIMARY KEY (ledger_id, logical_source)
);
CREATE INDEX IF NOT EXISTS archive_ledger_source_latest
  ON core.archive_ledger_source (logical_source, max_event_at DESC);

-- 鍵は **利用者と中身の組**（FR-29）。中身だけを鍵にすると、同じファイルを持つ
-- 2 人目の目録が ON CONFLICT で黙って落ち、解析器の版が上がったときに
-- その人の写しから読み直せなくなる（D8 が約束している戻り道）。
CREATE TABLE IF NOT EXISTS core.archive_file (
  sha256 text NOT NULL,
  user_id uuid NOT NULL,
  inner_path text NOT NULL,
  stored_path text NOT NULL,
  created_at timestamptz NOT NULL DEFAULT now(),
  PRIMARY KEY (user_id, sha256)
);
-- merge 前にこの移行を当てた開発 DB の、中身だけの鍵を利用者ごとの鍵へ移す。
DO $$
BEGIN
  IF EXISTS (
    SELECT 1 FROM pg_index i
      JOIN pg_class c ON c.oid = i.indrelid
     WHERE c.relname = 'archive_file' AND i.indisprimary AND i.indnatts = 1
  ) THEN
    ALTER TABLE core.archive_file DROP CONSTRAINT archive_file_pkey;
    ALTER TABLE core.archive_file ADD PRIMARY KEY (user_id, sha256);
  END IF;
END $$;

CREATE TABLE IF NOT EXISTS core.archive_shape_confirmation (
  id bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  user_id uuid NOT NULL,
  shape_hash text NOT NULL,
  shape jsonb NOT NULL,
  confirmed_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS core.archive_sighting (
  user_id uuid NOT NULL,
  path text NOT NULL,
  size_bytes bigint NOT NULL,
  modified_at timestamptz NOT NULL DEFAULT now(),
  sha256 text,
  first_seen_at timestamptz NOT NULL DEFAULT now(),
  seen_at timestamptz NOT NULL DEFAULT now(),
  PRIMARY KEY (user_id, path)
);
-- この change が merge 前に当てた開発 DB にも、D8 が要求する比較値を足す。
ALTER TABLE core.archive_sighting ADD COLUMN IF NOT EXISTS modified_at timestamptz NOT NULL DEFAULT now();
ALTER TABLE core.archive_sighting ADD COLUMN IF NOT EXISTS sha256 text;
ALTER TABLE core.archive_sighting ADD COLUMN IF NOT EXISTS first_seen_at timestamptz NOT NULL DEFAULT now();
ALTER TABLE core.archive_sighting ADD COLUMN IF NOT EXISTS consecutive_failures integer NOT NULL DEFAULT 0;
ALTER TABLE core.archive_sighting ADD COLUMN IF NOT EXISTS retry_after timestamptz;
CREATE TABLE IF NOT EXISTS core.archive_scan_counter (
  user_id uuid PRIMARY KEY,
  scanned_at timestamptz NOT NULL DEFAULT now(),
  attempts integer NOT NULL DEFAULT 0,
  successes integer NOT NULL DEFAULT 0
);
ALTER TABLE core.archive_scan_counter ADD COLUMN IF NOT EXISTS attempts integer NOT NULL DEFAULT 0;
ALTER TABLE core.archive_scan_counter ADD COLUMN IF NOT EXISTS successes integer NOT NULL DEFAULT 0;
CREATE TABLE IF NOT EXISTS core.archive_pending_shape (
  user_id uuid NOT NULL,
  sha256 text NOT NULL,
  inner_path text NOT NULL,
  shape_hash text NOT NULL,
  shape jsonb NOT NULL,
  PRIMARY KEY (user_id, sha256, inner_path)
);
-- 同じ開発 DB に早い版を当てた場合にも、安全な確認用の形を保存できるようにする。
ALTER TABLE core.archive_pending_shape ADD COLUMN IF NOT EXISTS shape jsonb NOT NULL DEFAULT '{}'::jsonb;

CREATE OR REPLACE FUNCTION core.reject_archive_ledger_change() RETURNS trigger AS $fn$
BEGIN
  RAISE EXCEPTION '書庫の台帳（%）は追記のみ', TG_TABLE_NAME;
END;
$fn$ LANGUAGE plpgsql;

CREATE OR REPLACE FUNCTION core.archive_append_only(table_name text) RETURNS void AS $fn$
BEGIN
  EXECUTE format('DROP TRIGGER IF EXISTS archive_append_only ON %s', table_name);
  EXECUTE format('CREATE TRIGGER archive_append_only BEFORE UPDATE OR DELETE ON %s FOR EACH ROW EXECUTE FUNCTION core.reject_archive_ledger_change()', table_name);
  EXECUTE format('DROP TRIGGER IF EXISTS archive_no_truncate ON %s', table_name);
  EXECUTE format('CREATE TRIGGER archive_no_truncate BEFORE TRUNCATE ON %s FOR EACH STATEMENT EXECUTE FUNCTION core.reject_archive_ledger_change()', table_name);
END;
$fn$ LANGUAGE plpgsql;
SELECT core.archive_append_only('core.archive_ledger');
SELECT core.archive_append_only('core.archive_ledger_source');
SELECT core.archive_append_only('core.archive_file');
SELECT core.archive_append_only('core.archive_shape_confirmation');
DROP FUNCTION core.archive_append_only(text);

INSERT INTO core.source (logical_source, display_name, expected_gap_sec, external_id_kind) VALUES
  ('c03-timeline-visit', 'タイムラインの訪問', 5184000, 'none'),
  ('c03-timeline-move', 'タイムラインの移動', 5184000, 'none'),
  ('c03-timeline-route', 'タイムラインの経路', 5184000, 'none'),
  ('c03-timeline-signal', 'タイムラインの生の信号', 5184000, 'none'),
  ('c03-legacy-location', '移行前のロケーション履歴', 5184000, 'none'),
  ('c03-legacy-visit', '移行前の訪問', 5184000, 'none'),
  ('c03-legacy-activity', '移行前の移動', 5184000, 'none'),
  ('c03-youtube-watch', 'YouTube の視聴履歴', 5184000, 'none'),
  ('c03-youtube-search', 'YouTube の検索履歴', 5184000, 'none'),
  ('c03-chrome-history', 'Chrome の履歴', 5184000, 'none'),
  ('s01-archive-inbox', '取り込み器', 86400, 'none')
ON CONFLICT (logical_source) DO NOTHING;
