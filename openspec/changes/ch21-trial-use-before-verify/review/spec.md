# CH21 の spec の検査（spec --draft・第 1 回の先読み。第 3 版）

## 機械の検査（2026-11-20、docs/ch21-upstream の ad69ff50e の上。spec・design・tasks は未コミット）

- `openspec validate ch21-trial-use-before-verify --strict` → `Change 'ch21-trial-use-before-verify' is valid`
- `python3 scripts/check_chain.py .` → `chain: OK (0 件 / 未回収 0 件 / warn 0 件)`（change なので観点 8 は無い）
- `python3 scripts/check_scenarios.py . ch21-trial-use-before-verify` → `scenarios: FAIL (担保なし 51 件 / 名無しの確認待ち 0 件)`。下流の前なので担保が無いのは予定どおり（呼び出し元の指示で指摘にしない）。
  tasks.md が名指しする `# Scenario:` / `== Scenario:` の印は 51 本で、spec の `#### Scenario:` 51 本と名前が一字一句合う（python の正規表現で両方の名前を抜き出して突き合わせた）
- `python3 scripts/review_triage.py . ch21-trial-use-before-verify` → `triage: OK`（指摘 16 件 / 仮決め 0 件 / 要件へ戻すもの 0 件。review/deep.md の分）
- `python3 scripts/spec_overlap.py candidates . ch21-trial-use-before-verify` → `[]`

## R1. 公開の題の台本の「検査を走らせられなければ全部伏せる」と、伏せる形の半分が Scenario を持たず、Task 2.1 の自己試験は gitleaks が無いと期待を変えて緑になる
- 成果物: openspec/changes/ch21-trial-use-before-verify/specs/verify-environment/spec.md・tasks.md
- 根拠: spec.md:170 は伏せる形に「この PC の利用者名・ホスト名・私設網の名前・`.env` の 8 字以上の値・`/mnt/`・`/Users/`・`\\`・IPv4」を挙げ、spec.md:172 は「検査を走らせられない…ときは、全部の題を伏せる」と書く。
  Scenario（spec.md:176-189）が見るのは `/home/`・`C:\`・メール・ホスト名・私設網の IPv4・私設網のホスト名・秘密の形・`@`/`#数字` だけで、利用者名・`.env` の値・`/mnt/`・`/Users/`・`\\`・私設網でない IPv4・「走らせられないとき全部伏せる」は Scenario に無い（Requirement の本文だけが言う）。
  さらに tasks.md:44 は「`tools/check-secrets.sh` が gitleaks を見つけられない木では、飛ばしたことを 1 行出して秘密の形の題だけを外す」と自己試験の期待を変える。spec.md:172 どおりなら、そのとき台本は 4 つとも伏せるはずで、spec.md:184 の「出力は 4 行、伏せは 3 つ」とも合わない。
  検査を走らせられないときに素通しで返す（fail-open）実装でも、自己試験はこの分岐で緑になる。`.env` の値は、tasks.md:40 が「cwd の木の `.env`」、spec.md:170 が「確認バッチの作業ツリーの `.env`」で、処置（harness2）が別の cwd で呼べば何も伏せない。
  同じ中身の写し（spec.md:220-221）は、前の回の索引（design.md:104 の `titles`）から題を写すが、写す題が公開の題の台本を通ったものか（索引に伏せる前と後のどちらを置くか）が決まっておらず、spec.md:224-227 の Scenario は私的な形の無い題（「並びがおかしい」）でしか見ない。
  Q4 は A（loss: exported）で、Q4 の context（deep-questions.json:140）は推奨の守りを「題も…公開に出す前に伏せる検査を通す。…通らなければ「題は手元の結果に」に置き換える」としている
- kind: irreversible
- loss: exported
- 提案: Scenario を足す:「検査を走らせられなければ、全部の題を伏せる」（差し込みで gitleaks の場所を無い所に向け、出力が全部「（題は手元の結果に）」）・「確認バッチの作業ツリーの .env の値と利用者名を含む題は伏せる」（`.env` は自己試験が作る）。
  写した回の Scenario の題に `/home/someone/x` を足し、写した段落でも伏せられていることを見る。
  tasks.md:44 の「秘密の形の題だけを外す」を消し、gitleaks が無い木では自己試験そのものを落とす（CI の `rust` の job は取れる）。`.env` を読む場所は「`--env <ファイル>` か cwd」と台本の口で決め、処置にも渡させる（D15 の 6）
- 処置: fixed D28 仮 —— （旧 D19。Q11 の答えで写す道が無くなり D28 に立て直した）伏せる側に倒した。spec に Scenario「利用者名と .env の値を含む題は伏せる」「検査を走らせられなければ、全部の題を伏せる」を足し、`/mnt/`・`/Users/`・`\\` を「私的な形を含む題は伏せる」に入れた。Task 2.1 は gitleaks を飛ばさない（CI は取れる）。写した題も通し直す（D10・Scenario「写した題も、公開の題の台本を通し直す」）

## R2. Q4 の推奨が名指す「足す試験の名前」が、D15・harness2 への依頼・spec のどこにも無く、処置の Scenario は根拠とログが公開に出ないことを見ていない
- 成果物: openspec/changes/ch21-trial-use-before-verify/design.md・harness2-request.json・specs/verify-environment/spec.md
- 根拠: Q4 の context（deep-questions.json:140）は「直しの commit の本文・Story の issue へのコメント・足す試験の名前も同じ」と書き、review/deep.md R1 の処置もこの 3 つを足したと言う。
  design.md:150（D15 の 6）・harness2-request.json:12（scope）・spec.md:348 は「手順書の問い・違和感の issue・直しの commit の本文・Story の issue へのコメント」までで、試験の名前（と試験の中身に入る文字列）が落ちている。spec.md:348 は Story の issue へのコメントも挙げていない。
  また spec.md:362-365 の Scenario が見るのは `.local/state`・`RESULT.md`・`/home/` の 3 つの字面だけで、候補の根拠（evidence）の文やログの抜き書きが commit・手順書・`gh` の本文に入っても落ちない（spec.md:126-129 の目録の Scenario は根拠の字面「画面の写し a.png」で見ているのに、公開に出る側では見ていない）
- kind: irreversible
- loss: exported
- 提案: D15 の 6・harness2-request.json の scope・spec.md:348 に「足す試験の名前と、試験の中の文字列」と「Story の issue へのコメント」を足す。spec.md:365 の THEN に「用意した候補の根拠の文（自己試験が決めた字面）がどこにも無い」を足す
- 処置: fixed D20 仮 —— Q4 の推奨（公開に出すものを減らす側）のまま、D12 の処置の題の規則に「足す試験の名前と試験の中の文字列」を足し、D15 の 6・harness2-request.json の scope に「足す試験の名前と試験の中の文字列」を足した。受け入れの Scenario は用意した根拠の文がどこにも無いことを見る

## R3. Q12 の推奨の「選別や直しが落ちた回も 1 行書く」が、harness2 への依頼に写っていない
- 成果物: openspec/changes/ch21-trial-use-before-verify/design.md・harness2-request.json
- 根拠: Q12 の points（deep-questions.json:431）「回さなかった回（範囲が空・前の結果を写した）と、選別や直しが落ちた回も 1 行書く」。前半は spec.md:144・:221 にある。
  後半（選別・直しは harness2 の処置）は design.md:143-154（D15 の 1〜9）にも harness2-request.json の what・scope にも無い。処置が落ちた回は、手順書に何も出ず「通らなかった 0」のバッチとして機械が merge しうる（Q3 の context が「前から」の誤りで心配しているのと同じ型）
- kind: technical
- 提案: D15 に 10 として「選別・直し・確かめ直しが落ちたか上限で止まった回は、決まった文言の 1 行を手順書に書く（理由は手元だけ）」を足し、harness2-request.json の what にも写す
- 処置: fixed design.md D15 —— 10 として足し、harness2-request.json の what にも写した

## R4. Q3 の推奨の数（直すのは 3 件まで・同じ候補は 1 回まで）と「迷えば本人へ」が、どの Scenario にも無い
- 成果物: openspec/changes/ch21-trial-use-before-verify/specs/verify-environment/spec.md
- 根拠: spec.md:350 は導出元に Q3（「迷えば本人へ・直すのは 3 件まで・1 回まで」）を挙げるが、spec.md:347-370 の本文にも Scenario にも 3・1・「迷えば本人へ」が無い。数は harness2-request.json:12 にだけある。
  ashiato2 の正典の側では、処置が上限なしで直しても、迷ったものを「前から」に倒しても、どの Scenario も落ちない
- kind: technical
- 提案: どちらかに揃える。(a) 数と「迷えば本人へ」は harness2 の約束と割り切り、spec.md:350 の導出元から Q3 を外して「Q3 の数は harness2 への依頼（design D15 の 3）が持つ」と書く。
  (b) real-run の Scenario に「同じ候補を 2 度目に見つけても直さない」を足す。R5 の方針と合わせて選ぶ
- 処置: fixed specs/verify-environment/spec.md —— (a) にした。最後の Requirement で、数と「迷えば本人へ」は harness2 への依頼（D15 の 2・3）が持ち、ashiato2 の正典は約束しないと書いた

## R5. 最後の Requirement は、harness2 の確認バッチの振る舞いを ashiato2 の正典に約束させ、まだ無い台本の名前を書いている。Scenario の 1 つは条件つきで空でも通る
- 成果物: openspec/changes/ch21-trial-use-before-verify/specs/verify-environment/spec.md
- 根拠: spec.md:346-349 は「THE SYSTEM SHALL 候補が、…別の席の選別を経て…Story のブランチで直って…手順書の問いになり…issue になる」と、選別・直し・問い・issue（すべて harness2 の `verify_tsukau.py` の振る舞い。design.md:131-154）を ashiato2 の SHALL で書く。
  archive すると `openspec/specs/verify-environment/spec.md` に残るが、それを確かめるのは CI に載らない `tools/tsukau/real-run.sh`（本物の claude・Sandbox）だけで、harness2 が後で処置を変えても ashiato2 の正典は落ちない。
  spec.md:346・:368 は `.harness2/scripts/verify_tsukau.py` という、まだ無く、design.md:178 自身が「別の名前・別の入口で入ったとき」を反転条件に挙げている名前を Scenario に固定している（観点 3: specs に実装の名前）。
  spec.md:360 の THEN は「判断が要ると選別された候補が**あれば**…問いがあり」で、そう選別された候補が 0 件なら何も見ずに通る。同じ THEN に「前から」→ issue の主張を束ねている（観点 2）。
  spec.md:355 は、席（LLM）が崩した文言を見つけ、選別の席（LLM）が「直す」に分けることを前提にするが、design.md:176 の「本物の候補が出なければ用意した候補で通す」の逃げ道は tasks.md:168 では「前から」の側にしか書かれていない
- kind: conflict
- 提案: 最後の Requirement を「ashiato2 は処置との契約（目録・公開の題の台本・`TSUKAU_RECHECK`）を守り、処置まで通ることを実地の台本 1 本で受け入れる」に絞り、選別・直し・問い・issue の SHALL は harness2-request.json（と harness2 の試験）に置く。
  台本の名前は「harness2 の確認バッチの処置の入口（依頼 D15 の 1）」と書き、名前は real-run.sh の頭の 1 か所に持つ。spec.md:357-360 を「判断が要る候補は問いになる」と「前からの候補は issue になる」の 2 本に分け、どちらも用意した候補（選別の席に渡る前に種類が決まっている形）で必ず 1 件ずつ起こす。
  直す側にも「席が崩した文言を見つけなければ、用意した候補を足す」を tasks.md:168 に書く
