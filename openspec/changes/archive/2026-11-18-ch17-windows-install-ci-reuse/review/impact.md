# impact-review — ch17-windows-install-ci-reuse

seen: IMP-01 low, IMP-02 medium, IMP-03 low, IMP-04 medium

## R1 — IMP-02 の「手元と merge gate で走る」が、他の change にとっては走らない
- 対象: impact.findings[IMP-02].derived / impact.dependencies[D5].assumes
- 種類: 根拠
- 指摘: E13（tasks.md:54）は「Linux の手元と merge gate で走らせる」としか言っておらず、これは CH17 自身の tasks の検証の話。`scripts/merge_gate.sh` が見るのは、head の CI・その change の tasks が `[x]`・check_chain / check_scenarios・deep.md の未決で、別の change が足した置き場に対して `windows-build-key.sh --check-workflow` を走らせる口は無い。`check_scenarios` は Scenario の印の有無を突き合わせるだけで台本を走らせない。つまり ST58 など後の change では、その change の tasks が `--check-workflow` を書かない限り、手元でも merge gate でも走らない（「CI の中では走らない」より強く「誰も走らせない」）。`undefined` の「足し忘れたとき落ちるか黙って通るか」は読めば分かる。restore が hit して置き場が欠けると upload-artifact は警告だけで通るが、続く install-windows が配布物を見つけられず落ちる。黙って通る道ではなく、「2 回目の push から原因の見えない形で落ちる」道。severity medium は妥当で、残るのは見えにくさ。
- 直し方の案: derived を「後の change の検証に書かれない限り誰も走らせない」に直し、undefined を「欠けた置き場は install-windows の失敗として現れる」と答えて、`upload-artifact` に `if-no-files-found: error` を付けて落ちる場所を手前に寄せる案（package-windows の step の範囲内）を options に足す。
- 処置: fixed: IMP-02 の text・derived を「後の change の tasks が書かない限り誰も走らせない」に直し、undefined を ST58 との合わせ方に替え、欠けた置き場は install-windows の失敗として現れると書き、options に if-no-files-found: error を足した

## R2 — E7 は「試験の形を 6 つに分ける」を支えない。実物は ST58 の枝の ci.yml にあり、名前も変わる
- 対象: impact.dependencies[D5].evidence[E7] / impact.findings[IMP-02].text
- 種類: 根拠 / 依存の見落とし
- 指摘: E7（st58 tasks.md:10）が言うのは install-windows が `update-apply-install.sh` を走らせることだけ。「6 つ」と置き場の増え方は、`origin/feat/st58-update-apply-rollback` の ci.yml（package-windows の upload-artifact が `package-windows-test-old` / `-new` / `-dev-old` / `-dev-new` / `-compat-old` / `-compat-new`）にだけあり、引用が無い。さらにその枝では `package-windows-test` という名前自体が消え、`package.sh … --test-build` は 1 つの step の中のループになる。CH17 の spec（「今と同じ名前で上げる」）と tasks 2.2 の `--check-workflow`（「2 つの `package.sh windows` の step」「`package-windows-test`」）は 2 つの手順と今の名前を前提に書かれており、ST58 が先に入ると Scenario の文と検査の形が両方合わなくなる（落ちる側で、静かではない）。D5 の `holds: unknown` は置き場の足し忘れだけを言い、この名前とループの食い違いと ci.yml の同じ job の文面の衝突を依存として持っていない。
- 直し方の案: E7 を ST58 の枝の ci.yml の行に差し替え（またはその枝の実物を読んだと書き）、D5 に「名前の変更・ループ化で `--check-workflow` の形と spec の Scenario の文が変わる」を足して、どちらが先に入るかを申し送りに残す。
- 処置: fixed: D5 の who・affects に ST58 の枝の ci.yml（ループで 6 つ・package-windows-test の名前が無くなる）を書き、coverage.examined に枝を読んだことを足し、spec の文と --check-workflow の形の食い違いを open（未解決）に出した。E7 は install-windows の台本の根拠として残す（枝の ci.yml は worktree に無く loc で引けない）

## R3 — 鍵の step が `github.sha` の merge の commit を fetch できるか。private repo で `persist-credentials: false` のままでは取れない見込み
- 対象: design D1「走っている run の CI の定義」/ tasks 2.1 / impact.findings（欠けている）
- 種類: 新しい帰結
- 指摘: 鍵の step は `git fetch --depth=1 <github.sha>` で merge の commit の ci.yml を取り、取れなければ鍵を空にして組む。ところが package-windows の checkout は `persist-credentials: false`（ci.yml:460 前後。この change は checkout を変えない）で、この repo は private（merge_gate.sh の「private + 無料プランでは掛けられない」）。資格情報なしの fetch は認証で落ちる見込みで、その場合は全 run で鍵が空になり、使い回しも保存も一度も起きないまま CI は緑になる（失敗側ではなく「効かない」側に倒れるため、spec の Scenario・自己試験・check-workflow のどれも気づかない）。tasks 2.1 も design も token の渡し方を書いていない。完了の判定 3 の再実行で hit するか見るので最終的には見つかるが、分析は「確かめていない」（not_examined）に merge の commit を fetch できることを含めておらず、IMP も unknowns も無い。overview の「効くのは同じ鍵の保存物があるときだけ」の前に、鍵が出ない道がある。
- 直し方の案: unknowns か IMP に「鍵の step が資格情報なしで merge の commit を fetch できるか」を足し、`GITHUB_TOKEN` を `http.extraheader` で渡す手順を tasks 2.1 に書くか、merge の commit でなく checkout 済みの head の ci.yml の blob を鍵に入れる設計に倒す（その場合は pull_request の run が merge の commit の ci.yml で走ることとの食い違いを D1 に書く）。
- 処置: kept: repo は公開（gh repo view rai-256/ashiato → PUBLIC）で、merge_gate の「private」の前提は当たらない。ただし資格情報なしで merge の commit を fetch できるかは走らせていないので coverage.not_examined に足した（効かない側に倒れること、完了の判定 3 で気づくこと）

