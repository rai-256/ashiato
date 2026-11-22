# 整合の確認 — FR-134 を ST61 の深掘り第 1 回の答え（Q1〜Q4。どれも推奨のまま）で改訂した（2026-11-23）

- 基準: `8ee21190466cddd383c16561bd01c036d4047bab`..`bd234ff4a974a843a9400634620ec16fa45e0555`

## 機械の入力（写し）

`python3 scripts/req_impact.py . --base 8ee21190466cddd383c16561bd01c036d4047bab`（改訂した要件 1 件・扉 0 項）:

- FR-134（改訂）: `docs/production-prep.md:47`・`docs/release.md:7`・`docs/stories/INDEX.md:85,337,555`・`docs/stories/stories.json:1472`・
  `openspec/changes/st61-auto-version-bump/deep-questions.json:17,25,62,134`・`deep.md:36,38,39,40,41,45`・`design.md:4,79,101,129,164`・`premises.md:53`・
  `proposal.md:4,10,31`・`review/deep.md:8,38,54,79,107`・`review/spec.md:5,130,133,134,137,138`・`specs/distribution/spec.md:14,80,154,219,239,259,281,323,407,480,940`

`python3 scripts/check_chain.py .`（chain: FAIL 1 件 / 未回収 0 件 / warn 0 件）:

- [FAIL] 整合の確認: 整合の確認の後に要件が変わった（この監査を名指ししない改訂）: FR-134（この監査がそれに当たる）
- 検査 10（判断の日より新しい訂正）は ST61 を出していない —— ST61 は走っている change を持つので検査の対象の外（`scripts/check_chain.py:217`）。下の C2 はその穴に当たる

`req_impact.py` は `docs/requirements.md` の中で FR-134 を名指しする他の項（FR-104:965・973、FR-105:983、FR-107:995、NFR-25:1389、§5:1675）を出さない。下の C1 はそこから拾った。

走っている change（`openspec/changes/` の archive 以外）は `st61-auto-version-bump` の 1 本だけ。下の C・K にこの change の中の文書への差し戻しは無い。
本人の答え（`deep-answers-1.txt`）は Q1〜Q4 が推奨のままで、spec は推奨で先に書いてあり合意済み（`agreement.json`）なので、答えを失うものは無い（失われるもの: なし）。

## 1. 改訂後の要件と、ほかの文書

## C1. 要件の §5「配布の方針」の本文が、改訂前の FR-134（main の merge ごとに番号を進める）のまま残る
- 重大度: 中
- 文書: docs/requirements.md:1675
- 要件: FR-134, FR-105
- 根拠: requirements.md:1675「**番号とチャンネルを分ける**（★ 2026-11-21 追加。依頼 #352。本人の決定）。**番号は main の merge ごとに機械が進め（FR-134）**、正式版・ベータ版・開発版は「どの番号を指すか」の札にする（FR-105）」。改訂後の FR-134（:1177）「WHEN main の commit が CI を通り、前の番号を付けた commit から製品（文書・仕様以外）が変わっている THE SYSTEM SHALL … その commit に版番号を 1 つ進めた組み立てを作る」・:1185「**番号は merge 1 つずつでなく、CI が通り、前の番号から製品が変わった main の commit に付ける**」。§5 の行は経緯の引用（★ の旧い本文）でなく方針の本文として書かれており、FR-134 を名指しして改訂前の引き金を述べる。同じ節の :1173 は依頼 #352 の題の逐語の引用なので別
- 直すところ: requirements（§5 の行を「番号は、CI が通り製品が変わった main の commit に機械が付け（FR-134）」に直す）
- 機械: 安く捕まえられる（`req_impact.py` が `docs/requirements.md` の中で改訂した ID を名指しする他の項の行も出せば、:1675 が一覧に載る。または改訂の後に `rg 'merge ごと|merge のたび' docs/requirements.md` を 1 回）
- 行き先: requirements（§5 の行を「CI が通り製品が変わった main の commit に機械が付け」に直し、★ でこの監査を名指しした）

