# CH13 の前提（外の形式・振る舞い）

この change は `tools/workbox.json` を置くだけで、その意味は harness2 の `scripts/workbox.py`（`feat/workbox`。2026-11-08 時点で
harness2 の main に未 merge）が決める。前提はその読み方と、ashiato2 の台本・検証コマンドの実物との突き合わせ。

| 前提 | 何が依るか | 確かめ方 | 結果 | 出所 | 確かさ |
|---|---|---|---|---|---|
| `HARNESS_WORKBOX` が既定 `off` で、`off` の間は `tools/workbox.json` を読まない | 「置いただけでは何も変わらない」（壊してはいけないもの） | harness2 `feat/workbox:scripts/workbox.py` の `mode()` と `enabled()` を読む | `mode()` は未設定で `off`、`enabled()` は `off` なら設定を読まずに None | `.harness2` `feat/workbox:scripts/workbox.py`（`def enabled`） | 2 |
| `docker` / `host_only` の正規表現は**検証コマンドの文字列**に `re.search` で当たる（コマンドが呼ぶ台本の中身は見ない） | どの検証に docker を渡すか | `feat/workbox:scripts/workbox.py` の `_match_any` / `needs_docker` を読む | `re.search(p, cmd)`。cmd は検証コマンドの字面 | `feat/workbox:scripts/workbox.py:118-129` | 2 |
| `docker` の欄に挙げた 22 の台本と `prep_files` の 6 つは、ashiato2 に実在する | 名前の打ち間違いで docker が渡らない・準備の検査が空振りする | `test -e tools/<名前>.sh` を全部に当てた | 全部ある | 実物（この worktree、2026-11-08） | 4 |
| **画面の e2e（`cd web && npm run test:e2e`）は docker を使う** | 画面を持つ Story の検証が箱の中で通るか | `web/playwright.config.ts` の `webServer.command` を読み、`tools/stack.sh` の `docker compose` を読む | `webServer.command` は `bash ../tools/stack.sh up`、`stack.sh` は `docker compose up -d --wait db`。検証コマンドの字面には `tools/stack.sh` が出ないので、**依頼の正規表現では docker が渡らない** | `web/playwright.config.ts:49-50`、`tools/stack.sh:99` | 3 |
| 画面の e2e を検証にする tasks は多い | 上の前提の影響の広さ | `grep -c test:e2e openspec/changes/**/tasks.md` | archive 済み 21 行、走っている `st56-collector-connection-setup` の 5.x | 実物 | 4 |
| **workflow の検査は `actionlint` が無ければ `docker run rhysd/actionlint` に退く** | 走っている st53・st56 系の検証が箱の中で通るか | ホストに `actionlint` があるか（`command -v actionlint`）と tasks の検証の字面 | ホストに無い。`st53-license-bundle` の tasks 196・216 行などが `actionlint "$f" \|\| docker run …` で、字面に `docker` の正規表現が当たらない | 実物 | 4 |
| `tools/smoke.d/` `seed.d/` `check-immutable.d/` の節は親の台本から読まれ、直に検証コマンドにはならない | `.d` の台本が docker を使っていても漏れにならないか | `tools/smoke.sh:1143` などの `for part in tools/smoke.d/*.sh` | 親（`smoke.sh` `seed.sh` `check-immutable.sh`）から読まれる。親は正規表現に当たる | 実物 | 3 |
| Android の道具（JDK・SDK・Robolectric の jar）は箱に見える場所にある | gradle の単体テストが箱の中で通るか | `tools/agent-env.sh` の値と workbox.py の `TOOLS_RO` / `CACHES_RW` | `~/.local/opt/jdk21`・`~/.local/opt/android-sdk`（`~/.local/opt` は ro）・`~/.m2/robolectric-jars`（`~/.m2` は rw） | 実物 + `feat/workbox:scripts/workbox.py`（`TOOLS_RO`） | 2 |
| harness2 の main に `feat/workbox` は入っていない | 完了の判定の `workbox.py run … pgrep` が走るか | `git merge-base --is-ancestor feat/workbox origin/main`（`.harness2`） | 入っていない（2026-11-08） | 実物 | 4 |

失われるもの（`loss`）のある決定は無い: このファイルは `HARNESS_WORKBOX` が `off` の間は読まれず、中身はいつでも書き直せる。

## 追記（独立レビュー R6・R9 の後）

