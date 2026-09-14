# ST16 下流実装の独立検証（code-verify）

対象: worktree `/home/yosis/dev/ashiato2-st16`、ブランチ `feat/st16-stay-derivation`、HEAD `2bfc3f3`（`git diff origin/main` の 1 commit）。
**コードも成果物も触っていない。** ガードを壊した確認はファイルを `/tmp` に退避して戻し、終了時に `git status --short` が空であることを確かめた。
指摘の番号は `review/spec.md`（R11〜R26）の続き（R27〜）。

DB は `st16-testdb`（127.0.0.1:55416）。途中で**別のセッションが同じ `ashiato` DB に対して `cargo test` を走らせ始め**（cwd `/tmp/st16-mut`）、
移行の `ALTER TABLE` と挿入が deadlock（`40P01`）して DB テストが一斉に落ちたので、変異の確認は同じコンテナに作った使い捨ての DB（`st16verify`。最後に DROP した）で行った。
smoke / check-immutable / seed は `COMPOSE_FILE=/tmp/st16-compose/docker-compose.yml COMPOSE_PROJECT_NAME=st16smoke`（55417 / BIND 127.0.0.1:18716）。

## 申告と実測

申告: tasks 30 件中 28 件 `[x]`（未了は 8.4 / 8.5）。

| 検証コマンド（申告） | 実測 | 一致 |
|---|---|---|
| `cargo test --workspace` | rc=0。collector 86 passed / server 200 passed | 一致 |
| `cargo clippy --workspace --all-targets -- -D warnings` | rc=0 | 一致 |
| `cargo fmt --check` | rc=0 | 一致 |
| `cd web && npm run test` / `lint` / `build` | rc=0（Test Files 14 / Tests 62 passed）/ rc=0 / rc=0 | 一致 |
| `python3 scripts/check_scenarios.py . st16-stay-derivation` | rc=0（`Scenario 204 件 / 担保あり 203 / 人間の確認待ち 1`。待ちの 1 件は collection-coverage）。独立に数え直すと、この change の `#### Scenario:` は 65 本、印の無いものは 0 本 | 一致 |
| `python3 scripts/check_chain.py .` / `openspec validate st16-stay-derivation --strict` | rc=0 / rc=0 | 一致 |
| `tools/check-migrations.sh` / `check-openapi.sh` / `check-boundaries.sh` | rc=0 / rc=0 / rc=0 | 一致 |
| `tools/check-immutable.sh`（55417） | rc=0（`書き換え禁止 OK`） | 一致（ただし R39） |
| `tools/smoke.sh`（55417 / 18716） | rc=0。段 41 で `["no-record","stay","no-record"]`、滞在 1 件 | 一致 |
| tasks の `CT <絞り込み>` 19 本（1.1〜5.2） | すべて rc=0 かつ `passed` ≥ 1。件数: migration 1 / ledgers 1 / source 1 / `stay::detect` 7 / `stay::gap` 3 / `stay::accuracy` 3 / raw 1 / row_shape 3 / identity 7 / idempotent 1 / erased_range 6 / keeps_locations 1 / serialized 1 / index 1 / auto_rebuild 4 / rebuild_api 5 / `derived_rebuild_is_not_folded` 1 / day_api 10 / walked_day 1 | 一致 |
| 7.1 `tools/seed.sh normal` → `GET /stays?date=2026-09-07`（まっさらな DB） | 滞在 9 / 移動 9 / 記録なし 2（23:00–23:31 と 23:59–24:00）。位置の行は **1,410 件**（tasks の本文は「1,440 件」だが 30 分の欠けを入れる指示と同時には成り立たず、実装は欠けの側を採っている） | 一致（件数） |
| 7.1 `tools/seed.sh max`（まっさらな DB） | 滞在 15。続けて `POST /stays/rebuild` しても `stays_before 15 / stays_after 15` | 一致（ただし R36） |
| 申告のガード破壊 5 件 | 再現した: 錠を外す → `stay_rebuild_is_serialized` だけ落ちる / 前日の位置を読まない → `stays_day_api_continuous_from_previous_day` だけ落ちる / NULL を作り直しの印と読む → `stay_erased_range_null_deleted_by` だけ落ちる。距離（係数 3.0 → `far_does_not_inherit`、1.5 → `near_inherits`）も落ちる | 一致 |

