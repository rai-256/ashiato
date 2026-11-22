# ST61 深掘りの問いの独立検査（deep-review）

対象: `openspec/changes/st61-auto-version-bump/deep-questions.json`（A: Q1・Q2、B: Q3〜Q6）と `premises.md`（P1〜P11）。呼び出し元が示した C（問わずに design の D（仮）にする予定）7 件も、問いと同じように確かめた。
手順 1〜8 は、問いの一覧を見る前にやり直した。突き合わせはその後にした。依頼 #352 の「議論で決めたこと」は本人の決定として扱い、聞き直しの対象にしていない。

## 手順ごとの記録

- 手順 1（要件どうしの衝突）: FR-134 は「main への merge のたびに、番号を 1 つ進めた組み立てを作る」と書く。今の CI は、main への push の run を次の push が取り消す。C の「祖先の番号付けをしない」と Q4 の推奨を合わせると、確認バッチの merge にはほぼ番号が付かない → R1。FR-134 の versionCode の補足（C4）と正典の「式を変えない」の食い違いは、Q2 が扱っている（ただし分類が違う → R4）。FR-104・NFR-25 と FR-134 のあいだに、ほかの衝突は無い（確かめた範囲: requirements.md:964-983・1177-1183・1381-1386）
- 手順 2（扉の幅）: 扉 34（識別子と署名鍵）は決定済み。ただしこの Story の文脈では「番号ごとに残す携帯の APK を、どの署名の状態で残すか」の幅がある。これは ST62 の未決（いつ署名するか）を先に閉じうる → R7
- 手順 3（新たに立つ一方通行）: 消すもの（番号の組み立て）は Q1 が扱う。ただし失わない側が安いので A ではない → R3。versionCode の式は導出の規則なので、後から変えても戻せる → R4。外に出すもの: ST61 自身は公開しない（下書きと、公開の設定の無い開発版。P9・K3）。ただし移る間の印 `v*` は外に出る。そこで番号と中身がずれる道がある → R8。列の型・保存形式: 組み立ての印 `<版番号>+<commit>` の形は今の形に含まれるので変わらない。ただし PR の組み立てに `0.0.0` を書くと、インストーラの既存の意味とぶつかる → R5
- 手順 4（日常）: 下書きは月に 27 個以上、本人の Releases の一覧にたまる（Q1 の量の問いに含まれる）。PR で組んだ物と手元で組んだ物を本人が入れる習慣（2026-11-19）に、Q6 の範囲が届いていない → R6
- 手順 5（既存コードと要件）: `ci.yml:5-8` の取り消し → R1。`changes.sh code` の判定は、製品の中身が変わったかを表していない → R2。Windows の組み立ての使い回しの鍵が番号を含まない → R6。CI の試験の組み立てが本物になりうる番号（0.4.0・0.5.0・0.6.0）を名乗る → R6。携帯の APK は debug の鍵で署名して残る → R7。組み立ての job は浅い checkout → R11。`installer` の `0.0.0` → R5
- 手順 6（過去の取りこぼし）: 次の観点が当てはまった
  - 「git の履歴を読む検査を変える決定では、CI の checkout の深さを premises に書く」（ST52 Q12）→ R11
  - 「キャッシュや前の成果物を使い回す問いでは、その置き場へ書ける者を全部挙げる」（CH17 Q4）→ R6
  - 「推奨に『失うものが無い』と書く前に、版・鍵を使う箇所をすべて辿る」（ST08 Q11）→ R5（Q6 の why）・R7
  - 「見積もり・根拠を実物で測っていない」→ R1（Q3・Q4 の回数は、番号になる回数ではなく merge の回数）
  - 時計の族は該当なし。番号の並びは first-parent の順で決まり、時刻を入力にしない（C）。ただし Q3 の「月が変わったら」は時刻を使う。B で既定もあるので R にはしない
