spec-digest: d72cd9771bc71a0ede0696f5b598118dc0cb050239a36adbcbdf4c0125215f37
- Q3: 写っていない（D2（仮）・spec・tasks・proposal・CH12.md・PR 本文には写った。design.md:118 の D6 にある「新しく出るのは `CLAUDE.md` の 1 行と検証の台本だけ」が推奨の範囲のまま残っており、ci.yml の chain job の 1 行が抜けている。決定を覆す話ではなく、1 文の直し）
- Q4: 写っていない（D7（仮）・spec・tasks 2.2・proposal・CH12.md・PR 本文には写った。同じ design.md:118 の D6 の文から、`AGENTS.md` の 1 行が抜けている。1 文の直し）
答え以外の変更: あり（仮でない D9 の `--self-test` に「先に、壊していない写しで引数なしの検査が rc=0 になることを見る」を足した（design.md:164・tasks 1.1。どの答えにも当たらない）／tasks 1.1 で、`--verbatim` を未実装のまま置く stub（`error: --verbatim は未実装` rc=2）を足し、「`--self-test` も落ちる」を書き足した（どの D にも当たらない）／仮の D に書かれた付随の判断: D2（仮）で台本を 2 本でなく 1 本にまとめた（`test-adr-docs.sh` を `check-adr.sh` に統合）・CI の本番の口でも CH12 で置いた 6 ファイル・4 つのメモリ名・check-private を見る・自己検査と本番を 1 行に束ねる、D7（仮）で `AGENTS.md` の行を検査の対象（正典の Scenario）にした）

## 確かめた根拠

- 答え: `deep-answers-1.txt` の Q3「足す（`tools/check-adr.sh` と CI の chain job の 1 行）」・Q4「AGENTS.md にも 1 行の指示を足す」。`deep.md:16-26` に同じ文で記録され、推奨と違うことも書かれている。選択肢の detail は `deep-questions.json`（Q3「一覧と実ファイルのずれを毎回止める。台本・試験・CI の分だけ差分と検証が増える」、Q4「Codex を含む agent が判断を ADR に書くようになる」）
- 差分: `git diff c38dc36d7 HEAD`（7 ファイル、+146 −70）。変わったのは `docs/briefs/CH12-pr.md`・`docs/changes/CH12.md`（完了の判定だけ。`## 依頼（逐語）` には触れていない）・design・premises・proposal・spec・tasks
- Q3 が写った場所
  - design D2（仮）の見出しと本文（design.md:50-71）。ci.yml の行の形・置き場・`--verbatim`/`--backlog` を CI に載せないことと、反転条件
  - Goals に追加し、Non-Goals から削除（design.md:24・旧 Non-Goals の「CI で毎回見ること」）
  - D8 の「正典に置くもの」（design.md:148-149。旧文は Q3 の推奨を名指していたので、写しとみなした）
  - D9 の引数なしの口に見出しの形・CI の行の検査、`--self-test` に 3 つの壊し方を追加（design.md:155-169。見出しの形は Q3 の問いの文面にある）
  - Risks・Migration Plan（design.md:180-186）
  - spec に Requirement「ADR の形は CI で PR ごとに確かめる」と Scenario 2 本（spec.md:68-82）。Scenario「ADR は決まった項目と見出しをこの順に持つ」（spec.md:44-46）
  - tasks の Global Constraints・1.1・2.2・3.1・4.1
  - proposal の What Changes・やらないこと・Impact
  - premises の C2 の 2 つ目の ★
  - CH12.md の完了の判定 1・6・8
  - PR 本文
- Q4 が写った場所
  - design D7（仮）（design.md:120-137）。`AGENTS.md` に置く行の文面と置き場。置き場の目印は AGENTS.md:16 に実在する
  - spec の Requirement の 1 文と Scenario「AGENTS.md の規約から docs/adr/ に辿れる」（spec.md:16・48-50）
  - tasks 2.2、CH12.md の完了の判定 5、proposal、PR 本文
- 推奨のまま残った文の探し方: `grep -rn "test-adr-docs\|CI には載せ\|載せない\|AGENTS.md は変え\|AGENTS.md には書かない\|表の 1 行だけ\|触らない\|\.github"` を change・CH12.md・PR 本文にかけ（`review/` は除く）、さらに design を通しで読んだ。残っていたのは上の D6（design.md:118）と go.json（下の注記）だけ。premises.md:25-26（C2 の本文と 1 つめの ★）は履歴なので対象外。design.md:3-4 の冒頭の注記は、5 行目に答えの注記が続くので残ってよい
- 「答え以外の変更」の分け方
  - 台本の改名と、見出しの形・CI の行・`AGENTS.md` の行の検査と壊し方は、答えを写したものとして扱った。ただし行を検査の対象にすること（D7（仮））や台本を 1 本にまとめること（D2（仮））は、答えが言っていない付随の判断なので `あり` に名指した
  - 自己検査の「壊していない写しで rc=0」は、design にも Q3 に結び付ける理由が無い（書かれている理由は、写し方を誤ったときに確かめが空振りしないため、という一般的なもの）。仮でない D9 の変更なので、`答え以外の変更の決定:` の行は書かない

## 写しで壊れたもの・注記

- `openspec validate ch12-memory-decisions-adr --strict`: valid（rc=0）。`python3 scripts/evidence.py lint openspec/changes/ch12-memory-decisions-adr`: OK（rc=0）。`python3 scripts/check_chain.py .`: chain OK
- Scenario の本数: spec の `#### Scenario:` は 13 本（Requirement ごとに 8・2・2・1）。tasks 1.1 の `echo "Scenario: …"` も 13 本で、名前は一字一句一致する（引数なし 11・`--self-test` 2。tasks の Global Constraints・PR 本文の「13 本（11+2）」とも合う）。`check_scenarios.py` は 13 本とも「担保なし」で FAIL するが、台本が未実装の上流の段なので壊れではない
- tasks 2.2 と 4.1 の検証コマンドは、`bash -c '…'` の引用と `printf "1\t0"` の比較で書き方の誤りは見つからなかった。ci.yml の chain job（.github/workflows/ci.yml:491〜）に `- run: ./tools/check-private.sh` は実在する。chain は最後の job なので、D9 の「`  chain:` から次の job の行の手前まで」は実際にはファイルの終わりまでになる。そのとき（次の job が無いとき）の扱いは D9 に書かれていない（小さな点）
- go.json は推奨の版のまま残っている（`tools/test-adr-docs.sh`・「CI には載せない」・「AGENTS.md には指示を足さない」・Scenario 9 本。go.json:6・51・198-219・338-349・391・410・487・525-526）。これは `spec --answers` の範囲ではなく、次の `reading` が直すものなので、ここでは指摘にしない
