# CH25 — CI の偶発的な赤を直す（gradle の部品の取り置き・時計のテストの許容幅）

> 軽い路（`lane.json` の `reason: same`）。深掘りと合意は省いてある。製品の振る舞いは変えず、CI の定義と試験の中だけを直す。

## Why

CI が中身と関係なく赤になり、毎回、人か機械が再実行している。原因は 2 つある。

1. gradle を使う job は部品の取り置き（キャッシュ）を持たない。毎回、数百個の部品を Maven Central などから取り直すので、配布元が一瞬おかしいだけで落ちる
   （2 日で 4 回。run 44684479094 の android では、実在する `org.ow2.asm:asm:9.8` などが「無い」と返った）
2. 時計の試験 `clock_reference_pc_behind_gives_negative_difference`（と進んでいる側の兄弟）は、偽のサーバの Date を用意の前に決めている。
   許容幅に用意の数ミリ秒が入らないので、Date の秒の切り捨てが 999 ms に近い回に幅を外れる（Windows の CI で `skew=-298996 width=4`）。製品のコードは正しい

## What Changes

- `.github/workflows/ci.yml`（`build-stamp`・`android`・`android-instrumented`・`package-linux`）・`nightly.yml`・`release.yml`（どちらも `package-linux`）の
  `actions/setup-java` の段に `cache: gradle` と、ロックを鍵に入れる `cache-dependency-path` を足す（design D1（仮）・D2（仮））。
  setup-java を持たない `chain` は、ci.yml で既に使っている `actions/cache` の段で取り置く（design D3（仮））。gradle を使う job はこの 7 つ（`check-ch25.py ci` が workflow を読んで数える）
- `crates/collector-windows/src/clock.rs` の `#[cfg(test)]` の中: 偽の取り込み口が、応答を返す瞬間の時刻にずらし（± 5 分）を足して Date を返す。
  用意の間に 1.5 秒眠る試験を足し、直す前に落ちることを証跡に残す（Task 1）
- 依存の版・ロック・ロックの検査・製品のコード・正典の Scenario と試験の幅は変えない。仕様を持たない（`.openspec.yaml` の `skip_specs: true`）。
  理由: 製品の振る舞いは何も変わらない。時計の試験は正典の 2 つの Scenario の印を持つ既存の試験で、Scenario も幅も字面のまま残す（`check-ch25.py clock` が見る）。
  CI の取り置きには正典の capability が無い（`openspec/specs` に CI の部品の取り寄せを言う Requirement は無い）

## Capabilities

### New Capabilities

なし

### Modified Capabilities

なし

## Impact

- 触れる予定のパス（`lane.json`）: `.github/workflows/ci.yml`・`.github/workflows/nightly.yml`・`.github/workflows/release.yml`・`crates/collector-windows/src/clock.rs`・`docs/changes/CH25.md`
- 依頼が挙げた `chain` の job は、`./tools/release/draft.sh --self-test` → `tools/release/build.sh apk` で gradle を呼ぶ（出力は捨てているので、ログに部品の取り寄せは出ない。spec-review R1）。
  依頼の「など」に当たる job は、上の 7 つで全部（`install-windows`・`package-windows`・`package-artifacts`・`rust`・`smoke` は gradle を呼ばない。
  `rust` が走らせる `tools/check-contract-version.sh` は引数なしでは gradle を呼ばず、呼ぶのは `--write` だけ）
- ロックの検査が今と同じに効く理由: 依存の版はどれも固定で（`collector-android/*.gradle.kts` に `+`・`latest`・`SNAPSHOT` が無い。2026-11-21 に grep）、
  ロック（`gradle.lockfile`）が解決を縛る。取り置きは解決の結果を変えず、取り寄せの手間だけを省く。`tools/check-gradle-locks.sh` は `GRADLE_USER_HOME` を変えない
- 取り置きの範囲（`premises.md`）: PR の run が保存した取り置きは、その PR の中でしか読めない。main への push が保存したものを、全部の PR・印の run が読む
- 走っている change との重なり: `ch22-soak-install-online-ci` も `.github/workflows/ci.yml` を触る（`install-windows` の job）。この change は gradle の 7 つの job の
  取り置きの指定だけを触るので、job が重ならない。後から merge する側が rebase で追従する。`ch24-zip-upgrade` はこれらのパスに触れない
- 触れないもの: `tools/**`・`collector-android/**`・`openspec/specs/`・`docs/testing.md`・製品のコード
