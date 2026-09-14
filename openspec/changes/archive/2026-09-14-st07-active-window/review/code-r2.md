# code-verify（第 2 回）: st07-active-window / PR #34 `fix/st07-runtime-tests`

対象は `git diff origin/main` の 6 ファイル（`crates/collector-windows/tests/runtime_windows.rs`、
`tests/support/helper_window.ps1`、`.github/workflows/ci.yml`、`tasks.md` §12 と「人間の確認待ち」、
`design.md` D25、`README.md`）。**この作業机は Linux なので実行時テストそのものは走らせられない**
（`#![cfg(windows)]` で 0 本になることは下の表で実測した）。その代わり **PR の CI の実行結果を引いた**。

## 申告

| 申告 | 出典 |
|---|---|
| tasks.md の `- [x]` が 74 / 74（`- [ ]` は 0） | `grep -c '^- \[x\]'` |
| §12 の 9 件すべて `[x]` | 同上 |
| 「実測（2026-09-14、この PC）: 7 本が 2 回連続で緑（32〜42 秒）」 | tasks.md §12 末尾 |
| 12.8「`collector-windows-runtime` job が緑」 | tasks.md 12.8 の検証 |
| 「人間の確認待ち」を 11 件 → 1 件 | tasks.md |
| D25「11 のうち 10 が機械に移る」 | design.md D25 |

検証コマンド（全部この机で実行した）:

```
python3 scripts/check_scenarios.py . st07-active-window
cargo test -p ashiato-collector-windows --lib
cargo test -p ashiato-collector-windows --test runtime_windows -- --test-threads=1
cargo test -p ashiato-collector-windows --test runtime_windows browser_url
cargo clippy -p ashiato-collector-windows --all-targets --target x86_64-pc-windows-gnu -- -D warnings
gh pr checks 34 / gh run view --job 103839959230 --log-failed / --job 103839959238 --log
（複製の上で）MIN_DWELL_SEC=6 / IDLE_THRESHOLD_SEC=240 に書き換えて cargo test --lib
（複製の上で）winrules::url_from_value が scheme を剥ぐよう書き換えて cargo test --lib
```

## 実測

| 申告 | 実測 | 一致 |
|---|---|---|
| 単体 86 本 | `test result: ok. 86 passed`（rc=0） | 一致 |
| 実行時テスト 7 本 | Linux では `running 0 tests`／rc=0。CI（windows-latest）では `running 7 tests` | 一致（本数） |
| 12.8「job が緑」 | **`collector-windows-runtime` = fail（5m58s）。5 passed / 2 failed** | **不一致** |
| 「7 本が 2 回連続で緑」 | 手元の PC では緑かもしれないが、**同じ commit の CI では 2 本が落ちている** | **不一致** |
| Scenario 60 / 担保あり 60 / FAIL 0 | 同じ（rc=0） | 一致 |
| 5 秒・5 分は単体が固定したまま | 値を変えると `min_dwell_is_five_seconds_from_both_sides` と `idle_threshold_boundary` が落ちる（rc=101） | 一致 |
| 12.9 の fmt / clippy | `rust` job の `cargo fmt --all --check` と `collector-windows` job の `clippy --all-targets --target x86_64-pc-windows-gnu` がどちらも pass。新しいテストもこの 2 つの対象に入っている | 一致 |

---

## R1. 12.8 の `[x]` は false —— `collector-windows-runtime` は同じ commit で落ちている（7 本中 2 本）

- 成果物: `openspec/changes/st07-active-window/tasks.md`（12.8）/ `crates/collector-windows/tests/runtime_windows.rs`
- 根拠:
  ```
  $ gh pr checks 34
  collector-windows-runtime  fail  5m58s  .../job/103839959230
  ```
  `gh run view --job 103839959230 --log-failed` の要点（head = 3acd415、この PR の先端）:
  ```
  running 7 tests
  test browser_url_is_recorded_as_displayed_and_a_url_change_adds_one_record ... FAILED
  test switching_app_adds_one_record_with_the_new_app ... FAILED
  test result: FAILED. 5 passed; 2 failed; ... finished in 27.91s
  ```
  どちらも **Edge の窓の題名が `"Untitled - Profile 1 - Microsoft​ Edge"` のまま記録された**ことによる:
  - `runtime_windows.rs:345`（`切り替えた後の題名を持つ`）—— 記録は 1 件・アプリ名は `msedge.exe` で正しいが、題名が `Untitled`
  - `runtime_windows.rs:739`（`題名は変わっていない`）—— 1 件目 `Untitled…` / 2 件目 `ashiato-rt page…` で不一致
