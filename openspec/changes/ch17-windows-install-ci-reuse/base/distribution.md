# distribution の書いた時点の正典（spec_merge.py snapshot。archive の 3-way の base。手で直さない）

### Requirement: コードを触った PR ごとに、配る形の 3 部品を公開せずに組む

WHEN コード（`docs/`・`openspec/`・`*.md` 以外）を触った PR が CI を走らせる THE SYSTEM SHALL サーバ（画面を組み込む。Windows 向けと Linux 向け）・携帯の APK・PC の収集アプリを、配るときと同じ手順で、出所の版を版番号にして組み立てる。
THE SYSTEM SHALL その組み立てで、配布物が 1 つでも組めない・配布物の版番号が食い違う・配る組み立ての検査に当たるときに CI を落とす。ただし携帯の APK は debug の鍵で署名し、`.debug` の印でない release の組み立てにし、`.dirty` の検査は綺麗な checkout の上で行う。
THE SYSTEM SHALL その組み立ての配布物をどこにも公開しない（CI の実行の成果物としてだけ置く）。
WHEN PR が `docs/`・`openspec/`・`*.md` だけを触る THE SYSTEM SHALL この組み立てを飛ばす。

導出元: Story の価値（配れる形を壊した Story が、その PR の時点で落ちる）。深掘り Q9（推奨）。premises P6・P9。

#### Scenario: コードを触った PR では、配る形の組み立てが走る
- **WHEN** `crates/server/src/lib.rs` を触った PR の CI の出し分けを判定する
- **THEN** 配る形の組み立ての job は走る

#### Scenario: 文書だけの PR では、配る形の組み立てを飛ばす
- **WHEN** `docs/` と `openspec/` だけを触った PR の CI の出し分けを判定する
- **THEN** 配る形の組み立ての job は飛ばされる

#### Scenario: 配る形を壊した PR は、その PR の CI で落ちる
- **WHEN** 配布物の 1 つの組み立てが失敗する木で、配る形の組み立ての手順を走らせる
- **THEN** 手順は 0 でない終了コードで終わる

### Requirement: 配布物は、本体の許諾・依存の許諾の一覧・対応するソースの場所を持つ

THE SYSTEM SHALL サーバの Linux 版・Windows 版、PC の収集アプリ、携帯の APK の 4 つの配布物に、次の 3 つのファイルを入れる。tar / zip では根に、APK では `assets/licenses/` に置く。
- `LICENSE`: 木の `LICENSE` と同じ中身（追加の許可を含む）
- `THIRD-PARTY-NOTICES.txt`: その配布物に入る依存の一覧。部品ごとに `== <名前> <版>` の見出し・`License: <式>` の行・許諾の全文と著作権の表示を書く
- `SOURCE.txt`: 対応するソースの場所
THE SYSTEM SHALL `THIRD-PARTY-NOTICES.txt` の依存に、次のものを入れる。
- サーバ: 実行ファイル（Windows 版はサーバと更新係の `ashiato-updater.exe` の両方）に入る Rust の部品、組み込んだ画面に入る Node の部品（開発だけの部品を除く）、Rust の標準ライブラリ（`rust-std`）
- PC の収集アプリ: 実行ファイルに入る Rust の部品と、Rust の標準ライブラリ（`rust-std`）
- 携帯の APK: `releaseRuntimeClasspath` の部品と、名指しの Google のライブラリが中に持つ第三者の一覧（各 aar の `third_party_licenses.txt` の中身。見出し `(gms の中の第三者)`）
IF ある部品の許諾の全文も著作権の表示も見つからない THEN THE SYSTEM SHALL 依存の一覧を組まずに、組み立てを落とす。
THE SYSTEM SHALL `SOURCE.txt` に、リポジトリの場所を書く。正式版・ベータ版の公開の組み立てでは、それに加えて印 `v<版番号>` を書く。それ以外の組み立てでは、組み立てた木の 40 桁の commit を書く。開発版と main への push の組み立てでは main の commit、PR の組み立てでは PR の head の commit になる。
THE SYSTEM SHALL PR の組み立てを、PR の head の commit そのものから組む（GitHub が作る、head とその時の main を合わせた merge の commit からは組まない）。`SOURCE.txt` の commit・組み立ての印・組んだソースが同じ commit を指す。
THE SYSTEM SHALL 配布物の検査で、期待する版番号と、配布物の `LICENSE` と比べる `LICENSE` を、配布物を組んだのと同じ commit の中身から取る。PR の CI の配布物を検査する job は、ほかの job と同じく head とその時の main を合わせた木で走り、期待値だけを PR の head の commit（main への push では push した commit）から取る。
IF 公開の組み立てに渡された印が `v<版番号>` と違う THEN THE SYSTEM SHALL 配布物を作らずに組み立てを落とす。
THE SYSTEM SHALL 3 つのファイルを組み立ての台本の中で入れる。正式版・ベータ版だけでなく、開発版・main への push・PR の組み立てにも同じ手順で入れる。
IF 配布物のどれかに 3 つのファイルのどれかが無い、`LICENSE` が期待する `LICENSE`（配布物を組んだ commit の `LICENSE`）と違う、サーバと PC の収集アプリの一覧に `rust-std` の項が無い、または携帯の APK の一覧に Google のライブラリの第三者の一覧が無い THEN THE SYSTEM SHALL 配布物の検査を落とし、その配布物を公開しない。
THE SYSTEM SHALL 配布物の検査で、`SOURCE.txt` を次の決まりで見る。合わなければ検査を落とす。
- 正式版・ベータ版として公開する配布物の検査（debug の鍵を許さない、チャンネルが `release` か `beta` の検査）: `v<期待する版番号>`
- 開発版の検査（チャンネルが `nightly` の検査）と、debug の鍵を許す検査（PR・main への push の組み立てと下書きの配布物の検査）: 組み立ての印と同じ 40 桁の commit

