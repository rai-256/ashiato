# impact-review — ch12-memory-decisions-adr

seen: IMP-01 low

## R1 — 「直すまで merge できない」は GitHub では言えず、CI の緑を前提にする harness2 側が dependencies に無い
- 対象: impact.situations[normal].can / walkthroughs[W1].figure.rows / scenarios[S1].result / dependencies（欠落）
- 種類: 依存の見落とし（言い切りを兼ねる）
- 指摘: W1 の表（「直すまで merge できない」）と S1 の result（「merge できるのは直してから」）は、止める主体を CI にしている。しかしこの repo は branch protection を持てない（`gh api repos/rai-256/ashiato2/branches/main/protection` が 403「Upgrade to GitHub Pro」）ので、GitHub は赤い PR の merge を拒まない。実際に止めるのは harness2 側で、`.harness2/scripts/auto_merge.py:18` は「いまの head の CI が全部緑」を merge の条件にしている。`story_ci.py:1-14` は赤の job を 1 回走らせ直し、まだ赤なら ci-fix の agent にログを渡す。`verify_ci_route.py` も同じく緑を待つ。どれも `dependencies` に無い。帰結は 2 つ。(a) 人間が手で押せば赤でも merge できる。(b) ADR と関係の無い PR（CLAUDE.md の表を作り直す PR など）が赤になると、自動 merge が止まり、ci-fix の agent が呼ばれる。agent は check-adr.sh か ADR を「直して」緑にしうる（IMP-01 の手間を agent が受ける）。
- 直し方の案: `harness2 の auto_merge.py / story_ci.py` を reads の dependency として足し、W1・S1 の文言を「自動 merge が止まる（人間は手で merge できる）」に直す。IMP-01 に ci-fix の agent が検査側を直す経路を `undefined` として書く。
- 処置: fixed: 依存 D6（auto_merge.py・story_ci.py。根拠 E8）を足し、W1・W2・S1・R2・L1 の「merge できない」を「CI が赤くなり自動 merge が止まる（人間は手で merge できる）」に直した。IMP-01 の undefined に ci-fix の agent がどちらを直すか決まっていないことを書いた

## R2 — IMP-01 が挙げた結合は 2 つだが、検査が掴む場所は 4 つある
- 対象: impact.findings[IMP-01].text / rules[R2].legit / lost[L1].after
- 種類: 根拠（範囲の取りこぼし）
- 指摘: design.md D9（:157-160）では、本番の口は次の 4 つを見る。(1) CLAUDE.md の表の行、(2) AGENTS.md の節の行、(3) `ci.yml` の `chain:` の中の `- run: ./tools/check-adr.sh --self-test && ./tools/check-adr.sh` の行がちょうど 1 つ、(4) 6 つのファイル名がある（spec の「CH12 で置いた README と 5 件の ADR がある」）。IMP-01 と L1 は (1)(2) だけを書き、R2.legit は ADR でない .md を加えている。(3) は ci.yml の chain job を触る正当な PR（他の検査と同じ 2 段に分ける、`if:` を付ける、ほかの job を挟む）で落ちる。(4) は ADR の slug を直す改名や統合で落ちる。R1.legit の「止まる正当な操作を見つけていない」は、(4) があるので言い切れない。README は「消さず置き換え済みにする」と決めているので止まる操作は減るが、改名は止まる。重大度は low で妥当（検査を同じ PR で直せば進める。記録を捨てる道は要らない）。ただし理由は「結合先が 2 つ」ではなく 4 つの結合、という形で書くのが正しい。
- 直し方の案: IMP-01 の text と L1.after に (3)(4) を足し、R1.legit の括弧を「ADR の改名・統合は止まる」に直す。
- 処置: fixed: IMP-01 の text・derived と L1・R2 に、chain job の check-adr.sh の行と CH12 の 6 つのファイル名の 2 つを足した（4 か所）。rules R1 の legit を「改名・統合は CI の停止（R2）で止まる」に直した。重大度は low のまま

