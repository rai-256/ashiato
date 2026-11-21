# CH22 深掘りの問いの独立検査（deep-review）

対象: `openspec/changes/ch22-soak-install-online-ci/deep-questions.json`（Q1〜Q7）・`premises.md`（P1〜P9）。
手順 1〜8 を一覧を見る前にやり直し、後から突き合わせた。議論で決めたこと（CH22.md「議論で決めたこと」）は聞き直しの対象にしていない。

## 手順ごとの記録

- 手順 1（要件どうしの衝突）: 正典 `spec.md:2523`（本物の配布元に問い合わせない）は改めることが決まっている。D18 の「job の終わりまで外さない」と「塞ぎを外して置く」の食い違いは Q5 の context が扱っている。
  ただし、決定の字義の順序（入れた後に外して置く）と、コードの 24 時間の確かめの間隔がかみ合わない → R5
- 手順 2（扉の幅）: D18 の「残す」は「最初のインストールより前に塞ぐ」部分だけと読める。Q5 の context と合っている。これ以外は該当なし（確かめた範囲: archive/2026-11-14-st57-update-notice/design.md:152-156）
- 手順 3（新たに立つ一方通行）: `loss` のある決定は見つからなかった。配布元から見えるのは runner の IP と時刻だけ。公開の CI ログに出るのは、捨てる runner の上の一時の秘密だけ → R13（C）。どの問いも B で正しい
- 手順 4（日常）: PR ごとの Windows の runner の時間（2 倍で数える）→ R12（C）。5 分という長さは本人の決定なので聞き直さない
- 手順 5（既存コードと要件）: R1・R2・R3・R4・R8
- 手順 6（過去の取りこぼし）: 次の観点がこの change に当てはまり、一覧に無かった
  - 「OS のサービス・コマンドを情報源にする決定は実機で確かめ、前提欄に書く」→ R7
  - 「生存信号が途中の失敗を見ていない」「記録が目的の事実を表していない」「検証コマンドが 0 本でも通る」→ R1
  - 時計の族は該当なし（確かめた範囲: `update-check/src/lib.rs:255-258` は `last > now` を「期限」と扱う。前の段の試験の差し込み `test-hooks.json` の `now` は、配る組み立てでは読まない（`updater/src/lib.rs:96-101`））
- 手順 7（族の網羅）: Q2 の兄弟（起こし直されずに止まったまま・終わりの健康）→ R2。Q4 の兄弟（記録が 1 行も無い）→ R1
- 手順 8（前提の確かさ）: P1（2）→ R6、P9（2）→ R5、P7（4）が通知の登録を確かめていない → R3、P2 の「1」の部分 → R7。`loss` のある決定が無いので、「確かさ 4 未満に loss のある決定が載る」型の指摘は無い
- 構成の記号（S-01 など）: 該当なし（`grep -noE "\b[SCDVA]-0[0-9]\b|FR-[0-9]+|NFR-[0-9]+" deep-questions.json` が rc=1）
- 推奨の有無: Q1〜Q7 のどれにも `recommended` がある（第 1 回の条件を満たす）

## R1. 置く段で更新係が本物の配布元に実際に問い合わせたかを見る判定が無い。このままだと問い合わせ 0 件でも緑になる
- 成果物: openspec/changes/ch22-soak-install-online-ci/deep-questions.json
- 根拠: crates/update-check/src/lib.rs:255-258（`due` は前回の確かめから 24 時間）/ crates/updater/src/lib.rs:241-243（期限でなければ問い合わせずに `None`）・:269-270（届かなかった回も `checked_at` を書く）・:298-305 / .github/workflows/ci.yml:613-614（最初のインストールより前に塞ぐ。その時に更新係は塞がれたまま 1 回確かめ、`checked_at` が立つ）/ crates/installer/src/main.rs:2819-2823（uninstall は更新係の置き場を残す）/ tools/release/update-check-install.sh:285（「切った更新の確認は、走らせ直しでも切ったまま」）
- kind: technical
- 提案: Q4 の兄弟として B の問いを足し、推奨を付ける。問いは「置く間に、段の始まりより後の時刻の確かめた記録が 1 行も無ければ落とすか。『確かめた』が 1 回は要るか」。context には、記録が出ないまま緑になる道を書く。道は 3 つ: 24 時間の間隔・塞ぎの外し漏れ・残った `check.json` の「切った」
- 処置: fixed deep-questions.json — Q4 を「本物の配布元に聞けたか」に組み直し、推奨を「段の始まりより後の『確かめた』が 1 回は無ければ落とす（上限だけ警告）」にした。記録が出ないまま緑になる道を why に書いた

