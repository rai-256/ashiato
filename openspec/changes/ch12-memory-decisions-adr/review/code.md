<!-- 処置の判定: re-review -->

# CH12 code_verify — 独立の検証（2026-11-09、HEAD a1703dba1・origin/main 0f38ad74e）

前の回（e90c75431）の後、コードは変わっていない（`git diff e90c75431 HEAD --stat` は `review/code.md` だけ）。
前の回の指摘は引き継がず、全部を走らせ直した。再現した 7 件は同じ番号 R1〜R7 のまま、根拠を今回の実測に書き直した。新しい指摘は無い。

## 申告と実測

申告: tasks の `[x]` は 5/5（1.1・2.1・2.2・3.1・4.1）。検証コマンドは下の 10 本（`--self-test` は 2.2 と 3.1 で同じもの）。

| タスク | 検証コマンド | 申告 | 実測 |
|---|---|---|---|
| 1.1 | `bash -n tools/check-adr.sh` | rc=0 | rc=0 一致 |
| 2.1 | `tools/check-adr.sh --verbatim` | rc=0 | rc=0 一致 |
| 2.2 | `tools/check-adr.sh` | rc=0 | rc=0 一致（印 11 行・`OK 網の名前・私設 IP は入っていない（2264 ファイル）`） |
| 2.2 / 3.1 | `tools/check-adr.sh --self-test` | rc=0 | rc=0 一致（印 2 行） |
| 2.2 | `tools/check-private.sh` | rc=0 | rc=0 一致 |
| 2.2 | `CLAUDE.md` `AGENTS.md` `ci.yml` の numstat が `1\t0` | rc=0 | rc=0 一致 |
| 3.1 | `tools/check-adr.sh --backlog` | rc=0 | rc=0 一致（4 行とも `残りに無い`・`OK backlog`） |
| 4.1 | main からの差分の範囲 | rc=0 | rc=0 一致 |
| 4.1 | `python3 scripts/check_scenarios.py . ch12-memory-decisions-adr` | rc=0 | rc=0 一致（`scenarios: OK`。warn 4 行はどれも他の change の印） |
| — | `python3 scripts/plan_fix.py pending openspec/changes/ch12-memory-decisions-adr` | — | `[]` |

`[x]` の検証は全部実在し、rc=0 になった。指摘は、検証が**見ていないところ**（ADR を足した後・検査そのものが壊れた場合・形の違う行・メモリを消した後）から出た。

## 手 1: 固定値を独立に再計算する

`--verbatim` の定数 `7e1b0a5c…2e58b94` を、台本の awk を使わずに python（`split('\n')`・`## 依頼（逐語）` から次の `## ` の手前まで・各行に改行を付けて `hashlib.sha256`）で計算し直した。
いまの `docs/changes/CH12.md`・登録の commit `ff9a78dd7`・origin/main で CH12.md を足した commit `b77e52a51` の 3 つとも、243 行・`7e1b0a5c755c8b19fcb7c138db6f2f87897d96baa46f90b3bc889ba872e58b94` で一致した。
6 つの塊も正規表現を使わずに `=== ファイル: … ===` の行で区切って取り出し、`docs/adr/` の 6 ファイルとバイト列で比べた。6/6 一致。

## 手 2: ガードをわざと壊す

使い捨ての clone（`git clone --no-local`、HEAD を checkout、`.harness2` は本物を指す）で壊した。

| 壊したもの | 結果 |
|---|---|
| ADR に私設 IP（192.168 の範囲の 1 つ）を足して `git add` | rc=1（check-private が `NG docs/adr/…:29` を出す） |
| `tools/check-private.allow` に `docs/adr/README.md` を足す | rc=1（`tools/check-private.allow に docs/adr/ の例外がある`） |
| `## 背景` と `## 決定` を入れ替える | rc=1（`見出しの順序が違う`） |
| `- 日付:` だけを変える | rc=1（`README の日付と ADR の日付が違う`） |
| 1 行目の `# ` を消す | rc=1（`README の題と見出しが違う`） |
| 一覧の項目と ADR の名前を両方大文字に | rc=1（`ファイル名が時刻と英小文字 slug の形でない`） |
| 衛星の ADR の出所から `foundation-for-satellites-premise.md` を消す | rc=1（`出所に foundation-for-satellites-premise.md が無い`） |
| CI の runner を模す（`.harness2` を外し `HOME` を空のフォルダに） | `--self-test` rc=0・`--backlog` rc=127（`error: adr_backlog.py の台本が無い`）。空の合格にならない |
| **形の正しい 7 件目の ADR を足し、一覧にも足して `git add`** | **rc=1（`docs/adr/ がまだ追跡されていない` ×2）→ R1** |
| **検査の行を 1 本ずつ `:` に潰して `--self-test`（19 本）** | **11 本で rc=0 のまま（ほかの行が代わりに拾わないのは 8 本） → R2** |
| **出所からメモリ名を 1 つ消して `--backlog`** | **rc=1 だが 4 行とも `残りにある` → R3** |
| **空のメモリの置き場で `--self-test`** | **rc=1 → R4** |
| **`- 状態: 検討中` を 2 行目として足す・一覧に形の違う項目を足す** | **どちらも rc=0 → R6** |
| **`- 出所:` の行を末尾（`## 影響` の後）へ移す** | **rc=0 → R7** |
| `- 状態: 置き換え済み（→ 209901010000-nope.md）`（指す先が無い） | rc=0（spec は形しか求めないので R にしない。手 6） |

一時の場所: `TMPDIR` を空のフォルダにして 4 つの口を走らせた（4 つとも rc=0）。後に残ったものは 0 件で、フォルダはそのまま `rmdir` できた。

## 手 3: Scenario と test を突き合わせる

`check_scenarios.py` は OK（13 本とも `tools/check-adr.sh` に印）。印の先の観測を 1 本ずつ見た。

