# CH20 の spec レビュー（spec --draft・第 1 回の先読み）

対象: `openspec/changes/ch20-real-release-source-nightly-check/`（proposal.md・specs/distribution/spec.md・design.md・tasks.md）と `docs/changes/CH20.md`。
deep.md の答えはすべて `（答え待ち・推奨: …）` なので、推奨を答えとして読んだ。

## 機械の検査（先に写す）

| コマンド | 結果 |
|---|---|
| `openspec validate ch20-real-release-source-nightly-check --strict` | `Change ... is valid`（rc=0） |
| `python3 scripts/check_chain.py .` | `chain: OK (0 件 / 未回収 0 件 / warn 0 件)`（rc=0。change なので観点 8 は無い） |
| `python3 scripts/check_scenarios.py . ch20-real-release-source-nightly-check` | `scenarios: FAIL (担保なし 19 件 / 名無しの確認待ち 0 件)`（rc=1）。19 件はどれもこの change の ADDED / MODIFIED の Scenario。実装の前なので印が無いのは当然。tasks 1.2（13 本）・2.1（1 本）・6.1（5 本）が 19 本全部の `echo "== Scenario: <名前>"` を名指ししている |
| `python3 scripts/review_triage.py . ch20-real-release-source-nightly-check` | `triage: FAIL (19 件)`（rc=1）。全部が `review/deep.md` の `処置:` の書き方についての指摘（R1・R2 の kind=conflict が仮でない・R1〜R9・R14・R15 の fixed の指す先が D<n> / <x.y> / ファイル名でない・R10〜R12 の rejected に根拠が無い・R11 の loss=uncaptured を rejected・R15 の loss=exported を fixed）。出力のまま写し、書き方の判定はここではし直さない（中身に関わる R11 だけ下の R6 で触れる） |
| `python3 scripts/spec_overlap.py candidates . ch20-real-release-source-nightly-check` | `[]`（候補 0 件） |

---

## R1. 「本物の状態」の判定は、検査の側（資格情報つきの `fetch`）の答えから期待を決める。けれども部品（資格情報なし・共有の IP）が実際に受けた答えは残らないので、両者が違う夜の判定が決まっていない
- 成果物: openspec/changes/ch20-real-release-source-nightly-check/specs/distribution/spec.md / design.md
- 根拠:
  - 検査の側は資格情報つきで、部品の上限を食わないよう分けている（design.md:69-71 D4）。一方、本物の状態の期待は `fetch` の答えから引く（design.md:134-140 D10 の表「本物の答え｜期待」）
  - 部品の記録には成否・理由・版しか無く、受けた状態コードは無い（`crates/update-check/src/lib.rs:131-137` の `CheckRecord`）。残すと決めた「本物の答え」も `fetch` の分だけ（spec.md:83・design.md:40）
  - 共有の IP では部品だけが `403` を受けうる（design.md:157 Risks）。資格情報つきの `fetch` はそのとき `200` を受ける。
    - D10 の表どおりに判定すると、「200・`[]` → 確かめた・なし」に対して部品の「問い合わせの上限」は不一致で、赤になる。これは Q8 の推奨（その夜は注意）と食い違う
    - 逆に部品の理由を信じて「届かない夜」にすると、「部品がその理由を正典どおりに記録したか」（spec.md:196）を確かめる手段が無い
  - 同じ形で、`fetch` が `200` なのに部品が「接続できない」「時間切れ」を記録する夜もある（`crates/updater/src/lib.rs:175-176`。TLS の失敗などもここに落ちる）。D8 では 2 晩は注意だけになり、3 晩目に「届かない」という誤った理由で赤になる
  - Requirement の「その晩に本物の配布元へ届かなかった」（spec.md:196）も Scenario の「本物の配布元への問い合わせが `403` だった記録した晩」（spec.md:205）も、どちらの問い合わせの話かを言っていない。D8 は「検査の側か部品が」としている（design.md:119）
  - 比べに使う repo（`cli/cli`）への `fetch` が `403` / 時間切れのとき、D8・Req4 では注意、D12 では赤（「比べに使う repo の答えを読めない」で赤。design.md:152）。2 つが食い違う
  - 本物が `5xx` を返した夜の扱いも決まっていない。Req4 の「届かない」に `5xx` は無い（spec.md:196）。proposal は「同じ判定で見る」とする（proposal.md:40）が、その夜は流し直しの土台（`fetch` の本文）が無い
