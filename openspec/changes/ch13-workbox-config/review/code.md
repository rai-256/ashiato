<!-- 処置の判定: re-review -->
# code_verify — CH13 `ch13-workbox-config`（2026-11-09）

確かめた木: `feat/ch13-workbox-config` の HEAD `f6b238393`（作業ツリーは clean）。`HARNESS_WORKBOX` を外した環境から実行。
壊して確かめたもの（`tools/workbox.json`・`docker-compose.yml`・未追跡のファイル 1 つ）は全部元に戻し、前と後で `git status --porcelain` と `git diff` が同じであることを毎回確かめた。

## 申告と実測

申告: `[x]` 12/14（5.2・5.3 は `[ ]`）。検証コマンドは tasks.md から `evidence.items` → `declared` → `unwrap_quiet` で取り出した 14 本。

| 項目 | 申告 | 検証コマンド（要点） | 実測 | 一致 |
|---|---|---|---|---|
| 1.1 | [x] | `json.load(tools/workbox.json)` | rc=0 | 一致 |
| 1.2 | [x] | D1 の塊と等しい・依頼の版から 4 つの欄だけ | rc=0 | 一致 |
| 2.1 | [x] | `! git grep -i workbox`（許した範囲の外） | rc=0 | 一致 |
| 2.2 | [x] | merge-base からの差分と未追跡が許した範囲の中 | rc=0 | 一致 |
| 2.3 | [x] | 検査 6 本 | rc=0（`境界 OK` まで） | 一致 |
| 2.4 | [x] | enable-before.md の測るもの 3 つ・節 | rc=0 | 一致 |
| 3.1 | [x] | `workbox.py run . -- bash -c "pgrep -fc /"` < 10 | rc=0、`pgrep: 1` | 一致 |
| 3.2 | [x] | `load_config` で読め、`RETIRED` を持たない | rc=0 | 一致 |
| 4.1 | [x] | `check-workbox-match.py` rc=0・`--request` rc=3・35 行以上 | rc=0（`--request` は 31/35 で rc=3） | 一致 |
| 4.2 | [x] | command-classes.md の 2 節・1,700 行以上 | rc=0 | 一致 |
| 5.1 | [x] | PR 本文に案内と `pgrep: <数>` | rc=0 | 一致 |
| 5.2 | **[ ]** | enable-before.md の harness2#165 の題を gh で見る | **rc=0** | **不一致**（R1） |
| 5.3 | **[ ]** | open の #275 と `issue: #275` の行 | **rc=0** | **不一致**（R1） |
| 5.4 | [x] | PR 本文に 2 つの issue の番号 | rc=0 | 一致 |

ほかに実行して一致したもの:
- `openspec validate ch13-workbox-config --strict` rc=0（`skip_specs` で delta 0）
- `gh issue view 275`: 題 `作業箱を有効にする前の宿題（CH13）`・OPEN。本文と `enable-before.md` の違いは先頭の `issue: #275` と空行の 2 行だけ（diff で確かめた。書き直しの後の版が issue に載っている）
- `gh issue view 165 -R rai-256/harness2`: 題が 5.2 のものと同じ・CLOSED（2026-11-09T00:02:06Z）
- `tools/workbox.json` の末尾は `}\n`（改行 1 つ。1.1 の本文の条件。検証コマンドはここを見ていない）

## 手 1: 固定値を独立に再計算する

- **分類の件数**（command-classes.md の `ホスト 34・箱 1792`）: 台本（`check-workbox-match.py`）も `workbox.host_only` も使わず、`tools/workbox.json` の `host_only` を自分で `re.search` に掛けて数え直した → `host 34 box 1792 計 1826`。表の行数 `grep -c "^- openspec/"` も 1826。一致（取り出しは harness2 と同じ `evidence.py` を使っているので、取り出し自体は独立でない）
- **`docker_images` の `postgres:17-alpine`**: `docker-compose.yml:3` の `image: postgres:17-alpine` と一致。`.harness2/scripts/workbox.py:205-225` がこの鍵を読み、ホストで pull・save して渡す（受理されるだけで効かない鍵ではない）。ただし compose の版との結び付きを固定するテストは無い（R2）
- **pgrep の閾値 10 が区別できること**: 箱の中は `1`、同じ時刻のホストの `pgrep -fc /` は `62`。3.1 はホストのプロセスを見たら落ちる
- **#164 で直ったという主張**（enable-before.md「済んだもの」の 1 行目）: `PATH` から `/usr/sbin` `/sbin` `/usr/local/sbin` を外したシェル（`grep -c sbin` が 0）から `workbox.py run . -- bash -c 'docker images'` → `docker_ok`・rc=0・そのときの `pgrep -fc /` は `4`。`.harness2/workbox/dockerd-up:27` が `PATH` の前に `/usr/local/sbin:/usr/sbin:/sbin` を足している。主張どおり

