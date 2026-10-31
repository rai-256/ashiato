# ST52 深掘りの問いの独立検査（deep-review）

対象: `openspec/changes/st52-release-channels/deep-questions.json`（Q1〜Q10）と `premises.md`（P1〜P11）。
手順 1〜8 は問いの一覧を見る前に行い、終えてから突き合わせた。指摘は R1 以降の通り。

## 手順ごとに確かめた範囲

- 手順 1（要件どうしの衝突）: FR-104/105/107・NFR-25・扉 34 と、関与する FR-106/117/124/127/128/130 を突き合わせた → R1（価値「自分の環境をその版で動かす」と FR-128＝ST56）・R7（FR-127＝ST53 より先に配る）
- 手順 2（扉の幅）: 扉 34 の「2 か所」と「1 回だけ入れ直す（未送信を送り切ってから）」→ R2・R5。開発版の識別子は扉が決めている → R6
- 手順 3（一方通行）: 鍵の置き場・鍵の作り方・versionCode・最初の印・印の取り下げ・記録に入る版の文字列 → R2・R3・R4・R9・R10・R12・R13
- 手順 4（日常）: 開発版の印を動かしたときの手元の `git fetch --tags`、本人の環境の切り替え → R11・R15
- 手順 5（既存コード）: `Config.kt`・`build.gradle.kts`・`crates/*/build.rs` の STAMP_SOURCES・`migration_ledger.rs`・2 本の検査台本・`ci.yml`・`lib.rs` の router を grep と実行で確かめた → R1・R9・R11・R12
- 手順 6（過去の取りこぼし）: 「書き直せない記録の形」→ R12。「申し送りの取りこぼし」→ `docs/handoff/ST52.md` の 3 項目（ST48 のタグの型・ST48/ST49 の版の出所・ST50 のタグの型）は Q6・Q10 が受けている。「保存場所が丸ごと消えた／入れ直した」→ Q1 が受けているが R1・R5 の誤りあり。「時計」→ 該当なし（版の付け方の日付は CI の時計で、端末の時計を入力にしない）
- 手順 7（族の網羅）: Q3 の兄弟（開発版の versionCode・PC の後戻り）→ R4・R14。Q2 の兄弟（開発版の鍵・鍵の作り方）→ R3・R13。Q10 の兄弟（緊急の修正の印・印の取り下げ）→ R9・R10。Q1 の兄弟（サーバの切り替え）→ R15
- 手順 8（前提の確かさ）: Q2 が確かさ 2 の P8 に載っている → R2。Q1 の選択肢の記述が確かさ 5 の P4 と合わない → R5。Q6 の推奨の仕組み（印から `-beta.N` を足す）は前提が無い → R12
- 構成の記号（S-01 など）: `grep -nE '\b(S|C|V|D)-0[0-9]\b' deep-questions.json` は 0 件。該当なし
- 後続 Story の範囲: Q4（ST54）・Q7（ST55）は境界を正しく引いている。取りこぼしは R1（ST56 の前に配布版へ切り替える帰結）・R7（ST53 の前に公開する帰結）

---

