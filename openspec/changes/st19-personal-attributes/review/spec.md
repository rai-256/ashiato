# ST19 spec レビュー（独立）

対象: `openspec/changes/st19-personal-attributes/` の proposal.md / specs/personal-entities/spec.md / design.md / tasks.md、
`docs/stories/ST19.md`（と ST22 / ST23 / ST24 の差分）、`docs/handoff/ST22.md` / `ST23.md` の ST19 の項。
突き合わせの正本: deep.md（Q1〜Q4、C1〜C13、要件へ戻すもの）、proto.html、`docs/requirements.md`、`docs/stories/INDEX.md`、
既存コード（`crates/server/src/{lib.rs,ingest.rs}`、`migrations/202609120944_gates.sql` ほか、`tools/check-immutable.sh`、`tools/seed.sh`、`web/src/{Root.tsx,App.tsx,stays.ts}`）。
**成果物は触っていない。**

実施日: 2026-09-15。ブランチ `docs/st19-upstream`。

## 機械の検査

| コマンド | 結果 |
|---|---|
| `openspec validate st19-personal-attributes --strict` | `Change 'st19-personal-attributes' is valid`（rc=0） |
| `python3 scripts/check_chain.py .` | `chain: OK (0 件 / 未回収 0 件 / warn 0 件)`（rc=0） |
| `python3 scripts/check_scenarios.py . st19-personal-attributes` | `scenarios: FAIL (担保なし 63 件)`（rc=1）。63 件はすべてこの change が足した Scenario。上流の段階ではテストが無いので想定どおり。tasks 7.2 が回収する |
| `python3 scripts/review_triage.py . st19-personal-attributes` | `triage: OK`（rc=0。このファイルを書く前の状態。review/deep.md の 9 件） |

手で確かめたこと:

- **Scenario と tasks の対応**: spec の `#### Scenario:` 63 本の名前を、tasks.md のバッククォートで囲まれた文字列と `comm` で突き合わせた。**割り当ての無い Scenario は 0、名前の重複も 0**
- **要件へ戻すもの**: FR-44 / FR-45 / PERM-4 / 扉 #3 / 扉 #15 に ★ 2026-09-15 が入っている（`docs/requirements.md:376` `:387` `:550` `:857` `:920`）。ST19 / ST22 / ST23 / ST24 の逐語は再生成済みで、check_chain が一致を確かめた
- **Q2 の逐語と spec**: 骨格・並び・訂正の畳み方・書く入口・精度を先に選ぶ は spec の SHALL と Scenario にある（`spec.md:316-320` `:366-368`）。食い違いは R1 の 1 点
- **Q1 / Q3 / Q4**: spec の「主張は書き換えられない」（:116-168）・「主張の既定の感度」（:297-312）・「いまの値」（:221-295）にある
- **design が引く既存コードの位置**: `ingest.rs:131`（`validate`）、`gates.sql:40` / `:82` / `:241-248`、`lib.rs:344-356`（`place_identifiers`）はコードと一致した。`coverage::must_sources()` だけが稼働状況に出ること（`lib.rs:1366`）も一致した。食い違いは R14 の 1 点（`check-immutable.sh`）
- **proposal の Capabilities と specs/**: `personal-entities` の 1 つで一致。INDEX の capability 表（`INDEX.md:75`）とも一致

---

## 観点 1: deep の決定が正典に写っているか

## R1. 「補足を押したときだけ出す」は Q2 の決定に無く、本人が見た proto では補足は行に常に出ていた
- 成果物: openspec/changes/st19-personal-attributes/specs/personal-entities/spec.md / design.md（D9）
- 根拠: Q2 の逐語（deep.md:41）は「2 つの時刻 - 『いつから』だけ。書いた日は主張を押したときだけ」で、補足には触れていない。proto の行の描画（proto.html:352-354 `claimRow`）は、どの設定でも `補足: …` を行の中に出す。spec.md:319 は「主張した日時と補足を、その主張を押したときにだけ出す」、spec.md:341 の THEN も「押した後は主張した日時と補足が見える」、design.md:182 も同じ。本人が選んだ画面から、補足を隠す軸を AI が足している
- kind: conflict
- 提案: spec.md:319 / :341 と D9 から補足を外し、proto と同じく補足は行に常に出す形に戻す。隠したいなら、B の問いとして別に立てる
- 処置: fixed D13 仮 — spec と D9 から補足を外し、proto と同じく補足は行に常に出す（押したときだけにするのは主張した日時だけ）。Scenario「補足は押さずに見える」を足した。反転条件は design D13

## R2. C12 の Scenario は、乱数が弱くても、乱数が残った列から導けても緑になる。tasks 2.4 の「わざと壊す」も落ちない
- 成果物: specs/personal-entities/spec.md（「消去した主張の値は、残った列から当てられない」）/ tasks.md 2.4
- 根拠: spec.md:179-180 の WHEN は、テストが自分で「消去の前と同じ種類・値・いつから・識別子・主張した日時から原文を組み立てる」。乱数を持たない原文を組むので、乱数が 1 バイトでも、`id` や `event_time` から導いた値でも、計算した鍵は必ず一致しない。128 bit（design.md:99）と「16 バイト以上」（design.md:103）はどちらも spec に無い。tasks.md:63-66 の「`nonce` を空文字にして落ちるか」は、D5 が短い乱数を断る（design.md:114）ので格納で落ちるか、テストが組む原文（`nonce` の欄なし）と格納された原文（`"nonce":""`）が違うまま一致しないかのどちらかで、確かめたいことを確かめない。「画面側の組み立てと同じ関数」は TypeScript で、Rust のテストからは呼べない
- kind: technical
- 提案: spec に数値つきの SHALL と Scenario を足す（「128 bit 未満の乱数を持つ主張は受け付けない」「消去の後に残るどの列にも乱数が現れない」）。2.4 のわざと壊すは「乱数を `id` から導く」「乱数を `payload` に写す」のように、Scenario が落ちるべき壊し方に直す
- 処置: fixed specs/personal-entities/spec.md — 「128 bit 以上の乱数を持ち、解析済みにも他の列にも写さない / 識別子・出来事の時刻・値から導かない」を SHALL にし、Scenario「乱数は解析済みに写らない」「同じ内容の 2 つの主張は別々の乱数を持つ」「乱数が短い主張は受け付けない」を足した。わざと壊す確かめは tasks 3.3（`payload` に乱数を残す）と 4.1（乱数を `id` から作る）に直し、design D4 に Rust と web への分け方を書いた

## R3. C8（値は本人が打った文字列を NFC で持つ）と、D1 の「解析済みは原文から組み直し、送り主の解析済みは使わない」が spec に無い
- 成果物: specs/personal-entities/spec.md / design.md（D1）
- 根拠: deep.md:97（C8）。spec が主張の値について言うのは「原文を受け取ったまま保持する」（spec.md:17 / :68）だけ。design.md:45 は `payload` を原文から組み直し、送り主の `payload` を捨てる。`GET /attributes` の `value` を原文と解析済みのどちらから読むかは D8（design.md:157-171）にも無い。NFD で打った値が NFC で返るか、送り主の `payload` に別の値を入れた要求がどう格納されるかは、spec では真偽が決まらない
- kind: technical
- 提案: 「主張の解析済みの値を原文から作り、NFC にする」「読み出しの値は NFC」を SHALL にして、NFD の値と、原文と食い違う `payload` を送る Scenario を 1 本ずつ足す
- 処置: fixed specs/personal-entities/spec.md — 「解析済みは送り主のものでなく原文から作り、NFC にする」を SHALL にし、Scenario「合成済みでない値は合成済みで読み出される」「原文と食い違う解析済みを送っても原文の値で格納される」を足した（tasks 3.2）

## R4. Q2 の「動かさない前提」のうち、表面を ui-direction の確定値にすることが spec に無い
- 成果物: specs/personal-entities/spec.md（「個人属性の画面の読み出しの失敗と下限」）
- 根拠: Q2 の逐語（deep.md:50）は「表面は ui-direction の確定値（色相 132°・彩度 30%・地 12・面 3 段・角 15px・境界線）」を前提に置く。spec.md:420-448 が言うのはコントラスト比・24px・フォーカス・明暗だけ。tokens.ts から引くことは design.md:187 にしか無いので、画面が別の色を直に書いても spec は落ちない
- kind: technical
- 提案: 「画面の色は `docs/ui-direction.md` の確定値だけから引く」を SHALL にし、tasks 4.4 にトークン以外の色の直書きが無いことを見る検証を足す（実装名は spec に入れない）
- 処置: fixed specs/personal-entities/spec.md — 「画面の色を ui-direction の確定値からだけ引く」を SHALL にし、Scenario「個人属性の画面は確定した色だけを使う」を足した。tasks 4.4 に色の直書きの grep を足した

---

## 観点 2: Scenario が検証可能か

## R5. 断る Scenario 6 本はどれも「理由の種別が示される」までしか言わず、どの種別を返しても緑になる
- 成果物: specs/personal-entities/spec.md（「形の合わない主張は受け付けない」）/ design.md（D5）
- 根拠: spec.md:84 / :89 / :94 / :99 / :104 / :109 の THEN は同じ一文。種別の名前と、どの条件にどの種別を返すかは design.md:112-119 の表にしかない。取り消す主張が無い要求に `malformed_claim` を返す実装でも、6 本すべてが通る。画面は種別を文に直して出す（design.md:192）ので、種別の取り違えは本人に見える誤りになる
- kind: technical
- 提案: 種別の値と条件の対応を spec の SHALL に移し、各 Scenario の THEN に期待する種別を書く（R12 と同時に直せる）
- 処置: fixed specs/personal-entities/spec.md — 理由の種別と条件の表を spec の SHALL に移し、断る Scenario の THEN に期待する種別の値を書いた。tasks 3.1 に「期待する種別を assert_eq! で見る」を書いた

## R6. `s01-attribute` に「本人が書いた」でない由来で送ると、主張として格納される。検査の一覧に由来が無い
- 成果物: design.md（D1 / D5）/ specs/personal-entities/spec.md / tasks.md 2.2
- 根拠: design.md:33 は `origin='authored'` と書くが、D5 の表（design.md:112-119）と tasks.md:52-56 に由来の検査は無い。いまの `validate`（`ingest.rs:130-155`）は、`collected` なら端末識別子を、`derived` なら何も求めない。なので `origin: "derived"` か、端末識別子つきの `collected` で送れば、主張の検査を通って格納される。`collected` の主張には ST03 の錠と門（`gates.sql:40-71` / `:82-145`）も掛かり、spec.md:121-122 の開口部（感度・削除の印・消去）と違う規則で縛られる。spec.md:72-74 の断る一覧にも由来と端末識別子が無い
- kind: technical
- 提案: 「本人が書いたでない主張 / 端末識別子を持つ主張は受け付けない」を spec の断る一覧と Scenario に足し、D5 の表に種別を 1 つ足す
- 処置: fixed specs/personal-entities/spec.md — 断る表に `claim_not_authored`（由来が本人が書いたでない / 端末識別子を持つ）を足し、Scenario 2 本を足した。design D5 の検査の順と tasks 3.1 に足した

## R7. 消去を通す条件が「その主張の台帳の行」になっていない。台帳があっても消去の形でない書き換えを拒む Scenario も無い
- 成果物: design.md（D2 の 2）/ specs/personal-entities/spec.md / docs/handoff/ST23.md
- 根拠: design.md:84-85 は「同じトランザクションに `core.erasure_ledger` の `scope = 'event'` の行があるときだけ」通す。ST03 の門は `l.event_id = NEW.id AND l.txid = pg_current_xact_id()` まで見ている（`gates.sql:112-116`）。D2 の書き方のままだと、別の記録の台帳を 1 行書いた同じまとまりで、何件でも主張を消去できる。spec.md:122 と :155-163 の Scenario も「台帳の行がある / 無い」しか分けないので、この差を検出しない。さらに、ST03 が実測で見つけた「台帳 1 行で改竄が通る」（`gates.sql:96-104` R51 / R95）と同じ形の Scenario（台帳の行があっても、値を別の値にする・`payload` だけ植える・鍵を変える書き換えは拒まれる）が無い。`docs/handoff/ST23.md:20` も D2 の書き方をそのまま写している
- kind: technical
- 提案: D2 の 2 と handoff に「その主張の識別子の行」を足す。spec に「別の記録の台帳の行では通らない」「台帳の行があっても消去の形でない書き換えは拒まれる」の 2 本を足し、tasks 1.2 に割り当てる
- 処置: fixed D2 — 台帳の照合を `event_id = NEW.id AND scope = event AND txid` にし、消去の形でなければ台帳があっても拒むと書いた。spec に Scenario「別の記録の台帳の行では主張の消去は通らない」「台帳の行があっても消去の形でない書き換えは拒まれる」を足し、tasks 1.2 に割り当て、わざと壊す確かめを照合を外す形にした。docs/handoff/ST23.md も直した

## R8. 1 本の Scenario が 2 つ以上を主張しているものが 13 本ある
- 成果物: specs/personal-entities/spec.md
- 根拠: THEN / AND で別々の主張を束ねているもの —— 「主張した日時といつからが別々に入る」（:31-32。2 つの時刻の分離と D-01 に入った時刻）/「補足と原文が残る」（:67-68）/「精度と日付が合わないいつからは受け付けない」（:93。精度の不一致 **または** 暦に無い日付）/「無い主張や別の利用者の主張は取り消せない」（:103。**または**）/「主張に削除の印と感度は付けられる」（:152-153）/「名前を変えても識別子と主張が変わらない」（:202-203）/「最初に住所と職業がある」（:208-209）/「空の名前と重なる名前は受け付けない」（:213。**または**）/「種類の台帳は書き換えも削除もできない」（:218-219）/「消したことにした主張は出ない」（:280）/「訂正で取り消した主張は畳まれる」（:346-347）/「精度を先に選ぶと選んだ欄だけが出る」（:391-392）/「文字のコントラストとフォーカスの下限」（:442-443）。「または」の Scenario は片方だけのテストで印が付き、緑になる
- kind: technical
- 提案: 「または」の 4 本は分ける。ほかは、片方だけ通っても緑になるものから分ける（少なくとも暦に無い日付・別の利用者・重なる名前・フォーカス）
- 処置: fixed specs/personal-entities/spec.md — 「または」の 4 本（精度と暦 / 無い主張と別の利用者 / 空の名前と重なる名前 / 削除の印と感度）と、フォーカス・前の名前の台帳・同時の初期化・消した主張の次・取り消しの展開・D-01 に入った時刻・補足と原文を分けた

## R9. 観測の手段が決まらない言葉と、2 通りに読める規則がある
- 成果物: specs/personal-entities/spec.md
- 根拠:
  - :352「予定であると分かる形で出て」—— 何が出れば真なのかが無い（proto は「予定」の札。proto.html:342）
  - :357「主張の値を直接変える操作はどこにも無い」—— 何を数えれば「無い」と言えるかが無い
  - :408「積めなかったことと理由が出て」—— 理由の文と種別の対応が spec に無い（R5）
  - :443「フォーカスの位置に輪郭が見える」—— 輪郭の条件（太さ・コントラスト）が無い
  - :224 / :230 / :245「後に書いた主張」—— 主張した日時（端末の時計）と D-01 に入った時刻（サーバの時計）のどちらで比べるかが無い。design.md:133 は主張した日時で、design.md:216 は端末の時計がずれうると認めている
  - :228「取り消されていない主張から取り消された主張を、積んだ主張とは分けて返し」—— 文として読めない（「取り消された主張を、取り消されていない主張とは分けて返し」の意か）
- kind: technical
- 提案: 予定は「『予定』の文字が出る」、編集は「主張の行を押しても値の入力欄が出ない / 書き換えの要求を送る操作が無い」のように観測できる形にする。「後に書いた」は主張した日時か D-01 に入った時刻かを SHALL に書く。:228 は文を直す
- 処置: fixed specs/personal-entities/spec.md — 予定は「予定」の文字、編集は「主張の行を押しても入力欄が出ず書き換えの要求を送る操作が無い」、フォーカスは「隣の色に対して 3:1 以上の輪郭」、「後に書いた」は主張した日時（同じなら D-01 に入った時刻）と SHALL に書き、:228 の文を直した。理由の文は「種別ごとに異なる文 / 届かなかったときは別の文」を SHALL にし、文そのものは design D9 の表に置いた

## R10. 「二重に押しても 1 件」の THEN はサーバの件数だが、tasks 4.3 は画面が送った原文しか見ない。押し直したときの主張した日時は「主張した日時は入力させない」と食い違う
- 成果物: specs/personal-entities/spec.md / tasks.md 4.3 / design.md（D9）
- 根拠: spec.md:413 の THEN は「主張は 1 件だけ増える」。tasks.md:95 の検証は「2 回押して送った原文が同じか、1 回だけ送られる」で、vitest の中にサーバの件数は無い。同じ原文で 1 件に畳まれるのは 2.3「同じ主張の再送は増えない」が別に確かめている。一方 design.md:189-190 は、届かずに送り直すときも最初に押した時刻の原文を再送するので、成功したのが何時間後でも主張した日時は最初に押した時刻になる。spec.md:403 の THEN「積んだ主張の主張した日時は『積む』を押した時刻である」は、どちらの押下かを言っていない
- kind: technical
- 提案: 画面の Scenario は「2 回押しても送る原文は 1 つ」に直す（件数はサーバの Scenario に任せる）。:403 は「その原文を最初に組んだときの押下の時刻」と書く
- 処置: fixed specs/personal-entities/spec.md — 画面の Scenario を「2 回押しても送る原文は 1 つ」に直し、主張した日時は「その原文を最初に組んだ押下の時刻」と SHALL に書いた。件数は Scenario「同じ主張の再送は増えない」（tasks 3.2）に任せ、tasks 4.3 の検証を 1 回目の fetch を失敗させて押し直す形にした

## R11. Requirement の本文だけが言っていて、Scenario が無いもの
- 成果物: specs/personal-entities/spec.md
- 根拠:
  - :74「取り消す主張が … 自分自身」—— Scenario が無い。design.md:118 の「主張のソースでない記録を指す」も spec の本文にも Scenario にも無い
  - :187「種類と名前の台帳の … 表の切り詰めを DB の側で拒む」—— :216-219 は書き換えと削除だけ
  - :189「いまの名前と重なる名前を受け付けない」—— 名前を変える操作（`POST …/names`）で重なる名前にする Scenario が無い（:211-214 は足すときだけ）
  - 種類の名前を変える要求が、無い種類や別の利用者の種類を指したときの扱いが本文にも Scenario にも無い（R16）
  - :229「削除の印の付いた主張を … 出さず」—— 印は無いが**本文を消去した**主張（原文と解析済みが空）をどう読むかが本文にも Scenario にも無い（R18）
- kind: technical
- 提案: 自分自身 / 主張でない記録 / 種類の台帳の切り詰め / 名前の変更での重なり / 別の利用者の種類の名前の変更 に Scenario を 1 本ずつ足す。消去した主張の読み方は R18 で決める
- 処置: fixed specs/personal-entities/spec.md — Scenario「自分自身は取り消せない」「主張でない記録は取り消せない」「種類の台帳は削除も切り詰めもできない」「いまある名前へは変えられない」「別の利用者の種類の名前は変えられない」「本文を消去した主張は出ず、その取り消しも効かない」を足した

---

## 観点 3: 置き場の誤り

## R12. design の D5 / D7 / D8 / D9 に、応答の形・断る条件・画面の文言など観測できる振る舞いが残っている
- 成果物: design.md / specs/personal-entities/spec.md
- 根拠: archive で正典から落ちるもの ——
  - D5（design.md:112-125）: 6 つの種別の名前と条件、`claim` が `id` と違えば断る、乱数が短ければ断る、前後の空白だけの値は空とみなす、**削除の印の付いた主張を取り消し先に指しても断らない**、**同じ主張を 2 件が取り消してもよい**
  - D7（design.md:151 / :154）: 名前を NFC にしてから重なりを判定する、空・重なる・無い種類は 400
  - D8（design.md:159-170）: `user_id` を省けば nil UUID、`claims` は `current` と `upcoming` を**含む**、種類は作った順、**感度で絞らない**
  - D6（design.md:131-132）: 取り消された主張がした取り消しも効く（取り消しの連鎖）
  - D9（design.md:180-186 / :192）: 人物・場所のタブを描かない、いまの値が無ければ「まだ書いていない」（spec.md:433 がこの文言を前提にしているが、文言は design にしか無い）、取り消す主張の選択の既定は主張した日時が最も新しいもの、受理でフォームを閉じる
- kind: technical
- 提案: 上の各項目を spec の SHALL と Scenario に移す。D 番号には実装の置き場（関数・列・トリガの名前）だけを残す
- 処置: fixed specs/personal-entities/spec.md — D5（種別と条件・空白だけは空・消した主張を取り消し先に指せる・複数の取り消し）、D7（NFC にしてから重なりを判定）、D8（claims は current と upcoming を含む・作った順・感度で絞らない）、D6（取り消しの展開）、D9（人物と場所のタブを描かない・「まだ書いていない」・取り消す主張の既定・受理でフォームを閉じる）を spec の SHALL と Scenario に移し、design には置き場だけを残した。利用者を省いたときの既定は ST19 固有でない（既存の読み出しと同じ）ので移さない

## R13. spec は ST24（PERM-4）と ST22 / ST23（FR-50 / FR-51 の開口部）と ST25（NFR-17〜22）の要件の条項を満たすが、INDEX に前倒しの記録が無く、proposal は「前倒しは無い」と書いている
- 成果物: proposal.md / docs/stories/INDEX.md
- 根拠: ST19 の `satisfies` は `[FR-44, FR-45]`（ST19.md:4）。spec.md:302 は PERM-4（INDEX.md:46 で ST24）、:126 は FR-50 / FR-51（ST22 / ST23）、:428 は NFR-17 / 18 / 19 / 22（ST25）を導出元に挙げる。proposal.md:50 は「capability の前倒しは無い」とだけ書き、要件の側の前倒しに触れない。ST16 は同じ形（本体の Story を変えず「派生」の条項だけを満たす）を INDEX.md:124-129 の表に残している
- kind: technical
- 提案: INDEX に ST16 と同じ形の「要件の側の前倒し」の表を足す（PERM-4 → ST24 / FR-50・FR-51 → ST22・ST23。NFR の画面の下限を ST16 でどう扱ったかに揃える）。proposal.md:50 に要件の側の前倒しがあることを書く
- 処置: fixed proposal.md — docs/stories/INDEX.md と合わせて。ST16 と同じ形で「要件の側の前倒し」の表（PERM-4 → ST24 / FR-50 → ST22 / FR-51 → ST23 / NFR-17〜23 → ST25）を足し、proposal の Capabilities に要件の側の前倒しがあることを書いた

---

## design の技術判断と既存コードの食い違い

## R14. `check-immutable.sh` の「本人が書いた記録は書き換えられる」の段は、`immutable-check` ではなく本人が書いた全行を書き換える。主張の行をこの段より前に入れると既存の段が落ちる
- 成果物: design.md（D2 の最後の段落）/ tasks.md 1.2
- 根拠: `tools/check-immutable.sh:154-155` は `UPDATE core.event SET payload = '{"edited":true}' WHERE origin = 'authored'`。design.md:88 は「`logical_source = 'immutable-check'` はそのまま残る —— 錠は主張のソースだけを見る」と書くが、文は主張の行も対象にする。tasks.md:37 は「主張の行を 1 件入れてから」とだけ書き、置く位置を決めていない。この段より前に主張の行を入れると、D2 の門が COMMIT で拒み、「NG 収集以外まで止めている」で落ちる。後に置くと、Scenario「主張以外の本人が書いた記録は従来どおり書き換えられる」（spec.md:165-168）の担保は、主張の行がまだ無い DB でしか確かめていないことになる
- kind: premise
- 提案: design.md:88 を事実に合わせる。tasks 1.2 に「既存の段の WHERE を `logical_source = 'immutable-check'` に絞り、主張の行を入れた**後で**その段が通ることを確かめる」を書く
- 処置: escalated — 本人の判断の前提ではなく design の記述の誤り（`check-immutable.sh` の既存の段は `origin = authored` の全行を書き換える）。D2 を事実に合わせて書き直し、tasks 1.2 に「既存の段の WHERE を `logical_source = immutable-check` に絞り、主張の行を入れた後に置く」を書いた。deep.md の「確かめたが問わなかったこと」に R14 として記録し、PR 本文で報告する（本人に選ばせる問いは立たない）

## R15. `GET /attributes` での種類の初期化は、名前の台帳に重複を残しうる。Requirement の「初めて読み出したとき」とも条件が違う
- 成果物: design.md（D7）/ specs/personal-entities/spec.md / tasks.md 3.2 / 5.2
- 根拠: design.md:152-153 は「種類が 0 件なら」2 つを入れ、`ON CONFLICT DO NOTHING` で同時の読み出しでも 2 つのまま、とする。ただし衝突で止まるのは `core.attribute_kind`（v5 の主キー）だけで、`core.attribute_kind_name` は IDENTITY の主キー（design.md:146）なので、2 本の読み出しが同時に 0 件を見れば名前の行が 2 本ずつ入る。この表は削除も切り詰めも拒む（design.md:150）ので、行は消せない。tasks.md:78 の「同時に走らせても 2 つ」は種類の数しか見ない。
  (b) 初期化は、POST が名前の重なりを見るときの利用者ごとの錠（design.md:151）を取るとは書いていないので、初期化と `POST /attributes/kinds {name:"住所"}` が重なると、いまの名前が「住所」の種類が 2 つできる。
  (c) spec.md:188 は「初めて読み出したとき、住所と職業がある状態にする」だが、D7 の条件は「種類が 0 件」。読み出す前に `POST /attributes/kinds` を撃った利用者には、住所と職業が置かれない。tasks.md:108 の seed は `/attributes/kinds` と `/ingest` で種類 5 を足すので、この順になりうる（proto の 5 種類に住所が含まれていれば、初期化の後に足すと重なりで 400 になる）。
  (d) `user_id` は名乗り（design.md:159）なので、任意の UUID で読み出すたびに消せない行が 4 本ずつ増える
- kind: technical
- 提案: 名前の行は種類の INSERT が行を返したときだけ入れ、初期化も同じ利用者ごとの錠の中で行う。spec.md:188 の条件を D7 と揃える（「種類を 1 つも持たない利用者が読み出したとき」か、「住所・職業の v5 の識別子が無ければ」）。tasks 3.2 の検証に名前の台帳の行数を足し、5.2 は初期化の後に足す順を書く
- 処置: fixed D7 — 初期化を GET と POST の両方の先頭で利用者ごとの錠の中で行い、名前の行は種類の INSERT が行を返したときだけ入れると書いた。spec の条件を「種類を 1 つも持たない利用者が読み出しか種類を足す操作を初めて行ったとき」に揃え、Scenario「同時に初めて読み出しても住所と職業は 1 つずつ」（名前の台帳の行数まで）「初めて種類を足す前に住所と職業が置かれる」を足した。(d) は design の Risks に受け入れとして書いた。tasks 5.2 に初期化の後に足す順を書いた

## R16. 名前を変える口が、種類がその利用者のものかを確かめると書いていない
- 成果物: design.md（D7）/ specs/personal-entities/spec.md
- 根拠: design.md:154 の `POST /attributes/kinds/{id}/names` は `{user_id, name}` を受け、断るのは空・重なる・無い種類だけ。`core.attribute_kind_name` は `kind_id` と `user_id` を別々に持ち（design.md:146-147）、2 つが同じ利用者だという制約も無い。別の利用者の種類の識別子を指すと、その種類に自分の `user_id` の名前の行が積まれうる（いまの名前は `kind_id` ごとの最大の `id` なので、相手の画面の名前が変わる）。D5 が取り消し先について「名乗った利用者の外の主張は壊せない」（design.md:218）とした守りが、種類には無い
- kind: technical
- 提案: D7 に「種類の利用者と要求の利用者が違えば、無い種類と同じに断る」を足し、spec に Scenario を足す（R11）
- 処置: fixed D7 — `(kind_id, user_id)` → `attribute_kind (id, user_id)` の外部キーと、名前を変える口で種類の利用者を確かめることを書いた。spec に SHALL と Scenario「別の利用者の種類の名前は変えられない」を足した

## R17. 取り消し先の検査を「同じトランザクションで行う」だけでは、同時に来た削除の印や消去を防げない
- 成果物: design.md（D5）
- 根拠: design.md:108 は「記録を入れるのと同じトランザクションで行う（取り消し先が同時に消去される競合を避ける）」。いまの取り込みは既定の分離レベルで、`ingest_one` の読み出しは行錠を取らない（`lib.rs:437-443` の `SELECT … FROM core.event WHERE id = $1` に `FOR …` が無い）。同じ形で取り消し先を読むだけでは、読んだ直後に別のまとまりが取り消し先を消去してコミットでき、主張は「消去された主張を取り消す」形で入る。D5 が「消去される競合を避ける」と書いた理由が成り立たない
- kind: technical
- 提案: 取り消し先を `FOR KEY SHARE`（か `FOR SHARE`）で読むと D5 に書く。そうしないなら、競合しても害が無いこと（R18 の読み方で吸収できること）を理由に書き換える
- 処置: fixed D5 — 「同じトランザクションで競合を避ける」を取り下げ、取り消し先は行錠を取らずに読み、競合しても読み出しが消した主張・消去した主張を出さないので害が無い、と理由を差し替えた（spec「消した主張を取り消し先に指せる」と揃う）

## R18. 本文を消去した（削除の印の無い）主張の読み方が決まっていない。ST23 への申し送りは読み出しがそれを扱う前提で書かれている
- 成果物: design.md（D6 / D8）/ specs/personal-entities/spec.md / docs/handoff/ST23.md
- 根拠: D6 の手順 1（design.md:131）は `core.event_live`（`deleted_at IS NULL`）で除くだけ。消去は `raw = ''` / `payload = '{}'` にして `deleted_at` に触れない（`gates.sql:105-109`。D2 の 2 も同じ形）ので、消去した主張は `event_live` に残り、種類も値も取り消し先も読めない。D6 の関数（種類ごとに並べる）と D8 の応答（`kind` ごと・`valid_from` 必須）はこの行の置き場を持たない（読み出しで失敗するか、黙って落とすかが決まらない）。`docs/handoff/ST23.md:23` は「消した主張が取り消していた主張は、読み出しで積んだ主張に戻る（削除の印と同じ）」と書くが、消去については D6 にも spec にも根拠が無い
- kind: technical
- 提案: 読み出しで「解析済みが空の主張は出さず、その取り消しも効かせない」を D6 と spec の SHALL / Scenario に足す（ST19 の錠は消去を通すので、ST23 を待たずに到達しうる）。handoff の記述をそれに合わせる
- 処置: fixed D6 — 手順 1 に「本文を消去した主張（raw が空）も除く」を足し、spec に SHALL と Scenario「本文を消去した主張は出ず、その取り消しも効かない」を足した。docs/handoff/ST23.md をそれに合わせた

## R19. 1 件だけ送って断られた `/ingest` は HTTP 400 を返す。画面が既存の読み出しの形（`!res.ok` で失敗）を写すと、断られた理由が「届かなかった」になる
- 成果物: design.md（D9）/ tasks.md 4.3
- 根拠: `ingest` は 1 件も受け付けなかったとき、1 件ごとの結果の本文つきで 400 を返す（`lib.rs:892-896`）。画面が主張を送るのは常に 1 件（design.md:190）なので、断られたときは必ず 400 になる。既存の画面の読み出しは `if (!res.ok) throw new Error(...)`（`web/src/App.tsx:53`）。D9（design.md:192）は「受理でなければ種別を文に直して出す」と書くが、400 の本文を読むことは書いていない。tasks.md:94-95 の「受理でない応答と通信の失敗の 2 通り」も、受理でない応答を 200 と 400 のどちらで作るかを決めていない
- kind: technical
- 提案: D9 に「400 でも本文の 1 件ごとの結果を読み、`accepted` と `error` で分ける。本文が読めないときだけ通信の失敗とする」を書き、4.3 のテストは受理でない応答を 400 で作ると書く
- 処置: fixed D9 — 「200 でも 400 でも本文の 1 件ごとの結果を読み、読めない・5xx・401・fetch が投げたときだけ届かなかったとする」と種別ごとの文の表を書き、tasks 4.1 / 4.3 に 400 で作る試験を書いた

## R20. `.down.sql` の前提が矛盾していて、戻すと種類の名前が失われる
- 成果物: design.md（D12）/ tasks.md 1.1
- 根拠: design.md:211 は「主張の行が 1 件でもあれば登録簿の行を消さない（外部キーで当たる）ことを前提に、関数・トリガ・2 表を落とす」。外部キー（`core.event.logical_source REFERENCES core.source`。`202609081618_envelope.sql:16`）で `DELETE` が当たると、その文はエラーになり、`psql -v ON_ERROR_STOP=1` で当てる戻し手順は途中で止まる（「消さない」ではなく「戻しが落ちる」）。止まらずに 2 表を落とすと、主張が原文の中で指す種類の識別子（design.md:52）に対応する名前が消え、追記のみにした名前の台帳（C7）が戻らない
- kind: technical
- 提案: 戻し手順は、主張の行が残っていれば登録簿の行も 2 表も残す（`DELETE … WHERE NOT EXISTS (…)` の形）と D12 に書く。tasks 1.1 の検証に、主張の行がある DB で `.down.sql` が rc=0 で終わり、2 表が残ることを足す
- 処置: fixed D12 — 戻し手順は主張の行が残っていれば登録簿の行と 2 表を残す形にし、tasks 1.1 に主張の行がある DB で .down.sql が rc=0 で名前の台帳が残る検証を足した

---

## 観点 4: tasks が検証を持つか

## R21. 依存順: 2.2 / 2.3 は、3.2 / 3.3 で作る種類と `GET /attributes` を前提にしている
- 成果物: tasks.md
- 根拠: 2.2（tasks.md:52-57）は「種類がその利用者にあるか」を検査するので、テストには種類が要る。種類を置くのは 3.2（最初の読み出し）と 3.3（`POST /attributes/kinds`）。2.3（tasks.md:58）は「DB と `GET /attributes` で読み戻す」ので、3.2 が無いと書けない。Scenario「主張した日時といつからが別々に入る」「なしの主張を受け付ける」の THEN は読み出しの欄で確かめる形になっている
- kind: technical
- 提案: 3.1〜3.3 を 2.2 の前に移すか、2.2 / 2.3 のテストは DB に直接種類を入れて DB から読み戻すと書き、`GET` で読み戻す分を 3.2 に移す
- 処置: fixed tasks.md — 章を依存の順（2 章: 解釈・導き方・種類・読み出し → 3 章: 取り込み口の分岐 → 4 章: 画面）に並べ替え、2 章の試験は主張を DB に直接入れると書いた

## R22. 検証が空振りするか、落ちるべきときに落ちないタスクがある
- 成果物: tasks.md
- 根拠:
  - 6.1（tasks.md:114-116）: 検証の `grep -c 'st19-personal-attributes' docs/handoff/ST22.md docs/handoff/ST23.md` は**いま既に**どちらも 1（ハンドオフの項は書き済み）。「実装した錠と一致しているか」を何も見ない。R7 / R18 のずれがあっても通る
  - 5.2（tasks.md:108-110）: `seed.sh` は何度も当てる前提（固定の識別子と `ON CONFLICT`。`tools/seed.sh:27-43`）だが、主張の原文に毎回新しい乱数を入れれば 2 回目で主張が 42 件になり、`== 21` が落ちる。同じ名前の種類を 2 回足せば 400 で `curl -sf` が落ちる。再実行してよいかどうかがタスクに無い
  - 2.4: R2 のとおり、わざと壊しても落ちない
  - 2.1 / 3.1 の `CT attributes::parse` / `CT attributes::view`: cargo の絞り込みはテストのパスの部分一致なので、`attributes.rs` の `mod tests`（既存の置き方。`stay.rs:472`）に置くと `attributes::tests::parse_…` になり 0 本で落ちる。落ちる向きなので空振りはしないが、モジュールの置き方の指定がどこにも無い
- kind: technical
- 提案: 6.1 は ST22 / ST23 の項に書いた条件（列の名前・台帳の条件・読み出しの扱い）を 1 つずつ実装と突き合わせる検証に直す。5.2 は種類と主張を固定の識別子と固定の乱数で入れ、「2 回続けて当てても 21」を検証にする。絞り込みは `attributes::tests::parse` のように置き方ごと書く

観点 4 の人間の確認待ち: 該当なし（確かめた範囲: tasks.md:128-132。この Story には物理的な操作が無く、「違和感」の 1 問だけを確認バッチに渡している。2026-09-14 の決定と合う）。
- 処置: fixed tasks.md — 6.1 を錠の列・台帳の照合・消去した主張の試験の 3 点を実装と突き合わせる検証に直した。5.2 を固定の識別子・固定の乱数・初期化の後に足す順にし、2 回続けて当てても 21 件を検証にした。わざと壊す確かめを 1.2 / 3.3 / 4.1 で落ちる壊し方に直した。CT の絞り込みを `attributes::tests::` / `attributes_tests::` と置き方ごと書いた

---

## 観点 5: Story が要件の現在の本文と一致しているか

- `check_chain.py` は rc=0（再生成との一致を含む）
- ST19.md の「価値」「完了の判定」（ST19.md:49-52）は deep と矛盾しない。Q1 で主張を消せるようになったが、完了の判定は「残っている」「別々に入っている」で、消す前提も消さない前提も置いていない。「書いた日」を画面で押したときだけにした Q2 は、deep.md:57 と D11（design.md:201-205）が「保存と読み出しでは別々に持ち、API と画面の押した後の両方で確かめる」としている
- `satisfies` に無い要件を満たしている点は R13

観点 5: 該当なし（R13 を除く。確かめた範囲: ST19.md / ST22.md / ST23.md / ST24.md の差分、INDEX.md:41-46 / :75、requirements.md の FR-44 / FR-45 / PERM-4 / 扉 #3 / #15 の ★）。
