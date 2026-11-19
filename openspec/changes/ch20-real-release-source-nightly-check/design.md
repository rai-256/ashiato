# CH20 design — 配る形の更新の確認を、本物の配布元で毎晩確かめる

## Context

動機は `proposal.md` の Why。いまの形:

- 更新係（`crates/updater`）と PC の収集アプリ（`crates/collector-windows/src/update_check.rs`）は、選び方を `crates/update-check`
  （`parse_releases`・`choose`）で共有する。試験の組み立て（`ASHIATO_RELEASE` が `1` でない組み立て。`cfg(ashiato_test_build)`）だけが宛先を差し替えられる:
  更新係は `ASHIATO_UPDATER_TEST_URL` と版の上書き `ASHIATO_UPDATER_VERSION`（`crates/updater/src/lib.rs:76-118`）、PC の収集アプリは
  `ASHIATO_COLLECTOR_TEST_UPDATE_URL`（設定の `base_url` も要る。版は組み立ての定数だけ。`update_check.rs:328-345`）。
  差し替えの宛先が `https://api.github.com` なら、どちらも本物の道（`RELEASES_URL`）を組む（premises P7）
- 配る組み立ての更新係は Windows のサービスで、インストールの後に起きて確かめ、確かめるたびに `C:\ProgramData\ashiato-updater\checks.jsonl` に 1 行書く
- PR の CI の `install-windows` は最初のインストールより前に更新係の外向きを塞ぐ（ST57 D18。`update-check-install.sh --block-outbound`）。この change は変えない
- 毎晩の検査の前例は `vulns-nightly.yml`（公開の `nightly.yml` から分け、検査の台本を入口にし、結果を成果物に残す）
- 2026-11-19 の本物: 正式版の道 `200`・`[]`、開発版の道 `404`（P1・P2）。`rai-256/ashiato` は公開の repo

## Goals / Non-Goals

**Goals**: 本物の配布元と 2 つの部品の組み合わせを毎晩機械で見る。偽の配布元と本物のずれを赤で知らせる。PR の CI は今のまま本物に触れない。

**Non-Goals**: 製品のコードを直すこと（3 つの食い違いを含む。Q9）。携帯の収集アプリ（Q2）。本人が 2026-11-19 に見たエラーの原因を探すこと（Q1）。
配布元にリリースを置くこと（Q4 の選択肢 3 は外に出るので採らない）。

## Decisions

### D1. 置き場は別の workflow、Windows の job 1 つ

`.github/workflows/release-source-nightly.yml`（schedule `cron: "0 16 * * *"` と `workflow_dispatch`）に job `check`（`windows-latest`・`timeout-minutes: 90`）を 1 つ置く。
`nightly.yml` には触れない（公開の流れから分ける前例 `vulns-nightly.yml` と同じ。review/deep.md R9 の C の既定）。権限は workflow の上で `contents: read`・`actions: read` だけ。
3 つの部品を 1 つの runner で回す（配る形のサーバは Windows でしか入らない。試験の組み立ての 2 部品は Linux でも動くが、runner を分けると答えの受け渡しが増える）。

代わり: `vulns-nightly.yml` に job を足す —— 脆弱性の検査と赤の理由が混ざり、権限（`issues: write`）も広がる。採らない。

### D2. 台本の口を 4 つに分け、判定は記録だけから行う

`tools/release/release-source-check.sh` の口:

| 口 | 何をするか | 本物に触れるか | 資格情報 |
|---|---|---|---|
| `fetch <置き場>` | 検査の側の問い合わせ（D4）で本物の答えを取り、状態コード・見出しの `Location`・本文をそのまま置く | 触れる（検査の側） | workflow の資格情報 |
| `collect <置き場>` | 部品を走らせ、確かめた記録を置く（本物の状態の 3 回と、手元の受け手からの流し直し。D3・D6・D7） | 触れる（部品の 3 回） | 渡さない |
| `history <置き場>` | 前の 2 回の実行の要約を読む（D8） | 触れない | workflow の資格情報（`actions: read`） |
| `judge <置き場>` | 置き場の記録だけを読み、判定・要約・rc を出す（rc=0 緑・rc=1 赤） | 触れない | 無し |

