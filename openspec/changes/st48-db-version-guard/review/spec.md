# ST48 spec-review（`spec --draft`。第 1 回の先読み）

検査対象: `openspec/changes/st48-db-version-guard/`（proposal / specs/distribution / design D1〜D17 / tasks）、`docs/stories/ST48.md`、`docs/requirements.md`（FR-117・FR-118・FR-129）、`docs/stories/INDEX.md`、`openspec/specs/`。
deep.md の Q1〜Q5 は `（答え待ち・推奨: …）` のままで、推奨を本人の答えとして読んだ。

## 機械の検査（先に写す）

| コマンド | rc | 結果 |
|---|---|---|
| `openspec validate st48-db-version-guard --strict` | 0 | valid |
| `python3 scripts/check_chain.py .` | 0 | chain: OK（要件 171 / Story 58。観点 8 の再生成との一致も通る） |
| `python3 scripts/check_scenarios.py . st48-db-version-guard` | 1 | この change の Scenario 34 本が「担保の無い Scenario」。実装の前なので予定どおりの状態（tasks.md 9.1 で rc=0 にする）。spec の 34 本の名前を tasks.md に逐語で grep し、**34 本とも tasks.md に名指しがある**ことを確かめた（欠け 0）。そのほかの warn は他の Story の印で、この change に関係しない |
| `python3 scripts/review_triage.py . st48-db-version-guard` | 1 | `deep.md R1〜R7: escalated なのに deep.md に R<n> が無い` の FAIL が 7 件（下の R2） |

---

## R1. 「まだ当たっていない版だけを当てる」（Q1 の推奨の中心）を偽にできる Scenario が無い。全部を毎回当て直して記録だけ `ON CONFLICT DO NOTHING` で足す実装でも、34 本が全部通る
- 成果物: openspec/changes/st48-db-version-guard/specs/distribution/spec.md
- 根拠: spec.md:63-67（「同じ版で当て直しても」は行の数・名前・`applied_at`・門の関数の定義を見る。同じ中身を当て直しても門の定義は同じなので変わらない）/ spec.md:69-72 と tasks.md:58（「末尾の版は当て直されない」を `applied_at` が変わらないことで見る）/ spec.md:74-77（「当て直さず」を `applied_at` で見る）。どれも版の記録の列しか見ず、版の中身が 2 回走ったかどうかは観測していない。deep.md:8 の推奨・proposal.md:17（「まだの分だけ当てる」にすると配った移行の中身を凍結できる）
- kind: technical
- 提案: 当て直すと跡の残る試験用の版で、当て直しが起きないことを観測する Scenario を足す。例: 試験用の並びの 1 つ目が試験用の表に 1 行入れる版で、2 回当てても行は 1 行のまま。または「指紋の違う版」の Scenario で、記録の後に中身を変えた版の新しい中身の効果（作る表など）が DB に無いことを見る
- 処置: fixed specs/distribution/spec.md —— Scenario「記録にある版は、中身をもう一度走らせない」を足し（試験用の表に 1 行足す版を 2 回当てて 1 行のまま）、tasks 2.2 に `st48_apply_does_not_rerun_recorded` を足した

## R2. deep.md に、escalated になった review/deep.md の R1〜R7 が 1 つも書かれていない（review_triage が 7 件 FAIL）
- 成果物: openspec/changes/st48-db-version-guard/deep.md
- 根拠: `python3 scripts/review_triage.py . st48-db-version-guard` rc=1（`[FAIL] deep.md R1: escalated なのに deep.md に R1 が無い` 〜 R7）/ `grep -n -E "R[0-9]+" deep.md` は 0 行 / review/deep.md:30・38・46・53・61・69・77 の処置はどれも `escalated`
- kind: technical
- 提案: 各 Q に、その問いを生んだ R の id を書く（review/deep.md の処置から: R1・R4 → Q1、R2 → Q2、R3・R6・R7 → Q4、R5 → Q5）。triage が通ることを確かめる
- 処置: fixed deep.md —— Q1（R1・R4）・Q2（R2）・Q4（R3・R6・R7）・Q5（R5）に review/deep.md の出所を書いた