## C2. ST61 の価値が「main に merge するたびに次の番号」のまま残り、judged も改訂前の日のまま
- 重大度: 中
- 文書: docs/stories/stories.json:1481
- 要件: FR-134
- 根拠: stories.json:1481（ST61.md:16）「本人として、版を上げる操作をせずに、**main に merge するたびに次の番号の組み立てができてほしい**」。同じ項の題（:1470）は「main の製品が変わるたびに版番号が自動で進む」、完了の判定 1 つ目（:1489）は「文書だけの merge には番号を付けず、CI が取り消された merge の中身は次の番号に入る」に直っている。deep.md:38・:47 の「要件へ戻すもの」は題と完了の判定 1 つ目だけを挙げ、価値を挙げていない。`"judged": "2026-11-21"`（:1502）のまま —— 題と完了の判定を 2026-11-23 に見直したのに判断の日が動いていない
- 直すところ: stories（ST61 の value を「CI が通り、製品が変わった main の commit ごとに次の番号の組み立てができてほしい」の形に、judged を 2026-11-23 にして make_story.py で ST61.md を貼り直す）
- 機械: できない（検査 10 は change のある Story を見ない —— `check_chain.py:217` の `has_change`。archive の後も `has_change` が真なので出ない）。安く捕まえられる: 検査 10 を change のある Story にも warn で広げる（2026-11-15 の ST58 の C1 と同じ穴）
- 行き先: stories（stories.json の ST61 の価値を「main に merge して製品が変わるたびに」に直し、judged を 2026-11-23 にして再生成した）

## C3. INDEX.md の Story 表の ST61 の題が、改訂前の「merge するたびに」のまま
- 重大度: 中
- 文書: docs/stories/INDEX.md:85
- 要件: FR-134
- 根拠: INDEX.md:85「| [ST61](ST61.md) | **main に merge するたびに版番号が自動で進む** | 6 | FR-134, FR-104, NFR-25 | ST52, ST54 | 34 |」。stories.json:1470「"title": "main の製品が変わるたびに版番号が自動で進む"」・ST61.md:3「title: main の製品が変わるたびに版番号が自動で進む」。INDEX の Story 表の題を stories.json と全件で突き合わせると、食い違いは ST61 の 1 件だけ
- 直すところ: その文書（INDEX.md:85 の題を stories.json に合わせる）
- 機械: 安く捕まえられる（`check_chain.py` の検査 9 は Story 表に ID があるかだけを見る —— `check_chain.py:173`。Story 表の 2 列目を stories.json の title と照らせば捕まる）
- 行き先: stories（INDEX.md の ST61 の題を stories.json に合わせた）

## C4. docs/release.md の「置き換え中」の注記が、番号を「main の merge ごとに」進めると書き、ST61 の tasks もその行を直さない
- 重大度: 中
- 文書: docs/release.md:7
- 要件: FR-134
- 根拠: release.md:7「★ 2026-11-21: **この手順書の §1・§6・§8 は置き換え中。** 要件の改訂（… FR-134〜FR-137）で、**版番号は main の merge ごとに機械が進め**、正式版・ベータ版・開発版は番号を指す札になった」。改訂後の FR-134（requirements.md:1177・:1185）は、CI が通り製品が変わった commit にだけ付け、文書・仕様だけの merge には付けない。ST61 の tasks.md:127 は release.md に「番号」の節を足すとし、:130 の検証は `tools/release/MAJOR`・`build-`・`欠番`・`左の数字` の有無だけを見るので、:7 の文は ST61 の後も残りうる。手順書を読む本人は、文書だけの merge でも番号が付くと読む
- 直すところ: その文書（:7 の「main の merge ごとに」を「CI が通り製品が変わった main の commit に」に）
- 機械: 安く捕まえられる（改訂の後に `rg 'merge ごと|merge のたび' docs/release.md docs/production-prep.md` を 1 回）
- 行き先: docs（docs/release.md:7 の注記を改訂後の FR-134 に合わせ、★ でこの監査を名指しした）

## C5. docs/production-prep.md の「バージョンの単一情報源」の行が、番号を「main の merge ごとに」進めると書く
- 重大度: 中
- 文書: docs/production-prep.md:47
- 要件: FR-134
- 根拠: production-prep.md:47「★ 2026-11-21: **置き換え中** —— 要件の改訂（依頼 #352。FR-134）で、**番号は main の merge ごとに機械が進め**、main に commit を積まない」。改訂後の FR-134（requirements.md:1177・:1185）と食い違う。ST61 の tasks.md:128 は同じ行に ★ を足す（「番号は main の番号付けが付け…」）とするが、既存の ★ 2026-11-21 の文は直さない
- 直すところ: その文書（★ 2026-11-21 の「main の merge ごとに」を「CI が通り製品が変わった main の commit に」に）
- 機械: 安く捕まえられる（C4 と同じ `rg`）
- 行き先: docs（docs/production-prep.md の「バージョンの単一情報源」の ★ 2026-11-21 の文言を改訂後の FR-134 に合わせた）

