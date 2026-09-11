-- 0003 の戻し。**戻すと原文が失われる** —— jsonb はキー順・重複キー・数値表記を
-- 正規化するので、この方向は不可逆になる。
--
-- **原文が JSON でない記録があると落ちる。** 0003 以降の原文は「取得元から受け取った
-- 文字列そのもの」であって JSON である保証が無い。戻す前に確かめる:
--   SELECT count(*) FROM core.event WHERE raw IS NOT NULL AND NOT pg_input_is_valid(raw, 'jsonb');
--
-- **`raw::jsonb IS NULL` では数えられない**（review R2 / 実測）—— このキャストは
-- JSON でない値に当たると NULL を返さず**例外を投げる**ので、件数が出ないうえ、
-- 例外の本文に入力の断片が載って**原文がオペレータの端末とシェル履歴に出る**（A-2）。
-- `pg_input_is_valid`（PG16+）は値を吐かずに真偽だけを返す。
-- **3 文をまとめて 1 つにする。** 囲まないと、ALTER が落ちたとき
-- （＝上の確認が本来止めるはずだったケースでは確実に落ちる）
-- core.event_live が消えたまま復元されず、/events が壊れる。
BEGIN;
DROP VIEW IF EXISTS core.event_live;
ALTER TABLE core.event ALTER COLUMN raw TYPE jsonb USING raw::jsonb;
CREATE VIEW core.event_live AS SELECT * FROM core.event WHERE deleted_at IS NULL;
COMMIT;
