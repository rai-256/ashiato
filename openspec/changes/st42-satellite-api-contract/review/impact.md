# impact-review — st42-satellite-api-contract

seen: IMP-01 medium, IMP-02 medium, IMP-03 low

## R1 — 走っている change との重なりの記述が実物と違う（ch26・st61 も `changes` の job と changes.sh を変える）
- 対象: impact.coverage.examined[3]（「ch21・ch26 は ci.yml・lib.rs の別の行で、衛星向けの道と判定に触れない」）・impact.dependencies の D6 / D7・T5
- 種類: 依存の見落とし
- 指摘: ch26 の tasks.md:13 は「`.github/workflows/ci.yml` で変えるのは `changes`（出力を足す）…」と書き、design.md:152 も「`changes`（出力を 2 つ足す）」と言う。ST42 の Task 8.1 も同じ `changes` の job の `outputs:` と `all()`（ci.yml:24-29・39）に `satellite` を足すので、「判定に触れない」は言えない。ST61 の tasks.md:90-93（6.1）は `tools/release/changes.sh` に `base` の分岐と `--self-test` を足し、`ci.yml` の `changes` の job を変える。ST42 も同じファイルの `case` と `usage` を変えるので、3 本が同じ場所を取り合う。dependencies には ST61 が `code` を読む D6 しか無く、ch26 と ST61 の重なりを書いた行が無い。
- 直し方の案: dependencies に「ST61 6.1・ch26 4.2 と、changes.sh の `case`・`changes` の job の `outputs:`・`all()` を同じ場所で変える」を `holds: unknown`、`verify`: 後から入る側が rebase して `changes.sh` の自己試験と `check-vulns.sh --self-test` を通す、で足し、coverage の文を直す。
- 処置: fixed: 依存 D8（ch26・st61 も changes の job と changes.sh を同じ所で変える。rebase の後の確かめ方つき）を足し、coverage.examined の ch26 の記述を直した

## R2 — rust の job に渡す「比べる相手」が changes の job と別に書かれ、ST61 の変更で黙ってずれる
- 対象: impact.targets[T5]・impact.dependencies（欠け）・scope.preconditions[2]
- 種類: 依存の見落とし
- 指摘: design.md:128 と tasks.md:115 は、`changes` の job の `BASE`（ci.yml:35。いま `pull_request.base.sha || event.before`）と同じ式を rust の job の `SATELLITE_BASE` に写す。ST61 6.1 は push の比べる相手を「前に成功した main の run」に変える（ST61 tasks.md:90-91）。写した側は自動では追随せず、ずれると main の push で別の commit と比べる。さらに changes の job は比べる相手が引けないと「全部走らせる」（ci.yml:40-42）が、ST42 の検査は「読めなければ落とす」（design.md:132）。force-push などで `event.before` が引けない main の push では、rust の job が走ったうえで赤になる。D6 は ST61 が `code` を読む向きしか見ていない。
- 直し方の案: `changes` の job に比べる相手を出力として 1 つ持たせて rust の job が読む形にするか、引けないときの扱い（落とすか通すか）を spec に 1 行足す。dependencies に D として足す。
- 処置: fixed: 依存 D9（比べる相手の写しが追随しない・引けないときの扱いが違う）と IMP-04（medium）を足した

## R3 — 「知らせの画面ができるまで外せない」は、名前の一致だけで抜けられる
- 対象: impact.overview・impact.rules[R2]・impact.findings[IMP-02].derived・sections.watch / lost
- 種類: 言い切り
- 指摘: design.md:153 の外せる条件 (b) は、版の一覧の `notice` が `openspec/specs/**/spec.md` の `### Requirement:` の名前と一字一句同じ、だけを見る。どの Requirement の名前でも通るので、本人が無関係な既存の名前を `notice` に書けば、知らせの画面が無くても条件 (b) は満たされる。overview の「前の版は知らせの画面ができるまで外せない」・R2 の「知らせの画面の Requirement の両方が無ければ落ちる」は、その画面があることを確かめる検査ではない。IMP-02 の `derived`「今その Requirement を持つ Story は無い」は、外せない理由としては足りない。実際は本人が名前を書けば外せるので、外せなさは実物より強く見える。
- 直し方の案: overview・R2・IMP-02 に「`notice` に書く名前が実在する Requirement かまでを見る。その Requirement が知らせの画面かは人が見る」を足す。
- 処置: fixed: overview・規則 R2・筋書き S2 を「notice に正典の Requirement の名前が要る」と言い直し、IMP-02 の derived と unknowns に名前の一致だけで抜けられることを書いた

