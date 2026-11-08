# CH12 spec-review（成果物どうしの整合）

対象: `openspec/changes/ch12-memory-decisions-adr/`（proposal・specs/decision-records・design・tasks）、`docs/changes/CH12.md`、`docs/briefs/CH12-pr.md`
前提として読んだもの: `deep.md`（第 1 回・推奨で先に書いた `spec --draft`）・`deep-questions.json`・`premises.md`・`review/deep.md`
確かめた日: 2026-11-08（worktree `ashiato2-up-ch12`、HEAD 4df534c8a、`origin/main` = 0adef6712）

## 機械の検査

| コマンド | 結果 |
|---|---|
| `openspec validate ch12-memory-decisions-adr --strict` | valid（rc=0） |
| `python3 scripts/check_chain.py .` | chain: OK（rc=0） |
| `python3 scripts/check_scenarios.py . ch12-memory-decisions-adr` | FAIL: 担保なし 11 件（この change の 11 本すべて）。印を出す `tools/test-adr-docs.sh` がまだ無いので、この段では想定どおり。tasks 1.1・3.1 に書かれた `echo "Scenario: …"` の 11 本は、spec の `#### Scenario:` と一字一句一致することを目で確かめた（引数なし 7・`--self-test` 2・`--backlog` 2） |
| `python3 scripts/review_triage.py . ch12-memory-decisions-adr` | **FAIL 8 件**（`review/deep.md` の処置の形。R1） |
| `python3 scripts/spec_overlap.py candidates . ch12-memory-decisions-adr` | `[]`（0 件） |
| `python3 .harness2/scripts/provisional.py check openspec/changes/ch12-memory-decisions-adr` | rc=0（台帳は 0 行） |

## R1. review/deep.md の処置が機械の受ける形でなく、triage が 8 件 FAIL する（R5 は fixed で閉じられない）
- 成果物: openspec/changes/ch12-memory-decisions-adr/review/deep.md
- 根拠: `review_triage.py` の出力。R1〜R6 の `処置: fixed: …` が指す先が `Q2`・`premises`・`5`・`Q1`・`C1` で、`D<n>`・`<x.y>`・change 直下のファイル名のどれでもない（review/deep.md:29,36,48,55,63,69）。R5 は `loss: exported`（review/deep.md:60）なので `fixed` で閉じられない（escalated にするか、失わない側を選んだ仮なら `provisional.py add`）
- kind: technical
- 提案: 中身は変えずに形だけ直す。R1・R2・R3・R6 は写った先（`premises.md`・`deep-questions.json`・design の D1/D4）を名指す。R5 は失わない側（出力を 4 つの名前の有無に絞る）を採ったので、design D5 に（仮）と反転条件を付けて `provisional.py add … --source review/deep.md#R5` で台帳に載せるか、escalated にする。R3・R4 は R2 の件が片付くまで fixed にしない
- 処置: fixed D5 仮 —— review/deep.md の R1〜R6 の処置を、写った先（D1・D3（仮）・D4（仮）・D5（仮）・premises.md・deep-questions.json）を名指す形に直した。R5（loss: exported）は失わない側を選んだ仮として design D5（仮）にし、`provisional.py add … --source review/deep.md#R5` で台帳に載せた

