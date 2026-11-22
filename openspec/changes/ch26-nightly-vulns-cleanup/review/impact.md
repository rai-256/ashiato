# impact-review — ch26-nightly-vulns-cleanup

seen: IMP-01 medium, IMP-02 medium (reflected), IMP-03 low, IMP-04 low, IMP-05 medium, IMP-06 low

## R1 — W2 の「改行だけ変えた → 今のまま」が、設計・仕様と食い違う
- 対象: walkthroughs[W2].figure.rows（4 行目）と cases。あわせて impact.rules[R1].legit
- 種類: 言い切り
- 指摘: W2 は「改行だけ変えた」PR を「古い側 = PR の lock・CI は今のまま」と書く。だが design D7 は、`changed` が分岐点からの差分に lock の名前があれば `pgsql=true` にし、`decide` が「同じ中身」（行末の CR を落として比べる）と言えば `package-windows` を落とす、と決めている（tasks 4.2 も同じ）。改行だけの差分は `changed` では true、`decide` では same になるので、CI は落ちる。W2 の表は仕様の Scenario「改行だけが違う 2 つの lock は、同じ中身と判定する」の「同じ」を「今のまま通る」と読み替えている。R1.rule の列挙（取れない・大きい版が違う・版が前と今でない）にも「同じ中身」が無く、R1.legit にも載っていない。
- 直し方の案: W2 の行を「CI が落ちる（同じ中身）」にするか、仕様側で改行だけの差分は `pgsql=false` に倒す。どちらにしても R1 の rule と legit に同じ中身の落ちを足す。
- 処置: fixed: W2 の表の「改行だけ変えた」を「落ちる（変えたのに中身が同じ）」にし、cases・ci.2・ci.3・R1 の rule と legit に改行だけの PR が落ちることを書いた

## R2 — 「緑なら小さい版の違う配布物の間の更新が通った」は、版が同じ lock の変更では言えない
- 対象: impact.findings[IMP-02].resolution（status: reflected）。acceptance[1]
- 種類: 直した・確かめたという申告
- 指摘: resolution は「2 つの zip の版が違えば CI が落ちるので、緑は小さい版の違う配布物の間の更新が通ったことを含む」と書く。しかし `decide` は中身が違い大きい版が同じなら `swap` を返し、`check-zips` が見るのは「古い側の版 = 前の lock の版・新しい側の版 = 今の lock の版」だけ（design D8）。dlls= や url・sha256 だけを変えて version を変えない PR（同じ 17.12-1 のまま）は `swap` になり、古い側も新しい側も 17.12-1 で `check-zips` を通り、緑になる。「版が違う」ことは強制されていない。この change 自身の PR（17.6-1 → 17.12-1）では成り立つが、IMP-02 の resolution は一般の約束として書かれている。W1 の problem の case（「17.12 → 17.12 の更新になり…版の確かめがこれを見つけて落とす」）も、前の lock 自体が同じ版のときは見つけない。
- 直し方の案: resolution と W1 の文を「前の lock と今の lock で version が違うときは」に限る。または `check-zips` に古い側 ≠ 新しい側を足すかを決め、足さないなら IMP に書く。
- 処置: fixed: IMP-02 の resolution を「この PR（version を変える）の緑は」に限り、unknowns に version を変えない lock の PR では 17.12→17.12 で通ることを足した。W1 の problem の case にも同じ限定を書いた

## R3 — D4（走っている CH24）が、同じ go.json の coverage と食い違い、実物でも古い
- 対象: impact.dependencies[D4]（holds: unknown）、evidence[E10]。coverage.examined[1]
- 種類: 根拠
- 指摘: D4 は「走っている CH24 …osv-scanner.toml と Cargo.lock を自分だけが変えるか unknown」と書くが、coverage.examined[1] は「CH24 は 2026-11-21 に merge 済み」と書く。origin/main の実物でも `git log origin/main -- osv-scanner.toml` の先頭は `2c8fc1db4 ch24: remove zip vulnerability exceptions` で、zip の例外の行（GHSA-8hxf-qr8p-62x4 など）は消えている。つまり D4 の「後から merge する側の rebase で衝突」は起きない。いま残る依存は、この枝の osv-scanner.toml・Cargo.lock が origin/main より古いこと（`git diff HEAD origin/main` で osv-scanner.toml が 28 行、Cargo.lock が 372 行違う）で、これを D4 は書いていない。tasks の Global Constraints は「実装は main の上で始める」としているが、「既にある行を動かさない」の基準になる `git merge-base origin/main HEAD` の元の版が、実装を main の上でやり直すかどうかで変わる。
- 直し方の案: D4 を「CH24 は merge 済み。この枝は origin/main より古く、実装は main の上で取り直す」に書き換え、holds と verify を実物に合わせる。
- 処置: fixed: D4 を「main に merge 済みの CH24」に直し、この枝が main より古いことを affects に書き、IMP-07（low、observed）で受けた

