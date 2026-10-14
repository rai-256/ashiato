# ST47 code review（whole-branch）

## final review（e32f33a..72325ee）

席: final reviewer（`superpowers:requesting-code-review` の `code-reviewer.md`）。入力は review package
`.superpowers/sdd/tasks/review-e32f33a9..72325eef.diff`、Task 1〜6 の指摘の記録の Minor / ⚠️、`docs/handoff/ST47.md`。
判定: **No（直せば merge できる）**。D1・D3・D5〜D11 は design どおり、CT の本数は `cargo test -p ashiato-server -- --list` で全部下限以上。
申し送りの C4（FR-50 の削除の連鎖から漏れない）は満たしている（`stay_store` の `c03-myactivity-` 前置き + 見張り `every_archive_logical_source_is_classified_as_location_or_item` が 3 つの行き先を通す）。

## R1. deep.md Q9 の前提の訂正（日本語の評価は先頭一致）がコード・design・spec・試験の材料に入っていない
- 成果物: crates/server/src/archive/youtube.rs:58-62 / design.md D4 / specs/external-ingestion/spec.md の Scenario「高く評価した動画は視聴に入らない」/ archive_tests.rs:2197・archive_flow_tests.rs:4256/4575/4764/4786 の材料
- 根拠: deep.md:115-122（本人の補足「日本語の評価は先頭一致（高く評価しました  / 低く評価しました ）。本物で 196 件」と「spec の段が直すもの」）。youtube.rs の `ja` の表は `ends_with: true` で `rating: ["を高く評価しました", "を低く評価しました"]`。本物の書庫では評価が 1 件も当たらず、196 件が `c03-youtube-watch` に入って凍結される（言語は視聴・検索で `ja` と当たるので確認待ちにも戻らない）。上流の spec の段が訂正を当てず、下流は古い D4 に忠実に実装した（計画の欠陥）
- kind: technical
- 提案: `Phrases` に向きを決まり文句の組ごとに持たせ（ja の評価は先頭・視聴と検索は末尾、en は先頭）、ja の評価を `"高く評価しました "` / `"低く評価しました "` の先頭一致に。design D4 の表と前置きの文、spec の Scenario の題名、試験の材料を先頭の形に。PARSER_VERSION "2" は未リリースなので上げ直さない
- 処置: fixed D4 — 068cf849。ja の評価を先頭一致に（`youtube.rs` の向きを組ごとに）、D4・spec の Scenario の WHEN・試験の材料を先頭形に、`archive_youtube_phrase_japanese_rating_is_a_prefix_not_a_suffix` を足した。tasks.md 2.2 の本文（日本語は末尾）は凍結のため古いまま —— PR 本文に挙げる

## R2. 申し送りの D20 / R17（正典 external-ingestion の 60 日の文と格子の並び）が spec delta に入っていない
- 成果物: specs/external-ingestion/spec.md（delta）/ docs/handoff/ST47.md
- 根拠: 申し送りは 2026-10-13 13:18 に置かれ、ST47 の spec の段（同 15:35）より前。deep.md:12 の「`docs/handoff/ST47.md` は無い」は 08:35 時点の観測のまま。delta の MODIFIED は「印を置くまで格納しない」の 1 本だけ。ST14 は「ST47 が external-ingestion を触っている」ことを理由にこの直しを譲っている
- kind: technical
- 提案: 正典 `openspec/specs/external-ingestion/spec.md` の Requirement「取り込み器は自分の論理ソースに 1 日 1 回の生存信号を送る」（:588）と「稼働状況の画面に書庫のソースの格子と最終日と直近に置いた書庫が出る」（:676 / :697 / Scenario :709-712）を MODIFIED で delta に写し（Scenario は全部逐語）、60 日の文を「格子は 60 日、通知は最後の書庫から 75 日（`collection-coverage`）」に、並びを「Must の 5 ソース・集計・書庫のソース・退役したソース」に直す。文だけで振る舞いは変わらない
- 処置: fixed 7.5 — dc8ecc1e。正典の 2 Requirement を MODIFIED で Scenario ごと逐語に写し、60 日の文 2 か所と並びの文・Scenario の THEN だけを直した。並びの変化は `web/src/__tests__/archive-order.test.tsx` に集計を挟む場合を足して固定

