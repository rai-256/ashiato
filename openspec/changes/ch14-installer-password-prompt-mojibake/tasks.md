# CH14 実装タスク — インストーラの合言葉の問いが文字化けする

読む順: `deep.md`（**本人の答え Q1〜Q5**。第 1 回は推奨で先に書いてある）→ このファイル → `specs/distribution/spec.md` → `design.md` → `premises.md` → `review/deep.md` → `docs/changes/CH14.md` → `CLAUDE.md`。

検証は各タスクの本文に書いてある。「動いた」ではなくコマンドと終了コードで判定する。Task 1・2・3.1・3.3・4・5・6・7 は Linux（WSL）だけで終わる（Task 5 の Windows でだけ組む試験は PR の CI の `package-windows` が走らせる）。Task 3.2 は Windows の PowerShell を使う（WSL から本人の PC の `powershell.exe`、CI は windows-latest）。
本物の Windows でインストールして窓の Scenario を通すのは PR の CI の `install-windows`（merge gate が CI の緑を見る）と、末尾の「人間の確認待ち」。

## Global Constraints

- **テストには `Scenario: <名前>` の印を置く。** install-check は bash なので `echo '== Scenario: <名前>'` を、その場合を確かめる直前に出す。`scripts/check_scenarios.py` が spec の全 Scenario と突き合わせ、印の無い Scenario を FAIL にする。印の名前は spec の `#### Scenario:` と**一字一句合わせる**
- **この change の Scenario は 20 本**（`distribution` の MODIFIED 1 つ（今の 5 本を逐語で写し、6 本を足した）と、ADDED 1 つ（子の命令の出力の読み。9 本。Task 5・6））。写した 5 本の印は `tools/release/install-check.sh` に既にある（**消さない・名前を変えない**）。MODIFIED で足した 6 本は Task 4 が、ADDED の 1 本（repair）は Task 6.2 が install-check に印を置き、`scenario_names`（`--list`）に名前を足す。ADDED のほかの 8 本は Task 5.2 の Rust の試験の `// Scenario:` が印で、**`scenario_names` に足さない**（足すと `install-windows` の `--list` と出力の突き合わせが「印が無い」で落ちる）。CI の `install-windows` は `--list` と出力の `== Scenario:` を突き合わせるので、名前を足して印を出さなければ落ちる
- **本人の決定（下流は変えない）**: Q1 範囲は依頼どおりの 2 つ（D1）/ Q2 子の命令の出力の化けは、表示も値の口もこの change で直す（D2。読み方は D8（仮）、確かめ方は D9（仮））/ Q3 最初の問いの前に一覧、断りに条件を添える（D3）/ Q4 Windows の CI と確認バッチで 1 回（D6・D7）/ Q5 問いは窓へ `WriteConsoleW` で直に書く（D4）
  （第 1 回は推奨で先に書いた。推奨と違った答えは `spec --answers` が写す）
- **合言葉の検査の中身は変えない**（`password::validate` の 5 つの理由と順番。16 文字・`change-me`・API の合言葉・ログインの印の鍵・2 回の一致）。**断りの行の形 `合言葉を受け付けない: <理由>。入れ直してください` を変えない**（install-check が `受け付けない: 短い` などの字面で確かめる）
- **合言葉を聞く関数は `read_web_passwords` の 1 つのまま**（3 つの口が同じ関数を通ることが、窓の試験を `password` の口だけで済ませる根拠。design Risks）
- **端末でないとき（管）の読み方を変えない**（標準入力の 2 行。問いの文は出さない）
- **`unsafe` は `crates/installer/src/console.rs` だけ**（窓へ書く D4 と、子の出力を窓のコード頁で読む D8 の両方。先頭に `#![cfg_attr(windows, allow(unsafe_code))]`。各 `unsafe` の塊に `// SAFETY:`）。ほかのファイルに `allow(unsafe_code)` を足さない
- **新しい依存は `windows-sys`（`cfg(windows)`・版 0.61）だけ**（features は Task 2.1 の 3 つと、Task 5.1 が足す `Win32_Globalization`）。`encoding_rs` などの文字の表の crate を足さない（D9）。rpassword は外さない・版を変えない
- **秘密（合言葉）を、端末の出力・試験の期待値の出力・命令の引数に出さない**。窓の試験も、打った合言葉は台本の入力のファイルで渡し、窓の出力からは消えていることを確かめる
- **`tools/release/conpty-run.ps1` の source は ASCII だけ**（Windows PowerShell 5.1 は BOM の無い UTF-8 を読み違える）。日本語の期待値は install-check（bash）に置く
- **install-check.sh の末尾に書き足さない**（窓の節は `password` の節の途中。design D6）。新しい節の見出しに ST / CH の番号を付けない（`tools/check-shared-appends.sh`）
- **子の命令の出力は `console::decode_child_output` だけで読む**（D8）。`crates/installer/src/main.rs` に `from_utf8_lossy` を残さない。英字だけの出力の値は変えない（`docs/changes/CH14.md` の「壊してはいけないもの」7）
- **長いコマンドは `scripts/quiet-run`**（cargo test）。試験の名前の接頭辞は `ch14_`。インストーラの単体の試験は `crates/installer/src/` の `#[cfg(test)]`

