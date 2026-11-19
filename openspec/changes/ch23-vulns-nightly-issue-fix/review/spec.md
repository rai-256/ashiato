# spec レビュー — ch23-vulns-nightly-issue-fix（CH23・軽い路 bug）

## 機械の検査

- `openspec validate ch23-vulns-nightly-issue-fix --strict` → `Change 'ch23-vulns-nightly-issue-fix' is valid`（rc=0）
- `python3 scripts/check_chain.py .` → `chain: OK (0 件 / 未回収 0 件 / warn 0 件)`（rc=0）
- `python3 scripts/check_scenarios.py . ch23-vulns-nightly-issue-fix` → rc=1。この change の 6 件が `[FAIL] 担保の無い Scenario`
  （知らせの段が落ちた夜は… / 知らせの issue が開いていれば… / 知らせの issue を立てられなければ… / ラベルが無いと空の文字で… / ラベルがあれば… / 知らせの段は…）。
  spec の段では試験がまだ無いので、これは予期どおり（tasks 1.2・2.1・2.2 が `== Scenario:` の印を置く）。ほかの FAIL は無い
- `python3 scripts/review_triage.py . ch23-vulns-nightly-issue-fix` → `指摘 0 件 / 仮決め 0 件 / 要件へ戻すもの 0 件`（rc=0）
- `python3 scripts/spec_overlap.py candidates . ch23-vulns-nightly-issue-fix` → `[]`（rc=0）
- `python3 scripts/evidence.py lint openspec/changes/ch23-vulns-nightly-issue-fix` → `[OK] 検証コマンド`。1.1〜3.2 の検証コマンドはどれも抽出できた（4.1 は人間の確認待ちで無し）

追加で手で確かめたこと（観点 4 の根拠）:
- tasks 3.2 の検証コマンドを今の木で書いたまま走らせた → rc=0（今の差分は `docs/changes/CH23.md` と change の中だけ）
- `tools/check-vulns-wiring.sh` と `--self-test` は今の木で rc=0（基準は緑）
- tasks 1.1 の手順を捨てる写し（`git archive HEAD` を `/tmp` に展開）で再演した:
  (a) 偽の `gh` の `label list` を「`GH_LABEL_EXISTS=1` でなければ何も出さない」に変え `tools/check-vulns.sh --self-test` → rc=1。
  最後の印は `== Scenario: 落ちた夜に、依存ごとに 1 件の issue が立つ`、直後に `error: issue を記録できなかった（gh の応答: Expecting value: line 1 column 1 (char 0)）`。
  (c) `gh_json` の `label list`・`issue list` だけ空白の応答を `[]` にする最小の直しで → rc=0（PostgreSQL の節まで通る）。1.1 は書いたとおり直す前に落ち、直した後に通る
- 3.1 の検証の `grep -E "^\| \`vulns-nightly.yml\`" docs/testing.md` は今の行（docs/testing.md:43）に当たる（`file-issues` で試して rc=0）

## 観点 1: 決定が正典に写っているか

deep.md は軽い路の申告の写しで「本人の答え」は無い。本人の決定（docs/changes/CH23.md:26-28「議論で決めたこと」）を 1 件ずつ当てた:
- 空の文字を空の一覧とみなす → spec.md:12 と Scenario「ラベルが無いと空の文字で答える gh でも…」（spec.md:28-30）。写っている（置き場は R4）
- 偽の gh を本物に合わせる → tasks 1.1 (a)・Scenario spec.md:29。写っている
- 赤が届かなかった理由を調べ、届く形にする → D2 と spec.md:3-11。調べた結果は確かさ 1 のまま（R11）。範囲は R10
- 見つかった当たりの処置はしない → tasks Global Constraints・3.2。写っている
- ラベルが無い状態でも動くことを試験で確かめる → spec.md:28-30。写っている

数値の決定は無い（題の文字列・2 行の本文は Scenario にある）。

