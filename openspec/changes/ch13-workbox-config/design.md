# CH13 design — 作業箱（コンテナ隔離）の設定を置く

> 第 1 回の深掘りの推奨の答え（Q1〜Q4）で先に書いた。
> 本人の答えが推奨と違えば、その Q の D（仮）を反転条件のとおりに書き直す。

## Context

動機は `proposal.md` の Why。いまの形（`premises.md` の表。2026-11-08 の repo と harness2 `feat/workbox` の `b4797fbbb`）:

- harness2 の作業箱は `HARNESS_WORKBOX` が `tests`（検証だけ）か `all`（agent も）のときだけ、repo の `tools/workbox.json` を読む。未設定は `off` で、設定を読まない（`premises.md` 1 行目。確かさ 2）
- 箱が検証に docker を渡すかは、検証コマンドの**字面**に `docker` の欄の正規表現が `re.search` で当たるかで決まる。コマンドが中で呼ぶ台本は見ない。`host_only` も同じ当て方で、当たればホストで走らせる（2 行目）
- harness2 の main に `feat/workbox` は入っていない。`load_config` は知らない鍵を検査しない（追記 1 行目）
- ashiato2 の中に `workbox` の語を読むものは無い（`git grep -i workbox` が `openspec/` と `docs/changes/CH13.md` の外で 0 件。2026-11-08）
- 依頼の中身を一時に置いて、リポジトリの検査 6 本（`check-private` `check-secrets` `check-spdx` `check-shared-appends` `check-tmp-hygiene` `check-boundaries`）が rc=0（2026-11-08、この worktree）

## Goals / Non-Goals

**Goals:**
- `tools/workbox.json` が依頼の中身のとおりにあり、JSON として読める
- off の間、ashiato2 の振る舞いが何も変わらない（読むものが無い・ほかのファイルを変えない・リポジトリの検査が通る）
- 箱を有効にする前に直す・測ることの一覧が、merge の後も repo に残る

**Non-Goals:**
- 依頼の欄の中身を変えること（足りないものは宿題の一覧に挙げる。依頼の指定）
- 箱の中での振る舞い（docker が渡るか・メモリに収まるか・pgrep が箱の中しか見ないか）をこの change で確かめること（D3（仮））
- 後の Story の検証コマンドの書き方の規約・検査（D5（仮））
- harness2 側（`scripts/workbox.py`・`evidence.py`・イメージの片付け）を直すこと

## Decisions

### D1. 中身は依頼の JSON と「JSON として読んで等しい」で確かめる

- 下流は `docs/changes/CH13.md` の「依頼（逐語）」の節から ```` ```json ```` の塊を取り出し（行頭の `> ` を外す）、`tools/workbox.json` と `json.load` どうしで比べる。
  依頼の本文を写し直した 2 つ目の正本を repo に置かない（依頼が唯一の正本）
- 字の並び（空白・改行・鍵の順）までは比べない。harness2 は JSON として読むだけで、字の並びは振る舞いに効かない。下流は依頼の塊をそのまま書き出す（Task 1.1）ので、実際には字の並びも同じになる
- 「依頼（逐語）」の節に JSON の塊がちょうど 1 つあることも確かめる（record・spec の段が「価値」を書き直しても、逐語の節は消さない約束。塊が増えたり消えたりしたら落ちる）

### D2（仮）. 有効にする前の宿題の一覧は change の中のファイルに置き、PR 本文と issue にも残す（Q1）

- 一覧の正本は `openspec/changes/ch13-workbox-config/enable-before.md`（上流が書いた。archive の後は `openspec/changes/archive/<日付>-ch13-workbox-config/enable-before.md` に移って残る）。
  PR 本文（`docs/briefs/CH13-pr.md`。上流も下流も同じファイルを PR 本文にする。`publish.sh`）に、一覧への案内と 0〜6 の見出しを載せる（Task 3.1）
- issue は下流の Task 3.2 が 1 つ立てる（題 `作業箱を有効にする前の宿題（CH13）`、本文は `enable-before.md`）。同じ題があれば立てない。
  グラフが下流へ渡す CH13 の issue は宿題の置き場にしない: 本文は `issue_body.py` が tasks から組み直し、下流の PR 本文が `closes` を書けば merge で閉じる（`.harness2/docs/flow-gates.md` の「`closes #N` は残タスク 0 のときだけ」）
- 外向きの作用（issue を立てる）を下流の agent が持つのは、この Task だけ。sandbox の外での実行の申請はしない（AGENTS.md は申請を `/dev/kvm` の検証だけに限っている）
- 反転条件: Q1 が「PR 本文に挙げるだけ」なら Task 3.2 を外し、`enable-before.md` を消して PR 本文だけにする。
  「新しい change の依頼として登録する」なら Task 3.2 を外し、`hx change` は人間の操作なので PR 本文に登録を促す一文を足す。
  下流で issue を立てられないと分かれば（申請が通らない）、Task 3.2 を外し、PR 本文に「merge の後に人間が立てる」1 行を置く

