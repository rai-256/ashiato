# ST16 実装タスク — 位置から滞在を作り、派生を作り直せる

読む順: `deep.md`（**最優先。本人が決めたこと。第 2 回 Q10〜Q12 を含む**）→ このファイル → `specs/derived-records/spec.md`
→ `specs/browsing-views/spec.md` → `design.md` → `docs/stories/ST16.md` → `docs/handoff/`（開始時と PR 前の 2 回）→ `CLAUDE.md`。

**移行は 1 本だけ足す**（design D10）。**名前は作成時刻 `YYYYMMDDHHMM_stays.sql`**（連番にしない）で、
`crates/server/src/lib.rs` の `MIGRATIONS` 配列の末尾に足す。
**`record-envelope` / `device-collection` / `collection-coverage` の要件と、既存の移行・索引・門・取り込みの受け入れの判定には触らない**
（ST03 / ST02 が archive 待ち。design D2）。

検証は各タスクの本文に書いてある。「動いた」ではなくコマンドと終了コードで判定する。
DB を使う検査は `docker compose up -d db` が前提。

## 0. 規律（**最初に読む**）

- **テストには `Scenario: <名前>` の印を置く。** Rust / TypeScript はコメント（`// Scenario: 区切りが伸びても識別子は変わらない`）、
  bash は `echo`。`scripts/check_scenarios.py` が spec の全 Scenario と突き合わせ、**印の無い Scenario を FAIL にする**
- 印の名前は spec の `#### Scenario:` と**一字一句合わせる**（空白は無視される）
- **この change が足す Scenario は 65 本**（`derived-records` 45 本 / `browsing-views` 20 本）
- **件数つき検証**: `cargo test <絞り込み>` は一致するテストが 0 本でも rc=0 になる（spec-review R26）。このファイルで
  **`CT <絞り込み>`** と書いたものは、次のコマンドが rc=0 になることを指す:
  `bash -o pipefail -c 'cargo test -p ashiato-server <絞り込み> 2>&1 | tee /tmp/ct.log' && grep -Eq 'test result: ok\. [1-9][0-9]* passed' /tmp/ct.log`
  （`cargo test` に絞り込みを 2 つ渡すと `unexpected argument` で落ちる。1 つずつ書く）
- **滞在のテストは利用者で隔離する**（design D10 / R13）。テストの接続先は開発 DB そのもので、基準の台帳は追記のみなので消せない。
  **`testdb::user()` で利用者を毎回新しく作り、その利用者の基準だけを変える。** 既定の利用者（`00000000-…`）の基準には触らない
- **作り直しを失敗させるテストは、作り直しの関数を差し替えて起こす**（design D5）。表の権限を剥がす・名前を変えるやり方は使わない
- **D2 / D4 / D5 / D6 / D8 は（仮）決め。** 反転条件は `design.md` にある。変えたらその D 番号の（仮）を外すか反転条件を書き直す
- **D3（識別子の引き継ぎ）は本人の決定**（第 1 回 Q1 / 第 2 回 Q10 / Q11）。割れたら重なり最大の 1 件だけが継ぎ、代表点が半径の 2 倍より離れていたら継がない。**下流は変えない**
- 滞在の値（座標・時刻）を**ログに出さない**（製造準備 A-2）。出すのは件数・日付・かかった時間だけ

## 1. 移行（design D10 / D2）

- [x] 1.1 移行 `migrations/YYYYMMDDHHMM_stays.sql` と `.down.sql` を足す —— `core.stay_criteria`（`user_id` あり。行は入れない）、
  `core.stay_absorbed`（`user_id` あり）、両方の UPDATE / DELETE / TRUNCATE を拒むトリガ、`core.source` に `s01-stay`（`external_id_kind = 'record'`）。
  **当て直せる形**（`IF NOT EXISTS` / `ON CONFLICT DO NOTHING`）。`MIGRATIONS` 配列の末尾に足す。
  検証: `tools/check-migrations.sh` rc=0、`CT stays_migration_applies_twice`（全版を 2 回当てて落ちないテストを足す）
- [x] 1.2 台帳 2 つが追記のみであることのテスト。検証: `CT stay_ledgers_are_append_only`
- [x] 1.3 `s01-stay` が登録され、`external_id_kind` が `'record'` であることのテスト。検証: `CT stay_source_is_registered`

## 2. 判定（design D7。DB に触らない純粋な関数）

`crates/server/src/stay.rs` に、位置の列と基準から滞在の列を返す関数を置く。

