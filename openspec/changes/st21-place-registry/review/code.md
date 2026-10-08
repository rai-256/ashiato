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

# code-verify（独立検証。HEAD a55de3e7。2026-10-08）

- 席: code-verify（実装者ではない）。**作業ツリーのコードは触っていない**（`git status --short` は最初から最後まで空）
- 壊す検査は `git archive HEAD` の複製 `~/.cache/st21verify/mut`（DB は別の compose `st21verify`・55621、ビルド先も別）でだけ行い、1 本ごとに元へ戻した（戻したことは `cmp` で HEAD と照合）
- PR はまだ無い（`gh pr list --head feat/st21-place-registry` は `[]`）。CI は一度も走っていない
- 証跡（`evidence.jsonl`）の最新の head は全項目 5ac5b4de より前（1.x は 003164c2 … 11.2 は a5c18fd3）。final review の直し（5ac5b4de。places.rs・PlaceForms.tsx・places.ts・e2e）の後に取り直した証跡が無いので、**全項目を HEAD で走らせ直した**

## 申告と実測

申告: tasks **32/32 `[x]`**（Task 1〜11）・人間の確認待ち 0・Scenario 156 本（うち 10 本は ST19 の印）。

| 項目 | 検証コマンド | 実測（HEAD a55de3e7） |
|---|---|---|
| 1.1 | `tools/check-migrations.sh` / `CT place_lock_migration_applies_twice` / `CT place_lock_down_keeps_rows` | 一致（rc=0 / 1 passed / 1 passed） |
| 1.2 / 1.3 | `CT place_container_append_only` / `CT place_lock_` | 一致（1 / 15 passed） |
| 1.4 | `tools/check-immutable.sh` → `grep "OK place"` | 一致（rc=0、`OK place` 14 行）。ただし錠の 7 枝は外しても緑（R18） |
| 2.1 | `CT place_container_endpoint` | 一致（4 passed） |
| 3.1〜3.5 | `CT place_ingest_rejects` / `_stores` / `_first_is_serialized` / `place_sensitivity` / `place_ingest_radius_bounds` | 一致（29 / 8 / 1 / 3 / 1 passed） |
| 4.1〜4.3 | `CT place_view_unit` / `place_window_unit` / `place_view_endpoint` / `place_view_today_is_tokyo` | 一致（11 / 11 / 26 / 1 passed） |
| 5.1〜5.3 | `CT place_match_` / `place_window_` / `place_view_stays` | 一致（8 / 20 / 5 passed） |
| 6.1 | `CT place_candidates_` | 一致（10 passed） |
| 7.1 | `tools/check-openapi.sh` | 一致（rc=0） |
| 7.2 / 7.3 | `tools/smoke.sh` → `OK place id unchanged` / `OK seed places` | 一致（rc=0、両方出る。「縦串 OK（… ST21 の場所まで）」） |
| 8.1〜8.3 | `VT places-model` / `master-view` / `places-tabs` / `places-view` / `places-colors` | 一致（11 / 19 / 3 / 10 / 1 passed）。ただし色の検査は画面の 2 ファイルしか見ていない（R19） |
| 9.1 / 9.2 | `VT places-add` / `VT places-change` | 一致（21 / 20 passed） |
| 10.1〜10.4 | `ET places.spec.ts`（`WEB_PORT=5221`。5180 は他の worktree の画面が使用中） | 一致（32 passed、rc=0） |
| 11.1 | `grep -q st21-place-registry docs/handoff/ST23.md && … ST25.md` | 一致（rc=0） |
| 11.2 | `check_scenarios.py` / `review_triage.py` / `check_chain.py` / `comm … \| wc -l` | **3 つ一致・1 つ不一致**: check_scenarios rc=0（790 件すべて担保、warn は R11 の既出のもの）、check_chain rc=0、comm は 10。**`review_triage.py` は rc=1**（R11〜R17 に処置が無い 7 件。a55de3e7 で写した final review の指摘で、finish が処置を付ける前の状態） |
| 11.3 | `cargo fmt --all --check` / `cargo clippy --workspace --all-targets -- -D warnings` / `cargo test --workspace` / `npm run lint && npm run test && npm run build` / e2e / immutable / migrations / openapi / smoke | 一致（すべて rc=0。workspace 691 passed、vitest 230 passed） |

