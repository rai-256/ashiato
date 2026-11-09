# CH13 design — 作業箱（コンテナ隔離）の設定を置く

> 第 1 回の深掘りの推奨の答えで先に書き、2026-11-08 に本人の答え（`deep.md` の Q1〜Q4。4 問とも推奨と違う）を `spec --answers Q1,Q2,Q3,Q4` で写した。
> 同じ日の第 2 回の合意で本人が「違和感がある」と返した（`deep.md` の「仕様への違和感（第 2 回の合意）」(1)〜(5)）。harness2 #161（issue #157）で
> 作業箱が検証の箱ごとに箱の中の dockerd を持つ形に変わったので、この版はそれに合わせて書き直した。
> D6〜D8 は（仮）を外した（D6・D7 は前提が消え、D8 は違和感 (2) で本人が決めた）。新しい仮は D9（仮）だけ。

## Context

動機は `proposal.md` の Why。いまの形（`premises.md` の表。2026-11-08 の repo と harness2 の main `d65ff3b4a`）:

- harness2 の作業箱は `HARNESS_WORKBOX` が `tests`（検証だけ）か `all`（agent も）のときだけ、repo の `tools/workbox.json` を読む。未設定は `off` で、設定を読まない（`premises.md` 1 行目）
- **harness2 #161（2026-11-08 13:55Z merge。issue #157 を閉じた）で形が変わった**: 検証の箱は 1 コマンドごとに使い捨てで、箱の中だけの rootless の dockerd を持つ（最初の `docker` の呼び出しで立つ）。
  検証の準備（`tools/verify-env.sh`）も同じ箱で走る。ホストの docker.sock は渡さない。字面の正規表現で docker を渡す形は無くなり、`needs_docker` は消えた
- 箱に入れるかどうかは `host_only` の正規表現だけで決まる（検証コマンドの字面に `re.search`。当たればホストで走る。`.harness2/scripts/evidence.py` の `in_box = … not workbox.host_only(…)`）
- `load_config` は知らない鍵で止まる（`KEYS` の外）。前の形の鍵 `docker` と `prep_files` は読み込みを止めず「もう効かない」と stderr に出す（`RETIRED`）
- 新しい鍵 `docker_images`: 検証の箱の dockerd に先に読み込むイメージ。ホストで pull・save した tar を箱に渡す（`~/.cache/harness2-workbox/images`）
- ashiato2 の中に `workbox` の語を読むものは無い（`git grep -i workbox` が `openspec/` と `docs/changes/CH13.md` の外で 0 件。2026-11-08）
- 既存の `openspec/**/tasks.md` の検証コマンド 1,786 本を harness2 の取り出し（`evidence.declared`）で集め、`host_only` だけで分けた（`premises.md` の第 2 回の追記）
- **上流で見つけた新しいこと**: ホストの `PATH` に `/usr/sbin` が無いと、箱の中の dockerd が `iptables not found` で立たない（箱はホストの `PATH` を受け継ぐ）。
  グラフを走らせる `hx-dev.service` の `PATH` にも `/usr/sbin` が無い（`premises.md` の第 2 回の追記。確かさ 5）

## Goals / Non-Goals

**Goals:**
- `tools/workbox.json` が D1 の中身（依頼の版に第 1 回の Q1 (1) と第 2 回の違和感 (1) の直しを当てたもの）のとおりにあり、JSON として読め、今の harness2 の `load_config` が止めず、もう効かない鍵を持たない
- 足した `host_only` が、harness2 と同じ判定で、当たるべき検証に当たり、当たってはいけない検証に当たらないことを機械で確かめる（Q3・違和感 (2)）
- off の間、ashiato2 の振る舞いが何も変わらない（読むものが無い・ほかのファイルを変えない・リポジトリの検査が通る）
- 箱を有効にする前に測ることの一覧と、harness2 #157 / #161 で済んだことへの案内が、merge の後も repo に残る（Q1 (3)・違和感 (3)）
- pgrep の確かめ（依頼の完了の判定）をこの change の中で済ませる（Q3・違和感 (4)）

