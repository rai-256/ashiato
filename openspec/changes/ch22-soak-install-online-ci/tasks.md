# CH22 実装タスク — 配る形を外につながる状態で入れて置き、落ち続けや誤りの続きを PR の CI で捕まえる

読む順: `proposal.md` → このファイル → `design.md` → `specs/distribution/spec.md` → `docs/changes/CH22.md` → `premises.md` → `CLAUDE.md`。

## Global Constraints

- **製品のコードを変えない**（本人の決定）: `crates/`・`collector-android/`・`web/`・`migrations/` に触れない。触ってよいのは `tools/release/soak-install.sh`（新しい）・
  `.github/workflows/ci.yml`・`docs/testing.md`・`docs/changes/CH22.md`・`openspec/changes/ch22-soak-install-online-ci/**` だけ。
  `tools/release/install-check.sh`・`update-check-install.sh`・`update-apply-install.sh`・`check-ng-observations.sh` は読むだけで変えない
- **ST57 D18 を残す**: `install-windows` の最初の `tools/release/update-check-install.sh --block-outbound` の手順を消さない・動かさない。塞ぎを外すのは置く段の台本の中だけ
- **置く段の位置**: `install-windows` の、インストールと更新の試験（`install-check.sh <zip>`・`update-check-install.sh <zip>`・`update-apply-install.sh <zip>`）の手順より後。
  その後ろに harness2 の `check_scenarios` の手順が続くのはよい（spec の「インストールと更新の試験の手順より後」と同じ）
- **段の判定を緩めない・外れを許す一覧を持たない**（深掘り Q7・design D12）。配布元の答えの形を見本と比べない。PC の収集アプリに送り先を設定しない。最初の走りで赤になったら design D12・D15 のとおりに扱う
- **NG は観測の部品の口で出す**（design D9）: `soak-install.sh` は頭で `tools/release/check-ng-observations.sh` を `source` し、`die`・`die_seen`・`ng_report` で落とす。
  `echo 'NG:` や裸の `exit 1` を書かない。誤りの行・確かめた記録の行を出すときも部品の伏せる関数を通す（公開の CI ログに出る）
- **Scenario の印**: 実地の確かめ（Windows の runner）は確かめの前に `echo "== Scenario: <名前>"`。`--list` は実地の印の名前を 1 行ずつ出す。
  自己試験は `echo "== Scenario:` を出さず、確かめのすぐ上に `# Scenario: <名前>` のコメントを置く。名前は spec の `#### Scenario:` と一字一句合わせる
- **自己試験は Linux で走る**（CI の `rust` の job）: 偽の Windows の命令（`sc.exe`・`netsh.exe`・`wevtutil.exe`・`tasklist.exe`・`taskkill.exe`・`reg.exe`・`icacls.exe`・`ashiato-setup.exe`・
  zip を開く命令・`ashiato-collector-windows.exe`・`curl`）を置き場の `bin` に置き、`PATH` の先に足す。偽の命令は受けた引数を呼ばれた順に 1 行ずつ記録のファイルへ書く。
  一時の置き場は `mktemp -d "${TMPDIR:-/tmp}/ch22-soak.XXXXXX"` で作って終わりに消す（`/tmp` の固定名に書かない）
- **実地の口を Linux で走らせる継ぎ目**（`tools/release/update-apply-install.sh:899-916` の形を写す）: 自己試験は `cd "$work"` の中で実地の関数を走らせ、`C:/…` の置き場の字面を作業の置き場の中の相対の道として読ませる。
  待ちの秒数（掃除の待ち・healthz の 150 秒・PC の収集アプリの 60 秒・置く 300 秒・読む間隔 5 秒）は台本の頭の変数に置き、自己試験の関数の中からだけ `reset_wait_seconds=0` の形で 0 か 1 に縮める。
  これは秒数の差し込みで、時刻の差し込みではない
- **時刻の差し込み**: 判定の関数は「置く段の始まり」「置く間の始まり」「PC の収集アプリを起こし直した時刻」を引数で受け取る（判定の関数は今の時刻を自分で読まない）。
  実地の口に、環境変数などで時刻を変える道を作らない
