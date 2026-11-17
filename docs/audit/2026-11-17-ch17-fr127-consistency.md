# 整合の確認 — FR-127 に CH17 の深掘り第 1 回（Q1・Q2）の ★ 2026-11-17 補足を足した（2026-11-17）

- 基準: `e978edb42e07753105930fb6012947ae22e0fc56`..`774f59cde5677e0216cda7988350fb15c794a6a6`

## 機械の入力（写し）

`python3 scripts/req_impact.py . --base e978edb42e07753105930fb6012947ae22e0fc56`（改訂した要件 1 件・扉 0 項）:

- FR-127（改訂）: `docs/handoff/ST53.md:18`・`docs/handoff/ST55.md:18`・`docs/handoff/ST60.md:6`・`docs/stories/INDEX.md:77,374`・`docs/stories/stories.json:1260`・
  `openspec/changes/ch17-windows-install-ci-reuse/deep.md:37,38,42`・`openspec/specs/distribution/spec.md:2061,2104,2142,2179,2269`

`python3 scripts/check_chain.py .`（chain: FAIL 1 件 / 未回収 0 件 / warn 0 件）:

- [FAIL] 整合の確認: 整合の確認の後に要件が変わった（この監査を名指ししない改訂）: FR-127（この監査がそれに当たる。★ 2026-11-17 はこのファイル名を名指ししている）
- 検査 10（判断の日より新しい訂正）は何も出していない。FR-127 を satisfies に持つのは ST53 だけで、その change は `openspec/changes/archive/2026-11-09-st53-license-bundle`（凍結）。★ 2026-11-17 を作るのは CH17（`satisfies: []`）

走っている change（`openspec/changes/` の archive 以外）は `ch17-windows-install-ci-reuse` と `st58-update-apply-rollback` の 2 本。
ch17 は spec の段で `MODIFIED` に書き直す予定（呼び出し元の指示）なので、ch17 の成果物と正典の食い違いは C1 の 1 件にまとめた。st58 は C4 で印を付けるだけ。
どちらも失われるもの（uncaptured / discarded / exported / rewrite-all）は無い（CI の成果物とキャッシュだけで、外に出る配布物は変わらない —— K の欄で確かめた）。

## 1. 改訂後の要件と、ほかの文書

## C1. 正典 distribution の 2 つの Requirement（と ch17 の spec・design）が、★ 2026-11-17 と反対の SHALL と Scenario を持つ —— 名指しの 2 つより広く、検査の期待値の行と 3 つの Scenario と印 4 か所に及ぶ
- 行き先: fix（ch17 の spec の段の `MODIFIED` で直す。下の行の全部を範囲に入れる）
- 重大度: 高
- 文書: openspec/specs/distribution/spec.md:782
- 要件: FR-127
- 根拠: ★ 2026-11-17（requirements.md:1110）「**PR の CI は、組み立ての入力（鍵）が同じ前の組み立ての Windows の配布物があれば、組み直さずにそれを使う。使い回した Windows の配布物の `SOURCE.txt` と組み立ての印は、それを組んだ commit…を指す**」。
  正典の反対の文:
  - spec.md:784「WHEN コード…を触った PR が CI を走らせる THE SYSTEM SHALL サーバ（画面を組み込む。Windows 向けと Linux 向け）…を、配るときと同じ手順で…組み立てる」・:786「CI の実行の成果物としてだけ置く」
  - spec.md:2259「PR の組み立てでは PR の head の commit になる」・:2260「THE SYSTEM SHALL PR の組み立てを、PR の head の commit そのものから組む…`SOURCE.txt` の commit・組み立ての印・組んだソースが同じ commit を指す」
  - spec.md:2261「期待する版番号と、配布物の `LICENSE` と比べる `LICENSE` を、**配布物を組んだのと同じ commit の中身から取る**。PR の CI の配布物を検査する job は…期待値だけを PR の head の commit…から取る」—— 使い回した run では「組んだ commit」と「head」が別になり、この 1 行の 2 文が両立しない（ci.yml:559 は `built/`＝head から取る。通るのは `Cargo.toml` と `LICENSE` が鍵に入るからで、それを文で言う必要がある）
  - Scenario spec.md:2319「PR の組み立ての配布物のソースの場所は、PR の head の commit」・:2329「PR の配布物は、PR の head の commit から組まれ、同じ commit の期待値で検査される」・:2340「PR の組み立てと下書きの配布物の検査は、head の commit のソースの場所で通る」
  印: `tools/release/notices.sh:360`・`tools/release/check-guards.sh:273,328`・`tools/release/check-artifacts.sh:445`（Scenario の名前を変えるなら、ここも直す。どれも ch17 の宣言したパス（`lane-verdict.json` の `paths`）の外）。
  ch17 側: `specs/distribution/spec.md` は `## ADDED Requirements` だけ（:1）、`design.md` D3「`package-artifacts` は変えない…組み立ての印の commit は前の commit になる」は振る舞いを design にだけ持つ（deep.md Q2 の答え「design D3 の振る舞いを spec の Scenario に移す」）
