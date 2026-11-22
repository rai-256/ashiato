# ST61 の前提（外の形式・振る舞い）

問いを立てる前に確かめたもの。確かさは story-upstream の 1〜5（1 言っただけ / 2 出所を示した / 3 辿った / 4 走らせた / 5 実物で再現した）。
2026-11-22 に確かめた。

**失われるもの（`loss`）のある問いは無い**（deep-review の R3・R4 で分け直した）。初めは「番号の組み立てをいつまで残すか」（`discarded`）と
「携帯の versionCode の式」（`exported`）を A にしたが、前者は失わない側（全部残す）が安く範囲の中なので C1 に、後者は式を後で変えても
それまでの最大値より上から始められる（値を下げずに済む）ので B（Q2）にした。

| # | 前提 | 何が依るか | 確かめ方 | 結果 | 出所 | 確かさ |
|---|---|---|---|---|---|---|
| P1 | CI の成果物（Actions の artifact）は、このリポジトリでは最長 90 日で消え、延ばせない | Q1（CI の成果物のままでは、90 日より前の番号の組み立てを後で札に付けられない） | `gh api repos/rai-256/ashiato/actions/permissions/artifact-and-log-retention` | `{"days":90,"maximum_allowed_days":90}` | 実物 | 5 |
| P2 | main への merge は 2026-10-21〜11-21 の 30 日で 115 回。そのうち製品のコードに触れたもの（`tools/release/changes.sh code` が真）は 27 回 | Q3・Q4（番号の進む速さ）と Q2（桁の上限に何か月で届くか）・Q1（残す量） | `git log origin/main --first-parent --merges --since=2026-10-21` の各 merge の差分を `tools/release/changes.sh code` に通した | 全 115・コード 27。枝の内訳は docs/archive 41・docs/ch 23・verify 21・docs/st 18・chore 5・docs/req 4・fix 2・docs 1 | 実物（git） | 5 |
| P3 | Story・change の実装の枝（`feat/st*`・`feat/ch*`）は main へ直に merge されず、確認バッチの枝（`verify/<日付>`）に束ねて merge される | Q3（「Story が入ったら」を merge の枝で見分けられるか） | P2 と同じ一覧で `feat/st` を含む merge を数えた（全期間） | 0 回。コードに触れた 27 回のうち 21 回が `verify/*`、残りは `fix/*`・`chore/*` | 実物（git） | 5 |
| P4 | 番号 1 つ分の組み立て（サーバ・PC・携帯をまとめた `release-package`）は約 62 MB | Q1（全部残すときの量） | `gh api 'repos/rai-256/ashiato/actions/artifacts?name=release-package&per_page=3'` | 61,802,226 / 61,802,213 / 61,682,641 バイト（2026-11-20〜21 の verify の枝） | 実物 | 5 |
| P5 | 下書きのリリースは、認証なしのリリースの一覧に出ない | 番号ごとの組み立ての置き場（design の D（仮））を、更新の確認（`/releases?per_page=20`）が「更新あり」と取り違えない | 公式文書 https://docs.github.com/en/rest/releases/releases#list-releases（"Only users with push access will receive listings for draft releases."）。ST57 の premises P5 と同じ | 出ない | 公式文書 | 2 |
| P6 | Android は、入っている versionCode より小さい APK を上書きで入れない（入れ直しになる） | Q2（配った後に式を変えて値が下がると、利用者全員が入れ直す） | 正典 `openspec/specs/distribution/spec.md:606-609`（「この式を…版をまたいで変えない」）と ST58 の計測の台本 `tools/android-st58-replace.sh:60-66`（大きい値への上書きを確かめる）。小さい値を断ることそのものは走らせていない | 大きい値への上書きは走らせて確かめ済み。小さい値の拒否は公式文書（https://developer.android.com/studio/publish/versioning）と正典が前提にしている | 実物（repo）/ 公式文書 | 3 |
| P7 | Android の versionCode の上限は 2,100,000,000 | Q2（桁を広げた式で左の数字をいくつまで持てるか） | 公式文書 https://developer.android.com/studio/publish/versioning | 2,100,000,000 | 公式文書 | 2 |
| P8 | いまの携帯の versionCode の式は `左×1,000,000 + 真ん中×10,000 + 右×100 + 末尾` で、真ん中・右が 99 を超えると組み立てが落ちる | Q2 | `tools/release/version.sh:30-40`・`collector-android/app/build.gradle.kts:38-48`・`tools/release/version-table.tsv` | そのとおり。末尾は正式版 99・ベータ版 N・開発版 0 | 実物（repo） | 2 |
| P9 | 配布元（GitHub Releases）には、まだリリースも印（`v*`・`nightly`）も 1 つも無い | Q2（配った APK が無い今は式を変えても入れ直しが起きない）・「移る間」の D（仮） | `gh release list`・`gh api repos/rai-256/ashiato/tags` | どちらも空 | 実物 | 5 |
| P10 | 本人の携帯の収集アプリは debug の鍵の組み立て（0.1.x） | Q2・Q5（正式な鍵の版へは鍵が違うので入れ直しがどのみち要る —— ST64） | `docs/stories/ST64.md` の価値（「携帯は debug の鍵の組み立て」） | そのとおり（本人の言葉。端末の versionCode は確かめていない） | 実物（repo の文書） | 2 |
| P11 | リリースの配布物の容量の上限（1 ファイル・全体） | C1 の「全部残す」の代償（どの決定もこれに載せない） | 確かめていない | —— | —— | 1 |
| P12 | main への push の CI は、次の merge の push に取り消される（`concurrency: ci-${{ github.ref }}`・`cancel-in-progress: true`） | Q1（merge ごとに組む形だと、確認バッチの merge の組み立てがほぼ完走しない） | `.github/workflows/ci.yml:5-8`。`gh run list --workflow ci --branch main --event push --limit 120`（2026-10-17〜11-21。deep-review R1） | success 66・cancelled 36・failure 18。取り消し 36 回のうち 22 回は製品のコードに触れる merge。確認バッチの merge（#361・#326 など 11 回）は直後の archive の merge に取り消された | 実物 | 5 |
| P13 | Actions の concurrency は、同じ group で待てる run を 1 つだけ持ち、新しい run が来ると前に待っていた run を取り消す | Q1 の「merge 1 つずつ組む」案（取り消しを止めても、3 本以上続く merge の途中が消える） | 公式文書 https://docs.github.com/en/actions/using-jobs/using-concurrency | そのとおり | 公式文書 | 2 |
| P14 | 組み立ての job（package-linux・package-windows）は浅い checkout（`fetch-depth` 無し = 1） | 番号付けが履歴（first-parent の順・前の番号からの merge）を読むなら、番号付けの job は深い checkout が要る（ST52 Q12 と同じ型） | `.github/workflows/ci.yml:32,72,108,737` に `fetch-depth` があり、組み立ての job（462-503・540-）には無い | 浅い | 実物（repo） | 2 |
| P15 | インストーラは、入っている版が `0.0.0` なら「版を読めない古いサーバ」の互換値とみなし、系統の検査を飛ばして更新する | Q4（番号を持たない組み立てに 0.0.0 は使えない） | `crates/installer/src/update_plan.rs:14-25`（deep-review R5） | そのとおり。版の大小は見ず、違えば更新・同じなら走らせ直し | 実物（repo） | 2 |
| P16 | main と PR の組み立ての携帯の APK は debug の鍵で署名して残る。正式な鍵の署名（`sign-apk.sh`）の試験は署名していない APK だけを入力にする | C6（番号ごとに残す APK に、後で組み直さずに正式な鍵を足せるか） | `tools/release/package.sh:16-17`・`collector-android/app/build.gradle.kts:256-259`・`tools/release/sign-apk.sh` の自己試験（deep-review R7） | そのとおり。debug の鍵で署名済みの APK を署名し直す道は試していない | 実物（repo） | 2 |
| P17 | CI の更新の試験の組み立ては、同じ commit から 0.4.0・0.5.0・0.6.0 などを名乗って組む（main の run でも） | Q4（本物の番号がそこに届くと、同じ番号で中身の違う物ができる） | `.github/workflows/ci.yml:576-600` | そのとおり | 実物（repo） | 2 |
| P18 | job の単位の `concurrency` は、その job の `if` が偽で飛ぶ run では順番待ちの席を取らない | D10（取り消された・落ちた CI の run が、待っている番号付けを取り消さない） | 確かめていない（公式文書 https://docs.github.com/en/actions/using-jobs/using-concurrency は job の単位の組を言うが、飛ぶ job の扱いを書いていない）。最初の本物の番号付けで見る（tasks 12.5） | —— | 知識 | 1 |
| P19 | workflow の `GITHUB_TOKEN`（`contents: write`）で、印 `build-*` を push でき、同じ名前の印が別の commit を指していれば push は断られる | D7（番号の取り合いを印の push が断る） | 公式文書 https://docs.github.com/en/actions/security-for-github-actions/security-guides/automatic-token-authentication（`contents: write` で push できる）。断られる側は git の振る舞い（既にある印の付け替えは `--force` が要る） | push できる・付け替えは断られる | 公式文書 / 知識 | 2 |
| P20 | 既にある印の名前で下書きのリリースを作れる（`gh release create <印> --draft --target <commit>`） | D1・D7（印 `build-<番号>` の下書き） | 公式文書 https://docs.github.com/en/rest/releases/releases#create-a-release（`tag_name` が既にあればその印を使う） | 作れる | 公式文書 | 2 |
| P21 | Actions の API `GET /repos/{owner}/{repo}/actions/workflows/ci.yml/runs?branch=main&event=push&status=success` は、新しい run から順に返す | D8（前に成功した main の run の commit） | 公式文書 https://docs.github.com/en/rest/actions/workflow-runs#list-workflow-runs-for-a-workflow | 新しい順（読み。走らせていない） | 公式文書 | 2 |
| P22 | Android は、入っている APK と同じ versionCode の APK を上書きで入れる（下げるのだけを断る） | D3（同じ番号のベータ版と正式版が同じ値） | 公式文書 https://developer.android.com/studio/publish/versioning の読み。走らせていない | 入れる | 公式文書 | 2 |

