# ST02 実装の独立検証（code-verify）

対象: `feat/st02-collection-coverage` / 作業ツリー `/home/yosis/dev/ashiato2-st02`
やり方: 申告されたコマンドを全部走らせ直し、**固定値を別実装で計算し直し、守るものを潰して落ちるか**を見た。

> **注記（再現性）**: 検証中、作業ツリーのファイルが**外部から 2 度書き換えられた**
> （`web/src/CoverageGrid.tsx` のセル色、`crates/server/src/coverage.rs` の `DAY_TZ` と
> `decide()` の評価順）。いずれも短時間で戻された。下の「実測」は
> `git status` が clean であることを確認した状態での値。読み取れた 1 件だけ記す ——
> `decide()` の①を②③④より前に上げる改変（第 5 回 Q19 の逆）は
> `coverage::tests::span_outranks_records` が落として捕まえた。
> 以降の改変実験は **HEAD の複製**（`git archive HEAD`）に対して行っており、作業ツリーは触っていない。

## 申告と実測

| 申告 | 実測したコマンド | 結果 |
|---|---|---|
| tasks 61/61 が `[x]` | `grep -c '^- \[x\]'` / `'^- \[ \]'` | 61 / 0 — **一致** |
| `cargo fmt --all --check` | 同左 | rc=0 — 一致 |
| `cargo clippy --workspace --all-targets -- -D warnings` | 同左 | rc=0 — 一致 |
| `cargo test --workspace` 71 件 | 同左 | rc=0 / `71 passed` — 一致 |
| `./tools/check-boundaries.sh` | 同左 | rc=0 — 一致 |
| `./tools/check-licenses.sh` | 同左 | rc=0（Rust 240 / Node 254 / Android 対象外）— 一致。ただし **R8** |
| `./tools/check-migrations.sh` | 同左 | rc=0 — 一致 |
| `./tools/check-openapi.sh` | 同左 | rc=0 — 一致 |
| `./tools/smoke.sh` | 同左 | **不一致** —— `cargo test` の直後に走らせると rc≠0（**R14**）。`docker compose down -v` の直後なら rc=0 |
| `./tools/check-panic-log.sh` | 同左 | rc=0 — 一致 |
| `./tools/check-immutable.sh` | 同左 | rc=0 — 一致 |
| web `npx tsc -b` / `lint` / `test` / `build` 26 件 | 同左 | 全部 rc=0 / `Tests 26 passed` — 一致 |
| android `assembleDebug` + `testDebugUnitTest` 95 件 | 同左 | rc=0 / test-results の `tests=` 合計 95 — 一致 |
| `check_scenarios.py` 98 / 97 担保 / 1 待ち | 同左 | `Scenario 98 件 / 印 109 個 / 担保あり 97 / 人間の確認待ち 1` — 一致 |

**捏造も空テストも無い。** 71 / 26 / 95 はすべて実在し、実際に走る。ずれは下の 15 件。

## 手 1: 固定値を独立に再計算する — **一致**

| 固定値 | 別実装で計算し直した値 | 判定 |
|---|---|---|
| 分母 365 の線 346.75 | `365*0.95 = 346.75`（python） | 一致 |
| 分母 200 の線 190 | `200*0.95 = 190.0` | 一致 |
| 位置の分母 90（04-01〜06-29） | `(date(2026,6,29)-date(2026,4,1)).days+1 = 90` | 一致 |
| ブラウザ履歴の分母 0（07-01 開始・06-30 に引く） | `last_counted=06-29 < start` → (0,0) | 一致 |
| 300 日目の残り 65 日 | `365-300 = 65` | 一致 |
| 遡り後の分母 38（02-01〜03-10） | `(date(2026,3,10)-date(2026,2,1)).days+1 = 38` | 一致 |
| 格子 3 段の隣接比（HSL 132/30、L=9/40/92） | python で WCAG 2.2 の式から再計算: **3.8923** と **3.8082**（8bit 量子化ありでも 3.9034 / 3.7960） | **一致**（3:1 以上） |
| `ingest::hash_is_pinned` の sha256 | `python3 -c 'hashlib...'` で `39d0ebc5…5df1` を再現 | 一致 |
| `hash_follows_text_not_structure` の sha256 | 同上で `738cb0ca…accf` を再現 | 一致 |

`state-contrast.test.ts` は**描くのと同じ定数**から計算しており、UIR-13 型の「測る色と描く色がずれる」
穴は **定数の側では**塞がっている（描く側は **R3**）。

## 手 2: ガードをわざと壊す

| 潰したもの | 何を走らせたか | 結果 |
|---|---|---|
| `0006` の `CREATE TRIGGER heartbeat_immutable` を削る | `./tools/check-immutable.sh`（複製） | **rc=1** / 「生存信号の raw が書き換えられた」ほか 12 件 NG — **本物の検査** |
| `0005` の `CONSTRAINT coverage_span_range CHECK` を削る | `cargo test --workspace`（複製・DB 作り直し） | `span_rejects_reversed_range` FAILED（70 passed / 1 failed）— **本物** |
| `DAY_TZ` を `"UTC"` にする | `cargo test --workspace`（複製） | 9 件 FAILED — **本物** |
| `check-licenses.sh` の Node 側を**対象ゼロ**にする | 同じ python ブロックを `web/node_modules` の無いディレクトリで実行 | `0 件を確認 / 不許可 0 件` で **rc=0** — **R8** |

`capturable=false` で `blockers` 空の拒否 / 成功 > 試行の拒否は、`heartbeat.rs` の単体検査が
`assert_eq!(r.validate(), Err(...))` で直接主張しているので、戻り値を `Ok` にすれば必ず落ちる。
冪等索引は `heartbeat_idempotent` が `count(*) = 1` と `duplicate == true` の両方を見ている。

---

## R1. 端末の時計が戻った信号 1 件で、収集開始日が恒久的に汚染され、NFR-13 の合否が「確定・未達」に落ちる

- 成果物: `crates/server/src/coverage.rs`（`touch_started_on`） / `crates/server/src/heartbeat.rs`（`validate`） / `crates/server/src/lib.rs`（`heartbeat_one` / `ingest_one`）
- 根拠: サーバを立てて `POST /heartbeat` を 2 件送った実測 ——

  ```
  正常な信号（emitted_at=2026-04-01T03:00:00Z）
    accepted=true / collection_started_on=2026-04-01
  端末の時計が 27 年戻った信号（emitted_at=1999-01-01T03:00:00Z）
    accepted=true / collection_started_on=1999-01-01
  GET /coverage/achievement の c01-location:
    {"collection_started_on":"1999-01-01","denominator":365,"achieved_days":0,
     "window_closed":true,"window_closes_on":"2000-01-01"}
  正しい日（2026-04-02）の信号をもう一度送っても
    collection_started_on=1999-01-01
  ```

  `heartbeat.rs:validate()` は `emitted_at` に何の範囲も持たない（見るのは `raw` / `blockers` / 回数の 3 つだけ）。
  `coverage.rs:touch_started_on` は `collection_started_on > 新しい日` のときだけ更新するので
  **前にしか動かない**。前へ動かす経路はあり、戻す経路がコードにもマイグレーションにも無い。
  結果として **窓が過去に閉じ**、`window_closed=true` → `confirmed=true` になり、
  成功条件 1 の合否が「**確定・未達**」として固まる。specs は
  「判定式は 1 年の計測が始まったら変えられない」と書いており、引き直せない。
  `docs/stories/ST05.md` を読んだ: FR-7 は**ずれを測定記録として残す**だけで、
  収集開始日の補正も入力の棄却も持っていない（`satisfies: [FR-7]` / 完了の判定 2 行）。**ST05 に落ちていない。**
- kind: irreversible
- 処置: escalated
- 提案: 受け口で `emitted_at` / `event_time` の範囲を切る（例: 受信時刻より未来、または登録簿に行が
  できた日より前を断る）か、`collection_started_on` を**行に持たず** `min()` で毎回引き直す形にして
  誤った 1 件を消せば戻るようにする。どちらを採るかは第 7 回 Q26 の答えに触るので本人へ返す。

## R2. 記録の格納と稼働記録の加算が同じトランザクションに無く、加算が落ちるとその日の稼働記録は二度と戻らない

- 成果物: `crates/server/src/lib.rs:250-290`（`core.event` の INSERT → `core.coverage` の UPSERT → `touch_started_on` が 3 本の独立した文）
- 根拠: 「加算だけが落ちた」状態を作って再送した実測 ——

  ```
  1 回目: {"duplicate":false,"accepted":true} / event_count=1
  加算だけが落ちた状態を作る → core.coverage の行数=0 / core.event の件数=1
  再送:   {"duplicate":true,"accepted":true}
  再送後の event_count=0 / core.event=1
  ```

  記録は `core.event` に残っているのに、稼働記録の**行ごと**消えたまま戻らない。
  冪等キーが再送を `duplicate` にするので、収集側は正常に完了したと見て未送信から取り除く。
  specs は「重複だけが届いた日についても、稼働していたことは記録する」と定めているが、
  この経路ではその行が立たない。端末が主語のソースではその日が達成日から永久に抜ける。
  引き金は異常な事象に限らない —— UPSERT が一過性に失敗すれば `internal()` で 500 になり、
  収集側が再送して同じ状態になる。0005 の再構築は「古い形のときだけ」走るので、引き直す経路も無い。
