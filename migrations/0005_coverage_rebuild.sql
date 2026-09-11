-- 0005 稼働記録の作り直しと、生存信号・停止/破棄の範囲・収集開始日（ST02 / design D2, D3, D5, D8）
-- 前進のみ。戻し手順は migrations/0005_coverage_rebuild.down.sql に置く（**不可逆**）。
--
-- **作り直す理由**: 0001 の core.coverage は (logical_source, day, state) が主キーで、
-- state が ('alive','stopped','dropped') の 3 値。ST02 が要る 7 状態も、
-- FR-34 / FR-9 の「期間」も乗らない（期間は note の自由文しか無かった）。
-- 状態は**行に焼かず導出する**（design D6）ので、この表は「その日に何件入ったか」だけを持つ。
--
-- **user_id を足す最後の安い機会がここ**（FR-29「すべてのテーブルに」/ 扉 #9）。
-- 0001 は core.event にしか列を置いていない。

-- 版を当て直しても表を消さないよう、**古い形のときだけ**作り直す。
-- run() は起動のたびに全部の版を当てるので、無条件に DROP すると毎回の起動で稼働記録が消える。
-- **条件は「0001 の形ちょうど」に絞る**（review/code.md の R23）。
-- 「`state` 列がある」だけを条件にすると、将来この判断が覆って（導出が重い等）
-- 作り直した後の `core.coverage` に `state` を足した瞬間、**次の起動で表が丸ごと消える**。
-- `user_id` が**無い**ことを併せて見れば、0001 の形のときにしか当たらない。
DO $$
BEGIN
  IF EXISTS (
    SELECT 1 FROM information_schema.columns
     WHERE table_schema = 'core' AND table_name = 'coverage' AND column_name = 'state'
  ) AND NOT EXISTS (
    SELECT 1 FROM information_schema.columns
     WHERE table_schema = 'core' AND table_name = 'coverage' AND column_name = 'user_id'
  ) THEN
    DROP TABLE core.coverage;
  END IF;
END $$;

CREATE TABLE IF NOT EXISTS core.coverage (
  user_id        uuid NOT NULL,                        -- FR-29 / 扉 #9
  logical_source text NOT NULL REFERENCES core.source(logical_source),
  day            date NOT NULL,                        -- Asia/Tokyo で区切った日（design D1 / 深掘り Q2）
  event_count    integer NOT NULL DEFAULT 0,           -- 新しく入った記録だけを数える（ST01 design D13）
  PRIMARY KEY (user_id, logical_source, day)
);

-- **記録から引き直す。** 旧 core.coverage は UTC で日を切っていた（既存の欠陥 1）ので、
-- 行を移しても日がずれている。core.event は全件残っているので、Asia/Tokyo で数え直すほうが正しい。
-- 重複は INSERT の時点で弾かれている（event_dedup_hash）ので、count(*) が
-- そのまま「新しく入った記録の数」にあたる。
INSERT INTO core.coverage (user_id, logical_source, day, event_count)
SELECT user_id, logical_source, (event_time AT TIME ZONE 'Asia/Tokyo')::date, count(*)
  FROM core.event
 GROUP BY 1, 2, 3
    ON CONFLICT (user_id, logical_source, day) DO NOTHING;

-- 生存信号（design D3 / FR-78）。**core.event に入れない** ——
-- origin が ('collected','authored','derived') の 3 値で、生存信号はどれでもない。
-- 4 つ目を足すと、記録を読むすべての経路が生存信号を記録として拾う。
CREATE TABLE IF NOT EXISTS core.heartbeat (
  id             uuid PRIMARY KEY,                     -- 収集側で生成（FR-21 と同じ向き）
  user_id        uuid NOT NULL,                        -- FR-29
  logical_source text NOT NULL REFERENCES core.source(logical_source),
  device_id      text,
  emitted_at     timestamptz NOT NULL,                 -- 収集側が信号を作った時刻
  received_at    timestamptz NOT NULL DEFAULT now(),   -- 受信時刻。**日に丸めない**（深掘り 第 4 回 Q14）
  capturable     boolean NOT NULL,                     -- 取得できる状態か（深掘り Q5）
  blockers       text[] NOT NULL DEFAULT '{}',         -- 満たされていないもの（permission / sensor / network）
  attempts       integer NOT NULL,                     -- 前回の信号からの取得の試行回数（第 5 回 Q17）
  successes      integer NOT NULL,                     -- そのうち成功した回数（同上）
  content_hash   text NOT NULL,                        -- 冪等キー（第 4 回 Q13）
  raw            text NOT NULL                         -- 原文の素通し。**text**（0003 と同じ理由）
);
-- 再送は常態（ST01 の Outbox は部分失敗の後で送り直す）。鍵が無いと行が増える。
--
-- **利用者識別子を鍵に入れる**（review/code.md の R13）。`content_hash` は
-- `logical_source` + `emitted_at` + `raw` からしか作られないので、入れないと
-- **別の利用者の同じ内容の信号が「重複」として黙って落ちる**（呼び出し側には成功に見える）。
-- 列は FR-29 で day one から持っているのに、一意性が利用者をまたいでいた。
-- 単一利用者のうちは挙動が変わらないので、**いま直すのがいちばん安い**（0005 と同じ理屈）。
DO $$
BEGIN
  IF EXISTS (
    SELECT 1 FROM pg_indexes
     WHERE schemaname = 'core' AND indexname = 'heartbeat_dedup'
       AND indexdef NOT LIKE '%user_id%'
  ) THEN
    DROP INDEX core.heartbeat_dedup;
  END IF;
