# ST23 の前提（外の形式と振る舞い）

問いを立てる前に確かめた。確かさの段は skill の §1.5（1 言っただけ 〜 5 実物で再現）。

| 前提 | 何が依るか | 確かめ方 | 結果 | 出所 | 確かさ |
|---|---|---|---|---|---|
| PostgreSQL は `UPDATE` の後も古い行の中身をデータファイルに残す。ふつうの `VACUUM` でも残り、`VACUUM FULL`（表の書き直し）で表のファイルから消える。WAL（先書きログ）には書き直しの後も残る | Q6（ディスクの痕跡を追い出すか）・完了の判定「本文が DB から消え」の意味 | 使い捨てのコンテナ（`postgres:17-alpine`。ashiato と同じ版）で `INSERT` → `CHECKPOINT` → データファイルを `grep` → `UPDATE raw='', payload='{}'` → `grep` → `VACUUM` → `grep` → `VACUUM FULL` → `grep`。WAL のファイルも `grep` | `UPDATE` 後: 表のファイルに 1 件残る / `VACUUM` 後: 1 件残る / `VACUUM FULL` 後: 新しいファイルに 0 件・古いファイルは無い / WAL: 1 ファイルに残る（`wal_level=replica`） | 実物（2026-10-21 実測） | 4 |
| ashiato の DB は WAL を外へ写していない（`archive_mode` の設定が repo に無い） | Q6 の「WAL に残る」がどこまで及ぶか | `grep -rn "archive_mode\|archive_command"`（compose・tools・設定） | 0 件。WAL は再利用で上書きされるまでデータディレクトリにだけある。バックアップ（FR-66。ST30）はまだ無い | repo | 2 |
| 書庫の写しは、読んだ製品のファイルを**丸ごと**持つ（記録 1 件ぶんだけを抜けない）。既定は「残す」 | Q4（書庫の写し） | `crates/server/src/archive/worker.rs:2327`（写すのは製品のファイルの全バイト `file.bytes`）/ `:1085-1096`（`core.archive_file` に `inner_path` と `stored_path`）/ `:1598-1600`（読み直しは写しのファイルを丸ごと読む）/ 正典 `external-ingestion`「本人のファイルは消さず、読んだ製品のファイルの写しを残す」/ 既存のテスト `archive_copy_uses_content_hash_as_the_single_copy_name` を走らせた（`scripts/quiet-run premise -- cargo test -p ashiato-server --lib archive_copy_uses_content_hash_as_the_single_copy_name` rc=0・1 本。写しの中身が渡したバイトと一致） | 写し 1 つ = 製品のファイル 1 つ（ロケーション履歴・マイアクティビティなど）。中の 1 件だけを消すには書き換えるしかなく、書き換えると「写しの中身が書庫の中のファイルと一致する」が崩れる | コード・正典・既存のテスト材料 | 4 |
| 本文の消去の後も `event_time` と `content_hash` は残り、同じ内容の再送は一意索引に当たって入らない | C1（鍵を残す）・壊してはいけないもの「消したものが再流入しない」 | `migrations/202609120944_gates.sql:105-110`（消去の形は `event_time` と `content_hash` を動かさない）/ `migrations/202609081618_envelope.sql` の `event_dedup_hash`（`logical_source, content_hash` の一意索引。削除済みの行とも衝突する） | 消去した行は鍵を持ったまま残るので、再送も書庫の読み直しも重複になる | コード（辿った: 消去の形は鍵を動かさない → 一意索引は削除済みの行とも衝突する。外れる場合 = 解析器を直して出来事の時刻の読み方が変わると鍵も変わり、新しい鍵で入る。Q4 の context に書いた） | 3 |
| 内容の鍵は塩の無い SHA-256（ソース・出来事の時刻・原文） | C1（鍵から原文を当てられるか） | `crates/server/src/ingest.rs:170-190` / `docs/handoff/ST23.md` の st19 Q1 | 候補の原文を持っている人は、鍵と照らして「その時刻にその原文があった」を確かめられる（原文を知らない人が鍵から原文を作ることはできない） | コード | 2 |
| 滞在（`origin = 'derived'`）の行の書き換えは、台帳の門を通らない | C5（滞在の消去にも台帳を要る形にする） | `migrations/202609120944_gates.sql:83-85`（`OLD.origin <> 'collected'` は門を素通し） | 滞在の本文は、いまは台帳の行なしで空にできる（FR-51「台帳の行が同じまとまりに無い消去は DB が拒む」が滞在に及んでいない） | コード | 2 |
| 滞在の本文を消すと、消した時間帯の終わりが引けなくなる | C13（範囲を台帳から読む）・Q3・Q5 の行 | `core.stay_erased` の定義（`migrations/202609271716_deletion_ledger.sql:60`。終わりは `payload->>'end'`）を使い捨ての `postgres:17-alpine` に当て、滞在を消去の形にする（review/deep.md R1） | 8:31–12:04 の滞在の `end_at - start_at` が `00:00:00.000001` になった | 実物（2026-10-21。deep-review の実測） | 4 |
| サーバの DB の役割（表の所有者でない `ashiato_app`）では `VACUUM FULL` が走らず、警告だけ出して成功で返る | Q6（書き直す選択肢の費用） | 使い捨ての `postgres:17-alpine` で、所有者でない役割に `SELECT, INSERT, UPDATE, DELETE` だけ付与して `VACUUM FULL`（review/deep.md R15） | `WARNING: permission denied to vacuum "event", skipping it`・rc=0・`relfilenode` 不変。書き直すには `MAINTAIN`（PG17）を足すか所有者で繋ぐ | 実物（2026-10-21。deep-review の実測） | 4 |
| 実物の量での表の書き直しの時間 | Q6（待つ時間） | 測っていない。`core.event` は 1 年で約 1 GB の見込み（`openspec/changes/archive/2026-10-17-st33-data-export/deep.md:44`） | 不明（下流で seed の最大量に当てて測る） | 既存の見積もり | 1 |

失われるもの（`loss`）のある決定（Q2・Q3・Q4）は、どれも**どれだけを消すか**の決定で、選択肢の失われ方は確かさ 4 の前提（書庫の写しの形・範囲が引けなくなること）に載せた。
確かさ 3 の「鍵が残るので再送は入らない」は、どの選択肢でも同じに効く帰結として context に書いただけで、選択肢の失われ方を分けない（外れる場合は Q4 の context に書いた）。
