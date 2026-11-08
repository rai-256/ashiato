<!-- 処置の判定: re-review -->

# CH12 code_verify — 独立の検証（2026-11-09、HEAD e90c75431・origin/main 0f38ad74e）

前の回（cb2bdb079 の時点）の後、コードは変わっていない（`git diff cb2bdb079 HEAD --stat` は `review/code.md` だけ）。
前の回の指摘は引き継がず、全部を走らせ直した。再現した R1〜R5 は同じ番号のまま根拠を今回の実測に書き直し、新しく見つけたものを R6 以降に足した。

## 申告と実測

申告: tasks の `[x]` は 5/5（1.1・2.1・2.2・3.1・4.1）。検証コマンドは下の 11 本。

| タスク | 検証コマンド | 申告 | 実測 |
|---|---|---|---|
| 1.1 | `bash -n tools/check-adr.sh` | rc=0 | rc=0 一致 |
| 2.1 | `tools/check-adr.sh --verbatim` | rc=0 | rc=0 一致 |
| 2.2 | `tools/check-adr.sh` | rc=0 | rc=0 一致（Scenario の印 11 行・`OK 網の名前・私設 IP は入っていない（2264 ファイル）`） |
| 2.2 | `tools/check-adr.sh --self-test` | rc=0 | rc=0 一致（印 2 行） |
| 2.2 | `tools/check-private.sh` | rc=0 | rc=0 一致 |
| 2.2 | `CLAUDE.md` `AGENTS.md` `ci.yml` の numstat が `1\t0` | rc=0 | rc=0 一致（3 つとも `+` 1 行だけ。内容は design D2・D7 の文面どおり） |
| 3.1 | `tools/check-adr.sh --backlog` | rc=0 | rc=0 一致（4 つとも `残りに無い`・`OK backlog`。台本は `.harness2/scripts/adr_backlog.py` で見つかった） |
| 3.1 | `tools/check-adr.sh --self-test` | rc=0 | rc=0 一致 |
| 4.1 | main からの差分の範囲 | rc=0 | rc=0 一致 |
| 4.1 | `python3 scripts/check_scenarios.py . ch12-memory-decisions-adr` | rc=0 | rc=0 一致（`scenarios: OK`。warn 4 行はどれも他の change の印） |
| — | `python3 scripts/plan_fix.py pending openspec/changes/ch12-memory-decisions-adr` | — | `[]` |

`[x]` の検証は全部実在し、rc=0 になった。指摘は、検証が**見ていないところ**から出た: ADR を足した後・検査そのものが壊れた場合・形の違う行・メモリを消した後。

## 手 1: 固定値を独立に再計算する

`--verbatim` の定数 `7e1b0a5c…2e58b94` を、台本の awk を使わずに計算し直した。python で行を `split('\n')` に分け、`## 依頼（逐語）` から次の `## ` の手前までを取り、各行に改行を付けて `hashlib.sha256` にかけた。
いまの `docs/changes/CH12.md`・登録の commit `ff9a78dd7`（design D1 が名指すもの）・rebase 後の登録の commit `b77e52a51`（origin/main で CH12.md を最初に足した commit）の 3 つとも、
243 行・`7e1b0a5c755c8b19fcb7c138db6f2f87897d96baa46f90b3bc889ba872e58b94` になり、一致した。
6 つの塊も、正規表現を使わずに `=== ファイル: … ===` の行で区切る別の実装で取り出し、`docs/adr/` の 6 ファイルとバイト単位で比べた。6/6 一致。
`--verbatim` を壊すと落ちる: ADR の 1 語を変えると rc=1（`15 行目が依頼（逐語）と一致しない`）、元の節の 1 字を変えると rc=1（`sha256 が違う`）、ADR と元の節を一緒に変えても rc=1（`sha256 が違う`）。

## 手 2: ガードをわざと壊す

使い捨ての clone（`git clone --no-local`）と `ADR_DIR` の写しで壊した。

