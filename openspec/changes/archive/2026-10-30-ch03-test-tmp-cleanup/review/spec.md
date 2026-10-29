# CH03 spec review（`spec --draft`。推奨の答えで先に書いた版）

対象: `proposal.md`・`specs/test-hygiene/spec.md`・`design.md`・`tasks.md`・`docs/changes/CH03.md`。
入力: `deep.md`（Q1〜Q8 の推奨）・`premises.md`（表と C1〜C4）・`deep-questions.json`・`review/deep.md`。
基準: HEAD 056f5126（docs/ch03-upstream）、`origin/main` f0c6cf14、走っている枝 `feat/st09-media-capture` 13e4fe6e・`feat/st13-scheduled-account-fetch` cd1a4c00・`feat/ch01-st17-review-fixes` 84acd2e4・`feat/ch02-export-next-record-window` 6fe91fed（2026-10-23）。

## 機械の検査

```
openspec validate ch03-test-tmp-cleanup --strict      → valid（rc=0）
python3 scripts/check_chain.py .                      → chain: OK（rc=0）
python3 scripts/check_scenarios.py . ch03-test-tmp-cleanup
                                                      → FAIL（担保なし 16 件。CH03 の 16 本の全部。rc=1）
python3 scripts/review_triage.py . ch03-test-tmp-cleanup
                                                      → FAIL（4 件: review/deep.md の R4・R5・R10・R12 が escalated なのに deep.md に R4/R5/R10/R12 が無い。rc=1）
```

- check_scenarios の 16 件は、draft の段では試験がまだ無いので出て当たり前。16 本の名前は全部 `tasks.md` のどれかの Task に一字一句で出ることを確かめた
  （`grep -cF` で 1 本あたり 1〜2 回: 8.2 / 2.2 / 5.2 / 7.1 の 7 本 / 7.2 / 1.3 の 4 本 / 8.1）
- 補助に、`scripts/evidence.py` の `shorthands()`・`declared()`・`lint()` を tasks.md に当て、verify-run が実際に走らせるコマンドを出した（R2 の根拠）。`lint` は 0 件

---

## R1. review_triage が FAIL: escalated の R4・R5・R10・R12 が deep.md のどの問いにも名指されていない
- 成果物: openspec/changes/ch03-test-tmp-cleanup/deep.md
- 根拠: `python3 scripts/review_triage.py . ch03-test-tmp-cleanup` → `[FAIL] deep.md R4: escalated なのに deep.md に R4 が無い`（R5・R10・R12 も同じ）。review/deep.md:52（R4 → Q6 の選択肢 2）・:59（R5 → Q7）・:94（R10 → Q8）・:108（R12 → Q1）で処置は付いているが、deep.md:5-43 には R の番号が 1 つも無い
- kind: technical
- 提案: deep.md の Q1・Q6・Q7・Q8 の問いの行に、出所の `（review/deep.md R12）` などを書き足す（答えの欄は触らない）。spec の段の成果物ではないが、この FAIL が残ると triage の関門で止まる
- 処置: fixed deep.md —— Q1・Q6・Q7・Q8 の問いの行に出所（review/deep.md R12・R4・R5・R10）を書き足した（答えの欄は触らない）

