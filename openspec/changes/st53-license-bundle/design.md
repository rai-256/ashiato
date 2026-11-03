# ST53 design — 配布物に許諾が揃い、Google Play 開発者サービスとの組み合わせを許す

## Context

動機は `proposal.md` の Why。いまの状態（premises P1〜P13・review/deep.md の手順 5）:

- `LICENSE` は AGPL-3.0 の全文だけ。`README.md:45` と `CONTRIBUTING.md:5` は `AGPL-3.0-only` と書く。`AGPL-3.0-only` の表示を持つファイルは 506 あり、そのうち 3 つは移行（`migrations/202609211400_browser_history_record_id{,.down}.sql`・`migrations/202610181200_media_capture.sql`）。
  ほかに、gradle の wrapper の 2 つが `Apache-2.0` を持つ
- `tools/check-licenses.sh` は Rust と Node だけを見る。Android の節は echo するだけで、何も検査していない。許可の一覧には `AGPL-3.0-only` が入っていて、他人の AGPL も通ってしまう。
  中間のファイルを `/tmp/meta.json` の固定名に書く
- `tools/release/build.sh` の tar / zip には、実行ファイルが 1 つ入るだけ。APK には許諾の assets が無い。`check-artifacts.sh`・`release.yml`・`nightly.yml` に許諾の語は 0 件
- 移行の指紋（`crates/server/src/migration_ledger.rs:331` の `fingerprint`）は、ファイルの中身の sha256 を DB に記録する。中身が変わると、起動のたびに警告が出る（D7 of ST48。止めはしない）
- 配布元 `rai-256/ashiato` は公開リポジトリ。Environment もルールセットも 0 個（P7）。エージェントの資格情報は管理者（P12）
- `tools/verify-prep.sh:58-61` は `adb devices` の最初の端末へ `adb install -r` で入れる
- Rust の標準ライブラリの許諾の文は、toolchain の `$(rustc --print sysroot)/share/doc/rust/` にある（`COPYRIGHT-library.html` と `licenses/{MIT,Apache-2.0,…}.txt`。手元で確かめた）

## Goals / Non-Goals

**Goals**: 最初の本物の印を打つ前に、次の 4 つを機械で止まる形にする。
- 許可の文
- ソースの表示
- 依存の許諾の検査（携帯を含む）
- 配布物の許諾の同梱
あわせて、正式な鍵を読む経路を、本人の承認なしには開かない形にする（ST52 IMP-01）。

**Non-Goals**:
- 最初の本物の印・正式な鍵を置くこと・本人のサーバの切り替え（Q4。ST53 が main に入った後に本人が `docs/release.md` の手順で行う）
- 携帯の許諾を読む画面（Q6）・サーバの画面にソースの場所を出すこと（Q9）。どちらも要件の未決として record の段が残す
- F-Droid 向けに `play-services-location` を置き換えること（§5。F-Droid に載せたくなったとき）
- 衛星向けの契約と SDK の Apache-2.0（ST42）

## Decisions

### D1. 許可の文の正本は英文の 1 ファイルで、`LICENSE` の先頭にそのまま写す

> Q1・Q2 の推奨（答え待ち）。A の問いで、本人の答えで変わる。

正本は `LICENSES/AdditionRef-ashiato-play-services.txt`。中身は `deep-questions.json` の Q1 の英文の案で、`Listed libraries:` の節に Q2 の 4 つの座標（`com.google.android.gms:<名前>`）を 1 行ずつ書く。
`LICENSE` は「正本の全行 → 空行 2 つ → AGPL-3.0 の全文」の並びにする。
許諾の文の検査（`tools/check-spdx.sh license`。表示の検査 `headers` と同じ台本。D5）は、次の 2 つを見る。
- `LICENSE` の先頭の N 行（N は正本の行数）が、正本と 1 字も違わないこと
- その後ろの AGPL-3.0 の全文の sha256 が、今の `LICENSE` の sha256（検査の中に定数で持つ）と同じこと
README のライセンスの節には、参考訳（「正本は英語」と明記）と正本へのリンクを置く。
- 選ばなかった案: 許可を別のファイルだけに置き、`LICENSE` は AGPL の全文のまま。`LICENSE` だけを写して配る人が、許可を落としやすい
- 反転条件: GitHub の許諾の判定（licensee）が `LICENSE` を「Other」と出すことが、利用者の判断を誤らせると分かったら、AGPL の全文を `LICENSE` に戻して許可を `LICENSE` の末尾に移す（Risks）

### D2. 携帯の依存の検査は、名指しを正本の `Listed libraries:` から読む

> Q2 の推奨（答え待ち）。

