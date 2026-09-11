# ST03 実装タスク — 同じ記録を何度送っても増えない

読む順: `deep.md`（**最優先。本人が決めた 26 件**）→ このファイル → `specs/` → `design.md` →
`docs/stories/ST03.md` → `CLAUDE.md`。

**移行は 0007 から**（ST02 が 0005 / 0006 を使う）。`collection-coverage` には触らない。

検証は各タスクの本文に書いてある。「動いた」ではなくコマンドと終了コードで判定する。
DB を使う検査は `docker compose up -d db` と `tools/seed.sh` が前提。

## 1. 登録簿の 3 列

- [ ] 1.1 `migrations/0007_source_columns.sql` で `core.source` に `external_id_kind text NOT NULL DEFAULT 'record' CHECK (external_id_kind IN ('record','subject','none'))`、`retired_on date`、`succeeds text REFERENCES core.source(logical_source)` を足す。検証: `psql -c "\d core.source"` に 3 列が出て、`cargo run --bin server` の起動が rc=0
- [ ] 1.2 同じ移行で、既存の端末ソースに `external_id_kind='none'` を当てる（`UPDATE core.source SET external_id_kind='none' WHERE external_id_kind='record' AND logical_source NOT IN (…外部ソース…)`）。検証: `tools/smoke.sh` が rc=0（当てないと端末の記録が全件 400 になる）
- [ ] 1.3 手書きの登録が散っている 5 か所（`tools/seed.sh` / `tools/smoke.sh` の 2 か所 / `tools/check-immutable.sh` / `collector-android/README.md`）に `external_id_kind` を明示する。検証: `grep -rn "INSERT INTO core.source" tools/ collector-android/README.md` の全件に列が入っている
- [ ] 1.4 `retired_on` が**日付**であることをテストで固定する（真偽値に変えられると、退役より前の途絶が遡って消える）。検証: `cargo test source_retired_on_is_a_date` が rc=0

## 2. 記録の 2 列と取り込みの契約

- [ ] 2.1 `migrations/0008_event_columns.sql` で `core.event` に `source_updated_at timestamptz` と `external_ref text` を足す（`external_ref` に索引は張らない）。検証: `psql -c "\d core.event"` に 2 列、`\di core.event*` に `external_ref` の索引が**無い**
- [ ] 2.2 `IngestRequest`（`crates/server/src/ingest.rs`）に `source_updated_at` と `external_ref` を `Option` で足す。**既存の収集側を壊さない**（省略できる）。検証: `cargo test units_are_optional_in_json` と新規 `optional_new_fields_parse` が rc=0
- [ ] 2.3 `docs/collector-contract.md` の要求の表に 2 項目、`error` の表に断りの種別（`missing_external_id` / `empty_external_id` / `id_reused`）を足す。検証: `cargo run --bin openapi > /tmp/o.json && diff <(jq -S . /tmp/o.json) <(jq -S . docs/openapi.json)` が rc=0
- [ ] 2.4 `validate` に外部識別子の検査を足す —— `external_id` が `Some` なら**非空**であること。検証: `cargo test empty_external_id_is_rejected` が rc=0（**ST01 が `device_id` の空文字で踏んだのと同型。格納の前に断らないと 2 件目で 500 になりまとめ送り全体が止まる**）

## 3. 索引を 2 段にする（BREAKING）

- [ ] 3.1 **先にコードを直す** —— `lib.rs` の `ON CONFLICT (logical_source, content_hash)` を、外部識別子の有無で経路を分け、**部分索引の述語を文に書く**形にする。検証: `cargo test` が rc=0（索引を変える前でも既存の振る舞いが壊れていないこと）
- [ ] 3.2 `migrations/0009_dedup_indexes.sql` で索引を作り替える（`design.md` D1 の 3 本）。検証: `psql -c "\di core.event*"` に 3 本が出て、`tools/smoke.sh` が rc=0
- [ ] 3.3 順序の検査 —— 3.1 より先に 3.2 を当てると取り込みが全件 500 になることを、移行のコメントに書く。検証: `grep -n "0009 より前に" openspec/changes/st03-idempotent-ingest/design.md` が当たり、移行ファイルにも同じ注意がある

## 4. 冪等の判定（2 段・利用者ごと）

