# CH22 実装タスク — 配る形を外につながる状態で入れて置き、落ち続けや誤りの続きを PR の CI で捕まえる

読む順: `proposal.md` → このファイル → `design.md` → `specs/distribution/spec.md` → `docs/changes/CH22.md` → `premises.md` → `CLAUDE.md`。

## Global Constraints

- **製品のコードを変えない**（本人の決定）: `crates/`・`collector-android/`・`web/`・`migrations/` に触れない。触ってよいのは `tools/release/soak-install.sh`（新しい）・
  `.github/workflows/ci.yml`・`docs/testing.md`・`docs/changes/CH22.md`・`openspec/changes/ch22-soak-install-online-ci/**` だけ。
  `tools/release/install-check.sh`・`update-check-install.sh`・`update-apply-install.sh`・`check-ng-observations.sh` は読むだけで変えない
- **ST57 D18 を残す**: `install-windows` の最初の `tools/release/update-check-install.sh --block-outbound` の手順を消さない・動かさない。塞ぎを外すのは置く段の台本の中だけ
- **置く段は job の最後**（`update-apply-install.sh` の手順より後、harness2 の `check_scenarios` の手順より前でもよいが、インストールと更新の試験の手順の後）
- **段の判定を緩めない・許す一覧を持たない**（深掘り Q7・design D12）。最初の走りで赤になったら Task 4 の手順に従う
- **NG は観測の部品の口で出す**（design D9）: `soak-install.sh` は頭で `tools/release/check-ng-observations.sh` を `source` し、`die`・`die_seen`・`ng_report` で落とす。
  `echo 'NG:` や裸の `exit 1` を書かない。誤りの行・確かめた記録の行を出すときも部品の伏せる関数を通す（公開の CI ログに出る）
- **Scenario の印**: 実地の確かめ（Windows の runner）は確かめの前に `echo "== Scenario: <名前>"`。`--list` は実地の印の名前を 1 行ずつ出す。
  自己試験は `echo "== Scenario:` を出さず、確かめのすぐ上に `# Scenario: <名前>` のコメントを置く。名前は spec の `#### Scenario:` と一字一句合わせる
- **自己試験は Linux で走る**（CI の `rust` の job）: 偽の Windows の命令（`sc.exe`・`netsh.exe`・`wevtutil.exe`・`tasklist.exe`・`taskkill.exe`・`ashiato-setup.exe`・`curl`）を
  置き場の `bin` に置き、`PATH` の先に足す。一時の置き場は `mktemp -d "${TMPDIR:-/tmp}/ch22-soak.XXXXXX"` で作って終わりに消す（`/tmp` の固定名に書かない）
- **時刻の差し込み**: 自己試験で「置く段の始まり」「置く間」を作るために、台本は判定の関数に時刻を引数で渡す（判定の関数は今の時刻を自分で読まない）。差し込みの口は台本の自己試験の中だけに置き、実地の口に環境変数で時刻を変える道を作らない
- 閾値は台本の頭の定数に置く: 同じ誤りの回数 `SOAK_SAME_ERROR_LIMIT=3`（D6（仮））・置く時間 300 秒（D5）・PC の収集アプリの起こし直しまで 60 秒（D7（仮））・読む間隔 5 秒（D4（仮））

## Task 1: 置く段の判定と、その自己試験