| 壊したもの | 結果 |
|---|---|
| ADR の写しに私設 IP（`<192.168 の範囲の 1 つ>`）を足して `git add` | rc=1（check-private が `NG docs/adr/…:32` を出す） |
| `tools/check-private.allow` に `docs/adr/README.md` を足す | rc=1（`error: tools/check-private.allow に docs/adr/ の例外がある`） |
| ADR の 1 件で `## 背景` と `## 決定` を入れ替える | rc=1（`見出しの順序が違う`） |
| ADR の 1 件の状態を `- 状態: 検討中` にする | rc=1（`状態の行が 0 個`） |
| ADR の 1 件の `- 日付:` だけを変える | rc=1（`README の日付と ADR の日付が違う`） |
| 一覧の項目と ADR の名前を両方大文字にそろえる | rc=1（`ファイル名が時刻と英小文字 slug の形でない`） |
| 衛星の ADR の出所から `foundation-for-satellites-premise.md` を消す | rc=1（`出所に foundation-for-satellites-premise.md が無い`） |
| CI の runner を模す（`.harness2` 無し・`HOME` は空） | `--self-test` rc=0・本番 rc=0・`--backlog` rc=127（`error: adr_backlog.py の台本が無い`）。空の合格にはならない |
| `awk` を mawk 1.3.4（20260129）にする | `--self-test` rc=0・本番 rc=0。一覧の行を 1 つ消すと rc=1（`{4}` の区間の正規表現も動く。runner の mawk の版とは違う） |
| `LC_ALL=C LANG=C` で走らせる | `--self-test` rc=0・本番 rc=0。`- 状態: 検討中` にすると rc=1（`状態の行が 0 個`） |
| **形の正しい 7 件目の ADR を足し、一覧にも足して `git add`** | **rc=1（`error: docs/adr/ がまだ追跡されていない` ×2）→ R1** |
| **検査の台本の行を 1 本ずつ `:` に潰す（19 本）** | **10 本は `--self-test` が rc=0 のまま → R2** |
| **`- 状態: 検討中` を 2 行目として足す・一覧に形の違う項目を足す** | **rc=0 → R6** |
| **`- 出所:` の行を `## 影響` の後へ移す** | **rc=0 → R7** |

一時の場所: 4 つの口を `TMPDIR` を空のフォルダにして走らせ、後に残ったものは 0 件だった。

## 手 3: Scenario と test を突き合わせる

`check_scenarios.py` は OK（13 本とも `tools/check-adr.sh` に印がある）。印の先の観測を 1 本ずつ見た。

- 「1 対 1 で対応する」: 一覧の項目のうち `- [YYYY-MM-DD ` の形のものしか数えない（:52）。形の違う項目は一覧に「ある」のに見えない（R6）
- 「この順に持つ」: 見出し 5 つの順と、項目 3 行の**値の形が正しいもの**の数しか見ていない。`- 状態:` の行の数・項目 3 行の位置は見ていない（R6・R7）
- 「CH12 で置いた README と 5 件の ADR がある」「網の名前の検査が通る」: `git ls-files docs/adr` が**ちょうど 6 行**であることを見る。spec の「がある」「返し」より狭い（R1）
- 「5 件の ADR の出所に、移したメモリのファイル名がある」: 観測は正しい（手 2 で消すと落ちる）。ただし自己検査はこの検査を守っていない（R2）
- そのほか（日付と題・並び・ファイル名の形・`CLAUDE.md`・`AGENTS.md`・CI の行）: 印の先の検査が主張を観測している。日付・ファイル名の形の検査が消えても自己検査が気づかないことは R2 に書いた

## 手 4: 本人の決定が test で固定されているか

| 決定 | 書き換えたときに落ちるもの |
|---|---|
| Q1 逐語のまま置く | `--verbatim`（手 1 で 1 語変えると rc=1）。CI には載らない（D1 のとおり） |
| Q2 repo の中のコマンドで、4 つの名前で判定する | `--backlog`（出所から 1 つ消すと rc=1。ただし出力の言い方は R3） |
| Q3 chain job の 1 行 | `check_ci` の `grep -Fx`（:135）。自己検査の `ci` の壊し方で、:136 を潰すと `--self-test` rc=1 |
| Q4 `AGENTS.md` の 1 行 | `check_guidance`（:124）。自己検査の `agents` の壊し方で、:125 を潰すと `--self-test` rc=1 |

