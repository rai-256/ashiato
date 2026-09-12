-- 戻し手順（202609120940_source_columns）。**不可逆** ——
-- 列を落とすと、どのソースが「記録ごと」の識別子を要求していたかが失われる。
-- 戻したあとに再び当てると、既存の行は `'none'` ではなく `'record'` の既定を受けるので、
-- **端末からの記録が全件 400 になる**（列が生まれた回の backfill は 1 度きり）。
ALTER TABLE core.source DROP CONSTRAINT IF EXISTS source_external_id_kind_known;
ALTER TABLE core.source DROP COLUMN IF EXISTS external_id_kind;