検査は、名指しの一覧の写しを持たない。正本のファイルの `Listed libraries:` から空行までの行を、`group:name` として読む。
名指しを足すのは正本の 1 か所だけで、そこを直せば、`LICENSE`（D1 の一致の検査で一緒に直る）と依存の検査が同じものを指す。
- 反転条件: Q2 の答えが「群の全部」になったら、名指しの行を `com.google.android.gms:*` の形にし、検査は `*` を群の一致として読む

### D3. 鍵を置く前に、承認者とエージェントの資格情報の切り替えを済ませる

> Q3 の推奨（答え待ち）。ST52 IMP-01 を引き受ける。A の問い。

`docs/release.md` の §3・§4 の順を入れ替え、「§3 エージェントの資格情報 → §4 ルールセットと Environment の承認者 → §5 鍵と Secrets」にする。
エージェントの資格情報は、権限を細かく絞った token（fine-grained）にする。持たせるのは、ハーネスが使う権限だけにする。
- Contents の読み書き（枝と印の push）
- Pull requests と Issues の読み書き
- Workflows の書き（workflow のファイルを push する）
- Actions の読み
Administration・Environments・Deployments・Secrets は持たせない。
ハーネスは `gh pr merge --merge`（`scripts/auto_merge.py:87`）と PR 本文の PATCH（`scripts/publish.sh:68`）を使う。どちらも管理の権限は要らない。
守りの本体は、GitHub が job を始める前に効く必須の承認者。workflow の中の `check-guards.sh approvals`（D11）は、本人が規則を外してしまったときの止めで、印の commit の workflow を書き換えられれば効かない（P11）。手順の文書にそう書く。
- 選ばなかった案（Q3 の他の選択肢）: 承認者だけ（管理者の資格情報が規則を外せる）、手元で署名（FR-104 の「印を付けるだけで並ぶ」を満たさない）
- 承認するときに本人が見るもの（`docs/release.md`）: 承認の画面に出る印の commit が main の祖先で、PR を通って入ったこと。見ずに押すと、守りは無いのと同じ（Risks）

### D4. 移行の 3 ファイルは、許諾の表示を書き換えない

> 深掘り Q10（第 2 回）の本人の答え。経緯: 仮で進めたが別の席（`provisional-review.md` の D4）が否とし（「いつでも書き換えられる」は最初の印の後に成り立たない）、Q10 で本人が「書き換えない」を選んだ

移行は中身の sha256 を DB に記録している（`migration_ledger.rs` の `fingerprint`）。表示の 1 行を書き換えると、既にその版を当てたどの DB（本人のサーバ・開発と確認の DB）でも、起動のたびに「記録した指紋と中身が違う版がある」の警告が出続ける。
3 つは `AGPL-3.0-only` のまま、表示の検査の例外の一覧に名前で載せる。`LICENSE`（D1）は木の全体に掛かるので、許可はこの 3 つにも及ぶ。追加の許可が要るのは APK に入るコードで、移行は APK に入らない（サーバの実行ファイルにだけ入る。`crates/server/build.rs:73`）。
- 受け入れたこと（Q10）: 最初の印 `v*` を打つと、配った移行の凍結の検査（`tools/check-released-migrations.sh:52-54`）が前進側の 2 つ（`.sql`）の中身の差を落とすので、その後はこの 2 つの表示を直せない。配った版のソースとサーバの実行ファイル（`crates/server/build.rs:114` の `include_str!`）に、WITH の無い表示が残る。戻し手順の `.down.sql` は凍結の検査の外（`check-released-migrations.sh:5`）で、後からも書き換えられる（§5 ★ 2026-11-03）
- 選ばなかった案（Q10 の他の選択肢）: 最初の印の前に書き換えて指紋の警告を受け入れる・指紋の計算から SPDX の行を除く形に変えてから書き換える（ST48 の決めた形の改訂で範囲を越える）
- Q5 の答え（推奨のまま「506 ファイル」）に対し、この D4 が答えの上に乗る例外として残る（書き換えるのは 503。review/spec.md R9）

### D5. 表示の検査は先頭 25 行の `SPDX-License-Identifier:` を見る。取り込んだコードは、表示を持つものだけを止める

> 深掘り Q11（第 2 回）の本人の答え（先頭 25 行・表示の無いファイルは見ない・例外は Q10 に従う）。経緯: 仮で進めたが別の席（`provisional-review.md` の D5）が否とし（例外の一覧が D4 に乗るのに台帳に無かった）、Q11 で本人が決めた

