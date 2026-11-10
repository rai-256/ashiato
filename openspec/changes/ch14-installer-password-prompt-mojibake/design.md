# CH14 design — インストーラの合言葉の問いが文字化けする

> 第 1 回の深掘りの推奨の答え（Q1〜Q5）で先に書いた（2026-10-20 の流れ）。本人の答えが推奨と違えば、その Q の D（仮）を反転条件のとおりに書き直す。
> 2026-11-10 の本人の答えで、推奨と違った Q2（表示も値の口も含める）を写した: D2 を書き直し、D8（仮）・D9（仮）を足した。ほかの D は変えていない。

## Context

動機は `proposal.md` の Why。いまの形（`premises.md` の表。2026-11-09 の repo）:

- 合言葉を聞く口は 1 つの関数 `read_web_passwords`（`crates/installer/src/main.rs:285`）で、インストールの最後（`:1199`）・`password`（`:1400`）・`repair`（最後に install を呼び、照らすための値が空なので `:1199` を通る）の 3 つがここを通る
- 標準入力が端末なら `rpassword::prompt_password("画面の合言葉: ")` を 2 回呼ぶ（`:291`・`:293`）。rpassword 7.5.4 は問いの文を `CONOUT$` を開いたファイルへ UTF-8 のバイトのまま書き（`lib.rs:283-288`）、窓はそれをコード頁（日本語の Windows は 932）で読むので化ける。読みは `ReadConsoleW` でエコーを止めて読む（`windows.rs:313-388`）
- ほかの行（案内・断り）は `terminal_line`（`main.rs:30`）が Rust の標準出力・標準エラーへ書く。Rust の標準出力は窓のとき `WriteConsoleW` で書くので化けない
- 端末でないとき（CI の `install-check.sh` は管で流す）は問いを出さず、標準入力の 2 行を読む。窓の経路は CI で一度も走っていない
- 断りの行は `合言葉を受け付けない: <理由>。入れ直してください`（標準エラー）。理由の語は `PasswordRefusal::reason`（`password.rs:17-25`）。CI は `受け付けない: 短い` などの字面で確かめている（`install-check.sh:975`・`:996`）
- 作業場の lint は `unsafe_code = "deny"`（`Cargo.toml:27`）。モジュールで許す前例は `crates/collector-windows/src/secret.rs:3`（`#![cfg_attr(windows, allow(unsafe_code))]`）
- `windows-sys 0.61.2` は rpassword の依存として既に `Cargo.lock` にある
- 子の命令の出力は、どれも `String::from_utf8_lossy` で読む（`main.rs:71` `reg.exe`・`:84-85` `output_tail`・`:162` `sc.exe sdshow`・`:193` `sc.exe query`・`:250` `whoami`・`:358` `netstat`・`:380` `tasklist`・`:408` PowerShell・`:745` `Get-Acl`）。本人の PC では `sc.exe`・`icacls`・`tasklist`・`reg.exe`・`whoami` が窓のコード頁（932）で管へ書き、`chcp 65001` の窓では UTF-8 か英語の文で書く（`premises.md`）。`whoami` の利用者名は `repair_ident_contents`（`commands.rs:1097`）で DB の認証の対応表の行になり、`reg.exe` のダウンロードの場所は `user_downloads`（`main.rs:59`。`--downloads` が無いときだけ）で設定とインストールに入る
- install-check は `--downloads` を必ず渡すので、`reg.exe` の読みは CI で一度も走っていない。Windows の job で `cargo test -p ashiato-setup` を走らせる step も無い（`package-windows` は組むだけ）

## Goals / Non-Goals

**Goals:**
- 日本語の Windows の窓で、合言葉の問いの文と条件の一覧が化けずに読める（install の最後・`password`・`repair` の 3 つの口。1 か所を直せば 3 つに効く）
- 最初の問いの前に、受け付ける条件を示す。短い・雛形の断りには条件を添える
- 本物の窓（疑似コンソール）で、問いが化けないこと・打った合言葉が出ないこと・向け替えても問いが窓に出ることを、Windows の CI が毎回確かめる
- 子の命令の出力を、子が書いた文字として読む。失敗の理由が日本語の Windows でも読め、英字でない利用者名・フォルダ名でも `repair` とインストールが止まらない（Q2）