## R3. `languages: []` の YouTube の形への印は、確認待ちの行が無いと `tools/archive-shape.sh --confirm` の断りをすり抜ける
- 成果物: tools/archive-shape.sh:36-52
- 根拠: 断りは `archive_pending_shape` にそのハッシュの行があるときだけ効く。ハッシュを一覧の外で知って先に印を置くと、表に無い言語のファイルが以後すべてその印で通る（deep.md 第 2 回 Q7 / design D6 が防ぐもの）。起きにくい（ハッシュを一覧の外で知る必要がある）
- kind: technical
- 提案: 格納側で、`languages` が空の YouTube の形は印があっても「印なし」と読む（`--confirm` の断りと二重にする）。それが大きければ断りの道具側で `languages: []` の YouTube の形のハッシュ（`hash_shape` が決定的に作る）を名指しで断る
- 処置: fixed D6 — 9f41b145。`worker.rs` の `is_file_shape_confirmed`（走査と `reread_archive`）と `confirmed_pending_copies` の SQL で、`languages` が空の YouTube の形は印があっても読まない。試験 `archive_youtube_shape_a_mark_on_an_unrecognized_language_does_not_store`。決まり文句を持たない合成の題名の材料を本物の形に揃えた

## R4. design の決定の一覧の D2 / D4 / D6 / D9 が「第 2 回 … で本人に問う中」のまま
- 成果物: design.md:26-36
- 根拠: 答えは deep.md に記録済み。PR 本文で D の（仮）を列挙するときに紛らわしい
- kind: technical
- 提案: 一覧の状態の欄を答えに合わせる（本人が決めたものは「第 2 回 Q<n> で本人が決めた」、仮のまま残るものは（仮）と反転条件）
- 処置: fixed design.md — 21177b53。一覧の状態の欄を第 2 回 Q7 / Q8 / Q9 / Q10 の答えに合わせた（D2 の製品の名前を持たないファイル・D4 の英語の表は仮のまま）

## fix wave（1f9685d..21177b5）の scoped re-review

席: re-reviewer（SDD の `re-review-prompt.md`）。R1〜R4 すべて ADDRESSED、fix の diff に新たな Critical / Important は無し。
fixer の申告: `cargo test -p ashiato-server` 835 passed / 0 failed（`drops_tests::drops_api_idempotent` が DB の deadlock で 1 回落ち、回し直して通過）、
`cargo clippy -p ashiato-server --all-targets -- -D warnings` rc=0。残り: tasks.md 2.2 の本文が古い（凍結のため触らない）。

## code-verify（HEAD 53af3084。独立検証）

席: code-verify。入力は deep / specs / design / tasks と HEAD の作業ツリー。コードは触っていない（変異と追加の試験は `/tmp` の複製でだけ走らせた）。

### 申告: 11/14（`[x]` 11。7.4・7.5・8.1 は `[ ]`）・検証コマンド一覧

