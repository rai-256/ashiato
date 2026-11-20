# impact-review — ch21-trial-use-before-verify

seen: IMP-01 medium, IMP-02 medium, IMP-03 medium, IMP-04 medium, IMP-05 high, IMP-06 low, IMP-07 medium

## R1 — IMP-05 を medium・guess に置いたまま、memory が「渡さない」と言い切っている
- 対象: impact.memory[1].text / impact.findings[IMP-05].severity
- 種類: 重大度
- 指摘: memory[1] は「本人の合言葉と確認バッチの DB は渡さない」と書くが、IMP-05 自身は「案内でしか止めていない・席が外を読むか／auto が止めるかは確かめていない」（guess）である。同じ分析の中で言い切りと未確認が食い違う。
  main の `.env` の場所は席から簡単に辿れる。`tools/verify-prep.sh:25-29` は `git rev-parse --git-common-dir` から main の根を出して `.env` を写している。
  席の使い捨て作業ツリーも同じ git の共通の置き場を持つので、同じ手順で届く。
  読まれた中身はモデルへの入力になり、後から取り戻せず、本人は気づけない。high の条件（利用者が自分で気づけない・配った後で直せない）に当たる。
  確認も実地 1 回の journal を見るだけで、1 回で「読まない」は言えない。
  「medium + guess」は、前提が外れたら消える推測の high を unknowns で受けた形としては成り立つ。ただしその場合、memory の「渡さない」と IMP-05 の `options` の「このまま受け入れる」を、いま進めてよい根拠として並べてはいけない。
- 直し方の案: memory[1] の「渡さない」を「渡さない作りにしたが、席が外を読まないことは未確認」に直す。IMP-05 は、実地で外を読まなかったと見るまでは配らない（high の閉じ方）に置く。
- 処置: fixed: IMP-05 を high に上げ（E13 で main の .env に辿れることを根拠に足した）、if_accepted（残るリスク・範囲・本人が CH21 の確認バッチで journal を見て判断し直す）を書いた。memory[1] の「渡さない」を「写さず案内で止めるだけ・機械では止めていない」に直した

## R2 — one_way_door の「戻せないのは題だけ」は IMP-05 と食い違う
- 対象: impact.memory[2].text
- 種類: 言い切り
- 指摘: 「後から戻せないのは公開の repo に出た題だけ」は限定を落とした断定である。IMP-05 の harm は「読んだ中身はモデルへの入力になる」と言っており、こちらも戻せない。
  また、本人の PC の収集アプリが Sandbox の窓を前景の記録に入れる件（lost[1].harm.link）も、いったん入った記録は取り消しにくい。
  premises 以外の記録（`photo-delete-story-gap` のとおり、本人の記録を消す口は持たない Story が多い）には、この change から言える手がかりが無い。ここでは「題だけ」と言える材料が無い、という指摘にとどめる。
- 直し方の案: 「題だけ」を「この change が公開側に出すのは題だけ」に狭め、モデルへ渡る分と、本人の記録に入る分を別の扉として挙げる。
- 処置: fixed: one_way_door を、公開に出た題・モデルに渡った入力（IMP-05）・ライフログに入った窓の記録（IMP-07）に直した

## R3 — S3 の「前景の窓の記録に Sandbox が入りうる」が IMP にならず、重大度も無い
- 対象: impact.lost[1].harm.link / impact.findings
- 種類: 新しい帰結
- 指摘: lost[1].harm は「本人の PC の収集アプリが記録する前景の窓に Sandbox が入りうる」と書き、`design.md` の Risks にも同じ文がある。しかし findings に IMP が無く、閉じ方も重大度も無い。
  本人のライフログ（形 A のデータ）に、AI の席が起こした窓が記録として残る。1 日 1〜5 回（premises.md:21）の確認バッチごとに起き、本人が気づく手がかりも、消す手順も書かれていない。
  who を「本人だけ」とするのは、記録が本人の DB に残る点では合っている。ただ「あとで消せるか」が unknown にも出ていない。
- 直し方の案: IMP を 1 つ立てる（medium 以上）。収集アプリの記録から席の窓を除く、記録に入ると本人に知らせる、受け入れる、のどれにするかを options に置く。
- 処置: fixed: IMP-07（Sandbox の窓が本人のライフログに入りうる。medium・guess）を足し、lost[1].harm.link を IMP-07 に繋いだ

## R4 — R7（片付けは自分の id だけ）の `legit` が「見つけていない」、`blocks_legit: false` は IMP-04 と食い違う
- 対象: impact.rules[R7].legit / blocks_legit
- 種類: 言い切り
- 指摘: D3 は「起こした id だけを `wsb stop --id` で止める」で、前の回の席が閉じ損ねた Sandbox は自分の id ではないので、次の回の片付けでも閉じられない。
  この change が残した Sandbox を、この change の道では誰も片付けない。これは正当な利用が止まる例で、IMP-04 が書いた帰結そのものである。
  R1 の `legit` では書いているのに、R7 では「この分析では見つけていない」と `false` になっている。
