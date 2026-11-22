# ST61 spec の独立検査（spec review）

対象: `openspec/changes/st61-auto-version-bump/`（第 1 回の先読み。deep.md の Q1〜Q4 は答え待ちで、推奨の答えで書いた版）。
読んだもの: proposal.md・specs/distribution/spec.md・design.md・tasks.md・deep.md・premises.md・review/deep.md・provisional.jsonl・deep-questions.json・
`docs/stories/ST61.md`・`docs/stories/INDEX.md`・`docs/requirements.md`（FR-104・FR-105・FR-107・FR-134・FR-135〜137・NFR-25）・正典 `openspec/specs/distribution/spec.md`・
突き合わせに `.github/workflows/{ci,release,nightly}.yml`・`tools/release/{changes,draft,check-artifacts,update-apply-install}.sh`。

## 機械の検査（先に写す）

| コマンド | 結果 |
|---|---|
| `openspec validate st61-auto-version-bump --strict` | `Change 'st61-auto-version-bump' is valid` |
| `python3 scripts/check_chain.py .` | `chain: OK (0 件 / 未回収 0 件 / warn 0 件)`。観点 8（再生成との一致）を含めて通る |
| `python3 scripts/check_scenarios.py . st61-auto-version-bump` | `scenarios: FAIL (担保なし 42 件 / 名無しの確認待ち 0 件)`。42 件は、この change が足した、まだ実装の無い Scenario。42 件とも tasks.md の Task が一字一句で名指ししている（`grep -F` で全件を確かめた）。上流の段では予定どおりの FAIL |
| `python3 scripts/review_triage.py . st61-auto-version-bump` | `triage: OK`（指摘 11 件 / 仮決め 4 件 / 要件へ戻すもの 0 件）。R11 の「要件へ戻すもの 0 件」を参照 |
| `python3 scripts/spec_overlap.py candidates . st61-auto-version-bump` | `[]`（候補 0 件） |

参考: `provisional.py check` は D1・D2 の別の席の確かめが無いことで FAIL になる。確かめはグラフの provisional_review が後で書くので、指摘にしない。

---

## R1. 番号付けの同時実行の組（concurrency）を workflow の単位に置くと、条件で飛ばす run が待ちの席を取り、成功した run の番号付けを取り消す。「新しい方の commit の番号に古い方の中身が入る」が成り立たない
- 成果物: openspec/changes/st61-auto-version-bump/specs/distribution/spec.md（Requirement「番号付けは取り消されずに 1 本ずつ流れ…」:162）・design.md D10（:132-133）・tasks.md 5.1（:79-80）
- 根拠:
  - tasks 5.1 は `on: workflow_run`（`types: [completed]`・`branches: [main]`）を受け、`concurrency: { group: number, cancel-in-progress: false }` を workflow の単位に置く。成功・push・この repo の条件は judge の job の `if` で見る
  - premises P13（公式文書）: 同じ組で待てる run は 1 つだけで、新しい run が来ると、前に待っていた run は取り消される。workflow の単位の組には、job の `if` を評価する前に、run ごと入る（公式文書 https://docs.github.com/en/actions/using-jobs/using-concurrency の読み。確かさ 2。走らせていない）
  - premises P12: main の push の CI の run は 120 回のうち cancelled 36・failure 18。どちらも `completed` で終わるので、number.yml の run を起こす
  - 流れの例: 製品に触れた A の CI が成功する。前の番号付けが走っている間、A の番号付けは待つ。続く B の CI が取り消されるか落ちて、B の番号付けが起きる。このとき A の番号付けは取り消され、B は `if` で飛ぶ。A には番号が付かない。番号は、次に成功する run まで付かない
  - `branches: [main]` は起こした run の枝の名前だけを見る。fork の PR の枝の名前が `main` でも run は起き、同じように待ちの席を取る（公式文書 https://docs.github.com/en/actions/writing-workflows/choosing-when-your-workflow-runs/events-that-trigger-workflows#workflow_run の `branches` の読み。確かさ 2）
  - spec:162 は「待っている番号付けが新しいものに入れ替わったときは、新しい方の commit の番号に古い方の中身が入る」と言う。入れ替えた側が番号を付けない run なら、この文は偽になる。Scenario「番号付けは、CI と別の、取り消さない 1 本ずつの流れにある」は workflow を読んで組の有無を見るだけなので、この場合を落とさない
- kind: technical
- 提案: 組を、条件を通った run だけが入る単位に移す。たとえば条件を workflow の入口で判定して先へ進まない run を組に入れないか、組を judge の job 以降に置く。Scenario「番号付けを起こさない run は、待っている番号付けを取り消さない」を足す（偽の number.yml で、組が workflow の単位にあれば check-guards が落ちる）。spec:162 の文は、この形に合わせて直す。**spec を変える指摘**
- 処置: fixed D10 —— 同時に走らせない組を判定の job に移し、条件を満たさない run が順番待ちに入らないようにした。spec の本文を直し、Scenario「番号付けを起こさない run は、待っている番号付けを取り消さない」を足した（tasks 5.1）

