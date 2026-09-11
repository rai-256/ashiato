# ST02 実装の独立検証・2 巡目（code-verify / commit 351fb98 のみ）

対象: `feat/st02-collection-coverage` の **351fb98 だけ**（`git diff 31f9710..HEAD`）。
これより前の変更は `review/code.md` で済んでいるので見ていない。
やり方: 申告されたコマンドを全部走らせ直し、**高さの勘定を手で積み直し、
新しい検査 12 本それぞれについて守るものを潰して落ちるかを見た**。

> **再現性の注記（重要）**: 検証中、作業ツリーが**外部から 3 度書き換えられた**
> （`crates/server/src/coverage.rs` の `>=` → `>`、`migrations/0007` の同じ箇所、
> `web/src/tokens.ts` の `ONE_SCROLL_PX = 1`）。同じ時刻に 4 本の兄弟セッションが
> 走っており（`tasks/` に `agent-*.jsonl` が 4 本）、**別のレビューアが同じ作業ツリーで
> 改変実験をしている**。そのため、改変実験はすべて **HEAD の複製**
> （`git archive 351fb98` → `/tmp/.../scratchpad/head`）に対して行った。
> 下の「実測」は複製、または `git status` が clean だった時点の値。**作業ツリーは直していない。**

## 申告と実測

| 申告 | 実測したコマンド | 結果 |
|---|---|---|
| `cargo test --workspace` 94 件 | 同左 | rc=0 / `94 passed` — **一致** |
| `cd web && npm run test` 43 件 | 同左 | rc=0 / `Tests 43 passed` / `Test Files 12 passed` — **一致** |
| `cargo fmt --all --check` | 同左 | rc=0 — 一致 |
| `cargo clippy --workspace --all-targets -- -D warnings` | 同左 | rc=0 — 一致 |
| `./tools/check-migrations.sh` | 同左 | rc=0 — 一致 |
| `./tools/check-boundaries.sh` | 同左 | rc=0 — 一致 |
| `./tools/check-openapi.sh` | 同左 | rc=0 — 一致 |
| `./tools/check-licenses.sh` | 同左 | rc=0（Rust 240 / Node 254 / Android 対象外）— 一致 |
| `./tools/check-immutable.sh` | `docker stop ashiato2-db-1` → 実行 | rc=0（13 項目 OK）— 一致 |
| `./tools/check-panic-log.sh` | 同上 | rc=0 — 一致 |
| `./tools/smoke.sh` | 同上（2 回） | **rc=7 / rc=1 — ただし環境由来。R11** |
| `check_scenarios.py . st02-collection-coverage` | 同左 | `Scenario 108 / 印 126 / 担保あり 107 / 確認待ち 1` — **一致** |
| `check_chain.py` | 同左 | `chain: OK` rc=0 — 一致 |
| `review_triage.py` | 同左 | rc=0 — 一致 |
| `openspec validate st02-collection-coverage --strict` | 同左 | `is valid` rc=0 — 一致 |
| tasks 群 12〜15 が全部 `[x]` | `grep -c '^- \[x\]' / '^- \[ \]'` | 80 / 2（残 2 は 16.1 / 16.2）— 一致 |
| `collector-android/` を 1 バイトも触っていない | `git show --stat 351fb98` | 該当ファイル 0 — 一致 |
| Android は未実行（JDK 無し） | `which java` | 無し。**こちらでも走らせていない** |
| 移行 0007 を当て直しても壊れない（tasks 14.1） | `psql -v ON_ERROR_STOP=1 -f 0007…sql` を 2 回 | rc=0 / rc=0（2 回目は NOTICE のみ）— 一致 |

**捏造も空テストも無い。** 94 / 43 はすべて実在し、実際に走り、潰せば落ちるものが大半だった。
ずれは下の 13 件。

## 手 1: 固定値を独立に再計算する — **一致**

`one-scroll.test.tsx` の勘定を、テストのコードを読まずに **CSS 宣言から手で積み直した**
（`main` の padding → `h1` → 達成パネル → 格子 5 本）。その後、複製に測定用の
`console.log` を足した版を走らせて突き合わせた。