## R1. 配布版の APK は接続先も資格情報も持たず送信しない —— ST56 の前に本人の携帯を切り替える帰結と、資格情報入りの APK を誰がどう組むかの問いが無い
- 成果物: openspec/changes/st52-release-channels/deep-questions.json
- 根拠: collector-android/app/build.gradle.kts:57-61（`BASE_URL` / `API_TOKEN` / `USER_ID` を組み立ての時に gradle の property か local.properties から焼き込む）/ collector-android/app/src/main/kotlin/dev/ashiato/collector/Config.kt:18-29（本番では override は常に null。揃わなければ `isComplete=false`）/ collector-android/app/src/main/kotlin/dev/ashiato/collector/LocationService.kt:927-930（揃わなければ送らず、取得は続けて未送信に積む）/ collector-android/app/src/main/kotlin/dev/ashiato/collector/Retention.kt:12-16（未送信は 90 日 / 2 GB で捨てる）/ docs/requirements.md FR-128（設定させる口は ST56）/ docs/stories/ST56.md:39（「配布元の APK を入れて初めて起動すると…設定すると送信が始まる」は ST56 の完了の判定）/ Q1 の選択肢 1 の detail「権限と接続先の設定はやり直す」（deep-questions.json:19）
- kind: conflict
- loss: discarded
- 提案: Q1 の「接続先の設定はやり直す」は誤り（配布版には設定の口が無い）。A の問いを 1 つ足す:「本人の携帯を配布版（正式な鍵）へ移すのは ST56 の前か後か。前なら、資格情報を焼き込んだ APK を誰がどこで正式な鍵で組むか」。ST56 の前に配布版を入れると、送らないまま 90 日で捨てる。CI のリリースで `-Pashiato.apiToken` を渡して組むと、公開のリリースに資格情報が出る（exported）。どの選択肢でも、公開する APK の `BuildConfig.API_TOKEN` が空であることを CI で確かめる（機械の検査）を design に置く
- 処置: escalated —— Q1 を「いつ・どう配布版へ移すか」に組み直し、推奨を「ST56 の後に送り切ってから入れ直す」にした。資格情報を焼き込む 1 本は手元で組み公開しない選択肢にした。P15 を足し、公開する APK の資格情報が空であることの検査を C1 に置いた

## R2. Q2 は「フォークからの PR には Secrets が渡らない」（確かさ 2）に載っているが、鍵が漏れる経路は同じリポジトリの枝から出る PR で、そこには Secrets が渡る
- 成果物: openspec/changes/st52-release-channels/deep-questions.json
- 根拠: premises.md P8（確かさ 2。公式文書の引用だけ）/ deep-questions.json:42（Q2 の context が P8 を挙げる）/ `git remote -v` → origin は `rai-256/ashiato`（配布元と同じ。CLAUDE.md「Story は `feat/st<NN>-<slug>`…で PR にする（グラフが作る）」—— agent が書く枝は同じリポジトリから PR になる）/ .github/workflows/ci.yml:2-4（`pull_request:` は枝を絞らない。PR は自分の workflow を変えて走らせられる）/ premises.md の冒頭「loss のある問い（Q1〜Q5）は、確かさ 4 以上の前提（P1〜P6）にだけ載せている」と合わない
- kind: premise
- loss: exported
- 提案: Q2 の context から「フォークは渡らない」を守りの根拠として外し、「同じリポジトリの枝からの PR（agent が書く枝を含む）は Secrets を読める。workflow を変える PR なら鍵も読める」と書き直す。推奨の選択肢の detail「PR では鍵を使わない」は job の書き方の約束でしかなく、守りにならない。費用の安い守り（Secrets を GitHub の Environment に置き、配る対象を `v*` の印と main に限る。必要なら承認者を付ける）を選択肢か detail に足す。確かさ 4 で確かめる（Environment に置いた値が PR の run から読めないことを、使い捨ての値で走らせて見る）
- 処置: escalated —— Q2 の context から「フォークは渡らない」を守りの根拠に使うのをやめ、同じリポジトリの枝からの PR は読めることを書いた。推奨を「印と main にだけ開いた Environment」にし、P12（確かさ 2。tasks で使い捨ての値を置いて 4 に上げる）を足した。premises の冒頭を問いごとの依る前提に直した

