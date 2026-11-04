# CH09 spec（--draft）の整合レビュー

対象: `openspec/changes/ch09-load-tolerant-timing-tests/`（proposal・specs 3 本・design・tasks・deep・premises・review/deep.md）と `docs/changes/CH09.md`。
第 1 回の答えは無く、`deep.md` の推奨を答えとして読んだ版を見た。

## 機械の検査（最初に写す）

| コマンド | rc | 結果 |
|---|---|---|
| `openspec validate ch09-load-tolerant-timing-tests --strict` | 0 | valid |
| `python3 scripts/check_chain.py .` | 0 | chain: OK（change なので観点 8 は無い） |
| `python3 scripts/check_scenarios.py . ch09-load-tolerant-timing-tests` | 1 | 担保なし 13 件。全部 `test-timing` の ADDED の Scenario で、spec の段なので試験はまだ無い。13 本とも tasks のどれかが印を名指している（1.1: 2 本・2.1: 3 本・3.1: 2 本・3.2・4.2・6.1・9.1・9.2・9.3 が各 1 本）。MODIFIED の 7 本は既存の試験に印があり、FAIL に出ていない |
| `python3 scripts/review_triage.py . ch09-load-tolerant-timing-tests` | 1 | `deep.md R6: kind=daily は仮でしか閉じられない`（下の R3） |
| `python3 scripts/spec_overlap.py candidates . ch09-load-tolerant-timing-tests` | 0 | `[]` |

ほかに走らせたもの:
- MODIFIED 2 本の写し: 正典（`openspec/specs/{external-ingestion,test-database}/spec.md`）の同じ名前の Requirement の全文と diff を取った。違うのは、導出元の行への追記・理由の 2 行（external-ingestion）/ 1 行（test-database）の追加と、書き直すと宣言した Scenario の WHEN / THEN だけ。Requirement の本文（SHALL の行）とほかの Scenario（external-ingestion 3 本・test-database 2 本）は字面まで同じ。**写しは正しい。**
- `scripts/issue_body.py` の `section()` を `docs/changes/CH09.md` に当てた（R2）

呼び出し元の 4 つの関心への答え:
- 「いまの振る舞いが残ること」の Scenario 化 → 製品の振る舞いは正典の既存の Scenario とその試験が持ち、CH09 は「製品の差分 0 行」（Scenario 12）と「名前・印・本数」（Scenario 13）で固定している。抜けは**既存の試験の期待の行**で、関門（`check_agreed_tests.py`）に当たる計画が無い → R1。1 の試験の「積んでいる途中の前景の変化が積み込みの後にならない」は正典 `desktop-collection` の「訪問の多い取得でも前景の観測は止まらない」が持っている（`openspec/specs/desktop-collection/spec.md:721-725`）ので抜けではない
- MODIFIED の判断と写し → 写しは正しい（上）。判断は Requirement 本文の「1 秒以内」との距離が広がる点だけ → R5（理由の行の字面は R11）
- tasks の検証・break-check・負荷の台本の順 → R1・R6・R7・R8・R12・R13。「直す前の再現を先に取る」は Task 1.2 が Task 3・4 の前にあり、順は成立している
- D と Q の対応・反転条件 → R9・R10。D1〜D11 は Q0〜Q10 と 1 対 1 に対応している（D1=Q1・D2=Q8・D3=Q2・D4=Q5・D5=Q4・D6=Q3・D7=Q6・D8=Q7・D9=Q9・D10=Q0・D11=Q10）

---

