# CH18 spec-review（成果物どうしの整合）

対象: `openspec/changes/ch18-workbox-libs-cpus/`（proposal・tasks・.openspec.yaml・lane.json・lane-verdict.json・deep.md）と `docs/changes/CH18.md`
照らした実物: `tools/workbox.json`、`.harness2/scripts/workbox.py`（`load_config`・`run_args`・`env_flags`・`main`）、`.harness2/scripts/evidence.py`（`CMD_LIKE`・`declared`・`run`・`lint`・`unverified`）、`.harness2/scripts/lanes.py`（`bookkeeping`）、`.harness2/graph/harness_graph/effects.py`（`previous_boxes`）
前例: `openspec/changes/archive/2026-11-10-ch13-workbox-config/`（design D4・tasks 2.2）、`openspec/changes/archive/2026-11-11-ch15-collector-setup-verifier-password/`（軽い路だが `reason: bug`）
確かめた日: 2026-11-17（worktree `ashiato2-up-ch18`、HEAD 3fbafc529、`origin/main` = f62c86dd7 = merge-base）。軽い路（`lane.json` の `reason: same`）、`skip_specs: true`

## 機械の検査

| コマンド | 結果 |
|---|---|
| `openspec validate ch18-workbox-libs-cpus --strict` | valid（rc=0。`skip_specs: true` で delta 0 を受ける） |
| `python3 scripts/check_chain.py .` | chain: OK（0 件 / 未回収 0 件 / warn 0 件） |
| `python3 scripts/check_scenarios.py . ch18-workbox-libs-cpus` | scenarios: OK（warn はこの change に関わらない既存の印だけ） |
| `python3 scripts/review_triage.py . ch18-workbox-libs-cpus` | triage: OK（指摘 0 / 仮決め 0 / 要件へ戻すもの 0。この review を書く前） |
| `python3 scripts/spec_overlap.py candidates . ch18-workbox-libs-cpus` | `[]`（0 件） |
| `python3 .harness2/scripts/evidence.py lint openspec/changes/ch18-workbox-libs-cpus` | `[OK]`（雛形・`/tmp` の固定名なし） |
| `python3 .harness2/scripts/evidence.py unverified openspec/changes/ch18-workbox-libs-cpus --task 1` | **`["1.3"]`**（「検証:」があるのにコマンドを 1 つも取り出せない。R1） |
| `python3 .harness2/scripts/evidence.py unverified … --task 2` | `[]` |

検証コマンドを実際に走らせた結果（`evidence.declared` で取り出した字面を `bash -c` で）:

| 項目 | base の木（今） | 直した設定の写し |
|---|---|---|
| 1.1 | rc=1（`AssertionError: 足した apt が 2 つでない: []`） | rc=0（worktree の外に置いた直した `tools/workbox.json` に対して、`GIT_DIR` はこの repo） |
| 1.2 | rc=1（`KeyError: 'cpus'`） | rc=0（同上） |
| 1.3（tasks.md:26 の字面。`code_spans` で手で取り出した） | rc=1（出力なし） | rc=0（出力なし。字面の `env -u HARNESS_WORKBOX_CONFIG` だけを `env HARNESS_WORKBOX_CONFIG=<直した写し>` に替えた。イメージ `harness2-workbox:332d02c46d8d` は作成済みで、作り直しは起きなかった） |
| 1.3 の中身の前に `cat /sys/fs/cgroup/cpu.max` と `dpkg-query -W -f="\${Package} \${Status}\n" …` を足した別の実行 | rc=1。`max 100000`、dpkg-query は 2 つとも `no packages found` | rc=0。`600000 100000`、`libbz2-dev install ok installed`・`liblzma-dev install ok installed` |
| 2.2 | rc=0 | — |
| 2.1 | 走らせていない（15〜20 分。上流と本人の実測に頼る） | — |

