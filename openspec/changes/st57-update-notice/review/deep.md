# ST57 の深掘りの検査

## 聞かない既定（C）

問いを立てた側が置いた C1〜C10 は `defaults.md`（change 直下）に移した。spec の段が design の D に写す。

## 手順ごとの結果（問いの一覧を見る前にやり直したもの）

- 手順 1（要件どうしの衝突）: 3 組。(a) FR-112 の「携帯・PC がそれぞれ配布元に確かめる」と、正典 data-sensitivity の「外へ要求を出すと本人の居場所が網の外へ出る」（`openspec/specs/data-sensitivity/spec.md:184`）の守りの理由 → R1。
  (b) FR-115「部品ごとに設定で切る」と FR-112 ★2026-11-03「更新係は FR-109 の置き場に書かず、サーバはそこを読むだけ」 → 一覧の Q3 と C6 が受けている（抜けなし。選択肢の記述は R12）。
  (c) FR-113 は携帯を「画面」とだけ書くが、推奨が通知を足す → R11。
- 手順 2（扉の幅）: 扉は無い（`docs/stories/ST57.md` の doors: []）。代わりに要件の語の幅として「利用者のチャンネル」（FR-112）が「入っている版のチャンネル」とも「本人が選んだチャンネル」とも読める → Q2 が受けている（前提の書き方は R15）。
- 手順 3（新たに立つ一方通行）: 外に出るものは 4 つ。問い合わせの元の IP と時刻（R1）・切る口より先に出る最初の問い合わせ（R2）・携帯の確かめの記録のクラウドのバックアップ（R3）・配った版に焼き込まれる「確かめ方」（宛先・読み方・証明書の根。R7。データは失わないが、配った版からは直せない）。列の型・鍵は無い（D-01 に何も足さない。結果は更新係の置き場のファイル）。
- 手順 4（日常に影響する選択）: 携帯の通知（Q5）・PC の通知の回数（Q6）・画面の帯（Q7）・確かめられない知らせ（Q9）・携帯の網（Q11）は一覧にある。抜けは携帯の通信量（R8。実測 3.9 MB/回の桁がありうる）。
- 手順 5（既存コードが要件を満たしていない箇所）: 更新の確認のコードはまだ 1 行も無い（`grep -rni "update.check|updatecheck|更新あり|updater|更新係"` が試験の 1 件以外 0 件）。
  満たしていないのではなく未着手。要件の★の古い記述が 1 つ: FR-115 ★「C-02 の設定はいまは環境変数だけ」（`docs/requirements.md:1035`）は、いまは `settings.json` を読む（`crates/collector-windows/src/config.rs:168`）。Q4 の context はいまの形で書けている。
  PC の収集アプリは TLS を持たない（`crates/collector-windows/Cargo.toml:18-21`、ST07 design D16 の反転条件 `openspec/changes/archive/2026-09-14-st07-active-window/design.md:253-260`）・携帯は見出しの User-Agent を自分で付けていない（`collector-android/app/src/main/kotlin/dev/ashiato/collector/HttpTransport.kt:52-59`）は premises の P8・P10 のとおり。
- 手順 6（過去の取りこぼし）: 当てはまったのは「時刻を入力にした判定の時計が戻る／進む」（R4）・「OS のサービスを情報源にする決定」（R9）・「手元の状態が丸ごと消えた」（R6 の末尾）。handoff（ST55 D14）は C6 が受けている。
- 手順 7（族の網羅）: C7 の「入れる／走らせ直す／更新」に「消す／repair」が無い（R10）。Q2 の「どの版を新しいとみなすか」に、開発版の入れ替えの途中・自部品の配布物が無いリリース・下書き・自分の版が一覧より新しい場合が無い（R6）。Q9 の「確かめられない」に網の切れ以外（404・403・読めない形）が無い（R7）。Q3/Q4 の「切る」に「最初の問い合わせより前に切る」が無い（R2）。
- 手順 8（前提の確かさ）: premises.md は「loss のある問いはこの回には無い」と書くが、R1〜R3 は exported を持つ。R1 が A になれば、P9（確かさ 2）が A の問いの前提になる。P5（確かさ 2）は Q2 の context が事実として使っている（R6 で前提に依らない形を提案）。P7（確かさ 2）は C2 の「1 ページ 100 件」が依る（R8）。確かさの段と確かめ方の食い違い（走らせていないのに 4 以上）は見つからなかった（P1・P2 は `curl` で 200・`x-ratelimit-limit: 60` を再現した）。
- 構成の記号: 問いの JSON とブリーフに S-01 / C-01 / C-02 / V-01 / D-01 は 0 件（`grep -oE "\b(S|C|V|D)-0[0-9]\b"`）。