**Non-Goals:**
- harness2 側（`scripts/workbox.py`・`workbox/dockerd-up`・`evidence.py`）を直すこと。上流で見つけた `PATH` の件は issue を立てるまで（D9（仮））
- 後の Story に箱の制約を知らせる規約・検査（Q2。D5）
- 正典の仕様と文書（`tools/*.sh`・`docs/testing.md`・`AGENTS.md`・`CLAUDE.md`）を変えること（Q4。D4）
- 測っていないメモリの前提（e2e 1 本が 6g・`all` で 5 本・agent の cargo が 4g）をこの change で測ること（Q1 (3)。宿題に残す）

## Decisions

### D1. 中身は依頼の版に Q1 (1) と違和感 (1) の直しを当てたもの。正本はこの節の JSON

- 本人が依頼の「書き換えない」を外した（Q1）。置く中身は「依頼の版に直しを当てたもの」（Q4 の補足）。**正本はこの節の JSON の塊**で、下流は塊を機械で取り出して `tools/workbox.json` に書く（Task 1.1。写し間違いを避ける）。
  依頼（`docs/changes/CH13.md` の「依頼（逐語）」）は逐語のまま残し、書き換えない
- 依頼の版から変えてよいのは 4 つの欄だけ（違和感 (5)）: `docker`（消す）・`prep_files`（消す）・`docker_images`（足す）・`host_only`（後ろに足す）。
  ほかの欄（`apt` `memory` `tmp_size` `env` `parallel_verify` `agent_memory`）は依頼の版と同じ。Task 1.2 がこの 4 つを機械で確かめる
- `docker` と `prep_files` を残すと、今の harness2 は読み込みのたびに「もう効かない」と出す（`RETIRED`）。効かない鍵を残すと、効いていると読み違える
- `docker_images` は `postgres:17-alpine` 1 つ（`docker-compose.yml` の `db` の image と同じ）。検証の準備が箱の中で DB を立てるたびに pull しないため。ほかのイメージ（workflow の検査が退く `rhysd/actionlint` など）は箱の中で pull する
- 字の並び（空白・改行・鍵の順）までは比べない。harness2 は JSON として読むだけ

依頼の版からの差分:

| 欄 | 依頼の版 | この版 | 出所 |
|---|---|---|---|
| `docker` | 正規表現 3 本 | 無い（消した） | 違和感 (1)。harness2 #161 で効かない |
| `prep_files` | 6 本 | 無い（消した） | 違和感 (1)。検証の準備は箱の中で走る |
| `docker_images` | 無い | `postgres:17-alpine` | 違和感 (1) |
| `host_only` | 4 本 | `tools/release/(build\|install-check)\.sh` `tools/android-side-by-side\.sh` `tools/usage-volume\.sh` を後ろに足した 7 本 | Q1 (1)・review/deep.md R3。違和感 (1) で残す |

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
  "docker_images": [
    "postgres:17-alpine"
  ]
}
```

### D2. 有効にする前の宿題の一覧は change の中のファイルに置き、PR 本文と issue にも残す（Q1 (3)・違和感 (3)）

- 一覧の正本は `openspec/changes/ch13-workbox-config/enable-before.md`（archive の後は `openspec/changes/archive/<日付>-ch13-workbox-config/enable-before.md` に移って残る）。
  載せるのは、測るもの（e2e 1 本が 6g に収まるか・`all` で 5 本のときの空き・agent の箱の中の cargo が 4g に収まるか。DB の容器も箱のメモリに数えられる）・
  `host_only` に足した台本の確かめ（Task 4.1 の結果）・上流で見つけた `PATH` の件（D9（仮））・harness2 #157 / #161 で済んだことへの案内。
  **#161 で消えたもの（docker が渡らない・当たりすぎ・`prep_files` の不足・後の Story の漏れ・古いイメージ）は一覧から消す**（違和感 (3)）。pgrep の確かめと鍵の確かめは Task 3 で済ませるので、一覧には結果だけを残す
- PR 本文（`docs/briefs/CH13-pr.md`。上流も下流も同じファイルを PR 本文にする。`publish.sh`）に、一覧への案内と、Task 3 の pgrep の結果を載せる（Task 5.1）
- ashiato2 の issue は下流の Task 5.3 が 1 つ立てる（題 `作業箱を有効にする前の宿題（CH13）`、本文は `enable-before.md`）。同じ題があれば立てない。
  グラフが下流へ渡す CH13 の issue は宿題の置き場にしない: 本文は `issue_body.py` が tasks から組み直し、下流の PR 本文が `closes` を書けば merge で閉じる（`.harness2/docs/flow-gates.md` の「`closes #N` は残タスク 0 のときだけ」）