1.3 の cgroup の確かめは、上流が書いたとおりに再現した（値は上の表の 2 行目の、出力を足した実行で見たもの。字面は rc しか返さない）。引用とエスケープ（`\${Status}`）も書いたまま走る。問題は字面ではなく、グラフが 1.3 を走らせないこと（R1）。

## R1. 1.3 の検証は先頭が `env` なので evidence.py が取り出さず、CPU の上限と包み（完了の判定 2・5）が証跡に残らない
- 成果物: openspec/changes/ch18-workbox-libs-cpus/tasks.md（1.3）・docs/changes/CH18.md（完了の判定 2・5）
- 根拠:
  - `evidence.py unverified … --task 1` が `["1.3"]` を返す。`CMD_LIKE`（evidence.py:68）は `./`・`tools/`・`scripts/`・`python3 ` などで始まるものだけを命令とみなし、`env -u …` は当たらない。`declared`（evidence.py:181-197）は「検証:」の後に backtick があるのに命令が無いと `[]` を返す。そのため `scripts/verify-run 1.3` は何も走らせず、Task gate も 1.3 に PASS を求められない（`cmd_unverified` の docstring、evidence.py:694-695）
  - tasks.md:25 の「1.1 の前に `scripts/verify-run 1.3` で FAIL を記録してから直してよい」は、この形では実行できない
  - 1.3 の字面は rc しか返さず、何も出力しない。完了の判定 5（CH18.md:69）は、人間が確認バッチで「PR の証跡で CPU の上限が 6 本か」を見る問いだが、証跡のログに `cpu.max` の値が出ない
  - 箱は worktree を同じパスに rw で mount する（workbox.py:257）。`CARGO_TARGET_DIR` は `tools/agent-env.sh` にも `workbox.json` の `env` にも無いので、`target/` はホストと箱で共有される。ホストには `liblzma-dev`・`libbz2-dev` が入っている（ホストで dpkg-query が 2 を返した）。worktree の `target/` がホストの組み立てで温まっていれば、2.1 の `cargo test` は箱の中でリンクし直さず、包みが無くても通りうる。このとき、base と直した版を分ける検証は 1.3 だけになる
- kind: technical
- 提案: 先頭を `CMD_LIKE` に当たる形にし、値を出してから判定する。たとえば `scripts/quiet-run cg -- env -u HARNESS_WORKBOX_CONFIG python3 .harness2/scripts/workbox.py run . -- bash -c 'cat /sys/fs/cgroup/cpu.max; dpkg-query -W -f="\${Package} \${Status}\n" liblzma-dev libbz2-dev; test "$(cat /sys/fs/cgroup/cpu.max)" = "600000 100000" && test "$(dpkg-query -W -f="\${Status}\n" liblzma-dev libbz2-dev | grep -c "install ok installed")" -eq 2'` rc=0。中身の形はこの review で箱の中で走らせて確かめた（base で rc=1、直した写しで rc=0）。evidence.py は `scripts/quiet-run <名前> --` を外して全文をログに取り（`unwrap_quiet`、evidence.py:364-367）、要約にも末尾を出すので、値が証跡に載る。別の形として、同じ中身を `openspec/changes/ch18-workbox-libs-cpus/check-box.sh` に置き、`bash openspec/changes/…/check-box.sh` を検証にしてもよい（予定のパスの中。CH13 は `check-workbox-match.py` を change の中に置いた）。この案の字面は `evidence.CMD_LIKE` に当たり、雛形も `/tmp` の固定名も無いことを確かめた（`unwrap_quiet` の後は `env -u …` から始まり、そのまま `bash -c` で走る）。直した後に `evidence.py unverified … --task 1` が `[]` になることを確かめる
- 処置: fixed 1.3 （先頭を `scripts/quiet-run cg --` にし、`cpu.max` と包みの状態を出してから判定する。`evidence.py unverified --task 1` が `[]`、base で rc=1・ログに `max 100000` と `no packages found` を確かめた）