- kind: technical
- 処置: fixed 11.1
- 備考: ★ kind を `irreversible` から `technical` へ変えた。**欠陥の帰結は不可逆だが、直し方に人間が選ぶ分岐が無い** —— 3 本を 1 トランザクションにまとめるのは FR-33 と specs から一意に決まる。捨てるものも要件の矛盾も日常への影響も無い。
- 提案: 3 本を 1 トランザクションにまとめる。あるいは `core.event` から
  `core.coverage` を引き直す経路（版に依らない再集計）を 1 本持つ。

## R3. 格子のセルに塗る色が状態と結びついているかを見る検査が無い —— 全セルを同じ色で塗っても 26/26 緑

- 成果物: `web/src/CoverageGrid.tsx:128` / `web/src/__tests__/state-contrast.test.ts`
- 根拠: HEAD の複製で `tone(BAND[bandOf(cell.state)])` を `tone(BAND.recorded)` に変えて
  `npx vitest run` → `Tests 26 passed (26)`。3 段の区別が画面から消えても検査は全部緑。
  `state-contrast.test.ts` が見るのは `BAND` という**定数**の相対輝度だけで、
  `week-select.test.tsx` が見るのは `data-band` **属性**だけ。
  **`background` に実際に入る値を読む検査がどこにも無い。**
  これは同じファイルの冒頭が「UIR-13 / UIR-38 —— 測る色と実際に描く色がずれていて
  全状態の 23.4% が 4.5:1 未満だった」と名指しした型そのもの。
  （検証中に外部が同じ改変を作業ツリーに入れたので、実害の型も一致している。）
- kind: technical
- 処置: fixed 11.2
- 提案: `WeekRow` が描いた `style.background` を読み、`tone(BAND[bandOf(state)])` と
  文字列で一致することを 3 段ぶん確かめる検査を 1 本足す。

## R4. NFR-19 の 24 CSS px が定数の自己参照で固定されていない —— 8 px にしても 26/26 緑

- 成果物: `web/src/tokens.ts:55`（`MIN_TARGET_PX`） / `web/src/__tests__/target-size.test.tsx` / `initial-viewport.test.tsx`
- 根拠: 複製で `MIN_TARGET_PX = 24` → `8` に変えて `npx vitest run` → `Tests 26 passed (26)`。
  `target-size.test.tsx` は `parseFloat(style.minHeight) >= MIN_TARGET_PX` と**同じ定数**を比べており、
  値を動かすと期待値も一緒に動く。`initial-viewport.test.tsx` の
  `rows * MIN_TARGET_PX <= 700` も同じ理由で緩む方向にしか動かない。
  `initial-viewport.test.tsx` は `INITIAL_WEEKS` については
  「**定数と突き合わせない**」と明記して `>=4 && <=6` を書いている（実際 `INITIAL_WEEKS=53` は捕まえる）。
  同じ配慮が `MIN_TARGET_PX` には入っていない。
- kind: technical
- 処置: fixed 11.2
- 提案: `target-size.test.tsx` の比較対象を**リテラルの 24** にする（NFR-19 の数値は要件側の値で、
  トークンの都合で動いてよい値ではない）。

## R5. 「1 年ぶん = 53 週」が固定されていない —— 20 週にしても 26/26 緑

- 成果物: `web/src/tokens.ts:61`（`YEAR_WEEKS`） / `web/src/__tests__/expand-to-year.test.tsx`
- 根拠: 複製で `YEAR_WEEKS = 53` → `20` に変えて `npx vitest run` → `Tests 26 passed (26)`。
  検査は `expect(Number(grid.getAttribute("data-weeks"))).toBe(YEAR_WEEKS)` と
  `toHaveLength(YEAR_WEEKS)` で、**期待値が実装と同じ定数**。
  specs の Scenario「そのソースの 1 年ぶん（**53 週**）が読める」は観測されていない。
- kind: technical
- 処置: fixed 11.2
- 提案: `toBe(53)` とリテラルで書く。仕込みのデータ（`53 * 7` 日）側は既にリテラル。

## R6. 第 4 回 Q8 の「主語の割り当て」が固定されていない —— 位置と写真を入れ替えても 71 件すべて緑

- 成果物: `crates/server/src/coverage.rs:28-30`（`DEVICE_SUBJECT` / `USAGE_SUBJECT`） / `crates/server/src/coverage/tests.rs:952`（`must_sources_are_the_five_of_nfr13`）
- 根拠: HEAD の複製で

  ```
  DEVICE_SUBJECT = ["c01-photo", "c01-app-usage"]
  USAGE_SUBJECT  = ["c01-location", "c02-window", "c02-browser-history"]
  ```

  に入れ替えて `cargo test --workspace` → `test result: ok. 71 passed; 0 failed`。
  `must_sources_are_the_five_of_nfr13` が見るのは
  「Device が 2 本 / Usage が 3 本」という**個数だけ**で、どのソースがどちらかを見ていない。
  `expected_gap_seeded` も名前の集合しか見ない。`tools/smoke.sh` の 27 も
  `sources|length == 5` と `has("achieved_days")` までしか見ない。
  この入れ替えは本人の答え（第 4 回 Q8、`conflict / irreversible` と分類された問い）を
  そのまま裏返すもので、specs が「写真を記録の有無で数えると**正常動作時から未達で固定される**」と
  書いた当の状態を作る。
- kind: technical
- 処置: fixed 11.1
- 備考: ★ kind を `conflict` から `technical` へ変えた。**本人の答え（第 4 回 Q8）を変えるのではなく、それを守る検査を足す**だけなので、人間が選ぶ分岐が無い。
- 提案: `must_sources()` の返り値を **(名前, 主語) の組で丸ごと**期待値と突き合わせる
  （5 行のリテラル）。個数では第 4 回 Q8 を守れない。

## R7. 「分母から除くのは丸ごと覆う**停止だけ**」が固定されていない —— 破棄も除くように変えても 71 件緑

- 成果物: `crates/server/src/coverage.rs:454`（`let live = f.iter().filter(|d| !d.stopped_full);`） / `crates/server/src/coverage/tests.rs:618`（`achievement_denominator_full_day_stop_only`）
- 根拠: 複製で `!d.stopped_full` → `!d.stopped_full && !d.dropped_full` に変えて
  `cargo test --workspace` → `test result: ok. 71 passed; 0 failed`。
  `achievement_denominator_full_day_stop_only` は停止しか置いておらず、**破棄の日を 1 つも作っていない**。
  specs は「分母から、FR-34 で記録された意図的な停止のうち 1 日を丸ごと覆うもの**だけ**を除く」で、
  破棄を除くと分母が縮んで達成が近づく（深掘り Q3 / 第 5 回 Q18 の「割合にした理由」に直接触る）。
- kind: technical
- 処置: fixed 11.1
- 提案: 同じ検査に「1 日を丸ごと覆う**破棄**の日は分母に**残る**」を 1 行足す。

## R8. CI の rust job に `npm ci` が無く、Node 依存のライセンス検査が「0 件を確認」で緑になる

- 成果物: `.github/workflows/ci.yml`（`rust` job） / `tools/check-licenses.sh`（Node の節）
- 根拠: `ci.yml` の `rust` job の steps は checkout → rust-toolchain → fmt → clippy → test →
  `check-licenses.sh` → … で、**setup-node も `npm ci` も無い**（`web` job は別ジョブで、
  そちらは `check-licenses.sh` を走らせない）。`web/node_modules` が無い状態で
  同じ python ブロックを走らせた実測:

  ```
    0 件を確認 / 不許可 0 件
  NODE_BLOCK_RC=0
  ```

  ローカル（node_modules あり）では `254 件を確認` になるので、**CI の緑とローカルの緑が別物**。
  スクリプト自身が Android について「黙っていると、緑が『確認済み』に読まれる」と書いて
  明示的に対象外と刷っているのに、Node は同じ穴が無言で開いている。
- kind: technical
- 処置: fixed 11.4
- 提案: `check-licenses.sh` の Node の節で `web/node_modules` が無ければ **NG で落とす**
  （「対象が無い」と「不許可が無い」を混ぜない）。あわせて CI の該当ジョブに `npm ci` を入れるか、
  ライセンス検査を `web` job へ移す。

## R9. 「区間の取得率」が日の合計に畳まれて返る —— Scenario の主語とテストの主語が違う

