
## settle 第 1 回（2026-11-10）

- R1・R8（premise / 同じ不具合）: escalated。deep.md 第 2 回 Q6、`deep-questions-r2.json`、`docs/briefs/CH14-deep-r2.html`（`ask_wizard.py` → 止める 2 / 仮でよい 0）。コードは変えていない
- R3（premise。手 7 の「落ちなかった」）: escalated。deep.md 第 2 回 Q7。台本は変えていない
- R2: `console::repair_ident_bytes`（`GetACP()` で `repair_ident_bytes_with_code_page` を呼ぶ）を足し、main.rs の repair はそれを **サービスを止める前** に呼ぶ（whoami の読みも止める前へ移した）。`ch14_unrepresentable_name_is_refused` に、その関数が 1252 で `Err("利用者名 pc\足跡 を Windows の文字コード 1252 で表せない")` を返す assert を足した（元の 2 行は残した）。
  壊して確かめた: `repair_ident_bytes_with_code_page` を「表せなくても UTF-8 のバイトで Ok」に壊し、windows-gnu で組んで本人の PC で `ch14_unrepresentable` → `FAILED`（console.rs:463 で panicked）。戻した
- R4・R14: `ch14_write_to_console_succeeds_off_windows` を `#[cfg(not(windows))]` に。本人の PC で windows-gnu の試験を `ch14_ --list` → lib 10・bin 1 の計 11（下限 11 とちょうど同じ。どの 1 本が消えても落ちる）。`ch14_` を走らせて 10 passed・1 passed
- R5: 断りの理由を、端末なら `console::write_to_console` で窓へ直に書く（管のときは今どおり標準エラー）。install-check の「標準出力と標準エラーを向け替えても…」の窓の入力を `short short PW2 PW2` にし、窓の文字（改行を除いた続き）に `受け付けない: 短い（16 文字以上）` があることを見る。**install の節は CI の install-windows でだけ走る（手元では `--self-test` と `bash -n` だけ）**
- R6・R7・R9: rejected（理由と再現は code.md の処置）。R9 は本人の PC で `chcp 65001` の `sc.exe query ashiato-nosuch-svc` が英語の ASCII、`chcp 932` で cp932 を書くことを `xxd` で見た。R6 は `whoami` が `<computer>\<user>` を返すことを本人の PC で見た
- R10: conpty-run.ps1 の時計を `Environment.TickCount` から `Stopwatch`（`ElapsedMilliseconds`、64 bit）に。ASCII のまま。本人の PC で `-Command 'set /p a=ask: & call echo got-%a%' -InputFile <hello-line>` → rc=0・窓に `got-hello-line`（入力の行が送られる）、`-Command 'ping -n 30 127.0.0.1 >nul' -TimeoutSec 3` → rc=124・5 秒（期限が効く）。`install-check.sh --probe-console` rc=0
- R11: `ch14_output_tail_reads_console_code_page` は `AllocConsole` が作った窓なら `FreeConsole` で閉じ、そうでなければ元のコード頁に戻す。並走の影響: ほかの `ch14_` はコード頁を指して読み（`decode_with_code_page`）、窓のコード頁を読む試験は lib にこれだけ（`grep -n "decode_child_output\|GetConsoleOutputCP" crates/installer/src/console.rs` の試験側はこの 1 本）。本人の PC で `--test-threads=4` で ok
- R12: main.rs の `ashiato_setup::console::write_to_console` を `console::write_to_console` に（試験の中の `ashiato_setup::console::decode_with_code_page` は bin の `mod tests` なので残した）
- R13: 呼び出しごとの手書きの読みを機械で止める: `crates/installer/clippy.toml` の `disallowed-methods` に `String::from_utf8_lossy`・`String::from_utf8`（理由に `console::decode_child_output`）。console.rs の Windows 以外の読みと、いまの読みと比べる試験だけ `allow`。
  壊して確かめた: main.rs の `whoami` を `String::from_utf8_lossy(&output.stdout)` に戻して `cargo clippy -p ashiato-setup --all-targets --target x86_64-pc-windows-gnu -- -D warnings` → rc=101 `error: use of a disallowed method std::string::String::from_utf8_lossy`・`note: 子の命令の出力は console::decode_child_output で読む（CH14 D8）`。戻した

