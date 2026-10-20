# ST13 spec の独立レビュー（spec stage・`--draft`。深掘り第 1 回は推奨の答えで書いた版）

対象: `openspec/changes/st13-scheduled-account-fetch/`（proposal.md / specs/external-ingestion/spec.md / specs/device-collection/spec.md / design.md / tasks.md）、
`docs/stories/ST13.md`、`docs/stories/INDEX.md`（末尾の 2026-10-21 の訂正と `device-collection` の行）。
突き合わせた正典・コード: `openspec/specs/external-ingestion/spec.md`、`openspec/specs/collection-coverage/spec.md`、`openspec/changes/st09-media-capture/`（と `feat/st09-media-capture` の移行）、
`crates/server/src/{testdb.rs,archive/*.rs,archive_flow_tests.rs,outage_store.rs,lib.rs}`、`collector-android/.../Heartbeat.kt`、`docs/collector-contract.md`、`tools/{smoke.sh,record-env.sh}`、`.harness2/scripts/board.py`。

推奨の答えを採ったこと自体は指摘していない（呼び出し元の指定）。

## 機械の検査（最初に写す）

| コマンド | 結果 |
|---|---|
| `openspec validate st13-scheduled-account-fetch --strict` | `Change 'st13-scheduled-account-fetch' is valid`（rc=0） |
| `python3 scripts/check_chain.py .` | `chain: OK (0 件 / 未回収 0 件 / warn 0 件)`（rc=0。観点 8 の再生成との一致も通っている） |
| `python3 scripts/check_scenarios.py . st13-scheduled-account-fetch` | `scenarios: FAIL (担保なし 49 件 / 名無しの確認待ち 0 件)`（rc=1）。上流なので印が無いのは当然。**49 + `[wait]` 1 本（数か月操作しなくても権限が剥がれない）= 50** で、tasks の Global Constraints の「足す Scenario は 50 本」と一致する。MODIFIED で写した既存の 21 本は既存の印で担保されている（FAIL に出ていない） |
| `python3 scripts/review_triage.py . st13-scheduled-account-fetch` | `triage: FAIL (12 件)`（rc=1）。すべて `review/deep.md` の処置の行の形（→ R1） |

MODIFIED の 3 本（「取り込み待ち置き場に置かれた書庫を読む」「1 件が読めなくても書庫の残りを読む」「置いた書庫ごとに追記のみの台帳が残る」）は、
正典の本文の全行と全 Scenario（11 本 / 4 本 / 6 本 = 21 本）が delta に残っていることを行単位で突き合わせて確かめた（正典にあって delta に無い行: 0 / 0 / 0）。
足した Scenario は 5 + 2 + 2 = 9 本、ADDED の 5 本の Requirement の Scenario は 8 + 4 + 8 + 3 + 11 = 34 本、`device-collection` は 7 本で、計 50 本は正しい。

---

## R1. `review/deep.md` の処置の行が triage の形に合わず、機械が FAIL を返している（12 件）
- 成果物: openspec/changes/st13-scheduled-account-fetch/review/deep.md
- 根拠: `python3 scripts/review_triage.py . st13-scheduled-account-fetch` → `triage: FAIL (12 件)`。内訳は、R1（loss=exported）と R3（loss=uncaptured）を `fixed` で閉じている、R4（kind=daily）を `fixed D<n> 仮` の形で閉じていない、R1/R3/R4/R5/R6/R8/R9 の `fixed` の指す先が `Q<n>` になっている（`D<n>` / `<x.y>` / change 直下のファイル名でない）
- kind: technical
- 提案: deep の段の処置を triage の語彙で書き直す。R1 / R3 は問い（Q1 / Q4）を足して人間に返したので `escalated`、R4 は `fixed D13 仮`、Q を指すものは問いのファイル `deep-questions.json` を指す。spec の成果物の中身には影響しない
- 処置: fixed review/deep.md —— 処置の行を triage の語彙に直した（R1 / R3 は Q1 / Q4 で人間に返したので escalated、R4 は fixed D13 仮、Q を指すものは D か deep-questions.json）

