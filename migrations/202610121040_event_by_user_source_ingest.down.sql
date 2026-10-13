-- 戻し手順。索引を落とすだけで、記録は失われない（「最後に届いた時刻」の読みが全行の走査に戻る）。
DROP INDEX IF EXISTS core.event_by_user_source_ingest;