## R3. 版の記録の置き場と形の凍結（C1。「後の版は置き場を消さない・移さない、形を変えるときは形の版を上げる」）が design D1 と試験 1 本にしか無く、正典に残らない
- 成果物: openspec/changes/st48-db-version-guard/design.md
- 根拠: design.md:51-52（凍結の試験 `st48_ledger_shape_is_frozen` と「列を足すだけでも `form` を上げる」）/ design.md:193（Risks: 後の版が記録を消す・移すと、古い版はその DB を記録の無い DB と読んで全部を当てる。緩和は D1 の凍結）/ tasks.md:39（凍結の試験は Scenario の印を持たない）/ spec.md:8-52 に、記録の置き場・形を後の版が変えないこと、形の版を持つことを言う文も Scenario も無い / premises.md:21（C1）・review/deep.md R2・R3（配った版に焼き付く）
- kind: technical
- 提案: `openspec archive` は design を正典に写さないので、後の Story（ST52 以後）が記録の形を動かしても spec は落ちない。Requirement「DB に当てた移行の版を、DB の中に記録する」に、記録の置き場と列の形は版をまたいで変えない・変えるときは形の版を上げる、を足し、凍結の試験をその Scenario の担保にする
- 処置: fixed specs/distribution/spec.md —— Requirement「DB に当てた移行の版を…」に置き場と形を変えない・変えるなら形の版を上げる、を足し、Scenario「版の記録の置き場と列の形は、決めた形のまま変わらない」を `st48_ledger_shape_is_frozen` の担保にした（tasks 1.2・design D1）

## R4. 止まったときの標準エラーの `db:` / `self:` の行の形と 20 件の上限は、ST58 が読み配った版に焼き付く出力なのに、design D6 にしか無い
- 成果物: openspec/changes/st48-db-version-guard/design.md
- 根拠: design.md:101-113（「ST58 が読む。配った版に残るので、行の頭の形を変えない」・`  db: <名前> (<版番号> <commit>)`・`  self:`・20 件まで）/ spec.md:92・101・126・135 は `error: kind=…` の 1 行目と、名前・版番号・commit が「ある」ことしか言わない
- kind: technical
- 提案: 観測できる出力の契約なので spec に置く（design に置くと archive で正典から落ちる）。Requirement「知らない版が当たった DB には…」に `db:` / `self:` の行の形と上限を書き、Scenario の THEN をその形で見る
- 処置: fixed specs/distribution/spec.md —— `error: kind=`・`  db: <名前> (<版番号> <commit>)`・`  self:` の形と 20 件の上限を Requirement に書き、db_ahead の Scenario の THEN をその形で見る（migrate・起動とも。tasks 4.2）

## R5. サーバの起動で指紋の違う版があるときに「起動して警告だけ」（D7・D17）が spec に無い
- 成果物: openspec/changes/st48-db-version-guard/specs/distribution/spec.md
- 根拠: design.md:121（「起動も同じく警告だけ」）・design.md:187（D17「指紋の違いは警告だけ」）/ spec.md:58 は migrate の指紋の違いだけを言う / spec.md:124-155（起動の Requirement）は指紋に触れない
- kind: technical
- 提案: 起動するか止まるかは観測できる振る舞いなので、起動の Requirement に 1 文と Scenario（指紋を書き換えた DB でサーバが `/healthz` に答え、その版の名前を含む警告が出る）を足す
- 処置: fixed specs/distribution/spec.md —— 起動の Requirement に指紋の違いは警告して起動する、を足し、Scenario「指紋の違う版があっても、サーバは警告を出して起動する」と tasks 5.2 `st48_server_starts_with_fingerprint_warning` を足した