## R2. 印を先に作る順（D7）を言う Scenario が無い。Scenario「番号を付けると…」は、印を作る要求を「置く手順」に入れていて、design・tasks と食い違う
- 成果物: specs/distribution/spec.md（Scenario「番号を付けると、下書き 1 つに配布物を置く要求だけを出す」:124-128）・design.md D7（:100）・D10（:134）・tasks.md 4.1（:66-69）
- 根拠:
  - Requirement の本文（:113）と D7・D10 は「番号を決めたら**組む前に**、judge の job で印 `build-<番号>` を push する」と言う。tasks 4.1 は `number.sh tag` と `number.sh place` を分け、tag は judge の job が呼ぶ（5.1）
  - Scenario :127 は、置く手順（place）が「印 `build-0.2.1` を作る要求と、下書きを作る要求」を出すと言う。place が印を作る実装でも通り、tag を先に呼ぶ実装はこの Scenario を満たさない
  - 「組み立てが落ちた番号は次の commit に回さない」（D7 :103）は、組む前に印があることで成り立つ。順を確かめる Scenario は無い（workflow を読む Scenario は、権限・キャッシュ・組・fetch-depth しか見ない）
- kind: technical
- 提案: Scenario を 2 つに分ける。1 つ目は「印を作る手順は、印 `build-<番号>` を作る要求だけを出す（同じ commit なら出さない）」。2 つ目は「置く手順は、下書きを作り配布物を置く要求だけを出し、印を作る要求を出さない」。workflow を読む Scenario に「組み立ての job は印を作る job の後にある（`needs`）」を足す。**spec を変える指摘**
- 処置: fixed D7 —— Scenario を「番号の印を作る手順は、印を作る要求だけを出す」と「番号を付けると、下書き 1 つに配布物を置く要求だけを出す」（印を作る要求を出さない）に分け、workflow の Scenario に「組み立ての job は印を作る job の後（needs）」を足した

## R3. 下書きの説明文の commit（`ashiato-build: <番号> <commit>`）を読む者がいない。印を消して作り直すと、前の commit の配布物が「揃っている」として残る
- 成果物: specs/distribution/spec.md（Requirement「番号を付けた組み立ては…上書きしない」:111-122、Scenario「同じ番号の下書きに配布物が揃っていれば、置き直さない」:135-138）・design.md D7（:100・:104）・Risks（:171）
- 根拠:
  - design の Risks :171 は「番号の印 `build-*` は消せる・動かせる」と認めている
  - 揃っているかの判定（Scenario :135-138・tasks 4.1 の `gh release view build-<番号>`）は、印の名前だけで下書きを引く。下書きの説明文の commit とも、下書きの `target_commitish` とも照らさない
  - 例: `build-0.2.1` を消して commit B に作り直す（手で・誤って）。A で組んだ配布物の揃った下書き `build-0.2.1` が残っていれば、place は何もしない。B を指す印の下に A の組み立てが残る。番号付けの判定が気づくのは、印と渡した commit が違うときだけ（:145-148）
- kind: technical
- 提案: place は、下書きの説明文の `ashiato-build:` の行の commit が渡した commit と同じかを照らす。違えば何も置かずに止まる。Scenario「下書きの説明文の commit が違えば、何も置かずに止まる」を足す。**spec を変える指摘**
- 処置: fixed D7 —— 置く手順が下書きの説明文の commit を照らし、違えば止まる文と Scenario「下書きの説明文の commit が違えば、何も置かずに止まる」を足した

## R4. 欠番の commit の番号付けを、後の番号が付いてから走らせ直したときの振る舞いが決まっていない。D4 の条件 2 と 4 が食い違う
- 成果物: design.md D4（:66-71）・specs/distribution/spec.md（Requirement 1 :9、Scenario「同じ commit の run を走らせ直しても、番号は変わらない」:41-44）
- 根拠:
  - D4 :71 は「番号の組み立てが落ちたら、番号付けの run を走らせ直して同じ番号の続きを置く」と言う
  - D4 の条件 2 は「前の番号の commit（印の付いた最後の commit）の子孫」を求め、条件 4 は「その commit に番号があれば、その番号を返して続きを置く」と言う。commit A の `build-0.2.1` の組み立てが落ちた後で B に `0.2.2` が付くと、A は最後の番号の commit の祖先になる。条件 2 では付けず、条件 4 では `0.2.1` を返す
  - Scenario :41-44 は、走らせ直す commit が最新の番号の commit の場合しか見ない。「祖先の run」の Scenario（:36-39）は、番号の無い祖先しか見ない
  - Scenario「組み立てが落ちた番号は、次の commit に回さない」（:101-104）も、A を走らせ直したときの振る舞いを言わない