値を変えても全部通る決定は無かった。ただし Q3 の答えの意図「後から足す ADR も同じ形に保たれる」（CH12.md の価値）は、R1 で、次の ADR を足す PR を誤った理由で止める形になっている。

## 手 5: tasks の `[x]` と実体

5 件とも、検証のコマンドが実在して rc=0 になった（上の表）。`cargo test --test` 型の検証は無い。
コマンドで確かめていない主張は 4.1 の「`docs/changes/CH12.md` の食い違いの節が最新」の 1 つだけで、これは事実でなくなっている（R5）。
4.1 の範囲の検査は `.superpowers/` を許しており、差分には `.superpowers/sdd/ch12-task-{2,4}/` の 2 つがある。CH12.md の完了の判定 8 の並びには無いが、ハーネスの成果物で、main にも他の change のものが 18 ファイルあるので、指摘にしない。

## 手 6: 隙間

この change は文書と検査の台本だけで、実行時に記録を持たない（捨てる経路は無い）。隙間は 3 つの状態で出た: 後で ADR を足す（R1）、メモリを消す（R4）、形の違う行がある（R6・R7）。
`置き換え済み（→ 存在しないファイル名）` の ADR も rc=0 で通る。spec は形しか求めていないので R にはしないが、README は「ADR は消さず、置き換え済みにして新しい ADR を足す」と決めており、後継の指す先が無いと辿れなくなる。
ADR の食い違いの一覧（CH12.md の節）のうち 2 つを、いまの repo で確かめ直した。どちらも健在だった: 5（`crates/winsvc/Cargo.toml` に `lints` の行が無い）、8（`GIT_DIR=/nonexistent` で `tools/ports.sh` を source すると `ASHIATO_DB_PORT=55432`・rc=0）。
defer: 該当なし（確かめた範囲: R1〜R7 はどれもこの change の `tools/check-adr.sh` と文書の中で閉じ、他の Story の持ち物に掛からない）。

## 手 7: 該当なし（`plan_fix.py pending` が `[]`）

---

## R1. 形の正しい ADR を 1 件足すと、CI の検査が「まだ追跡されていない」で落ちる
- 成果物: `tools/check-adr.sh` / `.github/workflows/ci.yml`（chain job）
- 根拠: 使い捨ての clone に `docs/adr/202611080000-sample-next.md` を足した（README の「形」どおり。日付・状態・出所と 5 つの見出し）。README の一覧の先頭に `- [2026-11-08 次の判断の見本](202611080000-sample-next.md)` も足した。
  未追跡のままでは `ADR_SELF_TEST_CHILD=1 tools/check-adr.sh` が rc=0。`git add docs/adr` の後は rc=1 で `error: docs/adr/ がまだ追跡されていない` が 2 回出た。
  `tools/check-adr.sh --self-test` も rc=1（`error: self-test の基準の木で本番検査が通らない`）。
  原因は `tools/check-adr.sh:151` の `[ "$count" -eq 6 ]`。追跡されているファイルが 6 つ**ちょうど**でなければ落ちる。`check_tracked` は印「CH12 で置いた README と 5 件の ADR がある」（:306）と「網の名前の検査が通る」（:316）の 2 回呼ばれる。
  Q3 の本人の答えの意図は「後から足す ADR も同じ形に保たれる」（CH12.md の価値）、design D2 は「ADR を足しても消しても成り立つべき性質」。いまの検査は、次に ADR を足す最初の PR を CI で止め、しかも誤った理由（未追跡）を出す。
  `check_tracked` は `ADR_DIR` を見ずに実の repo の索引を見るので、自己検査ではこの経路を試せない
- kind: technical
- 提案: 「6 つちょうど」を「6 つの名前がすべて `git ls-files docs/adr` にある」に変える（:152-154 で 1 つずつの確かめはもうしている）。自己検査に「形の正しい ADR を 1 件足すと**通る**」場合を足す