## R1. Q1 は B でなく A —— 携帯と PC が直に聞くと、問い合わせの元の IP と時刻が毎日配布元へ出て戻らない。失わない側（サーバだけが聞く）は要件を変える
- 成果物: openspec/changes/st57-update-notice/deep-questions.json
- 根拠: Q1 の why「どちらも記録は外へ出ません。後から作り直せば戻ります（仮でよい）」と、Q1 自身の context「配布元からは、携帯の問い合わせの元の IP（居場所で変わる）が見えます」・Q11 の context「大まかな居場所の手がかりになりえます」が食い違う / 正典が外向きの要求を禁じた理由は居場所が網の外へ出ること（`openspec/specs/data-sensitivity/spec.md:184`「地図のタイルなどを外から読むと、表示している範囲（＝本人の居場所）が網の外へ出る」）/ C2 は固定の `User-Agent: ashiato-update-check` と固定の道（`/repos/rai-256/ashiato/releases`）で、いまの利用者は本人 1 人なので、この見出しの問い合わせはどこから来ても本人のものと読める / CLAUDE.md の規則「失わない側が…要件を変えないなら P」に対し、Q1 の選択肢 2 は FR-112・FR-115 と API の合言葉の口の一覧（`openspec/specs/data-sensitivity/spec.md:492`）を書き直す / premises.md の「失われるもの（loss）のある問いは、この回には無い」
- kind: irreversible
- loss: exported
- 提案: Q1 を A にし、選択肢 1 に「出たもの: 携帯と PC の問い合わせの元の IP と時刻が毎日配布元に残る（取り消せない）」を書く。why の「どちらも記録は外へ出ません」は「記録は出ないが、問い合わせの元の IP と時刻は出る」に直す。premises.md の冒頭の 1 文も直す。Q11 はこの答えに従う問いとして残す（Q11 単独では居場所を減らせない —— 外出先の Wi-Fi も居場所を語ると Q11 の context 自身が書いている）。
- 処置: escalated Q1 —— A・loss exported にし、why と選択肢の irreversible を直した。premises.md の冒頭も直した。推奨は要件どおりのまま、本人が選ぶ

## R2. 切る口より先に最初の問い合わせが出る —— C5 の「インストール直後・初回起動に 1 回確かめる」と、Q3/Q4 の「切る」の族の抜け
- 成果物: openspec/changes/st57-update-notice/review/deep.md（C5）・deep-questions.json（Q3・Q4）
- 根拠: C5「インストール直後・初回起動に 1 回確かめる（既定は有効。FR-115）」/ Q3 の推奨は「インストーラの命令で切る」で、インストールが終わった後にしか打てない / Q4 の推奨は「携帯はアプリの画面のスイッチ」で、初回起動の後にしか触れない / `docs/stories/ST57.md` 完了の判定「設定で部品ごとに確認を切ると、その部品からの通信が無くなる」は切った後のことしか言っていない
- kind: irreversible
- loss: exported
- 提案: 族の兄弟「最初の問い合わせの前に切れるか」を問い（B）にするか C5 を直す。推奨の候補: 初回の確認は、その部品の設定が済んでから（携帯・PC は FR-128 の設定を保存した後、サーバはインストールの最後の段（画面の合言葉を決める段）で「更新の確認: 有効」を示した後）に始める。費用は小さく、範囲も要件も変えない。
- 処置: escalated Q4 —— 最初の問い合わせは切る口に触れられるようになった後（defaults.md C5）。Q3・Q4 の context に書いて本人に見せる

