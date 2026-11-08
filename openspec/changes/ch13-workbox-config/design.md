# CH13 design — 作業箱（コンテナ隔離）の設定を置く

> 第 1 回の深掘りの推奨の答えで先に書き、2026-11-08 に本人の答え（`deep.md` の Q1〜Q4。4 問とも推奨と違う）を `spec --answers Q1,Q2,Q3,Q4` で写した。
> 本人の答えで決まった D2〜D5 は（仮）を外した。答えを形にするために AI が決めた細部は D6〜D8（仮）。

## Context

動機は `proposal.md` の Why。いまの形（`premises.md` の表。2026-11-08 の repo と harness2 の main `1df37e05b`）:

- harness2 の作業箱は `HARNESS_WORKBOX` が `tests`（検証だけ）か `all`（agent も）のときだけ、repo の `tools/workbox.json` を読む。未設定は `off` で、設定を読まない（`premises.md` 1 行目）
- 箱が検証に docker を渡すかは、検証コマンドの**字面**に `docker` の欄の正規表現が `re.search` で当たるかで決まる。コマンドが中で呼ぶ台本は見ない。`host_only` も同じ当て方で、先に見る（当たればホストで走らせ、`docker` の欄は見ない。`.harness2/scripts/evidence.py:398-405`）
- harness2 の `feat/workbox` は 2026-11-08 07:27Z に main へ merge 済み（harness2 #152、`a1ef6bcf5`）。`scripts/workbox.py` は `.harness2` から呼べる。`load_config` は知らない鍵を検査しない
- ashiato2 の中に `workbox` の語を読むものは無い（`git grep -i workbox` が `openspec/` と `docs/changes/CH13.md` の外で 0 件。2026-11-08）
- D1 の中身を一時に置いて、リポジトリの検査 6 本（`check-private` `check-secrets` `check-spdx` `check-shared-appends` `check-tmp-hygiene` `check-boundaries`）が rc=0（2026-11-08、この worktree。置いたファイルは消した）
- 既存の `openspec/**/tasks.md` の検証コマンド 1,781 本を harness2 の取り出し（`evidence.declared`）で集め、依頼の版と D1 の版で分けた（`premises.md` の追記）

## Goals / Non-Goals

**Goals:**
- `tools/workbox.json` が D1 の中身（依頼の版に Q1 (1) の直しを当てたもの）のとおりにあり、JSON として読める
- 直した正規表現が、harness2 と同じ判定で、当たるべき検証に当たり、当たってはいけない検証に当たらないことを機械で確かめる（Q3）
- off の間、ashiato2 の振る舞いが何も変わらない（読むものが無い・ほかのファイルを変えない・リポジトリの検査が通る）
- 箱を有効にする前に測ることの一覧と、根本を直す harness2 の issue への案内が、merge の後も repo に残る（Q1 (2)(3)）
- pgrep の確かめ（依頼の完了の判定）をこの change の中で済ませる（Q3）

**Non-Goals:**
- harness2 側（`scripts/workbox.py`・`evidence.py`・イメージの片付け）を直すこと。issue を立てるまで（Q1 (2)。D7（仮））
- 後の Story に箱の制約を知らせる規約・検査（Q2。D5）
- 正典の仕様と文書（`tools/*.sh`・`docs/testing.md`・`AGENTS.md`・`CLAUDE.md`）を変えること（Q4。D4）
- 測っていないメモリの前提（e2e 1 本が 6g・`all` で 5 本・agent の cargo が 4g）をこの change で測ること（Q1 (3)。宿題に残す）

## Decisions

### D1. 中身は依頼の版に Q1 (1) の直しを当てたもの。正本はこの節の JSON

- 本人が依頼の「書き換えない」を外した（Q1）。置く中身は「依頼の版に Q1 (1) の直しを当てたもの」（Q4 の補足）。**正本はこの節の JSON の塊**で、下流は塊を機械で取り出して `tools/workbox.json` に書く（Task 1.1。写し間違いを避ける）。
  依頼（`docs/changes/CH13.md` の「依頼（逐語）」）は逐語のまま残し、書き換えない
- 依頼の版から変えてよいのは `docker` `host_only` `prep_files` の 3 つの欄だけ。ほかの欄（`apt` `memory` `tmp_size` `env` `parallel_verify` `agent_memory`）は依頼の版と同じ。
  `host_only` と `prep_files` は依頼の版の後ろに足すだけ。Task 1.2 がこの 3 つを機械で確かめる