## R4 — R4 の `basis: observed` は結論に対して強すぎる（`blocks_legit: false` 自体は支えられる）
- 対象: impact.rules[R4]
- 種類: 根拠
- 指摘: E1（ci.yml:559）と E2（check-artifacts.sh:167）が見せるのは、期待値を head から取り、配布物ごとに SOURCE.txt と印の commit が同じことを見る、という検査の形だけ。「使い回した Windows の zip が通る」は、鍵が Cargo.toml と LICENSE の blob を覆うことからの論理で、使い回した run はまだ 1 度も走っていない（not_examined に同じ趣旨がある）。結論は実物とも矛盾しない: check-artifacts は SOURCE.txt と印を commit で比べ、head との一致は見ない（:164-167）。サーバとインストーラ・更新係の印の一致の検査（:299-305）も同じ zip の中の比較で、Linux / APK の commit とは比べない。LICENSE は期待値を head の `built/LICENSE` から取るが、鍵に入るので同じ。よって `blocks_legit: false` は妥当。ただし根拠は `derived` が正しい。
- 直し方の案: basis を `derived` にし、legit の文に「完了の判定 3・4 の run で確かめる」を足す。
- 処置: fixed: R4 の basis を derived にし、legit に「完了の判定 3・4 の run で確かめる」を足した

## R5 — E11 の行は、D7 が頼る「HEAD を見張る」ではなく環境変数の行
- 対象: impact.dependencies[D7].evidence[E11]
- 種類: 根拠
- 指摘: crates/build-stamp/src/lib.rs:149 は `rerun-if-env-changed=ASHIATO_BUILD_COMMIT`。D7 の前提は「checkout が HEAD を書き直すので、戻した target からでも印が組み直される」で、これを支えるのは同じ関数の後半（:156 以降の `.git/HEAD` と HEAD が指す参照の見張り）。CI の package-windows は `ASHIATO_BUILD_COMMIT` を渡さない（ci.yml:119 は別の job のテスト用）ので、149 行の env の行は効かず、効くのは HEAD の mtime が restore した target の出力より新しいことだけ。この mtime の順序は未確認だが、外れても印が前の commit のまま残り、check-artifacts の「SOURCE.txt が印の commit と違う」で落ちる（静かに通らない）ので `holds: yes` で足りる。evidence の指す行だけが違う。
- 直し方の案: E11 の loc を HEAD を見張る行に直し、D7 の affects に「外れたら check-artifacts が落とす」を書く。
- 処置: fixed: E11 を crates/build-stamp/src/lib.rs:158（HEAD を見張る行）に直し、D7 の affects に「外れても check-artifacts が落とす」を書いた

## R6 — IMP-04 は起きる見込みを低く見ている。PR ごとの置き場に Rust の target が積まれる
- 対象: impact.findings[IMP-04].text / unknowns / severity
- 種類: 重大度（medium のまま、理由を直す）
- 指摘: D5 の Rust のキャッシュは `actions/cache`（restore と post の save）なので、組んだ PR の run ごとに、その PR の置き場へ `~/.cargo` と `target`（Windows では数 GB になりうる）を保存する。PR の置き場は他の PR から読めず（design D2 自身の退けた案の理由）、読めるのは main の保存物だけ。PR が数本並走すると 10 GB はすぐ現実になり、古い順に消えるため、全 PR が引ける main の保存物（と PostgreSQL の zip）が先に押し出される。text は「PR ごと・Cargo.lock ごとに積み」と書くが、unknowns は大きさと並走数だけで、「PR の置き場の保存物が main の保存物を押し出す」という向きを書いていない。誤った配布物は使わない（遅くなるだけ）ので high ではない。
- 直し方の案: Rust のキャッシュの save を main の push の run だけにする（PR は restore のみ）案を options に足し、unknowns に押し出しの向きを書く。
- 処置: fixed: IMP-04 の text と unknowns に PR の置き場が main の保存物を押し出す向きを書き、options に Rust のキャッシュの保存を main の push だけにする案を足した。severity は medium のまま

## R7 — overview の「できなくなる」が 1 つだけで、`lost` の 2 つ目（ch14_ の結果の使い回し）が出てこない
- 対象: impact.overview / impact.lost[1] / impact.findings
- 種類: 言い切り
- 指摘: overview は「できなくなるのは、release-package の 4 つの配布物が同じ commit を指すこと」とだけ言うが、`lost[1]` は使い回した run では文字化けの試験（`ch14_`）がその push で走らないと認めている。design D4 の反転条件（runner の環境で揺れる試験）は IMP にも unknowns にも上がらない。`ch14_` の入力が鍵に入ることは言えるが、同じ入力で結果が揺れない（コンソールのコードページ・runner の image の差）ことは `Re-run` で一度見るだけでは分からず、ImageVersion が鍵に入っているので hit した run の image は同じ版（または前の版）という限定も落ちている。
- 直し方の案: overview の「できなくなる」に `ch14_` を飛ばすことを足し、D4 の反転条件を unknowns か low の IMP に上げる。
- 処置: fixed: overview に ch14_ を前の run の結果に頼ることを足し、D4 の反転条件を IMP-05（low・guess）に上げ、scope.residual に足した