- 「1 対 1 で対応する」: 一覧の項目のうち `- [YYYY-MM-DD ` の形のものしか拾わない（:52）。形の違う項目は見えない（R6）
- 「この順に持つ」: 見出し 5 つの順と、項目 3 行の**値の形が正しいもの**の数しか見ない。`- 状態:` の行の総数・3 行の位置は見ない（R6・R7）。主張（項目と見出しの並び）と検査（見出しだけの並び）の階層が違う
- 「CH12 で置いた README と 5 件の ADR がある」「網の名前の検査が通る」: `git ls-files docs/adr` が**ちょうど 6 行**かを見る（:151）。spec の「がある」「返し」より狭い（R1）
- 「5 件の ADR の出所に、移したメモリのファイル名がある」: 観測は正しい（手 2）。ただし自己検査はこの検査を守っていない（R2）
- 「出所の行が無い ADR は落ちる」「決まった見出しの欠けた ADR は落ちる」: 自己検査の壊し方 `source`・`decision` で、期待する文面とファイル名まで見ている
- 日付と題・並び・ファイル名の形・`CLAUDE.md`・`AGENTS.md`・CI の行: 印の先の検査が主張を観測している（守りの抜けは R2）

## 手 4: 本人の決定が test で固定されているか

| 決定 | 書き換えたときに落ちるもの |
|---|---|
| Q1 逐語のまま置く | `--verbatim`（CI には載らない。D1 のとおり） |
| Q2 repo の中のコマンドで、4 つの名前で判定 | `--backlog`（出所から 1 つ消すと rc=1。出力の言い方は R3） |
| Q3 chain job の 1 行 | `check_ci`（:135）。:136 を潰すと `--self-test` rc=1 |
| Q4 `AGENTS.md` の 1 行 | `check_guidance`（:124）。:125 を潰すと `--self-test` rc=1 |

値を変えても全部通る決定は無かった。ただし Q3 の答えの意図「後から足す ADR も同じ形に保たれる」は、R1 で、次の ADR を足す PR を誤った理由で止める形になっている。

## 手 5: tasks の `[x]` と実体

5 件とも検証コマンドが実在して rc=0（上の表）。`cargo test --test` 型は無い。
コマンドで確かめていない主張は 4.1 の「`docs/changes/CH12.md` の食い違いの節が最新」の 1 つで、事実でなくなっている（R5）。

## 手 6: 隙間

この change は文書と検査の台本だけで、実行時に記録を持たない（捨てる経路は無い）。隙間は、後で ADR を足す（R1）・メモリを消す（R4）・形の違う行がある（R6・R7）で出た。
`置き換え済み（→ 存在しないファイル名）` も rc=0。spec は形しか求めないので R にしないが、README は「消さず置き換え済みにして新しい ADR を足す」と決めており、後継の指す先が無いと辿れなくなる。
食い違いの一覧のうち 5（`crates/winsvc/Cargo.toml` に `lints` の行が無い）はいまも健在（grep で 0 件）。
defer: 該当なし（確かめた範囲: R1〜R7 はどれもこの change の `tools/check-adr.sh` と文書の中で閉じ、他の Story の持ち物に掛からない）。

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
- 処置: fixed 1.1 （`check_tracked` は 6 つの名前が `git ls-files docs/adr` にあるかだけを見て、数を問わない。自己検査に正の対照「形の正しい ADR を 1 件足して `git add` しても通る」を足し、`ADR_GIT_ROOT` で使い捨ての木（`git init` した写し）の索引へ向けて試せるようにした）
- 判定: ADDRESSED（re-review 1 @7a4d8e6dfd806ec9223c49af885d8d2b98c983d0）
- 判定: ADDRESSED（re-review 2 @37d7af4976af2460d7b9578eed5b3554d60d344b）
- 判定: ADDRESSED（re-review 3 @c1eef3df96718f2286ecef74c5422f0f65f120e7）

## R2. 自己検査（CI が「台本が壊れても緑にならない」ために走らせる口）が、4 つのガードの消失を見抜かない
- 成果物: `tools/check-adr.sh`（`self_test`）
- 根拠: 使い捨ての clone で検査の台本の行を `:` に置き換え、`tools/check-adr.sh --self-test` を走らせた。
  ファイル名の正規表現（:88）→ rc=0 / 一覧と ADR の日付の比較（:80）→ rc=0 / 見出しの順序の比較（:111）→ rc=0 / 状態の行の検査（:102）→ rc=0 / 4 つ同時 → `--self-test` rc=0・本番 rc=0。
  対照: 見出しの個数（:107）を潰すと rc=1（`decision` の壊し方が拾う）。:111 が守るものは実在する: `## 背景` と `## 決定` を入れ替えた ADR は元の台本で rc=1（`見出しの順序が違う`）、:111 を消した台本で rc=0。
  `filename` の壊し方（大文字の名前に変える）は一覧との不一致（:67）で落ちるので、:88 が無くても緑。design D9 の 9 つの壊し方はすべて実装されているが、その 9 つは Scenario「一覧の日付と題」「ファイル名の形」「この順に持つ」の 3 本と、状態の値の規則を守っていない
- kind: technical
- 提案: 自己検査に、一覧と ADR の両方の名前を大文字にそろえる・ADR の `- 日付:` だけを変える・2 つの見出しを入れ替える・`- 状態: 検討中` にする、の 4 つの壊し方を足す（どれも期待する `error:` の文面まで見る）
- 処置: fixed 1.1 （自己検査の壊し方を 9 から 21 に足し、正の対照を 1 つ足した。どれも期待する `error:` の文面まで見る。追跡と allow の検査は `ADR_GIT_ROOT`・`ADR_PRIVATE_ALLOW` で自己検査から届く。検査の行を 1 本ずつ潰すと、自己検査が全部で落ちることを確かめた（report）
- 判定: ADDRESSED（re-review 1 @7a4d8e6dfd806ec9223c49af885d8d2b98c983d0）
- 判定: ADDRESSED（re-review 2 @37d7af4976af2460d7b9578eed5b3554d60d344b）
- 判定: ADDRESSED（re-review 3 @c1eef3df96718f2286ecef74c5422f0f65f120e7）

## R3. `--backlog` は 1 つのメモリが残っただけでも、4 つとも「残りにある」と出す
- 成果物: `tools/check-adr.sh`（`backlog` :274-285・`backlog_json_check`）
- 根拠: `ADR_DIR` の写しで衛星の ADR の出所から `foundation-for-satellites-premise.md` だけを消し、`ADR_BACKLOG=<harness2>/scripts/adr_backlog.py ADR_DIR=<写し> tools/check-adr.sh --backlog` → rc=1（正しく落ちる）が、
  出力は `worktree-port-allocation-followup.md: 残りにある` / `prefer-collision-free-allocation-over-checks.md: 残りにある` / `foundation-for-satellites-premise.md: 残りにある` / `lint-purity-ui-refresh-plan.md: 残りにある` / `NG backlog`。
  ほかの 3 つは出所に名指しされたままで、残りに無い。`backlog_json_check` が「4 つのどれかが在る」を 1 つの rc にまとめ、その rc で 4 行を同じ文面にしている。D5 は 1 行ごとに `<メモリ名>: 残りに無い|ある`。
  判定（rc）は正しいが、落ちたときにどれを直すかを出力が誤って言う