## R3. 開発版の APK をどの鍵で署名するかの問いが無い —— Q2 の推奨では、正式な鍵を毎晩の予定の実行で使う
- 成果物: openspec/changes/st52-release-channels/deep-questions.json
- 根拠: deep-questions.json Q2 の選択肢 1 の detail「署名する job は印のときと開発版のときだけ」/ docs/requirements.md 扉 34（開発版の識別子 `dev.ashiato.collector.dev` は決めているが、開発版の鍵は書いていない）/ docs/requirements.md FR-106（開発版は正式版と別のアプリ。正式版へ上書きで移る経路を持たない）
- kind: irreversible
- loss: exported
- 提案: 族の兄弟として問いを足すか、Q2 に選択肢を足す:「開発版（`.dev`）は別の鍵で署名する」。開発版は識別子が別なので、別の鍵にしても失うものは無い。開発版の鍵が漏れても、入れ直しは開発版を入れた人だけで済む。正式な鍵を使う回数は、毎日から印を打つときだけに減る。開発版の鍵も、配った後は替えられない（開発版を入れた携帯は上書きできなくなる）
- 処置: escalated —— Q11（開発版の鍵。A・exported）を足した。推奨は別の鍵

## R4. 開発版の APK の versionCode の付け方が無い（Q3 の式はベータと正式だけ）
- 成果物: openspec/changes/st52-release-channels/deep-questions.json
- 根拠: deep-questions.json Q3 の context（式は「ベータは N、正式は 99」だけ。Q6 の推奨の開発版 `<版>-dev.<YYYYMMDD>` は式の外）/ premises.md P5（小さい versionCode は `INSTALL_FAILED_VERSION_DOWNGRADE`。確かさ 5）
- kind: irreversible
- loss: exported
- 提案: Q3 の兄弟を足すか、Q3 の context に開発版の付け方を書く。開発版は別の識別子なので、正式版と別の数の並びにしてよい。ただし一度大きな数で配ると下げられない（開発版を入れた携帯は消して入れ直し、P3 で手元の未送信が消える）。例: 開発版も Q3 の式に寄せる／日付から付ける。後から乗り換えると、日付の数（10 桁）から式の数（6〜7 桁）へは下げられない
- 処置: escalated —— Q3 の context に開発版の付け方（同じ式で末尾 0）を書いた（Q3 は A のまま人間に問う）。同じ数の上書きが入ることを P13（エミュレータ。確かさ 5）に足した

## R5. Q1 の選択肢「署名を引き継ぐ」の不可逆の記述が P4 の実測と合わない。推奨の選択肢は扉 34 が既に決めている
- 成果物: openspec/changes/st52-release-channels/deep-questions.json
- 根拠: premises.md P4（「その後は新しい鍵だけの APK も上書きで入る」。確かさ 5）/ deep-questions.json:26（選択肢 2 の irreversible「引き継いだ後に配る全部の APK に…debug 鍵の履歴が残る。外すには、もう一度全員に入れ直してもらう」）/ docs/requirements.md 扉 34「いまの C-01 は debug の鍵で入っているので、本人の端末も正式な鍵の版へ切り替えるときに 1 回だけ入れ直す（端末の未送信を送り切ってから）」
- kind: premise
- 提案: (1) 引き継ぎの印（lineage）が要るのは本人の携帯へ入れる 1 本だけで、その 1 本は手元で組んで公開しない。以後に配る APK は新しい鍵だけでよい（P4）。この形なら、外に出るものも全員の入れ直しも無い。選択肢 2 の irreversible を書き直す。(2)「debug 鍵の合言葉は `android` で誰でも知っている」は、鍵ファイル（PC ごとに作られる `~/.android/debug.keystore`）まで知られているように読める。「守りの弱い鍵」と書く。(3) 選択肢 1（送り切ってから入れ直す）は扉 34 の決定そのもの。この問いに残すのは、事実の確認（入っているか・その APK を組んだ PC の debug 鍵が残っているか）と、扉と違う手順（引き継ぎ）を採るかだけにする。R1 とあわせて直す（切り替えの時期が ST56 の前なら、どちらの選択肢でも送信が止まる）
- 処置: fixed deep-questions.json —— Q1 の引き継ぎの選択肢を「手元で組む 1 本だけに付ける・以後は正式な鍵だけ」に直し、「合言葉は誰でも知っている」を「守りの弱い debug 鍵」にした。扉 34 の決定を context に書き、問うのは時期と、扉と違う手順を採るかだけにした