## R2. D2 の trigger は前置きの分からない名前を断るが、試験の登録簿の補助は `t-…` の名前で 150 か所から行を足し、`seed.sh` / `smoke.sh` / `check-immutable.sh` も前置きの無い名前を足している。Task 1.1 は「`testdb::source` は列を指定しないまま」と書き、そのまま実装すると既存の試験が広く落ちる
- 成果物: openspec/changes/st13-scheduled-account-fetch/design.md（D2）/ tasks.md（1.1）/ specs/external-ingestion/spec.md:11, :40-43
- 根拠: design.md:68（`c01-`/`c02-`/`c03-`/`s01-` 以外は「例外で断る」）/ design.md:71 と tasks.md:38（`testdb::source` は 3 列を知らずに INSERT し、D2 の trigger が入れる）/ crates/server/src/testdb.rs:153 と :180（名前は `format!("t-{prefix}-{uuid}")`）/ `grep -rn "testdb::source" crates/server/src | wc -l` → 150 / ほかに直接 INSERT する試験: registry_tests.rs:505, :536（`t-rollup-down-…`）、places_tests.rs:357（`place-lock-elsewhere`）/ **道具の台本も同じ**: tools/seed.sh:32（`seed-location`）、tools/smoke.sh:66（`smoke`）・:652（`smoke-ext`）、tools/check-immutable.sh:40 / :51 / :62 / :165 / :242（`ext-kind-check` `rebuild-check` `immutable-check` `hb-check` `gate-check`）。Task 1.1 の検証の `tools/check-immutable.sh` rc=0 自体が、この trigger で落ちる（`web/e2e` に INSERT は無い）
- kind: technical
- 提案: Task 1.1 に「`testdb::source*`・直接 INSERT する試験・`tools/seed.sh` `smoke.sh` `check-immutable.sh` は `acquisition` と `fetch_trigger` を指定する（共有の台本はその場の書き換え）」を足すか、D2 の trigger に前置きの無い名前の扱いを書き直す（spec:11 と :40-43 の「断る」もそれに合わせる）。どちらにしても、検証に「既存の全試験と縦串が通る」（`cargo test -p ashiato-server` の全体・`tools/smoke.sh`・`tools/seed.sh`）を足す
- 処置: fixed D2 —— 前置きの分からない名前は断らず、サーバ・他の記録から導くを入れる（spec の Scenario を「サーバの取り方で入る」に直した）。Task 1.1 の検証に cargo test の全体と smoke を足した

## R3. ST09 が先に merge すると、D3 の `UPDATE … WHERE acquisition IS NULL` が ST09 の 3 行を埋めず、`SET NOT NULL` で ST13 の移行が落ちる
- 成果物: openspec/changes/st13-scheduled-account-fetch/design.md（D2 / D3 / Migration Plan）/ tasks.md（1.1）
- 根拠: `git show feat/st09-media-capture:migrations/202610181200_media_capture.sql` の 9-16 行目で、`c01-screenshot` / `c01-image` / `c01-video` を 3 列を知らずに INSERT している / design.md:71-72 は「ST09 が後から merge したとき」だけを考えている（trigger は INSERT にしか効かない）/ design.md:78-90 の表に ST09 の 3 行は無い / tasks.md:36 は「D3 の既存の行の値」を入れて「最後に 2 列を NOT NULL にする」
- kind: conflict
- 提案: D3 に「表に無い既存の行は、D2 と同じ前置きの規則で埋める（`UPDATE` も同じ関数を使う）」を足し、どちらの merge 順でも通ることを Task 1.1 の検証にする（ST09 の 3 行を先に INSERT した DB に当てる試験）。Scenario「取り方の無いソースは登録簿に残らない」がこれを担保するので、試験の WHEN に「列を知らない移行が先に足した行がある」を含める
- 処置: fixed D2 仮 —— 前置きの規則を関数にし、移行の UPDATE も表に無い行をこの関数で埋める。Scenario「取り方を足す前に別の移行が足した行にも取り方が入る」と Task 1.1 の試験を足した