- 成果物: `crates/server/src/coverage.rs`（`facts()` の `sum(attempts) ... GROUP BY 1`） / `crates/server/src/coverage/tests.rs:340`（`attempt_ratio_is_carried_by_heartbeat`）
- 根拠: 同じ日に生存信号を 2 件（10/3 と 20/6）送って `GET /coverage` を引いた実測 ——

  ```
  {"attempts":30,"successes":9}
  ```

  返るのは**日の合計**で、区間ごとの取得率は取り出せない。
  specs の Scenario は「前回の信号からの間に取得を 360 回試みて 230 回成功したことを示す
  生存信号が届く → **その区間の**試行回数と成功回数が返る」。
  印の付いた検査は **1 日に生存信号を 1 件しか置いていない**ので、
  「日」と「区間」が一致してしまい、この違いを観測できない。
  想定間隔 6 時間なら 1 日 4 区間になるので、本番では常に畳まれる。
  第 5 回 Q17 が Q17 を入れた理由（「取得率が低い＝眠っていた」を区間で見分ける）が、
  日に畳んだ時点で薄まる —— ST01 の R46 が渡した宿題の答えが日単位に丸まっている。
- kind: technical
- 処置: fixed 11.1
- 備考: spec の逐語（「その区間の」）に実装を合わせた。**要件の側は動かさない** —— 第 5 回 Q17 の意図（区間で見分ける）がそのまま実装できるので、本人へ返す必要が無かった。
- 提案: `DayCell` に区間の列（`[{emitted_at, attempts, successes}]`）を返すか、
  Scenario の逐語を「その日の合計」に直す。**どちらにするかは第 5 回 Q17 の意図に触るので本人へ返す**。

## R10. 画面が UTC で日を切って API に渡すので、JST の 0〜9 時は直近の 1 日が格子に出ない（App.tsx にテストが 1 本も無い）

- 成果物: `web/src/App.tsx:8-14`（`yearRange`） / `web/src/__tests__/`（`App` の検査が無い）
- 根拠: `yearRange` は `today.toISOString().slice(0, 10)` を使う。node で実測 ——

  ```
  JST の今日: 2026-03-02
  yearRange: {"from":"2025-02-24","to":"2026-03-01"}
  ```

  サーバは `Asia/Tokyo` で日を切る（深掘り Q2）のに、画面は UTC の日を渡す。
  毎日 00:00〜09:00 JST のあいだ、格子の一番上に出る週に**今日が入らない**。
  `ls web/src/__tests__` に `App` の検査は無く、`from` / `to` の作り方を見るものが 1 本も無い。
  Scenario「直近の週が一番上にある」は `CoverageGrid` 単体（`days` を渡す側）でしか確かめていない。
- kind: technical
- 処置: fixed 11.2
- 提案: `yearRange` を `Intl.DateTimeFormat(..., { timeZone: "Asia/Tokyo" })` か
  `toLocaleDateString("sv-SE", { timeZone: "Asia/Tokyo" })` で組み、
  固定の UTC 時刻（例 `2026-03-01T23:00:00Z`）を渡して `to` が `2026-03-02` になる検査を 1 本置く。
  タイムゾーンの名前は `tokens.ts` ではなく**サーバの `DAY_TZ` と同じ 1 か所**から来ることを担保する。

## R11. FR-34（停止）と FR-9（破棄）は ST02 に書き手が無い —— Scenario はテストが自分で行を入れて緑になっている

- 成果物: `crates/server/src/coverage/tests.rs:396`（`stop_is_stored_as_time_range`） / `:431`（`drop_is_stored_with_count`） / `crates/server/src/testdb.rs`（`put_span`）
- 根拠: `grep -rn "coverage_span" crates/server/src/lib.rs` は 0 件 —— **API にも収集側にも
  `core.coverage_span` へ書く経路が無い**。2 つの検査はどちらも `testdb::put_span`（テスト用の直 INSERT）で
  自分で行を置き、その行が読めることを確かめている。
  specs の Requirement は「**WHEN 利用者が収集を停止する** THE SYSTEM SHALL …残す」
  「**IF 保持の上限を超えて記録を破棄する** THEN …残す」で、条件節にあたる契機が ST02 に無い。
  `check_scenarios.py` は印しか見ないので 2 件とも「担保あり」に数えられている。
  他 Story を確認した: `docs/stories/ST15.md:43`「止めた期間が稼働記録の画面で
  『意図的な停止』として区別されて出る」、`docs/stories/ST04.md:45`「上限を小さくして
  意図的に溢れさせると、破棄された期間と件数が ST02 の画面に出る」—— **どちらも持っている。隙間には落ちていない。**
- kind: defer
- 処置: deferred ST15
- 備考: ST15（停止の入力）と ST04（破棄）が書き手を持つことを `docs/stories/` で確認した。specs の 2 つの Requirement は「表がその形で持てる」ことを ST02 が満たしており、契機は他 Story にある。tasks に「他 Story が書き手を持つ Scenario」の節を足した。
- 提案: 処置は不要（ST15 / ST04 が持つ）。ただし specs 側の 2 つの Requirement を
  「表がその形で持てる」に書き直すか、tasks の「人間の確認待ち」ではなく
  **「他 Story が書き手を持つ」欄**を作って 2 件を移し、緑の意味を誤読させないようにする。

## R12. 生存信号の冪等キーに固定値の検査が無い（記録側は `hash_is_pinned` で固定してある）

- 成果物: `crates/server/src/heartbeat.rs:70`（`content_hash`） / 同 `:128`（`content_hash_ignores_collector_id`）
- 根拠: `grep -rn "[0-9a-f]\{40,\}" crates/` で出る固定値は `ingest.rs:193` と `ingest.rs:301` の 2 つだけ。
  生存信号側の検査は「id / device_id を変えても同じ鍵」「raw を変えれば別の鍵」の**不変性**だけで、
  鍵そのものを止めていない。`ingest.rs` の同じ検査には
  「**冪等キーの作り方が変わっている。保存済みの記録が全部ずれる**」という理由が書いてある。
  生存信号にも `heartbeat_dedup` の一意索引が同じ役目で載っており、
  作り方が変われば**保存済みの信号の再送が全部新しい行になる**（specs「同じ生存信号が
  複数回届いたとき、行を 1 つだけ残す」が壊れる）。ST01 と同じ危険が同じ形であるのに、同じ手当てが無い。
- kind: technical
- 処置: fixed 11.1
- 提案: `ingest.rs:190` と同じ形で、python の 3 行を docstring に添えて sha256 を 1 つ固定する。
  （検証で `hashlib` から `39d0ebc5…` と `738cb0ca…` の 2 つはどちらも再現できているので、同じ手が使える。）

## R13. 生存信号の冪等索引が `(logical_source, content_hash)` で、利用者識別子を含まない

- 成果物: `migrations/202609111111_coverage_rebuild.sql`（`CREATE UNIQUE INDEX heartbeat_dedup`） / `crates/server/src/lib.rs`（`ON CONFLICT (logical_source, content_hash) DO NOTHING`） / `crates/server/src/heartbeat.rs:70`
- 根拠: 索引の列に `user_id` が無く、`content_hash` も `logical_source` + `emitted_at` + `raw` の
  3 つからしか作られない（`user_id` も `device_id` も混ぜていない）。
  別の利用者が同じソース・同じ発信時刻・同じ原文を送ると、後の 1 件は
  `accepted: true, duplicate: true` として**黙って落ちる**（呼び出し側からは成功に見える）。
  specs はこの表に「利用者識別子を持たせる」（FR-29 / 扉 #9、**day one から持つ**）と定めており、
  列は持っているのに一意性が利用者をまたいでいる。
  `core.event` 側の索引は未確認だが、ST02 で新設したのはこの索引。
- kind: technical
- 処置: fixed 11.1
- 提案: 索引を `(user_id, logical_source, content_hash)` にし、`ON CONFLICT` も揃える。
  単一利用者のうちは挙動が変わらないので、**いま直すのがいちばん安い**（0005 と同じ理屈）。

## R14. `smoke.sh` は DB に既存データがあると落ちる —— `cargo test` の後に走らせると rc≠0

- 成果物: `tools/smoke.sh:53-56`
- 根拠: `cargo test --workspace`（本物の DB を使う）を走らせた直後に `./tools/smoke.sh` を実行した実測 ——

  ```
  == 6. 取り出す（論理削除を効かせたビュー越し）
     → 315 件
  1 件のはずが 315 件
  ```

  `docker compose down -v` を挟むと rc=0（`縦串 OK（実データ経路と稼働状況まで）`）。
  終了時の trap で `down -v` するので 2 回連続なら通り、CI（毎回まっさらなランナー）でも通るが、
  **申告の「rc=0」は前提を書かないと再現しない**。`tools/check-immutable.sh` も同じ前提を持つ。