| 値 | 手で積んだ値 | テストの実測 | 判定 |
|---|---|---|---|
| `h1` | 16 + 18×1.3 = **39.4** | 39.4 | 一致 |
| 達成パネル | 28 + (27.5 + 26.2 + 26.2 + 109.2) = **217.1** | 217.1 | 一致 |
| 格子 1 本 | 28 + (27.5 + 4×24 + 42) = **193.5** | 193.5 | 一致 |
| 5 本目の下端 | 12 + 39.4 + 217.1 + 5×193.5 = **1,236** | 1,236 | 一致（予算 1,280） |
| 2 本目の格子の下端 | **593.5** | 593.5 | 一致（予算 640） |
| `ONE_SCROLL_PX` | 640 × 2 = 1,280 | 1,280 | 一致 |

**勘定そのものは正しい。** 問題は余地と、勘定が見ていないもの（**R8**）。

## 手 2: ガードをわざと壊す — 新しい検査 12 本を 1 本ずつ

すべて HEAD の複製に対して行い、毎回 `diff -q` で復元を確認した。

| # | 潰したもの | 走らせたもの | 結果 |
|---|---|---|---|
| M1 | `touch_started_on` の `>= registered_at` を削る | `clock_skew_does_not_move_started_on` | **FAILED** — 本物 |
| M2 | `decide` の `day > r` を `day >= r` | `retired_days` | **FAILED** — 本物 |
| M3 | `achievement` の退役フィルタを削る | `retired_days_out_of_denominator` | **FAILED** — 本物 |
| M4 | `resolve_tips` を空の map にする | `must_source_resolves_to_successor` | **FAILED** — 本物 |
| M5 | `facts` の `c` を `core.coverage` に戻す | `recorded_follows_event_time` | **FAILED** — 本物 |
| M6 | `sources` の `r.started` を `s.collection_started_on` に戻す | `succession_inherits_started_on` | **FAILED** — 本物 |
| M7 | 移行 0007 の「NULL に戻す」UPDATE を削る | `migration_repairs_polluted_started_on` | **FAILED** — 本物 |
| M8 | 移行 0007 の **`>= registered_at` 2 か所**を削る | 同上 | **ok. 1 passed — 空振り（R6）** |
| M9 | `touch_started_on` の `>=` を `>` にする | `cargo test --lib` 全件 | **98 passed — 空振り（R7）** |
| W1 | `retiredLast` を素通しにする | `retired-source.test.tsx` | **1 failed** — 本物 |
| W2 | 退役でも畳まない（`shown` から分岐を外す） | 同上 | **2 failed** — 本物 |
| W3 | **`App.tsx` から `retiredLast` を外す** | `npm run test` 全件 + `tsc -b` | **43 passed / rc=0 — 空振り（R4）** |
| H-a | `INITIAL_WEEKS` 4 → 5 | one-scroll + initial-viewport | **2 failed** — 本物 |
| H-b | `SECTION_PAD_PX` 8 → 12 | one-scroll + initial-viewport + target-size | **1 failed**（1,284 px）— 本物 |
| H-c | `SECTION_GAP_PX` 12 → 24 | 同上 | **1 failed**（1,308 px）— 本物 |
| H-d | **週の帯の `padding` 0 → 1**（+ セルの `minHeight` を −2 に戻す） | 同上 | **8 passed — 空振り（R5）** |

## 手 3: Scenario と test を突き合わせる

`check_scenarios.py` は rc=0（107/108 担保）。印の先を逐語で読み直して、**3 件がずれていた**。

| Scenario | ずれ | 番号 |
|---|---|---|
| 汚れた収集開始日は引き直せる | THEN の「登録簿に行ができた日**以降で**いちばん古い」を、検査のソース (a) が記録 1 件しか持たないので観測していない | **R6** |
| 退役した名前の代わりに後継を数える | Requirement 本文は「Must の 5 ソースのうち**退役したものについて**」だが、実装は退役していない名前も差し替える | **R2** |
| 退役したソースは後ろで畳まれている | WHEN が「稼働状況の**画面を開く**」なのに、並び順の検査は `retiredLast()` を直接呼んだ配列でしか観測していない（`App` を描いていない） | **R4** |

## 手 4: 本人の決定が test で固定されているか

| 本人の答え（逐語） | 固定されているか |
|---|---|
| Q29「受けるが、**収集開始日の計算から外す**」記録も信号も捨てない | 受け口側は M1 で固定。**信号が残ることも** `clock_skew_` が読み出し口から確認している。○ |
| Q29 の閾値「登録簿に行ができた日より前」 | **境界が固定されていない**（M9）。移行側は**閾値ごと消しても通る**（M8）。**R6 / R7** |
| Q30「第 7 回 Q28（直近 4〜5 週）は変えない」 | `INITIAL_WEEKS = 4` は範囲内。`>= 4`（initial-viewport）と `<= 1,280 px`（one-scroll）で上下から挟まれている。○ |
| Q30「NFR-19（週の帯 24 px）は割らない」 | `target-size.test.tsx` が**リテラル 24** と突き合わせている。帯は `minHeight/minWidth: 24` のまま。○ |
| Q30「開いた直後に 2〜3 ソース」 | `>= 2` で固定。実測は**ちょうど 2 本**（3 本目の下端 787 px）。○ |
| Q31「古い名前を分母から外し」 | M3 で固定。○ |
| Q31「**新しい名前が窓を引き継ぐ**」 | 窓の**起点**だけ固定（M6）。**達成日は引き継がれず、引き継いだ瞬間に未達になる。R1** |

## 手 5: tasks の `[x]` と実体 — **一致**

群 12〜15 の 15 件すべてについて、本文の検証方法（テスト名・コマンド）が実在し rc=0 になることを確認した。
`clock_skew_` 系・`retired_days` 系・`succession_inherits_started_on` ・
`must_source_resolves_to_successor` ・`recorded_follows_event_time` はすべて実在する。
14.1 の「移行を当て直しても壊れない」も `psql` で 2 回当てて rc=0 を確認した。
**名前だけ挙がっていて存在しないテストは 1 本も無い。**

## 手 6: 隙間

指示された 3 点を実測した（複製 + 共有 DB）。

- **移行 0007 を当て直しても壊れない** — 2 回当てて rc=0。○（ただし R9 の副作用は別）
- **引き継ぎの鎖が輪になったとき** — `UPDATE core.source SET succeeds = …` で**輪は作れる**
  （`rows_affected = 1`。CHECK は自己参照しか塞いでいない）。**回り続けはしない**
  （`depth < 32` で止まり、`b.started = 2026-05-01` が返った）。→ **R12**
- **`resolve_tips` が分岐で非決定になるか** — ならない。`ORDER BY base, depth DESC, node` の
  `node` が同順位を切っており、2 本が同じ名前を引き継いだ実測で常に辞書順で小さいほうが選ばれた。○

そのほかに見つけた隙間が **R3 / R9 / R11**。

---

## R1. 引き継いだ瞬間に、引き継ぎ元の達成日が全部消える（Q31 の答えと逆の結果になる）

- 成果物: `crates/server/src/coverage.rs:695,726` / `openspec/changes/st02-collection-coverage/specs/collection-coverage/spec.md:466`
- 根拠: HEAD の複製に検証用テストを足して実測（`cargo test -p ashiato-server --lib probe_ -- --nocapture`）。
  旧名を 05-01 に開始・05-01〜05-10 の 10 日すべて記録あり → 05-10 に退役、
  後継が 05-11〜05-13 の 3 日。今日 05-14 で達成を引くと:

  ```
  PROBE-H2 counted=<新名> started=Some(2026-05-01) denom=13 achieved=3 met=false failing=["<新名>"]
  ```

  `achievement()` は `resolve_tips` で**先端の名前に差し替えてから** `facts(pool, user, name=先端, start=鎖の根の日, …)`
  を引く（`coverage.rs:726`）。窓の**起点**だけが根から来て、**その期間の記録は後継の名前でしか探さない**ので、
  引き継ぎ前の 10 日は「分母に入るが達成していない日」になる。
  spec は `:491` で「**名前を分けただけで成功条件 1 が落ちる**」のを防ぐと書いており、
  `retired_days_out_of_denominator` はその保護を**後継がいない場合についてだけ**固定している
  （後継がいると `retired_on` は先端＝退役していない行のものになり、フィルタが 1 日も落とさない）。
  既存の `must_source_resolves_to_successor` は `denominator` / `achieved_days` を 1 つも assert していない。
- kind: technical
- 処置: fixed 17.1
## R2. 退役していない名前も後継に差し替わる（spec は「退役したものについて」と書いている）

- 成果物: `crates/server/src/coverage.rs:293-312` / `specs/collection-coverage/spec.md:466-467`
- 根拠: 同じ実測。旧名を**退役させずに**後継だけ作り、旧名に 05-01 の記録を置いて達成を引くと:

  ```
  PROBE-H1 named=<旧名> counted=<新名> denom=1 achieved=0 met=false
  ```

  `resolve_tips` は基点が退役しているかを一切見ず、`WHERE retired_on IS NULL ORDER BY base, depth DESC`
  で**いちばん深い生きているノード**を返す。Requirement の逐語は
  「Must の 5 ソースのうち**退役したものについて**、その名前を引き継いだ…を代わりに数える」。
  運用上は「先に後継の行を作っておいて、切り替え日に旧名を退役させる」が自然なので、
  **後継の行を作った時点で、まだ動いている旧名の達成日が 0 になる**。
  Scenario「退役した名前の代わりに後継を数える」は退役済みの場合しか置いていない。
- kind: technical
- 処置: fixed 17.1
## R3. `GET /coverage` は定数 5 本しか返さないので、後継の格子は画面に出ず、退役の並べ替えも起きない

- 成果物: `crates/server/src/lib.rs:573` / `web/src/App.tsx:104` / `web/src/coverage.ts` の `retiredLast`
- 根拠: `lib.rs:573` は `coverage::must_sources()`（`DEVICE_SUBJECT` + `USAGE_SUBJECT` の定数 5 本）を
  そのまま `of_sources` に渡しており、`resolve_tips` を通していない（`achievement_get` だけが通す）。
  実測: 定数は `c01-location / c01-app-usage / c01-photo / c02-window / c02-browser-history`
  （`coverage.rs:28-30`）。したがって
  1. **後継の名前は `/api/coverage` に現れない** —— 名前を分けた後、達成パネルは後継を数えるのに
     格子は旧名（⑧が並ぶ畳まれた行）しか出さず、**その期間の稼働状況が画面から消える**
  2. `retiredLast` と「既定で畳む」が働くのは**定数 5 本のうちどれかが退役したとき**だけで、
     D36 / ST03 R63 の前提「退役は 1 本きりではなく増えるので Must の 5 本が押し出される」は
     この受け口では起こりえない。`retired-source.test.tsx` が組む「5 本 + 退役 1 本」の配列は
     API が返せない形。
- kind: technical
- 処置: fixed 17.1
## R4. `App.tsx` の `retiredLast` を外しても 43 件が全緑（Scenario は画面で観測されていない）

- 成果物: `web/src/App.tsx:5,104` / `web/src/__tests__/retired-source.test.tsx` /
  `specs/collection-coverage/spec.md:710`
- 根拠: HEAD の複製で `App.tsx` の `retiredLast(sources.value)` を `sources.value` に戻し、
  import も落として `npx vitest run` → `Test Files 12 passed / Tests 43 passed`、`npx tsc -b` rc=0。
  並び順を見ているのは `retiredLast([retired, ...fiveSources(...)])` の**関数直呼び**だけで、
  `App` を描いていない。Scenario の WHEN は「退役したソースを含む稼働状況の**画面を開く**」。
  （なお `app.test.tsx` の「サーバが返した順に格子を並べる」は退役なしの 5 本しか使っていないので当たらない。）
- kind: technical
- 処置: fixed 17.8
## R5. 「4 つとも必要で、1 つ戻すだけで検査が落ちる」は false —— 週の帯の余白だけは戻しても全緑

