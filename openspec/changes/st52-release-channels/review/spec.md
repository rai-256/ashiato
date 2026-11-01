# ST52 上流の成果物の独立検査（spec-review。mode: spec --draft）

対象: `proposal.md` / `specs/distribution/spec.md` / `specs/record-envelope/spec.md` / `design.md` / `tasks.md`、`docs/stories/ST52.md`、`docs/stories/INDEX.md` の訂正の節。
照らしたもの: `deep.md`（Q1〜Q11。答え待ちの推奨を答えとして読む）・`deep-questions.json`（推奨の選択肢）・`premises.md`（P1〜P16・C1〜C9）・`provisional.jsonl`（D1〜D6）・`review/deep.md`（R1〜R16）・`docs/handoff/ST52.md`・正典 `openspec/specs/distribution`・`record-envelope`（作業木と `origin/main`）。

## 機械の検査（最初に写す）

```
$ openspec validate st52-release-channels --strict
Change 'st52-release-channels' is valid

$ python3 scripts/check_chain.py .
  要件 171 件 / Story 58 本 / 扉 35 項
  [ok]   どれかの Story に拾われた要件: 169/171 件
  [ok]   INDEX の「Story の対象外」: NFR-11, NFR-8
chain: OK (0 件 / 未回収 0 件 / warn 0 件)

$ python3 scripts/check_scenarios.py . st52-release-channels
  Scenario 2349 件 / 印 2805 個 / 担保あり 2288 / 人間の確認待ち 1
  [FAIL] 担保の無い Scenario: …（この change の新しい Scenario 60 本。全部 st52-release-channels/specs の中）
scenarios: FAIL (担保なし 60 件 / 名無しの確認待ち 0 件)

$ python3 scripts/review_triage.py . st52-release-channels
  指摘 16 件 / 仮決め 5 件 / 要件へ戻すもの 0 件
  [FAIL] deep.md R1 / R2 / R3 / R4 / R6 / R7 / R8 / R12: escalated なのに deep.md に R<n> が無い
triage: FAIL (8 件)

$ python3 scripts/spec_overlap.py candidates . st52-release-channels
[]
```

- `scenarios: FAIL 60` は上流の段では予定どおり（印は下流が置く）。60 本は tasks が置き場を名指す本数と一致する（観点 4 の (c)）。写した MODIFIED の 23 本は全部、既存の印で担保がある
- `triage: FAIL 8` は指摘にする（R5）

---

## R1. 版の記録の正典の Scenario「組み立ての版番号が `crates/server` の版番号と一致」が、ADDED の「渡した版番号を持つ」と食い違ったまま残る（MODIFIED が要る）
- 成果物: openspec/changes/st52-release-channels/specs/distribution/spec.md
- 根拠: 正典 `openspec/specs/distribution/spec.md:37-39`（Requirement「DB に当てた移行の版を、DB の中に記録する」の Scenario「版の記録の 1 行は、名前・指紋・時刻・組み立ての版番号と commit を持つ」。THEN「組み立ての版番号が `crates/server` の版番号と一致」）/ delta `specs/distribution/spec.md:11`（ADDED「版の記録の「当てた組み立ての版番号」…に、同じ組み立ての同じ版番号を使う」）・`:38-41`（ADDED の Scenario「サーバの版の記録と db_ahead の行は、渡した版番号を持つ」→ `0.4.0-beta.2`）/ design.md:95（`BUILD_VERSION` を `env!("ASHIATO_VERSION")` に）/ この Scenario の印の試験 `crates/server/src/migration_ledger_tests.rs:334`（`assert_eq!(version, env!("CARGO_PKG_VERSION"))`）。`crates/server` の版番号（`CARGO_PKG_VERSION`）は版番号を渡しても出所の版 `0.4.0` のままなので、`ASHIATO_BUILD_VERSION=0.4.0-beta.2` の組み立てでは正典の Scenario は false になる。今は試験の組み立てが版番号を渡さないので、両方が緑のまま食い違う
- kind: technical
- 提案: ST48 の Requirement「DB に当てた移行の版を、DB の中に記録する」を MODIFIED に加える。Scenario の THEN を「組み立ての版番号が、その組み立ての版番号（渡した版番号。渡さなければ出所の版）と一致」にして `crates/server` という実装の名前を外す。ADDED の Scenario「サーバの版の記録と db_ahead の行は…」はその Requirement の下へ移す。tasks 2.2 で `migration_ledger_tests.rs:334` の比べ先を、印の版番号の部分に替える（定数と定数を突き合わせない —— testing.md §3）
- 処置: fixed specs/distribution/spec.md —— ST48 の「DB に当てた移行の版を、DB の中に記録する」を MODIFIED に加え、THEN を「その組み立ての版番号（渡した版番号。渡さなければ出所の版）と一致」にした。既存の試験の比べる値は tasks 2.2 で直す

## R2. ADDED「サーバの版の記録と db_ahead の行は、渡した版番号を持つ」の THEN は版の記録しか見ず、db_ahead の `self:` の行と更新の履歴の版は Scenario が無い
- 成果物: openspec/changes/st52-release-channels/specs/distribution/spec.md
- 根拠: delta `specs/distribution/spec.md:11`（Requirement 本文は「組み立ての印・版の記録・更新の履歴の版・`db_ahead` の行」の 4 つ）/ `:38-41`（題は db_ahead を名指すが、THEN は「版の記録のどの行も…`0.4.0-beta.2`」だけ）/ tasks.md:56（`st52_bin_ledger_build_version` は「版の記録の `build_version` を読む」だけ）。db_ahead の `self:` は `migration_ledger.rs:53,284` が `BUILD_VERSION` から、更新の履歴は印から作るので、片方だけが取り違えても落ちない
- kind: technical
- 提案: Scenario を 2 つに分ける:「…渡した版番号で組んだサーバが、知らない版を持つ DB で止まると、`  self:` の行の版番号は `0.4.0-beta.2`」「…migrate の後、更新の履歴の最後の行の今の版は `0.4.0-beta.2+` で始まる」。tasks 2.2 の試験を 2 本にし、`-- --list` の下限を 2 にする
- 処置: fixed 2.2 —— ADDED の Scenario を版の記録・db_ahead の `self:`・更新の履歴の 3 本に分け、2.2 の試験を 3 本（下限 3）にした

## R3. 携帯と画面の版番号の出所を根の `Cargo.toml` に移すのに、その 2 部品の「ソース」に `Cargo.toml` が入らない —— 出所を書き換えて commit しない組み立てが、`.dirty` の無い綺麗な印で別の版番号を名乗る
- 成果物: openspec/changes/st52-release-channels/specs/record-envelope/spec.md
- 根拠: delta `specs/record-envelope/spec.md:8`（4 部品とも出所の版）・`:9`（「その部品のソース」は正典のまま。S-01・C-02 は workspace の宣言を含むが、C-01 は「携帯の収集アプリの木」、V-01 は「画面の木」だけ）/ `collector-android/app/build.gradle.kts:212`（`.dirty` は `git status --porcelain -- collector-android` だけを見る）/ `web/build-stamp.ts:12`（`SOURCE_PATH = ":/web"`）。今は版番号が `build.gradle.kts`・`web/package.json` にあり、それぞれのソースの中なので、書き換えれば `.dirty` が付く。ST52 の後は、根の `Cargo.toml` の `version` を未 commit で `0.9.0` にした組み立てが、`0.9.0+<HEAD>` を `.dirty` 無しで名乗る（HEAD の `Cargo.toml` は別の版）。この印は、後から直せない記録に焼き込まれる（record-envelope の導出元 :12）
  あわせて、サーバに取り込んだ画面の印（V-01）を配布物の検査が読むかが書かれていない（design.md:65 は「4 つの配布物…どれも組み立ての印を持ち」。design.md:108 は画面の版を画面自身の印に任せる）
