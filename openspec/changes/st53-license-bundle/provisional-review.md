# ST53 仮で進めた決定の確かめ（deep-review・provisional）

対象: `provisional.jsonl` の D4・D5・D6・D7・D8・D9・D10・D11・D16（この時点では、どれにも節が無かった）。
1 件ずつ、「失わない」「費用」「follows」「rests_on」の 4 つを確かめた。問いを作った AI の「失わない」は、そのまま信じていない。

rests_on の漏れは、土台が**仮の D**（確認バッチで「変える」になりうる）のときだけ否とした。
本人が答えて合意した Q（Q1〜Q9。`deep.md` はどれも推奨のまま）に乗る漏れは、この change の中で土台が動かない。そこで根拠に書くだけにした。

## D4
- 判定: 否
- 根拠: 条件 1 で否。keeps は「いつでも 3 行を書き換えて例外から外せる」と言うが、成り立たない。
  - 理由: `tools/check-released-migrations.sh:52-54` は、印 `v*` の版にある移行について、前進側の中身の差（`git diff --quiet "$tag" HEAD -- "migrations/$name.sql"`）を落とす。
  - 当たるファイル: `migrations/202609211400_browser_history_record_id.sql:1` と `migrations/202610181200_media_capture.sql:1` の 2 つは、`migrations/ORDER:55`・`:61` にある前進側。表示はどちらも `AGPL-3.0-only`。`.down.sql` は凍結の外。
  - いつから: Q4 の答えでは、最初の印は ST53 が main に入った直後に本人が打つ。その時点から、この 2 つは表示を書き換えると凍結の検査に落ちる。
  - 失うもの: 配った印の版のソースに、WITH の無い表示が残る（exported）。この表示は `crates/server/build.rs:114` の `include_str!` で、サーバの実行ファイルにも入る。後から書き換えると、凍結した移行の書き直しになる。
  - D4 の失わない理由は「印 `v*` はまだ 0 個」（design.md:74）だけに乗っていて、最初の印の後を数えていない。
  - 失わない側（今のうちに 3 つを書き換える）の費用は、指紋の警告（`crates/server/src/migration_ledger.rs:518-530`）。既に当てたどの DB でも、起動のたびに出続ける。両側に失うものか費用があるので、A として本人に返す。

## D5
- 判定: 否
- 根拠: 条件 4 で否。加えて、台帳の chosen が design と食い違っている。
  - 条件 4: 例外の一覧（design.md:84「gradle の wrapper の 2 つと移行の 3 つ」）は、D4（仮）の決定そのものを含む。rests_on は `[]` で、D4 を挙げていない。
    D4 は上で否なので、D4 が「書き換える」に変われば、D5 の例外の一覧も変わる。表示の式（決めた式）は Q5 に乗るが、これは答え済み。
  - 食い違い: 台帳の chosen は「先頭 5 行」。design.md:80・specs/distribution/spec.md:55・tasks.md:58 は、review/spec.md R3 の処置の後で「先頭 25 行」になっている。
    `git grep` で確かめた。`collector-android/gradlew:18`・`gradlew.bat:16`・`crates/server/src/coverage/tests/mod.rs:21` は、5 行の窓には入らない。確認バッチの一覧（`provisional.py list`）は台帳の文を本人に見せるので、古い窓の決定を見せることになる。
  - 検査の範囲そのもの（表示の無いファイルを見ない）には、失うものは見つからなかった。repo の中の第三者のファイルは gradlew の 2 つだけ（review/deep.md 手順 5）。

