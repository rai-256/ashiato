# ST16 実装タスク — 位置から滞在を作り、派生を作り直せる

読む順: `deep.md`（**最優先。本人が決めた 9 件**）→ このファイル → `specs/derived-records/spec.md`
→ `specs/browsing-views/spec.md` → `design.md` → `docs/stories/ST16.md` → `docs/handoff/`（開始時と PR 前の 2 回）→ `CLAUDE.md`。

**移行は 1 本だけ足す**（design D10）。**名前は作成時刻 `YYYYMMDDHHMM_stays.sql`**（連番にしない）で、
`crates/server/src/lib.rs` の `MIGRATIONS` 配列の末尾に足す。
**`record-envelope` / `device-collection` / `collection-coverage` の要件と、既存の移行・索引・門には触らない**
（ST03 / ST02 が archive 待ち）。

検証は各タスクの本文に書いてある。「動いた」ではなくコマンドと終了コードで判定する。
DB を使う検査は `docker compose up -d db` が前提。

## 0. 規律（**最初に読む**）

- **テストには `Scenario: <名前>` の印を置く。** Rust はコメント（`// Scenario: 区切りが伸びても識別子は変わらない`）、
  TypeScript もコメント、bash は `echo`。`scripts/check_scenarios.py` が spec の全 Scenario と突き合わせ、
  **印の無い Scenario を FAIL にする**。`scripts/merge_gate.sh` がこの検査を見る
- 印の名前は spec の `#### Scenario:` と**一字一句合わせる**（空白は無視される）
- **この change が足す Scenario は 37 本**（`derived-records` 25 本 / `browsing-views` 12 本）
- **D3 / D4 / D5 / D6 は（仮）決め。** 反転条件は `design.md` にある。変えたらその D 番号の（仮）を外すか反転条件を書き直す
- **D3 は本人の選んだ選択肢の説明文と実装の形が違う**（性質は同じ）。理由は design D3 と deep.md「当初案を覆したもの」。
  **説明文のとおり「その日・場所から鍵を計算する」形に戻さない**
- 滞在の値（座標・時刻）を**ログに出さない**（製造準備 A-2）。出すのは件数・日付・かかった時間だけ

## 1. 移行（design D10 / D2）

- [ ] 1.1 移行 `migrations/YYYYMMDDHHMM_stays.sql` と `.down.sql` を足す —— `core.stay_criteria`（最初の行 100 / 10 / 10 /
  `{c01-location}`）、`core.stay_absorbed`、両方の UPDATE / DELETE / TRUNCATE を拒むトリガ、`core.source` に
  `s01-stay`（`external_id_kind = 'record'`）。**当て直せる形**（`IF NOT EXISTS` / `ON CONFLICT DO NOTHING`）。
  `MIGRATIONS` 配列の末尾に足す。
  検証: `tools/check-migrations.sh` rc=0、`cargo test migrations_apply_twice` rc=0（既存のテストが無ければ足す）
- [ ] 1.2 台帳 2 つが追記のみであることのテスト。検証: `cargo test stay_ledgers_are_append_only` rc=0
- [ ] 1.3 `s01-stay` が登録され、`external_id_kind` が `'record'` であることのテスト。
  検証: `cargo test stay_source_is_registered` rc=0
- [ ] 1.4 `c01-location` の 1 日ぶんを引く文を `EXPLAIN` し、`(logical_source, event_time)` の索引
  （`202609112113_source_lifecycle.sql:72`）を使っていることを確かめる（design Risks）。
  全表走査なら 1.1 の移行に索引を足す。検証: `EXPLAIN` の出力を PR 本文に貼る

## 2. 判定（design D7。DB に触らない純粋な関数）

`crates/server/src/stay.rs` に、位置の列と基準から滞在の列を返す関数を置く。

- [ ] 2.1 半径・最短のとどまり・重心の更新。
  Scenario: `半径の中に最短のとどまり以上いると滞在が 1 件できる` / `最短のとどまりより短い立ち寄りは滞在にならない` /
  `半径を出ると滞在が閉じる` / `長い滞在は区切られない` / `日付をまたぐ滞在は 1 件のまま`。
  検証: `cargo test -p ashiato-server stay::` rc=0
- [ ] 2.2 記録が無い区間で切る（`gap_minutes`。精度を問わずすべての記録で数える）。
  Scenario: `記録が欠けた区間の前後は別々の滞在になる` / `精度の悪い点しか無い区間は記録なしにならない`。
  検証: `cargo test -p ashiato-server stay::gap` rc=0
- [ ] 2.3 精度が半径より悪い記録を判定に使わない。`lat` / `lon` の無い記録（消去済み）を読み飛ばす。
  Scenario: `精度の悪い 1 点が混ざっても滞在は割れない`。
  検証: `cargo test -p ashiato-server stay::accuracy` rc=0