- 処置: fixed D18 仮 —— 最後の Requirement を「処置との契約を守り、処置まで通ることを実地の 1 回で受け入れる」に絞った。台本の名前は spec から外し（design D15 の 1 と tasks だけ）、用意した候補を 2 つ足して条件つきの THEN を無くした

## R6. 実地の確かめで「直す」の道は、偽の feat/ch99 に Story の作業ツリーも PR の CI も無いので、依頼の 7（remote と gh の差し替え）だけでは通らない
- 成果物: openspec/changes/ch21-trial-use-before-verify/design.md・harness2-request.json・tasks.md
- 根拠: 直しの道は統合の CI の直しと同じ（design.md:146、D15 の 3）。その台本 `.harness2/scripts/verify_ci_route.py:15-19` は「載せる先は、その Story の作業ツリー（`git worktree list` で feat/st<NN>-* を checkout しているもの）。作業ツリーが無い・汚れている・手元が origin と違うときは載せない」、:7 は「その Story の PR の CI が緑になるのを待つ（scripts/ci_wait.py）」。
  tasks.md:166-168 の real-run は bare repo と merge は作るが、`feat/ch99-tsukau-real` の作業ツリーも、PR も、その CI の結果を返す偽の `gh` も作らない。`docs/changes/CH99.md` も作らないが、表題の文書そのものが無いときの範囲の行は spec.md:29-30 に決まっていない（spec.md:30 は価値の段落が無いときだけを決める）。
  このままでは処置は D15 の 3「載せられなければ本人に聞く」に倒れ、spec.md:355 の「一時の bare repo の feat/ch99 に直しの commit があり」は満たせない。harness2-request.json:12 の「push 先の remote と gh をその場で差し替えて単独で走らせられること」も、この 2 つ（作業ツリーと PR の CI）に触れていない
- kind: technical
- 提案: tasks.md の 10.1 に、`feat/ch99-tsukau-real` の作業ツリーを一時に作る・`docs/changes/CH99.md` を偽の木に置く・偽の `gh` が `pr list`/`pr checks` に緑を返す、を書く。
  harness2-request.json の scope の 7 に「Story の作業ツリーと PR の CI の待ちも差し替えられること（または、実地の口では待たないこと）」を足す
- 処置: fixed tasks.md 10.1 —— 偽の Story の作業ツリー・`docs/changes/CH99.md`・`pr checks` に緑を返す偽の `gh` を足し、D15 の 7 に Story の作業ツリーと CI の結果の差し替えを足した

## R7. Task 10 は harness2 の処置が入るまで検証が通らず、CH21 の PR はそれまで main に入らない。「それまでの確認バッチでは席を呼んで段落と目録を書く」は成り立たない
- 成果物: openspec/changes/ch21-trial-use-before-verify/tasks.md・proposal.md・design.md・docs/changes/CH21.md
- 根拠: tasks.md:165 は「`.harness2/scripts/verify_tsukau.py` が無ければ…rc≠0（飛ばさない）」、:175 の検証は `tools/tsukau/real-run.sh` rc=0。harness2 の処置は Task 9.3 が依頼を置いてから改良グラフで作られるので、同じ下流の流れの中で Task 10 の検証は落ち続ける。
  `.harness2/docs/flow-gates.md` と `.harness2/skills/story/task.md` を `外の待ち|harness2 の改良を待|待つ Task|requires.*HI` で探したが、Task が外の repo の完了を待つ仕組みは見つからなかった。
  下流の次の Task は `.harness2/graph/harness_graph/story.py:1386-1399` が「impl の未完がある Task のうち、A を積んで保留にしたもの以外の最初の 1 つ」と決め、保留は本人の答え待ちの間だけ（:1390）。最後の段（PR を作る final）へは、回せる Task が無くなってから進む（:1402-1411）。
  だから Task 10 が残る間、CH21 の PR は作られない。`完了の判定` を読んで merge と archive の間で待つ関門も `.harness2/scripts`・`graph` に見つからなかった（読むのは stories_json.py・story_brief.py・verify_checklist.py だけで、verify_checklist.py:327 は「完了の判定からは問いを作らない」）。
  一方 proposal.md:45・design.md:137・:182 は「それまでの確認バッチでは、席を呼んで段落と目録を書くところまでが動く」と書き、Q1 の推奨の代償（deep-questions.json の Q1 options[0].cost）も「CH21 は席を呼んで結果を載せるところまでしか動かない」と本人に見せている。
  `tools/verify-prep.sh` の変更が main に入らなければ確認バッチは席を呼ばないので、本人が読む代償より実際の代償が大きい（harness2 の処置が入るまで、席は確認バッチで 1 度も回らない）。docs/changes/CH21.md の完了の判定「それまで CH21 は archive しない」も、merge と archive の間で待つ前提に立っている
- kind: conflict
- 提案: 本人が答える前に形を決める。(a) Task 10 を CH21 から外し、harness2 の処置が入った後に始める別の change（CH21 を requires に持つ）に移す。CH21 の完了の判定は Task 8・9.3 まで。Q13 の推奨の言い方（「処置まで実地で通す」）が変わるので、Q13 の推奨の detail を直してから本人に見せる。
  (b) CH21 のまま待つなら、Q1・Q13 の代償に「harness2 の処置が入るまで、席は確認バッチで回らない（CH21 の PR が main に入らない）」を書き、proposal.md:45・design.md:137・:182 を直す
- 処置: fixed D18 仮 —— (b) にした（Q13 の推奨は「待つ」のまま）。代償を「harness2 の処置が入るまで CH21 は main に入らず、その間は確認バッチで席が回らない」に直し、proposal・design D14・Risks・deep-questions.json の Q1 と Q13 の代償（と Q13 の 2 つ目の案の得）も同じに直した。本人は正しい代償で Q13 を選べる

## R8. 候補の番号が 3 通りある（手順書は題の並び、`TSUKAU_RECHECK` は candidates.jsonl の行目、席の台本は `id` の C<n>）
- 成果物: openspec/changes/ch21-trial-use-before-verify/specs/verify-environment/spec.md・design.md
- 根拠: spec.md:140 は手順書の番号を「`候補 1` から」、design.md:113 は「候補 <n>。`candidates.jsonl` の何行目か」とし、spec.md:140-142 は「候補の番号つきの題の一覧」を最後の行の `titles` から組む。
  spec.md:242 は `TSUKAU_RECHECK` の `candidate` を「`candidates.jsonl` の `candidate` 行目」とする。ところが `.harness2/scripts/tsukau.py` は、`titles` を `result["candidates"]`（壊れた行と題の空の行を飛ばしたもの。:168-185・:575）から組み、候補ごとに `id`（`C<n>`。:314）を振る。
  壊れた行か空行が 1 つあれば、手順書の「候補 2」・`titles` の 2 つ目・`candidates.jsonl` の 2 行目・`C2` が別の候補を指しうる。Q4 は公開に「候補の番号」を出し、処置は番号で直す候補を名指すので、取り違えると別の候補を直したと公開に書く
- kind: technical
- 提案: 番号は席の台本の `id`（`C<n>`）に揃える。手順書は「候補 C1」、`TSUKAU_RECHECK` の項目は `{"target","run","id"}`、題は `result.json` の `candidates` から `id` で引く。Scenario「確かめ直しは…」の GIVEN に壊れた行を 1 つ混ぜる
- 処置: fixed design.md D11 —— 番号は席の台本の `id`（`C<n>`）に揃えた。`TSUKAU_RECHECK` の項目は `{target, run, id}`、題は `result.json` の `candidates` から引く。確かめ直しの Scenario の GIVEN に壊れた行を混ぜた

## R9. 1 回の呼び出しの SIGTERM（1,560 秒）は、立てる上限と席の時間に片付け（300 秒）を数えていないので、正しく終わる途中の回を「打ち切った」にして結果を捨てる
- 成果物: openspec/changes/ch21-trial-use-before-verify/specs/verify-environment/spec.md・design.md
- 根拠: spec.md:72・design.md:88 は「900 + 600 + 60 = 1,560 秒で SIGTERM、片付けを 360 秒まで待つ」。席の台本は席の時間の後に、ログを写し（`.harness2/scripts/tsukau.py:548-556`、1 つ 200 MB まで）、`down` を最長 `down_timeout_s`（既定 300。:558-568）待つ。
  立てるのに 890 秒かかった回は、席の 600 秒の後の片付けの途中で SIGTERM を受ける。台本は SIGTERM で印を立てるだけなので（:647-649）、`down` を終えて `stop` が `time` の正しい最後の行を出す。
  それでも spec.md:144 は「打ち切ったときは（最後の行の JSON に依らず）「時間の上限で打ち切った」」、spec.md:115 は `not_run` にするので、席が残した候補は目録で `run` を持たず、処置にも手順書の件数にも届かない
- kind: technical
- 提案: SIGTERM までを 900 + 600 + 300 + 60 = 1,860 秒、待ちを 360 秒（合わせて 2,220 秒）にし、準備の全体の上限（6,540 秒）も数え直す。または「打ち切った後でも、最後の行が読めて `stop` が `done`/`time`/`tokens` なら `ran` として件数と題を写す」に変え、Scenario を足す
- 処置: fixed design.md D8 —— SIGTERM まで 1,860 秒（片付け 300 を数える）・待ち 360・合わせて 2,220、全体 7,140 に直し、打ち切った後でも最後の行が読めて終えていれば写す Scenario を足した

## R10. PC の相手でインストーラに渡す合言葉が決まっていない
- 成果物: openspec/changes/ch21-trial-use-before-verify/specs/verify-environment/spec.md・tasks.md
- 根拠: 画面の相手は「その場で作った乱数の合言葉」「`--repo` の `.env` は読まず」（spec.md:312-313）と Scenario（:319-327）を持つが、PC の相手の Requirement（spec.md:256-266）は合言葉に触れない。
  tasks.md:131 は「合言葉の渡し方は `tools/release/install-check.sh` の `install_with` と同じ」で、渡し方（標準入力に 2 行）だけを決め、値の出所を決めていない。
  確認バッチから呼ぶ回は `--repo` が `.env` の無い使い捨ての作業ツリー（spec.md:68-69・D9）なので本物は届かないが、手で呼ぶ `/tsukau`（spec.md:7。`--repo` は本人の checkout で、本物の `.env` がある）では、env-hook が `--repo` の `.env` から合言葉を読めば、本物の合言葉が Sandbox と案内（席が読み、モデルへ渡る）に入る。
  画面の相手はこの経路を spec.md:313 で塞いでいるが、PC の相手には同じ文が無い。docs/changes/CH21.md の「壊してはいけないもの」は本人の合言葉を席に触らせないことを挙げている
- kind: technical
- 提案: PC の Requirement に「その場で作った乱数の合言葉で入れ、`--repo` と確認バッチの作業ツリーの `.env` を読まない」を足し、Scenario「PC の相手は、本人の合言葉でなく、その場の合言葉で入れる」（偽の `wsb exec` が受けた標準入力に `.env` の値が無い）を足す
- 処置: fixed design.md D4 —— その場の乱数の合言葉で入れ、`.env` を読まないことを Requirement と Scenario「PC の相手は、本人の合言葉でなく、その場の合言葉で入れる」に足した

