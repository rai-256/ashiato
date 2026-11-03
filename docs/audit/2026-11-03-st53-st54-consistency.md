# 整合の確認: ST53（許諾の同梱）を、ST54 の上流（#235）が入った main へ rebase した組み合わせ（2026-11-03）

- 基準: `6c84c86752546890482bac0f81c831e38425b028`..`3d7192d51099eaeb09e55de7bdc592edfaabee79`

見る向きは 2 つ。
(1) main が改訂した FR-106 の ★ 2026-11-03（ST54 Q12・Q13: C-01 と C-02 は系統の違う S-01 へ送らず手元に残し、送り先の知らせを出す）と、ST54 の成果物（`openspec/changes/st54-dev-side-by-side/` 一式）・handoff ST56・stories.json の変更が、ST53 の成果物と食い違っていないか。
(2) ST53 が改訂した ID（FR-127 の ★ 群・§5「配布の方針」の ST53 の箇条）と、main から入った ST54 の成果物が食い違っていないか。
処置済みの監査（`2026-11-02-st53`・`2026-11-03-st53-r2`・`-r3`・`-rebase`・`2026-11-03-native-postgres`・`2026-11-03-st54-q12`）の決着は前提にした。

## 機械の入力（写し）

`python3 scripts/req_impact.py . --base 6c84c867…`: 改訂した要件は 1 件（FR-106）、扉は 0 項。

- FR-106（改訂）を名指ししている箇所: `docs/handoff/ST55.md:22`・`docs/handoff/ST56.md:3, 5, 8, 9`・`docs/stories/INDEX.md:78, 529`・`docs/stories/stories.json:1286`・`openspec/changes/st54-dev-side-by-side/` の中（agreement.json・base/distribution.md・deep-questions*.json・deep.md・design.md・proposal.md・provisional-review.md・review/deep.md・review/spec.md・specs/distribution/spec.md）・`openspec/specs/distribution/spec.md:734`
- ST53 の成果物（`openspec/changes/st53-license-bundle/`・`docs/stories/ST53.md`・`docs/handoff/ST53.md`）は FR-106 を名指ししていない

`python3 scripts/check_chain.py .`（chain: FAIL 1 件 / 未回収 0 件 / warn 0 件）:

- [FAIL] 整合の確認: 整合の確認の後に要件が変わった（この監査を名指ししない改訂）: FR-106（この監査で見た）

ほかに走らせたもの: `python3 scripts/spec_overlap.py check . st53-license-bundle` と `… st54-dev-side-by-side` はどちらも `{"ok": true, "candidates": 0}`。`python3 scripts/imp_ledger.py risks .` は「ST53 IMP-01: 判断し直す（責任者 本人・時期 ST54 の上流の合意まで）」「ST53 IMP-02: 同じ」を出した。`python3 scripts/check_scenarios.py . st53-license-bundle` は「担保なし 52 件」（実装の前なので担保が無いのは期待どおり）。

## 1. 改訂後の要件と、ほかの文書

