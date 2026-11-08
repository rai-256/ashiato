<!-- 処置の判定: re-review -->

# CH12 code_verify — 独立の検証（2026-11-08、HEAD cb2bdb079・origin/main 0f38ad74e）

## 申告と実測

申告: tasks の `[x]` 5/5（1.1・2.1・2.2・3.1・4.1）。検証コマンドは下の 11 本。

| タスク | 検証コマンド | 実測 |
|---|---|---|
| 1.1 | `bash -n tools/check-adr.sh` | rc=0 一致 |
| 2.1 | `tools/check-adr.sh --verbatim` | rc=0 一致 |
| 2.2 | `tools/check-adr.sh` | rc=0 一致（Scenario の印 11 行・`OK 網の名前・私設 IP は入っていない（2264 ファイル）`） |
| 2.2 | `tools/check-adr.sh --self-test` | rc=0 一致（印 2 行） |
| 2.2 | `tools/check-private.sh` | rc=0 一致 |
| 2.2 | `CLAUDE.md` `AGENTS.md` `ci.yml` の numstat が `1\t0` | rc=0 一致（3 つとも `+` 1 行だけ） |
| 3.1 | `tools/check-adr.sh --backlog` | rc=0 一致（4 つとも `残りに無い`・`OK backlog`。台本は `.harness2/scripts/adr_backlog.py` で見つかった） |
| 3.1 | `tools/check-adr.sh --self-test` | rc=0 一致 |
| 4.1 | main からの差分の範囲 | rc=0 一致（`openspec/` 以外は `docs/adr/` 6・`CLAUDE.md`・`AGENTS.md`・`ci.yml`・`tools/check-adr.sh`・`.superpowers/sdd/` 2） |
| 4.1 | `python3 scripts/check_scenarios.py . ch12-memory-decisions-adr` | rc=0 一致（`scenarios: OK`。CH12 の 13 本の名前は `tools/check-adr.sh` にそれぞれ 1 回ずつ、grep で確認） |
| — | `python3 scripts/plan_fix.py pending openspec/changes/ch12-memory-decisions-adr` | `[]` |

`[x]` の検証は全部実在して rc=0 になった。指摘は、検証が**見ていない側**（ADR を足した後・検査そのものの壊れ・メモリが消えた後）に出た。

## 手 1: 固定値を独立に再計算する

`--verbatim` の定数 `7e1b0a5c…2e58b94` を、台本の awk でなく python の `hashlib` と、行を `split('\n')` で取る別の取り出し方で計算した。
いまの `docs/changes/CH12.md`・登録の commit `ff9a78dd7`（design D1 の名指し）・rebase 後の登録の commit `b77e52a51` の 3 つで、どれも 243 行・`7e1b0a5c755c8b19fcb7c138db6f2f87897d96baa46f90b3bc889ba872e58b94`。一致。
6 つの塊も、正規表現を使わず `=== ファイル:` の行で区切る別の実装で取り出し、`docs/adr/` の 6 ファイルとバイト単位で比べた: 6/6 一致。
`--verbatim` の側も壊すと落ちる（ADR の写しの 1 語を変えると rc=1・27 行目を出す。README の末尾に空行を 1 つ足すと rc=1）。
（後者は D1 の「末尾の改行を 1 つにそろえて比べる」より厳しい。厳しい側なので指摘にしない。）

## 手 2: ガードをわざと壊す

使い捨ての clone（`git clone --no-local`）と `ADR_DIR` の写しで壊した。

