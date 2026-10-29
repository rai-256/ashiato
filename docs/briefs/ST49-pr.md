# ST49 記録ごとに、作った部品の版が残る

change: `openspec/changes/st49-record-component-version/`。満たす要件は FR-122・FR-123（扉 #32）。`requires: [ST01, ST48]`。
既存の 8 capability に ADDED（`record-envelope`・`device-collection`・`desktop-collection`・`derived-records`・`data-durability`・`record-deletion`・`subjective-log`・`personal-entities`。Scenario 66 本）。

S-01・C-01・C-02・V-01 の 4 つの部品は、組み立てるたびに印 `<版番号>+<commit 40 桁>[.dirty][.debug]` を作り直す（共通の作り方は `crates/build-stamp`。C-01 は `app/build.gradle.kts`、V-01 は `web/build-stamp.ts`）。
記録・破棄の報告・原本の欠けには、送った部品の版（`client_version`。資格情報で `collector_version` / `web_version` に振り分ける）と、受け取った S-01 の版（`server_version`）が残る。
画面の操作の台帳（削除・気分の通知・滞在の基準・個人属性の種類の名前）と滞在には S-01 の版が残る。書き出しにも同じ列が出る。

## 深掘りに積んだ未決（A）

なし（`deep.md` に `(未回答)` は 0 件）。深掘りは第 1 回 Q1〜Q4・第 2 回 Q5 とも答え済みで、要件へ戻すもの（Q1〜Q3 → FR-122 / FR-123）は record の段で `docs/requirements.md` に入れた。

## 仮決め（B）と Ruling

**仮決め**（design の（仮）と反転条件。`provisional.jsonl`）:
- **D1（仮）** 外部サービス由来の更新では、版は今の中身を作った版に動き、前の値は前の版に残る。反転条件: 最初の版で選び出したい事例が出たら、本表を据え置いて前の版から読む
- **D2（仮）** 滞在の作り直しで原文が変わったら、S-01 の版を作り直した版に替える。反転条件: 最初に書いた版で選び出したい事例
- **D3（仮）** 収集アプリの溜めは、読み戻した行の知らない欄を捨てずに送る。入れ替えは S-01 を先にする。
  final 後の `/code-review` R21 を受け、行をそのまま送る形から「型に通した 1 件に、行だけが持つ欄を足して送る」形に直した（古い行も今の既定値で埋まる）。知らない欄を残す振る舞いは同じ
- **D21（仮）** 収集した記録の版の書き換えは本表の門の対象（前の版を積むときだけ直せる）。反転条件: 前の版を残さずに直したい量の直しが出たら門の外へ
- D4 は仮ではなくなった（別の席が否 → 深掘り第 2 回 Q5 で本人が「今の中身を作った部品の版」と決めた）

**Ruling**（全 Task の ledger の `Ruling:` 行。逐語）:
- Task 2: `Ruling: ST41 の移行 202610231200_event_end_time.sql の書き換え（ビューと md5 の受け入れ）を戻さない — ORDER の全版を 2 回当てる試験が 15 本以上（stay_tests・end_time_tests・feelings_tests ほか）と check-immutable にあり、ST41 を戻すと後の版（ST49）が列を足したビューへ ST41 が列を減らす作り直しをして落ちる。前例は 202609120943 の同型の守り — 間違っていたときの費用: 当て済みの DB で sha256 の違いを知らせる warn が鳴る（当て直しはされない・データは無害）。人間は確認バッチで見て、戻すなら全版 2 回当ての試験の側を作り替える`
- Task 2: `parked — F1 ST41 の移行 202610231200_event_end_time.sql の本文の書き換え — Ruling: 人間が受け入れた（2026-10-28）。当て済みの DB の指紋違いの warn は受け入れる`

ほかの Task（1・3〜11）の ledger に `Ruling:` は無い。

**確認バッチで見てほしいこと**: ST41 の移行を書き換えたため、ST49 より前に ST41 を当てた DB（確認バッチの DB を含む）は `migrate` のたびに
`記録した指紋と中身が違う版がある（当て直さない） version="202610231200_event_end_time"` の警告を出し続ける（データは無害。消すには `STACK_RESET=1` で作り直す）。
理由と消し方は ST49 の移行の冒頭と design D19 にある（review/code.md R14・R17）。

## Task

11 Task・25 項目すべて `[x]`。残した Task は無い（`ci` / `human` の項目も無い）。

## 通した検証（`evidence.jsonl`。各項目の最後の実行）

