# 整合の確認 — 版番号を merge ごとの通し番号にし、正式版・ベータ版・開発版を札で分けた（依頼 #352。2026-11-21）

- 基準: `6642ee7558b70796b8e13922bf72e428ec0dfa7a`..`662224664f05ea2cee9376de4c6db11b0f3fc2c1`

## 機械の入力（写し）

`python3 scripts/req_impact.py . --base 6642ee7558b70796b8e13922bf72e428ec0dfa7a`（改訂した要件 13 件・扉 0 項）:

- FR-104: `docs/handoff/ST54.md:17,20`・`docs/production-prep.md:48`・`docs/stories/INDEX.md:76,332,379`・`docs/stories/stories.json:1231`・`openspec/specs/distribution/spec.md:26,544,611,640,729,755,875`・`openspec/specs/record-envelope/spec.md:896`
- FR-105: `docs/stories/INDEX.md:76`・`docs/stories/stories.json:1232`・`openspec/specs/distribution/spec.md:201,611,729,819,838,875`・`openspec/specs/record-envelope/spec.md:1268,1302`
- FR-107: `docs/stories/INDEX.md:76`・`docs/stories/stories.json:1233`・`openspec/specs/distribution/spec.md:948`
- FR-112: `docs/handoff/ST55.md:21`・`docs/handoff/ST57.md:13,19,21,25`・`docs/stories/INDEX.md:81,529`・`docs/stories/stories.json:1365`・`openspec/specs/distribution/spec.md:1459,2075,2494,2592,2684,2865,2901,3328`
- FR-113: `docs/handoff/ST57.md:9,16`・`docs/handoff/ST60.md:9,11`・`docs/stories/INDEX.md:81`・`docs/stories/stories.json:1366`・`docs/ui-direction.md:23`・`openspec/specs/distribution/spec.md:2901,2986,3190,3220,3243`
- FR-124: `docs/stories/INDEX.md:74,491`・`docs/stories/stories.json:1180`・`openspec/specs/data-durability/spec.md:939`・`openspec/specs/distribution/spec.md:819`・`openspec/specs/record-envelope/spec.md:1067,1209,1266`
- FR-127: `docs/changes/CH17.md:19`・`docs/handoff/ST53.md:18`・`docs/handoff/ST55.md:18`・`docs/handoff/ST60.md:6`・`docs/stories/INDEX.md:77,374`・`docs/stories/stories.json:1260`・`openspec/changes/ch24-zip-upgrade/{base/distribution.md:14,deep.md:26,review/spec.md:310,specs/distribution/spec.md:14}`・`openspec/specs/distribution/spec.md:2076,2119,2157,2194,2285,3349`
- FR-130: `docs/handoff/ST50.md:3,6`・`docs/handoff/ST51.md:24,27`・`docs/stories/INDEX.md:74,332,379,491`・`docs/stories/stories.json:1183`・`openspec/specs/record-envelope/spec.md:1167,1209,1212,1266,1302`
- FR-134〜FR-137（新規）: 要件の外で名指ししている箇所は無い
- NFR-25: `docs/stories/INDEX.md:76,332,379`・`docs/stories/stories.json:1234`・`openspec/specs/distribution/spec.md:544`

`python3 scripts/check_chain.py .`（chain: FAIL 5 件 / 未回収 4 件 / warn 0 件）:

- [FAIL] どの Story にも拾われていない要件 4 件: FR-134, FR-135, FR-136, FR-137
- [FAIL] ST52: FR-105 の未決が運ばれていない —— [NEEDS CLARIFICATION: 急ぎの直しの出し方 …]
- [FAIL] 整合の確認: 整合の確認の後に要件が変わった（この監査を名指ししない改訂）: FR-104 … FR-137 …（この監査がそれに当たる）
- [FAIL] ST50 / ST52 / ST57: stories.json + requirements.md から再生成した結果と一致しない
- warn 10（判断より新しい訂正）は 0 件。改訂した ID を拾う Story（ST50・ST52・ST53・ST57）はどれも archive 済み（`openspec/changes/archive/2026-10-31-st50-…`・`2026-11-02-st52-…`・`2026-11-09-st53-…`・`2026-11-14-st57-…`）

走っている change（archive 以外）は `ch22-soak-install-online-ci`・`ch24-zip-upgrade`・`ch25-ci-flake-gradle-cache-clock-test` の 3 本。C14 で印を付けるだけ。
どれも失われるもの（uncaptured / discarded / exported / rewrite-all）は無い。

ID を名指ししないが旧い形（印 `v*`・`-beta.N`・印 `nightly`・`Cargo.toml` の版が出所・版を手で上げる）を前提にしている文書とコードは、
`docs/release.md`・`docs/production-prep.md`・`docs/collector-contract.md`・`docs/testing.md`・正典 `distribution` / `record-envelope` / `dependency-vulnerability-scan`・
`tools/release/version.sh`・`tools/check-released-migrations.sh`・`tools/check-contract-version.sh`・`tools/check-nightly-migrations.sh`・`.github/workflows/{ci,release,nightly}.yml`・
`crates/update-check`・`crates/build-stamp`・`crates/installer`・C-01 の `UpdateCheck.kt` / `UpdateInstall.kt` / `build.gradle.kts` を読んだ。