- 手順 7（族の網羅）: Q4（文書だけの merge）の兄弟（取り消された run・落ちた run・道具だけの merge）→ R1・R2。Q6（PR の組み立て）の兄弟（手元の組み立て・CI の試験の組み立て・使い回しのキャッシュ）→ R6。Q1（消す）の兄弟（残した物が後で札に使えない形 = 署名）→ R7
- 手順 8（前提の確かさ）: Q2 の loss は P6（確かさ 3）に載る → R4 で loss ごと外す。Q1 は P11（確かさ 1）と P5（確かさ 2）に載る → R3・R9。C の「祖先の番号付けをしない」は GitHub の concurrency の振る舞いに載るが、premises に無い → R1。組み立ての checkout の深さも premises に無い → R11
- 説明の無い言葉: 大きなものは無い。Q2 の harm.unknown の「（P6）」と、harm.link の「扉 34」は内輪の言葉。どちらも括弧で言い換えてあるので R にはしない
- 推奨の有無: Q3〜Q6 のどれにも `recommended` がある

---

## R1. main への push の CI は次の merge に取り消されていて、確認バッチの merge はほぼ全部 run を完走していない。C の「祖先の番号付けをしない」と Q4 の推奨を合わせると、Story のコードを持つ merge に番号が付かない
- 成果物: openspec/changes/st61-auto-version-bump/deep-questions.json（Q3・Q4）・呼び出し元の C「同じ merge 群の並び（main の first-parent 順、祖先の番号付けをしない）」
- 根拠:
  - `.github/workflows/ci.yml:5-8`（`concurrency: group: ci-${{ github.ref }}`・`cancel-in-progress: true`）。main への push はどれも同じ group（`refs/heads/main`）に入る
  - `gh run list --workflow ci --branch main --event push --limit 120`（2026-10-17〜11-21）: success 66・cancelled 36・failure 18。取り消された 36 回のうち 22 回は、`tools/release/changes.sh code` が真になる merge（製品のコードに触れる merge）
  - 取り消された確認バッチの merge と、取り消しの元になった直後の merge: #361 の run（04:09:38〜04:15:39 cancelled）← #362 archive（04:15:21）。#326（11:34:58〜11:41:56）← #327 archive（11:41:38）。#350・#320・#301・#293・#286・#281・#256・#250・#232 も同じ形で cancelled。後ろの archive の merge は、どれも文書だけ
  - GitHub の concurrency の振る舞い（公式文書 https://docs.github.com/en/actions/using-jobs/using-concurrency）: 同じ group で待てる run は 1 つだけ。新しい run が来ると、前に待っていた run は取り消される。だから `cancel-in-progress: false` にしても、3 本以上続く merge（#361→#362→#363→#364）では途中の run が消える
- kind: conflict
- 提案: Q4 の族として B の問いを足す。問いは「run を完走しなかったコードの merge（次の merge に取り消された・落ちた）の番号をどうするか」。選択肢は次の 2 つ。
  1. 次の run が、文書だけの merge でも、番号の無い最新のコードの commit を組んで番号を付ける（C の「祖先の番号付けをしない」を外す）
  2. 番号付けを ci.yml の取り消される group から出し、別の workflow で 1 本ずつ流す

  C の「祖先の番号付けをしない」は、このままだと FR-134 の「merge のたびに」と食い違う。要件を変えずに D（仮）にはできない。問いに上げること。
  premises に 2 行を足す。1 つは main の run の取り消しの実測（上の数）、もう 1 つは concurrency で待てる run は 1 つの公式文書。
  Q3 と Q4 の「30 日で 27 回・確認バッチ 21 回」は merge の回数で、今の CI では番号になる回数ではない。そのことを context に書く。Q3 の gain「真ん中が上がった番号 = 本人が確かめた機能が入った組み立て」は、上の 1 か 2 が決まらないと成り立たない
- 処置: fixed D4 仮 —— 文書だけの merge の問いと合わせて Q1（kind: conflict）「main のどの commit に番号を付けるか」にして本人に聞く（第 1 回は推奨で先に書く）。spec の段で design の D4（仮）に Q1 の答えと反転条件を書く。推奨は「CI が通り、前の番号から製品が変わった commit に付ける」（取り消された merge の中身は次の番号に入る）。premises に P12・P13 を足し、Q3 を「前の番号からの間に確認バッチがあれば」に直した