| 項目 | コマンド | 終了コード |
|---|---|---|
| 1.1 | `cargo test -p ashiato-build-stamp st49_stamp_` | 0 |
| 1.3 | `cargo build -p ashiato-server` / `cargo test -p ashiato-server migration_ledger_` | 0 / 0 |
| 2.1 | `./tools/check-migrations.sh` | 0 |
| 2.2 | `cargo test -p ashiato-server st49_gate_` / `st49_cols_` / `dedup_tests` | 0 / 0 / 0 |
| 2.3 | `./tools/check-immutable.sh` / `./tools/check-shared-appends.sh` | 0 / 0 |
| 3.2 | `cargo test -p ashiato-server st49_ingest_` / `st49_dedup_` / `--lib st49_hash` / `--lib hash_is_pinned` / `dedup_tests` | 0（5 本とも） |
| 3.3 | `./tools/check-openapi.sh` / `grep -c client_version docs/collector-contract.md` ≥ 3 | 0 / 0 |
| 4.2 | `cargo test -p ashiato-server st49_update_` / `dedup_tests` | 0 / 0 |
| 5.2 | `cargo test -p ashiato-server st49_stay_` / `stay_tests` | 0 / 0 |
| 6.2 | `cargo test -p ashiato-server st49_ledger_` / `drops_tests` / `media_missing_tests` / `deletion_tests` / `feelings_remind` / `./tools/check-openapi.sh` | 0（6 本とも） |
| 7.2 | `cargo test -p ashiato-server st49_export_` / `export_` | 0 / 0 |
| 8.1 | `cargo test -p ashiato-collector-windows --lib st49_c02_` / `--lib` / `cargo clippy -p ashiato-collector-windows --all-targets --target x86_64-pc-windows-gnu -- -D warnings` | 0 / 0 / 0 |
| 9.1 | `./tools/check-build-stamp.sh android` | 0（1 回目は 1。直して 0） |
| 9.2 | `gradlew :app:testDebugUnitTest --tests '*St49OutboxTest*'` | 0 |
| 9.3 | `gradlew :app:testDebugUnitTest --tests '*St49DropDraftTest*'` / `:app:testDebugUnitTest` | 0 / 0 |
| 10.1 | `npx vitest run src/__tests__/buildStamp.test.ts` / `npx vitest run` / `npm run build` | 0 / 0 / 0 |
| 10.2 | `npm run test:e2e -- st49-client-version` | 0 |
| 11.1 | 申し送りの grep / `./tools/check-shared-appends.sh` | 0 / 0 |
| 11.2 | `cargo fmt --all -- --check` / `cargo clippy --workspace --all-targets -- -D warnings` / `cargo test -p ashiato-server` / `cargo test -p ashiato-build-stamp st49_` / 検査 4 本（migrations・boundaries・openapi・immutable）/ `tools/smoke.sh` / `check_scenarios.py` | 0（7 本とも。`cargo test -p ashiato-server` は 1 回目 101 → 直して 0） |

final の後の指摘の直し（d846989・47f6eda）で走らせたもの（review/code.md の各 R の処置に記録）: `check-build-stamp.sh` 4 節 rc=0、`cargo test -p ashiato-build-stamp` 8 passed、
`drops_tests` 15・`media_missing_tests` 6・`st49_` 45 passed、`:app:testDebugUnitTest` 636 本 failures 0・`:app:detekt` rc=0、`./tools/check-migrations.sh` rc=0。

## レビュー（`review/code.md`）

- final review: Critical 0 / Important 3 / Minor 13（R1〜R13）。fixed 9・rejected 4（R9〜R12。理由は各 R）
- code-verify: 申告と実測は一致。壊しても落ちなかった 2 つを R15（C-01 の版の配線）・R16（`.dirty` の道）に出し、どちらも試験を足して直した。R14（ST41 の移行の書き換えで指紋の警告が残る）は上の「確認バッチで見てほしいこと」
- security-review: 指摘なし（版を書く 3 つの口は、書き手を `authorize(..)?` の戻り値からしか作れない）
- `/code-review`: 8 件（R17〜R24）。fixed 7（R18 `.git/index` の監視を外す・R19 git の無い C-01 の組み立て・R20 commit の分解をやめる・R21 送る欄の補い（D3（仮））・R22 振り分けを `Writer::collector_version` に・R23 CI に `build-stamp` job・R24 移行の注記）・rejected 1（R17 は R14 と同じ事実）
- 直した後の範囲を限った再レビュー: 1 回（7da9d42..47f6eda）。R15・R16・R18〜R24 の処置は差分に入っていて、新しい指摘なし。古いままだった注記 2 か所（design D6 の `providers.exec` → `ValueSource`、`St49OutboxTest` の頭書き）だけ直した。`build-stamp` job の旗が `gradle.properties` / `settings.gradle.kts` / 各 crate の `Cargo.toml` を含まないことは気づきとして残す（今壊れる経路は無い）

## 未処置の申し送り

- `docs/handoff/ST50.md`: 古い S-01・古い収集アプリの組み合わせで版が落ちる間を、期限の後に送信契約の版（FR-125 / FR-130）で止める（D3（仮）・Risks）
- `docs/handoff/ST23.md`（R5）: ST23 が足す削除の台帳の INSERT に `server_version` を書く（`st49_ledger_inserts_name_server_version` が落として知らせる）
- `docs/handoff/ST29.md`: 衛星の資格情報は部品の版の欄に書かない（決定の記録。D9）
- `docs/handoff/ST52.md`: 組み立ての印の版番号の出所を 1 つにしたら、3 か所（`crates/build-stamp`・`app/build.gradle.kts`・`web/build-stamp.ts`）をそこから読むように替える
- 以前ここに挙げる予定だった「`check-build-stamp.sh` の CI への組み込み」（R3）は R23 で `build-stamp` job を足したので、残りではない
