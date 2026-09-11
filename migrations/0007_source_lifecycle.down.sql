-- 0007 の戻し手順。**不可逆**。
--
-- 失われるもの:
--
-- 1. `core.source.retired_on` —— どのソースがいつ退役したか。登録簿にしか無い事実で、
--    記録からは再現できない（退役は「もう記録が来ない」ことの宣言なので、
--    記録の不在からは「途絶」と区別できない）。
-- 2. `core.source.succeeds` —— 引き継ぎの鎖。これを落とすと、名前を分けたソースの
--    収集開始日が鎖の根に届かなくなり、**成功条件 1 の窓が新しい名前の登録日まで縮む**。
-- 3. 引き直した `collection_started_on` は戻らない（前進側は値を書き換えるだけで、
--    書き換える前の値をどこにも残していない）。
ALTER TABLE core.source DROP CONSTRAINT IF EXISTS source_succeeds_not_self;
ALTER TABLE core.source DROP CONSTRAINT IF EXISTS source_succeeds_fkey;
ALTER TABLE core.source DROP COLUMN IF EXISTS succeeds;
ALTER TABLE core.source DROP COLUMN IF EXISTS retired_on;