走らせた検証（`scripts/verify-run`。evidence.jsonl）: 2.2・3.1・3.2・4.2・5.1・5.2・6.1・6.2・8.1 が全部 PASS（8.1 は fmt・workspace の clippy・windows-gnu の clippy・`cargo test -p ashiato-setup`・check_scenarios）。Linux の `cargo test -p ashiato-setup`: lib 88 passed ほか全部 ok。
`python3 scripts/review_triage.py . ch14-installer-password-prompt-mojibake --no-verdict` → `triage: OK`

## settle 第 2 回（答えの後の見直し。2026-11-10）

- R1・R8（Q6 の答え: 値の口は窓のコード頁を先に読む）: `console.rs` の値の口 `decode_child_output` を `decode_value_with_code_page(bytes, 窓のコード頁)` に（65001 なら UTF-8、そうでなければ先にコード頁で `MB_ERR_INVALID_CHARS` を付けて読み、読めないときだけ UTF-8、どちらでもなければ 1 文字ずつの読み）。失敗の理由の末尾 `output_tail_text` は `decode_tail_output`（UTF-8 を先に。いまの `decode_with_code_page`）。`ch14_account_read_in_code_page_932` に `pc\凜子`（`70 63 5c ea a3 8e 71`。UTF-8 としても正しいことを assert）→ `pc\凜子`・repair の行が `凜子` を含み 932 で符号化できる・65001 では UTF-8 で読む、を足した。`ch14_ascii_child_output_is_unchanged` は両方の読みを 932・437・65001 で見る。main.rs の `ch14_downloads_read_in_code_page_932` は値の読みに替え、`D:\凜子` を足した。試験の本数は変えない（本人の PC で `--list` → lib 10・bin 1）。design D8 を書き直した（見出し・読み方・理由・反転条件）
  - 本人の PC（windows-gnu で組んだ exe）: `ch14_` → lib 10 passed・bin 1 passed
  - 壊して確かめた: `decode_value_with_code_page` を常に UTF-8 を先に見る形に壊す → `ch14_account_read_in_code_page_932` FAILED（console.rs:491）・`ch14_downloads_read_in_code_page_932` FAILED（`left: "D:\\꣎q"`）。戻した
  - `cargo clippy -p ashiato-setup --all-targets --target x86_64-pc-windows-gnu -- -D warnings` rc=0
- R3（Q7 の答え: 台本を強める）: `tools/check-ci-ch14-step.sh` が、run を `;`・`&&`・`||`・改行で区切った片のうち `--list` を含まない片に `cargo test -p ashiato-setup ch14_` があることまで見る。いま rc=0、末尾の実行を消す → rc=1、実行を `-- --list` 付きにする → rc=1、戻す → rc=0。tasks.md 6.1 の検証コマンドは変えていない
- `scripts/verify-run` 5.1・5.2・6.1・8.1 → 全部 PASS（evidence.jsonl）

## settle 第 3 回（R15。2026-11-10）

- R15: `crates/installer/src/commands.rs` の `repair_ident_contents` と `crates/installer/src/console.rs` の `repair_ident_bytes_with_code_page` を `pub(crate)` に（理由を doc に書いた）。main.rs から対応表を組める口は `console::repair_ident_bytes` だけになる。試験は変えていない
  - 壊して確かめた（`cargo build -p ashiato-setup --target x86_64-pc-windows-gnu`）: main.rs の書きを CH14 前の `fs::write(&ident, commands::repair_ident_contents(&account))` に戻す → rc=101 `error[E0603]: function repair_ident_contents is private`。`console::repair_ident_bytes_with_code_page(&account, 65001)` に替える → rc=101 `error[E0603]: function repair_ident_bytes_with_code_page is private`。どちらも戻した（`git diff crates/installer/src/main.rs` は空）
  - `scripts/verify-run 8.1` → fmt・workspace の clippy・windows-gnu の clippy・`cargo test -p ashiato-setup`・check_scenarios が全部 PASS（1 回目は fmt が FAIL。`cargo fmt --all` で console.rs の長い行を折って走らせ直した）
  - 本人の PC での windows-gnu の試験は走らせていない（見える範囲を狭めただけで、試験の中身と本数は変えていない）