- kind: technical
- 提案: どちらを先に見るかを決めて Requirement に書く。「既に印のある commit は、最新でなくてもその番号を返し、欠けた配布物を足す」なら、欠番を後から埋められる。逆なら D4 :71 の「走らせ直して続きを置く」に「次の番号が付く前に限る」を足す。どちらでも Scenario を 1 つ足す
- 処置: fixed D4 —— 既に番号の印がある commit は、最新でなくてもその番号を返して続きを置く側にした。本文と Scenario「欠番の commit を走らせ直すと、後の番号が付いていても、その番号を返す」を足した

## R5. 最初の番号の差分の基準が決まっていない。番号の印が 1 つも無いとき、「前の番号から製品が変わった」の比べる相手が無い
- 成果物: specs/distribution/spec.md（Requirement 1 :5、Requirement 2 :59、Scenario「最初の番号は 0.2.0」:71-74）・design.md D4（:67-68）・tasks.md 12.2（:134）
- 根拠:
  - Requirement 1（:5）と D4 の条件 2・3 は、どれも「前に番号を付けた commit」からの差分と子孫を見る。番号が 0 個のときの扱いは、Requirement 2 の (1)「`<左>.2.0`」が番号の値を決めるだけ
  - Scenario :71-74 の「製品に触れる merge の commit」は、何と比べて触れたのかを言わない（親との差分か、全部か）
  - 文書だけの merge が number.yml の入った後の最初の成功の run になると、実装によって `0.2.0` が付くか付かないかが変わる。この Story の merge 自身（確認バッチの merge。製品に触れる）に付くのかも決まらない。tasks 12.2 の「最初に製品に触れる merge の CI が通ると」は、どちらとも読める
- kind: technical
- 提案: 番号が無いときの基準を Requirement に書く。たとえば「first-parent の親との差分」か「番号付けの workflow を足した commit」。Scenario「番号の印が無い木の文書だけの merge には番号を付けない」（またはその逆）を足す
- 処置: fixed D4 —— 番号の印が無いときは first-parent の親からの差分で決める文と Scenario「番号の印が無い木では、文書だけの merge に番号を付けない」を足した

## R6. 確認バッチを数える範囲の端（前の番号の commit を含むか・番号を付ける commit を含むか）を言う Scenario が無い
- 成果物: specs/distribution/spec.md（Requirement 2 :61、Scenario「間に確認バッチの merge があれば…」:81-84）・design.md D5（:80）
- 根拠:
  - Scenario :81-84 では、確認バッチの merge は 2 つの番号の間にあり、番号を付けるのは後の文書だけの merge。端に当たらない
  - 日常に起きるのは端の場合。(a) 確認バッチの merge の run が成功して、その merge 自身に番号が付く（取り消されなければ）。(b) 前の番号が確認バッチの merge に付いていて、次の番号を決めるとき、その merge を数えると真ん中が 2 回上がる
  - D5 :80 の「前の番号の commit から番号を付ける commit までの main の first-parent の commit」は、端を含むかを言わない
- kind: technical
- 提案: 範囲を「前の番号の commit を除き、番号を付ける commit を含む」と本文に書く。(a)・(b) の Scenario を 1 つずつ足す（(a) `0.2.3` の後の確認バッチの merge 自身 → `0.3.0`、(b) その後の道具だけの merge → `0.3.1`）
- 処置: fixed D5 —— 範囲を「前の番号の commit を除き、番号を付ける commit を含む」と本文に書き、(a)(b) の Scenario を足した

## R7. MODIFIED の「出所の版と違う版番号を渡すと、組み立てが失敗する」の名前が、新しい振る舞いでは偽。REMOVED + ADDED にした理由と同じ形
- 成果物: specs/distribution/spec.md（MODIFIED「3 つの部品の版番号は、1 つの出所から決まる」:410-412、Scenario :439-442・:477-479）
- 根拠:
  - 新しい規則（:412）では、出所の版 `0.0.1` の木に `0.4.0-beta.2` を渡すと組み立てが通る。この Requirement の隣の Scenario（:428-432）がまさにそれを言う
  - Scenario「出所の版と違う版番号を渡すと、組み立てが失敗する」（:439-442）は、中身を `0.0.2` に直して通る形にした。名前は「出所の版と違えば落ちる」のままで、一般には偽。check_scenarios の印は名前で結ぶので、印を読む者は旧い規則を読む
  - 呼び出し元は、3 つの Requirement を「Scenario の名前が旧い振る舞いを言うので MODIFIED で残せず」REMOVED + ADDED にした。この Scenario も同じ形
  - 「出所の版の MAJOR が 0 でないと、版の検査が落ちる」（:477-479）は、新しい「出所の版が 0.0.x でないと」（:481-483）が含む。名前の言う「MAJOR」は、もう出所の版が持たない（THEN の括弧書きが自分で断っている）
- kind: technical
- 提案: :439 を「番号の形でも出所の版でもない版番号を渡すと、組み立てが失敗する」などの名前の新しい Scenario にする。印の場所（build-stamp・version.sh・gradle・TS）の名前も一緒に変える。:477 は消すか、:481 に寄せる。**spec を変える指摘**
- 処置: fixed D9 —— 「3 つの部品の版番号は、1 つの出所から決まる」を REMOVED にし、新しい名前の ADDED に移した。Scenario を「出所の版と違う 0.0.x を渡すと、組み立てが失敗する」に改め、「MAJOR が 0 でない」は 0.0.x の Scenario（`1.0.0` を含む）に寄せた。印の名前の付け替えを tasks 1.2・2.1 に書いた