## R1. 既存の正典の Scenario の下の期待の行を消すのに、`agreed-test-changes.md` の計画も `check_agreed_tests.py` の検証も無い。Task gate で止まる
- 成果物: openspec/changes/ch09-load-tolerant-timing-tests/tasks.md（3.1・3.3・5.1・9）
- 根拠:
  - `.harness2/skills/story/task.md:33-36`:「既にある Scenario のテストの期待（assert / expect の行）を、実装に合わせて書き換えない … Task gate の `check_agreed_tests.py` が止める … 期待は同じでテストの書き方だけを直すなら、`openspec/changes/<change>/agreed-test-changes.md` に `- Scenario: <名前> — <理由>` を書く」
  - `scripts/check_agreed_tests.py` の docstring: 通すのは「change の spec が MODIFIED / REMOVED / RENAMED で名指しした Requirement の Scenario」と「`agreed-test-changes.md` に書いたもの」だけ
  - Task 3.1 は `crates/collector-windows/src/runtime.rs:3610` の `/// Scenario: 訪問の多い取得でも前景の観測は止まらない`（正典 `desktop-collection/spec.md:721`）の下の `assert!(slowest < … POLL_INTERVAL_SEC …)`（`:3679-3682`）を消す。Task 3.3 は `:1934` の `/// Scenario: 基準の読み取りが長引いても前景の切り替えは記録に残る`（正典 `desktop-collection/spec.md:595`）の下の `assert!(started.elapsed() < 30s)`（`:1952-1955`）を消す。`desktop-collection` はこの change の MODIFIED に無い
  - Task 5.1 の `history/collect.rs:867-870`・`:966` の `assert!(started.elapsed() < 10s)` などを期限の手伝いに替えると、印の付いた試験の下の期待の行が変わりうる
  - `grep -rn agreed-test openspec/changes/ch09-load-tolerant-timing-tests/ docs/changes/CH09.md` は 0 件
- kind: technical
- 提案: 3.1・3.3（と 5.1 で印の下の期待の行を触るもの）に「`agreed-test-changes.md` に `- Scenario: <名前> — <理由>` を書く」を足す。理由は正典の Scenario の文が時間を言っていないこと（期待は同じ）。Task 9 に `python3 scripts/check_agreed_tests.py . --change ch09-load-tolerant-timing-tests` rc=0 を置く。人間が merge で見る `[NOTE]` が、そのまま「どの期待を書き換えたか」の一覧になる
- 処置: fixed tasks.md 3.1・3.3・5.1・9.4（agreed-test-changes.md に書く・check_agreed_tests.py を検証に）・design.md D13

## R2. `docs/changes/CH09.md` の「価値」に依頼の全文が見出しごと入っていて、機械が読む「完了の判定」は上流が書いた方ではない
- 成果物: docs/changes/CH09.md
- 根拠:
  - `docs/changes/CH09.md:12-59`: 「## 価値」の下に `# 実時間で…`（:14）・`## なぜ`（:16）・`## 依頼`（:31）・`## 範囲の外`（:47）・`## 完了の判定`（:52）が見出しのまま入っている。:59 に雛形の注記「上流の深掘りの後に、何のために変えるかを 1〜3 文に整える」が残っている
  - `scripts/issue_body.py:45-53` の `section()` を当てた結果: `section(t, "価値")` は `''`（:14 の H1 で切れる）。`section(t, "完了の判定")` は :52 の節（依頼の元の 3 行と雛形の注記）を返し、上流が書いた :118 の節を返さない。`issue_body.py:128` がこれを完了の判定として使う
  - ほかの change（CH03・CH06・CH07・CH08）の「価値」には見出しが無い（`grep -n "^#" docs/changes/CH0{3,6,7,8}.md`）
- kind: technical
- 提案: 「価値」を見出しの無い 1〜3 文に書き直す（依頼の全文は「依頼（逐語）」に引用で既にある）。`## 完了の判定` を文書に 1 つにする
- 処置: fixed ../../../docs/changes/CH09.md （「価値」を見出しの無い 3 文に。依頼の全文は「依頼（逐語）」にある。`## 完了の判定` は 1 つ。issue_body.section で確かめた）