- 直すところ: ch17 の spec（2 つの Requirement の `MODIFIED`。:2261 の期待値の取り方と、:2319・:2329・:2340 の 3 つの Scenario を含める）と、印の 4 か所
- 機械: 捕まえられた（spec-review R1・R2）。:2261 の期待値の行は R2 が挙げたかを確かめていない —— `openspec validate` と `check_scenarios.py` は名前の対応だけを見るので、文の両立は捕まえられない

## C2. ★ 2026-11-03 が「取れなければ ST53 の上流へ返す」とした PR の head の取り出しの確かめは、ST53 が archive されて宛先を失ったまま未確認で、★ 2026-11-17 はちょうどその未確認の場合（push し直す前の head）を `SOURCE.txt` に書く
- 行き先: fix（ch17 の完了の判定に、使い回した配布物の commit を空の clone で `git fetch origin <commit>` できるかを足す。取れない場合の扱いは ask）
- 重大度: 中
- 文書: docs/requirements.md:1108
- 要件: FR-127
- 根拠: ★ 2026-11-03（requirements.md:1108）「ただし PR の head も、PR を閉じた後や枝を push し直した後に取れるかは確かめておらず、確認バッチで確かめる（**取れなければ ST53 の上流へ返す**）」。
  確かめは ST53 の Task 11.5（`openspec/changes/archive/2026-11-09-st53-license-bundle/tasks.md` の 11.5「枝を rebase して force-push し直し…控えた 2 つ（push し直す前の head と最後の head）を `git fetch origin <commit>` で取れるかを見る」）で、
  確認バッチの答えは `docs/verify/20261105-2246.md:14`「| ST53-T11.5 | ST53 | 11.5 | 確認できない（まだ） |」。その後 ST53 は archive（2026-11-09）され、11.5 を引き受けた handoff・Story・change は無い（`docs/handoff/` を grep）。
  ★ 2026-11-17「使い回した Windows の配布物の `SOURCE.txt` と組み立ての印は、それを組んだ commit（前の PR の head か main の commit）を指す」—— 使い回しの元は同じ PR の前の push の run か main（design D2）で、
  この repo の枝は rebase して push し直す（`docs/audit/2026-11-03-st53-r2-consistency.md` の C1）ので、前の push の head は main からも枝からも辿れなくなる。merge された PR の最後の head は main から辿れる（同 C1）が、使い回した配布物はそれでない commit を指しうる。
  ★ 2026-11-03 の「落とされた写しのソースの場所は直せない」が、使い回しでは例外でなく通常の形になる
