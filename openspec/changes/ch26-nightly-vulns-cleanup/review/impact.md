# impact-review — ch26-nightly-vulns-cleanup

seen: IMP-01 medium, IMP-02 medium, IMP-03 low, IMP-04 low

## R1 — 「本人だけ」が、どこにも無い事実（repo の公開範囲）を言い切っている
- 対象: impact.lost[0].harm.who / impact.findings[IMP-03].harm.who / impact.findings[IMP-04].harm.who
- 種類: どれくらい困るか
- 指摘: lost の harm.who は「夜間の検査の結果と issue は本人の repo にだけ出る」と書く。repo の公開範囲は premises.md・design.md・docs/changes/CH26.md のどこにも書かれていない（grep で公開範囲の記述なし）。IMP-03・IMP-04 の「本人だけ」も同じ。repo が公開なら、例外にした 2 件の勧告の一覧が他人にも見える。確かめていない事実なので `unknown` に置くべき。
- 直し方の案: 「本人だけ」を外して「repo の公開範囲は確かめていない」を unknown へ移し、who は「夜間の検査の結果を見る人」に留める。
- 処置: fixed: lost と IMP-03・IMP-04 の harm.who から「本人だけ」を外し、repo の公開範囲を確かめていないことを unknown に移した

## R2 — situations の can が、まだ走っていない台本の結果を「できる」と書いている
- 対象: impact.situations[update / rollback / carryover / mixed].can
- 種類: 言い切り
- 指摘: tasks.md 3.1 の check-ch26.sh minor-swap は未実施で、`check-ch26.sh` も change の中にまだ無い（tasks はすべて未チェック）。一方 D7（17.12 の pg_dump の退避を 17.6 の pg_restore が読める）は `holds: unknown` のまま。同じ分析の中で D7 は未確認、situations の mixed.can は「戻す」と断定、と食い違う。situations の can は仕様の Scenario（spec.md）を写しただけで、その場面で本当にできるかは未確認。rollback の can「17.12 で足した記録も残る」も、確かめるのは Docker の Linux 像だけ。
- 直し方の案: can に「台本（tasks 3.1）で確かめる予定。Linux の PostgreSQL の部分のみ」と限定を付け、D7 の `unknown` と揃える。
- 処置: fixed: update・rollback・carryover・mixed の can を「仕様の約束。台本（tasks 3.1）で確かめる予定で、まだ走らせていない」に限定し、basis を spec にした（D7 の unknown とそろえた）

## R3 — acceptance の「8 つの Scenario」が実物の数と合わない
- 対象: impact.acceptance[0].text
- 種類: 言い切り
- 指摘: 同じ分析の coverage.examined は「12 の Scenario」と書く。実物は specs/distribution/spec.md が 8、specs/dependency-vulnerability-scan/spec.md が 4 で計 12。acceptance の「8 つの Scenario」は distribution だけを数えており、脆弱性検査側の 4 つの Scenario が通ることを完了の条件から落としている。
- 直し方の案: 「12 の Scenario」にそろえるか、どの Scenario を指すかを書く。
- 処置: fixed: acceptance を「12 の Scenario（脆弱性の検査 4・配布 8）」に直した

## R4 — IMP-01 は「確かめていない読み」を理由に medium とし、ほかの道（本人に聞く）が仕様から見えない
- 対象: impact.findings[IMP-01]（severity・verify・options）
- 種類: 重大度 / 閉じ方
- 指摘: IMP-01 は basis guess で、前提 P7（確かさ 1）が外れたら例外の理由が消える。unknowns は書いてあるが、verify の「本人の profiles.ini の IsRelative=0 の行を見る」は acceptance の確認バッチ項目（acceptance[2]）に入っていない。確認バッチが問うのは更新の結果と issue を閉じたかだけ。このままでは例外を 90 日置いたまま、反転条件（design.md:37「本人以外も書ける置き場の Firefox の profile を読む使い方が本人にある」）を誰がいつ見るかが決まらない。locate.rs:208 の `IsRelative=0` の読みは確かにあるが、E4 の行だけでは「利用者の置き場の外も読む」までは言えない（絶対パスを辿るのは前後の行。premises P16 は試験 :359-379 も根拠に挙げる）。
- 直し方の案: acceptance の確認バッチに「Firefox の profiles.ini に IsRelative=0 の行があるか」を足し、derived の根拠に P16 の試験の行も挙げる。
- 処置: fixed: IMP-01 に locate.rs の試験（E12）を根拠に足し、verify が確認バッチに無いことを unknowns と open（未解決）に書いた。tasks に確認の項目を足すのは仕様の変更なので、この段ではしない（違和感で返れば spec の段が足す）。重大度は medium のまま（kept の理由: 外す期限は 90 日で、反転条件が design D2 にある）

## R5 — IMP-02 の「いま進めて安全」の手当てが、本人の PC に配布版が入っていない場合に空になる
- 対象: impact.findings[IMP-02]（options・unknowns・harm）
- 種類: 閉じ方
- 指摘: IMP-02 の手当ては本人の PC での確認だけ。unknowns に「本人の PC に 17.6 同梱の配布版が入っているか確かめていない」とあり、acceptance[2] は「入っていない・同じ版」も答えとして受ける。入っていなければ、17.12 の配布物は Windows の更新の道を一度も通らないまま配られ、CI も小さい版が違う組を走らせない（E7）。このとき「だれがいつ確かめるか」（undefined）に答えが無い。harm.unknown に「Windows に固有の落ち方があるか」とあるが、インストーラは落ちたら戻す（E6）という前提で軽くしており、その戻しの道も同じ未確認の Windows の道の上にある。
- 直し方の案: 入っていない場合の持ち越し先（次に配布版が入った PC での確認の期限、または CI で base の lock から古い zip を組む change）を acceptance か IMP-02 の interim に書く。
- 処置: fixed: IMP-02 の unknowns と open（未解決）に「配布版が入っていなければどこでも確かめない・持ち越し先と期限が無い」を書いた。持ち越し先を決めるのは仕様の変更なので、この段ではしない