## R2. review/deep.md の R3・R4 の「fixed」は deep-questions.json に写っておらず、人間が答える Q1 の context は誤った事実を含んだまま
- 成果物: openspec/changes/ch12-memory-decisions-adr/deep-questions.json（Q1）・review/deep.md
- 根拠: `git log -- deep-questions.json review/deep.md` → どちらも 05c007015 の 1 commit だけで、処置の後に deep-questions.json は変わっていない。deep-questions.json:12 の Q1 の context は「3 つ」だけを挙げ、「ほかの事実（ポートの規則・pedantic の表・detekt の baseline・分けた crate がまだ無いこと）は repo と一致した」と言う。R3（review/deep.md:37-48）はポートの規則（`installer/src/commands.rs:107-117` の写し・`ports.sh` の 55432 への落ち）と pedantic（`winsvc` に `[lints]` が無い）が食い違うと実測し、「ほかは一致」の文を消したと処置に書く。R4（review/deep.md:55）は「Q1 を置き換えた」と書くが、Q1 は置き換え前の問い（逐語のまま置くか追記するか）のまま。`docs/briefs/CH12-pr.md:50` はこれを PR 本文で挙げるが、答えのフォーム（deep-questions.json）を読む人間には届かない
- kind: technical
- 提案: 本人が答える前（`draft.json` の `answers_before: 0`）に、deep の段で Q1 の context を R3 の実測に合わせて直す（少なくとも「一致した」の文を消し、8 つにする）。spec の段が問いを書き換えないのは妥当だが、PR 本文で挙げるだけでは答えの入口が誤ったまま残るので、呼び出し元が deep へ差し戻すか、答えを送る前に人間へ明示する
- 処置: fixed deep-questions.json —— Q1 の context に 8 つを書き、「repo と一致した」の文を消した。問い文と推奨の選択肢も 8 つ・CH12.md の節に合わせ、deep.md の Q1 の行も同じに直した（答えはまだ 0 件で、問いの向きと推奨の意味は変えていない）。kind は premise から technical に読み替えた: 決定の前提が崩れたのではなく、前の段の処置が問いに写っていなかった不備で、人間に新しく問うものは無い

## R3. R4 の実質（食い違いの一覧を、後で誰がどう見つけるか）がどの D にも無い
- 成果物: openspec/changes/ch12-memory-decisions-adr/design.md（D3）
- 根拠: review/deep.md:51 は「PR 本文は流れていく」「判断の正本は ADR なので、古い『影響』（接続は未決など）を読んだ agent が今の状態を取り違えうる」「手入れのきっかけ（issue・change・台帳）がどこにも無い」と指摘し、処置は「推奨: docs/changes/CH12.md の節」と書く。design.md:51-56 の D3 は置き場を `docs/briefs/CH12-pr.md` の 1 か所にし、CH12.md には一覧の節が無い（CH12.md:283-285 は「PR 本文に挙げる」とだけ書く）。premises.md:20 にも 8 つはあるが、archive の後に ADR から辿る経路は無い
- kind: defer
- 提案: R4 が求めた B の問い（推奨つき。例: `docs/changes/CH12.md` に節を置く／`hx change` で手入れの change を起こす／残さない）を R2 の差し戻しで Q1 に当てる。問わずに進めるなら、D3 に「PR 本文のほかに CH12.md の『依頼（逐語）』以外の節にも 8 つを残す」を足し、後で見つける経路を書く
- 処置: fixed D3 仮 —— D3 に、同じ一覧を `docs/changes/CH12.md` の `## ADR と repo の今の食い違い（直していない）` にも置く（archive の後も残る・手入れの change はそこから拾う）を書き、CH12.md に節を足した。Q1 の推奨の文面にも入れた

## R4. `--backlog` は rc を見ず、4 つのメモリが数える対象に無くても「残りに無い」で通る（空の合格）
- 成果物: openspec/changes/ch12-memory-decisions-adr/design.md（D4）・specs/decision-records/spec.md（「メモリの残りを数える台本は、CH12 の 4 つのメモリを残りに数えない」）・tasks.md 3.1
- 根拠: design.md:69「rc は見ない。JSON の各行のファイル名に 4 つの名前のどれかがあれば落とす」。`/home/yosis/dev/harness2-adr/scripts/adr_backlog.py` は `type: project|feedback` で `adr: no` の無いメモリだけを数え（`decisions()`）、`--json` は JSON の配列 1 つを出す。台本が落ちる（argparse の不一致・読み込みの例外）・出力の形が違う・4 つのメモリが置き場から消えるか `adr: no` が付く・型が変わる、のどれでも「4 つの名前が出力に無い」が成り立ち、rc=0 になる。正の対照は今は取れる: `--adr` に空のフォルダを渡すと 23 件が残り、4 つとも含まれ rc=1（実測）
- kind: technical
- 提案: D4 と tasks 3.1 に 3 つを足す。(a) 正の対照: 同じ置き場で `--adr <空の mktemp -d>` を走らせ、4 つの名前がすべて残りに出ることを先に確かめる（出なければ通さない）、(b) `adr_backlog.py` の rc が 0 か 1 で、出力が JSON の配列として読めること（でなければ通さない）、(c) Scenario の THEN にも「数える対象に 4 つが在ることを確かめた上で」を入れる
- 処置: fixed D4 仮 —— 判定を 3 つにした（空の `--adr` での正の対照・本番の rc が 0 か 1 で JSON の配列・本番に 4 つの名前が無い）。tasks 3.1 に、正の対照で落ちる偽の台本の自己検査を足した。`--backlog` は正典から外した（R10）