## R1. 「検査できなかった夜は知らせない」が、配線の検査の見ない workflow の分岐 1 つに依っている（正典「届かなかった夜は issue を立てない」が守られる根拠が無い）
- 成果物: openspec/changes/ch23-vulns-nightly-issue-fix/specs/dependency-vulnerability-scan/spec.md
- 根拠: spec.md:10「材料を issue にする段が走らなかった夜・rc=0 で終えた夜には走らない（検査できなかった夜は…issue を立てない）」。
  `file-issues` は材料が無いと rc=2 で終わる（tools/check-vulns-issues.py:120-126。正典の Scenario の自己試験も `file-issues` の rc=2 を期待する: tools/check-vulns.sh:413-414）。
  workflow でこれを rc=2 にしないのは、`if: always()` の段の中のシェルの分岐 `if [ -d … ] && find … | grep -q .; then … fi`（.github/workflows/vulns-nightly.yml:64-68）だけ。
  配線の検査（tools/check-vulns-wiring.sh:145-148）も新しい Scenario（spec.md:36-39）もこの分岐を見ない。分岐を消すと、osv.dev に届かない夜に `file-issues` が rc=2 → `report-unfiled` が `issue create` を受け、
  正典の Requirement（openspec/specs/dependency-vulnerability-scan/spec.md:52「rc=2 のとき、毎晩の検査は issue を立てない」）と Scenario「届かなかった夜は issue を立てない」（同:64-66）に反する。
  加えて spec.md:10 の括弧書きは、片方だけ検査できなかった夜（`vulns` が rc=2・`pg-minor` が rc=1 で PostgreSQL の材料だけがある）には成り立たない（`file-issues` が走り、gh が落ちれば知らせの issue が立つ）。
  また「段が走らなかった夜」は字面が違う（段は `if: always()` で毎晩走り、材料が無ければ rc=0 で終える）
- kind: technical
- 提案: Scenario を 1 つ足す（「材料が 1 つも無い夜は、`file-issues` の段は `file-issues` を呼ばずに rc=0 で終え、知らせの段は走らない」）。配線の検査に分岐の検査を足し、自己試験に「分岐を消した写しは落ちる」の `expect_fail` を足す（tasks 2.2）。
  spec.md:10 は「材料が 1 つも無い夜（どの検査も rc=0 か rc=2）には走らない。片方だけ検査できなかった夜は、残りの材料の `file-issues` が落ちたときに走る」と、混じった夜を名指す書き方にする
- 処置: fixed 2.2 — specs の Requirement の本文を「材料が 1 つも無い夜」「片方だけ検査できなかった夜」と名指す書き方に直し、Scenario「材料が無い夜は、file-issues を呼ばない」を足した。2.2 に配線の検査と、分岐を外した写しの expect_fail を足した

## R2. 配線の Scenario の `if:` の条件は、満たしても一度も走らない書き方を通す
- 成果物: openspec/changes/ch23-vulns-nightly-issue-fix/specs/dependency-vulnerability-scan/spec.md
- 根拠: spec.md:38「`if:` がその段の `outcome` の `failure` を条件にし」。GitHub Actions は `if:` に状態の関数（`success()`・`failure()`・`always()`・`cancelled()`）が無いと `success()` を暗に足すので、
  `if: steps.file.outcome == 'failure'` だけの段は、前の段が落ちた夜には決して走らない。この書き方は Scenario を満たす。D2（design.md:23）と tasks 2.2 は `failure() && …` と書くが、それは Scenario の外。
  tasks 2.2 の `expect_fail` は「段を消す」「`if: always()` にする」の 2 つだけで、`failure() &&` を外した写しを試さない。premises.md にもこの振る舞いの行が無い
- kind: technical
- 提案: Scenario の THEN を「`if:` が状態の関数（`failure()`）と、その段の `outcome == 'failure'` の両方を持つ」にし、tasks 2.2 に `failure() && ` を外した写しの `expect_fail` を足す。premises.md に暗の `success()` の行（出所: 公式文書）を足す
- 処置: fixed 2.2 — Scenario「知らせの段は、材料を issue にする段が落ちたときだけ走る」の THEN を `failure()` と `outcome == 'failure'` の両方にし、2.2 に `failure() && ` を外した写しの expect_fail を足した。premises.md に暗の `success()` の行（公式文書の逐語）を足した

## R3. 「開いていれば追記する」の Scenario は、ラベルで絞って探す実装でも通る（偽の gh が `--label` を無視するので、この change が直す型の取りこぼしが残る）
- 成果物: openspec/changes/ch23-vulns-nightly-issue-fix/specs/dependency-vulnerability-scan/spec.md
- 根拠: spec.md:20-22。偽の `gh` の `issue list` は引数の `--label` を見ずに同じ一覧を返す（tools/check-vulns.sh:387-390）。今の `file_issues` は `--label 脆弱性` で絞って探す（tools/check-vulns-issues.py:138）。
  `report-unfiled` がこれを写して `--label 脆弱性` で探しても、自己試験は開いた issue を見つけて追記するので通る。本物の `gh` ではラベル無しの知らせの issue は見つからず、落ちた夜ごとに同じ題の issue を立て直す。
  「ラベルで絞らない」は design D2（design.md:25）にだけあり、Requirement の本文（spec.md:7「ラベルを問い合わせも作りもしない」）は `issue list` の絞り込みに触れない