**Non-Goals:**
- 合言葉の検査の中身（16 文字・`change-me`・API の合言葉とログインの印の鍵・2 回の一致）を変えること（Q1）
- 端末でないとき（管）の読み方を変えること。問いの文は今どおり出さない
- rpassword を外すこと・版を上げること（読みは rpassword のまま）

## Decisions

### D1（仮）. 範囲は依頼どおりの 2 つ（Q1）

問いの文の書き方を直し（`read_web_passwords` の 1 か所）、最初の問いの前に条件を示す。検査の中身は変えない。

- 反転条件: 本人が Q1 で「文字化けだけ」を選べば、D3 の条件の一覧と断りの添え書きを外す。spec から条件に関わる SHALL 3 つ（最初の問いの前に 1 回だけ示す・端末のとき表示されないことを添える・断りに条件を添える）と、Scenario「端末でないときも、断りより前に合言葉の条件を 1 回だけ示す」「短い・雛形で断るときは、理由に条件を添える」を落とし、窓の Scenario「コード頁が UTF-8 でない窓でも、合言葉の問いと条件が読める」「標準出力と標準エラーを向け替えても、合言葉の問いは窓に出る」の THEN から条件の語と順を外して問いの文だけにする。tasks は 1.1 の `requirements_line` と 4.1 を外し、4.2 から条件の確かめを外す。「検査も見直す」なら正典の検査の文を変える Requirement の改訂が要り、範囲を問い直す

### D2. 子の命令の出力の化けは、表示も値の口もこの change で直す（Q2。本人の答え）

本人が Q2 で「表示も値の口も含める」を選んだ（2026-11-10。推奨「含めない」と違う）。失敗の理由に添える表示（`output_tail`・`tasklist`）も、値として使う口（`whoami` の利用者名・`reg.exe` のダウンロードの場所）も、子の命令の出力を読む口を全部 1 つの読み方（D8）に通す。値は `netstat`・`sc.exe`・PowerShell の数や SID のように英字だけのものも同じ口を通す（口ごとに読み方を分けると、次に足す子で取りこぼす）。読み方は D8、確かめ方は D9。

### D3（仮）. 条件の一覧は最初の問いの前に 1 回、断りには短い・雛形の条件を添える（Q3）

- `read_web_passwords` の loop の前に 1 回だけ、次の 1 行を出す（文言の正本は `password.rs` の関数 `requirements_line()`。D5）:
  `画面の合言葉を決めます。16 文字以上・change-me で始めない・自動で作った API の合言葉やログインの印の鍵と同じにしない（ふつうは当たりません）。入れた文字は表示されません`
  端末でないときは「入れた文字は表示されません」を付けない（管では表示の問題が無い）
- 断りの理由（`PasswordRefusal::reason`）は `TooShort` → `短い（16 文字以上）`・`Placeholder` → `雛形（change-me で始めない）`。ほかの 3 つは今のまま。断りの行の形 `合言葉を受け付けない: <理由>。入れ直してください` は変えない（`受け付けない: 短い` の字面は残るので、今の CI の grep はそのまま当たる）
- 一覧は断るたびには出し直さない（行が増える。Q3 の案 3 を採らない）。1 回だけ・端末のときだけ「表示されません」を添える、は spec の SHALL に写した（review/spec.md R6）
- 反転条件: Q3 で「一覧だけ」なら断りの添え書きを外す。「断るたびに出し直す」なら loop の中で一覧を出す

### D4（仮）. 問いの文と条件は、窓（`CONOUT$`）へ `WriteConsoleW` で直に書く（Q5）