`tools/check-spdx.sh` は `git ls-files -s` を読み、mode が `120000`（symlink）のものを飛ばす。残りのファイルの先頭 25 行から `SPDX-License-Identifier:` の値を取る。
窓を 25 行にするのは、gradle の wrapper の表示が 18 行目・16 行目にあるため（review/spec.md R3）。`crates/server/src/coverage/tests/mod.rs` は表示が 21 行目の `mod` の並びの後にあり、書き換えるときに先頭へ移す。
値が決めた式でなく、例外の一覧（`tools/check-spdx.sh` の中の、パスと許す値の組）にも無ければ落とす。例外の一覧は Q10（D4）の答えに乗り、gradle の wrapper の 2 つ（`Apache-2.0`）と移行の 3 つ（`AGPL-3.0-only`）の 5 つ。
表示の無いファイルは見ない（表示を全部に求めると、生成物・設定・文書の数百のファイルに見出しを足すことになる）。
§5 の「取り込んだコード」は、依存の検査では見えない。他所の表示を持ったまま写したファイルは、この検査が止める。表示を持たないものは止められない（Risks）。
- 選ばなかった案（Q11 の他の選択肢）: 表示の無いファイルも一覧にして落とす（数百のファイルに見出しを足す）
- 第三者のコードを写して取り込む必要が出たら、例外の一覧に足すか、`LICENSES/` に許諾を置く形を決める

### D6（仮）. 自分の部品は、名前と決めた式の組で通す。許可の一覧から `AGPL-3.0-only` を外す

> review/deep.md R8 の C（Q の外で決めた）。

Rust の節は、`cargo metadata` の `workspace_members` の部品を、許諾が `AGPL-3.0-only WITH AdditionRef-ashiato-play-services` のときだけ通す。
Node の節は、根（`web/package.json` の `name`）を同じ条件で通す。ほかの部品の判定は、許可の一覧で行う。
`AGPL-3.0-only` は、許可の一覧から外す。`AdditionRef-` の式も、許可の一覧には入れない。他人の部品は、この 2 つでは通らない。
ほかの許可の一覧は 1 つも変えない（壊してはいけないもの）。
- 失わない理由: 今の依存で落ちるものは無い（P3: Rust 357 部品・Node 306 部品に他人の GPL / AGPL は 0）。他人の AGPL が要る日には、一覧に戻せる
- 反転条件: 他人の AGPL-3.0 の部品を使うと決めたら、その部品に許可が及ばないことを README に書き、一覧に `AGPL-3.0-only` を戻す

### D7. 携帯の依存の許諾は、gradle の task が取り寄せた POM から読み、名前を対応表で SPDX に直す。名前が 2 つ以上なら全部を見る（AND）

> 深掘り Q12（第 2 回）の本人の答え（AND で読む）。経緯: 仮で「どれか 1 つが許可の一覧にあれば通す（OR）」としていたが、別の席（`provisional-review.md` の D7）が否とした（OR は通す側に倒れ、両方が掛かる部品を通すと名指しの外のプロプライエタリが APK に入る。「POM は gradle の cache に入っている」も誤り）

`collector-android/app/build.gradle.kts` に task `ashiatoDependencyPoms` を足す。
task は `releaseRuntimeClasspath` の解決済みの部品ごとに、detached configuration で `<group>:<name>:<version>@pom` を解決し、`<licenses>` が空なら `<parent>` を辿る（深さの上限 5）。
結果は `group:name:version<TAB>許諾の名前<TAB>URL` の行にして、引数で渡された置き場に書く。名前が 2 つ以上あれば 1 行ずつ書く。
POM は依存の解決のときには cache に入らない（`androidx.core/*`・`com.google.android.gms/*` は `.aar` だけ。provisional-review.md の D7）。そこで段を 2 つに分ける。
- 取り寄せる段（online）: task が `@pom` の detached configuration を解決し、gradle が `.pom` を取り寄せて cache に置く。CI の `package-linux`（新しい runner で cache が空）と、手元の最初の 1 回はこの段を通る。`tools/check-licenses.sh android` は既定でこの段から走らせる
- 読む段（`--offline`）: 取り寄せた後は、同じ task を `--offline` で走らせて同じ出力を得る。cache に無い `.pom` があれば、task は解決に失敗して落ちる（取れない部品を飛ばさない。組み立ても検査も落ちる側に倒れる）
`tools/check-licenses.sh` の Android の節は、この出力を読む。許諾の名前は `tools/licenses/pom-license-names.tsv`（`POM の名前<TAB>SPDX の名前`）で直す。
- 表に無い名前は落とす
- `Android Software Development Kit License` は SPDX の名前に直さない。表で `PROPRIETARY` に対応させ、名指し（D2）でだけ通す
- 1 つの部品が名前を 2 つ以上持てば、全部の名前が許可の一覧にあるか名指し（D2）に当たるときだけ通す（`AND` と読む）。1 つでも表に無い・許可の外・名指しの外なら落とす。本当に選べる 2 つ（デュアルライセンス）の部品が来たら、対応表か例外でどちらを選んだかを書いて通す（今の依存では 2 つ以上を持つ部品は 0。provisional-review.md の D7）
Google の `oss-licenses-plugin` は使わない。実行時に `play-services-oss-licenses` を引き、それは名指しの外の gms 群の部品なので、自分の検査で落ちる（review/deep.md の C 表）。
- 選ばなかった案（Q12 の他の選択肢）: OR で読む（名指しの外のプロプライエタリを通しうる）・2 つ以上並んだら全部を落として部品ごとに人が決める