## 手 1: 固定値の独立再計算 —— 問題なし

- `stay::tests::stay_raw_is_pinned` の期待文字列を python の `json.dumps(separators=(",",":"))` と `round(x*1e6)/1e6` で組み直して一致（`True`）。
  35.68123449 → 35.681234、139.76712551 → 139.767126、`.5Z` → `.500Z`
- seed の件数: `max` を入れた DB から `c01-location` 1,410 行を `COPY` で抜き、**haversine 距離で書いた別実装の D7** で判定し直すと 15 件で、`GET /stays` の 15 件と始まり・終わりまで一致（`15 True`）
- day_view の境目: 日の頭（`d0 - gap` を仮の点にする）・尻（`d1 + gap`）・今日（`now`）を、真の間隔との大小で場合分けして確かめた。
  頭と尻は「計算した間隔 ≥ gap なら真の間隔も ≥ gap」「計算した間隔 < gap のときは行が日の外に切り落とされる」で過不足が無い。seed normal の尻 23:59–24:00 の 1 分の記録なしも規則どおり

## 手 3 / 手 4: 本人の決定と test —— 固定されているもの

変異を 1 つずつ入れて `cargo test -p ashiato-server stay`（56 本）を走らせた結果。**落ちたので問題なし**:

| 決定 | 入れた変異 | 落ちたテスト |
|---|---|---|
| Q10（1 件だけが継ぐ） | `!taken[j]` を外す | `stay_identity_split_keeps_largest` ほか 4 本 |
| Q11（半径の 2 倍） | 係数 3.0 / 1.5 | `stay_identity_far_does_not_inherit` / `stay_identity_near_inherits`（新しい基準の半径を使うことも 230 m / 120 m で固定されている） |
| Q2（本人が消した行を変えない） | 割り当ての候補から `UserDeleted` を外す処理を消す | `stay_erased_range_row_untouched` ほか 3 本 |
| Q12（丸ごと隠す）/ D4 の印を外す | 印を外さない | `stay_erased_range_merge_hides_then_restores` ほか 2 本 |
| Q3（感度 1） | 挿入に `sensitivity = 2` を足す | `stay_row_shape_is_derived_with_criteria` |
| Q7（記録なしで切る） | 既定の間隔 10 → 15 | `stay::gap::gap_is_inclusive` ほか 5 本 |
| Q8（精度が半径より悪い点を使わない） | `acc > radius * 2` | `stay::accuracy::one_inaccurate_point_does_not_split` |
| Q9（上限を置かない） | 12 時間の上限を足す | `stay::detect::long_stay_is_not_split` |
| 既定 100 m / 10 分 | 120 m / 15 分 | `stay_raw_is_pinned` ほか 5 本 / 3 本 |
| 前の版を積む | `push_version` を消す | `stay_identity_extends` |
| web D11（ライトの文字） | `muted: 30 → 55` | `day-view-limits` の 4.5:1 |

design D5 / D8 / D10 / D11 / D12 の「下流 2026-09-14」の追記は、書いてある形と実装が一致していた（D5 の読む幅と 64 回・吸収では広げない、D8 の移動の範囲・正の長さ・`M/D`・`24:00`・新しい版から、D10 の自前の錠の関数と FK 3 本、D11 の 96/91/86・14/30、D12 の 2 通りの重なり）。
**ただし追記した規則のうち半分はテストで固定されていない**（R30〜R33）。

---

## R27. 「重なりが同じなら始まりの早い既存の滞在から割り当てる」のテストは、始まりで同点を解く処理を消しても 10 回中 4 回通る

- 成果物: crates/server/src/stay_tests.rs:600-636（`stay_identity_tie_goes_to_earlier`）/ crates/server/src/stay_store.rs:423
- 根拠: `stay_store.rs:423` の `.then(a.2.cmp(&b.2))` を消した複製で、`cargo test -q -p ashiato-server stay_identity_tie_goes_to_earlier` を 10 回 → `pass=4 fail=6`。戻すと 5/5 pass。
  同点の次の鍵は `e.id`（`Uuid::new_v4()`）なので、判定が識別子の乱数に落ちる。テスト自身が `stay_tests.rs:605` で「後に置いた方が識別子の順で先に来ることもあるように」と書いており、順を固定していない。
  （別件の実測: 変異を戻すときに元の mtime を復元したら cargo が再ビルドせず、変異入りのバイナリのまま 7 回中 4 回この 1 本だけが落ちた。**CI でも「たまに落ちる」に見える形**）