## R3. C4 の「確かめた記録を追記で残す」は、携帯ではクラウドのバックアップに乗って外へ出る
- 成果物: openspec/changes/st57-update-notice/review/deep.md（C4）
- 根拠: 携帯のバックアップの規則は 2 つの sharedpref だけを外し、ほかは既定のまま（`collector-android/app/src/main/res/xml/health_data_extraction_rules.xml:3-9`「ほかの置き場（collector など）は既定のまま」・`health_full_backup_content.xml` も同じ）/ `tools/check-android-backup-rules.sh` が見るのは `Config.SETUP_PREFS` だけ（同 1-30 行）/ C4 は記録に「時刻・成否と理由」を持たせる（成否と時刻は、携帯がいつ網に繋がっていたかの記録になる）
- kind: irreversible
- loss: exported
- 提案: C4 に「携帯では確かめの記録と『通知済みの版』の置き場をクラウドのバックアップと端末の移行から外し、`check-android-backup-rules.sh` にその名前を足す」を書く。費用は小さい。外さない側を採るなら、その理由を C に書く。
- 処置: escalated Q1 —— Q1 の context に「携帯の確かめの記録はクラウドのバックアップから外す」を書いた（defaults.md C4）

## R4. 時計の族が無い —— C5 の「24 時間」、Q9 の「7 日」、Q7 の「見つけた日」はどの時計で数えるか
- 成果物: openspec/changes/st57-update-notice/review/deep.md（C5）・deep-questions.json（Q7・Q9）
- 根拠: `python3 scripts/upstream_misses.py report` の「最終成功時刻・間隔で動く判定には『前回が未来にある』場合を必ず問い…」「時刻を入力にして状態を一方向に動かす決定には、端末の時計が大きく進む／戻る 1 件が来たときの結果を必ず 1 行で問う」/ この repo の収集アプリは時刻サーバに合わせない（`tools/check-no-time-server.sh:1-10`）ので、端末の時計は狂ったままでありうる / サーバの定期の枠は壁時計を読まないと決めている（`tools/check-scheduled-clock.sh:1-4`）が、更新係には同じ決まりが無い
- kind: technical
- 提案: C5 に 1 行足す: 「最後に確かめた時刻が今より未来なら、期限が来たとみなして確かめる」（戻ったときに何年も確かめない形を避ける）。Q9 の「7 日」も同じ規則で数えることを context に書く。進みすぎは 1 日に何回も聞くだけで上限（60 回/時）に遠いので、それも 1 行で書く。
- 処置: escalated Q9 —— Q9 の context に時計の規則を書いた（defaults.md C5）

## R5. 試験と CI が本物の配布元に聞く —— 既定は有効で、初回起動で確かめる
- 成果物: openspec/changes/st57-update-notice/review/deep.md（C2・C5）
- 根拠: C5（初回起動に 1 回確かめる）と FR-115（既定は有効）/ 計測テスト（`collector-android/app/src/androidTest`）・PC の実行時テスト（`crates/collector-windows/tests/runtime_windows.rs`）・録画は、入れたてのアプリを毎回起こす / GitHub の runner は IP を共有するので、認証なしの 60 回/時（premises P2）に他人の分で当たりうる / C2 が宛先を固定し、正典は系統を実行時の設定で変える口を持たない（`openspec/specs/distribution/spec.md:992-998`）ので、試験で宛先を差し替える口の形がまだ無い
- kind: technical
- 提案: C を 1 つ足す（または B の問い）: 「宛先は組み立ての時の定数で、試験の組み立てだけが手元の受け手に向ける。配る組み立て（release）は宛先を変える口を持たない」。C2 の FR-116 の試験（手元の受け手で見出しを数える）もこの口を使う。
- 処置: fixed defaults.md —— C8 を足した

## R6. Q2 の「どの版を新しいとみなすか」の族の抜け（開発版の入れ替えの途中・自部品の配布物の無いリリース・下書き・先行公開の印）
- 成果物: openspec/changes/st57-update-notice/deep-questions.json（Q2）
- 根拠: 開発版のリリースは `--prerelease` で作られる（`tools/release/publish-nightly.sh:31-33`）ので、「先行公開 = ベータ版」と読むと開発版をベータ版に数える / 入れ替えは「新しい名前の配布物を上げてから古い名前を消す」（同 35-41 行）ので、途中では 2 つの日付の配布物が並び、自部品の配布物がまだ無いこともある / 開発版の確かめの組み立ては印 `nightly-draft-<run>` の先行公開の下書き（`tools/release/draft.sh:77`）で、出ないことは premises P5（確かさ 2・公式文書だけ）に依る / Q2 の context「下書きのリリース…は、認証なしの問い合わせには出ません」は P5 を事実として書いている
- kind: technical
- 提案: Q2 の context に、選択肢によらない読み方を書き足す（C でよい）: ベータ版は印の形 `v<版>-beta.<N>` で読み、先行公開の旗では読まない／`draft: true` は P5 に依らず捨てる／版は自部品の配布物の名前（`ashiato-collector-dev-<版>.apk` など）から読み、自部品の配布物が無いリリースは数えない／自分の版が一覧のどれより新しいとき（手で組んだ版）は「更新なし」。「通知済みの版」の置き場が消えた（入れ直し）ときは 1 回出し直すだけで害が無いことも 1 行で書く。
- 処置: escalated Q2 —— context に選択肢によらない読み方を書いた。P5 に依らず下書きは捨てる