## R2. Q2・Q3 は「起こし直されずに止まったまま」を捕まえない。置き終わりの健康を見る問いが無い
- 成果物: openspec/changes/ch22-soak-install-online-ci/deep-questions.json
- 根拠: crates/server/src/service.rs:595-597・:663-667（起動の拒否は終了コード 0 を返すので、`failureflag 1` の回復は起こし直さない）/ 拒否はログに 1 行だけ書くので、Q3 の推奨（同じ誤りが 3 回）にも当たらない
- kind: technical
- 提案: Q2 を広げるか、兄弟の B を足して推奨を付ける。問いは「置き終わりに 3 つのサービスが実行中で、`/healthz` が 200 でなければ落とすか」。PC の収集アプリを置くなら、そのプロセスが残っているかも同じ問いに入れる
- 処置: fixed deep-questions.json — Q2 を「落ちた・止まったとみなす条件」に広げ、置き終わりに 3 つのサービスが実行中で /healthz が 200 か（PC の収集アプリを置くならプロセスが残るか）を同じ問いに入れた

## R3. Q1 の前提が誤り。PC の収集アプリは 5 分の間に配布元へ更新を確かめない
- 成果物: openspec/changes/ch22-soak-install-online-ci/deep-questions.json
- 根拠: crates/collector-windows/src/update_check.rs:148-163（初めて起きたときは案内の通知を出して戻り、確かめない）/ crates/collector-windows/src/runtime.rs:359-360・:510-515（定期の確かめの最初は起動の 1 時間後）/ update_check.rs:208（24 時間の間隔）/ crates/collector-windows/src/main.rs:222-253（通知の差出人を登録できなければ、確かめる係そのものを付けない）
- kind: premise
- 提案: Q1 の context の「初めて起きたときに配布元へ更新を確かめます」と、選択肢 1 の「更新の確認（本物の配布元との握手と一覧の読み取り）までは通ります」を直す。why の「握手の不具合は PC の収集アプリにもあった」も、置く段では捕まえられないと書き直す。確かめさせたいなら、状態（`intro_shown`）を先に置いてから起こす選択肢を足す。P7 は通知の登録を確かめていない
- 処置: fixed deep-questions.json — Q1 の why・context を直し（1 回目の起動では聞かない・定期は 1 時間後）、推奨を「設定せずに置き、途中で 1 回起こし直して確かめさせる」にした。premises に P10 を足した

## R4. Q4 の前提が誤り。理由の語は「ashiato 側の誤りか、外の揺れか」で分かれていない
- 成果物: openspec/changes/ch22-soak-install-online-ci/deep-questions.json
- 根拠: crates/updater/src/lib.rs:187-191（更新係は 403・429 以外の 5xx を含む状態の番号を「読めない答え」にし、時間切れでない接続の失敗は TLS の確かめの失敗も含めて「配布元に届かない」にする）/ crates/collector-windows/src/update_check.rs:479-485（PC の収集アプリは 5xx を「届かない」にする。同じ 503 で 2 つの部品の理由が違う。CH20 で挙がった既知の食い違い）/ crates/updater/src/lib.rs:396-401・crates/collector-windows/src/tls.rs:88（tls_online_* は `cargo test` の試験の組み立てで、配る組み立ての実行ファイルではない）
- kind: premise
- 提案: context に実際の対応表を書く。推奨の向きの根拠（読めない＝ashiato 側、届かない＝外）は成り立たない。配布元の 502・503 は赤になり、根の確かめを誤ったら警告で通る。これを踏まえて選択肢を組み直す（R1 の「1 回は確かめた」との組み合わせも含める）
- 処置: fixed deep-questions.json — Q4 の context に理由の語の実際の対応を書き、選択肢を組み直した（推奨は理由で分けず「確かめた」が 1 回あるかで見る）。premises に P12 を足した