### D8. 依存の一覧は配布物ごとに組み立ての時に作る。全文は部品が持つ許諾のファイルを優先し、無ければ SPDX の全文を使う

> 深掘り Q13（第 2 回）の本人の答え（携帯の部品は検査と同じ出力から。ほかはこの案のまま）。経緯: 仮で進めたが別の席（`provisional-review.md` の D8）が否とし（携帯の入力が D7 に乗るのに台帳に無かった）、Q13 で本人が決めた

新しい `tools/release/notices.sh <server|collector-windows|apk> <出力のファイル> [--target <triple>]` が `THIRD-PARTY-NOTICES.txt` を組む。部品の集め方は次のとおり。
- Rust: `cargo tree -p <crate> -e normal --target <triple> --prefix none --format '{p}'` の部品を集める（build と dev の依存は除く）。各部品の `manifest_path` の置き場から `LICENSE*`・`COPYING*`・`NOTICE*` を写す
- Rust の標準ライブラリ: 名前 `rust-std`、版は `rustc --version`、許諾は `MIT OR Apache-2.0`。全文は sysroot の `share/doc/rust/licenses/MIT.txt` と `Apache-2.0.txt`、著作権の表示は `COPYRIGHT-library.html` から取る。無ければ組み立てを落とす
- Node（サーバだけ）: `web/package-lock.json` の `packages` のうち、`dev: true` でないもの（根を除く）。全文は `web/node_modules/<名前>/` の `LICENSE*`。`build.sh` が `npm ci` した後に呼ぶ
- 携帯: D7 の task の出力の部品（検査と一覧が同じ部品の集合を見る。Q13）。全文は、部品の jar / aar の `META-INF/LICENSE*`・`NOTICE*`、無ければ `tools/licenses/texts/<SPDX>.txt`。
  名指しの gms の 4 つは、aar の根の `third_party_licenses.txt` を `third_party_licenses.json` の位置で切り出し、`(gms の中の第三者)` の見出しを付けて足す
部品の許諾のファイルが無いときは、`tools/licenses/texts/<SPDX>.txt` の全文に、部品の `authors`（Rust）か `author`（Node）を著作権の表示として添える。どちらも無ければ組み立てを落とす。
各項の形（`== <名前> <版>` → `License: <式>` → 全文）と、組めない部品で落ちることは spec にある。項の並びは名前の順にする（組み立てごとに同じ中身になる）。
- 多めに入れる側に倒す（`cargo tree` は target の部品を全部数える。実際に結び付かない部品が入っても、欠けるよりよい）。欠けた版は直せない（R10）
- 選ばなかった案（Q13 の他の選択肢）: 携帯の部品を検査と別の出所（`gradlew :app:dependencies` の木）から作る（検査と一覧で部品の集合がずれうる）
- 反転条件: 一覧が大きくなり、配布物の大きさで困ったら、Apache-2.0 の全文を 1 回だけ書き、各項は名前の参照にする

### D9. PR の組み立ては head の commit を checkout して組む。`SOURCE.txt` の印は `ASHIATO_SOURCE_REF` で渡し、無ければ組み立ての commit を書く

> 深掘り Q14（第 2 回）と Q16（第 3 回）の本人の答え（PR の組み立てでは head の commit を書く。PR の組み立てを head から組む —— checkout に head の commit を渡す）。経緯: 仮で「無ければ組み立ての commit（PR では合成の merge commit）」としていたが、別の席（`provisional-review.md` の D9）が否とした（合成の merge commit を PR を閉じた後に取れるかは確かめておらず、その前に PR の成果物が出る）。Q14 で「head の commit を書き、組み立ての印との比べはその場合だけ外す」と決めた後、別の席（review/spec.md R17）が、合わせた木から組むと `SOURCE.txt` の head と組んだソースが main の差だけ食い違うことを見つけ、Q16 で組む木を head にした。比べを外す分岐は要らなくなり、比べはすべての組み立てで残す（D17 は外した）

