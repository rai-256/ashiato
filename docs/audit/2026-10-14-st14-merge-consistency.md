# 整合の確認（2026-10-14・ST14 の FR-35 等の改訂を、main（ST21 / ST25 の archive・ST33 の NFR-16）を取り込んだ木で）

- 基準: `d7994a766b6057e9b60f3c8b74b21235865a9baf`..`4dc8e147381c39e20bdd5e81f43e4cf48e875051`

対象は、ST14（`openspec/changes/st14-outage-alert/`。上流・仕様への合意は accepted）を origin/main（`c2e7f08f`）へ rebase した木。
main 側が持ち込んだもの（merge-base `a8d13ebb` から `c2e7f08f`）: 要件の改訂は NFR-16 の 1 項目（ST33 の深掘り Q1 / Q5 / Q6〜Q9。本人が消した場面を除く・合言葉で暗号化した 1 つのファイル）、
ST21（`personal-entities`。`migrations/202610020030_places.sql`・`crates/server/src/places.rs`）と ST25（`browsing-views`。`day.rs` / `day_records.rs`・`migrations/202610121040_event_by_user_source_ingest.sql`）の archive、ST33 の上流、`docs/handoff/ST47.md` の追加。
重点は、main 側の改訂と archive が ST14 の文書（proposal / specs/collection-coverage / design / tasks / deep.md / `docs/handoff/ST14.md`）と食い違わないか。
文書とコードは head（`4dc8e147`）の内容で読んだ。ST33 の下流のコードは、ローカルの枝 `feat/st33-data-export`（`5f94b84e`。いまの main を含まない）で読んだ。
前回の監査（`2026-10-13-st14-consistency.md` の C1〜C16）と ST33 の監査（`2026-10-13-st33-merge-consistency.md` の C1〜C4 / K1〜K2）で処置済みの指摘は繰り返さない。

機械の入力:
- `req_impact.py --base d7994a76`: 改訂 4 件（FR-9 / FR-35 / FR-54 / NFR-16）・扉 0 項。FR-9 / FR-35 / FR-54 は ST14 自身の改訂（前回の監査で処置済み）。
  NFR-16 の名指しは `docs/handoff/ST33.md` / `ST41.md`・INDEX:56・stories.json:777・ui-direction:23・st33 の各文書・正典 `record-deletion`:60 で、**ST14 の文書には 1 か所も無い**
- `check_chain.py`: 未回収 0・warn 0。FAIL 1 件は「整合の確認の後に要件が変わった（この監査を名指ししない改訂）: NFR-16」（この監査が受けるもの。基準の head `4dc8e147` は NFR-16 の改訂を含む）
- `check_scenarios.py . st14-outage-alert`: Scenario 1106 件・担保あり 997。担保なし 109 件はすべて ST14 の change の Scenario（tasks.md の Global Constraints が言う上流での正常値と一致）。正典の Scenario の印は欠け 0

走っている change（archive 以外）: st08-browser-history / st14-outage-alert（改訂した側）/ st17-daily-feeling / st33-data-export。st33 に印を付ける（C1 と「触らなくてよい」）。