## R4 — D7 の leads_to が、いまの IMP-02 では受けていない帰結を指している
- 対象: impact.dependencies[D7]（holds: unknown, leads_to: IMP-02）、findings[IMP-02]
- 種類: 閉じ方
- 指摘: D7 の affects は「更新が落ちたときの戻しで記録が戻らない」（17.12 の pg_dump の退避を 17.6 の pg_restore が読めるか）。これは記録が戻らない帰結で、配った後では直せない側の話。ところが IMP-02 は第 1 回の合意の後、「Windows の配布物どうしの更新を機械で確かめない」に作り直され、reflected になった。IMP-02 の text・derived・resolution に、退避の互換が読めないときに何が起きるかは残っていない。いま D7 の帰結を持つ IMP は無い。確かめは tasks 3.1 の台本と CI の Scenario「更新の移行が落ちると、退避から DB を戻し…」の予定で、どちらもまだ走っていない。memory[one_way_door].text は「後から戻せる…17.12 で使った置き場は 17.6 でも使える（台本で確かめる）」と、確かめる前の約束を事実として書いている。
- 直し方の案: 退避の互換を独立した IMP にして unknowns に「台本・CI はまだ走っていない」と書くか、D7 の leads_to を外して verify だけに残す。one_way_door の text は「台本で確かめるまでは未確認」に限定する。
- 処置: fixed: D7 の帰結（17.12 の退避を 17.6 が戻せるか）を IMP-02 の resolution と unknowns に入れて受け、memory の one_way_door を「確かめる予定で、まだ走らせていない」に直した

## R5 — overview の「今の DB はそのまま使える」は、走らせていない確かめを言い切っている
- 対象: impact.overview
- 種類: 言い切り
- 指摘: overview は「配布物の PostgreSQL は 17.12 になり、今の DB はこの分析と台本の範囲ではそのまま使える」と書く。しかし situations（update / rollback / carryover / mixed）は、どれも「台本で確かめる予定」「まだ走らせていない」と書いており、D7 は holds: unknown、coverage.not_examined も 17.12 の zip と Windows の CI の結果を「実装の後」と書く。台本は未実装で、「台本の範囲では使える」は結果ではない。sections.uses の「次に検査が赤になったら、それは新しい勧告か例外の期限切れだと分かる」も、lock の検査・ネットワーク失敗・IMP-01 の同じ ID の書き換えを落とした断定。
- 直し方の案: overview を「そのまま使える見込みで、台本と CI の run で確かめる」に直し、sections.uses の文に「ほかの理由で赤になることもある」を足す。
- 処置: fixed: overview を「台本と CI で確かめる（まだ走らせていない）」に、sections.uses の tldr を「まず〜を疑える」に直した

## R6 — IMP-05 の medium は、この change の目的に当たる脆弱性が残る話なのに、W1 と sections で小さく見える
- 対象: impact.findings[IMP-05].harm・severity。walkthroughs[W1].exceptions
- 種類: 重大度
- 指摘: IMP-05 の harm.link は「直したはずの PostgreSQL の脆弱性が、その PC に残る」。この change の主目的（PostgreSQL を 17.12 にして夜間の検査を緑に戻す）が、配った PC では達成されていないのに、本人も CI も気づけない形で、「利用者が自分で気づけない」に当たる。一方で IMP-05 の unknowns は「入れ替え損ねる道が実際にあるかは確かめていない」で、道が無ければ消える。medium のままで妥当だが、高く置かない根拠は「コードはプログラムの置き場ごと入れ替える（P12・P17）」だけで、E 番号の evidence が付いていない（basis は derived）。遅れの検査（check-pg-minor.sh）は lock を見るので気づかない、という unknowns の 2 行目も「確かめていないこと」として書かれているが、tools/check-pg-minor.sh を読めば言える。
- 直し方の案: P12・P17 の根拠行を evidence に足して basis を observed にするか、入れ替え損ねる道を 1 つ探して unknowns を減らす。
- 処置: fixed: IMP-05 に E15（戻しの試験が pgsql を含めて置き場ごと戻す）と E16（check-pg-minor.sh は lock を読む）を足し、unknowns の 2 行目を確かめた事実に書き換え、W1 の例外に脆弱性が残っても気づけないことを書いた。重大度は medium のまま（kept の部分: 入れ替え損ねる道は見つけておらず、基本の道は置き場ごと入れ替える）

## 前回の確かめ
- 前回（第 1 回の合意の前）の R は、仕様が変わったため引き継がない。