- [ ] 1.1 `tools/release/soak-install.sh` を作り、判定の関数と `--self-test`・`--list` を置く（実地の口は Task 2）。判定の関数（design D4〜D7・D13）:
  - サービスの読みの列（時刻・サービス・状態・PID）と、システムの記録の 7031/7034 の行（時刻・サービス）と、置く段の始まりの時刻から、サービスごとの起こし直し・止まりの回数を出し、1 以上なら `die`（サービスの名前と回数を観測に）
  - 置き終わりの 3 つのサービスの状態と healthz の番号を見て、実行中でない・200 でないなら `die`
  - 4 つのログ（更新係の `updater.log`・サーバの日付のログ・DB のログ・PC の収集アプリの `collector.log`）から置く間の誤りの行を design D6 の規則で抜き出し、日時（と DB のプロセスの番号）を除いて数え、全部を伏せて出し、1 つのログで同じ誤りが `SOAK_SAME_ERROR_LIMIT` 以上なら `die`
  - 更新係の `checks.jsonl` と PC の収集アプリの `update-checks.jsonl` から、置く段の始まり（PC の収集アプリは台本の起こし直し）より後の行を出し、成功の行が無ければ、理由が全部「問い合わせの上限」なら `::warning::` の行を出して通し、それ以外は `die`（記録の行の形は `crates/updater/src/lib.rs` と `crates/collector-windows/src/update_check.rs` の書く側を読んで合わせる）
  - PC の収集アプリの PID の読みの列と台本が止めた時刻から、台本の 1 回のほかに消えた・変わったなら `die`
  `--self-test` に置く印（偽の入力を置き場に書き、判定を部分殻で走らせ、落ちるものは rc≠0 と標準エラーの中身を、通るものは rc=0 と出力の中身を見る）:
  `# Scenario: 置く間に 1 回でも起こし直されたら、サービスの名前と回数を出して落ちる`・
  `# Scenario: 起こし直されずに止まったままのサービスがあれば落ちる`・
  `# Scenario: 置き終わりに healthz が 200 でなければ落ちる`・
  `# Scenario: 置く段の始まりより前のサービスの終わりの記録は数えない`・
  `# Scenario: 同じ誤りが決まった回数だけあると落ちる`・
  `# Scenario: 決まった回数より少ない同じ誤りは、行を出して通る`・
  `# Scenario: 警告の段の行と置く段の始まりより前の行は数えない`・
  `# Scenario: 置く間に確かめた行があれば通る`・
  `# Scenario: 置く段の始まりより後の確かめた記録が 1 行も無ければ落ちる`・
  `# Scenario: 届かない・読めない答えの行だけなら落ちる`・
  `# Scenario: 問い合わせの上限の行だけなら、警告を出して通る`・
  `# Scenario: 台本が止めたほかに PC の収集アプリが終われば落ちる`・
  `# Scenario: 起こし直しより前の確かめた行だけなら、PC の収集アプリの確かめは無いとみなす`・
  `# Scenario: 置く段の出力に秘密の値が出ない`（状態の置き場の偽の設定の 5 つの秘密と台本の合言葉を、サーバのログと DB のログの誤りの行に混ぜる。標準出力と標準エラーのどこにも値が無く、`***` の行がある）・
  `# Scenario: 置く段の台本は観測を出さずに落ちる道を持たない`（`tools/release/check-ng-observations.sh "$0"` が rc=0）。
  どの「落ちる」Scenario も、落ちる入力から 1 か所だけ直した入力では通ることを並べて見る（判定がいつも落ちる形で緑にならない）
  検証: `tools/release/soak-install.sh --self-test` rc=0 / `tools/release/check-ng-observations.sh tools/release/soak-install.sh` rc=0 /
  `bash -c 'test "$(grep -c "^[[:space:]]*# Scenario: " tools/release/soak-install.sh)" -ge 15'` rc=0

## Task 2: 置く段の実地と、CI の配線

- [ ] 2.1 `soak-install.sh <配るサーバの zip> <配る PC の収集アプリの zip>` の実地の口を作る（design D2・D3・D5・D7）: 掃除（D2。消した後に残りが無いことを確かめ、残れば `die`）→
  置く段の始まりの時刻を取る → 塞ぎの規則 `ashiato-update-check-ci-outbound` を消し、`show rule` で無いことを確かめる（残れば入れずに `die`）→ 配る zip を `install-check.sh` と同じ口で入れる →
  healthz が 200 になるのを 150 秒まで待つ → PC の収集アプリを開いて送り先なしで起こす → 5 秒ごとに 3 つのサービスの `sc.exe queryex` と PC の収集アプリの `tasklist` を読み、60 秒で PC の収集アプリを止めて起こし直す → 300 秒の後に Task 1 の判定を全部通す。
  実地の印: `echo "== Scenario: 配る形のサーバを外につながる状態で 5 分置くと、落ちず・止まらず・誤りが続かず、本物の配布元に確かめる"`・
  `echo "== Scenario: 配る形の PC の収集アプリは、設定せずに起こし直すと本物の配布元に確かめ、動き続ける"`（それぞれの判定の前）。`--list` はこの 2 つを出す。
  自己試験に足す（偽の命令が受けた引数を順に記録する）: `# Scenario: 置く段は、前の段の入れたものと更新係の置き場を消し、塞ぎを外してから配る zip を入れる`（命令の順と、インストールの時に更新係の置き場が無いこと）・
  `# Scenario: 塞ぎを外せなければ、配る zip を入れずに落ちる`（`show rule` が規則を返し続ける偽の `netsh.exe`。インストーラの呼び出しが 0 回）
  検証: `tools/release/soak-install.sh --self-test` rc=0 / `bash -c 'test "$(tools/release/soak-install.sh --list | wc -l)" -eq 2'` rc=0
