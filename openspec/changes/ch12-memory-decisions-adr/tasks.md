# CH12 実装タスク — メモリにしか無かった判断を ADR に置く

読む順: `deep.md`（**本人の答え Q1〜Q4**。Q3・Q4 は推奨と違う）→ このファイル → `specs/decision-records/spec.md` → `design.md` → `premises.md` → `review/deep.md` → `docs/changes/CH12.md` → `CLAUDE.md`。

検証は各タスクの本文に書いてある。「動いた」ではなくコマンドと終了コードで判定する。Task 3 だけが手元のメモリ（`~/.claude/projects/…/memory/`）と harness2 の `adr_backlog.py` を要る。

## Global Constraints

- **ADR の本文は 1 字も変えない。** `docs/adr/` の 6 ファイルは `docs/changes/CH12.md` の `## 依頼（逐語）` の各「=== ファイル: <名前> ===」の塊そのもの（design D1 の取り出し方）。
  事実と食い違う箇所を見つけても直さず、`docs/briefs/CH12-pr.md` に挙げる（D3（仮））。**`docs/changes/CH12.md` の `## 依頼（逐語）` の節も変えない**（比べる元）
- **触ってよいのは `docs/adr/`（6 ファイル）・`CLAUDE.md`（表の 1 行の追加だけ）・`AGENTS.md`（指示の 1 行の追加だけ）・`.github/workflows/ci.yml`（chain job の 1 行の追加だけ）・`tools/check-adr.sh`（新しいファイル）だけ**（D2（仮）・D7（仮））。
  コード・移行・既存の `tools/` の台本・ci.yml のほかの段とほかの job・`tools/check-private.allow` は変えない。`CLAUDE.md` と `AGENTS.md` のほかの行と `<!-- harness2:… -->` の印も変えない
- **テストには `Scenario: <名前>` の印を置く。** 台本は bash なので、その検査を走らせる直前に `echo "Scenario: <名前>"` を出す。`scripts/check_scenarios.py` が spec の全 Scenario と突き合わせる。印の名前は spec の `#### Scenario:` と**一字一句合わせる**
- **この change の Scenario は 13 本**（`decision-records` の ADDED 4 つ: 8・2・2・1 本）。どれも `tools/check-adr.sh` が印を出す（引数なし 11 本・`--self-test` 2 本）。
  `--verbatim` と `--backlog` は CH12 の受け入れの 1 回の判定で、正典に Scenario を持たない（design D1・D8）ので、印を出さない
- **本人の決定（下流は変えない）**: Q1 食い違いは直さず PR 本文で挙げる（D3）/ Q2 repo の中のコマンドで確かめ、harness2 の側を待たない（D4）/ Q3 ADR の形の検査 `tools/check-adr.sh` を CI の chain job の 1 行で毎回走らせる（D2）/ Q4 `CLAUDE.md` の表の 1 行に加えて `AGENTS.md` にも 1 行の指示を足す（D7）
- **検証の出力に、公開されていないメモリの名前と説明を出さない**（D5）。`--backlog` が出すのは 4 つのメモリ名の有無だけ。`adr_backlog.py` の出力をそのまま流さない。手元のパス（`/home/…`）も台本に書かない（D4 の探し方で組む）
- 一時の場所は `mktemp -d`（`/tmp` の固定名に書かない）で、`trap` で必ず消す

## Task 1: ADR の形の検査の台本

- [x] 1.1 `tools/check-adr.sh` を design D9 のとおりに書く（冒頭のコメントに何を確かめるか・なぜ要るか（CH12）・4 つの口（引数なし・`--self-test`・`--verbatim`・`--backlog`）・CI の chain job が引数なしと `--self-test` を走らせること・`--verbatim` は ADR の手入れの後に落ちうること（D1））。
  引数なしの口は、次の 11 を順に確かめ、それぞれの前に印を出す:
  `echo "Scenario: README の一覧と docs/adr/ の ADR のファイルが 1 対 1 で対応する"` /
  `echo "Scenario: 一覧の日付と題は ADR の日付と見出しに一致する"` /
  `echo "Scenario: 一覧はファイル名の新しい順に並ぶ"` /
  `echo "Scenario: ADR のファイル名は時刻と英小文字の slug の形を持つ"` /
  `echo "Scenario: ADR は決まった項目と見出しをこの順に持つ"` /
  `echo "Scenario: CH12 で置いた README と 5 件の ADR がある"` /
  `echo "Scenario: CLAUDE.md の文書の表から docs/adr/ に辿れる"` /
  `echo "Scenario: AGENTS.md の規約から docs/adr/ に辿れる"` /
  `echo "Scenario: CI の chain job が ADR の形の検査を走らせる"` /
  `echo "Scenario: 5 件の ADR の出所に、移したメモリのファイル名がある"` /
  `echo "Scenario: docs/adr/ を含む追跡ファイルで、網の名前の検査が通る"`（check-private の前に `git ls-files docs/adr` が 6 ファイルを返すことを見る。D9）。
  `--self-test` の口は D9 の 9 つの壊し方を使い捨ての木で試し、どれでも引数なしの検査が rc≠0 になることを確かめる。
  出所の行を消す場合の前に `echo "Scenario: 出所の行が無い ADR は落ちる"`、`## 決定` の見出しを消す場合の前に `echo "Scenario: 決まった見出しの欠けた ADR は落ちる"` を出し、どちらも出力にそのファイル名が出ることも見る。
  `--verbatim` は D1、`--backlog` は Task 3 で足す（この段では `error: --backlog は未実装` で rc=2 でよい）。
  この段ではまだ `docs/adr/` が無いので、引数なしの口は落ちる（それでよい。Task 2 で通す）。
  検証: `bash -n tools/check-adr.sh` rc=0