ほかに `--plan`（部品が本物に問い合わせる段の一覧を出す）・`--self-test`・`--check-workflow`。
形の比べ・版の書き換え・流し直しの受け手・判定の本体は `tools/release/release_source.py`（python3 の標準だけ）に置き、台本はそれを呼ぶ。

置き場の形: `fetch/<名前>.status`・`.headers`・`.body`（検査の側の 4 つの答え。届かなければ `.status` に `unreachable:<理由>`）・
`replay/<場合>.body`（`collect` が流し直した本文。`judge` は本文のバイト数をここから読む）・`records/mode`（`collect` が最初に書く `full` か `replay-only`）・
`records/<部品>-<場合>.json`（部品の確かめた記録 1 行。配る形の更新係が 180 秒で書かなければ `records/updater-release-real.timeout`）・`history/`（前の実行の要約）。
`judge` は `records/mode` が `full` なのに記録の無い段を赤に、`replay-only` なら本物の 3 段を `WARN` 1 行ずつ（「本物の段を飛ばした」）に、`records/mode` が無ければ
「部品を走らせる段が記録を残していない」の赤 1 行にする。workflow は `history`・`judge`・成果物の保存を `if: always()` で走らせる（`collect` が途中で落ちても判定と保存が残る。review/spec.md R4）。

**判定を記録だけから行う**ので、`judge` は記録した材料で本物に問い合わせずに試せる（PR の CI の自己試験。D11）。`collect` の実地は毎晩の実行でしか走らない。

`collect` の流し直しは、Linux でも試験の組み立ての 3 部品（更新係・PC の収集アプリ・開発版の版の PC の収集アプリ）で動く（`collect --replay-only <置き場>`。`fetch` の代わりに記録した材料を置き場に置いて走らせる）。
これで下流は、本物にも Windows にも触れずに、流し直しの場合と判定のつながりを手元で確かめられる。

### D3. 配る形のサーバは検査の中で組んで入れ、サービスの最初の確かめを待つ

`collect` は `tools/release/build.sh server-windows`（配る形。`ASHIATO_RELEASE=1`）で zip を組み、`ashiato-setup.exe` で入れる（インストールの手順は
`install-check.sh` の呼び方に合わせるが、`install-check.sh` は呼ばない・変えない）。入れた後、更新係のサービスが `checks.jsonl` に 1 行書くのを 180 秒まで待ち、その行を記録とする。
`ashiato-updater.exe once` を手で呼ばない（本人の使い方はサービスで、サービスのアカウント・置き場の権限で起きる食い違いも拾う。Q3）。
外向きは塞がない（この workflow は PR の CI ではない）。組んだ zip の版は、zip の名前から読む（`build.sh` の既定の版）。

### D4. 問い合わせの数

1 回の実行で:

| 誰が | 道 | 回数 | 資格情報 |
|---|---|---|---|
| 配る形の更新係 | 自分の版の道（正式版の版なら一覧） | 1 | 無し |
| 試験の組み立ての更新係（版を `<組み立ての版の土台>-dev.<その日>` に上書き） | `/releases/tags/nightly` | 1 | 無し |
| 試験の組み立ての PC の収集アプリ | 自分の版の道 | 1 | 無し |
| 検査（`fetch`） | `rai-256/ashiato` の一覧・印 `nightly`、別の repo の一覧（1 件）と、その最新の印の 1 件 | 4 | workflow の資格情報 |

部品の 3 回は正典の上限（MODIFIED）どおり。検査の側は資格情報つきで、部品の問い合わせの上限（認証なしの IP ごと 60 回）を食わない（review/deep.md R12 の C の既定）。
問い合わせを繰り返さない（落ちた回を取り直さない。届かない夜は D8）。

### D5. 比べる基準は repo の材料に置く