- kind: technical
- 処置: fixed 11.4
- 提案: `smoke.sh` の冒頭に `docker compose down -v` を 1 行入れる（末尾と対にする）か、
  手順 6 の件数を「自分が入れた 1 件が見えること」に変える。

## R15. 「開いた直後に 5 ソースの直近 4 週が同時に見える」が、勘定の上でも成り立っていない

- 成果物: `web/src/__tests__/initial-viewport.test.tsx:57-62` / `web/src/CoverageGrid.tsx` / `web/src/App.tsx`
- 根拠: 検査が固定しているのは
  `FIVE.length * INITIAL_WEEKS * MIN_TARGET_PX = 5*5*24 = 600 <= 700` の 1 式だけ。
  実際に描かれる高さを `CoverageGrid.tsx` のリテラルから積むと（`web/` に CSS リセットは無く、
  `grep -rn "box-sizing" web/src web/index.html` は 0 件なので `content-box`）——

  | 部分 | px |
  |---|---|
  | `section` の `padding: 12` 上下 | 24 |
  | `h2`（`600 15px/1.3` + `margin 0 0 8px`） | 27.5 |
  | `WeekRow` 5 本（`minHeight 24` + `padding 1` 上下） | 130 |
  | 「1 年ぶんを見る」ボタン（`marginTop 8` + `padding 4px` 上下 + `minHeight 24`） | 40 |
  | `section` の `marginBottom: 24` | 24 |
  | **1 ソースあたり** | **245.5** |

  5 ソースで **1,227.5 px**。`App.tsx` の `padding 12`・`h1`・`AchievementPanel`（表 7 行）を足すと
  **1,400 px を超える**。360 CSS px 幅の端末の縦は 640〜800 px なので、
  Scenario「5 ソースすべてについて直近 4 週以上が、**スクロールせずに同時に見えている**」は成り立たない。
  検査の「勘定」は帯の高さだけを積み、**余白・見出し・ボタン・外側の余白（合計 ≈ 628 px）を落としている**。
  jsdom が実寸を測らないこと自体は正しく断ってあるが、**代わりに置いた勘定が現物と合っていない**。
- kind: premise
- 処置: escalated
- 備考: ★ kind を `technical` から `premise` へ上げた。**本人が第 7 回 Q28 で根拠にした「5 ソース × 5 行 × 24 px ≒ 600 px」が現物と合っていない** —— 見出し・達成パネル・ボタン・余白を足すと約 1,500 px。勘定を直すだけでは 640 px を通らず、決定の前提そのものが崩れている。
- 提案: 勘定を現物の式にする（`(padding*2 + h2 + INITIAL_WEEKS*rowH + button + marginBottom) * 5 + chrome <= 640`）。
  その式では 640 を通らないので、**余白を詰めるか、開いた直後の週数を減らすか、
  ソースを折り畳む**かの選択になる —— 第 7 回 Q28 の答え（直近 4〜5 週）に触るので本人へ返す。

---

## 手 3: Scenario と test の突合

`python3 scripts/check_scenarios.py .` = `98 件 / 印 109 個 / 担保あり 97 / 人間の確認待ち 1`（rc=0）。
印の先を 1 件ずつ読んだ結果、**主張の階層とテストの階層が違うもの**が 4 件:

| Scenario | 印の先 | ずれ |
|---|---|---|
| 想定間隔より細かい空きが取得率として残る | `attempt_ratio_is_carried_by_heartbeat` | spec は「区間」、テストは 1 日 1 信号なので「日」と見分けられない（**R9**） |
| 1 年ぶんは伸ばして見る | `expand-to-year.test.tsx` | spec は「53 週」、テストは `YEAR_WEEKS` そのもの（**R5**） |
| 360 px 幅でも操作対象が 24 px を割らない | `target-size.test.tsx` | spec は「24 CSS px」、テストは `MIN_TARGET_PX` そのもの（**R4**） |
| 開いた直後に 5 ソースの直近 1 か月が同時に見える | `initial-viewport.test.tsx` | spec は「スクロールせずに」、テストは 600≤700 の勘定（**R15**） |
| 半日の停止が時刻の範囲で残る / 破棄が期間と件数で残る | `stop_is_stored_as_time_range` / `drop_is_stored_with_count` | spec は「WHEN 停止する / IF 破棄する」、テストは自分で行を入れて読む（**R11**、ST15 / ST04 が持つ） |

残り 92 件は主張と一致していた。とくに次は**主張の階層まで一致**している ——
「日本時間の 0 時を境に別の日になる」（`14:59:59Z` と `15:00:01Z` の 2 件が別の日に入ることを DB から読む）、
「格納された生存信号は書き換えられない」（11 列を 1 つずつ `UPDATE` し、さらに中身が変わっていないことを読む）、
「拒否の応答に受け取った値が含まれない」（応答を文字列化してソース名と原文を `contains` で探す）。

## 手 4: 本人の決定が test で固定されているか

`deep.md` の答えのうち、値を書き換えて緑のままになるものを実際に試した。

| 本人の答え | 変えたもの | 結果 |
|---|---|---|
| 日境界は `Asia/Tokyo`（Q2） | `DAY_TZ` → `"UTC"` | **9 件 FAILED** — 固定されている |
| 分母から抜くのは丸ごと覆う停止だけ（Q3） | `!stopped_full` → `!stopped_full && !dropped_full` | **71 passed** — **固定されていない（R7）** |
| 判定は割合（分母の 95 %）（第 5 回 Q18） | — | `threshold - 346.75 < 1e-9` / `- 190.0 < 1e-9` で固定されている |
| 停止・破棄が記録より優先（第 5 回 Q19） | `decide()` で①を上へ（外部が入れた改変） | `span_outranks_records` FAILED — 固定されている |
| 主語の割り当て（第 4 回 Q8） | 位置 ↔ 写真 を入れ替え | **71 passed** — **固定されていない（R6）** |
| 収集開始日は作られた日・遡る（第 6 回 Q24 / 第 7 回 Q26） | — | `sets_started_on_first_arrival` / `achievement_reevaluates_when_start_moves_back` が固定。ただし**上限が無い（R1）** |
| 合否は 5 本の窓が閉じた日に確定（第 7 回 Q27） | — | `achievement_provisional_until_all_windows_close` が `confirmed` / `confirms_on` / `days_until_confirmed` を固定 |
| 格子は縦長・新しい週が上（第 6 回 Q25） | — | `newest-week-first.test.tsx` が `starts` の降順を固定 |
| 開いた直後は直近 4〜5 週（第 7 回 Q28） | `INITIAL_WEEKS` → 53 | `initial-viewport` が `<= 6` で捕まえる（**定数と突き合わせていない**）— 固定されている |
| 1 年は 53 週（第 7 回 Q28 の対） | `YEAR_WEEKS` → 20 | **26 passed** — **固定されていない（R5）** |
| セルは表示専用（第 4 回 Q16） | — | `tagName === "SPAN"` は効く。ただし `cell.getAttribute("onclick")` は React では常に `null` なので**空振り**（`tabindex` と `cursor` は効く） |
| NFR-19 の 24 px（第 4 回 Q16） | `MIN_TARGET_PX` → 8 | **26 passed** — **固定されていない（R4）** |
| 想定間隔 21600 / 86400（第 4 回 Q12） | — | `expected_gap_seeded` が 5 ソースぶんをリテラルで固定 |
| 3 段の隣接 3:1（第 5 回 Q21） | — | 値から計算し直す形。ただし**描く色は見ていない（R3）** |

## 手 5: tasks の `[x]` と実体

61 件を 1 件ずつ、本文の「検証:」に書かれたコマンド／テスト名が実在して rc=0 になるかを見た。
**テストが 0 本のままチェックされたものも、実在しないコマンドも無かった。**
`--tests '*Heartbeat*'` `'*HeartbeatCounters*'` `'*HeartbeatOutbox*'` `'*HeartbeatWithoutRecords*'` は
`collector-android/app/src/test/.../Heartbeat*.kt` の 5 本として実在し、
`testDebugUnitTest` の 95 件に入っている。
8.0 の「テスト 0 件でも走ること」も `web/package.json` の `"test": "vitest run"` として実体がある。

指摘できるのは 2 件で、どちらも上に R として出した ——
8.5c の「同時に見える」（R15）、8.5 の「24 × 24 CSS px を割らず」（R4）。
**タスクの文面が嘘なのではなく、検証コマンドが主張の階層に届いていない。**

## 手 6: 隙間

「捨てたものは復元できない」型を優先して探した結果が **R1**（時計が戻る → 収集開始日が汚染され戻せない）と
**R2**（プロセスの再起動 / 一過性の失敗 → 稼働記録の行が消えて再送では戻らない）。
そのほか見て、**隙間ではなかった**もの:

- 権限の拒否: `androidCapability` が `permission` / `sensor` / `network` を読み、
  `Capability.of` が `blockers` を必ず埋める。`Capability` の `init` が
  「理由の無い取れない」を組み立てさせない。受け口でも `Invalid::Blockerless` で断る。**塞がっている**
- 未送信の消失: 生存信号は `FileOutboxStore(File(filesDir, "heartbeat.jsonl"))` に載っており、
  記録の `outbox.jsonl` と**別ファイル**（読み戻しで取り違えない）。ST01 の「メモリのみで
  最大 5 分ぶんが無言で消える」は繰り返していない
- 時計が戻る（取得率側）: `AttemptCounters.attemptsNow` が `maxOf(expected, successes)` を返すので
  「成功 > 試行」の信号にならない。**塞がっている**（時計が戻る問題の**もう一方**が R1）
- 応答が読めない: `/heartbeat` は本文が配列でも裸のオブジェクトでも空配列でも
  **必ず結果の配列**を返す（`smoke.sh` 20d が `[]` と `5` の 2 通りで確かめている）

---

# `pr-review-toolkit` の 3 agent から（会話に返ったものを写す）

`code-reviewer` / `pr-test-analyzer` / `silent-failure-hunter` を
`git diff origin/main` に対してかけた。**上の R1〜R15（`code-verify`）と重なるものは、
そちらの R 番号に統合して本文に残した**（同じ欠陥に 2 つの番号を付けない）。
以下は重ならなかったぶん。

## R16. 取得の数えがインスタンスの中だけにあり、プロセスが死んだ区間が観測から落ちる

- 成果物: `collector-android/app/src/main/kotlin/dev/ashiato/collector/Heartbeat.kt`（`AttemptCounters`） / `LocationService.kt`
- 根拠: `silent-failure-hunter` の C-2 / `code-reviewer` の I8。`since` と `successes` を
  メモリだけに持っており、`START_STICKY` の立て直しで**両方とも新品になる**。
  6 時間の区間のうち 5 時間 50 分死んで 10 分前に立て直されると、次の信号は
  `attempts = 10, successes = 10` で**取得率 100 %** —— 死んでいた区間が分母ごと消える。
  `Outbox` は「インスタンスの中だけに積むと立て直しで無言で消える」を理由に
  `FileOutboxStore` へ落としたのに、**同じ理由が当てはまる数えは落とされていなかった**。
  これは第 5 回 Q17（ST01 の R46 の宿題の答え）が見分けようとした当の区間。
- kind: technical
- 処置: fixed 11.3
- 備考: `FileCounterStore` を足し、`filesDir/heartbeat-counters.txt` に置いた。
  `HeartbeatCountersTest` の「数えは立て直しをまたいで残る」が固定する。
  なお「起動時の `emit()` が 0/0 を送ってその日の取得率を薄める」という指摘は**却下**した ——
  合計に 0 と 0 を足しても比は動かない（`sum(attempts)` / `sum(successes)` のどちらも増えない）。

## R17. 置き場を 1 度読めなかっただけで、溜まっていた未送信が次の書き直しで消える

- 成果物: `collector-android/app/src/main/kotlin/dev/ashiato/collector/OutboxStore.kt`（`load` / `save`）
- 根拠: `silent-failure-hunter` の C-3。`load()` の `IOException` 経路が `emptyList()` を返し、
  それが `pending` の初期値になる。次の送信成功で `remove` → `save` が
  **ファイルを丸ごと上書き**するので、読めなかった分が痕跡なく消える。
  `salvage()`（退避）は `parse` が null を返す経路でしか呼ばれておらず、
  **退避が要る度合いが高い IO 失敗のほうが素通り**していた。
  隠れる誤り: EMFILE・一時的な I/O エラー・direct boot 中のアクセス・一時ロック。
- kind: technical
- 処置: fixed 11.3
- 備考: **退避はしない**（一過性の失敗で正常なファイルを脇へ退けると、そちらが損になる）。
  読み出しが失敗したら**全件の書き直しを断る**。送った分が残るので次の起動で再送になるが、
  取り込み口は冪等なので行は増えない（FR-22）——**消えるより再送のほうがまし**。
  `OutboxStoreTest` の「読めなかった未送信が、次の書き直しで消えない」が固定する。

## R18. 恒久的に断られた 1 件が未送信の先頭に居座ると、以後 1 件も送れなくなる

- 成果物: `collector-android/app/src/main/kotlin/dev/ashiato/collector/Sender.kt`（`flush` / `acceptedIds`）
- 根拠: `silent-failure-hunter` の H-1。`accepted = false` の項目は `Outbox.remove` に渡らないので
  未送信に残り続ける。`take(MAX_BATCH)` は**常に先頭から**取るので、先頭 200 件が
  恒久的な拒否（`unknown_source` / `malformed` / `invalid_counts`）で埋まると
  **新しい記録には永久に順番が回らない**。`Sender` のコメントは
  「1 件の恒久的な失敗が後続を永久に止める」ことを避けたと書いているが、
  避けられているのは**サーバ側だけ**（1 件ごとの結果を返す）で、収集側に抜け道が無かった。
  残るのは 5 分ごとの logcat 1 行で、画面には⑥「途絶」としか出ない。
- kind: technical
- 処置: fixed 11.3
- 備考: 断られた識別子を覚えて次の契機では先に飛ばす。**捨てはしない** ——
  捨てる判断は ST04（保持と破棄）の担当。全部が断られたものになったときは
  もう一度だけ当たり直す（サーバ側の一時的な事情かもしれない）。

## R19. 画面が「取得に失敗」と「データが無い」を区別できない

- 成果物: `web/src/App.tsx`
- 根拠: `silent-failure-hunter` の H-2。3 つ重なっていた ——
  (1) 読み込み中の状態が無く、`fetch` がハングすると**空白のまま永久に**「データが無い」に見える。
  (2) `sources` が `[]` でも同じ空白。
  (3) `Promise.all` なので**片方の失敗が両方を消す**（達成が 500 を返すと、
  正常に取れた格子 5 本も描かれない）。
  **この画面の目的は「データが無い」の意味を残すこと**なので、
  取得の失敗とデータが無いことを混ぜるのは要件そのものを壊す。
- kind: technical
- 処置: fixed 11.2
- 備考: 読み出しを `loading` / `ok` / `failed` の 3 状態にし、格子と達成を**別々に受ける**。
  失敗の文言に「達成日数が 0 なのではありません」「収集が止まったのではありません」を入れた。
  `app.test.tsx` が 5 本で固定する。

## R20. 登録簿に無い Must ソースを、画面から黙って落とす

- 成果物: `crates/server/src/lib.rs`（`coverage_get`） / `crates/server/src/coverage.rs`（`of_sources`）
- 根拠: `silent-failure-hunter` の H-3。`let Some(src) = … else { continue; }` で無言に飛ばしていた。
  `must_sources()` は Rust の定数、登録簿の行は migration 0005 の `INSERT` で、
  **同じ 5 つの名前が 2 か所にある**。ずれた瞬間に `/coverage` は 4 本だけを返し、
  画面は 4 本の格子を並べる —— **5 本目が「無い」ことすら表示されない**。
  対照的に `achievement` は同じ状況を `not_started` に載せて返しており、
  **同じ事実に対して 2 つの経路が違う扱いをしていて、片方だけが黙っていた**。
- kind: technical
- 処置: fixed 11.1
- 備考: 名前のずれ自体は `expected_gap_seeded` が CI で止める（定数と登録簿を突き合わせる）。
  実行時にずれたとき（登録簿から行を消した運用）のために、
  `of_sources` が「まだ開始していないソース」として返し、`warn` を残す。

## R21. `facts` の結合が利用者ごとに日を複製する

- 成果物: `crates/server/src/coverage.rs`（`facts` の `c` CTE）
- 根拠: `silent-failure-hunter` の H-4。`c` に集約が無く、`core.coverage` の主キーが
  `(user_id, logical_source, day)` なので、**利用者が 2 人いれば同じ日が 2 行**返り、
  `LEFT JOIN` で 1 日が 2 行に膨らむ。`achievement` の分母と達成日数が
  日数ではなく**行数**になる。生存信号の側（`h`）は `GROUP BY` があるので複製しない ——
  **片方だけ集約が無く**、症状が「分母だけがおかしい」という気付きにくい形で出る。
  FR-29 が「単一利用者でも day one から持つ」と決めているのは、この日が来ることを
  前提にしているから。
- kind: technical
- 処置: fixed 11.1
- 備考: `sum(event_count)::int … GROUP BY day` に直した。
  `coverage_is_separated_by_user` が「絞らずに引いても 1 日 1 行」を固定する。

## R22. 生存信号は `DELETE` で差し替えられる（0004 の 3 手の迂回と同じ型）