## Task 1: 条件の文言と断りの理由を、検査の定数から組む

- [x] 1.1 `crates/installer/src/password.rs` に `pub fn requirements_line(terminal: bool) -> String` を足す（design D3・D5。文言は D3 のとおり、`MIN_LEN`・`PLACEHOLDER` から `format!` で組む。`terminal` が偽なら「入れた文字は表示されません」を付けない）。
  `PasswordRefusal::reason` を `String` を返す形に替え、`TooShort` → `短い（{MIN_LEN} 文字以上）`・`Placeholder` → `雛形（{PLACEHOLDER} で始めない）`、ほかの 3 つは今の語のまま。呼び出し側（`main.rs` の断りの行）を合わせる。
  同じモジュールの `#[cfg(test)]` に次の 3 本を足す: `ch14_requirements_line_names_every_condition`（一覧に `MIN_LEN` の数・`PLACEHOLDER`・`API の合言葉`・`ログインの印の鍵` がある）/
  `ch14_requirements_line_mentions_hidden_only_on_terminal`（`true` のときだけ「表示されません」がある）/ `ch14_refusal_reason_keeps_word_and_adds_condition`（`TooShort` の理由が `短い` で始まり `MIN_LEN` の数を含む・`Placeholder` が `雛形` で始まり `PLACEHOLDER` を含む・ほかの 3 つは今の語と同じ）。
  検証: `bash -c 'n=$(cargo test -p ashiato-setup --lib ch14_ -- --list 2>/dev/null | grep -c ": test$"); [ "$n" -ge 3 ] && cargo test -p ashiato-setup --lib ch14_'` rc=0（`-- --list` で 3 本以上）/ `bash -c 'cargo test -p ashiato-setup --lib'` rc=0

## Task 2: 問いの文を窓へ直に書き、rpassword は読みだけにする

- [x] 2.1 `crates/installer/src/console.rs` を design D4 のとおりに書き（`pub fn write_to_console(text: &str) -> std::io::Result<()>`。`cfg(windows)` は `CONOUT$` を `CreateFileW` で開いて `WriteConsoleW`、失敗したら標準エラーへ書いて続ける。`cfg(not(windows))` は標準エラーへ書いて `flush`）、`lib.rs` に `pub mod console;` を足す。
  `crates/installer/Cargo.toml` の `[target.'cfg(windows)'.dependencies]` に `windows-sys = { version = "0.61", features = ["Win32_Foundation", "Win32_Storage_FileSystem", "Win32_System_Console"] }` を足す（`Cargo.lock` に新しい crate の項が増えないこと）。
  検証: `cargo clippy -p ashiato-setup --all-targets --target x86_64-pc-windows-gnu -- -D warnings` rc=0 / `cargo clippy -p ashiato-setup --all-targets -- -D warnings` rc=0 / `bash -c '! git diff origin/main -- Cargo.lock | grep -E "^\+name = "'` rc=0 / `tools/check-licenses.sh` rc=0