`docs/contracts/release-source-shape.json` に、偽の配布元（共有の材料と試験の受け手）が返している形を書く:
道ごとの答えの形（一覧の道は配列・1 件の道は**配列**。いまの試験の受け手は道に依らず配列を返す）・各リリースの読む項目と型
（`tag_name` 文字列・`draft` 真偽・`assets` 配列・`assets[].name` 文字列・`assets[].browser_download_url` 文字列・`assets[].digest` 文字列）・
本文の上限（65536 バイト）・部品の知っている状態コード（`200`・`301`・`302`・`307`・`308`・`403`・`404`・`429`。これ以外の本物の状態コードは配布元の障害として「届かなかった」に数える。spec）。
本文の上限は `crates/update-check/src/lib.rs` の `MAX_RELEASES_JSON_BYTES` と同じ値で、自己試験がそれを読んで突き合わせる（片方だけ変われば PR の CI で落ちる）。

1 件の道の基準を「配列」にするのは、偽の配布元がいまそう返しているから。直しの change が部品にオブジェクトを読ませるとき、基準も「オブジェクト」に直す
（基準は偽の配布元の写しで、正しい形の宣言ではない）。毎晩の本物の答えは成果物に残す（review/deep.md R11 の C の既定。いつから形が変わったかを後から引ける）。

### D6（仮）. 流し直しの場合

その晩の本物の答えから、版の名前だけを書き換えて組む。`rai-256/ashiato` に数えるリリースが無い間は、別の repo（D12）の最新のリリース 1 件を土台にし、
`assets` を自分の部品の配布物 4 つ（土台の最初の配布物の写しの `name` と `browser_download_url` の版を書き換えたもの）に置き換える。

| 場合 | 渡す部品 | 正典の期待 |
|---|---|---|
| 同じ版（`v<自分の版>` だけ） | 更新係・PC の収集アプリ | 確かめた・見つけた最新の版 = 自分の版（更新なし） |
| 新しい版がある（`v<自分の版>` と `v<PATCH+1>`） | 同上 | 確かめた・見つけた最新の版 = PATCH+1（更新あり） |
| 印 `nightly` の 1 件（オブジェクト。開発版の配布物の名前） | 更新係（版の上書きで開発版）・PC の収集アプリ（開発版の版で組んだ試験の組み立て。D7） | 確かめた・見つけた最新の版あり |
| 20 件の一覧（`v<自分の版>` から古い方へ 20 件。本文は `tools/release/nightly-notes.md`） | 更新係・PC の収集アプリ | 確かめた・見つけた最新の版 = 自分の版 |
| `503` | 同上 | 読めない答え |
| `api.github.com` でない宛先への `301` | 同上 | 読めない答え |

20 件にするのは、部品が `per_page=20` で取るので、本文が最も大きくなるのが 20 件の一覧だから。本文に本物の別の repo のリリースの説明でなく
`nightly-notes.md` を使うのは、自分の配布元のリリースの大きさに近づけるため。
**反転条件**: `rai-256/ashiato` にリリースが 1 つでも出たら、そのリリースの本物の大きさで 20 件を組む（別の repo の形は項目の比べだけに使う）。
`503` と `301` は本物に依らない作った答えだが、深掘り Q9 の推奨（検査は 3 つの食い違いを捕まえる形にする）で入れる。正典の Scenario「確かめられない理由を状態の番号で分ける」の
偽の受け手の試験が PC の収集アプリに `500` を当てていれば不要だった —— 直しの change がそれを足したら、この 2 つを外してよい。

### D7. 部品を走らせる入口

- 更新係（試験の組み立て）: `cargo build -p ashiato-updater` の実行ファイルを、`ASHIATO_UPDATER_DIR`（場合ごとの空の置き場）・`ASHIATO_UPDATER_TEST_URL`・`ASHIATO_UPDATER_VERSION`
  を与えて `once` で 1 回走らせ、置き場の `checks.jsonl` の最後の行を記録とする
- PC の収集アプリ（試験の組み立て）: `crates/collector-windows/tests/ch20_real_source.rs` に `#[ignore]` の試験を 1 本置く。環境
  `ASHIATO_CH20_UPDATE_URL`（宛先）と `ASHIATO_CH20_OUT`（記録の書き出し先）を読み、空の置き場に設定（`base_url`）と状態（案内を出した印）を書いて、
  `ASHIATO_COLLECTOR_TEST_UPDATE_URL` に宛先を渡し、`UpdateChecker::start` を 1 回呼んで確かめた記録を書き出す。場合ごとに
  `cargo test -p ashiato-collector-windows --test ch20_real_source -- --ignored --exact ch20_check_once` で走らせる。環境が無ければ何もせずに落ちる
  （`#[ignore]` なので、PR の CI の `cargo test` では走らない。`--ignored` を渡す step が `ci.yml` に無いことを `--check-workflow` が見る）