- 成果物: `web/src/CoverageGrid.tsx:128,152` / `openspec/changes/st02-collection-coverage/design.md` の D31〜D36（D35）/ commit message
- 根拠: HEAD の複製で 4 か所を 1 つずつ戻し、`one-scroll` + `initial-viewport` + `target-size` を走らせた。

  | 戻したもの | 結果 |
  |---|---|
  | `INITIAL_WEEKS` 4 → 5 | 2 failed |
  | `SECTION_PAD_PX` 8 → 12 | 1 failed（5 本目の下端 **1,284** px） |
  | `SECTION_GAP_PX` 12 → 24 | 1 failed（**1,308** px） |
  | **週の帯 `padding: 0` → `1`**（+ セル `minHeight: MIN_TARGET_PX - 2`） | **8 passed** |

  帯の余白を戻すと 4 行 × 5 本で +40 px、下端は 1,236 → 1,276 px で予算 1,280 px に収まる。
  design D35 と commit message の「4 つとも必要だった（1 つ戻すだけで検査が落ちる）」は、
  この 1 つについて成り立っていない。
  あわせて、この 1 か所だけは**見た目の代償がある** —— `gap: 2` は横方向しか効かないので、
  行どうしの余白が 0 になり、同じ段が続く週が縦につながって「1 行 = 1 週」の切れ目が消える。
  そこを見ている検査は無い（`week-order` は DOM の並びしか見ない）。
- kind: technical
- 処置: fixed 17.10
## R6. 移行 0007 の閾値（第 8 回 Q29 の本体）を消しても検査は通る

- 成果物: `migrations/0007_source_lifecycle.sql:62-75` / `crates/server/src/coverage/tests.rs` の `migration_repairs_polluted_started_on` / `specs/collection-coverage/spec.md:261`
- 根拠: HEAD の複製で、引き直しの UPDATE から
  `AND (e.event_time AT TIME ZONE 'Asia/Tokyo')::date >= (src.registered_at …)::date` と
  生存信号側の同じ 2 行を削って `cargo test --lib migration_repairs_polluted_started_on`
  → `test result: ok. 1 passed`。
  検査が置くソースは (a) 登録後の記録 **1 件だけ**、(b) 登録前の信号だけ の 2 本で、
  (b) は後段の「NULL に戻す」UPDATE が単独で救うため、閾値が消えても両方通る。
  **deep.md の実測（正常な信号 2026-04-01 と 1999 年の信号が同じソースに同居する）が再現されていない** ——
  それがこの移行の存在理由そのもの。Scenario の THEN 「登録簿に行ができた日**以降で**いちばん古い
  記録・生存信号の日になる」の「以降で」を観測しているテストが無い。
- kind: technical
- 処置: fixed 17.7
## R7. 閾値の境界（登録日ちょうどに届いた信号）がどのテストからも参照されていない

- 成果物: `crates/server/src/coverage.rs:213` / `specs/collection-coverage/spec.md:181`
- 根拠: HEAD の複製で `touch_started_on` の
  `($2 AT TIME ZONE '{tz}')::date >= (registered_at …)::date` を `>` に変えて
  `cargo test -p ashiato-server --lib` → `test result: ok. 98 passed; 0 failed`。
  spec の逐語は「登録簿にそのソースの行ができた日**より前**の記録・生存信号を除く」なので、
  **登録日当日は含む**が正。`testdb` の既定の登録日が `FAR_PAST = 2000-01-01` で、
  閾値を見る 2 本（`clock_skew_` / `migration_repairs_`）も登録日から 4〜6 日ずらした日付しか使っていない。
  境界を 1 日ずらすと、登録した当日に収集を始めたソースが「まだ開始していない」に落ちるが、
  誰も気付けない。
- kind: technical
- 処置: fixed 17.7
## R8. ひとスクロールの勘定は余地 44 px（3.4 %）しか無く、折り返しを 1 行も見ていない

- 成果物: `web/src/__tests__/one-scroll.test.tsx` / `web/src/tokens.ts:83-91` / `specs/collection-coverage/spec.md:705`
- 根拠: 複製に測定用の出力を足して実測 —— 5 本目の下端 **1,236 px**（予算 1,280、余地 **44 px**）、
  2 本目の格子の下端 **593.5 px**（予算 640、余地 **46.5 px**）、3 本目は 787 px で
  「開いた直後に見える」のは**ちょうど 2 本**。
  `declaredHeight` の `text` は `lineHeight` を **1 行ぶんだけ**足しており、
  **360 px 幅での折り返しを一切数えていない**。達成パネルの表は
  `360 − main の padding 24 − 節の padding 16 = 320 px` に 4 列（ソース名 / 達成日数 / 分母 / 線）を並べる形で、
  1 行でも折り返せば 18.2 px、3 行折り返せば予算を超える。
  テスト冒頭の「**いちばん高くなる形で測る**」も勘定の中でしか成り立たない ——
  `days_until_confirmed: null` の枝の文字列（`確定する日はまだ決まらない（収集を開始していないソースがある: …）`）は
  この勘定では同じ 1 行だが、実寸では 2〜3 行になる。
  Scenario の THEN は「2 画面ぶん（1,280 **CSS px**）以内に収まっている」で、勘定は px を名乗っている。
  （併せて: 予算側の `ONE_SCROLL_PX` / `VIEWPORT_H_PX` は実装が決めた定数なので、
  `tokens.ts` を書き換えれば検査は黙って通る。実測でその改変が別セッションから入っていた。）