- 閾値は台本の頭の定数に置く: 同じ誤りの回数 `SOAK_SAME_ERROR_LIMIT=3`（深掘り Q3。design D6（仮））・置く時間 300 秒（D5）・PC の収集アプリの起こし直しまで 60 秒（D7（仮））・読む間隔 5 秒（D4（仮））

## Task 1: 置く段の判定と、その自己試験

- [x] 1.1 `tools/release/soak-install.sh` を作り、判定の関数と `--self-test`・`--list` を置く（実地の口は Task 2）。判定の関数（design D4〜D7・D13）:
  - サービスの読みの列（時刻・サービス・状態・PID）と、システムの記録の 7031/7034 の行（時刻・サービス）と、置く間の始まりの時刻から、サービスごとの起こし直し・止まりの回数を出し、1 以上なら `die`（サービスの名前と回数を観測に）。
    システムの記録を読めたか（読めたなら置く間の件数）を落ちない回にも 1 行出す。読めなければ `::warning::` の行を出し、状態と PID の読みだけで判定する
  - 置き終わりの 3 つのサービスの状態と healthz の番号を見て、実行中でない・200 でないなら `die`
  - 4 つのログ（更新係の `updater.log`・サーバの日付のログ・DB のログ・PC の収集アプリの `collector.log`）から、置く間に書かれた誤りの行を design D6 の規則で抜き出し、日時と DB のプロセスの番号を除いて数え、
    全部を伏せて出し、1 つのログで同じ誤りが `SOAK_SAME_ERROR_LIMIT` 以上なら `die`。サーバのログの panic の行は 1 行で `die`
  - 更新係の `checks.jsonl` と PC の収集アプリの `update-checks.jsonl` から、置く段の始まり（PC の収集アプリは台本の起こし直し）より後の行を出し、成功の行が無ければ、理由が全部「問い合わせの上限」なら `::warning::` の行を出して通し、
    それ以外は `die`（記録の行の形は `crates/updater/src/lib.rs` と `crates/collector-windows/src/update_check.rs` の書く側を読んで合わせる）
  - PC の収集アプリの PID の読みの列と台本が止めた時刻から、台本の 1 回のほかに消えた・変わったなら `die`。`collector.log` に `outage_notice_unavailable` の行があるかを落ちない回にも 1 行出す（design D7）
  `--self-test` に置く印（偽の入力を置き場に書き、判定を部分殻で走らせ、落ちるものは rc≠0 と標準エラーの中身を、通るものは rc=0 と出力の中身を見る）:
  `# Scenario: 置く間に 1 回でも起こし直されたら、サービスの名前と回数を出して落ちる`・
  `# Scenario: 起こし直されずに止まったままのサービスがあれば落ちる`・
  `# Scenario: 置き終わりに healthz が 200 でなければ落ちる`・
  `# Scenario: 置く間より前のサービスの終わりの記録は数えない`・
  `# Scenario: システムの記録を読めなければ、警告を出し、状態とプロセスの番号で判定する`・
  `# Scenario: 同じ誤りが 3 回あると落ちる`・
  `# Scenario: 2 回の同じ誤りは、行を出して通る`・
  `# Scenario: 警告の段の行と置く段の始まりより前の行は数えない`・
  `# Scenario: 置く段の始まりと置く間の始まりの間に書かれた誤りの行は数えない`・
  `# Scenario: プロセスの番号だけが違う DB のログの行は、同じ誤りとして数える`・
  `# Scenario: サーバのログの panic の行は 1 行でも落ちる`・
  `# Scenario: 置く段の始まりより後に確かめた行があれば通る`・
  `# Scenario: 置く段の始まりより後の確かめた記録が 1 行も無ければ落ちる`・
  `# Scenario: 届かない・読めない答えの行だけなら落ちる`・
  `# Scenario: 問い合わせの上限の行だけなら、警告を出して通る`・
  `# Scenario: 問い合わせの上限と届かないの行が混ざれば落ちる`・
  `# Scenario: 台本が止めたほかに PC の収集アプリが終われば落ちる`・
  `# Scenario: 起こし直しより前の確かめた行だけなら、PC の収集アプリの確かめは無いとみなす`・
  `# Scenario: 置く段の出力に秘密の値が出ない`（状態の置き場の偽の設定の 5 つの秘密と台本の合言葉を、サーバのログと DB のログの誤りの行に混ぜる。標準出力と標準エラーのどこにも値が無く、`***` の行がある）・
  `# Scenario: 置く段の台本は観測を出さずに落ちる道を持たない`（`tools/release/check-ng-observations.sh "$0"` が rc=0）。
  どの「落ちる」Scenario も、落ちる入力から 1 か所だけ直した入力では通ることを並べて見る（判定がいつも落ちる形で緑にならない）。`SOAK_SAME_ERROR_LIMIT` が 3 であることも自己試験で見る
  検証: `tools/release/soak-install.sh --self-test` rc=0 / `tools/release/check-ng-observations.sh tools/release/soak-install.sh` rc=0 /
  `bash -c 'test "$(grep -c "^[[:space:]]*# Scenario: " tools/release/soak-install.sh)" -ge 20 && grep -q "^SOAK_SAME_ERROR_LIMIT=3$" tools/release/soak-install.sh'` rc=0