## R4 — 「配った日」が v* の全部の印（ベータ版を含む）で数えられる
- 対象: impact.scope.preconditions[1]・impact.findings[IMP-02]・walkthroughs[W2]
- 種類: 言い切り
- 指摘: 6 か月は「次の版の写しを最初に含んだ印 `v*`」の日から数える（design.md:152）。ST61 の design.md:149 は `v<番号>` と `-beta.<N>` の印を同じ `v*` として作る。ベータ版の印が最初なら、6 か月は正式版の前から始まる。W2・IMP-02 は「配って 6 か月」と読め、正式版を配ってからの 6 か月と受け取られる。印を作る仕組みが ST61 側にあり、どの印を数えるかは ST42 の spec にも `not_examined` にも書かれていない。
- 直し方の案: 「ベータ版の印も `v*` なので数えに入る（正式版からの 6 か月ではない）」を IMP-02 の `unknowns` か scope に足し、数えない印の形が欲しければ spec で決める。
- 処置: fixed: IMP-05（medium・unconfirmed。ベータ版の印も起点に入りうる）を足した

## R5 — 重なった README・CONTRIBUTING の書き足しが、守るために置かれた検査と衝突する（重大度は妥当だが根拠を足す）
- 対象: impact.findings[IMP-03]・impact.dependencies[D4]
- 種類: 根拠
- 指摘: D4 の `holds: no`（check-cla-section.sh:8 が `## ライセンス` を含む範囲を比べる）は実物のとおり。ただし tools/check-cla-section.sh の 2 行目のコメントは「ST53 の検証。本体の許諾の節と CLA の中身は変えない」と、変えないことを守るために置かれた検査だと書く。archive の st53 review/code.md:288 は、この範囲を `## ライセンス` まで広げたことを R8 の指摘への処置として記録する。ST42 の Task 5.1 は同じ節に Apache-2.0 を書き足し、その「変えない」を意図して破る。IMP-03 は「古い検査が食い違う」とだけ書き、この意図の違いと、直す側（台本の基準を変える・節を別の場所へ分ける）を選択肢に挙げていない。options は冒頭の文の直しだけで、台本の側の直しが無い。low は妥当。
- 直し方の案: IMP-03 の options に「`## ライセンス` の節でなく、新しい節に契約のライセンスを書く（check-cla-section.sh の範囲の外に置く）」を足し、D4 の `affects` に ST53 の守りたかったものを書く。
- 処置: fixed: IMP-03 の derived に check-cla-section.sh の置かれた理由を足し、options に範囲の外の新しい節に書く案を足した

## R6 — 衛星向けの道の URL の基点（`/api` の有無）が決まっていない
- 対象: impact.targets[T3]・impact.situations[normal]・uses[U1]
- 種類: 新しい帰結
- 指摘: lib.rs:4418 の `strip_api_prefix` は最も外側の層で `/api` を剥がし、web/vite.config.ts:12-14 も `/api` を剥がして中継する。画面側の呼び方は `/api/...` だが、仕様と契約は `/satellite/v1/me`（`servers` の欄の記述が見当たらない）。衛星の作り手がどちらの基点で呼ぶかはページから読めず、`/api/satellite/…` でも通る。さらに `Sunset` は「middleware の外側の層で道が `/satellite/v<N>/` で始まるものに付ける」（design.md D11）。この層が `strip_api_prefix` の内側にあれば `/api/` つきの求めにも付き、外側ならば付かない。どちらかは書かれていない。U1 の「版 1 のページを読めば呼べる」は、基点が書かれていなければ言えない。
- 直し方の案: 契約に `servers`（基点）か、基点の書き方をページの Scenario に足し、`Sunset` を付ける層と `strip_api_prefix` の前後を design に 1 行で決める。
- 処置: fixed: IMP-06（medium。/api の基点と Sunset の層が契約に無い。E14 lib.rs:4418）を足した

## R7 — sections.lost の結論 1 行が失うものの 2 つ目を落としている
- 対象: sections.lost.tldr・impact.lost[1]
- 種類: どれくらい困るか
- 指摘: lost は 2 件（版 1 の凍結、README・CONTRIBUTING だけの PR でも rust の job が走る）。tldr は凍結だけを書き、2 つ目（Actions の時間を使う）が結論の行から消えている。メモリにある Actions の予算切れの件（2026-09-19 実測、ci.yml のコメント）から、この 2 つ目は軽くない失い方として扱われている。harm の `unknown` に「回数・時間は測っていない」とある点は妥当。
- 直し方の案: tldr に「README・CONTRIBUTING だけの PR でもサーバの検査の job が走る」を足す。
- 処置: fixed: sections.lost.tldr に README だけの PR でも重い検査を待つことを足した

