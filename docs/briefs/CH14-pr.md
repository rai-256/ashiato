# CH14 —— インストーラの合言葉の問いが文字化けする

change: `openspec/changes/ch14-installer-password-prompt-mojibake/`

## deep.md に積んだ未決（A）

なし

## 仮決め（B）と Ruling

design の（仮）:
- D1（仮）. 範囲は依頼どおりの 2 つ（Q1）
- D3（仮）. 条件の一覧は最初の問いの前に 1 回、断りには短い・雛形の条件を添える（Q3）
- D4（仮）. 問いの文と条件は、窓（`CONOUT$`）へ `WriteConsoleW` で直に書く（Q5）
- D6（仮）. 窓の試験は、疑似コンソール（ConPTY）を起こす PowerShell の台本で、install-check の中から走らせる（Q4）
- D7（仮）. 確認バッチで本人が日本語の窓で 1 回見る（Q4）
- D8（仮）. 値の口は窓のコード頁を先に、失敗の理由の末尾は UTF-8 を先に読む（Q2 に伴う。順は Q6 の答え）
- D9（仮）. 読み方は Windows の CI の単体の試験で、英字でない名前は install-check の repair で確かめる（Q2 に伴う）

全 Task の ledger に `Ruling:` の行は無い。

## Task

8 Task のうち、埋めた Task は 8 本。

残した項目:
- 9.1（人間の確認待ち（確認バッチが持つ））: 9.1 **日本語の Windows の窓で問いと条件が読める**: 本人の PC（コード頁 932）で、管理者でない端末から `ashiato-setup.exe password` を開き、管理者の確認を通して開き直した窓で、条件の一覧と「画面の合言葉: 」「画面の合言葉（もう一度）: 」が化けずに読めること・打った文字が出ないこと・15 文字を入れると「短い（16 文字以上）」と断られることを見る（決め直す合言葉は本人が決める）

## 通した検証（`evidence.jsonl` の項目とコマンドごとの最新）