`ci.yml` の `package-linux`・`package-windows` は、checkout に `ref: ${{ github.event.pull_request.head.sha || github.sha }}` を渡す。PR では PR の head の commit、main への push では push した main の commit を checkout する（GitHub が作る、head とその時の main を合わせた merge の commit からは組まない。FR-127 ★ 2026-11-03（第 3 回））。
組んだソース・配布物の組み立ての印（ST52 の `artifact_stamp`）・`SOURCE.txt` が同じ commit を指す。commit を別に渡す環境変数は持たない（checkout した木の `git rev-parse HEAD` が head の commit そのもので、2 つの出所の一致を別に守らずに済む。Q16 の帰結）。
`release.yml` の組み立ての job は、`ASHIATO_SOURCE_REF: ${{ github.ref_name }}` を `build.sh` に渡す。
`build.sh` は、その値が `v<ASHIATO_BUILD_VERSION>` と同じときだけ印として書く。違えば組み立てを落とす。
`ASHIATO_SOURCE_REF` が無い組み立て（PR・main への push・開発版の `nightly.yml`）は、組み立ての印と同じ出所の commit —— `ASHIATO_BUILD_COMMIT`（空でなければ。ST52 の印の上書き。配る組み立ての job は渡さない）、無ければ `git rev-parse HEAD` —— の 40 桁を書く。40 桁の 16 進でなければ（`unknown` を含む）組み立てを落とす。PR では head の commit、main への push と開発版では main の commit になる（FR-127 ★ 2026-11-03）。
`SOURCE.txt` の中身は次の 3 行。
- `Repository: https://github.com/rai-256/ashiato`（`ASHIATO_SOURCE_REPO` で上書きできる。既定は配布元）
- `Ref: v<版>` か `Commit: <40 桁>`
- 対応するソースの取り方（`git clone` と `git checkout <印か commit>`）
`check-artifacts.sh` の `SOURCE.txt` の決まり:
- `--allow-debug-key` が無く `--channel release|beta`: `Ref: v<--expect-version>`
- `--channel nightly` と `--allow-debug-key`（PR・main への push の `package-artifacts` と `draft.sh`。`draft.sh:32`）: 配布物の組み立ての印の commit と同じ `Commit:`（40 桁の 16 進）。どの組み立ても組んだ木の commit を書くので、比べはすべての組み立てで残す（Q16）
既定のチャンネルは `release` のまま変えない（ST52 の呼び出しを壊さない。review/spec.md R1）。
組む木を head にすると、`package-linux` の job の中の検査 —— ST52 の重い自己試験（`check-artifacts.sh`・`sign-apk.sh`・`make-key.sh` の `--self-test`）と、この Story が足す `check-licenses.sh android`（D15）—— も head の木で走る（FR-127 ★ 2026-11-03（第 3 回）はそう狭めてある。整合の確認 K2）。main と合わせて初めて許可の外の依存が入る組み合わせは PR の CI では止まらず、merge gate が head を main に rebase した後の CI（`scripts/merge_gate.sh:7`・`:79`）と main への push の CI で止まる。`check-spdx.sh` は合わせた木で走る `chain` の job にもある（D15）。ほかの job（縦串・試験）は今どおり合わせた木で走る。
配布物を検査する `package-artifacts` は合わせた木のまま走らせ、期待する版番号と比べる `LICENSE` だけを配布物を組んだ commit から取る（D18。深掘り第 4 回 Q17）。
PR の head の commit は PR の枝の上にあり、閉じた後も `refs/pull/<n>/head` で残る見込みが高い。ただし確かめていない（premises P14・確かさ 2）。この repo の PR の枝は rebase して push し直すので、push し直す前の組み立ての head はどの ref からも辿れなくなるかもしれない（FR-127 ★ 2026-11-03・整合の確認 C1）。人間の確認待ち 11.5（確認バッチの中、ST53 が main に入る前）で、閉じた PR の head と、push し直す前の head の両方を `git fetch origin <commit>` で確かめる。
head を checkout した組み立ての印が `.dirty` を持たないこと・fork の PR でも head を取れることは premises P17・P18（11.1 で見る）。
- 選ばなかった案（Q14 の他の選択肢）: 合成の merge commit のまま確かめを待つ・merge の前に確かめてから決める
- 選ばなかった案（Q16 の他の選択肢）: 合わせた木から組むまま `SOURCE.txt` に head を書き、`check-artifacts.sh --source-commit <40 桁>` で渡した head と比べる（`SOURCE.txt` の head と組んだソースが main の差だけ食い違う）
- 反転条件: 配布元の名前が変わったら、`ASHIATO_SOURCE_REPO` の既定を変える。11.5 で閉じた PR の head か、push し直す前の head を取れないと分かったら、上流へ返す（A。取れない写しのソースの場所は直せない）