- [x] 2.2 `crates/installer/src/main.rs` の `read_web_passwords` を design D3・D4 のとおりに直す: loop の前に条件の一覧を 1 回（端末なら `console::write_to_console`、端末でなければ `terminal_line(…, false)`）。端末のとき、問いの文を `console::write_to_console("画面の合言葉: ")`・`("画面の合言葉（もう一度）: ")` で書いてから `rpassword::read_password()` で読む。
  `rpassword::prompt_password` を呼ばない。端末でないときの 2 行の読みと、断りの行の形は変えない。
  検証: `bash -c '! grep -rn "prompt_password" crates/installer/src'` rc=0 / `bash -c 'grep -c "fn read_web_passwords" crates/installer/src/main.rs | grep -qx 1'` rc=0 / `cargo clippy -p ashiato-setup --all-targets --target x86_64-pc-windows-gnu -- -D warnings` rc=0 / `bash -c 'cargo test -p ashiato-setup'` rc=0

## Task 3: 疑似コンソールで子を起こし、窓に出た文字を書き出す台本

- [x] 3.1 `tools/release/conpty-run.ps1` を design D6 のとおりに書く（source は ASCII だけ。冒頭のコメントに何をするか・なぜ要るか（CH14）・引数）。引数: `-Command <cmd.exe /d /c に渡す文字列>`・`-InputFile <1 行ずつ送る文字列のファイル>`（無くてよい）・`-Out <窓に出た文字を UTF-8 で書くファイル>`・`-TimeoutSec`（既定 120。install-check からは 4.2 のとおり長く渡す）。下敷きは `openspec/changes/ch14-installer-password-prompt-mojibake/premise-probe/probe.ps1`（本人の PC で動いた形）。
  `Add-Type` の C# で `CreatePseudoConsole`（120×40）・`CreateProcess`（`EXTENDED_STARTUPINFO_PRESENT` と `PROC_THREAD_ATTRIBUTE_PSEUDOCONSOLE`。**`STARTF_USESTDHANDLES` を立てて 3 つの std handle を空にし**、`bInheritHandles = false`、作業の場所は `C:\`。立てないと子は親の管を継ぐ —— `premises.md` 9 行目）を使い、出力の管を読み続ける。入力は、出力が 1 秒静まってから 1 行ずつ `\r` を付けて送る。子が終わったら子の終了コードで終わり、上限を超えたら子を止めて rc=124。
  `-Probe` を持たせる（陰性の対照。design D6）: `chcp 932` の窓で子を 2 つ順に起こし、出力を `-Out` の隣に `.raw.txt`・`.console.txt` で書く。どちらの子も最初に、`[Console]::IsInputRedirected` と `IsOutputRedirected` が偽なら `CONSOLE-OK` を窓に出す。
  `.raw.txt` の子は『画面の合言葉: 』（文字は `[char]0x753B` などで組む。source を ASCII に保つ）の UTF-8 のバイトを `[Console]::OpenStandardOutput()` に書く（`WriteFile` の経路。`[IO.File]::Open('CONOUT$', …)` は 5.1 では断られる）。`.console.txt` の子は同じ文を `[Console]::Out` に書く。
  検証: `bash -c 'LC_ALL=C grep -qP "[^\x00-\x7F]" tools/release/conpty-run.ps1 && exit 1 || exit 0'` rc=0
- [x] 3.2 `tools/release/install-check.sh` に `--probe-console` の口と、関数 `console_probe` を足す（`usage` の行も直す。口の分岐は末尾の分岐の並びの中、`--check-coverage` の前）。台本は `powershell.exe -NoProfile -NonInteractive -ExecutionPolicy Bypass -File <台本>` で走らせる。
  `console_probe` は 3.1 の `-Probe` を走らせ、VT の制御の並び（`ESC [ … 文字`・`ESC ] … BEL`）を除いた後に、次を確かめる（外れたら `die '窓の試験が化けを見分けられない: <どれ>'`）: 両方に `CONSOLE-OK` がある / `.console.txt` に『画面の合言葉』がある / `.raw.txt` に『画面の合言葉』が無い / `.raw.txt` に化けた並びがある
  （期待値は『画面の合言葉』の UTF-8 のバイトを、`.raw.txt` の窓の `chcp` が返したコード頁で読んだ文字列。python の `bytes.decode("cp<番号>", errors="replace")` で作り、置換の文字 `U+FFFD` の無い最初の 3 文字以上の並びを探す）。
  走らせる `powershell.exe` は、Windows の bash（CI）では `powershell.exe`、WSL では `/mnt/c/Windows/System32/WindowsPowerShell/v1.0/powershell.exe`（パスは `wslpath -w` で渡す）。どちらも無ければ rc=2 で `Windows の PowerShell が無い` を出す。
  検証: `tools/release/install-check.sh --probe-console` rc=0（WSL から本人の PC の PowerShell で。作業箱の中では rc=2 になる —— 外で走らせる）/ `tools/release/install-check.sh --self-test` rc=0