## R5. P9 と Q5 の前提が誤り。消す口で消しても入れ直した更新係は 24 時間確かめない。Q5 の推奨の順序は、決定の字義の順序と違う
- 成果物: openspec/changes/ch22-soak-install-online-ci/premises.md
- 根拠: crates/installer/src/main.rs:2819-2823・tools/release/install-check.sh:1330（`uninstall` は `C:\ProgramData\ashiato-updater`（status.json・check.json）と状態の置き場を残す）/ update-check/src/lib.rs:255-258 / 前の段が消すときは台本の掃除（tools/release/update-apply-install.sh:190-212 `reset_machine`、install-check.sh:655-670）を使う / CH22.md「入れた後に外向きの塞ぎを外して数分置き」
- kind: premise
- 提案: 3 つ直す
  - P9 の「消す口がある」を「uninstall は更新係の置き場を残す。消すのは台本の掃除」に直す
  - Q5 の推奨の「消す」を、更新係の置き場まで消す意味だと書く
  - 選択肢 2（最初の段の直後に置く）には「更新係は塞がれたまま確かめ終えているので、外しても 5 分の間は問い合わせない」を足す
  - あわせて、推奨の順序（外す → 入れる）が決定の字義（入れる → 外す）と逆になる理由を context に 1 行書く。字義の順序だと、更新係の確かめは塞がれた中で済んでしまう
- 処置: fixed deep-questions.json — P9 を直し（消す口は置き場を残す・台本の掃除が要る）、Q5 を「何の上に、どの順で置くか」にして、決定の字義の順序を変えることを why に名指しした。字義どおりの順で聞かせる案を選択肢 2 に置いた

## R6. P1 の内容が違う。更新係は 1 時間ごとに起きるが、確かめるのは前回から 24 時間経ったときだけ
- 成果物: openspec/changes/ch22-soak-install-online-ci/premises.md
- 根拠: crates/updater/src/lib.rs:298-305（1 時間ごとに `once` を呼ぶ）・:241-243（`due` でなければ問い合わせない）/ crates/update-check/src/lib.rs:255-258
- kind: premise
- 提案: P1 を「起きたときと 1 時間ごとに、前回の確かめから 24 時間経っていれば 1 回問い合わせる（状態が残っていれば 0 回）」に直す。直したら、P1 に依る Q6 の「数回しか聞きません」と R1・R5 を照らし直す
- 処置: fixed premises.md — P1 を「起きたときと 1 時間ごとに、24 時間経っていれば 1 回」に直した。Q6 は外した（R9）

## R7. Q2 が数える出所（サービス制御の記録・アプリケーションの誤りの記録）を runner で確かめた前提が無い
- 成果物: openspec/changes/ch22-soak-install-online-ci/premises.md
- 根拠: 過去の取りこぼしの観点「OS のサービス・コマンドを情報源にする決定では…実機で確かめ、深掘りの前提欄に書く」（`scripts/upstream_misses.py report`）/ premises.md に、イベントの記録を Git Bash から読めるか、更新係の panic が異常終了の記録になるかの行が無い / tools/release/install-check.sh:850・:869（前の段がわざとサービスを終わらせるので、記録は時刻で切らないと混ざる）/ P2 の「4 回目以降も起こし直す」（確かさ 1）は crates/server/src/service.rs:646 の注記「`reset= 86400` で 1 日 3 回まで」と食い違う
- kind: premise
- 提案: premises に「数える出所と読み方」の行を足し、確かさと確かめ方を書く。Q2 の context に、数える窓（入れ直しの後から）と、P2 の食い違い（4 回目以降に止まるなら、0.1.0 型は 3 回の後に止まったまま。R2 の兄弟になる）を書く
- 処置: fixed premises.md — premises に P11（数える出所。確かさ 1）を足し、P2 を「4 回目以降は確かめていない・注記と食い違う」に直した。Q2 の context に数える窓（入れ直しの後から）を書いた。止まったままは Q2 の置き終わりの状態で捕まえる

## R8. Q3 の「panic の文字はどこに出ても落とす」は、更新係と PC の収集アプリでは空振りする。「誤りの行」が何かも決まっていない
- 成果物: openspec/changes/ch22-soak-install-online-ci/deep-questions.json
- 根拠: crates/updater/src/main.rs:4（tracing は標準出力へ。サービスでは捨てられる。panic の hook も無い）/ panic の hook はサーバだけ（`grep -rn "panic::set_hook" crates/` → crates/server/src/lib.rs:4583 だけ）/ updater/src/lib.rs:313-320（`updater.log` は全部の行が誤り）/ crates/installer/src/db.rs:33（DB のログは LOG・ERROR・FATAL が混ざる）/ crates/collector-windows/src/sender.rs:249（PC の収集アプリは `send_failed` などを警告の段で書く）
- kind: premise
- 提案: context を直す。0.1.0 型の panic は、更新係ではログに出ず、Q2 の起こし直しでしか見えない。Q3 の中か兄弟の B に、ログごとの「誤りの行」の定義を推奨付きで置く（どの段：ERROR・FATAL・WARN。DB の起動中の FATAL を数えるか）
- 処置: fixed deep-questions.json — Q3 の context から「panic はどこでも落とす」を外し、置き場ごとの「誤りの行」の数え方を書いた。更新係の panic はプロセスの終わり（Q2）で見えると Q2 の context に書いた