## R6. Q4 は扉 34 が既に決めていて、失わない側が安い —— A でなく P（仮）にする
- 成果物: openspec/changes/st52-release-channels/deep-questions.json
- 根拠: docs/requirements.md 扉 34「開発版は `dev.ashiato.collector.dev`」（決定済）/ deep-questions.json Q4 の推奨の選択肢「irreversible: 無し（扉 34 で決めた値のまま）」、context「組み立ての設定 1 行（`applicationIdSuffix`）で済みます」/ CLAUDE.md「失わない側が高くつかず・範囲を越えず・要件を変えないなら、AI が仮で進む（P）」
- kind: irreversible
- 提案: 選択肢 3（同じ識別子）は扉 34 と食い違い、選べない。選択肢 1 は失うものが無く、設定 1 行で済み、要件を変えない。Q4 は `kind: open`（推奨あり）か台帳の P に落とす。`loss: exported` を外す。人間の時間を使わせない
- 処置: escalated —— Q4 を kind: open（仮でよい・推奨あり）にして問いに残し、loss を外し、扉 34 と食い違う選択肢「正式版と同じ識別子」を外した

## R7. 最初の公開の印（Q5）が、許諾の同梱（FR-127・扉 35＝ST53）より前に配布を起こすことが書かれていない
- 成果物: openspec/changes/st52-release-channels/deep-questions.json
- 根拠: docs/stories/ST53.md（`requires: [ST52]`。完了の判定「配布物に本体と依存の許諾の一覧が入っている」）/ docs/requirements.md FR-127「本体を公開する = 配布が発生する」、扉 35「付けて配った版は、その許可付きで出回り続ける（取り消せない）」/ premises.md P6（配布元は公開）/ deep-questions.json Q5 の why（凍結する 2 つだけを挙げる）
- kind: irreversible
- loss: exported
- 提案: Q5 の why か irreversible に、「ST53 の前に打つと、依存の許諾の一覧が入らず、Google Play 開発者サービスと組み合わせる追加の許可も付いていない APK が公開される（取り消せない）」と足す。打つ時期の選択肢に「ST53 の後に打つ」を足すか、ST53 の前はリリースを下書き（draft。公開しない）にする選択肢を足す。ST53 を前に置くかは Story の順の話なので、決めたら handoff に送る
- 処置: escalated —— Q5 の why に、ST53 の前に打つと許諾と追加の許可の無い APK が公開されること・開発版も公開が始まることを書いた。推奨を「ST52 は下書きのリリースで確かめ、最初の本物の印と開発版の公開は ST53 の後」にした。P16 を足した

## R8. Q5 の推奨「確認バッチで本人が打つ」は、どの commit に打つかが書かれていない。確認バッチの統合の枝に打つと「1 本の幹から付けた印」に反し、作り直された枝に凍結が残る
- 成果物: openspec/changes/st52-release-channels/deep-questions.json
- 根拠: docs/requirements.md FR-105「1 本の幹から付けた印（タグ）で分けて配る」/ .harness2/docs/flow-gates.md:200（確認バッチは ready な `feat/st*` を `verify/<tag>` に merge した枝。「通らなかった」なら作り直す —— flow-gates.md:290）/ tools/check-released-migrations.sh:18-37（`v[0-9]*` の印の版の移行を、どの枝の印でも凍結する）
- kind: irreversible
- loss: exported
- 提案: 選択肢 1 の detail を「確認バッチで手順を見たうえで、main に merge した後の main の commit に打つ」のように、打つ commit を明記する。統合の枝に打つと、まだ main に無い他の Story の移行と封筒の形がその版で凍結される。その枝が作り直されると、凍結だけが main の検査に残る（main の CI が落ち続ける）。印は main の上にしか打てない、を機械で止める（印を作る workflow が `git merge-base --is-ancestor <印> origin/main` を見る）
- 処置: escalated —— Q5 の context と選択肢に main の commit に打つことを書いた（Q5 は A）。印は main の祖先にだけ打てることを premises の C2 に置いた