## R4. 起動時に専用のフォルダを作ると、MODIFIED していない正典の Scenario「置き場が読めない日は取れない状態で残る」（専用のフォルダが存在しない状態で 1 日動く → 取れない）と食い違う。既存の試験も落ちる
- 成果物: openspec/changes/st13-scheduled-account-fetch/specs/external-ingestion/spec.md:258, :332-335
- 根拠: openspec/specs/external-ingestion/spec.md:682-685（WHEN「専用のフォルダが存在しない状態で取り込み器が 1 日動く」/ THEN「取得できない状態で専用のフォルダが満たされていない」）。この Requirement「取り込み器は自分の論理ソースに 1 日 1 回の生存信号を送る」は delta の MODIFIED に無い / delta:258 と :332-335 は「起動時に専用のフォルダが無ければ作る」/ crates/server/src/archive_flow_tests.rs:1431-1437（`remove_dir_all(&inbox.inbox)` してから `spawn` し、`capturable = false` を待つ。D8 の `create_dir_all` が入ると取れる状態になり、この試験は待ち続けて落ちる）
- kind: conflict
- 提案: 生存信号の Requirement も MODIFIED に入れ、（delta:327-330「片方の置き場が読めない走査は成功に数えない」も生存信号の回数の主張なので、その Requirement の下へ移す）その Scenario の WHEN を「専用のフォルダが読めない状態（起動後に消された・権限が無い・作れない）」に書き換える。tasks に `archive_flow_names_only_the_unreadable_inbox` の直し（起動の後で消す・権限を落とす）を足す。あわせて delta:322 の Scenario の題「専用のフォルダが**無くても**…」は WHEN（:324「読めない状態」）と合わせる（起動時に作るので「無い」は起動の後にしか起きない）
- 処置: fixed D17 仮 —— 起動時に専用のフォルダを作ることを D17（仮）にし、生存信号の Requirement を MODIFIED に入れ、「置き場が読めない日は取れない状態で残る」の WHEN を起動の後に消される形にし、「片方の置き場が読めない走査は成功に数えない」をそこへ移した。題を「読めなくても」に直し、Task 5.1 に既存の試験の直しを足した

## R5. D11 の書き方の規則と spec / tasks が食い違う（60 秒は D11 では「1 分ごと」になる）
- 成果物: openspec/changes/st13-scheduled-account-fetch/design.md:173 / specs/external-ingestion/spec.md:197 / tasks.md:104
- 根拠: design.md:173「3600 で割り切れれば N 時間、60 で割り切れれば N 分、それ以外は N 秒」→ 60 は「1 分ごと」/ spec:197 THEN「携帯端末の位置の行に『60 秒ごと』」/ tasks.md:104「60 → 『60 秒ごと』」/ proto.html:82 も「60 秒ごと」
- kind: technical
- 提案: D11 の規則を spec と proto に合わせて直す（例: 120 秒未満は秒で書く）。規則が観測できる文字を決めるので、境目の値（60 / 120 / 3600 / 86400）を Scenario か vitest の表に置く
- 処置: fixed D11 —— 120 秒未満は秒で書く規則にし、境目の値を Task 9.1 の vitest の表に置いた

## R6. Scenario「想定間隔より長い取得の間隔は断られる」は 300 秒の上限でも断られるので、想定間隔の制約が無くても緑になる
- 成果物: openspec/changes/st13-scheduled-account-fetch/specs/external-ingestion/spec.md:30-33
- 根拠: spec:32 WHEN「想定間隔が 1 日の `s01-archive-inbox` の取得の間隔を 2 日に変えようとする」/ design.md:58 の CHECK `logical_source <> 's01-archive-inbox' OR fetch_interval_sec <= 300` が 2 日（172800）を先に断る / Q1 の context の「聞かずに決める既定」（想定間隔より長い登録は断る）を担保する Scenario はこれ 1 本
- kind: technical
- 提案: 300 秒の上限が効かないソースで書く。例: 想定間隔 21600 秒の `c01-location` の取得の間隔を 21601 秒に変えようとする → 断られる。境目（21600 は通る）も 1 本あると、`<` と `<=` の取り違えも落ちる
- 処置: fixed specs/external-ingestion/spec.md —— c01-location の 21601 秒（断る）と 21600 秒（受け付ける）の 2 本にした

## R7. 「通知までの残り日数」の数え方が、`collection-coverage` の通知の判定（経過時間が 75 日を超える）と境目で一致する保証が無い
- 成果物: openspec/changes/st13-scheduled-account-fetch/specs/external-ingestion/spec.md:176-177, :199-207 / design.md:161
- 根拠: openspec/specs/collection-coverage/spec.md:1305（「いまの時刻から最後の書庫の作られた時刻を引いた経過が、通知までの日数を**超える**」= 時刻の経過で判定）と :1323-1326（75 日と 1 時間前 → 途切れ）/ design.md:161 は「同じ関数で出す…日は `Asia/Tokyo`」で、日の数え方（切り捨て・暦日の差）を決めていない / spec:177 は「残り日数が 0 以下」で「過ぎた」を出す。暦日で 75 日目の朝は、通知はまだ鳴らないのに「過ぎた」が出うる / Scenario は 53 日・80 日だけで、境目に当たらない
- kind: technical
- 提案: 残り日数の定義を Requirement に書く（例: 「通知が鳴る時刻（最後の書庫 + 通知までの日数）までの残りを日に切り上げ、鳴った後は『過ぎた』」）。`collection-coverage` の境目の Scenario と同じ 2 点（75 日と 1 時間前 → 過ぎた / 74 日と 23 時間前 → 過ぎていない）を Scenario にする。Task 8.1 の「途切れの判定と同じ日を返す」1 本を、この境目で比べる形にする
- 処置: fixed D10 —— 残り日数を「通知が鳴る時刻までの残りを日に切り上げ」と Requirement に書き、境目の 2 本（74 日 23 時間前 / 75 日 1 時間前）を Scenario と Task 8.1 の比べる試験にした