## R11. 時間の上限の数（1,560・360・6,540・2,400 秒）が、どの Scenario にも無い
- 成果物: openspec/changes/ch21-trial-use-before-verify/specs/verify-environment/spec.md
- 根拠: spec.md:71-72・:199 は 2,400・1,560・360・6,540 秒を SHALL で書くが、Scenario（spec.md:101-109・:208-211）は自己試験で 2 秒に縮めた値で見るだけで、本番の値を見るものが無い。tasks.md:21 は台本の頭に定数で置くとだけ言う。
  実装が 1,560 を 600 にしても spec は落ちない（D8 は仮でない決定。2,400 は D5（仮））
- kind: technical
- 提案: Scenario を 1 本足す:「打ち切りの秒数は、立てる上限・席の時間・片付けの和で決まる」（`verify-batch.sh --print-limits` の出力か、台本の頭の定数を読んで 1560/360/2400/6540 と、`tools/verify-prep.sh` の席の部分の既定 6540 を見る）。R9 で値を変えるならその値で
- 処置: fixed specs/verify-environment/spec.md —— Scenario「打ち切りの秒数は、立てる上限・席の時間・片付けの和で決まる」（`--print-limits`）を足した

## R12. Q9 の推奨の「画面も触る」に、席が PC の画面を操作する口が無く、画面の写しの口も Scenario を持たない
- 成果物: openspec/changes/ch21-trial-use-before-verify/design.md・specs/verify-environment/spec.md
- 根拠: Q9 の推奨は「窓を開いて画面も触る」（deep-questions.json の Q9）。design.md:60 の `tools/tsukau/pc.sh` は `exec <PowerShell の文>` と `shot <置き場>` の 2 つだけで、クリック・入力の口が無い。
  spec.md:263 は案内に「コマンドを走らせて出力を受け取る口と画面の写しを撮る口」を書かせるだけで、`pc.sh` の `exec` が出力を返すこと・`shot` が写しを撮れることはどの Scenario も見ない（spec.md:305-307 の本物の確かめも healthz とプロセスの有無だけ）。
  `pc.sh` の振る舞い（席が PC に辿り着く唯一の口）は design.md:60 にだけあり、archive で正典から落ちる（観点 3）
- kind: technical
- 提案: design D4 に操作の口（例: `pc.sh click <x> <y>`・`pc.sh type <文>`。`ExistingLogin` の PowerShell から SendInput）を決めるか、「操作は PowerShell の UI Automation を `exec` で」と案内に書くと決める。
  spec の PC の Requirement に `pc.sh` の口を書き、Scenario を足す:「`pc.sh exec` は Sandbox の中の出力を標準出力に返す」（偽の `wsb`）・本物の確かめに「`pc.sh shot` で画面の写しが 1 枚撮れる」
- 処置: fixed design.md D4 —— 操作は `exec` から UI Automation と `SendKeys`、案内に例を載せると決めた（反転条件つき）。`pc.sh exec` の Scenario を足し、案内の Scenario に `pc.sh shot` と操作の仕方を足した

## R13. Requirement の本文だけが言い、Scenario が言っていないことがある
- 成果物: openspec/changes/ch21-trial-use-before-verify/specs/verify-environment/spec.md
- 根拠:
  - spec.md:71「PC の配る zip の置き場を絶対パスで渡させる（`ASHIATO_TSUKAU_PC_ZIPS`）」—— 画面の置き場は spec.md:91-94 が見るが、PC の置き場を見る Scenario が無い
  - spec.md:69「（harness2 を）繋げなければ、その相手の席を呼ばない」と spec.md:143 の「台本が無い・harness2 を繋げなかったときは「席を呼べなかった」」—— Scenario が無い
  - spec.md:242「項目の無い相手は呼ばない」—— 呼ばない相手の段落の文言と目録の `state` が決まっていない（`skipped`・`copied` のどれでもなく、spec.md:115 の 4 つにも当たらない）。確かめ直しの結果を D10 の索引に書くかも決まっていない
  - spec.md:314「渡されていなければ（手で呼んだとき）`<state>/tree` で組む」・spec.md:315 の案内の「その回の画面の合言葉・本物のブラウザで開いて写しを撮るコマンド・サーバのログの場所」—— Scenario が無い（:334-337 は URL と「触らない」だけ）
  - spec.md:27「`web/index.html`・`web/public/`」—— 振り分けの Scenario（:34-52）が使わない
- kind: technical
- 提案: 上のそれぞれに Scenario を 1 本ずつ足す。確かめ直しで項目の無い相手は「回さなかった（直した候補が無い）」と `state: skipped` に決め、Requirement と Scenario に書く
- 処置: fixed specs/verify-environment/spec.md —— PC の置き場の絶対パス・harness2 を繋げなかったとき・確かめ直しで項目の無い相手（`skipped`・「回さなかった（直した候補が無い）」）・手で呼んだときに別の木で組む・案内の合言葉と写しのコマンドとログ・`web/index.html` と `web/public/` の振り分けに Scenario を足した

## R14. 1 つの Scenario が 2 つ以上の主張を束ねている
- 成果物: openspec/changes/ch21-trial-use-before-verify/specs/verify-environment/spec.md
- 根拠: spec.md:149-153（段落の形と、パスが無いこと。AND）・:229-232（main が進んだ回と PR のコミットが変わった回）・:294-298（ログを写してから止める順と、自分の id だけを止めること。AND）・
  :319-322（起動の場所・`.env` の中身・`--repo` が変わらないこと）・:329-332（渡した置き場が届く回と、無い回に断ること）・:357-360（R5）。どれも片方だけが通っても、印の試験の名前は同じで緑になる
- kind: technical
- 提案: それぞれ 2 本に分ける（例:「main が進めば呼び直す」「PR のコミットが変われば呼び直す」）。tasks.md の印の数（4.1 の 23・7.1 の 15）も合わせて直す
- 処置: fixed specs/verify-environment/spec.md —— 段落の形と手元のパス・main と PR・ログの順と自分の id・起動の場所と --repo・置き場が届く回と無い回、を 2 本ずつに分け、tasks の印の数を直した（最後の Requirement は R5 で作り直した）

## R15. 第 2 版の Task 番号が 2 か所に残っている
- 成果物: openspec/changes/ch21-trial-use-before-verify/tasks.md・design.md
- 根拠: tasks.md:22「本物の Sandbox を使う確かめ（Task 1・7・10）」。第 2 版（`git show e2c9031f8^:…/tasks.md`:19）は「Task 1・7・8」で、第 3 版で本物の Sandbox を使うのは Task 8（8.1）と Task 10。Task 1 は組み立てだけ、Task 7 は偽の `wsb` の自己試験。
  design.md:73「網を切った Sandbox で…通るか…Task 7 で確かめ」も第 2 版の番号（第 2 版の Task 7 が本物の Sandbox）で、第 3 版では Task 8（tasks.md:149 が同じことを書く）
- kind: technical
- 提案: tasks.md:22 を「Task 8・10」に、design.md:73 を「Task 8 で確かめ」に直す
- 処置: fixed tasks.md —— Global Constraints を「Task 8・10」に、design D6 を「Task 8 で確かめ」に直した

## R16. Task 8 が使う「Task 1 で組んだ配る zip」の置き場が HEAD のコミットで決まり、Task 8 の時には見つからない
- 成果物: openspec/changes/ch21-trial-use-before-verify/tasks.md
- 根拠: tasks.md:30 は build-pc.sh の既定の置き場を `dist/tsukau-pc/<HEAD のコミット>/`、tasks.md:129 は env-hook がそこを探すとする。Task 1 の検証（tasks.md:32）は `$TMPDIR/pc` に組み、verify-run の `TMPDIR` は検証ごとに消える。
  Task 2〜7 の commit で HEAD が進むので、tasks.md:147「Task 1 で組んだ配る zip を置いて」は、既定の置き場でも Task 1 の検証の置き場でも見つからず `UNAVAILABLE=` になる。`real-check-pc` が組み直すのか、置き場をどう渡すのかが書かれていない
- kind: technical
- 提案: tasks.md:147 に「`real-check-pc` は始めに `tools/tsukau/build-pc.sh` で一時の置き場に組み、`ASHIATO_TSUKAU_PC_ZIPS` で渡す」と書く（組み立ての時間の分、検証が長くなることも書く）
- 処置: fixed tasks.md 8.1 —— 始めに build-pc.sh で一時の置き場に組んで `ASHIATO_TSUKAU_PC_ZIPS` で渡すと書いた

## R17. build-pc.sh の写しの除外に `.env` と `dist/` が無い
- 成果物: openspec/changes/ch21-trial-use-before-verify/tasks.md
- 根拠: tasks.md:28 の `rsync -a --delete` の除外は `target`・`.git`・`node_modules`・`web/dist`・`collector-android/local.properties` だけ。verify-batch.sh は確認バッチの作業ツリー（tools/verify-prep.sh:26-30 が main の checkout の本物の `.env` を写した木）で build-pc.sh を呼ぶ（tasks.md:66）ので、本物の合言葉が Windows 側の `C:\dev\ashiato2-tsukau-build\src` に写る。
  `dist/` にはその回の `ashiato-server`・`web/`・APK と、組んでいる最中の `dist/verify-<tag>/tsukau-pc` 自身があり、毎回写して組み立ての時間（上限 2,400 秒）を食う
- kind: technical
- 提案: 除外に `.env`・`.env.*`・`dist/` を足し、自己試験か 1.1 の検証で「写した木に `.env` が無い」を見る
- 処置: fixed tasks.md 1.1 —— 除外に `dist/`・`.env`・`.env.*` を足し、検証で写しの木に `.env` と `dist` が無いことを見る

## R18. Task 5.1 と 6.1 の検証が、足した印の数を数えていない
- 成果物: openspec/changes/ch21-trial-use-before-verify/tasks.md
- 根拠: 5.1（tasks.md:106）の検証は `--self-test` rc=0 と `bash -n` だけで、5.1 が足す 3 本の印（tasks.md:103-105）を数えない（4.2 の後で 25、5.1 の後は 28 のはず）。
  6.1（tasks.md:124）も `--self-test` rc=0 と `tsukau.py check` だけで、8 本の印を数えない（7.1 の 15 で後から数えるが、6.1 の時点では 0 本でもチェックできる）
- kind: technical
- 提案: 5.1 に `bash -c 'test "$(grep -c "^ *# Scenario: " tools/tsukau/verify-batch.sh)" -ge 28'` rc=0、6.1 に同じ形で `tools/tsukau/env-hook.sh` が 8 以上を足す
- 処置: fixed tasks.md 5.1 —— 5.1 に 37 本以上、6.1 に 11 本以上の数えを足した

## R19. 「壊してはいけないもの」は fix の道を「CH21 は触らない」とするが、harness2 への依頼の 9 はその fix の後の振る舞いを変える
- 成果物: docs/changes/CH21.md・design.md・harness2-request.json
- 根拠: docs/changes/CH21.md の「壊してはいけないもの」2 つ目は「本人が手順書で「通らなかった」と答えた後の直し（確認バッチの `fix`）…（harness2 の確認バッチ。CH21 は触らない）」。
  design.md:154（D15 の 9）と harness2-request.json の what は「本人の「通らなかった」の後の fix の後にも、席の結果の段落を手順書に差し替える（今は録画などだけを差し替える）」で、CH21 の依頼で fix の後の手順書が変わる
