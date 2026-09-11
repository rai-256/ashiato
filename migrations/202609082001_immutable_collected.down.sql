-- 0002 の戻し手順（A-2: 各版に戻し手順を必ず書く）
DROP TRIGGER IF EXISTS event_immutable_collected ON core.event;
DROP FUNCTION IF EXISTS core.reject_collected_rewrite();