| 項目 | 検証コマンド（tasks の本文） | 申告（evidence.jsonl の head） | 実測（HEAD 53af3084） | 一致 |
|---|---|---|---|---|
| 1.1 | `tools/check-migrations.sh` / `CT archive_youtube_other_source_registered (≥1)` | PASS（e32f33a。CT は記録なし） | rc=0 / 1 本・通過。60 日を 30 日にした移行は**まっさらな DB で**落ちる（M20） | 一致 |
| 2.1 | `CT archive_youtube_kind_of (≥4)` / `CT every_archive_…_or_item (≥1)` | 記録なし（task-2-report の申告のみ） | 4 本 / 1 本・通過 | 一致 |
| 2.2 | `CT archive_youtube_route (≥14)` / `CT archive_youtube_phrase (≥5)` / `CT archive_ (≥1)` | 記録なし | 14 / 6 本・通過 / `archive_` は全 835 本の実行に含まれて通過 | 一致（本文の「日本語は末尾」は R1 の処置のとおり古い） |
| 3.1 | `CT archive_youtube_shape (≥8)` | 記録なし | 10 本・通過 | 一致 |
| 3.2 | `scripts/quiet-run smoke -- tools/smoke.sh` | PASS（54ecbd8。fix wave の前） | rc=0（`.superpowers/sdd/logs/20261014-212243-smoke.log`、該当の段 :101-103） | 一致 |
| 4.1 | `CT archive_youtube_reparse (≥5)` / `CT archive_flow_ (≥1)` | 記録なし | 5 本・通過 / 全体 835 本・通過 | 一致（ただし R5） |
| 5.1 | `CT archive_youtube_location (≥1)` / `CT archive_youtube_last_event (≥1)` | 記録なし | 1 / 1 本・通過 | 一致 |
| 6.1 | smoke rc=0 / `grep -qF '"kind": ?"YouTube"' tools/smoke.sh && ! grep -q YouTubeWatch …` / docs の grep | PASS（3123bae。fix wave の前） | rc=0 / rc=0 / rc=0 | 一致 |
| 7.1 | fmt + clippy `-D warnings` / `cargo test --workspace` | PASS（a63f638） | rc=0 / rc=0（133 + 835 + 7 本。`…-212513-wtest.log`） | 一致 |
| 7.2 | `python3 scripts/check_scenarios.py . st47-youtube-myactivity-split` | PASS | rc=0（Scenario 1028・担保あり 1028） | 一致 |
| 7.3 | check_chain / `openspec validate --strict` / check-migrations・openapi・boundaries・immutable・licenses | PASS | すべて rc=0。check-immutable は 1 回目 rc=1（「202609100000_immutable_origin が当たらない」）—— 自分の変異の実行と同じ DB を取り合った環境の衝突で、単独の 2 回目は rc=0 | 一致 |

`archive-order.test.tsx`（fix wave で触った唯一の web）: `npx vitest run src/__tests__/archive-order.test.tsx` 2 本・通過。

### 手 1: 固定値の独立の再計算 —— 一致
- python の `hashlib.sha256` で同一性の JSON（キー整列・区切り無し）を計算: `{"kind":"YouTubeWatch","products":[]}` = `f3e578ff3482…`（deep.md が確認バッチの volume で見た前置きと一致）/ `{"kind":"MyActivity","products":["検索"]}` = `32613b64…` / `{"kind":"ChromeHistory","products":[]}` = `4e6a8512…` / `{"kind":"YouTube","languages":["ja"],"products":["YouTube"]}` = `e130f952…`。試験の直書き（archive_flow_tests.rs の `sha256_hex(br#"…"#)`）はどれも実装の出力の写しではなく、仕様のバイト列そのもの
- 本物の書庫（`~/Downloads/Takeout/マイ アクティビティ/YouTube/マイアクティビティ.json`。件数だけを出した）を、(a) python で D1・D3・D4 を独立に書き直したものと、(b) サーバの `classify_files` → `shape_for_file` → `hash_shape`（複製に `#[ignore]` の試験を足して実行）に通した結果が一致: `products ["YouTube"]`・`languages ["ja"]`・watch 89,368 / search 5,643 / other 23,884（ad 20,093・rating 196）・hash `e130f952…`。deep.md の `watch?v=` 109,334 = 89,368 + 19,770 + 196、tasks 8.1 の「視聴 約 8.9 万・評価 196 前後」とも合う。他の 25 製品のファイルはどれも `MyActivity`（最初の製品が 1 種類）のまま

### 手 2: ガードをわざと壊す（複製で 22 変異。`archive` に当たる 183 本を走らせた）
- 落ちた（守られている）: `is_unconfirmable_shape` を外す（M01）/ `record_pending_file` の `ON CONFLICT` で形を更新しない（M03）/ 確認待ちの読み直しで印を照らさない（M04）/ 読み直しで確認待ちに積まない（M05）/ 取り直しで v2 の台帳の行を足さない（M07）/ 取り直しで形を置き換えない（M09）/ `PARSER_VERSION` を `"1"` に戻す（M18）/ `ledger_meta` を版で絞る（M19）/ 移行を `DO UPDATE` に（M21）
- `tools/archive-shape.sh --confirm` の断りを `if false` に潰した複製で smoke を走らせると rc=1（「言語の当たらない YouTube の形に印が置けてしまう」）。down.sql が頼る外部キーは `core.event` ほか 6 表に実在し、どれも `NO ACTION`（`foreign_key_violation` になる。CASCADE ではない）
- **生き残った**: M06（走査の輪から `refresh_pending_shapes` を外す）→ R5 / M08（評価を広告より先に見る）→ R6 / M17（その他の payload から `details` を落とす）→ R7 / M02（`confirmed_pending_copies` の SQL の断りを外す）→ R8