- kind: premise
- 処置: escalated
## R9. 状態の出どころを `core.coverage` から `core.event` に移したが、索引が無い（Seq Scan）

- 成果物: `crates/server/src/coverage.rs:385,527` / `migrations/0001_envelope.sql:37-40`
- 根拠: 共有 DB（`ashiato2-db-1`）で新しい問い合わせを `EXPLAIN (ANALYZE, BUFFERS)`:

  ```
  HashAggregate (actual time=8.814..8.825 rows=2)
    ->  Seq Scan on event  (actual time=5.163..8.545 rows=1705)
          Filter: (logical_source = 'c01-location')
          Rows Removed by Filter: 9502
  Execution Time: 10.250 ms          -- core.event は 11,207 行
  ```

  `core.event` の索引は `event_dedup_ext (logical_source, external_id) WHERE …` と
  `event_dedup_hash (logical_source, content_hash)` の 2 本だけで、
  `(user_id, logical_source, event_time)` も `((event_time AT TIME ZONE …)::date)` も無い。
  一方 `core.coverage` は `PRIMARY KEY (user_id, logical_source, day)` を持ち、
  **行数が「日数 × ソース数」で頭打ち**（365 × 5 ≒ 1,825）だった。
  `facts` と `active_days` の両方が移ったので、画面 1 回につき
  **5 ソース × 2 = 10 回の全走査**（+ 達成でもう 5〜10 回）が走り、
  記録が増えるほど直線的に遅くなる。0007 は索引を足していない。
  （`core.coverage` 側も `(NULL::uuid IS NULL OR user_id = $1)` のせいで索引が効いていないが、
  走査する行数の桁が違う。）
- kind: technical
- 処置: fixed 17.3
## R10. 状態は 8 になったのに、画面の Requirement / Scenario / API の説明は「7 状態」のまま

- 成果物: `specs/collection-coverage/spec.md:592,636,672` / `crates/server/src/lib.rs:563` /
  `docs/openapi.json:16,298` / `docs/stories/stories.json` の ST02 `done[2]`
- 根拠: `grep -n "7 状態"` の実測。

  ```
  spec.md:592  **7 状態それぞれの名前で文字で**表示する。
  spec.md:672  - **THEN** その 7 日ぶんの状態が、7 状態それぞれの名前で文字で表示される
  lib.rs:563   /// ソース × 日 の 7 状態を返す（FR-54）。
  openapi.json:16 "summary": "ソース × 日 の 7 状態を返す（FR-54）。"
  stories.json ST02 done[2] 「7 状態の名前で文字で出る」
  ```

  同じ spec の `:344` は「状態は **8 つ**のいずれかに決まる」に直っており、
  `STATE_NAME` も 8 件、`retired-source.test.tsx` が⑧を週の詳細で確かめている。
  `check-openapi.sh` は欄名しか見ないので通る。**8 番目だけが Requirement の本文から漏れている**ので、
  「週を選べば 8 状態すべてが名前で読める」という主張がどこにも正典として書かれていない。
  （`stories.json` は `check_chain.py` の再生成元なので、直す場所はそこ。）
- kind: technical
- 処置: fixed 17.9
## R11. `tools/smoke.sh` は独立に確認できなかった（この環境では別プロセスが同じ DB を書いている）