## R3 — D4 の根拠が読み違いで、st53 は chain job も触る
- 対象: impact.dependencies[D4].assumes / evidence[E7] / coverage.excluded[ch10・st53]
- 種類: 根拠
- 指摘: D4 は「ch10 は rust の job、st53 は組み立ての job で、どちらも chain job ではない」と言い、E7 は ch10 の proposal が st53 を説明した行（`proposal.md:50`、孫引き）である。ところが `openspec/changes/st53-license-bundle/tasks.md:214` は「`chain` の job（全 PR）: `tools/check-spdx.sh --self-test` と `check-spdx.sh`」を足すと書いている。main の chain job（`ci.yml` の check-spdx の 2 行）には既にあるので、いまの重なりは消えている。それでも「chain job ではない」は誤りで、`holds: yes` を支える quote になっていない。もう 1 点、`git diff origin/main...feat/ch10-emulator-start-timeout -- ci.yml` を見ると、ch10 は `install-windows` の `check_scenarios.py . st55-server-installer` の行（:464）の `if:` も書き換えている（rust の job に足す 2 行のほかに）。chain job の外だが、proposal の範囲と差分が違う。結論（重ならない）は変わらないが、根拠は branch の差分と st53 の tasks.md で取り直すのが正しい。
- 直し方の案: D4 の evidence を st53 の tasks.md:214 と ch10・st56 の branch の ci.yml の差分に差し替え、assumes を「chain job を触るのは st53 の check-spdx だけで main に入済み」に直す。
- 処置: fixed: 依存 D4 の assumes を「chain job を触るのは st53 の check-spdx だけで main に入済み。ch10 は rust の job と install-windows の if:」に直し、根拠 E7 を st53 の tasks.md:214 に差し替えた。coverage.excluded の理由も同じに直した

## R4 — rollback の applies:false の理由が Q3・Q4 の前のまま
- 対象: impact.situations[rollback].why
- 種類: 言い切り
- 指摘: 「戻すときは docs/adr/ と台本を消し、CLAUDE.md の 1 行を消すだけ」。いまは AGENTS.md の 1 行と `ci.yml` の 1 行も戻す対象で、検査（引数なしの本番）が 4 つとも見ている。一部だけ戻すと chain job が落ちる（例: docs/adr/ だけを revert → CLAUDE.md の行と AGENTS.md の行を探して落ちる。`ci.yml` の行だけを消すと check-adr.sh の自己検査が `ci.yml` の欠落を確かめて落ちる）。「だけ」は限定を落とした言い切り。データ・移行に触らない点は正しいので `applies: false` のままでよいが、why は 5 か所を一緒に戻す、と書くべき。
- 直し方の案: why を「docs/adr/・tools/check-adr.sh・CLAUDE.md・AGENTS.md・ci.yml の 1 行を同じ PR で戻す。一部だけ戻すと chain job が落ちる」に直す。
- 処置: fixed: situations の rollback の why を、5 か所を同じ PR で戻す（一部だけ戻すと chain job が赤くなる）に直した

## R5 — overview・scope の「CI が落とす」は、chain job の秘密が無い場合を落としている
- 対象: impact.overview / scope.covers[2]
- 種類: 言い切り
- 指摘: 「後の ADR の一覧の書き忘れはその PR の CI が落とす」。check-adr.sh 自体は chain job で HARNESS2_TOKEN と無関係に走る（`ci.yml` の check-private の直後に置く。token の有無で分岐するのは後ろの harness2 の段だけ）ので、この点は正しい。ただし R1 のとおり、落ちた CI を merge から外すのは自動 merge の条件で、GitHub の保護ではない。「CI が落とす」を「CI が赤くなる（自動 merge は止まる）」と言い直さないと、W1 の読み手は人間の merge も止まると受け取る。
- 直し方の案: overview の該当文を「その PR の CI が赤くなり、自動 merge は止まる」に限定する。
- 処置: fixed: overview を「CI を赤くし、自動 merge が止まる（GitHub の保護ではないので、人間は手で merge できる）」に直し、scope.covers の CI の行にも同じ限定を足した