- kind: technical
- 処置: fixed specs/distribution/spec.md （「届かない」を誰の問い合わせで決めるかを Requirement に書いた: 部品が本物の段で上限・時間切れ・届かないを記録したらその段は期待と比べず届かなかった段に数え、検査の側が届かなければその答えに依る判定を飛ばす。部品の受けた答えは見えないことを本文に書いた。cli/cli は届かなければ注意・200 で読めなければ赤（D12 も直した）。5xx は届かない夜に数える。Scenario「部品が本物の段で届かなかったと記録すれば、その段は期待と比べない」「部品の知らない状態コードは形を比べずに注意を出す」「比べに使う repo の答えを読めなければ赤」を足した。design D10・D12 も合わせた）
- 提案:
  - 「届かない」を誰の問い合わせで決めるかを Requirement に 1 文で書く。例: 本物に当てる段の部品が「問い合わせの上限・時間切れ・配布元に届かない」を記録したらその部品の本物の状態は注意にし、`fetch` が届かなければ本物に依る判定（形・流し直し）を飛ばす
  - 部品の受けた状態は検査で確かめられないことを Requirement の本文に書く
  - `cli/cli` が届かない夜と本物の `5xx` の夜を、それぞれ Scenario にする

## R2. Scenario「空の一覧で…」は、3 つの部品がそろって正式版の道の `[]` に答えるとしている。開発版の版の更新係は印 `nightly` の道（`404`）に当たるので、この GIVEN は成り立たない
- 成果物: openspec/changes/ch20-real-release-source-nightly-check/specs/distribution/spec.md
- 根拠:
  - spec.md:121「記録した本物の答え（正式版の道が `200`・本文 `[]`）と、3 つの部品がそれに「確かめた・見つけた最新の版なし」を記録した」。THEN も「3 つの部品が「正典どおり」」（spec.md:123）
  - 一方で次の 4 か所は、`[]` に当たるのが 2 部品、`404` に当たるのが開発版の更新係と分けている:
    - design.md:67（試験の組み立ての更新係は `/releases/tags/nightly`）
    - `crates/updater/src/lib.rs:140-146`（版が `-dev.` なら印 nightly の道）
    - spec.md:131-135（開発版の道の 404 の Scenario）
    - `docs/changes/CH20.md:51-52`（正式版の道は「確かめた・新しい版なし」、開発版の道は「配布元が見つからない」）
- kind: technical
- 処置: fixed specs/distribution/spec.md （GIVEN と THEN を配る形の更新係と PC の収集アプリの 2 つにした）
- 提案: GIVEN と THEN を「配る形の更新係と PC の収集アプリの 2 つ」に直す（開発版の更新係は spec.md:131 の Scenario が受け持つ）。

## R3. Scenario「流し直しは版の名前だけを書き換え…」の THEN は、別の repo のリリースを土台にしたときには満たせない。D6・Requirement は配布物を自分の名前の写し 4 つに置き換えると言っている
- 成果物: openspec/changes/ch20-real-release-source-nightly-check/specs/distribution/spec.md
- 根拠:
  - GIVEN「記録した別の公開の repo のリリース 1 件」（spec.md:157）、THEN「`tag_name` と `assets[].name`・`browser_download_url` の**版の部分だけ**が元と違い、ほかの項目は元と同じ値」（spec.md:159）
  - 一方で次の 3 か所は、配布物の名前そのものを差し替え、数を 4 つに変えると言っている:
    - spec.md:113（「自分の部品の配布物の名前を差し込んで作る」）
    - design.md:87-88（D6）・tasks.md:54-55（`assets` を土台の最初の配布物の写し 4 つに置き換える）
  - たとえば `cli/cli` の `gh_<版>_linux_amd64.tar.gz` が `ashiato-server-<版>-windows-x86_64.zip` になる。選び方は名前の接頭辞と接尾辞で版を読む（`crates/update-check/src/lib.rs:302-313` の `asset_version`）ので、版の部分だけを書き換えても部品には数えられない
  - `browser_download_url` を元の repo（`github.com/cli/cli/...`）のまま残すかどうかも決まっていない
  - tasks 2.1 の自己試験（tasks.md:57-58）も同じ文言を写していて、どちらにも実装できる
- kind: technical
- 処置: fixed specs/distribution/spec.md （Scenario を「流し直しは版の名前と配布物だけを差し替え、ほかの項目を土台のまま残す」に直し、THEN を変わるもの・残るもので書いた）
- 提案: THEN を、何が変わり何が残るかで書き直す。例: 「`tag_name`・`assets` の数（4）・各 `assets[].name` は自分の配布物の名前、`browser_download_url` は〈決めた形〉、各配布物の `digest` と、リリースの `draft` ほかの項目は土台と同じ」。`rai-256/ashiato` のリリースを土台にする場合（反転条件。design.md:101）は、別の Scenario にする。

## R4. workflow の `collect` が 0 でない終了コードで落ちると、`history` と `judge` が走らない。赤の理由が要約に出ず、`summary.json` も残らない
- 成果物: openspec/changes/ch20-real-release-source-nightly-check/tasks.md / specs/distribution/spec.md
- 根拠:
  - tasks.md:100-101 の step は `fetch`→`collect`→`history`→`judge`→`upload-artifact` の順。`if: always()` が付くのは `upload-artifact` だけ（tasks.md:101）
  - 配る形の組み立て（`build.sh server-windows`）やインストールが落ちれば、`collect` は 0 でない終了コードで終わる。そのとき次の 3 つが起きる:
    - spec.md:84「WHEN 検査が赤で終わる THE SYSTEM SHALL 赤の理由を 1 行ずつ…要約に出す」が満たされない
    - 翌晩の `history` は前の `summary.json` を読めない。D8 では「読めない」→今夜だけで数えるになる（design.md:121）
    - 本物に届かない夜が続いても、数え直しが続いて赤にならない
  - `--check-workflow` の壊した写しの節（tasks.md:108-110）にも、この形の検査は無い