導出元: FR-127（本体の許諾と依存の許諾の一覧を同梱する。対応するソースは同じ印のリポジトリで示す）。深掘り Q6・Q7（第 1 回の推奨）。review/deep.md R5・R6・R7・R10 の C。review/spec.md R1・R2・R5・R11・R16。D8・D9・D16・D18。深掘り Q13・Q14・Q15（第 2 回）・Q16（第 3 回）・Q17（第 4 回）。FR-127 ★ 2026-11-02・★ 2026-11-03・★ 2026-11-03（第 3 回）。
★ ST57: サーバの Windows 版の一覧に、更新係の実行ファイルに入る Rust の部品を足した（review/spec.md R11）。
動く印 `nightly` は翌日に別の commit を指すので、開発版の写しには動かない commit を書く（Q7）。配った写しに書いた場所は、後から直せない。

#### Scenario: 4 つの配布物は、許諾・依存の一覧・ソースの場所を持つ
- **WHEN** 配る形の 4 つの配布物の置き場に、配布物の検査を当てる
- **THEN** 4 つのどれにも `LICENSE`・`THIRD-PARTY-NOTICES.txt`・`SOURCE.txt` があり（携帯の APK は `assets/licenses/` の下）、検査の終了コードは 0

#### Scenario: 配布物の LICENSE が木の LICENSE と違えば、公開しない
- **GIVEN** サーバの Linux 版の tar の `LICENSE` を、AGPL の全文だけのものに差し替えた配布物の置き場
- **WHEN** 配布物の検査を当てる
- **THEN** 検査は 0 でない終了コードで終わり、`LICENSE` が違う配布物の名前が示される

#### Scenario: 許諾のファイルが欠けた配布物は、公開しない
- **GIVEN** サーバの Linux 版の tar から `THIRD-PARTY-NOTICES.txt` を抜いた配布物の置き場
- **WHEN** 配布物の検査を当てる
- **THEN** 検査は 0 でない終了コードで終わり、欠けたファイルと配布物の名前が示される

#### Scenario: Rust の実行ファイルの依存の一覧は、標準ライブラリを含む
- **WHEN** サーバと PC の収集アプリの配布物の `THIRD-PARTY-NOTICES.txt` を読む
- **THEN** どちらにも `== rust-std ` の見出しの項（`License: MIT OR Apache-2.0`）がある

#### Scenario: サーバの依存の一覧は、画面に入る Node の部品を含み、開発だけの部品を含まない
- **WHEN** サーバの配布物の `THIRD-PARTY-NOTICES.txt` を読む
- **THEN** `react` の項があり、`vitest` の項は無い

#### Scenario: 許諾の全文も著作権の表示も見つからない部品があれば、組み立てが落ちる
- **GIVEN** 許諾のファイルも作者の欄も持たず、許諾の式の全文の写しも無い部品を 1 つ持つ、偽の依存の一覧
- **WHEN** 依存の一覧を組む
- **THEN** 組む手順は 0 でない終了コードで終わり、その部品の名前が示され、依存の一覧のファイルはできない

#### Scenario: 携帯の APK の依存の一覧は、Google のライブラリの中の第三者を含む
- **WHEN** 携帯の APK の `assets/licenses/THIRD-PARTY-NOTICES.txt` を読む
- **THEN** `play-services-location` の aar の `third_party_licenses.txt` から取り出した、`(gms の中の第三者)` の見出しの項がある

#### Scenario: 正式版の配布物のソースの場所は、その版の印
- **GIVEN** 印 `v0.4.0` の公開の組み立て
- **WHEN** 配布物の `SOURCE.txt` を読む
- **THEN** `SOURCE.txt` はリポジトリの場所と印 `v0.4.0` を持つ

