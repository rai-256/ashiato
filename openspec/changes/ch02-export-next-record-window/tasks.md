# CH02 実装タスク — 書き出しの「次の記録」の引きを窓関数の一度読みにする

読む順: `deep.md`（**本人の答え Q1〜Q4**。第 1 回は推奨で先に書いてある）→ このファイル → `specs/data-durability/spec.md` → `design.md` → `premises.md` →
`docs/changes/CH02.md` → `crates/server/src/export/excluded.rs` → `CLAUDE.md`。

**移行は足さない。索引も足さない**（design Non-Goals）。**呼び出し側（`dump.rs`・`summary.rs`・`ledger.rs`）と公開の形は触らない**（design D5）。
**`SPAN_RULES` / `NO_COORD_SOURCES` / `COORD_SOURCES` などの一覧は触らない**（走っている ST09 が行を足す。proposal の Impact）。

検証は各タスクの本文に書いてある。「動いた」ではなくコマンドと終了コードで判定する。DB を使う検査は `docker compose up -d db` が前提。

## Global Constraints

- **テストには `Scenario: <名前>` の印を置く。** Rust はコメント（`// Scenario: 同じ時刻に並んだ終わりを持たない記録は、どれも時刻が後の次の記録までの幅で読む`）。
  `scripts/check_scenarios.py` が spec の全 Scenario と突き合わせ、印の無い Scenario を FAIL にする。印の名前は spec の `#### Scenario:` と**一字一句合わせる**
- **この change の Scenario は 28 本**（`data-durability` の MODIFIED 1 つ 26 本・ADDED 1 つ 2 本）。うち **CH02 で新しく足したのは 11 本**（MODIFIED に 9・ADDED に 2）。
  **新しく足した 11 本は全部、このファイルのどこかの Task が名指しする。** 残る 17 本は MODIFIED で写した既存分で、印は既存のテスト（`export_file_tests.rs`・`export_job_tests.rs`）のまま
- **件数つき検証**: このファイルで **`CT <名前>`** は
  `bash -c 'set -a; [ -f .env ] && . ./.env; set +a; scripts/quiet-run ct -- cargo test -p ashiato-server <名前>'` rc=0 かつ
  `cargo test -p ashiato-server <名前> -- --list 2>/dev/null | grep -c ': test$'` が、その項目に書いた本数（`-- --list で N 本以上`）以上
  を指す（`cargo test` に絞り込みを 2 つ渡すと `unexpected argument` で落ちる。1 つずつ書く。0 本でも rc=0 になるので `-- --list` の下限を必ず見る）
- **試験の名前の接頭辞は `ch02_`**: `ch02_span_`（Task 1・2）/ `ch02_same_`（Task 3）/ `ch02_scale_`（Task 4）。Task 5 は既存の量の試験を直す。新しい試験は `crates/server/src/export_window_tests.rs` に置き、
  `lib.rs` の他の `*_tests` と同じ形で `#[cfg(test)] mod export_window_tests;` を足す
- **新しい試験はまっさらな DB（`crate::stay_tests::fresh_db()` に `crate::MIGRATIONS` を全部当てたもの）で走らせる。** 共有の DB は他の試験の消した滞在が混ざり、
  いまの SQL がそこで 84 秒の型で遅い（design D6）。`export_excluded_sql_every_registered_source_is_classified_by_name` の移行の当て方に合わせる。試験の終わりに `drop` を待つ
- **本人の決定（下流は変えない）**: Q1 同じ時刻の記録は飛ばして「時刻が後の最初の記録」を次にする / Q2 いまの SQL を試験に残して新旧を突き合わせる /
  Q3 読んだ行の数を N と 4N で比べる + 量の試験で時間を 1 回記録 / Q4 消した滞在 0 件なら窓を回さず、消した時間の終わりより後は組ごとの最初の 1 行だけを窓に入れる。
  （第 1 回は推奨で先に書いた。本人の答えが違えば上流がここを書き直してから下流へ渡す）
