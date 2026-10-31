# ST52 の前提（外の形式・振る舞い）

問いを立てる前に確かめたもの。確かさは story-upstream の 1〜5（1 言っただけ / 2 出所を示した / 3 辿った / 4 走らせた / 5 実物で再現した）。
失われるもの（`loss`）のある問い（Q1・Q2・Q3・Q5・Q11）が依る前提は、Q1 が P2・P3・P4、Q2 が P2・P6、Q3 が P5・P13、Q5 が P6・P11、Q11 が P2・P3 で、どれも確かさ 4 以上。P8・P12・P16 は推奨の側の守りの前提で（外れても、失う側の選択は変わらない）、2 のまま tasks で 4 に上げる。

| # | 前提 | 何が依るか | 確かめ方 | 結果 | 出所 | 確かさ |
|---|---|---|---|---|---|---|
| P1 | サーバ（`ashiato-server`）は Windows（x86_64-pc-windows-msvc）向けに組み立てられ、実行ファイルが Windows で起動する | サーバの配布物を Windows 向けに作れるか（FR-104・FR-108 の `[NEEDS CLARIFICATION: S-01 を Windows 向けに組み立てられるかは試していない。期限: 配布の Story の上流]`）・Q7 | 2026-10-31、WSL から Windows 側の cargo（msvc）で `cargo build --release -p ashiato-server`（作業木の写しを `C:\dev\ashiato2-st52-build` に置いた）。できた `ashiato-server.exe --version` を Windows で起こした | rc=0（2 分 00 秒）。`ashiato-server.exe` 14.6 MB ができ、起こすと `知らない引数: --version（使えるのは migrate だけ）` と main の引数の検査まで動く。**DB に繋いで待ち受けるところは試していない**（DB の実行環境は ST55） | 実物 | 4 |
| P2 | 携帯で、入っている APK と**違う鍵**で署名した APK は上書きで入らない | Q1（いまの携帯の切り替え）・Q2（署名の場所） | エミュレータ（Android 35、`ashiato-api35`）。debug 鍵の versionCode 1 を入れた上に、新しい鍵だけで署名した versionCode 2 を `adb install -r` | `INSTALL_FAILED_UPDATE_INCOMPATIBLE` | 実物（エミュレータ） | 5 |
| P3 | 消して入れ直すと、アプリの手元のデータ（未送信の溜め・設定）は消える | Q1 | 同じエミュレータ。`run-as` で `files/` に印のファイルを置き、`adb uninstall` → 新しい鍵の APK を `adb install` | 入れ直した後に印のファイルが無い（`absent`） | 実物（エミュレータ） | 5 |
| P4 | 古い鍵から新しい鍵への**署名の引き継ぎ**（APK Signature Scheme v3 の鍵の回転）を付けた APK は、古い鍵の APK の上に上書きで入り、手元のデータが残る。その後は新しい鍵だけの APK も上書きで入る | Q1 の選択肢「鍵を引き継ぐ」 | `apksigner rotate`（debug 鍵 → 新しい鍵）で系譜を作り、`--next-signer --lineage` で署名した APK を debug 鍵の APK の上に `adb install -r`。続けて新しい鍵だけの APK を `-r` | どちらも `Success`。印のファイルは `present` のまま | 実物（エミュレータ） | 5 |
| P5 | versionCode が今入っているものより**小さい** APK は上書きで入らない | Q3（versionCode の付け方）・Q4（開発版の識別子） | エミュレータ。versionCode 2 を入れた上に versionCode 1 を `adb install -r` | `INSTALL_FAILED_VERSION_DOWNGRADE`（印のファイルは残る） | 実物（エミュレータ） | 5 |
| P6 | 配布元（`rai-256/ashiato`）は**公開**リポジトリで、配った物は誰でも取れる。Actions の標準の実行は公開リポジトリでは課金されない | Q5（最初の印は外に出る）・Q9（PR ごとに組む費用）・「壊してはいけないもの」の Actions の枠 | `gh api repos/rai-256/ashiato --jq .visibility` → `public`。課金は https://docs.github.com/en/billing/managing-billing-for-your-products/managing-billing-for-github-actions/about-billing-for-github-actions （"GitHub Actions usage is free for standard GitHub-hosted runners in public repositories"） | 公開（`public`・AGPL-3.0）。Actions の Secrets は `HARNESS2_TOKEN` だけ（署名の鍵はまだ無い） | 実物（公開の状態）＋公式文書（課金） | 4（公開）／2（課金） |
| P7 | 予定の実行（`schedule`）は既定の枝（`main`）でだけ走り、混んでいると遅れる・落ちることがある | 開発版の「1 日の終わり」（C4） | https://docs.github.com/en/actions/writing-workflows/choosing-when-your-workflow-runs/events-that-trigger-workflows#schedule | 文書どおり | 公式文書 | 2 |
| P8 | フォークからの PR の実行には Secrets が渡らない | 署名の鍵を Secrets に置いたときの漏れの経路（Q2） | https://docs.github.com/en/actions/security-for-github-actions/security-guides/using-secrets-in-github-actions （"secrets are not passed to the runner when a workflow is triggered from a forked repository"） | 文書どおり | 公式文書 | 2 |
| P9 | PC の収集アプリは CI の `windows-latest` で既に組み立てている | C-02 の配布物（C3） | `.github/workflows/ci.yml` の `collector-windows` / `collector-windows-runtime` の job | 有り | 実物（repo） | 4 |
| P10 | 版番号は今 3 か所に手書き（`Cargo.toml` の workspace `0.1.0`・`collector-android/app/build.gradle.kts` の `versionName = "0.1.0"` と `versionCode = 1`・`web/package.json` の `0.1.0`）。`crates/build-stamp`・gradle・`web/build-stamp.ts` がそれぞれから組み立ての印を作る | Q6（版の出所） | `grep -n version Cargo.toml web/package.json collector-android/app/build.gradle.kts` | 3 か所。どれも `0.1.0` | 実物（repo） | 4 |
| P11 | 配った移行の凍結（ST48 `tools/check-released-migrations.sh`）と送信契約の版の検査（ST50 `tools/check-contract-version.sh`）は、どちらもタグ `v[0-9]*`（ベータを含む）を「配った印」と読む。印が 0 個の間は何も凍結しない | Q5（最初の印）・Q10（開発版の印の名前） | 2 本の台本を読んだ（`git tag --list 'v[0-9]*'`） | そのとおり。今の印は `st50-pre-rebase` だけで、`v*` は 0 個 | 実物（repo） | 4 |
| P12 | GitHub の Environment に置いた Secrets は、Environment の配布の規則（印 `v*`・枝 `main`）に当たらない実行（同じリポジトリの枝からの PR を含む）からは読めない | Q2 の推奨の守り | https://docs.github.com/en/actions/managing-workflow-runs-and-deployments/managing-deployments/managing-environments-for-deployment#deployment-branches-and-tags 。**tasks で、使い捨ての値を Environment に置き、PR の実行から読めないことを走らせて確かめてから本物の鍵を置く** | 文書どおり（未走行） | 公式文書 | 2 |
| P13 | versionCode が**同じ** APK は上書きで入る | Q3（開発版は同じ版の間は同じ数） | P4 と同じエミュレータで、versionCode 2 の上に versionCode 2（鍵の引き継ぎ付き）を `adb install -r` | `Success` | 実物（エミュレータ） | 5 |
| P14 | 印を動かすと、それを取った clone の `git fetch --tags` が rc=1 になる（`git fetch` は rc=0 で、手元の印は古いまま） | Q10 | deep-review が使い捨ての repo で試した（2026-11-01、git 2.53.0。`! [rejected] nightly -> nightly (would clobber existing tag)`） | rc=1 | 実物 | 4 |
| P15 | 配布版の APK（接続先と資格情報を渡さずに組んだもの）は送信せず、取得は続けて未送信に積み、90 日 / 2 GB で捨てる | Q1 | `collector-android/app/build.gradle.kts:57-61`（組み立ての時に property から焼き込む）・`Config.kt:18-29`（揃わなければ `isComplete=false`）・`LocationService.kt:927-930`（送らずに積む）・`Retention.kt:12-16` を辿った | そのとおり。設定させる口は ST56（FR-128） | 実物（repo） | 3 |
| P16 | 下書き（draft）のリリースは書き手にしか見えず、公開するまで印を作らない | Q5 の推奨 | https://docs.github.com/en/repositories/releasing-projects-on-github/managing-releases-in-a-repository 。tasks で下書きを作って印が増えないことを確かめる | 文書どおり（未走行） | 公式文書 | 2 |