## R9. 緊急の修正の印（FR-105 の一時的な枝）が、ST48・ST50 の 2 本の検査を壊す —— Q10 の族の兄弟が無い
- 成果物: openspec/changes/st52-release-channels/deep-questions.json
- 根拠: docs/requirements.md FR-105「正式版にだけ緊急の修正を当てるときに限り、その版の枝を一時的に切る」/ tools/check-released-migrations.sh:18-37（すべての `v[0-9]*` の印の ORDER を今の木と比べる。修正の枝には、後から main で打ったベータの印の移行が無いので「消した」で落ちる）/ tools/check-contract-version.sh:108-112（「最後の印」を `--sort=-creatordate` の先頭で取る。main の上で、後から打った修正の印 `v0.2.1` が最後になると、それより前に打ったベータ `v0.3.0-beta.1` で上げた送信契約の版との比べが抜ける。main で欄を足して版を上げ忘れても、`v0.2.1` の版より大きいので通る）/ .github/workflows/ci.yml:91-93（`v*` を全部取り寄せる）
- kind: technical
- loss: discarded
- 提案: Q10 の兄弟として問いを足すか、design で決める項目として deep に残す。「修正の枝の上では、その枝の祖先にある印だけを数える（`git tag --merged HEAD`）」「最後の印は日付でなく、HEAD の祖先にある印から選ぶ」。決めないまま修正の枝を切ると、送信契約の版の上げ忘れを見逃す。それは FR-130 が防ぐはずの「新しい欄だけが黙って失われる」になる。申し送りの「型を変えるなら一緒に変える」に当たるので、ST48・ST50 の台本と一緒に直す
- 処置: fixed D1 仮 —— 祖先の印だけを数える・最後の印を祖先から選ぶ（ST48・ST50 の台本を ST52 が直す）。provisional.jsonl に載せた（design の D1（仮）は spec の段で書く）

## R10. 印の取り下げ（消す・付け直す）の扱いが無い —— 消すと、配った版の移行の凍結が黙って外れる
- 成果物: openspec/changes/st52-release-channels/deep-questions.json
- 根拠: tools/check-released-migrations.sh:18-21（印が無ければ何も凍結しない。今ある印だけを見る）/ tools/check-contract-version.sh:108-110（同じ）/ docs/requirements.md FR-124（印 `v*` で出した契約の版は全部受け付け続ける）
- kind: technical
- loss: exported
- 提案: Q10 の族として、「出した `v*` の印は消さない・動かさない」を機械で止めることを design に置く（GitHub のルールセットで `v*` の削除と更新を拒む）。壊れた版を出したときは、印を消さずに次の版を出す。決めないままだと、壊れたベータを消した時点で、その版を入れた DB に当たっている移行が main で書き換えられるようになる。失わない側は設定 1 つなので P でよい
- 処置: fixed D2 仮 —— `v*` の削除と更新をルールセットで拒み、入っていることを CI が見る。provisional.jsonl に載せた