- **仮決めの値は定数にして試験が名指しで固定する**（`docs/testing.md` §3）: `SCALE_N = 1_000`・`SCALE_FACTOR = 4`・`SCALE_LIMIT = 8`（design D7。4N の量が N の 8 倍未満）
- **窓の枠は design D1 の形（逆向きの並びで頭を固定した `RANGE … '1 microsecond' PRECEDING` の `min`）で書く。** 順向きで尾を固定した枠は 2 乗に戻る（`premises.md`）。
  読んだ行の数では見分けられないので、Task 4.4 の断片の文字列の試験で落とす（design D7）
- **窓に `payload` / `raw` を載せない**（design D2）。**窓に入れる行を緯度経度で絞らない**（spec: 緯度経度を持たない記録も次の記録になる。ST33 D17 の論拠）
- **一時表を作らない**（写しは `READ ONLY`。design D5）
- 書き出しの値（緯度経度・原文）をログと試験の出力に出さない（件数・時間・読んだ行の数だけ）

## Task 1: 試験の足場と、いまの SQL の写し

- [x] 1.1 `crates/server/src/export_window_tests.rs` を作り、まっさらな DB を作って全移行を当てる足場（`fresh()`）と、その DB に記録・前の版・消した滞在を置く道具
  （`export_excluded_tests::World` の `put_as` / `put_version` / `erased_stay` / `mark_deleted` と同じ列。**端末に `NULL` を渡せる**・**前の版の `user_id` を親と違う値にできる**・時刻をマイクロ秒で渡せる）を置く。
  断片を通して**残る**記録と前の版の id を返す `kept_ids(pool, sql: &ExcludedSql, ids) -> (BTreeSet<Uuid>, BTreeSet<Uuid>)`（`event_kept` / `version_kept` を通す。`export_excluded_tests.rs` の `kept` / `kept_versions` と同じ。
  前の版がファイルに出るかは親が除かれるかにも依るので、`excluded_version` の id だけで見ない）と、除く id の集合を返す `excluded_ids(pool, sql)` を置く
- [x] 1.2 いまの `excluded.rs` の `excluded_sql` / `overlap_sql` を、同じファイルの `legacy` モジュールに**逐語で**写す（`legacy::excluded_sql(snapshot_at)` と、記録の側の条件だけを
  `n.event_time > r.event_time` にした `legacy::excluded_sql_strict(snapshot_at)`）。写した日付と元の commit を doc コメントに書く。本体はこの Task では変えない
- [x] 1.3 足場の確かめ: まっさらな DB に既存の Scenario「終わりを持たない記録は、次の記録までの幅で重なれば出ない」と同じ配置を置き、`export::excluded_sql("now()")` と `legacy::excluded_sql("now()")` の
  除く集合が一致する試験 `ch02_span_legacy_matches_current_before_the_change` を置く。
  検証: `CT ch02_span_legacy` rc=0（`-- --list` で 1 本以上）

## Task 2: 次の記録を窓関数の一度読みにする

- [x] 2.1 先に試験を書く（どれも `fresh()` の DB。`kept_ids` で、記録のファイル・前の版のファイルに出るかを見る。配置は spec の Scenario の WHEN のまま）:
  - `ch02_span_same_time_pair_runs_to_the_later_record` — `// Scenario: 同じ時刻に並んだ終わりを持たない記録は、どれも時刻が後の次の記録までの幅で読む`。
    **いまの断片で落ちることを確かめてから** 2.2 に進む
  - `ch02_span_null_device_runs_within_null_devices` — `// Scenario: 端末を持たない記録は、端末を持たない同じソースの次の記録までの幅で読む`
  - `ch02_span_record_without_coordinates_ends_the_width` — `// Scenario: 緯度経度を持たない記録も、次の記録として幅の終わりになる`
  - `ch02_span_version_runs_to_the_next_record_of_the_parent_group` — `// Scenario: 前の版の幅は、親と同じ組の記録のうち前の版の時刻より後の最初の記録までで読む`
  - `ch02_span_version_stops_before_the_erased_time` — `// Scenario: 前の版の幅は、次の記録が消した時間の前にあれば消した時間に届かない`
  - `ch02_span_version_is_not_the_next_of_another_version` — `// Scenario: 前の版は、ほかの前の版を次の記録にしない`
  - `ch02_span_version_group_uses_the_parent_user` — `// Scenario: 前の版の組の利用者は親の利用者で読む`
  - `ch02_span_marked_record_is_not_the_next` — `// Scenario: 削除の印の付いた記録は、次の記録にならない`
  - `ch02_span_nothing_erased_keeps_everything` — `// Scenario: 消した滞在が無ければ、緯度経度を持つ記録も幅を問わず出る`
