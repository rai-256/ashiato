spec-digest: 453b2cd734a1b2e2fc0230dea5b8439cbf0970dda8f71d696faa5bab9e1ff151
- Q3: 写った
- Q4: 写った
答え以外の変更: あり（答えを写すのに伴う、D2（仮）の中の付随の判断 2 つ: 推奨で書いた版の `tools/test-adr-docs.sh` を答えの名指す `tools/check-adr.sh` の 1 本にまとめ、CH12 の受け入れの口（`--verbatim`・`--backlog`）も同じ台本に持たせた（design.md:56）／CI が走らせる本番の口（引数なし）では、ADR の形のほかに、CH12 で置いた 6 ファイル・4 つのメモリ名・`tools/check-private.sh` も見る（design.md:67）／D2 の見出しの印の形を `（仮・C2）` から `（仮）` に変え、C2 を見出しの末尾の「既定 C2 を改めた」へ移した（design.md:50。本文は変えていない））
答え以外の変更の決定: D2

## 確かめた根拠

3 回目の確かめ。対象は b11d054e5（D2 の見出しの印の形だけを変えた commit）。2 回目は a6f4ac7b6（1 回目 c2f5dba4f の指摘を処置した commit）を見た。b11d054e5 で design.md が変わったのは 50 行目の見出しの 1 行だけ（`git diff a6f4ac7b6 b11d054e5 -- design.md`）

- 答え
  - `deep-answers-1.txt` の Q3「足す（`tools/check-adr.sh` と CI の chain job の 1 行）」・Q4「AGENTS.md にも 1 行の指示を足す」
  - `deep.md:16-26` に同じ文で記録されている
- 差分は `git diff c38dc36d7 HEAD` で見た（`review/` を除いて 7 ファイル、+144 −69）
- 前回の指摘はどれも処置されていた（`git diff c2f5dba4f HEAD`）
  - design D6（design.md:118）: 新しく出るものの並びが「`CLAUDE.md`・`AGENTS.md`・ci.yml の 1 行ずつと検査の台本」に直った。推奨の範囲のまま残っていた最後の文で、これで Q3・Q4 とも `写った`
  - D9 と tasks 1.1 の「先に、壊していない写しで rc=0 を見る」は消えていた（`grep -n 壊していない` で 0 件）
  - tasks 1.1 の `--verbatim` の stub と「`--self-test` も落ちる」は、推奨版（c38dc36d7）と同じ文面に戻っていた。c38dc36d7 からの 1.1 の差分に残るのは、台本の名前・印 3 本・壊し方の数・CI の口の説明の 1 句だけで、どれも答えの写し
- D9 の CI の検査の範囲に足された「chain が最後の job なら、ファイルの終わり」（design.md:160）は、答えの写しと読んだ
  - これは Q3 の答え（CI の chain job の 1 行）を確かめる検査の範囲を定めたもので、前回「書かれていない」と注記した点を埋めたもの
  - 新しい判断を足したものではない（.github/workflows/ci.yml:491 の chain は実際に最後の job）
- Q3 が写った場所
  - design: D2（仮）、Goals・Non-Goals、D6、D8 の「正典に置くもの」、D9、Risks、Migration Plan
  - spec: Requirement「ADR の形は CI で PR ごとに確かめる」と、その Scenario 2 本・見出しの形の Scenario
  - そのほか: tasks の Global Constraints・1.1・2.2・3.1・4.1、proposal、premises の C2 の 2 つ目の ★、CH12.md の完了の判定 1・6・8、PR 本文
- Q4 が写った場所
  - design: D7（仮）・D6
  - spec: Requirement の 1 文、Scenario「AGENTS.md の規約から docs/adr/ に辿れる」
  - そのほか: tasks 2.2、proposal、CH12.md の完了の判定 5、PR 本文