- kind: conflict
- 提案: CH21.md の 2 つ目を「fix で直す道と、違和感の問いを issue にする道は残す（CH21 の依頼は fix の後に席の段落を差し替えることだけを足す。D15 の 9）」に直す
- 処置: fixed D15 仮 —— docs/changes/CH21.md を 「fix の道と違和感の issue の道は残す。依頼が足すのは fix の後に席の段落を差し替えることだけ（D15 の 9）」に直した

## 観点ごとの該当なし・確かめた範囲

- 観点 1（上の R1〜R4・R7・R12 のほか）: Q1（spec.md:111-134・design D14）・Q2（spec.md:347・D15 の 3）・Q5（spec.md:7・:18-21）・Q6（spec.md:23-62。依頼を狭める点は D2 と proposal の持ち越しにある）・
  Q7（spec.md:71・:96-99）・Q8（spec.md:261・:279-282。外につなぐ案に変わったときの直し方は design.md:74）・Q9 の窓（spec.md:262・:284-287）・Q10（spec.md:6・:67・:79・:242）・Q11（spec.md:217-237。D10（仮）は Q11 の「コミット」を木に読み替えた理由を持ち、台帳に載る）・
  Q12 の前半（spec.md:142-145・:155-164）・Q13（spec.md:344-370・tasks Task 10・CH21.md の完了の判定）は、推奨と同じことを言う Scenario がある。当初案を覆した印・要件へ戻すものは deep.md に無い
- 観点 3（上の R5・R12 のほか）: proposal.md の Capabilities（`verify-environment` を MODIFIED・Requirement 11 + 1）と `specs/` のディレクトリ・CH21.md の `capability` は合う（spec の Requirement は 12 本）。
  INDEX.md の capability 表は Story だけを載せ、走っている CH19・CH22・CH23 も載っていないので、CH21 が無いのは慣例どおり。design の D のうち観測できるもの（D2・D3・D6・D7・D10・D11・D12・D14・D16・D17）は spec に写っている（D4 の `pc.sh` を除く。R12）
- 観点 5: CH21.md の「完了の判定」は空でなく、deep の推奨・spec・tasks（Task 8・9.3・10・人間の確認待ち）と合う（R7 の待ちの仕組みを除く）。「依頼（逐語）」の範囲（差し込み口・画面と PC・処置の経路・止めない・公開に出さない・席の中身を変えない）を specs は越えていない。
  依頼の「fix で直す」を ci_fix の道に変えるのは Q2（premise）が本人に聞いている。第 2 版の古い参照は R15 の 2 か所のほかに見つからなかった（spec・design・tasks の Q1〜Q13・review/deep.md の R1〜R16 の番号は第 3 版の中身を指す。「issue も作らない」は席を呼ぶ台本についてだけ言い、第 3 版の依頼と食い違わない）。
  provisional.jsonl の 9 件（D1・D3・D5・D7・D9・D10・D12・D16・D18）は design の（仮）の 9 件と合う。lane.json の `paths` は第 1 版の `tools/trial-use/**` のまま。`.harness2/scripts/lanes.py` の冒頭（:1-30）と :329-342 では、申告の paths は軽い路（same / look / bug / recipe）に当たるかの裏取りにだけ使い、`reason: none` の申告はそのまま通常の路にする。通常の路で触るファイルを paths で縛る箇所は `paths` の grep で見つからなかったので指摘にしていない
- 観点 6: 該当なし（`spec --draft` の第 1 回の先読みで、本人の答えはまだ無い。answers.md は書かない）

## 観点 7: 走っている change との前提関係
- ch22-soak-install-online-ci / PR の CI は、インストールと更新の試験の後に、配る形を外につながる状態で初めから入れて 5 分置く: 前提（CH21 の distribution の MODIFIED が、名指しの (2) でこの Requirement の名前を引いている。CH22 の ADDED が無い正典に入ると、指す先の無い名指しになる）
- ch22-soak-install-online-ci / 更新を確かめる問い合わせは、版の一覧を取る決まった形だけを配布元へ送る: 前提（CH21 の MODIFIED は、CH22 の delta の本文を写して (3) を足したもの。`git show origin/main:openspec/changes/ch22-soak-install-online-ci/specs/distribution/spec.md` との差は、SHALL の 1 文の「2 つ」→「3 つ」と (3)、導出元の CH21 の 1 行だけで、11 の Scenario は一字一句同じ。CH22 の下流が本文を変えれば、CH21 の写しも書き直しになる）

requires は `docs/changes/CH21.md` に `[CH22]` があるので、R は立てない。

## 第 2 版（spec --answers Q8,Q11 --iwakan。2026-11-20）

### 機械の検査（2026-11-20、docs/ch21-upstream の 89f553a73 の上）

- `openspec validate ch21-trial-use-before-verify --strict` → `Change 'ch21-trial-use-before-verify' is valid`
- `python3 scripts/check_chain.py .` → `chain: OK (0 件 / 未回収 0 件 / warn 0 件)`
- `python3 scripts/check_scenarios.py . ch21-trial-use-before-verify` → `scenarios: FAIL (担保なし 86 件 / 名無しの確認待ち 0 件)`。下流の前なので、担保が無いのは予定どおり。
  内訳は verify-environment が 84 件、distribution が 2 件（「網に出る試験を走らせる手順は、握手の試験の 1 つだけである」「更新係の外向きの塞ぎを外す手順は、置く段の 1 つだけである」）。
  distribution の 2 件は CH22 の delta と同じ名前で、印は CH22 の下流が置く（CH21 の tasks に印は無い。requires の順で CH22 が先に入る前提）。
  tasks.md の `# Scenario:` と `== Scenario:` の名前は、python で抜き出して突き合わせると、spec の 84 本と一字一句合う。数の下限も合う: public-title.sh 5、verify-batch.sh 8 → 33 → 36 → 40、env-hook.sh 11 → 23 → 33。本物で確かめる口は 6 本
- `python3 scripts/review_triage.py . ch21-trial-use-before-verify` → 書く前は `triage: OK`（指摘 35 件）。R20 以降は処置が無いので落ちる（予定どおり）
- `python3 scripts/spec_overlap.py candidates . ch21-trial-use-before-verify` → 2 件（CH22 の 2 つの Requirement）。上の観点 7 で判定した

## R20. 携帯の相手で「専用の AVD が動いているか」を読む方法が、「ほかの serial を名指さない」Scenario と両立しない
- 成果物: openspec/changes/ch21-trial-use-before-verify/specs/verify-environment/spec.md・tasks.md・design.md
- 根拠: spec.md:406 と :421 は、専用の AVD（`ashiato-tsukau-api35`）のエミュレータが動いていれば断るとしている。tasks.md:186 は AVD の名前を「`getprop ro.boot.qemu.avd_name`。`tools/android-emulator.sh` の `emu_reject_existing_avd` と同じ読み方」で取るとしている。
  ところがその関数（tools/android-emulator.sh:44-58）は、`adb devices` の `emulator-*` すべてに `adb -s "$serial" shell getprop …`（取れなければ `emu avd name`）を打つ。
  一方、spec.md:433 の Scenario は GIVEN に `emulator-5554` を置き、THEN で「`devices` 以外の呼び出しはどれも自分の serial を `-s` に持ち、`emulator-5554` を名指す呼び出しは無い」と言う。tasks.md:15 と design.md:224（D22「`adb devices` だけは名指ししない」）も同じことを言う。
  tasks 9.1 のとおりに作ると、この Scenario は落ちる。Scenario を守ると、adb では AVD の名前を読めない。
  さらに tasks.md:74（4.1）の「打ち切った後の残り」の勘定は「`adb devices` と AVD の名前」で数えるが、Global Constraints（tasks.md:18・:20）には偽の `adb` があるのに、4.1 の偽物の一覧（tasks.md:78）には `adb` が無い。
  9.1 は「AVD が無ければ作る」（tasks.md:188）が、`avdmanager` は偽物の一覧（tasks.md:18・:20・:192）のどこにも無い。手元で走らせた自己試験が本物の avdmanager に触りうる（tasks.md:19 の「本物のエミュレータを使わない」）
- kind: technical
- 提案: どちらかに決めて揃える。
  (a) ほかの serial には読むだけの問い（`shell getprop ro.boot.qemu.avd_name`・`emu avd name`）を許し、spec.md:433 の THEN を「入れる・網を切る・reverse・止める呼び出しは、自分の serial だけを名指す」に書き直す。
  (b) adb を使わず、エミュレータのプロセスの `-avd ashiato-tsukau-api35` で見つける。
  あわせて、4.1 の偽物に `adb`、9.1 の偽物に `avdmanager` を足す
- 処置: fixed D22 仮 —— (b) を採った。専用の AVD が動いているかはエミュレータのプロセスの引数で見て、ほかの端末に adb で問わない（spec の携帯の Requirement と Scenario「専用の AVD のエミュレータが動いていれば、立てずに断る」・design D22・tasks 4.1・9.1）。4.1 の偽物に `ps`・`adb`、9.1 と Global Constraints の偽物に `avdmanager`・`ps`・`java` を足した

## R21. 違和感の本体「スマホも操作する」を、本物のエミュレータで確かめる Scenario が無い
- 成果物: openspec/changes/ch21-trial-use-before-verify/specs/verify-environment/spec.md・tasks.md
- 根拠: 本人の補足（逐語。deep.md:100）は「スマホも操作するようにしてください。エミュレータとかで」。
  本物の Scenario（spec.md:465-467）と Task 10.1（tasks.md:208）が見るのは、`UNAVAILABLE=` が無いこと・外に届かないこと・サーバが応えること・プロセスがあること・片付けの 5 つで、席が端末を操作できるかは見ていない。
  本物で使う口は、網の確かめの `phone.sh shell`（tasks.md:208）だけで、`shot`・`ui`・`input` は本物では通らない。
  操作の口（`phone.sh shell`・`shot`・`ui`、`input tap`。design.md:253-254・D25）のうち、Scenario で振る舞いを見るのは偽の `adb` での `shell` だけ（spec.md:445-448）。`shot` と `ui` は、案内に文字があるかを見るだけ（spec.md:453）。
  画面を出さない（`-no-window`）エミュレータで `exec-out screencap` と `uiautomator dump` が動くこと（design.md:225 が前提にしている）は、premises.md にも行が無い
- kind: technical
- 提案: 本物の Scenario を 1 本足す:「本物のエミュレータで、`phone.sh shot` が PNG を 1 枚撮り、`phone.sh ui` が携帯の収集アプリのパッケージ名を含む XML を返し、`phone.sh shell 'input keyevent KEYCODE_HOME'` が rc=0 で終わる」。
  これを Task 10.1 の `echo "== Scenario: …"` に足す
- 処置: fixed tasks.md 10.1 —— 本物の Scenario「本物のエミュレータで、席の口で端末を操作できる」（`phone.sh shot`・`ui`・`input`）を spec に足し、10.1 の `== Scenario:` と検証に足した。`-no-window` で写しと UI の木が取れることを premises.md に確かさ 1 で足した