- kind: technical
- 提案: 2 件の既存の滞在の `id` を固定値にし、遅い方の `id` が辞書順で先に来るようにする（`00000000-…` と `ffffffff-…`）。そうすれば始まりの比較を消したとき必ず落ちる。

## R28. 「読み出しに出ている既存の滞在を先に割り当てる」（spec の要件本文）を固定するテストが無い

- 成果物: crates/server/src/stay_store.rs:422 / specs/derived-records/spec.md の Requirement「作り直しても同じ滞在は同じ識別子を保つ」
- 根拠: `.then(a.1.cmp(&b.1))` を `.then(b.1.cmp(&a.1))`（吸収済み・隠れているものを先に）に反転 → `test result: ok. 56 passed; 0 failed`。
  要件本文は「重なりが同じ組は、読み出しに出ている既存の滞在を先に、それも同じなら始まりの早い既存の滞在を先に」と 2 段を書いているが、Scenario は後段（始まり）しか持たない。
  D3 は「本人の決定なので下流は変えない」とし、同点の順は「変えるなら本人に聞く」と書いている規則で、変えても 1 本も落ちない
- kind: daily
- 提案: 吸収済みの滞在と読み出しに出ている滞在が同じ重なりで並ぶ入力を 1 本足す（spec に Scenario を足すかは上流の判断）。

## R29. D12（端が触れるだけの重なり）の 2 つの決め方が、どちらも test で固定されていない

- 成果物: crates/server/src/stay_store.rs:408（識別子の引き継ぎ）/ :458（本人が消した時間帯）/ design.md D12（仮）
- 根拠:
  - `touches_erased` の `f.start <= *e && *s <= f.end` を `<` に変える（触れるだけでは隠さない）→ 56 passed / 0 failed
  - 引き継ぎの `if ov <= Duration::zero()` を `<` に変える（重なり 0 の組も候補にする）→ 56 passed / 0 failed
  - D12 は「09:00〜09:30 の既存の滞在は、09:30 に始まる新しい滞在の識別子にならない」「端が触れるだけでも隠す（既定は厳しい側）」と書き、反転条件まで置いているが、どちらに倒しても全部通る。Q12「少しでも重なる」の境目そのものがここ
- kind: daily
- 提案: 09:00〜09:30 を本人が消した後に 09:30 始まりの滞在ができる入力（隠れる）と、09:30 始まりの新しい滞在が 09:00〜09:30 の識別子を継がない入力を 1 本ずつ足す。

## R30. spec「作り直しの範囲の端でとどまりが続いているときは、とどまりが閉じるところまで範囲を広げる」を観測するテストが無い

- 成果物: crates/server/src/stay_store.rs:359-366（D5 の 2）/ :351-356（D5 の 1）/ crates/server/src/stay_tests.rs:1240-1257
- 根拠:
  - D5 の 2（集まりが端をまたぐなら広げる）を両端とも `if false && …` にした複製 → 56 passed / 0 failed
  - D5 の 1（端の近くの読み出しに出ている滞在まで広げる）だけを切った複製 → 56 passed / 0 failed。吸収済みでも広げるように変えた複製 → 56 passed / 0 failed
  - 唯一の Scenario テスト `stay_auto_rebuild_across_midnight_in_two_posts` は 0 時前が 20:00〜23:59 の 4 時間で、それ自体が滞在になるので D5 の 1 だけで通る。**2 の規則が要るのは「0 時前の分が最短のとどまりに満たない」ときだけ**
  - 実装そのものは正しい: 55417 のサーバに 23:52〜23:59（7 分）と 00:00〜00:20 を 2 回に分けて送ると、読み出しに出ている滞在は `2026-08-02T14:52:00Z|2026-08-02T15:20:00Z`（23:52〜00:20 の 1 件）
- kind: technical
- 提案: 上の 7 分 + 20 分の 2 回送りを DB テストに足し、始まりが 23:52 であることを見る（D5 の 2 を切ると 00:00 始まりになって落ちる）。

