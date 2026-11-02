# impact-review — st53-license-bundle

seen: IMP-01 high, IMP-02 high, IMP-03 medium, IMP-04 medium, IMP-05 low, IMP-06 medium

## R1 — IMP-02 の前提は「残ると」ではなく、いまの実機で既に成り立っている。high は妥当だが basis と unknowns が弱い
- 対象: impact.findings[IMP-02].text / .derived / .unknowns / .feasibility、dependencies[D5].holds
- 種類: 根拠 / 重大度
- 指摘: この作業木で `gh auth status` を見ると、`rai-256` の 1 つの account が `/home/yosis/.config/gh/hosts.yml` にあり、token の scopes は `repo`・`workflow`（クラシックの管理者相当）。`git config` は `credential.https://github.com.helper = !/usr/bin/gh auth git-credential` で、エージェントの git push も同じ token を使う。つまり「本人が残せば」ではなく、いまの全エージェントの資格情報がそれである。IMP-02 の `derived` は条件文のままで、E7・E8 は設計文と AGENTS.md の一般論しか指さず、hosts.yml や credential helper の実物は evidence に無い。high は支える（承認者を外す・保留の公開を承認する経路が、仕様から見える道では切り替えを正しく済ませても残りうる）が、unknowns に挙げた「どこで管理操作をするつもりか」だけでなく、「切り替えの手順が、この helper と hosts.yml の active account をどう差し替えるか」が仕様にも手順にも無いことが本体。
- 直し方の案: E に `gh auth status` の事実（scopes・helper）を足して前提を observed にし、unknowns に credential helper を足す。
- 処置: fixed: IMP-02 の text を「いまの資格情報が管理者そのもので、git の push も gh auth git-credential を通る」に直し、unknowns に credential helper と active account の差し替えを足した。basis は derived のまま（hosts.yml と helper は repo の外で、evidence の loc に置けない）

## R2 — IMP-01 の release_ok_why は、IMP-02 が未決のままでは「進めて安全」を支えない。IMP-02 には閉じ方が無い
- 対象: impact.findings[IMP-01].if_accepted.release_ok_why / .derived、findings[IMP-02].if_accepted
- 種類: 閉じ方
- 指摘: IMP-01 の derived は「承認できる資格情報を本人だけが持つなら」という条件つきで、その条件が IMP-02 で未成立と書かれている。release_ok_why が頼る 2 つの確かめのうち `agent-credential` は IMP-02 自身が「今の 1 つの資格情報しか見ない」と言う（design.md:157、tasks.md:167 も `gh api repos/<repo>` の `.permissions` 1 回）。もう 1 つの 11.2（tasks.md:221）が見るのは「絞った token で承認が断られる」で、hosts.yml や `GH_TOKEN` に管理者の token が残っていないことは見ない。よって順を固定して 11.2 を通しても、IMP-02 の経路が閉じたことにはならず、鍵を置いてよい根拠にならない。IMP-02 側は `if_accepted` に residual はあるが `release_ok_why` が無く、IMP-01 が閉じる前提を持たないまま high が 2 つ並んでいる。
- 直し方の案: tasks 11.2 か 8.1 に「gh の全 account と `GH_TOKEN`・設定のファイルに管理者の権限が無い」確かめを足し、IMP-01 の release_ok_why をそれに依存させる。足せないなら IMP-01 の「鍵を置いてよい」を言い切らない。
- 処置: fixed: IMP-01 の if_accepted から release_ok_why を外し（「受け入れる（リリースしてよい）」を出さない）、residual に「IMP-02 が閉じるまで」と「11.2 は管理者の token が残っていないことを見ない」を書いた。tasks に確かめを足すのは仕様の変更なので、この段ではしない（IMP-02 の答えで spec の段が直す）

## R3 — IMP-03 の「unknown」は弱すぎる。再実行に Actions の書きが要るのはほぼ確実で、手当ての選択肢は一つに寄る
- 対象: impact.findings[IMP-03].unknowns / .derived、dependencies[D4].holds、situations 内 S5 の「と読む」
- 種類: 根拠
- 指摘: `gh run rerun` は REST の "Re-run failed jobs" を呼び、fine-grained token では Actions の write が要る（読みでは 403）。「知識。走らせていない」のまま unknown に置くと、D3 の権限の一覧（design.md:60-65）が最初から走らせ直しを落とす設計だったことが埋もれる。呼び出し元は E4（verify_batch.sh:244）と E5（story_ci.py:45）の 2 か所で読み方は合っているが、story_ci.py 側は戻り値で走らせ直せた run を数えるので「黙って捨てる」は verify_batch.sh 側だけ。medium は妥当。もう 1 つの unknown（Actions の書きが承認に届くか）は、承認の API が別の権限（Deployments）なので届かない見込みが強い。ただし Actions の書きは `workflow_dispatch` 等で公開以外の workflow を走らせる権限でもあり、nightly の Environment（ST54 で鍵が載る）に対して何が変わるかは書かれていない。
- 直し方の案: D4 を「要る（確かめは 11.2）」の向きに書き、権限に Actions の書きを足す案を既定に、nightly 側の影響を 1 行で確かめる。
- 処置: fixed: dependencies[D4] を holds: no・derived に、IMP-03 の text と unknowns を「要ると読む」「story_ci.py は数える」「nightly 側は見ていない」に直した