## R22. Q8 の守り「塞いだことを確かめてから渡す」が、待ち受けの無い port に当てると、何も塞がなくても通る
- 成果物: openspec/changes/ch21-trial-use-before-verify/specs/verify-environment/spec.md・design.md・tasks.md
- 根拠: spec.md:331・design.md:82・tasks.md:153 は「私設網の宛先と手元の網の宛先へ TCP でつなぎ、届かないこと」とだけ書く。どの port に当てるか、塞ぐ前に届くことを確かめるか（対照）が決まっていない。
  premises.md:30 自身が、第 2 版の実測で「手元の網の <宛先>:80 は届かなかった（その宛先に何も無いだけかもしれない）」と書いている。待ち受けの無い port に当てれば、塞ぐ台本が何もしなくても「届かない」になる。
  偽の `wsb` の Scenario（spec.md:349-367）は決まった答えを返すだけで、本物の Scenario（spec.md:401）も同じ当て方で「届かず」を見る。このため、塞ぐ台本が空でも Task 8 は緑になりうる。
  Q8 は失う側のある問い（deep-questions.json の Q8 は `kind: irreversible`・`loss: exported`）で、選ばれた案の得は「塞げたことを確かめてから渡す」こと
- kind: irreversible
- loss: exported
- 提案: 確かめを「塞ぐ前に同じ宛先と port に届く（つながるか、拒否が返る）→ 塞いだ後は時間切れ」の対にする。塞ぐ前に届かない宛先しか無ければ「確かめられない」として全部塞ぐ側（さらに `UNAVAILABLE=`）へ倒す。
  Requirement に書き、Scenario を足す:「塞ぐ前の確かめが届かなければ、確かめられないとして全部塞ぐ」（偽の `wsb` の答えで）。拒否（届いた）と時間切れ（塞がれた）を分けて読むことも書く。
  塞ぐ前の確かめは、Sandbox から私設網へ 1 回つなぐことになる。代わりに、待ち受けがあることをホストの側（WSL が自分の宛先と port を見る）で確かめる形もある。どちらにするかは呼び出し元が選ぶ
- 処置: fixed D26 仮 —— 確かめを対照つきにした: その回だけ立てた一時の口（この PC の私設網と手元の網の宛先）に、塞ぐ前は届き、塞いだ後は時間切れになることを見る。塞ぐ前に届かなければ何も入れずに `UNAVAILABLE=`。確かめで届く先は自分の一時の口だけなので失うものは無い（spec の PC の Requirement・Scenario「塞ぐ前に一時の口に届かなければ、確かめられないとして何も入れずに断る」・design D6・D26・tasks 7.1・8.1）

## R23. 塞いで通す範囲（私設網・私用の範囲・リンクローカル）の組み立てを見る Scenario が無く、本物の確かめはこの PC の宛先しか見ない
- 成果物: openspec/changes/ch21-trial-use-before-verify/specs/verify-environment/spec.md・tasks.md・premises.md
- 根拠: spec.md:330 は、外向きを既定で塞ぎ、私設網（tailscale の範囲）と手元の網（私用の範囲・リンクローカル）を除いた外だけを通すとしている。design.md:79 は IPv6 の私設網の範囲も挙げる。
  範囲を組むのは Sandbox の中の PowerShell（tasks.md:152「範囲は数から組み、字面で書かない」）だが、自己試験は Linux で走り（tasks.md:18）、偽の `wsb` は走らせた台本の名前を記録するだけなので、範囲の中身はどの自己試験にも通らない。
  本物の確かめ（spec.md:401・tasks.md:177）が当てるのは、この PC の私設網の IPv4（`tailscale ip -4`）・Windows 側の既定のゲートウェイ・この PC の手元の網のアドレスだけ。
  私用の範囲の 1 つや IPv6 の私設網の範囲を除き忘れても、ほかの私設網の機器（本人の携帯など）に届く穴は、どの Scenario でも落ちない。
  premises.md:9-11 の「確かさ 1 のまま本人のサーバへ届く道は無い」は、確かめで当てた宛先についてしか言えない
- kind: irreversible
- loss: exported
- 提案: 通す範囲を env-hook.sh（bash）で数から組んで PowerShell に渡し、`env-hook.sh --print-net-rules` で出せるようにする。
  Scenario「通す範囲は、私設網・3 つの私用の範囲・リンクローカル・マルチキャスト・IPv6 の私設網を含まず、外の宛先の見本を含む」を足す（見本は自己試験が数から組む）。本物の確かめには `tailscale ip -6` も足す。
  premises.md:11 は「当てた宛先には届かないことを毎回確かめる。ほかの私設網の機器への守りは規則（確かさ 1）だけ」に改める
- 処置: fixed D29 仮 —— 通す外の範囲を env-hook.sh が bash で数から組み `--print-net-rules` で出す形にし、Scenario「通す外の宛先は、私設網と手元の網の範囲を含まない」（見本を当てる）を足した。本物の確かめに `tailscale ip -6` を足し、premises.md の冒頭を「当てた宛先は毎回確かめる。ほかの機器への守りは範囲の組み立てと規則まで」に改めた（design D6 にも守りの範囲を書いた）

## R24. 携帯の網の確かめは「外の宛先」だけで、エミュレータからホストの loopback（エミュレータがホストに付ける別名の宛先）へ届かないことを見ていない
- 成果物: openspec/changes/ch21-trial-use-before-verify/specs/verify-environment/spec.md・design.md・premises.md
- 根拠: spec.md:409 は「端末から外の宛先に届かないことを確かめ」、design.md:244 の確かめは `api.github.com` を引くことと外の IP への TCP。本物の Scenario（spec.md:467）も「外の宛先に届かず」だけを見る。
  一方 docs/changes/CH21.md:56 は「携帯を試すエミュレータはどこへもつながない（サーバへは `adb reverse` の道だけ）」と言う。
  エミュレータには、ホストの loopback を指す別名の宛先がある（Android のエミュレータの決まり）。そこには、確認バッチの環境など、ホストで待ち受けるものが居る。`svc wifi disable` と `svc data disable` でこの道も切れるかは確かめておらず、premises.md にも行が無い（premises.md:41 は `adb reverse` の道だけ）。
  当て方も R22 と同じで、待ち受けの無い port に当てれば何もしなくても通る
- kind: technical
- 提案: 確かめに、ホストの loopback の別名の宛先を足す（字面は数から組む）。当てるのはその回の専用環境の port で、塞ぐ前と後の対にする（R22 と同じ）。
  Scenario「網の確かめで、ホストに届くと答えれば、APK を入れずに断る」を足す。premises.md に「`svc wifi disable`・`svc data disable` で、外とホストの loopback への道が切れる」を確かさ 1 で足す（確かめるのは Task 10）
- 処置: fixed D31 仮 —— （旧 D24。R32 で立て直した）携帯の確かめを、切る前にホストの loopback の別名の宛先のその回のサーバの port に届くこと（対照）・切った後に届かないことと既定の経路が無いことの対にした。外へは問わない（問うこと自体が外へ出る）。Scenario「切った後もホストに届けば…」「切った後も外への経路があれば…」「切る前に届かなければ…」を足し、premises.md に `svc` で道が切れるかを確かさ 1 で足した

## R25. premises.md に、携帯の相手が寄りかかる確かめていない前提の行が無く、Task 10.1 が指す行の名前が premises.md に無い
- 成果物: openspec/changes/ch21-trial-use-before-verify/premises.md・tasks.md
- 根拠: design.md:237（D23（仮））は APK の組み立てを `up` の中（立てる上限 900 秒）に置く。PC の zip では、同じ理由で組み立てを `up` の外に出している（design.md:73「入れると `ready_timeout_s`（15 分）に数えられ、`target` が冷えた回は毎回「立てられなかった」になる」）。
  携帯の相手は、毎回まっさらな `<state>/tree` で組む（gradle の `build/` が毎回冷えている）。これに `-wipe-data` の冷えた起動と、専用環境の `stack.sh up` が重なるが、900 秒に収まるかは測っていない。premises.md に行が無く、tasks.md:209 は反転条件に挙げるだけ。
  ほかに、`svc` で端末の網を切れること（D24 が寄りかかる。R24）と、`-no-window` で写しと UI の木が取れること（R21）も行が無い。
  tasks.md:210 は「網を切ったエミュレータで、adb reverse の道だけでサーバに届く」という行に結果を書くとしているが、premises.md:41 の行は「端末の網（Wi-Fi と携帯の回線）を切っても、`adb reverse` の道で…ホストのサーバに届く」で、名前が合わない
- kind: premise
- 提案: premises.md に 3 行を確かさ 1 で足す（携帯の相手は 900 秒の中で立つ／`svc` で外とホストへの道が切れる／`-no-window` で `screencap` と `uiautomator dump` が動く）。どれも確かめ方は「Task 10 で測る」。
  tasks.md:210 の行の名前を premises.md:41 に合わせる。10.1 の検証に、その行の確かさが 5 になったことを grep で見る 1 つを足す
- 処置: rejected: 前提が成り立たない疑いは実物で反証した（行は足した）。spec の段に使い捨ての AVD（同じ image・`-no-window -wipe-data`）で、`svc wifi disable`・`svc data disable` で外への道（`dumpsys connectivity` の既定のネットワークが `none`）とホストの loopback の別名への道が切れ、`adb reverse` の道は通り、画面なしで `screencap`・`uiautomator dump`・`input` が動き、起動が 52 秒であることを確かめた（premises.md の 4 行。3 行は確かさ 5、900 秒の行は起動だけ測った（確かさ 2。R32 で 4 から下げた）。残る APK の組み立ての秒数は Task 10 が測り、収まらなくても env_failed で失うものは無く D22・D23（仮）の反転条件で直す）。`ip route` は切る前から既定の経路を見せないと分かったので、確かめを `dumpsys connectivity` に直した（spec・design D24・D26・tasks 9.1）。tasks 10.1 の行の名前も premises.md と合わせた

## R26. 第 2 版で足した本文のうち、Scenario が言っていないものがある
- 成果物: openspec/changes/ch21-trial-use-before-verify/specs/verify-environment/spec.md・design.md
- 根拠:
  - 目録の `earlier` の `run`（spec.md:150・:275）。harness2 の処置は、この `run` から前の回の判定を引き継ぐ（design.md:179、D15 の 8。harness2-request.json の what）。ところが Scenario（spec.md:281・:286）が見るのは `id` と `tag` だけで、`run` が前の回の実行のフォルダを指すかは見ていない（処置との契約の要）
  - PC: 「外にだけ届かなかったときは続け、案内にそう書く」（spec.md:331）と `INFO_NET=`（tasks.md:156）に Scenario が無い
  - 携帯: 「エミュレータか SDK か JDK が無い」ときに断ること（spec.md:406）と、片付けでその回の compose project を消すこと（spec.md:412。spec.md:458 は `<state>/tree` だけを見る）に Scenario が無い
  - 打ち切った後の残りに専用の AVD のエミュレータを数えること（spec.md:83）。spec.md:131-134 の Scenario は compose project でしか見ない
  - 携帯の段落が「携帯: 」で始まり、画面・PC・携帯の順で並ぶこと（spec.md:173・design.md の D11）。席を回した携帯の段落を見る Scenario が無い（spec.md:202 は「回さなかった」の文言だけ）
  - 携帯の APK に入る資格情報は、`ashiato.apiToken` と `ashiato.userId` である（collector-android/app/build.gradle.kts:194・:196）。録画の形は、`.env` の `API_TOKEN` と `ASHIATO_USER_ID` を `local.properties` に書く（tools/record-env.sh:182 あたり）。
    ところが spec.md:426 の GIVEN は、`--repo` の `.env` に `WEB_PASSWORD` しか置かない。`--repo` の `.env` の `API_TOKEN` を写す実装でも、この Scenario は通る。手で呼ぶ `/tsukau` の `--repo` は本物の `.env` を持つので、第 1 版の R10 と同じ型になる。
    また、資格情報は gradle の property が `local.properties` より先に読まれる（build.gradle.kts:22-28）。spec.md:408 が外しているのは「接続先」だけ
  - design.md:226（D22）は片付けで「収集アプリの送信の記録（`run-as` で読める範囲）」も写すとするが、spec.md:412 と tasks.md:191 は `logcat` だけ。design が spec より多くを言っている