`cargo test -p ashiato-server place_ -- --list` に出るのは `places_tests::` の 145 本だけ（他の試験に部分一致していない）。

## 手 1: 固定値を独立に再計算する —— 一致

python（`math` で WGS84 の Vincenty の逆問題・`zoneinfo`・`base64`）で、実装とは別の方法で計算した。

| 固定値 | 独立の計算 | テスト |
|---|---|---|
| 北へ 80 / 90 / 110 / 120 / 150 m（`m / 111_320` 度） | 測地線で 79.74 / 89.70 / 109.64 / 119.60 / 149.51 m | 照合と名前の無い所の試験。どれも 100 m の境の同じ側に立つ（近似の誤差は 0.33 %） |
| `Asia/Tokyo` の 2026-04-01 0 時 | `2026-03-31 15:00:00+00:00` | `place_view_today_is_tokyo`（14:59:59Z は予定、15:00Z はいまの座標） |
| 128 bit の乱数を base64url（`=` を除く） | 22 文字 | `NONCE_MIN_CHARS = 22`（`attributes.rs:28`）と `newNonce()` の 16 バイト |
| 24 時間の帯（8:30〜10:00 → 8 時台 30・9 時台 60、23:00〜翌 1:00 → 23 時台 60・0 時台 60） | 手で数えて同じ | `place_view_stays_hours_*` |
| コントラスト比の式（e2e の `luminance` / `ratio`） | WCAG 2.1 の定義（0.03928・0.2126/0.7152/0.0722・(L1+0.05)/(L2+0.05)）と同じ | `places.spec.ts:358-369` |

## 手 2: ガードをわざと壊す

**cargo（`place_` 145 本）で見たもの**（`~/.cache/st21verify/mut.py`）:

| 壊し方（複製で） | 結果 |
|---|---|
| 錠: 書いた日時 / 利用者 / D-01 に入った時刻の検査を外す | 落ちる（`place_lock_rejects_rewriting_the_written_time` / `_the_user`） |
| 門: 台帳の `event_id = NEW.id` を外す / 消去の形の `payload = '{}'` を外す | 落ちる（`…another_records_ledger` / `…a_forged_erasure`） |
| 行の削除の拒否を外す / 付け替えの拒否（他 → `s01-place`）を外す | 落ちる（`…deleting_the_row` / `…reassigning_another_record`） |
| 器: DELETE を通す / TRUNCATE を通す | 落ちる（`place_container_append_only`） |
| 器の口: 別の利用者でも 200 | 落ちる（`…rejects_another_users_id`） |
| `first` の場所の錠（`lock_place`）を外す | 落ちる（`place_ingest_first_is_serialized`） |
| 消去済みの直す先を場所も種別も見ずに通す（R3 の直しを戻す） | 落ちる（`…fixing_an_erased_record_of_another_place_or_kind`） |
| 「自分自身は直せない」（`target != record.id`）を外す | 緑。ただし等価な変異 —— 自分の行はまだ無いので `coord_supersedes_is_valid` が偽を返す。R14 の既出 |
| **錠: 座標の印 `external_ref` / 地域 / 由来 / 識別子 / エンベロープ / 外への付け替え / 端末識別子を外す（7 通り）** | **7 通りとも 145 本緑、`check-immutable.sh` も rc=0**（R18） |

**`tools/check-immutable.sh` で見たもの**（`~/.cache/st21verify/cimut.py`。DB は毎回空から）: 壊さない基準線 rc=0（`OK place` 14 行）。
書いた日時の錠を外す / 利用者の錠を外す / 行の削除の拒否を外す / 器の DELETE を通す / `.down.sql` が行の有無を見ずに器の表を落とす / 門の台帳の照合を外す の 6 通りは**どれも rc=1**（`NG 場所の記録の列が書き換えられた: event_time=…` など）。
R11 の「書いた日時」「利用者」の段は OK の行こそ出さないが、壊せば NG を出して rc=1 になる。