- [x] 3.3 `.github/workflows/ci.yml` の `install-windows` の、インストールの確かめの step（`tools/release/install-check.sh --list | sort -u …` で始まる `run:`）の前に、step を 1 つ足す: `shell: bash`・`run: tools/release/install-check.sh --probe-console`（design D6「runner で先に確かめる」。ほかの job と step は変えない）。
  検証: `bash -c 'python3 -c "import yaml,sys; j=yaml.safe_load(open(\".github/workflows/ci.yml\",encoding=\"utf-8\"))[\"jobs\"][\"install-windows\"][\"steps\"]; r=[s.get(\"run\",\"\") for s in j]; i=[n for n,x in enumerate(r) if \"--probe-console\" in x]; k=[n for n,x in enumerate(r) if \"install-check.sh --list\" in x]; sys.exit(0 if i and k and i[0]<k[0] else 1)"'` rc=0

## Task 4: install-check に 6 つの Scenario を足す

- [x] 4.1 管の道の確かめ。次の 2 つの印を、それぞれを確かめる直前に出す:
  `echo '== Scenario: 端末でないときも、断りより前に合言葉の条件を 1 回だけ示す'` —— 3 つの出力で確かめる: インストールの最後（`$WORK/install.out`）・`password` の口（今の「短い・既定の雛形・一致しない合言葉は受け付けない」の `printf … | "$SETUP" password >"$out" 2>&1` の出力）・`repair` の口（`install_with "$PW1" repair >"$out"` の直後に、別のファイルへ写して）。
  どれも、`16 文字以上`・`change-me で始めない`・`API の合言葉`・`ログインの印の鍵` を全部含む行がちょうど 1 つ・`受け付けない:` の行があればその最初の行より前・`表示されません` が無い。確かめは関数 1 つ（`assert_requirements_line <ファイル> <どの口>`）にし、印は password の口の確かめの直前に 1 回だけ出す（install と repair の出力は控えておき、そこでまとめて確かめる。repair の節が後なら repair の確かめは repair の節で関数を呼ぶだけにして、印は 1 回だけ）/
  `echo '== Scenario: 短い・雛形で断るときは、理由に条件を添える'`（password の口の出力に `受け付けない: 短い（16 文字以上）` と `受け付けない: 雛形（change-me で始めない）` がある）。今の確かめ（`受け付けない: 短い` などの grep）は変えない。
  `scenario_names` に 2 つの名前を足す（配列の途中。末尾に足さない）。
  検証: `tools/release/install-check.sh --self-test` rc=0 / `bash -n tools/release/install-check.sh` rc=0
