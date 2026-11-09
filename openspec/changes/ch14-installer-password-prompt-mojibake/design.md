# CH14 design — インストーラの合言葉の問いが文字化けする

> 第 1 回の深掘りの推奨の答え（Q1〜Q5）で先に書いた（2026-10-20 の流れ）。本人の答えが推奨と違えば、その Q の D（仮）を反転条件のとおりに書き直す。

## Context

動機は `proposal.md` の Why。いまの形（`premises.md` の表。2026-11-09 の repo）:

- 合言葉を聞く口は 1 つの関数 `read_web_passwords`（`crates/installer/src/main.rs:285`）で、インストールの最後（`:1199`）・`password`（`:1400`）・`repair`（最後に install を呼び、照らすための値が空なので `:1199` を通る）の 3 つがここを通る
- 標準入力が端末なら `rpassword::prompt_password("画面の合言葉: ")` を 2 回呼ぶ（`:291`・`:293`）。rpassword 7.5.4 は問いの文を `CONOUT$` を開いたファイルへ UTF-8 のバイトのまま書き（`lib.rs:283-288`）、窓はそれをコード頁（日本語の Windows は 932）で読むので化ける。読みは `ReadConsoleW` でエコーを止めて読む（`windows.rs:313-388`）
- ほかの行（案内・断り）は `terminal_line`（`main.rs:30`）が Rust の標準出力・標準エラーへ書く。Rust の標準出力は窓のとき `WriteConsoleW` で書くので化けない
- 端末でないとき（CI の `install-check.sh` は管で流す）は問いを出さず、標準入力の 2 行を読む。窓の経路は CI で一度も走っていない
- 断りの行は `合言葉を受け付けない: <理由>。入れ直してください`（標準エラー）。理由の語は `PasswordRefusal::reason`（`password.rs:17-25`）。CI は `受け付けない: 短い` などの字面で確かめている（`install-check.sh:975`・`:996`）
- 作業場の lint は `unsafe_code = "deny"`（`Cargo.toml:27`）。モジュールで許す前例は `crates/collector-windows/src/secret.rs:3`（`#![cfg_attr(windows, allow(unsafe_code))]`）
- `windows-sys 0.61.2` は rpassword の依存として既に `Cargo.lock` にある

## Goals / Non-Goals

**Goals:**
- 日本語の Windows の窓で、合言葉の問いの文と条件の一覧が化けずに読める（install の最後・`password`・`repair` の 3 つの口。1 か所を直せば 3 つに効く）
- 最初の問いの前に、受け付ける条件を示す。短い・雛形の断りには条件を添える
- 本物の窓（疑似コンソール）で、問いが化けないこと・打った合言葉が出ないこと・向け替えても問いが窓に出ることを、Windows の CI が毎回確かめる

**Non-Goals:**
- 合言葉の検査の中身（16 文字・`change-me`・API の合言葉とログインの印の鍵・2 回の一致）を変えること（Q1）
- 子の命令（`sc.exe`・`icacls`・`whoami`・`reg.exe`・`tasklist`）の出力を cp932 で読み替えること（Q2。別の change。proposal の Impact）
- 端末でないとき（管）の読み方を変えること。問いの文は今どおり出さない
- rpassword を外すこと・版を上げること（読みは rpassword のまま）

## Decisions

### D1（仮）. 範囲は依頼どおりの 2 つ（Q1）

問いの文の書き方を直し（`read_web_passwords` の 1 か所）、最初の問いの前に条件を示す。検査の中身は変えない。

- 反転条件: 本人が Q1 で「文字化けだけ」を選べば、D3 の条件の一覧と断りの添え書きを外す。spec から条件に関わる SHALL 3 つ（最初の問いの前に 1 回だけ示す・端末のとき表示されないことを添える・断りに条件を添える）と、Scenario「端末でないときも、断りより前に合言葉の条件を 1 回だけ示す」「短い・雛形で断るときは、理由に条件を添える」を落とし、窓の Scenario「コード頁が UTF-8 でない窓でも、合言葉の問いと条件が読める」「標準出力と標準エラーを向け替えても、合言葉の問いは窓に出る」の THEN から条件の語と順を外して問いの文だけにする。tasks は 1.1 の `requirements_line` と 4.1 を外し、4.2 から条件の確かめを外す。「検査も見直す」なら正典の検査の文を変える Requirement の改訂が要り、範囲を問い直す

### D2（仮）. 子の命令の出力の化けは、この change で直さない（Q2）

見つけた 2 種類（失敗の理由に添える表示 `output_tail`（`main.rs:81-90`）・`tasklist`（`:380`）と、値として使う口 `whoami` の利用者名（`:250`）・`reg.exe` のダウンロードの場所（`:71`））は、proposal の Impact に書き、別の change に起こす（人間が `hx change` で登録する）。

- 反転条件: 本人が Q2 で「値の口だけ」「全部」を選べば、該当の読み口を `MultiByteToWideChar(CP_OEMCP)` で読み替える Task と、日本語の名前での試験を足す（`psql`・`pg_ctl` は自分の lc_messages で書くので読み替えの対象かを子ごとに確かめる）

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

- 反転条件: Q4 で「Windows の CI だけ」なら、tasks 7.1 と `docs/changes/CH14.md` の完了の判定の確認バッチの行を外す。「確認バッチで本人が見るだけ」なら D6・Task 3・Task 4.2 と窓の Scenario 4 本を外し、7.1 を残す

## Risks / Trade-offs

- **ConPTY の出力は VT の並び**で、カーソルの移動で文字列が分かれることがある → 試験は VT の制御の並び（`ESC [ … 文字`・`ESC ] … BEL`）を除いてから探す。短い行（問いの文）だけを探す
- **runner の既定の窓のコード頁が 65001** なら化けが見えない → `chcp 932` で替え、`chcp` の出力で 65001 でないことを確かめる（D6）
- **`unsafe` が installer に入る**（D4）→ 1 つのモジュールに閉じ込め、`windows-gnu` 向けの clippy（`ci.yml:165`）が毎回組み立てる
- 窓の試験は `password` の口だけを通る。install の最後と `repair` は同じ関数を通るので、コードの形で同じと言える（関数を 1 つにしておく。`read_web_passwords` を口ごとに分けない）。管の道の条件の行は、install の最後・`password`・`repair` の 3 つの出力で確かめる（CI が既に取っている出力。review/spec.md R5）