- 1.1 `bash -c 'n=$(cargo test -p ashiato-setup --lib ch14_ -- --list 2>/dev/null | grep -c ": test$"); [ "$n" -ge 3 ] && cargo test -p ashiato-setup --lib ch14_'` → rc=0（PASS・head dae5668）
- 1.1 `bash -c 'cargo test -p ashiato-setup --lib'` → rc=0（PASS・head dae5668）
- 2.1 `cargo clippy -p ashiato-setup --all-targets --target x86_64-pc-windows-gnu -- -D warnings` → rc=0（PASS・head 7b64c19）
- 2.1 `cargo clippy -p ashiato-setup --all-targets -- -D warnings` → rc=0（PASS・head 7b64c19）
- 2.1 `bash -c '! git diff origin/main -- Cargo.lock | grep -E "^\+name = "'` → rc=0（PASS・head 7b64c19）
- 2.1 `tools/check-licenses.sh` → rc=0（PASS・head 7b64c19）
- 2.2 `bash -c '! grep -rn "prompt_password" crates/installer/src'` → rc=0（PASS・head 6ef0571）
- 2.2 `bash -c 'grep -c "fn read_web_passwords" crates/installer/src/main.rs | grep -qx 1'` → rc=0（PASS・head 6ef0571）
- 2.2 `cargo clippy -p ashiato-setup --all-targets --target x86_64-pc-windows-gnu -- -D warnings` → rc=0（PASS・head 6ef0571）
- 2.2 `bash -c 'cargo test -p ashiato-setup'` → rc=0（PASS・head 6ef0571）
- 3.1 `bash -c 'LC_ALL=C grep -qP "[^\x00-\x7F]" tools/release/conpty-run.ps1 && exit 1 || exit 0'` → rc=0（PASS・head 6ef0571）
- 3.2 `tools/release/install-check.sh --probe-console` → rc=0（PASS・head 6ef0571）
- 3.2 `tools/release/install-check.sh --self-test` → rc=0（PASS・head 6ef0571）
- 3.3 `bash -c 'python3 -c "import yaml,sys; j=yaml.safe_load(open(\".github/workflows/ci.yml\",encoding=\"utf-8\"))[\"jobs\"][\"install-windows\"][\"steps\"]; r=[s.get(\"run\",\"\") for s in j]; i=[n for n,x in enumerate(r) if \"--probe-console\" in x]; k=[n for n,x in enumerate(r) if \"install-check.sh --list\" in x]; sys.exit(0 if i and k and i[0]<k[0] else 1)"'` → rc=0（PASS・head 86fbbe7）
- 4.1 `tools/release/install-check.sh --self-test` → rc=0（PASS・head ce0411e）
- 4.1 `bash -n tools/release/install-check.sh` → rc=0（PASS・head ce0411e）
- 4.2 `tools/release/install-check.sh --self-test` → rc=0（PASS・head 6ef0571）
- 4.2 `bash -c 'L=$(tools/release/install-check.sh --list); for n in "端末でないときも、断りより前に合言葉の条件を 1 回だけ示す" "短い・雛形で断るときは、理由に条件を添える" "コード頁が UTF-8 でない窓でも、合言葉の問いと条件が読める" "窓で打った合言葉で決め直せる" "窓で打った合言葉は、窓に出ない" "標準出力と標準エラーを向け替えても、合言葉の問いは窓に出る"; do grep -Fxq -- "$n" <<<"$L" || { echo "--list に無い: $n"; exit 1; }; done'` → rc=0（PASS・head 6ef0571）
- 4.2 `bash -c 'o=$(python3 scripts/check_scenarios.py . ch14-installer-password-prompt-mojibake 2>&1); grep -q "^scenarios: " <<<"$o" || { echo "check_scenarios.py が要約を出さない"; exit 1; }; for n in "端末でないときも、断りより前に合言葉の条件を 1 回だけ示す" "短い・雛形で断るときは、理由に条件を添える" "コード頁が UTF-8 でない窓でも、合言葉の問いと条件が読める" "窓で打った合言葉で決め直せる" "窓で打った合言葉は、窓に出ない" "標準出力と標準エラーを向け替えても、合言葉の問いは窓に出る"; do grep -F "担保の無い Scenario: $n" <<<"$o" && exit 1; done; exit 0'` → rc=0（PASS・head 6ef0571）
- 5.1 `bash -c '! grep -n "from_utf8_lossy" crates/installer/src/main.rs'` → rc=0（PASS・head 210f0f9）
- 5.1 `bash -c 'git diff origin/main -- crates/installer/src | grep -E "^\+.*allow\(unsafe_code\)" | grep -v "^+++" | wc -l | grep -qx 1'` → rc=0（PASS・head 210f0f9）
- 5.1 `cargo clippy -p ashiato-setup --all-targets --target x86_64-pc-windows-gnu -- -D warnings` → rc=0（PASS・head 210f0f9）
- 5.1 `bash -c '! git diff origin/main -- Cargo.lock | grep -E "^\+name = "'` → rc=0（PASS・head 210f0f9）
- 5.1 `bash -c 'cargo test -p ashiato-setup'` → rc=0（PASS・head 210f0f9）
- 5.2 `cargo clippy -p ashiato-setup --all-targets --target x86_64-pc-windows-gnu -- -D warnings` → rc=0（PASS・head 210f0f9）
- 5.2 `bash -c 'for n in ch14_output_tail_reads_console_code_page ch14_unreadable_bytes_become_replacement ch14_utf8_child_output_is_kept_on_932 ch14_ascii_child_output_is_unchanged ch14_account_read_in_code_page_932 ch14_downloads_read_in_code_page_932 ch14_ident_line_is_literal_in_ansi_bytes ch14_unrepresentable_name_is_refused; do grep -rq "fn $n" crates/installer/src || { echo "無い: $n"; exit 1; }; done'` → rc=0（PASS・head 210f0f9）
- 5.2 `bash -c 'o=$(python3 scripts/check_scenarios.py . ch14-installer-password-prompt-mojibake 2>&1); grep -q "^scenarios: " <<<"$o" || { echo "check_scenarios.py が要約を出さない"; exit 1; }; for n in "窓のコード頁で書いた子の命令の失敗の理由が、化けずに読める" "どのコード頁でも読めない並びは、置換の文字にして止まらない" "UTF-8 で書く子の出力は、窓のコード頁が 932 でもそのまま読める" "英字だけの子の出力は、いまと同じ値として読む" "コード頁 932 で書いた日本語の利用者名を、同じ文字として読む" "コード頁 932 で書いた日本語のダウンロードの場所を、同じ文字として読む" "英字でない利用者名の対応表の行は、Windows の文字コードのバイト列で、文字どおりに書く" "Windows の文字コードで表せない利用者名なら、repair は理由を示して止まる"; do grep -F "担保の無い Scenario: $n" <<<"$o" && exit 1; done; exit 0'` → rc=0（PASS・head 210f0f9）
- 6.1 `bash -c 'python3 -c "import yaml,sys; j=yaml.safe_load(open(\".github/workflows/ci.yml\",encoding=\"utf-8\"))[\"jobs\"][\"package-windows\"][\"steps\"]; r=[s.get(\"run\",\"\") for s in j]; i=[n for n,x in enumerate(r) if \"cargo test -p ashiato-setup ch14_\" in x and \"-ge 11\" in x]; k=[n for n,x in enumerate(r) if \"package.sh windows\" in x]; sys.exit(0 if i and k and i[0]<k[0] else 1)"'` → rc=0（FAIL・head ee2d881）
- 6.1 `tools/check-ci-ch14-step.sh` → rc=0（PASS・head 210f0f9）
- 6.2 `tools/release/install-check.sh --self-test` → rc=0（PASS・head 6ef0571）
- 6.2 `bash -n tools/release/install-check.sh` → rc=0（PASS・head 6ef0571）
- 6.2 `bash -c 'tools/release/install-check.sh --list | grep -Fxq "英字でない名前の管理者でも repair でき、決めた合言葉でログインできる"'` → rc=0（PASS・head 6ef0571）
- 6.2 `python3 scripts/check_scenarios.py . ch14-installer-password-prompt-mojibake` → rc=0（PASS・head 6ef0571）
- 7.1 `bash -c 'sed -n 8p docs/install.md | grep -q "16 文字以上"'` → rc=0（PASS・head 86a1c2f）
- 8.1 `cargo fmt --all --check` → rc=0（PASS・head a28dff1）
- 8.1 `cargo clippy --workspace --all-targets -- -D warnings` → rc=0（PASS・head a28dff1）
- 8.1 `cargo clippy -p ashiato-setup --all-targets --target x86_64-pc-windows-gnu -- -D warnings` → rc=0（PASS・head a28dff1）
- 8.1 `bash -c 'cargo test -p ashiato-setup'` → rc=0（PASS・head a28dff1）
- 8.1 `python3 scripts/check_scenarios.py . ch14-installer-password-prompt-mojibake` → rc=0（PASS・head a28dff1）

## レビュー（`review/code.md`）

- 指摘 25 件（escalated 3・fixed 19・rejected 3）
- 処置の re-review: 4 回。最後の判定が NOT ADDRESSED のもの: なし

## 未処置の申し送り

なし