## R11. Q10 の context が、開発版の DB の食い違いを軽く書いている。印を動かすと、手元の `git fetch --tags` が rc=1 になる
- 成果物: openspec/changes/st52-release-channels/deep-questions.json
- 根拠: crates/server/src/migration_ledger.rs:295-298・363-367（記録に自分の知らない版があれば `Ahead` で、起動も移行も止まる。main で移行の名前を変える・消すと、開発版のサーバは起動しない）/ crates/server/src/migration_ledger.rs:418-440（中身の変わった版は警告だけで当て直さない。開発版の DB の形は黙って main とずれる）/ 実測（2026-11-01、git 2.53.0。使い捨ての repo で印 `nightly` を動かし `git push -f` → 別の clone で `git fetch --tags origin` は `! [rejected] nightly -> nightly (would clobber existing tag)` で rc=1。`git fetch` は rc=0 で、手元の `nightly` は古いまま）
- kind: premise
- 提案: context の「開発版の DB は、後から直された移行と食い違うことがあります」を、「名前を変えた・消した移行があると開発版のサーバは起動しなくなり、中身を変えた移行は警告だけで形がずれる」と書き直す。「開発版の DB は使い捨ての扱い」は、開発版の収集アプリが集めた記録を捨てうる決め方なので、ST54 へ申し送る（分ける仕組みは ST54）。日常への影響として、印を動かす方式では手元の clone の `git fetch --tags` が毎日 rc=1 になることを detail に足す（ハーネスの台本は今は `--tags` を使っていない。`grep -rn 'fetch.*--tags' tools scripts .harness2` は 0 件）
- 処置: fixed deep-questions.json —— Q10 の context を「名前を変えた・消すと起動しない、中身を変えると警告だけで形がずれる」に直し、`git fetch --tags` の rc=1 を書いた（P14）。開発版の DB の扱いは deferred ST54（premises の「他の Story へ回すもの」）

## R12. Q6 の推奨（`-beta.N` を印から足す）は、仕組みしだいで配った組み立ての印が `.dirty` になり、書き直せない記録に残る —— 仮でよい理由が成り立つ条件が書かれていない
- 成果物: openspec/changes/st52-release-channels/deep-questions.json
- 根拠: crates/server/build.rs:15-20 と crates/collector-windows/build.rs:6-11（`.dirty` を見る道に `Cargo.toml` と `Cargo.lock` が入る）/ crates/server/src/migration_ledger.rs:16（移行の台帳の `build_version` は `CARGO_PKG_VERSION`）/ web/build-stamp.ts:12（画面は `:/web` の変更で `.dirty`。`package.json` を書き換えると当たる）/ Cargo.lock:98-132（workspace の 3 crate の版を持つ）/ docs/openapi.json:10（`info.version` が checked-in）/ docs/requirements.md FR-122・FR-123（印は追記のみの記録 1 件ごとに残る）/ premises.md（印から版を足す仕組みの前提が無い）
- kind: technical
- loss: uncaptured
- 提案: Q6 は B のままでよいが、context か design に条件を足す:「配る組み立てでは、印の `-beta.N` を `Cargo.toml` などを書き換えずに版へ渡す（書き換えると `.dirty` が付く・`Cargo.lock` と `docs/openapi.json` がずれる）」「配る組み立ての印に `.dirty` / `.debug` が無いことを、リリースの workflow が確かめる」。書き換える方式で配ると、配った版が書いた記録の印がすべて `.dirty` になる。手元で直して組んだものと区別できなくなり、追記のみの記録は書き直せない。仕組みは 1 回走らせて前提（確かさ 4）にする
- 処置: escalated —— Q6 の context に「書き換えずに組み立ての時に版として渡す」条件を書いて問いに残し、配る印に `.dirty` / `.debug` が無いことの検査を premises の C8 に置いた

## R13. 正式な鍵の作り方（種類・長さ・有効期限）が問われていない —— 後で Play ストアへ移るときに受け付けられない鍵だと、作り直しになり全員が入れ直す
- 成果物: openspec/changes/st52-release-channels/deep-questions.json
- 根拠: docs/requirements.md §5「配布の方針」「C-01 は配布元に APK を置いて直に配る。いずれ Play ストアへ」/ docs/requirements.md 扉 34「配った後に…鍵を失うと、それまでに配った APK を上書き更新できない」/ deep-questions.json Q2 の context「鍵を作るのは本人です（作る台本は AI が用意します）」
- kind: technical
- loss: exported
- 提案: 問いにはしない（失わない側が安い）。design に P として置く。鍵を作る台本は、Play App Signing に今の鍵を持ち込める条件（鍵の種類・長さ・有効期限の下限）を満たす値で作る。条件は一次情報から premises に足す（今は確かさ 0）。作った鍵の値を `keytool -list -v` / `apksigner verify --print-certs` で台本が確かめる
- 処置: fixed D3 仮 —— 鍵の種類・長さ・有効期限を Play App Signing に持ち込める値にする。provisional.jsonl に載せた（条件の一次情報は tasks で premises に足す）