## Task 2: ADR を置き、CLAUDE.md・AGENTS.md・CI に 1 行ずつ足す

- [x] 2.1 `docs/changes/CH12.md` の `## 依頼（逐語）` から D1 の取り出し方で 6 つの塊を取り、`docs/adr/<名前>` に書く（README・5 件の ADR）。`tools/check-adr.sh --verbatim` を D1 のとおりに書く（先に節の sha256 が design D1 の定数と一致することを見る。次に 6 つの塊それぞれについて一致を見て、違えば塊の名前と最初に違う行を出す。塊が 6 つでなければ落ちる）。
  書いたら `git add docs/adr` で追跡に入れる（check-private は追跡しているファイルしか見ない）。
  検証: `tools/check-adr.sh --verbatim` rc=0
- [x] 2.2 `CLAUDE.md` の「文書とハーネス」の表の `docs/` の行の直後に `` | `docs/adr/` | Story をまたぐ判断の記録 | する | `` の 1 行を足す（D7（仮））。
  `AGENTS.md` の「## 実装と計画の規約」の導入の段落（「上流が `tasks.md` に書く検証・置き場も、…」の行）の直後に、design D7 の 1 行を足す（D7（仮））。
  `.github/workflows/ci.yml` の chain job の `- run: ./tools/check-private.sh` の直後に `      - run: ./tools/check-adr.sh --self-test && ./tools/check-adr.sh` の 1 行を足す（D2（仮））。
  検証: `tools/check-adr.sh` rc=0 / `tools/check-adr.sh --self-test` rc=0 / `tools/check-private.sh` rc=0 /
  `bash -c 'for f in CLAUDE.md AGENTS.md .github/workflows/ci.yml; do test "$(git diff --numstat origin/main...HEAD -- "$f" | cut -f1,2)" = "$(printf "1\t0")" || { echo "error: $f の差分が 1 行の追加でない"; exit 1; }; done'` rc=0

## Task 3: メモリの残りを数える台本で確かめる（手元）

- [x] 3.1 `tools/check-adr.sh --backlog` を design D4・D5 のとおりに書く（`ADR_BACKLOG` → `.harness2/scripts/adr_backlog.py` → `.harness2` の指す repo の worktree の順に探す。環境変数が設定されていればそれだけを使い、指す先が無ければ rc=127・
  メモリの置き場は `ADR_MEMORY` か main の checkout のパスから組む・`python3 -I <台本> --memory <置き場> --adr … --json`・判定は D4 の 3 つ（空の `--adr` での正の対照・本番の rc が 0 か 1 で JSON の配列・本番に 4 つの名前が無い）・
  `adr_backlog.py` の標準出力と標準エラーは一時の場所に受けて捨てる・出すのは D5 の形の行だけ）。Scenario の印は出さない（D8）。
  `--self-test` に次を足す: (a) `ADR_BACKLOG` を存在しないパスにすると rc=127 で、`error:` の行が台本の欠落を言う、(b) `ADR_BACKLOG` を使い捨ての木に置いた偽の台本（空の配列を出す）に向け、`ADR_MEMORY` を存在しないフォルダにすると rc=127 で、`error:` の行が置き場の欠落を言う、
  (c) `ADR_BACKLOG` を、空の `--adr` でも 4 つの名前を出さない偽の台本に向けると、正の対照で落ちる（rc≠0）、(d) `--backlog` を子として走らせ、標準出力と標準エラーの全行が D5 の形のどれかに当たる（台本かメモリが無ければ子は rc=127 で、形だけを見る）。
  検証: `tools/check-adr.sh --backlog` rc=0 / `tools/check-adr.sh --self-test` rc=0

## Task 4: 範囲と Scenario の突き合わせ

- [x] 4.1 main からの差分が、触ってよいものとこの change の文書だけであることと、spec の全 Scenario に印があることを確かめる。
  `docs/changes/CH12.md` の `## ADR と repo の今の食い違い（直していない）` の節が、下流で見つけたものも含めて最新であることを確かめる（足したら同じ節に足す。ADR と「依頼（逐語）」の節は直さない。design D3）。
  検証: `bash -c 'd=$(git diff --name-only origin/main...HEAD) || exit 1; test -n "$d" && test -z "$(printf "%s\n" "$d" | grep -vE "^(docs/adr/|CLAUDE\.md$|AGENTS\.md$|\.github/workflows/ci\.yml$|tools/check-adr\.sh$|openspec/|docs/changes/CH12\.md$|docs/briefs/|\.superpowers/)")"'` rc=0 /
  `python3 scripts/check_scenarios.py . ch12-memory-decisions-adr` rc=0