## C1. NFR-16 の書き出しは core の全表・Include の表の全列に分類を求めるが、ST14 が足す台帳 `core.outage_ledger` と登録簿の 3 列が、ST14 の文書のどこにも分類されていない
- 重大度: 中
- 文書: openspec/changes/st14-outage-alert/design.md:57
- 要件: NFR-16, FR-35
- 根拠: ST33 の spec（`openspec/changes/st33-data-export/specs/data-durability/spec.md:322-323`）「記録の schema のどの表も、書き出しに入れるか入れないかが決まっている状態にし、決まっていない表があれば書き出しを始めずに失敗として終える」「書き出しに入れる表のどの列も、出すか出さないかが決まっている状態にし…」。ST33 の design D8（:265）「後から表や列を足す Story は、ここに 1 行足さないと書き出しのテスト（Task 2）が落ちる」。`docs/handoff/ST33.md:32`「器を作る Story … が、ST33 の分類（design D8 の `export::CATALOG`）に自分の表と列を足す」。ST33 の下流の `export.rs`（`feat/st33-data-export`）は `"source"` を `Class::Include { file: "sources.jsonl", excluded_columns: &[], columns: SOURCE_COLUMNS }` に置いている。ST14 の design D2（:61-65）は `core.source` に `outage_target` / `alert_after_days` / `started_on_from` を、D7（:134-151）は新しい表 `core.outage_ledger` を足すが、proposal・design・tasks のどこにも書き出し・NFR-16・ST33 の語が無い（`grep` で 0 件）。Task 1（tasks.md:41-59）の作業と検証にも分類の行が無い。ST33 の下流が先に merge されると、ST14 の Task 11.3（`cargo test --workspace`）で ST33 の Scenario `いまの記録の schema の表と列はすべて分類されている` が落ち、tasks に無い作業が下流に出る。ST14 が先なら ST33 の下流が分類を足すことになるが、ST33 の design（:28「まだ main に無いもの」）は ST14 を挙げていない。どちらの順でも、台帳を書き出しに入れるか（ST33 は削除・消去・書き出しの台帳を `Exclude` にしている）と、本人が変えられる `alert_after_days` を `sources.jsonl` に出すかは、ST14 の側の判断として残る。台帳と 3 列は緯度経度を持たないので、NFR-16 の「除くもの」（削除の印・消した滞在の時間の緯度経度）には当たらない
- 直すところ: その文書（ST14 の design D2 / D7 と tasks Task 1 に「`export::CATALOG` が main にあれば、`outage_ledger` の分類と `source` の 3 列を足す」を書く）。st33 には印だけ（ST14 の表と列を ST33 の下流が受けるかは merge の順で決まる）
- 機械: 捕まえられる（ST33 の Scenario `いまの記録の schema の表と列はすべて分類されている`。ただし両方が main に入った後の 2 本目の CI で初めて落ちる）
- 行き先: handoff ST14（Task 1 で `export::CATALOG` があれば分類を足す。合意の後・tasks 凍結のため上流では直さない。st33 は印だけ）

## C2. ST14 の design と申し送りが、archive で無くなった `openspec/changes/st25-day-timeline/` を指している
- 重大度: 中
- 文書: openspec/changes/st14-outage-alert/design.md:272
- 要件: FR-54
- 根拠: design D23（仮）「ST25 の 1 日の画面（`openspec/changes/st25-day-timeline/specs/browsing-views/spec.md:354-355`）と同じ言葉にし」。`docs/handoff/ST14.md:86`「根拠: `openspec/changes/st25-day-timeline/deep.md` Q5」。ST25 は `openspec/changes/archive/2026-10-13-st25-day-timeline/` に移り、`openspec/changes/st25-day-timeline` は無い（`ls` が No such file）。同じ文言は正典 `openspec/specs/browsing-views/spec.md:582-583`（「スマホ 取得元に残っていない 始まり – 終わり」「スマホ 時計のずれで取れていない 始まり – 終わり」）と `web/src/messages.ts:215-216` にある。D23 が文言をそろえる相手を下流の実装者が開けない
- 直すところ: その文書（design D23 の参照を正典 `openspec/specs/browsing-views/spec.md:582-583` に、handoff ST14:86 の根拠を archive のパスに置き換える）
- 機械: 安く捕まえられる（文書の中の `openspec/changes/<名前>/` のパスが実在するかを見る検査。archive の後に全文書を走査する）
- 行き先: handoff ST14（design D23 の参照の読み替え。docs/handoff/ST14.md の根拠のパスはこの commit で archive に直した）

## C3. ST21 が main に入ったのに、ST14 の文書は `s01-place` を「まだ main に無い」「下流」と書き、spec は対象外の印を付けるソースに `s01-place` を挙げていない
- 重大度: 低
- 文書: openspec/changes/st14-outage-alert/design.md:13
- 要件: FR-35, FR-25
- 根拠: design:13「ST17 の `s01-feeling`・ST21 の `s01-place` はまだ main に無い」。design D2（:69）「無い名前は何もしない（ST17 / ST21 が先に入っていれば当たる…）」。proposal:100-101「ST21（下流。`personal-entities`）が `s01-place` を登録する」「ST25（下流。`browsing-views`）…」。tasks:13「並走中の st08 / st17 / st21 / st25 の change のファイルを触らない」。いまは `migrations/202610020030_places.sql:188-190` が `('s01-place', '場所', 86400, 'none')` を登録し、ST21 / ST25 は archive 済み。ST14 の移行の `UPDATE`（D2）は `s01-place` に当たるようになったが、spec の Requirement（spec.md:12「`c01-clock`・`c01-app-usage-rollup`・`s01-stay`・`s01-attribute` に『対象外』の印を付けて」）と Scenario（:51 の WHEN が読む 4 つ）は `s01-place` を挙げておらず、移行が実際に付ける印（6 つの名前）を spec が固定していない。判定の結果は変わらない —— 場所の記録は `origin='authored'` でしか受け付けられず（正典 `personal-entities` の `place_record_not_authored`、`places.sql:5`）、spec の (2) で外れる
- 直すところ: その文書（design:13 / proposal:100-101 / tasks:13 の状態を「archive 済み」に直す。spec の印の列挙に `s01-place` を足すか、D2 の `s01-place` は出どころで外れる側の重ねがけだと書く）
- 機械: できない（「まだ無い」「下流」は散文の状態）
- 行き先: handoff ST14（状態の散文の読み替えと、印の数え方。判定は変わらない）