## R8. 書庫のソースで「運んだ書庫が 1 冊も無い」行に何を出すかを spec が言っていない（本人が使わない製品・移行前の 3 本は退役前）
- 成果物: openspec/changes/st13-scheduled-account-fetch/specs/external-ingestion/spec.md:176-178, :209-217
- 根拠: spec:176 は書庫の行に必ず「通知までの残り日数と最後の書庫の日」を出すと言うが、静的な `c03-*` 11 本の多くは書庫が 1 冊も来ない / 退役は `worker.rs:971-983`（`retire_legacy_sources`）が移行前の記録を読んだときに付けるだけで、読む前の `c03-legacy-*` は退役していない / `collection-coverage` は記録も書庫も無いときの代わりの値を持たない（spec:1304 の代わりは「記録があるのに書庫が無い」ときだけ）/ proto.html:122-123 は `c03-legacy` を名前で「増えない」にし、ほかは全部「あと 22 日」と描いている
- kind: daily
- 提案: Requirement に「運んだ書庫も記録も無い書庫の行は、残り日数を出さず『まだ書庫が無い』と出す」の類を足し、Scenario を 1 本置く（Task 9.2 の偽データの「記録の無いソース」に、書庫の行の残り日数の欄の期待値を足す）
- 処置: fixed D10 仮 —— 書庫も記録も無い書庫の行は「まだ書庫が無い」を出す 1 文と Scenario を足した

## R9. `tools/record-env.sh` も `ASHIATO_ARCHIVE_SCAN_SEC=1` を設定している。D4 の起動を断る変更で録画の環境の S-01 が起動しなくなるが、tasks は `smoke.sh` の 1 行しか直さない
- 成果物: openspec/changes/st13-scheduled-account-fetch/tasks.md（2.1）/ design.md（D4・Risks）
- 根拠: tools/record-env.sh:143（`export ... ASHIATO_ARCHIVE_SCAN_SEC=1`）/ tools/smoke.sh:21 / tasks.md:48 と design.md:212 は `smoke.sh` と `docs/archive-inbox.md` だけを挙げる / `grep -rn ASHIATO_ARCHIVE_SCAN_SEC` で他に当たるのは config.rs だけ
- kind: technical
- 提案: Task 2.1 に `tools/record-env.sh` の書き換えを足し、検証に `grep -rn ASHIATO_ARCHIVE_SCAN_SEC tools/ docs/ crates/` が config.rs の断る文言以外 0 件、を置く
- 処置: fixed 2.1 —— tools/record-env.sh の書き換えと、環境変数の名前が config.rs の外に 0 件の grep を足した

## R10. 縦串の「起動後に登録簿を 1 秒にする」は、D4 の「sleep の直前に読む」と競合し、最初の 120 秒は効かないことがある。試験も共有の 1 行を書き換える
- 成果物: openspec/changes/st13-scheduled-account-fetch/design.md:96-98 / tasks.md:48
- 根拠: design.md:96（1 周 1 回、`sleep` の直前に読む）。起動 → 最初の走査 → 読む（UPDATE 前なら 120）→ 120 秒寝る、の順になると、UPDATE は 2 周目から効く。新しい DB では列は S-01 の起動時の移行で足されるので、UPDATE を起動前に置けない / 試験: `core.source` は利用者で分かれていない共有の行（docs/handoff/ST13.md の R35）。outage_registry_tests.rs:131 は「他の試験と共有する本物の行は書き換えない」をトランザクションで守っているが、取り込み器は自分の pool で読むので同じ手が使えない。並列の試験で 1 秒にする補助（tasks.md:48）と「初期値は 120 秒」の試験（spec:72-75）がぶつかる
- kind: technical
- 提案: D4 に、読んだ値を試験と縦串が差し替えられる口を書く（例: 取り込み器が読む論理ソースの名前か読み口を引数で渡し、試験は自分専用の行を使う。縦串は列が足されるのを待って UPDATE し、走査が 1 秒間隔になるまで待つ）。Task 2.1 の検証に「縦串の見回りが 1 秒になるまでの待ち」を入れる
- 処置: fixed D4 —— 待ちを 1 秒ずつ刻んで登録簿を読み、縮めると 1 秒以内に効く形にした。試験は自分専用の行を足して取り込み器に渡し、共有の行を書き換えない（Global Constraints にも書いた）