- kind: technical
- 提案: Requirement の本文に「開いた issue はラベルで絞らずに題で探す」を足し、Scenario の THEN に「偽の `gh` が受けた `issue list` の引数に `--label` が無い」を足す（または偽の `gh` が `--label` を見て絞るようにする）
- 処置: fixed 2.1 — specs の本文に「ラベルで絞らずに題で探す」を足し、Scenario「知らせの issue はラベルに触れない」の THEN に「受けた `issue list` の引数に `--label` は無い」を置いた

## R4. 足した Requirement に、題と別の主張（gh の空の応答の読み方）が同居し、ラベルの 2 つの Scenario がその下に入っている
- 成果物: openspec/changes/ch23-vulns-nightly-issue-fix/specs/dependency-vulnerability-scan/spec.md
- 根拠: spec.md:3 の題は「毎晩の知らせの段が落ちた夜は、決まった題の 1 件の issue で知らせる」。spec.md:12 の 2 つ目の SHALL（`label list`・`issue list` の空の文字を空の一覧として読む）と、
  Scenario「ラベルが無いと空の文字で答える gh でも…」「ラベルがあれば、作り直さない」（spec.md:28-34）は `file-issues` の話で、題の振る舞いと関係しない。archive の後、正典で空の応答の約束を探す人は、知らせの段の Requirement の中に見つけることになる。
  正典の文は変えておらず、実質の MODIFIED にはなっていない（正典の Scenario「落ちた夜に…」を本物の gh で成り立たせる追加）
- kind: technical
- 提案: ADDED の Requirement を 2 つに分ける（例: 「gh の一覧の問い合わせが空の文字を返したら、空の一覧として読む」）。spec.md:12 と spec.md:28-34 をそちらへ移す。どちらも ADDED のままなので軽い路の約束は変わらない
- 処置: fixed 1.2 — ADDED を 2 つに分け、空の応答とラベルの Scenario を新しい Requirement「gh がラベルの一覧に空の文字で答えたら、ラベルが無いと読む」へ移した（1.2 がその 3 つの Scenario を試験にする）

## R5. 観測できる振る舞いが design / tasks にだけあり、Scenario も印も無い
- 成果物: openspec/changes/ch23-vulns-nightly-issue-fix/design.md / tasks.md / specs/dependency-vulnerability-scan/spec.md
- 根拠:
  (a) D1（design.md:17）「`issue view` が空なら今のとおり `gh の応答:` で落とす」と tasks 1.2（tasks.md:73）の rc=2 の確かめ。Scenario も `== Scenario:` の印も無いので、試験を書かなくても 1.2 の検証（tasks.md:74-75）は通る。
  (b) spec.md:12 は `issue list` の空の文字も空の一覧と言うが、`issue list` に空の文字で答える Scenario は無い（偽の gh は `[]` のまま。tasks.md:58）。`report-unfiled` の「空の文字は `[]`」（tasks.md:80）も同じ。
  (c) tasks 2.1（tasks.md:79-80）の「引数が 1 つでなければ `error: 検査できなかった（使い方）` で rc=2」「件数が上限に達したら rc=2」は spec に無い
- kind: technical
- 提案: (a) は R4 で分けた Requirement に Scenario（「1 件の中身の問い合わせが空の文字なら、rc=2 で `gh の応答:` を出す」）として足すか、D1 から振る舞いの文を外す。
  (b) は Scenario を足すか、spec.md:12 を実測した `label list` だけに絞る（premises.md:6 で `issue list` は `[]` と実測済み）。(c) は Scenario にするか、tasks から外す
- 処置: fixed D1 — (a) Scenario「開いた issue のコメントの問い合わせが空の文字なら、rc=2 で落ちる」を足した（1.2）。(b) 空の扱いを実測した `label list` だけに絞った（specs・D1・proposal）。(c) 使い方の rc=2 は Scenario「report-unfiled の引数が違えば、rc=2 で落ちる」にし（2.1）、件数の上限の rc=2 は tasks から外して D2 に理由を書いた