- 外向きの作用（issue を立てる）を下流の agent が持つのは Task 5.2・5.3 だけ。sandbox の外での実行の申請はしない（AGENTS.md は申請を `/dev/kvm` の検証だけに限っている）
- 下流で issue を立てられないと分かれば（gh が網に出られない）、その Task を外し、PR 本文に「merge の後に人間が立てる」1 行を置くことを A として人間に返す

### D3. pgrep の確かめと鍵の確かめを tasks に置く（Q3・違和感 (4)）

- 本人の選択は「tasks に置き、harness2 の merge まで CH13 を止める」。補足のとおり harness2 は merge 済み（#152・#161）なので、止まらずに Task 3 で確かめる
- Task 3.1: `HARNESS_WORKBOX` を外したシェルから `python3 .harness2/scripts/workbox.py run . -- bash -c 'pgrep -fc /'` の出力が 10 未満。`workbox.py run` は `HARNESS_WORKBOX` を見ずに箱を起こす（`.harness2/scripts/workbox.py` の `main`）。
  `docker` を呼ばない箱では dockerd は立たない（`workbox/docker-shim` が最初の `docker` の呼び出しで立てる）ので、数は箱の init と `pgrep` だけ（違和感 (4)）。
  `HARNESS_WORKBOX=tests` の下で検証として走らせると、このコマンド自身が検証の箱に包まれ、箱の中から作業箱を入れ子に起こそうとする（review/deep.md R1 の形が変わったもの）。**この確かめは箱が off のときだけ成り立つ**（Global Constraints が下流に `HARNESS_WORKBOX` を設定させない）。
  上流で使い捨ての設定（D1 の中身）を指して走らせ、出力は `1`、rc=0（2026-11-08。`premises.md`）。`docker_images` があるので、初回はホストで `postgres:17-alpine` を pull・save する
- Task 3.2: 鍵の確かめ。harness2 の main の `workbox.load_config` で `tools/workbox.json` を読み（知らない鍵があれば止まる）、もう効かない鍵（`workbox.RETIRED`）を 1 つも持たないこと。
  `load_config` は効かない鍵を止めずに知らせるだけなので、そこは台本で落とす。前の版の「説明の文字列に鍵が出るか」（R9）より強い
- 正規表現の当たり方の確かめは Task 4（D8）

### D4. 仕様を持たない（`skip_specs: true`）。触るのは `tools/workbox.json` と change の中の文書だけ（Q4）

- 本人の答え: 「ファイル 1 つだけ。正典にも文書にも手を入れない」は変えない。中身は D1（Q4 の補足）
- 正典の `verify-environment`（検証の準備の約束）に Requirement を足すと、Scenario の印を持つ試験か台本が要り、範囲を越える。off の間は観測できる振る舞いが無い（proposal の Capabilities）
- change の中に置くもの: `enable-before.md`・`match-cases.json`（上流が書いた当たり方の表）・`check-workbox-match.py`（下流が書く確かめの台本）・`command-classes.md`（下流が台本で書き出す分類の一覧）。どれも archive で change と一緒に移る
- 変えないもの: `tools/*.sh`（`workbox.json` 以外）・`docs/testing.md`・`AGENTS.md`・`CLAUDE.md` の構成の表・`.github/workflows/`・`openspec/specs/`。下流が変えたら Task 2.2 が落ちる

### D5. 後の Story に箱の制約を知らせるものを、この change では置かない（Q2）

- 本人の答え: 根本は harness2 側で直す（字面で判定しない形になれば、後の Story に制約を知らせる必要が無くなる）
- harness2 #161 で字面の判定は無くなった（D7）。docker を使う検証は、後の Story が新しく書いても箱の中の dockerd で走る。残る字面の判定は `host_only` だけで、ホストが要る検証（Windows・エミュレータ）を新しい台本の名前で書けば箱に入る。
  これは `enable-before.md` に 1 行置く（後の Story に知らせる規約は置かない）

### D6. docker の正規表現は持たない（違和感 (1)。前の D6（仮）の錨と族の足し算は消した）

