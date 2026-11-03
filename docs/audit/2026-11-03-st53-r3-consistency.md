# 整合の確認（2026-11-03・ST53 の深掘り第 3 回の答え Q16 を FR-127 へ戻した改訂）

- 基準: `f4b044676fa24947867fa6a7e5497f380781dbb4`..`f0b2c798c155fe5f27460a326e57978acde723b4`

対象は commit f0b2c798 が足した 1 つの ★（`docs/requirements.md:1074` の FR-127 ★ 2026-11-03 補足（第 3 回））と、
`make_story.py` で作り直した `docs/stories/ST53.md:23`。答えの記録（`openspec/changes/st53-license-bundle/deep-answers-3.txt`・`deep-questions-r3.json`・`deep.md` の第 3 回の節）と突き合わせた。文書は head（`f0b2c798`）の内容で読んだ。

機械の入力:
- `req_impact.py . --base f4b04467…`: 改訂した要件 1 件（FR-127）・扉 0 項。名指しは `docs/stories/INDEX.md:76,371`・`stories.json:1260` と st53-license-bundle の中だけ
- `check_chain.py .`: 要件 171 / Story 58 / 扉 35・拾われた要件 169/171・未回収 0・warn 0。FAIL 1 件「整合の確認の後に要件が変わった（この監査を名指ししない改訂）: FR-127」（この監査が受けるもの）
- `check_scenarios.py .`: 担保の無い Scenario 50 件。すべて st53-license-bundle の change のもの（tasks は `- [x]` が 0 件）。正典の Scenario で印の無いものは 0

走っている change は `st53-license-bundle`（出どころ）と `st44-collector-credential-scope`（`SOURCE.txt`・`package-linux`・`head.sha` を grep して 0 件。交差しない）。
st53 の tasks は 1 件も済んでいないので、失われるもの（uncaptured / discarded / exported / rewrite-all）を持つ印は無い。
呼び出し元の指示どおり、st53 の design・spec・tasks・go.json・provisional が Q16 をまだ写していないことは、次の `spec --answers Q16` 段が直すものとして「直すところ」を `走っている change（st53。spec --answers で写す）` にし、ここで直すもの（requirements・ほかの文書・Story）と分けた。

## C1. 走っている change の D9・D17（仮）・spec・tasks・go.json が、PR の組み立てを合成の merge commit から組み、`SOURCE.txt` と組み立ての印の比べを外す前提のまま（印のみ）
- 行き先: leave: 走っている change（st53）の spec --answers Q16 が写す。`deep.md` の「spec の段へ渡すもの（第 3 回）」に、渡すものに無かった tasks 11.1 の反転・design の Risks の 1 項・`proposal.md:71` を足した
- 重大度: 中
- 文書: openspec/changes/st53-license-bundle/specs/distribution/spec.md:181
- 要件: FR-127
- 根拠: FR-127 ★ 2026-11-03 補足（第 3 回）「**PR の組み立ては PR の head の commit そのものから組む**（GitHub が作る、head とその時の main を合わせた merge の commit からは組まない）」。`deep.md:164`「比べはすべての組み立てで残す」「D17（仮）は外す」。
  change の側: `spec.md:181`「debug の鍵を許す検査（PR・main への push の組み立てと下書きの配布物の検査）: 40 桁の commit。組み立ての印とは比べない（PR の組み立ての印は GitHub が作る merge の commit を持ち、`SOURCE.txt` の head の commit と一致しない）」、`:242-245` Scenario「PR の組み立てと下書きの配布物の検査は、head の commit のソースの場所で通る」（GIVEN「組み立ての印と違う 40 桁の commit」）。
  `design.md:139`・`:152`「組み立ての印との比べは外す」、`:143` は `ASHIATO_SOURCE_COMMIT` を渡すだけで checkout の `ref` を書かない、`:224-229` D17（仮）、`:240` Risks「PR の組み立ては合成の merge commit（…）を checkout して組み、組み立ての印も merge commit を持つ」。`provisional.jsonl:16` に D17 が載ったまま。
  `tasks.md:21`（仮: D17）・`:149`（checkout の `ref` が無く、`ASHIATO_SOURCE_COMMIT` だけ）・`:159`・`:164`（印と比べない分岐と Scenario）、`:229` の 11.1「組み立ての印の commit（合成の merge commit）と違うことを見る」—— Q16 の後は同じでなければならず、このままでは確認バッチが正しい組み立てを落とす。
  `go.json:921`「Commit: <GitHub が PR のために作る merge commit>」・`:935`・`:1385`・`:2086-2096`（IMP-06。`:2091`「PR の CI は checkout に ref を渡さない」）、`proposal.md:71`（`ASHIATO_SOURCE_COMMIT` で渡すだけ）。
  `deep.md:148` の「spec の段へ渡すもの」は D9・D16・D17・spec・tasks の checkout と比べ・go.json・P14 を挙げるが、11.1（`tasks.md:229`）の反転・design の Risks（`:240`）・`proposal.md:71` は名指ししていない。
  コード: `.github/workflows/ci.yml:355`・`:377` の checkout は `with: { persist-credentials: false }` だけで `ref` を渡さない（今は合成の merge commit から組む）