- 字の並び（空白・改行・鍵の順）までは比べない。harness2 は JSON として読むだけ

依頼の版からの差分:

| 欄 | 依頼の版 | この版 | 出所 |
|---|---|---|---|
| `docker` | 台本の名前だけ（錨なし）3 本 | 全部に呼び出しの位置の錨（D6（仮））を置いた 6 本 | Q1 (1)・review/deep.md R4 |
| `docker` の台本 | 22 本 | `load-repeat` `st25-seed-check` `check-st23-derived-erasure` を足した 25 本 | Q1 (1)・R2 |
| `docker` の型 | —— | 画面の e2e（`npm run test:e2e`・`npx playwright test`）・`cargo test --workspace` / `--test server_startup` / 絞り込みなしの `-p ashiato-server` / 裸の `cargo test`・`actionlint`・`docker` を直に呼ぶもの | Q1 (1)・R2。D6（仮） |
| `host_only` | 4 本 | `tools/release/(build\|install-check)\.sh` `tools/android-side-by-side\.sh` `tools/usage-volume\.sh` を後ろに足した 7 本 | Q1 (1)・R3 |
| `prep_files` | 6 本 | `tools/db-roles.sql` `docker-compose.override.yml` を後ろに足した 8 本 | Q1 (1)・R5 |

```json
{
  "apt": [
    "postgresql-client",
    "libnss3",
    "libatk1.0-0t64",
    "libatk-bridge2.0-0t64",
    "libcups2t64",
    "libdrm2",
    "libxkbcommon0",
    "libxcomposite1",
    "libxdamage1",
    "libxfixes3",
    "libxrandr2",
    "libgbm1",
    "libasound2t64",
    "libpango-1.0-0",
    "libcairo2",
    "fonts-noto-cjk"
  ],
  "memory": "6g",
  "tmp_size": "4g",
  "docker": [
    "(^|[;&|(]\\s*|\\$\\(\\s*|-c\\s+['\"]\\s*)((bash|sh|setsid|exec|time|nice|timeout\\s+\\S+|env(\\s+-u\\s+\\S+)*)\\s+|[A-Za-z_]\\w*=\\S*\\s+)*(\\./)?tools/(smoke|seed|db|dev|stack|env-init|verify-env|docker-tidy|db-roles|pg_dump-docker|archive-shape|record-env|load-repeat|st25-seed-check|st23-compaction-volume)\\.sh",
    "(^|[;&|(]\\s*|\\$\\(\\s*|-c\\s+['\"]\\s*)((bash|sh|setsid|exec|time|nice|timeout\\s+\\S+|env(\\s+-u\\s+\\S+)*)\\s+|[A-Za-z_]\\w*=\\S*\\s+)*(\\./)?tools/check-(db-secret|exposure|pg-major|db-version-stop|panic-log|verify-env-reset|immutable|test-db-growth|test-db-killed|st23-derived-erasure)\\.sh",
    "(^|[;&|(]\\s*|\\$\\(\\s*|-c\\s+['\"]\\s*)((bash|sh|setsid|exec|time|nice|timeout\\s+\\S+|env(\\s+-u\\s+\\S+)*)\\s+|[A-Za-z_]\\w*=\\S*\\s+)*(npm\\s+run\\s+test:e2e|npx\\s+playwright\\s+test)\\b",
    "(^|[;&|(]\\s*|\\$\\(\\s*|-c\\s+['\"]\\s*)((bash|sh|setsid|exec|time|nice|timeout\\s+\\S+|env(\\s+-u\\s+\\S+)*)\\s+|[A-Za-z_]\\w*=\\S*\\s+)*cargo\\s+test\\b([^;&|'\"]*\\s--(workspace|test\\s+server_startup)\\b|\\s*($|[;&|)]|[0-9]?>|['\"](\\s|$|[;&|)]))|\\s+-p\\s+ashiato-server\\s*($|[;&|)]|[0-9]?>|['\"](\\s|$|[;&|)])))",
    "(^|[;&|(]\\s*|\\$\\(\\s*|-c\\s+['\"]\\s*)((bash|sh|setsid|exec|time|nice|timeout\\s+\\S+|env(\\s+-u\\s+\\S+)*)\\s+|[A-Za-z_]\\w*=\\S*\\s+)*actionlint\\b",
    "(^|[;&|(]\\s*|\\$\\(\\s*|-c\\s+['\"]\\s*)((bash|sh|setsid|exec|time|nice|timeout\\s+\\S+|env(\\s+-u\\s+\\S+)*)\\s+|[A-Za-z_]\\w*=\\S*\\s+)*docker\\s+[a-z]"
  ],
  "host_only": [
    "cmd\\.exe",
    "powershell\\.exe",
    "tools/android-emulator\\.sh",
    "connected\\w*AndroidTest",
    "tools/release/(build|install-check)\\.sh",
    "tools/android-side-by-side\\.sh",
    "tools/usage-volume\\.sh"
  ],
  "env": {
    "CARGO_BUILD_JOBS": "4"
  },
  "parallel_verify": 2,
  "agent_memory": "4g",
  "prep_files": [
    "tools/verify-env.sh",
    "tools/env-init.sh",
    "tools/ports.sh",
    "tools/db-roles.sh",
    "tools/docker-tidy.sh",
    "docker-compose.yml",
    "tools/db-roles.sql",
    "docker-compose.override.yml"
  ]
}
```