## R11. 「定期取得の枠は壁時計が戻る・**進む**ことで回数を変えない」の「進む」に Scenario が無い。「戻っても」の試験の (b) は枠の外で時計を読むので、枠が壁時計を読んでも落ちない
- 成果物: openspec/changes/st13-scheduled-account-fetch/specs/external-ingestion/spec.md:96, :117-120 / design.md:118
- 根拠: spec:96（戻る・進む）に対し、Scenario は :117「戻っても」だけ / Q1 の context「時計が戻った・進んだ・止まっていた間に契機を過ぎた、のどれでも起動の直後に 1 回で追いつく」/ design.md:118 (b)「壁時計を読む取る手が 1 時間戻った値を見ても回数が変わらない」は、枠が `tokio::time` だけで回っている試験では常に真で、検証しているのは (a) の grep だけ
- kind: technical
- 提案: 「壁時計が進んでも取りに行く回数は変わらない」を 1 本足す（担保は同じ grep の台本でよい）。design.md:118 の (b) は「担保は (a)」と明記して外すか、枠が時計を差し替えられる形にして本当に壁時計を動かす
- 処置: fixed D5 —— 「壁時計が進んでも」の Scenario を足し、担保は壁時計を読まない grep の台本と明記した（(b) を外した）

## R12. Requirement「Google 系の手作業は…」の本文「S-01 を起動し直す・設定を変えるといった操作を求めない」に Scenario が無い。足した 1 本は正典の Scenario とほぼ同じ
- 成果物: openspec/changes/st13-scheduled-account-fetch/specs/external-ingestion/spec.md:150, :157-160
- 根拠: spec:150 の「起動し直す」「設定を変える」を言う Scenario は無い（フォルダを作らないことだけ :167-170 にある）/ spec:157-160「Takeout をダウンロードのフォルダに落とすだけで入る」は、正典の openspec/specs/external-ingestion/spec.md:30-33「ダウンロードのフォルダの Takeout の書庫が読まれる」と WHEN / THEN がほぼ同じで、新しい主張を持たない
- kind: technical
- 提案: :157 の Scenario の WHEN に「専用のフォルダが無く、S-01 を前回の書庫の後に起動し直していない・環境変数も登録簿も触らない状態で」を入れ、2 か月前の書庫を読ませた同じプロセスのまま 2 冊目が入ることを言う（「起動し直さない」が観測できる）
- 処置: fixed specs/external-ingestion/spec.md —— Scenario の WHEN を「1 冊目を読ませた同じ S-01 のまま（起動し直さず、環境変数も登録簿も触らずに）2 冊目」にした

## R13. 休止の除外の 4 つの状態のうち「読めない」で何をするか（頼むか・画面に出すか・S-7 に出すか）が spec にも design にも無い
- 成果物: openspec/changes/st13-scheduled-account-fetch/specs/device-collection/spec.md:5-9 / external-ingestion/spec.md:180
- 根拠: device-collection:9 は 4 状態（外れている / 外れていない / 仕組みを持たない / 読めない）を原文に載せると言うが、:5-8 の頼む・出すは「外れていない」「仕組みを持たない」だけを扱う / design.md:179 は `ERROR` と例外を `unknown` に写すだけ / external-ingestion:180 の S-7 は「外れていない」のときだけ
- kind: technical
- 提案: 「読めない」のときの振る舞いを Requirement に 1 文と Scenario 1 本で決める（例: 頼まず、収集アプリの画面に読めなかったことを出す / S-7 には出さない）。Task 10.1 の「4 つの状態ごとに」の試験の期待値がこれで決まる
- 処置: fixed D12 仮 —— 読めないときは頼まず、収集アプリの画面に読めなかったことを出す（S-7 には出さない）。Requirement に 1 文と Scenario を足した