- kind: technical
- 提案: MODIFIED の「その部品のソース」で、C-01 と V-01 に「出所の版を持つ根の `Cargo.toml`」を足す（`Cargo.toml` の依存の書き換えでも `.dirty` になるのを避けたいなら、作業木から読んだ出所の版が `git show HEAD:Cargo.toml` の版と違えば `.dirty`、でもよい。どちらにしたかを本文に書く）。record-envelope に Scenario「根の `Cargo.toml` の出所の版だけを未 commit で書き換えた木で、携帯と画面の印は `.dirty` を持つ」を足す。D7 に「サーバの配布物に取り込んだ画面の印も読み、`.dirty` / `.debug` と版番号を見る」を足す
- 処置: fixed specs/record-envelope/spec.md —— C-01 と V-01 の「その部品のソース」に根の `Cargo.toml` を足し、Scenario「根の Cargo.toml を書き換えて commit していなければ、携帯と画面の印は .dirty を持つ」を足した（tasks 3.2・3.4）

## R4. 公開の設定が入っていない間（ST53 の後まで）、開発版の予定の実行は変更の無い日も毎日 4 部品を組む —— FR-107「変更の無い日は作らない」と、壊してはいけないもの「Actions の枠」に反する
- 成果物: openspec/changes/st52-release-channels/specs/distribution/spec.md
- 根拠: delta `specs/distribution/spec.md:207`（`nightly` と HEAD が違えば組む）・`:209`（設定が無い間は `nightly` を動かさない）・`:229-232`（Scenario「印 nightly がまだ無ければ、作り直す」）/ design.md:121（「入っていない間は組み立てと検査までで終え、印 `nightly` も動かさない」）/ premises.md P11（今の印に `nightly` は無い）。`nightly` が無く、動かさないので、`nightly-plan` は毎日 `build=yes` を返す。組み立てには Windows の runner の 2 部品が入る（design.md:83）/ docs/requirements.md:977-978（FR-107「変更の無い日は作らない（…Windows の実行は 2 倍で数えられ、過去に枠が尽きた）」）/ docs/stories/ST52.md「壊してはいけないもの」の 1 行目。P6 の「公開リポジトリでは課金されない」は確かさ 2
- kind: technical
- 提案: 公開の設定が無い間は、予定の実行は組まずに終える（`nightly-plan` が `publish=no` なら `build=no`。手で起こす `workflow_dispatch` だけ組む）、を Requirement と Scenario にする（「公開の設定が無ければ、`nightly` が無くても予定の実行は組まない」）。D15 をそれに合わせる
- 処置: fixed D15 —— 公開の設定が無い間の予定の実行は `build=no`（何も組まない）、手で起こした実行だけ組んで公開しない。Requirement・Scenario 3 本と tasks 1.1・9.3 を合わせた

## R5. review/deep.md で escalated にした 8 件（R1・R2・R3・R4・R6・R7・R8・R12）が deep.md に名指されず、triage が落ちる
- 成果物: openspec/changes/st52-release-channels/deep.md
- 根拠: `python3 scripts/review_triage.py . st52-release-channels` → `triage: FAIL (8 件)`。判定は `scripts/review_triage.py:206-208`（escalated の R の番号が deep.md の本文に無ければ落とす）。deep.md の Q1〜Q11 は R の番号を 1 つも書いていない（deep.md:1-58）
- kind: technical
- 提案: deep.md の該当する Q の節に出所を 1 行ずつ書く（Q1 ← R1・R5、Q2 ← R2、Q11 ← R3、Q3 ← R4、Q4 ← R6、Q5 ← R7・R8、Q6 ← R12）。record の段で答えに書き換えても行が残る置き方にする
- 処置: fixed deep.md —— 「問いの出所（review/deep.md の指摘）」の節を足し、Q ← R の対応を書いた

## R6. 扉 34 の改訂（鍵の置き場に GitHub の Environment を足す）が「要件へ戻すもの」として台帳に無い。開発版の鍵の控え（Q11 の推奨の「USB に 1 部」）も成果物に写っていない
- 成果物: openspec/changes/st52-release-channels/deep.md
- 根拠: `review_triage.py` の出力「要件へ戻すもの 0 件」/ proposal.md:65（「`docs/requirements.md` の扉 34（鍵の置き場に GitHub の Environment を足す。Q2 の推奨。record の段が ★ で改訂する）」）/ delta `specs/distribution/spec.md:171`（同じ）/ docs/requirements.md:1770（扉 34 は今「手元の USB と拠点外のバックアップの 2 か所」）。spec が要件の今の本文と違う置き場を書いているのに、record の段が拾う機械の印が無い。deep-questions.json の Q11 の推奨「開発版の Environment に置き、控えは USB に 1 部」の「控え」は、design.md:146（D19 の手順は「控えを USB と拠点外に置く（扉 34）」で正式な鍵だけ）にも tasks.md:163 にも無い
- kind: technical
- 提案: deep.md に「要件へ戻すもの」の節を置き、扉 34 の改訂（Q2 の答えしだい）を書いて triage が数えるようにする。D19 と tasks 11.1 の `docs/release.md` の項目に「開発版の鍵の控え（USB に 1 部）」を足し、11.1 の grep に入れる
- 処置: fixed deep.md —— 「要件へ戻すもの（答えしだい）」に扉 34 の改訂を置いた（record の段が ★ で改訂する）。開発版の鍵の控え（USB に 1 部）を D19 と tasks 11.1 の項目と grep に足した