- kind: technical
- 提案: 1 つずつ Scenario を足す。資格情報は、spec.md:426 の GIVEN に本人の `API_TOKEN` と利用者 id の見本を足し、記録と APK の組み立ての引数・`local.properties` のどこにも無いことを見る。とくに `earlier` は「`earlier` の `run` は、前の回の目録の `run` と同じ絶対パスである」を足す。D22 の `run-as` は、spec に足すか design から外すかを揃える
- 処置: fixed specs/verify-environment/spec.md —— Scenario を足した: 「目録の earlier の run は、前の回で席を呼んだ実行のフォルダである」「外にだけ届かなければ、続けて案内に書く」「エミュレータか SDK か JDK が無ければ、何も起こさずに断る」「携帯の片付けは、その回の project と木を消す」「打ち切った後に専用の AVD のエミュレータが残っていれば、片付けに残りがあると書く」「携帯の段落は「携帯: 」で始まり、画面・PC・携帯の順に並ぶ」「携帯の相手の APK に、本人の資格情報は入らない」（`API_TOKEN`・利用者 id を GIVEN に置き、資格情報は `-P` で渡して gradle.properties に負けない形を Requirement と design D23 に書いた）。D22 の `run-as` は design から外し logcat だけに揃えた

## R27. 新しく足した携帯と PC の Scenario が、R14 と同じく 2 つ以上の主張を束ねている
- 成果物: openspec/changes/ch21-trial-use-before-verify/specs/verify-environment/spec.md・tasks.md
- 根拠:
  - spec.md:425-428 の主張は 4 つ（`<state>/tree` で組む・接続先・本人の合言葉が無い・`--repo` が変わらない）
  - spec.md:430-433 は、起動の引数と、自分の serial だけを名指すことの 2 つ
  - spec.md:455-458 の主張は 4 つ（logcat が止める前・写したファイル・`emulator-5554` を名指さない・`<state>/tree` が無い）
  - spec.md:349-352 は、`Networking` を書かないことと、塞ぐ・確かめるが入れる前であることの 2 つ
  
  どれも片方だけが通っても、印の名前は同じなので緑になる（R14 の処置で、PC の片付けは 2 本に分けてある。spec.md:384-392）
- kind: technical
- 提案: R14 と同じく 2 本ずつ（:425-428 は 3 本）に分け、tasks.md の 7.1・9.1 の印の一覧と数の下限（23・33）を直す
- 処置: fixed specs/verify-environment/spec.md —— 束ねた Scenario を分けた（APK は「<state>/tree で組む」「接続先は…」「本人の資格情報は入らない」の 3 本、起動は「画面を出さず…」「自分の serial だけを名指す」、片付けは「ログを写してから止める」「ほかのエミュレータに触らない」「project と木を消す」、PC は「網をつないで起こす」「何かを入れる前に、塞いで確かめる」）。tasks 7.1・9.1 の印と下限（27・45）を直した

## R28. 手順書へ写すものの Requirement が「だけ」と言うのに、Q11 で足した文言を含んでいない
- 成果物: openspec/changes/ch21-trial-use-before-verify/specs/verify-environment/spec.md・design.md
- 根拠: spec.md:174 は、写すのを止まり方・件数・片付け・候補の番号と題「だけ」とし、spec.md:176-178 で決まった文言を全部並べる。
  ところが Q11 の Requirement は、段落に「前の回で試した: <ID>（<前の tag>）」を並べ、「回さなかった（この回に新しく入った<相手>のものが無い）」で始めるとしている（spec.md:275）。どちらも :174-178 に無い。
  確かめ直しの「直した候補を確かめ直した」（spec.md:307）も同じ。design.md:121-127 の D11 にも、この文言は無い。
  :170 の Requirement のとおりに作ると、:275 の Scenario（spec.md:281・:286）と食い違う
- kind: technical
- 提案: spec.md:174 の写すものに「前の回で試したものの ID と tag」を足す。:178 の決まった文言に「回さなかった（この回に新しく入った<相手>のものが無い）」と「直した候補を確かめ直した」を足す。D11 も同じに揃える
- 処置: fixed specs/verify-environment/spec.md —— 写すものに前の回で試したものの ID と tag を、決まった文言に「回さなかった（この回に新しく入った<相手>のものが無い）」「回さなかった（直した候補が無い）」「直した候補を確かめ直した」「前の回で試した: <ID>（<前の tag>）」を足した。design D11 も揃えた

## R29. 仮の台帳と design の見出しに、推奨（同じ中身なら写す）の名残がある
- 成果物: openspec/changes/ch21-trial-use-before-verify/provisional.jsonl・design.md
- 根拠:
  - provisional.jsonl:8 の D16 の `keeps` は「確かめ直しの後も通常の回は同じ中身でなければ全体を回す」で、Q11 の答え（新しく入ったものだけ）と違う
  - provisional.jsonl:11 の D19 の `question` は「前の回から写した題をどうするか」、`chosen` は「写した題も public-title.sh を通し直す」のまま
  - design.md:138 の D19 の見出しも「写した題も通し直す」のまま（本文の :141 は「写す道は無くなった」と書く）
  
  台帳は確認バッチで人間に見せるもので（CLAUDE.md「仮は台帳に載り…確認バッチで人間が見る」）、もう無い振る舞いを仮の決定として見せることになる。
  なお、deep.md:76 の「D10（仮。木で比べる）」は、第 1 回の時点のレビューの対応の記録なので、指摘にしていない
- kind: technical
- 提案: `provisional.py` で D16 と D19 を今の中身で立て直す（D16 の keeps は「確かめ直しの後も、ふだんの回は新しく入ったものを試す（D21）」、D19 は写す側を外す）。design.md:138 の見出しから「写した題も通し直す」を外す
- 処置: fixed D27 仮 —— provisional.py で D16 と D19 を resolve し、今の中身で D27（確かめ直し）・D28（検査できなければ全部伏せる）として立て直した。design の見出しと参照（proposal・tasks・harness2-request.json）も D27・D28 に直した

## R30. tasks.md に古い Task 番号が残り、読む順に distribution の delta が無い
- 成果物: openspec/changes/ch21-trial-use-before-verify/tasks.md
- 根拠: tasks.md:9 は「処置は依頼として置く（Task 9。design D15）」と書くが、依頼を置くのは Task 11.3（tasks.md:219。design.md:187 も「下流の Task 11」）で、Task 9 は携帯の相手。
  tasks.md:3 の読む順は `specs/verify-environment/spec.md` だけを挙げ、この版で足した `specs/distribution/spec.md`（Q8 の答え）を挙げていない
- kind: technical
- 提案: tasks.md:9 を「Task 11」に直し、tasks.md:3 の読む順に `specs/distribution/spec.md` を足す

### 第 2 版の観点ごとの確かめた範囲

- 観点 1:
  - Q8 の答え（外にはつなぎ、私設網と手元の網を塞ぐ）は、spec.md:325-401、distribution の (3)、design D6、tasks.md:14 と 7.1・8.1、CH21.md の壊してはいけないもの、proposal、premises に写っている。
    「網を切る」は、PC については「塞げなければ網を切る側に倒す」（Q8 の選んだ案の確かめていないこと）と、携帯の D24（仮）にだけ残る。推奨のときの Scenario「Sandbox は網を切って起こす」と案内の「網を切って」は消えている。
    Q8 の context は、外につなぐ案を選ぶと CH22 の後になることを本人に見せていた（deep-questions.json の Q8 の context）
  - Q11 の答え（新しく入ったものだけ）は、spec.md:269-301、目録の `earlier`、確かめ直し（spec.md:307）、design D10・D21（仮）、harness2-request.json の what と thoughts、tasks 4.1・4.2 に写っている。
    grep すると、`copied`・`copied_from`・`同じ中身なので…写した`・`7140` は、spec と harness2-request.json には残っていない。tasks には打ち消しとして残るだけ（tasks.md:77「`copied` は持たない」）。
    design.md:102 と :119 は「7,140 から延びた」「前の版は捨てた」という経緯として書いているだけ。名残は R29 の台帳と見出しだけ
  - 違和感（携帯）は、spec の差し込み口・範囲・呼び方・目録・段落・携帯の Requirement と、design D22〜D25・CH21.md・proposal・tasks 9・10 に写っている。数（9,360・1,860・360・2,400）は spec.md:138・:247・:263 の Scenario にある。
    操作できることの本物での確かめは R21
- 観点 3: proposal の Capabilities（verify-environment は Requirement 13、distribution は MODIFIED 1）と、`specs/` のディレクトリ・CH21.md の `capability: [verify-environment, distribution]` は合う。
  distribution の (3) は Scenario を持たない。ただし、許す側の名指しで、「塞いで確かめた後にだけ外へつなぐ」は verify-environment の spec.md:349-352・:401 が見るので、指摘にしていない（守りの中身の弱さは R22・R23）
- 観点 4: 依存の順（Task 6 の部品を Task 9 が使う。Task 4.1 の試した記録を 4.2 が読む）に逆向きは無い。本物の確かめ（Task 8・10・12）は、`/dev/kvm` と Sandbox が要ることが Global Constraints に書いてある
- 観点 5: CH21.md の完了の判定は、携帯（Task 10）と Task 11.3・3 つの段落に更新されていて、spec・tasks と合う。依頼（逐語）の「携帯は外」を越える分は、deep.md の違和感の `→ 対応:` が決定として持つ
- 仮の台帳: design の（仮）の 15 件（D1・D3・D5・D7・D9・D12・D15・D16・D18・D19・D20・D21・D22・D23・D24）と、provisional.jsonl の resolve されていない 15 件は合う。D10 は resolve 済みで、design の見出しからも（仮）が外れている
- `go.json` には推奨の版（網を切った Sandbox・前の結果を写す）が残っている。直すのは `reading --carry` の受け持ちなので、指摘にしていない
- 処置: fixed tasks.md —— Global Constraints の「Task 9」を「Task 11」に、読む順に `specs/distribution/spec.md` を足した


