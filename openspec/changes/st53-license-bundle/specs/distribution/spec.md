## ADDED Requirements

### Requirement: 本体の許諾に、名指しした Google Play 開発者サービスのライブラリと組み合わせて配る追加の許可を付ける

THE SYSTEM SHALL 本体の許諾を、GNU AGPL 第 3 版に第 7 条の追加の許可を 1 つ足したものにする。許可の文の正本は英文で、`LICENSES/AdditionRef-ashiato-play-services.txt` の 1 ファイルに置く。
THE SYSTEM SHALL 正本の `Listed libraries:` より前の行を、次の英文と 1 字も違わないものにする（段落の間は空行 1 つ）。

```
Additional permission under GNU AGPL version 3 section 7

If you modify this Program, or any covered work, by linking or combining it with the Google Play services libraries listed below (any version of them, but not a modified version), containing parts covered by the terms of the Android Software Development Kit License, the licensors of this Program grant you additional permission to convey the resulting work. Corresponding Source for a non-source form of such a combination shall not include the source code for the parts of those libraries used.
```

THE SYSTEM SHALL 正本の `Listed libraries:` の節に、名指しのライブラリを `- <group>:<name>` の形で 1 行ずつ書く。中身は `com.google.android.gms:play-services-location`・`com.google.android.gms:play-services-base`・`com.google.android.gms:play-services-basement`・`com.google.android.gms:play-services-tasks` の 4 つ。
THE SYSTEM SHALL `LICENSE` の先頭に正本のファイルの中身をそのまま置き、その後に AGPL-3.0 の全文を変えずに置く。
IF `LICENSE` の先頭が正本のファイルと 1 字でも違う、または AGPL-3.0 の全文が今の `LICENSE` の全文と違う THEN THE SYSTEM SHALL 許諾の文の検査を落とし、CI を落とす。
THE SYSTEM SHALL `README.md` のライセンスの節に、追加の許可が付いていること・正本は英文であること・日本語の参考訳を置く。`CONTRIBUTING.md` には、貢献も同じ許諾（追加の許可を含む）の下で配ることを書き足し、CLA の受け入れ条件は変えない。

導出元: FR-127, NFR-26, 扉 35。深掘り Q1・Q2（第 1 回の推奨）。premises P1・P10。review/deep.md R2・R3。review/spec.md R7・R8。§5「配布の方針」（許可の範囲は Google Play 開発者サービスのライブラリに限る）。
許可の文は配った版に付いたまま出回り、取り消せない（扉 35）。名指しを狭く持てば、必要になったときに次の版で広げられる。

#### Scenario: 追加の許可の正本は、深掘りで決めた英文と同じ
- **WHEN** 許諾の文の検査を今の木に当てる
- **THEN** 正本の `Listed libraries:` より前の行は、この Requirement の英文と 1 字も違わない

#### Scenario: LICENSE の先頭は追加の許可の正本と同じ
- **WHEN** 許諾の文の検査を今の木に当てる
- **THEN** `LICENSE` の先頭の行は `LICENSES/AdditionRef-ashiato-play-services.txt` の全行と同じで、検査の終了コードは 0

#### Scenario: LICENSE の許可の文を正本と変えると、許諾の文の検査が落ちる
- **GIVEN** `LICENSE` の追加の許可の文から `but not a modified version` を消した使い捨ての木
- **WHEN** 許諾の文の検査を当てる
- **THEN** 検査は 0 でない終了コードで終わり、`LICENSE` と正本が違うことが示される

#### Scenario: LICENSE の AGPL の全文を変えると、許諾の文の検査が落ちる
- **GIVEN** `LICENSE` の AGPL-3.0 の全文の 1 行を書き換えた使い捨ての木
- **WHEN** 許諾の文の検査を当てる
- **THEN** 検査は 0 でない終了コードで終わり、AGPL の全文が違うことが示される

#### Scenario: 許可の文の名指しは、位置のライブラリとそれが引く 3 つだけ
- **WHEN** 正本のファイルの `Listed libraries:` の節を読む
- **THEN** 名指しは `play-services-location`・`play-services-base`・`play-services-basement`・`play-services-tasks` の 4 つ（どれも `com.google.android.gms`）で、`play-services-ads` も `play-services-measurement` も無い