## R6. 「commit が空でない」は、組み立ての commit が取れずに `unknown` を書いても通る。R7（組み立てを区別する）の目的を担保しない
- 成果物: openspec/changes/st48-db-version-guard/specs/distribution/spec.md
- 根拠: spec.md:27（「commit が空でない」）/ design.md:127（D8: commit が取れなければ `unknown`）/ review/deep.md R7（0.1.0 だけでは組み立てを区別できない）/ tasks.md:55（「commit は空でない」）
- kind: technical
- 提案: THEN を「commit は組み立てに渡した `ASHIATO_BUILD_COMMIT`（無ければその木の `git rev-parse HEAD`）と一致する」にする。`unknown` に落ちる場合は別の Scenario で扱う
- 処置: fixed specs/distribution/spec.md —— THEN を「commit が組み立てに渡した `ASHIATO_BUILD_COMMIT` の値と一致する」にし、tasks 2.2 で `env!("ASHIATO_BUILD_COMMIT")` と（渡さない場合は）`git rev-parse HEAD` との一致を見る

## R7. Requirement の本文だけが言い、Scenario が言っていないことが 5 つある
- 成果物: openspec/changes/st48-db-version-guard/specs/distribution/spec.md
- 根拠:
  - spec.md:126（起動の `db_ahead` は DB の行の組み立ての版番号と commit も示す）↔ spec.md:135 の THEN は名前と自分の版番号と commit だけ（migrate の側の spec.md:101 は DB の行の版番号と commit を見ている）
  - spec.md:127（`db_behind` は「まだ当たっていない版の数」を示す）↔ spec.md:140・145 は数を見ない
  - spec.md:59（当てる版が無い回にも、アプリの役割への付与を当てる）↔ spec.md:79-82 は退避しか見ない
  - spec.md:94（版の中身が当たらないときも終了コード 1）↔ spec.md:114-117 は退避の失敗だけ。中身の失敗は spec.md:35-38 のライブラリの段で「失敗を返す」までで、実行ファイルの終了コードを見ない（ST58 が戻しを走らせる主な場合はこちら。FR-119）
  - spec.md:95（どの環境変数でも外せない）↔ spec.md:119-122 は `MIGRATE_BACKUP=off` の 1 つだけ、起動の側は無い。tasks.md:77 は思いついた名前の否定の grep だけ
  - spec.md:215（`tools/dev.sh` も止まって手順を示す）↔ spec.md:230-233 は `tools/stack.sh` だけ。tasks.md:131 は `dev.sh` に `COMPOSE_PROJECT_NAME` の字があるかの grep だけ
- kind: technical
- 提案: それぞれ Scenario の THEN に足すか、Scenario を 1 本足す（版の中身が落ちる試験用の並びで実行ファイルが 1 で終わり `kind=` が出ない、など）。測れない言い方（「どの環境変数でも」）は、外せないことを見る対象を列挙する形に直す
- 処置: fixed specs/distribution/spec.md —— 起動の db_ahead の THEN に DB の行の版番号と commit、db_behind に数（2 行消して 2）、Scenario「当てる版が無い回にも、アプリの役割への付与を当てる」「版の中身が当たらないと、終了コード 1 で終わる」を足し、環境変数は「`MIGRATE_BACKUP`・`MIGRATE_BACKUP_DIR`・`PG_DUMP` のどの値でも」と列挙して Scenario で 3 つとも見る。`dev.sh` は spec の本文から外し design D14 にだけ残した（同じ手順を出すだけで、確認バッチと e2e の経路は stack.sh）。tasks 4.2・5.2