## R31. R22 の処置で足した「一時の口」は、立て方を試す口と、立てられないときの決まりが無い。このままでは自己試験が Linux の CI で通らない
- 成果物: openspec/changes/ch21-trial-use-before-verify/tasks.md・specs/verify-environment/spec.md・premises.md
- 根拠: 処置（HEAD 50d685430）で、PC の相手は塞ぐ前に一時の口を 2 か所に立てるようになった。WSL 側は `tailscale ip -4`・`-6` の宛先、Windows 側はこの PC の手元の網のアドレスで待ち受ける（tasks.md:155、spec の PC の Requirement、design D26）。
  ところが Global Constraints の偽物の一覧（tasks.md:18・:20）にも 7.1 の偽物の説明にも、`tailscale` と、Windows 側で待ち受けを立てる命令（`powershell.exe` など）が無い。
  自己試験は Linux の CI（`rust` の job）で走り、そこには `tailscale` も Windows も無い（tasks.md:18）。一時の口を立てられなければ PC の相手は回らない（design D26 の費用）ので、インストールまで進むことを見る Scenario は、偽物が無い限り CI で落ちる（「立てるときは…」「何かを入れる前に、塞いで確かめる」「外にだけ届かなければ…」「PC の相手は、本人の合言葉でなく…」など）。
  あわせて、次のことが決まっていない:
  - 一時の口を立てられないとき（`tailscale` が無い・動いていない、Windows 側で待ち受けを立てられない）にどうなるかを、spec が決めていない。spec が言うのは「塞ぐ前に一時の口に届かなければ `UNAVAILABLE=`」だけ
  - Windows 側の待ち受けには、Windows のファイアウォールの受け入れの許可が要りうる（design D26 の費用は「かもしれない（Task 8 で確かめる）」と書く）。許可の問い合わせの窓が本人のデスクトップに出るかもしれないが、premises.md に行が無い
  - 「通す外の宛先は…含まない」の Scenario（spec）の見本は、tailscale の IPv4 と IPv6・3 つの私用の範囲・リンクローカル・マルチキャストだけ。Requirement が除くとしている IPv6 の私用の範囲と IPv6 のリンクローカルの見本が無い
  - 「立てるときは、塞いで確かめ、…」の THEN が並べる順に、塞ぐ前の確かめが無い。別の Scenario（「何かを入れる前に、塞いで確かめる」）では、塞ぐ前の確かめが塞ぐ台本より先に来る。THEN を呼ばれた順の全部と読むと、2 つの Scenario は食い違う
- kind: technical
- 提案: Global Constraints と 7.1 の偽物に `tailscale` と Windows 側の待ち受けを立てる命令を足し、`TSUKAU_TEST_` で渡す。
  spec に「一時の口を立てられなければ、何も入れずに `UNAVAILABLE=`（理由に口の名前）」と、その Scenario を足す。
  premises.md に「Windows 側の手元の網のアドレスで、許可の窓を出さずに一時の待ち受けを立てられる」を確かさ 1 で足す（確かめるのは Task 8）。
  見本に IPv6 の私用の範囲と IPv6 のリンクローカルを足す。順の THEN は「…がこの順に（ほかの呼び出しを挟んでよい）」と書くか、塞ぐ前の確かめを入れる
- 処置: fixed D26 仮 —— 偽物に `tailscale`・`powershell.exe` を足し（Global Constraints・7.1）、一時の口を立てられなければ何も入れずに `UNAVAILABLE=` を spec の Requirement と Scenario「一時の口を立てられなければ、何も入れずに断る」に足し（7.1 の下限 28・9.1 の下限 46）、premises.md に Windows 側の待ち受けの許可の窓の行を確かさ 1 で足し（Task 8.1 で確かめる）、見本に IPv6 の私用の範囲と IPv6 のリンクローカルを足し、「立てるときは…」の THEN に塞ぐ前の確かめを入れた

## R32. 処置の後に、仮の台帳・design の見出し・premises の行に、古い文と食い違いが残っている
- 成果物: openspec/changes/ch21-trial-use-before-verify/provisional.jsonl・design.md・premises.md
- 根拠:
  - 台帳の D23 の `chosen`（provisional.jsonl:16）は「合言葉は <state>/tree の local.properties にだけ書き」のまま。処置で design D23 と spec は、資格情報を `-P` で渡す形に変わった
  - 台帳の D24 の `chosen`（provisional.jsonl:17）は「外に届かないことを確かめてから渡す」のまま。処置で design D24 と spec は、外へは問わず、既定のネットワークが無いことと、ホストの loopback の別名に届かないことを、切る前と後の対で見る形に変わった
    （台帳は確認バッチで人間に見せる。D27・D28 のように立て直すか、今の中身で足し直す）
  - design.md:192 の D27 の見出しは「旧 D27」、design.md:141 の D28 の見出しは「旧 D28」と書く。立て直す前の番号は D16 と D19（provisional.jsonl の resolve の行）
  - D23 は資格情報を組み立ての引数（`-P`）で渡す。ところが、写しているはずの録画の形は、`ps` に見えるのを避けるために、わざとコマンド行に出さずに `local.properties` に書いている（tools/record-env.sh:176-179）。
    その回の乱数の資格情報なので失うものは小さいが、design D23 はこの違いと、それでよい理由を書いていない
  - premises.md の「網を切ったエミュレータで、adb reverse の道だけでサーバに届く」の行は、確かめ方の欄が「確かめていない。…（知識）」で始まるのに、結果は「実物」「5」で、同じ行の中で食い違う
  - premises.md の「携帯の相手は、…立てる上限の 900 秒の中で立つ」の行は、測ったのがエミュレータの起動（52 秒）だけなのに確かさが 4。D5（design.md:73）が PC の zip の組み立てを `up` の外に出した理由（冷えた組み立てが立てる時間を食う）に当たるのは、まっさらな `<state>/tree` での APK の組み立てで、そこは測っていない。
    同じ表の 4 は「数えた」もの（premises.md の「1 日に 1〜5 回作り直される」）に付いている
- kind: technical
- 提案: `provisional.py` で D23 と D24 を今の中身で立て直す。design.md:141・:192 の「旧 D28」「旧 D27」を「旧 D19」「旧 D16」に直す。
  D23 に、`-P` にした理由（`~/.gradle/gradle.properties` に負けない）と、その回の資格情報が `ps` に見えることを書く。
  premises.md の adb reverse の行の確かめ方を、実測の文に揃える。900 秒の行は、組み立てを測るまで確かさを 2 以下にする（Task 10 の検証は 5 になったことを見るので、そこで上がる）
- 処置: fixed D30 仮 —— D23・D24 を resolve して D30・D31 に立て直した（参照も直した）。D27・D28 の見出しを「旧 D16」「旧 D19」に直した。D30 に `-P` にした理由と `ps` に見える代償を書いた。premises.md の adb reverse の行の確かめ方を実測の文に揃え、900 秒の行は確かさを 2 に下げた（R25 の処置の文も直した）


R31・R32 の処置の確かめ: 新しい指摘なし

## 第 3 版（spec --answers Q8,Q11 と IMP-13 の直し。2026-11-22）

### 機械の検査（2026-11-22、docs/ch21-upstream の fa1e390a3 の上）

- `openspec validate ch21-trial-use-before-verify --strict` → `Change 'ch21-trial-use-before-verify' is valid`
- `python3 scripts/check_chain.py .` → `chain: OK (0 件 / 未回収 0 件 / warn 0 件)`（change なので観点 8 は無い）
- `python3 scripts/check_scenarios.py . ch21-trial-use-before-verify` → `scenarios: FAIL (担保なし 105 件 / 名無しの確認待ち 0 件)`。下流の前なので、担保が無いのは予定どおり。
  105 件は全部 verify-environment。distribution の 11 本は CH22 の archive で入った印が担保している。
  tasks.md の `# Scenario:`・`== Scenario:` の名前は、python で抜き出して突き合わせると、verify-environment の 105 本と一字一句合う。下限の数も合う: verify-batch.sh 8 → 38 → 41 → 45（4.1 の印は 30 本）、env-hook.sh 11 → 28 → 48（9.1 の印は 20 本）
- `python3 scripts/review_triage.py . ch21-trial-use-before-verify` → 書く前は `triage: OK`（指摘 48 件 / 仮決め 12 件）。R33 以降は処置が無いので落ちる（予定どおり）
- `python3 scripts/spec_overlap.py candidates . ch21-trial-use-before-verify` → `[]`
- Task 12.2 の検証コマンドを、書いたままこの作業ツリーで走らせた → 「席の PATH の口（harness2）が無い」の 1 行と rc=127（鍵が入る前の止まり方のとおり）。鍵と `tsukau.json` を置いた一時の木で走らせると rc=0。引用符はそのまま通る
- 正典の distribution（origin/main a5aee6d2b。CH22 は archive 済み）の「更新を確かめる問い合わせは、版の一覧を取る決まった形だけを配布元へ送る」と CH21 の MODIFIED の差は、10 行目の「2 つ」→「3 つ」と (3) の追加（文字単位で突き合わせ、ほかの字の差は無い）と、(3) を足した導出元の 1 行だけ（python の difflib で突き合わせた）

## R33. spec と Task 6.1 の自己試験は差し込み口の鍵を「4 つだけ」と固定しているのに、Task 12.2 と完了の判定は `seat_hide` を足す。手で呼ぶ席から adb を外すことは、specs のどこにも無い
- 成果物: openspec/changes/ch21-trial-use-before-verify/specs/verify-environment/spec.md・tasks.md・design.md・docs/changes/CH21.md
- 根拠: spec.md:6 は「鍵は `env_hook`・`targets`・`out_root`・`ready_timeout_s` の 4 つだけにする」と書く。Scenario「差し込み口の形が席の台本の契約に合う」（spec.md:10-12）の THEN も「鍵は…の 4 つだけ」で、Task 6.1 がこの印を `env-hook.sh --self-test` に置く（tasks.md:141）。
  一方で Task 12.2（tasks.md:265-267）は `tools/tsukau/tsukau.json` に `"seat_hide": ["adb", "emulator"]` を足し、docs/changes/CH21.md の完了の判定もそれを求める。
  12.2 の検証は `env-hook.sh --self-test` を走らせないので 12.2 は緑になる。ところがその後の Task 13.1（tasks.md:272）が `env-hook.sh --self-test` を走らせ、5 つ目の鍵でこの Scenario が落ちる。直さずに archive すると、正典にも「4 つだけ」が残る。
  また、手で呼ぶ `/tsukau` の席からも `adb` を外すこと（design D17 の足した行・D34（仮）。CH21.md の「壊してはいけないもの」も「手で呼ぶときにも当てる」と書く）は、観測できる振る舞い（`tsukau.json` の鍵と、手で呼んだ席の `PATH`）なのに、Requirement にも Scenario にも無い。design にしか無いので、archive で正典から落ちる
- kind: technical
- 提案: spec.md:6 と Scenario「差し込み口の形が席の台本の契約に合う」を「4 つ。harness2 の `seat_hide` が入った後は、それに `seat_hide`（`["adb", "emulator"]`）を足した 5 つ」と書き直す。Task 6.1 の自己試験は、鍵が入る前と後のどちらの形でも通るように書く。
  `seat_hide` について、Scenario を 1 本足す（例:「harness2 の鍵が入った後は、差し込み口は席から adb と emulator を隠すよう名指す」。WHEN は `tools/tsukau/tsukau.json` を読む、THEN は `seat_hide` がちょうど `["adb", "emulator"]`）。印は Task 12.2 に置く。
  鍵が入る前に Task 13.1 が走る順にならないよう、12.2 と 13.1 の順と止まり方（BLOCKED_INFRA）を揃える
- 処置: fixed specs/verify-environment/spec.md —— 差し込み口の Requirement の鍵を「4 つと、harness2 の鍵が入った後は `seat_hide`」に直し、手で呼ぶ席にも `seat_hide` で `adb` と `emulator` を見つけさせないことを本文に足した。Scenario「差し込み口の形が席の台本の契約に合う」の THEN を「あれば `seat_hide` だけ」にし（Task 6.1 は 12.2 の前と後のどちらでも通る）、Scenario「harness2 の鍵が入った後は、差し込み口は席から adb と emulator を隠すよう名指す」を足した（印は Task 12.2 の `tools/tsukau/check-seat-hide.sh`。rc=127 で BLOCKED_INFRA）。13.1 は 12.2 の後に走らせると書いた。design D34（仮）から Scenario を指した