## 1. 改訂後の要件と、ほかの文書

## C1. 改訂した 9 件（FR-104・105・107・112・113・124・127・130・NFR-25）を拾う Story は全部凍結済みで、新しい 4 件（FR-134〜137）はどこにも拾われず、5 つの NEEDS CLARIFICATION の期限「この改訂を受ける Story」に当たる Story が無い
- 重大度: 高
- 文書: docs/stories/stories.json:1229
- 要件: FR-104, FR-105, FR-107, FR-112, FR-113, FR-124, FR-127, FR-130, NFR-25, FR-134, FR-135, FR-136, FR-137
- 根拠: requirements.md:983「期限: この改訂を受ける Story の深掘り」（FR-105）・:1176（FR-134）・:1180（FR-135）・:1185「期限: この改訂を受ける Story の design」（FR-136）。
  拾う側: ST52（FR-104/105/107/NFR-25）・ST50（FR-124/130）・ST53（FR-127）・ST57（FR-112/113）はどれも `openspec/changes/archive/` にある。
  check_chain は「拾われた要件 172/178」に改訂した 9 件を数え、未回収には FR-134〜137 だけを出す。「ST52: FR-105 の未決が運ばれていない」は、凍結した ST52 へ運べと読める。
  新しい Story を FR-134〜137 だけで立てると、FR-112（追う札）・FR-124/130（札を付けた番号での凍結）・FR-127（正式版・ベータ版も commit）の改訂がどの生きた作業にも届かない
- 直すところ: 新しい Story（satisfies に FR-134〜137 と、改訂した 9 件のうち振る舞いの変わるもの —— 少なくとも FR-104・105・107・112・113・124・127・130・NFR-25 —— を入れる。未決 4 件の期限をその Story に向ける）
- 機械: FR-134〜137 は捕まえられた（check_chain の未回収）。改訂した ID の拾い手が全部 archive 済みであることは捕まえられない。安く捕まえられる（check_chain: 改訂の差分にある ID の satisfies が archive 済みの Story だけなら warn）
- 行き先: stories（/stories で FR-134〜137 と改訂した 9 件を拾う新しい Story を立て、未決 4 件の期限をその Story に向ける。archive 済みの ST50・52・53・57 には運ばない）

## C2. 「配った版」の凍結（移行・送信契約の版）は印 `v[0-9]*` と印 `nightly` だけを数える。改訂は「札を一度でも付けた番号」に変えたが、札を動かした後もその履歴が消えずに残ることも、検査がそれを読むことも、どこにも書かれていない
- 重大度: 高
- 文書: openspec/specs/distribution/spec.md:195
- 要件: FR-124, FR-129, FR-130, FR-105, FR-127
- 根拠: requirements.md:1112（FR-124 ★ 2026-11-21）「**「正式版として出した」は、正式版かベータ版の札を一度でも付けた番号を指す**（札を後で動かしても、一度付けた番号の契約の版は受け付け続ける）」・:1153（FR-130）「版 1 は、最初に正式版かベータ版の札を付けた番号で閉じる」。
  一方で :980「札を配布元でどう表すか（リリースの種類・印など）は、この改訂を受ける Story の design が決める」・:1135「番号ごとに印を打つか…は…design が決める」—— 印を打たない design を許している。
  正典: distribution spec.md:197「検査する配った印を、印 `v[0-9]*` のうち今の木の HEAD の祖先にあるものに限る」・:198「数える配った印が 1 つも無い THE SYSTEM SHALL 何も凍結せず、検査を通す」・:820「印を消すと、配った移行の凍結と送信契約の版の比べが、その版について黙って外れる」（だから :815 で印 `v*` を動かせなくした —— 札は動かすものになった）。
  record-envelope spec.md:1261「配った印（タグ `v*`。ベータ版の印を含む）で出した送信契約の版の要求と 1 件を、以後のすべての S-01 で受け付ける」・:1294。開発版の側も distribution spec.md:1336 の Requirement が印 `nightly` を数える。
  文書: `docs/collector-contract.md:175`「最初の配った印 `v*` までは上げない」・:210「「最初の印」は、その版を載せて最初に配った `v*` のタグ」・`docs/testing.md:218,228`。
  申し送り: `docs/handoff/ST52.md:7`「ST52 がリリースのタグを別の型にするなら、この台本の型を一緒に変える」・:21 —— 宛先の ST52 は凍結済みで、型を変えるのは次の Story。
  札を印 `v*` でない形（リリースの種類だけ・動く印）で表す design になると、検査は「印 0 個」で通り続け、配った移行の書き換え（FR-129 の前提が崩れ、本人の D-01 が「知らない版」で止まる・形がずれる）と、古い収集アプリの未送信の断り（FR-124）を CI が止めなくなる