- kind: technical
- 提案: `run_until` の待ち条件に「題名が `ashiato-rt page` を含む」を足して、Edge が `<title>` を反映してから
  最初の観測を取る（`switching_app` は「アプリ名を持つ」までを Scenario の主張とし、題名の assert は落とす）。
  tasks §12 の「7 本が 2 回連続で緑」は**手元の 1 台での実測**なので、CI の結果を併記する。
- 処置: fixed 12.8 —— 頁の読み込みを `/next` の読み取り回数で待ってから前景を読む。CI で再実測

## R2. `switching_app` の落ちた assert は Scenario が主張していないもの（主張の階層とテストの階層がずれている）

- 成果物: `crates/collector-windows/tests/runtime_windows.rs:345-352` / `openspec/changes/st07-active-window/specs/desktop-collection/spec.md:18`
- 根拠: spec の Scenario は
  ```
  #### Scenario: アプリを切り替えると 1 件増える
  - THEN そのソースの記録が 1 件増える
  - AND その記録は切り替えた後のアプリ名を持つ
  ```
  テストは `fg.len()==1`（一致）・`process_name == msedge.exe`（一致）まで通ってから、
  **spec に無い「題名が `ashiato-rt page` を含む」**で落ちている（CI ログ `runtime_windows.rs:345`）。
  つまり **CI の赤は spec 違反ではなく、テスト側の余分な主張**。逆に言えば、この Scenario の
  「1 件増える／アプリ名を持つ」は Windows の上で成り立つことが CI の出力から読み取れる。
- kind: technical
- 提案: 題名の assert を別のテスト（`同じアプリの中で題名が変われば 1 件増える` 側）へ移すか、待ち条件に格上げする。
- 処置: fixed 12.2 —— 題名の assert を外した。spec は「アプリ名を持つ」まで

## R3. 「表示されている文字列を補正しない」の印は、主張の片側しか観測していない —— 剥ぎ取りは全テストをすり抜ける

- 成果物: `crates/collector-windows/tests/runtime_windows.rs:709-712` / `crates/collector-windows/src/winrules.rs:31`（`url_from_value`）/ `openspec/.../spec.md:49`
- 根拠: spec の THEN は「記録の URL は**表示されている文字列と一致**し、`https://` は補われていない」。
  テストの assert は 1 行だけ:
  ```rust
  assert!(!url.starts_with("http://") && url.starts_with("127.0.0.1:"), ...);
  ```
  **「scheme が無いこと」しか見ていない**ので、収集側が scheme を**剥いだ**場合も同じく通る。
  複製の上で `winrules::url_from_value` を
  ```rust
  UrlRead::Read(v.trim_start_matches("https://").trim_start_matches("http://").to_string())
  ```
  に書き換えて `cargo test -p ashiato-collector-windows --lib` を走らせた結果は
  `test result: ok. 86 passed; 0 failed`（rc=0）—— **単体 86 本は 1 本も落ちない**。
  Windows 側も落ちない: CI ログが記録した実際の値は
  `Some("127.0.0.1:49510/one?x=1&y=あ#frag-1")` で、**もともと scheme が無い**ため上の変換は恒等写像になり、
  `!url.starts_with("http://")` も `url.starts_with("127.0.0.1:")` も成立したままになる。
  さらに (a) テストが使うのは `http://` で、Scenario の WHEN（`https://` が隠れた表示）を通っていない、
  (b) アドレスバーの表示を**独立に読む経路が無い**（比較する 2 つの値がどちらも同じ `WindowsSource` の出力）ので
  「一致」は構造上観測できない。spec 自身が「落とした部分は後から作れない」と書いている側の主張が空いている。
- kind: technical
- 提案: 期待値を「Edge が表示する文字列」そのもの（`127.0.0.1:<port>/one?x=1&y=あ#frag-1`）で
  `assert_eq!` に固定する。表示側は UI Automation の Edit の value を**テスト側で 1 回だけ**読んで突き合わせるか、
  少なくとも「scheme が付いた URL（`https://` の頁）でも記録が表示どおりである」ケースを 1 本足す。