- 直すところ: requirements（★ 2026-11-03 の宛先「ST53 の上流」を、いま引き受ける先に置き直す）と ch17 の完了の判定（`docs/changes/CH17.md` の 4. の run で、使い回した Windows の zip の `SOURCE.txt` の commit を空の clone で fetch する）
- 機械: 安く捕まえられる（CH17 の完了の判定 4. の run で `gh run download` → `SOURCE.txt` の `Commit:` を `git fetch origin <sha>`。rc≠0 で落ちる。常設にするなら `package-artifacts` に fetch の step を足すが、それは ch17 の範囲の外）

## C3. design D3 の反転条件「下書きで確かめる手順が commit で突き合わせる」に当たる手順が、もう `docs/release.md` にある
- 行き先: fix（ch17 の spec の段で D3 の反転条件を判定する。手順を直すなら docs）
- 重大度: 中
- 文書: docs/release.md:151
- 要件: FR-127
- 根拠: design.md D3 の反転条件「下書きに載る配布物の印の commit が、その run の commit と同じであることを要すると分かったら（**下書きで確かめる手順が commit で突き合わせる、など**）、`draft.sh` を使い回していない run に限る…か、`package-artifacts` が使い回した run では `release-package` を上げないようにする」。
  docs/release.md:151「`gh api repos/rai-256/ashiato/actions/runs/<run-id> --jq .head_sha` の値と**配布物の組み立ての印の commit が同じ**で `.dirty` が無いことを見る」（4 つの配布物すべてについて）。使い回した run の `release-package` では Windows の 2 つの zip の印が前の commit になり、この手順で食い違う。
  `tools/release/draft.sh:45-46` の下書きの説明文「配布物は PR の package（出所の版 $src）の組み立て（run $run_id）」も、Windows の 2 つについては別の run の組み立てになる（D3 は「のまま」とする）。
  いまの §9 は ST53 の PR の run（CH17 より前で、使い回しは起きない）を指すので、すぐに食い違うのはこの手順を新しい run で走らせ直したとき
- 直すところ: ch17 の spec / design（D3 の反転条件の判定）。手順を残すなら docs/release.md §9 に「使い回した run では Windows の 2 つは前の commit」を足す
- 機械: 安く捕まえられる（`draft.sh` で 4 つの配布物の組み立ての印の commit が揃うかを見る。揃わなければ止める・知らせる）

## C4. 走っている st58 が `package-windows` の試験の形の成果物を 1 つから 6 つに分け、試験の台本を 1 本足す —— ch17 の design D1・D2 と CH17 の「壊してはいけないもの」が前提にする成果物の名前と除く台本の一覧が変わる（印のみ）
- 行き先: fix（ch17 の spec の段で、st58 が先に main に入る場合の保存物の置き場と除く台本を決める）
- 重大度: 中
- 文書: openspec/changes/ch17-windows-install-ci-reuse/design.md:59
- 要件: FR-127
- 根拠: ch17 design D2「中身は `package-windows`・`package-windows-test` の 2 つの置き場」・「引いた 2 つを今と同じ名前（`package-windows-release` / `package-windows-test`）で上げる」。D1「除くもの（名指し）: 試験の台本 `tools/release/install-check.sh`・`tools/release/update-check-install.sh`・`tools/release/conpty-run.ps1`」。
  docs/changes/CH17.md:80「成果物の名前（`package-windows-release` / `package-windows-test` / `release-package`）」。
  st58 の枝（`git diff origin/main...origin/feat/st58-update-apply-rollback -- .github/workflows/ci.yml`）は `package-windows-test` の upload を消し、`package-windows-test-old`・`-new`・`-dev-old`・`-dev-new`・`-compat-old`・`-compat-new` の 6 つを足し（`ASHIATO_BUILD_VERSION="$v" tools/release/package.sh windows "$RUNNER_TEMP/package-windows-test-$name" --test-build`）、
  `install-windows` が読む試験の台本 `tools/release/update-apply-install.sh` を足す（st58 tasks.md:84）。
  st58 が先に入ると、2 つだけを保存する使い回しでは `install-windows` が 6 つの試験の形を受け取れない（CI は赤になる側で、黙って通りはしない）。`update-apply-install.sh` は D1 の除く一覧に無いので、それだけを変えた PR はビルドし直す（安全側。短縮が効かない）