- [ ] 2.4 `raw` の直列化を固定する（キーの順・緯度経度 6 桁）。同じ入力から 2 回作った `raw` がバイト単位で同じ
  ことと、期待する文字列そのものをテストで固定する（design D1）。
  検証: `cargo test -p ashiato-server stay_raw_is_pinned` rc=0

## 3. 作り直し（design D3 / D4 / D9。DB を使う）

`crates/server/src/stay_tests.rs` に置く。

- [ ] 3.1 滞在を `core.event` に書く（design D1 の列の表のとおり）。感度には何も書かない。
  Scenario: `滞在に作った基準と使った件数が載る` / `作った滞在の感度は外部 AI に出してよい`。
  検証: `cargo test stay_row_shape` rc=0
- [ ] 3.2 識別子の引き継ぎ（重なりの大きい順・1 対 1・同じなら読み出しに出ている方 → 始まりの早い方）。
  内容が変わるときだけ前の版を `core.event_version` に積む。割り当てのない既存の滞在に `deleted_by = 'rebuild:absorbed'` と
  吸収の台帳。
  Scenario: `区切りが伸びても識別子は変わらない` / `割れた滞在は重なりのいちばん大きい 1 件が識別子を継ぐ` /
  `吸収された滞在は読み出しから外れ、吸収先が残る` / `基準を戻すと吸収された滞在が同じ識別子で戻る`。
  **重なりが同じ組の順を固定するテストを 1 本足す**（design Risks）。
  検証: `cargo test stay_identity` rc=0
- [ ] 3.3 冪等。**内容が同じ作り直しで滞在が黙って消えないこと**を確かめる（deep-review R1 の型。
  `derived_rebuild_is_not_folded` は内容の違う 2 件しか見ていなかった）。
  Scenario: `基準を変えずに作り直しても滞在は増えも消えもしない`。
  検証: `cargo test stay_rebuild_is_idempotent` rc=0
- [ ] 3.4 本人が消した時間帯（design D4）。本人の削除はテストの中で `UPDATE core.event SET deleted_at = now(), deleted_by = 'user'` で作る
  （消す操作は ST22）。
  Scenario: `同じ基準で作り直しても消した滞在は戻らない` / `基準を変えて割れても、消した時間帯の断片は戻らない` /
  `作り直しで無くなった滞在は本人が消した時間帯にならない`。
  検証: `cargo test stay_erased_range` rc=0
- [ ] 3.5 作り直しで位置の記録が変わらないこと。前後で `c01-location` の行の件数・`raw`・`event_time`・`content_hash` を
  すべて比べる。
  Scenario: `半径を変えて作り直すと区切りが変わり、位置の記録は変わらない`。
  検証: `cargo test stay_rebuild_keeps_locations` rc=0

## 4. 契機と API（design D5 / D2）

- [ ] 4.1 `/ingest` のまとめ送り 1 回の後、基準の `sources` の記録を受け入れた日ごと・利用者ごとに作り直す。
  範囲は日を重なる滞在の端まで広げる。失敗は `kind = "stay.rebuild"` でログに残し、応答を変えない。
  Scenario: `位置を送るとその日の滞在が出る` / `位置が届かなかった日の滞在は変わらない` /
  `作り直しが失敗しても位置の記録は受け入れられる`（失敗はテストの中で吸収の台帳を読めなくして作る）。
  検証: `cargo test stay_auto_rebuild` rc=0
- [ ] 4.2 `POST /stays/rebuild`。本文に基準があれば台帳に新しい版を足してから全期間を作り直す。資格情報が要る。
  Scenario: `基準を変えても前の基準は残る` / `資格情報の無い作り直しの指示は断られる`。
  検証: `cargo test stays_rebuild_api` rc=0
- [ ] 4.3 `/ingest` が `s01-stay` を 400 で断る（`origin='derived'` 全体は断らない。
  `derived_rebuild_is_not_folded` が緑のままであること）。
  Scenario: `取り込みの口から滞在を送ると断られる`。
  検証: `cargo test ingest_rejects_stay_source derived_rebuild_is_not_folded` rc=0
- [ ] 4.4 `POST /stays/rebuild` と `GET /stays` を OpenAPI に載せる。検証: `tools/check-openapi.sh` rc=0

## 5. 1 日の一覧の読み出し（design D8 のサーバ側）

- [ ] 5.1 `GET /stays?date=YYYY-MM-DD` —— その日と重なる、読み出しに出ている滞在と、その間の移動・記録なしを時刻順に返す。
  位置の記録が無い日は `00:00–24:00` の記録なし 1 件。`criteria` はその日の滞在を作った基準の重複なしの並び。資格情報が要る。
  Scenario: `日付をまたぐ滞在は両方の日に出る` / `消した滞在と吸収された滞在は一覧に出ない` /
  `滞在の間に移動の行が出る` / `記録が欠けた時間は記録なしとして出る` / `位置の記録が無い日は丸ごと記録なしになる` /
  `基準を変えて作り直すと一覧の基準の表示が変わる`（API の `criteria` で確かめる）。
  検証: `cargo test stays_day_api` rc=0