## R14. 本人の端末は更新で入るのに、案内の Scenario は「初めて開く」としか言わない。更新後の初回で案内が出ることが担保されない
- 成果物: openspec/changes/st13-scheduled-account-fetch/specs/device-collection/spec.md:17-20, :27-30
- 根拠: device-collection:19 WHEN「収集アプリを初めて開く」/ design.md:219 Migration Plan「携帯の収集アプリは更新して 1 回開く（除外の案内が出る）」/ design.md:181 の印は `SharedPreferences` の 1 つで、更新の前から開いていた端末の「初めて」は印の有無で決まる
- kind: technical
- 分類の直し: 呼び出し元が premise から technical に直した（外の形式の前提ではなく、Scenario の WHEN が更新の場合を言っていない抜け。人間に問うことは無い）
- 提案: Scenario の WHEN を「自動で頼んだ印の無い状態で（新しく入れた・前の版から更新した）収集アプリを開く」にするか、更新の場合を 1 本足す。Robolectric の試験で印の無い状態を作れば足りる
- 処置: fixed D12 仮 —— 前の版から更新した端末も印を持たないので更新後の最初の 1 回で出る、を Requirement と Scenario の WHEN に書いた

## R15. INDEX の「代償: ST09 が走っている間、ST13 の下流は `衝突待ち`」は機械の判定と合わない。D12 の「ST09 の gate 済みの SHA に継ぎ足す」は「取り込まない」と同じ段落で矛盾する
- 成果物: docs/stories/INDEX.md（2026-10-21 の訂正）/ openspec/changes/st13-scheduled-account-fetch/design.md:183 / tasks.md:32 / proposal.md:78
- 根拠: .harness2/scripts/board.py:290-301（`衝突待ち` は状態が `着手可` の Story にしか付かない。ST13 は既に `上流` で、下流へ進むときに capability の重なりを見る箇所は無い）/ board.py:21-22（gate 済みの SHA の spec を読むのは requires の依存だけ）/ design.md:183「下流は ST09 の gate 済みの SHA の `MainActivity` に継ぎ足す（`story_deps.py` は ST09 を requires に持たないので取り込まない。衝突は rebase で解く）」—— 取り込まないなら ST13 の作業木の `MainActivity.kt` / `Heartbeat.kt` は main の形で、ST09 の形に継ぎ足せない / ST09 は両方のファイルを触る（Heartbeat の原文に接続の欄、MainActivity に写真の権限）
- kind: conflict
- 提案: どちらかに揃える。(a) ST13 の `requires` に ST09 を足す（story_deps が ST09 の gate 済みの SHA を取り込み、継ぎ足しが本当になる）か、(b) INDEX と proposal の「衝突待ち」の主張を取り下げ、D12 / Global Constraints を「main の形で書き、ST09 が先に入ったら rebase で `Heartbeat.kt` の `fields` と `MainActivity` の初回の流れを解く」に直す
- 処置: fixed D12 仮 —— (b) を採った。main の形で書き、ST09 が先に入ったら rebase で解く。INDEX と proposal の「衝突待ち」の主張を取り下げた

## R16. 生存信号の原文に除外の状態を足すのに、契約の正典 `docs/collector-contract.md` は「原文に載るのは稼働・取得可否・回数だけ」と書いたまま。tasks に契約の直しが無い
- 成果物: openspec/changes/st13-scheduled-account-fetch/tasks.md（10.1 / 8.1）/ design.md（D10・D12）
- 根拠: docs/collector-contract.md:528-529（「生存信号の原文に載るのは稼働・取得可否・回数だけ」）/ design.md:162（S-01 が `c01-location` の生存信号の `raw` の `unused_app_restrictions` を読む。原文をサーバが解く初めての経路）/ design.md:182 / tasks.md の触るファイルの一覧（:6-7）に `docs/collector-contract.md` が無い
- kind: technical
- 提案: Task 10.1 か 8.1 に、契約の「送る形」と「守ること」に `unused_app_restrictions`（4 つの値・`c01-location` だけ・S-01 が S-7 のために読む）を足すことを入れる。検証は `grep -n unused_app_restrictions docs/collector-contract.md` が 1 件以上
- 処置: fixed 10.1 —— docs/collector-contract.md に unused_app_restrictions を足すことと、その grep を検証に足した（design D12 にも書いた）

