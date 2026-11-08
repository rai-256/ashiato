# 作業箱を有効にする前の宿題（CH13）

`tools/workbox.json` は、依頼の版に第 1 回の深掘りの答え（Q1 (1)）の直しを当てて置いた（design D1）。`HARNESS_WORKBOX` を `tests` か `all` にする前に、
ここに挙げたものを測るか、harness2 の issue が閉じたかを見る。どれも既定（off）の間は効かない（CH13 の深掘り Q1 (3)。design D2）。

出所: 上流の調べ（`premises.md`）と深掘りの独立検査（`review/deep.md` の R1〜R10）。確かめた harness2 は main の `1df37e05b`（2026-11-08。`feat/workbox` は #152 で merge 済み）。

## 1. 測っていない（`all` で 5 本を走らせる前に）

- [ ] 画面の e2e 1 本が検証の箱の 6g に収まるか（release のビルド + Chromium。`web/playwright.config.ts:57` の待ちは 10 分。その間、席 1 つを握る）（R6）
- [ ] `all` で 5 本のときの空きメモリ（agent 4g × 5 本 + 検証 6g × 2 = 32g。WSL の VM と同じ大きさ。ホストの langgraph・DB の容器はこの外。箱の `/tmp` の tmpfs も箱のメモリに数えられる）（R6）
- [ ] agent の箱の中で agent 自身が走らせる cargo のビルドが 4g に収まるか（R6）

手元で確かめ済み（依頼の「確かめたこと」。`tests`）: 箱の中で cargo・node・JDK・psql・claude・codex が動く・`cargo test -p ashiato-server --lib access_log_tests` と `tools/check-migrations.sh`・`tools/check-pg-major.sh` が PASS・worktree 5 本の cargo test が席 2 つで順に流れて空きメモリの最低 12GB。

## harness2 の issue

根本は harness2 側で直す（Q1 (2)。design D7（仮））。閉じるまでは、下の残りがそのまま効く。番号は下流の Task 5.2 が書く。

- 作業箱: docker を渡すかを検証コマンドの字面で決めている —— 中で docker を使う台本を呼ぶ台本・新しい書き方の検証が漏れる。CH13 は錨と名前の足し算で、今ある検証コマンドの分だけ塞いだ（design D6（仮））。後の Story が新しく書く検証で同じ漏れが起きうる（CH13 の Q2 で、後の Story に知らせるものは置かないと決めた。design D5）
- 作業箱: load_config が知らない鍵を黙って無視する —— CH13 では 9 個の鍵が今の main の説明に出ることを確かめた（tasks 3.2）。後で名前が変わると黙って効かなくなる（R9）
- 作業箱: apt を変えるたびに古いイメージが溜まる —— 1GB 級のイメージが残り、古いタグを消す処理が無い（R10）

## 分類の一覧

既存の `openspec/**/tasks.md` の検証コマンドを、harness2 と同じ取り出しと判定で、ホストで走る / 箱で docker を渡す / 箱で docker を渡さない に分けた一覧は `command-classes.md`（下流の Task 4.2 が書き出す。件数もそこに写す）。
2026-11-08 の上流の走査（1,781 本）: ホスト 35・docker 299・箱 1,447。依頼の版ではホスト 18・docker 209・箱 1,554 だった。

## この change で済ませたこと（一覧から消したもの）

- docker が要るのに渡らない（画面の e2e・`cargo test --workspace` と `server_startup`・`actionlint`・中で docker を使う台本を呼ぶ 3 本）→ `docker` の欄に足した（R2）
- ホストが要るのに `host_only` に当たらない（`tools/release/build.sh`・`install-check.sh`・`android-side-by-side.sh`・`usage-volume.sh`）→ `host_only` に足した（R3）
- 当たりすぎ（台本の名前を grep するだけの検証に docker の socket が渡る）→ 呼び出しの位置に錨を置いた（R4）
- `prep_files` の不足（`tools/db-roles.sql`・`docker-compose.override.yml`）→ 足した（R5）
- 鍵の突き合わせ（R9）→ tasks 3.2。依頼の完了の判定の pgrep（R1）→ tasks 3.1（`HARNESS_WORKBOX` を外したシェルから。上流の試しでは `1`）
- 当たる・当たらないの確かめ → `match-cases.json` と tasks 4.1