## 手 2: ガードをわざと壊す

`tools/workbox.json` を壊した複製を置いて各検証を走らせた（全部元に戻した。`復元: 同じ`）:

| 壊し方 | 1.2 | 3.2 | 4.1 |
|---|---|---|---|
| `memory` を `8g` に | rc=1（D1 と違う） | rc=0 | rc=0 |
| `docker` 鍵を足す | rc=1 | rc=1（もう効かない鍵: docker） | — |
| 知らない鍵 `hostonly` を足す | — | rc=1（ValueError: 知らない鍵） | — |
| `host_only` から `tools/usage-volume\.sh` を消す | rc=1 | — | rc=3（34/35） |
| 鍵名を `host-only` に変える | — | rc=1 | rc=3（29/35） |
| `host_only` に `.` を足す（全部ホストへ） | — | — | rc=3（6/35） |

- 2.2: 範囲の外に未追跡の `tools/zz-ch13rv.txt` を置く → rc=1（そのファイル名を出す）
- 3.1: 上のとおりホストの数は 62 で、閾値の外
- 数値の欄（`memory` 等）を守るのは 1.2 だけで、1.2 は D1 の文書との一致を見る。設定だけの change なのでそれで足りる（手 4 の注記）

検査の外側: `host_only` が拾うべきホストの台本の網羅は、`match-cases.json` に載せた行しか見ない（R3）。

## 手 3: Scenario と test

該当なし（確かめた範囲: `skip_specs: true` で Scenario が無い。`python3 scripts/check_scenarios.py . ch13-workbox-config` rc=0・`scenarios: OK`。出た `[warn]` はどれも CH13 の外の既存の印）。

## 手 4: 本人の決定が test で固定されているか

- 違和感 (1)(5)「変えてよいのは 4 つの欄・ほかは依頼の版のまま」: 1.2 が依頼の逐語の JSON と比べて固定（`memory` を変えると rc=1 を確かめた）
- `docker_images: ["postgres:17-alpine"]`: 1.2 が文字列で固定。compose の版とは結ばれていない（R2）
- `host_only` の 3 本の追加: 1.2（並びまで）と 4.1（当たり方。1 本消すと rc=3）が固定
- Q3 pgrep < 10: 3.1 が固定（箱 1・ホスト 62）
- Q4 触る範囲: 2.1・2.2 が固定（範囲の外のファイルで rc=1）
- `memory 6g`・`agent_memory 4g`・`parallel_verify 2`・`CARGO_BUILD_JOBS 4` は 1.2 の文書との一致でだけ固定され、振る舞いの試験は無い。測るのは enable-before.md の宿題（測るもの 3 つ）で、設定だけの change なので指摘にしない

## 手 5: tasks の `[x]` と実体

`[x]` 12 本は全部、コマンドが実在し rc=0（上の表）。4.1 の台本 `check-workbox-match.py` と `match-cases.json`（35 行）は実在する。`cargo test --test` の形の検証は無い。
逆向きの不一致が 2 本（`[ ]` なのに通る）→ R1。

## 手 6: 隙間

- 捨てて戻らない経路: 無い（確かめた範囲: off の間 ashiato2 の側でこのファイルを読むものが無い（2.1 rc=0）・置くのは設定 1 つで、記録を持つコードに触れていない（2.2 rc=0））
- 箱に入ったときに「緑のまま空になる」型: `host_only` の漏れが 1 本あった → R3

## 手 7: 機械が承認した検証コマンドの訂正

該当なし（`python3 scripts/plan_fix.py pending openspec/changes/ch13-workbox-config` → `[]`）。

