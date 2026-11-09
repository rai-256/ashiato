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
- 処置: rejected: 5.2・5.3 が `[ ]` なのは正常。検証が `gh issue` を呼ぶので `.harness2/scripts/tasks_phase.py:36` の `NEEDS_PR` が「PR・gate」の工程に分け（`python3 scripts/tasks_phase.py openspec/changes/ch13-workbox-config/tasks.md` → `PR・gate 済 0 / 未 2 未: 5.2 5.3`・`実装 済 12 / 未 0`）、`task_mark.py:10,36` は impl の項目だけを `[x]` にする（`pr` は `[ ]` のまま残すのが正常と明記）。settle は `[x]` を書けない（settle.md「書いてはいけないもの」）。実体は揃っている: 5.2・5.3 の検証は今の木で rc=0（fix report）、evidence.jsonl にも 2 回 PASS。`pr_body.py` が残した `pr` の項目を理由つきで PR 本文に出す
- 判定: ADDRESSED（re-review 1 @1dcfb0980909c38eb8911f2d75a06489b28e8981）
- 判定: ADDRESSED（re-review 2 @2ac898b7ad11f759b459507b9037143af4910342）

## R2. `docker_images` の `postgres:17-alpine` が compose の版と結ばれておらず、compose を上げても全部の検査が通る
- 成果物: `tools/workbox.json` / `openspec/changes/ch13-workbox-config/check-workbox-match.py` / `docker-compose.yml`
- 根拠: 今は `docker-compose.yml:3` の `image: postgres:17-alpine` と一致する。複製で `docker-compose.yml` を `postgres:18-alpine` に変えて 1.1・1.2・3.2・4.1 を走らせると、4 本とも rc=0（元に戻して `復元: 同じ`）。`workbox.py:205-225` は `docker_images` の版をホストで save して箱に渡すので、ずれると先に渡すイメージが使われず、箱の中の dockerd が DB を立てるたびに pull に戻る。落ちる・記録が消える型ではない（遅くなるだけで、知らせが無い）
- kind: technical
- 提案: `check-workbox-match.py`（か tasks に 1 本）で「`docker_images` の各項が `docker-compose.yml` の `image:` に在る」を見る。範囲（Q4）で今は足さないなら、`enable-before.md` に「compose の版を上げたら `docker_images` も直す」を 1 行足す
- 処置: fixed 4.1
- 判定: ADDRESSED（re-review 1 @1dcfb0980909c38eb8911f2d75a06489b28e8981）

## R3. `host_only` の網羅に漏れがある: `tools/release/sandbox-probe.sh` は箱の中で「確かめられない」を書いて rc=0 で通る
- 成果物: `tools/workbox.json` / `openspec/changes/ch13-workbox-config/match-cases.json` / `openspec/changes/ch13-workbox-config/enable-before.md`（「ホストで走らせる台本」）
- 根拠: `grep -lE "powershell\.exe|cmd\.exe|/mnt/c/|emulator -avd|android-emulator\.sh|\badb\b|wsl\.exe" tools/**/*.sh` で出た台本のうち、箱の側の検証として走らせているのは `tools/release/sandbox-probe.sh`（`openspec/changes/archive/2026-11-07-st55-server-installer/tasks.md` 3.0。command-classes.md の「箱で走る」にある）。中身は `powershell.exe` で Windows サンドボックスを起こす（`sandbox-probe.sh:8,17,53`）。箱の中で走らせると `command -v powershell.exe` rc=1 → `P16=UNAVAILABLE` `P19=UNAVAILABLE` を書いて `probe_rc=0`（結果の置き場は箱の /tmp に向けた。作業ツリーは変わらず）。上流の R3 で足した 3 本（`release/(build|install-check)`・`android-side-by-side`・`usage-volume`）と同じ型の台本だが、`host_only` と `match-cases.json` のどちらにも無い。
  影響は小さい: 呼ぶのは archive 済みの 1 本だけで、今の木の検証は変わらない。また今の harness のシェルではホストでも `powershell.exe` が `PATH` に無く（interop は enabled・`/mnt/c/Windows/System32/WindowsPowerShell/v1.0/powershell.exe` は在る）、ホストで走らせても同じ UNAVAILABLE になる
  （同じ grep で出た `tools/verify-prep-device.sh --self-test` は偽の adb を使う self-test で、箱でよい。ch10 の `real-proof.sh` は本物の SDK と kvm を使うが、箱の中に `/dev/kvm` と `$ANDROID_HOME/emulator/emulator` と `adb` が在ることを確かめたので指摘にしない）