**検査の外側**: 色の静的検査（`places-colors.test.ts`）は `PlacesView.tsx` と `places.ts` しか読まない（R19）。

## 手 3: Scenario と test の突き合わせ

`python3 scripts/check_scenarios.py . st21-place-registry` rc=0（790 件すべて担保あり。この change の 156 本も含む）。
印の先を読んで主張の階層と比べたもの:

- `場所の記録の原文が 1 バイトも変わらずに残る` —— `raw` は `text`（`202609120943_version_and_ledger.sql:25`）で、空白・並び・`35.6800` を含む原文を `assert_eq!(stored.raw, raw)` で比べている。**バイト列の階層で観測している**
- `座標は丸めずに残る` —— 読み出しの `lat` / `lon` を `as_f64()` で比べる。spec の値（6 桁）は f64 で往復するので主張どおり
- `場所の記録の拒否の応答に値が含まれない` —— ハンドラの返す `IngestResult` を直列化した文字列で見ている（HTTP の層は本文を足さない）
- `登録を 2 回押しても場所は 1 つ` / `器の識別子が取られていたら…` —— 1 回目は `route.fetch()` で実サーバに届けて応答だけを捨て、送った本文の一致と `/api/places` の件数を見ている。主張どおり
- `場所の識別子は画面に出ない` —— `/api/places` の全識別子が `innerText` に無いこと。識別子は `name` 属性（`radius-<id>` など）には入るが、画面の文字ではないので主張の外
- `場所の画面は外へ求めを送らない` —— 宛先の origin を全部集めて画面の origin 1 つと比べる。主張どおり（「移った」の欄は開いていない）
- `名前が空だと断られて入力が残る` —— 空白だけの名前は「登録する」を押せないので、断る応答は差し替えで作っている。spec の WHEN は「断られる応答が返る」なので合っている
- `場所の画面は確定した色だけを使う` —— **主張は「場所の画面のコード」だが、試験は画面を描く 5 ファイルのうち 2 つしか集めていない**（R19）

## 手 4: 本人の決定が test で固定されているか

複製で 1 つずつ書き換え、テストを走らせて元に戻した。

| 決定 | 書き換え | 結果 |
|---|---|---|
| Q3: 場所の記録の既定の感度は外部 AI に出してよい（`places::DEFAULT_SENSITIVITY = 1`） | 2 にする | 落ちる（`place_sensitivity_constant_is_pinned` ほか 1 本） |
| 同上の「分岐として明示する」（D11（仮）） | `default_sensitivity` の場所の枝を外す | 緑（3 本とも）。一般の既定も 1 なので観測できない。値は上で固定されているので指摘にしない |
| Q4 / FR-48 ★: 広さの既定 100 m（`PLACE_DEFAULT_RADIUS_M`） | 150 にする | 落ちる（7 本） |
| C9: 名前の無い所をまとめる 100 m（`CANDIDATE_RADIUS_M`） | 120 にする | 落ちる（2 本） |
| D4（仮）: 広さの範囲 10〜5,000 m | 上限 6,000 / 下限 1 | 落ちる（2 本 / 1 本） |
| Q1: 名前の無い居た所は上位 10 件 ＋「残り N か所」（`CANDIDATE_TOP`） | 8 にする | 落ちる（vitest 3 本。e2e も `toHaveCount(10)`） |
| Q1: 広さの選択肢 50 / 100 / 200 / 300（`RADIUS_CHOICES`） | 300 → 500 | 落ちる（vitest 5 本） |
| Q1: 広さの最初は 100 m（`DEFAULT_RADIUS_M`） | 200 にする | 落ちる（vitest 2 本） |
| Q1: 最近居た順 / 居た所から選ぶだけ / 前の値は押したときだけ / Q2: 直すか移ったかを毎回聞く | （画面の構造） | e2e の印（`場所のカードは最近居た順に並ぶ` は `/api/places` の順と突き合わせ、`名前を付けるフォームに緯度経度の欄が無い`、`前の名前と座標は押したときだけ出る`、`座標を変えるには直すか移ったかを選ぶ`）が持つ |

値を変えても全部通る決定は無かった。

## 手 5: tasks の `[x]` と実体

