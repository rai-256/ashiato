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
