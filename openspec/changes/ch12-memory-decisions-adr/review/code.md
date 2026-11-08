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
- 根拠: 使い捨ての clone に `docs/adr/202611080000-sample-next.md`（README の形どおり。日付・状態・出所と 5 つの見出し）を足し、一覧の先頭に `- [2026-11-08 次の判断の見本](202611080000-sample-next.md)` を足した。
  未追跡のままでは `ADR_SELF_TEST_CHILD=1 tools/check-adr.sh` が rc=0。`git add docs/adr` の後は rc=1 で `error: docs/adr/ がまだ追跡されていない` が 2 回。`--self-test` も rc=1（`self-test の基準の木で本番検査が通らない`）。
  原因は `tools/check-adr.sh:151` の `[ "$count" -eq 6 ]`（追跡が 6 つ**ちょうど**でなければ落ちる）。`check_tracked` は :306 と :316 の 2 回呼ばれる。
  design D2 は本番の口が見るものを「ADR を足しても消しても成り立つべき性質」と書き、Q3 の意図は後から足す ADR を同じ形に保つこと。いまの検査は次に ADR を足す最初の PR を CI で止め、誤った理由（未追跡）を出す。
  `check_tracked` は `ADR_DIR` を見ずに実の repo の索引を見るので、自己検査ではこの経路を試せない
- kind: technical
- 提案: 「6 つちょうど」を「6 つの名前がすべて `git ls-files docs/adr` にある」に変える（:152-154 で 1 つずつの確かめはもうある）。自己検査に「形の正しい ADR を 1 件足すと**通る**」場合を足す

## R2. CI が走らせる自己検査は、8 本の検査の行が消えても緑のまま
- 成果物: `tools/check-adr.sh`（`self_test`）
- 根拠: 使い捨ての clone で、検査の行を 1 本ずつ `:` に置き換えて `tools/check-adr.sh --self-test` を走らせた（19 本）。
  rc=0 のままだったのは 11 本: :60・:80・:88・:100・:101・:102・:111・:151・:153・:170・:318。
  このうち 3 本は、本物の違反をほかの行が代わりに拾う: :60（件数）は :67、:100（1 行目の `# `）は :79（手 2 で 1 行目の `# ` を消すと `README の題と見出しが違う` で落ちた）、:101（日付の行の形）は :80。
  残りの 8 本（:80・:88・:102・:111・:151・:153・:170・:318）は、代わりに拾う行が無い。
  rc=1 で気づいたのは :67・:79・:85・:103・:107・:121・:125・:136。
  この 8 本のうち :80・:88・:111・:170・:318 は、元の台本なら本物の違反で落ちることを手 2 で確かめた（日付・ファイル名の形・見出しの順序・出所のメモリ名・allow）。:102 も、`ADR_DIR` の写しで状態を `- 状態: 検討中` に変えると rc=1（`状態の行が 0 個`）で落ちた。
  特に :170 は Scenario「5 件の ADR の出所に、移したメモリのファイル名がある」の唯一の検査だが、自己検査のどの壊し方も守らない（`source` は出所の行ごと消すので :103 で落ちる）。
  :151・:153・:318 は `ADR_DIR` でなく実の repo（`git ls-files`・`tools/check-private.allow`）を読むので、自己検査の木から届かない。
  design D2 は「自己検査を外すと、台本が壊れて何も落とさなくなっても CI は緑のまま」を 1 行に入れる理由にしているが、D9 の 9 つの壊し方ではこれらの行を守れない
- kind: technical
- 提案: 壊し方を足す: 名前を大文字にそろえる・`- 日付:` だけ変える・見出し 2 つを入れ替える・`- 状態: 検討中` にする・1 行目の `# ` を消す・出所の行は残してメモリ名だけ消す（どれも期待する `error:` の文面まで見る）。
  `check_tracked` と allow の検査は、見る場所を環境変数で差し替えられるようにして自己検査から届かせる

## R3. `--backlog` は、残りに戻ったメモリが 1 つだけでも 4 つとも「残りにある」と出す
- 成果物: `tools/check-adr.sh`（`backlog` :279-285・`backlog_json_check` :225-247）
- 根拠: clone で衛星の ADR の出所から `foundation-for-satellites-premise.md` だけを消し、`ADR_MEMORY=<手元のメモリの置き場> tools/check-adr.sh --backlog` を走らせた。rc=1（落ちること自体は正しい）。
  出力は 4 行とも `…: 残りにある`、最後に `NG backlog`。ほかの 3 つは出所に名指しされたままで残りには無い。
  `backlog_json_check` は「4 つのどれかが在る」を 1 つの rc にまとめ（:245）、:279-285 はその rc で 4 行を同じ文面にしている。D5 は名前ごとに `残りに無い|ある` と決めている。どれを直すべきかを出力が誤って言う
- kind: technical
- 提案: python の側で名前ごとの有無を返し、名前ごとに `残りに無い` / `残りにある` を出す