- [x] 4.2 `echo '== Scenario: API の合言葉やログインの印の鍵と同じ値は受け付けない'` の前に、窓の節を置く（design D6）。最初に `console_probe`（3.2）を走らせ、次の 4 つを確かめる。それぞれの前に印を出す:
  `echo '== Scenario: コード頁が UTF-8 でない窓でも、合言葉の問いと条件が読める'`（`conpty-run.ps1 -TimeoutSec "${ASHIATO_INSTALL_CHECK_SETUP_TIMEOUT:-600}" -Command 'chcp 932 >nul & chcp & "<SETUP>" password'`、入力は `$PW2` を 2 行。窓の 2 回の起動はどちらも同じ上限を渡す（`password` の口は起こし直しで最大 180 秒待つ。design D6）。`chcp` の出力のコード頁が 65001 でないこと・VT を除いた窓の文字に `画面の合言葉: `・`画面の合言葉（もう一度）: ` があること・`16 文字以上`・`change-me で始めない`・`API の合言葉`・`ログインの印の鍵`・`表示されません` を全部含む行があり、その位置が最初の `画面の合言葉: ` より前であること）/
  `echo '== Scenario: 窓で打った合言葉で決め直せる'`（同じ回の rc=0・`wait_healthz 150` の後に `$PW2` でログインできる）/
  `echo '== Scenario: 窓で打った合言葉は、窓に出ない'`（同じ回の窓の文字に `$PW2` が無い）/
  `echo '== Scenario: 標準出力と標準エラーを向け替えても、合言葉の問いは窓に出る'`（`-Command 'chcp 932 >nul & "<SETUP>" password >"<out.txt>" 2>"<err.txt>"'`、入力は `$PW2` を 2 行。窓の文字に `画面の合言葉: ` と `16 文字以上` を含む行があり、`out.txt`・`err.txt` に `画面の合言葉: ` が無い・rc=0・`wait_healthz 150`）。
  一時のファイルは `$WORK` の下（`/tmp` の固定名に書かない）。窓の出力のファイルも `assert_no_secret_in` に通す（API の合言葉・ログインの印の鍵）。
  `scenario_names` に 4 つの名前を足す。
  検証: `tools/release/install-check.sh --self-test` rc=0 / `bash -c 'L=$(tools/release/install-check.sh --list); for n in "端末でないときも、断りより前に合言葉の条件を 1 回だけ示す" "短い・雛形で断るときは、理由に条件を添える" "コード頁が UTF-8 でない窓でも、合言葉の問いと条件が読める" "窓で打った合言葉で決め直せる" "窓で打った合言葉は、窓に出ない" "標準出力と標準エラーを向け替えても、合言葉の問いは窓に出る"; do grep -Fxq -- "$n" <<<"$L" || { echo "--list に無い: $n"; exit 1; }; done'` rc=0 / `bash -c 'o=$(python3 scripts/check_scenarios.py . ch14-installer-password-prompt-mojibake 2>&1); grep -q "^scenarios: " <<<"$o" || { echo "check_scenarios.py が要約を出さない"; exit 1; }; for n in "端末でないときも、断りより前に合言葉の条件を 1 回だけ示す" "短い・雛形で断るときは、理由に条件を添える" "コード頁が UTF-8 でない窓でも、合言葉の問いと条件が読める" "窓で打った合言葉で決め直せる" "窓で打った合言葉は、窓に出ない" "標準出力と標準エラーを向け替えても、合言葉の問いは窓に出る"; do grep -F "担保の無い Scenario: $n" <<<"$o" && exit 1; done; exit 0'` rc=0（この Task の 6 本が担保なしの一覧に無い。ADDED のほかの印は後の Task が置くので、ここでは全体の rc を見ない）

## Task 5: 子の命令の出力を、窓のコード頁で読む

- [ ] 5.1 `crates/installer/src/console.rs` に design D8 のとおり `pub fn decode_child_output(bytes: &[u8]) -> String`（UTF-8 として正しければそのまま、そうでなければ `cfg(windows)` で `GetConsoleOutputCP()`（0 なら `GetOEMCP()`）のコード頁で読む。`cfg(not(windows))` は `from_utf8_lossy`）と、`cfg(windows)` の `pub fn decode_with_code_page(bytes: &[u8], code_page: u32) -> String`（UTF-8 として正しければそのまま、そうでなければ `MultiByteToWideChar(code_page, MB_ERR_INVALID_CHARS, …)`。失敗したら `IsDBCSLeadByteEx` で 1 文字ずつの長さを決めて同じ旗で読み、読めない文字だけを U+FFFD にする —— design D8。旗を付けない既定の読みは使わない）を足す。`Cargo.toml` の `windows-sys` の features に `Win32_Globalization` を足す。
  `crates/installer/src/main.rs` の子の出力の読み（`reg.exe`・`output_tail`・`sc.exe sdshow`・`sc.exe query`・`whoami`・`netstat`・`tasklist`・PowerShell の空き容量・`Get-Acl`。design Context の 9 か所）を全部 `console::decode_child_output` に替える。`output_tail` は、出力の末尾を組む中身（空行を除いた末尾 5 行を ` / ` で繋ぐ）を、**lib の `console.rs` の `pub fn output_tail_text(stdout: &[u8], stderr: &[u8]) -> String`**（読みは `decode_child_output`）へ移し、main.rs の `output_tail` はそれを呼ぶだけにする。中身は変えない（試験が `unsafe` の要る窓のコード頁の切り替えと同じモジュールで呼べるように。review/spec.md R20）。`repair` の DB の認証の対応表（`repair.pg_ident.conf`。`main.rs:1490`）は design D9 のとおりに書く: `console.rs` の `cfg(windows)` の `pub fn encode_with_code_page(text: &str, code_page: u32) -> Result<Vec<u8>, String>`（`WideCharToMultiByte(code_page, WC_NO_BEST_FIT_CHARS, …)`。表せない文字があれば `Err`）と、`GetACP()` で呼ぶ `pub fn encode_ansi`（`cfg(not(windows))` は UTF-8 のバイト）。`commands.rs` の `repair_ident_contents` は、英字でない文字を含む名前なら引用符でくくった文字どおりの行（`repair "<利用者>@<ドメイン>" ashiato`）を、英字だけなら今の正規表現の行を返す。書くときは `encode_ansi` を通し、`Err` なら `利用者名 <名前> を Windows の文字コード <番号> で表せない` を出して止まる。英字だけの名前の行のバイト列は変えない。`user_downloads` も、`reg.exe` の出力の文字列からダウンロードの場所を組む部分（`reg_query_value` → `expand_downloads_shell_folder` → 無ければ `<profile>\Downloads`）を、読みと分けた関数にする。
  検証: `bash -c '! grep -n "from_utf8_lossy" crates/installer/src/main.rs'` rc=0 / `bash -c 'git diff origin/main -- crates/installer/src | grep -E "^\+.*allow\(unsafe_code\)" | grep -v "^+++" | wc -l | grep -qx 1'` rc=0（`allow(unsafe_code)` を足すのは `console.rs` の 1 行だけ）/ `cargo clippy -p ashiato-setup --all-targets --target x86_64-pc-windows-gnu -- -D warnings` rc=0 / `bash -c '! git diff origin/main -- Cargo.lock | grep -E "^\+name = "'` rc=0 / `bash -c 'cargo test -p ashiato-setup'` rc=0