## R7. 配った版に焼き込まれる「確かめ方」は、その版からは直せない —— Q9 の「確かめられない」に網の切れ以外を入れる
- 成果物: openspec/changes/st57-update-notice/deep-questions.json（Q9）・review/deep.md（C2）
- 根拠: 宛先のリポジトリは名前が変わったことがある: `tools/release/check-guards.sh:6` の既定 `rai-256/ashiato2` はいま `gh repo view` で `rai-256/ashiato-private`（PRIVATE）に解決し、認証なしの `curl .../repos/rai-256/ashiato2/releases` は 404（2026-11-11 実測）/ 宛先の URL・一覧の読み方・証明書の根（PC の収集アプリと更新係が足す TLS の根を部品に同梱するか OS の根を使うか。いまは決まっていない。`crates/collector-windows/Cargo.toml:18-21`）・転送（301）を追うか、は配った版の中で凍り、壊れたら新しい版を知る手段そのものが無い / Q9 の context は「網に繋がらない・更新係が止まっている」だけを挙げる
- kind: technical
- 提案: Q9 の context に「確かめられない」の中身を族で並べる（網に届かない・時間切れ・403 の上限・404 / 301 の宛先の移動・200 で読めない形・`[]` は『確かめた、新しい版なし』）。証明書の根の出所を C か B の問いにする（同梱の根は配った版の中で古くなる。推奨の候補は OS の根）。FR-116 の試験が「道が固定」を見るなら、転送を追うかどうかも C2 に書く。
- 処置: escalated Q9 —— context に「確かめられない」の族を並べた。証明書の根と転送は defaults.md C2

## R8. C2 の「1 ページ 100 件」「ほかの見出しを足さない」は、携帯の通信量を版の数に比例して増やす
- 成果物: openspec/changes/st57-update-notice/review/deep.md（C2）・deep-questions.json（Q11）
- 根拠: 2026-11-11 実測、公開のリポジトリ `cli/cli` の認証なしの一覧: `per_page=100` は非圧縮 3,934,746 バイト・gzip で 346,807 バイト、`per_page=30` は gzip で 138,157 バイト、`per_page=1` は 3,726 バイト（`curl -s [--compressed] -w '%{size_download}'`）。ashiato の 1 リリースはこれより小さい（配布物 4 つ）が、説明文と配布物の項目の数だけ育つ / C2 の「ほかの見出しを足さない」を字どおり取ると `Accept-Encoding: gzip` も足せず、非圧縮で受ける / 開発版のリリースは 1 度作ったら作り直さず一覧の中で沈む（premises P7、並びの規則は未確認・確かさ 2）
- kind: daily
- 提案: C2 に、FR-116 の試験が許す見出しの集合を名前で書く（`Host`・`User-Agent`・`Accept`・`Accept-Encoding` は部品が勝手に足すかを含めて決める。gzip は版の一覧の取得の一部とみなしてよいか）。件数を版の数に依らない形にする（例: 正式版・ベータ版は少ない件数、開発版は `/releases/tags/nightly`）かを C で決め、Q11 の context に 1 回の通信量の見込みを数字で書く。
- 処置: escalated Q10 —— defaults.md C2 で見出しの集合を名指しし、gzip を許し、件数を 20 件と開発版の 1 件にした。1 回の通信量の見込みを Q10 の context に書いた