## 触らなかったもの

- Windows で**サーバが DB に繋いで画面を配る**ところは試していない（DB の実行環境と Windows のフォルダは ST55）。ST52 の確かめは Linux の CI と、Windows では組み立てと起動の入口まで
- 実機の携帯に今なにが入っているか（どの PC の debug 鍵で組んだか・未送信が何件か）は、本人の手元にしか無い → Q1 で聞く

## 聞かない既定（C。扉を開けたままにする側。spec の段で design の D 番号に写す）

| # | 既定 | なぜ聞かないか | 出所 |
|---|---|---|---|
| C1 | 公開するリリースの APK は、接続先と資格情報が空であることをリリースの workflow が確かめる（空でなければ公開しない） | 空にする側だけが漏れない。費用は検査 1 つ | review/deep.md R1 |
| C2 | 印は `main` の祖先の commit にだけ打てる（緊急の修正の枝 `release/<MAJOR>.<MINOR>` の上の正式版の印を除く）。workflow が `git merge-base --is-ancestor` で見て、違えば公開しない | FR-105 の「1 本の幹」を機械で守る | R8 |
| C3 | （仮 D1）配った印の検査（ST48 `check-released-migrations.sh`・ST50 `check-contract-version.sh`）は、今の木の祖先にある印だけを数え（`git tag --merged HEAD`）、「最後の印」も日付でなく祖先から選ぶ。申し送りの「型を変えるなら一緒に変える」に当たるので、ST52 が 2 本を直す | 緊急の修正の枝で検査が誤って落ちる・送信契約の版の上げ忘れを見逃す、を防ぐ側。数える印を絞るだけで型は変えない | R9 |
| C4 | （仮 D2）`v*` の印は消さない・動かさない。GitHub のルールセットで `v*` の削除と更新を拒む（設定は本人が入れる手順を置き、入っていることを `gh api` で CI が見る）。壊れた版は次の版を出して直す | 消すと配った版の凍結が黙って外れる。止める側は設定 1 つ | R10 |
| C5 | （仮 D3）正式な鍵と開発版の鍵は、Play App Signing に持ち込める種類・長さ（RSA 4096、有効期限 25 年以上）で作る。作る台本が `keytool -list -v` で確かめる | いずれ Play ストアへ（§5）。満たす側の費用は無い。条件の一次情報は tasks で premises に足す | R13 |
| C6 | 開発版は日本時間の毎日 0:00（cron は UTC 15:00）に起き、`nightly` の印の commit と `main` の HEAD が違うときだけ組む（予定の実行が 1 回落ちても、次の回で拾う） | 時刻は後から変えれば戻る | R16・P7 |
| C7 | リリースは 3 部品がそろったときだけ公開する（1 つでも組めなければ下書きのまま・開発版は前の版のまま） | 欠けた版を出さない側 | FR-104 |
| C8 | 配る組み立ての印（ST49）に `.dirty` と `.debug` が無いことと、3 部品の版が印と一致することを、リリースの workflow と PR の組み立て（Q9）が確かめる | 「3 部品の版の食い違いを CI が落とす」の機械の形 | 完了の判定・R12 |
| C9 | 緊急の修正の枝は FR-105 の例外にだけ切り、正式版の印を打ったら消す | 要件の本文どおり | FR-105 |

## 他の Story へ回すもの

- PC の収集アプリには後戻りを止めるもの（携帯の versionCode にあたるもの）が無い。新しい版から古い版へ入れ直すと、新しい版が積んだ未送信の新しい欄を古い版が落として送る → ST58（R14）
- 開発版の DB の扱い（幹の移行の書き直しで開発版のサーバが起動しなくなる・形がずれる）→ ST54（R11）
