# CH14 の前提（外の形式・振る舞い）

この change の決定は、Windows のコンソールへの書き方（rpassword と Rust の標準出力）に依る。

| 前提 | 何が依るか | 確かめ方 | 結果 | 出所 | 確かさ |
|---|---|---|---|---|---|
| 日本語の Windows の窓で、合言葉の問い（「画面の合言葉: 」）だけが化ける | この change の要否 | 本人の実地（2026-11-09。昇格して開き直した窓） | 『逕ｻ髱｢縺ｮ蜷郁ｨ闡』と出た。ほかの行は化けない | 実物（依頼の逐語） | 5 |
| rpassword 7.5.4 の `prompt_password` は、問いの文を `CONOUT$` を開いたファイルへ UTF-8 のバイトのまま `write_all` する（`WriteConsoleW` を通らない） | 化ける原因と直し方（問いを rpassword に書かせない） | 依存の source を読む | `prompt_password_with_config` が `OpenOptions::new().write(true).open(path)` に `prompt.to_string().as_bytes()` を書く。`OutputTarget::FilePath` の既定は `CONOUT$`。窓のコード頁（cp932）でバイトが読まれる | `~/.cargo/registry/src/*/rpassword-7.5.4/src/lib.rs:275-292`、`src/windows.rs:16`。`Cargo.lock` の rpassword は 7.5.4 | 2 |
| rpassword の読み（`read_password`）は、窓のときは `ReadConsoleW` で 1 文字ずつ読み、エコーを止める | rpassword を読みだけに残してよいか・日本語の合言葉が化けずに読めるか | 同じ source | 入力が窓なら `ReadConsoleW`（UTF-16）、`SetConsoleMode(ENABLE_PROCESSED_INPUT)` でエコーを止める | `rpassword-7.5.4/src/windows.rs:60`・`:313-388` | 2 |
| Rust の標準出力・標準エラーは、窓につながっているとき UTF-16 に直して `WriteConsoleW` で書く（コード頁に依らない） | 問いを標準出力へ移せば化けない | Rust の std の文書（`Stdout` の「Windows では窓に UTF-8 でない並びを書けない」）と、本人の実地でほかの行が化けないこと | 化けない | 公式文書（https://doc.rust-lang.org/std/io/struct.Stdout.html）＋実物（依頼） | 2 |
| CI（install-check.sh）は合言葉を標準入力へ管で流すので、`is_terminal()` が偽で rpassword を通らない | 今の CI でこの不具合が見えない理由・試験の置き場 | `tools/release/install-check.sh:592-600`（`printf … \| "$SETUP"`）と `crates/installer/src/main.rs:286` を読む | 管で流す。窓の経路は CI で一度も走っていない | 実物（repo） | 3 |
| 子の命令（sc.exe・icacls・reg.exe など）の出力は、失敗の理由に添えるとき `String::from_utf8_lossy` で読んでいる | 同じ族の別の化け（Q2） | `crates/installer/src/main.rs:81-90`（`output_tail`）を読む | 本人の PC で sc.exe・icacls の管の出力が cp932（chcp 932）で、UTF-8 として読むと化けた。whoami の利用者名・reg.exe の場所も同じ読み方（値として使う） | 実物（本人の PC、deep-review R3） | 4 |
| windows-latest の runner の窓のコード頁は 932 ではない（英語の OS） | CI の試験で cp932 を再現できるか | 確かめていない | 未確認。直す前は 65001 以外のどのコード頁でも化けるので、試験は 932 に依らずに見分けられる（deep-review R6） | 知識 | 1 |
| 疑似コンソール（ConPTY）で windows-latest の runner の step から（出力が `\| tee` の管でも）子を窓につないで起こせる | Q4 の CI の試験が組めるか（design D6） | runner では走らせていない。PR の CI の `install-windows` のインストールの前の step（`install-check.sh --probe-console`）が確かめる。外れたら design D6 の反転条件 | 未確認。ConPTY は Windows 10 1809 からの OS の機能（runner は Windows Server 2022 / 2025）。同じ形は本人の PC で通った（次の行） | 公式文書（https://learn.microsoft.com/windows/console/creating-a-pseudoconsole-session） | 2 |
| Windows PowerShell 5.1 の `Add-Type` の C# で `CreatePseudoConsole` を使い、親の出力が管でも、子を疑似コンソールの窓につなげる。UTF-8 のバイトを `WriteFile` で窓へ書く子は、cp932 の窓で本人が見たのと同じ化けを出す | design D6 の台本の形と陰性の対照 | 2026-11-09、WSL から本人の PC（Windows 11）の `powershell.exe` で、台本 `premise-probe/probe.ps1`・`child.ps1` を `premise-probe/run.sh`（親の出力を `\| tee` の管にして走らせる）で走らせた。結果は `premise-probe/result.txt`。子は `cmd.exe /d /c "chcp 932 >nul & chcp & powershell.exe -File child.ps1"` | `STARTF_USESTDHANDLES` を立てない形: 子の `IsInputRedirected`・`IsOutputRedirected` が真で、出力は親の管へ行った（窓につながらない）。立てて 3 つの handle を空にした形: どちらも偽、窓の文字（VT を除く）に `現在のコード ページ: 932`・`RAW[逕ｻ髱｢縺ｮ蜷郁ｨ闡・ ]`（`OpenStandardOutput` に UTF-8 のバイト）・`CON[画面の合言葉: ]`（`[Console]::Out`）。`[IO.File]::Open('CONOUT$', …)` は NotSupportedException（ファイルでないデバイス）。`.ps1` に日本語のコメントを書くと、5.1 が BOM の無い UTF-8 を cp932 で読み、次の行がコメントに飲まれて消えた（台本を ASCII にして直った） | 実物（本人の PC） | 5 |

失われるもの（`loss`）のある決定は無い: どれも表示と試験の置き場で、後から直せば戻る。