32 件すべてで、本文の検証コマンドが実在し HEAD で rc=0（表）。`CT` / `VT` / `ET` の絞り込みはどれも 1 本以上に当たる。
不一致は 11.2 の `review_triage.py` だけで、原因は a55de3e7 で写した R11〜R17 に処置がまだ無いこと（finish の工程が付ける）。
証跡は 5ac5b4de より前の head でしか取られていないので、Story gate の前に取り直しが要る（上の実測では全部通る）。

## 手 6: 隙間

- **権限の拒否**: 入力中に印が切れると、登録・変えるのフォームの入力ごと消える（R20）
- **件数が合わない**: 取り込みの結果が送った件数より少なくても「受理」と読み、フォームを閉じる（R21）
- **時計が戻る**: 画面の時計が遅れた端末から後で変えた名前・広さ・補足が、いまの値にならない（R22）
- 確かめて問題が無かったもの: 応答が読めない（R1 の直しで同じ器・同じ原文を送り直す。e2e で実サーバに届けて確かめている）/ 器だけが残る（読み出しに出ず、照合の的にもならない —— `places_and_targets` が名前と座標を持つ器だけを的にする）/
  場所を消す口（C13）は `docs/handoff/ST23.md:74` に、S-2 の見出しは `docs/handoff/ST25.md:6` に置かれている

## R18. 場所の記録の即時の錠の 10 枝のうち 7 枝は、外しても cargo 145 本と check-immutable.sh が緑（座標の印 `external_ref` と外への付け替えを含む）
- 成果物: migrations/202610020030_places.sql / crates/server/src/places_tests.rs / tools/check-immutable.sh
- 根拠: 複製で `202610020030_places.sql:65`（論理ソースを外へ）・`:68`（由来）・`:71`（識別子）・`:80`（地域）・`:88`（`external_ref`）・`:91`（端末識別子）・`:94`（エンベロープ）の検査を 1 つずつ外す → `cargo test -p ashiato-server place_` は 7 通りとも `ok. 145 passed`、`tools/check-immutable.sh` も 7 通りとも rc=0（`~/.cache/st21verify/mut2.log`）。固定されているのは書いた日時・利用者・D-01 に入った時刻の 3 枝だけ。
  `external_ref` は D4（仮）が「消去の後も残り、錠が凍結する」と書いた根拠そのもの（`has_coord_record` と、R3 で直した `coord_supersedes_is_valid` が読む）。外への付け替えは「別のソースへ移してから行ごと消す」を止める唯一の枝
- kind: technical
- 提案: `place_lock_` に列ごとの UPDATE が拒まれることを 1 列 1 本で見る試験を足す（`external_ref` と `logical_source` は `check-immutable.sh` の段にも）

## R19. 「場所の画面は確定した色だけを使う」の検査は画面を描く 5 ファイルのうち 2 つしか見ず、帯の部品に `#ff0000` を書いても緑
- 成果物: web/src/__tests__/places-colors.test.ts / web/src/PlaceBand.tsx / web/src/PlaceForms.tsx / web/src/controls.ts
- 根拠: `places-colors.test.ts:4-5,11` が読むのは `PlacesView.tsx` と `places.ts` だけ。複製で `PlaceBand.tsx` の枠を `border: "1px solid #ff0000"` にして `npx vitest run places-colors` → `Tests 1 passed`。
  24 区分の帯（`PlaceBand.tsx:25-26`）とフォームの面（`PlaceForms.tsx:199,268,319`）は検査の外。なお名前付き色の正規表現は `PlaceForms.tsx:154` の `border: "none"` に当たるので、そのまま対象を広げると誤検出する
- kind: technical
- 提案: 対象を場所の画面の全ファイル（`Place*.tsx`・`controls.ts`）にし、`none` を名前付き色から外す