## Task 2: 置く段の実地と、CI の配線

- [x] 2.1 `soak-install.sh <配るサーバの zip> <配る PC の収集アプリの zip>` の実地の口を作る（design D2・D3・D5・D7）: 掃除（D2。消した後に残りが無いことを確かめ、残れば塞ぎの規則に触れずに `die`）→
  置く段の始まりの時刻を取る → 塞ぎの規則 `ashiato-update-check-ci-outbound` を消し、`show rule` で無いことを確かめる（残れば入れずに `die`）→ 配る zip を `install-check.sh` と同じ口で入れる →
  healthz が 200 になるのを 150 秒まで待ち、置く間の始まりの時刻を取る → PC の収集アプリを開いて送り先なしで起こす → 5 秒ごとに 3 つのサービスの `sc.exe queryex` と PC の収集アプリの `tasklist` を読み、60 秒で PC の収集アプリを止めて起こし直す →
  300 秒の後にシステムの記録を読み、Task 1 の判定を全部通す。
  実地の印: `echo "== Scenario: 配る形のサーバを外につながる状態で 5 分置くと、落ちず・止まらず・誤りが続かず、本物の配布元に確かめる"`・
  `echo "== Scenario: 配る形の PC の収集アプリは、設定せずに起こし直すと本物の配布元に確かめ、動き続ける"`（それぞれの判定の前）。`--list` はこの 2 つを出す。
  自己試験に足す（Global Constraints の継ぎ目で実地の関数そのものを走らせる）:
  `# Scenario: 置く段は、前の段の入れたものと更新係の置き場を消し、塞ぎを外してから配る zip を入れる`（記録した命令の順）・
  `# Scenario: 配る zip を入れる時に、前の段の更新係の置き場が残っていない`（偽のインストーラが呼ばれた時に置き場の有無を書く）・
  `# Scenario: 消した後に前の段の入れたものが残れば、配る zip を入れずに落ちる`・
  `# Scenario: 塞ぎを外せなければ、配る zip を入れずに落ちる`（`show rule` が規則を返し続ける偽の `netsh.exe`。インストーラの呼び出しが 0 回）。
  加えて、実地の口が Task 1 の判定の関数を全部呼ぶことを `declare -f <実地の関数>` の中の名前で見る（1 つでも欠ければ `die`。実地の印 1 つが判定を束ねるので、呼び忘れを `comm` は捕まえない）
  検証: `tools/release/soak-install.sh --self-test` rc=0 / `bash -c 'test "$(tools/release/soak-install.sh --list | wc -l)" -eq 2'` rc=0