- 成果物: `migrations/202609111112_immutable_heartbeat.sql` / `tools/check-immutable.sh`
- 根拠: `silent-failure-hunter` の H-5 / `code-reviewer` の I10。
  トリガが `BEFORE UPDATE` のみで、`DELETE` は素通りした。`content_hash` は
  `logical_source` + `emitted_at` + `raw` から決まるので、**2 手で証拠を差し替えられる**:
  `DELETE` → 同じ鍵で `INSERT`。0002 と 0006 が自分で書いた脅威（psql を直に叩く運用・
  第三者製プラグイン）が**まさにこの 2 手を打てる**。
  しかも `check-immutable.sh` はこの穴を見つけたうえで閉じずに記録しており、
  その `if` は**どちらの分岐でも `fail` を触っていなかった**（検査ではなく検査に見える出力）。
- kind: technical
- 処置: fixed 11.1
- 備考: トリガを `BEFORE UPDATE OR DELETE` に広げ、`check-immutable.sh` の節を
  「削除できないことを確かめる」に直した（行が残っていることも読む）。
  生存信号には `core.event` の FR-50 に当たる正当な削除理由が無い。

## R23. 起動のたびに当たる 0005 の分岐が、戻りうる条件を見ている

- 成果物: `migrations/202609111111_coverage_rebuild.sql`
- 根拠: `silent-failure-hunter` の H-6。条件が「`state` 列がある」だけで、
  **`state` はこの Story が外した当の列**（design D6 が「状態は行に焼かず導出する」と決めた結果）。
  将来この判断が覆って `coverage` に `state` を materialize した瞬間、
  **次の起動で 0005 が表を丸ごと DROP する**。`core.event` から引き直すので、
  `heartbeat` 由来の日も `coverage_span` で説明されていた日も戻らない。
  `check-immutable.sh` の「当て直しても消えない」検査は**現行の schema でしか回らない**ので、
  この分岐が誤りに転じたことを検出できない。
- kind: technical
- 処置: fixed 11.1
- 備考: 条件を「`state` 列があり、かつ `user_id` 列が**無い**」に絞った（0001 の形ちょうど）。

## R24. 生存信号の数えが、積めるかどうかを知る前に消費される

- 成果物: `collector-android/app/src/main/kotlin/dev/ashiato/collector/Heartbeat.kt`（`HeartbeatEmitter.emit`）
- 根拠: `silent-failure-hunter` の H-7 / `pr-test-analyzer` の F9 / `code-reviewer` の I9。
  `counters.take()` を `outbox.add()` より先に呼んでいたので、積めなかった区間の
  試行と成功が消えた。**`peek()` の docstring に書いてある問題そのもの**が
  `emit()` 側で起きており、`peek()` は本番のどこからも呼ばれていない死にコードだった。
- kind: technical
- 処置: fixed 11.3
- 備考: `takeAfter { … }` を足し、**積めたときだけ戻す**形にした。
  `HeartbeatCountersTest` の「積めなかった区間の数えは残る」が固定する。

## R25. `HeartbeatResult` の欄名を確かめる検査が 1 つも無い

- 成果物: `tools/check-openapi.sh`
- 根拠: `silent-failure-hunter` の M-1。収集側は生存信号の応答も `IngestResult` として復号する
  （`Sender` は 1 つの型しか持たない）。`docs/openapi.json` には `HeartbeatResult` が
  別 schema としてあるのに**一度も見られていなかった**。サーバが `accepted` を改名すると、
  `ignoreUnknownKeys = true` + 既定値 false により収集側は**例外も出さずに全件 false と読み**、
  生存信号の未送信が永久に減らない —— スクリプト自身が `IngestResult` について
  書いている危険（「全部緑のまま」）と同一。
- kind: technical
- 処置: fixed 11.4
- 備考: 欄名の突き合わせに加えて、**記録と生存信号の応答が同じ形であること**も見る
  （収集側が同じ型で読むため）。

## R26. `internal()` がどの操作で落ちたかを捨てる

- 成果物: `crates/server/src/lib.rs`（`internal`）
- 根拠: `silent-failure-hunter` の M-2。`ingest_one` だけで 3 本、`heartbeat_one` で 3 本、
  読み出しでさらに数本の SQL が**すべて 1 行**に畳まれていた。SQLSTATE `08006` が出たとき、
  それが R2 の「記録は入ったが稼働記録が落ちた」なのか登録簿の照会が落ちただけなのかを
  ログから区別できない。**操作名は値ではない**ので、A-2（私的データを出さない）は
  出さない理由にならない。
- kind: technical
- 処置: fixed 11.1

## R27. 401 がサーバ側に何の痕跡も残さない

- 成果物: `crates/server/src/lib.rs`（`authorize`）
- 根拠: `silent-failure-hunter` の M-3。合言葉がずれた端末は 5 分ごとに 401 を受け続け、
  **サーバ側のログは完全に無音**だった。そのあいだ画面には⑥「途絶」が並ぶが、
  それが「端末が死んだ」のか「合言葉がずれている」のかを分ける情報を、
  サーバは握っていながら捨てていた。**この Story の目的（記録が無いことの意味を残す）から見て、
  意味を持つ失敗を捨てている箇所**にあたる。
- kind: technical
- 処置: fixed 11.1
- 備考: 出すのは「資格情報が有ったか無かったか」だけ（値は載せない）。

## R28. 記録の論理削除が稼働記録に届かない

- 成果物: `crates/server/src/lib.rs` / `migrations/202609111111_coverage_rebuild.sql`
- 根拠: `silent-failure-hunter` の M-5。ある日の記録を全件論理削除しても
  `core.coverage.event_count` は減らないので、画面はその日を①「記録あり」のまま出し、
  `/events` には 1 件も出ない。
- kind: technical
- 処置: rejected: **①のままが正しい。** 稼働記録が答えるのは
  「その日**収集は動いていたか**」であって「いまデータが残っているか」ではない（扉 #14）。
  ST01 の正典「稼働記録は新しく入った記録だけを数える」も**取り込みの事実**を数えると定めており、
  後から消したことで「収集が動いていなかった」に変わるのは意味が逆。
  破棄（FR-9）は `coverage_span(kind='dropped')` で⑤として出るので、
  **失われたことを示す経路は別に用意されている**。
  論理削除（FR-50）を持つ Story が画面にどう出すかを決めるなら、そのときに `coverage_span` へ
  3 つ目の `kind` を足す形で乗る。ST02 の担当ではない。

## R29. 想定間隔が 1 日未満のソースでは、②へ落ちる分岐が原理的に発火しない

- 成果物: `crates/server/src/coverage.rs`（`decide` の `near`）
- 根拠: `silent-failure-hunter` の M-6 / `pr-test-analyzer` の F13。
  `gap_days = expected_gap_sec / 86400` で、5 ソース中 4 本は `21600`（= 0.25 日）。
  `diff` は整数の日数なので、この分岐に到達する時点で `diff >= 1` となり
  `1 <= 0.25` は常に偽 —— 4 本については実質「記録も信号も無い日 = 即⑥」。
  振る舞いとしては spec の Scenario（想定間隔 6 時間なら 1 日の空白は途絶）どおりだが、
  **そのことがどこにも書かれておらず**、`expected_gap_sec` を 6 時間から 12 時間へ変えても
  判定が動かないことに設定を触った人が気付けない。
- kind: technical
- 処置: fixed 11.1
- 備考: design D17 の帰結であることを `outage_boundary_is_inclusive` の中に書き、
  24 時間（ブラウザ履歴）の**ちょうどの境界**（1 日は②、2 日は⑥）と
  6 時間の場合（隣の日でも⑥）を同じ検査で固定した。

## R30. `ExecutorFlushScheduler` は例外で黙って止まる設計を呼び出し側に委ねている

- 成果物: `collector-android/app/src/main/kotlin/dev/ashiato/collector/LocationService.kt`
- 根拠: `silent-failure-hunter` の M-7。`scheduleWithFixedDelay` は task が投げると
  **以後の実行を静かに打ち切る**（ログも例外も出ない）。呼び出し側の `runCatching` に
  安全性を委ねていたので、1 度忘れた日に**収集は前景通知を出したまま送信も生存信号も
  永久に止まり、logcat に 1 行も残らない**。
- kind: technical
- 処置: fixed 11.3
- 備考: `every()` の側で受け止める（`tick_crashed` を出す）。性質を構造で持たせた。

## R31. `ACCESS_NETWORK_STATE` を宣言していないので、実機では起動のたびにサービスが落ちる

- 成果物: `collector-android/app/src/main/AndroidManifest.xml` / `AndroidCapability.kt`
- 根拠: `code-reviewer` の C1。`ConnectivityManager.getNetworkCapabilities()` は
  `ACCESS_NETWORK_STATE` 必須で、無いと `SecurityException` を投げる。
  `grep -rn ACCESS_NETWORK_STATE collector-android/` は **0 件**だった。
  経路は `startBeating()` → `readCapability()` → `androidCapability()` → `hasNetwork`。
  `onStartCommand` の `try` は `fixSource.start` しか囲っておらず `startBeating()` はその外なので、
  **例外がプロセスを落とし、`START_STICKY` と合わさってクラッシュループになる**。
  緑だったのは `TestableLocationService` が `readCapability()` を上書きしていて
  本番経路を 1 本も通らず、CI も Android Lint を走らせていないため。