### D10（仮）. 許諾なしで置かれた PR の成果物は、ST53 が main に入った後に消す

公開リポジトリの PR の成果物（`package-linux`・`release-package`）は、今は許諾なしで 14 件ある（review/deep.md R5。90 日で消える）。
`docs/release.md` に、merge の後に本人が `gh api` で ST53 より前の commit の成果物を消す手順を置く。retention は変えない（ST53 の後の成果物は許諾を持つ）。
- 失わない理由: 消すのは本人の手で、消さなくても 90 日で消える。retention を縮めるのは後からできる
- 反転条件: 第三者が成果物を落とせないと分かったら、手順を消す

### D11（仮）. `approvals` は `release` の保護の規則を読み、本人の login を必須の承認者に、管理者の飛ばしを禁じる設定を求める

`check-guards.sh approvals [--repo <owner/name>] [--reviewer <login>]` は、`gh api repos/<repo>/environments/release` を読み、次の 3 つを求める。
- `protection_rules` に `type: required_reviewers` の規則があり、その `reviewers` に `type: User`・`login: <reviewer>` がある
- `can_admins_bypass` が `false`
- `prevent_self_review` は見ない（本人が印を打って本人が承認するため。true を求めると運用が詰む）
`<reviewer>` の既定は `ASHIATO_RELEASE_REVIEWER`、無ければ配布元の持ち主（`<owner>`）。
`agent-credential [--repo]` は、`gh api repos/<repo> --jq .permissions` を読み、`admin` と `maintain` がどちらも `false` であることを求める。公開の承認の権限（Deployments）は、この API から読めない。確かめは、人間の確認待ちで使い捨ての Environment に対して行う（tasks の末尾）。
`release.yml` の `publish` は、`check-guards.sh rules` の後、`sign-apk.sh` の前に `check-guards.sh approvals` を走らせる。`check-guards.sh workflows` は、その並びを確かめる。
- 失わない理由: どの欄を見るかは台本の中だけで、後から足せる。GitHub の API が欄の名前を変えたら、確かめが落ちる側（公開しない側）に倒れる
- 反転条件: Actions の `GITHUB_TOKEN` で `environments/release` を読めないと分かったら（人間の確認待ち 12.3）、公開の job では `approvals` を走らせず、`docs/release.md` の印を打つ前の手順だけに置く

### D12. 最初の本物の公開は、ST53 が main に入った後に本人が行い、手順の最後に公開した版を確かめる

> Q4 の推奨（答え待ち）。A の問い。

ST53 の確認バッチでは、ST52 と同じ `tools/release/draft.sh` で PR の配布物から下書きを作り、許諾の 3 点（D8・D9）と守りを見る。
`docs/release.md` §6「最初の印」の最後に、次の 2 行を置く。印 `v<版>` を checkout した手元で走らせる（`check-artifacts.sh` は置かれた木の `LICENSE` と比べる。配布物の検査は組んだのと同じ commit の木で走らせる —— spec の配布物の Requirement。review/spec.md R30）。
- `gh release download v<版> -D "$dir"`
- `tools/release/check-artifacts.sh "$dir" --channel <beta|release> --expect-version <版>`
`docs/stories/ST52.md` の完了の判定（「本物の公開は ST53 の確認で見る」）は、record の段が `stories.json` を直して作り直す。spec の段は直さない。
- 反転条件: Q4 の答えが「確認バッチの中で印を打つ」なら、その版に許可も一覧も入らないことを確認バッチの手順に書き、D12 を消す

### D13. 携帯の許諾は APK の中に置くだけ。サーバの画面にもソースの場所を出さない

> Q6・Q9 の推奨（答え待ち）。

APK は `assets/licenses/` に 3 点を置く。読む画面は作らない。
`build.sh apk` が、組んだ 3 点を置いた置き場を `-Pashiato.licenseAssets=<置き場>` で gradle に渡す。`build.gradle.kts` は、その置き場を `release` の `assets` の source set に足す。渡されなければ何も足さない（debug と単体の試験は今のまま）。
Play ストアへ出す前に画面を足すこと（Q6）と、一般向けの配布の前にサーバの画面にソースの場所を出すこと（Q9）は、record の段が `docs/requirements.md` に未決として書く（受ける Story はまだ無い）。

### D14. 確認バッチの準備の端末の選び方は、別のファイルの関数にして自己試験を持たせる