- kind: technical
- 処置: fixed 6.1 （history・judge・成果物の保存を if: always() にし、記録の無い段を judge が赤にする。records/mode を design D2 に書き、Scenario「判定と成果物の保存は、前の段が落ちても走る」「部品の記録が無い段は赤になる」と壊した写し (h) を足した）
- 提案: 次の 2 つのどちらかを tasks と design D2 に書き、`--check-workflow` の検査と壊した写しに加える。
  - `history` と `judge` の step に `if: always()` を付け、`collect` が落ちた理由を `judge` が赤の行として出す
  - `collect` が組み立てやインストールの失敗を `records/` に書いて 0 で終わる

## R5. 観測できる振る舞いが design と tasks にだけ置かれている（archive で正典から落ちる）
- 成果物: openspec/changes/ch20-real-release-source-nightly-check/design.md / tasks.md
- 根拠: どれも赤・緑・注意の出方を決めているのに、spec に無い:
  - D3 と tasks 4.1(a)（design.md:56・tasks.md:74-75）
    - 配る形の更新係のサービスが 180 秒のうちに確かめなければ赤
    - そのときの理由は「更新係のサービスが確かめなかった」
  - design Risks・tasks の Global Constraints（design.md:158・tasks.md:25）
    - 要約の先頭を「既に分かっている食い違い」と「それ以外の赤」に分ける
    - `docs/changes/CH20.md:50-51` の確認バッチの問いは、この分け方が在ることを前提にしている
  - D10 の本物の状態の期待（design.md:134-140）
    - 一覧の道が `200` でリリースがあれば、正典の選び方を当てた版になる
    - `403` と `429` は「問い合わせの上限」、ほかの状態コードは「読めない答え」
  - D5（design.md:79）
    - 部品の知っている状態コードは `200`・`301`・`302`・`307`・`308`・`403`・`404`・`429`
    - spec.md:164 の「状態コードが部品の知らない値なら赤」の中身はこの一覧で決まる。`5xx` は入っていないので、本物の `5xx` の夜は形の判定でも赤になる
  - D6 の場合ごとの期待（design.md:90-97）
    - 「同じ版 → 自分の版」「20 件 → 自分の版」「`301` の別の宛先 → 読めない答え」
    - spec の Scenario には一部しか無い
  - D8 の成果物の `retention-days: 30`（design.md:121）
    - 本物の答えを何日残すかの数値（R6）
- kind: technical
- 処置: fixed specs/distribution/spec.md （180 秒と理由・要約の分け方・本物の状態の期待・知っている状態コード・場合ごとの期待・保存の 90 日を Requirement に移し、180 秒と要約の分け方と状態コードの Scenario を足した。design には理由だけを残した）
- 提案:
  - 上の 6 つを ADDED の Requirement の本文と Scenario に移す
  - 特に 180 秒・状態コードの一覧・要約の分け方は、数値と語のとおり Scenario の THEN に書く
  - design には、なぜその値にしたかだけを残す

## R6. 毎晩の本物の答えを何日残すか（30 日）は、取らずに失うもの（review/deep.md R11 の uncaptured）の決定なのに、design の数値にしか無く、triage も落ちている
- 成果物: openspec/changes/ch20-real-release-source-nightly-check/design.md / review/deep.md
- 根拠:
  - review/deep.md:197 以降の R11 は、毎晩の本物の答えを残さなければ、いつから形が変わったかを後から引けないと指摘した。uncaptured の指摘
  - 処置は C の既定として design に残した。D5 は「毎晩の本物の答えは成果物に残す」（design.md:83）、D8 は `retention-days: 30`（design.md:121）
  - 30 日を過ぎた答えは取り直せない
  - spec の Scenario「本物の答えと部品の記録は、赤でも緑でも残る」（spec.md:104-107）には日数が無く、実装が 1 日にしても落ちない
  - `review_triage.py` は「deep.md R11: loss=uncaptured は人間に返すもの。rejected で閉じられない」で FAIL している
- kind: irreversible
- 処置: fixed D14 仮 （保存を 90 日（D13（仮）。review/deep.md R11 も同じ台帳）にし、spec の Requirement と Scenario に数値で書き、--check-workflow が 30 にした写しで落ちる（tasks 6.1 の (i)））
- 提案:
  - 残す日数（とそれを過ぎたら引けないこと）を Requirement と Scenario の THEN に数値で書く
  - R11 は escalated にするか、失わない側（長く残す・repo に置く等）を選んだ仮として `provisional.jsonl` に載せる