## R8. 1 つの Scenario に複数の主張を AND で束ねたものがある
- 成果物: openspec/changes/st48-db-version-guard/specs/distribution/spec.md
- 根拠: spec.md:22-23（名前の集合 + 印が無い）/ spec.md:27（指紋・時刻・版番号・commit の 4 つ）/ spec.md:32-33（終了コード・記録の 1 件・名前の集合・全部に印）/ spec.md:52（断られない + 書き出しに入らない）/ spec.md:66-67（記録 + 門の定義）/ spec.md:101-102 / spec.md:170-173（消す・名前を変えるの 2 つの木を 1 本で）/ spec.md:180-183（印の後の版 + `.down.sql` を 1 本で）/ spec.md:227-228 / spec.md:233
- kind: technical
- 提案: とくに spec.md:170-173 と 180-183 は 2 つの場合を 1 つの木で見る形にもなりうる（片方の場合だけで通る）。場合ごとに分けるか、tasks.md 6.1 の self-test が場合ごとに別の木で見ることを THEN に書く。ほかは呼び出し元の判断に任せる
- 処置: fixed specs/distribution/spec.md —— 2 つの場合を束ねた 170-173 と 180-183 を場合ごとの 4 本に分け、tasks 6.1 の self-test は場合ごとに別の木で見る。ほかの AND は rejected: 同じ 1 回の操作の結果の別の面で、片方だけ満たす実装を許さないために束ねている

## R9. 「並びの前にある未適用の版も当たる」の GIVEN が自分と食い違い、tasks の試験の作り方とも合わない
- 成果物: openspec/changes/st48-db-version-guard/specs/distribution/spec.md
- 根拠: spec.md:70（「`migrations/ORDER` の末尾の版だけが当たり、その前の版のうち 1 つが記録に無い」—— 末尾だけが当たっているなら、前の版は全部記録に無い）/ tasks.md:58（試験用の 3 版の並びで 1 つ目と 3 つ目を先に当て、2 つ目だけが当たる）
- kind: technical
- 提案: GIVEN を tasks に合わせる（「試験用の 3 つの版の並びのうち、1 つ目と 3 つ目だけが当たって記録された DB」）。あわせて spec.md:30・32 の「記録を 1 件入れた」「記録の行は 1 件のまま」は、版の記録と本人の記録のどちらかが字面で分からないので、本人の記録の側を別の言葉にする
- 処置: fixed specs/distribution/spec.md —— GIVEN を「試験用の 3 つの版の並びのうち、1 つ目と 3 つ目だけが当たって記録された DB」にし、本人の記録は「本人の記録（`core.event` の行）」と書き分けた

## R10. 試験の足場は、外側の取引の中で `pg_advisory_xact_lock(4820251)` を持って当てている。Task 3.1 はそれを外すことを書いておらず、`apply_migrations`（同じ鍵のセッションの錠・1 版 1 取引）と組むと、待ち合いか入れ子の取引になる
- 成果物: openspec/changes/st48-db-version-guard/tasks.md
- 根拠: crates/server/src/testdb.rs:88-100（`pool.begin()` → `pg_advisory_xact_lock(4820251)` → `MIGRATIONS` と `GRANTS` → `commit`）/ design.md:149-150（D11: 始めに `pg_advisory_lock(4820251)`、1 版ごとに取引）/ tasks.md:65（3.1 は「直に流す形から `apply_migrations` → `GRANTS` に替える」とだけ書く）
- kind: technical
- 提案: 3.1 に「外側の取引と `pg_advisory_xact_lock` を外し、取引の外の 1 本の接続で `apply_migrations` を呼ぶ（錠は `apply_migrations` が取る）」を書く。別の接続から呼ぶと、同じ鍵の xact の錠を持つ自分を待ち続ける
- 処置: fixed 3.1 —— 外側の取引と `pg_advisory_xact_lock(4820251)` を外し、取引の外の 1 本の接続で `apply_migrations` を呼ぶ、と書いた（design D11 にも）