- 直すところ: 走っている change（st53。spec --answers で写す。上の各行に加え、11.1 の「違うことを見る」を「同じことを見る」に反転、Risks の 1 項を外す、`proposal.md:71` に checkout の `ref` を足す）
- 機械: 安く捕まえられる（spec --answers の後に、st53 の change の文書に「合成の merge commit」「組み立ての印とは比べない」「D17」が残れば落とす grep。`provisional.py` の台帳に D17 が残れば check_chain が拾える）

## K1. Q16 の組み方で変えるのは `package-linux`・`package-windows` の 2 つの checkout だけで、配布物を検査する `package-artifacts` の job は合わせた木のまま。head から組んだ配布物を合わせた木の版番号（と、計画では木の `LICENSE`）と比べる
- 行き先: leave: 走っている change（st53）の spec --answers Q16 が design D9 と tasks に写す。`deep.md` の「spec の段へ渡すもの（第 3 回）」に `package-artifacts` の checkout にも同じ `ref` を渡すか、期待値を組み立ての木から取ることを足した
- 重大度: 中
- 文書: openspec/changes/st53-license-bundle/deep-questions-r3.json:19
- 要件: FR-127
- 根拠: 選んだ選択肢「`package-linux`・`package-windows` の checkout に `ref: ${{ github.event.pull_request.head.sha || github.sha }}` を渡す。…費用は workflow の 2 行」、`deep.md:164` も同じ 2 つだけを名指す。
  コード: `.github/workflows/ci.yml:388-400` の `package-artifacts` は `ref` を渡さずに checkout し（`:392-393`）、`tools/release/check-artifacts.sh package/all --allow-debug-key --expect-version "$(tools/release/version.sh source)"`（`:400`）で、合わせた木の `Cargo.toml` の `[workspace.package] version`（`tools/release/version.sh:13-15`）と比べる。
  一方、配布物の版番号は `package.sh` → `build.sh` が組んだ木の `version.sh source`（`tools/release/build.sh:18`）で決める。PR の枝を切った後に main が出所の版を上げると、head から組んだ配布物と、合わせた木の期待する版番号が食い違い、PR の CI が赤くなる。
  計画の 7.1（`tasks.md:156`「`LICENSE` が木の `LICENSE` と同じこと」）も同じ job の木と比べるので、main で `LICENSE` が変わった（FR-127 ★ 2026-10-25「追加の許可を足す Story は、`LICENSE`…を一緒に直す」）後の PR で同じことが起きる。
  `tools/release/draft.sh:15`・`:32` も、走らせた手元の木の `version.sh source` で PR の配布物を比べる（Q16 の前から同じ型だが、組んだ木が head になると、手元が main のときに食い違う幅が main の差だけ増える）。
  落ちる側に倒れるので配布物は漏れない。merge gate は head を origin/main に rebase してから CI を待つ（`scripts/merge_gate.sh:7`・`:79`）ので、門の時点では食い違いは消える