### D2. 有効にする前の宿題の一覧は change の中のファイルに置き、PR 本文と issue にも残す（Q1 (3)）

- 一覧の正本は `openspec/changes/ch13-workbox-config/enable-before.md`（archive の後は `openspec/changes/archive/<日付>-ch13-workbox-config/enable-before.md` に移って残る）。
  載せるのは測るものだけ（e2e 1 本が 6g に収まるか・`all` で 5 本のときの空き・agent の箱の中の cargo が 4g に収まるか）と、harness2 の issue への案内（D7（仮））。
  **D1 で直した項目は一覧から消す**（Q1 (3) の補足）。pgrep の確かめと鍵の突き合わせは Task 3 で済ませるので、一覧には結果だけを残す
- PR 本文（`docs/briefs/CH13-pr.md`。上流も下流も同じファイルを PR 本文にする。`publish.sh`）に、一覧への案内と、Task 3 の pgrep の結果を載せる（Task 5.1）
- ashiato2 の issue は下流の Task 5.3 が 1 つ立てる（題 `作業箱を有効にする前の宿題（CH13）`、本文は `enable-before.md`）。同じ題があれば立てない。
  グラフが下流へ渡す CH13 の issue は宿題の置き場にしない: 本文は `issue_body.py` が tasks から組み直し、下流の PR 本文が `closes` を書けば merge で閉じる（`.harness2/docs/flow-gates.md` の「`closes #N` は残タスク 0 のときだけ」）
- 外向きの作用（issue を立てる）を下流の agent が持つのは Task 5.2・5.3 だけ。sandbox の外での実行の申請はしない（AGENTS.md は申請を `/dev/kvm` の検証だけに限っている）
- 下流で issue を立てられないと分かれば（gh が網に出られない）、その Task を外し、PR 本文に「merge の後に人間が立てる」1 行を置くことを A として人間に返す

### D3. pgrep の確かめを tasks に置く。harness2 は merge 済みなので止まらない（Q3）

- 本人の選択は「tasks に置き、harness2 の merge まで CH13 を止める」。補足のとおり harness2 #152 が merge 済みなので、止まらずに Task 3 で確かめる
- Task 3.1: `HARNESS_WORKBOX` を外したシェルから `python3 .harness2/scripts/workbox.py run . -- bash -c 'pgrep -fc /'` の出力が 10 未満。`workbox.py run` は `HARNESS_WORKBOX` を見ずに箱を起こす（`.harness2/scripts/workbox.py` の `main`）。
  `HARNESS_WORKBOX=tests` の下で検証として走らせると、このコマンド自身が箱に包まれ、箱の中には docker が無いので落ちる（review/deep.md R1）。**この確かめは箱が off のときだけ成り立つ**（Global Constraints が下流に `HARNESS_WORKBOX` を設定させない）。
  上流で使い捨ての設定（D1 の中身）を指して走らせ、出力は `1`、rc=0（2026-11-08。`premises.md`）