- 第 1 回の答え（Q1 (1)）で、`docker` の欄に呼び出しの位置の錨を置き、e2e・`cargo test --workspace` などの族を足した（前の D6（仮））。harness2 #161 で `docker` の欄は効かなくなり、違和感 (1) で欄ごと消した
- 台帳では resolve した（`provisional.jsonl`。理由は「前提の欄が消えた」）

### D7. harness2 の根本の 3 つは harness2 #157 / #161 で閉じた。CH13 からは立てない（違和感 (3)）

- 第 1 回の答え（Q1 (2)）で立てる予定だった 3 つ（docker を渡すかを字面で決める・知らない鍵を黙って無視する・古いイメージが溜まる）は、harness2 #157（CLOSED）が 1 つの issue で挙げ、#161 が直した:
  字面の判定は箱の中の dockerd に置き換わり、`load_config` は知らない鍵で止まり、`prune_images` が 7 日より古い作業箱のイメージを消す（`.harness2/scripts/workbox.py` の `KEYS` `RETIRED` `prune_images`）
- `enable-before.md` の「harness2 #157 / #161 で済んだもの」の節に案内を置く。台帳では前の D7（仮）を resolve した

### D8. 当たり方の確かめは `host_only` だけで、change の中の表と台本で、harness2 の関数をそのまま呼ぶ（Q3・違和感 (2)）

- 表 `openspec/changes/ch13-workbox-config/match-cases.json`（上流が書いた）: コマンドと、期待する行き先（`host` ホストで走る / `box` 箱で走る）の組。
  足した `host_only` の台本が当たること・依頼の版から `host_only` にあるものが当たること・e2e や DB を使う検証や台本の名前を grep するだけの検証が箱に入ることを持つ
- 台本 `openspec/changes/ch13-workbox-config/check-workbox-match.py`（下流が書く）: `.harness2/scripts` の `workbox.host_only` を import して行き先を決める。`needs_docker` は呼ばない（無い）。
  正規表現を写し直さない（harness2 と同じ判定でなくなる）
- 分類の一覧 `command-classes.md`（台本の `--list` が書き出す）: 既存の `openspec/**/tasks.md` の検証コマンドを `.harness2/scripts/evidence.py` の取り出し（`items` → `declared` → `unwrap_quiet`）で集め、ホスト / 箱の 2 つの節に並べる。`enable-before.md` から案内する
- 台本が表を本当に見分けることも確かめる: 依頼の版の JSON（`docs/changes/CH13.md` の逐語から取り出す）に当てると、足した `host_only` の 4 行が外れて落ちる（Task 4.1）
- 違和感 (2) で本人が決めた（前の D8（仮）は台帳で resolve した）
- 反転条件: harness2 が判定の関数の名前を変えたら、台本の import を追従させる

### D9（仮）. 上流で見つけた `PATH` の件を harness2 の issue に 1 つ立てる（Q1 (2) に従う）

- 見つけたこと（2026-11-08。`premises.md` の第 2 回の追記、確かさ 5）: 検証の箱はホストの `PATH` を受け継ぐ。`PATH` に `/usr/sbin` が無いと、箱の中の dockerd（`workbox/dockerd-up`）が
  `failed to create NAT chain DOCKER: iptables not found` で立たない。`/usr/sbin` を足すと立ち、`docker_images` の `postgres:17-alpine` が読み込まれている。
  グラフを走らせる `hx-dev.service` の `PATH` にも `/usr/sbin` が無い。このまま `HARNESS_WORKBOX=tests` にすると、検証の準備（`tools/verify-env.sh`）が毎回 docker を呼ぶ（`clear_blocking_network` と `docker compose up -d --wait db`）ので、準備が落ち、箱に入る検証は docker を使わないものも含めて全部 BLOCKED_INFRA になる見込み（2026-11-08 の走査で 1,786 本のうち 1,751 本。review/spec.md R15）