## R11. Task 1.1 で `grants.sql` に `migration_ledger` の付与を足すと、記録の schema の無い DB に `grants.sql` を当てる経路が落ちる。`tools/check-immutable.sh` は psql で ON_ERROR_STOP のまま当てていて、D12 はそこを変えない
- 成果物: openspec/changes/st48-db-version-guard/tasks.md
- 根拠: tasks.md:35（1.1 で `grants.sql` に `USAGE` と `SELECT` を足す。検証は `cargo build` だけ）/ design.md:49（D1 の付与）・design.md:158（D12: check-immutable は psql で直に当てるまま、版の記録は持たない）/ tools/check-immutable.sh:1178・1341（`psql -v ON_ERROR_STOP=1 … < crates/server/src/grants.sql`）/ crates/server/src/testdb.rs:93-96（3.1 の前は bootstrap を流さずに `GRANTS` を当てる）/ schema は 2.1 の `apply_migrations` の bootstrap で初めて作られる
- kind: technical
- 提案: `grants.sql` の付与を schema が無くても通る形にする（`DO` で有無を見る）か、check-immutable.sh で `grants.sql` の前に `migration_ledger.sql` を流すことを D12 と 8.1 に書く。付与の追加は bootstrap を流す経路ができる 2.1・3.1 と同じタスクに置き、1.1 の検証に `CT` を足す
- 処置: fixed D1 —— アプリの役割への `migration_ledger` の付与は `grants.sql` でなく bootstrap（`migration_ledger.sql`）の末尾に置く。tasks 1.1 も同じに直した

## R12. 「退避に失敗したら移行を当てない」（壊してはいけないもの 2 つ目）は、いまの正典に Scenario が無い。D4 で退避の位置を動かすのに、spec は退避の失敗のとき何も当てないことを見ない
- 成果物: openspec/changes/st48-db-version-guard/specs/distribution/spec.md
- 根拠: docs/stories/ST48.md:40（移行の前の自動の退避）/ `check_scenarios.py` の warn「spec に無い Scenario を指す印: 退避に失敗したら移行を当てない (crates/server/src/migrate_backup.rs)」と「移行を当てる前に退避が置かれ、合言葉は引数でなく環境変数で渡る」（どちらも `openspec/specs/` に無い。`grep -rn -E "migrate|pre-migrate" openspec/specs` は 0 件）/ design.md:74-84（D4: 退避を接続・錠・読む、の後へ動かす）/ spec.md:114-117（退避の失敗は rc=1 と `kind=` が無いことしか見ない）
- kind: technical
- 提案: spec.md:114-117 の THEN に「版の記録と門の関数の定義は走らせる前と同じ（1 つも当たっていない）」を足す。宙に浮いている migrate_backup.rs の 2 本の Scenario を、この capability の Requirement の下に受けるかを決める
- 処置: fixed specs/distribution/spec.md —— Scenario「退避に失敗すると、終了コード 1 で何も当てない」（記録から消した 1 行が戻っていない）にし、Requirement に「退避に失敗したときは版を 1 つも当てない」を足した。migrate_backup.rs の宙に浮いた 2 本の印は ST48 の対象外（いまの正典に無いのは ST48 より前からで、この change では受けない。印の名前を変えずに残る）

## R13. Story の「壊してはいけないもの」3 つ目（確認バッチの `MIGRATE_BACKUP=off` の使い捨ての DB）は、review/deep.md R5 で誤りと分かった前提のまま残り、Q5 の推奨（確認バッチの DB は残す・回ごとに別の project）と食い違う
- 成果物: docs/stories/ST48.md
- 根拠: docs/stories/ST48.md:41 / review/deep.md:55-61（R5。`run.sh` は `STACK_RESET` を付けず、DB は回をまたいで残り、電話の記録が入る）/ tools/verify-prep.sh:80（`exec ./tools/stack.sh up`。`STACK_RESET` も `MIGRATE_BACKUP=off` も無い）/ tools/stack.sh:24・75・83（`MIGRATE_BACKUP=off` は `STACK_RESET=1` か `HARNESS_VERIFY=1` のときだけ）/ deep.md:28（Q5 の推奨）/ spec.md:211-237（確認バッチの DB を残す側で書かれている）/ `check_chain.py` は rc=0（この節は再生成との一致の外）
- kind: premise
- 提案: Story の 3 つ目を実物に合わせる（縦串と `STACK_RESET=1` の DB は使い捨てで退避しない・確認バッチの `run.sh` の DB は残して退避する）。spec.md:217・235-237 は前者を守っているので、Story の側だけを直せば揃う
- 処置: escalated —— 前提の誤り（確認バッチの DB は使い捨て）は、本人に渡す Q5 の context が正しい前提（回をまたいで残り、電話の記録が入る）で問うている（deep.md の Q5 に R13 を書いた）。Story の文は `docs/stories/stories.json` を直して `make_story.py` で再生成した（`check_chain.py` rc=0）