## C1. ST53 の IMP-01・IMP-02 は「判断し直す時期: ST54 の上流の合意まで」で閉じているが、その時期はもう過ぎた。ST54 の上流は判断し直さずに合意して main に入り、ST53 の第 5 回・第 6 回の合意は過ぎた時期のまま受け入れた
- 重大度: 高
- 文書: openspec/changes/st53-license-bundle/agreement.json:23, 39
- 要件: —（ST53 の Requirement「正式な鍵を読む公開は、本人の承認を経る」。扉 34）
- 根拠: ST53 の agreement.json:23・:39 `"due": "ST54 の上流の合意まで"`（IMP-01・IMP-02 とも `"release": "recheck"`）。go.json:2031・:2088 も同じ値。deep.md:207-209・:223-225「リリースの前の判断し直しは go.json の `if_accepted.recheck`（本人・ST54 の上流の合意まで）」。docs/release.md:29「記録上の期限は「ST54 の上流の合意まで」だが、鍵は一度読まれると取り返せないので、鍵を置くのがそれより先ならここで判断する」。いっぽう ST54 の agreement.json は `"status": "accepted"`・`"at": "2026-11-03T07:12:00Z"` で、#235 は 8184a7d9（2026-11-03 16:43 +0900）で main に入った。ST53 の合意は第 5 回が 08:24Z、第 6 回が 09:39:45Z（agreement.json の `at`）で、どちらも ST54 の合意より後なのに同じ時期で受け入れている。ST54 の成果物（go.json・deep.md・agreement.json・docs/briefs/ST54-pr.md）は ST53 の IMP-01・IMP-02 を一度も名指ししておらず、判断し直した記録はどこにも無い。release.md:29 は「鍵を置くのが時期より先なら」という向きしか考えていないが、実際は時期の方が先に来て、判断し直さないまま過ぎた。過ぎた時期のまま残るので、`imp_ledger.py risks` は今も「時期 ST54 の上流の合意まで」と、まだ先のことのように出す。`scripts/imp_ledger.py:184-186` の `_due_passed` は `YYYY-MM-DD` しか読まないので、この形の時期はいつまでも過ぎたことにならない
- 失われるもの: なし（文書だけ）。ただし判断し直す時期が無いまま進むと、IMP-02 の残るもの「エージェントが同じ利用者の設定から本人の管理者の token を読めば、承認者を外して正式な鍵を読める」に、いつ判断するかという止めが無くなる（鍵は一度読まれると取り返せない。spec「正式な鍵を読む公開は…」の導出元）
- 直すところ: その文書（ST53 の go.json の `if_accepted.recheck.due` を、まだ来ていない時点（例: 「正式な鍵を `release` に置く前」）にして IMP-01・IMP-02 を聞き直す。docs/release.md:29 の「記録上の期限は…」の段落と deep.md の写しも合わせる）
- 機械: できない（`_due_passed` が日付の形しか読まない）。安く捕まえられる: `imp_ledger.py`（と `agreement.py` の受け入れ）で `ST<NN> の上流の合意まで` の形の時期を、その Story の change の `agreement.json` の `status: accepted`（か archive 済み）で過ぎたと判定する
- 行き先: docs（`docs/release.md` §3 の「記録上の期限は…」を、期限は判断し直さないまま過ぎたので、期限に依らず正式な鍵を `release` に置く前に必ず判断する、に直した。go.json・agreement.json の `due` は本人が受け入れたときの記録なので書き換えない（書き換えると合意の digest が変わる）。`_due_passed` が日付の形しか読まない点は harness2 側の課題として本人に伝える）

## C2. ST53 の GO 資料は、ST54 の開発版の鍵（`nightly` の Environment）と、ST53 が絞るエージェントの資格情報との関係を「ST54 の上流はまだ無い」として見なかった。その ST54 の上流は入ったが、ST54 の側でもそこは見ていない
- 重大度: 中
- 文書: openspec/changes/st53-license-bundle/go.json:2470-2471
- 要件: FR-106, FR-127（扉 34）
- 根拠: go.json:2470-2471（`impact.coverage.not_examined`）「ST54 が開発版の APK を組む形と、開発版の Environment に鍵を置く時期と、この Story の資格情報の切り替えの前後関係」/「ST54 の上流はまだ無い。申し送り（docs/handoff/ST54.md）を読んだだけ」。go.json:2114（IMP-03 の unknowns）「nightly の Environment（ST54 で開発版の鍵が載る）に対して何が変わるかは見ていない」。ST53 design.md:60-66（D3）では、エージェントの資格情報が「Contents の読み書き（枝と印の push）」「Workflows の書き」を持ち、ハーネスが `gh pr merge` を使う。ST53 の spec の承認の Requirement は「正式版とベータ版の公開の job で、署名の前に `approvals` を通し」と範囲を限っている。いっぽう ST54 の spec「携帯の APK は、チャンネルごとに決めた識別子と鍵で署名する」は「開発版の鍵を開発版の予定の実行と main からの実行にだけ開いた置き場（開発版の Environment）から読み」、design.md:98-101 では `publish` の job（`environment: nightly`）が毎日の予定の実行で鍵を読む。承認者が付かないので、main に入った変更なら鍵を読める。ST54 の design・premises・tasks に「承認」「資格情報」「token」で Environment の守りを論じた箇所は無い（grep 0 件。tasks.md:30 は「ログに出さない」だけ）。docs/handoff/ST54.md は「開発版の鍵は一度配ると替えられない（替えると開発版を入れた携帯は消して入れ直し、手元の未送信が消える）」と書く。ST53 が「後で見る」とした点を、そのとき見る側（ST54 の上流）が拾わずに閉じた
- 失われるもの: なし（文書だけ）。ただし開発版の鍵が読まれて替えることになれば、開発版を入れた携帯の未送信が消える（`discarded` の型。handoff ST54）
- 直すところ: その文書（ST53 の go.json の `not_examined` の why を、ST54 の上流が入った今の事実に直す）。開発版の鍵の守り（エージェントの資格情報で main に入れた変更が `nightly` の鍵を読めるか）は、凍結した Story（ST54）への申し送りにするか、新しい問いにする
- 機械: できない（読み手の判断）。`not_examined` の why に出てくる「ST<NN> の上流はまだ無い」の形なら、その Story の change が在るかで安く古さを出せる
- 行き先: handoff ST54（`docs/handoff/ST54.md` の「ST53 との合わせ目」。開発版の鍵を `nightly` に置く前に、エージェントの資格情報で main に入れた変更が鍵を読めるかを確かめる。ST53 の go.json の `not_examined` は合意の時点の記録として残す）