- [x] 2.2 `excluded.rs` の `excluded_sql` / `overlap_sql` を design D1〜D5 の形に書き換える: 記録と前の版（親の鍵）を `UNION ALL` した鍵の列だけの並び（D2・D4）を、
  消した滞在 0 件なら One-Time Filter の門で読まず・時刻が `max(end_at)` 以下の行 + 組ごとにそれより後の最初の記録 1 行に絞り（D3）、D1 の枠で次の記録の時刻を求め、`id` で `payload` / `raw` に戻して
  (a)(b)(d) と緯度経度と重なりを当てる。`EXCLUDED_EVENT` / `EXCLUDED_VERSION` の CTE の名前と中身、`ExcludedSql` の公開の形は変えない。doc コメントの「行ごとの相関副問い合わせ」の説明を、
  窓の形と ST33 D5 を覆した理由（CH02）に書き直す。
  検証: `CT ch02_span_` rc=0（`-- --list` で 10 本以上。1.3 を含む。**1.3 は同じ時刻の組を置かないので新しい断片でも通る**）/ `CT export_excluded` rc=0（`-- --list` で 24 本以上）

## Task 3: 新旧の突き合わせ（変わらないことの固定）

- [x] 3.1 `ch02_same_matches_the_strict_legacy_on_every_family` — まっさらな DB に design D6 の偽データの族を全部置き（同じ時刻の組・端末 NULL・前の版の時刻のずれ 0 と 5 秒・前の版の `user_id` が親と違う・
  削除の印を間に挟む・緯度経度を持たない行や消した時間より後の行が次の記録になる配置・ほかの前の版が次の記録の候補にならない配置・1 µs の差・(a) の読めた終わりが記録の時刻より前・
  ソースの種類（`c01-location` の点・`end` の終わり・`visit_duration_us` の長さ・`c02-window` の次の記録まで）・消した滞在の前の版の広い時間）、
  `export::excluded_sql(t)` と `legacy::excluded_sql_strict(t)` の除く記録と前の版の id の集合と、`kept_ids` の残る集合が**一致する**ことを見る。
  `t` は `now()` と、固定の時刻の式 3 つ（`'<日>T00:30:00Z'::timestamptz`（消した滞在の始まりより前。組ごとの後の最初の 1 行が効く配置。design D3）・`'<日>T01:30:00Z'::timestamptz`（消した時間の中）・
  `'<日>T05:00:00Z'::timestamptz`（後））。各族の配置は、族ごとに 1 つは「除かれる」と「残る」の両方を含むことを assert する（どちらも空で一致するのを防ぐ）
- [x] 3.2 `ch02_same_differs_from_the_legacy_only_on_same_time_pairs` — 3.1 と同じ偽データで `legacy::excluded_sql("now()")`（いまの SQL そのもの）と比べ、
  差が「同じ時刻の組の、`(event_time, id)` の順で先の記録」の名指しした id だけで、新しい断片の側にだけあることを見る（Q1。除く側にだけ変わる）
- [x] 3.3 `ch02_same_nothing_erased_matches_the_legacy` — 消した滞在 0 件の DB（3.1 の偽データから消した滞在だけを置かない）で、新旧どちらも除く集合が空であることを見る
- [x] 3.4 `ch02_same_runs_in_a_read_only_transaction` — `BEGIN READ ONLY` の中で新しい断片の `excluded_event` / `excluded_version` を数えて、エラー無しで 3.1 と同じ件数になることを見る。
  検証: `CT ch02_same_` rc=0（`-- --list` で 4 本以上）