> Q8 の推奨（答え待ち）。

新しい `tools/verify-prep-device.sh` に、関数 `install_on_emulators <apk>` を置く。関数は `adb devices` を読み、状態が `device` で、名前が `emulator-` で始まる端末にだけ `adb -s <名前> install -r` を呼ぶ。
入れた端末の名前を標準出力に 1 行ずつ出す。1 台も無ければ、「エミュレータが無いので入れていない（手で入れる: `adb -s <エミュレータ> install -r <apk>`）」の 1 行を出して rc=0 で戻る（review/spec.md R10）。
`tools/verify-prep.sh` は、これを `source` して呼び、出力をそのまま手順書の文（`android_note`）にする。`verify-prep.sh` 自身は `adb` も `install` の文字も持たない。
`tools/verify-prep-device.sh --self-test` は、偽の `adb`（`PATH` の先に置く台本）で spec の Scenario を走らせる。
- 選ばなかった案: change で別に直す（Q8 の 2 つ目の選択肢）

### D15. 検査の置き場は、道具のある job に分ける

`tools/check-licenses.sh` は、引数で節（`rust`・`node`・`android`）を選ぶ。引数が無ければ 3 つ全部を走らせる。
CI の置き場は次のとおり。
- `web` の job: `../tools/check-licenses.sh rust node`（node_modules があり、gradle が無い）
- `package-linux` の job: `tools/check-licenses.sh android`（JDK と Android SDK がある）
`package-linux` は `changes` の `code` が true のときだけ走る。依存も表示もコードの変更でしか変わらないので、文書だけの PR では飛ばしてよい。
`check-spdx.sh`（`license` と `headers`）は軽いので、`chain` の job（全 PR で走る）にも置く。`LICENSE` を文書だけの PR で書き換えても止まる。`check-licenses.sh` は、節を選んだときも、選んだ節の対象が無ければ落ちる。

### D16. 開発版・main への push・PR の組み立ては、FR-127 の「同じ印」ではなく 40 桁の commit でソースを示す

> 深掘り Q7（第 1 回）と Q15（第 2 回）の本人の答え（開発版は main の commit、PR は Q14 で選んだ commit）。review/spec.md R14。経緯: 仮で「組み立ての commit」としていたが、別の席（`provisional-review.md` の D16）が否とし（D9 に乗るのに台帳に無かった）、Q15 で本人が決めた

FR-127 は「対応するソースは、同じ印（タグ）のリポジトリで示す」と言う。開発版の印 `nightly` は毎日動き、main への push と PR の組み立てには印が無いので、字面どおりには示せない。
正式版・ベータ版は印 `v*`（動かない。ST52 の D2）で示す。開発版と main への push は組み立てた main の commit、PR の組み立ては PR の head の commit（D9）で示す。要件の字面との差は FR-127 ★ 2026-11-02・★ 2026-11-03 が補っている。
- commit は印より狭く、動かない
- 選ばなかった案（Q15 の他の選択肢）: 開発版にも動かない印（`nightly-YYYYMMDD` など）を打ち、その印で示す（印が毎日 1 つ増え、ST52 の印の決まりに型が 1 つ足される）

### D17. （外した）main への push の組み立ても、`--allow-debug-key` の検査では `SOURCE.txt` の commit を形だけ見る

> 2026-11-03 に外した。前提（引数だけでは PR と main への push を分けられない）は誤りで（review/spec.md R18。`ci.yml:34` が既に式で PR と push を分けている）、深掘り Q16 で PR の組み立てを head から組むことにしたので、比べを外す分岐ごと要らなくなった（D9）。台帳からは `provisional.py resolve` で外した。番号は再び使わない

### D18. 配布物を検査する `package-artifacts` は合わせた木のまま走らせ、期待値だけを配布物を組んだ commit から取る（深掘り第 4 回 Q17）

> 仮で「`package-artifacts` も組んだ job と同じ commit を checkout する」としていたが、別の席（`provisional-review.md` の D18）が否とした（要件 FR-127 ★ 2026-11-03（第 3 回）は head の木で走るものを「組み立ての job の中の自己試験と許諾・表示の検査」に限っており、`package-artifacts` は組み立ての job でない）。台帳から外し、深掘り第 4 回 Q17 で本人に返した。本人は「合わせた木のまま走らせ、期待値だけを head の commit から取る（要件は変えない）」を選んだ（`deep-answers-7.txt`）。本人の決定なので（仮）に戻さない