- 直すところ: 走っている change（st53。spec --answers で写す。`package-artifacts` の checkout にも同じ `ref` を渡すか、期待する版番号と `LICENSE` を配布物の組み立ての木から取るかを design D9 に書き、tasks の `ci.yml` の検証の grep を合わせる）
- 機械: 安く捕まえられる（`ci.yml` で `package-*` を download する job と、組む job の checkout の `ref` が同じ式であることを grep で見る。tasks の検証に 1 行）

## K2. ★ の「縦串や試験は今どおり合わせた木で走る」が、`package-linux` の job の中の自己試験（と計画で足す許諾の検査）には当たらない
- 行き先: requirements（FR-127 ★ 2026-11-03（第 3 回）の括弧を「ほかの job の縦串や試験は今どおり合わせた木で走る。組み立ての job の中の自己試験と許諾・表示の検査は head の木で走る」に狭めた。選択肢の本文の「ほかの job」の限定に合わせた）
- 重大度: 中
- 文書: docs/requirements.md:1074
- 要件: FR-127, NFR-26
- 根拠: FR-127 ★ 2026-11-03 補足（第 3 回）「PR の配布物はその時の main と合わせた結果を映さない（縦串や試験は今どおり合わせた木で走る）」。
  コード: `.github/workflows/ci.yml:366-370` は、組み立てと同じ `package-linux` の job で `tools/release/check-artifacts.sh --self-test`・`tools/release/sign-apk.sh --self-test`・`tools/release/make-key.sh --self-test` を走らせる（この 3 つはほかの job に無い。`ci.yml` の `--self-test` を全部 grep した）。
  選んだ選択肢の本文（`deep-questions-r3.json:19`）は「縦串や試験など**ほかの job** は今どおり合わせた木で走る」と job で限っていたが、★ は「ほかの job」を落とした。
  計画: `proposal.md:71`「`package-linux` に `check-licenses.sh android` と `check-spdx.sh`」。選んだ選択肢は job の checkout に `ref` を渡す形なので、job ごと head の木になり、この 3 つの自己試験と、NFR-26 の携帯の依存の検査・表示の検査は合わせた木でなく head で走る。
  main と合わせて初めて許可の外の依存や表示の無いファイルが入る PR は、PR の CI では止まらず、merge gate の rebase の後（`scripts/merge_gate.sh:7`・`:79`）か main への push で止まる
- 直すところ: 走っている change（st53。spec --answers で写す。自己試験と許諾・表示の検査を `ref` を変えない job へ移すか、組み立ての step だけを head の木で走らせる）。移さないなら requirements（★ の括弧を「`package-linux` の job の中の検査を除き」に狭める）
- 機械: 安く捕まえられる（`ref:` に `head.sha` を持つ job の中に `--self-test`・`check-licenses.sh`・`check-spdx.sh` の step があれば落とす grep を tasks の検証に置く）

## C2. ST53 の完了の判定が「PR は 40 桁の commit」だけを固定し、組んだ木が head であることを含まない。Q16 を写さずに `ASHIATO_SOURCE_COMMIT` だけを渡す実装でも完了の判定を通る
- 行き先: leave: ★ が Story に逐語で入り（`make_story.py`）、spec --answers Q16 で「比べはすべての組み立てで残す」を写せば、`check-artifacts.sh` が `SOURCE.txt` と組み立ての印の食い違いで落とす
- 重大度: 低
- 文書: docs/stories/stories.json:1277
- 要件: FR-127
- 根拠: `stories.json:1277`（`ST53.md:51`）「配布物の `SOURCE.txt` がチャンネルの決まりどおり（正式版・ベータ版は印 `v*`、それ以外の組み立て —— 開発版・main への push・PR —— は 40 桁の commit）」。
  FR-127 ★ 2026-11-03 補足（第 3 回）「**PR の組み立ては PR の head の commit そのものから組む**」。いまの計画（`tasks.md:149`）の形 —— 合わせた木から組み、`SOURCE.txt` に head を書く —— でも、この判定の字面は満たされる。
  ST53.md の「満たす要件」は ★ を逐語で写す（`ST53.md:23`）ので、Story を読めば分かる
