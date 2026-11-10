<!-- 処置の判定: re-review -->
# CH14 code_verify — 独立検証（2026-11-10）

## 申告と実測

申告: tasks の `[x]` は 14/15（9.1 は人間の確認待ちで `[ ]`）。evidence.jsonl の item × command ごとの最後の `status` は PASS 38・FAIL 1（FAIL は 6.1 の旧コマンド 2026-11-10T09:54:01。機械の訂正で `tools/check-ci-ch14-step.sh` に置き換わり、その最後は PASS）。PR はまだ無い（`gh pr list --head feat/ch14-installer-password-prompt-mojibake` → `[]`）。そのため Windows の CI（`package-windows`・`install-windows`）は、この change のコードでまだ 1 回も走っていない。

| 項目 | 検証コマンド（tasks の本文） | 実測（この席で走らせた） | 一致 |
|---|---|---|---|
| 1.1 | `cargo test -p ashiato-setup --lib ch14_`（`--list` で 3 本以上）/ `cargo test -p ashiato-setup --lib` | rc=0・Linux で 5 本 | 一致 |
| 2.1 | clippy（windows-gnu・native）/ `Cargo.lock` に `+name =` が無い / `tools/check-licenses.sh` | 4 つとも rc=0 | 一致 |
| 2.2 | `prompt_password` が無い / `fn read_web_passwords` が 1 つ / clippy / `cargo test -p ashiato-setup` | 4 つとも rc=0 | 一致 |
| 3.1 | `conpty-run.ps1` が ASCII だけ | rc=0 | 一致 |
| 3.2 | `install-check.sh --probe-console`（WSL から本人の PC）/ `--self-test` | rc=0（`OK: 窓の試験は化けを見分けられる（コード頁 932）`）/ rc=0 | 一致 |
| 3.3 | `install-windows` で `--probe-console` が `--list` の step より前 | rc=0 | 一致 |
| 4.1 | `--self-test` / `bash -n` | rc=0 / rc=0 | 一致 |
| 4.2 | `--self-test` / `--list` に 6 つの名前 / check_scenarios に 6 本の担保なしが無い | 3 つとも rc=0 | 一致 |
| 5.1 | `main.rs` に `from_utf8_lossy` が無い / `allow(unsafe_code)` の追加が 1 行 / clippy / lock / `cargo test` | どれも rc=0 | 一致 |
| 5.2 | clippy（windows-gnu）/ 8 つの `fn` がある / check_scenarios | どれも rc=0。**さらに windows-gnu で組んだ試験を本人の PC（Windows・ACP/OEMCP 932）で走らせた: lib 11 本・bin 1 本（`ch14_` の計 12 本）が全部 ok。`ch14_output_tail_reads_console_code_page` は `chcp 437` の疑似コンソールの中でも ok（`Active code page: 437`。試験の `SetConsoleOutputCP(932)` が効いている）** | 一致 |
| 6.1 | `tools/check-ci-ch14-step.sh`（機械が承認した訂正。手 7） | rc=0 | 一致（ただし手 7 で「落ちなかった」→ R3） |
| 6.2 | `--self-test` / `bash -n` / `--list` に名前 / check_scenarios | どれも rc=0 | 一致 |
| 7.1 | `docs/install.md` の 8 行目に `16 文字以上` | rc=0 | 一致 |
| 8.1 | fmt / clippy（workspace・windows-gnu）/ `cargo test -p ashiato-setup` / `openspec validate --strict` / check_scenarios | どれも rc=0（`scenarios: OK`） | 一致 |

捏造・空の試験・無いコマンドは見つからなかった。指摘は下の 4 件。

## 手 1: 該当なし（確かめた範囲: 試験に直書きした固定値の全部）

試験に直書きした cp932 の 16 進（`指定された`・失敗の文の全体・`pc\足跡太郎`・`足跡表ソ`・`ダウンロード`）を python の `str.encode('cp932')` で計算し直し、5 つとも一致した。`é` は cp1252 で `e9`・cp437 で `82`（design D9 の主張どおり）、`足跡` は cp1252 で表せない。陰性の対照の化けの並び（`画面の合言葉: ` の UTF-8 を cp932 で読む）は `逕ｻ髱｢縺ｮ蜷郁ｨ闡…` で、本人が見た並びと同じ。英字だけの対応表の行 `repair /(?i)^owner@pc$ ashiato\n` は、直す前の `repair_ident_contents` の組み方（merge-base の commands.rs:1097-1118）と同じ形。

## 手 2: ガードをわざと壊す

作業ツリーの外に、`ashiato_setup` の lib を使う小さな exe を組み、`tools/release/conpty-run.ps1` で本人の PC の `chcp 932` の窓で走らせて、install-check の確かめ（`assert_window_prompt` の python を抜き出したもの・合言葉の grep・向け替えの 3 つの grep）に通した。本物の `password` の口は聞く前に昇格し直す（UAC）ので走らせていない。

| 壊した形 | 結果 |
|---|---|
| 直した形（条件・問いを `write_to_console`、読みは `read_password`） | `assert_window_prompt` rc=0・窓に合言葉なし |
| 問いを `rpassword::prompt_password` に戻す（直す前） | `assert_window_prompt` rc=1「窓に2つの問いが無い」（窓は `逕ｻ髱｢縺ｮ蜷郁ｨ闡・ `）→ 落ちる |
| 標準入力の行読みにする（打った文字が窓に出る） | 合言葉の grep が窓の文字に `install-check-reset-0123456` を見つける → 落ちる |
| 条件と問いを標準エラーへ書き、`>out 2>err` で走らせる | 「向け替えると問いが窓に出ない」→ 落ちる |

作業ツリーのコードを一時的に壊し、windows-gnu で組んだ `ch14_` の試験を本人の PC で走らせた（毎回控えから戻し、`git status --porcelain`・`git diff` が壊す前に戻ることを確かめた）:

| 壊したもの | 結果 |
|---|---|
| `console.rs:117` の `GetConsoleOutputCP()` を 1252 に固定 | `ch14_output_tail_reads_console_code_page` FAILED → 落ちる |
| `commands.rs:1099` の英字でない名前の分岐を `if false && …` | `ch14_ident_line_is_literal_in_ansi_bytes` FAILED → 落ちる（`ch14_account_read_in_code_page_932` は `contains("足跡太郎")` なので正規表現の行でも通る） |
| `main.rs:1506` の `encode_ansi` の `Err` で止まらず UTF-8 のバイトで書く | **Windows で 12 本・Linux で 5 本、全部 ok** → R2 |

