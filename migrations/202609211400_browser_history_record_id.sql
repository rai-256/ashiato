-- SPDX-License-Identifier: AGPL-3.0-only
-- ST08: 履歴は訪問ごとの外部識別子で更新・版管理する。
-- migrate() は起動ごとに全移行を当て直すため、既に履歴がある環境の
-- 宣言を戻さない。鍵を変えると既存行との対応が切れる（deep Q3）。
UPDATE core.source SET external_id_kind = 'record'
 WHERE logical_source = 'c02-browser-history'
   AND external_id_kind <> 'record'
   AND NOT EXISTS (
       SELECT 1 FROM core.event WHERE logical_source = 'c02-browser-history'
   );