## R2. CI が走らせる自己検査は、10 本の検査の行が消えても緑のまま
- 成果物: `tools/check-adr.sh`（`self_test`）
- 根拠: 使い捨ての clone で、検査の台本の行を 1 本ずつ `:` に置き換えて `tools/check-adr.sh --self-test` を走らせた（19 本）。
  緑のまま（rc=0）だったのは 10 本: :60（件数の比較）・:80（一覧と ADR の日付）・:88（ファイル名の形）・:100（1 行目の題）・:101（日付の行）・:102（状態の行）・:111（見出しの順序）・:151 と :153（追跡）・:170（出所のメモリ名）・:318（allow の例外）。
  rc=1 で気づいたのは :67・:79・:85・:103・:107・:121・:125・:136。
  :60 は :67 が代わりに拾うので実害は無い。残りの 9 本はどれも本物の違反を捕まえる行で、元の台本なら落ちることを手 2 で確かめた（日付・形・状態・順序・出所のメモリ名・allow）。
  特に :170 は Scenario「5 件の ADR の出所に、移したメモリのファイル名がある」の唯一の検査だが、自己検査はどの壊し方でも通らない（`source` の壊し方は出所の行ごと消すので :103 で落ちる）。
  :151・:153・:318 は `ADR_DIR` でなく実の repo（`git ls-files`・`tools/check-private.allow`）を読むので、自己検査の使い捨ての木からは届かない。R1 が自己検査に見えないのと同じ理由。
  design D2 は「自己検査を外すと、台本が壊れて何も落とさなくなっても CI は緑のまま」を理由に、自己検査を 1 行に入れている。design D9 の 9 つの壊し方は全部実装されているが、その 9 つでは上の行を守れない
- kind: technical
- 提案: 自己検査に次の壊し方を足す: 一覧と ADR の両方の名前を大文字にそろえる・ADR の `- 日付:` だけを変える・2 つの見出しを入れ替える・`- 状態: 検討中` にする・1 行目の `# ` を消す・出所の行は残してメモリ名だけを消す。どれも期待する `error:` の文面まで見る。
  `check_tracked` と allow の検査は、見る場所を環境変数で差し替えられるようにして、自己検査から届くようにする

## R3. `--backlog` は、残りに戻ったメモリが 1 つだけでも 4 つとも「残りにある」と出す
- 成果物: `tools/check-adr.sh`（`backlog` :274-285・`backlog_json_check` :225-247）
- 根拠: `ADR_DIR` の写しで、衛星の ADR の出所から `foundation-for-satellites-premise.md` だけを消し、`ADR_DIR=<写し> tools/check-adr.sh --backlog` を走らせた。rc=1 で、落ちること自体は正しい。
  しかし出力は 4 行とも `…: 残りにある` で、最後に `NG backlog` が出た。ほかの 3 つは出所に名指しされたままで、残りには無い。
  `backlog_json_check` は「4 つのどれかが在る」を 1 つの rc にまとめ（:245）、:279-285 はその rc で 4 行を同じ文面にしている。D5 は 1 行ごとに `<メモリ名>: 残りに無い|ある` と決めている。
  判定（rc）は正しいが、落ちたときに、どれを直せばよいかを出力が誤って言う
- kind: technical
- 提案: python の側で名前ごとの有無を返し（例: 残っている名前を 1 行ずつ出す）、名前ごとに `残りに無い` / `残りにある` を出す