## R8. MODIFIED の Scenario「DB に新しい版の知らない版があると…」の版の置き換えが、試験の台本と合わず、前後も逆になった
- 成果物: specs/distribution/spec.md（MODIFIED「この回の退避が無いとき…」:628-630）・tasks.md 7.1（:95）
- 根拠:
  - 置き換えた Scenario は「版 `0.0.3` が知らない版を当てた DB の上の、版 `0.0.4` のインストール」に「版 `0.0.3` のインストーラ」を走らせる。0.0.3 < 0.0.4 なので、古い版のインストーラを新しいインストールの上で走らせる文に読める
  - 試験の台本 `tools/release/update-apply-install.sh:701-702` は、名前に「知らない版」を含む Scenario を `compat_old_zip` → `compat_new_zip`（`0.4.1` → `0.6.0`）で走らせる。tasks 7.1 の後は `0.0.4` → `0.0.5`
  - 正典（:3754-3756）は既に `0.5.0` と書いていて、台本の `0.6.0` と食い違っていた。機械の置き換え（`0.5.0`→`0.0.3`）がその食い違いを引き継ぎ、前後の逆転も生んだ
- kind: technical
- 提案: Scenario のインストーラの版を、台本の組み合わせに合わせて `0.0.5` にする（「版 `0.0.5` が知らない版を当てた DB の上の、版 `0.0.4` のインストール」）。**spec を変える指摘**
- 処置: fixed 7.1 —— Scenario のインストーラの版を `0.0.5` にし、tasks 7.1 に互換の組で走らせることを書いた

## R9. 版の検査は、番号の形の版番号を誰からでも受け取る。「同じ番号で中身の違う組み立て」を止めるのは、番号付けの workflow だけが番号を渡すことだけ
- 成果物: specs/distribution/spec.md（MODIFIED :412、Requirement「番号を持たない組み立ては、0.0.x を名乗る」:194-200）・design.md D9（:124）・Risks
- 根拠:
  - :412 は「左がファイルの値と同じで、`<左>.<真ん中>` が `0.0` でない」版番号を、どの木からでも受け取る。`0.1.0`・`0.1.1`（今の形で使った番号）も通る
  - 旧い規則では、出所の版と違う版番号は組み立てが落ちた（正典 :536-537）。これが手元で別の番号を名乗る組み立てを止めていた
  - 番号を持たない組み立ての Requirement（:194-200）が「番号を名乗らない」と固定するのは、CI の更新の試験の組み立てだけ（Scenario :207-209）。PR の CI・手元の `build.sh` が `ASHIATO_BUILD_VERSION` に番号を渡さないことを見る Scenario は無い
  - 2026-11-19 の事故の形（手元で組んだ物を本人が入れた。FR-137 ★）は、この規則のままでも起こせる。design の Risks に、この緩めは書かれていない
- kind: technical
- 提案: 緩めたことと理由（組み立ては印を見られない・浅い木）を、design の Risks に書く。守りは ST64（札の無い組み立てを入れない）へ申し送る。ci.yml の `package-*`（PR と main の push）が `ASHIATO_BUILD_VERSION` に `0.0.` で始まらない値を渡さないことを、check-guards の検査と Scenario にする。ただし、組み立ての印の Scenario（:428-432）と tasks 2.2（:46）・2.4（:50）は、試験の中で番号の形の `0.4.0-beta.2` を組む。検査は、配る形の組み立ての job（`package-*`）が渡す値に限り、自己試験の組み立てには掛けない
- 処置: fixed D6 —— 緩めたことと理由を design の Risks に書き、配る形の組み立ての job が番号を渡さないことを Requirement・Scenario「配る形の組み立ての job は、番号を版番号に渡さない」・tasks 7.1 の検査にした。入れる側の守りは docs/handoff/ST64.md へ（tasks 11.1）