## R5. 「メモリの説明は出ない」の確かめが、`description` の語の有無で、主張を測っていない
- 成果物: openspec/changes/ch12-memory-decisions-adr/tasks.md 3.1・specs/decision-records/spec.md（「残りを数えた結果は、4 つのメモリ名の有無だけを出す」）
- 根拠: tasks.md:50「`.md` で終わる語が 4 つの名前のほかに無いこと・`description` の語が無いことで見る」。漏れるのは description の値（自由な文）で、`description` の語は JSON をそのまま流したときにしか出ない。整形して流す誤り（`- <名前>: <説明>`）は `.md` の検査で掛かるが、名前を落として説明だけを出す誤りは掛からない。また「台本の中で `--backlog` の自分の出力を受けて」は、出力し終える前の自分を検査する形で、外から何を走らせると真偽が決まるかが書かれていない。design D5（design.md:77）は stdout を受けて捨てるとだけ書き、`adr_backlog.py` の stderr の扱いが無い
- kind: technical
- 提案: 確かめを許可の一覧にする: `--self-test` が `--backlog` を子として走らせ、stdout と stderr の全行が `Scenario: …`・`<4 つの名前のどれか>: 残りに(無い|ある)`・結果の 1 行・`error: …` のどれかの形に当たることを見る（メモリが無い環境では子が rc=127 で、許可の形だけを確かめる）。D5 に「`adr_backlog.py` の stderr も受けて捨てる」を足す
- 処置: fixed D5 仮 —— D5 に標準エラーも受けて捨てること、出してよい行の形（4 つ）を書き、`--self-test` が `--backlog` を子として走らせて全行が形に当たることを見る形にした（tasks 3.1 (d)）

## R6. `ADR_BACKLOG`・`ADR_MEMORY` が「在るが指す先が無い」とき、次の候補へ落ちるかが決まっておらず、自己検査が手元で rc=127 にならない向きに書けてしまう
- 成果物: openspec/changes/ch12-memory-decisions-adr/design.md（D4）・tasks.md 3.1
- 根拠: design.md:62「環境変数 `ADR_BACKLOG`（台本のパス）があればそれ」。「あれば」が変数の有無か、ファイルの有無かが決まっていない。後者で書くと、`ADR_BACKLOG=/nonexistent` の自己検査（tasks.md:49）は 3. の探し方で `/home/yosis/dev/harness2-adr/scripts/adr_backlog.py` を拾い（`git -C /home/yosis/dev/harness2 worktree list` の 2 つめ）、rc=127 にならない。逆に台本が見つからない環境では、`ADR_MEMORY=/nonexistent` の自己検査も「台本が無い」で rc=127 になり、置き場の欠落の枝を通らずに緑になる
- kind: technical
- 提案: D4 に「環境変数が設定されていれば、それだけを使い、指す先が無ければ次へ落ちずに rc=127」と書く。`ADR_MEMORY` の自己検査は `ADR_BACKLOG` を実在する偽の台本（使い捨ての木に置く）へ向けて走らせ、置き場の枝を通ることを `error:` の文面で見る
- 処置: fixed D4 仮 —— 環境変数が設定されていればそれだけを使い、指す先が無ければ次へ落ちずに rc=127 と書いた。置き場の欠落の自己検査は、`ADR_BACKLOG` を使い捨ての木の偽の台本に向けて走らせ、`error:` の文面で枝を見る（tasks 3.1 (a)(b)）