#### Scenario: README のライセンスの節は、追加の許可と正本の言葉を示す
- **WHEN** 許諾の文の検査を今の木に当てる
- **THEN** `README.md` のライセンスの節は `AdditionRef-ashiato-play-services` と、正本が英文で日本語は参考訳であることを持つ。持たなければ検査は 0 でない終了コードで終わる

### Requirement: ソースと部品の許諾の表示は、追加の許可を含む 1 つの式にそろえる

THE SYSTEM SHALL 本体のソースの先頭の許諾の表示と、部品の許諾の欄（根の `Cargo.toml` の workspace の `license`・`web/package.json` の `license`・`web/package-lock.json` の根の `license`）を、`AGPL-3.0-only WITH AdditionRef-ashiato-play-services` にする。
API の契約の写し（`docs/openapi.json`）の許諾の欄も同じ式にする。
THE SYSTEM SHALL 次の 2 種類は、表示を書き換えない。
- 移行のファイル（`migrations/` の 3 つ。中身の指紋を DB に記録しているため）は `AGPL-3.0-only` のまま。最初の印から後は、前進側の 2 つ（`.sql`）の表示を直せない（配った移行の凍結の検査が中身の差を落とす）
- 第三者のファイル（gradle の wrapper の `collector-android/gradlew`・`gradlew.bat`。Apache-2.0）はそのまま
THE SYSTEM SHALL ソースの許諾の表示の検査を持つ。検査は、追跡しているファイル（symlink は辿らない）の先頭 25 行にある `SPDX-License-Identifier:` を読む。決めた式でもなく、上の例外の一覧（パスと許す値の組）にも合わない表示があれば、CI を落とす。

導出元: FR-127 ★ 2026-10-25（追加の許可を足す Story は、ソースの SPDX を一緒に直す）。深掘り Q5（第 1 回の推奨）。premises P4・P5・P6。review/deep.md R8・R11。review/spec.md R3・R16。深掘り Q10・Q11（第 2 回）。D4・D5。§5 ★ 2026-11-03。
§5 の「他人の GPL / AGPL のコードを取り込むと、その部分に許可が及ばない」について。repo に写したコードは依存の検査に現れない。写したコードが先頭 25 行に自分の許諾の表示を持っていれば、この検査が止める（表示を持たないものは止められない。D5・Q11）。

#### Scenario: 決めた許諾の表示でないソースがあれば、表示の検査が落ちる
- **GIVEN** 先頭に `// SPDX-License-Identifier: GPL-3.0-only` を持つ Rust のファイルを 1 つ足した使い捨ての木
- **WHEN** ソースの許諾の表示の検査を当てる
- **THEN** 検査は 0 でない終了コードで終わり、そのファイルの名前が示される

#### Scenario: 例外の一覧のファイルは、元の表示のままで表示の検査を通る
- **WHEN** ソースの許諾の表示の検査を今の木に当てる
- **THEN** 検査の終了コードは 0 で、例外として通したものに `collector-android/gradlew`・`collector-android/gradlew.bat`（`Apache-2.0`）と移行の 3 つのファイル（`AGPL-3.0-only`）の 5 つが示される

#### Scenario: 例外の一覧のファイルでも、許す値と違う表示なら表示の検査が落ちる
- **GIVEN** 移行の例外のファイルの表示を `GPL-3.0-only` に書き換えた使い捨ての木
- **WHEN** ソースの許諾の表示の検査を当てる
- **THEN** 検査は 0 でない終了コードで終わり、そのファイルの名前が示される

#### Scenario: 表示の検査は symlink の先を読まない
- **GIVEN** 木の外の `GPL-3.0-only` の表示を持つファイルを指す symlink を 1 つ足した使い捨ての木
- **WHEN** ソースの許諾の表示の検査を当てる
- **THEN** 検査の終了コードは 0 で、symlink の先のファイルは読まれない

#### Scenario: API の契約の写しは、部品と同じ許諾の式を持つ
- **WHEN** API の契約を作り直して、checked-in の写しと比べる
- **THEN** 写しの `info.license.identifier` は `AGPL-3.0-only WITH AdditionRef-ashiato-play-services` で、作り直したものとの差は無い

### Requirement: 配布物に入る依存の許諾を、携帯の依存も含めて検査する