- 直すところ: ch17 の spec / design（保存物を「`package-windows` の upload の全部」で定義するか、st58 の後に並べる）。st58 には何も書かない
- 機械: 安く捕まえられる（`windows-build-key.sh --check-workflow` に「`package-windows` の upload-artifact の path がすべて保存物の path に入っている」を足す）

## C5. ★ 2026-11-17 の括弧「前の PR の head か main の commit」は、別の PR の組み立てを使い回すと読める
- 行き先: requirements（「同じ PR の前の push の head か main の commit」に直す）
- 重大度: 低
- 文書: docs/requirements.md:1110
- 要件: FR-127
- 根拠: ★ 2026-11-17「それを組んだ commit（前の PR の head か main の commit）を指す」。ch17 design D2「使うのは `pull_request` の run だけ。`main` への push は毎回ビルドし、保存する（PR の run は main の保存物を引けるので…）」・Risks「PR の保存物はその PR の置き場にあり、他の PR と main は引けない」—— 使い回しの元は同じ PR の前の run か main で、ほかの PR ではない
- 直すところ: requirements
- 機械: できない
- 処置: 2026-11-17 record の段で 「同じ PR の前の push の head か main の commit。ほかの PR の組み立ては使わない」に直した（docs/requirements.md の FR-127 ★ 2026-11-17）

## C6. ★ 2026-11-17 の「CI の実行の成果物と CI のキャッシュにだけ置く」は、PR の `release-package` を下書きのリリースに載せる `draft.sh` を数えていない
- 行き先: requirements（「だけ」を外すか、「と、公開しない下書きのリリース」を足す）
- 重大度: 低
- 文書: docs/requirements.md:1110
- 要件: FR-127
- 根拠: ★ 2026-11-17「PR の配布物は公開しない（CI の実行の成果物と CI のキャッシュにだけ置く）ので、外に出る配布物の `SOURCE.txt` は変わらない」。
  `tools/release/draft.sh:31`「gh run download "$run_id" --name release-package」→ :44「release create "$tag" "${files[@]}" … --draft --target main」。下書きは公開されない（「公開しない」は成り立つ）が、置き場はほかにもある。正典 spec.md:786「CI の実行の成果物としてだけ置く」も同じ書き方（ST52 からの古い言い回し）
- 直すところ: requirements（正典の :786 は C1 の `MODIFIED` で一緒に）
- 機械: できない
- 処置: 2026-11-17 record の段で 「CI の実行の成果物としてだけ置く」の言い方をやめ「今どおり公開しない（CI のキャッシュにも置くが、公開はしない）」に直した（docs/requirements.md の FR-127 ★ 2026-11-17）

## C7. ★ 2026-11-17 が鍵の組み方（仮の D1）を要件に書き、D1 の反転条件が起きると ★ が古くなる
- 行き先: requirements（括弧の鍵の説明を「鍵は組み立てに入る入力から作る（中身は ch17 の design）」程度に留める）
- 重大度: 低
- 文書: docs/requirements.md:1110
- 要件: FR-127
- 根拠: ★ 2026-11-17「鍵は**試験の台本と文書を除く全部のファイル**と道具の版から作るので、中身は head から組んだものと同じになる想定」。
  ch17 design D1 は「（仮）」で、反転条件「Android だけ・Linux の道具だけの変更でも…ビルドし直していて、1 周の短縮が足りないと分かったら、除くものに `collector-android/` などを足す」・「ImageVersion を外し、代わりに VC の再頒布の置き場の版…を入れる」。反転すると ★ の「除く全部」が事実と違う
- 直すところ: requirements
- 機械: できない
- 処置: 2026-11-17 record の段で 鍵の説明を「組み立てに入る入力から作る。鍵の組み方は CH17 の design が持つ」に留めた（docs/requirements.md の FR-127 ★ 2026-11-17）