## R31. D5 の下流追記「重複（`duplicate: true`）も受け入れに含めて作り直す」が test で固定されていない

- 成果物: crates/server/src/lib.rs:919 / design.md D5「実装での形」
- 根拠: `if !result.accepted {` を `if !result.accepted || result.duplicate {` にした複製 → 56 passed / 0 failed。
  重複だけのまとめ送り（応答を取り落とした端末の再送）で作り直しが走るかどうかは、どちらに倒しても全部通る
- kind: daily
- 提案: 同じまとめ送りを 2 回送り、2 回目の前に作り直しを失敗させる差し替え（`StayRebuilder::from_fn`）で呼ばれた回数を数えるテストを足す。

## R32. D8 の「今日」の 2 規則（最後の位置まで移動を並べる / 最後の位置からいままでが間隔以上のときだけ記録なし）が test で固定されておらず、壊すと 5 分の「記録なし」が出る

- 成果物: crates/server/src/stay_store.rs:824（`tail`）/ :852（`move_upper`）/ crates/server/src/stay_tests.rs:1857-1885
- 根拠:
  - `let tail = if today { now } …` を `if false` にした複製 → 56 passed / 0 failed。`let move_upper = if today {` を `if false {` にした複製 → 56 passed / 0 failed
  - 変異入りのサーバと元のサーバを同じ 55417 の DB で起動し、「いまの 40 分前〜5 分前」の位置 36 件を送って `GET /stays?date=<今日>`:
    - 元: `[["no-record","15:00:00","12:54:00"],["stay","12:54:00","13:29:00"]]`
    - `tail` の変異: `[…,["stay","12:54:00","13:29:00"],["no-record","13:29:00","13:34:44"]]` —— 最後の位置からたった 5 分で「記録なし」が付く
  - `stays_day_api_today_stops_at_now` は「09:00 より後の時刻を持つ行が無い」だけを見ており、変異の行（08:55–09:00）はその条件を満たすので通る。design D8「最後の位置からいままでが `gap_minutes` 以上なら、そこまでを記録なしにする」の「以上なら」を誰も見ていない
- kind: daily
- 提案: 同じテストで `now = 09:00` のときに記録なしの行が 0 件であること、移動の行が 08:55 で終わることを足す。

## R33. D8 の「正の長さで重なる滞在だけを並べる」「基準は新しい版から並べる」がサーバ側で固定されていない

- 成果物: crates/server/src/stay_store.rs:791 / :814 / web/src/__tests__/day-view.test.tsx:120-144
- 根拠:
  - `if !(start < d1 && (end > d0 || start >= d0))` を `if !(start < d1 && end >= d0)`（前日の 24:00 ちょうどに終わる滞在も翌日に出す）→ 56 passed / 0 failed
  - `tags.sort_by_key(|t| std::cmp::Reverse(t.criteria_id));` を消す → 56 passed / 0 failed
  - 画面は `criteria[0]` を「一覧の上」にし、それと違う基準の行にだけ基準を添える（`DayView.tsx:138-161`）。画面のテストは応答を固定しているので、**サーバの並びが逆になると「どれが残り物か」の添え方が黙って反転する**
- kind: daily
- 提案: `stays_day_api_criteria_follow_rebuild` に基準の違う滞在が 2 件並ぶ日を足して `criteria[0]` が新しい版であることを見る。0:00 ちょうどに終わる滞在が翌日に出ないことを 1 本足す。

## R34. Scenario「範囲外の基準は断られる」の印の付いたテストは、THEN の「滞在も増えない」を観測していない

- 成果物: crates/server/src/stay_tests.rs:1453-1495
- 根拠: 読んだ行。`let before = (criteria_list(...).len(), stays(&app.pool, u).await);` で滞在を取っておきながら、最後は `let _ = before.1;` で捨てている。
  基準の版の件数も、範囲外 5 通りの直後ではなく**境目の正しい指示を 1 回通した後**に `before.0 + 1` で見ている。
  「基準も滞在も変えない」を実際に比べているのは印の無い `stays_rebuild_api_rejection_changes_nothing`（`radius_m: 0` の 1 通りだけ）
- kind: daily
- 提案: 印の付いたテストで範囲外 5 通りの直後に `stays()` と基準の一覧を比べる（または印を `rejection_changes_nothing` に移し、5 通りをそちらで回す）。