- kind: technical
- 提案: python 側で名前ごとの有無を返し（例: 残る名前を 1 行ずつ）、名前ごとに `残りに無い` / `残りにある` を出す
- 処置: fixed 3.1 （`backlog_json_check` が残りにある名前を 1 行ずつ返し、`backlog` は名前ごとに `残りに無い` / `残りにある` を出す。自己検査に「1 つだけ名指しを外すと、その 1 つだけ `残りにある`」を足した）
- 判定: ADDRESSED（re-review 1 @7a4d8e6dfd806ec9223c49af885d8d2b98c983d0）
- 判定: ADDRESSED（re-review 2 @37d7af4976af2460d7b9578eed5b3554d60d344b）
- 判定: ADDRESSED（re-review 3 @c1eef3df96718f2286ecef74c5422f0f65f120e7）

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
- 処置: fixed 3.1 （自己検査の `--backlog` の子は、偽の台本と使い捨てのメモリの置き場で走らせ、手元のメモリと harness2 に依らない。正の対照の失敗の文面を D5 の形 `error: …無い` にし、自己検査で rc と文面を見る）
- 判定: ADDRESSED（re-review 1 @7a4d8e6dfd806ec9223c49af885d8d2b98c983d0）
- 判定: ADDRESSED（re-review 2 @37d7af4976af2460d7b9578eed5b3554d60d344b）
- 判定: ADDRESSED（re-review 3 @c1eef3df96718f2286ecef74c5422f0f65f120e7）

## R5. 「harness2 の ADR はまだ harness2 の main に無い」は、もう事実でない（CH12.md の節と PR 本文）
- 成果物: `docs/changes/CH12.md`（`## ADR と repo の今の食い違い（直していない）` の末尾の段落） / `docs/briefs/CH12-pr.md:41` / `design.md` の Context（`adr_backlog.py` は `.harness2` に無い）
- 根拠: `git -C "$(readlink -f .harness2)" ls-tree --name-only origin/main docs/adr/` → 27 行（ADR 26 件 + README）。`scripts/adr_backlog.py` も origin/main にある（足した commit `ae1a0b77f`、2026-11-08 13:06 +0900。ADR は `1e140a4b3`）。
  ADR が題名で参照する 2 件も origin/main にある: 「取り違えは、被らない割り当てを機械が決めて防ぐ」→ `docs/adr/202610030000-collision-free-allocation.md`、「要件から来ない作業は change として Story と同じグラフに流す」→ `docs/adr/202610170001-change-flow.md`。
  手 2 の表の `tools/check-adr.sh --backlog` も、D4 の探し方の 2.（`.harness2/scripts/adr_backlog.py`）で台本を見つけている。
  それでも CH12.md の節は「2026-11-08 の時点で harness2 の main に無い（push も PR も無い）」、PR 本文は「まだ harness2 の main に無い」と書く。Task 4.1 はこの節が「最新」であることを `[x]` にしている（task-4-report は 8 件と未配置の 1 項目を数えただけ）。
  ADR の本文（逐語）は変えなくてよい。食い違いの一覧が、存在しない食い違いを 1 つ挙げている
- kind: daily
- 提案: CH12.md の節の末尾の段落と PR 本文の同じ段落を「harness2 の main に入った（`1e140a4b3`・`ae1a0b77f`）。参照の 2 件は行き先がある」に直す（`## 依頼（逐語）` の節の外なので `--verbatim` の sha256 は変わらない）
- 処置: fixed D3 仮 （CH12.md の節の末尾の段落・完了の判定の前置き、PR 本文の同じ段落を「いまは harness2 の main にある（`1e140a4b3`・`ae1a0b77f`）。参照の 2 件も行き先がある」に直した。design の Context と D3 には 2026-11-09 の追記を足した。`## 依頼（逐語）` の外なので `--verbatim` は rc=0 のまま）
- 判定: ADDRESSED（re-review 1 @7a4d8e6dfd806ec9223c49af885d8d2b98c983d0）
- 判定: ADDRESSED（re-review 2 @37d7af4976af2460d7b9578eed5b3554d60d344b）
- 判定: ADDRESSED（re-review 3 @c1eef3df96718f2286ecef74c5422f0f65f120e7）
- 判定: ADDRESSED（re-review 4 @9e70f08094e02a09942326096a8eeff37f3c1c1b）
- 判定: ADDRESSED（re-review 5 @898546c2cfe79ca307e5975f1490e0b6bb3a5f58）

## R6. 決まった形に当たる行しか数えないので、形の違う行は落ちずに見えなくなる
- 成果物: `tools/check-adr.sh:52`（一覧の項目）・`:102`（状態の行）
- 根拠: `ADR_DIR` の写しで試した。
  (a) `202610110001-ui-surface-only.md` の `- 状態: 採用` の次の行に `- 状態: 検討中` を足すと rc=0。:102 は `採用|置き換え済み（→ …）` に当たる行だけを数えるので「1 個」になる。
  spec の Scenario「ADR は決まった項目と見出しをこの順に持つ」は「`- 状態:` の行がそれぞれちょうど 1 つ」。
  (b) README の一覧に `- [2026-1-08 x](202611080000-x.md)`（日付の桁が違い、指すファイルも無い）を足すと rc=0。:52 は `^- \[[0-9]{4}-[0-9]{2}-[0-9]{2} ` に当たる行しか項目として拾わない。
  spec の Requirement は「一覧の項目は `- [<日付> <題>](<ファイル名>)` の形」、Scenario は「一覧にあって実ファイルが無いものが 0 件」で、どちらにも反するのに通る。
  指すファイルが実在すると落ちるが、その理由は「1 対 1 でない」で、本当の原因（項目の形）を言わない。
  （:101 の日付の行も同じ作りだが、`- 日付: 不明` を足すと README の日付との比較が拾って rc=1 になった）