## R6. 「落ちた夜ごとに 1 回」「UTC の日付」を確かめる Scenario が無い
- 成果物: openspec/changes/ch23-vulns-nightly-issue-fix/specs/dependency-vulnerability-scan/spec.md
- 根拠: spec.md:8「その夜の行を追記する（落ちた夜ごとに 1 回）」。同じ日に手動の起動（`workflow_dispatch`。vulns-nightly.yml:5）で 2 回落ちたら 1 回か 2 回か、決まっていない。
  tasks 2.1（tasks.md:79-84）は同じ夜の重複を見ないので、実装は 2 回追記する。spec.md:9「`夜: <UTC の日付>`」に対し、Scenario は `夜: ` で始まること（spec.md:18・22）しか見ない
- kind: technical
- 提案: 「落ちた run ごとに 1 回」と書き直す（重複を許す）か、同じ日付の行が既にあれば追記しないと決めて Scenario を足す。Scenario の本文の行を「`夜: ` と `date -u +%F` の日付」にする
- 処置: fixed D2 — 「落ちた run ごとに 1 回（同じ日に 2 回落ちれば 2 回追記する）」と決めて specs の本文と D2 に書き、追記の Scenario を 2 回走らせる形に、本文の Scenario を `date -u +%F` と比べる形にした（2.1）

## R7. 1 つの Scenario に主張が束ねてある
- 成果物: openspec/changes/ch23-vulns-nightly-issue-fix/specs/dependency-vulnerability-scan/spec.md
- 根拠: spec.md:17-18 は rc=0・`issue create` が 1 回・`--label` が無い・`label list`/`label create` を受けない・本文が 2 行だけ、の 5 つ。
  spec.md:38-39 は `id` を持つ段・`if:` の条件・`report-unfiled` に URL を渡す・`GH_TOKEN`・自己試験の 2 つの写し、の 6 つ。どれか 1 つの確かめを書き落としても Scenario の印は付く
- kind: technical
- 提案: 少なくとも「ラベルに触れない」（spec.md:17 の後半と :18 の前半）と「本文は 2 行だけ」を分け、配線の Scenario は「段がある」と「条件が落ちたときだけ」（R2）を分ける
- 処置: fixed 2.1 — 立つ・ラベルに触れない・本文の 2 行を 3 つの Scenario に、配線を「段がある」「落ちたときだけ」「材料が無い夜」の 3 つに分けた（2.1・2.2）

## R8. 1.1 の「直す前に落ちた」の証跡は、機械の判定では何の FAIL でも通る（tools/** が本番のコードに数えられず、1.2 も名指しの項目になる）
- 成果物: openspec/changes/ch23-vulns-nightly-issue-fix/tasks.md
- 根拠: scripts/lanes.py:53 は `tooling` を本番のコードでないものに数え、`red_green`（scripts/lanes.py:399-404）は FAIL の木に本番のコードの差分が無ければ「直す前」と認める。
  この change が触るのは全部 `tools/**`・`.github/**` なので、`tools/check-vulns-issues.py` を直した後の FAIL でも「直す前に落ちた」になる。
  また `_items_naming`（scripts/lanes.py:368-382）は本文に Scenario の名前を含む項目を拾うが、1.2 の本文（tasks.md:71「`== Scenario: 落ちた夜に、依存ごとに 1 件の issue が立つ` の節の後に」）が名前を逐語で含むので、1.2 も名指しの項目になり、1.2 のどの FAIL も赤の証跡に数えられる。
  手順そのものは正しい（上の再演で、(a) だけの木で `gh の応答: Expecting value` で落ち、最小の直しで通った）
- kind: technical
- 提案: 1.2 の本文から名前の逐語を外す（「1.1 の節の後に」）。1.1 (b) に「FAIL の記録の `tree` で `tools/check-vulns-issues.py` が base と同じこと（`git diff --quiet <base> <tree> -- tools/check-vulns-issues.py`）を、code review が証跡で確かめる」と書き、機械が見ないことを人の席に渡す
- 処置: fixed 1.1 — 1.2 の本文から名前の逐語を外した（「1.1 の節の後に」）。1.1 に、FAIL の記録の tree で `tools/check-vulns-issues.py` が base と同じことを code review の席が確かめる、と書いた