#### Scenario: 印が版番号と違う公開の組み立ては、配布物を作らずに落ちる
- **GIVEN** 版番号 `0.4.0` で、印として `v0.4.1` を渡した組み立て
- **WHEN** `SOURCE.txt` を組む
- **THEN** 手順は 0 でない終了コードで終わり、`SOURCE.txt` はできない

#### Scenario: 開発版と main への push の配布物のソースの場所は、組み立てた commit
- **GIVEN** main の commit を checkout した木で、印を渡さない組み立て（開発版か main への push の組み立て）
- **WHEN** 配布物の `SOURCE.txt` を読む
- **THEN** `SOURCE.txt` はリポジトリの場所と、組み立ての印と同じ 40 桁の commit を持ち、`nightly` の文字を持たない

#### Scenario: PR の組み立ての配布物のソースの場所は、PR の head の commit
- **GIVEN** main と合わせていない枝の commit（PR の head）を checkout した木で、印を渡さない組み立て
- **WHEN** 配布物の `SOURCE.txt` と組み立ての印を読む
- **THEN** `SOURCE.txt` はリポジトリの場所と、その head の 40 桁の commit を持ち、組み立ての印も同じ commit を持つ

#### Scenario: 組み立ての commit が 40 桁の 16 進でなければ、配布物を作らずに落ちる
- **GIVEN** 印を渡さず、組み立ての印の commit の上書き（`ASHIATO_BUILD_COMMIT`）に `main` を渡した組み立て
- **WHEN** `SOURCE.txt` を組む
- **THEN** 手順は 0 でない終了コードで終わり、`SOURCE.txt` はできない

#### Scenario: PR の配布物は、PR の head の commit から組まれ、同じ commit の期待値で検査される
- **WHEN** CI の配る形を組む job（サーバの Linux 版・携帯の APK と、Windows の 2 つ）と、その配布物を検査する job の checkout と検査の呼び出しを、workflow の検査で読む
- **THEN** 組む 2 つの job は、PR では PR の head の commit を、main への push では push した commit を checkout する
- **AND** 検査する job の作業の checkout は `ref` を持たない（合わせた木）
- **AND** 検査する job は、期待する版番号と比べる `LICENSE` を、組む 2 つの job と同じ commit を別の置き場に checkout したものから検査に渡す

#### Scenario: 配布物の LICENSE は、渡した期待する LICENSE と比べる
- **GIVEN** 台本の木の `LICENSE` と違う `LICENSE` のファイル（配布物を組んだ commit の `LICENSE` に当たる）と、その `LICENSE` を持つ配布物の置き場
- **WHEN** そのファイルを期待する `LICENSE` として渡して配布物の検査を当て、次に渡さずに当てる
- **THEN** 渡したときは検査の終了コードは 0、渡さないときは 0 でない終了コードで終わり、`LICENSE` が違うことが示される

#### Scenario: PR の組み立てと下書きの配布物の検査は、head の commit のソースの場所で通る
- **GIVEN** `SOURCE.txt` が組み立ての印と同じ 40 桁の commit（PR の head）を持つ配布物の置き場
- **WHEN** debug の鍵を許す配布物の検査（PR の組み立てと下書きの検査）を当てる
- **THEN** 検査の終了コードは 0

#### Scenario: debug の鍵を許す検査でも、組み立ての印の commit でないソースの場所の配布物は落ちる
- **GIVEN** `SOURCE.txt` に、commit でなく印 `v0.4.0` を持つ配布物の置き場と、`Commit: main` を持つ配布物の置き場と、7 桁の短い commit を持つ配布物の置き場と、組み立ての印と違う 40 桁の commit を持つ配布物の置き場
- **WHEN** それぞれに debug の鍵を許す配布物の検査を当てる
- **THEN** 4 つとも検査は 0 でない終了コードで終わり、`SOURCE.txt` が決まりと合わないことが示される

#### Scenario: ソースの場所がチャンネルの決まりと合わない配布物は、公開しない
- **GIVEN** `SOURCE.txt` に印でなく commit を持つ配布物の置き場
- **WHEN** `--channel beta --expect-version 0.4.0-beta.1` で配布物の検査を当てる
- **THEN** 検査は 0 でない終了コードで終わり、`SOURCE.txt` が決まりと合わないことが示される

#### Scenario: サーバの Windows 版の依存の一覧は、更新係だけが持つ部品を含む
- **WHEN** サーバの Windows 版の配布物の `THIRD-PARTY-NOTICES.txt` を読み、更新係の実行ファイルの依存の木と突き合わせる
- **THEN** 更新係だけが持つ暗号化した接続の部品（`rustls` など。サーバは持たない）の項がある
