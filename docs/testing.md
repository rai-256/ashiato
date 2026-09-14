# テストの書き方 —— 1 枚にまとめたもの

2026-09-14。テスト監査（`docs/briefs/test-audit.html`）で「規範が skill・agent・design.md・tasks の規律節・
testdb.rs の docstring に散っている」と出たので、ここに寄せる。**判断の出所は各所の D 番号と実測**で、
ここは目次と要点。矛盾したら D 番号の側が正しい（そのときはここを直す）。

## 1. どこに何を置くか

| 実行体 | 単体 | 本物を使う結合 | 実行時（OS を触る） | 縦串・検査 |
|---|---|---|---|---|
| server（Rust） | `ingest.rs` / `heartbeat.rs` の `#[cfg(test)]` | `api_tests.rs` / `dedup_tests.rs` / `coverage/tests.rs` / `registry_tests.rs`（**本物の PostgreSQL**。`testdb.rs`） | — | `tools/smoke.sh`（curl で外から）/ `tools/check-*.sh` |
| collector-windows（Rust） | 各モジュールの `#[cfg(test)]`（ubuntu で走る 86 本） | — | `tests/runtime_windows.rs`（**Windows の上でだけ**。テストが自分で窓を作る） | — |
| collector-android（Kotlin） | `src/test`（JUnit4 + Robolectric） | — | `src/androidTest`（エミュレータでも実機でも同じ） | — |
| web（React） | `src/__tests__`（vitest + jsdom） | — | — | 実寸は人間の確認待ち |

**CI が走らせる job**: `rust`（fmt / clippy / test + 検査 4 本）/ `collector-windows`（cross の clippy）/
`collector-windows-runtime`（windows-latest）/ `web` / `android`（単体）/ `android-instrumented`（エミュレータ）/
`smoke`（縦串 + panic-log + immutable）/ `chain`（token があるとき）。

## 2. テストは spec の Scenario から生まれる

```
specs/**/spec.md の #### Scenario:   → test に // Scenario: <名前> の印（一字一句。空白は無視）
                                    → scripts/check_scenarios.py が突合。印の無い Scenario は FAIL
                                    → 実機でしか確かめられないものは tasks.md「人間の確認待ち」に - Scenario: <名前>
```

- 1 つのテストに印を複数置いてよい。1 つの Scenario に印が複数あってもよい
- **印の先のテストが Scenario の主張の階層を本当に観測しているか**を、code-verify の手 3 が 1 件ずつ見る。
  spec が「バイト列」なら `->>` で取り出した文字列を見てはいけない（実測 ST01。`raw` が jsonb で全緑）
- 「人間の確認待ち」に置けるのは**機械が再現できない物理的な操作**（ロック・スリープ・電池・本物の GPS・
  時間そのもの）だけ。実機の OS を触る部分は実行時テストで確かめる（§5）

## 3. 本人の決定はテストが固定する

deep.md の「本人の答え」にある数値・列挙・する/しないは、**その値を書き換えたときに落ちるテスト**を持つ。
実測で毎 Story 同じ穴が開いた（ST01 R1: 60 秒・5 分を差し替えても全緑 / ST02 R3〜R7 / ST07 R7・R8・R19）。

- 値は `pub const` にし、テストはその定数を**名指しで**固定する（`assert_eq!(MIN_DWELL_SEC, 5, "深掘り Q6 の値が変わっている")`）
- **検査は定数と定数を突き合わせない**（ST02 design D27）。片側は固定の予算・期待値、もう片側は実際に入っている値
- 実行時テストで閾値を短くするときは `with_thresholds` のような注入口を使い、既定値の固定は単体に残す（ST07 §12）
- code-verify の手 4 が「値を変えても全部通る」を探す。見つかったら**テストを足す側**で閉じる

## 4. 本物を使う。飛ばさない

- DB を要するテストは**本物の PostgreSQL**を叩く（ST02 design D14）。振る舞いが DB の側にある（トリガ・一意索引・`AT TIME ZONE`）ので、
  模擬すると「3 手の迂回」の型の穴が残る。接続できなければ**落ちる**（飛ばすと DB の無い環境で全部緑になる）