- kind: daily
- 提案: `host_only` に `tools/release/sandbox-probe\.sh` を足し、`match-cases.json` に `want: host` の 1 行を足す（足すなら 1.2 の期待する並びも直す）。今は足さないなら、`enable-before.md` の「ホストで走らせる台本」に漏れとして 1 行置く（design D5 の「後で名前を足す」運用に乗せる）
- 処置: fixed D10 仮
- 判定: ADDRESSED（re-review 1 @1dcfb0980909c38eb8911f2d75a06489b28e8981）
- 判定: ADDRESSED（re-review 2 @2ac898b7ad11f759b459507b9037143af4910342）

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

## /code-review（baf9253..982802c）

見た: Claude Code の組み込みの `/code-review`（8 件）。写しは scripts/code_review_record.py

## R4. Tasks 5.2 and 5.3 are still `[ ]`, yet 5.4, whose text says it closes after 5.2 and 5.3, is `[x]`. Both 5.2 and 5.3 already pass their checks and their results exist (harness2#165, #275).
- 成果物: openspec/changes/ch13-workbox-config/tasks.md:64
- 根拠: The completion check in CH13.md lists 5.2 and 5.3. With those two still `[ ]`, the final or merge gate reads the change as unfinished, or treats 5.4 as closed out of order. Commit e342c083a ('Task 5 を完了にする') ticked only 5.1 and 5.4, so the checkboxes no longer match the actual state.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。settle が直し方を決める）
- 処置: rejected: R1 と同じ。`[ ]` の 5.2・5.3 は `gh issue` を呼ぶので `tasks_phase.py:36` が「PR・gate」の工程に分け、`task_mark.py:10` は impl だけを `[x]` にする（`pr` は `[ ]` のまま残すのが正常）。5.4 は gh を呼ばないので impl で、検証（PR 本文に 2 つの番号）は今の木で rc=0。5.2・5.3 の実体（harness2#165・#275）は先に在り、5.4 が順を飛ばして閉じたのではない
- 判定: ADDRESSED（re-review 1 @1dcfb0980909c38eb8911f2d75a06489b28e8981）

## R5. `host_only` does not list `tools/release/sandbox-probe.sh`. That script calls powershell.exe to start the Windows sandbox, the same kind of host-only script as the three entries just added.
- 成果物: tools/workbox.json:22
- 根拠: With HARNESS_WORKBOX=tests, st55 task 3.0 (`tools/release/sandbox-probe.sh p19`) is routed to the box (command-classes.md:1659). There `command -v powershell.exe` fails, the script writes P16/P19=UNAVAILABLE and exits rc=0, so the check passes green without probing anything. match-cases.json has no row for this script either, so check-workbox-match.py cannot catch the gap.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。settle が直し方を決める）
- 処置: rejected: 漏れそのものは本当（R3 と同じ）だが、`tools/workbox.json` に足すのは本人の決定に反する。tasks.md の Global Constraints の 1 つ目「足りないものに気付いたら、ファイルを直さずに PR 本文に挙げ、`enable-before.md` に 1 行足す」と違和感 (5)（tasks 1.2 が `host_only` は依頼の版に 3 本を足しただけであることを固定）。R3 で design D10（仮）として `enable-before.md` の「ホストで走らせる台本」と `docs/briefs/CH13-pr.md` に挙げ、反転条件（本人が足すと言えば D1・ファイル・1.2・match-cases を同じ commit で直す）を書いた
- 判定: ADDRESSED（re-review 1 @1dcfb0980909c38eb8911f2d75a06489b28e8981）