`ci.yml` の `package-artifacts` の作業の checkout は今のまま（`ref` を渡さない。PR では GitHub が作る、head とその時の main を合わせた木）。`check-artifacts.sh` もその木の台本を走らせる。
期待する版番号と、配布物の `LICENSE` と比べる `LICENSE` だけを、配布物を組んだ commit（`${{ github.event.pull_request.head.sha || github.sha }}`。PR では head の commit、main への push では push した commit）の中身から取る。
- 取り方: `package-artifacts` に 2 つ目の `actions/checkout` を足し、`ref: ${{ github.event.pull_request.head.sha || github.sha }}`・`path: built`・`persist-credentials: false` で、配布物を組んだ commit を作業の木の下の `built/` に置く。台本はその木から走らせない（データとしてだけ読む）
- 版番号: `tools/release/version.sh source built/Cargo.toml`（`version.sh:14` は既に Cargo.toml の道を受ける）
- `LICENSE`: `check-artifacts.sh` に `--expect-license <file>`（比べる `LICENSE` の道）を足し、`built/LICENSE` を渡す。渡さなければ今どおり台本の木の `LICENSE`（`$root/LICENSE`）と比べる。`draft.sh:32`・`release.yml`・自己試験の呼び出しは変えない（どれも組んだ木と台本の木が同じ）
- `SOURCE.txt` の比べ（組み立ての印の commit と同じ `Commit:`）は、期待値ではなく配布物の中どうしの比べなので、木に依らない（D9）
枝を切った後に main が出所の版か `LICENSE` を変えた PR でも、head から組んだ配布物を head の版番号・`LICENSE` と比べるので、CI は誤って赤くならない（整合の確認 K1）。merge gate が head を main に rebase した後の CI では、組む木と合わせた木が同じものを見る。
`draft.sh` は走らせた手元の木の版番号と `LICENSE` で比べる（`draft.sh:15`・`:32`。ST52 から同じ型）。`docs/release.md` の下書きの手順に、その PR の head を checkout した手元で走らせることを書く（10.2）。
- 失わない理由: 変わるのは検査が比べる相手の取り方だけで、配布物の中身も `SOURCE.txt` も変わらない。どちらの取り方でも、食い違えば落ちる側（公開しない側）に倒れる

## Risks / Trade-offs

- **npm と SPDX 2.x の道具が `AdditionRef-` を読めない**かもしれない（P5 は 3.0 の文書だけ）。`web/package.json` は `private: true` なので、npm は止まらないはず。tasks で `npm ci` と `npm run build` の rc=0 を確かめる
- **GitHub の許諾の判定が `LICENSE` を「Other」と出す**かもしれない（R3 の未確認）。利用者が許諾を読み違えないよう、README の最初の節で AGPL-3.0 と追加の許可を名指しする。反転条件は D1
- **承認者の守りは、本人が承認の画面を見ることに乗る**。見ずに押せば、main に無い commit の workflow が鍵を読める。`docs/release.md` に、承認の前に見ることを書く（D3）
- **fine-grained token の権限の名前と効き方は、公式文書の確かさ 2**（P12 後半）。人間の確認待ちで、使い捨ての Environment に対して承認を試み、断られることを 4 に上げる。断られなければ鍵を置かず、Q3 を人間に聞き直す
- **エージェントの token も本人の利用者として動く**。承認者の名前では本人とエージェントを分けられない。分けるのは権限（Deployments を持たない）だけ
- **表示の無い写したコードは止まらない**（D5）。§5 の「NFR-26 の検査が止める」は、依存と表示を持つファイルについてだけ成り立つ
- **`check-guards.sh` の配布元の既定は `rai-256/ashiato2`**（ST52）。Actions では `GITHUB_REPOSITORY` が入るので正しいが、手元では `--repo rai-256/ashiato` を付ける。`docs/release.md` のコマンドは `--repo` を付けて書く
- **PR の配布物は、その時の main と合わせた結果を映さない**（Q16 の帰結）。PR の組み立てと、その job の中の自己試験と許諾の検査は head の木で走る。main と合わせて初めて壊れる組み合わせは、merge gate の rebase の後の CI か main への push の CI で止まる（D9）
- **一覧が大きい**: 携帯の APK の一覧は gms の中の第三者（約 370 KB。R7）を含む。APK の大きさの増え方は数百 KB で、受け入れる

## Migration Plan

移行は無い。DB・API・記録の形は変えない。本人の手は `docs/release.md` の順（D3）で、ST53 が main に入った後に行う。

## Open Questions

- Q1〜Q4（A）の本人の答え。答えが推奨と違えば `spec --answers` が D1〜D3・D12 を書き直す
- 閉じた PR の head と、push し直す前の head を後から取れるか（premises P14。人間の確認待ち 11.5。取れなければ D9 の反転条件で上流へ返す）
