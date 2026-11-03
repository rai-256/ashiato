# ST53 — 配布物に許諾が揃い、Google Play 開発者サービスとの組み合わせを許す

## Why

ST52 で、印を付けると 4 つの配布物（サーバの Linux 版・Windows 版、PC の収集アプリ、携帯の APK）が並ぶ仕組みはできた。しかし配布物の中には実行ファイルが 1 つ入っているだけで、本体の許諾も依存の許諾も入っていない（review/deep.md の手順 5）。
さらに携帯の APK は、オープンソースでない Google のライブラリ（`play-services-location` ほか 3 つ。premises P1・P2）を組み込んでいる。AGPL-3.0 のままでは、これを組み合わせて配ってよいかの解釈が割れる（扉 35）。
本体を公開すれば、それが配布になる（§5 商用化の方針）。最初の本物の印を打つ前に、許可の文・許諾の一覧・携帯の依存の検査を揃える必要がある。
もう 1 つ、ST52 から重大な判断が申し送られている（IMP-01）。main に入っていない commit に印 `v*` を打つと、正式な鍵を読める経路がある。これも、本物の鍵を置く前に閉じなければならない（FR-127・NFR-26・扉 35）。

## What Changes

> 第 1 回の深掘りは**推奨の答えで先に**書いた（2026-10-20 の流れ）。Q1〜Q4 は A（失われるものがある）。本人の答えが推奨と違えば、`spec --answers` が書き直す。
> Q5〜Q9 は B（推奨のまま進めてよい）。

- **本体の許諾に追加の許可を付ける**（Q1・Q2 の推奨・扉 35）: GNU AGPL 第 7 条の追加の許可を、英文を正本にして足す。
  許可が受けるのは、名指しした Google のライブラリの 4 つ（`play-services-location`・`-base`・`-basement`・`-tasks`）だけ。どの版でもよいが、改変したものは含めず、そのソースは求めない。
  文は `LICENSES/AdditionRef-ashiato-play-services.txt` を正本にし、`LICENSE` の先頭にもそのまま写す。日本語の参考訳は README に置く。
  `LICENSE` の先頭と正本のファイルが 1 字でも違えば、検査が落ちる
- **ソースの許諾の表示を書き換える**（Q5 の推奨・FR-127 ★）: `AGPL-3.0-only` を宣言しているソースと、`Cargo.toml`・`web/package.json`（と lock の根）の表示を `AGPL-3.0-only WITH AdditionRef-ashiato-play-services` にする。
  API の契約の写し（`docs/openapi.json`）は作り直す。ただし**移行の 3 ファイルは書き換えない**（D4・Q10）。移行は中身の指紋を DB に記録しているので、書き換えるとどの DB でも指紋の警告が出続ける。最初の印から後は、前進側の 2 つの表示を直せないことを受け入れる（Q10）。
  そのため件数は、推奨の文の 506 ではなく 503 になる。新しい検査（`tools/check-spdx.sh`）は、先頭 25 行に決めた表示でない見出しを持つファイルを落とす。表示の無いファイルは見ない。例外は、gradle の wrapper の 2 つと移行の 3 つだけ（D5・Q11）
- **携帯の依存も許諾の検査に入れる**（NFR-26・Q2）: `tools/check-licenses.sh` に Android の節を作る。配る APK に入る依存（`releaseRuntimeClasspath`）の POM の許諾を読み、SPDX の名前に対応させる。
  1 つの部品に許諾が 2 つ以上並べば、全部が許可されているか名指しに当たるときだけ通す（AND。D7・Q12）。オープンソースの許諾でない部品は、追加の許可の正本に名指しした 4 つだけを通す。名指しは正本のファイルから読み、検査は自分で一覧を持たない。名指しの外の部品・許諾の読めない部品・対応表に無い許諾名の部品が入れば、CI が落ちる
- **自分の部品は名前で通し、他人の GPL / AGPL は止める**（review/deep.md R8 の C。D6（仮））: 許可の一覧から `AGPL-3.0-only` を外す。
  自分の crate と画面の根は、名前と決めた表示の組で通す。今の依存は、これで 1 つも落ちない（P3）