## R20. 入力中に印が切れる（401）と、名前を付ける・変えるのフォームが入力ごと消える（D13 は「届かなかった・入力はそのまま残っています」と書いている）
- 成果物: web/src/session.tsx / web/src/PlaceForms.tsx / web/src/places.ts / openspec/changes/st21-place-registry/design.md
- 根拠: 複製の vitest で `<Gate><PlacesView/></Gate>` を描き、名前「実家」・補足を入れて「登録する」→ 器の口が 401 → `after401: form= 消えた problem= (無い) body has 実家= false`。
  `session.tsx:24-25,40` は `/api/` のどの 401 でも中身を外して入力欄へ替える。`places.ts:332` と D13 は 401 を「届かなかった」として入力を残す前提で、`places-add.test.tsx:334-341` の 401 の試験は `Gate` 無しで描いているので、実際の組み立てで起きることを見ていない。
  打った名前・補足は記録になる前に捨てられ、戻す元が無い。押し直し用の器の識別子（`last.current`）も消えるので、401 が取り込みの段で起きたときは前の器が名前の無いまま残る
- kind: daily
- 提案: 入力中のフォームがある間は 401 で中身を外さない（ログインを上に重ねる）か、D13 の 401 の行を実際の振る舞いに書き直して本人に見せる。どちらかを選んで `Gate` を含めた試験で固定する

## R21. 取り込みの結果が送った件数より少なくても「受理」と読み、フォームを閉じて入力を捨てる
- 成果物: web/src/places.ts
- 根拠: `places.ts:340-342` は「空なら断られた・1 件でも未受理なら断られた・それ以外は受理」で、件数を見ない。複製の vitest で 4 件送ったことにして 1 件分の結果（受理）だけを返すと `outcome= {"at":"accepted"}`。
  受理でフォームが閉じる（D13）ので、返らなかった 3 件（名前・広さ・補足のどれか）は送れていないかもしれないまま入力が消える。サーバは 1 件ごとに 1 結果を返す契約（`lib.rs` の `IngestResult` の doc「送った順に並ぶ」）なので、件数が違うのは途中の経路が本文を切ったときだけ
- kind: technical
- 提案: 結果の件数が送った件数と違えば「届かなかった」にして入力を残す（同じ原文の送り直しは冪等）。vitest 1 本

## R22. 名前・広さ・補足のいまの値は画面の時計の「書いた日時」の順で決まり、時計が遅れた端末から後で変えた名前は「前の名前」に回る
- 成果物: crates/server/src/places.rs / web/src/PlaceForms.tsx / openspec/changes/st21-place-registry/design.md
- 根拠: 複製で「職場」（書いた日時 2026-09-01 09:00）を登録した後、1 日遅れた時計で「本社」（書いた日時 2026-08-31 09:00）を送る試験を足して走らせる → `ZZ name="職場" previous=["本社"]`。
  書いた日時は画面の `new Date()`（`PlaceForms.tsx:255,379`）で、並びは `order_key()` = 書いた日時 → D-01 に入った時刻（`places.rs:795`）。本人には「受理」と出てフォームが閉じ、カードの名前は変わらない。
  時計のずれを測る ST05 は C-01 / C-02 だけを対象にし（`docs/stories/ST05.md` の FR-7）、V-01 の時計を測る Story は無い。D-01 に入った時刻は残っているので、並べ直せば戻る（失うものは無い）
- kind: daily
- 提案: D6（仮）の反転条件に「端末の時計が遅れている」を足し、本人に聞くまでの既定を決める（例: 書いた日時が今の値より前の記録を受理したら画面がそれを出す、または同じ項目の並びを D-01 に入った時刻にする）

## 実行したコマンド（抜粋）

```bash
source tools/agent-env.sh && bash tools/verify-env.sh
cargo test -p ashiato-server place_                         # 145 passed
for f in <tasks の CT 18 本>; do cargo test -q -p ashiato-server $f; done
tools/check-migrations.sh; tools/check-immutable.sh; tools/check-openapi.sh; tools/smoke.sh
python3 scripts/check_scenarios.py . st21-place-registry; python3 scripts/review_triage.py . st21-place-registry; python3 scripts/check_chain.py .
cd web && npm run lint && npm run test && npm run build && WEB_PORT=5221 npx playwright test places.spec.ts
cargo fmt --all --check; cargo clippy --workspace --all-targets -- -D warnings; cargo test --workspace
python3 ~/.cache/st21verify/{mut.py,mut2.py,cimut.py,webmut.py}   # 複製での変異（DB 55621）
```