## C8. deep.md の Q1 の答えは「requirements.md の FR は変えない」と書き、★ 2026-11-17 は Q1 を出所に挙げて使い回しの方針そのものを FR-127 に書いた
- 行き先: leave: Q1 の方針が FR-127 にも載るのは Q2 の例外の前置きとして読める。直すなら deep.md の Q1 の答えの「FR は変えない」に「Q2 の ★ が前置きとして写す」を足す
- 重大度: 低
- 文書: openspec/changes/ch17-windows-install-ci-reuse/deep.md:24
- 要件: FR-127
- 根拠: deep.md:24-25「この Requirement の導出元は Story の価値（FR ではない）なので、requirements.md の FR は変えない」。requirements.md:1110「★ 2026-11-17 補足。CH17 の深掘り第 1 回 Q1・Q2…**PR の CI は、組み立ての入力（鍵）が同じ前の組み立ての Windows の配布物があれば、組み直さずにそれを使う**」—— 許諾の FR に CI の回し方の方針が入り、使い回しの導出元が「Story の価値（ST52）」と「FR-127」の 2 つに見える
- 直すところ: その文書（deep.md）か leave
- 機械: できない

## C9. ★ 2026-11-17 の振る舞いを作るのは CH17 だが、FR-127 を拾うのは凍結した ST53 だけで、鎖の上では改訂がどの生きた作業にも届いていない
- 行き先: leave: change は `satisfies: []` が全件（CH01〜CH17）の決まりで、CH17 の完了の判定と spec の `MODIFIED` が受ける。鎖で見えないことだけを記録する
- 重大度: 低
- 文書: docs/changes/CH17.md:4
- 要件: FR-127
- 根拠: CH17.md:4「satisfies: []」。stories.json:1257-1261 の ST53「"satisfies": [ "FR-127", "NFR-26" ]」、change は `openspec/changes/archive/2026-11-09-st53-license-bundle`。check_chain.py の検査 10 は FR-127 について何も出さない（warn 0 件）
- 直すところ: leave
- 機械: できない（change の `satisfies` を見る検査は無い）

## 2. 文書とコード

確かめたものは下の 1 件（指摘）と「触らなくてよい」の K の行。★ 2026-11-17 の使い回しそのもの（鍵の台本・保存と引き）はまだコードに無い（`tools/release/windows-build-key.sh` は無い・ci.yml:452-484 の `package-windows` に保存物のキャッシュは無い）—— ch17 がこれから作るので指摘にしない。

## K1. ch17 design D3 の「`check-artifacts.sh` は印と `SOURCE.txt` の commit の一致を見るので、どちらも前の commit で揃い、通る」は確かめられた。ただし `package-artifacts` の期待値は head の木から取るので、使い回しが通るのは `Cargo.toml`（版番号）と `LICENSE` が鍵に入っている間だけ
- 行き先: fix（C1 の `MODIFIED` の :2261 の行に、この条件を書く）
- 重大度: 低
- 文書: openspec/changes/ch17-windows-install-ci-reuse/design.md:76
- 要件: FR-127
- 根拠: tools/release/check-artifacts.sh:165-167「expected_commit=…SOURCE.txt の Commit…; [[ … "$expected_commit" == "$stamp_commit" ]] || die "$name の SOURCE.txt が組み立ての印の commit と違う"」（head とは比べない）。
  .github/workflows/ci.yml:549-559「ref: ${{ github.event.pull_request.head.sha || github.sha }} / path: built」→「--expect-version "$(tools/release/version.sh source built/Cargo.toml)" --expect-license built/LICENSE」。
  design D1 は `LICENSE` と（除く一覧に無いので）`Cargo.toml` を鍵に入れる。D1 の反転で除くものを足すときに、この 2 つを外すと `package-artifacts` が使い回した run で落ちる