- 推奨のまま残った文を探した方法
  - change・CH12.md・PR 本文に grep をかけた（`test-adr-docs`・`CI には載せ`・`AGENTS.md は変え`・`表の 1 行だけ`・`検証の台本だけ` など。`review/` は除く）
  - 当たったのは、推奨を退けたと書いた文（design.md:56・122）、答えを写した文（CH12.md:271）、履歴（premises.md:25 の C2 の本文）、go.json（下の注記）だけだった
- 「答え以外の変更」の分け方
  - 答えそのものとして扱ったもの
    - 自己検査と本番を 1 行に束ねること（答えが「1 行」と言っている）
    - 見出しの形の検査（Q3 の問いの文面にある）
    - CI の行・`AGENTS.md` の行が在ることの Scenario と、その検査・壊し方。spec に効く答えを Scenario にしたもので、`CLAUDE.md` の行の既存の Scenario と同じ形
  - 前回 D7（仮）の付随の判断として名指した「`AGENTS.md` の行を検査の対象にした」は、上の理由で CI の行の Scenario と同じく答えの写しに分け直した。D7 は仮なので、引き継ぎの判定はどちらに分けても変わらない
  - 残る 2 つ（台本を 1 本にまとめたこと・本番の口で CH12 に固有のものも見ること）は、答えが言っていない判断で、どちらも D2（仮）の本文に書かれている
  - D2 の見出しの印の変更（b11d054e5）は、答えの写しではないので `あり` に入れた。答えはどちらも印の形について何も言っていない
    - 変わったのは D2 の見出しの 1 行だけで、決定の中身（本文）は変わっていない
    - 推奨で書いた版（c38dc36d7）でも、D2 の見出しは `（仮・C2）` で、仮の印をすでに持っていた。仮でない決定を仮に変えたものではない
    - C2（聞かずに決めた既定）は、Q3 の答えで premises.md の C2 の 2 つ目の ★ がすでに改めており、見出しの末尾に「既定 C2 を改めた」として残っている
    - 同じ形は archive の ch05 の `### D4（仮）.` にある
    - ただし、この変更で `deep_draft.py` が D2 を仮と読むようになり、合意が引き継がれるかどうかの判定が変わる。この点を隠さないために名指した
  - 名指した変更はどれも D2 の中身（本文と見出し）として書かれており、D2 はいまの見出しで（仮）の印を持つ。そこで `答え以外の変更の決定: D2` と書いた。仮でない D に当たる変更や、どの D にも当たらない変更は見つからなかった

## 写しで壊れたもの・注記

- 機械の検査
  - `openspec validate ch12-memory-decisions-adr --strict`: valid
  - `python3 scripts/evidence.py lint openspec/changes/ch12-memory-decisions-adr`: OK
  - `python3 scripts/check_chain.py .`: chain OK
- Scenario の本数
  - spec は 13 本（Requirement ごとに 8・2・2・1）。tasks 1.1 の `echo "Scenario: …"` も 13 本で、名前は一字一句一致する（引数なし 11・`--self-test` 2）
  - `check_scenarios.py` は 13 本とも「担保なし」だが、台本が未実装の上流の段なので壊れではない
- tasks 2.2・4.1 の検証コマンドに、書き方の誤りは見つからなかった
- go.json は推奨の版のまま残っている（`tools/test-adr-docs.sh`・「CI には載せない」・「AGENTS.md には指示を足さない」・Scenario 9 本）。`spec --answers` の範囲ではなく、次の `reading --carry` が直すもの
- D2 の見出しの印は、いまは機械も仮と読む
  - b11d054e5 で D2 の見出しが `### D2（仮）.` になった。`deep_draft.py` の `_provisional`（`^#+\s*(D\d+)（仮）`）が D2 を拾うので、`status` は D2 を仮の決定と読む
  - 同じ `（仮・C<n>）` の形のまま残っているのは D5（`### D5（仮・C4）.`）。D5 は今回の答え以外の変更に含まれていないので、判定には効かない