## R9. サーバが「切ってある／動いていない／確かめられていない」をどこから知るかが無い
- 成果物: openspec/changes/st57-update-notice/deep-questions.json（Q3・Q9・Q10）
- 根拠: Q3 の context「切っている間、画面には『更新の確認: 切ってある』と出します」・推奨「更新係のサービスを止めて無効にする」/ Q10 の推奨「画面は『更新の確認: 動いていない』と出す」/ サーバが読めるのは更新係の置き場のファイルだけ（FR-112 ★2026-11-03、C6）。サービスを無効にするとファイルは書かれないので、「切った」と「壊れて止まった」がファイルからは同じに見える / `python3 scripts/upstream_misses.py report` の「OS のサービス・コマンドを情報源にする決定では、既定の起動種別・権限・止まっているときの出力を実機で確かめ、深掘りの前提欄に書く」/ premises.md にサービスの状態を誰が読めるかの前提が無い
- kind: technical
- 提案: C を 1 つ足す: 「切った印はインストーラ（管理者）が更新係の置き場に書き、サーバはその印と結果の時刻だけで 3 つを見分ける（サービスの状態を問い合わせない）」。サービスの状態を読む形にするなら、サーバのアカウントで `sc.exe query` が読めるかを premises に置き、Windows の CI で確かめる。
- 処置: fixed defaults.md —— C9 を足した（サービスの状態を問い合わせない）。Q3 の context にも書いた

## R10. C7 の族に「消す」「repair」が無い —— いまの消す口は 2 つのサービスしか消さない
- 成果物: openspec/changes/st57-update-notice/review/deep.md（C7）
- 根拠: `crates/installer/src/commands.rs:225-231`（`uninstall_commands` は `server_service` と `db_service` だけを止めて消す）/ 正典「2 つのサービスを止めて登録から消し」（`openspec/specs/distribution/spec.md:1962-1968`）/ 正典「状態の置き場は、2 つのサービスのアカウントだけが書け」（同 1581）と `repair` の口（同 1730）
- kind: technical
- 提案: C7 に「消す口は更新係のサービスも止めて登録から消し、更新係の置き場は状態の置き場と同じく残して場所を示す（または消す）」「`repair` は更新係に触れない」を足し、spec の段でこの 2 つの Requirement を MODIFIED にすることを書く。プログラムの置き場を消すので消し漏れても問い合わせは出ない見込みだが、壊れたサービスの登録が残る。
- 処置: fixed defaults.md —— C7 に消す口と repair を足した

## R11. Q5・Q9 の推奨は要件を足す —— 未回答のまま推奨で進むと、AI が要件を変えることになる
- 成果物: openspec/changes/st57-update-notice/deep-questions.json（Q5・Q9）
- 根拠: Q5 の推奨「画面に出し、新しい版ごとに 1 回だけ通知も出す…要件 FR-113 に足す」/ Q9 の推奨は FR-113 に無い 2 つめの知らせ（「更新を確かめられていない」）を 3 つの部品に足す / 規則「失わない側が…要件を変えないなら AI が仮で進む」（`CLAUDE.md`「進め方の要点」）/ 前例: 似た知らせの足し算は要件の★として足した（FR-126 ★2026-10-29、`docs/requirements.md:1090-1092`）
- kind: conflict
- 提案: どちらも失うものは無いので B のままでよいが、推奨を採ると要件の本文が変わることを note か why に書き、未回答のときの既定（仮で採る側）が要件の改訂を伴うことを明示する。既定を要件どおりの側に置くか、要件の改訂を「回答があったときだけ」にするかを決める。
- 処置: escalated Q5 —— Q5・Q9 の why と note に、推奨のまま進むと要件 FR-113 を ★ で改訂することを書いた

## R12. Q3 の選択肢 2 は、FR-112 の隔離の根拠を弱めることを書いていない
- 成果物: openspec/changes/st57-update-notice/deep-questions.json（Q3）
- 根拠: FR-112 の根拠「更新係は記録を持たないので、問い合わせの部品を持っても記録は外へ出せない」（`docs/requirements.md:1024`）/ 選択肢 2「サーバが更新係の設定を書ける口を 1 つ足す（更新係の置き場にサーバの書き込みを 1 か所だけ許す）」は、記録を持つプロセスから外へ送る部品を持つプロセスへの道を 1 本作る
- kind: conflict
- 提案: 選択肢 2 の detail に「サーバから更新係へ書ける道が 1 本できる。更新係が読むのは真偽の 1 値だけにし、ほかを読まないことを試験で固定する」を足す。
- 処置: escalated Q3 —— 選択肢 2 の detail に、道が 1 本できることと真偽の 1 値だけを読む試験を書いた