## R7. 自己検査の壊し方 4 つは、日付・題の不一致と並びの誤りを壊さない（Scenario が AND で 2 つを束ねている）
- 成果物: openspec/changes/ch12-memory-decisions-adr/specs/decision-records/spec.md（「一覧の日付と題は ADR の日付と見出しに一致し、新しい順に並ぶ」ほか）・design.md D9・tasks.md 1.1
- 根拠: design.md:104 の壊し方は「一覧の 1 項目を消す・出所の行を消す・名前を大文字にする・表の行を消す」の 4 つ。日付と題の照合も、降順の照合も、どれにも掛からないので、照合を書かない実装でも 2.2 の `--self-test` rc=0 が通る。spec.md:21-23 の Scenario は「日付と題が一致」と「降順」の 2 つの主張を 1 本に束ね、spec.md:55-57（無い AND 4 行 AND rc=0）・spec.md:63-65（rc=127 AND `error:` AND 判定の行が無い）・spec.md:73-75（rc=0 AND allow に行が無い）も同じ形
- kind: technical
- 提案: 自己検査に「一覧の 2 項目を入れ替える」「ADR の 1 件の見出しを 1 字変える」を足す。「日付と題」と「並び」は Scenario を分ける（印も分ける）
- 処置: fixed 1.1 —— 「日付と題」と「並び」の Scenario を分け、自己検査の壊し方に「一覧の 2 項目を入れ替える」「見出しを 1 字変える」を足した（design D9）。ほかの AND の Scenario は、1 つの性質を確かめる手順の条件（rc と出力）を並べたもので、分けると印が意味を持たなくなるので残した。「CLAUDE.md の表」は「条件を全部満たす行がちょうど 1 つ」に言い直した

## R8. 「網の名前の検査が通る」は、docs/adr/ が未追跡のうちに走ると docs/adr/ を見ずに通る
- 成果物: openspec/changes/ch12-memory-decisions-adr/specs/decision-records/spec.md（「docs/adr/ を含む追跡ファイルで、網の名前の検査が通る」）・tasks.md 2.2
- 根拠: `tools/check-private.sh:86` は `git ls-files -z` で対象を列挙する。下流が `git add` する前に verify-run が `tools/test-adr-docs.sh` / `tools/check-private.sh` を走らせると、docs/adr/ の 6 ファイルは対象に入らず rc=0
- kind: technical
- 提案: 台本は check-private を呼ぶ前に `git ls-files docs/adr` が README と 5 件の ADR を返すことを確かめ、返さなければ `error:` で落とす。Scenario の THEN にもそれを入れる
- 処置: fixed 1.1 —— 台本は check-private の前に `git ls-files docs/adr` が 6 ファイルを返すことを見る（design D9）。spec の Requirement と Scenario の THEN にも入れ、tasks 2.1 に `git add docs/adr` を書いた

## R9. 「依頼（逐語）の節を 1 字も変えない」（壊してはいけないもの）を確かめるものが無く、両方を同じに書き換えれば `--verbatim` が通る
- 成果物: docs/changes/CH12.md（壊してはいけないもの）・design.md D1・tasks.md 2.1・4.1
- 根拠: CH12.md:270「この文書の『依頼（逐語）』の節: 逐語との比較の元なので、1 字も変えない」。`--verbatim`（design.md:35-37）は docs/adr/ と CH12.md の節を比べるだけ。4.1 の差分の検査（tasks.md:57）は `docs/changes/CH12.md` の変更を許す。今の節は登録の commit ff9a78dd7 の節と一致する（`awk` で両方の節を取り出して diff → 同じ、242 行）が、それを固定する検証は無い。下流が食い違いを「直す」とき、ADR と節を一緒に書き換えれば、どの検証も落ちない。ADR の逐語の一致は D1 のとおり意図して Scenario にしていないので、ここが唯一の歯止めになる
- kind: technical
- 提案: `--verbatim` が、節を `git show ff9a78dd7:docs/changes/CH12.md` の同じ節と比べ、違えば落ちるようにする（D1 に書く）。比べる元の commit は台本に定数で書く
- 処置: fixed D1 —— `--verbatim` は先に、いまの「依頼（逐語）」の節の sha256 が、登録の commit `ff9a78dd7` の節から計算した定数と一致することを見る（rebase で SHA が消えても動くよう、commit でなく中身の値を台本に書く。いまの節と登録時の節で同じ値を確かめた）。tasks 2.1・CH12.md の完了の判定 2. にも書いた