## R9. Q6 は決定の字義でほぼ決まっている。問わずに C にできる
- 成果物: openspec/changes/ch22-soak-install-online-ci/deep-questions.json
- 根拠: CH22.md「正典 distribution の…を、この段と #329 の握手の試験（tls_online_*）に合わせて改める」（例外を 2 つ名指しする形が字義そのもの）
- kind: technical
- 提案: Q6 を外し、推奨の文（名指しの 2 つ）を spec の段の既定にする。残すなら、選択肢 2 が決定の範囲を広げる側だと note に書く。あわせて、collector の tls_online_* は版の一覧でなく `https://api.github.com/` の根を叩く（tls.rs:95）ので、「配布元への問い合わせ 2 本」という数え方を直す
- 処置: fixed deep-questions.json — Q6 を外し、note の「聞かずに決めたこと」に、例外を 2 つ名指しして改めると書いた（C。spec の段で design に置く）。PC の収集アプリの tls_online_* が根を叩くことは spec の段で文に反映する

## R10. Q7 の選択肢 2 と Q3 の選択肢 3 は、本人が決めたことを聞き直している
- 成果物: openspec/changes/ch22-soak-install-online-ci/deep-questions.json
- 根拠: CH22.md「製品のコードは変えない」（Q7 の選択肢 2「この change の中で製品も直す」）/ CH22.md「誤りの記録の連続…を機械が判定する」（Q3 の選択肢 3「落とさず、出力に出すだけ」）
- kind: technical
- 提案: 2 つの選択肢を外す。または「決定を覆す側」と明記する
- 処置: fixed deep-questions.json — Q7 の選択肢 2 と Q3 の選択肢 3 を外した

## R11. 判定の閾値（Q2〜Q4）を正典の Scenario に書くか、design と台本に留めるかの問いが無い
- 成果物: openspec/changes/ch22-soak-install-online-ci/deep-questions.json
- 根拠: .github/workflows/ci.yml:618-626（install-windows は、台本の `--list` と出力の `== Scenario:` の行を突き合わせる）/ 正典に閾値を書くと、後で変えるたびに MODIFIED が要る
- kind: technical
- 提案: B の問いとして推奨付きで足すか、spec の段の既定として design に書く。どちらでも、置く段の判定のどれを Scenario の印にするかを決めておく。決めないと、下流がその場で決めることになる
- 処置: fixed deep-questions.json — C にした。正典の Scenario には観測できる結果（起こし直し 0 回・置き終わりに実行中・確かめた 1 回）を書き、数の閾値は design の D（仮）に置く。note の「聞かずに決めたこと」に書いた

## R12. 置く段に時間の上限が無い（install-windows に timeout-minutes が無い）
- 成果物: openspec/changes/ch22-soak-install-online-ci/deep-questions.json
- 根拠: .github/workflows/ci.yml:598-601（install-windows に `timeout-minutes` が無いので、既定の 360 分を 2 倍で数える）/ ci.yml:239-240（collector-windows-runtime は固まった実測から 25 分にした）
- kind: technical
- 分類の直し: 席は daily とした。処置の側で technical に直した（本人の日常の負担でなく、CI の job の時間の上限という道具の既定で、本人に見せる決定が無い）
- 提案: 問いにしない（C）。design の既定として、置く段の待ちに上限を付け、job の `timeout-minutes` を足す
- 処置: fixed deep-questions.json — C にした。note の「聞かずに決めたこと」に書いた（spec の段で design の既定と job の timeout-minutes にする）

## R13. 段の出力に出すログの行は公開の CI ログに出る。伏せる部品を通すことを前提に書く
- 成果物: openspec/changes/ch22-soak-install-online-ci/deep-questions.json
- 根拠: `gh api repos/rai-256/ashiato --jq .private` → false / tools/release/check-ng-observations.sh:1-15（CH16 の観測の部品が、伏せる値を伏せて出す）/ Q2・Q3 の「見つけた誤りの行は全部、段の出力に出します」
- kind: technical
- 提案: 問いにしない（C）。Q2・Q3 の context に「出力は観測の部品（伏せる値を伏せる）を通す」と 1 行書く。秘密は捨てる runner の一時の値なので、失うものは無い
- 処置: fixed deep-questions.json — C にした。note の「聞かずに決めたこと」に書いた（出力は CH16 の観測の部品を通して秘密を伏せる）
