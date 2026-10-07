# code review — st21-place-registry

## final review（6503b7f..02193371）

席: final reviewer（`superpowers:requesting-code-review` の `code-reviewer.md`。全文 `.superpowers/sdd/tasks/final-review.md`）。判定 With fixes（Critical 0 / Important 2 / Minor 13）。
fix は 1 回（02193371..5ac5b4d、`.superpowers/sdd/tasks/final-fix-report.md`）、scoped re-review 1 回（`.superpowers/sdd/tasks/final-re-review.md`）で I1・I2・M1〜M5・M10・M11・M13 は ADDRESSED。

## R1. 登録で入力を変えて押し直すと器ごと組み直し、前の器と受理された記録が消せないまま残る（応答の取りこぼし後は同じ座標に名前付きの場所が 2 つ）
- 成果物: web/src/PlaceForms.tsx
- 根拠: web/src/PlaceForms.tsx:257-269（02193371。`AddForm` の key が名前・広さ・補足を含み、`buildRegistration` が毎回新しい placeId を引く）。e2e web/e2e/places.spec.ts:756-785 は /api を偽装していて副作用を見ていない
- kind: daily
- 処置: fixed D13 仮 — 器が受理されたら placeId を保ち、変わった項目の記録だけを組み直す（web/src/places.ts の `buildRegistration(input, now, prev)`）。名前が空白だけのうちは「登録する」を押せない。実サーバの e2e（places.spec.ts の応答を落として名前を打ち直す試験）と vitest 3 本。既存の vitest「入力を変えて押し直すと器の識別子も原文も組み直す」は D13 の決めを変えたので期待を置き換えた

## R2. 同じ座標の記録を直す fix が 2 件（兄弟の fix）のとき未決で、直した座標が「移る前」と表示される
- 成果物: crates/server/src/places.rs
- 根拠: crates/server/src/places.rs:479-541（versions）・:847-851（fixer が最初に書いた fix）、crates/server/src/lib.rs:788-797（取り込みは直す先の持ち主と種別しか見ない）
- kind: conflict
- 処置: fixed D7 仮 — 読み出しの側で、同じ直す先を持つ fix は最後に書いた 1 件だけを版にし、fixed_by もそれを指す（取り込みでは断らない）。単体試験 `place_window_unit_sibling_fixes_keep_only_the_last_written` / `place_view_unit_sibling_fixes_show_as_fixed_by_the_last_written`

## R3. coord_supersedes_is_valid が本文を消去した記録を場所も種別も見ずに直す先として通す
- 成果物: crates/server/src/places.rs
- 根拠: crates/server/src/places.rs:326-347（02193371）。Task 3 の直しで external_ref = place-coord:<place> が残るようになり、根拠の「どの場所か残らない」が崩れた
- kind: technical
- 処置: fixed D4 仮 — raw='' の行は external_ref == coord_marker(place) のときだけ通す。試験 `place_ingest_rejects_fixing_an_erased_record_of_another_place_or_kind`

## R4. places.ts のコメント「サーバは valid_from 順に並べる」が実態（書いた順）と違う
- 成果物: web/src/places.ts
- 根拠: web/src/places.ts:195（02193371）と crates/server/src/places.rs:831
- kind: technical
- 処置: fixed 8.1

## R5. 名前・広さをいまと同じ値のまま「変える」で送れ、「前の名前: <同じ名前>」が出る
- 成果物: web/src/PlaceForms.tsx
- 根拠: web/src/PlaceForms.tsx:376-379（02193371。Task 9 の F2）
- kind: technical
- 処置: fixed 9.2 — 同じ値の間は押せない。vitest 1 本

## R6. MasterView.tsx の末尾に指す先の無い doc コメントが残っている
- 成果物: web/src/MasterView.tsx
- 根拠: web/src/MasterView.tsx 末尾（02193371。control() を controls.ts へ移した残り）
- kind: technical
- 処置: fixed 8.2

## R7. smoke.sh の印が広さを変えていないのに広さの Scenario を名乗り、まとめの echo に ST21 が無い
- 成果物: tools/smoke.sh
- 根拠: tools/smoke.sh:958・:1026（02193371。Task 7 の F1 / F2）
- kind: technical
- 処置: fixed 7.2 — 広さを 100 → 200 に変える段と radius_m == 200 の確認を足し、まとめに ST21 を入れた

## R8. PlacesView.tsx のカードの行が D21 の「1 行に並べる」と違い flexWrap: "wrap"
- 成果物: web/src/PlacesView.tsx
- 根拠: web/src/PlacesView.tsx:150（02193371。Task 10 の F2）
- kind: technical
- 処置: fixed 8.3

## R9. e2e の while (closed.count() > 0) click に上限が無い
- 成果物: web/e2e/places.spec.ts
- 根拠: web/e2e/places.spec.ts:523（02193371。Task 10 の F3）
- kind: technical
- 処置: fixed 10.3 — 上限 50 回にし、最後に toHaveCount(0)

## R10. D16 が「錠の関数とトリガは落とす」とだけ書き、down が器の表が残るとき器の錠を残すことと食い違う
- 成果物: openspec/changes/st21-place-registry/design.md
- 根拠: migrations/202610020030_places.down.sql と design.md の D16（Task 1 の ⚠️）
- kind: technical
- 処置: fixed D16

## R11. check-immutable.sh の「書いた日時」「利用者」の段は印を 1 行に 2 つ並べ、OK の行を出さない（check_scenarios.py が spec に無い Scenario を指す印と warn）
- 成果物: tools/check-immutable.sh
- 根拠: Task 1 の F3。`python3 scripts/check_scenarios.py . st21-place-registry` の warn
- kind: technical
- 処置:

## R12. OpenAPI の POST /places の 400 の enum に unavailable が入り、401 / 500（GET /places* の 401 も）の本文が宣言されていない
- 成果物: docs/openapi.json
- 根拠: crates/server/src/bin/openapi.rs（Task 2 の F1）
- kind: technical
- 処置:

## R13. 範囲外の共有の道具を変えている（server_startup.rs は port の規則の 3 つ目の写し、agent-env.sh は .env の合言葉を全 agent の環境へ export）
- 成果物: crates/server/tests/server_startup.rs
- 根拠: crates/server/tests/server_startup.rs:21-47、tools/agent-env.sh、tools/verify-env.sh
- kind: technical
- 処置:

## R14. 試験の細部が欠けている（合計の大きい順の決着・assign / summarize の境界・401 で行が作られないこと・「自分自身」を直す枝）
- 成果物: crates/server/src/places_tests.rs
- 根拠: Task 6 の F1、Task 5 の F3、Task 2 の F2、Task 3 の F3
- kind: technical
- 処置:

## R15. candidates がまとまりに足すたびに全点の平均を取り直し O(n²)
- 成果物: crates/server/src/places.rs
- 根拠: crates/server/src/places.rs:1031-1037（Task 6 の F3）
- kind: technical
- 処置:

## R16. 連鎖の兄弟（負けた fix をさらに直す fix）で、根の fixed_by が版にならなかった負けの fix を指す（fix wave の re-review が出した新しい Minor）
- 成果物: crates/server/src/places.rs
- 根拠: places.rs の place_out の fixed_by（5ac5b4d）。表示は state=fixed のまま
- kind: technical
- 処置:

## R17. 「名前を変える」は空白だけの名前でも押せる（re-review の範囲外の観察）
- 成果物: web/src/PlaceForms.tsx
- 根拠: `.superpowers/sdd/tasks/final-re-review.md` の Out-of-Scope
- kind: technical
- 処置:
