# ST19 実装の独立検証 — review/code.md

対象: `feat/st19-personal-attributes`（`f1b4a9d`）。`git diff origin/main` の 24 ファイル。
実行環境: `COMPOSE_FILE=docker-compose.yml:/tmp/st19-db-override.yml`（DB は 127.0.0.1:55419）、
サーバは `BIND=127.0.0.1:18739` / `18749`（18719・18787 は別プロセスが占有していた）。
**コードは触っていない。** 手 2 / 手 4 の「わざと壊す」はすべて複製（`git archive` の別ツリー）か、
壊して → 測って → `git checkout --` で戻す形で行い、最後に `git status --short` が空であることを確かめた。

## 申告と実測

実装者の申告: tasks 21 件中 19 件 `[x]`（7.4 / 7.5 は PR 本文）・Scenario 106 本すべてに印・
`cargo test --workspace` 366 件と `web` 114 件が緑・検査一式 rc=0。

| 検証コマンド（申告の根拠） | 申告 | 実測 |
|---|---|---|
| `cargo test --workspace` | 366 件緑 | rc=0 / **366 passed**（86 + 280） → **一致** |
| `cd web && npx vitest run` | 114 件緑 | rc=0 / **114 passed**（18 ファイル） → **一致** |
| `cargo fmt --all --check` | rc=0（tasks 7.1） | **rc=1** → **不一致**（R1） |
| `cargo clippy --workspace --all-targets -- -D warnings` | rc=0 | rc=0 → 一致 |
| `cd web && npm run lint` / `npm run build` | rc=0 | rc=0 / rc=0 → 一致 |
| `tools/check-immutable.sh` | rc=0 | rc=0（ST19 の段 15 件すべて OK） → 一致 |
| `tools/check-openapi.sh` / `check-migrations.sh` / `check-boundaries.sh` / `check-licenses.sh` | rc=0 | 4 本とも rc=0 → 一致 |
| `tools/smoke.sh` | rc=0 | rc=0（42 段目まで通過） → 一致 |
| `tools/seed.sh normal` ×2 → `jq -e '... == 21 and 5'` | rc=0 | rc=0 / rc=0 / `true` → 一致（冪等も確認） |
| `python3 scripts/check_scenarios.py . st19-personal-attributes` | rc=0（106 本） | rc=0（全体 350 件中 担保 349 / `wait` 1 は ST04 側） → 一致 |
| `python3 scripts/check_chain.py .` / `openspec validate --strict` | rc=0 | rc=0 / rc=0 → 一致 |
| tasks の `CT` 絞り込み 8 本 | 件数つき | `parse` 6 / `view` 12 / `kinds` 7 / `read` 11 / `ingest` 12 / `store` 7 / `erasure` 2 / `migration_applies_twice` 1 —— **すべて 1 本以上。0 本のまま `[x]` のタスクは無い** |
| tasks の `VT` 対象 6 ファイル | — | `attributes` 14 / `master-view` 12 / `master-form` 15 / `master-view-limits` 5 / `app` 8 / `day-view` 18 —— **すべて実在・緑** |

**捏造も空テストも無い。** 不一致は `cargo fmt` の 1 件（R1）。残りの指摘は「テストが 0 本」ではなく
**「主張の階層とテストの階層が違う」**か、**spec が想定していない経路**に出た。

---

## R1. `cargo fmt --all --check` が rc=1。tasks 7.1 の `[x]` は成り立たず、CI はこの手前で落ちる

- 成果物: `crates/server/src/attributes_tests.rs` / `crates/server/src/stay_tests.rs` / `openspec/changes/st19-personal-attributes/tasks.md`（7.1）
- 根拠:
  ```
  $ cargo fmt --all --check ; echo rc=$?
  Diff in /home/yosis/dev/ashiato2-st19/crates/server/src/attributes_tests.rs:200
  Diff in /home/yosis/dev/ashiato2-st19/crates/server/src/stay_tests.rs:43
  rc=1
  ```
  2 か所とも **この change が触った行**（`attributes_tests.rs:200` の `assert!(mine > gates, ...)` と
  `stay_tests.rs:43` の `crate::MIGRATIONS.iter().any(...)`）。
  `.github/workflows/ci.yml:35` が `cargo fmt --all --check` を走らせるので、**PR を出せば CI はここで赤**になる
  （`gh pr list --head feat/st19-personal-attributes` は `[]`。まだ PR が無いので「CI 全緑」は実測されていない）。
  作業ツリーは clean（`git status --short` が空）なので、未フォーマットのまま commit 済み。
- kind: technical
- 提案: `cargo fmt --all` を当てて 1 コミット足す。tasks 7.1 は fmt を通してから `[x]` に戻す。
- 処置: fixed 7.1 —— `cargo fmt --all` を当てた。`cargo fmt --all --check` rc=0 を確認。申告が間違っていた —— 直前の編集（clippy の指摘と、移行の位置の assertion）の後にfmt を当て直しておらず、古い実行結果で 7.1 を `[x]` にしていた。PR 本文で訂正する。

## R2. Q3（本人が選んだ既定の感度＝ローカル AI まで）を固定しているテストが 1 本も無い。2 を 1 に変えても 366 件全部緑

- 成果物: `crates/server/src/attributes.rs:22`（`pub const DEFAULT_SENSITIVITY: i32 = 2;`）/ `crates/server/src/attributes_tests.rs:1278-1332`（`ingest_default_sensitivity`）/ `:1337-1352`（`default_sensitivity_matches_the_column`）
- 根拠: 定数だけを書き換えて全テストを走らせた。
  ```
  $ sed -i 's/pub const DEFAULT_SENSITIVITY: i32 = 2;/pub const DEFAULT_SENSITIVITY: i32 = 1;/' crates/server/src/attributes.rs
  $ cargo test -p ashiato-server
  test result: ok. 280 passed; 0 failed
  ```
  （確認後 `git checkout --` で戻した。`git status --short` は空）
  理由は、Scenario`主張はローカル AI までで格納される` の検査が
  `assert_eq!(i32::from(s), attributes::DEFAULT_SENSITIVITY)`（`:1298-1301`）と
  **実装の定数そのものと比べている**こと。`default_sensitivity_matches_the_column` も
  `got.starts_with(&DEFAULT_SENSITIVITY.to_string())` で、DB の列の既定（`lib.rs:301` の `1`）と
  照らしているだけなので、2 → 1 にすると両方が同時に真になる。
  `tools/smoke.sh` の 42 段目も主張の `sensitivity` を見ていない（`git diff origin/main -- tools/smoke.sh` で確認）。
  spec は「感度『ローカル AI まで』で格納する」と値を名指ししており、`deep.md` Q3 は本人が明示的に選んだ答え。
  **値を変えても全部通る状態**なので、ST24 が感度の操作を足すときに黙って緩む側へ落ちても誰も気付かない。
- kind: technical
- 提案: `ingest_default_sensitivity` の期待値をリテラル `2` にする（`NFR18_TEXT = 4.5` を
  `master-view-limits.test.tsx:21` でリテラルに持っているのと同じ作法）。
  `default_sensitivity_matches_the_column` は「**主張の**既定（2）と**主張以外の**既定（1）が**違う**こと」を
  リテラルで見る向きに変える。
- 処置: fixed 8.7 —— `ingest_default_sensitivity_is_pinned_to_two` を足し、リテラル `2`（PERM-2 の 4 段階の 3 番目）と突き合わせる。定数を 1 に変えると落ちる。

## R3. `tz_offset_min` が範囲外の主張を `/ingest` が受理し、`GET /attributes` から無言で消える。錠のせいで二度と出せない

- 成果物: `crates/server/src/attributes_store.rs:286-291`（`stored_claim_of` の `FixedOffset::east_opt(...)?`）/ `:283`（`rows.into_iter().filter_map(stored_claim_of)`）/ `crates/server/src/lib.rs`（`/ingest` に `tz_offset_min` の範囲検査が無い）/ `migrations/202609160220_personal_attributes.sql`（`core.reject_claim_delete`）
- 根拠: 走っているサーバ（`127.0.0.1:18749`）へ、形は正しく `tz_offset_min` だけ 100000 の主張を 1 件送った。
  ```
  ingest: 200 [{'id': '3f346d60-...', 'duplicate': False, 'accepted': True, 'error': None}]
  GET /attributes → claims: []  current: None
  ```
  行は DB に残っている:
  ```
  $ psql -c "SELECT id, tz_offset_min, left(raw,60) FROM core.event WHERE id='3f346d60-...';"
  3f346d60-5ea9-4928-acb8-eb45e4726606|100000|{"claim":"3f346d60-...","nonce":"bK5...
  ```
  そして消せない:
  ```
  $ psql -c "DELETE FROM core.event WHERE id='3f346d60-...';"
  ERROR:  主張は行ごと消せない（FR-44 / 深掘り C1）。消すなら削除の印か台帳つきの消去
  ```
  サーバのログにこの取り込みについての `WARN` / `ERROR` は 1 行も出ない
  （`grep "WARN\|ERROR" /tmp/st19-server2.log` は私が投げた 401 の 3 行だけ）。
  `stored_claim_of` は読めない行を `None` にし、`filter_map` が**黙って落とす**。
  `/attributes` の応答にも件数にも痕跡が残らない。
  同じ経路は `claim_from_payload`（`:322-353`）が `None` を返す行——`payload` に `kind` /
  `valid_from` / `precision` が無い行——にも効く。主張は `core.event` に **INSERT なら誰でも作れる**
  （錠が止めるのは UPDATE / DELETE / TRUNCATE だけ）ので、psql から 1 行入れれば同じ状態を作れる。
  **画面（`web/src/attributes.ts:185`）は `-new Date().getTimezoneOffset()` を送るので本人の操作では起きない**が、
  `/ingest` は公開された契約であり、seed・将来の収集側・時計のずれた端末からは届く。
  spec の「形の合わない主張は受け付けない」の表に `tz_offset_min` の行が無く、
  「種類ごとのいまの値と履歴を読める」も「読めない行をどうするか」を決めていない（＝ spec の隙間）。
- kind: technical
- 分類の直し: `code-verify` は `kind: loss` と書いたが、**`loss` は kind ではなく別の欄**（`uncaptured` / `discarded` / `exported` / `rewrite-all`）。中身も A ではない —— 値は `raw` に残っていて失われておらず、**「範囲外の地域のずれを受け付けるか」に本人へ示す選択肢が無い**（規則で決まる側）。technical として直した。
- 提案: `/ingest` の主張の分岐で `tz_offset_min` を `-1440..=1440` に絞って `malformed_claim` で断る
  （受け取る前に止めれば、消せない行が生まれない）。あわせて `stored_claim_of` が `None` を返した件数を
  `warn!` で件数だけ出す（値は出さない。tasks 0 章）。spec の表に 1 行足す。
- 処置: fixed 8.1 —— 取り込み口の主張の分岐で `tz_offset_min` を `-1439..=1439` に絞り、外れたら `malformed_claim` で断る（`ingest_rejects_out_of_range_tz_offset`）。併せて `stored_claim_of` の掛け算を `checked_mul` にし、落とすときは `tracing::error!(kind = "attributes_claim_unreadable")` で叫ぶようにした。`loss` だが人間へ返していない —— 値は `raw` に残っていて失われておらず、「範囲外の地域のずれを断る」に本人の判断を要する幅が無いため（B でもなく、規則で決まる側）。

## R4. `today_jst()` を UTC に変えても全テストが緑。`/attributes` が `Asia/Tokyo` で日を切ることは、どのテストからも観測されていない

- 成果物: `crates/server/src/lib.rs:1540-1549`（`today_jst`）/ `:1572`（`attributes_get`）/ `crates/server/src/attributes_tests.rs:493-527`（`read_today_is_asia_tokyo`）
- 根拠:
  ```
  $ # today_jst() の本体を chrono::Utc::now().date_naive() に置き換えて
  $ cargo test -p ashiato-server
  test result: ok. 280 passed; 0 failed
  ```
  （確認後に戻した。`git status --short` は空）
  Scenario `今日は Asia/Tokyo の日付で決まる` の検査は `attributes_store::attributes_view(&app.pool, u, today)` を
  **直に呼び、`today` を自分で `stay_store::jst_date(at)` から作って渡している**（`:517-521`）。
  つまり見ているのは `jst_date` の正しさ（ST16 の持ち物）だけで、
  **`GET /attributes` のハンドラがそれを使っていること**は誰も見ていない。
  spec の Scenario は「UTC で 2026-09-30T16:00 に**読み出し**」と読み出しの側を主張しており、
  テストの階層（store の関数）と主張の階層（HTTP の読み出し）がずれている。
  テスト自身の doc コメントが警告している「UTC で日を切っていると 9 時間だけ『予定』のまま」が、
  まさに素通りする。
- kind: technical
- 提案: `today_jst()`（または時刻を差し込める形の同等物）を通る経路で 1 本足す ——
  `attributes_get` に「いまの時刻」を注入できるようにするか、`today_jst` を
  `fn today_jst_at(now: DateTime<Utc>)` に割って `attributes_get` から呼び、
  `today_jst_at("2026-09-30T16:00:00Z") == 2026-10-01` を固定する。
- 処置: fixed 8.7 —— `App` に試験用の時刻の口（`App::at`）を足し（tasks 2.4 が「試験は時刻を差し込めるようにする」と書いていたのを、store の高さでしか満たせていなかった）、`read_handler_cuts_the_day_in_asia_tokyo` がハンドラを通して UTC 16:00 → JST 翌日を確かめる。`today_jst()` を UTC にすると落ちる。

## R5. 2 つの主張が同じ主張を取り消したとき、`superseded_by` は片方しか返らない（`HashMap` の上書き）

- 成果物: `crates/server/src/attributes.rs:376-380`（`superseded_by` の `HashMap` への `collect`）/ `crates/server/src/attributes_tests.rs:1064-1099`（`ingest_allows_two_claims_to_supersede_one`）
- 根拠: 走っているサーバに、同じ主張 `T` を取り消す主張 `X1` / `X2` を 2 件入れて 3 回読み出した。
  ```
  claims    : ['U2', 'U1', 'X2', 'X1', 'P3', 'P2', 'P1']
  superseded: [('T', 'X1')]      ← 3 回とも X1 だけ。X2 は落ちる
  ```
  `live.iter().filter_map(|c| c.claim.supersedes.map(|t| (t, c.claim.id))).collect::<HashMap<_,_>>()` は
  **同じ鍵を後勝ちで上書き**するので、どちらが返るかは `core.event_live` の行の並び（`ORDER BY` 無し。
  `attributes_store.rs:274-278`）が決める。
  spec は「1 つの主張を複数の主張が取り消すことを、その理由では断らない」と
  「どの主張で取り消されたかを添えて返す」を両方言っているのに、Scenario
  `1 つの主張を 2 つの主張が取り消せる` の検査は **`/ingest` が 2 件とも受理すること**までしか見ていない
  （`:1098` の `assert!(res.accepted)` で終わり）。読み出し側の落ちは誰も観測していない。
- kind: technical
- 提案: 設計としてどちらを返すか決める（例: 主張した日時が最も新しいもの）。
  `claims_of` に `ORDER BY` を足して並びを決め、`superseded_by` を `Vec` にするか
  「最も新しい取り消し」を明示的に選ぶ。Scenario を 1 本足して読み出し側で固定する。
- 処置: fixed 8.3 —— `claims_of` に `ORDER BY ingest_time, id` を足して入力の順を固定し、`view` は並べてから 最初の取り消しを勝たせる（`entry().or_insert()`）。`read_two_supersessions_are_deterministic` が 10 回読んで同じ答えになることを見る。spec は「どの主張で取り消されたかを添えて返す」としか書いていないので、どちらでもよいが決まっていることを固定した。

## R6. 画面の Scenario「積んだ主張はいつからの新しい順に出る」を、既に並んだ配列を渡すテストが受け持っている

- 成果物: `web/src/__tests__/master-view.test.tsx:104-118` / `openspec/changes/st19-personal-attributes/specs/personal-entities/spec.md`（同 Scenario）
- 根拠: spec の WHEN は「『いつから』が **2013 年 4 月・2023/3/18・2017 年 4 月** の主張を持つ種類を表示する」、
  THEN は「2023/3/18・2017 年 4 月・2013 年 4 月**の順に並ぶ**」。
  テストの入力は
  ```
  claim({id:"c1", value:"C", valid_from:{precision:"day",   date:"2023-03-18"}}),
  claim({id:"c2", value:"B", valid_from:{precision:"month", date:"2017-04"}}),
  claim({id:"c3", value:"A", valid_from:{precision:"month", date:"2013-04"}}),
  ```
  で**すでに出力の順**。テストの doc コメント自身が「並べ替えはサーバが持つ（画面は受け取った順に描く）」と
  書いているとおり、この検査が観測しているのは**素通し**であって並べ替えではない。
  並べ替えそのものは `attributes::tests::view_stacked_includes_current_and_upcoming` が固定しているので
  穴は塞がっているが、**この Scenario の印が刺さっている先は、その主張を見ていない**。
  （なお、私が独立に組んだ仕様どおりの導出と実装の出力は 11 件の組み合わせで一致した。手 1 の結果は下に書く。）
- kind: technical
- 提案: 印を `attributes.rs` の `view_stacked_includes_current_and_upcoming` にも置く（複数箇所可）か、
  画面のテストの入力を spec の WHEN どおり（2013 → 2023 → 2017）にして「受け取った順に描く」ことを
  明示的に固定する（どちらを主張としたいかを決める）。
- 処置: fixed 8.7 —— `view_stacked_includes_current_and_upcoming`（`attributes.rs`）にも同じ Scenario の印を置き、並べ替えそのものを持つ階層で担保する。画面側の印は「サーバが返した順にそのまま描く」を見るものとして残す。

## R7. `.down.sql` は主張が 0 件なら種類の 2 表を落とす。本人が足した種類は、主張を書く前なら戻しで消える

- 成果物: `migrations/202609160220_personal_attributes.down.sql:22-38` / `openspec/changes/st19-personal-attributes/tasks.md`（1.1 の検証）/ `tools/check-immutable.sh`（ST19 の戻しの段）
- 根拠: まっさらな DB（`scratch`）に全版を当て、**主張は 0 件のまま、種類を 1 つ足して**から戻しを当てた。
  ```
  $ psql -d scratch -c "INSERT INTO core.attribute_kind ...; INSERT INTO core.attribute_kind_name ... '本人が足した種類';
                        SELECT count(*) FROM core.attribute_kind_name;"
  1
  $ psql -d scratch -v ON_ERROR_STOP=1 < migrations/202609160220_personal_attributes.down.sql ; echo rc=$?
  rc=0
  $ psql -d scratch -c "SELECT count(*) FROM core.attribute_kind_name;"
  ERROR:  relation "core.attribute_kind_name" does not exist
  ```
  down.sql の注記は「主張が 1 件も無いときだけ、2 表と登録簿の行が落ちる（＝**当てたが使わなかった場合**）」と
  書いているが、**種類を足した時点で使われている**ので前提が事実と違う。
  tasks 1.1 の検証も `tools/check-immutable.sh` の段も、**主張が残っている側の枝**しか当てていない
  （`  OK 主張が残っていれば、戻しても種類の 2 表と登録簿の行と主張が残る`）ので、この枝はどこも通っていない。
  種類の名前は追記のみの台帳で、落とすと「その識別子が何という名前だったか」が永久に失われる ——
  down.sql が主張のために守っているものと同じもの。
- kind: technical
- 分類の直し: 同上。戻しで種類が消えるのは確かに失われる向きだが、**「戻しで本人の成果物を消さない」は扉の既定（捨てるより印を付けて入れる）が決める側**で、本人に 2 つの選択肢を示す判断ではない。technical として、消さない側へ直した。
- 提案: 条件を `core.event` の主張 **または** `core.attribute_kind` の行のどちらかが残っていれば残す、に広げる。
  変えないなら down.sql の注記を事実に直し（「種類を足していれば消える」）、
  `check-immutable.sh` にその枝を当てる段を 1 つ足す。

---
- 処置: fixed 8.6 —— `.down.sql` の条件を「主張が残っている または 種類が残っている」に広げた。`design.md` D12 の本文も直した。`loss` だが人間へ返していない —— 「戻しで本人の成果物を消さない」は扉の既定（捨てるより印を付けて入れる）が決める側で、本人に選択肢を示す性質の判断ではないため。

## 実行して**問題が出なかった**もの（手ごとの結果）

**手 1（固定値の独立再計算）— 不一致なし。**

- `content_hash`。design の「SHA-256(ソース, 出来事の時刻, 原文)」を Python（`hashlib` + `struct`）で
  長さ前置きの組み立てまで書き直し、DB に入っている主張 1 行から計算した。
  `60b200cdd6612b8cc5e3ff51c09d96662dea79d60229fc1686b49998228a929d` が
  格納値と**一致**。Scenario `消去後に残る列と正しい値から鍵を作り直せない` が
  「乱数を知らないから一致しない」を見ていることが、逆向き（乱数を知っていれば一致する）から裏づけられた。
- 初期の種類の識別子（`attributes_store.rs:53` の `Uuid::new_v5(&user_id, slug)`）。
  Python の `uuid.uuid5(UUID(user), "address" / "job")` と、走っているサーバが返した識別子が
  `ddd9edfc-7c6d-5cd1-b038-748a9ceee2bb` / `f74532bb-944e-53ac-9814-e3587f5cf3b5` で**一致**。

**手 2（ガードをわざと壊す）— すべて期待どおり落ちた。**

| 壊したもの | 結果 |
|---|---|
| 移行の門から `l.event_id = NEW.id` を外す（tasks 1.2 の指定） | `tools/check-immutable.sh` rc=1。`NG 別の記録の台帳 1 行で主張が消去できた（門が「その主張の」を見ていない）` |
| 門の `is_erasure` から `NEW.payload = '{}'::jsonb` を外す | rc=1。`NG 消去の顔で解析済みを差し替えられた（原文は消えているので引き直せない）` |
| `parse_claim` の `payload` を原文そのまま（`nonce` を除かない）にする | `attributes_tests::erasure_nonce_is_not_copied_to_the_payload` FAILED |
| `newNonce()` を乱数でなく固定値にする | `attributes.test.ts` の「乱数は毎回違い、識別子とも一致しない」FAILED |

**消去の門は「その主張の台帳」と「消去の形」の両方を見ている**（2 本とも独立に壊して確かめた）。
`core.reject_claim_delete()` も実測で効いている（R3 の DELETE が拒まれた）。

**手 3（Scenario と test の突合）— 106 本すべてに印があり、印の先は実在する。**
中身まで見て階層がずれていたのは R2 / R4 / R5 / R6 の 4 件。
`invalid_supersedes` の 5 Scenario（無い / 自分自身 / 主張でない記録 / 別の利用者 / 別の種類）は
すべて `assert_eq!(..., json!("invalid_supersedes"))` で種別まで見ており、種別 7 種すべてに
リテラルの `assert_eq!` がある。`主張の原文が 1 バイトも変わらずに残る` は
`raw` 列（`text`）を読み戻して `stored.as_bytes() == raw.as_bytes()` で比べており、
**ST01 で問題になった「列の型で主張が崩れる」型ではない**。

**手 4（本人の決定が test で固定されているか）— Q1 / Q2 / Q4 は固定されている。Q3 だけ固定されていない（R2）。**
画面（Q2）の 5 軸を 1 つずつ壊して確かめた:

| 壊したもの | 落ちたテスト |
|---|---|
| 書いた日時を常に出す（`{open && (` → `{true && (`） | `主張した日時は押す前は見えず、押すと見える` FAILED |
| 訂正の取り消しを畳まない（`{supOpen && (` → `{true && (`） | `取り消された主張は「訂正で取り消した 1 件」に畳まれ、押すと出る` FAILED |
| 積んだ主張を 3 件に切る（`kind.claims.slice(0,3)`） | `**積んだ主張を常に全部**、押さずに見せる` FAILED |
| 精度で絞らず全部の欄を出す | `選んだ精度の欄だけが出る` FAILED |
| `NONCE_MIN_CHARS` 22 → 11 | `parse_rejects_short_nonce` / `ingest_rejects_short_nonce` FAILED |

「カードごとに『書く』1 つ」は、テストが `getByRole("button", { name: "書く" })`（単数形。複数一致で例外）を
使っているので **proto の初期値（「変わった」「間違いを直す」の 2 つ）に戻せば落ちる**。
**下流で Q2 の構造が勝手に変わった形跡は無い。** 画面が長い側（畳まない）を保つコメントも
`MasterView.tsx:40-44` に残っている。

**手 5（`[x]` と実体）— 0 本のまま `[x]` のタスクは無い。**
`CT` 絞り込み 8 本と `VT` 6 ファイルを 1 件ずつ走らせ、件数と存在を上の表に書いた。
tasks に書かれた検証コマンドも個別に走らせた:
`git diff --stat origin/main -- migrations/202609120944_gates.sql` 空 /
`grep -c '"/attributes/kinds"' docs/openapi.json` = 1 / `grep -c '"/attributes"'` = 1 /
`grep -nE "#[0-9a-fA-F]{3,8}\b|rgb\(|hsl\(" web/src/MasterView.tsx` 0 件 /
6.1 の 3 本の `grep` 連鎖すべて rc=0。**唯一落ちたのが 7.1 の `cargo fmt`**（R1）。

**手 6（隙間）— R3 と R7 が出た。** ほかに当たって出なかったもの:
資格情報なしの `/attributes`・`/attributes/kinds`・`/attributes/kinds/{id}/names` は 3 本とも 401（Fake で素通りしていない）。
サーバのログに主張の値・補足・種類の名前は 1 度も出ない（`grep -c "東京都\|目黒\|北海道" /tmp/st19-server*.log` = 0。tasks 0 章）。
消去した主張・削除の印の付いた主張は読み出しのどこにも出ず、その取り消しも効かない（実装・テストとも一致）。
`GET /attributes?user_id=` を呼び出し元が名乗れる件は **ST29 の担当**（`deep.md`「確かめたが問わなかったこと」/
`docs/production-prep.md:167`）なので ST19 の指摘にしない —— `kind: defer`。

**独立再計算した `view()` の導出（要望 3）— 実装と一致した。**
spec「種類ごとのいまの値と履歴を読める」の 5 つの SHALL（比較の鍵・年/年月の丸め・「分からない」は最も古い側・
今日以前で最も新しいものが「いまの値」・同じ「いつから」は主張した日時が後・積んだ主張は新しい順で
「分からない」が最後・予定は古い順）を Python で書き直し、走っているサーバへ実際に主張を入れて突き合わせた。

- 11 件（`unknown` / `year` / `month` / `day` / 同じ「いつから」2 件 / 今日ちょうど / 未来 2 件 / 訂正の連鎖 B→A・C→B）:
  `current` / `upcoming` / `claims` / `superseded` の 4 つとも**一致**。
- 追加の 8 件（同じ「いつから」3 件の同着・同じ未来の日 2 件・1 つの主張を 2 件が取り消す）:
  `claims` は `P3, P2, P1`（主張した日時の降順）、`upcoming` は `U1, U2`（「いつから」の昇順）で**一致**。
  ここで唯一落ちたのが `superseded_by` の片落ち（R5）。

---

# pr-review-toolkit の指摘（R8 以降）

`code-reviewer` / `silent-failure-hunter` / `pr-test-analyzer` の 3 本を `git diff origin/main` に対して
走らせ、**R1〜R7 と重なるものは上へ寄せて**（重なりを下に明記）、残りを R 番号で写した。
3 本とも「指摘を出すだけで直さない」側で走らせている。

**R1〜R7 と重なったもの**:
`code-reviewer` の 1（`ORDER BY` が無い）と `silent-failure-hunter` の MEDIUM-7、`pr-test-analyzer` の F15 → **R5** /
`code-reviewer` の 2 と `silent-failure-hunter` の CRITICAL-1 → **R3** /
`pr-test-analyzer` の F8（黙って消える経路にテストが無い）→ **R3 / R12** /
`pr-test-analyzer` の F6（画面が予定を二重に判定）と `silent-failure-hunter` の LOW-11(b) → **R10**。

## R8. 種類の口の 400 の本文が、OpenAPI の宣言と食い違う（形も Content-Type も）

- 出所: `code-reviewer` 3（確信度 85）
- 成果物: `crates/server/src/lib.rs`（`kind_error_body`）/ `docs/openapi.json`
- 根拠: 宣言は `body = KindError`（= 裸の文字列 `"empty_name"`）だが、実物は `{"error":"empty_name"}`。
  さらに `impl IntoResponse for String` が `text/plain` を付けるので、`application/json` と宣言しているのに
  JSON では返らない。`tools/check-openapi.sh` は欄の名前しか見ないので通ってしまう。
  ST20 / ST21 が同じ口を引き継ぐ（proposal「S-6 の骨格を引き継ぐ」）ので、契約のずれはここで閉じるのが安い。
- kind: technical
- 処置: fixed 8.4 —— `KindErrorBody { error: KindError }` を切って `Json` で返し、`body =` をそれに合わせた。 `kinds_post_rejection_body_shape` が `{"error":"duplicate_name"}` と `{"error":"unknown_kind"}` を固定する。 `docs/openapi.json` を再生成（`tools/check-openapi.sh` rc=0）。

## R9. 種類の口だけ「断られた」と「届かなかった」が潰れている —— 500 も 401 も「その名前は重なっています」になる

- 出所: `silent-failure-hunter` CRITICAL-2 / `code-reviewer` 4 / `pr-test-analyzer` F11
- 成果物: `web/src/MasterView.tsx`（種類を足す・名前を変える）/ `web/src/attributes.ts`
- 根拠: `if (!res.ok) return false;` の 1 行で、400・401・500・502・404 がすべて
  「その名前は使えません（空か、いまある名前と重なっています）」になる。サーバは
  `{"error":"empty_name"|"duplicate_name"|"unknown_kind"}` を 3 種類返しているのに本文を一度も読まない。
  とくに `unknown_kind`（種類が無い）で「重なっています」と出るので、**本人は名前を変え続けて一生通らない**。
  **同じ PR の `readIngestResponse` が、まさにこの形を名指しで禁じている**（design D9 / spec-review R19）——
  主張の口では規律が効き、種類の口では逆をやっていた。テストも 400 しかモックしておらず、500 を足しても緑のまま。
- kind: technical
- 処置: fixed 8.4 —— `attributes.ts` に `readKindResponse` と `kindRejectionMessage` を置き、 `readIngestResponse` と同じ構造にした。400 は種別ごとの文、それ以外は `UNREACHABLE_MESSAGE`。 テストは 500 / 401 / 502 を回して「名前のせいにしない」ことと、400 の 3 種別が違う文になることを見る。

## R10. 「予定かどうか」の規則が画面にもあり、サーバと 2 か所に割れている

- 出所: `code-reviewer` 5 / `silent-failure-hunter` LOW-11(b) / `pr-test-analyzer` F6
- 成果物: `web/src/MasterView.tsx`（`ClaimRow`）
- 根拠: サーバは `ValidFrom::key()` と今日の比較で `upcoming` を組んで応答に入れているのに、
  画面は `claim.valid_from.date > today` の**文字列比較**で独立にもう一度判定していた。
  D6（仮）の反転条件（年をその年の初めから有効とみなすのをやめる、など）が満たされたとき、
  サーバだけ直すと画面の「（予定）」が黙ってずれる。`attributes.rs` の冒頭 doc と、
  同じ diff で `today_jst()` を `stay_store::jst_date` へ寄せた判断に真っ向から反する。
  `pr-test-analyzer` の実測: `const future = false` に置き換えても web 114 件が全部緑（無検査）。
- kind: technical
- 処置: fixed 8.5 —— `KindCard` が `kind.upcoming` の識別子の集合を作って `ClaimRow` に渡し、 画面は判定し直さない。`行の「（予定）」はサーバの upcoming に従う` を足し、 「いつから」が今日より前なのにサーバが予定と言う主張で、規則が画面にあれば落ちる形にした。

## R11. 「最も新しく書いた主張」を RFC 3339 の文字列比較で選んでいる

- 出所: `code-reviewer` 6（確信度 80）
- 成果物: `web/src/MasterView.tsx`（`WriteForm` の `newest`）
- 根拠: `asserted_at` は地域のずれつき（`…+09:00`）で、C4 のとおり地域は端末のものなので
  本人が移動すれば主張ごとに違うオフセットが混ざる。`2026-09-15T01:00:00+09:00`（= 14 日 16:00Z）と
  `2026-09-14T20:00:00-05:00`（= 15 日 01:00Z）は辞書順と絶対時刻の順が逆になり、
  **後に書いた主張でないものが「取り消す主張」の既定に選ばれる**。
  spec「取り消す主張は最も新しく書いた主張が選ばれている」に反する。
- kind: technical
- 処置: fixed 8.5 —— `Date.parse()` で絶対時刻に直して比べる。

## R12. 読み出しが 2 段階で行を落とすのに、件数を数えず、ログも出さない

- 出所: `silent-failure-hunter` HIGH-3 / `pr-test-analyzer` F8
- 成果物: `crates/server/src/attributes_store.rs`（`claims_of` / `stored_claim_of` / `kinds_of`）
- 根拠: `attributes.rs` と `attributes_store.rs` に `tracing::` が 1 行も無かった。落ちる経路は 3 本 ——
  (a) `stored_claim_of` が `None`（R3 の地域のずれ、`payload` が読めない）
  (b) `view` は `kinds` に無い種類の主張をどのリストにも入れない
  (c) `kinds_of` の内部結合が、名前の行が無い種類を結果から消し、**その種類の主張も全部消す**。
  `core.attribute_kind` は削除を拒むので、そうなった種類は**永久に「主張だけがある見えない種類」**になる。
  (c) は API 経由では到達しにくいが、`core.attribute_kind` へ直に INSERT する経路は
  `tools/check-immutable.sh` の中に既に実在する。**「到達しにくい」と「起きたら気付ける」は別。**
- kind: technical
- 処置: fixed 8.1 —— `claims_of` が読んだ数と組めた数を突き合わせ、差があれば `tracing::error!(kind = "attributes_claims_dropped", read, dropped)` を出す（値は載せない。製造準備 A-2）。 `kinds_of` は `LEFT JOIN` にして、名前の無い種類を `NAMELESS_KIND` の印で出す（捨てるより入れる）。 `read_kind_without_a_name_is_not_dropped` が固定する。

## R13. `seed.sh` も `smoke.sh` も「入れた件数」しか数えず、「読み出せた件数」を数えない

- 出所: `silent-failure-hunter` HIGH-4
- 成果物: `tools/seed.sh` / `tools/smoke.sh`
- 根拠: `seed.sh` は取り込みの受理数を厳密に検算しているが、その直後に印字するのは種類の数だけ。
  **R3 / R12 のどれかで 21 件が全部読み出しから消えても「21 件入れた」と印字して exit 0 する。**
  そのあと確認バッチの手順書を持った人間が、空のカードが並ぶ画面を見ることになる。
  `smoke.sh` の側も補えていない —— 主張 3 件・訂正なし・精度 `month` だけしか流しておらず、
  `view()` の手順 1・2・4（消去・取り消し・予定）を通る読み出しは縦串で 1 度も実行されない。
- kind: technical
- 処置: fixed 8.8 —— `seed.sh` の末尾に読み直しの検算を足した（入れた 21 件と読めた件数が一致すること、 住所の `superseded` 1・`upcoming` 1・精度 `unknown` 1、副業の `current.value` が `null` であること）。 `smoke.sh` には種類の口の段（合言葉 401 → 足す → 名前を変える → 読み直し）を足した。

## R14. 原文の ` ` が `payload` 経由で 500 になり、まとめ送り全体を止める

- 出所: `silent-failure-hunter` HIGH-5
- 成果物: `crates/server/src/attributes.rs`（`parse_claim`）
- 根拠: `IngestRequest::validate` が見るのは**原文のバイト列**（`raw.contains('\0')`）だが、
  主張の原文は JSON の**テキスト**なので、エスケープされた NUL は原文に 1 バイトも現れず検査を通る。
  解釈すると Rust の `String` に本物の U+0000 が入り、`payload`（`jsonb`）への INSERT が
  SQLSTATE 22P05 で落ちて**まとめ送り全体が 500 になる**。
  実測: `SELECT '{"a":"x y"}'::jsonb` → `ERROR: unsupported Unicode escape sequence`。
  画面からは `status >= 500` → 「サーバに届きませんでした」に見え、**本人は同じ入力で押し直し続ける**
  （原因は入力にあるのに、文は「届かなかった」と言っている）。
  `ingest.rs` の doc が名指しで防いでいる事故（「その 1 件が後続を永久に止める」）の、主張だけの抜け道。
- kind: technical
- 処置: fixed 8.2 —— `parse_claim` が値と補足の制御文字を `invalid_claim_value` で断る。 `ingest_rejects_control_characters` が、断ることとまとめ送りの後続が巻き添えで落ちないことの両方を見る。

## R15. 「前の書き込みが間違っていた」が、黙って「変わった」にすり替わる

- 出所: `silent-failure-hunter` MEDIUM-6
- 成果物: `web/src/MasterView.tsx`（`WriteForm`）
- 根拠: 取り消す主張は `useState(newest?.id ?? null)` の初期値だけで決まり、`null` のまま送信できた。
  主張を 1 件も持たない種類で「前の書き込みが間違っていた」を選ぶと選択肢が 0 個になり、
  `supersedes: null` の**普通の主張として受理され、フォームは黙って閉じる** ——
  本人は訂正したつもりで、記録には訂正でないものが残る。
- kind: technical
- 処置: fixed 8.5 —— 取り消す主張が決まっていないときは「積む」を `disabled` にし、理由を出す。 `取り消す主張が選べないときは積ませない` が、押しても送られないことまで見る。

## R16. `claim_from_payload` が `parse_claim` より緩く、欠けた欄を「本人が書いた値」に化けさせる

- 出所: `silent-failure-hunter` MEDIUM-8
- 成果物: `crates/server/src/attributes_store.rs`
- 根拠: 書き込み側は `value` を**必須の欄**として扱う（`null` が「なし」なので欠落と区別する）のに、
  読み出し側は `.and_then(|v| v.as_str())` で読むので、**欄が無くても数値でも `None`（＝「なし」）**になる。
  画面はそれを「なし（その属性が終わった）」と出す —— **本人が一度も書いていない主張を本人の主張として出す**（落とすより悪い）。
  `supersedes` も同じで、壊れた UUID は黙って `None` になり、**訂正が普通の追記に落ちて古い値がいまの値に戻る**。
  `precision` だけは厳しく、3 つの欄で扱いが割れていた。
- kind: technical
- 処置: fixed 8.1 —— `claim_from_payload` を `parse_claim` と同じ厳しさにした （`value` は欄が必須、`supersedes` / `note` は型が違えば読まない）。読めない行は R12 のログに載る。

## R17. `supersedes_is_valid` のコメントと実装が食い違っている

- 出所: `silent-failure-hunter` MEDIUM-9
- 成果物: `crates/server/src/attributes_store.rs`
- 根拠: doc は「**削除の印や消去は見ない**」と書いていたが、実装は `payload->'kind'` の一致を見るので、
  **消去された主張（`payload = '{}'`）は必ず `false`** になる（= `invalid_supersedes`）。
  コメントどおりなのは削除の印の側だけ。結果の害は小さいが、**次に読む人がこのコメントを根拠に判断する**。
  ST22 / ST23（消す操作）が入ったときに効いてくる。
- kind: technical
- 処置: fixed 8.1 —— コメントを実装に合わせ、どちらの向きかを結合テストで固定した

  （`ingest_supersedes_rejects_an_erased_claim`。削除の印は指せる／消去は指せない、を分けて見る）。**

## R18. 受理された直後の読み直しの失敗が、「積めなかった」と見分けられない

- 出所: `silent-failure-hunter` MEDIUM-10
- 成果物: `web/src/MasterView.tsx`
- 根拠: 主張はサーバに確かに入っているのに、直後の `GET /attributes` が落ちると画面全体が
  「個人属性を読み出せませんでした」になる。フォームは既に閉じ、入力は捨てられていて、
  **本人から見て「積めたのか」はどこにも書いていない**。分からないまま打ち直すと、
  乱数も識別子も別なので **2 件目が入る**（畳まれない。深掘り C2）。
- kind: technical
- 処置: fixed 8.5 —— 受理の事実を読み直しより先に画面の上へ立て、読み直しの結果で上書きしない。 `積めた後に読み直しが落ちても、「積めた」ことが画面に残る` が固定する。

## R19. 断られた本文が空のとき、「届かなかった」に化ける

- 出所: `silent-failure-hunter` LOW-11(a)
- 成果物: `web/src/attributes.ts`（`readIngestResponse`）
- 根拠: 配列でない本文と**空の配列**をまとめて `unreachable` にしていた。`/ingest` は本文の形が悪いと
  `(400, [])` を返すので、「サーバが断った」が「サーバに届きませんでした」になる。
  向きは安全側だが、**再送は毎回同じ結果になるので出口が無い**。
- kind: technical
- 処置: fixed 8.5 —— 配列でなければ「届かなかった」、空の配列は `rejected("unknown")`（＝断られた）に分けた。

## R20. `core.event` への直の INSERT には主張の形の検査が無い

- 出所: `silent-failure-hunter` LOW-11(c)
- 成果物: `migrations/202609160220_personal_attributes.sql`
- 根拠: 錠は UPDATE と DELETE だけを見る。`logical_source='s01-attribute'` の行を直に INSERT すれば、
  由来が `collected`・端末識別子つき・`payload` に `kind` 無し、のどれでも入る。入った後は消せず、
  読み出しからは落ちる。BEFORE INSERT のトリガで最低限を見れば、DB の側の主張が
  「取り込み口を通ったものだけ」になる。
- kind: technical
- 処置: rejected: 入れない。理由は 2 つ。(1) ST03 の錠も同じ性質で、`core.event` への直の INSERT は どのソースでも検査されない（`origin='collected'` の行を直に入れることもできる）。ここだけ INSERT を 締めると、主張のソースだけ別の規律になり、`record-envelope` の一般の振る舞いとずれる（design D10「変えないもの」）。 (2) 検査そのものが `tools/check-immutable.sh` の経路を塞ぐ —— 錠を psql から直に殴って確かめる 台本が、主張の行を置けなくなる。錠の外の行が読み出しから落ちることは R12 のログで観測できるようにした。 DB の側で INSERT まで締めるかは ST22 / ST23（消す操作）が錠を触るときに、`record-envelope` 全体の 話として決めるのが筋なので、`docs/handoff/ST22.md` に申し送りとして書いた。

## R21. 種類の 2 つの書き込み口に、ハンドラ層のテストが 1 本も無い（**合言葉を外しても全部緑**）

- 出所: `pr-test-analyzer` F1（重大度 9）
- 成果物: `crates/server/src/attributes_tests.rs` / `crates/server/src/lib.rs`
- 根拠: `kinds_*` の 8 本はすべて `attributes_store::add_kind` / `rename_kind` の**関数直呼び**で、
  ハンドラを通っていなかった。`pr-test-analyzer` の実測: `attributes_kind_post` から
  `authorize(&app, &headers)?;` を外しても **280 件全部緑**（＝合言葉なしで誰でも種類を足せる状態が緑）。
  同時に未検証だったもの: 経路のパス記法（axum 0.8 の `{id}`）/ 400 の本文の形 / `user_id` の既定。
  `smoke.sh` も `/attributes` しか叩いていないので縦串でも拾えない。
- kind: technical
- 処置: fixed 8.4 —— ハンドラを通すテストを 3 本足した（`kinds_post_requires_the_token` / `kinds_post_round_trip_through_the_handlers` / `kinds_post_rejection_body_shape`）。 `authorize` を外すと `kinds_post_requires_the_token` が落ちることを実測で確かめた。 `smoke.sh` にも種類の口の段（401 → 足す → 名前を変える → 読み直し）を足した。

## R22. 「主張の値を書き換える文は拒まれる」を `raw` に対して一度も試していない／門で見る 3 列に裏が無い

- 出所: `pr-test-analyzer` F2（重大度 8）
- 成果物: `tools/check-immutable.sh`
- 根拠: ST19 の段は値の書き換えを `payload = jsonb_set(...)` でしか試していなかった。
  **`raw` は値の正典で、消去の唯一の復元元**であり、Scenario の WHEN の最も直接的な読みはそちら。
  さらに構造的な穴として、列の点呼は `frozen_cols` を 1 列ずつ回して確かめているのに、
  `gated_cols`（`raw` / `payload` / `content_hash`）にはその確認が無く、
  **門が将来 `content_hash` を見なくなっても点呼は「知っている列」として通り続ける**。
- kind: technical
- 処置: fixed 8.7 —— `raw` と `content_hash` の素の書き換えを 1 つずつ試す段を足し、 `gated_cols` の 3 列も `frozen_cols` と同じ形のループで「台帳なしの素の書き換えは拒まれる」を確かめる。

## R23. 「送っている間は『積む』を押せなくする」に assertion が無い

- 出所: `pr-test-analyzer` F3（重大度 7）
- 成果物: `web/src/__tests__/master-form.test.tsx`
- 根拠: spec の SHALL だが、`disabled` を見るテストが無かった。実測: `disabled={sending}` を削っても
  web 114 件が全部緑。`built.current` のキャッシュがあるので実害は「同じ原文が 2 回飛ぶ」に留まるが、
  SHALL が固定されていない。
- kind: technical
- 処置: fixed 8.7 —— 応答を保留にした `fetch` で `送っている間は「積む」が押せない` を足した。

## R24. 3 段目の tie-break（D-01 に入った時刻）に到達できるテストが無い

- 出所: `pr-test-analyzer` F4（重大度 6）
- 成果物: `crates/server/src/attributes.rs`（`mod tests` の `stored()`）
- 根拠: `stored()` が `ingested_at` を `asserted_at` から導いているので、**2 つが同じで
  `ingested_at` だけ違う入力が作れない**。実測: `sort_key` の第 3 要素を固定値にしても 280 件緑。
- kind: technical
- 処置: fixed 8.7 —— `stored_at()` を足して `ingested_at` を別に渡せるようにし、 `view_third_tie_break_is_ingested_at` が「『いつから』も主張した日時も同じ 2 件」で D-01 に入った時刻が後の主張が勝つことを固定する。

## R25. `upcoming` の「古い順」が固定されていない

- 出所: `pr-test-analyzer` F5（重大度 6）
- 成果物: `crates/server/src/attributes.rs`（`mod tests`）
- 根拠: 予定が 2 件以上ある入力がどのテストにも無かった。実測: `upcoming.reverse()` を足しても 280 件緑。
- kind: technical
- 処置: fixed 8.7 —— `view_upcoming_is_oldest_first` を足した（予定 3 件を入れ替えて渡し、並びを見る）。

## R26. 「変わったを選んで主張を積める」の THEN が確かめられていない

- 出所: `pr-test-analyzer` F7（重大度 5）
- 成果物: `web/src/__tests__/master-form.test.tsx`
- 根拠: spec の THEN は「**画面を読み直した後、カードの主張が 1 件増え、その値と「いつから」が出ている**」だが、
  印の付いたテストは送った POST の本文しか見ていない。読み直しを見る別のテストは `serve(empty(), ...)` なので
  **読み直した後も主張は 0 件**。つまり「主張が画面に増えて見える」を通しで見たテストが 1 本も無かった。
- kind: technical
- 処置: fixed 8.7 —— 応答を 2 状態にして、`積んだ後、読み直した画面に主張が 1 件増えて見える` を足した。

## R27. 主張の原文の形が 4 か所に手写しされている

- 出所: `pr-test-analyzer` F9（重大度 5）
- 成果物: `web/src/attributes.ts` / `crates/server/src/attributes_tests.rs` /
  `tools/check-immutable.sh` / `tools/smoke.sh`
- 根拠: 各側は自分の写しに対して緑になるので、片側を「実装とテストを揃えて」直すと相手側が気付けない。
  `buildClaim` の出力を固定した JSON として持ち、Rust の `parse_claim` に食わせる黄金値テストがあれば 1 点に締まる。
- kind: technical
- 処置: fixed 8.7 —— `web` 側に「画面が組む原文の形」を固定した黄金値テストを置き （`buildClaim` の出力の欄と並びを固定）、Rust 側の `parse_claim_accepts_the_shape_the_web_builds` が 同じ文字列を読んで通ることを見る。片側だけ形を変えると、もう片側が落ちる。

## R28. 応答の形が違うときの画面（`unexpected_shape`）が通しで見られていない

- 出所: `pr-test-analyzer` F10（重大度 5）
- 成果物: `web/src/__tests__/master-view.test.tsx`
- 根拠: `isAttributesView` は単体で厚くテストされているが、`MasterView` の `load()` が
  **200 かつ形違い**のときに失敗を出すことは、どのテストも通していなかった（`open(500)` だけ）。
- kind: technical
- 処置: fixed 8.7 —— `200 で形の違う応答も、失敗として出す` を足した。

## R29. 「なしを選んで積める」が、本物のサーバなら断られる原文を送っている

- 出所: `pr-test-analyzer` F12（重大度 3）
- 成果物: `web/src/__tests__/master-form.test.tsx`
- 根拠: 精度「年月」のまま月を入れずに「積む」を押しており、`validFromOf` は `date: null` を返す。
  **本物のサーバなら `invalid_valid_from` で断られる**原文で、スタブが受理を返すから通っていただけ。
  Scenario の「積める」は成立していなかった。
- kind: technical
- 処置: fixed 8.7 —— 「いつから」を埋めてから押す形に直し、送った原文の `valid_from` も固定した。

## R30. 鍵の作り直しのテストが、乱数の導き方までは守っていない

- 出所: `pr-test-analyzer` F13（重大度 4）
- 成果物: `crates/server/src/attributes_tests.rs` / `web/src/__tests__/attributes.test.ts`
- 根拠: Rust 側の `guess` は本物の原文と `nonce` の有無だけが違うので、
  **乱数が `id` から決定的に導かれていても（D4 が防ぎたい当の攻撃）このテストは通る**。
  Rust 側は `NONCE` が全テスト共通の固定値なので気付けない。「乱数が `id` から導かれない」は
  web の 1 本だけが持っており、その 1 本が Rust 側の前提を支えている。
- kind: technical
- 処置: fixed 8.7 —— Rust 側のテストの doc に、この性質を支えているのは web の `同じ内容の 2 つの主張は別々の乱数を持つ` であることを明記して結んだ（実装は既に正しく、 web 側の deliberate break で落ちることも確かめてある）。

## R31. 「確定した色だけを使う」の網が粗い

- 出所: `pr-test-analyzer` F14（重大度 3）
- 成果物: `web/src/__tests__/master-view-limits.test.tsx`
- 根拠: 正規表現は `#hex` / `rgb(` / `hsl(` だけで、名前付き色（`white`）・`oklch(` / `color(` / `lab(` は素通る。
  `MasterView.tsx` は `background: "transparent"` を使っているので名前付き色を一律禁止にはできない。
- kind: technical
- 処置: fixed 8.7 —— 新しい色関数（`oklch(` / `lab(` / `lch(` / `color(` / `hwb(`）を網に足し、 併せて `transparent` 以外の名前付き色を禁じる形にした。

## 報告の閾値未満として受け取り、直さなかったもの

- **`add_kind` が失敗すると住所と職業まで巻き戻る**（`code-reviewer` 参考欄）——
  **処置: rejected（意図どおり）。** 1 つのまとまりで「初期化 → 重なりの確認」をやるのは、
  初期化と「住所」を足す要求が重なっても**いまの名前が「住所」の種類が 1 つ**であることを
  錠で保証するため（design D7）。巻き戻っても次の読み出しが置き直すので失われるものは無い。
  `tools/seed.sh` が回避策（先に読み出す）を書いているのは、この性質を前提にしている。
- **`GET /attributes` が毎回 `pg_advisory_xact_lock` を取る**（`code-reviewer` 参考欄）——
  **処置: deferred ST29。** 同じ利用者の読み出しが直列化する。単一利用者の想定では実害が出ず、
  読み出しに書き込みがあること自体の反転条件は design D7 が既に持っている
  （呼び出し元が限られる ST29 まで受け入れる、と Risks に書いてある）。
- **`GET /attributes?user_id=` を呼び出し元が名乗れる**（`code-verify` が指摘に立てなかったもの）——
  **処置: deferred ST29。** `deep.md`「確かめたが問わなかったこと」の担当どおり。