- 直すところ: requirements（FR-124 か FR-105 に「正式版・ベータ版の札を付けた番号の記録は、札を動かしても消えず、配った移行と送信契約の版の検査がそれを数える」を書く。開発版の札の側も同じ）と新しい Story（正典の 3 つの Requirement と `docs/collector-contract.md`・`docs/testing.md` を直す）
- 機械: できない（今の自己試験は印の型の中でしか試さない）。新しい Story の試験で「札を付けて動かした後も、その番号の移行を変えると落ちる」を固定すれば捕まえられる
- 行き先: requirements（FR-124 に ★ 2026-11-21 補足: 札を付けた番号の記録は消さず、移行の凍結と契約の版の検査がそれを数える。開発版の札も同じ）。正典 3 つの Requirement と `docs/collector-contract.md`・`docs/testing.md` は新しい Story（stories）

## C3. 印 `v*` の push に付けた守り（本人の承認・正式な鍵の置き場・脆弱性の関門・印のルールセット）が、組み直さずに札を動かす操作（FR-135）へ移ると書かれていない
- 重大度: 高
- 文書: openspec/specs/distribution/spec.md:2380
- 要件: FR-135, FR-127, FR-104
- 根拠: requirements.md:1177「WHEN 本人が正式版かベータ版の札を番号へ動かす操作をする THE SYSTEM SHALL その番号の組み立てを組み直さずに、その札の配布物として配布元に置く」・:1179「携帯の APK の正式な鍵の署名（扉 34）は、札を付けるときに足してもよい」。
  守りの側: distribution spec.md:667「正式な鍵を、印 `v*` の push の実行にだけ開いた置き場（配布用の Environment）から…読み」・:2382「承認されるまで、その Environment を使う公開の job は始まらず、正式な鍵も渡らない」・:2389「正式版とベータ版の公開の job で、署名の前に `approvals` を通し」。
  dependency-vulnerability-scan spec.md:291「WHEN 印 `v*` が push され、配る物を組む workflow（`release.yml`）が走るとき、THE SYSTEM SHALL 組み立ての job より前に…全体の検査…を走らせ、rc=0 でなければ組み立ても公開もしない」。
  `docs/release.md:43`「`release` Environment は tag `v*` だけを対象にし、本人を必須の承認者として付ける」・`docs/handoff/ST53.md:4`（IMP-01: 印を push できる人が main を通さずに正式な鍵を読める）。
  札の操作が印 `v*` の push でなくなる（あるいは組み立てが merge の時に済んでいて `release.yml` の組み立てを通らない）と、脆弱性の関門は札を付ける時点で走らず（merge の後に見つかった勧告を見ないまま正式版にできる）、鍵を読む job の承認もどこに付くかが決まっていない。FR-135 の未決（:1180）は「いつ署名するか」だけを問い、承認と脆弱性の関門を挙げていない
- 直すところ: requirements（FR-135 の未決に「正式な鍵を読む操作の本人の承認・札を付ける時点の脆弱性の検査を持ち越す」を書く）と新しい Story（3 つの Requirement と `docs/release.md` §3〜§6 を札の操作に付け替える）
- 機械: できない（`tools/check-vulns-wiring.sh` は `release.yml` の配線を見るだけで、札の操作が別の workflow になると素通りする）
- 行き先: requirements（FR-135 に ★ 2026-11-21 補足: 印 `v*` の push の守り —— 承認・鍵・脆弱性の関門・ルールセット —— を札の操作へ移す）。正典と `docs/release.md` §3〜§6 は新しい Story（stories）

## C4. merge ごとに PATCH を 1 つ進めると、携帯の versionCode の桁（PATCH ≤ 99）を 1 か月ほどで越える。正典はその式を「版をまたいで変えない」と固定し、FR-134 の MINOR の未決はこの上限に触れていない
- 重大度: 高
- 文書: openspec/specs/distribution/spec.md:606
- 要件: FR-134, FR-104, NFR-25
- 根拠: requirements.md:1173「右の数字（PATCH）と真ん中の数字（MINOR）は機械が決め」・:1176「[NEEDS CLARIFICATION: 真ん中の数字（MINOR）をいつ上げるか（Story が入ったら、など）…]」。
  正典 spec.md:606「versionCode を…`MAJOR×1,000,000 + MINOR×10,000 + PATCH×100 + 末尾` で計算し」・:608「MINOR と PATCH が 99 を超える版番号では、携帯の組み立てを失敗させる」・:609「この式を…版をまたいで変えない」。
  コード: `tools/release/version.sh:33-34`・`collector-android/app/build.gradle.kts:40-41`（`PATCH が上限を超えた`）。
  main への merge は 2026-10-21〜11-21 の 30 日で 110 回（`git log origin/main --first-parent --merges --since=2026-10-21 | wc -l`）。MINOR を上げない限り、100 回目の merge で携帯の組み立てが落ちる。
  末尾の 2 桁（ベータ版の N・正式版の 99・開発版の 0）は、番号がチャンネルを持たなくなった（FR-104）ので意味を失う。式を変えるなら、印は 0 本で配った APK は無い今のうちが安い（本人の携帯の版は「未確認」）