## C6. FR-134 の ★ は真ん中の上限に届いたら番号付けが止まるとするが、0.x の間は左を上げられない（NFR-25）ことを結んでいない
- 重大度: 低
- 文書: docs/requirements.md:1187
- 要件: FR-134, NFR-25
- 根拠: requirements.md:1187「真ん中を確認バッチごとに上げると、約 4 年で真ん中の上限に届く（**届けば番号付けが止まって知らせる**。同 IMP-07）」。design.md:85「D3 の式の真ん中の上限 999 に約 4 年で届く。**届く前に左を上げるか式を見直す**（D14 が止めて知らせる）」。requirements.md:1386「**NFR-25**: 版番号は、**一般向けの配布までは 0.x** とし、一般向けの配布の最初の正式版を **1.0** とする」。一般向けの配布が約 4 年のうちに無ければ、届いたときの手は「式を見直す」だけで、配った APK の versionCode を減らさない式に限られる（FR-134 :1187 の式は左の重みで真ん中の上限を決めている）。要件の文のままだと、届いたときに左を上げてよいと読める
- 直すところ: requirements（FR-134 の ★ に「0.x の間（NFR-25）に届けば、左を上げずに式を見直す」を足す）か leave（4 年先の話。届く前に IMP-07 で知らせる）
- 機械: できない
- 行き先: requirements（FR-134 に ★ 2026-11-23 補足: 0.x の間（NFR-25）に真ん中の上限に届けば、左を上げずに式を見直す）

## 2. 文書とコード

## K1. docs/release.md §1 は CI の `package` job と書くが、その名の job は無い
- 重大度: 低
- 文書: docs/release.md:11
- 要件: FR-134, FR-104
- 根拠: release.md:11「コードを変更した PR では、**CI の `package` job** が同じ版番号の Linux / Windows のサーバ、PC の収集アプリ、携帯の APK を組み立てる」。`.github/workflows/ci.yml` の job は `package-linux`（:462）・`package-windows`（:496）・`package-artifacts`（:705）で、`package` という名の job は無い（`changes` の `code` が真のときだけ走る —— :464 の `if: needs.changes.outputs.code == 'true'`）。§1 は :7 で置き換え中とされ、ST61 の tasks.md:127 が同じ手順書に「番号」の節を足す
- 直すところ: その文書（`package-linux` / `package-windows` / `package-artifacts` の job と書く。ST61 が §1 を書き直すときに一緒に）
- 機械: 安く捕まえられる（手順書の `` `<名>` job `` を ci.yml の job の名と照らす検査。今は無い）
- 行き先: docs（docs/release.md §1 の job 名を `package-linux`・`package-windows`・`package-artifacts` に直した）

## 未確認

- FR-134 の ★ 2026-11-23 の「待てる run が 1 つだけなので merge ごとには付けられない」（GitHub の concurrency の待ちの数）は、この repo の動きとしては確かめていない（premises P13 にあるだけ）
- 番号付けの workflow・番号の印 `build-*`・`tools/release/MAJOR`・0.0.x の組み立て・新しい versionCode の式は、まだコードが無いので照合していない（ST61 の実装の前）。ST61 の change の 55 の Scenario に印が無いのは同じ理由（`scripts/check_scenarios.py .` の FAIL は 55 件とも change の側）
- FR-134 の ★ の「main への merge は 2026-10-21〜11-21 の 30 日で 110 回」: `git log origin/main --first-parent --merges --since=2026-10-21 --until=2026-11-21` は 109 回（うち題に verify を含む merge 20 回）。期間の端の数え方の差で、約 4 年（真ん中 999 ÷ 年に約 240）の見積りは変わらない。premises P2 の 115 回とは数え方を突き合わせていない

## 触らなくてよいと判断したもの

確かめて食い違わなかったもの:

- FR-134 の「製品（文書・仕様以外）」: CI の `changes` の `code` の判定 `tools/release/changes.sh:7`（`^(docs/|openspec/)|\.md$` だけの差分なら偽）と同じ範囲。`docs/` と `openspec/` は配布物に埋め込まれていない（`crates/*/src`・`build.rs` に `include_str!` で `docs/`・`openspec/` を読む所は無い。`collector-android/app/build.gradle.kts:210`・`:241` の `docs/` は試験の入力だけ）
- FR-134 の「CI が取り消された merge」: `.github/workflows/ci.yml:6-8`（`group: ci-${{ github.ref }}`・`cancel-in-progress: true`）で main への push の run は次の push に取り消される
- FR-134 の ★ 2026-11-21 の「今の携帯の versionCode の式」: `collector-android/app/build.gradle.kts:38-48`（`major * 1_000_000 + minor * 10_000 + patch * 100 + tail`、MINOR・PATCH が 99 を超えると落とす）のとおり。★ 2026-11-23 の「本物の番号は 0.2.0 から（今の 0.1.0・0.1.1 より大きい）」: `Cargo.toml:6` は `version = "0.1.1"`。新しい式の 0.2.0（20,000）は今の 0.1.1（10,199）より大きい
- 正典の Scenario のテストの印: `scripts/check_scenarios.py .` の FAIL 55 件はすべて `st61-auto-version-bump` の change の側で、正典（`openspec/specs/`）の Scenario には全件印がある

走っている change（`st61-auto-version-bump`）が受けるもの（印だけ）:

- 正典 `openspec/specs/distribution/spec.md:604-609`「携帯の versionCode は版番号から計算する」（`MAJOR×1,000,000 + …`、「この式を…版をまたいで変えない」）は改訂後の FR-134（式を広げる。Q2）と食い違うが、change の spec が REMOVED（`specs/distribution/spec.md:923`）と ADDED「携帯の versionCode は、番号の 3 つの数字から計算する」（:274）で置き換える。同じく正典 :832「緊急の修正の枝」は REMOVED（:938）、:861「印（タグ）の名前で…」は REMOVED（:928）と ADDED（:309）
- change の中の文書（`deep-questions.json:4-5` の subtitle「main に merge するたびに」など）の旧い題は、合意の digest に結び付いた問いの記録で、差し戻しを書かない
- `docs/briefs/ST61-pr.md` の題「main に merge するたびに」と「答え待ち」は、gitignore（`.gitignore:34`）の生成物で、`scripts/pr_body.py` が PR のときに組み直す

ほかの Story と申し送り（「merge のたびに番号」を前提にしていないことを確かめた）:

- ST62〜ST64 の完了の判定・壊してはいけないもの（stories.json）は番号の付く時を述べない。ST63 の「main に新しい番号がある日の終わりに」は改訂後の FR-134 と FR-107 のまま合う
- `docs/handoff/ST62.md` の R3（番号ごとに下書きに全部残す）・R12（「main で直して次の番号を出す」）、`docs/handoff/ST64.md` の R9（PR の APK は `0.0.1`・versionCode 1）は、Q1・Q4 の答えと合う。宛先の ST62・ST64 は改訂で消えていない
- FR-105 の未決「急ぎの直しの出し方」（requirements.md:984）は ST61 の深掘りでは閉じておらず、`docs/handoff/ST62.md` の R12 が ST62 へ送っている。改訂後の FR-134 と食い違わない

経緯の引用で、本文ではないもの:

- requirements.md:998（FR-107 の ★「merge のたびに開発版向けにも組むと…」）・:1173（依頼 #352 の題の逐語）・:1174（本人の言葉）・:983（FR-105 の ★ 2026-11-21 訂正）・:1182-1183（FR-134 の ★ 2026-11-21 補足。「上の未決と一緒に…決める」は直下の ★ 2026-11-23 訂正が閉じている）・`docs/stories/INDEX.md:555`（2026-11-21 の訂正の経緯の箇条）
- ST61 の完了の判定 3 つ目「携帯の組み立ては 1 年分の merge の番号でも落ちない」は、番号の数が merge の数以下になったので改訂後も成り立つ（deep.md:47 も同じ判断）
- ST61.md:52 の「効いてくる未決」の定型文（「旧形式の未決は OPEN 表 / DDR 台帳側にある」）は `make_story.py` が未決の無い Story 54 本に同じく出す文で、この改訂の食い違いではない