## R10. 番号の組み立ての振る舞いのうち 3 つが design と tasks にしかない（署名していない APK の名前・CI のキャッシュへの保存・置く前の配布物の検査）。署名していない APK と debug の鍵の APK が同じ中身であることも、どこも言っていない
- 成果物: specs/distribution/spec.md（Requirement「番号を付けた組み立ては…」:113-118、Scenario「番号付けの workflow は、前の組み立てを使い回さず…」:155-157）・design.md D2（:46）・D10（:136）・D12（:150-153）・tasks.md 4.1・4.2
- 根拠:
  - 署名していない APK の名前 `ashiato-collector-<番号>-unsigned.apk` は、D2 :46 と tasks :16 にしかない。ST62 が下書きから読む名前で、正典は配布物の名前を「版をまたいで変えない（後の Story が版の確認で名前を読む）」として spec に置いている（:252）。`openspec archive` は design を正典へ写さない
  - D12 :150 は「番号付けの組み立ては CI のキャッシュを引かず、**保存もしない**」と言う。spec（:117・:157）は「引かない」だけ。number.yml は既定の枝の上で走るので、そこから保存したキャッシュは PR から引ける。鍵（`windows-build-key.sh`）は版番号を含まない（review/deep.md R6）。つまり「保存しない」が要の文になっている
  - D10 :136 は、置く前に配布物の検査（`check-artifacts.sh --allow-debug-key --expect-version`）を通すと言う。spec は、置く前の検査も、落ちたら置かないことも言わない。印の・`.dirty`・資格情報の検査を外した番号の組み立ても、spec の上では下書きに置ける。下書きは ST62 で札を付けて公開する物
  - D2 の「組み直さずに済む」は、署名していない APK と debug の鍵の APK が同じ組み立てであることに依る。tasks 4.2 は別の環境変数（`ASHIATO_APK_ALSO_UNSIGNED=1`）で 2 つ目を出す。2 つの組み立ての印・versionCode・中身が同じことを見る Scenario は無い。本人が試した APK（debug の鍵）と、ST62 が正式な鍵で署名する APK が別物になりうる
  - premises の C8（「使い回しの鍵に版番号を含める」）は D12 で「引かず保存しない」に変わった。D12 は C8 を名指すだけで、変えたとは書いていない
- kind: technical
- 提案: Requirement に 3 文を足す。(1) 署名していない APK の名前。(2) 番号の組み立てを CI のキャッシュに保存しない。(3) 置く前に配布物の検査を通し、落ちたら何も置かない。Scenario を足す。workflow を読む Scenario に「保存する手順を持たない」。place の自己試験に「検査に落ちた置き場では、配布元へ書く要求を 1 つも出さない」と「署名していない APK と debug の鍵の APK は、組み立ての印と versionCode が同じ」。D12 に C8 から変えたことを書く。**spec を変える指摘**
- 処置: fixed D12 —— Requirement に署名していない APK の名前・同じ組み立てであること・キャッシュへ保存しないこと・置く前の検査を足し、Scenario「配布物の検査に落ちた置き場では…」「署名していない APK は、debug の鍵の APK と同じ組み立て」を足した。D12 に C8 から変えたことを書いた

## R11. Story の完了の判定と題、FR-134 の 3 つの未決が、推奨の答えと食い違う。それを戻す先が機械で拾える形で無く、triage は「要件へ戻すもの 0 件」
- 成果物: docs/stories/ST61.md（題・完了の判定）・openspec/changes/st61-auto-version-bump/deep.md・proposal.md
- 根拠:
  - Story の題「main に merge するたびに版番号が自動で進む」と完了の判定 1 つ目「main に merge すると…前の番号より 1 つ進んだ番号の組み立てができる」は、Q1 の推奨（文書だけの merge・取り消された merge には付けない）と食い違う。proposal が名指すのは FR-134 の ★ 訂正だけで、Story の完了の判定には触れていない。確認バッチと archive の判定は、Story の完了の判定を読む
  - FR-134 には 3 つの未決がある。NEEDS CLARIFICATION「真ん中の数字をいつ上げるか」（期限: この Story の深掘り）・「式を変えるか」・「文書だけの merge でも番号を進めるか」（requirements.md:1181-1183）。Q3・Q2・Q1 が閉じる
  - 戻す先は proposal :10 と design D4 :76 に散文で書いてあるだけ。`review_triage.py` は「要件へ戻すもの 0 件」と数えるので、record の段で漏れても機械は止めない
- kind: conflict
- 提案: deep.md に「要件へ戻すもの」の節を置く。FR-134 の本文の ★ 訂正、3 つの未決の閉じ方、Story の題と完了の判定の直しを、Q ごとに機械で拾える形で書く（答えが推奨と違えば record が直す）。Story の完了の判定は、Q1 の答えに合わせて record の段で直すと proposal にも書く
- 処置: fixed D4 仮 —— D4 に「要件へ戻すもの」（FR-134 の本文と 3 つの未決、Story の題と完了の判定）を書き、proposal にも書いた。推奨と違う答えなら record の段が直す

## R12. 開発版の作り直し（D11）は FR-107 の新しい本文（ST63 の担当）を、修正の枝の REMOVED は FR-105 の未決（急ぎの直し。ST62 の担当）を、先に実装する。proposal は「前倒しは無い」と書く
- 成果物: proposal.md（:29「前倒しは無い」）・design.md D11（:140-146）・specs/distribution/spec.md（Requirement「開発版は、新しい番号が付いた日だけ…」:333-345、REMOVED「緊急の修正の枝…」:874-878）・tasks.md 8.1（:104）
- 根拠:
  - 「最も新しい番号を開発版向けに組む」は、FR-107 の ★ 2026-11-21 の新しい本文（requirements.md:995）。FR-107 は ST63 の satisfies にあり、ST61 には無い（INDEX.md:85・:87）
  - FR-105 には「急ぎの直しの出し方 —— main に merge して進んだ番号へ正式版の札を動かすか、正式版の番号から別の道を作るか」の NEEDS CLARIFICATION がある（requirements.md:984。期限はこの改訂を受ける Story の深掘りで、ST62 が FR-105 を持つ）。ST61 の REMOVED の Migration（:878）と tasks 8.1 の `docs/release.md` の直しは「main で直して次の番号を出す」を手順に書き、片方を先に選ぶ
  - どちらも、移る間に今の組み立てを緑に保つためで、理由はある（premises の C7）。ただ、前倒しと書かれていない。後で ST62・ST63 の深掘りが、決まっていない問いとして見落とす
