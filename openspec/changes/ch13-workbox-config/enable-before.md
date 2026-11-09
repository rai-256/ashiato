issue: #275

# 作業箱を有効にする前の宿題（CH13）

`tools/workbox.json` は、依頼の版に第 1 回の深掘りの答え（Q1 (1)）と第 2 回の合意の違和感 (1) の直しを当てて置いた（design D1）:
`docker` と `prep_files` を消し、`docker_images`（`postgres:17-alpine`）を足し、`host_only` に 3 本を足した。
`HARNESS_WORKBOX` を `tests` か `all` にする前に、ここに挙げたものを測るか、harness2 の issue が閉じたかを見る。どれも既定（off）の間は効かない（CH13 の深掘り Q1 (3)。design D2）。

出所: 上流の調べ（`premises.md`）と深掘りの独立検査（`review/deep.md` の R1〜R10）。確かめた harness2 は main の `d65ff3b4a`（2026-11-08。#152 と #161 が merge 済み）。

## 1. 測っていない（`all` で 5 本を走らせる前に）

- [ ] 画面の e2e 1 本が検証の箱の 6g に収まるか（release のビルド + Chromium + 箱の中の dockerd と DB の容器。`web/playwright.config.ts:57` の待ちは 10 分。その間、席 1 つを握る）（R6）
- [ ] `all` で 5 本のときの空きメモリ（agent 4g × 5 本 + 検証 6g × 2 = 32g。WSL の VM と同じ大きさ。ホストの langgraph はこの外。箱の `/tmp` の tmpfs と、箱の中の dockerd が立てる DB の容器も箱のメモリに数えられる）（R6）
- [ ] agent の箱の中で agent 自身が走らせる cargo のビルドが 4g に収まるか（R6）

手元で確かめ済み（依頼の「確かめたこと」。`tests`、harness2 #161 の前の形）: 箱の中で cargo・node・JDK・psql・claude・codex が動く・`cargo test -p ashiato-server --lib access_log_tests` と `tools/check-migrations.sh`・`tools/check-pg-major.sh` が PASS・worktree 5 本の cargo test が席 2 つで順に流れて空きメモリの最低 12GB。
#161 の形（箱の中の dockerd）では、まだ測っていない。

## 2. 上流で見つけたこと

有効にする前に直すものは、今は無い。見つけた 1 件は harness2 #164 で直り、下の「済んだもの」に移した。

## harness2 の issue

上流で見つけた件（design D9（仮））。同じ題の issue が既にあったので立てず、その番号を書く。issue は CLOSED（#164 で直った）。

- rai-256/harness2#165 作業箱: ホストの PATH に /usr/sbin が無いと箱の中の dockerd が iptables を見つけられず立たない —— 上の 2.

## harness2 #157 / #161 で済んだもの

第 1 回の深掘りで一覧に載せ、harness2 #157（issue。CLOSED）・#161（2026-11-08 merge）・#164（#165 を直した）で要らなくなったもの。CH13 では直さず、issue も立てない（design D7）:

- ホストの `PATH` に `/usr/sbin` が無いと、箱の中の dockerd が `iptables not found` で立たない → harness2 #165（CLOSED）。#164 で `workbox/dockerd-up` が `PATH` の前に `/usr/local/sbin:/usr/sbin:/sbin` を足すようになった。再現（2026-11-08、`premises.md`）は直る前の形: `PATH` に `/usr/sbin` が無いシェルから `workbox.py run . -- bash -c 'docker images'` → `failed to create NAT chain DOCKER: iptables not found`
- docker が要るのに渡らない（画面の e2e・`cargo test --workspace` と `server_startup`・`actionlint`・中で docker を使う台本を呼ぶ台本）→ 検証の箱は箱の中の dockerd を持ち、`docker` を呼べば立つ（R2）
- 当たりすぎ（台本の名前を grep するだけの検証にホストの docker の socket が渡る）→ ホストの socket は渡さない（R4）
- `prep_files` の不足（`tools/db-roles.sql`・`docker-compose.override.yml`）→ 検証の準備は箱の中で走り、main の版との突き合わせは要らない（R5）
- 後の Story の新しい検証で同じ漏れが起きる（字面の判定）→ docker の字面の判定が無くなった。残る字面の判定は `host_only` だけ（design D5）
- `load_config` が知らない鍵を黙って無視する → 知らない鍵で止まる。`docker` と `prep_files` は「もう効かない」と出る（R9。tasks 3.2 が今の main で確かめる）
- `apt` を変えるたびに古いイメージが溜まる → 7 日より古い作業箱のイメージを消す（R10）

## ホストで走らせる台本（`host_only` に足した分）

`host_only` は字面の判定のまま。依頼の版の 4 本に、ホストが要る台本 3 本を足した（R3）: `tools/release/(build|install-check).sh`（中で `powershell.exe` を呼ぶ）・`tools/android-side-by-side.sh`・`tools/usage-volume.sh`（エミュレータを自分で起こす）。
当たる・当たらないは `match-cases.json` と tasks 4.1 で確かめる（依頼の版ではこの 4 行が外れる）。後の Story がホストの要る台本を新しい名前で書くと箱に入るので、そのときは `host_only` に名前を足す（design D5）。

## 分類の一覧

既存の `openspec/**/tasks.md` の検証コマンドを、harness2 と同じ取り出しと判定（`host_only`）で、ホストで走る / 箱で走る に分けた一覧は `command-classes.md`（下流の Task 4.2 が書き出す。件数もそこに写す）。
2026-11-08 の上流の走査（1,786 本）: ホスト 35・箱 1,751。依頼の版ではホスト 18・箱 1,768 だった。
下流の Task 4.2 の書き出し（1,826 本。この change の tasks.md を含む）: ホスト 34・箱 1,792（HEAD 80faa50cc）。

## この change で済ませたこと

- 鍵の確かめ（R9）→ tasks 3.2（今の main の `load_config` で読め、もう効かない鍵を持たない）
- 依頼の完了の判定の pgrep（R1）→ tasks 3.1（`HARNESS_WORKBOX` を外したシェルから。`docker` を呼ばないので dockerd は立たない。上流の試しでは `1`）
- `host_only` の当たる・当たらないの確かめ → `match-cases.json` と tasks 4.1