- 直すところ: requirements（FR-134 の未決に、versionCode の桁の上限と式を今変えるかを足す。文書だけの merge でも番号を進めるかも決める —— PATCH の進みが変わる）と新しい Story（正典の versionCode の Requirement）
- 機械: 安く捕まえられる（新しい Story の試験で、1 か月分の merge だけ進めた番号の versionCode を計算させる）。今は 100 回目の merge の組み立てが落ちて初めて分かる
- 行き先: requirements（FR-134 に ★ 2026-11-21 補足: versionCode の PATCH ≤ 99 と merge の頻度を書き、式を変えるか・文書だけの merge でも進めるかを MINOR の未決と一緒に決める）。正典の versionCode の Requirement は新しい Story（stories）

## C5. 正典は版番号の出所を `Cargo.toml` の版にし、渡せる版番号を `-beta.<N>`・`-dev.<日付>` の形に限る。FR-134 は「main に commit を積まずに」機械が番号を進めるとした
- 重大度: 高
- 文書: openspec/specs/distribution/spec.md:533
- 要件: FR-104, FR-134, NFR-25
- 根拠: requirements.md:973「「版番号の出所」（`docs/production-prep.md`・`docs/release.md` §1 の `Cargo.toml` の workspace の `version`）は FR-134 で機械に移る」・:1172「main に commit を積まずに、版番号を 1 つ進めた組み立てを作る」。
  正典 spec.md:533「版番号を、根の `Cargo.toml` の `[workspace.package]` の `version`（以下「出所の版」）だけから決める」・:536「渡せる版番号は `<出所の版>`・`<出所の版>-beta.<N>`（N は 1〜98）・`<出所の版>-dev.<YYYYMMDD>` の 3 つの形に限り」・:540「API の契約…の版（`info.version`）を、渡した版番号でなく出所の版にする」・Scenario :558「出所の版と違う版番号を渡すと、組み立てが失敗する」。
  機械が `0.1.42` を渡すと、出所の版 `0.1.1` と違うので今の正典では組み立てが落ちる。FR-134 は MAJOR を本人が決める置き場も、checked-in の `docs/openapi.json` の `info.version` を何にするかも書いていない
- 直すところ: 新しい Story（正典の「3 つの部品の版番号は、1 つの出所から決まる」を書き直す。MAJOR の置き場と `info.version` を決める）
- 機械: 捕まえられる（`tools/check-version.sh`・`tools/release/version.sh check` が、出所と違う番号を渡した組み立てを落とす —— 新しい形を作り始めた時点で落ちる）
- 行き先: stories（新しい Story が正典の「版番号は 1 つの出所から決まる」を書き直し、MAJOR の置き場と `info.version` を決める）

## C6. 正典の更新の確認は「入っている版のチャンネル」（版番号の `-beta.`・`-dev.`）で数える印を選ぶ。FR-112 は追う札を部品ごとの設定にした
- 重大度: 中
- 文書: openspec/specs/distribution/spec.md:2580
- 要件: FR-112, FR-113, FR-115, FR-116
- 根拠: requirements.md:1038「同じ組み立てに正式版とベータ版の札が付きうるので、組み立ては自分がどちらの札で配られたかを知らない。**どの札を追うかは部品ごとの設定で持ち、既定は正式版**」。
  正典 spec.md:2580「新しい版かどうかは、入っている版のチャンネルで決める」・:2584「ベータ版（`<X.Y.Z>-beta.<N>`）の部品は…`v<A.B.C>-beta.<M>` の形のリリースを数える」・:2586「ベータ版を、先行公開の旗でなく印の形 `v<版>-beta.<N>` で見分ける」・:2592 導出元「FR-112（「利用者のチャンネル」）」・:2517「道を次の 2 つに限る…`/releases?per_page=20`、…`/releases/tags/nightly`」。
  コード: `crates/update-check/src/lib.rs:211`（`choose` が自分の版の `is_dev`・`is_beta`（:285）で選ぶ）・`collector-android/.../UpdateCheck.kt:93-117`・`UpdateInstall.kt:163`。
  追う札の設定を置く口（S-01 は FR-115 と同じインストーラの命令 —— spec.md:2931、C-01 は端末の設定、C-02 は設定のファイル）も、札を読む問い合わせの道（FR-116 の固定の形）もまだ無い
- 直すところ: 新しい Story（正典の 2 つの Requirement と共有の材料 `docs/contracts/update-check-vectors.json` を書き直す）
- 機械: できない（今の試験は旧い規則を固定しているので、要件の改訂では落ちない）
- 行き先: stories（新しい Story が更新の確認の 2 つの Requirement と `docs/contracts/update-check-vectors.json` を書き直す）