- Task 3.2: 鍵の突き合わせ（R9）。`tools/workbox.json` の鍵が全部、main の `scripts/workbox.py` の冒頭の説明に `"<鍵>"` の形で出る。上流で 9 個とも出ることを確かめた（`premises.md`）
- 正規表現の当たり方の確かめは Task 4（D8（仮））

### D4. 仕様を持たない（`skip_specs: true`）。触るのは `tools/workbox.json` と change の中の文書だけ（Q4）

- 本人の答え: 「ファイル 1 つだけ。正典にも文書にも手を入れない」は変えない。中身は D1（Q4 の補足）
- 正典の `verify-environment`（検証の準備の約束）に Requirement を足すと、Scenario の印を持つ試験か台本が要り、範囲を越える。off の間は観測できる振る舞いが無い（proposal の Capabilities）
- change の中に置くもの: `enable-before.md`・`match-cases.json`（上流が書いた当たり方の表）・`check-workbox-match.py`（下流が書く確かめの台本）・`command-classes.md`（下流が台本で書き出す分類の一覧）。どれも archive で change と一緒に移る
- 変えないもの: `tools/*.sh`（`workbox.json` 以外）・`docs/testing.md`・`AGENTS.md`・`CLAUDE.md` の構成の表・`.github/workflows/`・`openspec/specs/`。下流が変えたら Task 2.2 が落ちる

### D5. 後の Story に箱の制約を知らせるものを、この change では置かない（Q2）

- 本人の答え: 根本は Q1 (2) の harness2 の issue で直す（字面で判定しない形になれば、後の Story に制約を知らせる必要が無くなる）
- `enable-before.md` には、harness2 の issue が閉じるまで後の Story の新しい検証で同じ漏れが起きうることを、issue への案内と一緒に 1 行置く

### D6（仮）. 正規表現は呼び出しの位置に錨を置き、答えが挙げた型と同じ族を足す（Q1 (1)）

- 錨（`docker` の欄の 6 本に共通の頭）: コマンドの頭・`;` `&` `|` `(` の後・`$(` の後・`-c '` / `-c "` の後で、その後に起動の語（`bash` `sh` `setsid` `exec` `time` `nice` `timeout <秒>` `env [-u <名前>]…`）と変数の代入（`NAME=値`）が 0 個以上続き、`./` は任意。
  引数の位置（`grep … tools/record-env.sh`）・`bash -n`（構文の検査だけ）・`"$d/tools/env-init.sh"`（写した先の台本）には当たらない
- 答えが名指ししたものに加えて、同じ族を足した（答えの「JSON で直せるものは CH13 の中で直す」に従う。どれも既存の検証コマンドに実在する型）:
  - 画面の e2e の `npx playwright test`（`npm run test:e2e` と同じく `playwright.config.ts` の `webServer` が `tools/stack.sh up` を起こす。tasks に 31 行）
  - 絞り込みなしの `cargo test -p ashiato-server` と裸の `cargo test`（`server_startup` の試験も全部走る。`st51_bin_migrate_attempt_survives_restore` が docker を直に呼ぶ。根の `Cargo.toml` に `default-members` が無いので、裸の `cargo test` は `--workspace` と同じ。後に `2>&1` などが続く形も含む。review/spec.md R9。引用符は閉じのとき（後が空白・行末・`;&|)`）だけ終わりとみなし、`"$1" -- --list` のような絞り込みの引数は絞り込みなしと読まない。R10）
  - `docker` を直に呼ぶ検証（`docker network inspect` など 13 行）
- 当たり方の変化（1,781 本）: 依頼の版はホスト 18・docker 209・箱 1,554、この版はホスト 35・docker 299・箱 1,447。依頼の版で docker が渡り、この版で渡らなくなった 34 本は、どれも台本の名前を grep するか `bash -n` で構文を見るだけ（`premises.md` の追記）
- 残るもの: 絞り込みつきの `cargo test -p ashiato-server <名前>` が `st51_bin_migrate_attempt_survives_restore` に当たると docker が要る。既存の検証コマンドに当たるものは無い（2026-11-08 に走査）。根本（字面で決める）は D7（仮）の issue
- 反転条件: harness2 が字面でなく実行する台本で判定する形になったら、錨と族の足し算を外して台本の名前だけに戻す。箱を有効にした後に、錨で外れた検証が docker を要ると分かったら（Task gate が箱の中で落ちる）、その型を錨に足す