## K1. tasks 10.1 の検証コマンドは `tools/stack.sh up` が前景で待つので curl に進まず、ポートも worktree の口を読まない。main の ST25 が同じ形で落ち、深掘りの問いで直している
- 重大度: 中
- 文書: openspec/changes/st14-outage-alert/tasks.md:197
- 要件: —
- 根拠: tasks 10.1「検証: `bash -c 'set -a; . ./.env; set +a; tools/stack.sh up >/dev/null && curl -sf -H "authorization: Bearer ${API_TOKEN}" "http://127.0.0.1:${ASHIATO_PORT:-18787}/coverage?…" > /tmp/cov.json' && jq -e …`」。`tools/stack.sh:4`「前景で待つ（Ctrl-C で全部止まる）」・:115 `wait` —— `up` は戻らないので `&&` の後の curl が走らない。ポートは `tools/stack.sh:28` `BIND="127.0.0.1:${ASHIATO_HTTP_PORT}"`（`tools/ports.sh:13` が worktree ごとに `19000 + 2n`）で、`ASHIATO_PORT` という変数はどこにも無い。tasks:198 の「ポートは `tools/stack.sh` が出す値に読み替える」は `scripts/verify-run` が逐語で走らせるコマンドには効かない。main の ST25 は同じ型（`.env` の 18787 へ curl。18787 には ashiato2-integ のサーバがいて 401）で verify-run 6.1 が rc=22 になり、深掘り第 4 回 Q9 で本人に問い、`plan_fix.py` で `tools/st25-seed-check.sh`（`setsid tools/stack.sh up … &`・`"^画面:"` を待つ・`. tools/ports.sh` の後に `BIND=127.0.0.1:$ASHIATO_HTTP_PORT`・`set -e` を掛けない）へ直した（`openspec/changes/archive/2026-10-13-st25-day-timeline/plan-corrections.md:5-12`）
- 直すところ: その文書（tasks 10.1 の検証を `tools/st25-seed-check.sh` と同じ形の台本にする。上流のうちに直せば、下流で人間に問わずに済む）
- 機械: 安く捕まえられる（tasks.md の検証コマンドで `tools/stack.sh up` が `&` も `setsid` も無く `&&` に続くもの、`ASHIATO_PORT` を読むものを grep で止める）
- 行き先: handoff ST14（下流が Task 10 の前に plan_fix.py で台本へ直す。ST25 の前例）

## K2. design が引く `lib.rs` の行番号が、main の取り込み（ST21 / ST25 の口）でずれた
- 重大度: 低
- 文書: openspec/changes/st14-outage-alert/design.md:17
- 要件: —
- 根拠: design:17「取り込み（`lib.rs:1183`）と生存信号（`lib.rs:2062`）から呼ぶ」、:22「`authorize()`。`lib.rs:515-541`」、:234「`lib.rs:1183` の呼び出しは変えず」。いまの `crates/server/src/lib.rs` では `coverage::touch_started_on(` が :1359 と :2319、`async fn authorize(` が :538（`a8d13ebb` では :1183 / :2062 / :515 で、design の値は merge 前の木のもの）。関数名が添えてあるので辿れる
- 直すところ: その文書（行番号を関数名だけにするか、いまの行に直す）
- 機械: 安く捕まえられる（`path:line` の行に、添えた識別子があるかを見る検査）
- 行き先: handoff ST14（関数名で辿る）