- kind: defer
- 提案: proposal の capability の照合に、FR-107 の一部（移る間の開発版）を前倒すことと、その理由を書く。急ぎの直しは、REMOVED の Migration と `docs/release.md` を「ST62 が決めるまでの間の手順」と書く。tasks 11.1 で作る `docs/handoff/ST62.md` に、FR-105 の未決が ST61 の後も開いていること（修正の枝を戻すなら番号の付け方も要る）を足す
- 処置: fixed proposal.md —— proposal に要件の前倒し（FR-107 の一部）と、FR-105 の急ぎの直しは ST62 が決めるまでの手順にすぎないことを書き、docs/handoff/ST62.md に未決が開いたままであることを書く手順を tasks 11.1 に足した

## R13. 番号を持たない組み立ての携帯の versionCode は 1 になる。本人の携帯の 0.1.x（10,099・10,199）の上には、PR の APK を上書きで入れられない。Q4 の問いがこれを言っていない
- 成果物: specs/distribution/spec.md（Scenario「versionCode は版番号の 3 つの数字から決まる」:221-223 の `0.0.1`→1）・deep-questions.json Q4・design.md D6
- 根拠:
  - D3 の式で `0.0.1` は 1。`0.0.2`〜`0.0.5` も 2〜5
  - 正典 :606-609 と premises P6 によれば、Android は小さい versionCode の上書きを拒む。本人の携帯には `0.1.0+dfdec6d15…`（debug の鍵）が入っている（FR-137 ★ 2026-11-21）。PR の組み立ても debug の鍵で署名する（`collector-android/app/build.gradle.kts:256-259`）。CI の debug の鍵が本人の携帯の APK と同じかは確かめていない。同じであっても、versionCode 1 は 10,199 より小さいので、PR の APK を上書きで入れられない（今の 0.1.x の PR の組み立ては 10,099 以上だった）。入れ直すには消すしかなく、消すと手元の未送信を失う（FR-137 が守るもの）
  - 番号の組み立て（`0.2.0` = 20,000）は上書きで入る。PR の APK を入れられないのは、FR-136 の向き（札の無い組み立てを入れない）と同じとも読める
  - Q4 の context（deep-questions.json）は、インストーラが版の大小を見ないこと（P15）だけを書き、携帯で起きることを書いていない。日常の試し方が変わることを、本人は知らずに答える
- kind: daily
- 提案: Q4 の context（GO の 1 枚）と design D6 に、携帯の PR の APK は今の 0.1.x と番号の組み立ての上に上書きで入らない（試すには消すか、番号の組み立てを使う）ことを足す
- 処置: fixed D6 仮 —— D6（仮）に、携帯の PR の APK が 0.1.x と番号の組み立ての上に上書きで入らないことと、試し方を書いた。GO 資料で示す（第 1 回の問いの本文は既に渡す形で固まっているので直さない）

## R14. 本文が言っていて Scenario が無い文と、名前が中身と合わない Scenario
- 成果物: specs/distribution/spec.md
- 根拠:
  - Requirement 2 :64: 判定が「真ん中が 999 を超える・左が 209 を超える」で止まることは Scenario に無い。右だけが「携帯の versionCode に収まらない番号は付けない」（:96-99）にある。左の上限は、左の数字のファイルを 210 にした場合に起きる
  - Requirement 2 :62: 判定が「左の数字のファイルが数字 1 つでない」で止まることも、Scenario に無い（版の検査 :489-491 は別の手順）
  - :67「付けた番号を数え直さない」と :115「どれも消さずに残す」は、観測の手段を書いていない（消す要求を出さないことは :135-138 が一部を見る）
  - Requirement の題「新しい番号が付いた日だけ…作り直す」（:333）と Scenario の名前「nightly と最新の番号の commit が同じ日は、開発版を作らない」（:352）は「日」を言う。中身は「`nightly` の commit と最新の番号の commit が同じか」で、日を見ない。予定の実行が落ちた次の日にも作り直す（:362-365）ので、題の「付いた日だけ」は本文と合わない
- kind: technical
- 提案: 判定の Scenario に、真ん中 `0.999.x` の後の確認バッチ、左の数字のファイル `210`、`0.1` の 3 つを足す（どれも止まる）。:67 は観測できる形（「判定は番号の印だけから次の番号を決める」）に書き直すか、本文から外す。:333・:352 の「日」を「`nightly` が最新の番号の commit を指していれば」に直す
- 処置: fixed 3.1 —— 真ん中と左の上限・左の数字のファイルの形の Scenario を足し、「数え直さない」を観測できる文に直した。開発版の Requirement の題と 2 つの Scenario の名前から「日」を外した（tasks 9.1）

