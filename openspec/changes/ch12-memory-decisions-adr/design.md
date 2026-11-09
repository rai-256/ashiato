# CH12 design — メモリにしか無かった判断を ADR に置く

> 第 1 回の深掘りの推奨の答え（Q1〜Q4）と、聞かずに決めた既定（`premises.md` の C1〜C4。ここでは D1・D2・D5 と D6、印「C」）で先に書いた。
> 本人の答えが推奨と違えば、その Q の D（仮）を反転条件のとおりに書き直す。
> 第 1 回の答え（`deep-answers-1.txt`、2026-11-08）: Q1・Q2 は推奨のまま、Q3（検査を常設する）・Q4（`AGENTS.md` にも足す）は推奨と違う。D2・D7 をその答えで書き直した（`spec --answers Q3,Q4`）。

## Context

動機は `proposal.md` の Why。いまの形（`premises.md` の表。2026-11-08 の repo、`origin/main` = 0adef6712）:

- `docs/adr/` は無い。ADR の本文 6 つは `docs/changes/CH12.md` の「依頼（逐語）」に `> ` 付きの引用で置いてある（この段で上半分を書き直したので、素の写しは無くなった）
- 出所に挙げた 4 つのメモリは手元の `~/.claude/projects/-home-yosis-dev-ashiato2/memory/` に実在する。CI の runner には無い
- harness2 の `scripts/adr_backlog.py` は、この repo の `.harness2`（harness2 の main の checkout）には無い。harness2 の手元の worktree の 1 つ（枝 `docs/adr-backfill`）にだけある。
  （2026-11-09 追記: その後 harness2 の main に入り（`ae1a0b77f`。ADR は `1e140a4b3`）、`.harness2/scripts/adr_backlog.py` で見つかる。`review/code.md` R5）
  この台本はメモリの置き場を cwd のパスから決め、無ければ rc=0 で終わる（空の合格。`review/deep.md` R1）。`--memory DIR`・`--adr DIR`（重ねてよい）・`--json` を受ける
- `tools/check-private.sh` は追跡ファイル全部を見る（CI の chain job と pre-commit）。`tools/check-private.allow` で意図した例外を外せる
- `CLAUDE.md` の「文書とハーネス」の表は `docs/`・`openspec/`・`.harness2`・symlink・`AGENTS.md` の 5 行

## Goals / Non-Goals

**Goals:**
- 依頼の 6 ファイルを逐語のまま `docs/adr/` に置き、`CLAUDE.md` の表から辿れるようにする（spec `decision-records` の 1 つ目の Requirement）
- 移した 4 つのメモリが ADR の出所に名指しされ、harness2 の数える台本の残りから外れることを確かめる（2 つ目）
- 網の名前・私設 IP を含めない（3 つ目）
- ADR の形（一覧と実ファイルの一致・ファイル名の形・見出しの形）を CI の chain job で毎回見る（Q3 の本人の答え。D2（仮））
- agent が ADR を書く場所を知る: `CLAUDE.md` の表の 1 行と、`AGENTS.md` の 1 行の指示（Q4 の本人の答え。D7（仮））

**Non-Goals:**
- ADR の本文の手入れ（食い違いの直し・「その後」の追記）（Q1。D3（仮））
- harness2 の ADR 24 件と `adr_backlog.py` を harness2 の main に入れること（この repo の外。Q2）
- ハーネスのメモリ（19 件）を ADR にすること（harness2 の側の仕事）

## Decisions

### D1（C1）. 6 ファイルは依頼の逐語のまま置き、逐語との一致を 1 回だけ機械で見る

比べる元は `docs/changes/CH12.md` の `## 依頼（逐語）` の節（次の `## ` 見出しの手前まで）。各行の行頭の `> ` を外し、`>` だけの行は空行にする。
塊は `=== ファイル: <名前> ===` の行の次の行から、次の `=== ファイル:` の行の手前か節の終わりまで。塊の先頭と末尾の空行を落とし、末尾の改行を 1 つにそろえたものが、
`docs/adr/<名前>` の中身（末尾の改行を 1 つにそろえたもの）とバイト単位で一致すること。塊は 6 つで、名前も 6 つちょうど。