## R7. ST52.md の「完了の判定」の 1 行目と 3 行目、「価値」の「自分の環境をその版で動かす」は、Q5・Q2・Q1 の推奨のもとでは ST52 の確認バッチで観測できない
- 成果物: docs/stories/ST52.md
- 根拠: docs/stories/ST52.md:55（「`v0.x.y` の印を付けると、配布元の正式版に 3 部品が同じ版で並び…」）・:57（「携帯の APK は正式な鍵で署名されている」）・:14（「自分の環境をその版で動かしたい」）/ deep-questions.json Q5 の推奨の detail（「ST52 の完了の判定のうち「印を付けると並ぶ」は下書きで確かめ、本物の公開は ST53 の確認で見る」）/ design.md:19-22（Non-Goals: 本物の印を打たない・本人の携帯もサーバも移さない・鍵を作らない）/ tasks.md:167-170（確認バッチは下書きと手順書を読むだけ）。下書きの APK は PR の組み立てで debug 鍵（design.md:128）
- kind: conflict
- 提案: 推奨のまま答えが来たら、record の段で完了の判定を「印の判定と 3 部品の組み立てを下書きのリリースで確かめる」「正式な鍵で署名する手順が使い捨ての鍵で通り、鍵を置けば印で署名される」に書き換える（本物の公開・正式な鍵・本人の環境の切り替えは ST53 の確認へ送る）。答えが「ST52 で公開を始める」なら今の本文のままでよい。どちらでも、archive の判定の前に完了の判定と spec を合わせる
- 処置: fixed D22 仮 —— 推奨のもとでの完了の判定の読み替え（下書き・使い捨ての鍵・手順書）を D22（仮）に置き、Q5 に依る反転条件と record の段での stories.json の書き直しを書いた。provisional.jsonl に載せた

## R8. 観測できる振る舞いが design にだけある（D17 の配布物の名前・D18 の識別子とチャンネルの取り違えで公開しない・D12 の OpenAPI の版）
- 成果物: openspec/changes/st52-release-channels/design.md
- 根拠: design.md:134-135（D17。配布物の 4 つの名前。「名前は ST57・ST58 が版の確認で読むので、変えるときは後続の Story と一緒に変える」）。delta spec は「4 つ」と並びしか言わない（`specs/distribution/spec.md:89`・`:101`）。archive は main specs しか更新しないので、ST57・ST58 が読む名前が正典から落ちる / design.md:141（D18「署名した後に…識別子がチャンネルのものか」を見て公開しない）・tasks.md:123（「取り違えは rc≠0」）に当たる Scenario が無い / design.md:98（D12「OpenAPI の `info.version` は出所の版のまま」）。delta `:11` は「組み立ての印・版の記録・更新の履歴・db_ahead に同じ版番号」と言い、API 契約の版がその外にあることを言わない
- kind: technical
- 提案: D17 の名前を Requirement「印（タグ）の名前で、3 つのチャンネルに分けて配る」の本文に移し、Scenario「正式版の印で…」の THEN に 4 つの名前を書く。D18 の取り違えに Scenario「開発版の識別子の APK を正式な鍵で署名しようとすると、署名せずに 0 でない終了コードで終わる」を足す。D12 の OpenAPI の版を Requirement 1 の本文に 1 文で書く（「API 契約の版は出所の版のまま、渡した版番号を入れない」）
- 処置: fixed specs/distribution/spec.md —— D17 の配布物の名前を Requirement の本文と Scenario「1 つのリリースの配布物は、決めた名前の 4 つで、同じ版番号を持つ」に、識別子と鍵の取り違えを本文と Scenario に、OpenAPI の版を本文と Scenario に移した

## R9. (b) PR の組み立ての APK の署名と、配る組み立ての「署名しない」の分け目が、`ASHIATO_RELEASE=1` 1 つに載っていて食い違う。指紋の期待値の出所も無い
- 成果物: openspec/changes/st52-release-channels/design.md
- 根拠: design.md:139（D18「`ASHIATO_RELEASE=1` のときは `signingConfig` を当てない」）/ design.md:105（D13。`ASHIATO_RELEASE=1` は「配る組み立て」で、画面が無ければ panic）/ design.md:113-114（D14。PR の `package` は「携帯の APK（release・debug 鍵で署名）」を `build.sh` で組む）/ delta `specs/distribution/spec.md:303`（PR は「配るときと同じ手順で」組む）・`:297-299`（配る組み立てで画面が無ければ失敗）。PR の `build.sh` が `ASHIATO_RELEASE=1` を立てれば APK は署名されず、Scenario「PR の組み立ての APK は debug の鍵で署名される」（`:183-185`）が立たない。立てなければ、PR は「配る組み立てで画面が無ければ失敗」の守りを通らず、配るときと同じ手順にならない。どちらにするかがどこにも書かれていない
  また、tasks.md:114 の `check-artifacts.sh` は署名の指紋を `ASHIATO_EXPECT_CERT_SHA256` と比べるが、D18 の Secrets の一覧（design.md:140）にも tasks 9.2 の `publish` の手順（tasks.md:144）にも、その値の置き場が無い
- kind: technical
- 提案: 署名の切り替えを `ASHIATO_RELEASE` から分ける（例: `ASHIATO_APK_UNSIGNED=1` は `release.yml`・`nightly.yml` の組み立ての job だけ）。PR は `ASHIATO_RELEASE=1` のまま debug 鍵で署名する、と D14・D18 に書く。指紋の期待値は Environment の変数（秘密でない）に置く、と D18 と tasks 9.2 に足す
- 処置: fixed D18 —— 署名しない切り替えを `ASHIATO_APK_UNSIGNED=1`（release.yml・nightly.yml の組み立ての job だけ）に分け、PR は `ASHIATO_RELEASE=1` のまま debug 鍵で署名する。指紋の期待値は Environment の変数 `ASHIATO_CERT_SHA256`（D18・tasks 9.3）

## R10. (b) 配布用の Environment を枝 `main` にも開く理由が設計のどの経路にも無く、開くと main に入った workflow の変更から正式な鍵が読める
- 成果物: openspec/changes/st52-release-channels/specs/distribution/spec.md
- 根拠: delta `specs/distribution/spec.md:165`（正式な鍵は「印 `v*` の push と main からの実行にだけ開いた置き場」）・`:189`（配布用の Environment の配る対象は「印 `v*` と枝 `main`」）/ design.md:115（正式な鍵を使うのは `release.yml` の `publish` だけで、`on: push: tags: ['v*']`）・design.md:116（`nightly.yml` は Environment `nightly`）。正式な鍵を main の実行で使う job は無い / `.github/workflows/ci.yml:3`（`push: { branches: [main] }`。main の上の workflow は main から走る）。MEMORY の「見終えた PR は機械が merge」により、agent が書いた workflow の変更は機械の merge で main に入りうる。Q2 は A で答え待ち（推奨の label に「印と main」）
- kind: irreversible
- 提案: 配布用の Environment の配る対象を印 `v*` だけにする（失うものは無い。`release.yml` は印の ref で走る）。Q2 の推奨の文面を「印にだけ開いた Environment」に直して答えを受けるか、`main` を残すなら、main から正式な鍵を使う job が要る理由を D18 に書く。あわせて `check-guards.sh environments` を `publish` の前にも走らせる（tasks.md:144 は `rules` だけ）
- 処置: fixed D21 仮 —— 配布用の Environment を印 `v*` にだけ開く（失うものは無い。release.yml は印の ref で走る）。Q2 の推奨の狭い側として provisional.jsonl に載せ（rests_on Q2）、spec の本文・Scenario・tasks 9.1・11.1 を合わせた