## R3. deep-review の R6（kind: daily）が仮でない処置で閉じている
- 成果物: openspec/changes/ch09-load-tolerant-timing-tests/review/deep.md（R6）
- 根拠: `python3 scripts/review_triage.py . ch09-load-tolerant-timing-tests` → `[FAIL] deep.md R6: kind=daily は仮でしか閉じられない`。`review/deep.md` R6 の処置は `fixed deep-questions.json Q5（…）・Q9（…）`。対応する `design.md:72` の `D4（仮）`（と `:113` の `D9（仮）`）は既に（仮）と反転条件を持つ
- kind: technical
- 提案: R6 の処置を `fixed D4 仮`（Q9 の分も含めるなら `D4 D9 仮`）の形に書き換える
- 処置: fixed review/deep.md （R6 の処置を `fixed D4 D9 仮` に）

## R4. Q5 の「30 秒」と、Requirement 2 の「文言に待った時間と最後に見た値」は、どの Scenario も確かめていない。直さなくても全部緑になる
- 成果物: openspec/changes/ch09-load-tolerant-timing-tests/specs/test-timing/spec.md（Requirement「条件を待つ試験は実時間の期限まで待ち、届いた時点で進む」）・tasks.md（5.1・7.1・8.1）
- 根拠:
  - `specs/test-timing/spec.md:35-38`: 本文は「相手が進むのを実時間で待つ箇所を…実時間の期限（Rust と Kotlin は 30 秒）で落ちる形にする。落ちたときの文言には、待った時間と最後に見た値を出す」。この Requirement の Scenario は画面の設定（:41-44）と 2 の試験の break-check（:46-49）の 2 本だけで、30 秒の数値はどの Scenario にも無い（Q5・Q8 の実時間 30 秒、Q1 の見張り 30 秒も同じ）
  - `design.md:156`（D14）: `check-inventory.sh` は「候補を持つファイルが全部 `inventory.md` に出ること」を見るだけ。tasks 5.1 の検証（`CW history`・`CT scheduled`・`CT outage`）と 7.1（`KT *UnavailableLogGateTest*`）は試験が通ることしか見ない
  - したがって `scheduled_tests.rs` の `for _ in 0..200`（25ms×200）や `UnavailableLogGateTest.kt:34` の `await(5, TimeUnit.SECONDS)` が残ったままでも、一覧に「直した」と書けば全部の検証が通る
- kind: technical
- 提案: どちらかにする。(a) Scenario を足し、`check-inventory.sh`（または別の検査）が「一覧で `条件の待ち`・`直した` とした行に、回数の待ち（`for _ in 0..<数>` ＋ `settle(`/`sleep(`）・`await(5, TimeUnit` が HEAD で残っていない」ことを見る。(b) Requirement の本文を、確かめている範囲（一覧への掲載・2 の試験）に狭める。30 秒を Scenario に残さない限り、実装が値を変えても spec は落ちない
- 処置: fixed specs/test-timing/spec.md （Scenario「一覧で直したとした条件の待ちに…」を足した）・design.md D14・tasks.md 8.1・5.1（`timing-ok:` の印）

## R5. external-ingestion の Requirement 本文「1 秒以内」に対し、書き直した Scenario は 60 秒未満しか見ない。30 秒ごとにしか読み直さない退行が通る
- 成果物: openspec/changes/ch09-load-tolerant-timing-tests/specs/external-ingestion/spec.md
- 根拠:
  - `specs/external-ingestion/spec.md:6`: 「変わってから 1 秒以内に新しい間隔で待ち始める」（変えない本文）
  - `:24-25`: 「変えた後の走査が 10 回入るまでの時計の経過は 60 秒未満」。正典の旧 Scenario は「10 秒の間に 5 回以上」（約 2 秒に 1 回）、旧試験は「10 回で 10 秒ちょうど」（`crates/server/src/archive_scan_interval_tests.rs:255-270`）
  - `design.md:55`（D2）: 進め直しがあっても「間隔 1 秒なら 10 回は 10〜20 秒で入る」。60 秒との間（約 40 秒）は、取り込み器が間隔を 30 秒ごとにしか読み直さない（`archive/worker.rs:2220-2249` の 1 秒ごとの読み直しを 30 秒にする）退行でも埋まる。:15 が言う「古い間隔のまま眠る退行（1,200 秒）」は捕まるが、それより短い遅れは捕まらない
  - Q8 の推奨の数は「例: 60 秒未満」（`deep.md:48`）で、根拠のある値として決めたものではない