## R2. 完了の判定 2 の「base の設定では成り立たない」は、記録が「してよい」（任意）なので証跡に残らないことがある
- 成果物: openspec/changes/ch18-workbox-libs-cpus/tasks.md（1.3）・docs/changes/CH18.md（完了の判定 2）
- 根拠: CH18.md:66 は完了の判定に「base の設定では成り立たない（tasks 1.3）」を入れている。tasks.md:25 は、1.1 の前に FAIL を記録することを「してよい」と書くだけ。`same` の路では、機械が red を求めるのは `bug` だけ（lanes.py:506-508。`red_green` を呼ぶのは `reason == "bug"` のときだけ）。なので、記録しなければ判定 2 の後半を裏付けるものは「上流が 2026-11-17 に確かめた」という文だけになる（この review でも base の箱で `max 100000` と包み無しを再現したが、証跡ではない）
- kind: technical
- 提案: R1 を直したうえで、1.3 を「1.1 の前に `scripts/verify-run 1.3` で FAIL を記録する（必須）。FAIL のログに `max 100000` と `no packages found` が出ていることを code-verify が見る」に改める。必須にしないなら、CH18.md:66 の後半を「（上流の実測。証跡は求めない）」とし、判定から外す
- 処置: fixed 1.3 （1.1 の前の FAIL の記録を必須にし、ログの中身を確かめる手順を書いた）・fixed docs/changes/CH18.md （完了の判定 2・5）

## R3. 2.1 の `export TMPDIR=$(mktemp -d)` は、verify-run が渡す一時フォルダの下にフォルダを作って残す
- 成果物: openspec/changes/ch18-workbox-libs-cpus/tasks.md（2.1）
- 根拠:
  - verify-run は検証ごとに `.superpowers/sdd/tmp/v…` を作り、`TMPDIR` で渡す（evidence.py:412・481）
  - `env_flags`（workbox.py:275-284）は `ENV_DENY`（workbox.py:90）に無い変数をすべて箱へ継ぐ。`TMPDIR` は `ENV_DENY` に無く、worktree は同じパスで mount される。なので、箱の中の `mktemp -d` は箱の tmpfs ではなく、その一時フォルダの下にフォルダを作る（このセッションでも `mktemp -d` は `.superpowers/sdd/tmp/a/tmp.…` に作られた）
  - 実測: `TMPDIR` を `.superpowers/sdd/tmp/vrev…` に向けて `env HARNESS_WORKBOX=tests python3 .harness2/scripts/workbox.py run . bash -c 'echo "$TMPDIR"; export TMPDIR=$(mktemp -d); echo "$TMPDIR"'`（2.1 と同じく `--` 無し）を走らせると、箱の中の `TMPDIR` はそのフォルダで、`mktemp -d` は `…/vrev…/tmp.JwZsqMlf0y` に作られ、rc=0 の後もホストに残った
  - 作ったフォルダは誰も消さないので、`left = sorted(os.listdir(tmp))`（evidence.py:482）が毎回 `tmp_left` を記録する。今は `HARNESS_TMP_LEAK` が未設定（`tools/agent-env.sh` に無い）なので「注意」で済む。`fail` にした日からは、rc=0 でも FAIL になる（evidence.py:501）
  - 1.3 は `env -u HARNESS_WORKBOX_CONFIG` で別の設定ファイルを外すが、2.1 は外さない。今は hx-dev.service に `HARNESS_WORKBOX_CONFIG` が無いので実害は無い。それでも、1.3 と 2.1 が違う設定の箱を見うる
  - `HARNESS_WORKBOX=tests` は CLI の `run` では効かない（workbox.py:569-596 の `main` は `mode()` を見ない）。依頼の逐語どおりなので害は無い
- kind: technical
- 提案: 2.1 に「`tmp_left` の注意（`tmp.…` が 1 つ）は、この字面から出るもので、落ちたとは数えない」と書き添える。数えたくないなら、外側の単引用符の中の先頭を `export TMPDIR=$(mktemp -d); trap "rm -rf \"\$TMPDIR\"" EXIT;` にする（この字面は `bash -c '…'` の中でフォルダを消すことをホストで確かめた。箱の中では走らせていない）。あわせて `env` に `-u HARNESS_WORKBOX_CONFIG` を足し、1.3 と同じ設定を見るようにする
- 処置: fixed 2.1 （`-u HARNESS_WORKBOX_CONFIG` と `trap "rm -rf …" EXIT` を足し、依頼の逐語から変えた 3 点を本文に書いた。trap がフォルダを消すことをホストで確かめた）