---

## R1. 5.2・5.3 が `[ ]` のままだが、検証は通り実体もある
- 成果物: `openspec/changes/ch13-workbox-config/tasks.md` / `openspec/changes/ch13-workbox-config/enable-before.md` / `docs/briefs/CH13-pr.md`
- 根拠: 5.2・5.3 の検証コマンドを走らせて両方 rc=0。evidence.jsonl にも implement（730220eaf）と fixer（df6c1e90c）の 2 回で PASS。実体: `enable-before.md` に `- rai-256/harness2#165 作業箱: …`（gh で題が一致・CLOSED）、冒頭に `issue: #275`（gh で OPEN、本文は enable-before.md と同じ）。それでも `tasks.md` の 5.2・5.3 は `- [ ]`。`git show e342c083a`（「Task 5 を完了にする」）は 5.1 と 5.4 だけを `[x]` にしている。5.4 の本文は「5.2・5.3 の後に閉じる」で、`docs/changes/CH13.md` の完了の判定も 5.2・5.3 を挙げる
- kind: technical
- 提案: 5.2・5.3 を `[x]` にする（同じ題の issue があったので立てなかった、という扱いは tasks の本文が認めている）。わざと開けたのなら、理由を tasks か PR 本文に 1 行書く。どちらにしても final の前に印と実体を揃える

## R2. `docker_images` の `postgres:17-alpine` が compose の版と結ばれておらず、compose を上げても全部の検査が通る
- 成果物: `tools/workbox.json` / `openspec/changes/ch13-workbox-config/check-workbox-match.py` / `docker-compose.yml`
- 根拠: 今は `docker-compose.yml:3` の `image: postgres:17-alpine` と一致する。複製で `docker-compose.yml` を `postgres:18-alpine` に変えて 1.1・1.2・3.2・4.1 を走らせると、4 本とも rc=0（元に戻して `復元: 同じ`）。`workbox.py:205-225` は `docker_images` の版をホストで save して箱に渡すので、ずれると先に渡すイメージが使われず、箱の中の dockerd が DB を立てるたびに pull に戻る。落ちる・記録が消える型ではない（遅くなるだけで、知らせが無い）
- kind: technical
- 提案: `check-workbox-match.py`（か tasks に 1 本）で「`docker_images` の各項が `docker-compose.yml` の `image:` に在る」を見る。範囲（Q4）で今は足さないなら、`enable-before.md` に「compose の版を上げたら `docker_images` も直す」を 1 行足す

## R3. `host_only` の網羅に漏れがある: `tools/release/sandbox-probe.sh` は箱の中で「確かめられない」を書いて rc=0 で通る
- 成果物: `tools/workbox.json` / `openspec/changes/ch13-workbox-config/match-cases.json` / `openspec/changes/ch13-workbox-config/enable-before.md`（「ホストで走らせる台本」）
- 根拠: `grep -lE "powershell\.exe|cmd\.exe|/mnt/c/|emulator -avd|android-emulator\.sh|\badb\b|wsl\.exe" tools/**/*.sh` で出た台本のうち、箱の側の検証として走らせているのは `tools/release/sandbox-probe.sh`（`openspec/changes/archive/2026-11-07-st55-server-installer/tasks.md` 3.0。command-classes.md の「箱で走る」にある）。中身は `powershell.exe` で Windows サンドボックスを起こす（`sandbox-probe.sh:8,17,53`）。箱の中で走らせると `command -v powershell.exe` rc=1 → `P16=UNAVAILABLE` `P19=UNAVAILABLE` を書いて `probe_rc=0`（結果の置き場は箱の /tmp に向けた。作業ツリーは変わらず）。上流の R3 で足した 3 本（`release/(build|install-check)`・`android-side-by-side`・`usage-volume`）と同じ型の台本だが、`host_only` と `match-cases.json` のどちらにも無い。
  影響は小さい: 呼ぶのは archive 済みの 1 本だけで、今の木の検証は変わらない。また今の harness のシェルではホストでも `powershell.exe` が `PATH` に無く（interop は enabled・`/mnt/c/Windows/System32/WindowsPowerShell/v1.0/powershell.exe` は在る）、ホストで走らせても同じ UNAVAILABLE になる
  （同じ grep で出た `tools/verify-prep-device.sh --self-test` は偽の adb を使う self-test で、箱でよい。ch10 の `real-proof.sh` は本物の SDK と kvm を使うが、箱の中に `/dev/kvm` と `$ANDROID_HOME/emulator/emulator` と `adb` が在ることを確かめたので指摘にしない）