## K3. design D7 は台帳の追記のみを「`archive_ledger` と同じ関数」で縛るが、その関数は ST12 の `.down.sql` が依存を見ずに落とす。ST16 / ST19 / ST21 はこの理由で関数を共有しない
- 重大度: 低
- 文書: openspec/changes/st14-outage-alert/design.md:149
- 要件: —
- 根拠: design D7「追記のみ: `UPDATE` / `DELETE` を断るトリガ（既存の台帳と同じ関数を使う。`archive_ledger` の :142）」。D7 が引く :142 は `SELECT core.archive_append_only('core.archive_ledger');` で、この補助は同じ移行の :146 で `DROP FUNCTION` され実行時には無い。残っていて使えるのはトリガの関数 `core.reject_archive_ledger_change()` だけで、それは `migrations/202609181600_archive_ingestion.down.sql:21` が `DROP FUNCTION IF EXISTS core.reject_archive_ledger_change();`（`CASCADE` なし）で落とす。`outage_ledger` のトリガが残っている間はこの down が依存で失敗する。main に入った ST21 の `migrations/202610020030_places.sql:34-35`「ST03 の `core.reject_truncate()` を使わない —— 使うと、その関数を落とす `202609120944_gates.down.sql` が依存で当たらなくなる（ST16 / ST19 と同じ理由）」、`202609160220_personal_attributes.sql:58-59`「ST16 が実測で踏んだ」。全版を逆順に戻すなら ST14 の down が先に台帳を落とすので当たるが、1 版だけを戻すと当たらない
- 直すところ: その文書（D7 を「台帳のために関数を新しく作る」に直す。ST21 の `core.reject_place_change()` と同じ形）
- 機械: 安く捕まえられる（各 `.down.sql` を単独で当てる検査。`tools/check-migrations.sh` は形だけを見ている）
- 行き先: handoff ST14（台帳用の関数を新しく作る）

## 未確認

- ST47（上流。YouTube のマイアクティビティを視聴・検索・その他に分ける）が足す新しい `c03-*` の論理ソースが、`alert_after_days = 75` を持って登録されるか。ST14 の移行の `UPDATE … WHERE logical_source LIKE 'c03-%' AND alert_after_days IS NULL` は起動のたびに当て直される（`places.sql:14` の「`migrate()` は起動のたびに全版を当てる」）ので、ST47 の移行が ST14 より後の版でも 2 回目の起動で揃う。1 回目の起動から次の起動までは NULL（想定間隔 60 日の 3 倍 = 180 日で判定）になりうる。ST47 がソースを移行で足すか、`ensure_myactivity_source` のような実行時の口で足すかは、ST47 の上流がまだ決めていない（`docs/handoff/ST47.md:3`）。ST14 の Scenario `書庫のソースは 75 日で登録されている` が移行を 1 回当てた DB の全 `c03-%` を読むなら、ST47 の CI で落ちて気づける
- ST33 の下流（`feat/st33-data-export`）が、取り込んだ後の main で `core.place`（ST21）を `CATALOG` に足すか。いまの枝の `CATALOG` に `place` は無く、枝はいまの main を含まない。ST33 自身の Scenario が rebase の後に落ちるので、ST14 の文書の問題ではない
- `tools/check-immutable.sh` に ST14 の台帳の追記のみを psql から直に確かめる節を足すか（ST21 は足した）。tasks 1.2 は `tools/check-immutable.sh` rc=0 を検証に挙げるが、節を足すとは書いていない。足すかは下流の判断で、食い違いではない

## 触らなくてよいと判断したもの（理由つき）