## R4. 2.1 が試験の中身で落ちたときの扱いが無い。触ってよい範囲では直せないので、実装と直しを繰り返しうる
- 成果物: openspec/changes/ch18-workbox-libs-cpus/tasks.md（2.1・Global Constraints）
- 根拠: tasks.md:31 は、落ちたらログで「組み立てのリンク」か「試験の中身」かを分けると書く。しかし扱いを書いているのは前者（PR 本文に挙げて人間に返す）だけ。tasks.md:10 の Global Constraints は試験とコードに触れることを禁じているので、試験の中身で落ちた場合（CPU を 6 本に絞ったための時間切れ、並走による揺れを含む）、Task agent が範囲の中で取れる手は無い。範囲の外を直せば `lanes.py` が通常の路へ昇格させる。直さなければ FAIL のまま fix が繰り返される。本人の実測（CH18.md:32）は 2 つの包みを足した写しで取ったもので、`cpus: 6` を置いた状態で全体が通ったかは書かれていない
- kind: technical
- 提案: 2.1 に「試験の中身で落ちたら、コードも試験も直さない。落ちた試験の名前とログの場所を PR 本文に挙げ、同じ試験がホストの base で通るか（`scripts/quiet-run` で 1 本だけ）を並べて、人間に返す」と書く。3 つ目の包みが要った場合も、同じく人間に返すことを明記する（完了の判定 1 が「2 つだけ」なので、足すかどうかは人間が決める）
- 処置: fixed tasks.md （Global Constraints に、箱の中の試験が落ちても直さず A として人間に返すことを書き、2.1 の本文も合わせた）

## R5. 2.2 は `docs/briefs/CH18*` を許していない。グラフが置く PR 本文で、走らせ直したときに落ちる
- 成果物: openspec/changes/ch18-workbox-libs-cpus/tasks.md（2.2・Global Constraints）・docs/changes/CH18.md（完了の判定 4）
- 根拠:
  - 2.2（tasks.md:34）が許すのは `tools/workbox.json`・`docs/changes/CH18.md`・`openspec/changes/ch18-workbox-libs-cpus/`・`.superpowers/` だけ
  - 前例の feat の merge は、change の外に `docs/briefs/` を持ち込んでいる。CH15 の a50a655ca は `docs/briefs/CH15-pr.md`・`CH15-deep-r1.html`、CH13 の 4471508b7 は `docs/briefs/CH13-pr.md`・`CH13-deep-r1.html`（`git diff --name-only <m>^1 <m>`）
  - `lanes.py` の `bookkeeping`（lanes.py:133-136）は `docs/briefs/` と `.superpowers/` を数えない。CH13 の 2.2 は `docs/briefs/CH13` を許していた（archive の tasks.md:36）
  - 今は `same_code`（evidence.py の `EXCLUDE`）が `docs/briefs` を除くので、PR 本文を変えても 2.2 の PASS は古い扱いにならない。しかし feat が `docs/briefs/CH18*` を commit した後に fix の回などで 2.2 を走らせ直すと、FAIL になる
- kind: technical
- 提案: 2.2 の許す範囲に `docs/briefs/CH18` を足す（`|docs/briefs/CH18`）。Global Constraints（tasks.md:10）と完了の判定 4（CH18.md:68）にも「グラフが置く `docs/briefs/CH18*` と `.superpowers/` は数えない」と添え、`lanes.py` と同じ線にする
- 処置: fixed 2.2 （`docs/briefs/CH18` を許す）・fixed tasks.md （Global Constraints）・fixed docs/changes/CH18.md （完了の判定 4）

