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

## R2. 「Windows の文字コードで表せない利用者名なら、repair は理由を示して止まる」の試験は符号化の関数だけを見ており、repair が止まらなくても全部通る
- 成果物: crates/installer/src/console.rs:418-423（`ch14_unrepresentable_name_is_refused`）/ crates/installer/src/main.rs:1505-1507 / tools/release/install-check.sh（英字でない名前の repair は `é` で、1252 で表せる道だけを通る）
- 根拠: main.rs:1505-1506 を `let ident_bytes = console::encode_ansi(&ident_text).unwrap_or_else(|_| ident_text.into_bytes());`（表せなくても止まらず UTF-8 で書く）に壊して、windows-gnu で組んだ `ch14_` を本人の PC で走らせた → lib 11 passed・bin 1 passed（rc=0）。Linux の `cargo test -p ashiato-setup ch14_` も 5 passed。`grep -n 表せない tools/release/install-check.sh` は 0 件。Scenario の THEN（「行は組まれず、…理由に示して止まる」）は repair の振る舞いだが、試験は `encode_with_code_page("足跡", 1252)` が `Err` に `1252` を含むことしか見ていない（主張は repair の階層、試験は関数の階層）。戻した後 `git status --porcelain`・`git diff` は壊す前と同じ
- kind: technical
- 提案: 対応表の中身を組む部分（`repair_ident_contents` → 符号化 → 理由の文）を、コード頁を引数で受ける 1 つの関数（例 `repair_ident_bytes(account, code_page) -> Result<Vec<u8>, String>`）にして main.rs はそれを `GetACP()` で呼ぶだけにし、`ch14_unrepresentable_name_is_refused` はその関数が 1252 で `Err`（`利用者名 … を Windows の文字コード 1252 で表せない`）を返すことを見る

## R3. 6.1 の訂正後の検証コマンドは、CI の step が試験を 1 本も走らせなくても通る（訂正前のコマンドも同じ）
- 成果物: tools/check-ci-ch14-step.sh / .github/workflows/ci.yml:428-430 / tasks.md 6.1 / plan-corrections.md / correction-review.md
- 根拠: ci.yml:429 の末尾の `; cargo test -p ashiato-setup ch14_` を消し（`-- --list` で数えて下限を見るだけの step）、`tools/check-ci-ch14-step.sh` → rc=0。訂正前のコマンド（tasks.md 6.1 の旧）も同じ形で rc=0。どちらも run の文字列に `ashiato-setup ch14_` と `-ge 11` を含む step が `package.sh windows` より前にあるかしか見ず、`-- --list` の部分の字面で満たされる。訂正は検証を弱めてはいないが、検証は 6.1 の守るもの（`ch14_` の試験を下限つきで走らせる）を守っていない。`plan_fix.py check` → rc=1（6.1 を「落ちなかった」と読む）。戻した後 `git diff` は空
- kind: premise
- 提案: 台本で、step の run が下限の確かめの後に `cargo test -p ashiato-setup ch14_`（`-- --list` の付かない呼び出し）を持つことまで見る。例: run を `;` で区切った片のうち `--list` を含まない片に `cargo test -p ashiato-setup ch14_` があること

## R4. CI の下限 11 は、Windows で実際に数えると 12 本で、Task 5.2 の試験が 1 本消えても通る
- 成果物: .github/workflows/ci.yml:429 / crates/installer/src/console.rs:298-301（`ch14_write_to_console_succeeds_off_windows`）
- 根拠: windows-gnu で組んだ試験を本人の PC で `ch14_ --list` → lib と bin の合計 `: test$` が 12（lib 11 = Task 1 の 3・Task 5.2 の 7・`ch14_write_to_console_succeeds_off_windows`。bin 1 = `ch14_downloads_read_in_code_page_932`）。`…_off_windows` は名前に反して Windows でも組まれて数えられるので、下限 11（Task 1 の 3 + Task 5.2 の 8）は Task 5.2 の 8 本のうち 1 本が消えても満たされる。Scenario の印が消えれば `check_scenarios.py` が落とすので、効かないのは印を残して試験の本体だけ消えた場合
- kind: technical
- 提案: 下限を 12 にするか、`ch14_write_to_console_succeeds_off_windows` を `#[cfg(not(windows))]` にして名前と組まれる場所を合わせる

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
