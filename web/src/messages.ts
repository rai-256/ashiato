// SPDX-License-Identifier: AGPL-3.0-only
/**
 * 画面の文言（製造準備 A-2「日本語のみ。ただし文言をコードから分離し、後から言語を足せる形にする」）。
 *
 * **画面に出る文字はここにだけ書く。** 部品（`*.tsx`）と、表示の文を組む関数（`stays.ts` `coverage.ts`
 * `attributes.ts` の `*Label` / `*Message` など）は `msg("<画面>")` で引く。直書きは
 * `eslint.config.js` の `no-restricted-syntax`（JSX の地の文・日本語の文字列）が止める。
 *
 * 言語を足すときは、`Messages` の形を満たす別の表を作って `msg` の引き先を切り替える。
 * いまは日本語だけを持つ（i18n の仕組みは入れていない —— 製造準備の「やらないこと」）。
 */

const ja = {
  /** 数の書き方（`toLocaleString` / `Intl.DateTimeFormat`）に渡す地域。 */
  locale: "ja-JP",

  /** 共通の部品 */
  common: {
    loading: "読み込み中…",
    toCoverage: "稼働状況へ",
    cancel: "やめる",
  },

  /** ログイン（session.tsx） */
  session: {
    logout: "ログアウト",
    logoutFailed: "ログアウトできなかった。もう一度押してください。",
    password: "合言葉",
    login: "ログイン",
    refusal: {
      wrong: "合言葉が違います。",
      throttled: "試しすぎで、しばらく受け付けません。1 分ほど待ってからもう一度。",
      unreachable: "サーバに届かなかったか、サーバで失敗しました。もう一度試してください。",
    },
  },

  /** 画面の行き先（Root.tsx） */
  root: {
    invalidDate: (day: string): string => `アドレスの日付（${day}）を日付として読めません。`,
    toToday: "今日の一覧へ",
  },

  /** S-1 稼働状況（App.tsx） */
  coverage: {
    title: "収集が動いていたか",
    toDay: "1 日の画面へ",
    toMaster: "マスタ管理へ",
    achievementFailed: (why: string): string =>
      `達成の読み出しに失敗しました（${why}）。達成日数が 0 なのではありません。`,
    coverageFailed: (why: string): string =>
      `稼働状況の読み出しに失敗しました（${why}）。収集が止まったのではありません。`,
    empty: "ソースが 1 本も返りませんでした（登録簿を確認してください）。",
  },

  /** 成功条件 1 の達成（AchievementPanel.tsx） */
  achievement: {
    label: "成功条件 1 の達成",
    title: "収集が動いていた日数",
    met: "5 本すべてが分母の 95 % 以上",
    notMet: "分母の 95 % に届かないソースがある",
    separator: " — ",
    confirmed: "確定",
    provisional: "暫定",
    undecided: (notStarted: string[]): string =>
      `確定する日はまだ決まらない（収集を開始していないソースがある: ${notStarted.join("、")}）`,
    daysLeft: (days: number): string => `確定まであと ${days} 日`,
    colSource: "ソース",
    colAchieved: "達成日数",
    colDenominator: "分母",
    colThreshold: "線",
    notStarted: "未開始",
  },

  /** 直近に置いた書庫の箱と、書庫のソースの最終日（LatestArchive.tsx / archives.ts。ST12） */
  archive: {
    title: "直近に置いた書庫",
    failed: "書庫の状態を読み出せませんでした（置いた書庫が無いのではありません）",
    loading: "書庫の状態を読み込み中…",
    reading: (file: string, inner: string, items: string, since: string): string =>
      `読んでいます: ${file} ${inner} ${items} 件まで（${since} から）`,
    place: {
      dedicated_inbox_unreadable: "専用のフォルダ",
      downloads_unreadable: "ダウンロードのフォルダ",
    } as Record<string, string>,
    anyPlace: "置き場",
    placeSeparator: "・",
    unreadablePlaces: (places: string): string => `${places} が読めません`,
    inboxNever: "取り込み器はまだ一度も動いていません",
    inboxStale: (days: number): string => `取り込み器の最後の確認: ${days} 日前`,
    pendingShape: (archives: number): string =>
      `形の確認を待っている書庫が ${archives} 冊あります（tools/archive-shape.sh で形を見て印を置く）`,
    empty: "まだ書庫が置かれていません",
    unnamed: "名前の分からない書庫",
    unreadable: (name: string, seen: string, why: string): string => `${name}（${seen}）— 読めなかった書庫です（${why}）`,
    storeFailed: (name: string, seen: string): string =>
      `${name}（${seen}）— 格納に失敗しています（1 時間ごとに読み直します）`,
    alreadyRead: (name: string, seen: string, previously: string | null): string =>
      `${name}（${seen}）— 既に読んだ書庫です（${previously === null ? "前に読んだ時刻が分かりません" : `${previously} に読んだものと同じ中身`}）`,
    pendingLatest: (name: string, seen: string): string => `${name}（${seen}）— 形の確認を待っています`,
    read: (name: string, seen: string, inserted: number, duplicate: number, unreadable: number): string =>
      `${name}（${seen}）— 入った ${inserted} · 既にあった ${duplicate} · 読めなかった ${unreadable}`,
    more: (hidden: number): string => `ほか ${hidden} 件`,
    /** 読めなかった理由の種別（design D7 の 4 つ） */
    unreadableKind: {
      unsupported_format: "扱えない形式",
      broken_zip: "書庫が壊れている",
      html_only: "HTML しか入っていない",
      no_known_content: "読める中身が 1 つも無い",
    } as Record<string, string>,
    unknownReason: "理由は不明",
    notYet: "まだ無い",
    lastEvent: (day: string, days: number): string => `${day} まで（${days} 日前）`,
  },

  /** ソースごとの格子（CoverageGrid.tsx / coverage.ts） */
  grid: {
    succeeded: (named: string): string => ` — ${named} を引き継ぎ`,
    retired: (on: string | null): string => ` — ${on} に退役`,
    /** 書庫のソースの見出しに添える注記（ST12 / design D12） */
    archiveNote: (note: string): string => ` — ${note}`,
    collapse: "直近だけにする",
    showRetired: "退役したソースを見る",
    expand: "1 年ぶんを見る",
    week: (start: string): string => `${start} の週`,
    /** 8 状態の名前（FR-54）。週を選んだときにこの文字が出る */
    state: {
      recorded: "記録あり",
      alive_no_record: "動いていた・記録なし",
      alive_not_capturable: "動いていたが取れない状態だった",
      stopped: "意図的な停止",
      dropped: "破棄された期間",
      outage: "途絶",
      before_start: "導入前",
      retired: "退役",
    },
    droppedCount: (count: string): string => `— ${count} 件`,
    droppedRange: (count: string, from: string, to: string): string => `うち ${count} 件を破棄（${from}〜${to}）`,
    droppedRangeNoCount: (from: string, to: string): string => `破棄（${from}〜${to}）`,
  },

  /** S-2 1 日の画面（DayView.tsx / stays.ts） */
  day: {
    /** 題は日付だけ（ST25。滞在だけの一覧ではなくなった） */
    title: (dateLabel: string): string => dateLabel,
    toCoverage: "稼働状況",
    toMaster: "マスタ管理",
    prev: "前の日",
    next: "次の日",
    pickDate: "日付を指定",
    failed: (why: string): string =>
      `1 日の並びの読み出しに失敗しました（${why}）。滞在や記録が無いのではありません。`,
    actionFailed: "操作に失敗しました。もう一度お試しください。",
    criteria: (label: string): string => `この一覧は ${label} で作った`,
    criteriaOthers: (labels: string[]): string => `（一部の滞在は ${labels.join("、")} で作った）`,
    future: "まだ来ていない日です。",
    empty: "この日の滞在はありません。",
    listLabel: "1 日の並び",
    range: (start: string, end: string): string => `${start} – ${end}`,
    lengthAndRange: (long: string, range: string): string => `${long} ・ ${range}`,
    alsoRange: (range: string): string => ` ・ ${range}`,
    rowCriteria: (label: string): string => `${label} で作った`,
    erased: (range: string): string => `消した ${range}`,
    restore: "戻す",
    move: (long: string): string => `移動 ${long}`,
    noRecord: (range: string): string => `記録なし ${range}`,
    /** どの行にも入らない時間の記録の行（ST25 / D19（仮）） */
    other: (range: string): string => `ほかの記録 ${range}`,
    /** 種類のしるし（線の輪の中の記号）。**文字に添えるだけ**で、読み上げには出さない（2026-10-12） */
    mark: { stay: "●", move: "➝", "no-record": "…", erased: "×", other: "◦" },
    /** ブラウザ履歴がまだ届いていない（ST25 / D10） */
    browserPending: (through: string): string => `ブラウザ 最後に届いたのは ${through}。それより後の訪問はまだ届いていない`,
    recordsLoading: "記録を読み込み中…",
    recordsFailed: (why: string): string => `記録の読み出しに失敗しました（${why}）。`,
    recordsLabel: "開いた行の記録",
    /** 位置の記録の題（ST25 / D23（仮））。ソースの表示名（`携帯端末の位置`）ではなく「場所」と出す */
    locationTitle: "場所",
    detailLoading: "詳細を読み込み中…",
    detailFailed: (why: string): string => `詳細の読み出しに失敗しました（${why}）。`,
    detailCount: (name: string, count: number): string => `${name} ${count} 件`,
    erase: "この滞在を消す",
    eraseConfirm: (count: number): string => `この滞在と一緒に消える位置の記録 ${count} 件です。消しますか？`,
    eraseYes: "消す",
    /** 長さ（`3 時間 34 分` / `42 分`） */
    minutes: (m: number): string => `${m} 分`,
    hoursMinutes: (h: number, m: number): string => `${h} 時間 ${m} 分`,
    /** 基準（`半径 100 m / 10 分`） */
    criteriaLabel: (radiusM: number, minMinutes: number): string => `半径 ${radiusM} m / ${minMinutes} 分`,
    /** 日付（`2026 年 9 月 12 日`） */
    dateLabel: (y: number, mo: number, d: number): string => `${y} 年 ${mo} 月 ${d} 日`,
  },

  /** 1 日の画面の要約・取れていない時間・記録なしの事情・開いた中身（timeline.ts。ST25） */
  timeline: {
    /** 区切り */
    sep: " ・ ",
    topSep: " · ",
    lead: " — ",
    pc: "PC",
    phone: "スマホ",
    browser: "ブラウザ",
    archive: (name: string): string => `書庫: ${name}`,
    /** 要約のブラウザの行の頭（`ブラウザ 12 件`） */
    browserCount: (count: number): string => `ブラウザ ${count} 件`,
    archiveCount: (name: string, count: number): string => `書庫: ${name} ${count} 件`,
    reason: {
      before_start: "位置の収集を始める前",
      stopped: "止めていた",
      dropped: (n: number): string => `端末が破棄 ${n} 件`,
      archive: (n: number): string => `書庫に位置 ${n} 件`,
    },
    gap: {
      away: (range: string): string => `離席 ${range}`,
      stopped: (range: string): string => `PC 停止 ${range}`,
      excluded: (range: string, n: number): string => `除外 ${range}（${n} 件。本文なし）`,
      clockSkew: (range: string): string => `スマホ 時計のずれで取れていない ${range}`,
      retention: (range: string): string => `スマホ 取得元に残っていない ${range}`,
    },
    record: {
      away: "離席",
      stopped: "PC 停止",
      excluded: (n: number): string => `除外（${n} 件。本文なし）`,
      unavailable: (source: string): string => `${source} 取れていない`,
    },
    /** 地域の時刻（`（London 8:02）`） */
    region: (name: string, time: string): string => `（${name} ${time}）`,
    /** 畳んだ行の件数 */
    count: (n: number): string => `${n} 件`,
  },

  /** S-6 マスタ管理（MasterView.tsx / attributes.ts） */
  master: {
    title: "マスタ管理",
    tabAttributes: "個人属性",
    tabPlaces: "場所",
    failed: (why: string): string => `個人属性を読み出せませんでした（${why}）。`,
    addKind: "種類を足す",
    rename: (name: string): string => `「${name}」の名前を変える`,
    stored: (name: string): string => `「${name}」に主張を積みました。`,
    notYet: "まだ書いていない",
    upcoming: (value: string, validFrom: string): string => `${value}（予定）・${validFrom}`,
    write: "書く",
    supersededCount: (n: number): string => `訂正で取り消した ${n} 件`,
    supersededMark: "（訂正で取り消し）",
    upcomingMark: "（予定）",
    note: (note: string): string => `補足: ${note}`,
    whatToWrite: "何を書くか",
    modeChange: "変わった",
    modeFix: "前の書き込みが間違っていた",
    whichToSupersede: "どの主張を取り消すか",
    supersedeTarget: "取り消す主張",
    claimOption: (value: string, validFrom: string): string => `${value} ／ ${validFrom}`,
    value: "値",
    none: "なし（その属性が終わった）",
    validFrom: "いつから",
    precision: { year: "年", month: "年月", day: "年月日", unknown: "分からない" },
    validFromYear: "いつから（年）",
    validFromMonth: "いつから（月）",
    validFromDay: "いつから（日）",
    noteField: "補足（任意）",
    noteLabel: "補足",
    noTarget: "取り消す主張を選んでください（この種類にはまだ主張がありません）",
    submit: "積む",
    decide: "決める",
    /** 「いつから」の読める文。精度を丸めない（深掘り C3） */
    validFromUnknown: "いつからかは分からない",
    validFromYearLabel: (y: number): string => `${y} 年から`,
    validFromMonthLabel: (y: number, m: number): string => `${y} 年 ${m} 月から`,
    validFromDayLabel: (y: number, m: number, d: number): string => `${y} 年 ${m} 月 ${d} 日から`,
    assertedAt: (at: string): string => `${at} に書いた`,
    /** 値の「なし」（その属性が終わった）。「まだ書いていない」と混ぜない */
    noneValue: "なし",
    rejection: {
      invalid_claim_value: "値が空です。値を入れるか「なし」を選んでください",
      invalid_valid_from: "「いつから」の日付が読めません",
      unknown_attribute_kind: "この種類が見つかりません。画面を読み直してください",
      invalid_supersedes: "取り消す主張が見つかりません。画面を読み直してください",
    },
    kindRejection: {
      empty_name: "名前を入れてください",
      duplicate_name: "その名前は、いまある種類と重なっています",
      unknown_kind: "この種類が見つかりません。画面を読み直してください",
    },
    rejectedOther: (kind: string): string => `受け付けられませんでした（${kind}）`,
    unreachable: "サーバに届きませんでした。入力はそのまま残っています",
    /** 場所（ST21。PlacesView.tsx / PlaceForms.tsx / PlaceBand.tsx / places.ts） */
    places: {
      section: "場所",
      add: "場所を足す",
      failed: (why: string): string => `場所を読み出せませんでした（${why}）。`,
      empty: "場所がまだありません。",
      changeName: "名前を変える",
      changeRadius: "広さを変える",
      changeCoord: "座標を変える",
      neverStayed: "まだ居たことが無い",
      lastDay: (day: string): string => `最後に居た日 ${day}`,
      radius: (m: number): string => `広さ ${m} m`,
      /** 1 行に並べる項目の区切り */
      sep: " · ",
      previous: (n: number, open: boolean): string => `前の名前・座標 ${n} ${open ? "▾" : "▸"}`,
      previousName: (name: string): string => `前の名前: ${name}`,
      band: "24 区分の帯（0 時台から 23 時台）",
      candidatesFailed: (why: string): string => `居た所を読み出せませんでした（${why}）。`,
      candidatesEmpty: "居た所がまだありません。",
      rest: (n: number): string => `残り ${n} か所`,
      radiusLegend: "広さ",
      meters: (m: number): string => `${m} m`,
      candidateMeta: (count: number, first: string, last: string): string => `${count} 件 · ${first} 〜 ${last}`,
      giveName: "名前を付ける",
      name: "名前",
      noteField: "補足（任意）",
      noteLabel: "補足",
      register: "登録する",
      change: "変える",
      newCoord: "新しい座標（名前の無い居た所から）",
      coordOption: (coord: string, total: string, last: string): string => `${coord} · ${total} · 最後に居た日 ${last}`,
      coordReason: "座標を変える理由",
      coordFix: "前の座標が間違っていた",
      coordMove: "この場所が移った",
      validFrom: "いつから",
      moveNote: (name: string): string => `前の座標で居た時間も「${name}」のまま`,
      hours: (n: number): string => `${n} 時間`,
      minutes: (n: number): string => `${n} 分`,
      fixed: "直した",
      upcoming: "予定",
      upcomingFrom: (date: string): string => `予定（${date} から）`,
      beforeMove: "移る前",
      beforeMoveUntil: (date: string): string => `移る前（〜${date}）`,
      rejection: {
        invalid_place_name: "名前が空です。名前を入れてください",
        invalid_radius: "広さが範囲の外です",
        invalid_valid_from: "「いつから」の日付が読めません",
        unknown_place: "この場所が見つかりません。画面を読み直してください",
        invalid_coord_supersedes: "直す座標が見つかりません。画面を読み直してください",
        invalid_coord_change: "この場所の座標の変え方が合いません。画面を読み直してください",
        invalid_coordinate: "座標が読めません",
        place_id_taken: "場所を作れませんでした。もう一度「登録する」を押してください",
      },
      rejectedOther: (kind: string): string => `受け付けられませんでした（${kind}）`,
    },
  },

  /** 毎日の気分（feelings.ts / FeelView.tsx。ST17） */
  feel: {
    /** 快–不快の言葉（-2〜+2）。色で分けず、数値と言葉で出す（深掘り C1） */
    valenceWord: {
      "-2": "とても不快",
      "-1": "不快",
      "0": "どちらでもない",
      "1": "快",
      "2": "とても快",
    } as Record<string, string>,
    /** 暦の記入あり（日の記録が無く滞在にだけある日）/ 未記入の印 */
    markStayOnly: "滞",
    markMissing: "未",

    /** S-3 主観入力の画面（FeelView.tsx。design D12） */
    title: (dateLabel: string): string => dateLabel,
    prevMark: "‹",
    nextMark: "›",
    prevDay: "前の日へ",
    nextDay: "次の日へ",
    loading: "読み込み中…",
    loadFailed: "気分を読み出せませんでした（書いていない、という意味ではありません）",
    invalidDate: (raw: string): string => `日付が読めません（${raw}）。`,
    toToday: "今日の気分を書く",
    valenceQuestion: "この日はどう感じましたか（快–不快）",
    valenceGroup: "快–不快",
    valenceButton: (signed: string, word: string): string => `${signed} ${word}`,
    energyLabel: "元気さ（任意）",
    energyGroup: "元気さ",
    energyButton: (signed: string): string => `元気さ ${signed}`,
    noteLabel: "ひとこと（任意）",
    notePlaceholder: "例: 久しぶりに走った",
    keepNote: "ひとことを残す",
    noteOver: (n: string, max: string): string => `${n} / ${max} 文字 · 長すぎます`,
    resend: "もう一度送る",
    saved: (value: string, at: string, date: string): string => `保存しました: ${value} · 記入 ${at}（対象の日 ${date}）`,
    savedNoteOnly: (at: string, date: string): string => `保存しました: ${at}（対象の日 ${date}）`,
    held: (max: string): string => `ひとことが長すぎるので保存していません（${max} 文字まで）`,
    unreachable: "サーバに届きませんでした。選んだ値とひとことはそのまま残っています",
    rejection: {
      invalid_note: (max: string): string => `ひとことが長すぎます（${max} 文字まで）`,
      invalid_feeling_target: "この日（滞在）には書けませんでした。画面を読み直してください",
    },
    rejectedOther: (kind: string): string => `受け付けられませんでした（${kind}）`,
    otherSide: (phase: string, value: string): string => `${phase}に書いた: ${value}`,
    phaseWord: { before: "前", after: "後" } as Record<string, string>,
    history: (n: number): string => `書き直した履歴 ${n} 件`,
    historyRow: {
      energy: (signed: string): string => `元気さ ${signed}`,
      note: (note: string, cut: boolean): string => `「${note}${cut ? "…" : ""}」`,
      writtenAt: (at: string): string => `記入 ${at}`,
    },
    historySeparator: " · ",
    at: (mo: number, d: number, hm: string): string => `${mo} 月 ${d} 日 ${hm}`,

    /** 滞在の行（FeelParts.tsx。design D13） */
    staysHeading: "滞在ごとの気分",
    staysLoading: "滞在を読み込み中…",
    staysNone: "この日の滞在はありません",
    staysFailed: "滞在を読み出せませんでした（滞在が無い、という意味ではありません）",
    stayRange: (from: string, to: string): string => `${from} – ${to}`,
    stayGroup: (range: string): string => `滞在 ${range} の快–不快`,
    carriedFrom: "前の日から",
    hiddenStays: (n: number): string => `消した滞在の気分 ${n} 件（ここには出しません）`,
    savedStay: (value: string, at: string, range: string): string => `保存しました: ${value} · 記入 ${at}（滞在 ${range}）`,

    /** 1 日の画面（S-2）の頭の欄と滞在の行（DayFeeling.tsx。design D14 / D20） */
    dayLabel: (value: string): string => `気分 ${value}`,
    dayValue: (signed: string, word: string): string => `${signed} ${word}`,
    dayStays: (n: number): string => `滞在ごとに ${n} 件`,
    dayNoValence: "記入あり",
    dayMissing: "未記入",
    dayBeforeStart: "使い始める前",
    dayFuture: "これから",
    dayLoading: "読み込み中…",
    dayFailed: "気分を読み出せませんでした",
    dayWrite: "書く",

    /** 暦（design D14） */
    calendarHeading: "この 30 日",
    calendarFailed: "暦を読み出せませんでした（未記入、という意味ではありません）",
    weekdays: ["日", "月", "火", "水", "木", "金", "土"] as readonly string[],
    calendarDay: (date: string, state: string): string => (state === "" ? date : `${date} ${state}`),
    calendarState: {
      recorded: "記入あり",
      missing: "未記入",
      before_start: "使い始める前",
      future: "これから",
    } as Record<string, string>,

    /** 通知の時刻（design D10） */
    reminderHeading: "通知の時刻",
    reminderCurrent: (at: string): string => `いまの通知の時刻: ${at}`,
    reminderUnknown: "いまの通知の時刻を読み出せませんでした",
    reminderLabel: "時刻（時:分）",
    reminderSave: "通知の時刻を保存",
    reminderSaved: (at: string): string => `通知の時刻を ${at} にしました`,
    reminderNotSaved: "通知の時刻を保存できませんでした。入れた時刻はそのまま残っています",
  },
};

/** 文言の表の形。言語を足すときはこの形を満たす表を作る。 */
export type Messages = typeof ja;

/** 画面ごとの文言を引く。 */
export function msg<K extends keyof Messages>(screen: K): Messages[K] {
  return ja[screen];
}