THE SYSTEM SHALL 依存の許諾の検査に、Rust・画面（Node）・携帯（gradle / maven）の 3 つの節を持たせる。携帯の節は、配る携帯の APK に入る依存（`releaseRuntimeClasspath`）の部品ごとに POM の許諾を読む。
POM が許諾を持たなければ、親の POM を辿って読む。読んだ許諾の名前は、対応表で SPDX の名前に直す。1 つの部品が許諾の名前を 2 つ以上持てば、全部の名前が許可されているか名指しに当たるときだけ通す（1 つでも許可の外なら落とす）。
IF 依存の POM を取り寄せられない（ネットワークにも gradle の cache にも無い） THEN THE SYSTEM SHALL その部品を飛ばさず、検査を落とす。
THE SYSTEM SHALL オープンソースの許諾でない部品のうち、追加の許可の正本の `Listed libraries:` に名指しされた部品だけを通す。名指しの一覧は正本のファイルから読み、検査の中に写しを持たない。
IF 携帯の依存に、名指しの外のオープンソースでない部品・許諾を読めない部品・対応表に無い許諾の名前の部品のどれかがある THEN THE SYSTEM SHALL 検査を落とし、その部品の座標と許諾の名前を示す。
THE SYSTEM SHALL 許可の一覧から `AGPL-3.0-only` を外す。自分の部品（workspace の crate と画面の根）は、名前と決めた許諾の式の組で通す。他人の部品の GPL / AGPL の許諾は、どれも検査が落とす。
THE SYSTEM SHALL ほかの許可の一覧は減らさない。一覧は `MIT`・`MIT-0`・`Apache-2.0`・`BSD-2-Clause`・`BSD-3-Clause`・`ISC`・`Unicode-3.0`・`Unicode-DFS-2016`・`Zlib`・`CC0-1.0`・`MPL-2.0`・`0BSD`・`Apache-2.0 WITH LLVM-exception`・`CDLA-Permissive-2.0`・`BlueOak-1.0.0`・`CC-BY-4.0`・`Python-2.0` のまま。
IF ある節の検査する対象が見つからない（`web/node_modules` が無い・gradle が依存を解決できない） THEN THE SYSTEM SHALL その節を通さず、対象が無いことを示して検査を落とす。

導出元: NFR-26 ★ 2026-10-25（C-01 を含めて機械で確かめる。許可の外のプロプライエタリな依存が入ったら止める）。FR-127。深掘り Q2（第 1 回の推奨）。premises P1・P2・P3。review/deep.md R8 の C。review/spec.md R6・R7・R11・R16。D6（仮）・D7。深掘り Q12（第 2 回）。
壊してはいけないもの: 依存の許諾の検査の既存の許可一覧（`AGPL-3.0-only` を外すのは、自分の部品を名前で通す形に替えるためで、今の依存は 1 つも落ちない。P3）。

#### Scenario: 今の携帯の依存は、許諾の検査を通る
- **WHEN** 依存の許諾の検査の携帯の節を今の木に当てる
- **THEN** 検査の終了コードは 0

#### Scenario: 今の携帯の依存で、オープンソースでないとして通すのは名指しの 4 つだけ
- **WHEN** 依存の許諾の検査の携帯の節を今の木に当てる
- **THEN** オープンソースでない部品として通したと示されるものは、名指しの 4 つの座標だけ

#### Scenario: 名指しの外の Google のライブラリを足すと、許諾の検査が落ちる
- **GIVEN** 今の携帯の依存の一覧に、許諾が `Android Software Development Kit License` の `com.google.android.gms:play-services-ads` を 1 行足した偽の依存の一覧
- **WHEN** 依存の許諾の検査の携帯の節を当てる
- **THEN** 検査は 0 でない終了コードで終わり、`com.google.android.gms:play-services-ads` と許諾の名前が示される

#### Scenario: 対応表に無い許諾の名前の部品があれば、許諾の検査が落ちる
- **GIVEN** POM の許諾の名前が対応表に無い部品を 1 つ持つ、偽の依存の一覧
- **WHEN** 依存の許諾の検査の携帯の節を当てる
- **THEN** 検査は 0 でない終了コードで終わり、その部品の座標と許諾の名前が示される

#### Scenario: 許諾の名前を 2 つ持つ部品は、片方でも許可の外なら許諾の検査が落ちる
- **GIVEN** 許諾の名前が `The Apache Software License, Version 2.0` と `Android Software Development Kit License` の 2 つを持つ、名指しの外の部品の偽の依存の一覧
- **WHEN** 依存の許諾の検査の携帯の節を当てる
- **THEN** 検査は 0 でない終了コードで終わり、その部品の座標と `Android Software Development Kit License` が示される