## R10. `--backlog` の 3 本は CH12 の受け入れの 1 回の判定で、D1 の基準では正典に置かないもの。正典が private で未 merge の外の台本を名指す
- 成果物: openspec/changes/ch12-memory-decisions-adr/specs/decision-records/spec.md（2 つ目の Requirement の後半と 3 本の Scenario）・design.md D1・D8
- 根拠: design.md:39-40 の D1 は「常に成り立つ性質ではない」ことを理由に逐語の一致を正典から外す。spec.md:41-43 と 55-65 は、手元にしか無いメモリ（design.md:11）と、harness2 の main に無く push もされていない `adr_backlog.py`（premises.md:9。harness2 は PRIVATE）に依る判定を SHALL にしている。archive の後、メモリが索引として書き換えられるか消されれば、この Scenario は空で成り立つか（R4）、台本が無ければ常に BLOCKED になる。`check_scenarios.py` は正典の Scenario に印を求め続けるので、`--backlog` の口を永く持つことにもなる。また Q3 の推奨（deep.md:18）は「一致はこの change の検証で 1 回だけ見る」と言い、D2 で CI に載せないので、1 つ目の Requirement（一覧の 1 対 1・並び）も archive の後に誰も確かめない SHALL として正典に残る（D8 の「正典が固定する」は CI が無い限り成り立たない）
- kind: technical
- 提案: `--backlog` の判定（R4 の正の対照を含む）は `--verbatim` と同じく tasks と完了の判定（CH12.md:279）だけに置き、Requirement からは「出所にメモリのファイル名を名指しする」までを残す。1 つ目の Requirement を正典に残すなら、Q3 の推奨の文面との違い（1 回だけ見るが、正典は常に成り立つと言う）を D8 に書く
- 処置: fixed D8 —— `--backlog` の 3 本を spec から外し、Requirement は「出所に名指しする」までにした（判定は D4・tasks 3.1・CH12.md の完了の判定 4. だけ）。D8 に、正典に置くもの・置かないものと、Q3（毎回は見ない）と正典（何が成り立つべきか）が食い違わない理由を書いた

## R11. premises の C2（台本に触らない・足すのは 6 ファイルと 1 行だけ）と D2（台本を 1 本足す）が食い違ったまま
- 成果物: openspec/changes/ch12-memory-decisions-adr/premises.md（C2）・design.md D2
- 根拠: premises.md:25「C2. 振る舞いを変えない（コード・台本・CI に触らない）。足すのは `docs/adr/` の 6 ファイルと `CLAUDE.md` の表の 1 行だけ」。design.md:43-47 の D2 は C2 を写すと言いつつ `tools/test-adr-docs.sh` を足す（理由は書いてある）。D2 は「仮・C2」で、Q3 の推奨（仮）と C2 の既定と、C2 からの逸脱（AI の既定）を 1 つの D に混ぜている。GO 資料で人間が読む C2 は「台本に触らない」のまま
- kind: technical
- 提案: premises の C2 を「足す台本は検証用の 1 本だけ（CI に載せない）」に直すか、台本を足すことを D2 から別の D（AI の既定。（仮）の印の要否を明示）に分ける
- 処置: fixed premises.md —— C2 の下に「★ 2026-11-08 spec の段で改めた: 検証用の台本を 1 本足す（CI に載せない）」を書き足し、design D2 から参照した

## R12. 4.1 の差分の検査は、`origin/main` が引けないと何も見ずに rc=0 で通る
- 成果物: openspec/changes/ch12-memory-decisions-adr/tasks.md 4.1
- 根拠: 実測 `bash -c '! git diff --name-only origin/nonexistent...HEAD | grep -vE "^(docs/adr/)"'` → `fatal: ambiguous argument` を出して rc=0。pipefail が無く、`!` は grep の rc だけを反転する
- kind: technical
- 提案: 先頭に `git rev-parse --verify origin/main >/dev/null &&` を置くか、`set -o pipefail` を付ける
- 処置: fixed 4.1 —— 差分を先に変数に取り、git が失敗すれば exit 1、空でないことと許す形の外が無いことを見る形にした（`origin/nonexist` で rc=1、いまの木で rc=0 を確かめた）

## 観点ごとの該当なし