## R35. 本人が消した滞在の時間は、1 日の一覧で「移動」として出る

- 成果物: crates/server/src/stay_store.rs:851-886（移動は「滞在にも記録なしにも入らない時間」）/ specs/browsing-views/spec.md「滞在と滞在の間は移動の行として出る」
- 根拠: 55417 のサーバに同じ地点 10:00〜11:00 と 5 km 先 11:01〜11:31 を送り、`GET /stays`:
  - 消す前: `["stay","01:00","02:00"],["move","02:00","02:01"],["stay","02:01","02:31"]`
  - 10:00〜11:00 の行を `deleted_by='user'` にした後: `["move","2026-08-01T01:00:00Z","2026-08-01T02:01:00Z"],["stay","02:01","02:31"]` —— 画面では「移動 1 時間 1 分」
  - spec は「消した滞在は一覧に出ない」「滞在と滞在の間は移動」を別々に定めているが、消した後の穴を何で埋めるかは決めていない。とどまっていた時間を「移動」と書くのは、記録なしを移動と同じ顔にしない（扉 #14）のと同じ型の取り違え。作り直しで吸収された時間でも同じ経路を通る
- kind: daily
- 提案: 穴は「消した時間」の行にするか何も出さないか（ST22 が消す操作を持つので、決めるのは ST22 の上流でもよい）。決めるまでは design D8 に（仮）として書く。

## R36. `tools/seed.sh max` を `normal` の後に同じ DB へ入れると rc=22 で落ち、一覧は 9 件のまま。run.sh はその失敗を握りつぶす

- 成果物: tools/seed.sh（位置の `id` を `uuid5(…/{when})` で作る）/ tools/verify-prep.sh:78
- 根拠: 55417 のまっさらな DB で `tools/seed.sh normal`（rc=0、滞在 9）→ 続けて `tools/seed.sh max` → `seed max after normal rc=22`、`GET /stays` は `{"stay":9,"move":9}` のまま。
  サーバのログは `kind="id_reused" logical_source=c01-location` が並ぶ —— `id` が時刻だけから作られ、normal と max で同じ `id` に別の本文が乗る。
  run.sh は `docker compose up -d --wait db`（`down -v` しない）で DB を持ち越し、`./tools/seed.sh "${SEED:-normal}" >/dev/null || echo "warn: seed が落ちた（続ける）"` なので、**前回 normal で起動した手元で `SEED=max ./run.sh` を叩くと 15 件ではなく 9 件の画面が出る**
- kind: daily
- 提案: `uuid5` の名前に `MODE`（または地点の並び）を含める。落ちたら run.sh を止める。

## R37. 位置の記録を消しても、その日の滞在は作り直されない（消した位置から作った滞在が読み出しに残る）

- 成果物: crates/server/src/lib.rs:903-968（作り直しの契機は `/ingest` の受け入れだけ）/ docs/stories/ST22.md / docs/handoff/ST22.md
- 根拠: 使い捨て DB に `stays_rebuild_api_rejection_changes_nothing` が残した行を引くと、`c01-location` がすべて `deleted_by='test'` の利用者の `s01-stay` が
  `deleted_at IS NULL`（読み出しに出ている）で `{"start":"2026-09-08T00:00:00Z",…,"lat":35.6812,"lon":139.7671,"points_used":31,…}` のまま残っている（6 行を確認）。
  過ぎた日にはもう位置が届かないので、手で `POST /stays/rebuild` を叩くまで残る。
  `grep -n "滞在\|派生\|作り直\|FR-31" docs/stories/ST22.md` は 0 件。`docs/handoff/ST22.md` は「滞在を消すときの `deleted_by` の値」と「消した時間帯の掛け方」だけで、**位置を消したときに滞在を作り直す約束はどこにも無い**
- kind: defer
- 提案: `docs/handoff/ST22.md` に「位置の記録に削除の印を付けたら、その日の滞在を作り直す（`stay_store::rebuild_day`）」を足す（`処置: followup ST22` の候補）。

## R38. 位置の本文を消去しても、滞在の原文と履歴に座標が残る（FR-51「消去は履歴に残した前の版にも及ぶ」が派生に届かない）

