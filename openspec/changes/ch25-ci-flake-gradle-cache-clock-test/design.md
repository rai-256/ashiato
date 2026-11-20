# CH25 の設計

軽い路（`reason: same`）。決めることは取り置きの形と、配る組み立て（nightly・release）にも使うかの 2 つ。どちらも workflow の数行を戻せば元に戻る（`loss` なし）。

## Decisions

### D1（仮）. 取り置きは、既にある `actions/setup-java` の段の `cache: gradle` で行う

依頼は `setup-java` の `cache: gradle` と `gradle/actions/setup-gradle` のどちらかを「作るときに決める」とした。上流で `setup-java` に仮決めする。
理由: 6 つの job はどれも既に `setup-java`（commit の SHA で固定済み）を持ち、`with` に 1 つ足すだけで済む。新しい外の action（`uses:`）が増えない。
取り置くのは `~/.gradle/caches` と `~/.gradle/wrapper`。鍵は OS と `**/*.gradle*`・`**/gradle.properties`・`**/gradle-wrapper.properties` などの hash（`premises.md`）。
代償: 鍵は ubuntu の 6 つの job で同じになり、先に終わった job の取り置きが保存される（鍵が当たれば保存し直さない）。
その job が使わない部品（たとえば detekt）は、ほかの job では取り置きに無く、今と同じく配布元から取る。
反転条件: merge の後も、取り置きを復元した job が配布元から部品を取れずに落ちる、または確認バッチ 4.1 (2) で取り寄せが減っていないと分かったら、
job ごとに取り置きを持つ `gradle/actions/setup-gradle`（既定で main だけが書く）に替える。

### D2（仮）. 配る組み立て（nightly.yml・release.yml の `package-linux`）も取り置きを使う

依頼の「原因」は `ci.yml` だけを挙げるが、「どこまで」は「gradle を使う全 job」なので含める（`tools/release/build.sh apk` が gradle を呼ぶ）。
取り置きの中身を書けるのは main への push の run だけで、PR の run が書いたものは、ほかの PR・印の run から読めない（`premises.md`）。
部品の checksum の確かめ（gradle の dependency verification）は今も無い。配布元から取るときも確かめていないので、取り置きから取っても確かめの強さは変わらない。
反転条件: 部品の確かめ（`verification-metadata.xml`）を入れる、または取り置きの中身を main 以外が書ける形に変わったら、配る組み立ては取り置きを使わない形を見直す。