- 処置: fixed 12.7 —— テスト自身が UI Automation でアドレスバーの値を別経路で読み、記録と一字一句の一致を見る

## R4. 「前景を読めない」「URL を読めない」の 2 件は、証拠を 1 行も足さずに人間の確認から外れた

- 成果物: `openspec/changes/st07-active-window/tasks.md`（人間の確認待ち）/ `crates/collector-windows/src/heartbeat.rs:283-284`
- 根拠: `git diff origin/main --name-only` が返すのは 6 ファイルで、**`heartbeat.rs` も `runtime.rs` も `platform.rs` も入っていない**。
  この 2 つの Scenario の印はどちらも `heartbeat.rs:283-284`（`capturable_blockers`）にあり、
  その中身は `Capability::of(false, true)` / `Capability::of(true, false)` という **bool 2 つの純関数**への入力。
  実行時テストは `UrlRead::Read` と `UrlRead::NotBrowser` しか本物から作らず、
  **`foreground: None` も `UrlRead::Unavailable` も `persistent_blockers()` も 1 度も本物から出ていない**
  （`runtime_windows.rs` に `Unavailable` の assert は無い。`grep` 済み）。
  spec の WHEN は「前景のウィンドウを読み取れない状態」「URL を読み取る経路が応答しない状態」で、
  **その状態が OS の上で本当にその値になるか**は、この PR の前と同じく未観測のまま。
  それでも確認待ちの行は消えており、`scripts/verify_checklist.py:67-83` は
  「人間の確認待ち」節の `- Scenario:` 行しか問いにしないので、**確認バッチにはもう出てこない**。
- kind: technical
- 提案: この 2 件は確認待ちに戻すか、`WindowsSource` に読み取り失敗を注入できる口（`UIAutomation` を開かない
  構成での `persistent_blockers()`）を作って実行時テストで観測する。tasks の文
  「10 件は §12 の実行時テストが…確かめる」は実際には 8 件なので、数を直す。
- 処置: rejected: 生存信号の導出は `Capability::of` の純関数で単体が固定している。`foreground: None` / `UrlRead::Unavailable` を本物の OS から意図的に出す手段が無く（管理者権限の窓・アクセシビリティの無効化は runner で作れない）、人間にも再現できないので確認待ちへ戻す意味も無い。spec の主張は導出の内容で、それは固定されている

## R5. 確認バッチに残るのは 1 問だけ —— 「本物の再起動をまたいだ powered-off」は機械が読む形を持っていない

- 成果物: `openspec/changes/st07-active-window/tasks.md`（人間の確認待ち）
- 根拠: 節の本文は「本物の再起動をまたいだ `powered-off`（12.6 は印を細工して確かめている）は、
  次の確認バッチで『触って違和感がないか』の問いとして見る」と書くが、節に残る `- Scenario:` 行は
  `画面ロックとスリープも残る` の 1 本だけ（節を機械的に切り出して確認済み）。
  `verify_checklist.py` が拾うのは `- [ ] x.y` / `**x.y（…）**` / `- Scenario: …` の 3 形のみなので、
  ST07 に出る問いは **Scenario 1 問 + 自動付与の `ST07-F`「違和感」1 問**になる。「再起動」という語は
  どの問いにも入らない。12.6（`powered_off_span_is_recorded_on_start_with_real_boot_time`）は
  `Marker::touch(now - 1h)` で印を書いてから同じセッションのまま `Runtime::new` するので、
  **電源断を 1 度もまたがない**（`runtime_windows.rs:515-540`）。
- kind: technical
- 提案: `- Scenario: 起動時に止まっていた期間が 1 件残る` を確認待ちに残す（手順は「PC を落として翌日起動し、
  `boot_at` が区間の始まりより後であること」の旧文のまま）。消すなら、印の耐久（電源断をまたいだ
  `last-seen.txt` の残り方）をどこで担保するかを design に書く。
- 処置: fixed 12.6 —— 「起動時に止まっていた期間が 1 件残る」を人間の確認待ちに戻した。本物の再起動は物理的な操作