## R9. 「壊してはいけないもの」の job の赤の付き方と、2.2 の「ほかの段は変えない」を確かめる検証が無い
- 成果物: openspec/changes/ch23-vulns-nightly-issue-fix/tasks.md
- 根拠: docs/changes/CH23.md:77「勧告が残る夜に `vulns`・`pg-minor` の job が赤になる付き方（design D4）」と tasks 2.2（tasks.md:87「ほかの job・段・権限・`timeout-minutes` は変えない」）。
  赤は検査の段の `exit "$rc"`（vulns-nightly.yml:23・42）で付くが、配線の検査はこれを見ない。`vulns-nightly.yml` は予定のパスなので 3.2 も検査の段の書き換えを落とさない
- kind: technical
- 提案: 2.2 に検証を足す。例: base の `vulns-nightly.yml` と比べた差分が `issues` の job の中だけであること、または配線の検査に `vulns`・`pg-minor` の段が `exit "$rc"` で終わることの確かめ
- 処置: fixed 2.2 — base の `vulns-nightly.yml` と `issues` の job より前が同じで、`exit "$rc"` が 2 つあることの検証を足した（今の木で rc=0 を確かめた）

## R10. 依頼の「赤に気づける形にする」を、検査できなかった夜（rc=2）を外して絞ったのは軽い路の制約で、本人の決定ではない
- 成果物: openspec/changes/ch23-vulns-nightly-issue-fix/design.md
- 根拠: docs/changes/CH23.md:28「どこまで: …赤に気づける形にすること」。design.md:32-37（D3）は rc=2 の夜を正典のまま「Actions の赤だけ」に残す。理由は「`MODIFIED` で、軽い路の外」（design.md:35）。
  本人に見える場所は docs/changes/CH23.md:90 の「残るもの」だけで、確認バッチの問い（完了の判定 5・tasks 4.1）には入っていない。osv.dev に何晩も届かない間は、この change の後も 9 晩の赤と同じく誰にも届かない
- kind: defer
- 提案: 取り返しのつかない決定ではないので、このまま仮で進めてよい。確認バッチの問いに「検査できなかった夜も issue で知りたいか（知りたければ正典を変える change を通常の路で立てる）」を足し、本人に選ばせる
- 処置: fixed 4.1 — 確認バッチの問い (3)「検査できなかった夜も issue で知りたいか」を足し、docs/changes/CH23.md の完了の判定 5 にも書いた

## R11. 「赤が気づかれなかった理由を調べる」が確かさ 1 のまま終わり、確認バッチの問いもその理由を聞いていない
- 成果物: openspec/changes/ch23-vulns-nightly-issue-fix/premises.md / tasks.md
- 根拠: 依頼の「何を」（docs/changes/CH23.md:26「赤が 9 晩気づかれなかった理由（知らせがどこに届くか）を調べ」）と「そのとき考えたこと」（同:32「調べてから決める」）。
  premises.md:8 は公式文書の要約経由で「逐語の本文はまだ確かめていない」（確かさ 1）。公式文書の本文を取り直せば確かめられる。
  tasks 4.1（tasks.md:103）が聞くのは #334〜#337 の issue の知らせが届いたかだけで、11-10〜11-18 の予定した workflow の失敗の知らせ（メール）が届いていたか・埋もれたかは聞いていない。D2 の「issue で知らせる」はそれが分からないまま選んである（design.md:9-11）
- kind: technical
- 再分類: spec-review の kind は premise。上流が公式文書の原稿で前提を確かめ、technical に分け直した（処置を見よ）
- 提案: premises.md:8 を公式文書の逐語で確かめ直す（機械でできる）。tasks 4.1 に「11-10〜11-18 の失敗の知らせのメールは届いていたか（届いていたなら、なぜ気づかなかったか）」を足す
- 処置: fixed premises.md — github/docs の原稿 `content/actions/reference/workflows-and-actions/events-that-trigger-workflows.md:1052` を `gh api` で読み、逐語を確かめた（確かさ 2）。本人の受け取り（届いたか・なぜ気づかなかったか）は機械から見えないので、4.1 の問い (2) にした。kind は technical に分け直した（前提は確かめ終え、残るのは失う側の無い確認バッチの問い）

## 観点 2: Scenario が検証可能か
R2・R3・R5・R6・R7。ほかの Scenario（spec.md:24-26・32-34）は観測できる言葉で書かれている。