## Task 4: 読んだ行の数が線形に近いことと、窓の枠の試験

- [x] 4.1 計画の JSON（`EXPLAIN (ANALYZE, BUFFERS, FORMAT JSON)`）の木から、`Relation Name` が `event` / `event_version` の節の
  `(Actual Rows + Rows Removed by Filter + Rows Removed by Index Recheck) × Actual Loops` を足す `rows_read(plan: &Value, skip_cte: Option<&str>) -> u64` を置く（design D7。`skip_cte` は名指しした CTE の下の節を数えない）。
  計画の JSON の形を 1 つ手で書いた材料に当てる単体の試験 `ch02_scale_rows_read_sums_the_event_scans` を置く
- [x] 4.2 `ch02_scale_four_times_the_rows_reads_less_than_eight_times` — `// Scenario: 記録を 4 倍にしても、除く範囲の判定が読む量は 4 倍の桁に収まる`。
  まっさらな DB 2 つに、消した滞在 1 件（01:00〜02:00）と、00:00〜00:59 に散らした地図の座標つきの `c02-window`（**端末を記録ごとに違う値にする**）を `SCALE_N` 件と `SCALE_N × SCALE_FACTOR` 件、
  別の利用者の `c02-window` を 03:00 以降に同じ数だけ置き、`ANALYZE` を当ててから、`SELECT (SELECT count(*) FROM excluded_event), (SELECT count(*) FROM excluded_version)` を `EXPLAIN` して
  `rows_read` の比が `SCALE_LIMIT` 未満であることを見る。**同じ試験で `legacy::excluded_sql("now()")` にも当て、比が `SCALE_LIMIT` 以上であることを assert する**（相関副問い合わせに戻すと落ちる。
  review/spec.md R2 の実測で比 16）。消した滞在を 00:00 より前に置いた配置（行の大半が `max(end_at)` より後）でも新しい断片の比が `SCALE_LIMIT` 未満であることを見る。読んだ行の数と比だけを出す
- [x] 4.3 `ch02_scale_nothing_erased_reads_no_records` — `// Scenario: 消した滞在が無ければ、除く範囲の判定は記録の表を読まない`。
  消した滞在 0 件の DB で同じ `EXPLAIN` を取り、`rows_read(plan, Some("erased_spans"))`（消した滞在とその前の版を探す `erased_spans` の CTE の下を除く）が 0 であることを見る。
  **同じ試験で `legacy::excluded_sql("now()")` では 0 にならない（表を読む）ことも assert する**（design D3 の門がいまより強い保証であることの固定）
- [x] 4.4 `ch02_scale_window_frame_is_head_fixed` — `export::excluded_sql("now()").ctes()` の文字列に design D1 の枠
  （`ORDER BY event_time DESC` と `RANGE BETWEEN UNBOUNDED PRECEDING AND '1 microsecond' PRECEDING`）があり、`FOLLOWING AND UNBOUNDED FOLLOWING` が無いことを assert する
  （順向きで尾を固定した枠は読んだ行の数では見分けられない。design D7）。
  検証: `CT ch02_scale_` rc=0（`-- --list` で 4 本以上）

## Task 5: 量の試験で時間を記録する

- [x] 5.1 `export_file_tests.rs` の `export_volume_one_run_reports_time_and_memory` に、環境変数 `ASHIATO_EXPORT_VOLUME_ERASED`（既定 `1`。`0` なら消した滞在を置かない）を足す。
  `0` のときは「消した時間の中の座標の行が残っている」の assert を「全部残る（`mine == rows`）」に替える。既定の振る舞い（2 万行・消した滞在あり）は変えない。
  検証: `CT export_volume_one_run_reports_time_and_memory` rc=0（`-- --list` で 1 本以上）