- **4 つの配布物に許諾の 3 点を入れる**（FR-127・Q6・Q7 の推奨）: `LICENSE`（追加の許可を含む）・`THIRD-PARTY-NOTICES.txt`・`SOURCE.txt` の 3 つ。
  依存の一覧には、部品ごとの名前・版・許諾・許諾の全文と著作権の表示を書く。Rust の標準ライブラリと、Google のライブラリが中に持つ第三者の一覧も入れる（R7）。
  携帯の APK では、この 3 つを APK の中（`assets/licenses/`）に置く。許諾を読む画面は作らない（Q6）。
  `SOURCE.txt` に書く対応するソースの場所は、正式版・ベータ版なら印 `v<版>`、それ以外は 40 桁の commit（Q7）。開発版と main への push は main の commit、PR の組み立て（と、それを載せる下書き）は PR の head の commit（Q14・Q15）。
  入れるのは組み立ての台本（`tools/release/build.sh`）なので、PR の成果物・下書き・開発版にも同じものが入る。`check-artifacts.sh` は、どれかが欠けた配布物を公開しない
- **正式な鍵を読む公開は本人の承認を経る**（Q3 の推奨・ST52 IMP-01）: Environment `release` に、本人を必須の承認者として付ける。さらに、エージェントの資格情報を、管理と公開の承認の権限を持たない token に替える。
  この 2 つを済ませてから鍵を置く手順にする（`docs/release.md` の順を入れ替える）。
  `tools/release/check-guards.sh` に副コマンドを 2 つ足す。`approvals` は、承認者が付いていて管理者が承認を飛ばせないことを確かめる。`agent-credential` は、今の資格情報が管理者でないことを確かめる。
  公開の job も、署名の前に `approvals` を通す
- **最初の本物の公開は、ST53 が main に入った後に本人が行う**（Q4 の推奨）: ST53 の確認バッチでは、PR の組み立ての配布物で作った下書きのリリースを使い、許諾と守りを見る。
  本物の印・正式な鍵・本人のサーバの切り替えは、`docs/release.md` の手順で本人が行う。手順の最後には、公開した版を取り寄せて `check-artifacts.sh` に通す行を置く。
  ST52 の完了の判定の文（`docs/stories/ST52.md`）は、record の段がこの答えに合わせて書き直す
- **確認バッチの準備は、エミュレータにだけ APK を入れる**（Q8 の推奨・ST52 code_verify R30・K1）: `tools/verify-prep.sh` は、`adb devices` のうち `emulator-` で始まる端末にだけ debug の APK を入れる。
  エミュレータが無ければ何も入れず、手で入れる場合のコマンドを示す
- **許諾なしで置かれた PR の成果物を、merge の後に消す**（R5 の C。D10（仮））: 手順を `docs/release.md` に置く
- サーバの画面にソースの場所を出すこと（Q9）と、携帯の許諾の画面（Q6）は作らない。どちらも、要件の未決として record の段が `docs/requirements.md` に残す

**BREAKING ではない**: API・記録の形・収集アプリの送り先は変えない。配布物の名前（ST52 の D17）も変えない。tar / zip / APK の中に入るファイルが増えるだけ。
許可は権利を足す方向の変更で、AGPL-3.0 の下で受けていた権利は減らない。CLA（`docs/CLA.md`）は書き換えない（壊してはいけないもの）。

## Capabilities

### New Capabilities

（無し）

### Modified Capabilities

- `distribution`: 追加の許可・ソースの許諾の表示・依存の許諾の検査（携帯を含む）・配布物の許諾の同梱・承認を経る公開（ADDED 5）
- `verify-environment`: 確認バッチの準備がエミュレータにだけ APK を入れる（ADDED 1）