## R6. `ROOT = Path(__file__).resolve().parents[3]` assumes the script sits at `openspec/changes/<change>/`. After archive it moves to `openspec/changes/archive/<date>-ch13-workbox-config/`, where ROOT becomes `openspec/` and the script breaks.
- 成果物: openspec/changes/ch13-workbox-config/check-workbox-match.py:19
- 根拠: Rerunning the archived 4.1/4.2 checks, or calling this script to regenerate command-classes.md, fails at `import evidence` with ModuleNotFoundError because `openspec/.harness2/scripts` does not exist. The failure happens at module load, outside the try block that maps errors to rc=1.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。settle が直し方を決める）
- 処置: fixed 4.1
- 判定: ADDRESSED（re-review 1 @1dcfb0980909c38eb8911f2d75a06489b28e8981）

## R7. `docker_images: ["postgres:17-alpine"]` is a hard-coded copy of `docker-compose.yml:3`, and no check ties the two together.
- 成果物: tools/workbox.json:36
- 根拠: If the compose image is bumped to postgres:18-alpine, checks 1.1, 1.2, 3.2 and 4.1 all still pass at rc=0 (shown by the code-verify break test). workbox.py keeps pulling and saving the old image for the box, so the box's dockerd pulls the new image on every DB start: slower runs and failures when offline, with no warning.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。settle が直し方を決める）
- 処置: rejected: R2 と同じ指摘（`docker_images` と compose の結び付きが無い）。R2 の処置で `check-workbox-match.py`（tasks 4.1）が `docker-compose.yml` の `image:` と突き合わせ、compose を `postgres:18-alpine` にすると rc=3 になる（fix report）。`tools/workbox.json:36` 自体は D1 の正本と等しくなければならず（tasks 1.2）、変える所が無い
- 判定: ADDRESSED（re-review 1 @1dcfb0980909c38eb8911f2d75a06489b28e8981）

## R8. Task 2.4 says downstream must not remove the '2. 上流で見つけたこと' section. Downstream kept the heading but replaced its only item with '今は無い' and moved the reproduction steps to the 'done' section.
- 成果物: openspec/changes/ch13-workbox-config/enable-before.md:20
- 根拠: Issue #275 was created from enable-before.md. If its body was taken before the rewrite, the issue and the file disagree. Also, check 2.4 (`grep -qF "iptables not found"`) now passes only because that text sits in the 'done' section, so the check no longer confirms the section's intended content is still there.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。settle が直し方を決める）
- 処置: fixed 2.4
- 判定: ADDRESSED（re-review 1 @1dcfb0980909c38eb8911f2d75a06489b28e8981）
- 判定: ADDRESSED（re-review 2 @2ac898b7ad11f759b459507b9037143af4910342）

## R9. `request_config` extracts the JSON block with its own line-based parser, separate from the regex in the tasks 1.2 check (`^> ?` strip plus a ```json regex). It also does not check that there is exactly one block.
- 成果物: openspec/changes/ch13-workbox-config/check-workbox-match.py:33
- 根拠: If CH13.md's verbatim-request section gains a second ```json block or a line like `>```json`, the script silently uses the first block while 1.2 fails with an assertion. The two checks then read different JSON, and the --request rc=3 result checks the wrong input.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。settle が直し方を決める）
- 処置: fixed 4.1
- 判定: ADDRESSED（re-review 1 @1dcfb0980909c38eb8911f2d75a06489b28e8981）

## R10. The comprehension calls `route(cfg, c["cmd"])` twice per case: once in the filter and again to build the tuple.
- 成果物: openspec/changes/ch13-workbox-config/check-workbox-match.py:56
- 根拠: Every pattern is matched twice per case. The cost is small, but evaluating the same check twice is redundant; compute `got` once (e.g. `[(c, g) for c in cases if (g := route(cfg, c['cmd'])) != c['want']]`).（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。settle が直し方を決める）
- 処置: fixed 4.1
- 判定: ADDRESSED（re-review 1 @1dcfb0980909c38eb8911f2d75a06489b28e8981）