## 6. 画面（design D8。`web`）

- [ ] 6.1 `#/day/YYYY-MM-DD` で 1 日の一覧を出す（省けば今日。Asia/Tokyo）。S-1 はルート `/` のまま。
  互いへの行き先を上端に置く。行の形は deep.md Q4 の proto の出力のとおり（見出し＝時刻の範囲 / 長さ・始まり – 終わり /
  「移動 42 分」/「記録なし 8:20–16:40」を**文字で**区別 /「この一覧は 半径 100 m / 10 分 で作った」）。
  読み込み中・失敗・0 件を分ける。
  Scenario: `1 日歩き回った後、その日の滞在が一覧で出る`（`GET /stays` の応答を固定して描画を見る）/
  `読み出しに失敗すると失敗と出る` / `前の日へ移ると前の日の滞在が出る`。
  検証: `cd web && npm run test` rc=0
- [ ] 6.2 下限の検査を足す（既存の `text-contrast.test.ts` / `target-size.test.tsx` と同じ形）。
  Scenario: `一覧の文字はライトでもダークでも 4.5:1 を下回らない` / `日を移る操作は 24 px を下回らない` /
  `キーボードで移るとフォーカスの位置が見える`。
  検証: `cd web && npm run test` rc=0
- [ ] 6.3 検証: `cd web && npm run lint && npm run build` rc=0、`tools/check-boundaries.sh` rc=0

## 7. 偽データと起動の確認

- [ ] 7.1 `tools/seed.sh` の `normal` / `max` が、**1 日 1,440 件の `c01-location` の位置**（60 秒ごと、水平精度のばらつきあり）を
  まとめ送り（200 件ずつ）で入れ、既定の基準で滞在がそれぞれ **9 件 / 15 件**できるようにする
  （ファイル冒頭の説明がもともと「滞在 9 件ぶん / 15 件」と書いている）。途中に 30 分の記録の欠けを 1 つ入れる。
  `empty` は変えない。既存の `seed-location` の 9 件は残す（他 Story の画面が使う）。
  検証: `tools/seed.sh normal` の後に `curl -sf -H "authorization: Bearer $API_TOKEN" "http://127.0.0.1:18787/stays?date=<その日>"` の
  `kind == "stay"` が 9 件、`kind == "no-record"` が 1 件以上
- [ ] 7.2 `tools/smoke.sh` に、位置を送って `GET /stays` に滞在が出ることの確認を 1 段足す。検証: `tools/smoke.sh` rc=0

## 8. まとめの検査

- [ ] 8.1 検証: `cargo test --workspace` rc=0、`cd web && npm run test && npm run lint && npm run build` rc=0
- [ ] 8.2 検証: `python3 scripts/check_scenarios.py . st16-stay-derivation` rc=0（37 本すべてに印か、人間の確認待ち）
- [ ] 8.3 検証: `python3 scripts/check_chain.py .` rc=0、`openspec validate st16-stay-derivation --strict` rc=0、
  `tools/check-migrations.sh` / `tools/check-openapi.sh` / `tools/check-boundaries.sh` / `tools/check-immutable.sh` がすべて rc=0
- [ ] 8.4 PR 本文に **仮決め（D3 / D4 / D5 / D6）と反転条件**を列挙する。**D3 は選択肢の説明文と形が違うことを冒頭に書く**。
  `docs/handoff/` を読み直す（開始時と PR 前の 2 回）

## 人間の確認待ち

**本人の目と、実機の 1 日でしか確かめられないもの。** 確認バッチ（`/verify`）でまとめて見る。
**書式は `- Scenario: <名前>` の裸の形**（チェックボックスも番号も注釈も付けない）。やり方は次の行の引用に置く。

- Scenario: 1 日歩き回った後、その日の滞在が一覧で出る
  > 端末を持って 1 日過ごした翌日、`run.sh` の画面で `#/day/<その日>` を開き、居た場所の数だけ時刻の範囲の行が並ぶことを見る
- Scenario: 半径を変えて作り直すと区切りが変わり、位置の記録は変わらない
  > 手順書の `POST /stays/rebuild`（半径 50 m）を叩き、同じ日の一覧の行が増え、上端の基準が「半径 50 m / 10 分」になることを見る。
  > 位置の記録の件数が叩く前と同じことを手順書の `psql` の 1 行で見る
- Scenario: 記録が欠けた時間は記録なしとして出る
  > 偽データ（`SEED=normal`）の日を開き、「記録なし」の行が「移動」と文字で違って見えることを見る
- Scenario: キーボードで移るとフォーカスの位置が見える
  > PC のブラウザで Tab を押して前の日・次の日・日付の入力を順に移り、どこにいるか分かることを見る