| 壊したもの | 結果 |
|---|---|
| ADR の写しに 私設 IP（`<192.168 の範囲の 1 つ>`） を足して `git add` | rc=1（check-private が `NG docs/adr/…:29` を出す） |
| `tools/check-private.allow` に `docs/adr/README.md` を足す | rc=1（`error: tools/check-private.allow に docs/adr/ の例外がある`） |
| ADR の 1 件の `## 背景` と `## 決定` を入れ替える | rc=1（`見出しの順序が違う`） |
| CI の awk を mawk 1.3.4 にする（`PATH` の先頭に `awk → mawk`） | 本番 rc=0・`--self-test` rc=0（`{4}` の区間の正規表現も動く。ただし手元の mawk は 20260129 版で、runner の版とは違う） |
| **形の正しい 7 件目の ADR を足し、README の一覧にも足して `git add`** | **rc=1（`error: docs/adr/ がまだ追跡されていない` ×2）→ R1** |
| **検査の台本の 4 つのガードを潰す（下）** | **`--self-test` rc=0 のまま → R2** |

## 手 3: Scenario と test の突き合わせ

13 本とも印の直後の関数が Scenario の主張を観測している（ファイル名の比較・`- 日付:` と 1 行目の見出しの比較・`sort -r` との比較・正規表現・
見出しの個数と行番号・`git ls-files`・表の列・節の行・chain の範囲の `grep -Fx`・出所の行・check-private と allow）。主張の階層の違いは無かった。
ただし「CH12 で置いた README と 5 件の ADR がある」と「網の名前の検査が通る」の 2 本は `git ls-files docs/adr` が**ちょうど 6 行**であることを見ており、
spec の「がある」「返し」より狭い（R1）。「一覧の日付と題」「ファイル名の形」「この順に持つ」は、印の先の検査を消しても自己検査が緑のまま（R2）。

## 手 4: 本人の決定が test で固定されているか

| 決定 | 書き換えたときに落ちるもの |
|---|---|
| Q1 逐語のまま置く | `--verbatim`（手 1 で 1 語変えて rc=1）。CI には載らない（D1 のとおり） |
| Q2 repo の中のコマンドで・4 つの名前で判定 | `--backlog`（出所から 1 つ消して rc=1 → ただし出力は R3） |
| Q3 chain job の 1 行 | `check_ci` の `grep -Fx`・自己検査の `ci` の壊し方（行を消すと rc≠0） |
| Q4 `AGENTS.md` の 1 行 | `check_guidance`・自己検査の `agents` の壊し方 |

値を変えても全部通る決定は無かった。

## 手 5: tasks の `[x]` と実体

5 件とも検証のコマンドが実在し rc=0（上の表）。`cargo test --test` 型の検証は無い。
コマンドを持たない主張は 4.1 の「`docs/changes/CH12.md` の食い違いの節が最新」の 1 つ。`.superpowers/sdd/ch12-task-4/task-4-report.md` は「8 件と harness2 の ADR 未配置を含み、D3 の一覧と一致」とだけ書く。
harness2 の main を見ると、未配置の 1 項目はもう事実でない → R5。PR 本文 `docs/briefs/CH12-pr.md` は上流の段（d3e01fccd。main にある）が置いたもので、下流の欠けではない。

## 手 6: 隙間

この change は文書と検査の台本だけで、実行時に記録を持たない（捨てる経路は無い）。隙間は「後で ADR を足す」と「メモリを消す」の 2 つの状態で出た（R1・R4）。
`.harness2` が無い clone では `--backlog` が `error: adr_backlog.py の台本が無い` rc=127 で止まり、空の合格にならないことも確かめた。
defer: 該当なし（確かめた範囲: R1〜R4 はどれもこの change の `tools/check-adr.sh` の中で閉じ、他 Story の持ち物に掛からない）。

## 手 7: 該当なし（`plan_fix.py pending` が `[]`）

---