## R14. Q3 の兄弟: PC の収集アプリには後戻りを止めるもの（versionCode にあたるもの）が無い —— ベータ版から正式版へ戻すと、手元の未送信が古い形で送られる
- 成果物: openspec/changes/st52-release-channels/deep-questions.json
- 根拠: docs/requirements.md FR-105「ベータ版から正式版へは、正式版が追いつくのを待つ」/ crates/collector-windows/src/outbox.rs:1-13・58（未送信は JSONL。古い版は `serde_json::from_str` で読み、知らない欄を黙って落とす。`deny_unknown_fields` は無い —— `grep -n deny_unknown_fields crates/collector-windows/src/outbox.rs` は 0 件）/ premises.md P5（携帯は OS が止める）
- kind: defer
- loss: discarded
- 提案: ST52 で決めることではないので、ST58（入れる操作）へ申し送る。PC で新しい版から古い版へ入れ直すと、新しい版が積んだ未送信の新しい欄を古い版が落として送る。止めるか、知らせるかを決めるのは ST58。ST52 の deep に「PC には後戻りを止めるものが無い」と 1 行残す
- 処置: fixed D5 仮 —— ST52 では止めない（入れる操作と一緒に ST58 が決める）。provisional.jsonl に載せ、premises の「他の Story へ回すもの」に ST58 宛てを書いた

## R15. Q1 の兄弟: 本人のいまのサーバ（main から組んで動かしている）を配布版へ移すとき、D-01 が配布版より先に進んでいれば起動しない
- 成果物: openspec/changes/st52-release-channels/deep-questions.json
- 根拠: crates/server/src/migration_ledger.rs:295-298・363-367（記録に知らない版があれば `Ahead`）/ docs/requirements.md FR-105「チャンネルを移るとき DB は戻さない」/ deep-questions.json Q7 の context（「本人の環境を ST55 より前に配布版へ切り替えるなら」）
- kind: daily
- 提案: 失うものは無い（起動しないだけで、DB は書き換わらない）。問いにはせず、Q7 か design に「本人のサーバは、印を打った commit より後の main で組んだものを動かしていれば、次の印まで配布版へ移れない」と 1 行足す。移す手順（印を打った直後に、その印の版へ移す）を確認バッチの手順書に置く
- 処置: fixed D4 仮 —— 印の直後にその版へ移す手順を確認バッチに置く。Q7 の context にも書いた。provisional.jsonl に載せた

## R16. FR-107 の「1 日の終わり」と「その日の変更」の決め方が一覧にも design の候補にも無い（予定の実行は遅れる・落ちる。P7 は確かさ 2）
- 成果物: openspec/changes/st52-release-channels/deep-questions.json
- 根拠: docs/requirements.md FR-107 / premises.md P7（確かさ 2。予定の実行は main だけで、混むと遅れる・落ちる）
- kind: defer
- 提案: 失うものは無いので問いにしない。design に P で置く。「その日の変更」は日付の窓でなく、「`nightly` の印の commit と main の HEAD が違うか」で決める（予定の実行が 1 回落ちても、次の回で拾う）。「1 日の終わり」は日本時間で決め、cron は UTC で書く
- 処置: fixed premises.md C6 —— 日本時間 0:00 に起き、`nightly` の commit と main の HEAD が違うときだけ組む、を C6 に置いた