## R2. Q4 の「コードに触れる merge だけ進める」の gain「番号ごとに中身が違う」は誤り。判定（`changes.sh code`）は、道具・CI・試験だけの merge もコードに数える。落ちた main の run の番号という兄弟の場合も一覧に無い
- 成果物: openspec/changes/st61-auto-version-bump/deep-questions.json（Q4）
- 根拠:
  - `tools/release/changes.sh:6-11`（`docs/`・`openspec/`・`*.md` 以外は全部コードとみなす）
  - 30 日のコードの merge 27 回の差分（`git diff --name-only <m>^1 <m>`）では、次の merge が製品の部品（crates・collector-android・web・migrations）に触れていない: #286（`tools/workbox.json` だけ）・#280（`tools/lanes.json` だけ）・#305（`harness.json`）・#152（`.gitleaksignore`）・#350（`.github/workflows/vulns-nightly.yml` と `tools/` だけ）
  - 上の `gh run list` の failure 18 回（うち 3 回はコードの merge。#265 など）
- kind: technical
- 提案:
  - Q4 の gain を「番号はおおむね製品の変わり目に付く。道具・CI・試験だけの merge にも付く」に直す
  - points の「文書だけの merge で組むと、中身が同じ組み立てに別の番号が付く」に、コードの merge でも同じことが起きると足す
  - 兄弟の場合「main の run が落ちた merge は番号を使うか（番号を決めるのは組み立てが通った後か）」を、R1 で足す問いか D（仮）に入れる
  - Q4 の推奨は FR-134 の本文を書き換える。B は答えが無くても推奨で進むので、答えが無いまま要件が変わる。context に「答えが無い間は要件を直さず、design の D（仮）の範囲で扱う」か、その逆かを書く
- 処置: fixed deep-questions.json —— Q1 の points と context に「道具・CI・試験だけの変更にも番号が付く」を書き、gain の「番号ごとに中身が違う」を消した。落ちた run の場合は Q1 の推奨（CI が通った commit に付ける）に含めた。要件の本文は記録の段で ★ 訂正で直すと context に書いた（第 1 回は GO と 1 枚で本人が見る）

## R3. Q1 は A ではなく P。失わない側（全部残す）は安く、範囲を越えず、要件を変えない。消し方は、最初に消える 90 日後より前に、札を作る ST62 で決められる
- 成果物: openspec/changes/st61-auto-version-bump/deep-questions.json（Q1）・premises.md（P5・P11）
- 根拠:
  - `gh repo view --json visibility` → `PUBLIC`。公開のリポジトリでは、リリースの添付ファイルの置き場に課金が無い
  - Q1 の推奨の cost は「月に約 1.7 GB」だけ（P2・P4。27×62 MB）
  - 置き場は C で下書きのリリースと決めてある
  - `.harness2/docs/flow-gates.md:275`「`loss` があっても、失わない側が高くつかず・範囲を越えず・要件を変えないなら P として仮で進む」
  - cost が載る P11（容量の上限）は確かさ 1。置き場の前提の P5（下書きは認証なしの一覧に出ない）は確かさ 2
- kind: irreversible
- loss: discarded
- 提案:
  - Q1 を問いから外し、「全部残す」を `provisional.py` の台帳に載せる D（仮）にする。消し方（90 日・30 個など）は ST62 の深掘りへ申し送る
  - P11 は GitHub の公式文書（リリースの 1 ファイルは 2 GiB 未満、全体の容量に上限は無い、の記述）で 2 に上げる
  - 残すなら、Q1 の gain の「戻したりできる」と cost に、公開のリポジトリでは置き場の代金が掛からないことを書く
- 処置: fixed D1 仮 —— 旧 Q1 を問いから外し、premises の C1（全部残す・下書きのリリース）にした。provisional.jsonl に D1 を載せた。spec の段で design に D1（仮）と反転条件（容量の上限・下書きが一覧に出る）を書く

## R4. Q2 の `loss: exported` は成り立たない。versionCode の式は導出の規則で、後で変えても、配った値より大きくなる式に切り替えられる
- 成果物: openspec/changes/st61-auto-version-bump/deep-questions.json（Q2）・premises.md（P6）
- 根拠:
  - 今の式（`tools/release/version.sh:30-40`・`collector-android/app/build.gradle.kts:38-48`）で 0.99.x が取る最大値は 99×10,000 + 99×100 + 99 = 999,999。広げた式（左×10,000,000 + 真ん中×10,000 + 右）の 0.100.0 は 1,000,000。真ん中が 100 に届く時に切り替えれば、値は下がらない
  - 一般に、新しい式に「それまでに配った最大値」を足せば、値は必ず上がる。上限は 2,100,000,000（P7）
  - 「今の式のまま」の irreversible「配った後に桁が足りなくなり式を変えると、値が下がって上書きできない」は、切り替える時と式を選べば起きない
  - 仮に loss があっても、失わない側（桁を広げる）の cost は「試験・対応表・正典の Requirement を書き直す」で、この Story の範囲に入る。FR-134 の ★ が「式を変えるかを決める」をこの Story に向けている（requirements.md:1182-1183）
  - loss を支える P6 は確かさ 3（小さい値を拒むことを走らせていない）
