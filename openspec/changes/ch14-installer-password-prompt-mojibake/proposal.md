# CH14 — インストーラの合言葉の問いが文字化けする

> 第 1 回の深掘りの答えを待たずに、推奨の答え（`deep.md` の `（答え待ち・推奨: …）`）で先に書いた（2026-10-20 の流れ）。
> 本人の答えが推奨と違えば、`spec --answers` がその問いだけを写す。2026-11-10 の本人の答えで、推奨と違った Q2（表示も値の口も含める）を写した（下の What Changes の 5 つ目・Impact）。`docs/changes/CH14.md` の「壊してはいけないもの」「完了の判定」も、この段で推奨の答えに合わせて書いた。

## Why

2026-11-09、本人が日本語の Windows で `ashiato-setup.exe` を開くと、昇格して開き直した窓で、合言葉の問い（「画面の合言葉: 」）だけが『逕ｻ髱｢縺ｮ蜷郁ｨ闡』のように化けた。
原因は rpassword の `prompt_password` が問いの文を窓（`CONOUT$`）へ UTF-8 のバイトのまま書き、窓がそれをコード頁 932 で読むこと（`premises.md`）。ほかの行は Rust の標準出力（`WriteConsoleW`）を通るので化けない。
CI は合言葉を管で流すので、窓の経路を一度も通っておらず、見逃した。あわせて、本人は「短い」とだけ断られ、何文字要るか分からなかった。

## What Changes

- **問いの文を、窓へ `WriteConsoleW` で直に書く**（Q5 の推奨。design D4（仮））。rpassword は読み（`read_password`）だけに使う。直すのは `read_web_passwords` の 1 か所で、インストールの最後・`password`・`repair` の 3 つの口に効く。標準出力・標準エラーを向け替えても、問いは窓に出る（今と同じ）
- **最初の問いの前に 1 回だけ、受け付ける条件を 1 行で示す**（Q3 の推奨。D3（仮））。端末でないとき（管）も標準出力に出す（3 つの口の出力で確かめる）。短い・雛形で断るときは理由に条件を添える（「短い（16 文字以上）」「雛形（change-me で始めない）」）。文言は検査と同じ定数から組む（D5）
- **本物の窓で確かめる試験を Windows の CI に足す**（Q4 の推奨。D6（仮））。疑似コンソール（ConPTY）でインストーラの `password` の口を起こし、コード頁が UTF-8 でない窓で問いと条件が読め条件が問いより前にあること・窓で打った合言葉で決め直せること・打った合言葉が窓に出ないこと・向け替えても問いが窓に出ることを確かめる。化けを見分けられることを、rpassword と同じ書き方の子で化けた並びが出ることから先に確かめる（陰性の対照）。ConPTY が runner で使えるかは確かめていないので、`install-windows` のインストールの前の step で先に確かめる
- **確認バッチで本人が日本語の窓で 1 回見る**（Q4 の推奨。D7（仮））。CI が通らない「昇格して開き直した窓」を含む
- **子の命令の出力を、子が書いた文字として読む**（Q2 の本人の答え「表示も値の口も含める」。design D2・D8（仮））。インストーラが子の命令（`sc.exe`・`icacls`・`whoami`・`reg.exe`・`tasklist` ほか）の出力を読む口を全部 1 つの読み方に通す: UTF-8 として正しければそのまま、そうでなければ窓のコード頁で読む。失敗の理由に添える出力が日本語の Windows でも読め、英字でない利用者名で `repair` が、英字でないダウンロードの場所でインストールが止まらない。英字だけの出力は今と同じ値になる
- **読み方を Windows の CI で確かめる**（D9（仮））。コード頁を指した読みの単体の試験を `package-windows` の job で走らせ、install-check に英字でない名前の管理者の `repair` を足す
- **やらないこと**: 合言葉の検査の中身は変えない（Q1）。端末でないときの読み方は変えない

## Capabilities

### New Capabilities

（無い）

### Modified Capabilities

- `distribution`: Requirement「インストーラは、子の命令の出力を窓のコード頁で読み、Windows の文字コードで表せる名前なら英字でなくても止まらない」を足す（Q2）。Requirement「インストールの最後に、端末で画面の合言葉を決め、そのままログインできる」に、条件を最初の問いの前に 1 回だけ示すこと（端末のときは表示されないことも添える）・断りに条件を添えること・問いを窓のコード頁と向け替えに依らず窓へ出すことを足す（3 つの口に効く文で書く）。`password` と `repair` の Requirement は「インストールの最後と同じ検査で」と書いてあり、変えない

## Impact

- 製品のコード: `crates/installer/src/main.rs`（`read_web_passwords`・子の出力を読む 9 か所・`output_tail`・`user_downloads`・`whoami`）・`crates/installer/src/password.rs`（条件の文言と断りの理由）・新しいファイル `crates/installer/src/console.rs`（窓へ書く・子の出力を窓のコード頁で読む。`unsafe` はここだけ）・`crates/installer/src/lib.rs`（`pub mod console;`）
- 依存: `crates/installer/Cargo.toml` の `[target.'cfg(windows)'.dependencies]` に `windows-sys = "0.61"`（features に `Win32_Globalization` を含む。rpassword が既に使う版。`Cargo.lock` に新しい crate は増えない）
- 試験: `tools/release/install-check.sh`（`scenario_names` に 7 つの名前（Rust の試験の 8 本は足さない）・`--probe-console` の口・`password` の節の途中に窓の節。末尾には足さない）・新しいファイル `tools/release/conpty-run.ps1`
- CI: `.github/workflows/ci.yml` の `install-windows` に step を 1 つ（`install-check.sh --probe-console`。インストールの確かめの前）。`package-windows` に step を 1 つ（`cargo test -p ashiato-setup ch14_`。D9）。install-check に 3 人目の管理者（英字でない名前）と `repair` の節（今の「別の管理者が repair」の直後）。新しい Scenario は install-check の中から印を出すので、`--list` と出力の突き合わせ（`:452-459`）がそのまま数える
- 文書: `docs/install.md:8`（合言葉の条件を書き足す。行を直すだけ）
- API・移行・画面・収集アプリ: 変えない
- **子の命令の出力の化け（Q2。この change で直す）**: 子の命令の出力を `from_utf8_lossy` で読む口が cp932 で化ける。(1) 表示: `output_tail`（`main.rs:81-90`。失敗の理由に添える）・`tasklist`（`:380`）。(2) 値: `whoami` の利用者名（`:250` → repair の DB の認証の対応表。英字でない利用者名だと一致せず repair が止まる見込み）・`reg.exe` のダウンロードの場所（`:71`。英字でないフォルダ名だとインストールが止まる見込み）。本人の PC は利用者名が英字で、今は当たらない。PostgreSQL の SSPI が英字でない利用者名を認証できるかは確かめていない（design D9 の反転条件）
- **走っている change との重なり**（`origin/main` = 95cba8c4 との差分）:
  - ch11（`ci.yml` に `changes` の出力と新しい job `vulns`）・ch12（`ci.yml` の `chain` job に 1 行）: この change は `ci.yml` の `install-windows` と `package-windows` にそれぞれ step を 1 つ足すだけ（ch11・ch12 はどちらの job にも触らない）。重ならない
  - ch13（`tools/workbox.json` の `host_only` に `tools/release/(build|install-check)\.sh` を足す）: install-check.sh を名前で指すだけで中身に触らない。重ならない
  - どれも走っている change へ差し戻さない。merge の順は決めない