- **NFR-16 の「除くもの」（削除の印の付いた記録・消した滞在の時間に入る緯度経度を持つ行）と ST14** —— ST14 が足す台帳（`episode_id` / `device_id` / `last_alive_at` / `judged_at`）と登録簿の 3 列は緯度経度を持たず、削除の印の対象でもない。取りこぼしの期間（`c01-app-usage` の `gap`）・時計の測定（`c02-window` の `clock-skew`）は既存の記録で、ST14 は数え方を変えるだけで新しい記録を作らない。響くのは「書き出しの対象」の分類だけ（C1）
- **ST21 の `s01-place` の論理ソース名・登録簿・本人が書く分類** —— 名前は `s01-place`（`places.rs:20` の `SOURCE`・`places.sql:189`）、想定間隔 86400、`external_id_kind = 'none'`。記録は `origin='authored'` だけを受け付け端末識別子を持たない（正典 `personal-entities` の `place_record_not_authored`、`places.sql:5`）。ST14 の spec の (2)（収集した記録を 1 件も持たず、本人が書いた記録を持つ）・design D3・`docs/handoff/ST14.md:132`「ST21 が `origin='authored'` で登録」と一致する。ST21 が足す論理ソースは `s01-place` の 1 つだけ（`INSERT INTO core.source` は `places.sql:188` のみ）。`places.sql` の注記「`expected_gap_sec` は稼働状況に使われない」は、ST14 の後も `outage_target = false` で外れるので成り立つ
- **ST25 / ST21 が稼働の判定（生存信号・最後の記録の数え方・日の状態）に触れているか** —— 触れていない。main の差分で `coverage.rs` と正典 `collection-coverage` は変わっていない。`day.rs` / `day_records.rs` は `crate::coverage` を使わず（import は `day_records` と `stay_store` だけ）、`collection_started_on` を直に読むのは位置（`c01-location`）とブラウザ履歴（`c02-browser-history`）だけで、ST14 の D16（集計の `started_on_from`）の外。`lib.rs` の差分は移行 2 本と口 4 本（`/day` `/day/records` `/places` `/places/candidates`）で、`touch_started_on` と生存信号の口の中身は変わっていない。ST25 の索引 `event_by_user_source_ingest`（`(user_id, logical_source, ingest_time)`）は足すだけで、ST14 の D4 が使う `event_by_source_time`（`(logical_source, event_time)`。`202609112113_source_lifecycle.sql:71`）は残っている
- **ST25 の `GET /day` の `Gap` / `GapKind` と ST14 の `DayCell.gaps`** —— OpenAPI の schema 名は ST25 が `Gap` / `GapKind` を取り、ST14 の design D15 は `GapSpan` で重ならない。理由の値（`retention` / `clock_skew_abandoned`）は同じ。区間の鍵が ST25 は `start` / `end`、ST14 は記録の payload に合わせた `begin` / `end` で、口が違う（`/day` と `/coverage`）ので読み違えは起きない
- **稼働状況の画面のアドレスが `#/coverage` に移った（ST25）** —— `docs/handoff/ST14.md:83` が既に書いており、main の `surface.spec.ts:29` / `archive-layout.spec.ts:78` / `coverage-year.spec.ts:22` は `/#/coverage` に直っている。ST14 の新しい e2e（`coverage-gaps` / `coverage-rollup` / `coverage-scheme`）は既存の e2e の開き方に倣えば当たる。design:25 の `App.tsx:72` / `CoverageGrid.tsx:32` / `AchievementPanel.tsx:18`（`useScheme`）・`WeekDetail`（:215）・`orderCoverageWithArchives` は今の行と一致する
- **D23（仮）の区切り `〜` と ST25 の `–`、`スマホ` の前置き** —— D23 が S-1 の破棄の文字にそろえて `〜` にすると明記している。理由の言葉そのもの（「取得元に残っていない」「時計のずれで取れていない」）は正典 `browsing-views`:582-583 と同じ
- **正典 `personal-entities`（ST21）の「稼働状況の画面からマスタ管理へ行ける」（:1559）と ST14 の集計の格子** —— 入口の置き場は ST21 の範囲で、ST14 の「Must の 5 本はひとスクロール以内」は main の `archive-layout.spec.ts` が入口を足した後の画面で測っている
- **`docs/handoff/ST44.md` と `ST47.md`** —— ST14 宛ての例外（`GET /outages` を収集アプリの資格情報で読む。ST44.md:15-21）と 60 日の文・並びの条項（ST47.md:12-32）は rebase の後も残っている。main が足した ST47.md の C4（FR-50 の連鎖）は ST14 に関わらない
- **走っている change への印**（差し戻しではない）:
  - `st33-data-export` —— C1（ST14 の台帳と登録簿の 3 列の分類。どちらが先に merge するかで受ける側が決まる）。`core.place` の分類が無いのは ST33 自身の検査が拾う（未確認）。失われるもの無し —— 分類が無ければ書き出しは始まらずに失敗するので、黙って外へ出ることも黙って落ちることもない
  - `st17-daily-feeling` / `st08-browser-history` —— 前回の監査の印から変わらない（main の取り込みで ST14 との重なりは増えていない）