- [ ] 5.2 `#[cfg(all(test, windows))]` の試験を 8 本足す（design D9）。置き場は、呼ぶ関数のあるモジュール（`console.rs` か `main.rs` の `mod tests`。窓のコード頁を切り替える試験は `unsafe` が要るので `console.rs` に置く。main.rs の試験は bin の target なので、数えるときは `--lib` を付けない —— Task 6.1）。期待するバイト列は試験の中に 16 進で持ち、`python3 -c "print('<文>'.encode('cp932').hex(' '))"` で作る（Linux の Python は cp932 の表を持つ。本人の PC の子が cp932 で書くことは `premises.md`）。どれも試験の直前に `// Scenario: <名前>` を置く:
  `ch14_output_tail_reads_console_code_page`（`// Scenario: 窓のコード頁で書いた子の命令の失敗の理由が、化けずに読める`。**窓のコード頁を選ぶ部分を通す**: 試験の中で窓が無ければ `AllocConsole`、`SetConsoleOutputCP(932)` にしてから（終わりに元のコード頁へ戻す）、cp932 の「指定されたサービスはインストールされたサービスとして存在しません。」のバイト列を `console::output_tail_text` で組んだ出力の末尾が、その文を含み `U+FFFD` を含まない。`console.rs` に置く）/
  `ch14_unreadable_bytes_become_replacement`（`// Scenario: どのコード頁でも読めない並びは、置換の文字にして止まらない`。cp932 の「指定された」のバイト列の後ろに `0x82` を足した並びを `decode_with_code_page(…, 932)` で読むと、`指定された` で始まり `U+FFFD` で終わる文字列が返る）/
  `ch14_utf8_child_output_is_kept_on_932`（`// Scenario: UTF-8 で書く子の出力は、窓のコード頁が 932 でもそのまま読める`。日本語を含む UTF-8 の行を `decode_with_code_page(…, 932)` で読んでも同じ文字列）/
  `ch14_ascii_child_output_is_unchanged`（`// Scenario: 英字だけの子の出力は、いまと同じ値として読む`。英字だけの `whoami`（`pc\owner`）・`reg.exe query` の出力・`sc.exe query` の出力を、コード頁 932 と 437 で読んだ値が `String::from_utf8_lossy` の値と同じ）/
  `ch14_account_read_in_code_page_932`（`// Scenario: コード頁 932 で書いた日本語の利用者名を、同じ文字として読む`。cp932 の `pc\足跡太郎` を 932 で読むと `pc\足跡太郎`・それから組んだ `repair_ident_contents` の行が `足跡太郎` を含む）/
  `ch14_downloads_read_in_code_page_932`（`// Scenario: コード頁 932 で書いた日本語のダウンロードの場所を、同じ文字として読む`。cp932 の `reg.exe query` の出力（値 `%USERPROFILE%\ダウンロード`）を 932 で読んで組んだ場所が `<profile>\ダウンロード`）。
  `ch14_ident_line_is_literal_in_ansi_bytes`（`// Scenario: 英字でない利用者名の対応表の行は、Windows の文字コードのバイト列で、文字どおりに書く`。`pc\足跡表ソ` から組んだ行を `encode_with_code_page(…, 932)` で符号化したバイト列が、`repair "` ＋ `'足跡表ソ@pc'.encode('cp932')` ＋ `" ashiato\n` と同じ（`表`・`ソ` の 2 バイト目 `5c` を含み、正規表現の記号 `/(?i)` を含まない）。英字だけの `pc\owner` の行は今の正規表現の行と同じバイト列）/
  `ch14_unrepresentable_name_is_refused`（`// Scenario: Windows の文字コードで表せない利用者名なら、repair は理由を示して止まる`。`足跡` を `encode_with_code_page(…, 1252)` で符号化すると `Err` で、理由に `1252` を含む）。
  Linux では組まれない（`cfg(windows)`）。走らせるのは Task 6.1 の CI の step。
  検証: `cargo clippy -p ashiato-setup --all-targets --target x86_64-pc-windows-gnu -- -D warnings` rc=0（Windows の試験も組み立てる）/ `bash -c 'for n in ch14_output_tail_reads_console_code_page ch14_unreadable_bytes_become_replacement ch14_utf8_child_output_is_kept_on_932 ch14_ascii_child_output_is_unchanged ch14_account_read_in_code_page_932 ch14_downloads_read_in_code_page_932 ch14_ident_line_is_literal_in_ansi_bytes ch14_unrepresentable_name_is_refused; do grep -rq "fn $n" crates/installer/src || { echo "無い: $n"; exit 1; }; done'` rc=0 / `bash -c 'o=$(python3 scripts/check_scenarios.py . ch14-installer-password-prompt-mojibake 2>&1); grep -q "^scenarios: " <<<"$o" || { echo "check_scenarios.py が要約を出さない"; exit 1; }; for n in "窓のコード頁で書いた子の命令の失敗の理由が、化けずに読める" "どのコード頁でも読めない並びは、置換の文字にして止まらない" "UTF-8 で書く子の出力は、窓のコード頁が 932 でもそのまま読める" "英字だけの子の出力は、いまと同じ値として読む" "コード頁 932 で書いた日本語の利用者名を、同じ文字として読む" "コード頁 932 で書いた日本語のダウンロードの場所を、同じ文字として読む" "英字でない利用者名の対応表の行は、Windows の文字コードのバイト列で、文字どおりに書く" "Windows の文字コードで表せない利用者名なら、repair は理由を示して止まる"; do grep -F "担保の無い Scenario: $n" <<<"$o" && exit 1; done; exit 0'` rc=0（この Task の 8 本が担保なしの一覧に無い。repair の 1 本は Task 6.2 が置く）