- kind: technical
- 提案: 上限を D2 の普段の幅に寄せる（例: 30 秒未満）か、「変えた後の最初の走査は時計の経過 N 秒以内」の Scenario を足して「1 秒以内」に近い所を固定する。どちらも採らないなら、:14-15 の理由に「この Scenario が捕まえるのは 60 秒を超える遅れだけ」と書き、本文と Scenario の距離を正典に残す
- 処置: fixed D2 仮（上限を 30 秒未満に。30 秒ごとの読み直しの退行も捕まえる。数秒の遅れは捕まえないことを正典の理由の行に書いた）・specs/external-ingestion/spec.md・proposal.md・tasks.md 4.1・docs/changes/CH09.md

## R6. break-check は HEAD から使い捨ての木を作るので、3.1・4.1 を commit してからでないと 3.2・4.2 は古い試験を走らせて落ちる。tasks に順が書かれていない
- 成果物: openspec/changes/ch09-load-tolerant-timing-tests/tasks.md（3.2・4.2）・design.md（D11）
- 根拠:
  - `design.md:131`（D11）: `git worktree add --detach <一時の場所> HEAD` に `git apply` する
  - tasks 3.2（:60-63）・4.2（:74-77）は 3.1・4.1 の commit を前提にすると書いていない。`.harness2/skills/story/task.md:44` は「全部 commit する」を求めるが、項目の途中の commit の時点は決めていない
  - 3.1 を commit する前に 3.2 を走らせると、使い捨ての木には旧い 1 の試験（`FakeReader::new`・`slowest` の assert。`runtime.rs:3629`・`:3679-3682`）が入り、壊すと「見回り 1 回が … かかった」で落ちる。期待の文言 `見張りが読みを放した` が出ないので rc=1 になり、壊すと落ちる力があるのに FAIL と記録される
- kind: technical
- 提案: 3.2・4.2 の本文に「3.1 / 4.1 の試験の書き換えを commit してから走らせる」と書く。または `break-check.sh` が作業木の未 commit の差分（`git diff HEAD`）を先に当ててから壊す差分を当てる形にし、2.1 の `--self-test` にその場合を 1 つ足す
- 処置: fixed tasks.md 3.2・4.2（先に commit）・design.md D11

## R7. 直した後の 10 回（9.3）は `--load host` に固定で、直す前の再現（1.2）が `--load db` でしか出なかったとき、直したことの証にならない
- 成果物: openspec/changes/ch09-load-tolerant-timing-tests/tasks.md（1.2・9.3）・design.md（D7）
- 根拠:
  - tasks 1.2（:38-40）: 「2 で再現しなければ `--load db` でもう一度当て」
  - tasks 9.3（:120）: 「`tools/load-repeat.sh --times 10 --load host` を当て」
  - `review/deep.md` R7・R1: 2 の試験の落ち方は CPU の混み具合でなく DB の往復の長さで決まる見込みで、host の busy loop で延びるかは確かめられていない
- kind: technical
- 提案: `load-before.txt` に試験ごとに再現した負荷（`host` / `db`）を残し、9.3 の `after` はその負荷で 10 回通す（両方で再現したなら両方）。完了の判定（`docs/changes/CH09.md:123`）も「直す前に落ちた負荷で」と書く
- 処置: fixed D7 仮（load-before.txt に再現した負荷を残し、after はその負荷で通す）・tasks.md 1.2・9.3・docs/changes/CH09.md