- 標準入力が端末のとき、条件の一覧と 2 つの問いの文を `CONOUT$` を開いた handle へ `WriteConsoleW`（UTF-16）で書く。改行の無い問いも、`WriteConsoleW` はバッファを持たないので読みの前に窓に出る（R4 の flush の申し送りはこれで要らなくなる）
- 読みは rpassword の `read_password()`（`CONIN$` から `ReadConsoleW`・エコーを止める）。`prompt_password` は呼ばない
- 標準入力が端末でないとき、条件の一覧は `terminal_line`（標準出力）へ書く（CI の記録で読める）。問いの文は今どおり出さない
- 新しいモジュール `crates/installer/src/console.rs`（`lib.rs` に `pub mod console;`）に閉じ込める。公開する関数は `write_to_console(text: &str) -> std::io::Result<()>` の 1 つ:
  - `cfg(windows)`: `CreateFileW("CONOUT$", GENERIC_READ | GENERIC_WRITE, FILE_SHARE_READ | FILE_SHARE_WRITE, …, OPEN_EXISTING)` → `WriteConsoleW` → `CloseHandle`。`unsafe` はこのモジュールだけ（先頭に `#![cfg_attr(windows, allow(unsafe_code))]`、各 `unsafe` の塊に `// SAFETY:` の注記）
  - `cfg(not(windows))`: 標準エラーへ書いて `flush`（インストーラは Windows の製品で、Linux の口は単体の試験の組み立てのためだけ）
  - 窓を開けない（`CreateFileW` が失敗）・`WriteConsoleW` が失敗したときは、標準エラーへ書いて続ける（問いが出ないまま止まるより、向け替え先にでも出る方がよい）。これは内部の倒し方で、振る舞いとしては約束しない（spec に書かない。標準入力が窓なのに窓を開けない場面を CI で作れない。review/spec.md R6）
- 依存: `windows-sys = { version = "0.61", features = ["Win32_Foundation", "Win32_Storage_FileSystem", "Win32_System_Console"] }` を `[target.'cfg(windows)'.dependencies]` に足す（rpassword が既に使う版で、`Cargo.lock` に新しい crate は増えない。`tools/check-licenses.sh` を通す）
- 費用: Windows の口のコードが 1 つのモジュール・`unsafe` が数行増える。標準エラーに書く案（Q5 の案 2）なら `unsafe` も依存も要らないが、標準エラーを向け替えると問いが見えなくなる
- 反転条件: Q5 で「標準エラー」「標準出力」を選べば、`console.rs` と `windows-sys` を足さず、`terminal_line(…, true/false)` の後に `flush` してから `read_password()` を呼ぶ。Scenario「標準出力と標準エラーを向け替えても、合言葉の問いは窓に出る」を、選んだ流れを向け替えない形に書き直す

### D5. 条件の文言は検査の定数から組む（聞かない既定。review/deep.md R7）

`password.rs` に `pub fn requirements_line(terminal: bool) -> String` を足し、`MIN_LEN`・`PLACEHOLDER` から組む。`PasswordRefusal::reason` は `&'static str` から `String` を返す形に替える（`短い（{MIN_LEN} 文字以上）`）。同じモジュールの `#[cfg(test)]` で、一覧と断りに `MIN_LEN` の数と `PLACEHOLDER` の語が出ることを確かめる（依頼が一度「12 文字」と誤って登録されたので、別に書くとまたずれる）。

### D6（仮）. 窓の試験は、疑似コンソール（ConPTY）を起こす PowerShell の台本で、install-check の中から走らせる（Q4）

