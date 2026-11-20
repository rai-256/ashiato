# CH25 — CI の偶発的な赤を直す（gradle の部品の取り置き・時計のテストの許容幅）（上流・軽い路）

軽い路（`reason: same`。製品の振る舞いは変えない）。深掘りと合意は省いてある。仕様・spec-review・Task ごとの実装とレビュー・merge gate・確認バッチは残る。
change: `openspec/changes/ch25-ci-flake-gradle-cache-clock-test/`（`skip_specs: true`。正典の Scenario は変えない）

## 何を直すか

- **gradle の部品の取り置き**: gradle を使う 7 つの job（ci.yml の `build-stamp`・`android`・`android-instrumented`・`package-linux`・`chain`、nightly.yml と release.yml の `package-linux`）が、
  部品を毎回配布元から取り直さず、CI の取り置きから取る。依頼が挙げた `chain` は `draft.sh --self-test` → `build.sh apk` で gradle を呼ぶ（spec-review R1 で分かった）
- **時計の試験**: 偽の取り込み口が、応答を返す瞬間の時刻にずらし（± 5 分）を足して Date を返す。用意の間に 1.5 秒眠る試験を足し、直す前に必ず落ちることを証跡に残す。
  製品のコード（`HttpDateClock::now`）・正典の 2 つの Scenario・試験の幅の字面は変えない（`check-ch25.py clock` が見る）

## 本人が決めたこと（依頼）

- 依存の版を変えない・ロックの検査が今と同じに効く・時計の試験の幅を弱めない・製品のコードを変えない
- 取り置きの形（`setup-java` の `cache: gradle` か `setup-gradle`）は「作るときに決める」—— 上流で D1（仮）に仮決めした

## 仮決め（`design.md`）

- **D1（仮）**: setup-java を持つ 6 つの job は `setup-java` に `cache: gradle` と、ロックを鍵に入れる `cache-dependency-path` を足す（新しい外の action が増えない）。
  反転条件: merge の後も部品を取れずに落ちる・取り寄せが減らないなら `gradle/actions/setup-gradle` に替える
- **D2（仮）**: 配る組み立て（nightly・release の `package-linux`）も取り置きを使う。main の取り置きを書けるのは main の run だけで、部品の checksum の確かめは今も無い（強さは変わらない）
- **D3（仮）**: setup-java を持たない `chain` は、ci.yml で既に使っている `actions/cache` で job ごとの鍵で取り置く（JDK を変えない）

## 規模

- capability の前倒し: なし（仕様を持たない change）
- tasks: 3 Task・8 項目と、人間の確認待ち 1 つ（CI の上でしか見えない取り置きの復元と効き目）
- spec-review: 指摘 6 件（すべて technical）。6 件とも fixed。A（人間に返す問い）は 0 件

## 残るもの

- 取り置きは鍵が変わった最初の run と、main が保存する前の PR では効かない
- setup-java を持つ 6 つの job は同じ鍵を使うので、先に保存した job が使わない部品は取り置きに無い（D1 の代償と反転条件）
- 走っている `ch22-soak-install-online-ci` も ci.yml を触る（`install-windows`）。job は重ならない

🤖 Generated with [Claude Code](https://claude.com/claude-code)