## R8. 負荷の台本・break-check の Requirement 本文が言う止め方・打ち切りが、Scenario にも self-test にも無い
- 成果物: openspec/changes/ch09-load-tolerant-timing-tests/specs/test-timing/spec.md（Requirement「負荷の下で…」「確かめたい性質を壊すと…」）・tasks.md（1.1・2.1）
- 根拠:
  - `specs/test-timing/spec.md:55`: 「台本が終わるときは、通っても落ちても、かけた負荷を止める」。Scenario（:58-62）と tasks 1.1（:33-35）の self-test は `--load none` と `--load host` の `pgrep -P` だけを見る。`design.md:99-100`（D7）の `--load db`（`tools/db.sh` の `exec` の中の busy loop）と、`trap` の INT・TERM は確かめていない。`tools/db.sh:9` は `exec docker compose "$@"` で、DB のコンテナの中の過程は台本の子でないので `pgrep -P` では数えられない
  - `:77`: 「命令には上限の時間を付け、上限で殺されたときも rc=1」。Scenario（:81-94）と tasks 2.1（:47-50）の self-test は「通った」「文言が違う」「差分が当たらない」の 3 つで、上限で殺す場合が無い（3.2・4.2 は `--timeout 900` に頼っている）
- kind: technical
- 提案: 1.1 の self-test に「`--load db` の後に DB のコンテナの中に busy loop が残っていない」（`tools/db.sh exec db pgrep …` などで数える）と「INT で止めても負荷が残らない」を、2.1 に「上限より長く眠る命令は rc=1 と『上限で殺した』」を足し、それぞれ Scenario にする。`--load db` を確かめないなら D7 と 1.2 から外す
- 処置: fixed specs/test-timing/spec.md （Scenario「負荷の台本を途中で止めても…」「上限より長くかかる命令は…」）・tasks.md 1.1・2.1・design.md D7・D11

## R9. 反転条件が、選択肢のうち 3 つを持っていない
- 成果物: openspec/changes/ch09-load-tolerant-timing-tests/design.md（D2・D3・D6）
- 根拠: `deep-questions.json` の options と design の反転条件の突き合わせ:
  - Q8 の 4 つめ「進める前の固定の眠りを延ばす」→ D2 の反転条件（`design.md:61-62`）は「刻みを細かく」「眠りに入ったことを観測」だけ
  - Q2 の 4 つめ「全部探して全部直す」→ D3 の反転条件（`:69-70`）は「Windows の試験も直す」「e2e なども探して載せる」だけ
  - Q3 の 4 つめ「製品のコードに数える口を足して数で見る」→ D6 の反転条件（`:94`）は「どちらも直さない」「上限を延ばす」だけ（Q0 に従う旨も無い）
  - ほかの D1・D4・D5・D7〜D11 は、推奨以外の選択肢を全部持っている
- kind: technical
- 提案: 3 つを足す。Q8 の 4 つめは `settle(25)` を延ばす値と、正典の Scenario を MODIFIED のまま残すか外すか。Q2 の 4 つめは e2e・計測テスト・縦串を直す項目と走らせ方（`docs/testing.md` §4.5 など）。Q3 の 4 つめは「Q0 が足してよいときだけ `stay*.rs` に `#[cfg(test)]` の数える口」
- 処置: fixed design.md D2・D3・D6（反転条件を足した）

## R10. D10 は答え待ちの Q0 に依るのに（仮）が無く、D14 は出所（Q / C）が無い
- 成果物: openspec/changes/ch09-load-tolerant-timing-tests/design.md（D10・D14）
- 根拠: `design.md:120` `### D10. 製品のコードの差分が 0 行であることは、検査で見る（Q0）`。D1〜D9・D11 は推奨の答えから書いて（仮）が付き、`deep-questions.json` の note も「どの問いも…後から書き直せば戻ります（仮でよい）」。`:151` `### D14. 洗い出しの一覧の置き場と形` は Q も「C」も持たず、冒頭（`:3`）の「推奨の答え（Q0〜Q10）と、聞かずに決めたもの（D12・D13。印「C」）」のどちらにも入らない。観点 6 で答え以外の変更を名指して合意を引き継げるのは（仮）の D だけ
- kind: technical
- 提案: D10 に（仮）を付ける（反転条件は既にある）。D14 の出所を書く（聞かずに決めたなら「C」と、一覧の形を変えても戻せるなら（仮））
- 処置: fixed D10 仮・design.md D14（C）