#### Scenario: 許諾の名前を 2 つ持つ部品は、両方が許可されていれば通る
- **GIVEN** 許諾の名前が `The Apache Software License, Version 2.0` と `The MIT License` の 2 つを持つ部品の、偽の依存の一覧
- **WHEN** 依存の許諾の検査の携帯の節を当てる
- **THEN** 検査の終了コードは 0

#### Scenario: POM が許諾を持たない部品は、親の POM の許諾で判定する
- **GIVEN** 許諾を持たない POM と、Apache-2.0 の許諾を持つその親の POM を置いた、使い捨ての maven の置き場から解決する依存
- **WHEN** 携帯の依存の POM を書き出し、依存の許諾の検査の携帯の節を当てる
- **THEN** その部品は親から読んだ許諾として示され、Apache-2.0 として通る

#### Scenario: 名指しの一覧を正本から消すと、その部品で許諾の検査が落ちる
- **GIVEN** 正本のファイルの `Listed libraries:` から `play-services-tasks` の行を消した使い捨ての正本と、`play-services-tasks` を持つ偽の依存の一覧
- **WHEN** 依存の許諾の検査の携帯の節を当てる
- **THEN** 検査は 0 でない終了コードで終わり、`com.google.android.gms:play-services-tasks` が示される

#### Scenario: 他人の部品の AGPL の許諾は、許諾の検査が落とす
- **GIVEN** 許諾が `AGPL-3.0-only` の他人の crate を 1 つ持つ、偽の Rust の依存の一覧
- **WHEN** 依存の許諾の検査の Rust の節を当てる
- **THEN** 検査は 0 でない終了コードで終わり、その crate の名前が示される

#### Scenario: 自分の部品は、決めた許諾の式で許諾の検査を通る
- **WHEN** 依存の許諾の検査の Rust と画面の節を今の木に当てる
- **THEN** 検査の終了コードは 0 で、workspace の crate と画面の根は `AGPL-3.0-only WITH AdditionRef-ashiato-play-services` として通る

#### Scenario: 許可の一覧は、AGPL-3.0-only を外したほかは前と同じ
- **WHEN** 依存の許諾の検査の許可の一覧を読む
- **THEN** 一覧はこの Requirement に並べた 17 の値とちょうど同じで、`AGPL-3.0-only` を持たない

#### Scenario: 検査する対象が無い節は、通さずに落ちる
- **GIVEN** gradle が依存を解決できない（携帯の組み立ての道具が無い）環境
- **WHEN** 依存の許諾の検査の携帯の節を当てる
- **THEN** 検査は 0 でない終了コードで終わり、対象が無いことが示される

#### Scenario: node_modules が無ければ、画面の節は通さずに落ちる
- **GIVEN** `web/node_modules` が無い木
- **WHEN** 依存の許諾の検査の画面の節を当てる
- **THEN** 検査は 0 でない終了コードで終わり、対象が無いことが示される

### Requirement: 配布物は、本体の許諾・依存の許諾の一覧・対応するソースの場所を持つ