- PC の収集アプリ（開発版の版）: 版は組み立ての定数なので、`ASHIATO_BUILD_VERSION=<組み立ての版の土台>-dev.<その日>` で試験の組み立てをもう 1 つ組み
  （土台が同じなら組み立ての規則が通る。`crates/build-stamp/src/lib.rs:41-58`）、`nightly-object` の流し直しにだけ当てる（本物には当てない。部品の問い合わせは 3 回のまま。review/spec.md R15）

製品のコードに口を足さない（既存の試験の組み立ての口だけを使う）。

### D8. 届かない夜の数え方

`judge` は、その晩に検査の側か部品が本物に届かなかった（`403`・`429`・時間切れ・接続できない）かを `summary.json` の `reached` に書く。
「晩」は予定の起動（`event == schedule`）の実行で数え、手動の起動は数えない（review/spec.md R9）。
`history` は `gh run list --workflow release-source-nightly.yml --event schedule`（今の実行を除いた完了した直近 2 回）と `gh run download -n release-source-summary` で前の `summary.json` を読む。
読めなければ（成果物の期限切れ・最初の実行・API の失敗）その晩だけで数え、注意を出す。成果物の保存期間は D13（仮）。

### D9（仮）. 赤の知らせ方

赤は GitHub が予定の workflow の失敗を知らせる既定の仕組み（cron を最後に変えた人へのメール・Actions の画面）に任せる。issue は立てない
（直しの change が入るまで毎晩赤のままなので、issue にすると同じ題が積み重なる）。review/deep.md R10 の C の既定。
**反転条件**: 3 つの食い違いの直しが入り緑になった後に、赤に 3 日以上気づかなかったことが起きたら、`vulns-nightly.yml` と同じく題ごとに 1 件の issue を立てる形に変える。

### D10. 期待は正典から決める

`judge` の期待（D6 の表と、本物の状態の期待）は正典の Requirement の文から決め、`release_source.py` に表で持つ。部品のいまの振る舞いから写さない
（写すと、3 つの食い違いが緑になる）。本物の状態の期待:

| 本物の答え | 期待 |
|---|---|
| 一覧の道が `200` で数えるリリースが無い | 確かめた・見つけた最新の版なし |
| 一覧の道が `200` で数えるリリースがある | 確かめた・見つけた最新の版は、`fetch` の本文に正典の選び方を当てた版（共有の材料 `update-check-vectors.json` の例で自己試験が当て方を確かめる） |
| 開発版の道が `404` | 配布元が見つからない（緑。要約に「依頼と正典の食い違い」） |
| `200` でも読めない形 | 読めない答え |
| `403`・`429`・`5xx`・ほかの知らない状態コード・時間切れ・接続できない | 判定しない（届かない夜。D8） |

部品が本物の段で「問い合わせの上限」「時間切れ」「配布元に届かない」を記録したら、検査の側が `200` でもその段は判定せず届かなかった段に数える
（部品と検査の側は資格情報の有無で上限が違い、共有の IP では部品だけが `403` を受けうる。部品が受けた答えは検査から見えない。review/spec.md R1）。

### D11. 自己試験と PR の CI

`release-source-check.sh --self-test` は、`tools/release/release-source-check.d/` の材料（2026-11-19 に記録した本物の答え（`[]`・`404`）・
別の repo のリリース 1 件の答え・場合ごとの作った確かめた記録・前の実行の要約）に `judge` を当て、spec の判定の Scenario を確かめる（各 Scenario の前に
`echo "== Scenario: <名前>"`）。版の書き換え（D6）と基準の上限の突き合わせ（D5）も見る。`--check-workflow` は workflow の配線の Scenario を見て、
壊した写しで落ちることも確かめる。どちらも PR の CI の `chain` の job（ubuntu）に 1 行ずつ足す（CH17 の前例）。本物に問い合わせない。