## R2. tasks の略記 `LEAK` と `CW` を verify-run が読めず、依頼 3 の検証（2.3・3.3・4.1・6.2・8.2）は何も走らない
- 成果物: openspec/changes/ch03-test-tmp-cleanup/tasks.md
- 根拠: `scripts/evidence.py` の `SHORTHAND = r"\*\*`([A-Z]{1,3}) (<[^>`]+>)`\*\*"` は 1〜3 文字の名前しか拾わないので、4 文字の `LEAK` は略記にならない。`CW` は定義（tasks.md:19「同じ形で `-p ashiato-collector-windows --lib`」）の後ろの最初の backtick に `<名前>` が無いので雛形を持たない。
  `shorthands(tasks.md)` の結果は `{'CT', 'KT'}` だけで、`declared()` は **2.3・3.3・4.1・6.2・8.2 が `[]`**（`LEAK …`・`CW` は `CMD_LIKE` に当たらず黙って落ちる）。
  8.2 は `docs/changes/CH03.md:68-69` の「完了の判定」の 1 つ目が名指す検証で、4.1 は PC の収集アプリの試験の全部を走らせる唯一の検証。どちらも証跡無しで `[x]` が付けられる。
  あわせて、`CT` の雛形は 1 つ目の backtick（`bash -c '… cargo test -p ashiato-server <名前>'`）だけで、`-- --list … | grep -c ': test$'` の下限（tasks.md:18。各項目の「99 本以上」など）は散文に残り、走らない
  （いまの木では `-- --list` で export_ 129・migrate_backup 4・archive_tests 73・archive_flow_tests 105・collector-windows --lib 283 本。書いた下限は満たす）
- kind: technical
- 提案: `LEAK` を 3 文字以下（例 `LK <引数…>`）にする。`CW` は `bash -c '… cargo test -p ashiato-collector-windows --lib <名前>'` を 1 つの backtick で書き、4.1 は絞り込みを渡す形（`CW ::`など空でない引数）にするか、全文を直書きする。
  略記は仮引数を 1 つしか置き換えないので、下限を効かせたい項目は略記を使わず、`bash -o pipefail -c '… cargo test -p ashiato-server export_ && [ "$(cargo test -p ashiato-server export_ -- --list | grep -c ": test$")" -ge 99 ]'` のように**全文を 1 つの backtick に直書きする**（`CT` の雛形に 2 つ目の仮引数を置くと、展開の後に残って `lint` が落とす）。直した後に `evidence.py` の `declared()` で 5 項目が空でないことを確かめる
- 処置: fixed tasks.md —— 略記を CT・CW・LK・KT・NL（`-- --list` の下限）の 1〜3 文字・1 仮引数にし、CW は `<名前>` を持つ雛形、4.1 は `CW ::` と下限の直書きにした。`evidence.declared()` で全項目が空でないこと、NL と 8.3 が今の木で rc=0 になることを走らせて確かめた

## R3. CH03.md の「壊してはいけないもの」が空。spec にも、試験の期待値・印・名前を変えないことの Scenario が無い（Requirement の本文だけが言っている）
- 成果物: docs/changes/CH03.md / openspec/changes/ch03-test-tmp-cleanup/specs/test-hygiene/spec.md
- 根拠: docs/changes/CH03.md:60-62 は雛形の「（上流が深掘りの後に書く。…）」のまま。依頼の「触らない」（CH03.md:33）と C4（premises.md:42）と D7（design.md:104-109）が言うことは、spec では Requirement 1 の本文（spec.md:12「試験の期待値・`Scenario:` の印・試験の名前は、この約束のために変えない」）だけにあり、`#### Scenario:` が無い。
  同じく本文 spec.md:11「試験が使う一時フォルダは、その試験が最後に使うまで在り続ける」にも Scenario が無い（D1 の早すぎる drop。書き出しが置き場を作り直す場合は Scenario 1 の「空のまま」で間接に落ちるが、作り直さない経路では何も落ちない）。
  機械の確かめは Task 8.3 にあるが、`Scenario:` を含む消した行が 0 と、`#[test]`・`@Test` の数が減っていないことだけで、試験の名前と assert の行（D7「差分に出さない」）は見ていない
- kind: technical
- 提案: CH03.md の「壊してはいけないもの」に、(a) 既存の試験の期待値・`// Scenario:` の印・試験の名前、(b) 製品のコードの置き場の決め方（export.rs など）を書く。spec に「CH03 の差分は、既存の試験の印・名前・assert を消さない」の Scenario を 1 本足し、8.3 の検査（印は既にある形）に試験の名前の集合の比較を足してその印を置く。
  「最後に使うまで在り続ける」は Scenario にできる形（例: 手伝いが返した値を束縛しない書き方を検査が落とす、か、`tmp-leak-check` が拾う経路に限ると本文で絞る）にするか、本文から外して design D1 の注意に留める
- 処置: fixed 8.3 —— 消した行の印と試験の名前が足した行にもあることを 8.3 に足した（消してよい手伝いの名前は design D7 に列挙）。docs/changes/CH03.md の「壊してはいけないもの」を書いた（proposal.md の冒頭の注記に記録）。Requirement 1 の本文から「最後に使うまで在り続ける」「期待値を変えない」を外し、Scenario にしない理由を design D7 に書いた（CH03 の差分の性質で後も続く振る舞いではない）

## R4. Kotlin の手伝いを property の初期化で呼ぶ試験は 4 つでなく 10 ファイル。D2 と Task 5.1 は 4 つにしか `lazy` / `@Before` を指示していない
- 成果物: openspec/changes/ch03-test-tmp-cleanup/design.md（D2）/ tasks.md（5.1・6.1）/ premises.md（C2）
- 根拠: `grep -rnE '^\s{4}(private |internal )?(val|var) \w+(: [^=]+)? = (testOutbox\(|testHeartbeatOutbox\(|TestStores\(|AppUsageWorld\(|St04World\()' collector-android/app/src/test` = 10 ファイル（`AppUsageFixtures.kt:35`・`UnreadableLineTest.kt:19`・`SegmentMigrationTest.kt:19`・`RollupImportTest.kt:469`・`WriteFailedTest.kt:18`・`DropReportSendTest.kt:17`・`RetentionTest.kt:15`・`RetentionNotifierTest.kt:30`・`DropReportTest.kt:17`・`DrainTest.kt:17`）。
  D2（design.md:50-53）は手伝いを `TestStores(dir: File, …)` にして呼び出し側が `tmp.newFolder()` を渡す形にするが、`private val st = TestStores(tmp.newFolder())` は規則の `before()` より前に走り `IllegalStateException`（premises.md:14 の前提そのもの）。D2 が `lazy` / `@Before` を言うのは `createTempDirectory` を直に呼ぶ 4 つ（design.md:15・52）だけ。
  あわせて数が食い違う: design.md:15・proposal.md:43 は「17 ファイル・49 か所」（`TestStores(` を数えていない）、tasks.md:90 は「26 ファイル」（`TestStores(` を含む。呼び出しは 60 行）
- kind: technical
- 提案: D2 と 5.1 に「手伝いを property の初期化で呼んでいる 10 ファイルも `by lazy` か `@Before` に移す」と名指しで足す（5.1 の全体の走りで落ちて気づく形ではあるが、fix の回数を使う）。design / proposal の数を `TestStores(` を含む 26 ファイル・60 行に揃える
- 処置: fixed D2 —— 手伝いを property の初期化で呼ぶ 10 ファイルを D2 と 5.1 に名指しし、数を 26 ファイル・60 行に揃えた（proposal・design・premises）

## R5. 走っている st09・st13 は、rebase の後に検査で落ちるだけでなく、手伝いの引数が変わってコンパイルが通らなくなる。proposal の Impact と Q4 の context に無い
- 成果物: openspec/changes/ch03-test-tmp-cleanup/proposal.md（Impact の「走っている Story・change との重なり」）/ premises.md:16-17
- 根拠: `git diff origin/main...<branch> -- crates collector-android | grep '^+'` で、C2 / D1 が形を変える手伝いを新しく呼んでいる:
  st09 は `testHeartbeatOutbox()`・`var outbox … = testOutbox()`・`val st = TestStores()`・`class OriginalHarness(val st: TestStores = TestStores())`・`TestStores(segmentBytes = 300)` ×2、
  st13 は `testHeartbeatOutbox()` 1 か所（premises.md:16 は st13 を Rust の 4 か所だけと書く）と、`archive_flow_tests.rs` の `Inbox::new(` 15 か所・`inbox.root` などの欄 16 か所（Task 3.2 が `Inbox` を `TempDir` を持つ形に寄せる）。
  手伝いの既定の引数を外す（D2）と、これらは検査の前に `compileDebugUnitTestKotlin` で落ちる。ch01・ch02 の追加行には当たらないことも確かめた。
  また ch01 は `docs/testing.md` も直している（`git diff --stat origin/main...feat/ch01-st17-review-fixes`）が、Task 7.4 が直す同じファイルなのに「本文が衝突しうるファイル」に無い
- kind: premise
- 提案: Impact に 3 つ目の項「rebase の後にコンパイルが通らない（手伝いの引数）」として st09・st13 の上の呼び出しを書き、`docs/testing.md`（ch01）を衝突の一覧に足す。Q4 の推奨（gate → fix）はそのままでよいが、
  D1 の 3.2 は `Inbox::new(name)` と `root`・`inbox` などの欄の型を変えずに `TempDir` を別の欄で持つ形を既定にすると、st13 の 31 か所が壊れずに済む（D2 の Kotlin は引数を足す以上避けられないので、そう書く）
- 処置: escalated —— 走っている枝の手間の見積もり（Q4 の前提）が違っていたので、Q4 の context に「コンパイルが通らない」（st09・st13 の手伝いの呼び出し）と ch01 の docs/testing.md を足した（deep-questions.json。deep.md の Q4 に出所を書いた）。推奨は変えない（Rust の Inbox は形を残す: D1・3.2）。proposal の Impact にも書いた

## R6. Task 1.4 の検証は「いまの木で漏れること」を rc=1 で見るので、Task 6 の後に走らせ直すと必ず落ちる。code-verify の手 5 が `[x]` を走らせ直す
- 成果物: openspec/changes/ch03-test-tmp-cleanup/tasks.md（1.4）
- 根拠: tasks.md:52 の検証は `[ $rc -eq 1 ] && grep -q "seg" …`。Task 6.1 が `SegmentStoreTest` を直すと rc=0 になり、この検証は FAIL になる。`docs/testing.md:142`「code-verify の手 5 が `[x]` を 1 件ずつ走らせ直す」、`.claude/agents/code-verify.md:59-61`「`[x]` のタスクを 1 件ずつ取り、本文の検証方法が実在し、rc=0 になるかを見る」。
  もう 1 つ、判定の手掛かりが弱い: rc=1 は試験の失敗・0 本・残り、のどれでも出て、区別は `grep -q "seg"` の 3 文字だけ（試験が落ちたときの出力にも小文字の `seg` は出うる）
- kind: technical
- 提案: 前提（試験の JVM の `java.io.tmpdir` が `ASHIATO_TEST_TMPDIR` に向く）を、いつ走らせても同じ結果になる形で確かめる。例: `java.io.tmpdir` が環境変数の値と等しいことを assert する小さな試験（`Ch03TmpdirProbeTest`、環境変数が無ければ skip）を足し、
  `ASHIATO_TEST_TMPDIR=$(mktemp -d …) KT *Ch03TmpdirProbeTest*` を検証にする。漏れを拾えることは 1.3 の `--self-test`（偽の命令で 1 つ残す）が持っている。残すなら、grep は台本が残りを挙げる行の決まった形（例 `残り: seg…`）で見る
- 処置: fixed 1.4 —— 「いまの木で漏れる」をやめ、java.io.tmpdir が環境変数の値になることを見る探りの試験（Ch03TmpdirProbeTest）にした。いつ走らせても同じ結果になる

## R7. D5（検査を commit 前にも掛ける）は観測できる振る舞いなのに design にしか無く、spec は CI の chain job を Requirement 3 の Scenario の AND で言うだけ
- 成果物: openspec/changes/ch03-test-tmp-cleanup/design.md（D5）/ specs/test-hygiene/spec.md（Requirement 2・3）
- 根拠: 依頼 2 は「落ちる検査（tools/check-*.sh と CI）を足す」（CH03.md:29-30）。spec の Requirement 2（spec.md:29-71）はどこで走るかを書かず、chain job は Requirement 3 の最後の Scenario の `AND`（spec.md:100）にだけ、`tools/pre-commit.sh` は spec のどこにも無い（`grep -n 'pre-commit' specs/test-hygiene/spec.md` = 0）。D5（design.md:78-83）の「commit 前に掛ける」は commit が断られるという観測できる振る舞いで、archive で正典から落ちる。
  Requirement 3 の最後の Scenario は rust / android job（台本から走る）と chain job（検査を走らせる）の 2 つの主張を 1 本に束ねている
- kind: technical
- 提案: Requirement 2 に「CI の chain job と commit 前の検査が、`--self-test` と本体を走らせる」の Scenario を 1 本立て（印は 8.1 の self-test の ci.yml を読む段と、`tools/pre-commit.sh` を読む段）、Requirement 3 の最後の Scenario から chain job の AND を外す
- 処置: fixed specs/test-hygiene/spec.md —— Requirement 2 の本文に chain job と commit 前を書き、Scenario「検査は CI の chain job と commit 前に走る」を立てた（印は 7.3）。Requirement 3 の最後の Scenario から chain job の AND を外した

## R8. 検査の他の観測できる決まり（離れた印・コメントの中・追跡されないファイル）と、残りを測る台本の出力の形が design にしか無い
- 成果物: openspec/changes/ch03-test-tmp-cleanup/design.md（D3・D4・D6）
- 根拠: D3（design.md:60）「追跡されていないファイルは見ない」、D4（design.md:72-73）「2 行以上離れた印は数えない」「コメントの中の文字列も拾う」、Kotlin は「同じ行か 1 行上に `@Rule` / `@get:Rule`」（design.md:70。spec.md:34 は「付かない」とだけ言い、何行上までかが無い）、
  D6（design.md:87・95-96）「引数が無ければ `--workspace`」「残った名前を最大 20 件と総数」「試験の rc が 0 でなければその rc で終わる」（spec.md:91 は「0 でない rc」で、本文 spec.md:78「試験の終了コードで落とす」より弱い）「rc=0 なら根を消し rc=1 なら残す」。
  どれも検査・台本の外から見える応答の形で、`openspec archive` は design を正典に写さない
- kind: technical
- 提案: 少なくとも「2 行以上離れた `tmp-ok:` は数えない」「`@Rule` は 1 行上まで」「試験が落ちたときは試験の rc をそのまま返す」を Scenario にする（self-test に 1 段ずつ）。spec.md:61-63 の tmp-ok の Scenario は WHEN が「同じ行か 1 行上」の両方を 1 本に入れているので、同じ行・1 行上・2 行上を分ける。残りは design に置くなら、仮でない実装の決めとして読めるように「（spec に無い。変えても落ちない）」と添える
- 処置: fixed specs/test-hygiene/spec.md —— tmp-ok を「同じ行か 1 行上なら通る」「理由無し・2 行以上離れたら通らない」の 2 本に分け、@get:Rule の 1 行上を通る・2 行上は落ちるを Scenario に入れ、試験の rc をそのまま返す（rc=101）にした。残り（追跡ファイルだけ・名前の件数・根を残す）は design に「spec に無い実装の決め」と添えた

## R9. `history/ledger.rs` の試験の手伝いの名前が `temp_dir()` で、D4 の `temp_dir\(\)` に当たる。Task 4.1 は名前を変えるとは書いていない
- 成果物: openspec/changes/ch03-test-tmp-cleanup/tasks.md（4.1）/ design.md（D4 の「いまの木で印が要る所」）
- 根拠: `grep -rn 'temp_dir()' crates | grep -v 'env::temp_dir()'` → `crates/collector-windows/src/history/ledger.rs:146,165`（呼び出し）・`:183`（`fn temp_dir() -> std::path::PathBuf`）。spec.md:33 は「Rust の `temp_dir()`（`std::env::` の有無を問わない）」を落とすので、4.1 の言うとおり「`TempDir` を返す形にする」だけでは 3 行が残り、7.2 で落ちる。design.md:74-75 の「印が要る所」の一覧にも無い
- kind: technical
- 提案: 4.1 に「`ledger.rs` の手伝いは `TempDir` を返し、名前を `temp_dir` でないもの（例 `scratch()`）に変える」と書き、D4 の一覧に足す
- 処置: fixed 4.1 —— ledger.rs の手伝いの名前を変えると書き、D4 の一覧に足した

## R10. Task 3.2 の「`workers` の Drop の順は変えない」と「`TempDir` は最後に落ちる欄に置く」が、いまのコードの順と食い違う
- 成果物: openspec/changes/ch03-test-tmp-cleanup/tasks.md（3.2）
- 根拠: `crates/server/src/archive_flow_tests.rs:24-39`。いまは `impl Drop for Inbox` の `remove_dir_all(&self.root)` が欄の drop より**先**に走り、その後で `workers`（最初の欄）が落ちる。つまり置き場が消えてから取り込み器が止まる。`TempDir` を最後の欄に置くと、取り込み器が止まってから置き場が消える —— 順は変わる（よい向きに見えるが、「変えない」とは逆）
- kind: technical
- 提案: 3.2 を「取り込み器（`workers`）が止まってから置き場が消える順にする（いまは逆。`TempDir` を `workers` より後ろの欄に置く）」と、意図した順で書き直す
- 処置: fixed 3.2 —— 「取り込み器が止まってから置き場が消える（いまは逆）」と意図した順で書いた

## R11. capability `test-hygiene` を立てた判断: schema は「tooling」を skip_specs の例に挙げ、INDEX の capability 表にも無い
- 成果物: openspec/changes/ch03-test-tmp-cleanup/proposal.md（Capabilities）/ docs/stories/INDEX.md
- 根拠: `.harness2/openspec/schemas/story/schema.yaml:63-66`「Use `skip_specs: true` only when no spec-level behavior changes (pure refactor, tooling, docs)… Do not invent a requirement just to satisfy validation」。
  既存の検査（`check-log-private.sh`・`check-private.sh`・`check-secrets.sh`・`check-shared-appends.sh`）はどれも capability を持たず、`docs/flow-gates.md` の表だけに載る（`ls openspec/specs` の 12 本に道具は無い）。
  `docs/stories/INDEX.md:72-92` は「名前は後から変えられない」として capability を表に載せるが、`test-hygiene` の行も前倒しの注記も無い（proposal.md:34 に理由はある）。
  立てる側の理由もある: 失敗した試験のフォルダが消える（proposal.md:26）と、検査が書き方を断ることは外から見える振る舞いで、16 本の Scenario が check_scenarios で印に縛られる。skip_specs にするとそれが無くなり、tasks の検証だけが残る。
  立てると正典に `tools/check-tmp-hygiene.sh`・`tempfile::TempDir`・`TemporaryFolder` の名前が残り、道具の名前や検査の文言を変える後の change は `test-hygiene` を MODIFIED することになる
- kind: defer
- 提案: 残すなら、INDEX.md の capability 表に `test-hygiene`（積む: CH03）の行と「道具の capability を初めて立てた。理由: …」の注記を足し、proposal に schema の「tooling は skip_specs」の文言を退けた理由を 1 行書く。skip_specs にするなら、16 本の Scenario を tasks の検証（self-test の段）に移し、失うのは正典での固定だけだと proposal に書く
- 処置: rejected: capability を残す。根拠: `.claude/skills/story-upstream/SKILL.md:46`（change の capability は INDEX.md に足さない）・`.harness2/openspec/schemas/story/schema.yaml:63-66` の「skip_specs は spec の振る舞いが変わらないときだけ」に対し、この change は失敗した試験のフォルダが消える（proposal.md の What Changes）・検査が書き方を断る・CI が残りで落ちる、の外から見える振る舞いを足す。理由は proposal の Capabilities に書いた

## R12. commit 前の検査は追跡ファイルの全体を見るので、rebase の後の st09・st13 では漏れを直し切るまでどの commit も止まる。Q4 の推奨は「gate → fix で直す」としか言っていない
- 成果物: openspec/changes/ch03-test-tmp-cleanup/design.md（D5）/ proposal.md（Impact）
- 根拠: D5（design.md:81）「`tools/pre-commit.sh` に `check-log-private.sh` と同じ形で足す（追跡ファイル全体）」、`tools/pre-commit.sh:16-17` は検査が落ちれば `exit 1`。deep.md:23 の Q4 の推奨は「rebase の後、それぞれの gate → fix で直す」で、fix が自分の直しと関係の無い commit を作るとき（直しを Task に分けて進めるなど）も止まることには触れていない。R5 のコンパイルの直しと検査の直しを同じ commit に入れる必要がある
- kind: daily
- 提案: D5 か proposal の Impact に「rebase の後の st09・st13 は、漏れを全部直す commit までどの commit も commit 前の検査に止まる（直しは 1 つの commit にまとめる）」と書く。commit 前は staged のファイルだけを見る形（`check-private.sh --staged` と同じ）にする案も、D5 の退けた案か反転条件として比べる
- 処置: fixed D5 仮 —— 追跡ファイル全体を見る帰結（rebase の後は直しを 1 つの commit にまとめる）・退けた案（staged だけ）・反転条件（rebase の後の直しを 2 回以上妨げたら staged だけにする）を D5 に書き、proposal の Impact にも書いた

## R13. premises の「確かさ 1〜2 は下流の最初の Task が走らせて確かめる」のうち、Rust の側（`TMPDIR` を向けて cargo・試験の道具が根に何も残さない）を確かめる Task が Task 1 に無い
- 成果物: openspec/changes/ch03-test-tmp-cleanup/tasks.md（Task 1）/ design.md（D6）
- 根拠: premises.md:22（`std::env::temp_dir()` と `tempfile` は `TMPDIR` を読む。確かさ 2）と premises.md:28「確かさ 1〜2 で止めた前提は、どれも下流の最初の Task が走らせて確かめる」。Task 1 で前提を走らせるのは 1.4（Android の `java.io.tmpdir`）だけ。
  D6 の反転条件（design.md:101「Rust で cargo か試験の道具が根に残すと分かったときも同じ」）を最初に踏むのは 2.3 の LEAK で、R2 のとおりいまは走らないので、実際には 8.2 か CI まで分からない。
  （doc-test は `cargo test --workspace --doc -- --list` で 0 本なので、`--no-run` が doc-test を組まない点は今日は効かない）
- kind: technical
- 提案: Task 1 に、書き換えの前の木で `tools/tmp-leak-check.sh rust -p ashiato-collector-windows --lib`（DB が要らず速い）を走らせ、残りの名前が ashiato の試験の頭（`ashiato-…` など）だけで、cargo や試験の道具のものが無いことを見る項目を足す。1.4 と同じく、Task 4 の後に走らせ直しても落ちない形にする（R6）。例: 残りの名前の全部が既知の試験の頭に当たることだけを見て rc=0
- 処置: fixed 1.5 —— 書き換えの前でも後でも通る形で、PC の収集アプリの試験の残りの名前が全部 ashiato の頭であることを見る項目を足した。Android の側は 6.2

## R14. Task 4.2 の `runtime_windows.rs` の書き換えは、手元では型の検査（clippy）しか通らない。走るのは CI の windows-latest だけと書かれていない
- 成果物: openspec/changes/ch03-test-tmp-cleanup/tasks.md（4.2）
- 根拠: tasks.md:84-85 の検証は `cargo clippy … --target x86_64-pc-windows-gnu`（組むだけ）。`.github/workflows/ci.yml:110-133` の `collector-windows-runtime` job（`runs-on: windows-latest`）だけが `cargo test -p ashiato-collector-windows --test runtime_windows` を走らせる。書き換えは自前の `TempDir`・`Cleanup` の型を外すので、Drop の順や `%TEMP%` の下の置き場が変わる
- kind: technical
- 提案: 4.2 に「走る確かめは PR の CI の `collector-windows-runtime` job（windows-latest）。手元では clippy まで」と書く。WSL から Windows 側の cargo で走らせる道（`C:\dev\ashiato2-rt`）を使うなら、その検証コマンドを足す
- 処置: fixed 4.2 —— 手元では clippy まで、走る確かめは CI の collector-windows-runtime job と書いた

## R15. CH03.md の「価値」が依頼の逐語の写しと雛形の注記のままで、1〜3 文に整えられていない
- 成果物: docs/changes/CH03.md
- 根拠: CH03.md:12-33 は「依頼（逐語）」（:37-58）と同じ文で、:35 に雛形の注記「（`hx change` の依頼の逐語。上流の深掘りの後に、何のために変えるかを 1〜3 文に整える）」が残っている。spec の段で「完了の判定」は書いたが、「価値」は書いていない
- kind: technical
- kind の直し: daily から technical へ（日常の選択ではなく、雛形の注記の書き残し）
- 提案: 「価値」を proposal の Why と同じ向きの 1〜3 文（例: 試験が一時フォルダを食い潰して並走する検証を落とすのを止め、再発を検査と CI で止める）にし、:35 の注記を消す。内訳の実測は「依頼（逐語）」に残っているので削ってよい

---

## 観点ごとの、該当なしと確かめた範囲

- 観点 1（deep の決定が写っているか）: R3・R7・R8 のほかは該当なし。確かめた範囲: Q1（範囲 = crates/ 全体と app/src/test、`tmp-ok:`）→ spec.md:31-36・48-50・65-67、D3。Q2（兄弟と外す口）→ spec.md:33-34・44-46・52-55、D4。Q3・Q6・Q7・Q8 → design の Non-Goals・spec.md:13・proposal.md:26-27・tasks の Global Constraints。
  Q4（木の全体）→ spec.md:69-71、D3。Q5（CI の既存の段・空のフォルダ・1 本以上）→ spec.md:73-100、D6。数値は「1 本以上」だけで、spec.md:19・85-87 にある。C1 → D1・3.2・4.2、C2 → D2（R4）、C3 → D8・7.4、C4 → D7・8.3（R3）。覆した当初案・要件へ戻すものは無い（`satisfies: []`）
- 観点 2（検証可能か）: R3・R7・R8 のほかは該当なし。確かめた範囲: 16 本の WHEN / THEN はどれも rc・ファイルの有無・名前の列挙で、「正しく」「適切に」の語は無い
- 観点 3（置き場）: R7・R8・R11 のほかは該当なし。確かめた範囲: proposal の Capabilities（`test-hygiene` のみ）と `specs/` のディレクトリは一致。spec に関数名・列名は無い（台本の名前・ライブラリの名前は R11）
- 観点 4（tasks の検証）: R2・R6・R13・R14 のほかは該当なし。確かめた範囲: 依存順（1 足場 → 2〜6 書き換え → 7.2 検査の本体 rc=0 → 7.3 CI と commit 前に掛ける → 8.1 CI の試験の段を台本へ）は鍵の順として正しい。`cargo test` の `-p` と `--workspace` は D6 / 1.3 の「引数が無ければ `--workspace`、あればその引数だけ」で両立し、`--no-run` に絞り込みを渡してもよい。
  gradle の `--rerun` は wrapper が 8.13（7.6 以降にある）。8.3 の `git grep -hc … | paste -sd+ | bc` は `bc` が手元にある。1.1 の crate 名（`ashiato-server`・`ashiato-collector-windows`）、4.1 の 14 ファイル、3.1 の 14 か所、`config.rs:157`・`autostart.rs:75`・`migrate_backup.rs:207` が `#[cfg(test)]` の後ろ（:140・:67・:165）であることは実物と一致。人間の確認待ちが要る項目は無い
- 観点 5（CH03.md）: R3・R15 のほかは該当なし。「完了の判定」は deep の推奨・spec と合っているが、1 つ目が名指す Task 8.2 の検証は R2 のとおり走らない。依頼の範囲の越え（製品のコードも縛る・CI で測る）は Q1・Q5 の推奨に決定がある
- 観点 6: 該当なし（stage は `spec --draft`。`--answers` ではないので answers.md は書かない）

## 実物との照合で食い違いが無かったもの（参考）

- `.github/workflows/ci.yml`: rust job の `- run: cargo test --workspace`（:74）、android job の `defaults.run.working-directory: collector-android` と `- run: ./gradlew :app:testDebugUnitTest`（:199・:212）、chain job の `check-log-private.sh` の 2 行（:301-302）は tasks 7.3・8.1 の書き方と一致
- `tools/check-log-private.sh`: bash の中の python3 標準だけ・`// log-ok: <理由>`・3 行以内・`#[cfg(test)]` 以降は見ない・`git ls-files` は D3〜D5 の前例の記述と一致。`tools/pre-commit.sh:16-17` の形も 7.3 と一致
- `collector-android/app/build.gradle.kts:49-61` の `unitTests.all` と `ASHIATO_ROBOLECTRIC_JARS` の形、`junit:junit:4.13.2`（:160）は D2・D6 の前提と一致
- `/target` は `.gitignore:7` にあり、`target/tmp-leak/` は追跡されない
- 処置: fixed proposal.md —— docs/changes/CH03.md の「価値」を 2 文に整えて雛形の注記を消し、proposal.md の冒頭の注記に CH03.md を書き直したことを記録した