この一致は `tools/check-adr.sh --verbatim` が見る。**spec の Scenario にしない**（正典に置かない）: README 自身が「決定が後で変わったら、状態を置き換え済みにして新しい ADR を足す」と
書いており、後の ADR の手入れで本文は変わってよい。逐語の一致は CH12 の受け入れの 1 回の判定で、常に成り立つ性質ではない。
`--verbatim` は CH12 の archive の後に走らせると、手入れの後では落ちうる（台本の冒頭のコメントにそう書く）。

**比べる元そのものも固定する**（`review/spec.md` R9）: `--verbatim` は先に、いまの `docs/changes/CH12.md` の節の sha256 が、依頼を登録したときの節の値
`7e1b0a5c755c8b19fcb7c138db6f2f87897d96baa46f90b3bc889ba872e58b94` と一致することを見る。違えば `error:` で落とす。値は台本に定数で書く。
節の取り出し方: `## 依頼（逐語）` の行から、次の `## ` で始まる行の手前までの行（末尾の空行も含む）を、各行の後に改行 1 つを付けて並べたバイト列（243 行）。
上流が登録の commit `ff9a78dd7` の節と、いまの節の両方で同じ値になることを確かめた。commit の SHA でなく中身の値にするのは、枝が rebase されると SHA が履歴から消えうるから。
こうしないと、ADR と節を一緒に書き換えたときにどの検証も落ちない。

### D2（仮）. 足すのは 6 ファイル・`CLAUDE.md` と `AGENTS.md` の 1 行ずつ・ADR の形の検査 `tools/check-adr.sh`・CI の chain job の 1 行（Q3。既定 C2 を改めた）

本人の答え（Q3。推奨と違う）: 「足す（`tools/check-adr.sh` と CI の chain job の 1 行）」。README の一覧と `docs/adr/` の実ファイルの一致・ファイル名の形・見出しの形を、
この change の 1 回の検証で終わらせず、CI で毎回見る。推奨（足さない・この change の検証で 1 回だけ見る）は採らない。

C2 は「足すのは 6 ファイルと `CLAUDE.md` の 1 行だけ」だった（`premises.md` の C2 に、この段で改めたことを書き足した。`review/spec.md` R11）。spec の Scenario は印をコードの拡張子のファイルからしか拾わない（`scripts/check_scenarios.py`）ので、
印を持つ台本を 1 本足す。名前は本人の答えのとおり `tools/check-adr.sh`（推奨で書いた版の `tools/test-adr-docs.sh` を、この名前の 1 本にまとめた。台本を 2 本に分けない —— 正典の Scenario の印も、CH12 の受け入れの口（`--verbatim`・`--backlog`）も同じ 1 本が持つ）。

`.github/workflows/ci.yml` の chain job の `- run: ./tools/check-private.sh` の直後に、次の 1 行を足す:

```
      - run: ./tools/check-adr.sh --self-test && ./tools/check-adr.sh
```

- 1 行で自己検査と本番の両方を走らせる（chain job のほかの検査は `--self-test` と本番を 2 段に分けているが、本人の答えは「1 行」。自己検査を外すと、台本が壊れて何も落とさなくなっても CI は緑のまま）
- 置き場は chain job（出し分けの対象外で、何が変わっても必ず走る。`docs/` だけの PR でも見る）
- `--verbatim` と `--backlog` は CI に載せない（前者は D1 のとおり後の手入れで落ちうる。後者は CI の runner にメモリも harness2 の台本も無い）。`--self-test` の中で `--backlog` を子として走らせる D5 の形の検査は、CI では台本が見つからず rc=127 の枝を通る（形だけを見る）
- 本番の口（引数なし）は、CH12 で置いた 6 ファイル・4 つのメモリ名・`tools/check-private.sh` も見る（どれも正典の Scenario で、ADR を足しても消しても成り立つべき性質。README は ADR を消さず「置き換え済み」にすると決めている）。check-private は chain job で 2 回走るが、0.2 分の job の中の数秒