### D7（仮）. harness2 の issue は根本の 3 つを 1 つずつ立てる（Q1 (2)）

- 下流の Task 5.2 が `rai-256/harness2` に 3 つ立てる。題は `作業箱: docker を渡すかを検証コマンドの字面で決めている`・`作業箱: load_config が知らない鍵を黙って無視する`・`作業箱: apt を変えるたびに古いイメージが溜まる`。
  同じ題の issue が既にあれば立てない。本文は ashiato2 での実測（`enable-before.md` と `review/deep.md` の R2・R4・R9・R10 の根拠）と、この change の PR への案内
- 1 つずつにするのは、直す場所（`workbox.py` の判定・`load_config`・イメージの片付け）が別で、閉じる時期も別だから
- 立てた番号は `enable-before.md` の「harness2 の issue」の節に書く
- 反転条件: 本人が 1 つにまとめたいと言えば、同じ本文を 1 つの issue に束ねる。harness2 側で同じ題の issue が先に立っていれば、それに ashiato2 の実測を足すだけにする

### D8（仮）. 当たり方の確かめは change の中の表と台本で、harness2 の関数をそのまま呼ぶ（Q3）

- 表 `openspec/changes/ch13-workbox-config/match-cases.json`（上流が書いた）: コマンドと、期待する行き先（`host` ホストで走る / `docker` 箱で docker を渡す / `box` 箱で docker を渡さない）の組。
  答えが挙げた 3 つの族（docker が要るコマンドが当たる / 台本の名前を grep するだけのコマンドが当たらない / powershell・エミュレータを使う台本が `host_only` に当たる）を全部持つ
- 台本 `openspec/changes/ch13-workbox-config/check-workbox-match.py`（下流が書く）: `.harness2/scripts` の `workbox.host_only` と `workbox.needs_docker` を import して、`evidence.py` と同じ順（`host_only` が先）で行き先を決める。
  正規表現を写し直さない（harness2 と同じ判定でなくなる）
- 分類の一覧 `command-classes.md`（台本の `--list` が書き出す）: 既存の `openspec/**/tasks.md` の検証コマンドを `.harness2/scripts/evidence.py` の取り出し（`items` → `declared` → `unwrap_quiet`）で集め、行き先ごとに並べる。`enable-before.md` から案内する
- 台本が表を本当に見分けることも確かめる: 依頼の版の JSON（`docs/changes/CH13.md` の逐語から取り出す）に当てると落ちる（Task 4.1）
- 反転条件: harness2 が判定の関数の名前を変えたら、台本の import を追従させる。ashiato2 に同じ確かめが要る change が続くなら、台本を `tools/` へ移す（そのときは Q4 の範囲を本人に問い直す）

## Risks / Trade-offs

- [harness2 の鍵の名前が後で変わると、このファイルの欄は黙って効かなくなる] → Task 3.2 が今の main で突き合わせる。根本は D7（仮）の issue（`load_config` の検査）
- [錨が狭すぎて、箱を有効にした後に docker が要る検証が渡らない] → 当たり方の表（D8（仮））と 1,781 本の分類で確かめた範囲だけが担保。未知の書き方は D6（仮）の反転条件で足す
- [宿題の一覧が読まれないまま有効にされる] → 有効にするのは本人が環境変数を変える操作で、ashiato2 の側に止める口は無い。一覧は PR 本文の先頭と issue に置く（D2）
- [下流の agent が gh で網に出られない（Codex の sandbox など）] → gh の接続失敗の文言は verify-run の環境の欠落の判定（`.harness2/scripts/evidence.py` の `INFRA_SIGNS`）に当たらず、5.2・5.3 の証跡は BLOCKED でなく FAIL になる。直しても通らないので、fix は D2 の最後の行を A として人間に返す
- [Task 3.1 は docker と作業箱のイメージを要る] → 下流の検証の環境に docker が無ければ、`workbox.py` の `Unavailable` で落ちる。上流のこの機械では 1 秒で通った（イメージは作ってあった）
- [下流が D1 の JSON を写し間違える] → Task 1.2 の比較が落ちる

## Migration Plan

ファイルを足すだけ。戻すときはファイルを消す（off の間は何も読まないので、消しても振る舞いは変わらない）。