- kind: daily
- 提案:
  - Q2 から `loss` を外し、B（推奨: 桁を広げる）に落とすか、D（仮）にする
  - 「今の式のまま」の irreversible の文は消す。代わりに cost を「真ん中か右が 100 に届いた日の組み立てが落ち、その時に式を変える（変える時を選べば値は下がらない）」にする
  - P6 を確かさ 4 に上げる台本（premises の「Q2 の前提を 4 に上げる」）は、loss を外せば要らない
- 処置: fixed D3 仮 —— kind を irreversible から daily に直した（この指摘自身が loss は成り立たないとした）。Q2 の loss を外して B にし、「今の式のまま」の irreversible を cost に直した。spec の段で design の D3（仮）に Q2 の答えと反転条件を書く

## R5. Q6 の推奨「0.0.0」は、インストーラの既存の意味（「版を読めない古いサーバ」の互換値。系統の検査を飛ばす）とぶつかる。cost の「古い版として扱われる」は事実と違う
- 成果物: openspec/changes/st61-auto-version-bump/deep-questions.json（Q6）
- 根拠:
  - `crates/installer/src/update_plan.rs:13-18`: `installed == "0.0.0"` なら系統（開発版か正式版か）を見ずに `Update`。`crates/installer/src/main.rs:1192-1196` は、ST57 より前のサーバの互換値として 0.0.0 を返す。`crates/installer/src/lib.rs:28` の試験（`version_action("0.0.0", "0.5.0-dev.20261116") == Update`）が固定している
  - だから、PR の組み立て（0.0.0）を入れた PC では、次に入れる開発版が正式版の系統の上へ「更新」として進む
  - 入っている版と比べる時も、`update_plan.rs:19-25` は順序を見ない（同じ系統で版が違えば `Update`）。0.2.x の上に 0.0.0 を入れても「古い版」としては扱われず、更新として進む
  - Q2 の推奨の式では 0.0.0 の versionCode は 0×10,000,000 + 0 + 0 = 0。P7 と同じ公式文書（https://developer.android.com/studio/publish/versioning）は、versionCode を正の整数とする。0 で組めるか・入れられるかは確かめていない
- kind: premise
- 提案:
  - Q6 の推奨を、0.0.0 以外の、インストーラの互換値とぶつからない形にする。そうでなければ、互換値の意味を変える作業を Q6 の cost に書く
  - cost の「古い版として扱われる」を「今のインストーラは順序を見ないので、本物の番号の上にも更新として入る（止めるのは ST64）」に直す
  - why の「後から値を変えても…戻せる」の前に、入れた PC の更新の履歴に `0.0.0+<commit>` が追記で残ることを書く
  - versionCode が 0 になる組み合わせ（Q2 × Q6）を premises に 1 行足して確かめる
- 処置: fixed deep-questions.json —— Q4 の推奨を「本物に使わない 0.0.x（1 以上）」に直し、0.0.0 が使えない理由と、インストーラが版の大小を見ないこと（premises P15）を書いた

