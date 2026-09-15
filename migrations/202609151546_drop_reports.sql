-- 端末からの破棄の報告（ST04 / FR-9 / FR-33。design D7 / D16）
-- 前進のみ。戻し手順は migrations/202609151546_drop_reports.down.sql に置く。
--
-- **破棄の報告は「バッファから破棄されたのか」の唯一の証拠**（扉 #14）。端末は捨てた時点を過ぎると
-- 何も持たないので、端末・理由・時間ごとの件数は受け手が行として持つ（deep.md C8 / C12）。
--
-- **`core.coverage_span` の `kind = 'dropped'` とは別の表にする**（design D7）。
-- `coverage_span` は停止（ST15）の置き場でもあり、端末も理由も時間ごとの件数も持たない。
-- 形を変えずに残し、稼働状況は 2 つを合わせて読む（design D8）。
--
-- **当て直せる形にする**（`migrate()` は起動のたびに全版を当てる）。

CREATE TABLE IF NOT EXISTS core.drop_report (
  id             uuid PRIMARY KEY,                     -- 端末が振った識別子（冪等キーには混ぜない）
  user_id        uuid NOT NULL,                        -- FR-29
  logical_source text NOT NULL REFERENCES core.source(logical_source),
  device_id      text NOT NULL,                        -- どの端末が捨てたか（C8）
  reason         text NOT NULL
                   CHECK (reason IN ('age','bytes','write_failed','unreadable')),
  range_start    timestamptz,                          -- 最初に捨てた記録の出来事の時刻
  range_end      timestamptz,                          -- 終わり（含まない。C3 / R4）
  count          integer NOT NULL,
  created_at     timestamptz NOT NULL,                 -- 端末が報告を作った時刻
  received_at    timestamptz NOT NULL DEFAULT now(),   -- 受信時刻。**日に丸めない**
  content_hash   text NOT NULL,                        -- 冪等キー（logical_source + raw）
  raw            text NOT NULL,                        -- 原文の素通し。**text**（0003 と同じ理由）
  CONSTRAINT drop_report_count CHECK (count > 0),
  -- 1 件だけの破棄でも範囲が空にならない（R4。`coverage_span` はここで 500 を返していた）
  CONSTRAINT drop_report_range CHECK (range_end IS NULL OR range_start < range_end),
  -- 範囲は両端を持つか、どちらも持たない（読めなかった行・数えきれなかった分）
  CONSTRAINT drop_report_range_pair CHECK ((range_start IS NULL) = (range_end IS NULL))
);

-- 再送は常態（端末は凍結した報告を同じ原文で送り直す）。**利用者識別子を鍵に入れる**
-- （生存信号の R13 と同じ理由。入れないと別の利用者の同じ報告が黙って落ちる）。
CREATE UNIQUE INDEX IF NOT EXISTS drop_report_dedup
  ON core.drop_report (user_id, logical_source, content_hash);
-- 稼働状況はソース × 期間で引く（design D8 / D9）
CREATE INDEX IF NOT EXISTS drop_report_by_source_range
  ON core.drop_report (logical_source, range_start);

-- 出来事の時刻の 1 時間（UTC）ごとの件数（C12）。範囲と総件数からは日ごとの件数を割り戻せない
CREATE TABLE IF NOT EXISTS core.drop_report_hour (
  report_id uuid NOT NULL REFERENCES core.drop_report(id),
  hour      timestamptz NOT NULL,
  count     integer NOT NULL CHECK (count > 0),
  PRIMARY KEY (report_id, hour)
);

-- ------------------------------------------------------------------ 2 表の錠
--
-- 生存信号（0006）と同じく**全列の更新も削除も拒む**。`UPDATE` だけを止めると
-- 削除して入れ直す 2 手で差し替えられる（ST02 の R22）。`TRUNCATE` は行トリガで撃たれないので文トリガを別に置く。
-- **他の版の関数を使わない** —— 使うと、その関数を落とす戻し手順が依存で当たらなくなる（ST16 の実測）。
CREATE OR REPLACE FUNCTION core.reject_drop_report_change() RETURNS trigger AS $fn$
BEGIN
  RAISE EXCEPTION '破棄の報告（%）は書き換えも削除もできない（FR-9 / 扉 #14）', TG_TABLE_NAME;
END;
$fn$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS drop_report_immutable ON core.drop_report;
CREATE TRIGGER drop_report_immutable
  BEFORE UPDATE OR DELETE ON core.drop_report
  FOR EACH ROW EXECUTE FUNCTION core.reject_drop_report_change();

DROP TRIGGER IF EXISTS drop_report_no_truncate ON core.drop_report;
CREATE TRIGGER drop_report_no_truncate
  BEFORE TRUNCATE ON core.drop_report
  FOR EACH STATEMENT EXECUTE FUNCTION core.reject_drop_report_change();

DROP TRIGGER IF EXISTS drop_report_hour_immutable ON core.drop_report_hour;
CREATE TRIGGER drop_report_hour_immutable
  BEFORE UPDATE OR DELETE ON core.drop_report_hour
  FOR EACH ROW EXECUTE FUNCTION core.reject_drop_report_change();

DROP TRIGGER IF EXISTS drop_report_hour_no_truncate ON core.drop_report_hour;
CREATE TRIGGER drop_report_hour_no_truncate
  BEFORE TRUNCATE ON core.drop_report_hour
  FOR EACH STATEMENT EXECUTE FUNCTION core.reject_drop_report_change();