## R34. 「席の Bash は、呼ばれた環境の PATH を引き継ぐ」という前提は確かさ 3 のままで、上げる Task が無い。偽の席では、IMP-13 の本体が本物で効くかが分からない
- 成果物: openspec/changes/ch21-trial-use-before-verify/premises.md・tasks.md・design.md
- 根拠: premises.md:50 の結果の欄は「席の Bash が呼ばれた環境の `PATH` を引き継ぐかは、本物の席では確かめていない」で、確かさは 3。
  spec の Scenario「席を呼ぶ PATH には Android の道具の置き場が無く、adb が名前で見つからない」が見るのは、偽の席の台本が受けた `PATH` と、その中の `command -v` だけ。つまり、`tsukau.py` に渡した環境までしか見ていない。
  本物の席（`claude -p`）は、Bash の道具を呼ぶたびに利用者のシェルの初期化から作った写し（`~/.claude/shell-snapshots/` の `export PATH=…`）を読み込む。席の `PATH` が外した形のまま席の Bash に届くかは、このシェルの初期化と Claude Code の版に依る。
  ほかの確かさの低い前提には、上げる Task がある（1.2・8.1・10.1 が premises.md の行を書き換える）。この行には無い。本物の席を呼ぶ Task 12.1（real-run.sh）も、席の中で `command -v adb` を見ない。design D32 の反転条件（logs/claude.jsonl に絶対パスや待ち受けの跡）は、席が名前で `adb` を呼べた場合を拾わない
- kind: premise
- 提案: Task 12.1 か新しい項目で、本物の席の Bash に `command -v adb; command -v emulator` を走らせて、両方とも空なことを確かめる。たとえば real-run.sh で準備の席の部分を本物で走らせるとき、席の案内に頼むか、`logs/claude.jsonl` の Bash の出力を読む。
  `echo "== Scenario: 本物の席の Bash で、adb と emulator が名前で見つからない"` を出し、spec に同じ名前の Scenario を足す。結果を premises.md:50 に書き、確かさを 5 にする。
  あわせて、D32 の反転条件に「本物の席の Bash で名前で見つかったとき」を足す
- 処置: rejected: 前提は成り立つ。2026-11-22 に `$ANDROID_HOME` の下を外した `PATH` で本物の `claude -p` を起こし、席の Bash で `command -v adb`・`command -v emulator` が rc=1（見つからない）、`command -v git` が `/usr/bin/git` だった（premises.md の行を実物・5 にした）。指摘の提案は取り入れた: 下流の Task 4.3 `verify-batch.sh real-check-seat-path` が `verify-batch.sh` の組む `PATH` で同じことを本物で確かめ直し、spec に Scenario「本物の席の Bash で、adb と emulator が名前で見つからない」、design D32 に本物で確かめることと反転条件（見つかれば A）を足した

## R35. 「席の PATH から外す」Requirement の本文が言うことのうち、`$ANDROID_SDK_ROOT`・`emulator` の見つけ方・`tsukau.log` の理由・`$ANDROID_HOME` が無いときを、どの Scenario も見ていない
- 成果物: openspec/changes/ch21-trial-use-before-verify/specs/verify-environment/spec.md・design.md・tasks.md
- 根拠: spec の呼び方の Requirement に足した本文（spec.md:84）は、外す置き場を「`$ANDROID_HOME` と `$ANDROID_SDK_ROOT` の下」、確かめるものを「`adb` と `emulator` が名前で見つからない」、見つかったときの理由の置き場を「`dist/verify-<tag>/tsukau.log` にだけ」と書く。
  足した 2 本の Scenario は、`$ANDROID_HOME` だけを偽に向ける（`$ANDROID_SDK_ROOT` が別の置き場を指す場合を見ない）。外に置くのも偽の `adb` だけ（外の `emulator` で呼ばない場合を見ない）。`tsukau.log` に理由があり、段落と目録に理由の字面が無いことも見ない。
  次の 2 つは spec にも design にも決まりが無い:
  - `$ANDROID_HOME` と `$ANDROID_SDK_ROOT` が無いか空の環境で、`verify-batch.sh` がどう外すか。「`$ANDROID_HOME` の下」を前置きの一致で書くと、空の値は `/` の下、つまり全部の置き場に当たる。そうなると、Android の SDK を持たない PC で、画面と PC の席が `claude` も `git` も見つけられなくなる。携帯の範囲が無い回でも同じ
  - design D32 は `env-hook.sh` が「頭で」道具の置き場を足し、`$ANDROID_HOME` か道具が無ければ「今の『SDK が無ければ断る』と同じく `UNAVAILABLE=`」と書く。`env-hook.sh` は 3 つの相手を 1 本で持つので、この断りが携帯の相手だけに掛かるのか、画面と PC にも掛かるのかが読めない
  あわせて、自己試験は偽の `$ANDROID_HOME` だけを渡す（tasks.md:21・:80）。走らせる環境の `$ANDROID_SDK_ROOT` と `PATH` の扱いは書いていない。ホストの `PATH` に `$ANDROID_SDK_ROOT` の外の `adb` があると（`ANDROID_SDK_ROOT` が無く、本物の `platform-tools` だけが `PATH` にある場合など）、D33（仮）が効く。そのため、席を呼ぶほかの Scenario が環境次第で落ちる
- kind: technical
- 提案: Scenario を足すか、今の Scenario の GIVEN を広げる:「`$ANDROID_SDK_ROOT` だけが指す置き場の道具も外す」「外に `emulator` だけがあっても呼ばない」「外せなかった回の理由は `tsukau.log` にあり、段落と目録に無い」「`$ANDROID_HOME` と `$ANDROID_SDK_ROOT` が無くても、画面の席は今の `PATH` のまま呼ぶ」。
  design D32 に、`env-hook.sh` が道具を探して断るのは携帯の相手だけだと書く。tasks の Global Constraints に、自己試験は `ANDROID_SDK_ROOT` も偽に向けるか消し、`PATH` を試験が組んだ置き場だけにする、と書く
- 処置: fixed D32 —— Requirement に「値が無いか空の変数は外す置き場に数えない」を足し、design D32 に理由と「道具を探して断るのは携帯の相手だけ」を書いた。Scenario「Android の道具の置き場の外に emulator だけがあっても、どの相手の席も呼ばない」「ANDROID_SDK_ROOT だけが指す置き場の道具も、席の PATH から外す」「Android の SDK が無い環境では、席の PATH を変えずに画面の席を呼ぶ」を足し、「外にも adb があれば」の THEN に `tsukau.log` の理由と段落・目録に無いことを足した（4.1 の下限 41・4.2 は 44・5.1 は 48）。Global Constraints に、自己試験は `ANDROID_HOME`・`ANDROID_SDK_ROOT` を偽に向けるか消し、席に渡る `PATH` を試験が組んだ置き場だけにする、を足した。9.1 に携帯の相手だけが断ることを書いた

## R36. 「phone.sh は serial を引数で受けず」の Scenario は、serial を引数で渡す場合を試していない。`phone.sh` の道具が無いときの止まり方は design にしか無い
- 成果物: openspec/changes/ch21-trial-use-before-verify/specs/verify-environment/spec.md・design.md
- 根拠: Scenario「phone.sh は serial を引数で受けず、ほかの端末を名指さない」の WHEN は、`ANDROID_SERIAL=emulator-5554` で呼ぶ場合と、引数に serial を含まない `shell`・`shot`・`ui` だけ。題の「引数で受けず」を試す呼び方（たとえば `phone.sh -s emulator-5554 shell …`）が無い。
  `adb` は `-s` を明示すれば `ANDROID_SERIAL` を見ないので、この WHEN は `-s` を付ける実装なら必ず通り、引数を読む実装も落とせない。引数で serial を渡されたときの振る舞い（断るのか、無視するのか）も決まっていない。
  design D25（足した行）の「`$ANDROID_HOME` が無いか、そこに `adb` が無ければ rc≠0 で『携帯の道具が見つからない』と 1 行出す」は、観測できる振る舞いなのに Requirement にも Scenario にも無い。tasks.md:200 も、文言は持つが印を持たない。
  細部: design D32 は、外す置き場を「`platform-tools`・`emulator`・`cmdline-tools/latest/bin`。今の `tools/android-env.sh` の 7 行が足すもの」と書く。ところが tools/android-env.sh:7 が足すのは `$JAVA_HOME/bin`・`cmdline-tools/latest/bin`・`platform-tools` で、`emulator` は足さない（premises.md:50 の「名前の `emulator` は無い」と合う）。外す範囲は「`$ANDROID_HOME` の下を全部」なので振る舞いは変わらないが、根拠の文が実物と違う
- kind: technical
- 提案: WHEN に serial を引数で渡す呼び方を足し、THEN に rc≠0（か、`-s emulator-5600` のまま）を決めて書く。
  Scenario「携帯の道具が見つからなければ、phone.sh は 1 行で断る」を足す（偽の `$ANDROID_HOME` に `adb` を置かず、rc≠0 と「携帯の道具が見つからない」を見る）。印を Task 9.1 に足し、下限を 49 に上げる。
  D32 の括弧の文は、android-env.sh:7 の実物に合わせて直す
- 処置: fixed D25 —— Requirement に「最初の引数が `shell`・`shot`・`ui` でなければ `adb` を呼ばずに rc≠0」「`$ANDROID_HOME` の下に `adb` が無ければ『携帯の道具が見つからない』で rc≠0」を足し、Scenario「phone.sh は serial を引数で受けず…」の WHEN に `phone.sh -s emulator-5554 shell …`（rc≠0・`adb` を呼ばない）を足した。Scenario「携帯の道具が見つからなければ、phone.sh は 1 行で断る」を足した（9.1 の下限 49）。design D25 に引数の扱いを書き、D32 の括弧の文を `tools/android-env.sh:7` の実物に合わせた

### 第 3 版で確かめて指摘にしなかったもの

- IMP-13 の本人の補足（逐語「席を呼ぶ間は adb を席の PATH から外し、phone.sh だけが使えるようにする」）は、呼び方の Requirement と携帯の Requirement の本文・Scenario 4 本（足した 2 本と直した 2 本）・design D32・D25・tasks の Global Constraints・4.1・9.1・11.2 に写っている。「`phone.sh` だけ」は案内の Scenario の THEN にある
- `tools/emulator-start.sh` は `adb` を名前で呼ぶ（:117・:119・:141・:143）。`emulator` は `$ANDROID_HOME/emulator/emulator`（:64）で呼ぶ。design D32 の「`env-hook.sh` は自分のプロセスの `PATH` の先に足してから読み込む」は、これと合う。携帯の相手の Scenario「受けた PATH に adb が無くても…」が、そのことを固定している
- `tsukau.py` は、席（:493-495）にも差し込み口（:447-449 の `hook_env`）にも `os.environ` を渡す。そのため `verify-batch.sh` で外した `PATH` は差し込み口にも届き、差し込み口が自分で足し戻す設計が要る。design D32 はそのとおりに書いている。D34（仮）の依頼（harness2-request-seat-path.json）は「差し込み口に渡す環境は変えない」と書き、これと合う
- 仮でない D32 は本人の補足そのもの（外すこと・`phone.sh` だけ）。仮の D33（外しても見つかれば呼ばない）と D34（仮）（手で呼ぶ道は harness2 の鍵）は、補足が決めていない細部で、外しても失うのは席が回らないことと待つことだけ。この分け方に食い違いは無い

観点 7: 該当なし（spec_overlap.py candidates が 0 件。CH22 は 2026-11-21 に archive 済みで、requires の CH22 は満たされている）