### 手 3: Scenario と test —— 印は全部ある（rc=0）。階層が違うのは R5 の 3 本
- `[warn]` の 8 件（spec に無い Scenario を指す印）は、この branch の diff の `Scenario:` の印に含まれない（merge-base 由来）。
- 31 本の印の先を 1 本ずつ読んだ。THEN の「入らない」側の 0 件も assert している（例: `archive_youtube_route_both_paths_share_the_watch_source` は 3 ソースとも時刻で数える）。`分けた論理ソースの最終日が稼働状況に出る` は `/archives/status` と同じ `status()` の `sources` を見ている
- WHEN が「解析器の版を上げて**走査が来る**」の 3 本（`…新しい形で一覧に出る` / `旧い形の印があっても…` / `読み終えた書庫の…`）は、試験が関数を直に呼ぶ —— R5

### 手 4: 本人の決定が test で固定されているか
- 固定されている（値を変えると落ちる）: Q1・Q2 の行き先（route 14 本）/ Q4 の ja 先頭一致（M23）・en の表（M24）・ja の評価 2 種（M25）・当たった言語の表だけで見る（M15）・題名は最初に当たった言語だけ（M14）/ Q6 の `{"YouTube"}` 完全一致（M12）・ハッシュに言語（M13）/ C2 の 60 日（M20。ただし共有の開発 DB では `DO NOTHING` に隠れて通る。CI のまっさらな DB でだけ効く）・`DO NOTHING`（M21）/ C3 の広告をその他に数える（M16）/ C4 の版（M18）/ C6 の旧い形の印（`archive_youtube_shape_the_old_youtube_seal_does_not_hold`）
- **固定されていない**: Q3 の「広告の判定を評価より先に」（R6）

### 手 5: tasks の `[x]` と実体 —— 該当なし（確かめた範囲: `[x]` 11 項目の検証コマンドを HEAD で全部走らせ、CT は `-- --list` で本数を数えた。`cargo test --test` 型は無い。evidence.jsonl に CT の記録が無い項目と、fix wave の前の head の記録しか無い smoke は、上の表のとおり HEAD で走らせ直して一致）

### 手 6: 隙間
- 本物の初日の経路（YouTube と他の製品が同居した、v1 で読み終えた書庫の版の上げ）を複製の試験で通した: 旧い形の印があっても YouTube のファイルは新しい形 `e130f952…` で確認待ち、他のソースの件数は不変、v2 の `read` と `pending_shape` は 1 行ずつ、2 周目も増えない、新しい形に印を置くと watch / search / その他が 1 / 1 / 1 で入り、`read` の行は増えない。**記録が失われる経路は見つからなかった**
- 版の上げの取り直しが走査の輪で回っていることは試験で見ていない（R5）。回らなくても旧い形の印が無ければ何も入らない側に倒れる（`ingest_confirmed_pending` が読む直前にいまの形で照らす。M04 が落ちる）ので、失われるのではなく「一覧が旧い形のまま」になる