### Q2 の前提を 4 に上げる

P6 の「小さい versionCode を断る」は、エミュレータ（`/dev/kvm` が戻ったら）で `adb install -r` に小さい値の APK を渡して
`INSTALL_FAILED_VERSION_DOWNGRADE` を見れば 4 になる。Q2 の推奨（桁を広げる）は、今の値（0.1.1 → 10,199）より大きい値から始まり、
P7 の上限の内側に収まるので、P6 が外れても推奨の側では何も失わない。外れて困るのは「今の式のまま」を選んで後で変える側だけ。

P5（下書きの見え方）は、下流の Task で使い捨ての下書き 1 つと認証なしの `curl` で 4 に上げる（deep-review R9）。C1 の置き場だけがこれに依り、外れたら置き場を変える（失うものは無い）。

## 聞かずに決めるもの（C。spec の段で design の D（仮）にし、反転条件を書く）

spec の段で design に置く D 番号（review/deep.md の処置と provisional.jsonl が指す）: **D1 = C1・D2 = C6・D3 = Q2 の答え・D4 = Q1 の答え**。

| # | 決めること | 既定（扉を開けたままにする側） | なぜ聞かないか | 反転条件 |
|---|---|---|---|---|
| C1 | 番号を付けた組み立てをいつまで残すか | **全部残す**。番号ごとに配布元の下書きのリリースに置く（携帯・PC・サーバの配布物と、組み立ての印） | 失わない側が安い（月に約 1.7 GB・公開のリポジトリで置き場の代金が無い —— R3）。消し方は、最初に消す必要が出る前に、札を作る ST62 で決められる | 置き場の容量の上限に当たる（P11）・下書きが認証なしの一覧に出る（P5 が外れる） |
| C2 | 番号の記録 | 付けたときに記録し（★ spec の段で、印 `build-<番号>` に変えた。下書きの一覧は push できる人にしか見えないので、判定を git だけで閉じる —— design D7）、後から数え直さない。次の番号は記録の最大から決める | 台帳は追記のみ。規則（Q3）を後で変えても過去の番号は動かない | —— |
| C3 | 最初の番号 | **0.2.0**（Q3 の推奨なら最初の確認バッチで 0.2.0 になる） | どちらも 0.1.1 より大きく、失うものが無い（R10）。0.1.2 だと versionCode が今の APK より小さくなりうる | —— |
| C4 | 左の数字の置き場 | repo のファイル 1 つ（本人が PR で変える）。API の契約の `info.version` は版番号にしない（main に commit を積まないため） | 置き場は後で移せる | —— |
| C5 | 番号付けの job | 深い checkout（P14）・main の first-parent の順・同じ commit の再実行は同じ番号で、置いた組み立てを上書きしない | 同じ番号で中身の違う物を作らない（FR-134）の実装 | —— |
| C6 | 番号ごとに残す携帯の APK の署名 | **署名していない APK も一緒に残す**（debug の鍵のものは本人が試しに入れる用） | 正式な鍵をいつ足すかは ST62 の未決（FR-135）。署名していないものを残せば、どちらに決めても組み直さずに済む（R7） | ST62 が main の組み立てで正式な鍵を付けると決めたら、残すのは署名済みだけでよい |
| C7 | ST62・ST63 までの移る間の印 `v*` と開発版 | 印 `v<番号>`（と `-beta.N`）は、**その番号を付けた commit を指すときだけ**組んで公開する。開発版の作り直しは、HEAD でなく**最新の番号を付けた commit** を `<番号>-dev.<日付>` で組む | 今の正式版・開発版の組み立てを緑のまま保つ（壊してはいけないもの）。番号と中身がずれない（R8） | —— |
| C8 | Windows の組み立ての使い回し | 使い回しの鍵に版番号を含める（★ spec の段で、番号の組み立てはキャッシュを引かず保存もしない形に変えた。鍵は変えない —— design D12） | 番号の違う組み立てを取り違えない（R6） | —— |