## R13. Q7 は画面の構造を文字で問うている
- 成果物: openspec/changes/st57-update-notice/deep-questions.json（Q7）
- 根拠: Q7 は「どの画面でも上の帯に小さく出す」か「設定の画面にだけ出す」かの置き場の問いで、帯が既存の見出し（`web/src/App.tsx:113` の `<header>`、134 行の設定への入口）とどう並ぶか・狭い幅で何行になるかは文字では分からない
- kind: daily
- 提案: `kind: visual` にし、いまの見出しの行に帯を足した形と設定の画面だけの形の 2 つを playground の HTML（`proto`）で示す。Q9 の「確かめられていない」も同じ所に出るので、2 つが同時に出た形も描く。
- 処置: escalated Q7 —— visual にし、proto.html（docs/briefs/ST57-proto.html）で置き場・状態・幅を示した

## R14. Q4・Q5 の前提「携帯のアプリの画面は権限を求めて収集を始める入口だけ」は、いまの実装と合わない
- 成果物: openspec/changes/st57-update-notice/deep-questions.json（Q4・Q5）
- 根拠: 携帯には設定の文字列を貼る画面がある（`collector-android/app/src/main/AndroidManifest.xml:89-91` の `.SetupActivity`、`MainActivity.kt:121`・`499` から開き、`SetupNotifier.kt:92-96` の通知からも開く）
- kind: premise
- 提案: Q4 の context を「携帯には権限の入口と、設定の文字列を貼る画面（PC の設定の窓と同じ ST56）がある」に直す。Q4 の推奨のスイッチの置き場も、Q5 の「画面に出す」の置き場も、この設定の画面を候補に挙げる。
- 処置: escalated Q4 —— Q4・Q5 の context を、設定の画面 SetupActivity がある形に直した

## R15. Q2 の context は、正典が固定する「系統」と「チャンネル」を混ぜている
- 成果物: openspec/changes/st57-update-notice/deep-questions.json（Q2）
- 根拠: 正典が実行時に変えられないとするのは系統（開発版か、それ以外か）だけで、ベータ版と正式版は同じ系統（`openspec/specs/distribution/spec.md:992-998`、`crates/collector-windows/src/toast_names.rs:3-7` は `Dev`・`Official` の 2 つ）/ Q2 の context「チャンネルは、組み立てのときの版番号から決まります（正典: 実行時の設定で系統を変える口は無い）」は、ベータ版を追う設定（選択肢 2）が正典に反するかのように読める / FR-112 の「利用者のチャンネル」は「入っている版のチャンネル」とも「本人が選んだチャンネル」とも読め、選択肢 1 はベータ版の人が正式版を入れるとベータ版から外れる
- kind: premise
- 提案: context を「正典が固定するのは系統（開発版かどうか）だけ。ベータ版と正式版のどちらを追うかは、設定で選ばせても正典に反しない」に直す。
- 処置: escalated Q2 —— context を、系統とチャンネルを分けて書き直した

## R16. Q10 は不要（要件が Linux の入れ方を後の Story に置いている）
- 成果物: openspec/changes/st57-update-notice/deep-questions.json（Q10）
- 根拠: FR-131「S-01 の Linux の配布物は ST52 が作るが、インストール・サービスの登録・更新の経路はまだ無い」「Linux の配布は一般向けの配布の前までに足す」と、Linux で D-01 をどう入れるかの [NEEDS CLARIFICATION]（`docs/requirements.md:1117-1121`）。選択肢 2 の systemd の timer は、まだ無い Linux の入れ方の一部を先に作ることになる
- kind: defer
- 提案: Q10 を外して C にする（「この Story は Windows だけ。Linux の更新係の起こし方は FR-131 の Story が持つ」）。推奨の「Linux 向けの配布物にも実行ファイルを入れる」は配布物の中身を変える（正典の 4 つの配布物の中身）ので、入れるかどうかも C に書く。
- 処置: fixed defaults.md —— 旧 Q10 を外して C10 にした（旧 Q11 を Q10 に繰り上げた）