- kind: technical
- 処置: fixed 11.3
- 備考: マニフェストに宣言し、端末を読む 3 つの口を `runCatching { … }.getOrDefault(false)` で
  包んだ。**倒す向きは「取れない」側** —— 読めていないのに「取れている」と報告するのは、
  壊れているのに「動いていた」と残すのと同じ（深掘り Q5 が塞いだ当の型）。
  CI への Android Lint 追加は**入れていない**（ST02 の担当外。`HANDOFF` ではなく
  `docs/flow-gates.md` の課題として PR 本文に書く）。

## R32. 1 日に取得可否が混在する生存信号の検査が無い（`bool_and` に変えても緑）

- 成果物: `crates/server/src/coverage.rs`（`facts` の `bool_or`） / `coverage/tests.rs`
- 根拠: `code-reviewer` の I1 / `pr-test-analyzer` の F14。`bool_or(capturable)` を
  `bool_and` に変えても 71 件すべて通った。**その日に取得できる信号と取れない信号の両方がある**
  検査が 1 件も無かった —— これは現実にいちばん起きる形（日の途中で権限が剥がれる）で、
  spec が名指しで書き分けている区別（「(6) 生存信号があれば（**すべて取得できない状態なら**）」）。
  壊れると②が③に化け、利用主語 3 ソースの達成日が黙って落ちる。
- kind: technical
- 処置: fixed 11.1

## R33. 1 日を丸ごと覆わない「破棄」が未検査（条件を緩めても緑）

- 成果物: `crates/server/src/coverage/tests.rs`
- 根拠: `code-reviewer` の I2。`partial_stop_does_not_decide_state` は `stopped` しか見ておらず、
  テスト中の `dropped` は全部が丸 1 日だった。`dropped_full` を「一部でも重なれば真」に
  緩めても全緑。spec は「1 日を丸ごと覆わない**停止・破棄**について、その日の状態を決めさせない」と
  両方を書いている。
- kind: technical
- 処置: fixed 11.1

## R34. 週を選ぶと、いちばん暗い段が背景に埋もれる（自前のガードを割る）

- 成果物: `web/src/CoverageGrid.tsx` / `web/src/__tests__/state-contrast.test.ts`
- 根拠: `code-reviewer` の I5 / `pr-test-analyzer` の F12。実際に描く値から WCAG 2.2 の定義で
  計算し直すと、非選択時は `other(L9)` vs `surface2(L24)` が **1.878:1** だが、
  選択時は `other(L9)` vs `surface1(L18)` で **1.422:1** に落ちる。
  検査は `SURFACE.surface2` しか見ないので緑のまま —— design D23 が名指しで警告している
  「測る色と実際に描く色がずれている」型がそのまま再現していた。
  **いちばん見たい週で格子がいちばん読めない。**
  併せて、閾値の `1.5` は**要件のどこにも無い数字**で、「通るように選んだ数」に見えた。
- kind: technical
- 処置: fixed 11.2
- 備考: 選択を**面の明るさではなく輪郭**（`outline: 2px solid` の `muted` = surface2 上で 5.195:1）で
  表し、セルが乗る面が 1 つに決まることを `week-select.test.tsx` が固定する。
  `1.5` は残したが、「セルが乗る面は surface2 だけ」を構造で担保したうえでの値にした。

## R35. 週の帯に `role="row"` を付けてボタンの意味を潰している

- 成果物: `web/src/CoverageGrid.tsx`
- 根拠: `code-reviewer` の I6。`<button>` に `role="row"` を明示すると暗黙の button ロールを
  上書きするので、**この画面で唯一の主要な操作対象が支援技術からボタンとして消える**。
  `aria-pressed` は ARIA 1.2 で button ロールにしかサポートされる状態が無いため、
  **選択中であることが一切伝わらない**。子の `role="gridcell"` が全部 `aria-hidden` なので、
  セルを持たない行という不正な grid 構造にもなっていた。
- kind: technical
- 処置: fixed 11.2
- 備考: `role` を外し、`data-week` / `data-cell` で構造を表す（検査もそちらを見る）。

## R36. 起動時の `emit()` だけ `runCatching` の外にある

- 成果物: `collector-android/app/src/main/kotlin/dev/ashiato/collector/LocationService.kt`
- 根拠: `code-reviewer` の I7。同じ `startBeating()` の中で、刻みからの `emit()` は守られているのに
  起動時の 1 発だけ裸だった。`readCapability()` も `outbox.add()` も投げうるので、
  ここが R31 をクラッシュループに変換していた。このファイル自身が権限拒否のところで立てた規律
  （「**落とさずに何もしない** —— 落ちると次の起動まで収集が止まり、成功条件 1 に直接効く」）と
  食い違っていた。
- kind: technical
- 処置: fixed 11.3

## R37. 畳み戻しても、見えない週の詳細が残る

- 成果物: `web/src/CoverageGrid.tsx`
- 根拠: `code-reviewer` の I11。`selectedWeek` を `shown` ではなく `weeks` から探していたので、
  1 年ぶんに伸ばして 30 週目を選び「直近だけにする」を押すと、**行は消えるのに日付リストは出たまま**。
  どの行にも選択の印が立っていない状態で、閉じる手段が「もう一度伸ばして同じ帯を押す」しかなかった。
- kind: technical
- 処置: fixed 11.2

## R38. `README` の「2 桁小さい」が数として成り立たない

- 成果物: `collector-android/README.md` / `openspec/changes/st02-collection-coverage/specs/device-collection/spec.md`
- 根拠: `code-reviewer` の I12。360 分 ÷ 14.2 分 ≒ **25 倍**で、「2 桁」（100 倍以上）ではない。
  「1 日 24 回」から維持窓の間隔を読むと約 1 時間で、6 時間との比は 6 倍。
  **どちらの読みでも 2 桁にならない。** 結論（生存信号は Doze の維持時間帯に乗る）は
  25 倍でも 6 倍でも成り立つので**振る舞いの誤りではなく根拠の誤り**だが、
  design D10 が WCAG の 729:1 を実際に計算して結論を覆したのと同じ規律に照らして直す。
- kind: technical
- 処置: fixed 11.3
- 備考: spec 側の根拠ブロックにも同じ誤りがあったので訂正した（Requirement と Scenario は動かない）。

## R39. `blockers` が読み出し口から返らない —— Scenario の後半が未実装

- 成果物: `crates/server/src/coverage.rs`（`DayCell` / `facts`） / `docs/openapi.json` / `web/src/coverage.ts`
- 根拠: `pr-test-analyzer` の F1。spec の Scenario は
  「取得できない状態が理由とともに残る … **AND 何が満たされていないか（権限）が返る**」だが、
  `grep -n blockers crates/server/src/coverage.rs` は **0 件**だった。
  印の付いた検査（`uncapturable_keeps_its_reason`）は `testdb::put_heartbeat` が入れた行を
  読み直しているだけで、**`testdb.rs` と PostgreSQL の `text[]` を検査していた**。
  実装側を「返さない」に書き換えても（すでにそうなっていた）緑。
- kind: technical
- 処置: fixed 11.1
- 備考: `DayCell.blockers` を足し、日ごとに重複を畳んで返す。
  `blockers_are_returned_from_coverage` が読み出し口越しに固定する。
  **`check_scenarios.py` は印しか見ない**ので、この型（Scenario の逐語が満たされていなくても
  印は付く）は機械では止まらない。PR 本文にも書く。

## R40. `FakeScheduler` が task を捨てているので、周期 emit が一度も走っていない

- 成果物: `collector-android/app/src/test/kotlin/dev/ashiato/collector/LocationServiceTest.kt`
- 根拠: `pr-test-analyzer` の F5。`override fun every(periodMs, task) { this.periodMs = periodMs }` で
  task を捨てていたので、`scheduler.every(HEARTBEAT_INTERVAL_MS) { emitter.emit() }` の
  **中身を空のラムダに書き換えても全緑**だった。緑を保っていたのは起動時の直呼びだけで、
  spec の「想定間隔**ごとに**送られる生存信号」の要が未検査。
- kind: technical
- 処置: fixed 11.3
- 備考: `fire()` を足し、2 回 fire したら 3 件（起動時 1 + 周期 2）になることを固定した。
  記録側の `flusher` にも同じ穴が ST01 から続いているが、**ST02 の担当外**なので触っていない
  （`FakeScheduler` の `fire()` はそちらでも使える形にしてある）。

## R41. 生存信号の送り先が `/heartbeat` であることを誰も見ていない