## R4. メモリを消すと、手元の `--self-test` が rc=1 になる（CI は緑のまま）
- 成果物: `tools/check-adr.sh`（`self_test` :469-480・`backlog` :262-265）
- 根拠: 空のフォルダを置き場にして（4 つのメモリを消した後を模す）`ADR_MEMORY=<空> tools/check-adr.sh --self-test` を走らせた。rc=1、出力は
  `error: self-test の --backlog 子が予期せず失敗した: 1` と `error: self-test の --backlog 子が公開してはいけない行を出した: error: --adr の空の正の対照で 4 つのメモリを数えられない`。
  自己検査の (d) は子の `--backlog` を**手元の本物の**台本とメモリで走らせ、rc が 0 か 127 でなければ落とす。正の対照が落ちると rc=1 になる。その `error:` の行は `無い` で終わらず、台本自身の D5 の形（:476 `^error: .*無い$`）にも当たらない。
  CH12 の目的は「メモリは失われうるので正本を repo に移す」で、移した後にメモリを消すのは自然な次の操作。そのとき tasks 2.2・3.1 の検証（verify-run は rc=1 を FAIL と読む）が、台本の壊れでなく環境の事情で赤になる。CI の runner は rc=127 の枝を通るので見えない
- kind: technical
- 提案: (d) の子を偽の台本と使い捨ての置き場に向け、手元のメモリに依らせない。正の対照が落ちたときの文面を D5 の形に合わせる

## R5. 「harness2 の ADR は、まだ harness2 の main に無い」は、もう事実でない（CH12.md の節・PR 本文・design）
- 成果物: `docs/changes/CH12.md:288`・`:303` / `docs/briefs/CH12-pr.md:41` / `design.md:13`・`:76`・`:146`
- 根拠: `git -C "$(readlink -f .harness2)" ls-tree --name-only origin/main docs/adr/` が 28 行。`scripts/adr_backlog.py` も origin/main にある（`ae1a0b77f`、2026-11-08 13:06 +0900）。
  ADR が題名で参照する `docs/adr/202610030000-collision-free-allocation.md` と `docs/adr/202610170001-change-flow.md` も origin/main にある。
  `--backlog` も D4 の探し方の 2.（`.harness2/scripts/adr_backlog.py`）で台本を見つけている。
  それでも CH12.md:303 は「2026-11-08 の時点で harness2 の main に無い（push も PR も無い）」、PR 本文:41 は「まだ harness2 の main に無い」と書く。Task 4.1 はこの節が「最新」であることを `[x]` にしている
- kind: daily
- 提案: CH12.md の節の末尾の段落と PR 本文の同じ段落を「harness2 の main に入った（`ae1a0b77f` など）。参照の 2 件は行き先がある」に直す。節は `## 依頼（逐語）` の外なので `--verbatim` の sha256 は変わらない

## R6. 決まった形に当たる行しか数えないので、形の違う行は落ちずに見えなくなる
- 成果物: `tools/check-adr.sh:52`（一覧の項目）・`:102`（状態の行）
- 根拠: clone で試した。(a) `202610110001-ui-surface-only.md` の `- 状態: 採用` の次に `- 状態: 検討中` を足すと rc=0（:102 は `採用|置き換え済み（→ …）` に当たる行だけを数えて「1 個」）。
  Scenario「ADR は決まった項目と見出しをこの順に持つ」は「`- 状態:` の行がちょうど 1 つ」。
  (b) README の一覧に `- [2026-1-08 x](202611080000-x.md)`（日付の桁が違い、指すファイルも無い）を足すと rc=0（:52 は `^- \[[0-9]{4}-[0-9]{2}-[0-9]{2} ` に当たる行しか拾わない）。
  Requirement の「項目は `- [<日付> <題>](<ファイル名>)` の形」と Scenario の「一覧にあって実ファイルが無いものが 0 件」のどちらにも反するのに通る
- kind: technical
- 提案: 先に `^- 状態:` と一覧の節の `^- ` で数えてから形を確かめ、形の違う行をファイル名と一緒に `error:` で出す。自己検査に (a)(b) を足す

## R7. 日付・状態・出所の 3 行の位置を見ていない
- 成果物: `tools/check-adr.sh`（`check_shape` :94-115）
- 根拠: clone で `202610110001-ui-surface-only.md` の `- 出所:` の行を末尾（`## 影響` の後）へ移すと rc=0。:111 の順序の検査は 5 つの見出しだけを見る。
  Scenario「ADR は決まった項目と見出しをこの順に持つ」は「`- 日付:`・`- 状態:`・`- 出所:` の行が…、見出しが…、この順にある」、Requirement は「README の『形』のとおり」（3 行は題の直後、`## 背景` の前）。
  主張（項目と見出しの並び）と検査（見出しだけの並び）の階層が違う
- kind: technical
- 提案: 3 行が `日付 → 状態 → 出所` の順で、どれも `## 背景` より前にあることを確かめる。自己検査に「出所の行を `## 影響` の後へ移す」壊し方を足す

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