## C7. 正典は「開発版かどうかは版番号（`-dev.`）で決まる」とし、インストーラ・更新の確認・versionCode もそれに乗る。FR-104 は番号にチャンネルの意味を持たせないとし、開発版の組み立てが自分の系統をどう持つかは書いていない
- 重大度: 中
- 文書: openspec/specs/distribution/spec.md:1004
- 要件: FR-104, FR-105, FR-106, FR-107
- 根拠: requirements.md:965「版番号は MAJOR.MINOR.PATCH で、**どの組み立てかだけを表し、チャンネル（正式版・ベータ版・開発版）の意味を持たない**」・:978「**開発版の札だけは、その番号を開発版向けに組んだ物を指す**」・:994「最も新しい番号を開発版向けに組み」。
  正典 spec.md:1004「自分が開発版か正式版か（以下「系統」）を、組み立てのときの版番号だけから決める。版番号が `-dev.` を含めば開発版」・:941「開発版の版番号を `<出所の版>-dev.<作り直した日の日本時間の YYYYMMDD>` にする」。
  コード: `crates/build-stamp/src/lib.rs:32-38`（`channel` は `-dev.` の有無）・`crates/installer/src/update_plan.rs:19`（系統が違えば入れない）。
  開発版向けの組み立ての版番号に印（`-dev`）を付けてよいのか、番号は同じで別の欄に系統を持つのか、で FR-121 の更新の履歴・FR-122 の記録の版・配布物の名前が変わる
- 直すところ: requirements（FR-104 か FR-106 に、開発版向けの組み立てが系統をどこに持つか —— 番号に付けてよいかを書く）
- 機械: できない
- 行き先: requirements（FR-105 に ★ 2026-11-21 補足: 開発版向けも番号は同じで、系統は番号と別に組み立ての種類と同じ所に持つ。AI の推奨（仮））。正典とコードは新しい Story（stories）

## C8. 正典の開発版の作り直しは「幹にその日の変更」と「出所の版を上げる必要」で判定する。FR-107 は札の指す番号より新しい番号があるかに変え、FR-134 は手で番号を上げない
- 重大度: 中
- 文書: openspec/specs/distribution/spec.md:937
- 要件: FR-107, FR-134
- 根拠: requirements.md:994「WHEN 1 日の終わりに、開発版の札が指す番号より新しい番号（FR-134）がある THE SYSTEM SHALL 最も新しい番号を開発版向けに組み、開発版の札をその番号へ動かす」。
  正典 spec.md:940「印 `nightly` の commit と main の HEAD が同じなら何も組まずに終わり、違えば main の HEAD から…組んで」・:943「IF 出所の版の正式版の印 `v<出所の版>` が main の祖先にある THEN …開発版を組まずに終え、出所の版を次の版に上げる必要があることを示す」・Scenario :989「出所の版の正式版を出した後は、出所の版を上げるまで開発版を組まない」。
  コード: `tools/release/version.sh` の `nightly_plan`（「出所の版を上げる必要がある」）・`.github/workflows/nightly.yml:19`
- 直すところ: 新しい Story（正典の Requirement を書き直す）
- 機械: できない
- 行き先: stories（新しい Story が開発版の作り直しの Requirement と `nightly_plan` を FR-107 の形に書き直す）

## C9. 正典と `docs/release.md` に、FR-105 から外した「緊急の修正の枝」の Requirement と手順が残っている
- 重大度: 中
- 文書: openspec/specs/distribution/spec.md:832
- 要件: FR-105, FR-134
- 根拠: requirements.md:982「★ 2026-11-21 訂正。旧い本文の「正式版にだけ緊急の修正を当てるときに限り、その版の枝を一時的に切る」は、番号が main の merge にだけ付く形（FR-134）と合わないので外した」・:983 未決。
  正典 spec.md:834「正式版だけに緊急の修正を当てるときに限り、配った正式版の印 `v<MAJOR>.<MINOR>.<PATCH>` から枝 `release/<MAJOR>.<MINOR>` を切ることを許し」・:838 導出元「FR-105（正式版にだけ緊急の修正を当てるときに限り、その版の枝を一時的に切る）」・:243 Scenario「修正の枝の上では、枝の祖先の印を数える」・:870「例外は、緊急の修正の枝 `release/<MAJOR>.<MINOR>` の上の正式版の印」。
  `docs/release.md:108`「緊急修正が必要なときは正式版から `release/<MAJOR>.<MINOR>` の修正の枝を切り」・:118-123（`Cargo.toml` の版を上げて commit し `git tag v0.1.1`）。コード `tools/release/version.sh` の `plan`（`release/*` の枝を許す）
- 直すところ: 新しい Story（未決を決めた後に、正典の Requirement を REMOVED か MODIFIED にし、`docs/release.md` §8 と `version.sh plan` を合わせる）
- 機械: 捕まえられた（req_impact が spec.md:838 を FR-105 の名指しとして挙げた。食い違いかどうかは見ない）
- 行き先: stories（急ぎの直しの未決（FR-105）を新しい Story の深掘りで決めた後に、正典の Requirement・`docs/release.md` §8・`version.sh plan` を合わせる）