## 観点 3: 置き場の誤り
R4・R5（a）。design の D1 の `issue view` と D2 の「ラベルで絞らない」（R3）が観測できる振る舞いで、spec に無い。
specs の `tools/check-vulns.sh report-unfiled`・`GH_TOKEN`・`id`・`outcome` は実装の名前だが、正典も同じ形（`tools/check-vulns.sh file-issues`・`if: always()`）で書いているので挙げない。
proposal の Capabilities（`dependency-vulnerability-scan`、ADDED のみ）と specs のディレクトリは一致する。change なので INDEX の capability 表は当てない。

## 観点 4: tasks が検証を持つか
R8・R9。1.1〜3.2 はどれも検証コマンドと rc を持ち、書いたまま抽出・実行できる（3.2 は今の木で rc=0、1.1 は再演で FAIL → PASS）。
依存順は 1.1 → 1.2 → 2.1 → 2.2 → 3.x で、`report-unfiled`（2.1）が配線の検査（2.2）より前にある。人間の確認待ち 4.1 は `物理: realdata` の印を持つ。
1.2・2.1 の 1 つ目の検証（`grep -qF "== Scenario: …"`）は印の有無だけを見るが、同じ項目の 2 つ目の検証（自己試験の rc=0）が判定を担う。

## 観点 5: change の完了の判定・壊してはいけないもの・依頼の範囲
`check_chain.py` は OK。docs/changes/CH23.md の「完了の判定」は 5 つ書かれ、design・specs と合っている。
「壊してはいけないもの」のうち、正典の毎晩の約束・`issues: write` の 1 job は正典の既存の Scenario が固定する。rc=2 の夜は R1、job の赤の付き方は R9。
依頼の範囲を越える specs は無い。欠いているものは R10・R11。

## 観点 6: 答えが写ったか
該当なし（`spec --answers` の呼び出しではない。軽い路で深掘りを省いている）。

## 観点 7: 走っている change との前提関係
観点 7: 該当なし（spec_overlap.py candidates が 0 件）。
補足: proposal.md:105 は走っている change として `ch19-emulator-only-class` だけを挙げるが、origin/main には `ch22-soak-install-online-ci`（`distribution`）も走っている。
`git show origin/main:openspec/changes/ch22-soak-install-online-ci/…` には `check-vulns`・`vulns-nightly` を触る記述が無く（review/spec.md:201 で名前に触れるだけ）、重ならない。

## 2 巡目

1 巡目の R と `- 処置:` の行には触れていない。直した後の spec・design・tasks・premises・proposal・docs/changes/CH23.md を読み直した。

### 機械の検査（2 巡目）

- `openspec validate ch23-vulns-nightly-issue-fix --strict` → valid（rc=0）
- `python3 scripts/check_chain.py .` → `chain: OK (0 件 / 未回収 0 件 / warn 0 件)`
- `python3 scripts/check_scenarios.py . ch23-vulns-nightly-issue-fix` → `scenarios: FAIL (担保なし 15 件 …)`。15 件はどれもこの change の新しい Scenario で、spec 段では予期どおり（印は tasks 1.2・2.1・2.2・2.3 で入る）
- `python3 scripts/review_triage.py . ch23-vulns-nightly-issue-fix` → `指摘 11 件 / 仮決め 0 件 / 要件へ戻すもの 0 件`・`triage: OK`
- `python3 scripts/spec_overlap.py candidates . ch23-vulns-nightly-issue-fix` → `[]`
- `python3 scripts/evidence.py lint openspec/changes/ch23-vulns-nightly-issue-fix` → `[OK] 検証コマンド`
- tasks 2.2 の 3 つ目の検証（`issues` の job より前が base と同じで、`exit "$rc"` が 2 つある）を今の木で書いたまま走らせた → rc=0
- 名指しの Scenario の名前を本文に含む tasks の項目は 1.1 だけになった（tasks.md:5 は前置きなので項目に数えない）

### (a) R11 を premise から technical に分け直したこと

妥当と判断する。R11 が premise だったのは、公式文書の逐語を確かめていなかったから（1 巡目の根拠）。これは github/docs の原稿の逐語（premises.md:66、確かさ 2）で、機械で確かめ終えた。
残るのは本人だけが答えられる受け取りの事実（届いたか・なぜ気づかなかったか）で、tasks 4.1 (2) に問いとして置かれている。どちらに転んでも失うものは無い。escalated にする必要は無い。
同じ型（機械で確かめられる前提を確かさ 1 のまま置いている）が D5 に残っているので、R12 として別に挙げる。