検査の外側: 5.1 の grep は `main.rs` だけを見るので、`grep -rn from_utf8 crates/installer/src`（`console.rs` を除く）を走らせた → 0 件（rc=1）。`main.rs` の外に子の出力を読む口は残っていない。

## 手 3: Scenario と test を突き合わせる

`python3 scripts/check_scenarios.py . ch14-installer-password-prompt-mojibake` → `scenarios: OK`（20 本に印がある）。印の先を 1 件ずつ見た。主張とテストの階層が違うのは R2 の 1 件（Scenario は「repair が止まる」、試験は「符号化が `Err` を返す」）。`コード頁 932 で書いた日本語の利用者名を、同じ文字として読む` は、試験の名前（`足跡太郎`）では成り立つが、名前によって成り立たない（R1）。窓の Scenario 4 本は印の先の確かめが壊した形を落とすことを手 2 で確かめたが、windows-latest の runner ではまだ走っていない（PR が無い）。

## 手 4: 該当なし（確かめた範囲: Q1〜Q5 の答えと、そこから来る値）

- Q2（子の出力を窓のコード頁で読む）: 手 2 の 1252 固定で `ch14_output_tail_reads_console_code_page` が落ちる。ただし読み方 D8（仮）の「UTF-8 として正しければそのまま」は、R1 のとおり実在しうる名前で破れる
- Q3（最初の問いの前に 1 回・断りに条件）: `password.rs` の `MIN_LEN` を一時的に 20 にして `cargo test -p ashiato-setup` → 既存の `tests::st55_password_length_boundary_is_sixteen` が FAILED（rc≠0）。`ch14_` の 3 本は定数から読むので通る（D5 のとおり）。`install-check.sh --self-test` は rc=0（`16 文字以上` の字面は install-check.sh:297・:1045・:1090 にあり、実地の確かめは Windows の CI でだけ走る。ここでは走らせていない）。戻した後 `git diff -- crates` は空。管の道の「1 回だけ・断りより前・表示されません無し」は `assert_requirements_line` で、`--self-test` が 2 回・後ろ・「表示されません」あり・条件なし、の 4 つの外れを落とす
- Q5（`CONOUT$` へ `WriteConsoleW`）: 手 2 の標準エラーの形を向け替えの確かめが落とす
- Q1・Q4: 範囲と確かめる場所の決定で、値として書き換えられる定数は無い

## 手 5: 該当なし（確かめた範囲: `[x]` の 14 項目）

上の表のとおり、`[x]` の 14 項目の検証コマンドは全部実在し、rc=0。`cargo test --test <名前>` 型は無い。5.2 の 8 本は `fn` の実在だけでなく、本人の PC の Windows で走って ok になることまで確かめた。

## 手 6: 隙間

- 読み方 D8 の「UTF-8 として正しければそのまま」を、cp932 の 2 バイト文字（かな・JIS 第 1・第 2 水準の漢字 6,533 字）の 2 文字の並び全部で調べた: 973,305 / 42,680,089（約 2.3%）が UTF-8 としても正しい並びになる。先頭が第 2 水準の漢字（cp932 の 1 バイト目 `e0`〜`ea`）の名前で起きる。実在しうる名前の `凜子`（`ea a3 8e 71`）を本人の PC でインストーラの読み方に通した → R1
- 表せない名前で `repair` が止まるのは、サービスを止め・一時の書きを足し・`server.json.repair` と `repair.pg_hba.conf` を書いた後（main.rs:1458-1506 を読んだ。走らせていない）。DB には触る前なので、ここからは記録の消失を挙げない
- 捨てたものが復元できない経路（記録の消失）は、この change の差分には見つからなかった

## 手 7: 機械が承認した検証コマンドの訂正

`python3 scripts/plan_fix.py pending …` → 6.1 の 1 件（`at` 2026-11-10T09:54:06+00:00、新 `tools/check-ci-ch14-step.sh`）。壊して確かめた結果は `correction-review.md`（落ちなかった）。`python3 scripts/plan_fix.py check openspec/changes/ch14-installer-password-prompt-mojibake` → rc=1、`[FAIL] 6.1 2026-11-10T09:54:06+00:00: 壊しても新しいコマンドが落ちなかった（検証が守っていない）`（gate は「結果が無い」でなく「落ちなかった」として読んでいる）→ R3。

---