## R7. Q10 の推奨「1 回の実行で数回まで」は部品の 3 回にしか写っていない。検査の側の `fetch`（本物の配布元への問い合わせを含む）の回数と、PR の CI で `fetch` を走らせないことが spec に無い
- 成果物: openspec/changes/ch20-real-release-source-nightly-check/specs/distribution/spec.md
- 根拠:
  - Q10 の推奨: 「毎晩の検査だけは、1 回の実行で数回まで本物に問い合わせてよい」（deep.md:53）
  - MODIFIED が上限を付けるのは、部品が問い合わせる回数（3 回）だけ（spec.md:12）
  - 検査の側の 4 回は design D4（design.md:69）と tasks 5.1 の自己試験（tasks.md:92）にしか無い。そのうち `rai-256/ashiato` への 2 回は本物の配布元への問い合わせ
  - Scenario「PR ごとの CI は本物の配布元に問い合わせない」の THEN が名指すのは `collect` と `ch20_real_source` だけ（spec.md:73）。`fetch` は本物に資格情報つきで当たるのに入っていない
  - tasks 6.1 の検査は `fetch`・`--ignored`・`--include-ignored` も見る（tasks.md:106-107）。Scenario と tasks で範囲が違う
- kind: technical
- 処置: fixed specs/distribution/spec.md （検査の側の 4 回の 1 文と Scenario「検査の側が本物の答えを取るのは 1 回の実行で 4 回」を足し、MODIFIED の Scenario の THEN に fetch を加えた。tasks 5.1・6.1 も合わせた）
- 提案:
  - ADDED の「毎晩、本物の配布元との…」に、検査の側の問い合わせの回数（4 回）を数値で書いた 1 文と Scenario（`--plan` か `fetch` の自己試験で数える）を足す
  - MODIFIED の Scenario の THEN に `release-source-check.sh` の `fetch` を加える

## R8. Requirement の本文だけが言っていて、Scenario が言っていないことがある（実装が落としても spec が落ちない）
- 成果物: openspec/changes/ch20-real-release-source-nightly-check/specs/distribution/spec.md
- 根拠: 次の文には Scenario が無い。
  - spec.md:112: 流し直しの 6 つの場合のうち「`api.github.com` でない宛先への `301`」
    - tasks 4.1 の検証（tasks.md:81）では見るが、Scenario は無い
  - spec.md:163: 比べる項目の「本文の大きさ」
    - 何と比べて、どうなれば赤かが決まっていない。spec.md:164 の IF にも大きさは入っていない。基準の上限（65536）を超えたら赤か、記録だけかが読めない
  - spec.md:164: 「状態コードが部品の知らない値である THEN 赤」
  - spec.md:165: 「部品が読まない項目の…値の変化では赤にしない」
    - Scenario（spec.md:170-174）は「増えても」だけを見る
  - spec.md:196: 届かない夜は「本物の答えに依る判定（本物の状態・答えの形・流し直し）を飛ばし」
    - 飛ばしたことを観測する Scenario が無い
  - spec.md:199: 「本物に依らない流し直しの場合（`503`・別の宛先への `301`）を、届かない夜も判定する」
- kind: technical
- 処置: fixed specs/distribution/spec.md （301・一覧の本文の上限（超えたら赤）・知らない状態コード（注意）・値の変化（緑）・届かない夜に飛ばした判定の名前・届かない夜の 503 の Scenario を足した）
- 提案: 各文に Scenario を 1 つずつ足す（`judge` に記録した材料を当てる形でよい）。「本文の大きさ」は、赤にする条件を本文に書くか、比べる項目から外す。

## R9. 「3 晩続けば赤」は Scenario の名前と本文で単位が違う（晩と実行）。手動の起動が入ると数が変わる
- 成果物: openspec/changes/ch20-real-release-source-nightly-check/specs/distribution/spec.md / design.md
- 根拠:
  - Q8 の推奨と Scenario の名前は「3 晩続けて届かなければ赤」（deep.md:43・spec.md:209）
  - 本文と D8 は「前の 2 回の実行」で、`gh run list` の直近 2 回を数える（spec.md:197・design.md:120）
  - workflow は `workflow_dispatch` も持つ（spec.md:92）。そのため同じ晩に手で 2 回起こすと、1 晩で赤になる
  - CH20.md の確認バッチは「待てなければ手動で起動」を勧めている（docs/changes/CH20.md:49）
- kind: technical
- 処置: fixed specs/distribution/spec.md （晩を予定の起動の実行で数えると本文に書き、Scenario「手動の起動の実行は晩に数えない」を足した。design D8・tasks 5.1 も --event schedule にした）
- 提案: 数える単位を決めて本文と Scenario を揃える。予定の起動（`schedule`）だけを数えるか、実行の回数で数えて名前を「3 回続けて」にする。