### (b) 1 巡目の処置の確かめ

- R1: ADDRESSED — spec.md:29-30 が「材料が 1 つも無い夜」「片方だけ検査できなかった夜」を名指すようになった。Scenario「材料が無い夜は、file-issues を呼ばない」（spec.md:69-72）が分岐を配線の検査で固定し、分岐を外した写しの `expect_fail` もある（tasks 2.2 の tasks.md:44・46）
- R2: ADDRESSED — spec.md:66 が `failure()` と `outcome == 'failure'` の両方を求め、spec.md:67 と tasks.md:46 に `failure() && ` を外した写しの `expect_fail` がある。premises.md:67 に暗の `success()` の逐語の行がある
- R3: ADDRESSED — spec.md:26「ラベルで絞らずに題で探す」と、spec.md:41「受けた `issue list` の引数に `--label` は無い」。ラベルで絞る実装はこの Scenario で落ちる
- R4: ADDRESSED — ADDED の Requirement が 3 つになり、空の応答とラベルの Scenario は「gh がラベルの一覧に空の文字で答えたら、ラベルが無いと読む」（spec.md:3-20）の下に移った
- R5: ADDRESSED — (a) spec.md:7 と Scenario spec.md:18-20、(b) 空の扱いを `label list` だけに絞った（spec.md:5・design.md:15-18）、(c) 使い方は Scenario spec.md:55-57 に、件数の上限は理由つきで外した（design.md:26）
- R6: ADDRESSED — spec.md:27「落ちた run ごとに 1 回。同じ日に 2 回落ちれば 2 回追記する」、spec.md:45 が `date -u +%F` と比べ、spec.md:48-49 が 2 回走らせて 2 回の `issue comment` を見る
- R7: ADDRESSED — 立つ / ラベルに触れない / 本文の 2 行（spec.md:35-45）と、配線の 3 つ（spec.md:59-72）に分かれた。配線の Scenario に残る `AND`（写しを書き換えると落ちる）は、同じ主張を確かめる手段なので束ねとはみなさない
- R8: ADDRESSED — 1.2 の本文から名前の逐語が消え（tasks.md:27「1.1 の節の後に」）、1.1 に、FAIL の記録の tree を code review の席が確かめることが書かれた（tasks.md:23-24）。機械の判定が空振りすることは変わらず、確かめは人の席に移っている
- R9: ADDRESSED — tasks.md:49 の検証。今の木で rc=0
- R10: ADDRESSED — tasks 4.1 (3)（tasks.md:70）と docs/changes/CH23.md の完了の判定 5

### (c) D5 と担当者の Requirement

正典との矛盾は見つからなかった。正典が固定するのは題・ラベル・本文・重複・閉じないこと（openspec/specs/dependency-vulnerability-scan/spec.md:211-254）で、担当者には触れない。
既存の自己試験は `GITHUB_REPOSITORY_OWNER` を外して走らせる（tasks.md:53）。`--assignee` は引数の末尾に置く（tasks.md:51）ので、`^issue create --title …` の見方（tools/check-vulns.sh の PostgreSQL の節）も変わらない。
人に返すべき A も無い。D5 とリポジトリ全体を watch してもらう案（design.md:49）は、どちらも戻せて安い。
Scenario（spec.md:81-91）はどれも、偽の `gh` の記録で真偽が決まる。

## R12. D5 の前提「担当者にされた人には知らせが届く」が確かさ 1 のままだが、公式文書の原稿で機械が確かめられる
- 成果物: openspec/changes/ch23-vulns-nightly-issue-fix/premises.md
- 根拠: premises.md:69 は「公式文書では確かめていない」・確かさ 1。github/docs の原稿を `gh api repos/github/docs/contents/<path> -H 'Accept: application/vnd.github.raw'` で読むと、次の逐語がある。
  - `content/subscriptions-and-notifications/concepts/about-notifications.md:39-41`「you are automatically subscribed to conversations by default when you have: … Been assigned to an issue or pull request.」
  - `content/rest/activity/notifications.md:55`「`assign` | You were assigned to the issue.」

  あわせて次も確かめた。
  - `gh api users/rai-256 --jq .type` → `User`
  - `gh api -i repos/rai-256/ashiato/assignees/rai-256` → `HTTP/2.0 204 No Content`（担当者にできる）

  design.md:58 の Risk（持ち主が組織なら付けられない）は今は当たらない。`GITHUB_REPOSITORY_OWNER` を Actions が既定で渡すこと（tasks.md:52）にも、premises の行が無い