## R11. The PR body says nothing needs fixing before enabling ('有効にする前に直すものは残っていない'), but enable-before.md still has three unmeasured items (whether e2e fits in 6g, free memory at 5 runs, whether cargo fits in 4g) and no note about the sandbox-probe gap.
- 成果物: docs/briefs/CH13-pr.md:17
- 根拠: A reader who sees 'nothing left to fix' in the PR body may set HARNESS_WORKBOX=tests without doing the measurements. Out-of-memory in the box, or the probe's empty green pass, then shows up in real runs.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。settle が直し方を決める）
- 処置: fixed 5.1
- 判定: ADDRESSED（re-review 1 @1dcfb0980909c38eb8911f2d75a06489b28e8981）
- 判定: ADDRESSED（re-review 2 @2ac898b7ad11f759b459507b9037143af4910342）

## final review（baf9253..96ce730）

見た: SDD の final reviewer（`requesting-code-review` の `code-reviewer.md`）。Critical・Important は無し、判定は「merge してよい（Minor は任意）」。R1〜R11 の直しは今の木で効いている。Task 1・4・5 の ⚠️ は全部確かめて成り立つ（1.2 rc=0・`--request` で外れる 4 行は足した 3 本の型とちょうど一致・5.1/5.4 の検証 rc=0）。`--list` を今の HEAD で書き出し直しても `command-classes.md` と差分なし。以下の 3 件は Task の review の Minor が手付かずのまま残っていたもの。

## R12. `docs/briefs/CH13-pr.md` に上流のときの言い回しが残り、「下流で確かめたこと」と食い違って読める
- 成果物: docs/briefs/CH13-pr.md:1
- 根拠: `docs/briefs/CH13-pr.md:1` の題が「（上流）」のまま、:15「下流が同じ本文で ashiato2 の issue を 1 つ立てる」（#275 は立っている）、:15「harness2 #157 / #161 で済んだ項目は一覧から消し、案内だけを置いた」（`enable-before.md` は「済んだもの」を節として持つ）、:20「`command-classes.md`（下流が書き出す）」、:22「上流の試しでは `1`。下流が値を書き足す」（:8 に `pgrep: 1` が既にある）。merge する人間が読む本文。Task 5 の review の F2（`.superpowers/sdd/ch13-task-5/task-5-findings.md`）が手付かず
- kind: technical
- 提案: 済んだことを過去の形に書き直し、題の「（上流）」を外す。:15 の案内は `enable-before.md` の今の節の形に合わせる
- 処置: fixed 5.1
- 判定: ADDRESSED（re-review 2 @2ac898b7ad11f759b459507b9037143af4910342）

## R13. `enable-before.md` の「harness2 の issue」の節に作業の経緯の 1 文が残っている
- 成果物: openspec/changes/ch13-workbox-config/enable-before.md:24
- 根拠: `enable-before.md:24`「同じ題の issue が既にあったので立てず、その番号を書く」は宿題の一覧を読む人に要らない作業の経緯で、issue #275 の本文にも同じ文が載っている。Task 5 の review の F3（`.superpowers/sdd/ch13-task-5/task-5-findings.md`）が手付かず
- kind: technical
- 提案: 1 文を消すか事実の形（「#165 は既存の issue」）にし、`enable-before.md` を変えたら `gh issue edit 275 --body-file` で issue の本文も揃える（5.3 の検証が本文の一致を見るなら合わせて確かめる）
- 処置: fixed 5.3
- 判定: ADDRESSED（re-review 2 @2ac898b7ad11f759b459507b9037143af4910342）

## R14. `enable-before.md` の分類の件数で、総数が増えたのにホストが 35 → 34 に減った理由が書いていない
- 成果物: openspec/changes/ch13-workbox-config/enable-before.md:54
- 根拠: `enable-before.md:53-54` は上流の走査（1,786 本）でホスト 35、下流の書き出し（1,826 本）でホスト 34。総数が 40 増えてホストが 1 減った理由が無く、上流と下流で取り出し方が違うのか、1 本が外れたのかを読み手が判断できない。Task 4 の review の F3（`.superpowers/sdd/ch13-task-4/task-4-findings.md`）が手付かず
- kind: technical
- 提案: 上流の走査と `command-classes.md` を突き合わせ、ホストから外れた 1 本（と理由）を 1 行足す
- 処置: fixed 4.2
- 判定: ADDRESSED（re-review 2 @2ac898b7ad11f759b459507b9037143af4910342）