## settle 第 4 回（final review の R16〜R21。2026-11-10）

- R16: settle は correction-review.md を書けないので、commit されずに残っていた古い記録（強める前の台本で「落ちなかった」）を `.superpowers/sdd/ch14-settle/correction-review.stale-6.1-20261110T095406.md` へ移した。`plan_fix.py pending` → 6.1（at 2026-11-10T09:54:06+00:00）が返る。gate の前のグラフの `provisional_review` が code-verify の手 7 で記録し直す。`plan_fix.py check` はいま rc=1「別の席の確かめが無い」
  - 手で同じ壊し方を確かめた: ci.yml の ch14 の step の末尾 `; cargo test -p ashiato-setup ch14_` を消す → `tools/check-ci-ch14-step.sh` rc=1、戻す → rc=0。`git status --porcelain`・`git diff` は壊す前と同じ
- R17: `run_as_admin` の時間切れを `管理者 $user の実行が終わらない` に、タスク名と .cmd を `ashiato-install-check-admin` に（後片付けの rm も）
- R18: `window_run` は `.vt` を読めないとき `$WORK/$name.ps.out` を標準エラーへ出し、窓の台本の rc を添えて die する
- R19: `repair_ident_contents` の doc に英字でない名前の枝（引用した文字どおりの行・cp932 の 0x5C）を書いた。コードは不変
- R20: ci.yml の「Windows の runner では build.sh が…」を `package.sh windows` の step の直前へ移した
- R21: tasks.md 5.1 に読み順の注を 1 行（検証コマンドと [x] は不変。declared は 5 本のまま）
- 走らせたもの: `tools/release/install-check.sh --self-test` rc=0 / `bash -n tools/release/install-check.sh` rc=0 / `cargo clippy -p ashiato-setup --all-targets --target x86_64-pc-windows-gnu -- -D warnings` rc=0 / `cargo test -p ashiato-setup` rc=0 / `cargo fmt --all -- --check` rc=0 / `tools/check-ci-ch14-step.sh` rc=0 / ci.yml の YAML 読み込み OK / `review_triage.py --no-verdict` → triage: OK
- 本人の PC・Windows の CI では走らせていない（install-check の窓・管理者の節は install-windows でだけ走る）

## settle 第 5 回（final review 2 回目の R22〜R25。2026-11-10）

- R22: design D9 に「残る危険」の 1 行（SSPI の名前は大文字と小文字を区別せずに突き合わされ、文字どおりの行は `pg_strcasecmp` で比べるので、cp932 の 2 バイト目 `0x41`〜`0x5A` が重なる。受け入れる理由・悪用に要るもの・反転条件）。コードは変えない。PostgreSQL の source は走らせて確かめていない（記憶による。SSPI は `check_usermap(..., true)` で大文字と小文字を区別しない）
- R23: `console.rs` の `lossy_decode` は 65001 なら `String::from_utf8_lossy`（関数に理由つきの allow）。`ch14_unreadable_bytes_become_replacement` に 65001 の並び（`指定された` + `0x82`）を 2 つの読みで足した（試験の本数は変えない）
  - 本人の PC（windows-gnu で組んだ exe を `C:\Users\Public` へ写して走らせた）: lib の `ch14_` 10 passed・bin 1 passed
  - 壊して確かめた: 65001 の枝を外す → `ch14_unreadable_bytes_become_replacement` FAILED（console.rs:458、`left: "����������������"` / `right: "指定された�"`）。戻した
- R24: 自己試験の説明の 3 行を `self_test() {` の直前へ移した
- R25: 片方にはしない（tasks 4.2 が窓の節の最初の `console_probe` を決めていて凍結、3.3 の step も検証コマンドが見る）。`(console_probe)` の上に、正本はこちら・CI の step は早く落とすため、とコメント
- 走らせたもの: `scripts/verify-run 8.1` → fmt・workspace の clippy・windows-gnu の clippy・`cargo test -p ashiato-setup`・check_scenarios 全部 PASS / `tools/release/install-check.sh --self-test` rc=0 / `bash -n tools/release/install-check.sh` rc=0 / `review_triage.py --no-verdict` → triage: OK
- Windows の CI の install-windows では走らせていない（R24・R25 はコメントだけ）