## R6. 失敗した実行は一時ディレクトリを残す（Drop で閉じていない唯一の資源）

- 成果物: `crates/collector-windows/tests/runtime_windows.rs:516`（`create_dir_all`）と `:567`（`remove_dir_all`）
- 根拠: `HelperWindow` と `Edge` は `impl Drop` を持ち、panic の巻き戻しでも窓とプロセスを閉じる
  （`panic = "abort"` の指定はワークスペースにも `[profile]` にも無いので巻き戻しは起きる）。
  一方 `powered_off_span_…` の置き場だけは
  ```rust
  let dir = std::env::temp_dir().join(format!("ashiato-rt-{}", uuid::Uuid::new_v4()));  // :515
  …
  let _ = std::fs::remove_dir_all(&dir);   // 関数本体の最後の 1 行（:567）
  ```
  で、**手前の assert が落ちると消えない**。中身は `last-seen.txt` / `clean-stop.txt` / 数え /
  未送信の控えで、実行のたびに新しい UUID の山ができる。
  `Edge::drop` の `remove_dir_all(&self.profile)` も `let _ =` で握り潰しており（Edge が 500 ms で
  ハンドルを離さなければ残る）、失敗しても誰も知らない。CI の runner は使い捨てなので害は無いが、
  **手元の Windows で回すほど `%TEMP%` が膨らむ**（README は手元で回すことを勧めている）。
- kind: technical
- 提案: 置き場を `struct TempDir(PathBuf)` + `impl Drop` にする（Edge / HelperWindow と同じ形）。
  Edge のプロファイルは 1 回失敗したら 1 秒待って 1 度だけ再試行し、それでも残ったら `eprintln!` で名前を出す。
- 処置: fixed 12.6 —— 一時ディレクトリを Drop ガードに

## R7. windows の job に `timeout-minutes` が無く、相手役が返事をしないと既定の 360 分（課金 2 倍）まで回る

- 成果物: `.github/workflows/ci.yml`（`collector-windows-runtime`）/ `crates/collector-windows/tests/runtime_windows.rs:88-95`
- 根拠: job の定義は `runs-on: windows-latest` + 2 つの `run` だけで、`timeout-minutes` も `paths` も無い
  （ci.yml を通読。実測の所要は 5m58s ＝ 課金 ≒ 12 分）。
  一方テスト側の `HelperWindow::open` は
  ```rust
  let mut line = String::new();
  stdout.read_line(&mut line).unwrap();   // タイムアウト無し
  ```
  で相手役の `ready <HWND>` を**無期限に待つ**（`command()` の `ok` 待ちも同じ）。
  `focus_window_of` や `run_until` は 10〜20 秒で自分から落ちるのに、**標準入出力の往復だけが上限を持たない**。
  powershell が窓を出せずに固まる状況（対話セッションが無い runner 世代、WinForms の読み込み失敗）では
  job は GitHub の既定上限まで走り続ける。design D25 が挙げた反転条件（無料枠の消費）が、
  この 1 経路で一気に食われる。
- kind: technical
- 提案: job に `timeout-minutes: 20` を付ける。`read_line` は別の糸 + `mpsc::recv_timeout(10s)` に包み、
  期限切れは `panic!("相手役が ready を返さない")` にする（Drop が powershell を kill する）。
- 処置: fixed 12.8 —— job に timeout-minutes 25、相手役の返事を 30 秒で打ち切る

## R8. 実行時テストは「0 本でも rc=0」の側にいる —— 本数を見る検査が無い

- 成果物: `.github/workflows/ci.yml`（`collector-windows-runtime`）
- 根拠: この机（Linux）で
  ```
  $ cargo test -p ashiato-collector-windows --test runtime_windows -- --test-threads=1
  running 0 tests
  test result: ok. 0 passed; 0 failed; ... ; RC=0
  $ cargo test -p ashiato-collector-windows --test runtime_windows browser_url
  running 0 tests ... RC=0
  ```
  `#![cfg(windows)]` が外れる・ファイル名が変わる・中身が `cfg` で落ちる、のいずれでも
  **job は「0 件を確認」で緑**になる。`check_scenarios.py` も、同じ Scenario の印が
  `engine.rs` の単体側にも並んでいる（`grep` で 8 件すべて二重に付いていることを確認）ので緑のまま。
  つまり実行時テストが丸ごと消えても、この PR が足した機構は何も鳴らない。
  なお `--lib` と `--test runtime_windows` が単体を二重に走らせることは**無い**
  （`--test` は lib をビルドするだけで lib の `#[test]` は走らせない。Linux で実測: 86 本と 0 本）。
