-- 0006 生存信号の書き換えを DB で拒む（FR-78 / 深掘り 第 4 回 Q13 / design D4）
-- 前進のみ。戻し手順は migrations/0006_immutable_heartbeat.down.sql に置く。
--
-- **生存信号は証拠である。** 記録が 0 件の日に「動いていなかった」のか「壊れていた」のかを
-- 分ける唯一の材料で、書き換えられると扉 #14 の区別がそのまま嘘になる。
-- 本人が「core.event と同じ保護を掛ける」と決めた（第 4 回 Q13）。
--
-- **全列の更新を拒む。** core.event の 0002 / 0004 は論理削除（FR-50）のために
-- deleted_at / deleted_by を通しているが、生存信号には論理削除が無いので通し道を作らない。
--
-- **迂回路になる列を最初から持たせない**（0004 の教訓）。core.event は origin を
-- 'authored' へ動かしてから原文を書き換え、'collected' へ戻す 3 手で迂回できた。
-- core.heartbeat には分類列が無く、かつここが全列を拒むので、同じ形の迂回路が無い。

CREATE OR REPLACE FUNCTION core.reject_heartbeat_rewrite() RETURNS trigger AS $fn$
BEGIN
  RAISE EXCEPTION '生存信号は書き換えられない（FR-78 / 深掘り 第 4 回 Q13）';
END;
$fn$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS heartbeat_immutable ON core.heartbeat;
CREATE TRIGGER heartbeat_immutable
  BEFORE UPDATE ON core.heartbeat
  FOR EACH ROW EXECUTE FUNCTION core.reject_heartbeat_rewrite();