## R6. 「番号を持たない組み立て」の族が PR だけで、兄弟が抜けている。抜けているのは、手元の組み立て・CI の試験の組み立て（本物と同じ 0.4.0・0.5.0・0.6.0 を名乗る）・Windows の使い回しのキャッシュ（鍵に番号が入らない）の 3 つ
- 成果物: openspec/changes/st61-auto-version-bump/deep-questions.json（Q6）
- 根拠:
  - CI の試験の組み立て: `.github/workflows/ci.yml:579-584` は、更新の試験の zip を出所の版 `0.4.0`・`0.5.0`・`0.4.1`・`0.6.0` で組み、artifact に上げる（`ci.yml:588-600`）。main への push の run でも組む
  - Q3 の推奨（確認バッチごとに真ん中を上げる。月に約 21）と Q5 の推奨（0.2.0 から）なら、本物の番号は数週間で 0.4.x・0.5.x に届く。そうなると、同じ番号で中身の違う組み立てを CI が作る（FR-134「同じ番号で中身の違う組み立てを作らない」）
  - 手元の組み立て: `tools/release/build.sh` を版番号を渡さずに走らせると、出所の版で組む。`tools/check-build-stamp.sh:250-254` の no-env の組み立ても同じ。2026-11-19 は、本人が CI の外の組み立ても入れていた（FR-137 ★ の 0.1.0+de49710d1）
  - 使い回しのキャッシュ: `tools/release/windows-build-key.sh:8-24`（鍵 = 木の ls-tree と道具の版。外から渡す版番号は入らない）・`ci.yml:534-535`（引くのは PR の run だけ）・`ci.yml:614-615`（保存は event を問わない。main への push の run も保存する）。番号を環境変数で渡すと、main で組んだ番号付きの組み立てを、同じ鍵の PR が自分の組み立てとして引ける
- kind: technical
- 提案:
  - Q6 の decide を「PR・手元・CI の試験の組み立て（main の番号付けの外で組むもの全部）に何を書くか」に広げる
  - CI の試験の組み立ての版は、本物の番号の列と重ならない値にすることを D（仮）に置く
  - 使い回しの鍵に番号を入れるか、main への push では保存しないかを D（仮）に置く
- 処置: fixed deep-questions.json —— Q4 の範囲を PR・手元・CI の試験の組み立てに広げた（試験は 0.0.2 から）。Windows の使い回しの鍵は premises の C8（鍵に版番号を含める）

## R7. 扉 34 の幅: 番号ごとに残す携帯の APK の署名の状態が決まっていない。今の main の組み立ては debug の鍵で署名して残す。札を付けるときに正式な鍵で署名し直す道は、試されていない
- 成果物: openspec/changes/st61-auto-version-bump/deep-questions.json（一覧に無い）
- 根拠:
  - `tools/release/package.sh:17`（main と PR の組み立ては `ASHIATO_APK_UNSIGNED` を付けずに `apk` を組む）
  - `collector-android/app/build.gradle.kts:256-259`（UNSIGNED でなければ debug の署名）
  - `release.yml` は署名しないで組み（`ASHIATO_APK_UNSIGNED: "1"`）、publish の job で `sign-apk.sh` が署名する
  - `tools/release/sign-apk.sh` の自己試験は署名していない APK だけを入力にする（`official-unsigned.apk`・`dev-unsigned.apk`）。debug の鍵で署名済みの APK を署名し直す道は試していない
  - FR-135 の未決（requirements.md:1187）は「main の組み立てで付けるか、札を付けるときに足すか」で、期限は ST62
- kind: irreversible
- loss: discarded
- 提案:
  - debug の鍵の APK だけを残すと、ST61 から ST62 までに付いた番号には、組み直さずに正式な鍵を足せないかもしれない。そうなると、Q1 と同じく「札を付けられない番号」になる
  - 失わない側（署名していない APK も一緒に残す。debug の鍵のものは本人が試しに入れる用）は安く、範囲の中に収まる。D（仮）にして台帳に載せる
  - 「debug の鍵で署名済みの APK を正式な鍵で署名し直せるか」を premises に足す
- 処置: fixed D2 仮 —— premises の C6（署名していない APK も一緒に残す）と P16 を足し、provisional.jsonl に D2 を載せた。spec の段で design に D2（仮）と反転条件（ST62 が main で正式な鍵を付けると決めたら）を書く

## R8. C の「移る間の印 `v*` と開発版の作り直しは、今の形を最新の番号で動かし続ける」は、番号と中身がずれる道を開く。`v*` が付いた commit・開発版で組む commit が、その番号の commit と同じであることが書かれていない
- 成果物: 呼び出し元の C（design の D（仮）にする予定のもの）
- 根拠:
  - `.github/workflows/release.yml:2-4,14-20`: 印 `v*` の push で、印の付いた commit を組み直す
  - `tools/release/version.sh` の `plan` は、印の版を出所の版（`check_version`）とだけ比べる。印の commit がその番号を付けた commit かは見ない
  - `nightly_plan` は main の HEAD を `$source-dev.$date` で組む
  - R1 のとおり、HEAD は「番号の無いコードの merge」を含みうる。そのとき「最新の番号」で組むと、開発版の 0.2.7-dev は 0.2.7 と中身が違う
  - FR-107（requirements.md:994）は「最も新しい番号を開発版向けに組み」で、HEAD ではなく番号の commit を組むと読める
  - 印 `v*` は配った版として凍り、外に出る（distribution spec.md:813-830）