## R10. Scenario に主張を束ねたものがある
- 成果物: openspec/changes/ch20-real-release-source-nightly-check/specs/distribution/spec.md
- 根拠:
  - spec.md:89-92「毎晩の検査は公開の流れから独立し、読む権限だけで走る」
    - THEN は (a) schedule と手動の起動 (b) 権限が 2 つだけ (c) `nightly.yml` に job も `needs` も無い、の 3 つの主張
  - spec.md:176-180「読む項目が消えるか型が変われば赤」
    - GIVEN が 2 つの答え（`digest` が無い・`draft` が文字列）で、THEN が「どちらも赤」
  - 片方だけ壊れても、もう片方が通れば印は緑のまま見える
  - tasks 6.1 の壊した写し（tasks.md:108-110）は (a)〜(g) を別々に見るので、検査の側では分かれている
- kind: technical
- 処置: fixed specs/distribution/spec.md （独立・権限・起動の 3 つと、消えた・型が違うの 2 つに Scenario を分けた）
- 提案: Scenario を主張ごとに分ける（名前も分ける）。

## R11. tasks 4.1 の検証は、`--replay-only` のときに本物に当てる 3 段の記録が無いことを `judge` がどう扱うかに依るのに、その扱いが書かれていない。(a)〜(c) の実地は検証が無いまま `[ ]` を閉じられる
- 成果物: openspec/changes/ch20-real-release-source-nightly-check/tasks.md
- 根拠:
  - tasks.md:81 の検証は、`--replay-only` の後の `judge` で `^NG\t(updater|collector)\t` がちょうど 5 行になることを求める
  - `--replay-only` は (a)〜(c) を飛ばす（tasks.md:78）。そのため `records/` には `updater real-dev`・`collector real` が無い
  - tasks 4.1(a) は、記録が来ないときに `judge` が `NG` を出すと決めている（tasks.md:74-75）。同じ扱いが `updater real-dev`・`collector real` にも及べば、7 行で落ちる。飛ばす扱いにしたのなら、どこにもそう書かれていない
  - 材料の別の repo の 1 件（オブジェクト）は `NG shape` も出す（spec.md:182-186）。正規表現は `shape` を数えないので検証は通る。けれども CH20.md の archive の判定「手元の流し直しで 3 つの食い違いの 5 行だけが赤」（docs/changes/CH20.md:45）は、出力の字面とは合わない
  - 4.1 の (a)〜(c)（本物と Windows のサービス）には検証のコマンドが無い。人間の確認待ち・merge 後の最初の実行で見るとも書かれていない。確かめるのは CH20.md:49 の確認バッチだけ
- kind: technical
- 処置: fixed 4.1 （replay-only の置き場では本物の 3 段を WARN「本物の段を飛ばした」にすると書き、検証にその 3 行を足した。(a)〜(c) は merge 後の最初の毎晩の実行で確かめると書いた。CH20.md の文言を部品の行の数に合わせた）
- 提案:
  - `judge` が `--replay-only` の置き場（本物の段の記録が無い）をどう出すかを tasks 4.1 に 1 文で書く。例: 飛ばした段は `WARN` 1 行
  - CH20.md の文言を「部品の行（updater・collector）で 5 行」に合わせる
  - (a)〜(c) に「merge 後の最初の毎晩の実行で確かめる（確認バッチ）」と書く

## R12. 下流が本物の配布元に触れる手順が tasks 1.2 に残っている（自己試験の材料を実装の時に `curl` で記録する）
- 成果物: openspec/changes/ch20-real-release-source-nightly-check/tasks.md
- 根拠:
  - tasks.md:18-19「材料のうち本物の答え（`[]`・`404`・別の repo のリリース 1 件）は、実装の時に 1 回だけ記録して commit する」
  - tasks.md:37-38 も、`rai-256/ashiato` の 2 つの道と `cli/cli` の 2 つの道を実装の時に記録するとしている
  - 下流（Task agent・Codex の sandbox）が外へ出られるかは決まっていない。出られなければ 1.2 で止まる
  - `rai-256/ashiato` の 2 つは前提の確かめで実物を取ってある（premises.md P1: `200`・`content-length: 2`・本文 `[]`。P2: `404`）
- kind: technical
- 処置: fixed tasks.md （本物の答え 4 つを上流で material/ に記録し、Global Constraints で写して使い取り直さないと書いた）
- 提案: 材料（4 つの答えの状態・見出し・本文）を上流のうちに `tools/release/release-source-check.d/` に置く。または、取れなければ人間の確認待ちとして止まる、と tasks に書く。下流の検証は本物に触れない、と Global Constraints に揃える。