- 成果物: `collector-android/app/src/test/kotlin/dev/ashiato/collector/HttpTransportTest.kt`
- 根拠: `pr-test-analyzer` の F7。`HttpTransport(…, "/heartbeat")` を `"/ingest"` に
  書き換えても全緑だった。現実に起きるのは「サーバが `malformed` で全件断り、
  未送信が永久に溜まり、logcat に 1 行出るだけ」—— `HttpTransport` のコメントが
  自ら警告している失敗の型そのもの。
- kind: technical
- 処置: fixed 11.3

## R42. `HEARTBEAT_INTERVAL_MS` が「登録簿の 6 時間」と一致していることが固定されていない

- 成果物: `collector-android/app/src/test/kotlin/dev/ashiato/collector/LocationServiceTest.kt`
- 根拠: `pr-test-analyzer` の F8。`assertEquals(HEARTBEAT_INTERVAL_MS, periodMs)` の 1 行だけで、
  **定数を 1 分にしても緑**だった。tasks 7.1 が「ずらすと正常な運用が⑥に見える」と書いている
  当の値で、サーバ側は `expected_gap_seeded` が `21_600` をリテラルで固定している。
- kind: technical
- 処置: fixed 11.3
- 備考: `assertEquals(21_600_000L, HEARTBEAT_INTERVAL_MS)` を足して 2 か所を縫い合わせた。

## R43. 進行中の停止・破棄（`ended_at IS NULL`）と、覆う/覆わないの端が未検査

- 成果物: `crates/server/src/coverage/tests.rs`
- 根拠: `pr-test-analyzer` の F10。`testdb::put_span` に `None` を渡す呼び出しが **0 件**だった。
  「利用者がいま止めていて、まだ再開していない」は FR-34 の最も普通の状態。
  併せて、`<=` / `>=` を `<` / `>` に変えても落ちない入力しか無かった
  （ぴったり覆うか明らかな半日しか置いていない）。
- kind: technical
- 処置: fixed 11.1
- 備考: `open_ended_stop_covers_every_later_day` と `span_edges_decide_whether_a_day_is_covered`
  （1 秒足りない範囲が「丸ごと」と判定されないこと）を足した。

## R44. 「面の上で見える」の閾値 1.5 が発明された数字

- 成果物: `web/src/__tests__/state-contrast.test.ts`
- 根拠: `pr-test-analyzer` の F12。段どうしの 3:1（NFR-23）は本物だが、
  4 本目の assert は**要件のどこにも無い 1.5** を閾値にしていた。実測は
  `other` ↔ `surface2` が 1.878、`alive_no_record` ↔ `surface2` が 2.072。
  「通るように選んだ数」に見え、通した瞬間に判断が消える。
- kind: technical
- 処置: fixed 11.2
- 備考: **R34 と同じ処置に畳んだ。** 選択を面の明るさで表すのをやめ、
  「セルが乗る面は `surface2` だけ」を構造で担保したうえで 1.5 を残した ——
  この値は NFR-23（隣接 3:1）ではなく「格子の広がりが読める」ための運用値で、
  要件ではないことを検査の本文に明記した。

## R45. 想定間隔の「ちょうど」の境界が無い

- 成果物: `crates/server/src/coverage/tests.rs`
- 根拠: `pr-test-analyzer` の F13。`diff <= gap_days` の等号がどちらに倒れるかを
  決める入力が無く、`<=` を `<` に変えても落ちなかった。
- kind: technical
- 処置: fixed 11.1

## R46. 利用者による分離が振る舞いとして一度も確かめられていない

- 成果物: `crates/server/src/api_tests.rs`（`user_id_on_all_coverage_tables`） / `coverage/tests.rs`
- 根拠: `pr-test-analyzer` の F15。`information_schema.columns` を引くだけで、
  **列が使われているかは見ていない** —— `facts()` / `active_days()` から
  `user_id = $1` の条件を全部削っても緑だった（全検査が固有のソース名で隔離されているため）。
- kind: technical
- 処置: fixed 11.1
- 備考: `coverage_is_separated_by_user` を足した。R13（冪等索引に `user_id` が無い）も
  同じ検査が守る。

## R47. 「重複だけが届いた日も行は立つ」を確かめられる入力になっていない

- 成果物: `crates/server/src/api_tests.rs` / `tools/smoke.sh`
- 根拠: `pr-test-analyzer` の F16。どちらも「1 回目（新規）→ 2 回目（重複）」で、
  **行は 1 回目で立っていた**ので「重複のときに UPSERT ごと飛ばす」実装でも通った。
- kind: technical
- 処置: fixed 11.1
- 備考: 冪等キーが利用者を含まないことを使い、**別の利用者に重複だけが届く**入力にした
  （その利用者には行がまだ無い）。

## R48. `achievement_endpoint` の assert が原理的に落ちない

- 成果物: `crates/server/src/api_tests.rs`
- 根拠: `pr-test-analyzer` の F17。`achieved_days <= denominator` は同じイテレータから
  数えているので構造上落ちない。実質「5 本返る」と「401」だけを見る検査だった。
- kind: technical
- 処置: fixed 11.1
- 備考: 並び（design D19）と主語をリテラルで固定し、`confirmed` / `not_started` / `confirms_on` が
  ソースの状態と噛み合うことを見るようにした。

## R49. `decide()` の優先順位のうち、試されていない対がある

- 成果物: `crates/server/src/coverage/tests.rs`
- 根拠: `pr-test-analyzer` の F18。7 段の順序表のうち実際に組み合わせた入力があるのは
  3 対だけで、**導入前 vs 破棄・停止 vs 取得可の信号・記録 vs 生存信号**が未観測だった。
- kind: technical
- 処置: fixed 11.1
- 備考: `decide()` は純関数なので、`DayFacts` を直に組んだ表駆動の検査を 1 本足して
  8 通りの順序を全部固定した（DB 越しの検査はそのまま残す）。

## R50. `sandwiched_gap_is_alive` の名前と中身が逆

- 成果物: `crates/server/src/coverage/tests.rs`
- 根拠: `pr-test-analyzer` の指摘。名前は「挟まれた空白は②」なのに、中身は
  `assert_eq!(…, DayState::Outage)` で**逆を確かめていた**。docstring の
  `Scenario: 途絶は収集側の報告なしに立つ` も名前と合っていなかった。**読んだ人が必ず誤読する。**
- kind: technical
- 処置: fixed 11.1
- 備考: 名前どおり「挟まれて②」を確かめる形に直し、⑥の側と
  「収集側の報告なしに立つ」は別の検査（`outage_stands_without_any_report`）に分けた。
  tasks 5.3 が挙げている検証コマンドの名前は変えていない。

## R51. `HeartbeatWithoutRecordsTest` の assert が常に真

- 成果物: `collector-android/app/src/test/kotlin/dev/ashiato/collector/HeartbeatWithoutRecordsTest.kt`
- 根拠: `pr-test-analyzer` の指摘。`assertEquals(0, events.size())` の `events` は
  **この検査が一度も書き込まないまっさらな outbox** なので常に真。
  `emitter.emit()` を直に呼んでいるので、「記録の契機から呼んでいる」実装
  （＝この検査が防ごうとしている当の設計）でも緑になる。
- kind: technical
- 処置: fixed 11.3
- 備考: 配線を見ているのは `LocationServiceTest` 側だと本文に明記し、
  assert の意味（emitter が記録の側に触らないこと）を文言で限定した。

## R52. `HeartbeatError::InvalidRaw` が API 経由で一度も出ていない

- 成果物: `crates/server/src/api_tests.rs`
- 根拠: `pr-test-analyzer` の指摘。`heartbeat.rs` の単体検査が `validate()` を直接叩くのみで、
  他の 4 つの理由（`Malformed` / `InvalidCounts` / `MissingBlockers` / `UnknownSource`）は
  API 経由で確かめられているのに 1 本だけ欠けていた。
- kind: technical
- 処置: fixed 11.1

## R53. 生存信号の受け口で、空配列・非配列の本文が未検査

- 成果物: `crates/server/src/api_tests.rs`
- 根拠: `pr-test-analyzer` の指摘。`heartbeat_post` の `_ => BAD_REQUEST` 分岐が未検査。
  平文を返すと収集側がパースに失敗し、状態符号の意味を失う（`/ingest` は smoke 20d が見ている）。
- kind: technical
- 処置: fixed 11.1

## R54. セルにハンドラが無いことの検査が空振り（`onclick` 属性は React では常に null）

- 成果物: `web/src/__tests__/target-size.test.tsx`
- 根拠: `code-verify` の「手 4」の注記。`cell.getAttribute("onclick")` は React では
  常に `null` を返すので、**セルにハンドラを付けても捕まらない**
  （`tabindex` と `cursor` は効いていた）。
- kind: technical
- 処置: fixed 11.2
- 備考: React が付ける内部 props に `onClick` が生えていないことを見る形に替えた。
