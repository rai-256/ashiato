# CH12 実装タスク — メモリにしか無かった判断を ADR に置く

読む順: `deep.md`（**本人の答え Q1〜Q4**。第 1 回は推奨で先に書いてある）→ このファイル → `specs/decision-records/spec.md` → `design.md` → `premises.md` → `review/deep.md` → `docs/changes/CH12.md` → `CLAUDE.md`。

検証は各タスクの本文に書いてある。「動いた」ではなくコマンドと終了コードで判定する。Task 3 だけが手元のメモリ（`~/.claude/projects/…/memory/`）と harness2 の `adr_backlog.py` を要る。

## Global Constraints

- **ADR の本文は 1 字も変えない。** `docs/adr/` の 6 ファイルは `docs/changes/CH12.md` の `## 依頼（逐語）` の各「=== ファイル: <名前> ===」の塊そのもの（design D1 の取り出し方）。
  事実と食い違う箇所を見つけても直さず、`docs/briefs/CH12-pr.md` に挙げる（D3（仮））。**`docs/changes/CH12.md` の `## 依頼（逐語）` の節も変えない**（比べる元）
- **触ってよいのは `docs/adr/`（6 ファイル）・`CLAUDE.md`（表の 1 行の追加だけ）・`tools/test-adr-docs.sh`（新しいファイル）だけ**（D2（仮））。コード・移行・既存の `tools/` の台本・`.github/`・`AGENTS.md`・`tools/check-private.allow` は変えない。
  `CLAUDE.md` のほかの行と `<!-- harness2:… -->` の印も変えない
- **テストには `Scenario: <名前>` の印を置く。** 台本は bash なので、その検査を走らせる直前に `echo "Scenario: <名前>"` を出す。`scripts/check_scenarios.py` が spec の全 Scenario と突き合わせる。印の名前は spec の `#### Scenario:` と**一字一句合わせる**
- **この change の Scenario は 11 本**（`decision-records` の ADDED 3 つ: 5・5・1 本）。どれも `tools/test-adr-docs.sh` が印を出す（引数なし 7 本・`--self-test` 2 本・`--backlog` 2 本）
- **本人の決定（下流は変えない）**: Q1 食い違いは直さず PR 本文で挙げる（D3）/ Q2 repo の中のコマンドで確かめ、harness2 の側を待たない（D4）/ Q3 台本は CI に載せない（D2）/ Q4 agent への案内は `CLAUDE.md` の表の 1 行だけ（D7）
  （第 1 回は推奨で先に書いた。推奨と違った答えは `spec --answers` が写す）
- **検証の出力に、公開されていないメモリの名前と説明を出さない**（D5）。`--backlog` が出すのは 4 つのメモリ名の有無だけ。`adr_backlog.py` の出力をそのまま流さない。手元のパス（`/home/…`）も台本に書かない（D4 の探し方で組む）
- 一時の場所は `mktemp -d`（`/tmp` の固定名に書かない）で、`trap` で必ず消す

## Task 1: 検証の台本

- [ ] 1.1 `tools/test-adr-docs.sh` を design D9 のとおりに書く（冒頭のコメントに何を確かめるか・なぜ要るか（CH12）・4 つの口（引数なし・`--self-test`・`--verbatim`・`--backlog`）・`--verbatim` は ADR の手入れの後に落ちうること（D1））。
  引数なしの口は、次の 7 つを順に確かめ、それぞれの前に印を出す:
  `echo "Scenario: README の一覧と docs/adr/ の ADR のファイルが 1 対 1 で対応する"` /
  `echo "Scenario: 一覧の日付と題は ADR の日付と見出しに一致し、新しい順に並ぶ"` /
  `echo "Scenario: ADR のファイル名は時刻と英小文字の slug の形を持つ"` /
  `echo "Scenario: CH12 で置いた README と 5 件の ADR がある"` /
  `echo "Scenario: CLAUDE.md の文書の表から docs/adr/ に辿れる"` /
  `echo "Scenario: 5 件の ADR の出所に、移したメモリのファイル名がある"` /
  `echo "Scenario: docs/adr/ を含む追跡ファイルで、網の名前の検査が通る"`。
  `--self-test` の口は D9 の 4 つの壊し方を使い捨ての木で試し、どれでも引数なしの検査が rc≠0 になることを確かめる。出所の行を消す場合の前に
  `echo "Scenario: 出所の行が無い ADR は落ちる"` を出し、出力にそのファイル名が出ることも見る。
  `--verbatim` は D1、`--backlog` は Task 3 で足す（この段では `error: --backlog は未実装` で rc=2 でよい）。
  この段ではまだ `docs/adr/` が無いので、引数なしの口は落ちる（それでよい。Task 2 で通す）。
  検証: `bash -n tools/test-adr-docs.sh` rc=0