THE SYSTEM SHALL サーバの Linux 版・Windows 版、PC の収集アプリ、携帯の APK の 4 つの配布物に、次の 3 つのファイルを入れる。tar / zip では根に、APK では `assets/licenses/` に置く。
- `LICENSE`: 木の `LICENSE` と同じ中身（追加の許可を含む）
- `THIRD-PARTY-NOTICES.txt`: その配布物に入る依存の一覧。部品ごとに `== <名前> <版>` の見出し・`License: <式>` の行・許諾の全文と著作権の表示を書く
- `SOURCE.txt`: 対応するソースの場所
THE SYSTEM SHALL `THIRD-PARTY-NOTICES.txt` の依存に、次のものを入れる。
- サーバ: 実行ファイルに入る Rust の部品、組み込んだ画面に入る Node の部品（開発だけの部品を除く）、Rust の標準ライブラリ（`rust-std`）
- PC の収集アプリ: 実行ファイルに入る Rust の部品と、Rust の標準ライブラリ（`rust-std`）
- 携帯の APK: `releaseRuntimeClasspath` の部品と、名指しの Google のライブラリが中に持つ第三者の一覧（各 aar の `third_party_licenses.txt` の中身。見出し `(gms の中の第三者)`）
IF ある部品の許諾の全文も著作権の表示も見つからない THEN THE SYSTEM SHALL 依存の一覧を組まずに、組み立てを落とす。
THE SYSTEM SHALL `SOURCE.txt` に、リポジトリの場所を書く。正式版・ベータ版の公開の組み立てでは、それに加えて印 `v<版番号>` を書く。それ以外の組み立てでは 40 桁の commit を書く。開発版と main への push の組み立てでは組み立てた main の commit、PR の組み立てでは PR の head の commit を書く。
IF 公開の組み立てに渡された印が `v<版番号>` と違う THEN THE SYSTEM SHALL 配布物を作らずに組み立てを落とす。
THE SYSTEM SHALL 3 つのファイルを組み立ての台本の中で入れる。正式版・ベータ版だけでなく、開発版・main への push・PR の組み立てにも同じ手順で入れる。
IF 配布物のどれかに 3 つのファイルのどれかが無い、`LICENSE` が木の `LICENSE` と違う、サーバと PC の収集アプリの一覧に `rust-std` の項が無い、または携帯の APK の一覧に Google のライブラリの第三者の一覧が無い THEN THE SYSTEM SHALL 配布物の検査を落とし、その配布物を公開しない。
THE SYSTEM SHALL 配布物の検査で、`SOURCE.txt` を次の決まりで見る。合わなければ検査を落とす。
- 正式版・ベータ版として公開する配布物の検査（debug の鍵を許さない、チャンネルが `release` か `beta` の検査）: `v<期待する版番号>`
- 開発版の検査（チャンネルが `nightly` の検査）: 組み立ての印と同じ commit
- debug の鍵を許す検査（PR・main への push の組み立てと下書きの配布物の検査）: 40 桁の commit。組み立ての印とは比べない（PR の組み立ての印は GitHub が作る merge の commit を持ち、`SOURCE.txt` の head の commit と一致しない）

導出元: FR-127（本体の許諾と依存の許諾の一覧を同梱する。対応するソースは同じ印のリポジトリで示す）。深掘り Q6・Q7（第 1 回の推奨）。review/deep.md R5・R6・R7・R10 の C。review/spec.md R1・R2・R5・R11・R16。D8・D9・D16・D17（仮）。深掘り Q13・Q14・Q15（第 2 回）。FR-127 ★ 2026-11-02・★ 2026-11-03。
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
- **GIVEN** 印も PR の head の commit も渡されない組み立て（開発版か main への push の組み立て）
- **WHEN** 配布物の `SOURCE.txt` を読む
- **THEN** `SOURCE.txt` はリポジトリの場所と、組み立ての印と同じ 40 桁の commit を持ち、`nightly` の文字を持たない

#### Scenario: PR の組み立ての配布物のソースの場所は、PR の head の commit
- **GIVEN** 組み立ての印と違う 40 桁の commit を PR の head の commit として渡した組み立て
- **WHEN** 配布物の `SOURCE.txt` を読む
- **THEN** `SOURCE.txt` はリポジトリの場所と、渡した head の commit を持ち、組み立ての印の commit を持たない

#### Scenario: PR の組み立ての head の commit が 40 桁の 16 進でなければ、配布物を作らずに落ちる
- **GIVEN** PR の head の commit として `main` を渡した組み立て
- **WHEN** `SOURCE.txt` を組む
- **THEN** 手順は 0 でない終了コードで終わり、`SOURCE.txt` はできない

#### Scenario: PR の組み立てと下書きの配布物の検査は、head の commit のソースの場所で通る
- **GIVEN** `SOURCE.txt` が組み立ての印と違う 40 桁の commit（PR の head）を持つ配布物の置き場
- **WHEN** debug の鍵を許す配布物の検査（PR の組み立てと下書きの検査）を当てる
- **THEN** 検査の終了コードは 0

#### Scenario: ソースの場所がチャンネルの決まりと合わない配布物は、公開しない
- **GIVEN** `SOURCE.txt` に印でなく commit を持つ配布物の置き場
- **WHEN** `--channel beta --expect-version 0.4.0-beta.1` で配布物の検査を当てる
- **THEN** 検査は 0 でない終了コードで終わり、`SOURCE.txt` が決まりと合わないことが示される

### Requirement: 正式な鍵を読む公開は、本人の承認を経る