- [x] 2.1 半径・最短のとどまり・重心の更新・消去された記録の読み飛ばし。
  Scenario: `半径の中に最短のとどまり以上いると滞在が 1 件できる` / `最短のとどまりより短い立ち寄りは滞在にならない` /
  `半径を出ると滞在が閉じる` / `長い滞在は区切られない` / `日付をまたぐ滞在は 1 件のまま` / `本文を消去した位置の記録は判定に数えない`。
  検証: `CT stay::detect`
- [x] 2.2 記録が無い区間で切る（`gap_minutes`。精度を問わず緯度経度を持つすべての記録で数える）。
  Scenario: `記録が欠けた区間の前後は別々の滞在になる` / `精度の悪い点しか無い区間は記録なしにならない`。
  検証: `CT stay::gap`
- [x] 2.3 精度が半径より悪い記録を判定に使わない。精度の欄が無い記録は使う。
  Scenario: `精度の悪い 1 点が混ざっても滞在は割れない` / `精度を持たない位置の記録は判定に使われる`。
  検証: `CT stay::accuracy`
- [x] 2.4 `raw` の直列化を固定する（キーの順・緯度経度 6 桁）。同じ入力から 2 回作った `raw` がバイト単位で同じことと、
  期待する文字列そのものをテストで固定する（design D1）。検証: `CT stay_raw_is_pinned`

## 3. 作り直し（design D3 / D4 / D5 の錠 / D9 / D10。DB を使う）

`crates/server/src/stay_tests.rs` に置く。

- [x] 3.1 滞在を `core.event` に書く（design D1 の列の表のとおり）。感度には何も書かない。基準は利用者ごと（無ければ既定を最初の版として書く）。
  Scenario: `作った滞在は派生させたに分類される` / `滞在に作った基準と使った件数が載る` / `作った滞在の感度は外部 AI に出してよい` /
  `基準は利用者ごとに分かれる` / `判定に使わなかった位置の記録は残る`。
  検証: `CT stay_row_shape`
- [x] 3.2 識別子の引き継ぎ（design D3。代表点が半径の 2 倍より離れた組を捨ててから）。重なりの大きい順・1 対 1・
  同じなら読み出しに出ている方 → 始まりの早い方。内容が変わるときだけ前の版を積む（終わりだけ伸びても積む）。
  割り当てのない既存の滞在に `deleted_by = 'rebuild:absorbed'` と吸収の台帳。候補は `origin='derived'` の `s01-stay` だけ。
  Scenario: `区切りが伸びても識別子は変わらない` / `区切りが伸びると前の版が残る` /
  `割れた滞在は重なりのいちばん大きい 1 件が識別子を継ぐ` / `離れた滞在には識別子を継がない` /
  `重なりが同じなら始まりの早い既存の滞在から割り当てる` / `吸収された滞在は読み出しから外れる` / `吸収された滞在の行と吸収先が残る` /
  `基準を戻すと吸収された滞在が同じ識別子で戻る` / `取り込みの口から送った滞在は作り直しで置き換わる`。
  検証: `CT stay_identity`
- [x] 3.3 冪等。**内容が同じ作り直しで滞在が黙って消えないこと**を確かめる（deep-review R1 の型）。
  Scenario: `基準を添えずに作り直しても滞在は増えも消えもしない` / `基準を添えずに作り直しても前の版は増えない`。
  検証: `CT stay_rebuild_is_idempotent`
- [x] 3.4 本人が消した時間帯（design D4。一部だけ重なる滞在は丸ごと隠す）。本人の削除はテストの中で
  `UPDATE core.event SET deleted_at = now(), deleted_by = 'user'`（と `deleted_by = NULL`）で作る（消す操作は ST22）。
  Scenario: `同じ基準で作り直しても消した滞在は戻らない` / `本人が消した滞在の行は作り直しで変わらない` /
  `基準を変えて割れても、消した時間帯の断片は戻らない` / `消した範囲より長い滞在に統合されると丸ごと隠れる` /
  `基準を戻して重ならなくなった断片は戻る` / `削除した者の欄が空の削除も本人が消したものとして扱う` /
  `作り直しで無くなった滞在は本人が消した時間帯にならない`。
  検証: `CT stay_erased_range`
- [x] 3.5 作り直しで位置の記録が変わらないこと。前後で `c01-location` の行の件数・`raw`・`event_time`・`content_hash` をすべて比べる。
  Scenario: `半径を変えて作り直すと区切りが変わる`（地点 A と 70 m 離れた地点 B。spec の入力のとおり）/ `作り直しで位置の記録は変わらない`。
  検証: `CT stay_rebuild_keeps_locations`
- [x] 3.6 錠と範囲（design D5）。同じ利用者の同じ日の作り直しを `tokio::join!` で 2 本同時に走らせる。
  Scenario: `同じ日の作り直しが同時に 2 回走っても滞在は二重にならない`。検証: `CT stay_rebuild_is_serialized`
