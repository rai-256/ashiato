-- 0001 記録のエンベロープ（要件 FR-18〜FR-29 / 扉 #6〜#13, #16）
-- 前進のみ。戻し手順は migrations/202609081618_envelope.down.sql に置く。
CREATE SCHEMA IF NOT EXISTS core;

-- ソースの登録簿。ここに 1 行足すだけで新しいソースを受け付ける（FR-61）
CREATE TABLE IF NOT EXISTS core.source (
  logical_source   text PRIMARY KEY,
  display_name     text NOT NULL,
  expected_gap_sec integer NOT NULL,          -- 想定される最大の無通信間隔（FR-35 が 3 倍で通知）
  registered_at    timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS core.event (
  id              uuid PRIMARY KEY,                     -- 収集側で生成（FR-21）
  user_id         uuid NOT NULL,                        -- 単一利用者でも day one から（FR-29）
  logical_source  text NOT NULL REFERENCES core.source(logical_source),
  external_id     text,                                 -- 外部サービス上の識別子（FR-23）
  device_id       text,                                 -- provenance（FR-24）
  origin          text NOT NULL
      CHECK (origin IN ('collected','authored','derived')),  -- 由来の区別（FR-25）
  event_time      timestamptz NOT NULL,                 -- 出来事が起きた時刻（FR-19）
  ingest_time     timestamptz NOT NULL DEFAULT now(),   -- D-01 に入った時刻（FR-19）
  tz_offset_min   integer NOT NULL,                     -- UTC からのずれ（FR-20）
  tz_id           text NOT NULL,                        -- タイムゾーン識別子（FR-20）
  schema_version  integer NOT NULL,                     -- 書かれた時点の版（FR-26）
  unit_system     text NOT NULL DEFAULT 'si',           -- 単位と座標系（FR-28）
  crs             text NOT NULL DEFAULT 'EPSG:4326',
  sensitivity     smallint NOT NULL DEFAULT 1
      CHECK (sensitivity BETWEEN 0 AND 3),              -- 4 段階（PERM-2）。既定は「外部 AI 可」
  content_hash    text NOT NULL,                        -- 冪等の判定（FR-22）
  raw             jsonb NOT NULL,                       -- 原文をそのまま（FR-18）
  payload         jsonb NOT NULL,                       -- 解析済み
  deleted_at      timestamptz,                          -- 論理削除（FR-50）
  deleted_by      text
);

CREATE UNIQUE INDEX IF NOT EXISTS event_dedup_ext
  ON core.event (logical_source, external_id) WHERE external_id IS NOT NULL;
CREATE UNIQUE INDEX IF NOT EXISTS event_dedup_hash
  ON core.event (logical_source, content_hash);

-- 論理削除はビュー越しでしか読めない形にする（A-3 の決定）
CREATE OR REPLACE VIEW core.event_live AS
  SELECT * FROM core.event WHERE deleted_at IS NULL;

-- 稼働記録。欠損の意味を後から区別する唯一の手段（FR-33 / FR-34 / FR-9）
CREATE TABLE IF NOT EXISTS core.coverage (
  logical_source text NOT NULL REFERENCES core.source(logical_source),
  day            date NOT NULL,
  state          text NOT NULL CHECK (state IN ('alive','stopped','dropped')),
  event_count    integer NOT NULL DEFAULT 0,
  note           text,
  PRIMARY KEY (logical_source, day, state)
);