- 直すところ: ch17 の spec（C1 と一緒に）
- 機械: 安く捕まえられる（`windows-build-key.sh --self-test` に「`Cargo.toml` と `LICENSE` が違えば鍵が変わる」を足す）

## 未確認

- GitHub Actions のキャッシュの範囲（PR の run が引けるのは同じ PR の保存物と base の枝の保存物だけ）。GitHub の一般の振る舞いとして C5 の根拠にしたが、この repo では走らせていない（ch17 design D2・Risks も同じ前提）
- GitHub が、force-push で枝から外れた前の head を `git fetch origin <sha>` で返すか（C2。ST53 の P14・11.5 で未確認のまま）
- spec-review の R2 が正典 spec.md:2261（期待値の取り方）まで挙げているか。`review/spec.md` は読んでいない
- ★ 2026-11-17 の「鍵は…中身は head から組んだものと同じになる想定」について、組み立てが `docs/`・`openspec/`・`*.md`・除く 3 つの台本を読まないことは `grep` で見た範囲だけ（`tools/release/{build,package,notices,pgsql}.sh` に `docs/`・`.md`・3 つの台本の名前が無い、`crates/` の `cfg(test)` でない `include_str!`/`include_bytes!` は `grants.sql`・`migration_ledger.sql`・`tools/db-roles.sql`・`OUT_DIR` のものだけ、`crates/update-check/src/lib.rs:562` の `docs/contracts/` は :270 の `#[cfg(test)]` の中、`web/vite.config.ts` に `docs`・`md` が無い）。cargo や vite が間接に読むものは見ていない

## 触らなくてよいと判断したもの

- `docs/handoff/ST53.md:18`・`docs/handoff/ST55.md:18`・`docs/handoff/ST60.md:6`: FR-127 の依存の許諾の一覧（`notices.sh server`）への足し込みの話で、`SOURCE.txt` と PR の組み立ての commit に触れない
- `docs/stories/INDEX.md:77,374`・`docs/stories/stories.json:1260` 以降の ST53 の done「配布物の `SOURCE.txt` がチャンネルの決まりどおり（…PR —— は 40 桁の commit）」: 使い回しでも 40 桁の commit で、head とは言っていない。凍結した Story の判定は崩れない
- `openspec/specs/distribution/spec.md:2061,2104,2142,2179`: 導出元の FR-127 は許諾の文・表示・依存の検査の側で、★ 2026-11-17 の範囲の外
- ★ 2026-11-17 の「外に出る配布物の `SOURCE.txt` は変わらない」: 確かめた。`.github/workflows/release.yml:76-80` は同じ run の中で `build.sh server-windows` を直に呼び、`nightly.yml:73-76` は同じ run の `nightly-*` だけを落とす。どちらも ci.yml の保存物を引かない
- ★ 2026-11-03（第 3 回）の「ほかの job の縦串や試験は今どおり合わせた木で走る。組み立ての job の中の自己試験と許諾・表示の検査は head の木で走る」: 自己試験と許諾の検査は `package-linux`（ci.yml:440-450）にあり、使い回しの対象でない。使い回したときに飛ぶ `ch14_` の試験（ci.yml:470）は design D4 が扱い、入力は鍵に入る
- `docs/changes/CH14.md:50`「`cargo test -p ashiato-setup ch14_` が Windows の CI（`package-windows`）で rc=0」: CH14 は済んだ change。使い回した run で飛ぶのは同じ鍵（同じ `crates/installer/`）の run で通った後だけ（D4）
- `docs/release.md:89-91` の PR の成果物の掃除（`workflow_run.head_sha` で ST53 より前かを見る）: ST53 より前の組み立ては許諾の 3 点を持たず鍵が違うので、ST53 後の run が ST53 前の Windows の配布物を使い回すことは無い
- `docs/stories/ST53.md` の「> …（続きは requirements.md）」: Story の逐語の引用を切る決まりの形（ST01〜 にもある）
- 語の揺れ: 名前を変えた概念は無い