## C10. `docs/release.md` §1・§4・§6 と `docs/production-prep.md` の「バージョンの単一情報源」が、`Cargo.toml` の版を手で上げて commit し印 `v*` を打つ旧い形のまま
- 重大度: 中
- 文書: docs/release.md:11
- 要件: FR-104, FR-105, FR-134, FR-135
- 根拠: requirements.md:973「「版番号の出所」（`docs/production-prep.md`・`docs/release.md` §1 の `Cargo.toml` の workspace の `version`）は FR-134 で機械に移る」・:1172「main に commit を積まずに」。
  `docs/release.md:7`「版番号の出所は `Cargo.toml` の workspace の `version` だけとする…ベータ版は `v<出所>-beta.N` の印」・:11「正式版を公開した後は、次の開発を始める前に `Cargo.toml` の workspace の `version` を次の版へ上げて commit する」・:71「`git tag v<版番号> <commit>` で最初の印を main の commit に打つ」。
  `docs/production-prep.md:47`「★ **出所は `Cargo.toml` の workspace の `version`。ほかは読む（ST52）**」。
  改訂の ★ は両方を名指しして「機械に移る」と書いたが、両文書に改訂を指す注記が無い。本人が今 §6 の手順で最初の印を打つと、FR-135 の札でなく旧い形の正式版ができる
- 直すところ: その文書（両方に「依頼 #352 で置き換え中。新しい Story が書き直すまで §1・§6・§8 の手順で印を打たない」の注記）と新しい Story（本文の書き直し）
- 機械: できない
- 行き先: docs（`docs/release.md` §1 の頭と `docs/production-prep.md` の「バージョンの単一情報源」に「置き換え中。印を打たない」の注記を足した）。本文の書き直しは新しい Story（stories）

## C11. FR-136 の未決の 2 つの選択肢は、それぞれ別の決定とぶつかる（問い合わせなら FR-116 の決まった形、印を足すなら FR-135 の「組み直さずに」と指紋）が、未決はそれを書いていない
- 重大度: 中
- 文書: docs/requirements.md:1185
- 要件: FR-136, FR-135, FR-116, FR-112
- 根拠: requirements.md:1185「[NEEDS CLARIFICATION: 入れる側が「札を一度でも付けたか」をどう知るか（配布元に問い合わせるか、札を付けるときに配布物へ印を足すか）…]」。
  FR-116（:1061）「配布元へ、版の一覧を取る問い合わせ以外の情報を送らない」と正典 spec.md:2517 の道の固定は更新の確認の側だが、インストーラが配布元に問い合わせると、外へ出る口がもう 1 つ増え（網の無い PC では入れられない）、同じ検査に入るかが決まっていない。
  配布物へ印を足すと zip の中身が変わり、FR-135 の「組み直さずに」（:1179「札を付けた後の配布物は、ベータ版で試した物と同じ中身になる」）と、更新の確認が照らす指紋（spec.md:3591「版の一覧の指紋と照らす」）に当たる。
  今のインストーラは版番号の違いだけを見て更新へ進む（`crates/installer/src/update_plan.rs:19-25`: 系統が同じで版が違えば `Update`。札の有無も、古い番号への戻しも見ない）
- 直すところ: requirements（未決の文に、それぞれの選択肢が当たる決定を書き足す）
- 機械: できない
- 行き先: requirements（FR-136 の未決に ★ 2026-11-21 補足: 問い合わせる案は FR-116 と網の無い PC、記しを足す案は FR-135 の組み直さないと指紋に当たる）

## C12. 凍結した Story の価値と完了の判定が旧い形を固定し、再生成した Story の文書が要件と食い違っている
- 重大度: 低
- 文書: docs/stories/stories.json:1242
- 要件: FR-104, FR-105, FR-112, FR-124, FR-127, FR-130
- 根拠: stories.json:1242（ST52 の価値）「印（タグ）を付けるだけで、…同じ版で正式版・ベータ版・開発版に並び」・:1249（ST52 の完了）「`v0.x.y` の印・`-beta.N` の印・開発版の日次の判定」・:1193（ST50 の breaks）「最初の印 `v*`（ベータ版を含む）までは版 1 に含める」・:1277（ST53 の完了）「正式版・ベータ版は印 `v*`」・:1383（ST57 の完了）「1 日 1 回自分のチャンネルを確かめ」。
  check_chain「ST50 / ST52 / ST57: stories.json + requirements.md から再生成した結果と一致しない」
- 直すところ: 凍結した Story への申し送り（ST50・52・53・57 は archive 済みで作り直さない。`make_story.py` で文書だけ貼り直し、新しい Story が置き換えると INDEX に書く）
- 機械: 捕まえられた（check_chain の再生成の一致。stories.json の価値・完了の旧い前提は捕まえられない）
- 行き先: stories（/stories で `make_story.py` により文書を貼り直し、ST50・52・53・57 の旧い形は新しい Story が置き換えると INDEX に書く。凍結した Story は作り直さない）

## C13. 改訂した FR-127 の本文と §5 の箇条に、置き換えたはずの「印（タグ）で示す」が残る
- 重大度: 低
- 文書: docs/requirements.md:1127
- 要件: FR-127
- 根拠: requirements.md:1127（本文）「対応するソースは、同じ印（タグ）のリポジトリで示す」と :1134（★ 2026-11-21）「上の本文の「同じ印（タグ）のリポジトリで示す」…を置き換える」。
  §5 :1639「対応するソースの場所（FR-127。正式版・ベータ版は印、それ以外は commit）」。FR-105 は本文を書き換えたが、FR-127 は本文を残して ★ で打ち消している