## R15. 仮の決定 D1 の台帳の rests_on が空。反転条件が依る P5・P11 が載っていない
- 成果物: openspec/changes/st61-auto-version-bump/provisional.jsonl（D1）
- 根拠:
  - provisional.jsonl の D1 は `"rests_on": []`
  - design D1 :41-42 の反転条件は P5（下書きは認証なしの一覧に出ない。確かさ 2）と P11（容量の上限。確かさ 1）に依る
  - review/deep.md R9 の提案は「D（仮）の台帳の rests_on に P5 を入れる」。処置（fixed premises.md）は確かめを下流に置いただけで、台帳は直していない
  - 台帳は provisional_review と確認バッチが読む。tasks 12.1 で P5 が外れても、台帳から D1 へは辿れない
- kind: technical
- 提案: D1 の rests_on に P5・P11 を入れる（`provisional.py` で行を足し直す）
- 処置: fixed D1 —— 台帳の D1 の rests_on に P5・P11 を入れた（どちらも design.md の D1 にある）

## R16. tasks の検証の弱いところ（D8 の配布元への問い合わせ・3.1 の grep・7.1 の確かめ）
- 成果物: openspec/changes/st61-auto-version-bump/tasks.md（3.1 :62・6.1 :88-91・7.1 :96・人間の確認待ち :129-135）
- 根拠:
  - 6.1: 「前に成功した main の push の run」を Actions の API（`gh api …/runs?branch=main&event=push&status=success`）から読むのは workflow で、自己試験は環境変数で渡した commit しか見ない。問い合わせの形（`status=success` の絞り・並び順・`actions: read`）を本物の配布元で確かめるところが、tasks にも人間の確認待ちにも無い。外れると、毎回「全部走らせる」に落ちる（D8 の反転条件）か、誤った基準で code の job を飛ばす
  - 3.1 の検証の grep は名前の一部で探す。「番号を付けない」は「文書だけの merge の commit には番号を付けない」と「…祖先の run が後から成功しても、番号を付けない」の両方に当たり、「番号を付けずに止まる」も 2 つの Scenario に当たる。どちらか片方の印が無くても数は 14 になる（自己試験の rc と Task 11 の check_scenarios が後ろで受けるので、実害は小さい）
  - 7.1 は「`install-check.sh`・`update-check-install.sh`・`soak-install.sh` に 0.4.0 などの版の値が無いことを確かめる」とあるが、コマンドが無い（今は 3 つとも当たりが無いことを `grep -nE '0\.(4|5|6)\.[0-9]'` で確かめた）
- kind: technical
- 提案: 人間の確認待ちに「main に入った後、最初の文書だけの merge の run の `changes` の job の出力に、基準の commit（前に成功した run の `head_sha`）が出る」を足す。3.1 の grep は Scenario の名前の全文で探す。7.1 の確かめは 7.1 の検証の `grep` に 3 つのファイルを足す
- 処置: fixed 3.1 —— 3.1 の検証を Scenario の名前の全文で数える形にし、7.1 の grep に 3 つのファイルを足し、人間の確認待ちに 12.4（main の差分の基準）を足した

## R17. 版の検査の 4 つの実装（bash・Rust・gradle・TypeScript）が同じ D9 の規則を持つことを、gradle と TypeScript で確かめる試験が無い
- 成果物: openspec/changes/st61-auto-version-bump/tasks.md（Global Constraints :20・2.1 :38-40・2.2 :41-46・2.3 :47-48・1.1 :29）・design.md D9（:124-125）
- 根拠:
  - design :12 は、版の検査が 4 つの実装（`version.sh`・`crates/build-stamp/src/lib.rs`・`build.gradle.kts`・`web/build-stamp.ts`）にあると言う。D9 :124 は 4 つを同じ規則にすると言う。新しい規則は 3 つ: 左の数字が `MAJOR` と違えば落ちる・`<左>.<真ん中>` が `0.0` で出所の版と違えば落ちる・`0.0.0` を断る
  - Global Constraints :20 の「同じ値の表（`version-table.tsv`）を 4 か所の試験が読む」の表は versionCode の表。中身は `<版番号>\t<versionCode>` と、拒む `-beta.99`・`-beta.0`・`-beta.+5` の行だけ（`tools/release/version-table.tsv`）。tasks 1.1 :29 の書き直しの後も、拒むのは `0.1000.0`・`0.1.10000`・`210.0.0`・`0.4.0-beta.99` の 4 つで、D9 の新しい拒み（`1.4.0`・`0.0.2`・`0.0.0`）は表に入らない
  - Scenario「渡した番号の左の数字が左の数字のファイルと違うと、組み立てが失敗する」の印を置くのは Rust（2.1）だけ。gradle（2.2）には左の数字の試験が無い。TypeScript（2.3）は試験を足すが、印を持たない
  - `check_scenarios.py` は印 1 つで通る。gradle か TypeScript が規則から外れても（たとえば `MAJOR` を読まずに通す）、検査は落ちない。今の 4 つの実装が同じ規則でそろっていたのは、表を 4 か所で読んでいたから