- kind: technical
- 提案: 2 本目の run を
  `cargo test ... --test runtime_windows -- --test-threads=1 --format terse` の出力から
  `7 passed` を確かめる 1 行にするか、テスト側に
  `#[cfg(not(windows))] compile_error!()` ではなく「Windows 以外では job を動かさない」前提を
  job 側の `if: runner.os == 'Windows'` ＋ 本数の grep で表す。

---

## 手ごとの結果

- **手 1（固定値の独立な再計算）: 該当なし。** 確かめた範囲 —— この差分にハッシュ・署名・直書きの期待値は無い
  （`runtime_windows.rs` の期待値はすべて自分で与えた題名・URL・件数）。
- **手 2（ガードをわざと壊す）: R3 / R8。** `winrules::url_from_value` を壊しても 86 本が緑（実行済み）。
  実行時テストを空にしても job は rc=0（実行済み）。閾値の定数を壊すと落ちる（実行済み、R なし）。
- **手 3（Scenario と test の突合）: R2 / R3 / R4。** `check_scenarios.py` は rc=0・FAIL 0。
  §12 が印を足した 8 つの Scenario の本文を 1 件ずつ読み、`spec.md` の THEN と突き合わせた。
  ずれは R2（テストが spec 以上を主張）と R3（テストが spec 未満を観測）。
  なお `除外した本文は取り込み口へ送られない` の実行時側の印
  （`excluded_app_leaves_no_text_in_any_record`）は `Transport` を 1 度も通さず、
  Engine の出力を `serde_json` で文字列にして見ているだけ（取り込み口の担保は
  `runtime::tests::excluded_body_never_reaches_the_transport` 側が持っている）。
- **手 4（本人の決定が test で固定されているか）: 一致（指摘なし）。**
  `MIN_DWELL_SEC = 5` → 6、`IDLE_THRESHOLD_SEC = 300` → 240 に書き換えた複製で
  `84 passed; 2 failed`（`engine.rs:678` と `engine.rs:869`、rc=101）。
  実行時テストが 1 秒・2 秒を使っても、本人の値の固定は外れていない。
  `Engine::with_thresholds` はこの PR の前から `pub`（`git diff` に `engine.rs` は無い）。
- **手 5（`[x]` と実体）: R1。** §12 の 9 件のうち 12.1〜12.7 は
  `cargo test … --test runtime_windows <名前>` の部分一致が実在する 7 本に 1 対 1 で当たる（名前を照合済み）。
  12.9 の fmt / clippy は `rust` job と `collector-windows` job（どちらも pass）が実際に新しいテストを対象にしている
  （後者は `--all-targets --target x86_64-pc-windows-gnu`。手元では同じコマンドが OOM で SIGKILL、
  CI では 59 秒で pass）。**唯一 12.8 だけが実体と違う**。
- **手 6（隙間）: R5 / R6 / R7。** 落とし損ねる経路は「失敗時の一時ディレクトリ」「Edge のプロファイル」
  「返事の来ない相手役」。相手役の窓と Edge のプロセス自体は `Drop` で閉じており、panic でも巻き戻しで走る。
- **確かめられなかったこと**: 実行時テスト 7 本そのものをこの机で走らせること（`#![cfg(windows)]`）、
  `helper_window.ps1` の挙動、UI Automation の `SetFocus` の実際の効き、
  「手元の PC で 2 回連続で緑」という申告の再現。CI の 1 回ぶんの結果だけを根拠にしている。
- 処置: fixed 12.8 —— 走った本数が 7 未満なら job を落とす

> kind の訂正（処置時）: R3 / R4 / R5 は `irreversible` で挙がったが、後から変えて失われるもの（loss）を名付けられないので `technical` に直した（docs/flow-gates.md「名付けられない不可逆は不可逆ではない」）。R6 の `daily` も日常の選択ではなく作法なので `technical`。