## R11. external-ingestion に足した理由の行の「10 秒ちょうどで 5 回以上」は、旧い Scenario とも旧い試験とも、proposal・CH09.md とも字面が合わない
- 成果物: openspec/changes/ch09-load-tolerant-timing-tests/specs/external-ingestion/spec.md
- 根拠: `specs/external-ingestion/spec.md:14`「止めた時計の上で『10 秒ちょうどで 5 回以上』と確かめると」。正典の旧 Scenario は「10 秒待つ / 5 回以上」、旧試験は「10 回・経過 [10, 11) 秒」（`archive_scan_interval_tests.rs:255-270`）。`proposal.md:33`・`docs/changes/CH09.md:130` は「10 秒ちょうどで 10 回」
- kind: technical
- 提案: 正典に入る文なので、何を書き直したかを 1 つに揃える（例:「旧い Scenario は『10 秒の間に 5 回以上』、試験は『10 回で 10 秒ちょうど』を確かめていた。どちらも…」）
- 処置: fixed specs/external-ingestion/spec.md （理由の行を旧い Scenario と旧い試験の字面に揃えた）

## R12. tasks の検証のうち、長いコマンドの待ち方と本数の下限が規約から外れているもの
- 成果物: openspec/changes/ch09-load-tolerant-timing-tests/tasks.md（1.2・5.1・3.2・4.2）
- 根拠:
  - 1.2（:41）は、負荷の下で `cargo test` を 2 本×10 回（再現しなければ db でさらに 10 回）走らせる `load-proof.sh before` を `scripts/quiet-run` を通さずに走らせる。AGENTS.md「長いコマンド」は `cargo test` を `scripts/quiet-run` で 1 回待つと決めている（9.3 は `quiet-run load` を通している）
  - 5.1（:87）の `CW history`・`CT scheduled`・`CT outage` には `NL` の下限が無い。AGENTS.md「`cargo test <名前>` 型は `-- --list` で 1 本以上あることも確かめる」
  - 3.2・4.2 の `--timeout 900` は、使い捨ての木の組み立てが元の木の `target/` を使い回すことに頼っている（`design.md:132`「共有してよい」、`:189`）。tasks 2.1・3.2・4.2 は `CARGO_TARGET_DIR` の共有を求めていない
- kind: technical
- 提案: 1.2 を `scripts/quiet-run before -- …` で包む。5.1 の 3 本に `NL` を添える（`NL ashiato-server scheduled <下限>` など）。2.1 に「`CARGO_TARGET_DIR` を元の木の `target/` に向ける」を書く（D11 の「してよい」を tasks の決まりにする）
- 処置: fixed tasks.md 1.2（quiet-run）・5.1（NL）・2.1（CARGO_TARGET_DIR を既定に）

## R13. test-timing の「break-check（取り込み器）」の Scenario は 2 つのことを言うが、4.2 の検証は片方しか見ない
- 成果物: openspec/changes/ch09-load-tolerant-timing-tests/specs/test-timing/spec.md・tasks.md（4.2）
- 根拠: `specs/test-timing/spec.md:49` THEN「文言に待った時間と最後に見た走査の数が出る」。tasks 4.2（:75）は `--expect 待った時間` だけを渡し、`break-check.sh` の形（`design.md:131`）は `--expect` を 1 つ受ける
- kind: technical
- 提案: `--expect` を複数受ける（または正規表現 1 つで両方を縛る）ようにして `最後に見た数` も見る。見ないなら Scenario から「最後に見た走査の数」を外す
- 処置: fixed tasks.md 4.2（`--expect` を 2 つ）・design.md D11・specs/test-timing/spec.md（`--expect` を複数受ける）

