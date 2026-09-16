-- 戻し手順（202609151546_drop_reports）。**不可逆** ——
-- 端末から届いた破棄の報告（端末・理由・範囲・時間ごとの件数・原文）が失われる。
-- 端末は送れた報告を持たないので、戻した後に取り直す手段が無い。
-- 稼働状況は `core.coverage_span` の `dropped` だけを読む形に戻る（その日の「破棄された期間」と印が消える）。
DROP TRIGGER IF EXISTS drop_report_hour_no_truncate ON core.drop_report_hour;
DROP TRIGGER IF EXISTS drop_report_hour_immutable ON core.drop_report_hour;
DROP TRIGGER IF EXISTS drop_report_no_truncate ON core.drop_report;
DROP TRIGGER IF EXISTS drop_report_immutable ON core.drop_report;
DROP TABLE IF EXISTS core.drop_report_hour;
DROP TABLE IF EXISTS core.drop_report;
DROP FUNCTION IF EXISTS core.reject_drop_report_change();