- kind: technical
- 提案: 先に `^- 状態:` と、一覧の節の `^- ` で数えてから、それぞれの形を確かめる（形の違う行は、ファイル名と一緒に `error:` で出す）。自己検査に (a)(b) の壊し方を足す
- 処置: fixed 1.1 （状態・日付の行は形を問わず `^- 状態:` / `^- 日付:` で数えてから値の形を見る。一覧の節の `- ` の行は全部項目として読み、形の違う行は `README の一覧の項目の形が違う: <行>` で落とす。自己検査に (a)(b) を足した）
- 判定: ADDRESSED（re-review 1 @7a4d8e6dfd806ec9223c49af885d8d2b98c983d0）
- 判定: ADDRESSED（re-review 2 @37d7af4976af2460d7b9578eed5b3554d60d344b）
- 判定: ADDRESSED（re-review 3 @c1eef3df96718f2286ecef74c5422f0f65f120e7）

## R7. 日付・状態・出所の 3 行の位置を見ていない
- 成果物: `tools/check-adr.sh`（`check_shape` :94-115）
- 根拠: `ADR_DIR` の写しで、`202610110001-ui-surface-only.md` を 3 通りに変えた。どれも rc=0。
  `- 出所:` の行をファイルの末尾（`## 影響` の後）へ移す / `- 日付:` の行を末尾へ移す / `- 状態:` と `- 出所:` の行を入れ替える。
  :111 の順序の検査は 5 つの見出しだけを見る。spec の Scenario「ADR は決まった項目と見出しをこの順に持つ」は「`- 日付:`・`- 状態:`・`- 出所:` の行が…、見出しが…、この順にある」。
  Requirement は「README の『形』のとおり」で、README の形では 3 行が 1 行目の題の直後、`## 背景` の前にある。
  主張（項目と見出しの並び）と検査（見出しだけの並び）の階層が違う
- kind: technical
- 提案: 3 行が `日付 → 状態 → 出所` の順で、どれも `## 背景` より前にあることを確かめる。自己検査に「出所の行を `## 影響` の後へ移す」壊し方を足す
- 処置: fixed 1.1 （日付 → 状態 → 出所の順で、どれも `## 背景` より前にあることを見る。自己検査に「出所の行を末尾へ移す」を足した）
- 判定: ADDRESSED（re-review 1 @7a4d8e6dfd806ec9223c49af885d8d2b98c983d0）
- 判定: ADDRESSED（re-review 2 @37d7af4976af2460d7b9578eed5b3554d60d344b）
- 判定: ADDRESSED（re-review 3 @c1eef3df96718f2286ecef74c5422f0f65f120e7）

## security review（a658d12..dd63369）

見た経路:
- 差分の入口: 新しい API・画面・listen・依存は無い。足したのは `docs/adr/` の文書 6 件、`tools/check-adr.sh`、CI の chain job の 1 行、`CLAUDE.md`・`AGENTS.md` の案内 1 行ずつ（`git diff --stat` で 15 ファイルを確かめた）（3）
- 手元のメモリ → `--backlog`（`find_memory_dir` :210-223 → `adr_backlog.py` の stdout/stderr は `$tmp` の中のファイルへ :260・:268 → 出力は固定文の 4 行と `OK|NG backlog` だけ :279-291）。手元の本物のメモリで走らせると 4 行と `OK backlog`、rc=0（4）
- 偽の `adr_backlog.py`（stderr と JSON の `description` に `SECRET-…` を出し、5 つめの名前も返す）を `ADR_BACKLOG` に差して `--backlog`: 出力は 4 行と `NG backlog`、`SECRET` は 0 件（4）
- 追跡ファイル → 公開の repo（`gh repo view` は `PUBLIC`）: 差分の全ファイルに、手元のメモリ 43 件の名前を突き合わせた。出たのは移した 4 件と、harness2 の ADR の名前（`change-flow`。`review/code.md` の R5）だけ。ほかのメモリの説明に特有の語（`KB5127648`・`OOM kill`・`agent-logs`・`herdr` など）も 0 件。`/home/`・`ts.net`・私設 IP も無い（4）
- CI: `check-adr.sh` の step（ci.yml:503）は、`secrets.HARNESS2_TOKEN` を渡す step（:542・:549）より前にある。token は job の env に無い（:493 の注記どおり）ので、この台本から token は読めない。runner にはメモリの置き場が無く、自己検査の子の `--backlog` は rc=127 の枝を通る（:222・:469-475）（2）

安全の決め手: `--backlog` は、手元のメモリを読む台本の出力を一行も出さず、決まった 4 つの名前の有無だけを固定文で出す —— 確かさ 4。上の偽の台本（秘密の名前と説明を stdout・stderr に出すもの）を差して走らせ、出力の `SECRET` が 0 件、rc=1、出力は `…: 残りにある` ×4 と `NG backlog` だった。

指摘なし

範囲外: `tools/check-private.sh` は網の名前・私設 IP だけを見て、メモリの名前・`~/.claude` の中身の写しは見ない（差分が触れたのは allow の検査 :317 だけ）。

## /code-review（a658d12..706d67b）

見た: Claude Code の組み込みの `/code-review`（10 件）。写しは scripts/code_review_record.py

## R8. self-test starts its child runs through the relative "$0" after the script has already done `cd "$ROOT"`. The children are only found when the script is invoked from the repo root.
- 成果物: tools/check-adr.sh:396
- 根拠: `cd tools && ./check-adr.sh --self-test` sets $0 to ./check-adr.sh, which does not exist under ROOT, so every child exits 127. I ran it: rc=1, with 'self-test の基準の木で本番検査が通らない' and every break case reporting the wrong error. Fix: resolve the path once, e.g. SELF="$ROOT/tools/check-adr.sh", and use that (lines 396/433/441/456/465/470).（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。settle が直し方を決める）
- 処置: fixed 1.1 （`SELF="$ROOT/tools/<台本の名前>"` を一度決め、子は全部それで起こす。`cd tools && ./check-adr.sh --self-test` が rc=0）
- 判定: ADDRESSED（re-review 1 @7a4d8e6dfd806ec9223c49af885d8d2b98c983d0）
- 判定: ADDRESSED（re-review 2 @37d7af4976af2460d7b9578eed5b3554d60d344b）
- 判定: ADDRESSED（re-review 3 @c1eef3df96718f2286ecef74c5422f0f65f120e7）