## R6. 壊してはいけないもの 3（`HARNESS_WORKBOX` を設定しない）を、どの検証も見ていない
- 成果物: openspec/changes/ch18-workbox-libs-cpus/tasks.md（Global Constraints・2.2）・docs/changes/CH18.md（壊してはいけないもの 3）
- 根拠: CH18.md:60 と tasks.md:12 は「`HARNESS_WORKBOX` を設定しない・hx-dev.service に触れない」を求めている。しかし hx-dev.service とその drop-in（`~/.config/systemd/user/hx-dev.service.d/`）は repo の外にあり、2.2 の `git diff` / `git ls-files` には映らない。今の hx-dev.service に `HARNESS_WORKBOX` は無い（`systemctl --user cat hx-dev.service`）。作業箱が off の間は、Task agent はホストで走るので、そこに書ける
- kind: technical
- 提案: 検証にはしない。merge の後に人間が有効にすると、走らせ直したときに必ず落ちるため。代わりに、2.2 の本文に「code-review の席が、`systemctl --user cat hx-dev.service` に `HARNESS_WORKBOX` が無いこと、drop-in が増えていないことを見る」と書く。PR 本文にもその出力を 1 行載せる
- 処置: fixed tasks.md （Global Constraints に、検証にはせず最後のレビューの席が `systemctl --user cat hx-dev.service` を見る・PR 本文に 1 行載せると書いた）

## R7. 1.1 は「整形を今のファイルに合わせ、末尾に改行 1 つ」を見ていない
- 成果物: openspec/changes/ch18-workbox-libs-cpus/tasks.md（1.1）
- 根拠: tasks.md:19 は整形と末尾の改行を求めているが、1.1 の検証（tasks.md:21）は JSON として読んだ値しか比べない。今の `tools/workbox.json` は `json.dumps(json.loads(t), indent=2, ensure_ascii=False) + "\n" == t` が True。安く固定できる
- kind: technical
- 提案: 1.1 の検証の末尾に `;assert open("tools/workbox.json",encoding="utf-8").read()==json.dumps(n,indent=2,ensure_ascii=False)+"\n","整形が今のファイルと違う"` を足す。`cpus` をどこに置いても、読んだ順で並べ直すので通る

---
- 処置: fixed 1.1 （整形と末尾の改行をバイトで比べる assert を足した）

## 観点 1: deep の決定が正典に写っているか

該当なし（確かめた範囲: `deep.md` は機械が置いた軽い路の申告の写しで、「本人の答え」は無い。依頼の数値は次の検証が持つ。`cpus` 6 は 1.1・1.2、`parallel_verify` 2 は 1.1、2 つの包みは 1.1・1.2・1.3、`600000 100000` は 1.3。ただし 1.3 はグラフが走らせない（R1））。

## 観点 2: Scenario が検証可能か

該当なし（確かめた範囲: `skip_specs: true` で delta が無く、新しい Scenario は無い。完了の判定の文は、tasks の検証コマンドで観測できる形で書かれている。数値・包みの名前・パスはどれも字面で判定できる。穴は R1・R2）。

## 観点 3: 置き場の誤り

該当なし（確かめた範囲: design.md は無い。proposal.md の Capabilities は New・Modified とも「なし」で、`specs/` は無い。CH18.md の front matter は `capability: []`・`satisfies: []`。`grep -rl 'workbox\|作業箱' openspec/specs` は 0 件で、正典に作業箱の capability は無い。proposal.md:18 が引く CH13 の D4（archive の design.md:125-130）は、引用どおり「`skip_specs: true`、off の間は観測できる振る舞いが無い」と言っている）。