- テストどうしの隔離は `logical_source` の接頭辞で取る（`testdb::source`）。表を作り直さない
- **`cargo test` と `tools/smoke.sh` を並列に走らせない。** smoke は先頭で `docker compose down -v` する
- 画面（jsdom）は実寸を測れない。「指定と勘定」を固定し（D23 / D35）、実寸は人間の確認待ち
- Fake は判断の無い境界（送信・置き場）にだけ置く。DB・サーバ・OS は本物

## 5. 実行時テスト（OS を触る部分）

**Windows**（`crates/collector-windows/tests/runtime_windows.rs`、ST07 design D25）

- テストが自分で窓を作る（WinForms の相手役 `tests/support/helper_window.ps1`、Edge）→ 本物の `WindowsSource` に読ませる → `Engine` に通して記録を数える
- 前景にするのはテスト側の UI Automation `SetFocus`。相手役はメッセージポンプを止めない（stdin を同期で待つと固まる）
- 走らせ方: Windows の上で `cargo test -p ashiato-collector-windows --test runtime_windows -- --test-threads=1`。**触らない**
- 手元は `C:\dev\ashiato2-rt` のような Windows 側の clone で（`\\wsl$` 越しのビルドは遅い）

**Android**（`collector-android/app/src/androidTest`）

- 本物の framework で前景サービス・権限の入口・HTTP を通す。位置は Play services の mock mode、HTTP は端末の中の 127.0.0.1
- 手元: `./tools/android-emulator.sh`（AVD を作って画面なしで起動 → `connectedDebugAndroidTest` → 止める。WSL2 は `/dev/kvm` が要る）
- 実機: adb で繋いで `./gradlew :app:connectedDebugAndroidTest -Pashiato.baseUrl=http://127.0.0.1:18787`（同じテスト）
- `-Pashiato.baseUrl` は平文 HTTP を 127.0.0.1 に許すためだけ

## 6. tasks の検証コマンドは実在し、rc=0 になる

- 各タスクの本文に検証を書く。「動いた」ではなく**コマンドと終了コード**
- `cargo test <名前>` は前方一致。`cargo test --test <名前>` は `tests/<名前>.rs` が**実在する**こと
  （実測 ST03 1.1: `--test migrations` は無いターゲットで、`[x]` のまま残っていた）
- code-verify の手 5 が `[x]` を 1 件ずつ走らせ直す

## 7. 人間の確認

- Story ごとに求めない。確認バッチ（`scripts/verify_batch.sh` → `/verify`）で 1 回
- **正しさのテストではない。** 手順書が聞くのは、完了の判定を目で見ること、物理的な操作、Story ごとに 1 問の「触ってみて違和感は無かったか」
- 実機は「同じテストを走らせられる」形だけ用意し、手順には組み込まない（常に繋げるわけではない）

## 8. 送る前に走らせるもの（CI と同じ）

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
docker compose up -d --wait db && cargo test --workspace          # 本物の DB
./tools/check-migrations.sh && ./tools/check-boundaries.sh && ./tools/check-openapi.sh
(cd web && npm ci && ../tools/check-licenses.sh && npx tsc -b && npm run lint && npm run test && npm run build)
(cd collector-android && ./gradlew :app:assembleDebug :app:testDebugUnitTest)
cargo clippy -p ashiato-collector-windows --all-targets --target x86_64-pc-windows-gnu -- -D warnings
./tools/smoke.sh && ./tools/check-panic-log.sh && ./tools/check-immutable.sh
# Windows の上で:  cargo test -p ashiato-collector-windows
# エミュレータ:    ./tools/android-emulator.sh
```

## 9. 検査の検査

code-verify（`.claude/agents/code-verify.md`）の 6 つの手が、道具を使わない mutation testing になっている
（固定値の再計算・ガードを壊す・Scenario と test の突合・決定の固定・`[x]` の実在・隙間）。
機械化するなら `cargo-mutants` を純粋関数（`ingest.rs` / `coverage.rs` / `collector-windows` の engine）に限って。