## R9. check_tracked requires exactly 6 tracked files under docs/adr. This contradicts README, which says to keep adding ADRs and to add a new one when a decision is replaced.
- 成果物: tools/check-adr.sh:151
- 根拠: Adding a 7th ADR and `git add`-ing it gives count=7, so CI fails with the wrong message 'docs/adr/ がまだ追跡されていない'. The check should be 'the expected files are included', or 'every file in ADR_DIR is tracked'.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。settle が直し方を決める）
- 処置: fixed 1.1 （R1 と同じ直し。数の一致をやめ、6 つの名前の追跡だけを見る）
- 判定: ADDRESSED（re-review 1 @7a4d8e6dfd806ec9223c49af885d8d2b98c983d0）
- 判定: ADDRESSED（re-review 2 @37d7af4976af2460d7b9578eed5b3554d60d344b）
- 判定: ADDRESSED（re-review 3 @c1eef3df96718f2286ecef74c5422f0f65f120e7）

## R10. In --verbatim, if the file content differs but splitlines() gives the same lines, first_difference is None and `None + 1` raises a TypeError.
- 成果物: tools/check-adr.sh:372
- 根拠: An ADR with no trailing newline, an extra trailing newline, or CRLF line endings (.gitattributes was also changed) gives actual != wanted but identical line lists. Python crashes with a traceback instead of naming the block and the differing line.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。settle が直し方を決める）
- 処置: fixed 2.1 （`first_difference` が None のときは「末尾の改行か改行の形が依頼（逐語）と一致しない」を出す。ADR は改行を変換せずに読み、CRLF も落とす）
- 判定: ADDRESSED（re-review 1 @7a4d8e6dfd806ec9223c49af885d8d2b98c983d0）
- 判定: ADDRESSED（re-review 2 @37d7af4976af2460d7b9578eed5b3554d60d344b）
- 判定: ADDRESSED（re-review 3 @c1eef3df96718f2286ecef74c5422f0f65f120e7）

## R11. --self-test runs the real --backlog against the developer's memory and harness2, and only accepts rc 0 or 127. Its result depends on the local environment.
- 成果物: tools/check-adr.sh:470
- 根拠: Once the ADR is written, the natural next step is deleting the 4 memory files. Then the empty --adr positive control fails and returns rc=1, so `--self-test` fails on that machine with '--backlog 子が予期せず失敗した: 1'. The emitted line 'error: …数えられない' also fails the `無い$` regex, which adds a 'line that must not be published' error. CI happens to pass only because there is no memory there.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。settle が直し方を決める）
- 処置: fixed 3.1 （R4 と同じ直し。自己検査は本物の `--backlog` を走らせない）
- 判定: ADDRESSED（re-review 1 @7a4d8e6dfd806ec9223c49af885d8d2b98c983d0）
- 判定: ADDRESSED（re-review 2 @37d7af4976af2460d7b9578eed5b3554d60d344b）
- 判定: ADDRESSED（re-review 3 @c1eef3df96718f2286ecef74c5422f0f65f120e7）

## R12. run_main calls the same check function again under several Scenario labels (check_index_and_files 4 times, check_guidance and check_tracked twice each). The labels do not correspond to individual checks.
- 成果物: tools/check-adr.sh:296
- 根拠: A single title mismatch prints the same error 4 times, under 4 different Scenario headings. That makes it look as if the file-correspondence and order Scenarios failed too, and the reader cannot tell which Scenario actually broke. The fix is to split the functions by Scenario.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。settle が直し方を決める）
- 処置: fixed 1.1 （一覧とファイル名は `load_index` で 1 回だけ読み、Scenario ごとに `check_correspondence`・`check_list_titles`・`check_list_order`・`check_filenames`・`check_claude`・`check_agents`・`check_tracked`・`check_private_scope` に分けた。どの関数も 1 回だけ呼ぶ）
- 判定: ADDRESSED（re-review 1 @7a4d8e6dfd806ec9223c49af885d8d2b98c983d0）
- 判定: ADDRESSED（re-review 2 @37d7af4976af2460d7b9578eed5b3554d60d344b）
- 判定: ADDRESSED（re-review 3 @c1eef3df96718f2286ecef74c5422f0f65f120e7）

## R13. run_main runs tools/check-private.sh itself, but the chain job in CI already runs check-private.sh on the line just before. It runs twice.
- 成果物: tools/check-adr.sh:321
- 根拠: The CI chain job runs `./tools/check-private.sh` and then `./tools/check-adr.sh`, so the same full-repo scan runs twice. Any failure is also reported twice. Leaving check-private to CI and dropping the call here is enough.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。settle が直し方を決める）
- 処置: rejected: 2 回走ることは再現した（`.github/workflows/ci.yml:502` の `./tools/check-private.sh` の次の行 :503 が check-adr.sh で、check-adr.sh も呼ぶ）。ただし design.md D9 が「`tools/check-private.sh` は repo の根で走らせる」と check-adr.sh の中で呼ぶことを決めている。Scenario「docs/adr/ を含む追跡ファイルで、網の名前の検査が通る」の印は check-adr.sh にあり、tasks 2.2 の検証（verify-run）は check-adr.sh を単独で走らせるので、呼ぶのをやめるとその印の先が網の名前を何も観測しなくなる。重複の費用は chain job で 1 回分（job 全体で 0.2 分。ci.yml:501 の注記）
- 判定: ADDRESSED（re-review 1 @7a4d8e6dfd806ec9223c49af885d8d2b98c983d0）
- 判定: ADDRESSED（re-review 2 @37d7af4976af2460d7b9578eed5b3554d60d344b）
- 判定: ADDRESSED（re-review 3 @c1eef3df96718f2286ecef74c5422f0f65f120e7）

## R14. If tools/check-private.allow does not exist, grep exits 2 and the condition is false, so the check passes silently.
- 成果物: tools/check-adr.sh:317
- 根拠: If the allow file is renamed or moved, the check that should reject a docs/adr/ exception never runs and nobody notices. Missing files are not handled as errors.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。settle が直し方を決める）
- 処置: fixed 1.1 （allow の置き場（`ADR_PRIVATE_ALLOW`、既定 `tools/check-private.allow`）が無ければ `error: … が無い` で落とす。自己検査に「allow を消す」と「allow に docs/adr/ を足す」を足した）
- 判定: ADDRESSED（re-review 1 @7a4d8e6dfd806ec9223c49af885d8d2b98c983d0）
- 判定: ADDRESSED（re-review 2 @37d7af4976af2460d7b9578eed5b3554d60d344b）
- 判定: ADDRESSED（re-review 3 @c1eef3df96718f2286ecef74c5422f0f65f120e7）