コード・移行・既存の `tools/` の台本は変えない。CI は chain job の 1 行だけ（ほかの段・ほかの job は変えない）。

反転条件: ADR の形の検査が正当な ADR を止める（README の形に合う ADR を検査が落とす）なら、検査を README の形に合わせて直す。CI から外すのは、本人が「毎回見なくてよい」と決め直したときだけ。

### D3（仮）. 食い違いは直さずに PR 本文で挙げる（Q1）

ADR の本文と repo の今が食い違う箇所は、`premises.md` の表にある 8 つ（Q1 の context の 3 つと、`review/deep.md` R3 の 5 つ）。
加えて、README と 3 つの ADR が参照する harness2 の `docs/adr/` が、まだ harness2 の main に無いこと（2026-11-09 追記: その後 main に入った（`1e140a4b3`）ので、PR 本文と CH12.md の節には「いまはある」と書く。`review/code.md` R5）。これらを `docs/briefs/CH12-pr.md` に一覧で挙げる。ADR には書き足さない。

**2026-11-09 追記（`review/code.md` R26）**: PR 本文（`docs/briefs/CH12-pr.md`）は 2026-11-08 からグラフの `finish` が台本 `scripts/pr_body.py` で組み直し、前の本文から引き継ぐのは `Ruling:` の行だけ（`pr_body.py` の `rulings`・`build`）。手で一覧を足しても次の `finish` で消えるので、一覧の置き場は下の `docs/changes/CH12.md` の節だけにする。この節は PR の差分（`docs/changes/CH12.md`）に入るので、merge の前に人間が PR で読める。Q1 の意図（直さずに挙げ、人間が後で change を起こせる）は変わらない

**後で見つける置き場**（`review/deep.md` R4・`review/spec.md` R3）: PR 本文は merge の後に流れていくので、同じ一覧を `docs/changes/CH12.md` の
`## ADR と repo の今の食い違い（直していない）` の節にも置く（「依頼（逐語）」の外。archive の後も repo に残る）。ADR の手入れの change を起こすとき、ここから拾う。
手入れの change はこの change では起こさない（本人が一覧を見て決める）。

反転条件: Q1 の答えが「(1)(2) の ADR の『影響』の末尾に『その後』の 1 行を足す」なら、その 2 件の ADR だけ末尾に 1 行を足し、D1 の逐語の一致はその 2 件について「逐語の塊が先頭からそのまま入っている」に緩める。

### D4（仮）. 完了の判定は repo の中のコマンドで確かめ、harness2 の側を待たない（Q2）

`tools/check-adr.sh --backlog` が harness2 の `adr_backlog.py` を走らせる。どこにも手元のパスを書かずに、次の順で探す:

1. 環境変数 `ADR_BACKLOG`（台本のパス）が設定されていれば、それだけを使う。指す先が無ければ次へ落ちずに rc=127（`review/spec.md` R6）
2. `.harness2/scripts/adr_backlog.py`（harness2 の main に入れば、ここで見つかる）
3. `.harness2` の指す repo の worktree（`git -C "$(readlink -f .harness2)" worktree list --porcelain` の順）のうち、`scripts/adr_backlog.py` を持つ最初のもの

メモリの置き場は、環境変数 `ADR_MEMORY` が設定されていればそれだけ（指す先が無ければ rc=127）、設定されていなければこの repo の main の checkout（`git worktree list --porcelain` の 1 つめ）のパスから、
`adr_backlog.py` と同じ規則（英数字以外を `-` に）で `~/.claude/projects/<それ>/memory` を組む。worktree のパス（`…-up-ch12` など）から組まない（R1 の空の合格）。
走らせ方は `python3 -I <台本> --memory <置き場> --adr docs/adr --json`。`--adr` には `docs/adr` だけを渡す（4 つのメモリはこの repo の ADR が名指す。harness2 の ADR は要らない）。
判定（`review/spec.md` R4。「名前が出力に無い」だけでは、台本が落ちても・数える対象から外れても通るので、次の 3 つを全部見る）:

1. **正の対照**: 同じ置き場で `--adr <空の mktemp -d>` を走らせ、4 つの名前が**すべて**残りに出ること（数える対象に 4 つが在る）。出なければ落とす
2. 本番（`--adr docs/adr`）の rc が 0 か 1 で、標準出力が JSON の配列として読めること（rc=1 はハーネスのメモリが残る限り普通）。でなければ落とす
3. 本番の配列の各要素のファイル名に、4 つの名前のどれも無いこと

台本かメモリの置き場のどちらかが見つからなければ rc=127 と `error: …が無い` の 1 行で終える。verify-run は rc=127 を環境の欠落（BLOCKED_INFRA）と読み、FAIL にしない（`.harness2/scripts/evidence.py:437-440`）。

反転条件: Q2 の答えが「harness2 の ADR が main に入るのを待つ」なら、探し方を 2. だけにし、この change は harness2 の merge まで `--backlog` の検証で止まる。

### D5（仮・C4）. 検証の出力には、4 つのメモリ名の有無だけを出す

`adr_backlog.py` の標準出力（残りのメモリ 1 件ごとのファイル名と description）と標準エラーは、`tools/check-adr.sh` の中で一時の場所に受けて捨て、外に出さない（`review/spec.md` R5）。出すのは 4 行
（`<メモリ名>: 残りに無い` か `<メモリ名>: 残りにある`）と、結果の 1 行だけ。verify-run は出力の末尾 20 行を証跡（commit される）に入れ、この repo は公開なので、
ハーネスのメモリの名前と説明が外に出る経路になる（`review/deep.md` R5）。4 つの名前は既に `docs/changes/CH12.md` に公開されている。
出してよい行の形は 4 つだけ: `Scenario: …` は使わない（`--backlog` は正典に Scenario を持たない。D8）ので、`<4 つの名前のどれか>: 残りに(無い|ある)`・`OK …` / `NG …` の結果の 1 行・`error: …`。
`--self-test` が `--backlog` を子として走らせ、標準出力と標準エラーの全行がこの形のどれかに当たることを見る。子は偽の台本（秘密の印を標準出力と標準エラーに出す）と使い捨てのメモリの置き場で走らせ、手元のメモリと harness2 に依らない（`review/code.md` R4）。

（仮）である理由: 失わない側（出さない）を選んだ。反転条件は、メモリの残りを公開してよいと本人が決めたとき（その場合も一覧を出す理由は無いので、変える見込みは小さい）。
台帳 `provisional.jsonl` に載せる（`review/deep.md` R5 の `loss: exported`）。

### D6（C3）. 外に出ることは問わない

ADR の本文は、依頼の逐語として既に公開の origin にある（`premises.md` の 1 行目）。新しく出るのは `CLAUDE.md`・`AGENTS.md`・`.github/workflows/ci.yml` の 1 行ずつ（D2（仮）・D7（仮））と検査の台本だけで、どれも私的なものを含まない。

### D7（仮）. agent への案内は `CLAUDE.md` の表の 1 行と、`AGENTS.md` の 1 行の指示（Q4）

本人の答え（Q4。推奨と違う）: 「AGENTS.md にも 1 行の指示を足す」。推奨（`CLAUDE.md` の表の 1 行だけ）は採らない。
選択肢の detail のとおり、Codex を含む agent が判断を ADR に書くようになり、どの判断を ADR にするかを agent が決めることになる（README の「いつ書くか」に従って）。

`CLAUDE.md` の行は `| \`docs/adr/\` | Story をまたぐ判断の記録 | する |`。置き場は `docs/` の行の直後（`docs/` の下の特別な置き場なので、親の行の次に読める）。