## R17. design D10 にだけある観測できる振る舞い: 「いちばん新しい記録」から削除済みを除く・取り方の中の並びは表示名の順・ソースの節は書き出しの節の前
- 成果物: openspec/changes/st13-scheduled-account-fetch/design.md:160, :163, :166
- 根拠: design.md:160（`latest_at` は「削除済みを含めない」）。正典の最終日は削除された記録の出来事も含める（openspec/specs/external-ingestion/spec.md:712）ので、同じ一覧の書庫の行と書庫以外の行で削除の扱いが逆になる / design.md:163（取り方の順 → `display_name`）と :166（書き出しの節の前）は画面で観測できる / spec:175 は「いちばん新しい記録の時刻」とだけ書く
- kind: technical
- 提案: spec:175 に「削除した記録を数えない」を書き、Scenario 1 本（最新の記録を削除するとその前の記録の時刻が出る）を置く。並びは :174 に「同じ取り方の中は表示名の順」を足し、e2e の「見出しの順と行の数」のアサートに行の順を足す
- 処置: fixed D10 —— いちばん新しい記録は削除した記録も数える（書庫の最終日と揃える）と決め、spec に書いて Scenario を足した。並び（見出しの順）と節の位置（書き出しの前）も spec と e2e のアサートに足した

## R18. M8 の直しは「画面を誤らせる」の直しなのに、Scenario は台帳の種別だけを見る。画面の文言は design にだけある
- 成果物: openspec/changes/st13-scheduled-account-fetch/specs/external-ingestion/spec.md:342-345 / design.md:141
- 根拠: spec:264（「I/O の失敗を『形が対応していない』と呼ぶと、本人は…読み違える」）/ spec:345 THEN は台帳の理由の種別だけ / design.md:141 の `messages.ts` の文言（「ファイルを読み出せませんでした。置き直すか、ファイルの権限を確かめてください」）は spec に無い / 画面の Scenario は `web/e2e` で担保する規約（AGENTS.md）
- kind: technical
- 提案: 「読み出せないファイルは稼働状況の『直近に置いた書庫』に形の問題と別の文字で出る」を 1 本足し、Task 5.1 に e2e（偽データに `file_unreadable` の台帳の行）を足す。種別だけで足りる判断なら、spec:264 の「読み違える」の根拠を台帳の読み手（CLI）に限ると書く
- 処置: fixed 5.2 —— 稼働状況の「直近に置いた書庫」に形の問題と別の文字で出る 1 文と Scenario を spec に足し、e2e を Task 5.2 に置いた

## R19. Task 1.2 の `CT source_acquisition_matches_collector` は `-p ashiato-server` で走るが、サーバの crate は `collector-windows` に依存していないので Rust の定数を比べられない
- 成果物: openspec/changes/st13-scheduled-account-fetch/tasks.md:42-43
- 根拠: tasks.md:17 の `CT` は `cargo test -p ashiato-server` / crates/server/Cargo.toml に `collector` を含む依存は無い（`grep -n collector crates/server/Cargo.toml` → 0 件）/ 比べる定数は crates/collector-windows/src/history/fetch.rs:9 `HISTORY_INTERVAL = 24h`
- kind: technical
- 提案: 比べる試験の置き場を決める。`collector-windows` 側に移行の SQL を読む試験を置き `cargo test -p ashiato-collector-windows <名前> -- --list` で 1 本以上を確かめるか、`tools/check-source-intervals.sh` に Rust の定数の grep も入れて `CT` を外す
- 処置: fixed 1.2 —— Rust の定数も tools/check-source-intervals.sh の grep で比べ、CT を外した

## R20. 「外れていなくても取得できる状態は変わらない」の WHEN は位置の権限しか固定しないので、接続やセンサで満たされていないものが出ると、除外と関係なく偽になる
- 成果物: openspec/changes/st13-scheduled-account-fetch/specs/device-collection/spec.md:42-45
- 根拠: device-collection:44 WHEN「位置の権限があり、…外れていない端末で」/ :45 THEN「取得できる状態を示し、満たされていないものは空」/ 位置の取得できる状態は権限・センサ・接続で決まる（openspec/changes/st09-media-capture/specs/device-collection/spec.md:781、docs/collector-contract.md の `blockers` の表 `permission` / `sensor` / `network`）/ Requirement の主張は device-collection:10「除外の状態によって…変えない」で、比べる形
- kind: technical
- 提案: 比べる形に書き直す。WHEN「同じ端末の状態で、除外の状態だけを『外れている』と『外れていない』にして生存信号を送る」/ THEN「2 つの生存信号の取得できる状態と満たされていないものは同じ」。Task 10.1 の試験もこの 2 本を比べる