## R11. (b) Scenario「PR の組み立ての APK は debug の鍵で署名される」の後半「どの鍵の置き場も読まない」に、観測の手段が無い
- 成果物: openspec/changes/st52-release-channels/specs/distribution/spec.md
- 根拠: delta `specs/distribution/spec.md:183-185`（THEN「証明書は debug の鍵のもので、正式な鍵の置き場も開発版の鍵の置き場も読まない」）/ tasks.md:124（印を置く試験は `sign-apk.sh --self-test` で、証明書が `~/.android/debug.keystore` のものかだけを見る）。「読まない」は Environment の守り（P12。tasks 9.2 の使い捨ての Environment での確かめ）の側にあり、この試験では偽にならない
- kind: technical
- 提案: Scenario を 2 つに分ける。「PR の組み立ての APK の署名の証明書は debug の鍵のもの」と、「PR の実行からは、配布用と開発版の Environment の値を読めない」（tasks 9.2 の確かめに印を置く。作れなかったときは 11.2 の人間の確認待ちに名前付きで置く）
- 処置: fixed specs/distribution/spec.md —— Scenario から「どの鍵の置き場も読まない」を外した。PR の実行から Environment が読めないのは GitHub の保証（premises P12）で、確認バッチの使い捨ての Environment の確かめ（tasks の「人間の確認待ち」）が確かさを 4 に上げる

## R12. D1 の「最後の印は日付でなく版番号で選ぶ」を、Scenario が区別できない
- 成果物: openspec/changes/st52-release-channels/specs/record-envelope/spec.md
- 根拠: delta `specs/record-envelope/spec.md:61`（「祖先にあるものの中で、版番号が最も大きいもの（同じ MAJOR.MINOR.PATCH ではベータ版より正式版が大きく、ベータ版どうしは N の大きい方）。印を打った日時では選ばない」）/ `:121-125`（Scenario は、`v0.3.1` が別の枝 `release/0.3` にあって main の祖先でない形。祖先に絞るだけで `v0.3.1` は外れるので、日付で選んでも版番号で選んでも同じ結果になる）/ design.md:29（`order` は `git` の `versionsort.suffix` に頼らない）。`v0.4.0-beta.10` と `v0.4.0-beta.9`、`v0.4.0` と `v0.4.0-beta.2` の並びを固める Scenario は無い
- kind: technical
- 提案: Scenario を足す:「どちらも祖先にある `v0.4.0-beta.9`（後に打った）と `v0.4.0-beta.10` では、最後の配った印は `v0.4.0-beta.10`」「同じ commit の `v0.4.0-beta.2` と `v0.4.0` では `v0.4.0`」。印は tasks 4.2（`check-contract-version.sh --self-test`）か 1.1（`version.sh order`）に置く
- 処置: fixed 4.2 —— record-envelope に Scenario「最後の配った印は、ベータ版の番号を数として比べて選ぶ」「同じ版のベータ版と正式版では、正式版を最後の配った印に選ぶ」を足し、4.2 の自己試験に割り当てた

## R13. 1 つの Scenario が 2 つの主張を束ね、別々のタスクの試験が半分ずつ持つ（片方だけ通っても緑になる）
- 成果物: openspec/changes/st52-release-channels/specs/distribution/spec.md
- 根拠: delta `specs/distribution/spec.md:98-101`「正式版の印で、3 部品が同じ版の通常のリリースになる」（リリースの種類＋4 つの配布物＋印の版番号）→ tasks.md:41（1.1 が判定の部分）と tasks.md:117（6.2 が配布物と版番号の部分）。`check_scenarios.py` は印が 1 つあれば担保ありと数える / `:234-237`「公開の設定が入っていなければ…」の THEN「印 `nightly` は動かない」は YAML の `if:`（design.md:121）の側で、`nightly-plan` の試験（tasks.md:43）では偽にならない / `:76-78`「桁を超える版番号では、携帯の組み立てが失敗する」は `version.sh code` の計算だけを試し（tasks.md:40）、gradle の `GradleException`（tasks.md:62）を試す Scenario が無い / `:287-290`「収集アプリの送り先の道は、今のまま受ける」の THEN「今と同じく受け付けられる」は状態符号が無い
- kind: technical
- 提案: 「正式版の印で…」を判定（種類と版番号）と配布物（4 つの名前・印の版番号）の 2 つに分ける。「印 `nightly` は動かない」は `check-guards.sh --self-test` が `nightly.yml` の印を動かす手順の `if:` を読む Scenario にする。「桁を超える」は「携帯の組み立て（gradle）に `0.100.0` を渡すと組み立てが落ちる」を足す。送り先の道は THEN に状態符号（今の `/heartbeat` が返す符号）を書く
- 処置: fixed specs/distribution/spec.md —— 「正式版の印で…」を判定と配布物の 2 本に、「公開の設定が…」を予定の実行・手で起こした実行・workflow の条件の 3 本に、「桁を超える」を計算と gradle の組み立ての 2 本に分け、それぞれ 1 つの Task に置いた

## R14. tasks の依存順が逆になっている（作る前の道具を検証が使う）
- 成果物: openspec/changes/st52-release-channels/tasks.md
- 根拠: tasks.md:123（7.1 の `--self-test` は「使い捨ての鍵 2 つ（`make-key.sh` で作る）」）→ `make-key.sh` は tasks.md:126 の 7.2 で作る / tasks.md:140-142（9.1 の検証が `check-guards.sh --self-test` と、`release.yml`・`nightly.yml` への actionlint）→ `check-guards.sh` は tasks.md:148 の 9.3、2 本の workflow は tasks.md:143 の 9.2 で作る / tasks.md:144・147（9.2 の検証も 9.3 の `check-guards.sh`）/ tasks.md:67（3.1 が 6.2 の `check-artifacts.sh apk-version` を使う。これは本文に断りがある）
- kind: technical
- 提案: 7.2 を 7.1 の前に、9.3 を 9.1 の前に置く。9.1 の actionlint は `ci.yml` だけにし、`release.yml`・`nightly.yml` の actionlint は 9.2 の検証に移す
- 処置: fixed tasks.md —— make-key.sh を 7.1・sign-apk.sh を 7.2 に、check-guards.sh を 9.1 に置き、actionlint はそのファイルを作った項目（9.2 は ci.yml、9.3 は release.yml・nightly.yml）に移した

## R15. 9.2 の検証の grep は、確かめをしなくても今のまま通る
- 成果物: openspec/changes/st52-release-channels/tasks.md
- 根拠: tasks.md:147（`grep -Eq "^\| P12 .*\| (4|2) \|$" …/premises.md`）/ premises.md の P12 の行は今「| 公式文書 | 2 |」で終わる。使い捨ての Environment で確かめても、作れずに飛ばしても、何もしなくても rc=0。design.md:158 の「読めたら鍵を置かず、Q2 の答えを人間に聞き直す」も、作れなかったときの 11.2 への回し（tasks.md:170）も、機械が強いない
- kind: technical
- 提案: 終了条件を 2 つに分ける:「P12 の確かさが 4」か「P12 の行に `未走行:` と理由があり、かつ tasks 11.2 に `st52-probe` の確認待ちの名前がある」。どちらでもなければ落ちる grep にする
- 処置: fixed tasks.md —— 下流の中の P12 の grep を外した。配布元での確かめは「人間の確認待ち」に移し、確認バッチが premises の確かさを書く