## R4 — 依存の見落とし: git credential helper が gh の login に束ねられている
- 対象: impact.targets[T5]、dependencies（D5 の周辺）
- 種類: 依存の見落とし
- 指摘: エージェントの push・印の push は `gh auth git-credential` 経由（上記 helper）なので、資格情報の切り替えは「gh の active account の差し替え」と同義で、別の account を足すだけでは active が変わらない。D3 の「ハーネスの token の差し替え」（tasks.md:203）はその置き場を決めていない。また T5 の searched は「gh の呼び出しを全部拾った」と書くが、dependencies には D4（rerun）だけで、`gh issue create/edit`（publish.sh:47-57、prepare.sh:114）、`gh pr close --delete-branch`（verify_batch.sh:274）、`gh pr ready`・`gh pr comment`（merge_gate.sh）が絞った権限（Issues・Pull requests・Contents）でできるかの突き合わせの結果が見えない。多くは通ると読めるが、通る根拠が findings にない。
- 直し方の案: 突き合わせた gh の呼び出しと必要な権限の表を dependencies か evidence に置く。
- 処置: fixed: dependencies に D9（PR・issue の gh の操作。D3 の権限で足りると読む）を足し、credential helper は IMP-02 の unknowns に置いた

## R5 — 依存の見落とし: ST54 の申し送りが依存に載っていない
- 対象: impact.dependencies、targets[T4]
- 種類: 依存の見落とし
- 指摘: docs/handoff/ST54.md は、ST54 が `nightly.yml` に携帯の APK を足し `check-artifacts.sh --channel nightly` の期待を 4 つに戻すと書く。その検査は ST53 が許諾の 3 点を足す対象そのもので（T4）、ST54 が携帯の APK の組み立てを足すとき、`ashiato.licenseAssets`・`SOURCE.txt` の commit 書式・`check-licenses.sh android` の節（tasks 10.1 は package-linux 側のみ）を満たさねば開発版が 3 点を欠く。分析の `rollback`・`mixed` はこの組み合わせを見ていない。また ST54 が nightly の Environment に鍵を置く点は、IMP-01 と同じ「AI が書いた変更が main に入ると鍵を読める」の別の口（D21 は意図的）だが、本 Story の資格情報の切り替えとの前後関係が書かれていない。
- 直し方の案: dependencies に ST54 を reader/writer として 1 件足し、「3 点と SOURCE.txt を ST54 が壊さない」を確かめる場所（ST54 の tasks への申し送り）を指す。
- 処置: fixed: dependencies に D10（ST54 が build.sh を通して APK を組む前提）を足し、ST54 との前後関係を coverage.not_examined に置いた

## R6 — 言い切り: situations.recovery.can は条件を落としている
- 対象: impact.situations[recovery].can
- 種類: 言い切り
- 指摘: 「本人が承認者の規則を外してしまっても、approvals が署名の前で止める」。approvals は GITHUB_TOKEN で Environment を読めること（IMP-04、未確認）に乗り、規則を外した状態では job が承認なしで始まり Secrets が job に渡ってから止まる（署名の前の step で止めるだけで、鍵は job の環境にある）。cannot 側の「書き換えられていれば効かない」は書かれているが、can 側も「止まるのは署名と公開で、鍵は job に渡っている」と書かないと、止めの強さを読み違える。
- 直し方の案: can に「鍵は job に渡った後で止まる」と、IMP-04 が未確認である旨を足す。
- 処置: fixed: situations[recovery].can に「鍵が job に渡った後で止まる」「IMP-04 は未確認」を足した

## R7 — 言い切り: IMP-02 の option 1 が解決を言い過ぎる
- 対象: impact.findings[IMP-02].options[0]
- 種類: 言い切り
- 指摘: 「管理者の操作はブラウザだけで行い…gh には管理者の token を置かない」としても、エージェントは本人と同じ OS の利用者で動くため、ブラウザのセッション・別の credential store・環境変数の経路が残る。option 2（別の OS の利用者）だけが利用者の分離に届く。option 1 は「gh の設定のファイルの経路を閉じる」までで、overview の scope.residual（管理者の資格情報がエージェントから届く限り…）とのつながりで読むべき内容。
- 直し方の案: option 1 の説明に「ブラウザ・環境変数の経路は残る」を足す。
- 処置: fixed: IMP-02 の option 1 に「ブラウザのセッションや環境変数の経路は残る」を足した
