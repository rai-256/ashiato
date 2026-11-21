# impact-review — ch22-soak-install-online-ci

seen: IMP-01 medium, IMP-02 medium, IMP-03 medium, IMP-04 low

## R1 — D3 の holds が assumes と逆向き
- 対象: impact.dependencies[D3].holds
- 種類: 根拠
- 指摘: D3 の assumes は「本物の api.github.com へ繋ぐことが正典で許されている」で holds は yes。だが挙げた E4（openspec/specs/distribution/spec.md:2523）は「自動の試験は本物の配布元に問い合わせない」で、いまの正典は許していない。affects 自身も「字面とは食い違っていた」と書く。assumes が外れているので no が正しい。no にすると leads_to（T3 の MODIFIED が要る理由）が機械の表にも立つ。
- 直し方の案: holds を no にして leads_to を足す（または assumes を「MODIFIED の後は許される」に書き換える）。
- 処置: fixed: D3 の assumes を「この change の後の正典で名指しで許される（今の字面は許していない）」に書き換え、affects も食い違いを明示した。MODIFIED の後は成り立つので holds は yes のまま

## R2 — overview が「配布元に聞けない」で赤と言い、上限の警告通しを落としている
- 対象: impact.overview
- 種類: 言い切り
- 指摘: overview は「配布元に聞けない…のどれかで job が赤になる」とだけ言う。実物では 403/429 は RateLimited（crates/updater/src/lib.rs:187、crates/collector-windows/src/update_check.rs:481）で、上限だけの回は警告で通る（IMP-01・IMP-02）。overview の読者は、配布元に断られても赤になると受け取る。限定が overview に無い。
- 直し方の案: overview に「上限（403・429）の答えだけの回は警告で通り、読み取りは確かめない」を 1 文足す。
- 処置: fixed: overview に「上限（403・429）の答えだけの回は警告で通り、一覧の読み取りまでは確かめない」を足した

## R3 — IMP-03 は初回のリリース公開で全 PR が赤になりうる。medium でよいか、閉じ方の材料が無い
- 対象: impact.findings[IMP-03].severity / options
- 種類: 重大度
- 指摘: 前提 P5（premises.md:9）は公開のリリースが 0 件で、置く段が通る道は一度も確かめられていない。更新係は choose の失敗を Unreadable にし（crates/updater/src/lib.rs:252-254）、Q4 でそれは赤になる。したがって最初のリリースを公開した直後から、Windows の通しが走る PR が全部赤になりうる。直しは製品側（読み取り）か置く段の側で、配った後に気づく形になる。1 つ目の option「このまま受け入れる」は、赤になった時点で何が止まるか（merge の関門）を書いていない。medium は妥当だが、unknowns は「読めるか」だけで、公開前に確かめる安い手（公開前の下書きの形を fixture にして updater の choose に通す試験）を option に挙げていない。
- 直し方の案: 2 つ目の option を「公開前に、公開予定の形の一覧を updater の parse/choose に通す自己試験を足す」に具体化し、release_ok_why 相当（いま進めて安全な理由）を書く。
- 処置: fixed: IMP-03 の 2 つ目の option を「公開前に、公開予定の形の一覧を読み取りと版の選び方に通す試験を足す（別の change）」に具体化し、1 つ目に merge の関門が止まることを書き、本文にいま進めて安全な理由（公開のリリースが 0 件）を足した。重大度は medium のまま

## R4 — D2 の「no」と IMP-04 は妥当だが、組み直しの範囲は観測していない
- 対象: impact.dependencies[D2] / findings[IMP-04]
- 種類: 根拠
- 指摘: tools/release/windows-build-key.sh:17 の EXCLUDE_PATHS に soak-install.sh が無いのは事実で no は支えられる。ただ basis が observed のまま「配布物を組み直す」と言い切るには、鍵が `tools/release/` 全体を入力に取るのか（除くパス以外）を鍵の計算の本体まで読む必要があり、挙げた quote は `EXCLUDE_PATHS=(` の 1 行だけ。除くパスに無い＝入力に入る、は derived の飛躍に近い。low なので影響は小さい。
- 直し方の案: 鍵が入力に取るパスの集合を読んだ行を evidence に足すか、basis を derived にする。
- 処置: fixed: 鍵が ls-tree の全部の行から除くパスを落として作ることを読んだ行（tools/release/windows-build-key.sh:380）を E18 として D2 の evidence に足した

## 補足（指摘ではない）
なし（D1・D4・D5・D9・D11 は実物と合った。D11 の CH19 は ch19-emulator-only-class の proposal.md:14 が android-emulator.sh だけを触ると言い、ほかの進行中の change に ci.yml・tools/release を触るものは見つからなかった）。