## R16. 10.1 と 9.2 の「本物の配布元で 1 回走らせる」は、この Story の PR の CI の実行を要るが、下流の実装の中では PR が無い。誰がいつ走らせるかが書かれていない
- 成果物: openspec/changes/st52-release-channels/tasks.md
- 根拠: tasks.md:157（10.1「この Story の PR の `package` の run を使う」）・tasks.md:159（検証は P16 の確かさが 4）/ tasks.md:145（9.2「PR の枝から読もうとする workflow を 1 回だけ走らせ」）/ AGENTS.md（「レビュー前の `feat/st*` の push は、どの経路でも `.githooks/` が止める」。executor は push / PR をしない）。PR の `package` の run が無いまま 10.1 の検証は満たせない
- kind: technical
- 提案: 10.1 の本物の配布元での実行と 9.2 の使い捨ての Environment での確かめを、「PR の CI が緑になった後に走らせる手順」として別の項目にする。人間の確認待ちか、PR の後のグラフの段のどちらで走らせるかを書く。下流の実装の中で閉じる検証は `--self-test` までにする
- 処置: fixed tasks.md —— 下流の検証は `--self-test` で閉じ、配布元での実行（下書き・使い捨ての Environment・ルールセットの読み）を「人間の確認待ち（確認バッチ。PR の CI が緑になった後）」の節にコマンドつきで置いた

## R17. 6.2 の `check-artifacts.sh --self-test` は本物の 4 つの配布物を要るが、6.1 の `--self-test` が Linux で組むのは 2 つだけ
- 成果物: openspec/changes/st52-release-channels/tasks.md
- 根拠: tasks.md:112（6.1「`--self-test` は Linux で `server-linux` と `apk` を組み」）/ tasks.md:115（6.2「本物の組み立てでできた配布物（Task 6.1 の出力と…）に当てる」）と、そこで担保する Scenario「綺麗な release で資格情報の無い配布物は、検査を通る」（4 つがそろう前提。delta :156-159）・「正式版の印で…」（4 つ）。Windows 向けのサーバと PC の収集アプリの zip を Linux の自己試験でどう用意するかが無い（`tools/check-build-stamp.sh:5` の C-02 は rlib を読む形で、zip ではない）
- kind: technical
- 提案: 6.2 に、Windows の 2 つの配布物の作り方を書く（例: Linux で組んだ C-02 の rlib と Windows 向けの組み立ての印の文字列を D17 の名前の zip に詰めた写し。どちらにしたかを書く）。本物の 4 つでの通しは 9.1 の `package` の job（CI）の結果で見る、と分けて書く
- 処置: fixed 6.2 —— 自己試験は Linux で本物に組んだ 2 つと、Windows の 2 つの写し（C-02 の rlib の印の文字列を D17 の名前の zip に詰めたもの）でそろえる、と書いた。本物の 4 つの通しは 9.2 の package の job が見る

## R18. 5.3 の e2e の検証のコマンドは、合言葉の読み込みと縦串の起動が無く、そのままでは走らない
- 成果物: openspec/changes/st52-release-channels/tasks.md
- 根拠: tasks.md:107（`bash -c 'cd web && npm run test:e2e -- st52-served-ui.spec.ts'`）/ `web/e2e/global-setup.ts:16`（`WEB_PASSWORD` が無ければ throw）/ docs/testing.md §4.5（起動は `tools/stack.sh up`）/ `web/playwright.config.ts:19,31`（`baseURL` は `WEB_PORT` だけ。新しい spec は `$BIND` を自分で組み立てる必要がある）/ 前例 `openspec/changes/archive/2026-10-31-ch08-erasure-nested-and-late-stays/tasks.md:125`（`set -a; [ -f .env ] && . ./.env; set +a;` を付けている）。また、縦串が先に（画面を取り込まない形で）立っていると `reuseExistingServer` でそれを使い、`$BIND` の `/` が 404 になる
- kind: technical
- 提案: 検証を `bash -c 'set -a; [ -f .env ] && . ./.env; set +a; STACK_RESET=1 tools/stack.sh up && cd web && npm run test:e2e -- st52-served-ui.spec.ts'` の形にする（既存の e2e の行も同じ）。spec の本文に「`BIND` の環境変数から絶対 URL を組む」を書く
- 処置: fixed 5.3 —— 検証を `.env` の読み込みと `STACK_RESET=1` つきの形にし、e2e は環境変数 `BIND` から絶対 URL を組むと書いた

## R19. `/api` の道で受けた画面の読み出しが、読み出しの記録に今と同じ道で残ることを、Scenario も検証も見ていない
- 成果物: openspec/changes/st52-release-channels/specs/distribution/spec.md
- 根拠: design.md:107（D13「認可・読み出しの記録・`net_guard` は剥がした後の道で今と同じに当たる」）/ delta `specs/distribution/spec.md:264`（「同じ認可・同じ応答」だけ。読み出しの記録に触れない）/ tasks.md:101（`cargo test -p ashiato-server access_log` は既存の試験で、どれも `/api` の前置き無しの道を叩く）/ `crates/server/src/lib.rs:4256-4259`（読み出しの記録は `route_layer` で `MatchedPath` を使う。剥がす層の置き方しだいで、記録される道が `/api/…` になる・記録されない）。配る形では画面の読み出しは全部 `/api/…` から入る
- kind: technical
- 提案: Scenario を足す:「画面を組み込んだサーバで、ログインした画面の資格情報で `/api/events` を読むと、読み出しの記録に `/events` を読んだときと同じ 1 行が残る」。印は `st52_web_tests.rs` に置く
- 処置: fixed 5.1 —— Scenario「/api の道で読んだ画面の読み出しも、今と同じ読み出しの記録が残る」を足し、`st52_web_access_log` に割り当てた（下限 5）

## R20. 名前と数の食い違い（proposal の ADDED の数・道具の名前・Requirement の題の部品の数）
- 成果物: openspec/changes/st52-release-channels/proposal.md
- 根拠: proposal.md:54（requires の説明が ST48・ST49・ST51 だけで、MODIFIED する ST50 の Requirement（archive 済み）を挙げない）/ proposal.md:45（「ADDED 10」。数えると delta の `## ADDED Requirements` の下は 11 で、「手で起こした確かめの組み立ては、下書きのリリースにだけ置き…」が列挙から抜けている）/ proposal.md:62（`tools/release/` の一覧に `sign-apk.sh` が無い）・tasks.md:24（統一する名前の一覧に、tasks.md:141 で初めて出る `tools/release/package.sh` が無い）/ delta `specs/distribution/spec.md:3`（題「3 つの部品の版番号」）と `:17-20`・`:22-26`（Scenario は「4 つの部品」。画面を数える）
- kind: technical
- 提案: proposal の数と列挙を 11 に直し、requires の説明に ST50 を足す。道具の名前の一覧に `sign-apk.sh`・`package.sh` を足す。題を「3 つの部品と画面の版番号は…」にするか、本文で「4 つ（サーバ・画面・携帯・PC）」と数え方を書く

---

## 観点ごとの確かめた範囲

