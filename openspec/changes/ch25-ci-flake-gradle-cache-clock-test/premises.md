# CH25 の前提（外の形式・振る舞い）

| 前提 | 何が依るか | 確かめ方 | 結果 | 出所 | 確かさ |
|---|---|---|---|---|---|
| `setup-java` の `cache: gradle` は `~/.gradle/caches` と `~/.gradle/wrapper` を取り置く。鍵は OS・arch と `**/*.gradle*`・`**/gradle.properties`・`**/gradle-wrapper.properties` などの hash。後片付けの段で保存し、鍵が当たったときは保存し直さない | D1（取り置きの範囲と、job の間で同じ鍵になること） | https://github.com/actions/setup-java（README の Caching）と docs/advanced-usage.md を読んだ（2026-11-21） | 記載のとおり | 公式文書 | 2 |
| 取り置きは、同じ ref か default branch のものだけを復元できる。PR の run が作ったものは merge ref に属し、base の branch やほかの PR からは読めない。default branch へ書けるのは push・schedule などの run だけ | D2（配る組み立てが読む取り置きを書けるのは main の run だけ）・確認バッチ 4.1（PR の初回は取り置きが無い） | https://docs.github.com/en/actions/reference/workflows-and-actions/dependency-caching の「Restrictions for accessing a cache」（2026-11-21） | 記載のとおり。印（tag）の run は自分の印と default branch から読む（文書の鍵の照合の節からの推論） | 公式文書 | 2 |
| `gradle/actions/setup-gradle` は既定で default branch の job だけが取り置きを書く | D1 の反転条件で替える先 | https://github.com/gradle/actions/blob/main/docs/setup-gradle.md（2026-11-21） | 記載のとおり | 公式文書 | 2 |
| 依存の版はどれも固定で、動く版（`+`・`latest`・`SNAPSHOT`）が無い | 取り置きがロックの検査の結果を変えないこと | `grep -n 'latest\|+"\|SNAPSHOT' collector-android/*.gradle.kts collector-android/app/build.gradle.kts`（2026-11-21） | 0 件。ロックは `collector-android/app/gradle.lockfile`・`buildscript-gradle.lockfile` | 実物 | 3 |
| 時計の試験は Linux でも走る（`#[cfg(windows)]` の外） | Task 1 の検証を手元と ubuntu で走らせられること | `scripts/quiet-run cl -- cargo test -p ashiato-collector-windows --lib clock::tests::clock_reference -- --list`（2026-11-21） | 5 本 | 実物 | 4 |