- 台本 `tools/release/conpty-run.ps1`（新しいファイル）: `Add-Type` の C# で `CreatePseudoConsole` を使い、子を窓につないで起こし、窓に出た文字（VT の並び）を UTF-8 のファイルへ書き出す。入力は、出力が 1 秒静まってから 1 行ずつ送る（`\r`）。上限（`-TimeoutSec`。install-check からは `install_with` と同じ `${ASHIATO_INSTALL_CHECK_SETUP_TIMEOUT:-600}` 秒を渡す。`password` の口はサービスを起こし直して最大 180 秒ポートを待つので、短い上限では起こし直しの途中で止めてしまう）を超えたら子を止めて rc≠0。台本の source は ASCII だけにする（Windows PowerShell 5.1 は BOM の無い UTF-8 を cp932 に読み、日本語のコメントが次の行を飲み込む。`premises.md` 9 行目で実際に起きた。日本語の期待値は bash の側に置き、台本は書き出しだけ）。確かめに使った台本 `premise-probe/probe.ps1` を下敷きにしてよい
- **子の標準の handle は疑似コンソールにつなぐ**: CI は install-check の出力を `| tee` で流す（`ci.yml:453`）ので、台本の標準の handle は管になっている。`STARTUPINFOEX` に `STARTF_USESTDHANDLES` を**立て**、`hStdInput`・`hStdOutput`・`hStdError` を空（`IntPtr.Zero`）にし、`bInheritHandles = false` で起こす。立てないと、子は親の管を継いで窓につながらない（`premises.md` 9 行目。2026-11-09 に本人の PC で確かめた: 立てない形は子の `IsInputRedirected` が真、立てた形は偽）。作業の場所は `C:\`（WSL の UNC のままだと `cmd.exe` が断りの行を出す）
- 呼び出しは `powershell.exe -NoProfile -NonInteractive -ExecutionPolicy Bypass -File <台本>`（前例 `tools/release/sandbox-probe.sh:17`。WSL からは UNC の未署名の台本になり、既定の実行ポリシーでは読まれない。review/spec.md R3）
- 子は `cmd.exe /d /c "chcp 932 >nul & chcp & <exe> password"` で起こす（向け替えの場合は `<exe> password >out.txt 2>err.txt`）。`chcp` の出力からコード頁を読み、65001 なら試験を落とす（UTF-8 の窓では化けが見えない）。932 に替えられなくても、65001 でなければ直す前の形は化ける（`premises.md` 7 行目）
- **化けを見分けられることを先に確かめる（陰性の対照）**: 台本に `-Probe` を持たせ、同じ `chcp 932` の窓で子を 2 つ順に起こす。(1) 『画面の合言葉: 』の UTF-8 のバイトを窓へバイトのまま書く PowerShell（rpassword と同じ書き方。`[Console]::OpenStandardOutput()` へバイトを書く —— `WriteFile` の経路。Windows PowerShell 5.1 の `[IO.File]::Open('CONOUT$', …)` は「ファイルでないデバイス」として断るので使えない。`premises.md` 9 行目）、(2) 同じ文を `[Console]::Out` に書く PowerShell（`WriteConsoleW` の経路）。どちらの子も、先に `[Console]::IsInputRedirected` と `IsOutputRedirected` が偽であることを ASCII の印（`CONSOLE-OK`）で窓に出す。
  install-check は、両方に `CONSOLE-OK` があること（子が窓につながっている）・(2) に『画面の合言葉』が**ある**こと・(1) に『画面の合言葉』が無く、**化けた並び**（『画面の合言葉』の UTF-8 のバイトを `chcp` が返したコード頁で読んだ文字列。bash 側で python の `bytes.decode('cp<番号>', errors='replace')` から作る。932 なら本人が見た『逕ｻ髱｢縺ｮ蜷郁ｨ闡』と同じ並び）が**ある**ことを確かめてから本物の Scenario に進む。どれかが外れたら `die`（書きが窓へ届かないのに「化けを見分けた」ことにしない。review/spec.md R2）
- **runner で先に確かめる**: `install-windows` の job の、インストールの確かめの前の step で `tools/release/install-check.sh --probe-console` を走らせる（インストールに数分かかる前に、ConPTY が runner の step から使えるかが分かる）
- install-check の置き場: `echo '== Scenario: API の合言葉やログインの印の鍵と同じ値は受け付けない'` の前（そこまでに合言葉は `$PW2` に決め直してある）。窓の Scenario で決め直す合言葉も `$PW2` にし、後の節の前提（`$PW2` でログインできる）を変えない。Windows でだけ走る（Linux の `--self-test` は印と `--list` だけを見る）
- 前提の確かさ: 本人の PC（Windows 11）の Windows PowerShell 5.1 で、親の出力を `| tee` の管にしたまま、この形（`Add-Type` の C#・`STARTF_USESTDHANDLES` と空の handle）で子を窓につなぎ、(1) の子が『逕ｻ髱｢縺ｮ蜷郁ｨ闡』（本人が見た化けと同じ並び）を、(2) の子が『画面の合言葉: 』を窓に出すことを確かめた（`premises.md` 9 行目。確かさ 5）。windows-latest の runner の上では走らせていない（`premises.md` 8 行目。ConPTY は Windows 10 1809 からの OS の機能で、runner の Windows Server 2022 / 2025 にある。確かさ 2）。外れても失うものは無い（後から試験の置き場を替えれば戻る）
- 反転条件: PR の CI の `install-windows` の `--probe-console` の step が落ち、原因が台本の書き方でなく runner で ConPTY が使えない（`CreatePseudoConsole` が失敗する・子が `CONSOLE-OK` を出さない）と分かったら、下流は窓の Scenario 4 本を「人間の確認待ち」へ移さず（`物理:` で名付けられない）、A として人間に返す（Windows の CI の別の手——`collector-windows-runtime` の job で Rust の試験から ConPTY を起こす——か、窓の Scenario を外して別の change に出すかを問う）

### D7（仮）. 確認バッチで本人が日本語の窓で 1 回見る（Q4）

CI の runner は既に管理者で、昇格して開き直した窓（本人が化けを見た窓）を開かず、コード頁も 932 とは限らない。確認バッチで本人が、日本語の Windows で `ashiato-setup.exe password` を（管理者でない端末から）開き、昇格して開き直した窓で問いと条件が読めることを 1 回見る。tasks の「人間の確認待ち」に `> 物理: device` で置く。

- 反転条件: Q4 で「Windows の CI だけ」なら、tasks 9.1（人間の確認待ち）と `docs/changes/CH14.md` の完了の判定の確認バッチの行を外す。「確認バッチで本人が見るだけ」なら D6・Task 3・Task 4.2 と窓の Scenario 4 本を外し、9.1 を残す

### D8（仮）. 子の出力は、UTF-8 として正しければそのまま、そうでなければ窓のコード頁で読む（Q2 に伴う）

- `console.rs` に `pub fn decode_child_output(bytes: &[u8]) -> String` を足し、main.rs の子の出力の読み（Context の 9 か所）を全部これに替える。main.rs に `from_utf8_lossy` を残さない
- 読み方: (1) バイト列が UTF-8 として正しければそのまま（英字だけの出力・`ashiato-server migrate` のような Rust の子・`chcp 65001` の窓で書く子）。(2) そうでなければ `cfg(windows)` で `GetConsoleOutputCP()` のコード頁（0 なら、つまり窓が無ければ `GetOEMCP()`）を `MultiByteToWideChar` で UTF-16 に直して読む。読めない並びは置換の文字（U+FFFD）にする（いまの `from_utf8_lossy` と同じく止まらない。spec の SHALL に写した。review/spec.md R16）。作り方: まず `MB_ERR_INVALID_CHARS` を付けて全体を `MultiByteToWideChar` で読み、失敗したら、先頭から `IsDBCSLeadByteEx(code_page, b)` で 1 文字の長さ（真なら 2 バイト、偽なら 1 バイト）を決めて 1 文字ずつ同じ旗で読み、読めない文字（末尾で 2 バイト目が欠けたものを含む）だけを U+FFFD にする。旗を付けない既定の読みは、読めない並びを U+FFFD でなく別の文字（本人の PC の窓では `・`）にするので使わない（`premises.md` 9 行目の `RAW[…闡・ ]`。review/spec.md R23）。`cfg(not(windows))` は今どおり `from_utf8_lossy`
- 試験のために、コード頁を引数で受ける `decode_with_code_page(bytes: &[u8], code_page: u32) -> String`（`cfg(windows)`）と、`output_tail` を出力のバイト列とコード頁から組む形に分ける（`output_tail` の中身の「空行を除いた末尾 5 行を ` / ` で繋ぐ」は変えない）
- 窓のコード頁にした理由: 本人の PC で、`tasklist`・`reg.exe`・`sc.exe` は `chcp 437` の窓では英語の文、`chcp 65001` の窓では英語の文（`whoami` は UTF-8）、既定（932）の窓では cp932 で管へ書いた（`premises.md`）。子は OEM のコード頁でなく、引き継いだ窓のコード頁に従う
- UTF-8 を先に見る理由: Rust の子（`ashiato-server migrate`）は窓のコード頁に依らず UTF-8 で書く。cp932 の日本語の並びが偶然 UTF-8 として正しくなることは、2 文字以上の文ではほぼ無い（cp932 の 2 バイト目は 0x40〜0xFC で、UTF-8 の続きのバイト 0x80〜0xBF の並びの規則に揃わない）
- `unsafe` は `console.rs` に置く（D4 と同じモジュール。D4 の「公開する関数は `write_to_console` の 1 つ」は窓へ書く側のことで、読む側の `decode_child_output`・`decode_with_code_page` はこの D8 が足す。`MultiByteToWideChar`・`GetConsoleOutputCP`・`GetOEMCP`）。`windows-sys` の features に `Win32_Globalization` を足す（`Cargo.lock` に新しい crate は増えない）
- `psql`・`pg_ctl` は自分の lc_messages の文字で書く。UTF-8 なら (1)、窓のコード頁と同じなら (2) で読め、どちらでもなければ置換の文字が出る（いまと同じ。止まらない）
- 反転条件: 本人の PC か CI で、窓のコード頁で書かない子（OEM・ANSI のコード頁で書く子）が見つかったら、その子だけコード頁を指して読む。UTF-8 として正しい cp932 の出力が実際に見つかったら、(1) と (2) の順を子ごとに決める

### D9（仮）. 読み方は Windows の CI の単体の試験で、英字でない名前は install-check の repair で確かめる（Q2 に伴う）

- コード頁を指した読み（`decode_with_code_page`）は `cfg(windows)` でしか組めない（Rust の側に cp932 の表が無い。`encoding_rs` などの新しい依存は足さない）。ADDED の Scenario のうち repair 以外の 6 本は、`crates/installer/src/console.rs`・`main.rs` の `#[cfg(all(test, windows))]` の `ch14_` の試験にし、期待するバイト列は試験の中に 16 進で持つ（Linux の Python の `'…'.encode('cp932')` で作る。本人の PC の子が cp932 で書くことは `premises.md`）。窓のコード頁を選ぶ部分（`GetConsoleOutputCP`）は、「窓のコード頁で書いた子の命令の失敗の理由が、化けずに読める」の試験が `SetConsoleOutputCP(932)` にした窓（無ければ `AllocConsole`）から `decode_child_output` を通る `console::output_tail_text` を呼んで通す。この試験は `unsafe` が要るので `console.rs` に置き、出力の末尾を組む中身も lib の `console.rs` へ移す（main.rs の `output_tail` は呼ぶだけ。review/spec.md R20）。ほかの 5 本はコード頁を指して読む（review/spec.md R13）。読めない並びの Scenario は、後ろが欠けた cp932 の並びで確かめる（R21）。コード頁 932 と 437 は Windows が必ず持つ（英語の OS でも `MultiByteToWideChar(932, …)` は使える）
- 走らせる場所: `.github/workflows/ci.yml` の `package-windows`（windows-latest・Rust の toolchain がある）に step を 1 つ足し、`cargo test -p ashiato-setup ch14_` を `-- --list` の下限つきで走らせる（`--lib` を付けない。main.rs の試験は bin の target にある。review/spec.md R11）。`ch14_` に絞るのは、これまで Windows で一度も走っていないインストーラのほかの単体の試験を、この change で巻き込まないため
- Scenario「英字でない名前の管理者でも repair でき、決めた合言葉でログインできる」は install-check の、今の「別の管理者が repair しても、本人は変わらない」の直後に置く。管理者は `make_users` で作る 3 人目（名前は `ashiato-ci-é`。`é` は runner の ANSI のコード頁 1252（`e9`）でも窓のコード頁 437（`82`）でも表せ、UTF-8（`c3 a9`）とバイト列が違うので、読み違えも対応表の書き違えも落ちる。`remove_users` で消す）。`run_as_admin_b` と同じ形（`schtasks` で、その利用者の上がった札で `.cmd` を走らせる）で `repair` を起こす（`chcp` は替えない —— runner の ANSI のコード頁は `chcp` でも `Set-Culture` でも替わらず、日本語の名前は 1252 で表せない。review/spec.md R25）。日本語の名前は単体の試験で確かめる。確かめは、repair の rc=0・`wait_healthz`・`$PW1` でログインできる（あわせて DB の oid と記録の件数が変わらない）。外れたら、残っていれば一時の利用者規則（`repair.pg_ident.conf`）と `repair-db.log` の末尾を出してから止まる（読み違え（D8）・書き違え・反転条件 (b) を、CI の出力から見分ける。review/spec.md R14）
- 前提の確かさ: runner で `net user` が `é` を含む名前の利用者を作れるかは確かめていない（`premises.md`。確かさ 1）。PostgreSQL 17 は SSPI の利用者名を ANSI のコード頁のバイト列で受け取り、変換せずに対応表の行と突き合わせる（source を読んだ。確かさ 2）ので、UTF-8 で書く今の行（`main.rs:1490`・`commands.rs:1097-1110`）は英字でない名前で食い違う
- **そのため、`repair` の DB の認証の対応表（`repair.pg_ident.conf`）は、Windows の ANSI のコード頁（`GetACP()`）のバイト列で書く**（`console.rs` に `cfg(windows)` の `encode_with_code_page(text: &str, code_page: u32) -> Result<Vec<u8>, String>`（`WideCharToMultiByte(code_page, WC_NO_BEST_FIT_CHARS, …)` と、表せない文字があれば `Err`）と、それを `GetACP()` で呼ぶ `encode_ansi`。`cfg(not(windows))` は UTF-8 のまま）。ANSI のコード頁で表せない名前なら、合わない行を書かずに理由（`利用者名 <名前> を Windows の文字コード <番号> で表せない`）を出して止まる（spec の SHALL。review/spec.md R26）
- **英字でない名前の行は、正規表現でなく引用符でくくった文字どおりの行にする**（`repair "<利用者>@<ドメイン>" ashiato`。ドメインが無ければ `"<利用者>"`）。PostgreSQL は正規表現の行を `pg_mb2wchar_with_len` で文字に直してから比べ、cp932 の 2 バイト目には `\`（`表`＝`95 5c`）・`[` `]` `{` `}` `|` `^` が入りうるので、バイト列を正規表現に入れると行が壊れるか別の名前に当たる（review/spec.md R24）。文字どおりの行は `token_matches_insensitive`（大文字と小文字を区別しないバイトの比べ）で比べる（`hba.c`。確かさ 2）。cp932 の 2 バイト目は 0x40〜0xFC で、引用符（0x22）・空白・`#` にはならない。Windows の利用者名は `"` を持てない。英字だけの名前は今の正規表現の行のまま（バイト列を変えない。`docs/changes/CH14.md` の「壊してはいけないもの」7）
- 反転条件: (a) runner で `é` の名前の利用者を作れなければ、1252 と 437 の両方で表せる別の英字でない文字（`ü` など）にする。(b) 読みは正しく、対応表を ANSI のコード頁の文字どおりの行で書いても（単体の試験が通り、CI の出力の `repair.pg_ident.conf` の行が Windows の名前の ANSI のバイト列と同じ）PostgreSQL が英字でない名前を認証できないと分かったら、DB の側の直しはこの change の範囲を越えるので、A として人間に返す（DB の認証の対応表の書き方を変えるか、別の change に出すか）