skip_specs と `same` の規則について: story-upstream の `--light` は、`same` で「いまの振る舞いが残ること」を Scenario で固定するよう求めている。この change で残すべき「いまの振る舞い」は、作業箱が off の間に何も変わらないことだが、ashiato2 の側で観測できるものは無い。ashiato2 の台本・コード・CI は `tools/workbox.json` を読まない（`git grep -il workbox -- . ':!tools/workbox.json' ':!openspec/changes' ':!docs'` が返すのは `.superpowers/sdd/ch13-settle/report.md`（CH13 の作業の記録）だけで、読み手ではない）。off の間にこのファイルを読むのは harness2 だけで、`effects.previous_boxes`（effects.py:575）が mode を見ずに `load_config` を呼ぶ。ここが壊れないことは 1.2（`load_config` で読める）が固定する。なので、Scenario の代わりに tasks の検証で固定する形は成り立つ。穴は 1.3 が走らないこと（R1）だけ。CH15 は `reason: bug` なので、`same` の前例にはならない（proposal は CH15 を引いていない）。

## 観点 4: tasks が検証を持つか

- 1.1・1.2・2.2: 命令と終了条件があり、base の木で落ち、直した設定で通ることを確かめた（上の表）
- 1.3: 字面は正しく走るが、グラフは取り出さない（R1）。red の記録は任意（R2）
- 2.1: 命令と rc=0 はある。依頼の逐語を `scripts/quiet-run wb --` と `env` で包んだだけで、包みは argv をそのまま渡す。quiet-run の要約と rc の扱いは evidence.py が既に扱う形（`CMD_LIKE` の `scripts/`）。残りの点は R3・R4
- `/tmp` の固定名: 無い（`evidence.py lint` が OK。2.1 の `mktemp -d` は R3）
- 予定のパスの外に触れる Task: 無い（どの Task も `tools/workbox.json` 以外を書かない）
- 依存の順: 1.1 → 1.2 → 1.3 → 2.1 → 2.2 で正しい（1.3 の red は 1.1 の前）
- 人間の確認待ち: 無い。1.3・2.1 は docker を要るが、ホストの docker で走る（Global Constraints の tasks.md:14 に書いてある）。前提は「verify の時点で hx-dev.service の作業箱が off」で、これも tasks.md:14 に書いてある。on の後に走らせ直したときの振る舞い（verify-run が 1.3・2.1 を箱に包み、その中で `workbox.py run` がもう 1 段の箱を起こそうとする）は、走らせていない推定。merge の後に有効にする順なら起きない

## 観点 5: change の依頼と完了の判定

- 「完了の判定」は空でなく、5 項とも tasks の項目を名指ししている
- 「壊してはいけないもの」と検証の対応:
  - (1) ほかの欄 → 1.1
  - (2) off の間の振る舞い → 1.2 と 2.2。ashiato2 に読み手が無い（観点 3）
  - (3) `HARNESS_WORKBOX` を設定しない → 検証なし（R6）
  - (4) 製品・試験・正典・文書 → 2.2
- 完了の判定と検証の対応:
  - 1 → 1.1・1.2
  - 2 → 1.3（走らない。R1・R2）
  - 3 → 2.1
  - 4 → 2.2（`docs/briefs` は R5）
  - 5 → 確認バッチ。証跡に値が出ない（R1）
- 「依頼（逐語）」の範囲（`apt` に 2 つ・`cpus` 6・`parallel_verify` は 2 のまま・変えるのは `tools/workbox.json` だけ・有効にするのは人間）を、tasks は越えても欠いてもいない
- A（後から変えると失われるものがある問い）は出ない。`cpus` の値も包みの追加も設定ファイルの 1 行で戻せる。skip_specs も、後から Scenario を足せば済む。失う側を具体的に示せるものは見つからなかった

## 観点 6: 答えが写ったか

該当なし（`spec --answers` の呼び出しではない。軽い路で深掘りの答えが無い）。

観点 7: 該当なし（spec_overlap.py candidates が 0 件）。走っている `ch16-install-check-ng-observations`・`ch17-windows-install-ci-reuse`・`st58-update-apply-rollback` と、`origin/docs/*-upstream`・`origin/feat/*` のどのブランチも、`origin/main` からの差分に `tools/workbox.json` を持たない（proposal.md:36 の主張と一致）。