- [ ] 4.1 外部識別子を持たない記録: 内容ハッシュで畳む。検証: `cargo test dedup_by_hash_when_no_external_id` が rc=0（3 回送って 1 行）
- [ ] 4.2 外部識別子を持つ記録: 識別子で畳む。検証: `cargo test dedup_by_external_id` が rc=0（3 回送って 1 行）
- [ ] 4.3 内容が同じで識別子が違えば 2 行入る。検証: `cargo test same_content_different_external_id_makes_two_rows` が rc=0
- [ ] 4.4 利用者識別子が違えば畳まれない。検証: `cargo test different_user_not_deduped` が rc=0
- [ ] 4.5 **`content_hash` の作り方が ST01 のままであること**を固定する（Q15 —— 利用者識別子は索引にだけ）。検証: 既存の `cargo test hash_is_pinned` が**期待値を変えずに** rc=0
- [ ] 4.6 収集側の識別子が同じで内容が違えば 400。**同じ要求の他の記録は格納される**。検証: `cargo test id_reused_is_rejected_without_stopping_the_batch` が rc=0

## 5. 更新と履歴

- [ ] 5.1 `migrations/0010_version_and_ledger.sql` で `core.event_version`（**`raw` は `text`**）と `core.erasure_ledger` を作る。どちらも `txid xid8 NOT NULL DEFAULT pg_current_xact_id()` を持つ。検証: `psql -c "SELECT data_type FROM information_schema.columns WHERE table_name='event_version' AND column_name='raw'"` が `text`
- [ ] 5.2 履歴は感度も削除の印も持たない（Q21）。検証: `psql` で `event_version` に `sensitivity` / `deleted_at` の列が**無い**ことを確かめる検査を `tools/check-immutable.sh` に足し、rc=0
- [ ] 5.3 読み出し用のビュー `core.event_version_live`（親と束ね、親の感度と削除を引き継ぐ）を作る。検証: `cargo test history_follows_parent_sensitivity_and_deletion` が rc=0（親を締める / 消すと履歴の版も同じ扱いになる）
- [ ] 5.4 外部識別子が一致し内容が違う到着で、既存行を更新し前の版を履歴へ。検証: `cargo test external_update_keeps_one_row_and_one_version` が rc=0（**Story の完了の判定 2**）
- [ ] 5.5 履歴の原文が更新前のものとバイト単位で一致する（並び・重複キー・指数表記を含む原文で）。検証: `cargo test version_raw_is_byte_identical` が rc=0

## 6. 更新時刻による順序

- [ ] 6.1 届いた更新時刻が保存済みより古ければ書き換えない。検証: `cargo test stale_update_is_ignored` が rc=0（新→旧 の順で送って内容が巻き戻らず、履歴も増えない）
- [ ] 6.2 更新時刻を持たない到着が**保存済みの値を消さない**。検証: `cargo test missing_updated_at_does_not_clear_stored_value` が rc=0
- [ ] 6.3 同じ更新時刻で内容だけ違う到着は「新しい」として扱う（`>=`）。検証: `cargo test same_updated_at_still_applies` が rc=0（`>` だと `accepted` を返しながら内容が変わらず、応答から見えない）

## 7. 書き換えと消去の門（BREAKING）

- [ ] 7.1 `migrations/0011_gates.sql` で `AFTER UPDATE … DEFERRABLE INITIALLY DEFERRED` の制約トリガを置く。**消去の形（原文が空）のときは台帳を、それ以外は履歴を見る**。検証: `tools/check-immutable.sh` の新しい台本が rc=0
- [ ] 7.2 履歴を書かない書き換えが拒まれ、**書けば通る**。検証: 同上の台本に 2 本とも入っていて rc=0
- [ ] 7.3 **台帳の行があっても、消去でない書き換えは通らない**。検証: 同上（絞りが無いと台帳 1 行で改竄が通る）
- [ ] 7.4 履歴と台帳が**追記のみ** —— `UPDATE` / `DELETE` / `TRUNCATE` の 3 経路とも拒まれる。検証: 同上
- [ ] 7.5 **本表の `DELETE` と `TRUNCATE` も拒む**（いまの 0002 / 0004 は `UPDATE` しか見ていない）。検証: 同上
- [ ] 7.6 凍結一覧に `external_id` と `external_ref` を足す（`source_updated_at` は足さない）。検証: 同上（**足さないと、識別子を 1 文書き換えるだけで同じ本文が 2 行入る**）
- [ ] 7.7 `tools/check-immutable.sh` を上の 6 本を通す台本に作り替える。**既存の「4 列の UPDATE が拒まれる」だけを消して終わりにしない**。検証: `.github/workflows/ci.yml` の該当ジョブが緑