- 直すところ: leave（★ が Story に逐語で入り、spec --answers で「比べはすべての組み立てで残す」を写せば、`check-artifacts.sh` が `SOURCE.txt` と組み立ての印の食い違いで落とす）。直すなら Story（`stories.json` の done[4] に「PR の組み立ては head の commit から組み、`SOURCE.txt` と組み立ての印が同じ commit」と足して `make_story.py`）
- 機械: 捕まえられる（C1 を写した後の `check-artifacts.sh --allow-debug-key` の比べ）

## 未確認

- `actions/checkout` に `ref: ${{ github.event.pull_request.head.sha }}` を渡したとき、fork の PR（head が fork にある）でも既定の `fetch-depth: 1` で head を取れるか（GitHub は base の repo に `refs/pull/<n>/head` を置くので取れるはずだが、走らせていない）
- head から組んだ組み立ての印に `.dirty` が付かないこと（detached HEAD の checkout で `crates/build-stamp` と `app/build.gradle.kts` の判定が綺麗とみなすか。`tools/check-build-stamp.sh` は使い捨ての clone でしか見ていない）
- merge gate の rebase の後、門の CI と実際の merge の間に main が進んだときに、merge される木と最後に組んだ head の差がどう扱われるか（`scripts/merge_gate.sh` の後半は読んでいない）

## 触らなくてよいと判断したもの（理由つき）

- **`check_chain.py` の FAIL（整合の確認の後に FR-127 が変わった）**: この監査が受ける。改訂した側が要件の ★ からこの監査を名指せば消える
- **`docs/stories/ST53.md:23`**: 要件 `:1074` と逐語で同じ（`make_story.py` の再生成）。題・価値・完了の判定は変わっていない
- **正典 `distribution` の「コードを触った PR ごとに、配る形の 3 部品を公開せずに組む」（`openspec/specs/distribution/spec.md:899-918`）と ST52 の価値「配れる形を壊した Story がその PR の時点で落ち」（`stories.json:1242`）**: head から組むと、main と合わせて初めて壊れる組み合わせは毎回の PR の CI では落ちない。ただし merge gate が head を origin/main に rebase（依存を取り込む枝は merge）してから、その head の CI を待つ（`scripts/merge_gate.sh:7`・`:73`・`:79`）ので、merge の前の CI では head が合わせた木になる。正典の Requirement も Scenario（「配布物の 1 つの組み立てが失敗する木で…」）も組む木を名指さず、字面は崩れない
- **FR-127 ★ 2026-11-02 の「PR では GitHub が作る merge の commit か PR の head か —— は ST53 の design D9（仮）が持ち」と ★ 2026-11-03（第 2 回）の「GitHub が作る merge の commit は PR を閉じた後に取れるかが確かめられておらず」**: 経緯の ★ で、第 2 回の ★ が「これで決まった」と書き、第 3 回の ★ が組む木を足した。書く commit（head）は変わらず、食い違わない
- **premises P14（`premises.md:23`）と 11.5（`tasks.md`）**: head を後から取れるかの確かめで、Q16 の問いの context も「この問いとは別」と書く。組む木を変えても前提は同じ
- **`tools/release/draft.sh:44` の `--target main`**: 下書きのリリースの印の行き先が main で、配布物は PR の head から組まれる。下書きは公開せず印を作らず、確かめた後に消す（`docs/release.md:91`）。Q16 の前（合成の merge commit）でも main と違っていた
- **`docs/release.md:9`・`:97`**: PR の `package` の job が出所の版で組むと書くだけで、組む木を名指さない。head の `Cargo.toml` の版で組むことと食い違わない（版番号の比べの食い違いは K1）
- **`docs/production-prep.md`・`docs/handoff/ST53.md`・`docs/testing.md`・README・CONTRIBUTING**: PR の組み立ての木・`SOURCE.txt` の commit に触れない（grep で 0 件）
- **語の揺れ**: 要件は「GitHub が作る merge の commit」、change は「合成の merge commit」と呼ぶが、どちらも同じものを指し、改訂で名前を変えた概念ではない
- **走っている change `st44-collector-credential-scope`**: `SOURCE.txt`・`package-linux`・`head.sha` を grep して 0 件