- 本人の Q1 (2) は「根本は harness2 側の issue にする」。直す場所は harness2 の `dockerd-up`（か箱の `PATH`）で、ashiato2 の `tools/workbox.json` の `env` で回避しない（違和感 (5) が `env` を依頼の版のままとしている）
- 下流の Task 5.2 が `rai-256/harness2` に 1 つ立てる。題は `作業箱: ホストの PATH に /usr/sbin が無いと箱の中の dockerd が iptables を見つけられず立たない`。同じ題があれば立てない。
  本文は上の再現（コマンドと出力。`dockerd-up` は rc=75）・影響（箱に入る検証の全部が BLOCKED_INFRA）・直す場所の候補（`workbox/dockerd-up` の `PATH="$bin:$PATH" setsid rootlesskit …` で `/usr/sbin:/sbin` を足せば利用者の unit に依らない。harness2 の `systemd/hx-tick@.service` の `PATH` にも `/usr/sbin` が無い）と、この change の PR への案内。立てた番号を `enable-before.md` の `## harness2 の issue` の節に書く
- 失わない理由: issue は閉じれば戻る。ashiato2 の振る舞いは変わらない（off の間は読まれない）
- 反転条件: harness2 側で同じ件が先に直った・issue が立っていれば、立てずにその番号を書く。本人が issue でなく宿題の一覧に置くだけでよいと言えば、Task 5.2 を外す

### D10（仮）. 下流で見つけた `host_only` の漏れ（`tools/release/sandbox-probe.sh`）は、ファイルを直さず宿題の一覧と PR 本文に挙げる（review/code.md R3）

- 見つけたこと（code-verify・/code-review。review/code.md R3・R5）: `tools/release/sandbox-probe.sh` は中で `powershell.exe` を呼んで Windows サンドボックスを起こす（D1 で `host_only` に足した 3 本と同じ型）が、`host_only` にも `match-cases.json` にも無い。箱の中では `P16=UNAVAILABLE` `P19=UNAVAILABLE` を書いて rc=0 で通る。呼ぶ検証は archive 済みの st55 の 3.0 だけ
- 決めたこと: `tools/workbox.json` は D1 の JSON のまま直さない。`enable-before.md` の「ホストで走らせる台本」に漏れとして 1 行置き、PR 本文にも挙げる（Global Constraints の 1 つ目「足りないものに気付いたら、ファイルを直さずに PR 本文に挙げ、`enable-before.md` に 1 行足す」と、違和感 (5) の「`host_only` は依頼の版に 3 本を足しただけ」に従う）
- 失わない理由: off の間は `host_only` を誰も読まない。呼ぶ検証は archive 済みの 1 本で、今の木の検証は変わらない。有効にする前に一覧を見て 1 行足せば戻る
- 反転条件: 本人が CH13 の中で `host_only` に足すと言えば、D1 の JSON・`tools/workbox.json`・tasks 1.2 の期待する並び・`match-cases.json`（`want: host` の 1 行）を同じ commit で直す。`HARNESS_WORKBOX` を有効にする前に足されていなければ、有効にする操作の前に足す

## Risks / Trade-offs

- [harness2 の鍵の名前が後で変わる] → 今の `load_config` は知らない鍵で止まるので、黙っては効かなくならない。Task 3.2 が今の main で確かめる
- [ホストが要る検証を新しい台本の名前で書くと箱に入る] → `host_only` は字面の判定のまま。`enable-before.md` に 1 行（D5）。足すときは `host_only` に名前を足す
- [宿題の一覧が読まれないまま有効にされる] → 有効にするのは本人が環境変数を変える操作で、ashiato2 の側に止める口は無い。一覧は PR 本文の先頭と issue に置く（D2）
- [下流の agent が gh で網に出られない（Codex の sandbox など）] → gh の接続失敗の文言は verify-run の環境の欠落の判定（`.harness2/scripts/evidence.py` の `INFRA_SIGNS`）に当たらず、5.2・5.3 の証跡は BLOCKED でなく FAIL になる。直しても通らないので、fix は D2 の最後の行を A として人間に返す
- [Task 3.1 は docker と作業箱のイメージと `postgres:17-alpine` を要る] → 下流の検証の環境に docker が無ければ、`workbox.py` の `Unavailable` で落ちる。上流のこの機械では 1 秒で通った（イメージも tar も作ってあった）
- [下流が D1 の JSON を写し間違える] → Task 1.2 の比較が落ちる

## Migration Plan

ファイルを足すだけ。戻すときはファイルを消す（off の間は何も読まないので、消しても振る舞いは変わらない）。