## R15. The `sed -E 's/…$/&/'` replaces each match with itself (`&`), so it does nothing. It is dead code.
- 成果物: tools/check-adr.sh:53
- 根拠: It looks like it filters out list lines that are not in the link format, but it does not. Lines in the wrong format go straight into list_file, and readers misread what the check covers.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。settle が直し方を決める）
- 処置: fixed 1.1 （何もしない `sed` を消した。形の違う項目は R6 の直しで error になる）
- 判定: ADDRESSED（re-review 1 @7a4d8e6dfd806ec9223c49af885d8d2b98c983d0）
- 判定: ADDRESSED（re-review 2 @37d7af4976af2460d7b9578eed5b3554d60d344b）
- 判定: ADDRESSED（re-review 3 @c1eef3df96718f2286ecef74c5422f0f65f120e7）

## R16. find_backlog_script uses adr_backlog.py from the first worktree that has one in `git worktree list` for the harness2 repo. It may run an unmerged branch's version.
- 成果物: tools/check-adr.sh:203
- 根拠: If the main checkout of .harness2 does not have the script yet but a working branch's worktree does, that unreviewed version decides the --backlog result, and the result changes depending on which worktrees exist.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。settle が直し方を決める）
- 処置: rejected: 探し方の 3.（`.harness2` の指す repo の worktree）は design.md D4 が決めた順で、Risks（design.md の「harness2 の worktree の探し方は、別の枝の古い `adr_backlog.py` を拾いうる」）に挙げて受けている。しかも 2. が先に当たる: `ls .harness2/scripts/adr_backlog.py` は在り、`.harness2` は harness2 の `main` の checkout（`git -C "$(readlink -f .harness2)" branch --show-current` → `main`、台本は `ae1a0b77f`）なので、3. に落ちるのは main に台本が無い場合だけ。判定は 4 つのメモリ名の有無だけで、版の違いに依らない
- 判定: ADDRESSED（re-review 1 @7a4d8e6dfd806ec9223c49af885d8d2b98c983d0）
- 判定: ADDRESSED（re-review 2 @37d7af4976af2460d7b9578eed5b3554d60d344b）
- 判定: ADDRESSED（re-review 3 @c1eef3df96718f2286ecef74c5422f0f65f120e7）

## R17. --verbatim runs `python3 -` without -I, unlike backlog_json_check and the adr_backlog.py call, which use -I.
- 成果物: tools/check-adr.sh:337
- 根拠: Because the script has `cd "$ROOT"`, a module such as `re.py` or `pathlib.py` placed in the repo root or on PYTHONPATH gets imported and changes the result of the verbatim check. This is inconsistent with the rest of the same file, which isolates with -I.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。settle が直し方を決める）
- 処置: fixed 2.1 （`--verbatim` の python を `python3 -I` にした。自己検査の中の python も `-I`）
- 判定: ADDRESSED（re-review 1 @7a4d8e6dfd806ec9223c49af885d8d2b98c983d0）
- 判定: ADDRESSED（re-review 2 @37d7af4976af2460d7b9578eed5b3554d60d344b）
- 判定: ADDRESSED（re-review 3 @c1eef3df96718f2286ecef74c5422f0f65f120e7）

## final review（a658d12..2d9391c）

whole-branch の final reviewer（`superpowers:requesting-code-review` の code-reviewer）。HEAD でも origin/main を merge した使い捨ての木でも、引数なし・`--verbatim`・`--backlog`・`--self-test` は rc=0。Task の ⚠️ はすべて HEAD で確かめ済み（どれも merge 前の直しは不要）。判定: With fixes（R18）。

## R18. 自己検査が「最も古い ADR がポートの ADR で状態が `採用`」を暗に仮定しており、README どおりに ADR を足す・置き換えると CI の chain job が赤になる（R1 の直しが不完全）
- 成果物: `tools/check-adr.sh`
- 根拠: `tools/check-adr.sh:613` は対象を `find … -name '20*.md' | sort | head -n1`（ファイル名で最も古い ADR）で動的に選ぶが、壊し方 `source-name`（:481。出所に `worktree-port-allocation-followup.md` がある前提）・`untracked`（:500。対象がハードコードした 6 つの名前の 1 つである前提）・`status-value` / `status-extra`（:489・:490。sed が `^- 状態: 採用$` にしか当たらない）は対象がポートの ADR で状態が `採用` のままだと仮定している。R1 の正の対照 `next-adr` は日付が 2099 で一覧の最後に並ぶため、この経路を通らない。
  使い捨ての worktree で実測: (1) 形の正しい `202609140000-human-verify-scope.md` を足し README に 1 行足して `git add` → 本番は rc=0、`--self-test` は rc=1（`source-name`・`untracked` が「検査が落ちなかった」「期待する error を出さなかった」）。(2) README の「決定が後で変わったら状態を置き換え済みにする」に従いポートの ADR を `- 状態: 置き換え済み（→ 202611090000-port-v2.md）` に変える → 本番は rc=0、`--self-test` は rc=1（`status-value`・`status-extra`）。
  ci.yml の行は `--self-test && 本番` なので、正しい ADR を含む無関係の PR が赤になる。メモリには 2026-10-03 より前の判断（human-verify-scope 2026-09-14 など）が残っており、ADR にすれば先頭へ並ぶ。Q3 の答え・design D2「ADR を足しても消しても成り立つべき性質」に反する
- kind: technical
- 提案: (a) 対象を `202610030000-worktree-port-allocation.md` に名前で固定し、無ければ error にする。(b) `status-value` / `status-extra` の sed を `^- 状態: .*` に当たる形へ緩める（期待する error の文面は変えない）。(c) 正の対照を 2 つ足す —— 既存の ADR を置き換え済みにした木、どの ADR より古いファイル名の ADR を足した木で、どちらも本番が通ること。直した後、上の 2 つの再現で `--self-test` が rc=0 になることを確かめる
- 処置: fixed 1.1 （提案の (a)(b)(c) のとおり。自己検査の対象は `202610030000-worktree-port-allocation.md` に名前で固定し、無ければ `self-test の対象 … が無い` で落とす。`status-value` / `status-extra` は値に依らず最初の `- 状態:` の行を壊す。正の対照を `next-adr` に `superseded`（対象を置き換え済みにして後継を足す）・`older-adr`（どの ADR より古いファイル名の ADR を一覧の末尾に足す）を足した 3 つにした。根拠の 2 つの再現で `--self-test` が rc=0（report））
- 判定: ADDRESSED（re-review 2 @37d7af4976af2460d7b9578eed5b3554d60d344b）
- 判定: ADDRESSED（re-review 3 @c1eef3df96718f2286ecef74c5422f0f65f120e7）