| 前提 | 何が依るか | 確かめ方 | 結果 | 出所 | 確かさ |
|---|---|---|---|---|---|
| 確かめた harness2 の版は `feat/workbox` の `b4797fbbb`（2026-11-08 にこの worktree で読んだ先頭）。`load_config` は知らない鍵を黙って無視する | 鍵の名前が merge までに変わると欄が黙って効かなくなる | `git -C .harness2 rev-parse --short feat/workbox`・`load_config` を読む | 鍵を検査しない | `feat/workbox:scripts/workbox.py`（`def load_config`） | 2 |
| 画面の e2e 1 本が検証の箱の 6g に収まる | `memory` の値 | 測っていない | — | — | 1 |
| `all` で 5 本のとき（agent 4g×5 + 検証 6g×2 = 32g）に空きが足りる | `agent_memory`・`parallel_verify` | 測っていない（測ったのは `tests` の cargo test 5 本だけ） | — | 依頼の「確かめたこと」 | 1 |
| agent の箱の中で agent 自身が走らせる cargo のビルドが 4g に収まる | `agent_memory` | 測っていない | — | — | 1 |

確かさ 1 の行には決定を載せない（どれも `all` で有効にする前に測る項目。Q1 の宿題に入れる）。

## 追記（第 1 回の答えの後。spec --answers、2026-11-08）

前提の変化: harness2 の `feat/workbox` は main へ merge 済み（本人の Q3 の補足）。上の表の「main に入っていない」の行は古い。

| 前提 | 何が依るか | 確かめ方 | 結果 | 出所 | 確かさ |
|---|---|---|---|---|---|
| harness2 の main に `feat/workbox` が入っている | Task 3・4 が `.harness2/scripts/workbox.py` を呼べるか | `git -C .harness2 merge-base --is-ancestor a1ef6bcf5 origin/main` | 入っている（#152、2026-11-08 16:27 JST）。読んだ main は `1df37e05b` | 実物 | 4 |
| `tools/workbox.json` の 9 個の鍵が、main の `workbox.py` の冒頭の説明に `"<鍵>"` で出る | 欄が黙って効かなくならないか（R9） | 説明の文字列に鍵を当てた | 9 個とも出る（`cpus` `rw` `ro` は使っていない） | `.harness2/scripts/workbox.py` の冒頭 | 4 |
| `host_only` は `docker` より先に見る（両方に当たればホスト） | 足した `host_only` が docker の欄より勝つか | `evidence.py` の `run_one` を読む | `in_box = … not workbox.host_only(…)` の後で `needs_docker` | `.harness2/scripts/evidence.py:398-405` | 2 |
| 中継から来た依頼は、`prep_files` のうち main にも木にも無いファイルを「違う」と読まない | `docker-compose.override.yml`（追跡しない・無いことが多い）を足して断られないか | `changed_from_main` を読む | main の版が無ければ `want` は空、木に無ければ `have` も空で、等しい | `.harness2/scripts/evidence.py:518-529` | 3 |
| `workbox.py run` は `HARNESS_WORKBOX` を見ずに箱を起こし、`-t` を付けない | Task 3.1 が端末なしの検証で走るか | `main()` と `wrap` を読み、走らせた | `env -u HARNESS_WORKBOX HARNESS_WORKBOX_CONFIG=<D1 の中身> python3 .harness2/scripts/workbox.py run . -- bash -c 'pgrep -fc /'` → `1`、rc=0、1 秒 | 実物（この機械、2026-11-08） | 5 |
| D1 の正規表現は、`match-cases.json` の 35 行で期待どおりに分かれる | Q3 の当たり方の確かめ | `workbox.host_only` → `needs_docker` を import して当てた | 35 行とも一致。依頼の版では grep だけの行と e2e などの行が外れる | 実物 | 4 |
| 既存の検証コマンド 1,781 本の分かれ方 | D6（仮）の錨で何が外れ、何が足されるか | `evidence.items` → `declared` → `unwrap_quiet` で集めて両方の版に当てた | 依頼の版: ホスト 18・docker 209・箱 1,554 / D1: ホスト 35・docker 299・箱 1,447。依頼の版で docker が渡り D1 で渡らない 34 本は、どれも grep・`bash -n`・写した先の台本の呼び出し | 実物 | 4 |
| `server_startup` の試験のうち docker を直に呼ぶのは `st51_bin_migrate_attempt_survives_restore` だけで、既存の検証コマンドの絞り込みでそれに当たり、D1 で docker が渡らないものは無い | D6（仮）の残り | `grep -n 'Command::new("docker")\|pg_dump-docker' crates/server/tests/server_startup.rs` と、1,781 本の `cargo test` の絞り込みを試験の名前に当てた | 1904・1949・1973 行（同じ試験の中）。当たるものは 0 本 | 実物 | 4 |
| D1 の中身を置いた木で、リポジトリの検査 6 本が rc=0 | 壊してはいけないもの 3 | 一時に置いて走らせ、消した | 6 本とも rc=0 | 実物（この worktree） | 4 |
| `tools/db-roles.sql` は追跡されている。`docker-compose.override.yml` は追跡されず、木にあれば `env-init.sh` が使う | `prep_files` の足し算が空振りしないか | `git ls-files`・`tools/env-init.sh:24` | `db-roles.sql` はある。override は無いのが普通（R5 の意図どおり、置いた木でだけ効く） | 実物 | 4 |