### D3（仮）. 依頼の完了の判定の pgrep の確かめは、この change の検証に置かない（Q3）

- harness2 の main に `scripts/workbox.py` が無い今、検証に置くと証跡は BLOCKED_INFRA になり、ハーネスの Task gate は BLOCKED の項目を持つ Task を閉じない（`.harness2/docs/flow-gates.md` の「関門の 3 段」の Task gate）。
  CH13 が harness2 の merge まで下流で止まる
- さらに `HARNESS_WORKBOX=tests` の下でこのコマンドを走らせると、コマンド自身が `docker` の欄にも `host_only` にも当たらずに箱に包まれ、箱のイメージには docker が無いので落ちる（review/deep.md R1）。
  確かめるときは `HARNESS_WORKBOX` を off にしてホストから走らせる。この注意ごと宿題の一覧に載せる
- 反転条件: Q3 が「tasks に置き、harness2 の merge まで止める」なら Task 4 を足す（`HARNESS_WORKBOX` を外した環境で `python3 .harness2/scripts/workbox.py run . -- bash -c 'pgrep -fc /'` の出力が 10 未満）。
  「harness2 の merge を着手の前提にする」なら、同じ Task を足したうえで、harness2 の merge まで上流の合意を出さない（グラフの外。本人が待つ）

### D4（仮）. 仕様を持たない（`skip_specs: true`）。ファイル 1 つだけを置く（Q4）

- 正典の `verify-environment`（検証の準備の約束）に Requirement を足すと、Scenario の印を持つ試験か台本が要り、ファイル 1 つの範囲を越える。off の間は観測できる振る舞いが無い（proposal の Capabilities）
- 変えないもの: `tools/*.sh`（`workbox.json` 以外）・`docs/testing.md`・`AGENTS.md`・`CLAUDE.md` の構成の表・`.github/workflows/`・`openspec/specs/`。下流が変えたら Task 2.2 が落ちる
- 反転条件: Q4 が「検証の環境の正典に Requirement を足す」なら `.openspec.yaml` の `skip_specs` を外し、`specs/verify-environment/spec.md` に ADDED を 1 つ（置き場と既定 off の意味）と、それを確かめる台本の Task を足す。
  「`CLAUDE.md` の構成の表に書き足す」なら Task 2.2 の許す範囲に `CLAUDE.md` を足し、`tools` の行の「ハーネスの差し込み口」に `workbox.json` を 1 語足す Task を置く

### D5（仮）. 後の Story に箱の制約を知らせる規約・検査を、この change では置かない（Q2）

- 箱を有効にするときにまとめて決める。宿題の一覧に「後の Story が新しく書く検証で同じ漏れが起きる」を載せる
- 反転条件: Q2 が「`docs/testing.md` に書く」なら、`docs/testing.md` の §4 の表の近くに数行（検証コマンドの字面で docker を渡す・中で docker を使う台本は字面に出す）を足す Task を置き、Task 2.2 の許す範囲に足す。
  「検査を足す」なら、走っている change の tasks の検証コマンドを `tools/workbox.json` の `docker` の欄と突き合わせる `tools/check-workbox-docker.sh`（新しいファイル）と CI の 1 行を足す Task を置く（仕様を持つかは D4 の反転と一緒に決め直す）

## Risks / Trade-offs

- [harness2 の鍵の名前が merge までに変わると、このファイルの欄は黙って効かなくなる] → 宿題の一覧の 1 行目。harness2 の merge の後、`scripts/workbox.py` の冒頭の鍵の一覧と突き合わせ直す
- [宿題の一覧が読まれないまま有効にされる] → 有効にするのは本人が環境変数を変える操作で、ashiato2 の側に止める口は無い。一覧は PR 本文の先頭に置き、change の中に残す（D2）
- [下流の agent が gh で網に出られない（Codex の sandbox など）] → gh の接続失敗の文言は verify-run の環境の欠落の判定（`.harness2/scripts/evidence.py` の `INFRA_SIGNS`）に当たらず、3.2 の証跡は BLOCKED でなく FAIL になる。直しても通らないので、fix は D2 の反転（PR 本文に「merge の後に人間が立てる」1 行・3.2 を外す）を A として人間に返す。上流で避ける手は無い（issue を立てる口が publish にしか無い）
- [下流が依頼の JSON を写し間違える] → D1 の比較が落ちる
- [上流の検査が仕様 0 本の change を想定していない] → `openspec validate --strict`・`check_scenarios.py`（specs/ が無ければ飛ばす）は通る。GO 資料（`go_brief.py`）は Requirement 0 本で組めるかを reading の段で確かめる

## Migration Plan

ファイルを足すだけ。戻すときはファイルを消す（off の間は何も読まないので、消しても振る舞いは変わらない）。