- 観点 1: 推奨の写り（R2・R3 を除く）は該当なし。確かめた範囲: Q1 → D3（仮）、Q2 → D4（仮）、Q3 → D2（仮）、Q4 → D7（仮）、C1 → D1、C2 → D2（R11）、C3 → D6、C4 → D5。各 D の反転条件は Q の 2 つ目の選択肢と合う。決定の数値（6 ファイル・4 つの名前・rc=127・ファイル名の正規表現）は Scenario にある。当初案の残りは無い。requirements.md へ戻すものは無い（change）
- 観点 3（置き場）: design の D のうち観測できる振る舞い（D5 の出力の形、D4 の rc=127）は Scenario にも在る。proposal の Capabilities（`decision-records`）と specs/ のディレクトリは一致。change の capability を INDEX.md に足さない・CH12.md の `capability: []` は CH03・CH09・CH10 と同じ扱い。実装の名前は R10 で挙げたもののほか、`CLAUDE.md`・`docs/adr/`・`tools/check-private.sh` はこの capability の外から見える置き場そのものなので指摘しない
- 観点 4: 依存の順（台本 → ADR を置く → `--backlog` → 範囲の確かめ）は正しい。1.1 の検証が `bash -n` だけなのは、docs/adr/ がまだ無い段なので妥当（2.2 で `--self-test` を走らせる）。人間の確認待ちは無い（3.1 は手元の機械で走り、無ければ BLOCKED_INFRA）
- 観点 5（change）: CH12.md の「完了の判定」は 7 項目あり、deep の推奨と specs に合う（R9・R10 を除く）。依頼の範囲は越えていない（足すのは検証の台本だけで、R11 のとおり CH12.md の「壊してはいけないもの」には書かれている）。依頼の完了の判定（`--adr <harness2>/docs/adr` を渡す）を D4 が `--adr docs/adr` だけにしたのは、4 つのメモリが repo の ADR に名指されるので結論が変わらない
- 観点 6: 該当なし（`spec --draft`。`--answers` ではない）
- 観点 7: 該当なし（spec_overlap.py candidates が 0 件）

## 呼び出し元が特に見てほしいと言った点への答え

- `--backlog` が空の合格にならないか: メモリの置き場が無い・台本が無いときは rc=127 で、`.harness2/scripts/evidence.py:438-439` が BLOCKED_INFRA と読む（INFRA_SIGNS に当たらなくても rc=127 で BLOCKED_INFRA）。Story gate は BLOCKED_INFRA を PASS にしない（evidence.py:47）。ただし置き場と台本が在るときの空の合格は残る（R4）。自己検査の枝の取り違え（R6）
- 公開の経路: 手元のメモリ 42 本の名前のうち、この枝の追跡ファイル（change の全成果物・CH12.md・PR 本文）と repo 全体に出るのは、既に公開されている 4 つだけ（`git grep -F` で確かめた）。evidence の `summary`（末尾 20 行、commit される）は D5 が絞る。残る穴は stderr の扱いと確かめ方の弱さ（R5）。verify-run のログ本体は `.superpowers/sdd/`（自己 ignore）で commit されない
- D1（逐語の一致を正典にしない）: 妥当。README（CH12.md:51）が後の「置き換え済み」を認めるので常には成り立たない。ただし同じ基準を `--backlog` にも当てるべき（R10）、歯止めの比べる元を固定すべき（R9）
- D8（`skip_specs` を使わない）: 妥当。前例（`test-hygiene`・`test-timing`）があり、`skip_specs` がグラフを通るかは確かめられていない。ただし CI に載せない（D2）ので「正典が固定する」の言い分は弱い（R10）
- review/deep.md R4 の扱い: この段が問いを書き換えないのは妥当だが、PR 本文で挙げるだけでは足りない。答えの入口（deep-questions.json）の Q1 の context が R3 の実測と食い違ったまま人間に出る（R2）。R4 の実質の問い（後で見つける置き場）も D に無い（R3）。triage も R1〜R6 の処置の形で落ちる（R1）
- D4 のメモリの置き場（`git worktree list --porcelain` の 1 つめ）: 今は `/home/yosis/dev/ashiato2`（枝は `docs/archive-st53-redo` で main ではない）だが、置き場はパスで決まるので `-home-yosis-dev-ashiato2/memory` に当たり、動く