## 8. 削除済みの保護

- [ ] 8.1 削除済みと内容ハッシュが一致する記録を、外部識別子が違っても格納しない。検証: `cargo test deleted_content_does_not_return_via_other_external_id` が rc=0
- [ ] 8.2 同じ判定を**更新の経路**にも当てる。検証: `cargo test update_cannot_resurrect_deleted_content` が rc=0（当てないと、生きている別の行が外部の更新で消した本文に化ける）
- [ ] 8.3 取り込まなかった記録を**受理**として返す（収集側が再送を諦められる）。検証: `cargo test deleted_duplicate_is_accepted` が rc=0
- [ ] 8.4 外部識別子を持たない記録では判定を撃たない（`event_dedup_hash` が削除済みを含めて弾く）。検証: `cargo test no_extra_query_without_external_id` が rc=0

## 9. 畳んで読む置き場

- [ ] 9.1 利用者・ソース・内容ハッシュが一致する複数行を 1 件として読む形を作る。検証: `cargo test folded_view_returns_one_row` が rc=0
- [ ] 9.2 **適用は後続 Story**であることを spec とコードのコメントに書く。検証: `grep -n "閲覧・検索・AI・書き出し" openspec/changes/st03-idempotent-ingest/specs/record-envelope/spec.md` が当たる

## 10. 取り込みのトランザクション

- [ ] 10.1 **1 件 1 トランザクション**にし、取り込みと稼働記録の書き込みを同じトランザクションに束ねる。検証: `cargo test one_bad_item_does_not_roll_back_others` が rc=0（門は COMMIT 時に落ちるので、束ねると 1 件の失敗が全件を巻き戻す）
- [ ] 10.2 重複のとき、**格納されている行の識別子**を返す（いまは送り主の識別子を返しており、DB に無い値になる）。検証: `cargo test duplicate_returns_stored_id` が rc=0

## 11. 収集側が断られた記録を諦める

- [ ] 11.1 `Sender.kt` で、恒久的に断られた記録（要求そのものが不正）を未送信から取り除く。**一時的な失敗（到達できない / サーバ側 / 資格情報）では取り除かない**。検証: `./gradlew test --tests '*SenderTest*'` が rc=0
- [ ] 11.2 捨てた件数と理由の種別を残す。検証: 同上（テストがログ行を確かめる）

## 12. 通し

- [ ] 12.1 `tools/smoke.sh` に ST03 の 3 クラス（端末 / 記録ごと / 対象ごと）の再送を足す。検証: rc=0
- [ ] 12.2 **Story の完了の判定 4 項目**を通しで確かめる（`docs/stories/ST03.md`）。検証: `tools/smoke.sh` と `cargo test` が rc=0 で、4 項目それぞれに対応する test がある
- [ ] 12.3 **台帳の行と実際の消去の突き合わせ**（DB は件数を検算しない。台帳 1 行で 4 行消せる）。検証: `tools/check-immutable.sh` に件数の照合を足して rc=0
- [ ] 12.4 `python3 scripts/check_chain.py .` と `python3 scripts/review_triage.py . st03-idempotent-ingest` が rc=0

## 13. ST02 への申し送り（**この change では実装しない**）

ST02（PR #20・draft）に渡してある。**ST03 が merge されてから ST02 が追従する。**

- [ ] 13.1 引き継ぎの確認のみ —— PR #20 のコメントに 4 件（状態 7 → 8 / `retired_on` は日付 / `coverage.rs:423,460` の「記録あり」を `core.event` から引く / 退役の格子を畳む）と、第 8 回 Q31 の答え（収集開始日を引き継ぎの鎖から引く）が残っていること。検証: `gh pr view 20 --comments` に該当のコメントがある