## C3. FR-108 の ★ は `docs/release.md` を §7 と番号で指す。ST53 の Task 10.2 に加えて、main から入った ST54 の Task 11.1 も release.md に節を足すが、足す位置を決めておらず、§7 の参照のことを ST54 の側は知らない
- 重大度: 中
- 文書: docs/requirements.md:1004
- 要件: FR-108
- 根拠: requirements.md:1004「ST53 の後に Docker の DB のまま切り替えない（Q5 の「移さない」と合わせる。`docs/release.md` §7）」。docs/release.md:80「## 7. 本人の環境を移す」。前回の監査 rebase の C2 の決着は handoff ST53（docs/handoff/ST53.md:35-37）だけに書かれていて、内容は「Task 10.2 で release.md の §3〜§5 を組み替えるときは…番号を合わせ（ずれるなら要件の参照を節の名前に直す）」。新しく入った openspec/changes/st54-dev-side-by-side/tasks.md:161-163 は「`docs/release.md` に節を足す: 開発版の鍵を作る…・開発版を並べて入れる…・開発版のサーバと DB…・開発版の DB が止まったら…」と書き、足す位置を決めていない。中身から見ると §2（鍵を作る。release.md:21「### 開発版」）〜§5 の近く、つまり §7 より前に入る。docs/handoff/ST54.md には §7 の参照の制約が無い。ST53 と ST54 のどちらが後に入っても、§7 の番号はもう一度ずれうる
- 失われるもの: なし
- 直すところ: requirements（FR-108 の ★ の参照を節の名前「本人の環境を移す」で指す。前回の C2 の決着が残した選択肢。こうすれば 2 つの Story がそれぞれ番号を合わせなくて済む）。走っている ST53・ST54 の tasks には印だけを付ける
- 機械: 安く捕まえられる（`release.md` §<n> の参照と見出しの番号・名前の突き合わせ。いまはどの検査も見ない）
- 行き先: handoff ST54（`docs/handoff/ST54.md` の「ST53 との合わせ目」。ST53 の側は `docs/handoff/ST53.md` の rebase C2 の申し送りが既に同じ制約を持つ）

## C4. ST54 の tasks は、ST53 が先に main に入った場合に受ける制約（新しいファイルの許諾の表示・配布物の許諾の 3 点・期待する LICENSE の比べ方・release.md の順の検査）を書いていない。ST54 の proposal は「ST53 は requires に無い」とだけ言う
- 重大度: 低
- 文書: openspec/changes/st54-dev-side-by-side/proposal.md:52
- 要件: FR-127, FR-106
- 根拠: ST54 proposal.md:52「ST53（許諾）は requires に無い —— ST54 は本物の開発版を公開しない（Q9）」。ST54 の proposal・design・tasks・spec・premises で FR-127・`SOURCE.txt`・`THIRD-PARTY`・許諾・SPDX を grep すると、この 1 行しか出ない。ST53 が先に入ると、ST54 の下流は次の 4 つに当たる。
  (a) ST53 の spec「決めた許諾の表示でないソースがあれば、表示の検査が落ちる」と tasks.md:16 の Q5（式 `AGPL-3.0-only WITH AdditionRef-ashiato-play-services`）。ST54 が足す Rust・Kotlin のファイル（`st54_tests.rs` ほか）に今の書き方で `AGPL-3.0-only` の表示を付けると、`chain` の job の `check-spdx.sh` が落ちる。
  (b) ST53 tasks.md:159-165（7.1）で、`check-artifacts.sh` はどの配布物にも 3 点を求めるようになる。ST54 tasks.md:140-143（10.1）が `--self-test` に作る開発版の 4 つの偽の配布物に 3 点が無いと、rc=0 を期待する Scenario「開発版のリリースの配布物は、開発版の APK を含む 4 つで…」が許諾の欠けで落ちる。
  (c) ST53 design.md:243 では「`draft.sh` は走らせた手元の木の版番号と `LICENSE` で比べる」で、`docs/release.md` の下書きの手順は「その PR の head の commit を checkout した手元で走らせる」（tasks.md:224）となる。ST54 tasks.md:155・:173 の `draft.sh --channel nightly` は `nightly.yml` の run（main の HEAD から組む）の配布物を載せるのに、どの木で走らせるかを書いていない。
  (d) ST53 の Task 10.2 の検証（tasks.md:226）は、`ASHIATO_SIGNING_KEYSTORE_B64` が最初に出る行が「エージェントの資格情報」より後にあることを見る。ST54 の Task 11.1 が開発版の鍵の Secrets（`publish-nightly.sh` が読む `ASHIATO_SIGNING_*`。ST54 tasks.md:145）を §3 より前（§2「開発版」）に書くと、この順は崩れる。
  (a)(b)(d) は CI か検証の rc で止まり、黙っては通らない。(c) が食い違うのは、手元の木と run の commit で `LICENSE` が違うときだけ