- **観点 1（deep の決定が写っているか）**: Q1〜Q11 の推奨と C1〜C9・D1〜D6 を 1 件ずつ spec / design / tasks と突き合わせた。数値（versionCode の式と 30002・30099・30199・1000099・40000、ベータの N は 1〜98、MINOR・PATCH は 99 まで、cron `0 15 * * *`、RSA 4096・25 年以上（`-validity 10000` は約 27.4 年）、日付 `YYYYMMDD` の日本時間）はどれも Scenario にある。食い違いは R1（ST48 の古い Scenario が残る）・R4（C6 と FR-107）・R6（扉 34 の改訂の台帳・Q11 の控え）。C9 の「打ったら消す」は Requirement の本文（delta :345）にあり、手順書の検証（tasks.md:166 の `修正の枝`）で見る
- **観点 2（検証可能か）**: R2・R11・R12・R13・R19
- **観点 3（置き場）**: R8。`proposal.md` の Capabilities（distribution・record-envelope）と `specs/` のディレクトリは一致。INDEX.md の capability 表は distribution（105 行）にも record-envelope（91 行。訂正の節に理由あり）にも ST52 がある。spec の中の実装の名前は、正典から写した `crates/server`（R1）のほかに `BASE_URL`・`API_TOKEN`・`USER_ID`（delta :135・:154）があるが、検査の出力に出す項目の名前として観測の言葉になっているので指摘にしない
- **観点 4（tasks）**: (c) 新しい 60 本の Scenario は、`grep -cF` で 1 本ずつ数えて全部が tasks.md のどこかに名指されている（0 件は写した MODIFIED の 23 本だけで、ST48 の 7・ST49 の 6・ST50 の 10。tasks.md:15-17 の数と一致）。cargo の件数つきの検証は `-- --list` の下限を持つ（2.1 ≥3・2.2 ≥1・5.1 ≥4・5.2 ≥1）。gradle の 3.1 は表の行の数（≥8）を見る。`$TMPDIR` は tasks.md:9 の共通の規則と 7.1。存在を確かめた: `ashiato-build-stamp`・`crates/server/tests/server_startup.rs`・`tools/check-openapi.sh`・`tools/check-build-stamp.sh` の節 `android`。`web/src/__tests__/build-stamp.test.ts` は今無い（3.2 で作る）。`actionlint` は手元に無く docker の代わりに頼る。指摘は R14〜R18
- **観点 5（Story と要件）**: `check_chain.py` は OK（観点 8 の再生成との一致を含む）。satisfies（FR-104・FR-105・FR-107・NFR-25）の外の要件を満たそうとする Requirement は無い（扉 34 は doors にある）。完了の判定と価値の食い違いは R7
- **MODIFIED の写し**: distribution「配った移行は、名前も中身も変えられない」は正典（作業木 :192-234、origin/main も同じ本文）の本文 4 文と Scenario 7 本を落とさず写し、本文 1 文と Scenario 2 本を足した。record-envelope の 2 つ（正典 :881-920・:1266-1328。origin/main と同じ）も本文と Scenario 6 本・10 本を落とさず写し、印の版番号を「宣言した」から「1 つの出所」に替えた文と Scenario 2 本ずつを足した
- **観点 6（答えが写ったか）**: 該当なし（mode は spec --draft。`--answers` ではないので `review/answers.md` は書かない）

## 観点 7: 該当なし（spec_overlap.py candidates が 0 件）

作業木の枝は origin/main より古い（ST51 の archive #221 が入っていない）が、`diff <(git show origin/main:openspec/specs/distribution/spec.md) openspec/specs/distribution/spec.md` の差は ST51 が足した 3 つの Requirement（origin/main の 284 行目以降）だけで、この change が MODIFIED する Requirement の外。origin/main の `openspec/changes/` に archive 以外の change は無い。
- 処置: fixed proposal.md —— ADDED を 11 に、MODIFIED の列挙に版の記録を足し、requires の説明に ST50 を足し、道具の一覧に `sign-apk.sh`・`package.sh`・`version-table.tsv` を足した。Requirement の本文に「部品」は 4 つの組み立て（サーバ・画面・携帯・PC）だと書いた


---

# spec --answers Q1,Q4,Q11 の段のレビュー（2026-11-01）

機械の検査（作業木。HEAD 3881e93c からの未 commit の差分を含む）:
- `openspec validate st52-release-channels --strict`: valid
- `check_chain.py .`: OK（0 件 / 未回収 0 / warn 0）
- `check_scenarios.py . st52-release-channels`: FAIL（担保なし 74 件。tasks.md:15 の「ST52 が新しく印を置くのは 74 本」と同じ数で、実装の前の既定）
- `review_triage.py . st52-release-channels`: OK（指摘 36 / 仮決め 7 / 要件へ戻すもの 1）
- `spec_overlap.py candidates . st52-release-channels`: `[]`（0 件）

## R21. Q1・Q4 の推奨の文が 3 か所に残る（名前の統一の行・台帳の D22・作り直しの Requirement の部品の数）
- 成果物: openspec/changes/st52-release-channels/tasks.md
- 根拠: tasks.md:24「`ASHIATO_APK_UNSIGNED=1`（携帯の APK を署名せずに組む。`release.yml`・`nightly.yml` の組み立ての job だけ）」—— 同じ差分で design.md:143（D18）と tasks.md:158（9.3）は「`release.yml` の組み立ての job だけ」「`nightly.yml` は ST54 まで携帯の APK を組まない」に直している。
  provisional.jsonl:8 の D22 の `question`「Q5・Q2・Q1 の推奨のもとで…」—— design.md:171 の D22 の本文は「Q5・Q2 の答えのもとでは…（本人の携帯には Q1 の答えのとおり収集アプリがまだ入っていない）」に直したが、台帳（確認バッチで人間が見る）は推奨の前提のまま。
  specs/distribution/spec.md:245「違えば main の HEAD から 3 部品を組んで開発版を作り直す（ST54 までは携帯の収集アプリを組まず、サーバと PC の収集アプリだけ）」—— 「3 部品」（サーバ・携帯・PC。spec.md:3・:359 の数え方）を組むと言った同じ文で携帯を組まないと言う
- kind: technical
- 提案: tasks.md:24 を「`release.yml` の組み立ての job だけ（`nightly.yml` は ST54 まで携帯の APK を組まない）」に、台帳 D22 の `question` を design の D22 と同じ前提（Q5・Q2 の答えと Q1 の答え）に、spec.md:245 を「main の HEAD から開発版の部品（ST54 まではサーバと PC の収集アプリの 2 部品）を組んで作り直す」に直す
- 処置: fixed tasks.md —— tasks.md の名前の統一の行を `release.yml` だけに、台帳 D22 の `question` を Q5・Q2 の答えと Q1 の答えの前提に、spec の作り直しの Requirement を「開発版の部品（ST54 まではサーバと PC の収集アプリの 2 部品）」に直した

