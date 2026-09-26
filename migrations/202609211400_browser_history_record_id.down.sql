-- SPDX-License-Identifier: AGPL-3.0-only
-- rollback でも既に取り込んだ履歴の鍵の宣言は変えない。
UPDATE core.source SET external_id_kind = 'none'
 WHERE logical_source = 'c02-browser-history'
   AND external_id_kind <> 'none'
   AND NOT EXISTS (
       SELECT 1 FROM core.event WHERE logical_source = 'c02-browser-history'
   );