## 追記（第 2 回の合意の違和感の後。spec、2026-11-08）

前提の変化: harness2 #161（issue #157）が main に入り、作業箱の形が変わった。上の表の `docker` の正規表現・`prep_files`・`needs_docker`・「`load_config` は知らない鍵を黙って無視する」の行は古い。

| 前提 | 何が依るか | 確かめ方 | 結果 | 出所 | 確かさ |
|---|---|---|---|---|---|
| harness2 の main に #161 が入っている | この版の D1・Task 3・4 | `git -C .harness2 log --oneline -2` | `d65ff3b4a Merge pull request #161 from rai-256/feat/workbox-dind`。harness2 #157 は CLOSED（`gh issue view 157 -R rai-256/harness2`） | 実物 | 4 |
| `load_config` は知らない鍵で止まり、`docker` と `prep_files` は止めずに「もう効かない」と出す | D1 で 2 つの鍵を消す理由・Task 3.2 | `.harness2/scripts/workbox.py` の `KEYS` `RETIRED` `load_config` を読み、D1 の中身を `load_config(text=…)` で読んだ | D1 の 8 個の鍵は全部 `KEYS` の中・`RETIRED` に当たる鍵は無い。読み込みは警告なし | 実物 | 4 |
| 箱に入れるかは `host_only` だけで決まる（`needs_docker` は無い） | D8・Task 4 | `evidence.py` の `in_box = … not workbox.host_only(…)` と `grep -n needs_docker .harness2/scripts/*.py`（0 件） | `host_only` に当たればホスト、それ以外は箱（箱の中で準備と検証が走る） | `.harness2/scripts/evidence.py:398` | 4 |
| `docker-compose.yml` の DB のイメージは `postgres:17-alpine` | `docker_images` の値 | `grep -n image: docker-compose.yml` | `image: postgres:17-alpine` | `docker-compose.yml:3` | 4 |
| `workbox.py run` は `docker` を呼ばない限り dockerd を立てない | Task 3.1 の数が 10 未満のまま（違和感 (4)） | D1 の中身を `HARNESS_WORKBOX_CONFIG` で指し、`env -u HARNESS_WORKBOX … workbox.py run . -- bash -c 'pgrep -fc /'` を走らせた | `1`、rc=0、1 秒（`postgres:17-alpine` の tar は `~/.cache/harness2-workbox/images` に save 済み）。`docker images` を先に呼ぶと dockerd が立ち、数は `4` | 実物（この機械） | 5 |
| **箱の中の dockerd はホストの `PATH` に `/usr/sbin` が無いと立たない** | 有効にした日に docker を使う検証と準備が落ちる（D9（仮）・enable-before の 2.） | このセッションの `PATH`（`/usr/sbin` 無し）で `workbox.py run . -- bash -c 'docker images'`、`PATH=$PATH:/usr/sbin:/sbin` を足して同じもの。`systemctl --user show hx-dev.service -p Environment` | 無し: `failed to create NAT chain DOCKER: iptables not found`（rootlesskit の子が exit 1）。足す: `postgres:17-alpine` が出て rc=0、12 秒。`hx-dev.service` の `PATH` は `…:/usr/local/bin:/usr/bin:/bin` で `/usr/sbin` が無い | 実物（この機械） | 5 |
| `host_only` だけの判定で、`match-cases.json` の 35 行が期待どおりに分かれる | D8・Task 4.1 | `workbox.host_only` を import して D1 と依頼の版に当てた | D1: 35 行とも一致（host 6・box 29）。依頼の版: 足した `host_only` の 4 行が外れる | 実物 | 4 |
| 既存の検証コマンド 1,786 本の分かれ方 | Task 4.2 の件数の目安 | `evidence.items` → `declared` → `unwrap_quiet` で集め、`host_only` だけで分けた | D1: ホスト 35・箱 1,751 / 依頼の版: ホスト 18・箱 1,768 | 実物 | 4 |