## D6
- 判定: 確か
- 根拠:
  - 条件 1: 失うものは無い。
    - 自分の部品: `cargo metadata --format-version 1 --offline --all-features` の workspace_members は `ashiato-build-stamp`・`ashiato-collector-windows`・`ashiato-server` の 3 つ。どれも `license.workspace`（Cargo.toml:8）を継いでいる。
    - 他人の部品: GPL を含むのは `r-efi`（`MIT OR Apache-2.0 OR LGPL-2.1-or-later`。MIT で通る）だけ。
    - Node: `web/package-lock.json` の dev でない部品は react・react-dom・scheduler の 3 つで、GPL は 0。
    - 許可の一覧（`tools/check-licenses.sh:9`）から `AGPL-3.0-only` を外しても、今の依存で落ちるものは無い。
  - 条件 2: 費用は検査の台本の数行で、範囲の内。§5 の「他人の GPL / AGPL は止める」に合わせる向きで、要件の本文は変えない。
  - 条件 3: follows は無い。
  - 条件 4: 土台は Q5 の式（答え済み）。仮の D には乗らない。

## D7
- 判定: 否
- 根拠: 条件 1 で否。design.md:110 は「1 つの部品が名前を 2 つ以上持てば、どれか 1 つが許可の一覧にあれば通す（`OR` と読む）」とする。
  - これは通す側（公開する側）の読み方で、keeps の「落ちる側（公開しない側）に倒れる」と合わない。
  - POM の `<licenses>` に 2 つ並ぶとき、選べる（OR）のか両方が掛かる（AND）のかは、POM の形式からは決まらない。
  - 両方が掛かる部品（例: Apache-2.0 と `Android Software Development Kit License` を並べる部品）を OR で通すと、名指し（D2）の外のプロプライエタリが APK に入る。その版は配った後に戻らない（exported）。
  - 失わない側は「全部が許可の一覧にあるか、名指しに当たるときだけ通す（AND）」で、費用は同じ台本の 1 行。
  - 今の依存では起きない。`releaseRuntimeClasspath` の 55 部品（2026-11-02 に `gradlew --offline :app:dependencies` で得て、POM を Google の maven と Maven Central から取った）のうち、`<license>` が 1 つのものが 52、0 のもの（guava の 3 つ。親を辿る）が 3 で、2 つ以上のものは 0。
  - 前提の誤りも 1 つ見つかった（D7 の否の条件ではない）。design.md:107 は「POM は依存の解決のときに gradle の cache に入っている」と言うが、`~/.gradle/caches/modules-2/files-2.1/` の `androidx.core/*` と `com.google.android.gms/*` には `.aar` しか無く、`.pom` は無い（全体でも `.pom` は 77 個）。
    そのため、反転条件の「cache の `files-2.1/.../<name>-<version>.pom` を直に読む」には、読むものが無い。`@pom` の detached configuration が `--offline` で解決できるかは、走らせていない。解決できなければ組み立てが落ちる側に倒れるので、失うものは無い。

## D8
- 判定: 否
- 根拠: 条件 4 で否。携帯の入力は「D7 の task の出力の部品」（design.md:121）で、D7（仮）に乗る。rests_on は `[]` で、D7 を挙げていない。D7 は上で否なので、D7 が変われば D8 の携帯の部分も変わる。
  - 中身（多めに入れる側に倒す）には、失うものは見つからなかった。
    - 画面: Node の dev でない部品は react・react-dom・scheduler。`git ls-files web` に、第三者の資産（字形・画像）は無い。
    - Rust: build の依存は自分の `ashiato-build-stamp` だけ（crates/server/Cargo.toml:34-35・crates/collector-windows/Cargo.toml:30-31）。`build.rs` が実行ファイルに入れるのは、自分の移行と画面の組み立ての結果（crates/server/build.rs:114・:149）。
    - Windows: 正式版は windows-latest の native（.github/workflows/release.yml:54-58）。
  - 名指しの gms の 4 つは Q2（答え済み）。