## final review（baf9253..eaa0042）

見た: SDD の final reviewer（`requesting-code-review` の `code-reviewer.md`）の 2 回目。Critical・Important は無く、判定は「With fixes（どれも Minor）」。今の木で 1.2・2.1〜2.3・3.2・4.1 は rc=0。`--list` で書き出し直した結果は `command-classes.md` と同じ（host 34・box 1,792）。R14 の 34 は `03bd08301`・`7942f1603` で数え直して成り立った。issue #275 の本文は `enable-before.md` と揃っている。settle の 2 つの commit（1dcfb0980・2ac898b7a）が新しく壊したものは無い。Task 4 の F1（`--request` の rc=3 が、足した行が外れたことまで見ていない）は、1.2（依頼の `host_only` が今の版の先頭部分）と既定の版の 35/35 一致から論理的に担保されているので、指摘にしない。

## R15. 4.1・4.2 の証跡が、settle で書き換えた後の台本のものではない
- 成果物: openspec/changes/ch13-workbox-config/evidence.jsonl
- 根拠: `evidence.jsonl` の 4.1・4.2 の最新は head `b7d8ed62d`（台本を置いた commit）。`openspec/changes/ch13-workbox-config/check-workbox-match.py` は 1dcfb0980（R2・R6・R9・R10 の処置）で 51 行変わったが、その後の `verify-run 4.1` / `4.2` の記録が無い。`.superpowers/sdd/ch13-settle/report.md` は手で走らせた結果だけを書いている。Task gate は `openspec/` を除いた木の hash で見る（`.harness2/scripts/evidence.py:46-47`）ので、change の中の台本の書き換えでは古い証跡が捕まらない。今の木で 4.1 の検証コマンドは rc=0（reviewer が確かめた）ので害は無いが、台帳が今の台本を証明していない（確かさ 3）
- kind: technical
- 提案: `scripts/verify-run 4.1` と `scripts/verify-run 4.2` を今の木で 1 回ずつ走らせ、証跡を足す。ハーネス側の課題（settle が検証コマンドの指す台本を変えたら、その項目の取り直しを求める）は harness2 に回す
- 処置: fixed 4.1

## R16. PR 本文の「仮決め」の節に D10（仮）が無い
- 成果物: docs/briefs/CH13-pr.md:45
- 根拠: `docs/briefs/CH13-pr.md:45-48` の見出しは「仮決め（design.md の（仮）。台帳 `provisional.jsonl`）」だが、挙げているのは D9 だけ。settle 1（1dcfb0980、R3・R5 の処置）で `design.md` と `provisional.jsonl` に D10（仮）（`tools/release/sandbox-probe.sh` を `host_only` に足さず宿題に置く）が増えたのに、この節を合わせていない。同じ本文の :18 には D10 が出ているので、本文の中で数が合わない。`scripts/pr_body.py` が台帳から仮決めを出すので PR では見えるが、手書きの節と食い違って読める（確かさ 3）
- kind: technical
- 提案: :47 の下に D10（仮）の 1 行（何を決めたか・反転条件）を足す
- 処置: fixed 5.1

## R17. 上流のときの未来形の言い回しが残っている（R12 と同じ型）
- 成果物: openspec/changes/ch13-workbox-config/enable-before.md:53, docs/briefs/CH13-pr.md:56
- 根拠: `enable-before.md:53`「（下流の Task 4.2 が書き出す。件数もそこに写す）」—— 直後の :55-57 に書き出した結果が既にある。`docs/briefs/CH13-pr.md:56`「14 項目（5.4 を足した）」と :57「どれも同じ題があれば立てない」は計画のときの書き方のまま（#165 は既存、#275 は立てた）。merge する人間が読む本文（確かさ 3）
- kind: technical
- 提案: 済んだことを過去の形に直す。`enable-before.md` を変えたら `gh issue edit 275 --body-file` で issue #275 の本文も揃え、5.3 の検証で一致を確かめる
- 処置: fixed 5.3