---

## 観点ごとの確かめ

- **観点 1（deep の決定が正典に写っているか）**: 本人の答えはまだ無い（全問 `答え待ち・推奨`）。推奨の答えは Q1 → ADDED の Requirement 3 本（登録簿の取り方・見回りの間隔・定期取得の枠）、Q2 → MODIFIED 3 本（R25・R34・M7・M8）、Q3 → S-7 の Requirement、Q4 → `device-collection`、Q5 / Q6 → design D13 / D14 の（仮）と、spec の「変える操作を置かない」・「Google 系の手作業」に写っている。
  Q1 の「聞かずに決める既定」は、別の列（spec:8、D1）・長い間隔を断る（spec:9、Scenario は R6 の欠け）・前回の時刻を持たない / 起動の直後に 1 回 / 戻らない時計（spec:94-97、D5。「進む」は R11 の欠け）として写っている。
  数値（120 秒・300 秒・10 分・2 か月・75 日・60 秒）はどれも Scenario にある。要件へ戻すもの（FR-15 の ★）は design.md:200 で record の段に送っており、この版では `docs/requirements.md` に入っていない（答えの前なので順序は正しい）。
- **観点 2（検証可能か）**: R6・R7・R11・R12・R13・R14・R20。そのほかの Scenario の THEN は数・文字・状態で決まる。主張を 2 つ束ねた THEN（:135、:140 ほか）は Global Constraints（tasks.md:15）が 1 つずつ assert すると定めている。
- **観点 3（置き場）**: R5・R17・R18。spec に関数名・crate 名・列名は入っていない（論理ソースの名前と環境変数の名前は本人が触る設定なので可）。proposal の Capabilities（`external-ingestion` / `device-collection`）と `specs/` のディレクトリは一致する。INDEX の capability 表にも ST13 は両方にある（`device-collection` の追加は 2026-10-21 の訂正に理由がある。太字は他の訂正での追加と同じ使い方）。
- **観点 4（tasks の検証）**: R2・R3・R9・R10・R19。各タスクにコマンドと終了条件（`CT … (≥N)` / `GT … (≥N)` / e2e の本数 / 台本の rc）がある。参照した `tools/check-migrations.sh` `check-immutable.sh` `check-shared-appends.sh` `check-openapi.sh` `archive-shape.sh`、`web` の `test:e2e`、`tools/smoke.d` `tools/seed.d` は実在する。新しく作る台本（`check-source-intervals.sh` `check-scheduled-clock.sh`）は「わざと壊した木で rc≠0」の確かめがある。依存の順（移行 → 見回り → 枠 → 台帳 → 置き場 → 回数 → 縦串 → API → 画面 → 端末）に逆転は無い。人間の確認待ちは 1 本で、物理（time）と理由がある。
- **観点 5（Story と要件）**: 該当なし（確かめた範囲: `check_chain.py` の観点 8 は OK。ST13.md の「完了の判定」1「登録簿で設定でき、その間隔で自動取得が走る」は、見回りの間隔の Requirement と定期取得の枠の Scenario（:77-80、:112-115）に、2「Google 系は 2 か月に 1 回の手作業以外に操作が要らない」は Requirement「Google 系の手作業は…」と Q5 の推奨に対応する。「壊してはいけないもの: ST12 の取り込み経路と最終日の表示」は MODIFIED が正典の全 Scenario を残しているので固定されている。ただし既存の生存信号の Scenario との衝突は R4。specs が `satisfies`（FR-15 / NFR-3 / NFR-12）の外を満たそうとしている箇所は、`device-collection` の Q4 が NFR-12 の後半の持ち主として ST13 だけ（INDEX:213）なので外ではない）。
- **既存の振る舞い（呼び出し元の指定）**: 取り込み器の生存信号の成功回数の定義は変えていない（delta:265、:327-330 は正典 :672-675 と同じ数え方）。10 分以内に読み始めるは、2 回の走査 × 300 秒 = 600 秒で上限と合う（D4）。ST14 の通知の日数は同じ列（`alert_after_days`）を使うが、日の数え方の境目は R7。
- **観点 6**: 該当なし（`spec --answers` ではない）。
- 処置: fixed specs/device-collection/spec.md —— 同じ端末の状態で除外の状態だけを変えた 2 つの生存信号を比べる形にした