## R5. 版の上げで確認待ちの形を取り直すことが、走査の輪の上で試験されていない
- 成果物: crates/server/src/archive/worker.rs:2050（`spawn_inspecting` の `refresh_pending_shapes` の呼び出し）/ crates/server/src/archive_flow_tests.rs:4940・5010・5043（`archive_youtube_reparse_*`）/ spec の Scenario `解析器の版が上がると確認待ちのファイルは新しい形で一覧に出る` ほか 2 本
- 根拠: 複製で `worker.rs:2050` の呼び出しを `Ok(())` に置き換えた変異（M06）で `cargo test -p ashiato-server archive` が 183 passed / 0 failed。3 本の Scenario の WHEN は「解析器の版を上げて**走査が来る**」だが、試験は `refresh_pending_shapes` / `reparse_older_versions` / `ingest_confirmed_pending` を直に呼んでいて、design D9 の 2（取り直しを `ingest_confirmed_pending` より先に回す）の並びもどの試験も見ていない。主張の階層（走査）と試験の階層（関数）が違う
- kind: technical
- 提案: `old_pending` の後に `inbox.spawn(true)` で走査を起こし、`until(pending_hashes == [new_yt_hash])` と v2 の `pending_shape` の行 1 つを待つ試験を 1 本足す（`Inbox` の道具だけで書ける）。旧い形の印を置いた形でも 1 本足せば D9 の 2 の並びも固定できる
- 処置: fixed D9 — `archive_youtube_reparse_the_scan_loop_refreshes_pending_shapes` を足した（`inbox.spawn(true)` で走査を起こし、確認待ちの形が新しいハッシュになるのを待つ → 周を重ねても `pending_shape` の行は 1 つ・何も入らない → 新しい形に印を置くと次の走査で視聴 2 件が入り確認待ちが消える）。**旧い形の印は置かない**: 置くと `ingest_confirmed_pending` の読み直し（`record_pending_file` の `ON CONFLICT` で形を更新する）も形を書き換えるので、印ありの形では M06 が生き残った（実測 rc=0）。印なしにして M06 で rc=101。D9 の 2 の並びは、読む直前にいまの形で照らす（M04 が落ちる）ので順を入れ替えても観測できる違いが無く、試験では固定しない

## R6. 「広告の判定を評価より先に当てる」（deep Q3）を、値を変えても通る
- 成果物: crates/server/src/archive/worker.rs:759（`youtube_route`）/ archive_flow_tests.rs の `archive_youtube_route_*`・`archive_youtube_shape_shows_counts_by_destination`
- 根拠: 複製で評価の判定を `details` より前に置いた変異（M08）で 183 passed / 0 failed。`details` と評価の決まり文句を同時に持つ項目が、どの試験の材料にも無い。行き先はどちらも `c03-myactivity-youtube` なので、違いが出るのは形の一覧の `destinations` の `ad` / `rating` の内訳だけ。本物の書庫では両方を持つ項目は 0 件（python で数えた）なので、いまの実害は無い
- kind: technical
- 提案: `archive_youtube_shape_shows_counts_by_destination` に「`details` を持ち、題名が `高く評価しました ` で始まる」項目を 1 件足し、`ad` に数えて `rating` に数えないことを assert する
- 処置: fixed D3 — `archive_youtube_shape_shows_counts_by_destination` に「`details` を持ち、題名が `高く評価しました ` で始まる」項目を足し、`ad` 2・`rating` 1・`other` 4 を assert。M08（評価を広告より先に）で rc=101

## R7. その他の行き先の payload の欄（`product` / `title` / `url` / `details`）を見る試験が無い
- 成果物: crates/server/src/archive/worker.rs:831-836 / design D3 の表 / tasks 2.2
- 根拠: 複製でその他の payload から `details` を落とした変異（M17）で 183 passed / 0 failed。ST47 の試験は論理ソースごとの件数と `payload->>'title'`（位置の試験）しか見ていない。payload は原文から作り直せる（D3）ので低い
- kind: technical
- 提案: route の試験の 1 本（広告）で、`c03-myactivity-youtube` の行の payload が `product = "YouTube"`・`details` を持つこと、視聴の行が `channel_name` を持つことを assert する
- 処置: fixed D3 — `archive_youtube_route_an_ad_does_not_go_to_watch` に `subtitles` を持つ視聴を 1 件足し、`c03-myactivity-youtube` の payload の `product` / `title` / `url` / `details` と、`c03-youtube-watch` の payload の `title` / `url` / `channel_name` / `channel_url`（`details` を持たない）を assert。M17（YouTube のその他から `details` を落とす）で rc=101