- [ ] 2.2 `.github/workflows/ci.yml` を直す: `install-windows` の最後（`update-apply-install.sh` の手順の後）に置く段の手順を足し（`timeout-minutes: 15`。D8（仮））、
  `tools/release/soak-install.sh package/package-windows-release/ashiato-server-*.zip package/package-windows-release/ashiato-collector-windows-*.zip | tee "$RUNNER_TEMP/soak.log"` を走らせ、
  `--list` と走った `== Scenario:` の行を `comm` で突き合わせて欠けがあれば落とす（ほかの 3 つの台本と同じ形）。`install-windows` の job に `timeout-minutes: 100`（D8（仮））。
  D18 の注記（`ci.yml:607-608`「この規則は job の終わりまで外さない」）を「置く段（`soak-install.sh`）で外すまで外さない」に直す。`rust` の job の `update-apply-install.sh --self-test` の隣に `./tools/release/soak-install.sh --self-test` を足す。
  自己試験に CI の定義を読む確かめを足す（読むのは `$root/.github/workflows/ci.yml`。写しを置き場に作って壊した形で落ちることも見る）:
  `# Scenario: 置く段は、ほかのインストールと更新の試験の後に、最後の段として走る`（置く段の手順が `install-check.sh`・`update-check-install.sh <zip>`・`update-apply-install.sh <zip>` の手順より後、
  `--block-outbound` の手順が最初の `install-check.sh <zip>` の手順より前、置く段の手順と `install-windows` の job に `timeout-minutes`）・
  `# Scenario: 本物の配布元へ繋ぐ自動の試験は、握手の試験と置く段だけである`（`--ignored`・`--include-ignored` を持つ手順が `collector-windows-runtime` の `tls_` に絞った 1 つだけ、
  `soak-install.sh <zip>` と `delete rule` を持つ手順が `install-windows` の置く段の 1 つだけ。`tls_` に絞らない `--ignored` の手順を足した写しと、別の手順に `delete rule` を足した写しで落ちる）
  検証: `tools/release/soak-install.sh --self-test` rc=0 /
  `bash -c 'f="$1"; actionlint "$f" || docker run --rm -v "$PWD:/repo" -w /repo rhysd/actionlint:1.7.7 "$f"' _ .github/workflows/ci.yml` rc=0 /
  `bash -c 'grep -q "soak-install.sh --self-test" .github/workflows/ci.yml && grep -q "soak-install.sh --list" .github/workflows/ci.yml && grep -q -- "--block-outbound" .github/workflows/ci.yml'` rc=0 /
  終了の条件: PR の CI の `install-windows` が緑（下流の手元では走らない。Task 4）

## Task 3: 文書

- [ ] 3.1 `docs/testing.md` の CI の job の表の `install-windows` の行に、最後の置く段（配る形を外につながる状態で入れて 5 分置き、起こし直し・止まり・同じ誤りの続き・配布元に確かめた記録を見る）を足し、
  「release の台本」の節に `tools/release/soak-install.sh` を足す（観測の部品を使うこと・自己試験は `rust` の job）
  検証: `bash -c 'grep -q "soak-install.sh" docs/testing.md && grep -q "置く段" docs/testing.md'` rc=0

## Task 4: 最初の走りの記録と、赤のときの扱い

- [ ] 4.1 PR の CI の `install-windows` の置く段の出力を読み、確かさ 1 だった前提を `premises.md` に記録する（行を書き換えず、結果・出所・確かさの欄を直し、run の番号を出所に書く）:
  P2（置く間の起こし直しの回数。正常なら 0）・P8（何もしないで置いたときの誤りの行。ログごとの行と回数）・P11（`wevtutil` で 7031/7034 が読めたか）・P7（PC の収集アプリが通知を出して確かめる係を付けたか。`update-checks.jsonl` の行の有無）。
  赤になったら design D12 のとおり: 原因が製品の不具合なら、判定・閾値・製品のコードを変えず、`BLOCKED` で終えて段の出力の抜き書き（サービス・行・記録）を報告に書く（直しの change は人間が登録し、CH22 の `requires` に足す）。
  原因が段の側なら台本を直し、閾値や数え方を変えるときは D4・D6・D7 の反転条件に当たることを `Ruling:` と design の反転条件の下に書く。配布元の揺れ（5xx・時間切れ）なら走らせ直す
  検証: `bash -c 'grep -Eq "^\| P8 .*\| [2-5] \|$" openspec/changes/ch22-soak-install-online-ci/premises.md && grep -Eq "^\| P11 .*\| [2-5] \|$" openspec/changes/ch22-soak-install-online-ci/premises.md'` rc=0 /
  終了の条件: PR の CI の `install-windows` が、置く段を含めて緑

## Task 5: 全体

- [ ] 5.1 Scenario の印の突き合わせと、台本の検査
  検証: `python3 scripts/check_scenarios.py . ch22-soak-install-online-ci` rc=0 / `tools/release/soak-install.sh --self-test` rc=0 /
  `tools/release/install-check.sh --self-test` rc=0 / `tools/release/update-check-install.sh --self-test` rc=0 / `tools/release/update-apply-install.sh --self-test` rc=0 /
  `bash -c 'git diff --name-only origin/main...HEAD | grep -Ev "^(tools/release/soak-install\.sh|\.github/workflows/ci\.yml|docs/testing\.md|docs/changes/CH22\.md|openspec/changes/ch22-soak-install-online-ci/.*)$" | grep -v "^docs/briefs/" | { ! grep -q .; }'` rc=0（予定のパスの外に触れていない）

## 人間の確認待ち

なし（画面も本人の操作も変わらない。置く段は PR の CI が機械で見る）
