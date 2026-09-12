-- 戻し手順（202609120943_version_and_ledger）。**不可逆** ——
-- `core.event_version` を落とすと、外部サービス由来の更新で置き換わった**前の版の原文が
-- 永久に失われる**（Q1 の答えで、前の版はここにしか無い）。
-- `core.erasure_ledger` を落とすと、本文を消した事実の台帳が消える（FR-51 が「残す」と決めている）。
DROP VIEW IF EXISTS core.event_folded;
DROP VIEW IF EXISTS core.event_version_live;
DROP TABLE IF EXISTS core.erasure_ledger;
DROP TABLE IF EXISTS core.event_version;