## R13. `--check-workflow` が `yaml` を使うことと Global Constraints の「python は標準のものだけ・`python3 -I`」が食い違い、自己試験は `yaml` が無いと「飛ばした」で通る
- 成果物: openspec/changes/ch20-real-release-source-nightly-check/tasks.md / design.md
- 根拠:
  - tasks.md:22「python は標準のものだけ・`python3 -I` で呼ぶ」・design.md:46（D2「python3 の標準だけ」）
  - tasks.md:103「`--check-workflow`…（`python3` と `yaml` で読む）」・tasks.md:110「`python3` か `yaml` が無ければこの節を飛ばしたと 1 行出して続ける」
  - `python3 -I` は利用者の site-packages を読まないので、`pip --user` で入れた `yaml` は見えない
  - CH17 の前例では、`windows-build-key.sh --check-workflow` そのものは `yaml` が無ければ落ちる（`.github/workflows/ci.yml:724` のコメント）。自己試験の側は yaml があるときだけ走る（`tools/release/windows-build-key.sh:4`）
  - CH20 の tasks は、`--check-workflow` そのものが `yaml` の無いときに落ちるとも飛ばすとも書いていない（tasks.md:103）
  - CI の `chain` で `yaml` が見えなければ、壊した写しの節が飛んだまま緑になる。tasks.md:113 の「飛ばした」が無いことの検証は手元でしか走らない
- kind: technical
- 処置: fixed tasks.md （Global Constraints に --check-workflow だけの例外（yaml・-I なし・yaml が無ければ落ちる）を書き、6.1 にも書いた）
- 提案:
  - 例外を Global Constraints に書く。例: `--check-workflow` だけ `yaml` を使い、`-I` を付けない
  - `--check-workflow` そのものは `yaml` が無ければ落ちる、と tasks 6.1 に書く（CH17 の `--check-workflow` と同じ）

## R14. 長いコマンド（`cargo test`・`collect --replay-only` の組み立て）を `scripts/quiet-run` を通さずに走らせる検証がある
- 成果物: openspec/changes/ch20-real-release-source-nightly-check/tasks.md
- 根拠:
  - AGENTS.md「数十秒以上かかるもの…`cargo test`…は repo の根から `scripts/quiet-run` で走らせ」
  - tasks.md:68-69（3.1。`cargo test -p ashiato-collector-windows ...` を裸で 2 回）
  - tasks.md:81（4.1。`collect --replay-only` が更新係と PC の収集アプリを組んで場合ごとに `cargo test` を呼ぶ）
  - 7.2 は `scripts/quiet-run st57 -- cargo test ...` の形（tasks.md:129・131）
- kind: technical
- 処置: fixed 3.1 （3.1 の 2 本目と 4.1 の 2 本目を scripts/quiet-run で走らせる形にした）
- 提案: 3.1 の 2 本目と 4.1 の 2 本目を `scripts/quiet-run <名前> -- bash -c '…'` の形にする（`--list` の数え上げは 7.2 と同じく裸でよい）。

## R15. 食い違い (1) は PC の収集アプリにもあるのに、流し直しは更新係にしか当てない（PC の収集アプリは開発版の版で組めるのに組んでいない）
- 成果物: openspec/changes/ch20-real-release-source-nightly-check/design.md / proposal.md
- 根拠:
  - proposal.md:38「Rust の 2 部品は配列しか読まず」。PC の収集アプリも `parse_releases` で読む（`crates/collector-windows/src/update_check.rs:213-214`・`:409-410`）
  - D6 の表・tasks 4.1(d) は `nightly-object` を「更新係だけ」に当てる（design.md:94・tasks.md:76）
  - PC の収集アプリの版は `ASHIATO_BUILD_VERSION` で同じ土台の `-dev.` を付けて組める（`crates/build-stamp/src/lib.rs:41-58`。土台が同じなら通る）。組めないのは土台の違う版だけ
  - 直しの change が更新係の側だけで直すと、PC の収集アプリの食い違いは赤のまま気づかれない
- kind: technical
- 処置: fixed D7 （PC の収集アプリを開発版の版でもう 1 つ組み、nightly-object を当てる（本物には当てない）。proposal の表 (1)・spec・tasks 4.1 の 6 行を合わせた）
- 提案: 次のどちらかにする。
  - PC の収集アプリを開発版の版（`<土台>-dev.<日>`）でもう 1 つ組み、`nightly-object` を当てる
  - 当てないなら、proposal の表 (1) に「PC の収集アプリは見ない（理由）」と書く

## R16. proposal の Impact は「改めるのは本文の 1 文だけ」と言うが、MODIFIED は 1 文の書き換えに加えて SHALL を 1 文と Scenario を 1 つ足している
- 成果物: openspec/changes/ch20-real-release-source-nightly-check/proposal.md
- 根拠:
  - 正典 `openspec/specs/distribution/spec.md`（2515 行からの Requirement）と delta の MODIFIED を diff した結果、違いは 4 つ:
    - 本文 9 行目の書き換え 1 文
    - SHALL の追加 1 文（spec.md:12）
    - 経緯の行の追加 1 行（spec.md:15）
    - Scenario「PR ごとの CI は本物の配布元に問い合わせない」の追加（spec.md:70-73）
  - 既存の 9 つの Scenario は名前も中身も一字も変わっていない（diff に出ない）。proposal.md:53-54 の主張はそのとおり
  - proposal.md:70「改めるのは正典の Requirement の本文の 1 文だけ」
