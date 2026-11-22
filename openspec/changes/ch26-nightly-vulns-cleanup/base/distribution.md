# distribution の書いた時点の正典（spec_merge.py snapshot。archive の 3-way の base。手で直さない）

### Requirement: PR の CI は、組み立ての入力が同じ Windows の配布物を使い回し、インストールの試験は全部走らせる

WHEN コードを触った PR が CI を走らせ、組み立ての鍵が同じ前の組み立ての Windows の配布物（Windows の配布物の job が上げる配布物の全部。今は配る形と試験の形の 2 つ）が CI のキャッシュに残っている THE SYSTEM SHALL Windows の配布物をビルドせずにそれを使い、インストールと更新の試験を走らせる。
THE SYSTEM SHALL Windows の配布物の job が上げる配布物の全部を、保存し、引く（一部だけを使い回さない）。
THE SYSTEM SHALL 組み立ての鍵を、次に名指しするものだけを除いた、追跡している全部のファイルの中身と、走っている CI の定義と、組み立てに使う道具の版（Rust・Node・runner の image）から作る。除くのは `tools/release/install-check.sh`・`tools/release/update-check-install.sh`・`tools/release/conpty-run.ps1`・`docs/` と `openspec/` の下・名前が `.md` で終わるファイルだけ。名指ししていないファイルが 1 つでも変われば、鍵は変わる。
THE SYSTEM SHALL 前の組み立ては鍵が完全に一致したときだけ使い、鍵の一部が一致するだけの組み立ては使わない。
WHEN Windows の配布物をビルドする THE SYSTEM SHALL Rust の依存のビルドを、Rust の版が同じ前のビルドから戻して使う。
WHEN 鍵を出せない THE SYSTEM SHALL 使い回さずにビルドする。
WHEN 幹（`main`）への push が CI を走らせる THE SYSTEM SHALL 使い回さずにビルドする。
THE SYSTEM SHALL 使い回した run でも、インストールと更新の試験の Scenario を全部走らせ、1 つでも走らなければ CI を落とす。

導出元: CH17（受け皿の依頼 #308。本人の決定: 試験の台本だけが変わった push では、前に作った Windows の配布物を使い回してインストールの試験だけを走らせる。範囲は CI の Windows の配布物とインストールの試験の job だけ）。
深掘り第 1 回 Q1・Q2（本人の答え）で、Requirement「コードを触った PR ごとに、配る形の 3 部品を公開せずに組む」と「配布物は、本体の許諾・依存の許諾の一覧・対応するソースの場所を持つ」の本文を、使い回しを認め、使い回した配布物は組んだ commit を指すように改めた（下の MODIFIED。FR-127 ★ 2026-11-17）。

#### Scenario: 試験の台本と文書だけが違う 2 つの木は、同じ組み立ての鍵になる
- **GIVEN** `tools/release/install-check.sh`・`tools/release/update-check-install.sh`・`tools/release/conpty-run.ps1`・`docs/` の文書だけが違う 2 つの commit と、同じ道具の版
- **WHEN** それぞれの commit で組み立ての鍵を出す
- **THEN** 2 つの鍵は同じ

#### Scenario: 組み立てに入るファイルが 1 つでも違えば、鍵が変わる
- **GIVEN** `crates/server/src/lib.rs`・`migrations/ORDER`・`web/src/App.tsx`・`tools/release/build.sh`・`tools/release/pgsql.lock`・`tools/licenses/` の 1 ファイル・`LICENSE`・`Cargo.toml`・`collector-android/` の 1 ファイル・`Cargo.lock`・`.github/workflows/ci.yml` のどれか 1 つだけが違う 2 つの commit と、同じ道具の版
- **WHEN** それぞれの commit で組み立ての鍵を出す
- **THEN** どの組でも 2 つの鍵は違う

#### Scenario: 名指ししていないファイルが増えれば、鍵が変わる
- **GIVEN** 一方にだけ `tools/release/` の新しい台本が 1 つ足された 2 つの commit と、同じ道具の版
- **WHEN** それぞれの commit で組み立ての鍵を出す
- **THEN** 2 つの鍵は違う

#### Scenario: 道具の版が違えば、鍵が変わる
- **GIVEN** 同じ commit と、Rust の版の文字列だけが違う 2 つの道具の版
- **WHEN** それぞれで組み立ての鍵を出す
- **THEN** 2 つの鍵は違う

#### Scenario: 鍵を出せない木では、鍵を出さずに落ちる
- **GIVEN** git の木でない置き場（親の置き場の git の木も見ない）
- **WHEN** 組み立ての鍵を出す
- **THEN** 手順は 0 でない終了コードで終わり、標準出力に鍵を出さない

#### Scenario: 前の組み立ては、鍵が空でなく完全に一致した PR の run でだけ引く
- **GIVEN** この repo の CI の定義
- **WHEN** Windows の配布物の job の手順を読む
- **THEN** 前の組み立ての配布物を引く手順は、PR の run で鍵が空でないときだけ走り、鍵の完全な一致で引き、前方一致の引き方を持たない

#### Scenario: 引けたときだけ組み立てを飛ばし、今と同じ名前で上げる
- **GIVEN** この repo の CI の定義
- **WHEN** Windows の配布物の job の手順を読む
- **THEN** インストーラの試験と配布物の組み立ての手順は引けなかったときだけ走り、配布物を上げる手順は引けたかどうかに依らず今と同じ成果物の名前で走る

#### Scenario: 組み立てた配布物は、鍵が空でなく組み立てが成功したときだけ保存する
- **GIVEN** この repo の CI の定義
- **WHEN** Windows の配布物の job の手順を読む
- **THEN** 配布物を保存する手順は、引けず・鍵が空でなく・それまでの手順が成功したときだけ走り、引く手順と同じ鍵と置き場を持つ

#### Scenario: 上げる配布物は全部、保存と引きの置き場に入る
- **GIVEN** この repo の CI の定義
- **WHEN** Windows の配布物の job の手順を読む
- **THEN** その job が成果物として上げる置き場は 1 つ残らず、前の組み立てを引く手順と保存する手順の置き場に入っている

#### Scenario: 鍵を出す手順は、道具の版と走っている CI の定義を鍵に入れる
- **GIVEN** この repo の CI の定義
- **WHEN** Windows の配布物の job の鍵を出す手順を読む
- **THEN** その手順は、Rust の版・Node の版・runner の image の名前と版・走っている run の CI の定義を、鍵の台本に渡す

#### Scenario: ビルドするときは、Rust の依存のビルドを戻す
- **GIVEN** この repo の CI の定義
- **WHEN** Windows の配布物の job の手順を読む
- **THEN** 引けなかったときだけ、Rust の依存の置き場と組み立ての置き場を、Rust の版と依存の固定の一覧の hash を持つ鍵で戻し、Rust の版が同じ前の鍵からも戻せる

#### Scenario: 使い回した run でも、インストールと更新の Scenario が 1 つ欠ければ CI が落ちる
- **GIVEN** この repo の CI の定義
- **WHEN** Windows のインストールの試験の job の手順を読む
- **THEN** その job は、配る形と試験の形の 2 つの配布物でインストールと更新の試験を走らせ、走った Scenario を台本の `--list` の全部と突き合わせ、欠けがあれば 0 でない終了コードで終わる手順を、使い回したかどうかで分けずに持つ