## R1. 形の正しい ADR を 1 件足すと、CI の検査が「まだ追跡されていない」で落ちる
- 成果物: `tools/check-adr.sh` / `.github/workflows/ci.yml`（chain job）
- 根拠: 使い捨ての clone で `docs/adr/202611080000-sample-next.md`（README の「形」どおり。日付・状態・出所・5 つの見出し）を足し、README の一覧の先頭に `- [2026-11-08 次の判断の見本](202611080000-sample-next.md)` を足して `git add docs/adr`。
  `ADR_SELF_TEST_CHILD=1 tools/check-adr.sh` → rc=1・`error: docs/adr/ がまだ追跡されていない` ×2。`tools/check-adr.sh --self-test` → rc=1・`error: self-test の基準の木で本番検査が通らない`。
  原因は `tools/check-adr.sh:151` の `[ "$count" -eq 6 ]`（追跡されているファイルが 6 つ**ちょうど**でなければ落ちる）。`check_tracked` は印「CH12 で置いた README と 5 件の ADR がある」と「網の名前の検査が通る」の 2 回呼ばれる（:306・:316）。
  Q3 の本人の答えは「後から足す ADR も同じ形に保たれる」、design D2 は「ADR を足しても消しても成り立つべき性質」。いまの検査は、次の ADR を足す最初の PR を CI で止め、誤った理由（未追跡）を出す。自己検査は `check_tracked` が `ADR_DIR` を見ず実の repo の索引を見るので、この経路を試せない
- kind: technical
- 提案: 「6 つちょうど」を「6 つの名前がすべて `git ls-files docs/adr` にある」に変える（:152-154 の個別の確かめはもうある）。自己検査に「形の正しい ADR を 1 件足すと**通る**」の正の壊し方を足す

## R2. 自己検査（CI が「台本が壊れても緑にならない」ために走らせる口）が、4 つのガードの消失を見抜かない
- 成果物: `tools/check-adr.sh`（`self_test`）
- 根拠: 使い捨ての clone で検査の台本の行を `:` に置き換え、`tools/check-adr.sh --self-test` を走らせた。
  ファイル名の正規表現（:88）→ rc=0 / 一覧と ADR の日付の比較（:80）→ rc=0 / 見出しの順序の比較（:111）→ rc=0 / 状態の行の検査（:102）→ rc=0 / 4 つ同時 → `--self-test` rc=0・本番 rc=0。
  対照: 見出しの個数（:107）を潰すと rc=1（`decision` の壊し方が拾う）。:111 が守るものは実在する: `## 背景` と `## 決定` を入れ替えた ADR は元の台本で rc=1（`見出しの順序が違う`）、:111 を消した台本で rc=0。
  `filename` の壊し方（大文字の名前に変える）は一覧との不一致（:67）で落ちるので、:88 が無くても緑。design D9 の 9 つの壊し方はすべて実装されているが、その 9 つは Scenario「一覧の日付と題」「ファイル名の形」「この順に持つ」の 3 本と、状態の値の規則を守っていない
- kind: technical
- 提案: 自己検査に、一覧と ADR の両方の名前を大文字にそろえる・ADR の `- 日付:` だけを変える・2 つの見出しを入れ替える・`- 状態: 検討中` にする、の 4 つの壊し方を足す（どれも期待する `error:` の文面まで見る）

## R3. `--backlog` は 1 つのメモリが残っただけでも、4 つとも「残りにある」と出す
- 成果物: `tools/check-adr.sh`（`backlog` :274-285・`backlog_json_check`）
- 根拠: `ADR_DIR` の写しで衛星の ADR の出所から `foundation-for-satellites-premise.md` だけを消し、`ADR_BACKLOG=<harness2>/scripts/adr_backlog.py ADR_DIR=<写し> tools/check-adr.sh --backlog` → rc=1（正しく落ちる）が、
  出力は `worktree-port-allocation-followup.md: 残りにある` / `prefer-collision-free-allocation-over-checks.md: 残りにある` / `foundation-for-satellites-premise.md: 残りにある` / `lint-purity-ui-refresh-plan.md: 残りにある` / `NG backlog`。
  ほかの 3 つは出所に名指しされたままで、残りに無い。`backlog_json_check` が「4 つのどれかが在る」を 1 つの rc にまとめ、その rc で 4 行を同じ文面にしている。D5 は 1 行ごとに `<メモリ名>: 残りに無い|ある`。
  判定（rc）は正しいが、落ちたときにどれを直すかを出力が誤って言う