## R1. 日本語の名前によっては、子の出力を UTF-8 と取り違えて別の文字で読む（`凜子` → `꣎q`。対応表の符号化が「932 で表せない」で `Err` になる）
- 成果物: crates/installer/src/console.rs:112-125（`decode_child_output`）・:162-168（`decode_with_code_page` の UTF-8 を先に見る分岐）/ crates/installer/src/main.rs:249（`whoami`）・:80（`user_downloads`）・:1505-1506 / design.md D8（仮）
- 根拠: cp932 の 2 文字の並びの総当たり（python）→ 973,305 / 42,680,089 が UTF-8 としても正しい。windows-gnu で組んだ小さな exe を本人の PC（ACP/OEMCP 932）で走らせた（rc=0）:
  - `whoami` の出力 `70 63 5c ea a3 8e 71 0d 0a`（`pc\凜子` + CRLF の cp932）を `console::decode_child_output` → `"pc\\꣎q"`、`console::decode_with_code_page(…, 932)` → `"pc\\꣎q"`。それを `commands::repair_ident_contents` → `"repair \"꣎q@pc\" ashiato\n"`、`console::encode_ansi` → `Err("Windows の文字コード 932 で表せない")`。main.rs:1505-1506 はこの `Err` を `利用者名 … を …` にして `?` で返す形（repair 全体は走らせていない）
  - `reg.exe query` の形の出力（値 `D:\` + `ea a3 8e 71`）を `console::decode_child_output` → 値の行が `REG_EXPAND_SZ    D:\\꣎q`
  design D8 の「2 文字以上の文ではほぼ無い」は外れ、D8 自身の反転条件「UTF-8 として正しい cp932 の出力が実際に見つかったら、(1) と (2) の順を子ごとに決める」に当たる。単体の試験の名前（`足跡太郎`・`ダウンロード`）は UTF-8 として正しくならない並びなので、この場合を見ていない
- kind: premise
- 提案: D8 の反転条件のとおり、窓のコード頁で書く子（`whoami`・`reg.exe`・`sc.exe`・`tasklist` など。premises.md 15 行目）は窓のコード頁が 65001 でなければ先に窓のコード頁で読み、UTF-8 を先に見るのは Rust の子（`ashiato-server migrate`）だけにする。あるいは利用者名・ダウンロードの場所は子の出力を読まず API（`GetUserNameExW`・`SHGetKnownFolderPath` など）から取る。どちらでも `凜子` の並びを試験に足す
- 処置: escalated —— deep.md 第 2 回 Q6（`docs/briefs/CH14-deep-r2.html`）で答えが出た。本人が推奨（値の口は窓のコード頁を先に読む）を選んだ（2026-11-10、`deep-answers-2.txt`）。答えのとおりに入れた: `console::decode_child_output`（main.rs の値の口 `whoami`・`reg.exe`・`sc.exe`・`tasklist`・`netstat`・`icacls`・PowerShell）は、窓のコード頁が 65001 でなければ先にそのコード頁で読み（`decode_value_with_code_page`）、読めないときだけ UTF-8 を見る。UTF-8 を先に見るのは失敗の理由の末尾 `output_tail_text` だけ（`decode_tail_output`）。`pc\凜子`（`70 63 5c ea a3 8e 71`）を `ch14_account_read_in_code_page_932` に、`D:\凜子` を `ch14_downloads_read_in_code_page_932` に足した（試験の本数は変えない。CI の下限 11 のまま）。design D8 を書き直した。値の口を UTF-8 を先に読む形に戻すと本人の PC でどちらも FAILED（`pc\꣎q`・`D:\꣎q`）

## R2. 「Windows の文字コードで表せない利用者名なら、repair は理由を示して止まる」の試験は符号化の関数だけを見ており、repair が止まらなくても全部通る
- 成果物: crates/installer/src/console.rs:418-423（`ch14_unrepresentable_name_is_refused`）/ crates/installer/src/main.rs:1505-1507 / tools/release/install-check.sh（英字でない名前の repair は `é` で、1252 で表せる道だけを通る）
- 根拠: main.rs:1505-1506 を `let ident_bytes = console::encode_ansi(&ident_text).unwrap_or_else(|_| ident_text.into_bytes());`（表せなくても止まらず UTF-8 で書く）に壊して、windows-gnu で組んだ `ch14_` を本人の PC で走らせた → lib 11 passed・bin 1 passed（rc=0）。Linux の `cargo test -p ashiato-setup ch14_` も 5 passed。`grep -n 表せない tools/release/install-check.sh` は 0 件。Scenario の THEN（「行は組まれず、…理由に示して止まる」）は repair の振る舞いだが、試験は `encode_with_code_page("足跡", 1252)` が `Err` に `1252` を含むことしか見ていない（主張は repair の階層、試験は関数の階層）。戻した後 `git status --porcelain`・`git diff` は壊す前と同じ
- kind: technical
- 提案: 対応表の中身を組む部分（`repair_ident_contents` → 符号化 → 理由の文）を、コード頁を引数で受ける 1 つの関数（例 `repair_ident_bytes(account, code_page) -> Result<Vec<u8>, String>`）にして main.rs はそれを `GetACP()` で呼ぶだけにし、`ch14_unrepresentable_name_is_refused` はその関数が 1252 で `Err`（`利用者名 … を Windows の文字コード 1252 で表せない`）を返すことを見る
- 処置: fixed 5.2
- 判定: ADDRESSED（re-review 1 @38bd85827ef801aa65f0e9f0677d03245fbf14fa）
- 判定: ADDRESSED（re-review 2 @b47dcd1144891d0a34faa0d0d2ffc609d9cd450c）
- 判定: ADDRESSED（re-review 3 @7883a8b56223d0d54307e12291921b74080df056）

## R3. 6.1 の訂正後の検証コマンドは、CI の step が試験を 1 本も走らせなくても通る（訂正前のコマンドも同じ）
- 成果物: tools/check-ci-ch14-step.sh / .github/workflows/ci.yml:428-430 / tasks.md 6.1 / plan-corrections.md / correction-review.md
- 根拠: ci.yml:429 の末尾の `; cargo test -p ashiato-setup ch14_` を消し（`-- --list` で数えて下限を見るだけの step）、`tools/check-ci-ch14-step.sh` → rc=0。訂正前のコマンド（tasks.md 6.1 の旧）も同じ形で rc=0。どちらも run の文字列に `ashiato-setup ch14_` と `-ge 11` を含む step が `package.sh windows` より前にあるかしか見ず、`-- --list` の部分の字面で満たされる。訂正は検証を弱めてはいないが、検証は 6.1 の守るもの（`ch14_` の試験を下限つきで走らせる）を守っていない。`plan_fix.py check` → rc=1（6.1 を「落ちなかった」と読む）。戻した後 `git diff` は空
- kind: premise
- 提案: 台本で、step の run が下限の確かめの後に `cargo test -p ashiato-setup ch14_`（`-- --list` の付かない呼び出し）を持つことまで見る。例: run を `;` で区切った片のうち `--list` を含まない片に `cargo test -p ashiato-setup ch14_` があること
- 処置: escalated —— deep.md 第 2 回 Q7 で答えが出た。本人が推奨（台本を強める。検証コマンドの字面は変えない）を選んだ（2026-11-10、`deep-answers-2.txt`）。答えのとおりに `tools/check-ci-ch14-step.sh` を、step の run を `;`・`&&`・`||`・改行で区切った片のうち `--list` を含まない片に `cargo test -p ashiato-setup ch14_` があることまで見るように強めた。tasks.md 6.1 は変えていない。壊して確かめた: ci.yml の末尾の `; cargo test -p ashiato-setup ch14_` を消す → rc=1、それを `-- --list` 付きにする → rc=1、戻す → rc=0（`git diff .github/` は空）

## R4. CI の下限 11 は、Windows で実際に数えると 12 本で、Task 5.2 の試験が 1 本消えても通る
- 成果物: .github/workflows/ci.yml:429 / crates/installer/src/console.rs:298-301（`ch14_write_to_console_succeeds_off_windows`）
- 根拠: windows-gnu で組んだ試験を本人の PC で `ch14_ --list` → lib と bin の合計 `: test$` が 12（lib 11 = Task 1 の 3・Task 5.2 の 7・`ch14_write_to_console_succeeds_off_windows`。bin 1 = `ch14_downloads_read_in_code_page_932`）。`…_off_windows` は名前に反して Windows でも組まれて数えられるので、下限 11（Task 1 の 3 + Task 5.2 の 8）は Task 5.2 の 8 本のうち 1 本が消えても満たされる。Scenario の印が消えれば `check_scenarios.py` が落とすので、効かないのは印を残して試験の本体だけ消えた場合
- kind: technical
- 提案: 下限を 12 にするか、`ch14_write_to_console_succeeds_off_windows` を `#[cfg(not(windows))]` にして名前と組まれる場所を合わせる
- 処置: fixed 6.1
- 判定: ADDRESSED（re-review 1 @38bd85827ef801aa65f0e9f0677d03245fbf14fa）
- 判定: ADDRESSED（re-review 2 @b47dcd1144891d0a34faa0d0d2ffc609d9cd450c）
- 判定: ADDRESSED（re-review 3 @7883a8b56223d0d54307e12291921b74080df056）

