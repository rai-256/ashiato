# 仮で進めた決定の確かめ（provisional_review）

## D1
- 判定: 確か
- 根拠:
  - 1（失わない側で本当に失わないか）: 「全部残す」は何も消さない。配布元の物を消す口は `tools/release/publish-nightly.sh:39` の `gh release delete-asset nightly …`（印 `nightly` だけ）しか無く、`build-*` の下書きには掛からない（`grep -rn "release delete\|delete-asset\|-X DELETE" tools .github`）。置く口は `tasks.md:68` の `number.sh place` で、`--clobber`（上書き）・公開・`v*` の操作をせず、欠けた配布物を足すだけ。組み立てが落ちた番号も、同じ commit の run を走らせ直せば下書きに足せる（`design.md:71`・`:109` の D7）。CI の成果物の 90 日（P1）は下書きに移すので掛からない。`uncaptured` / `discarded` / `exported` / `rewrite-all` のどれも、この選択からは起きない
  - `exported` の見落としが無いか: 下書きが認証なしの一覧に出る（P5。確かさ 2）なら配布物が外に出る。ただしこれは置き場（C）の性質で、「全部残す」でも「N 日で消す」でも同じだけ出る。D1 の選択では差が出ない。P5 は `tasks.md:136` の 12.1 で 4 に上げ、外れたら置き場を変える（`design.md:42`）。ただし 12.1 は「人間の確認待ち」なので、merge の直後に付く最初の番号の下書きが 12.1 より先にできうる。その場合も、出るのは公開のリポジトリのソースから組んだ物で、更新の確認は `draft: true` を数えない（`design.md:41`）
  - 2（cost）: 月に約 1.7 GB（P2・P4 はどちらも確かさ 5。27 回 × 62 MB）。公開のリポジトリなので置き場の代金は無い（review/deep.md R3 の `gh repo view --json visibility` → `PUBLIC`）。消す仕組みを作らないので Story の範囲は越えず、FR-134・FR-135 の本文も変えない。容量の上限（P11）は確かさ 1 のまま（R3 の提案の「公式文書で 2 に上げる」はされていない）。ただし上限に当たっても、置き済みの番号は失われず、置けなかった番号は `number.sh place` が落ちるだけ。走らせ直せば足せる
  - 3（follows）: null。該当なし
  - 4（rests_on）: [P5, P11]。cost の根拠の P2・P4（確かさ 5）と P1（確かさ 5）は載っていないが、どれも確かさ 5 で、外れても D1 の向きは変わらない。足りない土台の決定は無い

## D2
- 判定: 確か
- 根拠:
  - 1（失わない側で本当に失わないか）: 署名していない APK は、今の正式な鍵の署名の道（`release.yml:62` の `ASHIATO_APK_UNSIGNED: "1"` → `tools/release/sign-apk.sh`）が入力にするものと同じ作り方（`collector-android/app/build.gradle.kts:256-259`。違いは `signingConfig` だけ）。識別子は `dev.ashiato.collector`（番号に `-dev.` が無いので `build.gradle.kts:186` の接尾辞が付かない）で、`sign-apk.sh:26-28` の `official` の検査を通る。だから札（正式版・ベータ版）を付けるときに組み直さずに正式な鍵を足せる。debug の鍵の APK も並べて残すので、本人が試しに入れる物も失わない。`uncaptured` / `discarded` / `exported` / `rewrite-all` のどれも、この選択からは起きない
  - 範囲の確かめ: 開発版の札には、この APK は使えない（`sign-apk.sh:27` は `dev` に `dev.ashiato.collector.dev` を求める）。ただし FR-105（`docs/requirements.md:978`）が「開発版の札だけは、その番号を開発版向けに組んだ物を指す」と決めていて、FR-135（`:1190`）の範囲は正式版とベータ版だけ。D2 の「組み直さずに済む」は FR-135 の範囲の主張なので誤りではない
  - 2（cost）: 番号ごとに APK が 1 つ（数十 MB 以下）と、Linux の組み立ての job で `assembleRelease` が 1 回増える（`build.sh` の apk は `--no-build-cache`・`kotlin.incremental=false` で毎回組む。`tools/release/build.sh:301-305`）。公開のリポジトリの Actions なので代金は無く、口は `tasks.md:73` の 4.2 の環境変数 1 つで Story の範囲に収まる。要件の本文は変えない
  - 3（follows）: null。該当なし
  - 4（rests_on）: [] だが、design D2（`design.md:48`）は P16（確かさ 2。debug の鍵で署名済みの APK を署名し直す道は試していない）に載り、置き場は D1 に載る。どちらが外れても D2 から失うものは出ない。P16 が外れれば（署名し直せれば）署名していない APK が要らなくなるだけで、D1 が置き場を変えても APK は一緒に移る。台帳の `rests_on` に P16・D1 を足すと読みやすいが、判定は変わらない
  - 気づき（否の理由ではない）: 署名していない APK と debug の鍵の APK は、別々の gradle の組み立てから出る。組み立てが再現できなければ、本人が試した debug の鍵の APK と、後で正式な鍵を足した APK は、中身が同じだと言い切れない。FR-135 の「試した物と同じ中身」を言い切るなら、組み立てを 1 回にして、debug の鍵の APK を署名していない APK から `apksigner` で作る形がある（4.2 の実装で選べる）