- 失われるもの: なし
- 直すところ: 凍結した Story への申し送り（docs/handoff/ST54.md に「ST53 が先に main に入ったら、新しいファイルの表示は ST53 の式・偽の配布物に 3 点・`draft.sh --channel nightly` は run の head を checkout した手元で・開発版の鍵の Secrets の節は §3〜§5 の順の後」と書く）か leave（CI が止める）
- 機械: (a)(b)(d) は捕まえられる（`check-spdx.sh`・`check-artifacts.sh --self-test`・10.2 の検証）。(c) はできない（手元の木の選び方）
- 行き先: handoff ST54（`docs/handoff/ST54.md` の「ST53 との合わせ目」に (a)〜(d) を書いた）

## 2. 文書とコード

## K1. ST53 の tasks.md:13 は「Scenario は 51 本（`distribution` 48・`verify-environment` 3）」と書くが、Q17 を写した後の spec は 52 本（49・3）ある
- 重大度: 中
- 文書: openspec/changes/st53-license-bundle/tasks.md:13
- 要件: FR-127
- 根拠: tasks.md:13「**この change の Scenario は 51 本**（`distribution` 48・`verify-environment` 3）」。`grep -c '^#### Scenario:'` で数えると、specs/distribution/spec.md は 49、specs/verify-environment/spec.md は 3。`python3 scripts/check_scenarios.py . st53-license-bundle` は「担保なし 52 件」。増えたのは Q17 の写し（6efd3205）で足した「配布物の LICENSE は、渡した期待する LICENSE と比べる」（spec.md:249）で、6c84c867 の spec では 48 本だった。新しい Scenario は Task 7.1 の本文と検証（tasks.md:176 の grep が 9 本）が名指ししているので、古いのは合計の数だけ。review/spec.md:267 の「51 件…tasks.md:13 の「51 本（48・3）」と合う」も Q17 の前の記録
- 失われるもの: なし
- 直すところ: その文書（tasks.md:13 を 52 本（49・3）に）
- 機械: 安く捕まえられる（tasks.md の「Scenario は N 本」の宣言と spec の `#### Scenario:` の数の突き合わせ。`check_scenarios.py` か `openspec validate` の後段で）
- 行き先: leave: 数の宣言だけが古く、増えた Scenario は Task 7.1 の本文と検証が名指ししていて、`check_scenarios.py` が spec の全 Scenario を突き合わせるので、黙って抜けることは無い。tasks.md を直すと合意の digest が変わり、文の訂正のためだけに聞き直しになる。下流が Task に [x] を付けるときに直してよい

## 走っている change の印（差し戻しは書かない）