- kind: technical
- 提案: python 側で名前ごとの有無を返し（例: 残る名前を 1 行ずつ）、名前ごとに `残りに無い` / `残りにある` を出す

## R4. メモリを消すと、手元の `--self-test` が rc=1 になる（CI は緑のまま）
- 成果物: `tools/check-adr.sh`（`self_test` :469-480・`backlog` :262-265）
- 根拠: 空のフォルダをメモリの置き場にして（4 つのメモリを消した後を模す）`ADR_BACKLOG=<harness2>/scripts/adr_backlog.py ADR_MEMORY=<空> tools/check-adr.sh --self-test` → rc=1、
  `error: self-test の --backlog 子が予期せず失敗した: 1` と `error: self-test の --backlog 子が公開してはいけない行を出した: error: --adr の空の正の対照で 4 つのメモリを数えられない`。
  自己検査の (d) は子の `--backlog` を**手元の本物の**台本とメモリで走らせ、rc が 0 か 127 でなければ落とす。正の対照が落ちると rc=1 なので自己検査が赤になる。
  さらにその error の行は `無い` で終わらないので、台本自身の D5 の形の検査（:476 `^error: .*無い$`）に当たらない。
  CH12 の目的は「メモリは失われうるので正本を repo に移す」で、移した後にメモリを消すのは自然な次の操作。そのとき手元の `--self-test`（tasks 2.2・3.1 の検証、verify-run で走る）が台本の壊れでなく環境で赤になる。
  CI の runner は台本が無く rc=127 の枝を通るので見えない。正の対照の失敗は rc=1 で、verify-run は環境の欠落（BLOCKED_INFRA）でなく FAIL と読む
- kind: technical
- 提案: 自己検査の (d) は子を偽の台本と使い捨ての置き場に向けて走らせ、手元のメモリに依らせない。正の対照の失敗の文面を D5 の形に合わせる（または形の正規表現を、台本が出しうる `error:` の文面に合わせる）

## R5. 「harness2 の ADR はまだ harness2 の main に無い」は、もう事実でない（CH12.md の節と PR 本文）
- 成果物: `docs/changes/CH12.md`（`## ADR と repo の今の食い違い（直していない）` の末尾の段落） / `docs/briefs/CH12-pr.md:41` / `design.md` の Context（`adr_backlog.py` は `.harness2` に無い）
- 根拠: `git -C "$(readlink -f .harness2)" ls-tree --name-only origin/main docs/adr/` → 27 行（ADR 26 件 + README）。`scripts/adr_backlog.py` も origin/main にある（足した commit `ae1a0b77f`、2026-11-08 13:06 +0900。ADR は `1e140a4b3`）。
  ADR が題名で参照する 2 件も origin/main にある: 「取り違えは、被らない割り当てを機械が決めて防ぐ」→ `docs/adr/202610030000-collision-free-allocation.md`、「要件から来ない作業は change として Story と同じグラフに流す」→ `docs/adr/202610170001-change-flow.md`。
  手 2 の表の `tools/check-adr.sh --backlog` も、D4 の探し方の 2.（`.harness2/scripts/adr_backlog.py`）で台本を見つけている。
  それでも CH12.md の節は「2026-11-08 の時点で harness2 の main に無い（push も PR も無い）」、PR 本文は「まだ harness2 の main に無い」と書く。Task 4.1 はこの節が「最新」であることを `[x]` にしている（task-4-report は 8 件と未配置の 1 項目を数えただけ）。
  ADR の本文（逐語）は変えなくてよい。食い違いの一覧が、存在しない食い違いを 1 つ挙げている
- kind: daily
- 提案: CH12.md の節の末尾の段落と PR 本文の同じ段落を「harness2 の main に入った（`1e140a4b3`・`ae1a0b77f`）。参照の 2 件は行き先がある」に直す（`## 依頼（逐語）` の節の外なので `--verbatim` の sha256 は変わらない）