- 成果物: `tools/smoke.sh` / `docker-compose.yml`
- 根拠: `docker stop ashiato2-db-1` のうえで 2 回走らせた。

  ```
  1 回目 rc=7  Error: マイグレーション 0002_immutable_collected の適用に失敗
               duplicate key value violates unique constraint "pg_proc_proname_args_nsp_index"
  2 回目 rc=1  == 6. 取り出す → 842 件 / 「1 件のはずが 842 件」
  ```

  原因を切り分けた: `docker compose down -v` の直後に**空の** DB を上げて放置し、5 秒おきに数えると

  ```
  2026-09-11 12:19:35+00 | core のテーブル数 6   ← 誰も触っていないのに増える
  ```

  `pg_stat_activity` に `172.24.0.1` から 24 本の接続、`core.event` に `t-*`（テスト用の
  `logical_source`）が 3,163 行。`ps` で `/home/yosis/dev/ashiato2` を cwd に持つ
  `target/debug/ashiato-server`（9/10 起動・`BIND=100.85.27.45:18787`・
  `DATABASE_URL=…:55432/ashiato`）が生きており、別 worktree の `cargo test` も同じ port を使っている。
  **したがって rc≠0 はこの commit の欠陥ではなく環境由来**で、申告の rc=0 を否定する材料にはならない。
  ただし `smoke.sh` が「port 55432 と compose project を独占している」ことを前提に
  **手順 6 の件数で合否を出す**構造は変わっておらず（`review/code.md` の R14 と同じ穴）、
  この commit はそこに手を入れていない。**CI 以外では再現しない検査**であることは記録しておく。
- kind: premise
- 処置: escalated
## R12. 引き継ぎの鎖は輪にできる。回り続けはしないが、32 段を超えると黙って根に届かない

- 成果物: `migrations/0007_source_lifecycle.sql:36-46` / `crates/server/src/coverage.rs:237,248-258,293-312`
- 根拠: 複製で実測。`A.succeeds = NULL` → `B.succeeds = A` → `UPDATE A SET succeeds = B` が
  **エラーにならず通る**（`PROBE-H3 輪を作れたか: Ok(1)`）。CHECK は
  `succeeds IS DISTINCT FROM logical_source` だけなので 2 本以上の輪は塞げない（移行のコメントも
  そう書いている）。**回り続けはしない** —— `depth < 32` で止まり `b.started = Some(2026-05-01)` が返った。
  問題は打ち切り方で、`CHAIN_MAX_DEPTH` を超えた鎖は**例外も警告も無しに**
  「そこまでの min」を根として返す（`sources`）／「32 段目のノード」を先端として返す（`resolve_tips`）。
  成功条件 1 の窓の起点がその場で縮むが、値だけを見ても縮んだと分からない。
  32 という値を固定しているテストも無い（1 でも 2 世代の検査は通る）。
- kind: technical
- 処置: fixed 17.5
## R13. `deep.md` の Q31「列そのものは ST03 が作る」が実装と食い違ったまま残っている

- 成果物: `openspec/changes/st02-collection-coverage/deep.md`（第 8 回 Q31 の「効く先」）/
  `openspec/changes/st02-collection-coverage/design.md` の D32 / `migrations/0007_source_lifecycle.sql`
- 根拠: `deep.md` の Q31 は「**列そのものは ST03 が `record-envelope` 側で作る**（FR-61）」と書いている。
  実装は `migrations/0007_source_lifecycle.sql` で **ST02 が** `retired_on` / `succeeds` を作り、
  design D32 と tasks 14 で「2026-09-11 の判断」として覆している（ST03 design D7 が根拠）。
  この commit は `deep.md` のその行を触っていない（`git show 351fb98 -- …/deep.md` の変更は
  第 8 回の見出しと未回答の印だけ）。
  **`deep.md` は本人の答えを逐語で残す文書**なので、本人の答えではない注記（「効く先」）に
  誤りが残ると、次に読む人が正典として読む。
- kind: premise
- 処置: escalated
---

## 走らせていないもの

- **Android（gradle）** —— この環境に JDK が無い（`which java` が空）。
  この commit が `collector-android/` を 1 バイトも触っていないことは `git show --stat` で確認した。
- **実寸のレイアウト（実ブラウザ）** —— jsdom は測らない。R8 はそのぶんの余地を数えたもので、
  実寸の判定は `tasks.md` の「人間の確認待ち」が持つ。