## R19. 使い捨ての木の掃除が `trap … RETURN` だけで、中断（SIGINT・kill）では動かない
- 成果物: `tools/check-adr.sh`
- 根拠: `tools/check-adr.sh:313`・`:388`・`:599` の掃除は `trap … RETURN` のみ。中断した `--self-test` は mktemp の下に約 27 本の木の写しを残す。`load_index` の mktemp も同じ。tasks.md の Global Constraints「trap で必ず消す」に照らすと中断の経路が抜けている（Task 1 F9・F14 の残り）
- kind: technical
- 提案: 入口で一時ディレクトリを 1 つの親の下に集め、`trap … EXIT INT TERM` で消す
- 処置: fixed 1.1 （入口で `TMP_ROOT` を 1 つ作り、`load_index`・`backlog`・`verbatim`・`self_test` の一時ファイルは全部その下に置く。`trap … EXIT` で消し、INT・TERM は 130・143 で exit させて EXIT の trap を通す。`trap … RETURN` は消した。`--self-test` の process group に SIGINT・SIGTERM を送ると、残るファイルが 0（旧版は SIGINT で 286）（report））
- 判定: ADDRESSED（re-review 2 @37d7af4976af2460d7b9578eed5b3554d60d344b）
- 判定: ADDRESSED（re-review 3 @c1eef3df96718f2286ecef74c5422f0f65f120e7）

## R20. `section_lines` が見出しを正規表現として扱う
- 成果物: `tools/check-adr.sh`
- 根拠: `tools/check-adr.sh:39`。Task 2 の指摘 F4 が残っている。今の呼び出し元の 2 つの見出しにメタ文字は無いので実害は無い
- kind: technical
- 提案: 見出しを固定文字列で比べる（awk の `$0 == h` など）
- 処置: fixed 1.1 （`index($0, start) == 1` の固定文字列の前方一致にした。AGENTS.md の見出しは `## 実装と計画の規約（…）` と後ろに続くので、完全一致でなく前方一致。`## 規約(x)` を探す試しで、旧版は `## 規約x` の節を返し、新版は `## 規約(x)` の節を返す（report））
- 判定: ADDRESSED（re-review 2 @37d7af4976af2460d7b9578eed5b3554d60d344b）
- 判定: ADDRESSED（re-review 3 @c1eef3df96718f2286ecef74c5422f0f65f120e7）

## R21. ファイル名の日付と `- 日付:` の一致、`置き換え済み（→ X）` の X の実在を誰も見ていない（spec の隙間）
- 成果物: `openspec/changes/ch12-memory-decisions-adr/specs/decision-records/spec.md` / `tools/check-adr.sh`
- 根拠: `docs/adr/README.md` は「時刻は判断が決まった日」と書くが、ファイル名の日付と `- 日付:` の一致を検査が見ない。R18 の再現 (2) では存在しない後継 `202611090000-port-v2.md` を指したまま本番が rc=0 で通った。spec が求めていないので台本の漏れではなく spec の隙間
- kind: premise
- loss: rewrite-all —— 合わない日付・指す先の無い置き換えが CI を通り、ADR の履歴の辿りが切れる（後で検査を足すと、それまでに入った ADR を直すことになる）
- 提案: この change では直さず、別の change の候補にする
- 処置: escalated —— deep.md 第 2 回 Q5 で答えが出た。本人が推奨「この change で足す」を選んだ（2026-11-09、`deep-answers-2.txt`）ので、答えに沿って design D9 のとおり直した。spec decision-records の 1 つ目の Requirement に 2 文と Scenario 2 本（「ADR のファイル名の日付は ADR の日付に一致する」「置き換え済みの ADR が指す後継は docs/adr/ にある」）、`tools/check-adr.sh` に `check_filename_dates`・`check_successors` と自己検査の壊し方 `filename-date`・`successor-missing`（どちらも期待する error の文面まで見る）を足し、design D9 に検査と壊し方を書いた。再現と出力は `.superpowers/sdd/ch12-settle/report.md`

## final review（a658d12..38fba73）

whole-branch の final reviewer（2 回目。`superpowers:requesting-code-review` の code-reviewer）。R18〜R21 の直し（2d9391c..38fba73）を 1 行ずつ読んだ。引数なし・`--self-test`（`LC_ALL=C` でも）・`--verbatim`・`scripts/check_scenarios.py` は rc=0。R1〜R21 と重なる指摘も、処置の後の再発も無い。判定: Ready to merge（Minor 3 件）。

## R22. `check_successors` の防御の枝 2 つ（`README.md` と `/` を含む名前）を自己検査が見ていない
- 成果物: `tools/check-adr.sh`
- 根拠: `tools/check-adr.sh:184` は `[ "$successor" = README.md ] || [[ "$successor" == */* ]] || [ ! -f … ]` で落とす。design D9 も「`/` を含む名前と `README.md` は落とす」と書く。しかし自己検査の壊し方 `successor-missing`（:519・:638）は、存在しない名前 `209912310000-missing-next.md` しか試さない。前の 2 つの条件を消しても `--self-test` は緑のまま。あわせて、自分自身を後継に指す置き換えも今の検査を通る（spec では禁じていない）
- kind: technical
- 提案: 自己検査の一覧（:704）に壊し方を 2 つ足す。後継を `README.md` にするものと、`../README.md`（`docs/adr/` の外に実在するファイル）にするもの。期待する文面は今と同じ `置き換え済みの後継 … が` でよい
（確かさ 4）
- 処置: fixed 1.1 （自己検査に壊し方 `successor-readme`（後継を `README.md` に）と `successor-outside`（後継を `docs/adr/` の外に実在する `../../CLAUDE.md` に。使い捨ての木の根に写した CLAUDE.md を指す）を足し、期待する文面 `置き換え済みの後継 … が` まで見る。`check_successors` の `README.md` の条件を外すと `successor-readme` が、`*/*` の条件を外すと `successor-outside` が自己検査を rc=1 で落とすことを確かめた（report）。自分自身を後継に指す置き換えは spec が禁じていないので、この回では足さない）
- 判定: ADDRESSED（re-review 3 @c1eef3df96718f2286ecef74c5422f0f65f120e7）

