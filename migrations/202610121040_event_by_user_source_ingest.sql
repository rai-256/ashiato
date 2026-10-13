-- ブラウザ履歴が「最後に届いた時刻」を索引の末尾 1 つで引く（ST25 / design D10（仮）。final review R6）。
--
-- `GET /day` は日を開くたびに `max(ingest_time)` を利用者の `c02-browser-history` について読む。
-- 既存の索引は `ingest_time` を持たないので、利用者のブラウザ履歴の全行を読んでいた
-- （10 万行で 21〜23 ms の実測。索引があれば 0.1 ms）。行を足すだけで既存の記録は変えない。
CREATE INDEX IF NOT EXISTS event_by_user_source_ingest
  ON core.event (user_id, logical_source, ingest_time);