- kind: technical
- 処置: fixed proposal.md （「本文の 1 文を書き換え、上限の 1 文と Scenario 1 つを足す」に直した）
- 提案: proposal.md:70 を「本文の 1 文を改め、上限の 1 文と Scenario 1 つを足す」に直す。


## R17. tasks 1.2 の自己試験の Scenario のうち 2 つは、2.1 で組む流し直しの答えが無いと THEN を出せない（1.2 が 2.1 より前にある）
- 成果物: openspec/changes/ch20-real-release-source-nightly-check/tasks.md
- 根拠:
  - tasks 1.2 は自己試験で「20 件の一覧を読めなければ赤」を確かめる（tasks.md:42）。この Scenario の THEN は「場合の行に流し直した本文のバイト数が出る」（spec.md:153）
  - 「印 nightly のリリース 1 件を読めなければ赤」の GIVEN は「別の公開の repo のリリース 1 件の形に…差し込み、オブジェクトのまま流し直した答え」（spec.md:145）
  - 1.2 の材料は「本物の答え・場合ごとの確かめた記録・前の実行の要約」だけ（tasks.md:37-38）。流し直した本文を置くとは書いていない
  - 流し直しの答えを組むのは 2.1（tasks.md:53-55）で、1.2 より後
  - `judge` が本文のバイト数をどこから読むか（2.1 で組んだ答えか、`collect` が置き場に書いた本文か、手で書いた材料か）が tasks・design D2 に無い
- kind: technical
- 処置: fixed 2.1 （nightly-object と full-20 の 2 つの Scenario の確かめを 2.1 に移し、judge が本文のバイト数を replay/<場合>.body から読むと design D2 に書いた）
- 提案:
  - `judge` が流し直しの本文の大きさを置き場のどのファイルから読むかを design D2 か tasks 1.1 に書く
  - 1.2 の材料にその形のファイルを手で置く。そうしないなら、2 つの Scenario の確かめを 2.1 へ移す

---

## 観点ごとの確かめた範囲

### 観点 1: deep の決定が正典に写っているか

| Q | 推奨 | 写った先（spec / tasks） | 判定 |
|---|---|---|---|
| Q1 | 原因探しは任せる | 何にも効かない（proposal の Why・design の Non-Goals） | 写っている |
| Q2 | 更新係と PC の収集アプリ | spec.md:79-80・:87 / tasks 4.1(a)〜(c) | 写っている |
| Q3 | 更新係は配る形・PC の収集アプリは試験の組み立て | spec.md:80・Scenario spec.md:94-97（`--plan` の段の一覧） / tasks 3.1・4.1(a)(c) | 写っている。配る形のサービスを待つ振る舞いは design だけ（R5） |
| Q4 | その晩の答えを流し直し、版の名前を書き換える | spec.md:112・Scenario :125-129・:155-159 / tasks 2.1・4.1(d) | Scenario :155-159 が D6 と食い違う（R3） |
| Q5 | 読む項目だけを比べる | spec.md:163-165・Scenario :170-186 / tasks 1.1・1.2 | 写っている。「本文の大きさ」の判定が空（R8） |
| Q6 | 別の公開の repo の最新のリリース | spec.md:113・:166・Scenario :188-192 / tasks 1.2・2.1・5.1 | 写っている |
| Q7 | 正典どおりなら緑・食い違いを要約に | spec.md:114・Scenario :131-135 / tasks 1.1・1.2 | 写っている。「空の一覧」の Scenario が開発版の道と混ざっている（R2） |
| Q8 | 注意・3 晩で赤 | spec.md:196-199・Scenario :203-219（数値 3 晩・403 あり） / tasks 1.1・1.2・5.1 | 誰の「届かない」か（R1）、単位が晩か実行か（R9） |
| Q9 | 製品は直さず、検査は 3 つを捕まえる | spec.md:116-117・tasks の Global Constraints・7.2・4.1 の 5 行 | 写っている（R15 は (1) の範囲） |
| Q10 | PR の CI は問い合わせない・毎晩の検査は数回まで | MODIFIED spec.md:11-12・Scenario :70-73 / tasks 4.1（`--plan` 3 行）・5.1（4 回）・6.1・6.2 | 部品の 3 回だけで、検査の側の回数と `fetch` が無い（R7） |

当初案を覆したもの（R7 の「入っている版を変える」→「一覧の版の名前を書き換える」、R8 の「PC の収集アプリも配る形」→「試験の組み立て」）は、覆した後の内容だけが spec・design・tasks に残っている（`grep -n "1 つ古い\|古い版で回す\|配る形の PC\|PC の収集アプリは配る" specs/distribution/spec.md design.md tasks.md proposal.md` は 0 件・rc=1）。
要件へ戻すもの: 無い（FR-116 は変えない。proposal.md:69-70）。

### 観点 2: Scenario が検証可能か
R1・R2・R3・R8・R9・R10。