- 直し方の案: R7 の `legit` に「前の回が閉じ損ねた自分の Sandbox は、次の回も片付かない」を書き、`blocks_legit` を `true` にして IMP-04 に結ぶ。
- 処置: fixed: R7 の legit を「前の回が閉じ損ねた Sandbox は誰も閉じない」にし、blocks_legit を true、筋書き S4（IMP-04）を足した

## R5 — rollback の `cannot`「見つけていない」は、題が公開済みという帰結を落としている
- 対象: impact.situations[rollback].cannot
- 種類: 言い切り
- 指摘: 「CH21 を戻すと…手元の結果は repo の外に残る」「cannot: この分析では見つけていない」は、戻した後でも、すでに公開の repo に出た verify PR の題は消えない（memory[2] の扉そのもの）、という帰結を書いていない。
  `can` が「元の形に戻る」と言えるのは準備の挙動だけである。
- 直し方の案: `cannot` に「公開した題・PR 本文は戻らない」を書く。
- 処置: fixed: rollback の cannot を「すでに出た題は公開の repo の履歴に残る」にした

## R6 — overview の「AI が配る形で一度使い」は画面の相手には当たらない
- 対象: impact.overview / impact.problem
- 種類: 言い切り
- 指摘: PC の相手は配る zip を網を切った Sandbox へ入れる（D5・D6）。画面の相手は D7 のとおり、`tools/stack.sh up` で docker の DB と verify-prep が組んだサーバ・画面を立てる開発の形で、インストーラも組み込みの配る DB も通らない。
  動機の #329 は「配る形を入れて普通に使った」ときの不具合で、画面の席はその型を見る席ではない。overview は両方を「配る形」と言っているので、画面側のできることを広く見せている。
  `design.md:D2` の注記（配る zip のサーバは画面を中に持つ）は PC の相手の中で画面に辿り着ける、という話で、画面の相手の話ではない。
- 直し方の案: overview を「PC は配る形で、画面は開発の形で」と分けて書く。
- 処置: fixed: overview を「PC は配る形を Sandbox に入れ、画面は専用の開発の形で立てる」に直した

## R7 — IMP-03 の影響が錠を待つものだけで、確認バッチを組むグラフ側への効きが落ちている
- 対象: impact.findings[IMP-03].text / harm.link
- 種類: 根拠
- 指摘: 錠は `verify.py:108`（verify_batch.sh）と `verify.py:364-370`（record-run）の両方を囲み、準備が延びれば待たされるのは記録と fix の撮り直しだけでなく、同じ確認バッチの組み直し（ci_fix・conflict_fix の後の再 batch）も含む。
  `effects.run` は `subprocess.run` で時間制限が無く、flock も待ちの上限が無い（`effects.py:155-186`）。最大 6,540 秒の間、グラフの 1 つの節が止まる。
  `verify.py:98-99` のコメントのとおり、組んでいる thread は `autoverify.building` に数えられ、次の自動起動を止める。
  待つのは「記録と fix」だけ、と言うと軽く見える。
- 直し方の案: harm.link と text に、確認バッチの組み直しと次の自動起動も待つことを足す。
- 処置: fixed: IMP-03 の本文と harm.link、D2 の affects に ci_fix・conflict_fix の後の作り直しと次の確認バッチの自動の起動を足した

## R8 — 依存の見落とし: docs/testing.md を触る CH23
- 対象: impact.dependencies / targets[T5]
- 種類: 依存の見落とし
- 指摘: T5 は `ci.yml` と `docs/testing.md` を触ると書くが、D5 は CH22 の `ci.yml` だけを挙げている。
  走っている CH23 の tasks（`ch23-vulns-nightly-issue-fix/tasks.md:60`）も `docs/testing.md` の 1 行を書き足す。CH22 も表の `install-windows` の行を書き足す（tasks.md:99）。
  行は別で文字の上では当たらないが、`no_dependents_why`「重なりは D5・D6 の依存で扱う」は `docs/testing.md` の重なりを扱っていない。
  ci.yml の足す位置は、CH22 が `ci.yml:149`、CH21 が `:158` の後で、離れている（D5 は合っている）。
- 直し方の案: T5 の `docs/testing.md` に CH22・CH23 を依存に足し、どの行・どの節が重ならないかを書く。
- 処置: fixed: 依存 D10（CH22・CH23 の docs/testing.md の書き足し。E11・E12）を足した

