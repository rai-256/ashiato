# CH25 の設計

軽い路（`reason: same`）。決めることは取り置きの形・配る組み立て（nightly・release）にも使うか・setup-java を持たない `chain` の取り置きの 3 つ。どちらも workflow の数行を戻せば元に戻る（`loss` なし）。

## Decisions

### D1（仮）. 取り置きは、既にある `actions/setup-java` の段の `cache: gradle` で行う

依頼は `setup-java` の `cache: gradle` と `gradle/actions/setup-gradle` のどちらかを「作るときに決める」とした。上流で `setup-java` に仮決めする。
理由: setup-java を持つ 6 つの job は、既に `setup-java`（commit の SHA で固定済み）の段があり、`with` に足すだけで済む。新しい外の action（`uses:`）が増えない。
取り置くのは `~/.gradle/caches` と `~/.gradle/wrapper`。既定の鍵の材料は `**/*.gradle*`・`**/gradle-wrapper.properties` などで、ロック（`*.lockfile`）も `gradle.properties` も入らない（`premises.md`）。
ロックだけを書き直す変更（`--write-locks`）で取り置きが古いまま残らないよう、`cache-dependency-path` に `collector-android/**/*.gradle.kts`・`collector-android/**/*.lockfile`・`collector-android/gradle/wrapper/gradle-wrapper.properties` を並べ、鍵をこの 3 つの hash にする（`cache-dependency-path` は既定の材料を置き換えるので、wrapper の版も明示する）。
代償: 鍵は ubuntu の 6 つの job で同じになり、先に終わった job の取り置きが保存される（鍵が当たれば保存し直さない）。
その job が使わない部品（たとえば detekt）は、ほかの job では取り置きに無く、今と同じく配布元から取る。
反転条件: merge の後も、取り置きを復元した job が配布元から部品を取れずに落ちる、または確認バッチ 4.1 (2) で取り寄せが減っていないと分かったら、
job ごとに取り置きを持つ `gradle/actions/setup-gradle`（既定で main だけが書く）に替える。

### D2（仮）. 配る組み立て（nightly.yml・release.yml の `package-linux`）も取り置きを使う

依頼の「原因」は `ci.yml` だけを挙げるが、「どこまで」は「gradle を使う全 job」なので含める（`tools/release/build.sh apk` が gradle を呼ぶ）。
default branch（main）の取り置きを書けるのは main の run（push・schedule など）だけで、PR の run が書いたものは、ほかの PR・印の run から読めない（`premises.md`）。
部品の checksum の確かめ（gradle の dependency verification）は今も無い。配布元から取るときも確かめていないので、取り置きから取っても確かめの強さは変わらない。
反転条件: 部品の確かめ（`verification-metadata.xml`）を入れる、または取り置きの中身を main 以外が書ける形に変わったら、配る組み立ては取り置きを使わない形を見直す。

### D3（仮）. `chain` は、ci.yml で既に使っている `actions/cache` の段で取り置く

`chain` は setup-java を持たず、runner に入っている JDK で `tools/release/draft.sh --self-test` が `build.sh apk` を通して gradle を呼ぶ（spec-review R1）。
setup-java を足すと JDK が替わる（振る舞いが変わりうる）ので足さない。ci.yml の `package-windows` が既に使っている `actions/cache`（同じ commit の SHA）で、
`~/.gradle/caches` と `~/.gradle/wrapper` を、job の名前を頭に付けた鍵（`gradle-chain-<OS>-<gradle.kts・ロック・wrapper の版の hash>`）で取り置く。
鍵が job ごとなので、D1 の「先に保存した job の取り置き」の取り合いに加わらない。新しい外の action は増えない。
反転条件: D1 を `gradle/actions/setup-gradle` に替えるときは、`chain` も同じ形にそろえる。`draft.sh --self-test` が gradle を呼ばなくなったら、この段を外す。