## R22. 新しい Scenario「開発版のリリースの配布物は、ST54 まで携帯の APK を除く 3 つ」が 2 つの主張を束ね、足す「携帯の APK」が何かを決めていない
- 成果物: openspec/changes/st52-release-channels/specs/distribution/spec.md
- 根拠: spec.md:131-134 の THEN「検査の終了コードは 0 で、同じ置き場に携帯の APK を足して走らせると、検査は 0 でない終了コードで終わり、その APK の名前が示される」—— 通す側（3 つで rc=0）と断る側（APK があれば rc≠0）の 2 つ（R13 と同じ型。片方だけ作っても印 1 つで緑になる）。
  足す APK の名前が無い: 開発版の名前 `ashiato-collector-dev-<版番号>.apk`（spec.md:112）だけを断る実装でも、正式の名前 `ashiato-collector-<版番号>.apk` や名前の違う `*.apk` が置き場にあれば通りうる。spec.md:114 は「携帯の APK が置き場にあれば公開しない」で、名前を問わない
- kind: technical
- 提案: 「開発版の配布物の検査は、携帯の APK を除く 3 つで通る」と「開発版の置き場に携帯の APK があれば、検査は落ちてその名前を示す」に分け、後者の GIVEN に `ashiato-collector-dev-0.4.0-dev.20261102.apk` と `ashiato-collector-0.4.0-dev.20261102.apk` のどちらでも落ちることを書く（tasks 6.2 の名指しも 2 本に）
- 処置: fixed specs/distribution/spec.md —— Scenario を「開発版の配布物の検査は、携帯の APK を除く 3 つで通る」と「開発版の置き場に携帯の APK があれば、検査は落ちてその名前を示す」に分け、後者に開発版の名前と正式の名前の 2 つの APK を書いた。tasks 6.2 の名指しも 2 本にした

## R23. 「開発版は ST54 まで携帯の APK を組まない」は Requirement の本文と design だけにあり、Scenario も検査も無い
- 成果物: openspec/changes/st52-release-channels/specs/distribution/spec.md
- 根拠: spec.md:245（「ST54 までは携帯の収集アプリを組まず」）・design.md:119（D14）・tasks.md:158（9.3）。Scenario は spec.md:131 の配布物の検査（公開の前に置き場の APK を断る）だけで、`nightly.yml` が APK の job を持たないことは見ない。
  tasks.md:150 の `check-guards.sh workflows` が `nightly.yml` から見るのは cron と `publish` の `if:` と書く権限の置き場だけ、9.3 の自己試験に足す Scenario も「0:00 に起きる」「公開の設定が無ければ印を動かさない」だけ。
  `nightly.yml` に APK の job があっても公開の前に落ちるので配布物には出ないが、そのときは開発版が毎回公開されなくなり（C7 で前の版のまま）、組む Actions の分も使う
- kind: technical
- 提案: Scenario「開発版の作り直しの workflow は、ST54 まで携帯の APK を組む job を持たない」を足し、`check-guards.sh workflows` と 9.3 の `--self-test` に割り当てる（APK の組み立て・`sign-apk.sh` の呼び出しが `nightly.yml` にあれば rc≠0）
- 処置: fixed 9.3 —— Scenario「開発版の作り直しの workflow は、ST54 まで携帯の APK を組む手順を持たない」を足し、`check-guards.sh workflows` と 9.3 の `--self-test` に割り当てた

## R24. `nightly` の公開の手順は `--allow-debug-key` なしで `check-artifacts.sh --channel nightly` を呼ぶが、指紋の期待値を `nightly` に置かない —— APK の無い置き場で指紋と資格情報の検査をどう扱うかが書かれていない
- 成果物: openspec/changes/st52-release-channels/tasks.md
- 根拠: tasks.md:159「`nightly.yml` の `publish` は `check-artifacts.sh --channel nightly` → D15 の入れ替え（署名しない）」/ tasks.md:123（6.2）「（`--allow-debug-key` でなければ）署名の指紋が `ASHIATO_EXPECT_CERT_SHA256` と同じ」・「APK の `BASE_URL`・`API_TOKEN`・`USER_ID` が空」/ design.md:146（D18）「`nightly` の変数は開発版の鍵と一緒に ST54 が置く」。
  `nightly` の実行では `ASHIATO_EXPECT_CERT_SHA256` が空で、APK も無い。6.2 の自己試験（tasks.md:126）は `--channel nightly` を当てるが、期待値が空のときの振る舞いを決めていないので、「期待値が空なら落とす」と作れば開発版の公開が毎回落ち、「APK が無ければ指紋を見ない」と作るかは実装の判断になる
- kind: technical
- 提案: D7 と 6.2 に「`--channel nightly` は APK を期待しないので、指紋と `BuildConfig` の検査を当てない（`ASHIATO_EXPECT_CERT_SHA256` が空でよい）。`--channel release|beta` で期待値が空なら落とす」を書き、6.2 の自己試験に期待値を空にした `--channel nightly` の rc=0 を足す
- 処置: fixed D23 —— D7 と 6.2 に「`--channel nightly` は指紋と `BuildConfig` を見ず期待値は空でよい」を書き（release・beta の言い直しは R27 で消した）、6.2 の自己試験は期待値を空にした `--channel nightly` で rc=0 を見る

## R25. tasks 11.1 は「`docs/handoff/ST54.md` に D23 の道具を 1 行足す」と書くが、この段で既に足してある。11.1 の検証の grep は今のまま通る
- 成果物: openspec/changes/st52-release-channels/tasks.md
- 根拠: tasks.md:174「`docs/handoff/ST54.md`（…D23（仮）で ST52 が用意した道具を 1 行足す）」/ 作業木の docs/handoff/ST54.md:16-17（「ST52 が用意するもの（ST52 の design D23（仮））…」「ST54 がやるもの…」。この差分で足した）。
  tasks.md:175 の検証 `grep -q "ST52" docs/handoff/ST53.md && grep -q "ST52" docs/handoff/ST54.md && grep -q "ST52" docs/handoff/ST58.md` は、ST53 が 5 件・ST54 が 4 件（HEAD でも 3 件）で既に当たり、効くのは ST58（0 件）だけ（R15 と同じ型）
- kind: technical
- 提案: 11.1 の ST54 の分を「開発版の DB の扱い（R11）の節を足す。開発版の携帯の APK と鍵の節（Q4・Q11・D23）は上流が書いた。実装が道具の名前を変えたら直す」にし、検証を節の見出しか固有の語（例 `grep -q "R11" docs/handoff/ST54.md`・`grep -q "D5" docs/handoff/ST58.md`）で見る
- 処置: fixed 11.1 —— ST54 の分を「開発版の DB の扱いの節を足す（携帯の APK と鍵の節は上流が書いた）」にし、検証を `ASHIATO_NIGHTLY_PUBLISH`（ST53）・`R11`（ST54）・`D5`（ST58）の固有の語に替えた（どれも今は当たらない）