END $$;
CREATE UNIQUE INDEX IF NOT EXISTS heartbeat_dedup
  ON core.heartbeat (user_id, logical_source, content_hash);
-- 日の集計はソース × 期間で引く。
CREATE INDEX IF NOT EXISTS heartbeat_by_source_time
  ON core.heartbeat (logical_source, emitted_at);

-- 停止（FR-34）と破棄（FR-9）を 1 つの範囲表に入れる（design D8）。
-- どちらも「ある期間、そのソースの記録が無いことに理由がある」という同型の事実。
CREATE TABLE IF NOT EXISTS core.coverage_span (
  id             uuid PRIMARY KEY,
  user_id        uuid NOT NULL,                        -- FR-29
  logical_source text NOT NULL REFERENCES core.source(logical_source),
  kind           text NOT NULL CHECK (kind IN ('stopped','dropped')),
  started_at     timestamptz NOT NULL,
  ended_at       timestamptz,                          -- NULL = まだ続いている（ST15 が閉じる）
  event_count    integer,                              -- 破棄した件数（FR-9）
  note           text,
  CONSTRAINT coverage_span_range CHECK (ended_at IS NULL OR started_at < ended_at)
);
CREATE INDEX IF NOT EXISTS coverage_span_by_source
  ON core.coverage_span (logical_source, started_at);

-- 収集開始日（FR-79）と利用者識別子（FR-29）。
-- **主キーは logical_source のまま変えない** —— core.event.logical_source が FK で参照しており、
-- (user_id, logical_source) にすると FK と既存の全行が壊れる。
-- FR-29 は「列を持たせる」であって主キーにせよとは言っていない。
ALTER TABLE core.source ADD COLUMN IF NOT EXISTS user_id uuid;
ALTER TABLE core.source ADD COLUMN IF NOT EXISTS collection_started_on date;

-- 既存のソースの収集開始日は、**いちばん古い記録が作られた日**を当てる（第 6 回 Q24）。
-- 受信時刻（ingest_time）ではない —— 圏外で溜めて送ると記録のある日が「導入前」になる。
-- **1 件も無いソースは NULL のまま**（第 5 回 Q22。登録簿にあるだけで計測を始めない）。
UPDATE core.source s
   SET collection_started_on = e.first_day
  FROM (
    SELECT logical_source, min((event_time AT TIME ZONE 'Asia/Tokyo')::date) AS first_day
      FROM core.event GROUP BY logical_source
  ) e
 WHERE s.logical_source = e.logical_source
   AND (s.collection_started_on IS NULL OR s.collection_started_on > e.first_day);

-- NFR-13 が名指しする Must の 5 ソースを登録簿に置く（FR-35 の想定間隔つき）。
-- **これが無いと途絶の判定も、利用が主語の 3 ソースの達成日も成立しない**（review/spec.md の R4）。
-- 想定間隔は FR-35 の**初期値**なので、既にある行は動かさない（本人が変えた値を戻さない）。
INSERT INTO core.source (logical_source, display_name, expected_gap_sec) VALUES
  ('c01-location',        '携帯端末の位置',        21600),
  ('c01-app-usage',       '携帯端末のアプリ利用',  21600),
  ('c01-photo',           '端末に保存された写真',  21600),
  ('c02-window',          'PC のウィンドウ',       21600),
  ('c02-browser-history', 'PC のブラウザ履歴',     86400)
    ON CONFLICT (logical_source) DO NOTHING;