**capability の照合**: INDEX.md の capability 表は、ST53 を `distribution`（「許諾の同梱」）に割り当てている。許可・表示・検査・同梱・公開の守りは、どれも配布物と公開の振る舞いなので `distribution` で合っている。
Q8（`tools/verify-prep.sh`）は確認バッチの作業ツリーの準備で、正典 `verify-environment` に既にある（env-init を呼ぶ Requirement。CH06）。`verify-environment` は change が立てた capability で、INDEX の表に行は無い。
後続の Story の振る舞いを先に書いたのではないので、前倒しには当たらない。INDEX は直さない。

**requires**: ST52 は archive 済み（2026-11-02。正典 `openspec/specs/distribution` にある）。この change は正典の Requirement を書き換えず、ADDED だけで足す。
既存の Scenario「鍵の置き場は、印と main のそれぞれにだけ開いている」（`check-guards.sh environments`）は変えない。承認者の確かめは別の副コマンドにする。

## Impact

- 許諾の文: `LICENSE`（先頭に追加の許可）、新しい `LICENSES/AdditionRef-ashiato-play-services.txt`（正本）、`README.md`（ライセンスの節と参考訳）、`CONTRIBUTING.md`（許可の下で受けることを書き足す。受け入れ条件は変えない）
- ソースの表示: `SPDX-License-Identifier: AGPL-3.0-only` を持つ 503 ファイル（移行の 3 つと gradle の wrapper の 2 つは除く）、`Cargo.toml` の workspace の `license`、`web/package.json` と `web/package-lock.json` の根の `license`、`docs/openapi.json`（作り直し）
- 検査: `tools/check-licenses.sh`（節の選択・Android の節・自分の部品・`$TMPDIR`）、新しい `tools/check-spdx.sh`、新しい `tools/licenses/pom-license-names.tsv`（POM の許諾名と SPDX の対応表）、
  `collector-android/app/build.gradle.kts`（依存の POM を書き出す task・許諾の assets の置き場）
- 配布物: `tools/release/build.sh`（3 点を入れる）、新しい `tools/release/notices.sh`（依存の一覧を組む）、`tools/release/check-artifacts.sh`（3 点の検査）、`tools/release/check-guards.sh`（`approvals`・`agent-credential`）
- CI: `.github/workflows/ci.yml`（`web` の job は `check-licenses.sh rust node`、`package-linux` に `check-licenses.sh android` と `check-spdx.sh`）、`.github/workflows/release.yml`（`publish` で `check-guards.sh approvals`、印を `ASHIATO_SOURCE_REF` で組み立てへ渡す）、`ci.yml` の `package-linux`・`package-windows`（checkout に `ref: ${{ github.event.pull_request.head.sha || github.sha }}` を渡し、PR では head の commit から組む。Q14・Q16）と `package-artifacts`（合わせた木のまま走らせ、同じ `ref` を `built/` に checkout して、期待する版番号と比べる `LICENSE` をそこから取る。`check-artifacts.sh --expect-license`。Q17・D18）
- 確認バッチの準備: `tools/verify-prep.sh`（端末の選び方）、新しい `tools/verify-prep-device.sh`（選び方と自己試験）
- 文書: `docs/release.md`（順の入れ替え・資格情報の切り替え・承認者・最初の公開の後の確かめ・PR の成果物の掃除・ST53 の確認バッチ）、`docs/testing.md`（検査の行）、`docs/production-prep.md`（依存の許諾の検査の行）、`CLAUDE.md`（構成の `tools` の行）
- record の段に渡すもの（spec の段は直さない。`deep.md` の「record の段へ渡すもの」）: `docs/stories/ST52.md` の完了の判定の書き直し（Q4）、`docs/stories/ST53.md` の完了の判定に承認者・`SOURCE.txt`・確認バッチの準備を足すこと（review/spec.md R13）、FR-127 に ★ で開発版・PR の組み立ては commit で示すと補うこと（Q7・D16・Q15）、`docs/requirements.md` の未決 2 つ（Q6 の携帯の許諾の画面・Q9 のサーバの画面のソースの場所）
- **走っている change との重なり**: `openspec/changes/` で archive 以外は、この change だけ。重なりは無い
- 本人の手（下流は配布元に書かない）: エージェントの資格情報の切り替え、Environment の承認者、ルールセット、鍵、最初の印、PR の成果物の削除