## R4. メモリを消すと、手元の `--self-test` が rc=1 になる（CI は緑のまま）
- 成果物: `tools/check-adr.sh`（`self_test` :469-480・`backlog` :262-265）
- 根拠: 空のフォルダをメモリの置き場にして（4 つのメモリを消した後を模す）`ADR_MEMORY=<空> tools/check-adr.sh --self-test` を走らせた。rc=1 で、出力は次の 2 行。
  `error: self-test の --backlog 子が予期せず失敗した: 1` と `error: self-test の --backlog 子が公開してはいけない行を出した: error: --adr の空の正の対照で 4 つのメモリを数えられない`。
  同じ条件で `--backlog` だけを走らせると rc=1。自己検査の (d) は、子の `--backlog` を**手元の本物の**台本とメモリで走らせ、rc が 0 か 127 でなければ落とす。正の対照が落ちると rc=1 なので、自己検査が赤になる。
  さらにその `error:` の行は `無い` で終わらないので、台本自身の D5 の形の検査（:476 `^error: .*無い$`）にも当たらない。
  CH12 の目的は「メモリは失われうるので、正本を repo に移す」。移した後でメモリを消すのは自然な次の操作で、そのとき手元の `--self-test`（tasks 2.2・3.1 の検証で、verify-run で走る）が、台本の壊れでなく環境の事情で赤になる。
  verify-run は rc=1 を、環境の欠落（BLOCKED_INFRA）でなく FAIL と読む。CI の runner は台本が無く rc=127 の枝を通るので、CI では見えない
- kind: technical
- 提案: 自己検査の (d) は、子を偽の台本と使い捨ての置き場に向けて走らせ、手元のメモリに依らせない。正の対照が落ちたときの文面を D5 の形に合わせる（または形の正規表現を、台本が出しうる `error:` の文面に合わせる）

## R5. 「harness2 の ADR は、まだ harness2 の main に無い」は、もう事実でない（CH12.md の節・PR 本文・design）
- 成果物: `docs/changes/CH12.md:288`・`:303` / `docs/briefs/CH12-pr.md:41` / `design.md:76`・`:146`（Context の「`adr_backlog.py` は `.harness2` に無い」も同じ）
- 根拠: `git -C "$(readlink -f .harness2)" ls-tree --name-only origin/main docs/adr/` で 28 行が出た。`scripts/adr_backlog.py` も origin/main にある（足した commit は `ae1a0b77f`、2026-11-08 13:06 +0900）。
  ADR が題名で参照する 2 件も origin/main にある: `docs/adr/202610030000-collision-free-allocation.md` と `docs/adr/202610170001-change-flow.md`。
  手 2 の表の `--backlog` も、D4 の探し方の 2.（`.harness2/scripts/adr_backlog.py`）で台本を見つけている。
  それでも CH12.md:303 は「2026-11-08 の時点で harness2 の main に無い（push も PR も無い）」と書き、PR 本文:41 は「まだ harness2 の main に無い」と書く。
  Task 4.1 はこの節が「最新」であることを `[x]` にしている。ADR の本文（逐語）は変えなくてよいが、食い違いの一覧が、実在しない食い違いを 1 つ挙げている
- kind: daily
- 提案: CH12.md の節の末尾の段落と、PR 本文の同じ段落を「harness2 の main に入った（`ae1a0b77f` など）。参照の 2 件は行き先がある」に直す。この節は `## 依頼（逐語）` の外なので、`--verbatim` の sha256 は変わらない

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

## R7. 日付・状態・出所の 3 行の位置を見ていない
- 成果物: `tools/check-adr.sh`（`check_shape` :94-115）
- 根拠: `ADR_DIR` の写しで、`202610110001-ui-surface-only.md` を 3 通りに変えた。どれも rc=0。
  `- 出所:` の行をファイルの末尾（`## 影響` の後）へ移す / `- 日付:` の行を末尾へ移す / `- 状態:` と `- 出所:` の行を入れ替える。
  :111 の順序の検査は 5 つの見出しだけを見る。spec の Scenario「ADR は決まった項目と見出しをこの順に持つ」は「`- 日付:`・`- 状態:`・`- 出所:` の行が…、見出しが…、この順にある」。
  Requirement は「README の『形』のとおり」で、README の形では 3 行が 1 行目の題の直後、`## 背景` の前にある。
  主張（項目と見出しの並び）と検査（見出しだけの並び）の階層が違う
- kind: technical
- 提案: 3 行が `日付 → 状態 → 出所` の順で、どれも `## 背景` より前にあることを確かめる。自己検査に「出所の行を `## 影響` の後へ移す」壊し方を足す
