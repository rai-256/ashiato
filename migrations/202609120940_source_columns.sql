-- 登録簿に「外部識別子の粒度」を足す（ST03 / FR-61 / FR-23。深掘り Q4 / Q13 / Q16 / Q18）
-- 前進のみ。戻し手順は migrations/202609120940_source_columns.down.sql に置く。
--
-- **ST03 が作るのはこの 1 列だけ。** `retired_on` と `succeeds` は ST02 が
-- 202609112113_source_lifecycle で作っている（design D7。読む側が稼働記録なので）。
--
-- **既定は `'record'`（＝断る側）**（Q16 / Q18）。緩い側に倒すと、印を書き忘れた
-- ソースの記録が識別子なしで入り、**後から識別子を足す手段が無い**。
-- 断る側なら全件 400 になって気付く —— ただし端末では `Sender.kt` のログ 1 行に
-- しかならないので、**気付くのは稼働状況の画面**（想定間隔の 3 倍後）。

DO $$
DECLARE
  just_added boolean := false;
BEGIN
  IF NOT EXISTS (
    SELECT 1 FROM information_schema.columns
     WHERE table_schema = 'core' AND table_name = 'source'
       AND column_name = 'external_id_kind'
  ) THEN
    ALTER TABLE core.source
      ADD COLUMN external_id_kind text NOT NULL DEFAULT 'record';
    just_added := true;
  END IF;

  -- **この列が生まれた瞬間に居た行だけを `'none'` へ倒す**（tasks 1.2）。
  --
  -- ST03 より前に登録簿へ入っている行は、外部サービスという概念そのものが無かった
  -- 時代のもの（端末と PC の 5 ソース）なので、識別子を持たない。
  -- 既定の `'record'` のままにすると **端末からの記録が全件 400 になる**。
  --
  -- **「まだ外部ソースが無いから全行」ではなく「列を足した回だけ」で絞る**のが要。
  -- `migrate()` は起動のたびに全版を当て直すので、`WHERE external_id_kind='record'`
  -- のような条件で毎回撃つと、**後から登録した外部ソースが再起動のたびに
  -- `'none'` へ落ちる**（＝識別子の要求が黙って消える）。
  IF just_added THEN
    UPDATE core.source SET external_id_kind = 'none';
  END IF;
END $$;

-- 列挙は後から締める（`CHECK` は列の追加と分けて、当て直しでも壊れないようにする）。
DO $$
BEGIN
  IF NOT EXISTS (
    SELECT 1 FROM information_schema.table_constraints
     WHERE constraint_schema = 'core' AND constraint_name = 'source_external_id_kind_known'
  ) THEN
    ALTER TABLE core.source
      ADD CONSTRAINT source_external_id_kind_known
      CHECK (external_id_kind IN ('record', 'subject', 'none'));
  END IF;
END $$;