## security review（dae5668..281892e）

見た経路:
- 窓での合言葉の入力: `read_web_passwords` → `ask_hidden`（crates/installer/src/main.rs:285-289）→ 問いの文は `console::write_to_console`（`CONOUT$` へ `WriteConsoleW`。console.rs:14-22, 254-296）、読みは `rpassword::read_password()`。読みは変更前の `prompt_password` と同じ `read_password_with_config`（rpassword-7.5.4 src/lib.rs:259-268, 271-293）で、入力は `CONIN$`（src/windows.rs:15）・既定は `PasswordFeedback::Hide`（src/config.rs:125-128）。書きの口を替えただけで、読みと表示の消し方は変わらない（2）
- 合言葉が窓・向け替えた先に出ないこと: install-check.sh の「窓で打った合言葉は、窓に出ない」「標準出力と標準エラーを向け替えても…」が VT の流れ・窓の文字・向け替えた 2 つのファイルに `$PW2`・API の合言葉・ログインの印の鍵が無いことを見る（tools/release/install-check.sh の該当 Scenario。CI の Windows で走る。手元では走らせていない）（2）
- 条件の一覧: `password::requirements_line`（password.rs:30-39）は定数（`MIN_LEN`・`PLACEHOLDER`）だけから組み、合言葉・API の合言葉・鍵の値を含まない。断りの理由 `PasswordRefusal::reason` も同じ（password.rs:16-26）（3）
- repair の利用者規則: `whoami`（main.rs:240-250。`decode_child_output` で読む）→ `repair_ident_contents`（commands.rs:1097-1108。英字でない名前は `repair "<user>@<domain>" ashiato` の引用した文字どおりの行）→ `console::encode_ansi`（表せなければ `Err` で止まる）→ `fs::write(&ident, …)`（main.rs:1505-1509）→ 一時の PostgreSQL が読む（4）
- 子の命令の出力の読み（reg / sc / whoami / icacls / netstat / tasklist / wmic）: `decode_child_output` は文字列にするだけで、使い道（`reg_query_value`・`sddl_from_sdshow`・`service_state_code`・`owner_sid_from_acl`）とエラー文への載せ方は変更前と同じ。新しく外へ出す値は無い（3）
- 待ち受け・接続先: 差分に新しい listen は無い（repair の `127.0.0.1:0` は既存）。外への送信先も足していない（2）
- 依存: `windows-sys 0.61`（Microsoft の公式 crate。Cargo.lock には既に 0.61.2 があり、installer の依存に足しただけ）、`cfg(windows)` だけで機能は Foundation / Globalization / Storage_FileSystem / System_Console の 4 つ（crates/installer/Cargo.toml:19-20）（2）
- 試験の台本: 新しい管理者 `ashiato-ci-é` の合言葉は都度 `secrets.token_urlsafe` で作り（install-check.sh:123 付近）、後始末で `net user … /delete` する（install-check.sh:625）（2）

安全の決め手: 英字でない名前の対応表の行は、PostgreSQL が「引用した 1 つの名前」として読み、正規表現（`/` で始まる形）としても注釈（`#`）としても読まない。Windows の利用者名と computer 名は `"` を持てないので、引用は閉じられない —— 確かさ 4（`"` を持てない点は Windows の命名規則に拠り 2）。
台本: `/tmp/ch14sec`（repo の外の使い捨ての crate）から本物の `ashiato_setup::commands::repair_ident_contents` と `console::encode_ansi` を呼び、`pc\足跡 #太郎`・`pc\足跡(1).$`・`pc\^足跡`・`pc\足跡'x`・`足跡`・`PC-1\Tarō` の行を組んで、使い捨ての `postgres:17-alpine` の `pg_ident.conf` に置き `pg_ident_file_mappings` を読んだ。出力:
```
1|repair|足跡 #太郎@pc|ashiato|
2|repair|足跡(1).$@pc|ashiato|
3|repair|^足跡@pc|ashiato|
4|repair|足跡'x@pc|ashiato|
5|repair|足跡|ashiato|
6|repair|Tarō@PC-1|ashiato|
```
6 行とも `error` は空で、`sys_name` は名前そのもの（`#` 以降も切れず、`^` `$` `(` `.` は文字のまま）。英字の名前のときは変更前の `/(?i)^…$` の行のまま（console.rs の `ch14_ident_line_is_literal_in_ansi_bytes` が見る）。正規表現（大文字小文字を区別しない）から文字どおり（区別する）に替わったので、合う名前の範囲は広がらず狭まる。

指摘なし

## /code-review（dae5668..2507852）

見た: Claude Code の組み込みの `/code-review`（10 件）。写しは scripts/code_review_record.py

## R5. On a terminal, the prompt and the requirements line are written straight to the window, but the refusal reason still goes through terminal_line(..., true), i.e. stderr; when stderr is redirected, the window never shows why a password was refused.
- 成果物: crates/installer/src/main.rs:324
- 根拠: Run `ashiato-setup.exe password 2>err.txt` in a window and type a 10-character password. The refusal '合言葉を受け付けない: 短い（16 文字以上）…' is written to err.txt, and the window just shows '画面の合言葉: ' again with no reason. The 'redirected stdout/stderr' scenario added in this change only checks the prompt, so this is not caught.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。settle が直し方を決める）
- 処置: fixed 2.2
- 判定: ADDRESSED（re-review 1 @38bd85827ef801aa65f0e9f0677d03245fbf14fa）