## R14. spec に開発の道具の名前（台本・環境変数・compose の project）と DB の関数名が入っている。正典に前例はほぼ無い
- 成果物: openspec/changes/st48-db-version-guard/specs/distribution/spec.md
- 根拠: spec.md:67・102（`core.reject_collected_rewrite`）/ spec.md:121（`MIGRATE_BACKUP=off`）/ spec.md:213-237（`dist/verify-<tag>/run.sh`・`tools/stack.sh`・`tools/dev.sh`・`STACK_RESET=1`・`COMPOSE_PROJECT_NAME`・`--check-only`・docker compose の project）/ `grep -rn -E "tools/[a-z-]+\.sh" openspec/specs` は 1 件だけ（external-ingestion/spec.md:304）、`grep -rln -E "STACK_RESET|COMPOSE_PROJECT_NAME|dist/verify" openspec/specs` は 0 件
- kind: technical
- 提案: `core.reject_collected_rewrite` は FR-129 の本文にもあり、門の定義を見る手段として残してよい。Requirement「開発と確認の DB が…」は、台本・環境変数の名前を Requirement の本文から外して「確認バッチの起動」「使い捨ての DB を作り直す指定」などの言い方にし、名前は Scenario の WHEN / THEN に寄せる（台本の名前を変えたときに直す場所を 1 つにする）。この Requirement を製品の capability `distribution` に置くこと自体も、呼び出し元の判断に任せる
- 処置: rejected: この Requirement の受け手は確認バッチの起動と開発の台本そのもので、Scenario を実物の名前で検証する。本文から名前を外すと、Scenario の名前とどれが対応するかが読めなくなる。`core.reject_collected_rewrite` は FR-129 の本文と同じ言い方

---

## 観点ごとの確かめ

- **観点 1（deep の決定が正典に写っているか）**: 推奨の 5 問は写っている。Q1 → spec.md:54-87・157-188 / Q2 → spec.md:15-16・29-33 / Q3 → spec.md:127・137-145 / Q4 → spec.md:91-94・114-117（終了コード 3 と 1）/ Q5 → spec.md:95・211-237。「当初案を覆したもの」「要件へ戻すもの」は deep.md に無い。数値は終了コード 3・境の時刻 `202610250000` が spec にある。premises.md の C1〜C7 は design に写っている（C1 → D1・D5、C2 → D2・D9、C3 → D1・D8、C4 → D1、C5 → D10・D12・D16、C6 → D15、C7 → D11）。中身の担保の弱さは R1・R3・R6
- **観点 2**: R1・R6・R7・R8・R9
- **観点 3**: R3・R4・R5・R14。proposal の Capabilities（`distribution` の新設のみ）・`specs/distribution/` のディレクトリ・INDEX.md:105 の capability 表（ST48 が先頭）は一致する。前倒しは無い。Modified の無いこと（正典に migrate の振る舞いが無い）は `grep -rn -E "migrate|pre-migrate" openspec/specs` 0 件で確かめた
- **観点 4**: R10・R11。そのほかのタスクはコマンドと終了条件（`CT` / `NL` / `IT` / `NI` の本数の下限つき）を持つ。人間の確認待ち（実機・実データ）が要るタスクは無い
- **観点 5**: `check_chain.py` rc=0（観点 8 を含む）。satisfies（FR-117・FR-118・FR-129）の外の要件を満たそうとする Requirement は無い（各 Requirement の「導出元」は 3 つのどれか）。「完了の判定」3 つは spec.md:98-102・132-135・63-67 に当たる（ただし 3 つ目の担保は R1）。「壊してはいけないもの」は R12・R13
- **観点 6**: 該当なし（`--draft`。`--answers` ではない）