## R9 — D2（錠）の `holds: no` は、E2 が主張を支える範囲を超えている
- 対象: impact.dependencies[D2]
- 種類: 根拠
- 指摘: E2 は `with verify_tree_lock(ctx):` の 1 行で、「準備は組み立ての時間で終わる」という assumes を壊す根拠にはならない。壊すのは、これから足す準備の延びのほうである。
  いまある準備の長さ（cargo build --release・npm ci・gradle）が既に分単位であることを、`observed` として書いていない。延びの量は IMP-03 の unknowns にある。
  `basis: observed` と書くなら「錠は verify_batch.sh 全体を囲む」までにとどめ、延びは `derived` と分けるほうが正確である。
- 直し方の案: D2 の `affects` を「錠は準備を囲む（observed）。延びた分だけ待つ（derived）」に分ける。
- 処置: fixed: D2 の basis を derived にし、refs（spec の上限）と E1・E2 を分けて書いた


## R10 — IMP-05（high）の閉じ方が、確かめる前に有効にする形のままで、いま進めて安全とは言えない
- 対象: impact.findings[IMP-05].options[0] / if_accepted / verify
- 種類: 閉じ方
- 指摘: 推奨の選択肢は「受け入れて進め、実地の記録で外を読んでいないと見てから確認バッチで有効にする」だが、`if_accepted` の recheck は「CH21 の確認バッチ（実地の 1 回の journal を見たとき）」である。
  「見てから有効にする」と「CH21 の確認バッチで見る」は順序が合わない。CH21 を merge すると `verify-batch.sh` は次の確認バッチから席を呼ぶ。有効・無効を切る機械の口（環境変数や設定）は spec・tasks に無い。
  本人が journal を見る前に、席が本人の合言葉を読める状態が配られる。IMP-05 自身の unknowns も「数回読まなくても以後は読まない、とは言えない」と書いており、1 回の journal は閉じ方にならない。
  また `if_deferred`（interim / safe_now）が無く、先送りの手当ても書かれていない。
- 直し方の案: 席を呼ぶかどうかの切り替え（既定は呼ばない）を spec の Requirement に足し、journal を見てから本人が有効にする順にする。または 3 つ目の案（本人の .env が無い別の置き場で走らせる）を推奨に移す。
- 処置: fixed: recheck の時期を「CH21 の merge の前（下流の実地の 1 回 real-run の journal を見たとき）」にし、residual と undefined に「呼ぶかどうかを切り替える口が仕様に無い」を書き、選択肢に「仕様を直す（切り替えを足し既定は呼ばない）」を足した。reading の段は仕様を変えないので、切り替えの追加は本人が IMP-05 で「仕様を直す」を選んだときに spec の段が行う

## R11 — IMP-07 の「窓を開かない案に戻す」は、仕様の前提と両立しない選択肢になっている
- 対象: impact.findings[IMP-07].options[1] / harm.who
- 種類: 根拠
- 指摘: `design.md:D4` は「窓を開く前は ExistingLogin で走らず、System では利用者のデスクトップが無い。収集アプリは利用者のセッションで起こさないと『入れて普通に使う』形にならない」と書く。窓を開かない案では PC の席そのものが成り立たない可能性が高いが、options はそれを書かずに並べている。実際に選べる案かが、確かめられていない。
  harm.who「本人だけ（記録は本人の画面にだけ出る）」は、記録の出先（集計・同期・書き出し）を確かめた形跡が無い。change・premises・design のどれにも無い事実の言い切りである。`unknown` に移すのがよい。
- 直し方の案: options[1] に「PC の席をやめる（Q4 の別の案）」と実際の代償を書き、harm.who の「画面にだけ出る」を unknown に移す。
- 処置: fixed: 窓を開かない案に D4 の前提と両立するか未確認と書き、harm.who の言い切りを外して unknown に「だれに見えるか」を移した

## R12 — D10 は CH22 の書き足しとの重なりを「別の場所」と言うが、節の見出しを足す共有の追記の検査には触れていない
- 対象: impact.dependencies[D10]
- 種類: 依存の見落とし
- 指摘: D10 の evidence（E11・E12）は CH22・CH23 が「自分の行」を書き足すことまでしか言わない。CH21 が `docs/testing.md` に足す「1 節」の位置（表の途中か末尾か）は spec・tasks のどこで決めているか分からない。
  3 つの change が同じファイルの末尾に足すと、merge で衝突する（AGENTS.md の共有ファイルの末尾の取り合いと同じ型）。末尾でなく自分の節を持つ位置に置くと決めてあるなら、その行を D10 の根拠に足す。
- 直し方の案: tasks の該当 Task に、足す位置（どの見出しの後か）を 1 行で書き、D10 の根拠にする。
- 処置: fixed: D10 を holds: unknown にし、節の位置が決まっていないことを open（未解決）に出した。tasks を直すのは spec の段（reading は仕様を変えない）