- [x] 5.2 使い捨ての DB で 6 通り（`ASHIATO_EXPORT_VOLUME_ROWS` = 10000 / 100000 / 1200000 × `ASHIATO_EXPORT_VOLUME_ERASED` = 1 / 0）を走らせる。1 通りごとに（design D8）:
  `docker compose exec -T db psql -U postgres -c 'CREATE DATABASE ch02_volume_<行数>_<0|1>'` →
  `bash -c 'set -a; . ./.env; set +a; export DATABASE_OWNER_URL="${DATABASE_OWNER_URL%/*}/ch02_volume_<行数>_<0|1>" ASHIATO_EXPORT_VOLUME_ROWS=<行数> ASHIATO_EXPORT_VOLUME_ERASED=<0|1>; scripts/quiet-run vol -- cargo test -p ashiato-server export_volume_one_run_reports_time_and_memory -- --nocapture'` rc=0 →
  出力の `kind=export rows=… seconds=… summary_seconds=…` を design の「計測の記録」の表に写す →
  `docker compose exec -T db psql -U postgres -c 'DROP DATABASE ch02_volume_<行数>_<0|1> WITH (FORCE)'`
  （`psql` の利用者名は compose の `POSTGRES_USER` に合わせる）。`summary_seconds` が 3 秒以上かを D13 の欄に書く（替えるかは本人が決める。替えない）。
  120 万行が 30 分を超えて回らなければ design D8 の反転条件どおり 10 万行までにして、その旨を表の下に書く。
  検証: design.md の「計測の記録」の表に空の欄が無い（`grep -c '| |' openspec/changes/ch02-export-next-record-window/design.md` が 0）

## Task 6: 既存の試験を全部通す

- [x] 6.1 ST33 の export の試験を名前で全部走らせる。
  検証: `CT export_excluded` rc=0（`-- --list` で 24 本以上）/ `CT export_file` rc=0（`-- --list` で 43 本以上）/ `CT export_job` rc=0（`-- --list` で 38 本以上）/
  `CT export_ledger` rc=0（`-- --list` で 3 本以上）/ `CT export_tests` rc=0（`-- --list` で 18 本以上）/ `CT ch02_` rc=0（`-- --list` で 18 本以上）
- [x] 6.2 export の試験を CH02 の木で全部通す（依頼「ST09 の試験も通す」。design D10）。**CH02 の下流の worktree の中で**（`scripts/` の symlink・`.env`・port をそのまま使う）走らせる。
  ST09 のコードを重ねた上での試験（`st09_exp_` / `export_`）は、確認バッチの統合の木に任せる（下の訂正）。その旨を PR の本文に書く。終わった後に `git status --short` が空であることを確かめる。
  検証: `CT export_` rc=0（`-- --list` で 126 本以上）/ `git status --short` が空
  （訂正 2026-10-23: 検証の入口を「CT st09_exp_」から上の入口に。受け入れ条件は変えない。理由: ST09 を merge --no-commit で重ねて撮った証跡は merge --abort の後に作業ツリーと食い違い、Task gate で常に古いコード扱いになる（ST09 の移行がテスト DB に残り export の試験 77 本も落とした）。CH02 の木では st09_exp_ が 0 本で FAIL。ST09 の上での試験は確認バッチの統合の木に任せる 実証: evidence.jsonl 2026-10-23T09:33:29+00:00 FAIL。承認: ユーザー 2026-10-23 の会話（6.2 は CT export_ だけにし、ST09 の上での試験は確認バッチに任せる案を承認）。記録: plan-corrections.md）
- [x] 6.3 crate 全体の試験と静的検査。
  検証: `bash -c 'set -a; [ -f .env ] && . ./.env; set +a; scripts/quiet-run ct -- cargo test -p ashiato-server'` rc=0 /
  `scripts/quiet-run clippy -- cargo clippy -p ashiato-server --all-targets -- -D warnings` rc=0 / `python3 scripts/check_scenarios.py . ch02-export-next-record-window` rc=0