- [x] 3.7 `c01-location` の 1 日ぶんを引く文を `EXPLAIN` するテストを置き、計画に `event_by_source_time` を含み
  `Seq Scan on event` を含まないことを確かめる（design Risks）。含むなら 1.1 の移行に索引を足す。
  検証: `CT stay_day_query_uses_index`

## 4. 契機と API（design D5 / D2）

- [x] 4.1 `/ingest` のまとめ送り 1 回の後、基準の `sources` の記録を受け入れた日ごと・利用者ごとに作り直す（範囲の広げ方は D5）。
  失敗は `kind = "stay.rebuild"` でログに残し、応答を変えない。**`/ingest` の受け入れの判定には触らない**（`s01-stay` を断らない）。
  Scenario: `位置を送るとその日の滞在が出る` / `位置が届かなかった日の滞在は変わらない` /
  `0 時の前後で別々に届いても日付をまたぐ滞在は 1 件のまま`（**`/ingest` を 2 回叩く DB のテスト**。純粋な関数では確かめない）/
  `作り直しが失敗しても位置の記録は受け入れられる` / `作り直しの失敗は位置の値を含まずに記録される`（ログを捕まえて緯度・経度・時刻の文字列が無いことを見る）。
  検証: `CT stay_auto_rebuild`
- [x] 4.2 `POST /stays/rebuild`（本文の基準がいまと違えば版を足す。同じか省けば足さない。範囲外は 400）と `GET /stays/criteria`。資格情報が要る。
  Scenario: `基準を変えても前の基準は一覧に残る` / `いまと同じ基準を添えても基準の版は増えない` / `範囲外の基準は断られる` /
  `資格情報の無い作り直しの指示は断られる`。
  検証: `CT stays_rebuild_api`
- [x] 4.3 既存の `derived_rebuild_is_not_folded`（ST03）が緑のままであること（取り込みの受け入れの判定を変えていないことの担保）。
  検証: `CT derived_rebuild_is_not_folded`
- [x] 4.4 `POST /stays/rebuild` / `GET /stays/criteria` / `GET /stays` を OpenAPI に載せる。検証: `tools/check-openapi.sh` rc=0

## 5. 1 日の並びの読み出し（design D8 のサーバ側）

- [x] 5.1 `GET /stays?date=YYYY-MM-DD` —— その日と重なる、読み出しに出ている `origin='derived'` の滞在と、その間の移動・記録なしを時刻順に返す。
  記録なしは前後の日の位置も含めて間隔で測り、今日はいまより後を返さない。`criteria` はその日の滞在を作った基準の重複なしの並び。
  Scenario: `1 日の並びは種類と時刻を持つ` / `解釈できない日付は断られる` / `資格情報の無い 1 日の並びの求めは断られる` /
  `日付をまたぐ滞在は両方の日に出る` / `消した滞在と吸収された滞在は一覧に出ない` / `滞在の間に移動の行が出る` /
  `記録が欠けた時間は記録なしとして出る` / `位置の記録が無い日は丸ごと記録なしになる` /
  `前の日から途切れず続く記録は日の頭を記録なしにしない` / `今日の一覧はいまより後を記録なしにしない`（「いま」を差し替えられる形にする）/
  `基準を変えて作り直すと一覧の基準の表示が変わる`（API の `criteria` で確かめる）。
  検証: `CT stays_day_api`
- [x] 5.2 1 日歩き回った日の件数（自宅・職場・昼の店・職場・自宅 → 滞在 5 件・移動 4 件）をサーバの応答で確かめる
  （画面のテストは応答を固定するので、サーバが何件返すかはここで見る。R21）。
  Scenario: `1 日歩き回った後、その日の滞在が一覧で出る`。検証: `CT stays_day_api_walked_day`

## 6. 画面（design D8。`web`）

- [x] 6.1 `#/day/YYYY-MM-DD` で 1 日の一覧を出す（省けば今日。Asia/Tokyo）。S-1 はルート `/` のまま、互いへの行き先を上端に置く。
  行の形は deep.md Q4 の proto の出力と読み方 3 のとおり（見出し＝時刻の範囲 / 長さ・始まり – 終わり /
  「移動 42 分」/「記録なし 8:20–16:40」を**文字で**区別 / 一覧の上に「この一覧は 半径 100 m / 10 分 で作った」/ 基準が混ざる日は行に添える）。
  読み込み中・失敗・0 件を分ける。
  Scenario: `1 日歩き回った後、その日の滞在が一覧で出る`（`GET /stays` の応答を固定して描画を見る）/
  `基準の違う滞在が混ざると行に基準が添えられる` / `読み出しに失敗すると失敗と出る` /
  `日付を含むアドレスでその日の一覧が開く` / `前の日へ移ると前の日の滞在が出る` / `稼働状況の画面の入口は変わらない`。
  検証: `cd web && npm run test` rc=0 かつ出力に `Tests` の行があり `failed` を含まない