- `openspec/changes/st53-license-bundle`・`openspec/changes/st54-dev-side-by-side`: 両方の tasks が同じファイルに手を入れる。重なるのは `tools/release/check-artifacts.sh`（ST53 7.1 / ST54 9.x・10.1）、`tools/release/check-guards.sh`（ST53 6.1・8.1・8.2 / ST54 10.3）、`docs/release.md`（ST53 10.2 / ST54 11.1。さらに ST52 が残した「ST54 まで…」の文が release.md:21-23・:35・:47 にあり、ST53 10.2 は触れず、ST54 が書き直す側）、`CLAUDE.md` の「構成」の `tools` の行（CLAUDE.md:22。ST53 10.3 が `licenses/`、ST54 11.1 が `android-side-by-side.sh` を同じ行に足す）。後から入る側は rebase と移し替えで済み、uncaptured・discarded・exported・rewrite-all のどれにも当たらない。印を付けるだけ
- `openspec/changes/st44-collector-credential-scope`: この基準の窓では変わっておらず、FR-106・FR-127 とも名指ししていない

## 未確認

- ST53 の go.json の `recheck.due` を直すと、合意の digest（agreement.json の `digest`）が変わって聞き直しになるか。`agreement.py` の digest に impact.findings の if_accepted が入るかは見ていない
- ST54 の `nightly.yml` を手で起こしたとき（Task 12.1）、組むのが起こした枝の木か main の HEAD か（`nightly.yml` の checkout の ref）。C4 (c) の LICENSE の食い違いが実際に起きるかはこれで決まる
- GitHub の `nightly` Environment に、エージェントの資格情報（ST53 D3 の権限）で main に入れた workflow の変更から Secrets が渡るか（C2）。文書から読んだだけで、走らせていない
- ST54 の下流が新しい Rust・Kotlin のファイルに SPDX の表示を付けるか（C4 (a)）。今の慣習（`crates/server/src` の 109 ファイルが表示を持つ）から付けると読んだだけ

## 触らなくてよいと判断したもの（理由つき）

- **FR-106** の ★ 2026-11-03（C-01・C-02 は系統の違う S-01 へ送らず手元に残し、送り先の知らせを出す）: ST53 の成果物は「系統」（開発版 / それ以外。契約の版の答えの見出し）に触れない。`SOURCE.txt` と `check-artifacts.sh` が分けるのは「チャンネル」（`release` / `beta` / `nightly` と、PR・main への push）の軸で、両者は直交する。PR と main への push の組み立ては、ST54 の spec「版番号を渡さずに組んだ部品は正式版として振る舞う」では正式版の系統、ST53 では commit で示す組み立てで、どちらの決まりも相手の決まりを固定しない
- ST53 の go.json の `impact.dependencies[9]` の前提（開発版の APK も `build.sh apk` を通って組まれ、3 点と commit の `SOURCE.txt` を持つ）: ST54 design.md:101 の「組み立ての job に `build.sh apk` がある」で成り立つ。署名は公開の job で後から行い、assets は変わらない
- ST53 の spec の `--channel nightly` の決まり（組み立ての印と同じ 40 桁の commit）と ST54 の `publish-nightly.sh`・`draft.sh --channel nightly`（どちらも `check-artifacts.sh --channel nightly` を通す）: どちらも配布物の中の `SOURCE.txt` と組み立ての印を比べるので、食い違わない
- ST53 の承認の Requirement は「正式版とベータ版の公開の job」に限るので、ST54 の `nightly` の `publish` の job に `approvals` が無いことと矛盾しない（守りが弱いことは C2）
- Requirement と Scenario の名前の衝突: `spec_overlap.py check` は両方とも candidates 0。ST53 と ST54 の Scenario の名前の積集合は空。重複して出るのは ST54 の MODIFIED が正典から写した Scenario だけで、期待どおり。ST53 は ST54 が REMOVED する 2 つの Requirement を名指ししていない
- ST54 が新しい Rust の crate や Android の依存を足さないこと（ST54 tasks.md:32）: ST53 の許可の一覧（「17 の値とちょうど同じ」の Scenario）を揺らさない
- handoff ST56（新規）のコードの参照（`crates/server/src/contract_version.rs:52`・`ContractGate.kt:27`・`crates/collector-windows/src/runtime.rs:819`）: 行の中身まで一致した。ST53 とは関係しない
- stories.json の ST54 の done の追加（「本物の開発版の公開と鍵は、ST53 の後に本人が始める」）: ST53 の Q4（最初の本物の公開は ST53 が main に入った後に本人が行う）と合う。ST58 の requires に ST54 を足したことは ST53 に関係しない
- ST54 が FR-106 を名指ししている自分の成果物（design.md:106・130・187-193、review/spec.md ほか）と handoff ST56 の中身: `2026-11-03-st54-q12` で決着済み。出し直さない