`AGENTS.md` の行は、「## 実装と計画の規約（上流の計画 agent・Task agent・reviewer に共通）」の導入の段落（「上流が `tasks.md` に書く検証・置き場も、…」の行）の直後に 1 行:

```
Story をまたぐ判断（製品の前提・コードの構成・道具の規則）は `docs/adr/` に ADR として書く。いつ書くか・形の正本は `docs/adr/README.md`、1 つの change で閉じる判断はその `design.md` の `D<n>`。
```

- 置き場を `### テスト` などの小節（`<!-- harness2:… -->` の印を持つ）の外にするのは、ADR が特定の層の規約でなく、計画・実装・レビューの全部に掛かるから。印の塊には触らない
- いつ書くか・形の規則の正本は `docs/adr/README.md` の 1 か所のまま。`AGENTS.md` の行は置き場と正本への道だけを言い、規則を写さない（写すと 2 か所がずれる）
- この行は `tools/check-adr.sh` が見る（「実装と計画の規約」の節に、`docs/adr/` と「Story をまたぐ判断」を含む行がちょうど 1 つ）。後の change が消すと CI が落ちる

反転条件: agent が Story の中で閉じる判断まで ADR にし始めて `docs/adr/` が膨らむなら、行の言い方を README の「いつ書くか」の言い方に寄せて直す（行は消さない）。

### D8. capability `decision-records` を新しく立て、`skip_specs` は使わない

schema は docs の change に `skip_specs: true` を勧めるが、この repo の change で `skip_specs` を使った前例は無く、Requirement が 0 件の change がグラフ
（`check_scenarios`・`go_brief`・merge gate・archive）を通るかを確かめていない。また、索引と実ファイルの対応・出所の名指しは外から機械で確かめられる性質で、
CH10 の `emulator-startup`・CH09 の `test-timing` と同じく、開発の約束として正典に置ける。

正典に置かないもの（`review/spec.md` R10）: 依頼の逐語との一致（D1）と、`adr_backlog.py` の残りの判定（D4）。どちらも CH12 の受け入れの 1 回の判定で、
後者は手元にしか無いメモリと、CI の runner に無い台本（harness2 の `adr_backlog.py`）に依る。どちらも `docs/changes/CH12.md` の完了の判定と tasks の検証にだけ置く。

正典に置くもの（索引と実ファイルの 1 対 1・並び・ファイル名の形・見出しの形・`CLAUDE.md` の表・`AGENTS.md` の行・CI の行・出所の名指し・網の名前）は、ADR を足すたびに成り立つべき性質。
Q3 の本人の答えで台本を CI の chain job に載せるので、archive の後も PR ごとにこれを確かめる（D2（仮））。見出しの形・`AGENTS.md` の行・CI の行も、同じ理由で正典に置く。

### D9. 検証の台本の形

`tools/check-adr.sh`（bash。冒頭のコメントに何を確かめるか・なぜ要るか（CH12）・走らせ方の 4 つの口・CI の chain job が引数なしと `--self-test` を走らせること）:

- 引数なし（CI の chain job が走らせる口）: 1 つ目と 3 つ目の Requirement と、2 つ目の「出所に名指しがある」を見る。各検査の前に `echo "Scenario: <名前>"` を出す。
  見る `docs/adr/` は環境変数 `ADR_DIR`（既定 `docs/adr`）、`CLAUDE.md` は `ADR_CLAUDE_MD`（既定 `CLAUDE.md`）、`AGENTS.md` は `ADR_AGENTS_MD`（既定 `AGENTS.md`）、CI の定義は `ADR_CI_YML`（既定 `.github/workflows/ci.yml`）。
  - 見出しの形: README 以外の各 ADR について、1 行目が `# ` で始まり、`- 日付: YYYY-MM-DD`・`- 状態: 採用` か `- 状態: 置き換え済み（→ <ファイル名>）`・`- 出所: ` の 3 行がそれぞれちょうど 1 つあり、
    `## 背景`・`## 決定`・`## 退けた案`・`## 反転の条件`・`## 影響` の 5 つの見出しがそれぞれちょうど 1 つ、この順に出る。足りない・重なる・順が違う見出しを、ファイル名と一緒に `error:` で出す
  - `AGENTS.md`: `## 実装と計画の規約` の行から次の `## ` の行の手前までに、`` `docs/adr/` `` と「Story をまたぐ判断」の両方を含む行がちょうど 1 つ
  - ファイル名の日付と置き換えの後継（第 2 回 Q5 の本人の答え。`review/code.md` R21）: 各 ADR のファイル名の先頭 8 桁を `YYYY-MM-DD` にしたものが `- 日付:` の値に一致する。`- 状態: 置き換え済み（→ X）` の X は `docs/adr/` にある README 以外のファイル（`/` を含む名前と `README.md` は落とす）。違えばファイル名と一緒に `error:` で出す
  - CI: `ADR_CI_YML` の `  chain:` の行から次の job の行（行頭の空白 2 つ + 名前 + `:`）の手前（chain が最後の job なら、ファイルの終わり）までに、`- run: ./tools/check-adr.sh --self-test && ./tools/check-adr.sh` の行がちょうど 1 つ（D2（仮））
  `tools/check-private.sh` は repo の根で走らせる（`ADR_DIR` を変えても、check-private は repo の追跡ファイルを見る）。
  check-private は `git ls-files` の範囲しか見ないので、呼ぶ前に `git ls-files docs/adr` が README と 5 件の ADR を返すことを確かめ、返さなければ `error: docs/adr/ がまだ追跡されていない` で落とす（`review/spec.md` R8）
- `--self-test`: `mktemp -d` の使い捨ての木に `docs/adr/`・`CLAUDE.md`・`AGENTS.md`・`.github/workflows/ci.yml` を写し、`ADR_DIR`・`ADR_CLAUDE_MD`・`ADR_AGENTS_MD`・`ADR_CI_YML` をそこへ向けて壊す。
  壊し方は 9 つ（一覧の 1 項目を消す・一覧の 2 項目を入れ替える・ADR の 1 件の見出しを 1 字変える・ADR の 1 件の出所の行を消す・ADR の名前を大文字にする・`CLAUDE.md` の表の行を消す・
  ADR の 1 件から `## 決定` の見出しを消す・`AGENTS.md` の指示の行を消す・CI の定義の chain job から `check-adr.sh` の行を消す）で、
  どれでも検査が rc≠0 になることを確かめる（`review/spec.md` R7）。
  `review/code.md` R2・R6・R7 で 12 を足した（一覧の形の違う項目・1 行目の `# ` を消す・`- 日付:` だけ変える・出所の行は残してメモリ名だけ消す・出所の行を末尾へ移す・`- 状態: 検討中` にする・状態の行を 2 つにする・見出し 2 つを入れ替える・一覧と ADR の名前を両方大文字にする・ADR を 1 件追跡から外す・`check-private.allow` に `docs/adr/` を足す・allow を消す）。
  第 2 回 Q5（R21）で 2 つ足した（ADR と一覧の日付をそろえて変え、ファイル名の日付とだけ食い違わせる・置き換え済みにして無い後継を指す）。
  レビューの直しで 5 つ足した（`- 日付:` の行を 2 つにする・日付の値の後ろに字を足して `YYYY-MM-DD` の形を崩す・`README.md` を消す・`docs/adr/` ごと消す・`docs/adr/` を丸ごと追跡から外す）。
  `review/code.md` R22 で 2 つ足した（置き換えの後継を `README.md` にする・`docs/adr/` の外に実在するファイル `../../CLAUDE.md` にする）。合わせて 9＋12＋2＋5＋2＝30 で、台本の壊し方の一覧（`self_test` の `for case_name in …`）と同じ。
  どれも期待する `error:` の文面まで見る。追跡と allow の検査は `ADR_GIT_ROOT`・`ADR_PRIVATE_ALLOW` で使い捨ての木（`git init` した写し）へ向ける。正の対照は 3 つで、どれでも引数なしの検査が rc=0 になることを見る: 形の正しい ADR を 1 件足して追跡する（`next-adr`。R1）・既存の ADR を置き換え済みにして後継の ADR を足す（`superseded`。R18）・どの ADR より古いファイル名の ADR を一覧の末尾に足す（`older-adr`。R18）。使い捨ての木は、台本の起動時に `mktemp -d` で作る `TMP_ROOT` の下に置き、`trap 'rm -rf "$TMP_ROOT"' EXIT` で台本の終わりにまとめて消す（INT・TERM は exit に変えて EXIT の trap を通す。R19）。「出所の行が無い ADR は落ちる」の印はここで出す。
  `--backlog` の欠落の 2 つ（`ADR_BACKLOG` を無いパスに・`ADR_MEMORY` を無いフォルダにして、`ADR_BACKLOG` は使い捨ての木に置いた偽の台本に向ける）と、D5 の出力の形もここで見る。
  欠落の 2 つは `error:` の文面で、どちらの枝を通ったかを見る（R6）。CI の runner には `.harness2` もメモリも無いので、そこで走る自己検査は手元のものに依らない（偽の台本と使い捨ての木だけ）