- [x] 2.2 `.github/workflows/ci.yml` を直す: `install-windows` の、`update-apply-install.sh` の手順の後に置く段の手順を足し（`timeout-minutes: 15`。D8（仮））、
  `tools/release/soak-install.sh package/package-windows-release/ashiato-server-*.zip package/package-windows-release/ashiato-collector-windows-*.zip | tee "$RUNNER_TEMP/soak.log"` を走らせ、
  `--list` と走った `== Scenario:` の行を `comm -23` で突き合わせて欠けがあれば落とす（ほかの 3 つの台本と同じ形）。`install-windows` の job に `timeout-minutes: 100`（D8（仮））。
  D18 の注記（`ci.yml:607-608`「この規則は job の終わりまで外さない」）を「置く段（`soak-install.sh`）で外すまで外さない」に直す。`rust` の job の `update-apply-install.sh --self-test` の隣に `./tools/release/soak-install.sh --self-test` を足す。
  自己試験に CI の定義を読む確かめを足す。読むのは `$root/.github/workflows/*.yml` の全部で、手順の `run:` の命令だけを数える（`#` で始まる注記の行は除く。`ci.yml:255` の注記にも `--ignored` の字面がある）。
  どれも、置き場に作った写しを 1 か所だけ壊すと落ちることを見る:
  `# Scenario: 置く段は、インストールと更新の試験の手順より後に走る`・
  `# Scenario: 更新係の外向きを塞ぐ手順は、最初のインストールの試験の手順より前のままである`・
  `# Scenario: 置く段の手順と、Windows のインストールの試験の job に時間の上限がある`・
  `# Scenario: インストールと更新の試験と置く段の 4 つの台本の Scenario の突き合わせは残る`（`install-check.sh`・`update-check-install.sh`・`update-apply-install.sh`・`soak-install.sh` の 4 つの `--list` と `comm -23`）・
  `# Scenario: 網に出る試験を走らせる手順は、握手の試験の 1 つだけである`・
  `# Scenario: 更新係の外向きの塞ぎを外す手順は、置く段の 1 つだけである`
  検証: `tools/release/soak-install.sh --self-test` rc=0 /
  `bash -c 'f="$1"; { actionlint -ignore SC2155 "$f" || docker run --rm -v "$PWD:/repo" -w /repo rhysd/actionlint:1.7.7 -ignore SC2155 "$f"; }' _ .github/workflows/ci.yml` rc=0
  （SC2155 は `package-windows` の手順（`ci.yml:487`）に今ある 2 件で、この change の範囲の外。ほかの指摘は新しいものとして落とす）/
  `bash -c 'grep -q "soak-install.sh --self-test" .github/workflows/ci.yml && grep -q "soak-install.sh --list" .github/workflows/ci.yml && grep -q -- "--block-outbound" .github/workflows/ci.yml'` rc=0 /
  終了の条件: PR の CI の `install-windows` が、置く段を含めて緑（下流の手元では走らない。赤のときは design D12・D15）

## Task 3: 文書

- [x] 3.1 `docs/testing.md` の CI の job の表の `install-windows` の行に、最後の置く段（配る形を外につながる状態で入れて 5 分置き、起こし直し・止まり・同じ誤りの続き・panic・配布元に確かめた記録を見る）を足し、
  「release の台本」の節に `tools/release/soak-install.sh` を足す（観測の部品を使うこと・自己試験は `rust` の job）
  検証: `bash -c 'grep -q "soak-install.sh" docs/testing.md && grep -q "置く段" docs/testing.md'` rc=0

## Task 4: 全体

- [ ] 4.1 Scenario の印の突き合わせと、台本の検査
  検証: `python3 scripts/check_scenarios.py . ch22-soak-install-online-ci` rc=0 / `tools/release/soak-install.sh --self-test` rc=0 /
  `tools/release/install-check.sh --self-test` rc=0 / `tools/release/update-check-install.sh --self-test` rc=0 / `tools/release/update-apply-install.sh --self-test` rc=0 /
  `bash -c 'git diff --name-only origin/main...HEAD | grep -Ev "^(tools/release/soak-install\.sh|\.github/workflows/ci\.yml|docs/testing\.md|docs/changes/CH22\.md|openspec/changes/ch22-soak-install-online-ci/.*)$" | grep -v "^docs/briefs/" | { ! grep -q .; }'` rc=0（予定のパスの外に触れていない）

## 人間の確認待ち

なし（画面も本人の操作も変わらない。置く段は PR の CI が機械で見る）。
置く段は緑の回でも、サービスごとの起こし直しの回数・置く間の誤りの行・システムの記録を読めたか・`outage_notice_unavailable` の行の有無・確かめた記録の行を出すので、
確かさ 1 だった前提（P2 の後半・P7・P8・P11）の観測は PR の CI のログに残る。`premises.md` への写しは、この change の Task にしない（PR の後にしか通らない検証を持つ Task には、今のグラフでは誰も印を付けない）