- 直すところ: requirements（FR-127 の本文を「番号を組んだ commit で示す」に書き換え、旧い本文を ★ に引用する。§5 :1639 の括弧を直す）
- 機械: できない
- 行き先: requirements（FR-127 の本文を「その番号を組んだ main の commit で示す」に書き換え、★ 2026-11-21 訂正に旧い本文を引いた。§5 の V-01 の箇条の括弧も直した）

## C14. 走っている change 3 本は、この改訂が書き直す同じ Requirement と workflow を触る（印のみ）
- 重大度: 低
- 文書: openspec/changes/ch22-soak-install-online-ci/specs/distribution/spec.md:246
- 要件: FR-112, FR-116, FR-107, FR-135, FR-127
- 根拠: ch22 の MODIFIED が更新の問い合わせの道を「開発版でない部品は `/repos/rai-256/ashiato/releases?per_page=20`、開発版の部品は `/repos/rai-256/ashiato/releases/tags/nightly`」（:246）・Scenario「開発版の部品は印 nightly の 1 件だけを取る」（:283）で写し直す —— C6 で新しい Story が書き直す Requirement と同じ。
  ch25 は `.github/workflows/nightly.yml`・`release.yml` の `package-linux` を触る（`lane.json` の paths）—— C3・C8 で書き直す workflow と同じ。
  ch24 は FR-127 を許諾の一覧の導出元として名指すだけで、`SOURCE.txt` の Requirement には触れない
- 直すところ: leave（新しい Story は、ch22・ch25 の archive の後に 3-way で載せる）
- 機械: できない
- 行き先: leave: 走っている ch22・ch24・ch25 は止めない。新しい Story はそれらの archive の後に 3-way で載せる（並べる席が前提として扱う）

## C15. 「印」が、組み立ての印（`<版番号>+<commit>`）・印（タグ）・札の表し方の 3 つの意味で使われ、FR-136 の未決の「配布物へ印を足す」がどれか読めない
- 重大度: 低
- 文書: docs/requirements.md:1185
- 要件: FR-136, FR-105
- 根拠: requirements.md:1185「札を付けるときに配布物へ印を足すか」・:980「札を配布元でどう表すか（リリースの種類・印など）」。
  組み立ての印は正典 distribution spec.md:315「自分の組み立ての印を、`<版番号>+unknown…` の印も含めて」で別の意味に使われている
- 直すところ: requirements（:1185 を「配布物へ札を付けた記しを足すか」など、組み立ての印・タグと別の語にする）
- 機械: できない
- 行き先: requirements（FR-136 の未決の「配布物へ印を足す」を「記しを足す」にし、組み立ての印・タグと別のものと ★ に書いた）

## 2. 文書とコード

## K1. 配った移行と送信契約の版の凍結は、印 `v[0-9]*` と印 `nightly` が origin に 1 つも無いので、今は何も凍結していない。本人の PC に入っている 0.1.x の移行を守る検査は無い
- 重大度: 中
- 文書: docs/requirements.md:1186
- 要件: FR-137, FR-129, FR-124
- 根拠: FR-137（:1186）「本人の PC が今の形の版（0.1.x。札の無い組み立て）から新しい形の最初の版へ移る THE SYSTEM SHALL D-01 と D-02 の記録を失わずに移す」。
  `git ls-remote --tags origin` は空（2026-11-21）。`tools/check-released-migrations.sh:27,34`（`git tag --list 'v[0-9]*'`）・`tools/check-contract-version.sh:118`・`tools/check-nightly-migrations.sh:61`（`refs/tags/nightly`）・`.github/workflows/ci.yml:130-132,139-141` は印が無ければ通す。
  0.1.0（`de49710d1`。`docs/install.md:29`）から HEAD までの `migrations/` の差は追加だけ（`git diff --name-status de49710d1 HEAD -- migrations/ | grep -v '^A'` は空）なので、今は壊れていない。
  ただし新しい形の最初の札を付けるまでに、0.1.x に載った移行の名前か中身を変える merge が入っても CI は止めず、本人の D-01 は移る時に「知らない版」（FR-129）で止まるか、形が黙ってずれる
- 直すところ: 新しい Story（FR-137 の設計で、本人の PC に入っている 0.1.x の commit を「配った版」として凍結の検査に数える）
- 機械: 安く捕まえられる（`check-released-migrations.sh` に 0.1.x の commit を固定で数える行を足す）
- 行き先: stories（新しい Story が FR-137 の設計で、本人の PC に入っている 0.1.x の commit を配った版として凍結の検査に数える）

