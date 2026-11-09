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

失われるもの（`loss`）のある決定は無い: どれも表示と試験の置き場で、後から直せば戻る。
| 疑似コンソール（ConPTY）で windows-latest の runner の step からインストーラを窓につないで起こせる | Q4 の CI の試験が組めるか | 確かめていない（spec の段で確かめる） | 未確認 | 知識 | 1 |