### D12（仮）. 比べに使う別の公開の repo は `cli/cli`

GitHub 自身の CLI の repo で、リリースを定期に出し、配布物に `digest` を持つ。台本の先頭の 1 か所に置く。
その repo に届かない夜（`403`・`5xx`・時間切れ）は、ほかの届かない夜と同じく注意にし、そこから組む判定（形・`nightly-object`・`full-20`）を飛ばす。
`200` なのに読めない（JSON でない・リリースが無い）ときだけ「比べに使う repo の答えを読めない」で赤にする（review/spec.md R1）。
**反転条件**: その repo のリリースが止まる・消える・`digest` を持たなくなったら、別の repo に替える。
`rai-256/ashiato` にリリースが出た後も、1 件の道の形を見るために残す（印 `nightly` が無い間）。

### D13（仮）. 本物の答えは成果物に 90 日残す

毎晩の本物の答え・部品の記録・要約を、成果物として `retention-days: 90`（公開の repo で選べる最長）で残す。repo には書かない（workflow の権限を読むだけに保つ。D1）。
90 日を過ぎた晩の答えは引けなくなるが、形が変わった晩は、その晩の `judge` が道と項目の名前つきの赤で出し、要約が 90 日残る。90 日を過ぎて失うのは、
基準と同じ形だった晩の答え（値だけが違う）である。review/deep.md R11・review/spec.md R6（取らずに失うもの）を、失わない側に寄せた仮として台帳に載せる。
**反転条件**: 90 日より前の晩の答え（値）を引きたい場面が 1 度でも起きたら、要約に答えの値の指紋（sha256）を足して repo の外の置き場（release の資産など）に移すかを決め直す。

### D14（仮）. 保存期間は spec の数値で固定し、配線の検査で縮められないようにする

D13 の 90 日を spec の Requirement と Scenario（「判定と成果物の保存は、前の段が落ちても走る」）に数値で書き、`--check-workflow` が `retention-days` を 90 未満にした写しで落ちる
（review/spec.md R6。design にだけ置くと、実装が 1 日にしても落ちない）。
**反転条件**: D13 を変えたら、ここも同じ値に変える。

## Risks / Trade-offs

- [共有の runner の IP は、認証なしの上限 60 回を他の利用者と分け合う。部品の 3 回が `403` になる夜がある] → 届かない夜は注意だけにし、3 晩続いたときだけ赤（D8）。部品が `403` を「問い合わせの上限」と記録したかは判定する
- [直しの change が入るまで毎晩赤のまま。赤が当たり前になり、新しい赤を見落とす] → 要約の先頭に「既に分かっている食い違い (1)(2)(3)」と「それ以外の赤」を分けて出す（spec。`judge` が名前つきの既知の食い違い（部品と場合の組）を持つ。直しが入ったら直しの change が外す）
- [別の repo（D12）に依る] → 読めなければ理由つきで赤。反転条件で替える
- [Windows の runner の時間が毎晩 40〜60 分（配る形の組み立てを含む）] → 公開の `nightly.yml` とは別に払う。組み立ての使い回しはしない（CH17 のキャッシュは PR の鍵）
- [サービスの最初の確かめが 180 秒で来ない] → その場合を赤にし、「更新係のサービスが確かめなかった」と出す（spec。本人が 2026-11-19 に見た種類に近いかもしれない。P5）。180 秒は、サービスの起動と問い合わせの全体の時間切れ（30 秒）に余裕を持たせた値
- [インストールが runner の状態（既存のサービス）で落ちる] → runner は毎回まっさらで、job の中でだけ入れる
- [基準（D5）が偽の配布元の写しであることを忘れ、正しい形と読まれる] → 基準のファイルの先頭に「偽の配布元が返している形の写し」と書き、直しの change の tasks で基準も直すよう申し送る（`docs/handoff/` は直しの change の番号が決まってから）

## Migration Plan

足すだけ。止めるときは workflow を消すか無効にする。PR の CI と配る物には影響しない。

## Open Questions

なし