## D9
- 判定: 否
- 根拠: 条件 1 で否。PR の組み立てで `SOURCE.txt` に書く commit は、合成の merge commit（`refs/pull/<n>/merge`）になる。
  - 理由: `.github/workflows/ci.yml:355` の checkout は `ref` を渡さない。
  - この commit を PR を閉じた後も取れるかは、premises P14 の確かさ 2 で、走らせていない。
  - PR の成果物は公開リポジトリで 90 日、誰でも落とせる側として扱っている（review/deep.md R5、retention の指定なし）。取れなかった場合、もう落とされた写しのソースの場所は直せない（exported）。成果物を消しても、落とされた写しは戻らない。
  - D9 は確かめを確認バッチ 11.1 に回している。それまでの間に ST53 の PR 自身と後の PR が、この形の成果物を出す。
  - 失わない側: PR の組み立てでは PR の head の commit（`github.event.pull_request.head.sha`）を書き、組み立ての印との比べはその場合だけ外す（design.md の D9 の反転条件そのもの）。費用は ci.yml の環境変数 1 つと比べの分岐で、範囲の内。head が閉じた後も `refs/pull/<n>/head` で取れるかも、確かさ 4 には上げていない。決め方は 2 つある。
    - 11.1 を merge の前に走らせる
    - head の側で始める
  - 正式版・ベータ版（印 `v*`）と開発版（main の commit）の部分には、失うものは見つからなかった。`nightly.yml:33-51` は `ASHIATO_SOURCE_REF` を渡さないので commit になる。土台の Q7 は答え済み。

## D10
- 判定: 確か
- 根拠:
  - 条件 1: 失うものは見つからなかった。
    - 消すのは、本人が ST53 より前の commit の成果物を消すことだけ。
    - それを使うのは `tools/release/draft.sh:31`（`gh run download … --name release-package`）だけで、CI を走らせ直せば作り直せる。ST53 の後は `check-artifacts.sh` が許諾の 3 点を求めるので、古い成果物はどのみち下書きに使えない。
    - retention は変えない（`.github/workflows/*.yml` で retention を指定するのは playwright-report の 7 日だけ）。
    - 既に落とされた写しは、どちらの側を選んでも戻らない。この決定で増える損失は無い。
  - 条件 2: 費用は手順の数行。
  - 条件 3: follows は無い。
  - 条件 4: 土台は Q4（main に入った後。答え済み）。

## D11
- 判定: 確か
- 根拠:
  - 条件 1: `approvals` は、公開の job（`environment: release`、release.yml:66）の中で走る後ろの止めで、どの欄を見るかは台本の中だけで変えられる。欄が無ければ落ちる（公開しない）側。
  - 確かめた: 配布元の持ち主は `gh api users/rai-256` → `type: User`。今の token の `gh api repos/rai-256/ashiato --jq .permissions` は `admin:true, maintain:true`。
  - 確かめていない（D11 の否の条件ではない）: 持ち主の個人のリポジトリなので、`agent-credential` が読む `.permissions` は、token の権限でなく利用者の役割（持ち主 = admin）を返すかもしれない。権限を絞った token でこの欄がどう出るかは、走らせていない（P12 後半の確かさ 2）。
    その場合、`agent-credential` は常に落ちる（公開しない側）。失うものは無いが、確かめ方を別の形（管理の権限が要る API を呼んで 403 を確かめる、など）に替える手間が出る。人間の確認待ち（tasks の末尾）で確かめる。
  - `approvals` は「本人の login が reviewers にある」だけを見て、ほかの承認者がいないことは求めない。ほかの承認者を足せるのは管理の権限を持つ者だけで、D3 でエージェントの資格情報から外す。足すのは後からできる。
  - 条件 2: 費用は副コマンド 2 つで、範囲の内（ST52 IMP-01 の引き受け。Story の完了の判定に足してある）。
  - 条件 3: follows は無い。
  - 条件 4: rests_on の Q3 は答え済み。D3 は仮でない。

## D16
- 判定: 否
- 根拠: 条件 4 で否。cost は「なし（D9 の書き方のまま）」で、design.md:204 も「それ以外は組み立ての印の 40 桁の commit で示す（D9）」と書く。D16 は D9（仮）に乗っているのに、rests_on は `["Q7"]` だけで、D9 を挙げていない。
  - D9 は上で否。PR の組み立てを head の commit に替えれば、D16 の「組み立ての commit で示す」の中身も変わる。
  - Q7（答え済み）との向きは合っている（deep.md「Q7 は推奨のまま。D16（仮）は答えと一致した」）。