## K2. 正式版・ベータ版の配布は今も印の push で組み直し、`SOURCE.txt` に印を書く。FR-135 の「組み直さずに」と FR-127 の「正式版・ベータ版も commit」と逆
- 重大度: 中
- 文書: docs/requirements.md:1133
- 要件: FR-127, FR-135
- 根拠: requirements.md:1133「**正式版・ベータ版も、印でなく番号を組んだ main の commit（40 桁）で対応するソースを示す**」・:1177「組み直さずに」。
  `.github/workflows/release.yml:4`（`tags: ['v*']`）→ :36・:59 の `package-linux` / `package-windows` が組み直し、:51・:56・:82・:88 で `ASHIATO_SOURCE_REF: ${{ github.ref_name }}` を渡す。
  `tools/release/notices.sh:39-41`（`ASHIATO_SOURCE_REF` があれば `Ref: v<版>` を書き、commit を書かない）。正典 distribution の `SOURCE.txt` の Requirement（spec.md:2263〜）と ST53 の完了（stories.json:1277）も同じ
- 直すところ: 新しい Story
- 機械: できない（今の自己試験は旧い規則を固定している）
- 行き先: stories（新しい Story が `release.yml` の組み直しと `notices.sh` の `Ref: v<版>` を、札の操作と commit の形に直す）

## K3. FR-107 の ★ が「別に調べる」とした毎晩の配布の 15 秒の空振りは、公開の設定（リポジトリ変数 `ASHIATO_NIGHTLY_PUBLISH`）が無いときの、正典どおりの振る舞い
- 重大度: 低
- 文書: docs/requirements.md:999
- 要件: FR-107
- 根拠: requirements.md:999「毎晩の配布（nightly）が毎日 15 秒ほどで何も出さずに終わっている件は、この改訂に含めない（#352 の「どこまで」。別に調べる）」。
  `gh variable list` は空。2026-11-20T22:01Z の run の plan は `build=no`・`publish=no`・`version=0.1.1-dev.20261121`。
  `tools/release/version.sh` の `nightly_plan`:「`[[ "$event" == schedule && "${ASHIATO_NIGHTLY_PUBLISH:-}" != on ]]` なら `build=no`」。
  正典 distribution spec.md:942「WHILE 本人が開発版の公開の設定を入れていない THE SYSTEM SHALL 予定の実行で何も組まず、何も公開せず」。`docs/release.md:64`「設定が無い間は公開しない」。直近 5 回はどれも 12〜19 秒
- 直すところ: requirements（★ の「別に調べる」に、原因が公開の設定の未設定であることを書く。開発版の札を動かし始めるときに設定を入れる）
- 機械: できない
- 行き先: requirements（FR-107 に ★ 2026-11-21 補足: 15 秒の空振りの原因は公開の設定の未設定で、正典どおり。札を動かし始めるときに設定を入れる）

## 未確認

- 本人の PC に入っている 0.1.x がどの commit か（PR の head なら、main に入る前に中身の変わった移行が本人の D-01 に当たっている可能性がある。K1 の「0.1.0 からの差は追加だけ」は main の 0.1.0 との比較で、PR の head とは比べていない）
- 本人の携帯の C-01 の版と署名の鍵。PR の組み立て（debug の鍵）なら、正式な鍵で署名した新しい形へは Android が上書きを拒み、入れ直しで C-01 の未送信を失いうる。FR-137 は PC（D-01・D-02）だけを書く
- 札の表し方の候補（GitHub のリリースの種類・動く印・別の一覧のファイル）ごとに、FR-116 の決まった形の問い合わせで追う札の番号を読めるか
- 正典の Scenario のテストの印の全件の突き合わせ（`scripts/check_scenarios.py` は change 名か `--all` が要る。この改訂は正典を変えていないので走らせていない）
- ch22・ch25 がどの段にいるか、archive の順

## 触らなくてよいと判断したもの

- requirements.md の ★ の経緯の中の印 `v*`・`-beta.N`・`nightly`（:969 FR-104 ★ 2026-11-01、:1110 FR-124 ★ 2026-10-29、:1152 FR-130 ★ 2026-10-29、:1653 §5 ★ 2026-11-03 の「最初の印から後は」、:1895 扉 34 の ★）—— 経緯の引用で、後の ★ が置き換えを書いている（:1653 の「最初の印」は C2 で検査の側を直せば読み替えで足りる）
- `docs/stories/INDEX.md:332,374,379,491,529` —— 経緯の記録
- `docs/handoff/ST50.md`・`ST51.md`・`ST54.md`・`ST55.md`・`ST57.md`・`ST60.md` の名指し —— 宛先は凍結済みか、改訂した部分（札・番号）と関わらない。ST52 の申し送り :7・:21 だけは C2 に入れた
- `docs/ui-direction.md:23`（S-7）—— FR-112 の追う札の設定の置き場は FR-115 と同じで、S-01 ではインストーラの命令（spec.md:2931）。画面の対応表を変える必要は無い
- `docs/install.md:29,82` —— 0.1.0 / 0.1.1 と「別の版が入っています」の今の事実を書いていて、正しい（`crates/installer/src/commands.rs:247`）
- 正典 distribution の「S-01 の版が変わると、更新の履歴に 1 行残る」の印 `0.1.0+<commit>`（spec.md:303〜）—— 組み立ての印で、札とは関わらない
- `docs/release.md` に `## 9.` が 2 つあり、その下が `11.x` の番号 —— この改訂と関わらない
