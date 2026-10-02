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
    toDay: "1 日の滞在へ",
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

  /** ソースごとの格子（CoverageGrid.tsx / coverage.ts） */
  grid: {
    succeeded: (named: string): string => ` — ${named} を引き継ぎ`,
    retired: (on: string | null): string => ` — ${on} に退役`,
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

  /** S-2 1 日の滞在（DayView.tsx / stays.ts） */
  day: {
    title: (dateLabel: string): string => `${dateLabel}の滞在`,
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
    /** 種類のしるし（線の輪の中の記号）。**文字に添えるだけ**で、読み上げには出さない（2026-10-12） */
    mark: { stay: "●", move: "➝", "no-record": "…", erased: "×", other: "◦" },
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

  /** S-6 マスタ管理（MasterView.tsx / attributes.ts） */
  master: {
    title: "マスタ管理",
    tabAttributes: "個人属性",
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
  },
};

/** 文言の表の形。言語を足すときはこの形を満たす表を作る。 */
export type Messages = typeof ja;

/** 画面ごとの文言を引く。 */
export function msg<K extends keyof Messages>(screen: K): Messages[K] {
  return ja[screen];
}