## R6. The new non-ASCII branch writes an exact quoted name, which drops the ASCII path's `(@.*)?` realm tolerance for an account with no domain, and its case-insensitive matching depends entirely on PostgreSQL's behaviour.
- 成果物: crates/installer/src/commands.rs:1099
- 根拠: account = '足跡' (no backslash) produces `repair "足跡" ashiato`. SSPI with include_realm=1 sends '足跡@PC', so the map does not match and repair fails authentication. The ASCII form `/(?i)^owner(@.*)?$` would have matched. A domain that differs in case or form from whoami's output (e.g. FQDN vs NetBIOS) also has no regex fallback.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。settle が直し方を決める）
- 処置: rejected: 名前に `\` が無い場合は起きない。`account` は `whoami(&[])` の出力（`crates/installer/src/main.rs:1466`）で、`whoami`（引数なし）は常に `<ドメインか computer 名>\<利用者>` を返す（本人の PC で `/mnt/c/Windows/System32/cmd.exe /d /c whoami` → `<computer>\<user>` の形で、利用者の前に `5c` が 1 つある）。ドメインのある英字の行も `/(?i)^owner@pc$` で `(@.*)?` の許しは持たない（`crates/installer/src/commands.rs:1125`）ので、FQDN と NetBIOS の違いへの許しは英字の行にも無く、英字でない枝で狭まったものは無い。大文字と小文字は、文字どおりの行を PostgreSQL が `token_matches_insensitive`（`pg_strcasecmp`。ASCII の範囲で区別しない）で比べ、ドメイン（英字）の大文字と小文字の違いは英字の行と同じく合う（security review の台本の `pg_ident_file_mappings` の 6 行が文字どおりに読まれた）
- 判定: ADDRESSED（re-review 1 @38bd85827ef801aa65f0e9f0677d03245fbf14fa）
- 判定: ADDRESSED（re-review 2 @b47dcd1144891d0a34faa0d0d2ffc609d9cd450c）
- 判定: ADDRESSED（re-review 3 @7883a8b56223d0d54307e12291921b74080df056）

## R7. The non-ASCII admin used in the real-machine check is 'ashiato-ci-é', which is single-byte in cp1252, so the check never covers the cp932 case D9 is written for: double-byte names whose second byte is 0x5C. On a Japanese host (ACP 932), 'é' cannot be encoded at all, so the scenario fails.
- 成果物: tools/release/install-check.sh:305
- 根拠: On the windows-latest runner (ACP 1252), the regex-breaking 0x5C second byte (e.g. 表/ソ) never appears in the ident file, and the scenario passes even if the cp932 path is broken. On a ja-JP machine, encode_ansi refuses 'é' under 932 ('Windows の文字コード 932 で表せない'), so the scenario dies.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。settle が直し方を決める）
- 処置: rejected: 英字でない名前の管理者に `é` を使い、日本語の名前（cp932 の 2 バイト目が `0x5C`）を単体の試験で確かめるのは design D9 の決定（runner の ANSI のコード頁は 1252 で `chcp` でも `Set-Culture` でも替わらず、日本語の名前は表せない。review/spec.md R25）。0x5C の場合は `ch14_ident_line_is_literal_in_ansi_bytes`（`crates/installer/src/console.rs`。`足跡表ソ` のバイト列に `0x5c` があることを見る）が担保し、この settle で本人の PC（ACP 932）で ok を確かめた。install-check の install の節を走らせるのは CI の `install-windows`（windows-latest。`.github/workflows/ci.yml:459-460`）だけで、日本語の Windows で走らせる道は無い（手元の検証は `--self-test`・`--probe-console` だけで、どちらも利用者を作らない）
- 判定: ADDRESSED（re-review 1 @38bd85827ef801aa65f0e9f0677d03245fbf14fa）
- 判定: ADDRESSED（re-review 3 @7883a8b56223d0d54307e12291921b74080df056）

## R8. decode_child_output treats any byte sequence that happens to be valid UTF-8 as UTF-8, so cp932 output that is also valid UTF-8 is silently misread with no U+FFFD to signal the problem.
- 成果物: crates/installer/src/console.rs:190
- 根拠: A whoami/reg/sc output line in cp932 that happens to form valid UTF-8 multibyte sequences (e.g. lead E0–EF followed by 80–BF trail bytes) is returned as the wrong characters. That garbles the account name passed to repair_ident_contents or the Downloads path, and the failure only shows up later as an auth or path error.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。settle が直し方を決める）
- 処置: escalated —— R1 と同じ不具合。deep.md 第 2 回 Q6 の答え（値の口は窓のコード頁を先に読む）を R1 と一緒に入れた（console.rs の `decode_value_with_code_page`。UTF-8 として正しい cp932 の並び `凜子` を試験に足した。詳しくは R1 の処置）

## R9. Child output is decoded with the installer's own console output code page. If the user's window is set to `chcp 65001`, output from children that write in the OEM code page (cp932) is decoded as UTF-8 and every non-ASCII character becomes U+FFFD.
- 成果物: crates/installer/src/console.rs:194
- 根拠: In a window after `chcp 65001`, a child that writes its error in OEM cp932 (e.g. sc.exe) is decoded with code page 65001, so the reason reaches the user as '���…'. This is the same kind of garbled text this change set out to fix.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。settle が直し方を決める）
- 処置: rejected: 前提が実物と違う。本人の PC（OEMCP・ACP 932）で `/mnt/c/Windows/System32/cmd.exe /d /c "chcp 65001 >nul & sc.exe query ashiato-nosuch-svc" | xxd` → `[SC] EnumQueryServicesStatus:OpenService FAILED 1060:` の後は `The specified service does not exist as an installed service.`（英語の ASCII）。同じ命令を `chcp 932` で走らせると `8e 77 92 e8 82 b3 …`（cp932 の『指定されたサービスは…』）。子は OEM のコード頁でなく引き継いだ窓のコード頁に従って書き、65001 の窓では日本語の cp932 を書かない（`premises.md` の「子の命令…は、引き継いだ窓のコード頁に従って管へ書く」の行と同じ。確かさ 5）。65001 で U+FFFD になる並びは出ない
- 判定: ADDRESSED（re-review 1 @38bd85827ef801aa65f0e9f0677d03245fbf14fa）
- 判定: ADDRESSED（re-review 2 @b47dcd1144891d0a34faa0d0d2ffc609d9cd450c）

## R10. The deadline and quiet checks use Environment.TickCount (a signed 32-bit int that wraps after about 24.9 days of uptime), so if it wraps mid-run, the timeout never fires and no input lines are sent.
- 成果物: tools/release/conpty-run.ps1:174
- 根拠: On a host with long uptime, TickCount wraps from +2^31 to -2^31 during the run. `TickCount > deadline` stays false forever and `TickCount - last >= 1000` never becomes true, so no password lines are sent and the CI step hangs until the job-level timeout. Environment.TickCount64 or Stopwatch avoids this.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。settle が直し方を決める）
- 処置: fixed 3.1
- 判定: ADDRESSED（re-review 1 @38bd85827ef801aa65f0e9f0677d03245fbf14fa）

## R11. ch14_output_tail_reads_console_code_page calls AllocConsole and changes the process-wide console output CP during a parallel test run, and it leaves the allocated console in place, so other tests in the same binary that read the console CP can be affected.
- 成果物: crates/installer/src/console.rs:412
- 根拠: During cargo test, another test runs decode_child_output or output_tail_text at the same moment and is decoded with 932 instead of the expected CP, so it fails intermittently. On a developer machine, AllocConsole also opens an extra console window.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。settle が直し方を決める）
- 処置: fixed 5.2
- 判定: ADDRESSED（re-review 1 @38bd85827ef801aa65f0e9f0677d03245fbf14fa）
- 判定: ADDRESSED（re-review 2 @b47dcd1144891d0a34faa0d0d2ffc609d9cd450c）

## R12. main.rs already imports `console`, but ask_hidden and read_web_passwords call it by the full path `ashiato_setup::console::write_to_console`, so the same module is referenced two ways in one file.
- 成果物: crates/installer/src/main.rs:618
- 根拠: The `console::` short form at the other call sites (decode_child_output and others) and the full path here are mixed, making the code harder to grep and maintain. `console::write_to_console` is enough.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。settle が直し方を決める）
- 処置: fixed 2.2
- 判定: ADDRESSED（re-review 1 @38bd85827ef801aa65f0e9f0677d03245fbf14fa）

## R13. The pattern `Command::new(..).output()` → `console::decode_child_output(&output.stdout)` is repeated by hand at about 8 call sites; a single helper that runs a child and returns decoded text would fix the root at one point.
- 成果物: crates/installer/src/main.rs:581
- 根拠: Any new child-process call that forgets decode_child_output and goes back to String::from_utf8_lossy reintroduces cp932 garbling. Nothing in the code enforces the decoding, so this recurs.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。settle が直し方を決める）
- 処置: fixed 5.1
- 判定: ADDRESSED（re-review 1 @38bd85827ef801aa65f0e9f0677d03245fbf14fa）

## R14. The test is named ch14_write_to_console_succeeds_off_windows but has no cfg, so it also runs on Windows; it is counted towards CI's 11-test minimum without testing what its name says.
- 成果物: crates/installer/src/console.rs:376
- 根拠: On the Windows runner it writes an empty string to CONOUT$ (or falls back to stderr), which can never fail, yet it is counted towards the 'ch14_ ≥ 11' minimum. If a real test is removed, this vacuous test makes the count gate weaker.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。settle が直し方を決める）
- 処置: fixed 6.1
- 判定: ADDRESSED（re-review 1 @38bd85827ef801aa65f0e9f0677d03245fbf14fa）
- 判定: ADDRESSED（re-review 2 @b47dcd1144891d0a34faa0d0d2ffc609d9cd450c）

## settle_review（re-review 1 @38bd858）: 処置の直しが新しく壊したもの

## R15. ch14_unrepresentable_name_is_refused は repair_ident_bytes_with_code_page を直に呼ぶ。main.rs が repair_ident_bytes を通らない形に戻っても、試験は通る。
- 成果物: crates/installer/src/console.rs
- 根拠: crates/installer/src/console.rs:524（re-review 1 の New Breakage。Minor）
- kind: technical
- 処置: fixed 5.1 —— 試験を足すのでなく、main.rs が `repair_ident_bytes` を通らずに対応表を組めないようにした。`commands::repair_ident_contents` と `console::repair_ident_bytes_with_code_page` を `pub(crate)` にし、bin（main.rs）から見える口を `console::repair_ident_bytes`（`GetACP()`）だけにした。試験はどちらも lib の中なので直に呼べる。壊して確かめた: main.rs を CH14 の前の形 `fs::write(&ident, commands::repair_ident_contents(&account))` に戻す → windows-gnu の build が `error[E0603]: function repair_ident_contents is private`、`console::repair_ident_bytes_with_code_page(&account, 65001)` に替える → `error[E0603]: function repair_ident_bytes_with_code_page is private`。戻した
- 判定: ADDRESSED（re-review 2 @b47dcd1144891d0a34faa0d0d2ffc609d9cd450c）

## final review（dae5668..ce544aa）

見た: SDD の review package（`.superpowers/sdd/tasks/review-dae5668bd..ce544aad9.diff`）を `requesting-code-review` の `code-reviewer.md` で。R1〜R15 と Task 1〜6 の findings の Minor・⚠️ を突き合わせた。Critical なし。HEAD で確かめたもの（Linux）: `cargo test -p ashiato-setup` lib 88 passed・windows-gnu の clippy rc=0・`install-check.sh --self-test` rc=0・`check_scenarios.py` OK・`check-shared-appends.sh` rc=0・`check-ci-ch14-step.sh` rc=0（ci.yml の写しから末尾の `; cargo test -p ashiato-setup ch14_` を消すと rc=1）。R1・R3・R8 は判定の行が無いが、HEAD で直っている（console.rs:180-191 の読み順と `凜子` の 2 本・R3 の台本）。
Windows の CI（`package-windows` の `ch14_` の step・`install-windows` の `--probe-console`・窓の Scenario 4 本・`ashiato-ci-é` の repair）は、この change のコードでまだ 1 度も走っていない。merge gate が PR の CI の緑を見るので R にしない。runner は窓 437・ANSI 1252 で、引数なしの `whoami` が管へどちらのコード頁で書くかは本人の PC（どちらも 932）では見分けられない —— CI で落ちたらまずそこを疑う（D9 の反転条件）。

## R16. 6.1 の訂正の確かめ（手 7）の記録が強める前の台本のままで、`plan_fix.py check` が rc=1 のまま（しかも記録のファイルが commit されていない）
- 成果物: openspec/changes/ch14-installer-password-prompt-mojibake/correction-review.md / openspec/changes/ch14-installer-password-prompt-mojibake/tasks.md 6.1 / tools/check-ci-ch14-step.sh
- 根拠: HEAD で `python3 scripts/plan_fix.py check openspec/changes/ch14-installer-password-prompt-mojibake` → rc=1・`[FAIL] 6.1 2026-11-10T09:54:06+00:00: 壊しても新しいコマンドが落ちなかった（検証が守っていない）`。この「落ちなかった」は R3 の前の台本に対するもので、Q7 の答えで強めた後は同じ壊し方で rc=1 になる（この席で実測）。`correction-review.md` は `git status` で `??`（untracked）で、きれいな checkout では結果が無くなる。`.harness2/docs/flow-gates.md` の merge gate はこの訂正を通さない
- kind: technical
- 提案: 6.1 の訂正の確かめ（手 7）を HEAD の `tools/check-ci-ch14-step.sh` でやり直し、「落ちた」の結果を `correction-review.md` に記録して commit する（`plan_fix.py check` が rc=0 になること）。コードは変えない
- 処置: fixed 6.1 —— 記録はこの席が書かない（settle は correction-review.md を書けない。書くのは code-verify の手 7）。commit されずに残っていた古い記録（強める前の台本で「落ちなかった」。その結果は R3・deep.md 第 2 回 Q7 として処置済み）を `.superpowers/sdd/ch14-settle/correction-review.stale-6.1-20261110T095406.md` へ移して commit した。これで `plan_fix.py pending` が 6.1（at 2026-11-10T09:54:06+00:00）を返し、gate の前にグラフの `provisional_review` が code-verify の手 7 をいまの台本で起こし直して記録を書き、commit する（plan_fix で <at> を取り直す道は、旧＝いまの台本が PASS するので無い）。この席でも同じ壊し方を手で確かめた: ci.yml の step の末尾 `; cargo test -p ashiato-setup ch14_` を消す → `tools/check-ci-ch14-step.sh` rc=1、戻す → rc=0（`git status --porcelain`・`git diff` は壊す前と同じ）。いまの `plan_fix.py check` の FAIL は「別の席の確かめが無い」で、provisional_review の後に消える
- 判定: ADDRESSED（re-review 3 @7883a8b56223d0d54307e12291921b74080df056）

## R17. 汎用にした `run_as_admin` の時間切れの文言とタスク名が「管理者 B」のまま
- 成果物: tools/release/install-check.sh:579・590
- 根拠: `run_as_admin` は `$1` の管理者で走らせる関数になったが、`name='ashiato-install-check-b'`・`script='…-b.cmd'` と `die '管理者 B の実行が終わらない'` が残る（Task 6 の findings F1）。CI で最も落ちそうな新しい経路（`ashiato-ci-é` の repair）が時間切れになると、どの管理者で止まったかを取り違える
- kind: technical
- 提案: `die "管理者 $user の実行が終わらない"` にする。タスク名は呼び出しごとに替えるか、名前の意味を doc に書く
- 処置: fixed 6.2 —— 時間切れを `die "管理者 $user の実行が終わらない"` に、タスクと .cmd の名前を `ashiato-install-check-admin`（どの管理者でも同じ 1 つの名前。呼び出しごとに作って消すことを doc に書いた）にし、後片付けの `rm` の名前も合わせた。`install-check.sh --self-test` rc=0・`bash -n` rc=0
- 判定: ADDRESSED（re-review 3 @7883a8b56223d0d54307e12291921b74080df056）

## R18. 窓の台本が `.vt` を残さず落ちたとき、`window_run` は台本の出力（`.ps.out`）を出さずに止まる
- 成果物: tools/release/install-check.sh:307-316（`window_run`）
- 根拠: `conpty-run.ps1` の出力（`hr=`・`err=` など）は `$WORK/<名前>.ps.out` に向けられ、`.vt` が無いと `die "窓の文字を読めない: $name"` だけで終わる。窓の Scenario はまだ windows-latest で 1 度も走っていない（Task 3 / 4 の ⚠️）ので、初めて落ちたときの手がかりが CI のログに残らない
- kind: technical
- 提案: die の前に `$WORK/$name.ps.out` を標準エラーへ出す（台本は入力の行を出さないので秘密は載らない。載らないことを既存の秘密の検査に通すとなおよい —— Task 4 の F3）
- 処置: fixed 3.1 —— `.vt` を読めないとき、die の前に `$WORK/$name.ps.out`（窓の台本の `hr=`・`err=` など）を標準エラーへ出し、die の文に窓の台本の rc を添えた。conpty-run.ps1 は入力の行を標準出力へ出さない（入力は窓へ送るだけ、窓の文字は `-Out` のファイルへ）ので合言葉は載らない。`--self-test` rc=0・`bash -n` rc=0（窓の節は CI の install-windows でだけ走る）
- 判定: ADDRESSED（re-review 3 @7883a8b56223d0d54307e12291921b74080df056）

## R19. `repair_ident_contents` の doc が、英字でない名前の枝（引用した文字どおりの行）を書いていない
- 成果物: crates/installer/src/commands.rs:1094-1100
- 根拠: doc は「大文字と小文字は…区別しない正規表現にし、英数字以外の文字は ARE の符号の書き方にして」とだけ言うが、直後の `if !account.is_ascii()` の枝は `repair "<user>@<domain>" ashiato` の引用した行を組む（CH14 D9。Task 5 の findings F1）。doc を読んだ人は英字でない名前も正規表現になると取り違える
- kind: technical
- 提案: doc に「英字でない名前は、ANSI のコード頁で符号化する前提で引用した文字どおりの行にする（D9。正規表現にすると cp932 の 2 バイト目の `0x5C` が式を壊す）」の 1 文を足す
- 処置: fixed 5.1 —— doc に「英字だけの名前は…正規表現」と限り、「英字でない名前は正規表現にせず、引用符でくくった文字どおりの行（`repair "<user>@<domain>" ashiato`）にする。ANSI のコード頁で符号化する前提で、正規表現にすると cp932 の 2 バイト目の `0x5C` が式を壊す（CH14 D9）」を足した。コードは変えていない。windows-gnu の clippy rc=0
- 判定: ADDRESSED（re-review 3 @7883a8b56223d0d54307e12291921b74080df056）

## R20. ci.yml の「Windows の runner では build.sh が…」のコメントが、新しい ch14 の step の上に来ている
- 成果物: .github/workflows/ci.yml:426
- 根拠: このコメントは `tools/release/package.sh windows` の step の説明だが、CH14 が step をその間に差し込んだので ch14 の試験の step の上に並ぶ（Task 6 の findings F2）
- kind: technical
- 提案: コメントを `package.sh windows` の step の直前へ移す
- 処置: fixed 6.1 —— コメントを `tools/release/package.sh windows "$RUNNER_TEMP/package-windows"` の step の直前へ移した（ch14 の step の上は CH14 のコメントだけ）。YAML として読める・`tools/check-ci-ch14-step.sh` rc=0
- 判定: ADDRESSED（re-review 3 @7883a8b56223d0d54307e12291921b74080df056）

## R21. tasks.md 5.1 の本文が Q6 より前の読み順（「UTF-8 として正しければそのまま」）のまま
- 成果物: openspec/changes/ch14-installer-password-prompt-mojibake/tasks.md:76
- 根拠: deep.md 第 2 回 Q6 の答えで、値の口（`decode_child_output`）は窓のコード頁を先に読む形に替わり、design D8 も書き直された（R1 の処置）。tasks.md 5.1 は `decode_child_output`・`decode_with_code_page` とも「UTF-8 として正しければそのまま」と書いたままで、実装（console.rs の `decode_value_with_code_page` / `decode_tail_output`）と計画が食い違う。逸脱は正当で記録もあるが、PR を読む人が計画の文を正と取る
- kind: technical
- 提案: 5.1 の本文に「（Q6 の答えで、値の口は窓のコード頁を先に読む形に替えた。design D8・review/code.md R1）」の注を足すか、本文を今の読み順に直す（`[x]` と検証コマンドは変えない）
- 処置: fixed 5.1 —— 5.1 の本文の後に注を 1 行足した（Q6 の答えで値の口 decode_child_output はコード頁を先に読む形に替えた・UTF-8 を先に見るのは失敗の理由の末尾だけ・design D8・R1）。本文の検証コマンドと `[x]` は変えていない（evidence.declared の 5.1 は 5 本のまま）
- 判定: ADDRESSED（re-review 3 @7883a8b56223d0d54307e12291921b74080df056）

## final review 2（dae5668..2379363）

見た: SDD の review package（`.superpowers/sdd/tasks/review-dae5668bd..23793634f.diff`）を `requesting-code-review` の `code-reviewer.md` で。R1〜R21 と Task 1〜6 の findings の Minor・⚠️ を突き合わせた。Critical・Important なし。7883a8b56（R16〜R21 の処置）が新しく壊したものは無い（タスク名 `ashiato-install-check-admin` は作る口と後始末の `rm` で揃う・`.ps.out` に入力の行は載らない・doc / コメント / 5.1 の注は実装と合う）。HEAD で確かめたもの（Linux / WSL）: `cargo test -p ashiato-setup` lib 88・bin 4 passed、clippy（native・windows-gnu）rc=0、`install-check.sh --self-test`・`check-ci-ch14-step.sh`・`check-shared-appends.sh` rc=0、`check_scenarios.py` OK、`--probe-console`（本人の PC）rc=0（38bd858 の Stopwatch の C# が組めて動く）。Windows で走る `ch14_` は 11 本（password 3・console の windows_tests 7・main 1）で CI の下限 `-ge 11` と合う。`plan_fix.py check` の rc=1 は R16 の処置どおり provisional_review 待ち。
Windows の CI でしか確かめられないもの（窓の Scenario 4 本の引用符・`ashiato-ci-é` を `net user` / `schtasks /RU` で作れるか・その repair）は未確認のまま。merge gate が PR の CI の緑を見るので R にしない。
R にしない細かい残り（Task の findings のまま。merge を止めない）: Task 2 F2（途中まで書いて失敗すると問いが二重に出る）・Task 1 F1・Task 5 F4 / F5・Task 3 F3 / F4・Task 6 F3。

## R22. 英字でない名前の文字どおりの行は、PostgreSQL の大文字小文字を区別しない比べ方で、別の日本語の名前にも当たりうる
- 成果物: openspec/changes/ch14-installer-password-prompt-mojibake/design.md（D9）/ crates/installer/src/commands.rs:1104-1112
- 根拠: SSPI の正規表現でない行は `pg_strcasecmp`（1 バイトずつ `A`〜`Z` を小文字にする）で比べられる。cp932 の 2 バイト目は 0x41〜0x5A を取りうるので、`ア`（83 41）と `ヂ`（83 61）、`イ`（83 43）と `ツ`（83 63）が同じになる。管理者 `pc\アキ` の repair の行は `pc\ヂキ` にも当たる。R6 の処置はドメイン（英字）の大文字小文字だけを論じ、security review の「合う名前の範囲は広がらず狭まる」はこの場合に当たらない。確かさは中（PG17 の `hba.c` の `check_ident_usermap` と `src/port/pgstrcasecmp.c` を読んだ記憶による。走らせてはいない）。悪用には似た名前のローカルの利用者を先に持ち、repair の一時の DB（127.0.0.1 の空き番号・短い間）に繋ぐことが要る
- kind: technical
- 提案: 正規表現に戻すと D9 の 0x5C に戻るのでコードは変えない。design D9 に「残る危険（cp932 の 2 バイト目の大文字小文字の重なり）と受け入れる理由」を 1 行足す

## R23. 窓のコード頁が 65001 のとき、`lossy_decode` は 1 バイトずつ読み、UTF-8 として正しくない所が 1 つでもあると正しい多バイト文字まで全部 U+FFFD にする
- 成果物: crates/installer/src/console.rs:250-270（`lossy_decode`）
- 根拠: `IsDBCSLeadByteEx(65001, …)` は常に偽なので width は常に 1 になり、多バイトの UTF-8 の文字は 1 バイトでは `strict_decode` を通らない。`String::from_utf8_lossy` より悪い（Task 5 findings F3。HEAD でも残る）。表示の口だけで、起きることは少ない
- kind: technical
- 提案: `decode_with_code_page`（または `lossy_decode`）で 65001 なら `String::from_utf8_lossy` に任せる（その行だけ clippy の `disallowed-methods` を理由つきで allow する）

## R24. `self_test` の説明のコメントが `console_probe` の上に載っている
- 成果物: tools/release/install-check.sh:1454-1457
- 根拠: 「Linux で走る自己試験。本物の Windows は無いので…」は `self_test` の説明だが、CH14 が `console_probe` をその間に差し込んだので `console_probe` の上に並ぶ（Task 3 findings F5。R20 と同じ種類のずれ）
- kind: technical
- 提案: コメントを `self_test()` の直前へ移す

## R25. install-windows で `console_probe` が 2 回走る
- 成果物: tools/release/install-check.sh:1067-1068 / .github/workflows/ci.yml（install-windows の `--probe-console` の step）
- 根拠: 独立の step（`install-check.sh --probe-console`。Task 3.3）と、`check_release_zip` の中の `(console_probe)` の両方で走る。害は無いが、どちらが陰性の対照の正本か読む人が迷い、CI の時間が延びる
- kind: technical
- 提案: 片方にする（3.3 の検証コマンドが step の順を見るので、残すのは独立の step の側。`check_release_zip` の中の呼び出しを消すか、残す理由をコメントに書く）