## R8. 「断りは二重」のうち SQL 側（`confirmed_pending_copies`）は、外しても試験が通る
- 成果物: crates/server/src/archive/worker.rs:1129 / design D6 の「断りは二重にする」
- 根拠: 複製で `AND NOT (p.shape->>'kind' = 'YouTube' AND p.shape->'languages' = '[]'::jsonb)` を消した変異（M02）で 183 passed / 0 failed。Rust 側の `is_unconfirmable_shape` を外す（M01）と `archive_youtube_shape_a_mark_on_an_unrecognized_language_does_not_store` が落ちるので、格納は守られている。SQL 側が死ぬと、言語の当たらない形に印を置かれたとき走査のたびにその写しを読み直す（格納はしない）だけ
- kind: technical
- 提案: 同じ試験で `confirmed_pending_copies` が該当の行を返さないことを直に assert するか、SQL 側を「読み直しの手間を省くだけ」と design D6 に書き分ける
- 処置: fixed D6 — `archive_youtube_shape_a_mark_on_an_unrecognized_language_does_not_store` で `confirmed_pending_copies` が空であることを直に assert。M02（SQL の断りを消す）で rc=101

## R9. 7.5 の検証の grep は申し送りの見出し 3 本のうち 1 本しか拾わない
- 成果物: tasks.md 7.5 / docs/handoff/ST47.md
- 根拠: `grep -oE '^## [a-z0-9-]+ R[0-9]+' docs/handoff/ST47.md` の出力は `## st14-outage-alert R17` の 1 行だけ。`## st14-outage-alert D20` と `## 2026-10-13-st33-merge-consistency.md C4` は検査の外（PR 本文に無くても rc=0）。中身は満たされている（D20 は fix dc8ecc1e、C4 は `archive_youtube_location_marks_only_the_located_other_item` と見張り）。7.5 は `[ ]` なので、これから走る検査の外側の話
- kind: technical
- 提案: PR 本文に 3 本とも見出しで書く（tasks は凍結なので検査は直さない）。同じ型の検査を他の Story で書くなら `^## .+ [A-Z][0-9]+` で拾う
- 処置: fixed 7.5 — `docs/briefs/ST47-pr.md` に申し送りの見出し 3 本（`st14-outage-alert D20` / `st14-outage-alert R17` / `2026-10-13-st33-merge-consistency.md C4`）を見出しのとおりに書いた。tasks.md 7.5 の grep は凍結なので直さない

## R10. tasks 8.1 の件数の確かめは、手元の本物の書庫で機械が済ませられる
- 成果物: tasks.md 8.1 / design D4（仮）の反転条件
- 根拠: 手 1 のとおり、本物の書庫をサーバの `shape_for_file` に通すと `languages ["ja"]`・watch 89,368・search 5,643・ad 20,093・rating 196 が出て、8.1 が人間に見てもらう数（約 8.9 万・約 5,600・約 2 万・196 前後）と一致した。8.1 の「物理: realdata —— 合成では再現できない」は合成の材料については正しいが、本物のファイルはこの機械にあり、件数は機械が判定できる（CLAUDE.md の「機械が判定できることは機械に決めさせる」）
- kind: daily
- 提案: 確認バッチの準備（`tools/verify-prep.sh`）で本物の書庫の形の一覧を出して件数を手順書に載せておき、人間には稼働状況の格子の 3 本と「違和感」だけを見てもらう。D4（仮）の日本語の側の反転条件は、この実測で「当たっている」と PR 本文に書ける
- 処置: fixed D4 仮 — design D4 の反転条件に本物の書庫での実測（`languages ["ja"]`・視聴 89,368・検索 5,643・広告 20,093・評価 196）を書き、日本語の側は機械で確かめ済みとした。PR 本文で「8.1 の件数の照合は済み、人間は格子の 3 本と違和感だけ」と挙げる。`tools/verify-prep.sh` に本物の書庫の形の一覧を出させる直しは、Story をまたぐ道具の変更なので ST47 では入れない（tasks は凍結）

## fix wave（53af3084..）の scoped re-review（code-verify の R5〜R10）

席: re-reviewer（SDD の `re-review-prompt.md`）。R5〜R8・R10 ADDRESSED、R9 は PR 本文（`docs/briefs/ST47-pr.md`）に 3 見出しが入ることを条件に ADDRESSED。
fix の diff に新たな Critical / Important は無し（変更は試験・design・記録だけ）。範囲外の観察: `cargo test -p ashiato-server archive` の 1 回目で
`archive_youtube_last_event_is_reported_per_logical_source` と `archive_erased_window_persistent_marking_failure_is_ledgered_and_throttled` が落ち、単独と 2 回目の全体では通った（共有の開発 DB での時間の取り合いの疑い。flaky として記録に留める）。