## Task 6: 読み方を Windows の CI で確かめる

- [ ] 6.1 `.github/workflows/ci.yml` の `package-windows` の、最初の `tools/release/package.sh windows` の step の前に、step を 1 つ足す: `shell: bash`・`run: n=$(cargo test -p ashiato-setup ch14_ -- --list | grep -c ": test$"); [ "$n" -ge 11 ] || { echo "ch14_ の試験が $n 本しかない"; exit 1; }; cargo test -p ashiato-setup ch14_`（`--lib` を付けない —— main.rs（bin）の試験も数える。Task 1 の 3 本と Task 5.2 の 8 本。フィルタ `ch14_` でほかの試験は巻き込まない。design D9。ほかの job と step は変えない）。
  検証: `bash -c 'python3 -c "import yaml,sys; j=yaml.safe_load(open(\".github/workflows/ci.yml\",encoding=\"utf-8\"))[\"jobs\"][\"package-windows\"][\"steps\"]; r=[s.get(\"run\",\"\") for s in j]; i=[n for n,x in enumerate(r) if \"cargo test -p ashiato-setup ch14_\" in x and \"-ge 11\" in x]; k=[n for n,x in enumerate(r) if \"package.sh windows\" in x]; sys.exit(0 if i and k and i[0]<k[0] else 1)"'` rc=0