- `--verbatim`: D1（CI に載せない）
- `--backlog`: D4・D5。一時の出力は `mktemp` の場所（`/tmp` の固定名に書かない）（CI に載せない）
- 落ちるときは `error: ` で始まる行で、何が・どのファイルで違うかを出す

## Risks / Trade-offs

- [ADR の本文が repo の今と食い違ったまま入る（8 つ）] → 依頼の決定（「書き換えはしない」）。`docs/changes/CH12.md` の節に一覧で挙げ（PR の差分に入る。PR 本文は台本が組むので一覧を持たない）、人間が後で change を起こせるようにする（D3）
- [README と ADR からの harness2 の ADR への参照に、上流の時点では行き先が無かった] → 2026-11-08 に harness2 の main に入り（`1e140a4b3`）、解けた（`review/code.md` R5・R25）。参照は題名で、パスのリンクではないので、リンク切れの検査には掛からない。いまはあることを `docs/changes/CH12.md` の節の末尾に書いた
- [`--backlog` は手元でしか走らず、CI では確かめられない] → 完了の判定の 4. は手元の検証（verify-run）の証跡で確かめる。メモリが無い環境では BLOCKED_INFRA で止まり、空の合格にはしない（D4）
- [harness2 の worktree の探し方は、別の枝の古い `adr_backlog.py` を拾いうる] → 台本の入出力（`--memory`・`--adr`・`--json`）だけに依り、判定は 4 つのメモリ名の有無だけなので、版の違いに強い。`.harness2` に入れば 2. が先に当たる
- [ADR の形の検査が chain job で毎回走るので、README の形から外れた ADR を足す PR は CI で落ちる] → Q3 の本人の答えの意図そのもの。形を変えたいときは README と検査を同じ change で直す（D2（仮）の反転条件）
- [`AGENTS.md` と `.github/workflows/ci.yml` は走っている change（ch10・st56）も触る] → 触る節が違う（`AGENTS.md` は ch10 が `/dev/kvm` の節、ci.yml は ch10・st56 が rust と android の job）。gate が main に rebase するので、後に入る側が追従する
- [`--verbatim` は ADR の手入れの後に落ちる] → CH12 の受け入れの口と台本のコメントに書く。正典の Scenario にはしない（D1）

## Migration Plan

文書と検査の台本を足すだけ。戻すときは `docs/adr/`・`tools/check-adr.sh` を消し、`CLAUDE.md`・`AGENTS.md`・`.github/workflows/ci.yml` の chain job の 1 行ずつを消す。データ・移行・配布物に影響は無い。

## Open Questions

（無い）