## Task 2: ADR を置き、CLAUDE.md の表に 1 行を足す

- [ ] 2.1 `docs/changes/CH12.md` の `## 依頼（逐語）` から D1 の取り出し方で 6 つの塊を取り、`docs/adr/<名前>` に書く（README・5 件の ADR）。`tools/test-adr-docs.sh --verbatim` を D1 のとおりに書く（6 つの塊それぞれについて一致を見て、違えば塊の名前と最初に違う行を出す。塊が 6 つでなければ落ちる）。
  検証: `tools/test-adr-docs.sh --verbatim` rc=0
- [ ] 2.2 `CLAUDE.md` の「文書とハーネス」の表の `docs/` の行の直後に `` | `docs/adr/` | Story をまたぐ判断の記録 | する | `` の 1 行を足す（D7（仮））。
  検証: `tools/test-adr-docs.sh` rc=0 / `tools/test-adr-docs.sh --self-test` rc=0 / `tools/check-private.sh` rc=0 /
  `bash -c 'test "$(git diff --numstat origin/main...HEAD -- CLAUDE.md | cut -f1,2)" = "$(printf "1\t0")"'` rc=0

## Task 3: メモリの残りを数える台本で確かめる（手元）

- [ ] 3.1 `tools/test-adr-docs.sh --backlog` を design D4・D5 のとおりに書く（`ADR_BACKLOG` → `.harness2/scripts/adr_backlog.py` → `.harness2` の指す repo の worktree の順に探す・メモリの置き場は `ADR_MEMORY` か main の checkout のパスから組む・`python3 -I <台本> --memory <置き場> --adr docs/adr --json`・rc は見ない・出すのは 4 行と結果の 1 行）。
  次の 3 つの印を出す: 4 つの名前の判定の前に `echo "Scenario: メモリの残りを数える台本は、CH12 の 4 つのメモリを残りに数えない"`、出力を確かめる前に `echo "Scenario: 残りを数えた結果は、4 つのメモリ名の有無だけを出す"`、
  そして `--self-test` に、`ADR_BACKLOG` を存在しないパスにした場合と `ADR_MEMORY` を存在しないフォルダにした場合の 2 つを足し、その前に `echo "Scenario: 数える台本かメモリの置き場が無ければ、通ったことにしない"` を出す（どちらも rc=127 と `error:` の行、4 つの名前の判定の行が出ないこと）。
  「4 つのメモリ名の有無だけを出す」は、台本の中で `--backlog` の自分の出力を受けて、`.md` で終わる語が 4 つの名前のほかに無いこと・`description` の語が無いことで見る。
  検証: `tools/test-adr-docs.sh --backlog` rc=0 / `tools/test-adr-docs.sh --self-test` rc=0

## Task 4: 範囲と Scenario の突き合わせ

- [ ] 4.1 main からの差分が、触ってよいものとこの change の文書だけであることと、spec の全 Scenario に印があることを確かめる。
  `docs/briefs/CH12-pr.md` の「ADR の本文と repo の今が食い違う箇所」が、下流で見つけたものも含めて最新であることを確かめる（足したら同じ節に足す。ADR は直さない）。
  検証: `bash -c '! git diff --name-only origin/main...HEAD | grep -vE "^(docs/adr/|CLAUDE\.md$|tools/test-adr-docs\.sh$|openspec/|docs/changes/CH12\.md$|docs/briefs/|\.superpowers/)"'` rc=0 /
  `python3 scripts/check_scenarios.py . ch12-memory-decisions-adr` rc=0