- [ ] 6.2 `tools/release/install-check.sh` に、英字でない名前の管理者の `repair` を足す（design D9）。`make_users` で 3 人目の管理者（変数 `ADMIN_NONASCII='ashiato-ci-é'`、合言葉は今の 2 人と同じ形で 14 文字以内）を作って Administrators に入れ、`remove_users` で消す。`run_as_admin_b` を、走らせる利用者と合言葉を引数で受ける形にするか、同じ形の関数をもう 1 つ置く（末尾に足さない。`chcp` は替えない —— design D9）。
  `echo '== Scenario: 別の管理者が repair しても、本人は変わらない'` の節の直後に、`echo '== Scenario: 英字でない名前の管理者でも repair でき、決めた合言葉でログインできる'` を出し、設定を退けて（`with_settings_write mv "$SETTINGS" …`）その管理者で `repair` を起こし（入力は `$PW1` を 2 行。今の別の管理者の repair と同じファイルで渡す）、rc=0・`wait_healthz 150`・`$PW1` でログインできる、を確かめる（あわせて DB の oid と `core.source` の件数が repair の前と同じことも見る。今の repair の Scenario と同じ確かめ）。
  外れたら、`.out` と、残っていれば `$STATE/settings/repair.pg_ident.conf`（利用者名の行）（`xxd` で）と `$STATE/settings/repair-db.log` の末尾を出してから `die`（読み違え（D8）か、対応表の書き違えか、PostgreSQL の認証（反転条件 (b)）かを CI の出力から見分ける。どちらのファイルも合言葉を持たない）。
  `scenario_names` に名前を足す（配列の途中。末尾に足さない）。
  検証: `tools/release/install-check.sh --self-test` rc=0 / `bash -n tools/release/install-check.sh` rc=0 / `bash -c 'tools/release/install-check.sh --list | grep -Fxq "英字でない名前の管理者でも repair でき、決めた合言葉でログインできる"'` rc=0 / `python3 scripts/check_scenarios.py . ch14-installer-password-prompt-mojibake` rc=0

## Task 7: 文書

- [ ] 7.1 `docs/install.md:8` の「画面の合言葉を 2 回入力して決めます。」の後に、条件（16 文字以上・`change-me` で始めない）と、入れた文字は表示されないことを書き足す（行を直すだけ。末尾に足さない）。
  検証: `bash -c 'sed -n 8p docs/install.md | grep -q "16 文字以上"'` rc=0

## Task 8: 全体の確かめ

- [ ] 8.1 作業場全体の形と、この change の検査を通す。
  検証: `cargo fmt --all --check` rc=0 / `cargo clippy --workspace --all-targets -- -D warnings` rc=0 / `cargo clippy -p ashiato-setup --all-targets --target x86_64-pc-windows-gnu -- -D warnings` rc=0 / `bash -c 'cargo test -p ashiato-setup'` rc=0 / `openspec validate ch14-installer-password-prompt-mojibake --strict` rc=0 / `python3 scripts/check_scenarios.py . ch14-installer-password-prompt-mojibake` rc=0

## 人間の確認待ち（確認バッチ。PR の CI が緑になった後）

Scenario ではない —— 窓の Scenario 4 本は Windows の CI（疑似コンソール）が見る。ここは CI では見られない、日本語の Windows の昇格して開き直した窓での確かめ（design D7）。

- [ ] 9.1 **日本語の Windows の窓で問いと条件が読める**: 本人の PC（コード頁 932）で、管理者でない端末から `ashiato-setup.exe password` を開き、管理者の確認を通して開き直した窓で、条件の一覧と「画面の合言葉: 」「画面の合言葉（もう一度）: 」が化けずに読めること・打った文字が出ないこと・15 文字を入れると「短い（16 文字以上）」と断られることを見る（決め直す合言葉は本人が決める）
  > 物理: device —— CI の runner は既に管理者で、昇格して開き直す新しい窓を開かず、窓のコード頁も日本語の Windows の 932 とは限らない