- 成果物: crates/server/src/stay.rs:189-207（`raw` に重心の緯度経度）/ crates/server/src/stay_store.rs:535-551（前の版へ `raw`・`payload` を写す）/ :623-646（吸収は行を消さず `raw` を残す）/ docs/stories/ST23.md
- 根拠:
  - 使い捨て DB（テスト 1 周ぶん）で `SELECT count(*) FROM core.event_version WHERE logical_source='s01-stay' AND payload ? 'lat'` → **1443**。いま居る滞在は作り直しのたびに版を積む（design Risks で 1 日 288 回・年 10 万行）ので、長い滞在ほど座標を持つ前の版が増える
  - 読んだ行: 消去の操作（ST23）はまだ無い。作り直しの契機は `/ingest` だけ（R37 で実測）なので、消去で位置の `payload` が `'{}'` になっても滞在は作り直されない。作り直しても、吸収（`absorb`）は `raw` を残し、`push_version` が写した前の版は `core.event_version` に残る（台帳の無い書き換えは門が拒む）
  - `grep -n "滞在\|派生" docs/stories/ST23.md` は 0 件。ST23 は滞在ができる前に書かれた Story で、**消去の範囲に派生と派生の履歴が入っていない**
- kind: defer
- 提案: `docs/handoff/ST23.md` を作り、「位置の本文を消去するときは、その時間と重なる `s01-stay` の行と `core.event_version` の前の版も消去の対象にする（台帳に載せる）」を渡す。

## R39. 滞在の移行の戻し手順（`202609142125_stays.down.sql`）を当てる検査がどこにも無い

- 成果物: migrations/202609142125_stays.down.sql / tools/check-immutable.sh:523-536
- 根拠: `check-immutable.sh` の戻しの段は `ST03_UP=(… 5 本)` を固定で並べており、`grep -n stay tools/check-immutable.sh` は 0 件（`check-migrations.sh` は down.sql の存在と記載を静的に見るだけ）。
  手で当てた結果は正しかった: 55417 で全版を当て、滞在 2 行（1 行は本人の削除）・基準 1 行・吸収 1 行を置いて down を当てると rc=0、
  削除の無い行は `rebuild:rolled-back`、本人の削除は `user` のまま、台帳 2 表は消える。続けて up を当て直すと rc=0
- kind: daily
- 提案: `check-immutable.sh` の戻しの段の並びを `MIGRATIONS` から引くか、`202609142125_stays` を足す（R117 と同じく「構文と依存」を機械に見させる）。

---

## 該当が無い・問題なしの手

- **手 2（ガードを壊す）**: 台帳 2 表の UPDATE / DELETE / TRUNCATE は `stay_ledgers_are_append_only` がトランザクションの中で撃って確かめており、検査の対象を空にして緑になる形ではない。錠・距離・NULL の削除・前日の読み込みの 4 つの申告は再現した（上の表）
- **手 3（Scenario と test）**: 65 本すべてに印があり、R34 以外は印の先のテストが WHEN/THEN の階層で観測していた（「作り直しで位置の記録は変わらない」は件数・`raw`・`event_time`・`content_hash`・`payload`・`deleted_at` を丸ごと比べる。「作り直しの失敗は位置の値を含まずに記録される」はエラーの本文にわざと値を入れてログを捕まえる）。
  「区切りが伸びても識別子は変わらない」の WHEN は「位置の記録が届く」だが、テストは取り込み口を通さず行を置いて `rebuild_day` を呼ぶ —— 取り込み口からの経路は `stay_auto_rebuild_*` が別に持つので指摘にしない
- **手 5（`[x]` と実体）**: 19 本の `CT` はすべて実在し件数 ≥ 1。7.1 の検証コマンドは 18787（別セッションのサーバ）を叩く形なので、18716 で同じ `jq -e` 相当を確かめた（9 / 15）。`cargo test --test <名前>` 形のタスクは無い
- **手 6（隙間）**: 取り込み口からの外部更新は削除済みの滞在を書き換えない（`lib.rs:771` の `SkippedDeleted`）ので Q2 の別経路は塞がっている。作り直しの失敗ログは SQLSTATE だけで本文を出さない（`lib.rs:1434-1441` も同じ）。
  失われる経路として見つけたのは R37 / R38（どちらも後続 Story の担当で、その Story が持っていない）