- kind: daily
- 提案: `host_only` に `tools/release/sandbox-probe\.sh` を足し、`match-cases.json` に `want: host` の 1 行を足す（足すなら 1.2 の期待する並びも直す）。今は足さないなら、`enable-before.md` の「ホストで走らせる台本」に漏れとして 1 行置く（design D5 の「後で名前を足す」運用に乗せる）

## security review（baf9253..a3597d3）

見た経路:
- 差分の実体は `tools/workbox.json`（作業箱の設定）1 つと、change の文書・台本だけ。サーバ・画面・収集アプリ・移行・記録を持つコードには触れていない（`git diff --stat` で 8 ファイル、コードは 0）（2）
- 設定の読み元: 箱の中の agent から中継で来た依頼 → `evidence.py:391-404` → `box_config`（`evidence.py:508-515`）が **main の版**の `tools/workbox.json` を `git show` で読む → `host_only` に当たればホストで走らせず BLOCKED_INFRA。worktree の設定を書き換えても効かない（4）
- `host_only` の判定: `workbox.py:138-143` は `re.search`（字面のどこかに当たればホスト）。今の設定で `bash tools/usage-volume.sh` / `cat ~/.config/gh/hosts.yml  # cmd.exe` / `echo connectedXAndroidTest; env` は host、`cargo test -p ashiato-server` / `tools/smoke.sh` は box（台本で確かめた）。ホストに出るのは中継を通らない（グラフの Task gate の）実行だけで、そちらは `off` の今と同じくホストの権限で走る（4）
- `env`: 足すのは `CARGO_BUILD_JOBS=4` だけ。`rw` / `ro` / `env_deny` は置いていない → 既定の許可リスト（`workbox.py:243-283`。`$HOME` 丸ごとは渡さず、`GH_TOKEN` `SSH_AUTH_SOCK` 等は `ENV_DENY`）から広がらない（3）
- `docker_images: postgres:17-alpine`: ホストで pull・save して箱の dockerd に渡す（`workbox.py:205-225`）。既存の `docker-compose.yml:3` と同じタグで、新しい取得元は増えない（2）
- `apt`: 16 包みは Ubuntu の本家の名前（`t64` 付きは 24.04 以降の改名）。新しい apt の取得元・鍵は足していない（2）
- 新しい listen・平文の通信・証明書の検証を切る設定: 無い（設定の鍵に該当するものが無い。`KEYS` は `workbox.py:62-63`）（2）

安全の決め手: 「箱の中の agent から来た検証は、設定を main の版で読み、`host_only` に当たるものはホストで走らせない」（`evidence.py:391-404`）—— 確かさ 4。
`cd .harness2 && python3 -m pytest tests/test_workbox.py -k Relayed -v` → `test_config_edited_in_the_box_is_ignored` / `test_host_only_is_never_run_from_the_box` / `test_edited_prep_never_runs_on_the_host` を含む 5 本 PASSED。
CH13 の後に入れる予定の `HARNESS_WORKBOX=tests` では agent は箱に入らないので、この設定で agent の届く範囲は `off` の今から広がらない（`workbox.py:enabled` の `what == "agents" and m != "all"`）。

指摘なし

範囲外: `.harness2/tests/test_workbox.py::Relay::test_inside_verify_run_goes_through_the_relay_and_keeps_rc` がこの worktree の深いパスで `OSError: AF_UNIX path too long` で落ちる（中継の socket を worktree の中に置くため。harness2 側。`all` で深いパスの worktree だと中継が立たない）。
範囲外: `all` のとき Task gate がホストで取り直す `host_only` の台本（`tools/usage-volume.sh` 等）は箱から書ける worktree にあり、中身はホストで走る。ハーネスは「事故の隔離で、わざと抜けようとする agent への境界ではない」と明記している（`workbox.py:487-491`）。