- kind: technical
- 提案: 受ける版番号と拒む版番号の表（versionCode と別の列でもよい）に、`1.4.0`（`MAJOR` が `0`）・`0.0.2`（出所 `0.0.1`）・`0.0.0`・`0.4.0-beta.2`（受ける）を足し、4 つの実装の試験が同じ表を読むようにする。gradle と TypeScript の試験にも同じ Scenario の印を置く
- 処置: fixed 1.1 —— 受ける・断る版番号の表 `tools/release/version-check-table.tsv` を足し、4 つの実装の試験（1.1・2.1・2.2・2.3）が読む形にし、gradle と TypeScript にも印を置く

---

## 観点ごとの確かめの範囲

- 観点 1（deep の決定が正典に写っているか）: Q1〜Q4 の推奨は、どれも Scenario にある。Q1 は Requirement 1 の 8 本、Q2 は式と 20000・10000000・上限 999／9,999／209 の数値、Q3 は真ん中の上がり方、Q4 は `0.0.1`〜`0.0.5` と `0.0.0` を断ること。C1〜C8 も確かめた。C3 の 0.2.0 と C7 の印・開発版は Scenario にある。C8 の変更と、design だけにある D2・D12 の文は R10。台帳は R15。要件へ戻すものは R11。triage の「仮決め 4 件」は review/deep.md の `fixed D1 仮`・`D2 仮`・`D3 仮`・`D4 仮` を数え、台帳（provisional.jsonl）は D1・D2 の 2 行だけ。D3・D4 は Q2・Q1 の答え待ちの決定で、design の見出しは（仮）を持つ。答えの後に D3・D4 の（仮）を外すか台帳に載せるかを、record の段で決めること（意図どおりかは呼び出し元が見る）
- 観点 2（検証可能か）: R1・R2・R4〜R6・R14。THEN が 2 つ以上を束ねる Scenario（:124-128・:155-157）は、tasks 5.1 が「条件を 1 つ外した偽の workflow で rc≠0」と分けて試すので、R にしなかった（:124-128 は R2 で分けることを提案した）
- 観点 3（置き場）: R10。specs に入っている実装の名前は `tools/release/MAJOR`・`tools/release/changes.sh code`・`fetch-depth: 0`・`contents: write`。`tools/release/MAJOR` は本人が変える外の口。`fetch-depth: 0`・`contents: write` は「workflow を読む」Scenario の観測の対象（workflow の設定）で、関数名・crate 名・列名ではないので R にしなかった。ただ、正典の distribution にはどちらも 1 度も出てこない（`grep -c` が 0）。`tools/release/changes.sh code` は Requirement 1 の導出の説明（:12）にあり、SHALL の文には無い。proposal の Capabilities（`distribution`）は specs のディレクトリと INDEX.md:85・:112 に合う。前倒しは R12
- 観点 4（tasks の検証）: 全 Task にコマンドと rc の終了条件がある。42 本の新しい Scenario は、すべて Task が名指しする。順序は Task 1（規則と式）→ 3（判定）→ 4（置き場）→ 5（workflow）で、依存の向きに合う。人間の確認待ちは 12.1〜12.3（配布元に書く確かめ）。弱いところは R16。4 つの実装の揃いは R17
- 観点 5（Story と要件）: check_chain は観点 8 を含めて OK。完了の判定との食い違いは R11、satisfies の外は R12
- 観点 6: 該当なし（`spec --answers` ではない。第 1 回の先読み）

## 既存の Scenario との食い違い（呼び出し元の名指しの 4 か所）

- 印の版（`version.sh plan`）: 新しい Requirement の Scenario は、「出所の版が `0.4.0` の main の commit」を「番号 `0.4.0` を付けた main の commit」に置き換えた。印の判定の木（release.yml の `fetch-depth: 0`）は、番号の印も取り出す（actions/checkout の README「`fetch-depth: 0` to fetch all history for all branches and tags」の読み。確かさ 2）。食い違いは無い
- 開発版（`nightly-plan`）: 旧い「出所の版の正式版を出した後は組まない」の Scenario は REMOVED と一緒に消え、tasks 9.1 が印を消すと名指す。食い違いは R14 の題と名前の「日」だけ
- 確かめの下書き（`draft.sh`）: `draft.sh` は `version.sh check` だけを呼び（:14・:53）、`plan`（番号の無い commit の印を断る）を呼ばない。`0.0.1-beta.1` は D9 の (1) で通る。食い違いは無い
- 版の検査（4 つの実装）: R7（名前の偽）・R9（番号の形を誰からでも受け取る）・R17（gradle と TypeScript の揃いを見る試験が無い）

観点 7: 該当なし（spec_overlap.py candidates が 0 件）