THE SYSTEM SHALL 配布用の Environment `release` に、本人を必須の承認者として付け、管理者が承認を飛ばせない設定にする。承認されるまで、その Environment を使う公開の job は始まらず、正式な鍵も渡らない。
THE SYSTEM SHALL 本人が印を打って本人が承認する運用を止めない（自分の承認を禁じる設定を求めない）。
THE SYSTEM SHALL AI のエージェントが使う資格情報を、リポジトリの管理の権限と公開の承認の権限を持たないものにする。
THE SYSTEM SHALL 本人が鍵を置く手順の順を「エージェントの資格情報の切り替え → ルールセットと承認者 → 鍵」にする。
THE SYSTEM SHALL 守りの確かめの台本に、2 つの副コマンドを持たせる。どちらも、満たさなければ 0 でない終了コードで終わる。
- `approvals`: `release` に必須の承認者が付き、本人がその中にいて、管理者が承認を飛ばせないことを確かめる
- `agent-credential`: 今の資格情報がリポジトリの管理者でないことを確かめる
THE SYSTEM SHALL 正式版とベータ版の公開の job で、署名の前に `approvals` を通し、通らなければ署名も公開もしない。
THE SYSTEM SHALL 公開の job の中の確かめが守りの本体でないことを、手順の文書に書く。job の定義は印の commit の中の workflow で、書き換えられるため。守りの本体は、GitHub が job を始める前に効く承認である。

導出元: ST52 の IMP-01（印を push できる人が main を通さずに正式な鍵を読めるかもしれない。後の Story で扱う → ST53）。扉 34。深掘り Q3（第 1 回の推奨）。premises P7・P8・P11・P12。review/deep.md R4。D11（仮）。
一度読まれた鍵は、読まれなかったことにできない。鍵を替えると配った APK は上書きできなくなり、全員に入れ直しを頼むことになる（ST52 の P2）。
公開の承認の権限は API から読めないので、資格情報がそれを持たないことは確認バッチで確かめる（tasks の「人間の確認待ち」）。

#### Scenario: 承認者の無い配布用の Environment では、公開の前の確かめが落ちる
- **GIVEN** `release` に保護の規則が無い偽の配布元
- **WHEN** `approvals` の確かめを走らせる
- **THEN** 確かめは 0 でない終了コードで終わり、必須の承認者が無いことが示される

#### Scenario: 管理者が承認を飛ばせる設定では、公開の前の確かめが落ちる
- **GIVEN** `release` に本人が必須の承認者として付いているが、管理者が承認を飛ばせる偽の配布元
- **WHEN** `approvals` の確かめを走らせる
- **THEN** 確かめは 0 でない終了コードで終わり、管理者が承認を飛ばせることが示される

#### Scenario: 本人が承認者で管理者が飛ばせなければ、公開の前の確かめを通る
- **GIVEN** `release` に本人が必須の承認者として付き、管理者が承認を飛ばせず、自分の承認を禁じる設定が無い偽の配布元
- **WHEN** `approvals` の確かめを走らせる
- **THEN** 確かめの終了コードは 0

#### Scenario: 本人が承認者にいなければ、公開の前の確かめが落ちる
- **GIVEN** `release` の必須の承認者が本人でない利用者だけの偽の配布元
- **WHEN** `approvals` の確かめを走らせる
- **THEN** 確かめは 0 でない終了コードで終わる

#### Scenario: 管理者の資格情報では、エージェントの資格情報の確かめが落ちる
- **GIVEN** リポジトリの権限が `admin: true` と返る偽の配布元
- **WHEN** `agent-credential` の確かめを走らせる
- **THEN** 確かめは 0 でない終了コードで終わり、管理者の権限があることが示される

#### Scenario: 管理者でない資格情報では、エージェントの資格情報の確かめを通る
- **GIVEN** リポジトリの権限が `admin: false`・`maintain: false`・`push: true` と返る偽の配布元
- **WHEN** `agent-credential` の確かめを走らせる
- **THEN** 確かめの終了コードは 0

#### Scenario: 公開の job は、署名の前に承認者を確かめる
- **WHEN** 守りの確かめで、公開の workflow を読む
- **THEN** 公開の job の手順で、`check-guards.sh approvals` が署名の手順（`sign-apk.sh`）より前にある。そうでなければ確かめは 0 でない終了コードで終わる