- kind: technical
- 提案: C の文を次のように直す。
  - `v<番号>` の印は、その番号を付けた commit を指すときだけ組んで公開する。違えば止める
  - 開発版は、最新の番号を付けた commit を組む（HEAD ではない）

  どちらも version.sh の試験の Scenario に置く
- 処置: fixed premises.md —— C7 を「印 v<番号> はその番号を付けた commit を指すときだけ組む・開発版は最新の番号を付けた commit を組む」に直した

## R9. 番号の置き場（下書き）を支える P5「下書きは認証なしの一覧に出ない」は確かさ 2 で、安く 4 にできる。外れると、全部の部品の更新の確認が下書きを「新しい版」と読む
- 成果物: openspec/changes/st61-auto-version-bump/premises.md（P5）・Q1 の context
- 根拠:
  - P5 の確かめ方は公式文書だけ（確かさ 2）
  - 更新の確認の道は `/releases?per_page=20`（distribution spec.md:2515-2596）
  - 下書きが見えれば、月に 27 個以上の下書きが 20 件の窓を埋める。本物の札の付いた番号が窓から押し出されるか、下書きが「更新あり」になる
- kind: premise
- 提案: 実物で走らせて P5 を確かさ 4 にする。手順は、使い捨ての下書きを 1 つ作り、認証なしの `curl https://api.github.com/repos/<owner>/<repo>/releases?per_page=20` に出ないことを見て、消す。D（仮）の台帳の rests_on に P5 を入れる
- 処置: fixed premises.md —— P5 を 4 に上げる確かめ（使い捨ての下書きと認証なしの curl）を下流の Task に置くと書いた。依るのは C1 の置き場だけで、外れても失うものは無い

## R10. Q5 は人間に問う重みがない。どちらも 0.1.1 より大きく、Q3 の推奨なら最初の確認バッチで 0.2.0 になる。0.1.2 の側には versionCode が今の APK より小さくなることが書かれていない
- 成果物: openspec/changes/st61-auto-version-bump/deep-questions.json（Q5）
- 根拠:
  - Q5 の why「どちらを選んでも失うものは無い」
  - Q3 の推奨（確認バッチの merge で真ん中を上げ、右を 0 に戻す）なら、0.1.2 から始めても、最初の確認バッチの merge で 0.2.0 になる
  - Q2 の推奨の式では 0.1.2 → 10,002。今の式の 0.1.0 は 10,099、0.1.1 は 10,199
- kind: defer
- 提案:
  - Q5 は C（0.2.0 から）にして、問いから外す
  - 残すなら「0.1.2 から」の cost に、Q2 の推奨の式では 0.1.2〜0.1.98 の versionCode が今の APK（0.1.0・0.1.1）より小さいことを書く。本人の携帯は鍵が違い、どのみち入れ直すので、失うものは無い
- 処置: fixed premises.md —— 旧 Q5 を問いから外し、C3（0.2.0 から）にした

## R11. 番号付けが git の履歴（first-parent の順・祖先かどうか）を読むのに、組み立ての job の checkout の深さが premises に無い。今の組み立ての job は浅い checkout
- 成果物: openspec/changes/st61-auto-version-bump/premises.md
- 根拠:
  - `.github/workflows/ci.yml:462-471`・`:496-503`（package-linux・package-windows の checkout に `fetch-depth` が無い。既定は深さ 1）
  - `tools/release/version.sh` の `require_full_history`（浅い木では止める）
  - C の「main の first-parent 順、祖先の番号付けをしない」は、祖先かどうかの判定を要る
  - 過去の取りこぼし ST52 Q12（浅い checkout で検査が黙って通った）
- kind: premise
- 提案:
  - premises に「番号付けを走らせる job の checkout の深さ」の行を足す
  - tasks には `git clone --depth=1` の木で番号付けの台本を走らせ、止まることを確かめる項目を置く
- 処置: fixed premises.md —— P14（組み立ての job は浅い checkout）と C5（番号付けの job は深い checkout）を足した

