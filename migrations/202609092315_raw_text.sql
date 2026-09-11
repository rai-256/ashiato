-- 0003 原文を text で保存する（深掘り 第 2 回 / FR-18 / specs record-envelope）
-- 前進のみ。戻し手順は migrations/202609092315_raw_text.down.sql に置く。
--
-- **`jsonb` は原文を保たない。** 0001 で原文を `jsonb` にしたが、この型は
-- 「受け取ったもの」ではなく「**構造として同じもの**」を保存する。実測:
--   {"b":1,"a":2,"a":3}  →  {"a": 3, "b": 1}    キー順が変わり、重複キーが消える
--   {"n":1.100,"m":1e2}  →  {"m": 100, ...}     指数表記と末尾の 0 が展開される
--
-- 深掘り 第 1 回で原文の NFC 正規化を拒んだ理由（署名の検証・外部サービスとの照合）は、
-- **型が正規化する限り達成できない**。spec の「バイト単位で一致する」はこの型では偽だった。
--
-- 代償: **原文への SQL クエリはできなくなる**（`raw->>'lat'` が引けない）。
-- 引く側は `payload` を使う。これは本人が受け入れた（深掘り 第 2 回）。

-- 版を当て直しても表を書き換えないよう、型を見てから当てる。
-- `run()` は起動のたびに全部の版を当てるので、無条件だと 1 年ぶんの行を毎回書き直す。
DO $$
BEGIN
  IF EXISTS (
    SELECT 1 FROM information_schema.columns
     WHERE table_schema = 'core' AND table_name = 'event'
       AND column_name = 'raw' AND data_type <> 'text'
  ) THEN
    -- **ビューを外してから型を変える。** core.event_live は `SELECT *` なので raw に依存し、
    -- 付いたままだと「ビューが依存している」で落ちる。CASCADE は使わない ——
    -- 他にぶら下がっているものがあれば、黙って消さずにここで落ちてほしい。
    DROP VIEW IF EXISTS core.event_live;
    ALTER TABLE core.event ALTER COLUMN raw TYPE text USING raw::text;
    CREATE VIEW core.event_live AS SELECT * FROM core.event WHERE deleted_at IS NULL;
  END IF;
END $$;