## Risks / Trade-offs

- **ConPTY の出力は VT の並び**で、カーソルの移動で文字列が分かれることがある → 試験は VT の制御の並び（`ESC [ … 文字`・`ESC ] … BEL`）を除いてから探す。短い行（問いの文）だけを探す
- **runner の既定の窓のコード頁が 65001** なら化けが見えない → `chcp 932` で替え、`chcp` の出力で 65001 でないことを確かめる（D6）
- **`unsafe` が installer に入る**（D4）→ 1 つのモジュールに閉じ込め、`windows-gnu` 向けの clippy（`ci.yml:165`）が毎回組み立てる
- **日本語の Windows で、実物の子の命令の日本語の出力を `decode_child_output` が読むことは、CI も確認バッチも確かめない**（英語の runner では日本語の文を出す子が無い。確認バッチの tasks 9.1 は合言葉の問いだけを見る）→ 子が窓のコード頁で書くことは本人の PC の実物で確かめ（`premises.md`。確かさ 5）、その符号化のバイト列を単体の試験が窓のコード頁 932 で読む（D9）。2 つを合わせて確かめたことにする（review/spec.md R19）
- 窓の試験は `password` の口だけを通る。install の最後と `repair` は同じ関数を通るので、コードの形で同じと言える（関数を 1 つにしておく。`read_web_passwords` を口ごとに分けない）。管の道の条件の行は、install の最後・`password`・`repair` の 3 つの出力で確かめる（CI が既に取っている出力。review/spec.md R5）