## R26. ST54 への申し送りが「検査の期待を 4 つに戻す」だけで、正典に残る時限の例外（Requirement の本文 4 文と Scenario 1 本）を ST54 が書き直すことを名指さない
- 成果物: docs/handoff/ST54.md
- 根拠: docs/handoff/ST54.md:17「`tools/release/check-artifacts.sh --channel nightly` の期待を 4 つに戻す（FR-104 ★ 2026-11-01 の例外を外す）」。
  archive で `openspec/specs/distribution/spec.md` に入るのは spec.md:111（「ただし開発版のリリースには、ST54 まで…並べず」）・:112（「開発版に並べるのは ST54 から」）・:114（「開発版は ST54 まで…携帯の APK が置き場にあれば公開しない」）・:245（「ST54 までは携帯の収集アプリを組まず」）と Scenario「開発版のリリースの配布物は、ST54 まで携帯の APK を除く 3 つ」。
  ST54 が台本の期待だけを 4 つに戻すと、この Scenario の印の試験（tasks 6.2）が落ちるか、Scenario を残したまま試験を外すことになる。「ST54 まで」は Story の番号で、正典の文が自分で期限切れを知る手段は無い
- kind: technical
- 提案: 申し送りの「ST54 がやるもの」に「distribution の『印（タグ）の名前で、3 つのチャンネルに分けて配る』『開発版は、幹にその日の変更があった日だけ作り直す』を MODIFIED で書き直し、上の 4 文の『ST54 まで』の例外と Scenario を外す（開発版も 4 つ）」を足す
- 処置: fixed D23 —— D23（仮）の申し送り先 `docs/handoff/ST54.md` の「ST54 がやるもの」に、正典 distribution の 2 つの Requirement を MODIFIED で書き直して「ST54 まで」の例外の文と Scenario 3 本を外すことを足した

## 観点 6（spec --answers Q1,Q4,Q11）の確かめた範囲

- 推奨で書いた版（HEAD 3881e93c）からの差分（`git diff`。7 ファイル、+60 / -41）を全部読んだ。結果は `review/answers.md`
- Q1: 写った（design.md:20・:171・:173、proposal.md:35、tasks.md:19、premises.md:28）。推奨の残りは台帳 D22 の `question`（R21）
- Q4: 写った（spec.md:111・:112・:114・:245・Scenario :131、design D7・D10・D14・D17・D18・D19・D23、tasks 6.2・9.3・11.1、proposal.md:18・:20、ST54 の申し送り）。推奨の残りは tasks.md:24 と spec.md:245 の「3 部品」（R21）
- Q11: 写った（spec.md:197「正式な鍵と別の開発版の鍵」、design.md:151「控えは USB に 1 部」と「ST54 が `make-key.sh dev` で作って `nightly` に置く」、tasks 11.1、proposal.md:25、tasks.md:21）。呼び出し元の注意どおり、答えのファイルの「補足: st53で」は IMP-01 のもので、Q11 の写しとして探していない
- 整合の確認 `docs/audit/2026-11-01-st52-door34-consistency.md` の C6 が名指した箇所を 1 つずつ当てた: 入れ直し（proposal:35・design:20・tasks:19・premises:28）は全部直った。Q4 の箇所のうち、design:146（今の :147）・spec:109/:112（今の :111/:114）・:192（今の :197）は直った。`applicationIdSuffix`（tasks.md:69）・Scenario「開発版の APK は、.dev の識別子と開発版の鍵で署名されている」・取り違えの Scenario・`sign-apk.sh` の開発版の自己試験（tasks.md:137-138）・D3 の開発版の鍵・spec の台本が開発版の鍵も作る文・開発版の Environment の検査は、D23（仮）で ST52 に残した（使い捨ての鍵で確かめる道具）。C6 が「一緒に決めるもの」とした nightly の数え方は spec.md:114 と新しい Scenario、開発版の Environment と鍵の検査の渡し方は D23 と ST54 の申し送りで決めた。C6 が挙げていない残りは R21 の tasks.md:24
- 答え以外の変更: D23（仮）の新設とそれに伴う変更（`--channel` の口・Scenario 1 本・数 104→105 と 73→74・D7/D17/D18/D19・tasks 6.2/9.3/11.1・ST54 の申し送り 2 行・台帳 D23）と、spec.md:198 に足した「正式な鍵は開発版の作り直しの実行にも渡さない」。後者は D21（仮）の本文（design.md:160-164「正式な鍵を使うのは `release.yml` の `publish` だけで、main の上の実行には正式な鍵の要る job が無い」「`nightly` は…枝 `main` に開く」）の帰結として書かれていると読み、D21 に当てた。どちらも仮
- 振る舞いを変えない出所の注記の更新（spec の導出元の「Q2・Q11（推奨）」→「本人の答え。推奨と同じ」、「Q2 の推奨」→「Q2。扉 34 ★ 2026-11-01」、proposal.md:65 の扉 34 と FR-104 を「改訂済み」に、design の Open Questions・tasks.md:23 の注記）は、第 2 回の答えの記録と record の段の済みを写したもので、決定の変更には数えていない

## R27. R24 の直しで足した「`release`・`beta` で指紋の期待値が空なら落とす」は、design（仮でない D7）と tasks の説明にしか無く、Scenario も自己試験の割り当ても無い。D23（仮）の中身でもない
- 成果物: openspec/changes/st52-release-channels/design.md
- 根拠: design.md の D7（「`release`・`beta` で `--allow-debug-key` でなく期待値が空なら落とす。Q4・D23（仮）」）/ tasks.md 6.2 の説明（「`release`・`beta` で期待値が空なら落とす」）。6.2 の `--self-test` が名指す Scenario に、期待値が空の `release`・`beta` を断るものは無い（名指すのは nightly の 2 本と既存の 6 本）。
  `grep -n "期待値" specs/distribution/spec.md` は :132（nightly の Scenario の GIVEN「署名の指紋の期待値は空」）だけで、Requirement にも Scenario にも release/beta の側は無い（観測できる振る舞いが design にだけある。R8 と同じ型。archive で正典から落ちる）。
  D23（仮）の節（design.md の D23）は nightly の検査と道具の範囲だけを言い、release/beta の指紋の検査に触れない。D7 の括弧の末尾の「D23（仮）」は nightly の部分の出所としては当たるが、release/beta の部分は D23 からは出てこない（強いて言えば D18 の「署名した後に指紋を確かめてから公開する」の帰結で、D18 も仮でない）
- kind: technical
- 提案: 「印の配布物の検査は、指紋の期待値が空なら落ちる」の Scenario を spec（「携帯の APK は、チャンネルごとに決めた識別子と鍵で署名する」か「配る組み立ては綺麗で…」の Requirement）に足し、6.2 の自己試験に割り当てる。合意を引き継がせたいなら、release/beta の側を D23 の節に書き足すのではなく（D23 の中身ではない）、仮の決定として台帳に別に立てる（例 D24（仮）。反転は期待値の検査を外せば戻る）か、本人の確認に回す
- 処置: fixed D23 —— D7 と 6.2 から「`release`・`beta` で期待値が空なら落とす」の文を消した。今までの D7（`--allow-debug-key` でなければ指紋が期待値と同じ）で、空の期待値は一致しないので落ちる —— 新しい振る舞いではなく、足した文は言い直しだった。残るのは `--channel nightly` で指紋と `BuildConfig` を見ない部分だけで、D23（仮）の中身
