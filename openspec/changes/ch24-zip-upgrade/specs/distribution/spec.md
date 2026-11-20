## MODIFIED Requirements

### Requirement: 配布物に入る依存の許諾を、携帯の依存も含めて検査する

THE SYSTEM SHALL 依存の許諾の検査に、Rust・画面（Node）・携帯（gradle / maven）の 3 つの節を持たせる。携帯の節は、配る携帯の APK に入る依存（`releaseRuntimeClasspath`）の部品ごとに POM の許諾を読む。
POM が許諾を持たなければ、親の POM を辿って読む。読んだ許諾の名前は、対応表で SPDX の名前に直す。1 つの部品が許諾の名前を 2 つ以上持てば、全部の名前が許可されているか名指しに当たるときだけ通す（1 つでも許可の外なら落とす）。
IF 依存の POM を取り寄せられない（ネットワークにも gradle の cache にも無い） THEN THE SYSTEM SHALL その部品を飛ばさず、検査を落とす。
THE SYSTEM SHALL オープンソースの許諾でない部品のうち、追加の許可の正本の `Listed libraries:` に名指しされた部品だけを通す。名指しの一覧は正本のファイルから読み、検査の中に写しを持たない。
IF 携帯の依存に、名指しの外のオープンソースでない部品・許諾を読めない部品・対応表に無い許諾の名前の部品のどれかがある THEN THE SYSTEM SHALL 検査を落とし、その部品の座標と許諾の名前を示す。
THE SYSTEM SHALL 許可の一覧から `AGPL-3.0-only` を外す。自分の部品（workspace の crate と画面の根）は、名前と決めた許諾の式の組で通す。他人の部品の GPL / AGPL の許諾は、どれも検査が落とす。
THE SYSTEM SHALL ほかの許可の一覧は減らさない。一覧は `MIT`・`MIT-0`・`Apache-2.0`・`BSD-2-Clause`・`BSD-3-Clause`・`ISC`・`Unicode-3.0`・`Unicode-DFS-2016`・`Zlib`・`CC0-1.0`・`MPL-2.0`・`0BSD`・`Apache-2.0 WITH LLVM-exception`・`CDLA-Permissive-2.0`・`BlueOak-1.0.0`・`CC-BY-4.0`・`Python-2.0`・`bzip2-1.0.6` の 18 の値。
IF ある節の検査する対象が見つからない（`web/node_modules` が無い・gradle が依存を解決できない） THEN THE SYSTEM SHALL その節を通さず、対象が無いことを示して検査を落とす。

導出元: NFR-26 ★ 2026-10-25（C-01 を含めて機械で確かめる。許可の外のプロプライエタリな依存が入ったら止める）。FR-127。深掘り Q2（第 1 回の推奨）。premises P1・P2・P3。review/deep.md R8 の C。review/spec.md R6・R7・R11・R16。D6（仮）・D7。深掘り Q12（第 2 回）。CH24 の深掘り Q-G1（本人の答え 2026-11-20: zip 9 の既定の features のまま、許可の一覧に `bzip2-1.0.6` を足す）。
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
- **THEN** 一覧はこの Requirement に並べた 18 の値とちょうど同じで、`AGPL-3.0-only` を持たない

#### Scenario: POM を取り寄せられない部品があれば、許諾の検査が落ちる
- **GIVEN** 使い捨ての maven の置き場に、`.module` と jar はあるが `.pom` が無い部品を 1 つ足した依存（依存の解決は通る）を、ネットワークを使わずに解決する環境
- **WHEN** 携帯の依存の POM を書き出し、依存の許諾の検査の携帯の節を当てる
- **THEN** 検査は 0 でない終了コードで終わり、その部品の座標と、POM を取れないことが示される（依存の解決の失敗ではない）

#### Scenario: 検査する対象が無い節は、通さずに落ちる
- **GIVEN** gradle が依存を解決できない（携帯の組み立ての道具が無い）環境
- **WHEN** 依存の許諾の検査の携帯の節を当てる
- **THEN** 検査は 0 でない終了コードで終わり、対象が無いことが示される

#### Scenario: node_modules が無ければ、画面の節は通さずに落ちる
- **GIVEN** `web/node_modules` が無い木
- **WHEN** 依存の許諾の検査の画面の節を当てる
- **THEN** 検査は 0 でない終了コードで終わり、対象が無いことが示される