## R23. tasks.md の Task 1.1 の本文が直しに追いついていない（数と印の一覧）
- 成果物: `openspec/changes/ch12-memory-decisions-adr/tasks.md`
- 根拠: Global Constraints（`tasks.md:14`）は「引数なし 13 本」に直っている。一方 Task 1.1（`tasks.md:23`）は「次の 11 を順に確かめ」のままで、印の一覧に R21 の 2 本（「ADR のファイル名の日付は ADR の日付に一致する」「置き換え済みの ADR が指す後継は docs/adr/ にある」）が無い。`tasks.md:35` も「D9 の 9 つの壊し方」のままで、台本と CH12.md の 28 と合わない。同じファイルの中で数が食い違う
- kind: technical
- 提案: 数と印の一覧を台本に合わせる（Task の当時の数を残すなら、そう注記する）
（確かさ 4）
- 処置: fixed 1.1 （Task 1.1 の数を「13（Task の当時は 11。第 2 回 Q5 で 2 本足した）」に直し、印の一覧に R21 の 2 本を足した。壊し方の数も「Task の当時は 9 つ。今は 30」と書いた。Global Constraints の「引数なし 13 本」と台本の印 13 本に合う）
- 判定: ADDRESSED（re-review 3 @c1eef3df96718f2286ecef74c5422f0f65f120e7）

## R24. design D9 の自己検査の説明が台本と合わない（壊し方の数・正の対照・掃除の作り）
- 成果物: `openspec/changes/ch12-memory-decisions-adr/design.md`
- 根拠: `design.md:165-171` に書いた壊し方は 9＋12＋2＝23 で、台本（`tools/check-adr.sh:704` の一覧）と CH12.md の 28 と合わない。抜けているのは `date-extra`・`date-form`・`readme-missing`・`adr-dir-missing`・`untracked-all` の 5 つ。正の対照も「形の正しい ADR を 1 件足す（R1）」だけで、R18 で足した `superseded`・`older-adr` が無い。「`trap` で木を消す」も、R19 の後の作り（`TMP_ROOT` を `trap … EXIT` で消す）と合わない
- kind: technical
- 提案: 抜けた 5 つの壊し方、正の対照の 3 つ、`TMP_ROOT` と `trap … EXIT` を D9 に書き足す
（確かさ 4）
- 処置: fixed D9 （抜けた 5 つ（`date-extra`・`date-form`・`readme-missing`・`adr-dir-missing`・`untracked-all`）と R22 の 2 つを足し、9＋12＋2＋5＋2＝30 で台本の一覧と同じと書いた。正の対照を `next-adr`・`superseded`・`older-adr` の 3 つにし、掃除を `TMP_ROOT` と `trap … EXIT`（INT・TERM は exit で EXIT の trap を通す）に直した。CH12.md の完了の判定の「28 の壊し方」も 30 に直した）
- 判定: ADDRESSED（re-review 3 @c1eef3df96718f2286ecef74c5422f0f65f120e7）
- 判定: ADDRESSED（re-review 5 @898546c2cfe79ca307e5975f1490e0b6bb3a5f58）

## settle_review（re-review 4 @9e70f08）: 処置の直しが新しく壊したもの

## R25. Risks の「ADR への参照に、行き先がまだ無い。PR 本文に書く」が古いまま残っている。ADR は main にあり、新しい PR 本文もこの件を書いていない。
- 成果物: openspec/changes/ch12-memory-decisions-adr/design.md
- 根拠: openspec/changes/ch12-memory-decisions-adr/design.md:183（re-review 4 の New Breakage。Minor）
- kind: technical
- 処置: fixed D3 （Risks の 2 行を今に合わせた。harness2 の ADR への参照の行は「上流の時点では行き先が無かった → 2026-11-08 に harness2 の main に入り（`1e140a4b3`）解けた。いまはあることを `docs/changes/CH12.md` の節の末尾に書いた」にし、「PR 本文に書く」を消した。食い違いの行の「PR 本文に一覧で挙げ」も R26 に合わせて CH12.md の節に直した）
- 判定: ADDRESSED（re-review 5 @898546c2cfe79ca307e5975f1490e0b6bb3a5f58）

## R26. 組み直した PR 本文は食い違い 8 件の一覧を落としている。design D3 の「PR 本文で挙げる」とは形が合わない。一覧は CH12.md の節にある。
- 成果物: docs/briefs/CH12-pr.md
- 根拠: docs/briefs/CH12-pr.md（re-review 4 の New Breakage。Minor）
- kind: technical
- 処置: rejected: PR 本文は成果物として直せない。`docs/briefs/CH12-pr.md` はグラフの `finish` が `scripts/pr_body.py` で毎回組み直し、前の本文から引き継ぐのは `## 仮決め（B）と Ruling` の節の `Ruling:` の行だけ（`scripts/pr_body.py` の `rulings` と `build`。`build` は一覧の節を持たない）。手で一覧を足しても、この settle の後の `finish` で消える（9e70f08 で 8 件の一覧が消えたのがその再現）。`scripts/` は harness2 への symlink で、この change では直さない。形の食い違いは design の側で解いた: D3（仮）に 2026-11-09 の追記を足し、一覧の置き場を `docs/changes/CH12.md` の `## ADR と repo の今の食い違い（直していない）` の節だけにした（PR の差分に入るので merge の前に人間が読める。Q1 の意図「直さずに挙げ、後で change を起こせる」は変わらない）。Risks の行と CH12.md の完了の判定の文も合わせた
- 判定: ADDRESSED（re-review 5 @898546c2cfe79ca307e5975f1490e0b6bb3a5f58）

## settle_review（re-review 5 @898546c）: 処置の直しが新しく壊したもの

## R27. D3 の本文と見出しが「PR 本文に一覧で挙げる」と元のまま残り、同じ節の 2026-11-09 の追記（一覧の置き場は CH12.md の節だけ）と食い違う。追記を読めば分かるが、本文と見出しが古い。
- 成果物: openspec/changes/ch12-memory-decisions-adr/design.md
- 根拠: openspec/changes/ch12-memory-decisions-adr/design.md:77（re-review 5 の New Breakage。Minor）
- kind: technical
