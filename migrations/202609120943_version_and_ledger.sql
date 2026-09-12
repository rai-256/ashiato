-- 履歴表と消去の台帳、そして読み出しの 2 つのビュー
-- （ST03 / 深掘り Q1 / Q12 / Q17 / Q21 / Q23 / Q8。design D4 / D6）
-- 前進のみ。戻し手順は migrations/202609120943_version_and_ledger.down.sql に置く。

-- ------------------------------------------------------------------ 履歴（前の版）
--
-- **`raw` は `text`**（0003 と同じ理由）。`jsonb` はキー順を変え、重複キーを落とし、
-- 数値表記を展開する。Q1 の答えで**前の版の原文は履歴表にしか無くなる**ので、
-- ここで `jsonb` にすると取り返しがつかない（R39）。
--
-- **感度も削除の印も持たせない**（Q21 / D6）。持たせると、親を締めても前の版が
-- 緩いまま残る（実測）。**伝播の処理が存在しなければ書き忘れようがない。**
-- 読み出しは `core.event_version_live` 越しにだけ行う。
--
-- `txid` は門（*_gates）が「この書き換えと同じまとまりで履歴が書かれたか」を
-- COMMIT の瞬間に見るためのもの。**後付けの列ではない** —— 既定値で必ず入る。
CREATE TABLE IF NOT EXISTS core.event_version (
  id                uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  event_id          uuid NOT NULL REFERENCES core.event(id),
  user_id           uuid NOT NULL,                 -- Q12。FR-18 / FR-29 の適用範囲がここにも及ぶ
  logical_source    text NOT NULL,
  version_no        integer NOT NULL,              -- 1 から。古い順
  event_time        timestamptz NOT NULL,
  content_hash      text NOT NULL,
  raw               text NOT NULL,                 -- **text**（R39）
  payload           jsonb NOT NULL,
  source_updated_at timestamptz,
  external_ref      text,
  -- **出来事の時刻を読むための欄**（R111）。更新は `event_time` と一緒にこれらも動かすので、
  -- 持たせないと**前の版の現地時刻と単位の宣言が失われる**（原文は残るが、
  -- 「その原文をどの地域・どの版として読んだか」は原文の中に無い）。
  tz_offset_min     integer NOT NULL DEFAULT 0,
  tz_id             text    NOT NULL DEFAULT 'UTC',
  schema_version    integer NOT NULL DEFAULT 1,
  unit_system       text    NOT NULL DEFAULT 'si',
  crs               text    NOT NULL DEFAULT 'EPSG:4326',
  superseded_at     timestamptz NOT NULL DEFAULT now(),
  txid              xid8 NOT NULL DEFAULT pg_current_xact_id(),
  UNIQUE (event_id, version_no)
);

-- **当て直しでも列が揃う。** `CREATE TABLE IF NOT EXISTS` は既存の表に列を足さないので、
-- 先に作った DB（R111 より前の形）にも当たるようにしておく。
ALTER TABLE core.event_version ADD COLUMN IF NOT EXISTS tz_offset_min  integer NOT NULL DEFAULT 0;
ALTER TABLE core.event_version ADD COLUMN IF NOT EXISTS tz_id          text    NOT NULL DEFAULT 'UTC';
ALTER TABLE core.event_version ADD COLUMN IF NOT EXISTS schema_version integer NOT NULL DEFAULT 1;
ALTER TABLE core.event_version ADD COLUMN IF NOT EXISTS unit_system    text    NOT NULL DEFAULT 'si';
ALTER TABLE core.event_version ADD COLUMN IF NOT EXISTS crs            text    NOT NULL DEFAULT 'EPSG:4326';

CREATE INDEX IF NOT EXISTS event_version_by_event ON core.event_version (event_id);

-- ------------------------------------------------------------------ 消去の台帳
--
-- Q23。**消去（本文を本当に消す）は、台帳の行が同じまとまりにあるときだけ通る。**
-- Q17 が残した唯一の開口部がこれで、縛らないとその 1 つの穴から原文が全部消える。
--
-- **台帳自身は追記のみ**（R50）。ここに開ける必要のある操作は 1 つも無い
-- （FR-51 が「残す」と決めている）ので、**Q10 → Q17 → Q23 の連鎖はここで止まる**。
CREATE TABLE IF NOT EXISTS core.erasure_ledger (
  id             uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  event_id       uuid NOT NULL,                    -- **FK を張らない**（消去は記録より後まで残る）
  user_id        uuid NOT NULL,
  logical_source text NOT NULL,
  scope          text NOT NULL CHECK (scope IN ('event', 'version')),
  erased_at      timestamptz NOT NULL DEFAULT now(),
  erased_by      text NOT NULL,
  reason         text,
  txid           xid8 NOT NULL DEFAULT pg_current_xact_id()
);

CREATE INDEX IF NOT EXISTS erasure_ledger_by_event ON core.erasure_ledger (event_id);

-- ------------------------------------------------------------------ 読み出しの 2 つの形
--
-- **履歴は親と束ねたビュー越しにだけ読む**（R49 / D6）。履歴表を直に引くと、
-- 親の感度と削除が効かないまま前の版の本文が出る（実測）。
-- 本表が `core.event_live` で同じ危険を塞いでいるのと同じ手当て（製造準備 A-3）。
CREATE OR REPLACE VIEW core.event_version_live AS
  SELECT v.id, v.event_id, v.user_id, v.logical_source, v.version_no,
         v.event_time, v.content_hash, v.raw, v.payload,
         v.source_updated_at, v.external_ref, v.superseded_at,
         v.tz_offset_min, v.tz_id, v.schema_version, v.unit_system, v.crs,
         e.sensitivity,          -- **親の値**（Q21。履歴は自分の感度を持たない）
         e.origin,
         e.external_id
    FROM core.event_version v
    JOIN core.event e ON e.id = v.event_id
   WHERE e.deleted_at IS NULL;   -- 親が消えれば履歴も消える（Q12 / Q21）

-- Q8。**内容の鍵が同じ複数行を 1 件として読む置き場。**
-- 外部識別子を優先すると同じ内容の行が複数立ちうるので、読む側が畳めないと
-- 画面と分析が二重になる。
--
-- **実際に使うのは後続の Story**（閲覧・検索・AI・書き出し）。ST03 は置き場だけを作る。
CREATE OR REPLACE VIEW core.event_folded AS
  SELECT user_id,
         logical_source,
         content_hash,
         count(*)                                            AS folded_rows,
         min(event_time)                                     AS event_time,
         (array_agg(id           ORDER BY ingest_time, id))[1] AS id,
         (array_agg(raw          ORDER BY ingest_time, id))[1] AS raw,
         (array_agg(payload      ORDER BY ingest_time, id))[1] AS payload,
         (array_agg(origin       ORDER BY ingest_time, id))[1] AS origin,
         max(sensitivity)                                    AS sensitivity,  -- **厳しい側に倒す**（扉 #15）
         array_agg(external_id ORDER BY ingest_time, id)
           FILTER (WHERE external_id IS NOT NULL)            AS external_ids
    FROM core.event_live
   GROUP BY user_id, logical_source, content_hash;