- [x] 6.2 下限の検査を足す（既存の `text-contrast.test.ts` / `target-size.test.tsx` と同じ形）。
  Scenario: `一覧の文字はライトでもダークでも 4.5:1 を下回らない` / `日を移る操作は 24 px を下回らない` /
  `キーボードで移るとフォーカスの位置が見える`。
  検証: `cd web && npm run test` rc=0
- [x] 6.3 検証: `cd web && npm run lint && npm run build` rc=0、`tools/check-boundaries.sh` rc=0

## 7. 偽データと起動の確認

- [x] 7.1 `tools/seed.sh` の `normal` / `max` が、**2026-09-07（Asia/Tokyo）の 1 日 1,440 件の `c01-location` の位置**（60 秒ごと、水平精度 8〜45 m の揺れ）を
  まとめ送り（200 件ずつ）で入れる。既定の基準で滞在が **9 件 / 15 件**できるように、互いに 500 m 以上離れた地点に 20 分以上ずつとどまらせる。
  **最後の滞在の後に 30 分の記録の欠けを 1 つ**入れる（滞在の件数を変えない位置）。`empty` は変えない。既存の `seed-location` の 9 件は残す。
  検証: `tools/seed.sh normal` rc=0 の後、
  `curl -sf -H "authorization: Bearer $API_TOKEN" "http://127.0.0.1:18787/stays?date=2026-09-07" | jq -e '([.entries[]|select(.kind=="stay")]|length)==9 and ([.entries[]|select(.kind=="no-record")]|length)>=1'` rc=0
  （`max` なら `==15`）
- [x] 7.2 `tools/smoke.sh` に、位置を送って `GET /stays` に滞在が出ることの確認を 1 段足す（`jq -e` で件数を見る）。検証: `tools/smoke.sh` rc=0

## 8. まとめの検査

- [x] 8.1 検証: `cargo test --workspace` rc=0、`cd web && npm run test && npm run lint && npm run build` rc=0
- [x] 8.2 検証: `python3 scripts/check_scenarios.py . st16-stay-derivation` rc=0（65 本すべてに印か、人間の確認待ち）
- [x] 8.3 検証: `python3 scripts/check_chain.py .` rc=0、`openspec validate st16-stay-derivation --strict` rc=0、
  `tools/check-migrations.sh` / `tools/check-openapi.sh` / `tools/check-boundaries.sh` / `tools/check-immutable.sh` がすべて rc=0
- [ ] 8.4 PR 本文に **仮決め（D2 / D4 / D5 / D6 / D8）と反転条件**を列挙する。検証: `gh pr view --json body -q .body | grep -c "D[24568]（仮）"` が 5 以上
- [ ] 8.5 `docs/handoff/` を読み直す（開始時と PR 前の 2 回）。検証: `ls docs/handoff/ST16.md 2>/dev/null` が空か、あればその各項目に PR 本文で触れている

## 人間の確認待ち

**本人の目と、実機の 1 日でしか確かめられないもの。** 確認バッチ（`/verify`）でまとめて見る。
**書式は `- Scenario: <名前>` の裸の形**（チェックボックスも番号も注釈も付けない）。やり方は次の行の引用に置く。

- Scenario: 1 日歩き回った後、その日の滞在が一覧で出る
  > 端末を持って 1 日過ごした翌日、`run.sh` の画面で `#/day/<その日>` を開き、居た場所の数だけ時刻の範囲の行が並ぶことを見る
- Scenario: 半径を変えて作り直すと区切りが変わる
  > 偽データ（`SEED=normal`）の 2026-09-07 を開いた後、手順書の `POST /stays/rebuild`（半径 30 m）を叩き、同じ日の一覧の行が増え、
  > 上端の基準が「半径 30 m / 10 分」になることを見る
- Scenario: 作り直しで位置の記録は変わらない
  > 上の作り直しの前後で、手順書の `psql` の 1 行（`c01-location` の件数と内容の鍵の集計）が同じ値を返すことを見る
- Scenario: 記録が欠けた時間は記録なしとして出る
  > 偽データ（`SEED=normal`）の 2026-09-07 を開き、「記録なし」の行が「移動」と文字で違って見えることを見る
- Scenario: キーボードで移るとフォーカスの位置が見える
  > PC のブラウザで Tab を押して前の日・次の日・日付の入力を順に移り、どこにいるか分かることを見る