- kind: technical
- 提案: premises.md:69 に上の逐語を写して確かさを上げ、担当者にできることの実測の行と、`GITHUB_REPOSITORY_OWNER` の既定の環境変数の行（出所: 公式文書）を足す。実際にメールや受信箱に届くか（知らせの設定）は本人にしか分からないので、4.1 (1) に残す
- 処置: fixed premises.md — 担当者の購読と知らせ（公式文書・確かさ 2）・本人を担当者にできること（実物・確かさ 4）・`GITHUB_REPOSITORY_OWNER`（公式文書 variables.md:59・確かさ 2）の 3 行にした。design の「組織なら付けられない」の Risk を、立て直しの後に残るものへ書き直した

## R13. 「担当者を付けられなければ知らせの段が拾う」（D5）は成り立たない。知らせの段も同じ `--assignee` で落ちる
- 成果物: openspec/changes/ch23-vulns-nightly-issue-fix/design.md / specs/dependency-vulnerability-scan/spec.md
- 根拠: design.md:51「担当者を付けられない（`gh` が落ちる）ときは、issue を立てられなかったのと同じに rc=2 で落とす（知らせの段が拾う）」。spec.md:76 と tasks.md:51 は `report-unfiled` にも同じ `--assignee` を付ける。
  担当者で落ちる夜は、`file-issues` も `report-unfiled` も同じ所で落ちる。D2 の原則（design.md:28「同じ所で落ちる経路は知らせにならない」）に反し、赤だけが残る。
  加えて、これは正典の `file-issues` の経路に新しい落ち方を足す。担当者 1 つで、その夜の依存の issue が全部立たない。本物の `gh` が付けられない login でどう落ちるかは premises に無く、偽の `gh` も表さない（この change が直す型の取りこぼし）。
  今は付けられる（R12 の 204）ので、起きる見込みは低い
- kind: technical
- 提案: 少なくとも `report-unfiled` は、担当者つきの `issue create` が落ちたら担当者無しで 1 回立て直すことにし、その Scenario（「担当者を付けられなくても、知らせの issue は担当者無しで立つ」）を足す。`file-issues` も同じにするか、design.md:51 の「知らせの段が拾う」を消して、残るものに書き直す
- 処置: fixed D5 — 担当者つきの `issue create` が落ちたら、担当者を外して 1 回だけ立て直すと決め、specs の Requirement の本文と Scenario「担当者を付けられなければ、担当者無しで立て直す」を足した（2.3 が試験にする）

## R14. 確認バッチの問い 4.1 (1) は、merge の後に観測できる場面が来ないことがある
- 成果物: openspec/changes/ch23-vulns-nightly-issue-fix/tasks.md
- 根拠: tasks.md:67「merge の後の最初の毎晩の検査…で、本人を担当者にして立った issue の知らせが本人に届いたか」。今の当たりは #334〜#337 が開いたままで、`file-issues` は追記するだけで担当者を付けない（spec.md:78、design.md:50）。
  そのため新しく担当者つきの issue が立つのは、新しい依存に勧告が出た夜か、知らせの段が落ちた夜だけになる。どちらも起きないまま確認バッチが来ると、「届いたか」に答える材料が無い。
  あるいは「届かなかった」と取り違え、D5 の反転条件（design.md:52）を誤って引く
- kind: technical
- 提案: 4.1 (1) に観測の手順を足す。例えば、docs/changes/CH23.md の「残るもの」にある「閉じて立て直させる」を手順にし、merge の後に #334〜#337 のどれか 1 つを本人が閉じて `workflow_dispatch` で起動し、担当者つきで立て直された issue の知らせが届いたかを聞く。新しい issue が立たなかったときは「観測できなかった」と答えられるようにする
- 処置: fixed 4.1 — 担当者つきの issue が立たないときに、本人が 1 つ閉じて手動で起動する手順と、「観測できなかった」と答える道を足した

## R15. tasks の読む順が D5 を指していない
- 成果物: openspec/changes/ch23-vulns-nightly-issue-fix/tasks.md
- 根拠: tasks.md:3「`design.md`（D1〜D4）」。2.3（tasks.md:51）は D5 を名指す
- kind: technical
- 提案: 「D1〜D5」に直す
- 処置: fixed tasks.md — 読む順を D1〜D5 にした