## R14. Requirement「CH09 は製品のコードを変えず、既存の試験を減らさない」は change の名と分岐点の差分を言うので、archive の後の正典では意味を持たない（前例あり・低）
- 成果物: openspec/changes/ch09-load-tolerant-timing-tests/specs/test-timing/spec.md（:96-111）
- 根拠: Scenario（:103-111）は「分岐点から HEAD までの差分」を比べ、印は change の中の台本（tasks 9.1・9.2）にだけ置かれる。archive の後は `openspec/changes/archive/` の中に移り、誰も走らせない。前例: `openspec/specs/test-hygiene/spec.md:73`「CH03 の後の木は、検査を通る」
- kind: technical
- 提案: 正典に入れるなら、change を名指さない言い方（「試験の時間の直しは製品のコードに触れない」など）にするか、`docs/changes/CH09.md` の「壊してはいけないもの」と tasks の検査に留める。前例があるので、直さないことも選べる
- 処置: rejected: 前例（`openspec/specs/test-hygiene/spec.md:73` の「CH03 の後の木は、検査を通る」）と同じ形で、印は change の中の台本に git で残り、archive の後も check_scenarios.py が拾う。この change の「壊してはいけないもの」を機械に固定する置き場として残す

---

観点 1: deep の決定が正典に写っているか —— Q0（R14 の Requirement・Scenario 12）・Q1（Requirement 1 の 30 秒・2,000 件）・Q3（test-database の MODIFIED の 60 秒・30 秒）・Q6（10 回）・Q8（1,001ms・60 秒）・Q9（10,000ms・30,000ms）・Q10（Requirement 4）は Scenario か本文にある。数値の抜けは R4（Q5 の 30 秒）。Q2・Q4・Q7 は design（D3・D5・D8）と一覧の約束で、観測できる振る舞いを言わないので Scenario は要らない。「当初案を覆したもの」「要件へ戻すもの」は無い（change・答え待ち）。

観点 2: R4・R5・R8・R13。それ以外の Scenario は WHEN / THEN が rc・件数・ms・`git status --porcelain` で言われ、検証できる（確かめた範囲: test-timing 13 本・MODIFIED 2 本）。

観点 3: R14。design の D のうち観測できる振る舞いを言うもの（D7 の rc、D11 の rc と打ち切り）は spec の Requirement に入っている（打ち切りの Scenario の抜けは R8）。proposal の Capabilities（`test-timing` 新・`external-ingestion`・`test-database`）と `specs/` のディレクトリは一致する。change の capability は INDEX に足さないと proposal（:43）が書いている。`docs/changes/CH09.md` の front matter は `capability: []` のまま（CH03 も同じ形。CH01・CH08 は書いている）で、`board.py` は情報として読むだけで着手を止めないので指摘にしない。

観点 4: R1・R6・R7・R8・R12。順は Task 1（台本と直す前の再現）→ 2（break-check）→ 3・4（1 と 2 の試験と壊す差分）→ 5〜7 → 8（一覧）→ 9（検査と 10 回）で、依存の逆転は無い。人間の確認待ちは無い（実機・実データを使わない）。

観点 5: R2（change の文書）。「壊してはいけないもの」5 項目は、Scenario 12・13、Scenario 3・5（壊すと落ちる力）、正典 `desktop-collection` の既存の Scenario、MODIFIED 2 本に当たる（期待の行の抜けは R1）。specs は「依頼（逐語）」の 1〜3 と完了の判定の範囲の中で、越えている所（2 の試験の進め直し・打ち切りの型・画面の設定・2 の試験の break-check）は Q8・Q3・Q9・D11 が持っている。

観点 6: 該当なし（`spec --draft` で、`spec --answers` でない）。

## 観点 7: 走っている change との前提関係
該当なし（spec_overlap.py candidates が 0 件）