### 観点 3: 置き場の誤り
- design に置かれた観測できる振る舞い: R5（R6 の日数を含む）
- specs に入っている実装の名前: 台本の名前（`release-source-check.sh` の `collect` / `judge` / `--plan`）、試験の名前（`ch20_real_source`）、環境変数（`GH_TOKEN`）。正典にも同じ形の前例がある（既存の Scenario の `update-check-install.sh --block-outbound`・`ASHIATO_UPDATER_TEST_`）ので、指摘にはしない
- Capabilities: proposal の Modified は `distribution` だけで、`specs/` も `distribution/` だけ。`CH20.md` の `capability: [distribution]` とも合う。change なので INDEX の Story の表には載らない（`docs/stories/INDEX.md:108` の `distribution` の行は Story だけ）

### 観点 4: tasks が検証を持つか
- R11・R12・R13・R14・R17
- 1.1 の検証は構文だけ（`bash -n`・`py_compile`。tasks.md:35-36）で、中身は 1.2 の自己試験が見る。1.2 が同じ Task の続きなので、指摘にはしない
- 依存の順: 4.1 は 2.1 の受け手と 3.1 の入口を使い、6.1 の workflow は 4 つの口を呼ぶ。この並びは順どおり。1.2 と 2.1 の間に逆向きが 1 つある（R17）
- 6.2 の差分の数え方（`^[+-][^+-]` が 2 行）は、`+++`・`---` の見出しを数えず、足した 2 行だけを数える。書いたまま走る

### 観点 5: change の CH20.md
- 「完了の判定」は空でない（docs/changes/CH20.md:42-53）。archive の判定の「5 行だけが赤」の字面が出力と合わない（R11）
- 確認バッチの問いが前提にしている「既に分かっている食い違い / それ以外の赤」の分け方は、spec に無い（R5）
- 「壊してはいけないもの」:
  - 製品のコード・触れない台本・共有の材料が変わらないこと: tasks 7.2 の `git diff` で固定
  - いまの偽の配布元の試験が通ること: `st57_once`・`st57_pc_update`・`update-check-install.sh --self-test`（tasks.md:127-132）
  - PR の CI が本物に触れないこと: Scenario spec.md:70-73。`fetch` が入っていない（R7）
  - `nightly.yml` から独立していること: Scenario spec.md:89-92
- 依頼の範囲を越えた・欠いたもの:
  - 依頼の「最新が自分と同じ版・新しい版がある」は、流し直しの Scenario（spec.md:125-129）で受けている
  - 「応答の形が変わったことも拾う」は Requirement 3 で受けている
  - 依頼の外に出たもの（`503`・別の宛先への `301`・20 件）は、Q9 の推奨（3 つを捕まえる）に依ると spec.md:116 と design.md:102 に書かれている

### (b) 3 つの食い違いを「初日から赤」で捕まえるか（コードで確かめた範囲）
- 流し直しの側は、コードのとおりなら tasks 4.1 の 5 行になる:
  - (1) 更新係: `parse_releases` は配列だけを読む（`crates/update-check/src/lib.rs:172-178`）。オブジェクトは `Unreadable` になる（`crates/updater/src/lib.rs:234-235`）。期待は「確かめた」なので赤
  - (2) 更新係と PC の収集アプリ: 64 KiB を超えると `TooLarge` → `Unreadable` で、赤。`nightly-notes.md` は 464 バイト、配布物 4 つの 1 件は約 9 KB（review/deep.md:106-107）なので、20 件で上限を十分に超える
  - (3) PC の収集アプリ:
    - `503` は `StatusCode(503)` を経て `Unreachable` になる（`update_check.rs:479-485`）
    - 別の宛先への `301` も `Unreachable` になる（`update_check.rs:394-397`）
    - ureq 3.4.1 は `max_redirects(0)` のとき 3xx を応答として返す（`config.rs:203`。`max_redirects > 0 && max_redirects_will_error`）ので、この道に入る
  - 更新係は `503` を `Unreadable`、別の宛先を `Unreadable` と記録する（`crates/updater/src/lib.rs:174`・`:189-193`）ので緑。`same`・`newer` は選び方が配布物の名前だけで決まり、`browser_download_url` と `digest` を見ない（`choose`）ので緑
- 形の比べの側: 本物の `rai-256/ashiato` の 1 件の道は今 `404` で本文が無い。そのため「1 件の道がオブジェクト」の赤は、`cli/cli` の `releases/tags/<印>` を見て出る（spec.md:182-186）。成り立つ
- 本物の状態の側は R1 のとおり、`fetch` と部品の答えが違う夜の判定が決まっていない。配る形の段の組み立て・インストールの失敗は、R4 のとおり赤の理由を残さない

## 観点 6: 答えが写ったか
該当なし（mode は `spec --draft`。名指しの Q は無い）。

観点 7: 該当なし（spec_overlap.py candidates が 0 件。`spec_overlap.py check` も `{"ok": true, "candidates": 0}`）
