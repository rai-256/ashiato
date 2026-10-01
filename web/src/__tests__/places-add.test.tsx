// SPDX-License-Identifier: AGPL-3.0-only
/**
 * 場所を足す（ST21 / tasks 9.1 / design D13）。応答を固定して、選択肢の数・送るもの・押せる状態を見る。
 * 画面の Scenario の印は `web/e2e`（本物のブラウザ）が持つ。ここは補助。
 */
import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { PlacesView } from "../PlacesView";
import type { Candidate } from "../places";

const json = (status: number, body: unknown): Response =>
  ({ ok: status >= 200 && status < 300, status, json: () => Promise.resolve(body) }) as Response;
const HOURS = Array.from({ length: 24 }, () => 0);
const accepted = (n: number): Response =>
  json(200, Array.from({ length: n }, () => ({ id: "x", duplicate: false, accepted: true, error: null })));
const refused = (kind: string): Response => json(400, [{ id: null, duplicate: false, accepted: false, error: kind }]);

function cand(i: number): Candidate {
  return {
    lat: 35 + i / 100,
    lon: 139 + i / 100,
    stays: { count: i + 1, minutes: 30 * (i + 1), first_day: "2026-08-01", last_day: "2026-09-30", hours: HOURS },
  };
}

interface Call {
  url: string;
  method: string;
  body: unknown;
}
let calls: Call[];
let candidates: Candidate[];
/** 器・取り込みの応答。呼ぶたびに先頭から取り、尽きたら最後を使う */
let containerReplies: (Response | Error)[];
let ingestReplies: (Response | Error)[];
let holdIngest: Promise<void> | null;

function next(list: (Response | Error)[]): Promise<Response> {
  const r = list.length > 1 ? list.shift()! : list[0];
  return r instanceof Error ? Promise.reject(r) : Promise.resolve(r);
}

beforeEach(() => {
  calls = [];
  candidates = [];
  containerReplies = [json(200, { id: "x" })];
  ingestReplies = [accepted(3)];
  holdIngest = null;
  vi.stubGlobal(
    "fetch",
    vi.fn(async (url: string, init?: RequestInit) => {
      const method = init?.method ?? "GET";
      calls.push({ url, method, body: init?.body ? JSON.parse(init.body as string) : null });
      if (url === "/api/places" && method === "GET") return json(200, { today: "2026-10-01", places: [] });
      if (url === "/api/places/candidates") return json(200, { candidates });
      if (url === "/api/places" && method === "POST") return next(containerReplies);
      if (url === "/api/ingest") {
        if (holdIngest) await holdIngest;
        return next(ingestReplies);
      }
      throw new Error(`想定外の求め ${method} ${url}`);
    }),
  );
});
afterEach(() => vi.unstubAllGlobals());

const posts = (url: string): Call[] => calls.filter((c) => c.url === url && c.method === "POST");
const records = (c: Call): Record<string, unknown>[] =>
  (c.body as { raw: string }[]).map((i) => JSON.parse(i.raw) as Record<string, unknown>);

async function openAdd(): Promise<void> {
  render(<PlacesView scheme="dark" />);
  fireEvent.click(await screen.findByRole("button", { name: "場所を足す" }));
}
async function openForm(i = 0): Promise<HTMLElement> {
  await openAdd();
  const buttons = await screen.findAllByRole("button", { name: "名前を付ける" });
  fireEvent.click(buttons[i]);
  return screen.findByTestId("place-add-form");
}

describe("places-add: 名前の無い居た所", () => {
  it("13 か所なら 10 件と「残り 3 か所」。押すと 13 件", async () => {
    candidates = Array.from({ length: 13 }, (_, i) => cand(i));
    await openAdd();
    expect(await screen.findAllByTestId("place-candidate")).toHaveLength(10);
    fireEvent.click(screen.getByRole("button", { name: "残り 3 か所" }));
    expect(screen.getAllByTestId("place-candidate")).toHaveLength(13);
    expect(screen.queryByRole("button", { name: /残り/ })).toBeNull();
  });

  it("10 か所ちょうどなら「残り」は出ない", async () => {
    candidates = Array.from({ length: 10 }, (_, i) => cand(i));
    await openAdd();
    expect(await screen.findAllByTestId("place-candidate")).toHaveLength(10);
    expect(screen.queryByRole("button", { name: /残り/ })).toBeNull();
  });

  it("各居た所に合計・帯・件数・最初と最後の日・「名前を付ける」が出る", async () => {
    candidates = [cand(1)];
    await openAdd();
    const row = await screen.findByTestId("place-candidate");
    expect(within(row).getByTestId("candidate-total").textContent).toBe("1 時間");
    expect(within(row).getAllByRole("img", { name: /24 区分の帯/ })).toHaveLength(1);
    expect(within(row).getByTestId("candidate-meta").textContent).toBe("2 件 · 2026-08-01 〜 2026-09-30");
    expect(within(row).getByRole("button", { name: "名前を付ける" })).toBeTruthy();
  });

  it("居た所が 1 つも無いと「居た所がまだありません」（失敗とは別）", async () => {
    await openAdd();
    expect((await screen.findByTestId("candidates-empty")).textContent).toContain("居た所がまだありません");
    expect(screen.queryByRole("alert")).toBeNull();
  });

  it("居た所の読み出しの失敗は失敗として出し、「居た所が無い」と読ませない", async () => {
    vi.stubGlobal("fetch", vi.fn(async (url: string) => (url === "/api/places" ? json(200, { today: "x", places: [] }) : json(500, {}))));
    await openAdd();
    expect((await screen.findByTestId("candidates-failed")).textContent).toContain("読み出せませんでした");
    expect(screen.queryByTestId("candidates-empty")).toBeNull();
  });

  it("開くまでは居た所を読まない", async () => {
    render(<PlacesView scheme="dark" />);
    await screen.findByRole("button", { name: "場所を足す" });
    expect(calls.map((c) => c.url)).toEqual(["/api/places"]);
  });
});

describe("places-add: 名前を付けるフォーム", () => {
  it("名前・座標（書き換えられない）・広さ（最初は 100 m）・補足があり、緯度経度の欄も位置の操作も無い", async () => {
    candidates = [cand(0)];
    const form = await openForm();
    expect(within(form).getByLabelText("名前")).toBeTruthy();
    expect(within(form).getByLabelText("補足")).toBeTruthy();
    expect(within(form).getByTestId("place-add-coord").textContent).toBe("35.0000, 139.0000");
    expect(form.querySelectorAll("input[type=text], input:not([type])")).toHaveLength(2);
    const radios = within(form).getAllByRole("radio") as HTMLInputElement[];
    expect(radios.map((r) => r.parentElement?.textContent)).toEqual(["50 m", "100 m", "200 m", "300 m"]);
    expect(radios.map((r) => r.checked)).toEqual([false, true, false, false]);
    expect(within(form).queryByText(/いまの位置|現在地/)).toBeNull();
    expect(within(form).getByRole("button", { name: "登録する" })).toBeTruthy();
    expect(within(form).getByRole("button", { name: "やめる" })).toBeTruthy();
  });

  it("「やめる」で閉じる", async () => {
    candidates = [cand(0)];
    const form = await openForm();
    fireEvent.click(within(form).getByRole("button", { name: "やめる" }));
    expect(screen.queryByTestId("place-add-form")).toBeNull();
  });

  it("登録は器 → 記録の束の順。座標は居た所の中心・広さは選んだ値・補足は入れたときだけ", async () => {
    candidates = [cand(0), cand(1)];
    const form = await openForm(1);
    fireEvent.change(within(form).getByLabelText("名前"), { target: { value: "スーパー" } });
    fireEvent.click(within(form).getByRole("radio", { name: "200 m" }));
    fireEvent.change(within(form).getByLabelText("補足"), { target: { value: "夕方に寄る" } });
    fireEvent.click(within(form).getByRole("button", { name: "登録する" }));
    await waitFor(() => expect(screen.queryByTestId("place-add-form")).toBeNull());

    const order = calls.filter((c) => c.method === "POST").map((c) => c.url);
    expect(order).toEqual(["/api/places", "/api/ingest"]);
    const placeId = (posts("/api/places")[0].body as { id: string }).id;
    const recs = records(posts("/api/ingest")[0]);
    expect(recs.map((r) => r.field)).toEqual(["name", "coord", "radius", "note"]);
    for (const r of recs) expect(r.place).toBe(placeId);
    expect(recs[0].name).toBe("スーパー");
    expect(recs[1]).toMatchObject({ lat: 35.01, lon: 139.01, change: "first", valid_from: null, supersedes: null });
    expect(recs[2].radius_m).toBe(200);
    expect(recs[3].note).toBe("夕方に寄る");
    expect(new Set(recs.map((r) => r.nonce)).size).toBe(4);
    for (const i of posts("/api/ingest")[0].body as { logical_source: string; origin: string }[]) {
      expect(i).toMatchObject({ logical_source: "s01-place", origin: "authored" });
    }
  });

  it("広さは既定の 100 m でも送り、補足が空なら補足の記録は送らない", async () => {
    candidates = [cand(0)];
    ingestReplies = [accepted(3)];
    const form = await openForm();
    fireEvent.change(within(form).getByLabelText("名前"), { target: { value: "駅" } });
    fireEvent.click(within(form).getByRole("button", { name: "登録する" }));
    await waitFor(() => expect(posts("/api/ingest")).toHaveLength(1));
    const recs = records(posts("/api/ingest")[0]);
    expect(recs.map((r) => r.field)).toEqual(["name", "coord", "radius"]);
    expect(recs[2].radius_m).toBe(100);
  });

  it("書いた日時は押した時刻（入力欄は無い）", async () => {
    candidates = [cand(0)];
    const form = await openForm();
    fireEvent.change(within(form).getByLabelText("名前"), { target: { value: "駅" } });
    vi.useFakeTimers({ toFake: ["Date"] });
    vi.setSystemTime(new Date("2026-10-02T03:04:05Z"));
    try {
      fireEvent.click(within(form).getByRole("button", { name: "登録する" }));
    } finally {
      vi.useRealTimers();
    }
    await waitFor(() => expect(posts("/api/ingest")).toHaveLength(1));
    for (const i of posts("/api/ingest")[0].body as { event_time: string }[]) {
      expect(i.event_time).toBe("2026-10-02T03:04:05.000Z");
    }
    expect(form.querySelectorAll("input[type=date], input[type=datetime-local], input[type=time]")).toHaveLength(0);
  });

  it("受理で閉じ、場所と居た所を読み直す", async () => {
    candidates = [cand(0)];
    const form = await openForm();
    fireEvent.change(within(form).getByLabelText("名前"), { target: { value: "駅" } });
    const before = calls.length;
    fireEvent.click(within(form).getByRole("button", { name: "登録する" }));
    await waitFor(() => expect(screen.queryByTestId("place-add-form")).toBeNull());
    await waitFor(() => {
      const after = calls.slice(before).map((c) => `${c.method} ${c.url}`);
      expect(after).toContain("GET /api/places");
      expect(after).toContain("GET /api/places/candidates");
    });
  });

  it("送っている間は「登録する」を押せない", async () => {
    candidates = [cand(0)];
    let release!: () => void;
    holdIngest = new Promise<void>((r) => (release = r));
    const form = await openForm();
    fireEvent.change(within(form).getByLabelText("名前"), { target: { value: "駅" } });
    const btn = within(form).getByRole("button", { name: "登録する" }) as HTMLButtonElement;
    fireEvent.click(btn);
    await waitFor(() => expect(btn.disabled).toBe(true));
    fireEvent.click(btn);
    release();
    await waitFor(() => expect(screen.queryByTestId("place-add-form")).toBeNull());
    expect(posts("/api/ingest")).toHaveLength(1);
  });

  it("届かず押し直すと、同じ器の識別子と同じ原文を送る。入力を変えれば組み直す", async () => {
    candidates = [cand(0)];
    ingestReplies = [new Error("net"), accepted(3)];
    const form = await openForm();
    fireEvent.change(within(form).getByLabelText("名前"), { target: { value: "駅" } });
    fireEvent.click(within(form).getByRole("button", { name: "登録する" }));
    expect((await screen.findByTestId("place-problem")).textContent).toBe("サーバに届きませんでした。入力はそのまま残っています");
    fireEvent.click(within(form).getByRole("button", { name: "登録する" }));
    await waitFor(() => expect(screen.queryByTestId("place-add-form")).toBeNull());
    const [a, b] = posts("/api/ingest");
    expect(b.body).toEqual(a.body);
    expect(posts("/api/places")[1].body).toEqual(posts("/api/places")[0].body);
  });

  it("入力を変えて押し直すと器の識別子も原文も組み直す", async () => {
    candidates = [cand(0)];
    ingestReplies = [new Error("net"), accepted(3)];
    const form = await openForm();
    const name = within(form).getByLabelText("名前");
    fireEvent.change(name, { target: { value: "駅" } });
    fireEvent.click(within(form).getByRole("button", { name: "登録する" }));
    await screen.findByTestId("place-problem");
    fireEvent.change(name, { target: { value: "駅前" } });
    fireEvent.click(within(form).getByRole("button", { name: "登録する" }));
    await waitFor(() => expect(posts("/api/ingest")).toHaveLength(2));
    const [a, b] = posts("/api/ingest");
    expect(records(a)[0].nonce).not.toBe(records(b)[0].nonce);
    expect((posts("/api/places")[1].body as { id: string }).id).not.toBe((posts("/api/places")[0].body as { id: string }).id);
  });

  it("器が place_id_taken で断られた後の押し直しは識別子を作り直す", async () => {
    candidates = [cand(0)];
    containerReplies = [json(400, { error: "place_id_taken" }), json(200, { id: "x" })];
    const form = await openForm();
    fireEvent.change(within(form).getByLabelText("名前"), { target: { value: "駅" } });
    fireEvent.click(within(form).getByRole("button", { name: "登録する" }));
    expect((await screen.findByTestId("place-problem")).textContent).toBe(
      "場所を作れませんでした。もう一度「登録する」を押してください",
    );
    expect(posts("/api/ingest")).toHaveLength(0);
    fireEvent.click(within(form).getByRole("button", { name: "登録する" }));
    await waitFor(() => expect(screen.queryByTestId("place-add-form")).toBeNull());
    const [first, second] = posts("/api/places");
    expect((second.body as { id: string }).id).not.toBe((first.body as { id: string }).id);
    expect(records(posts("/api/ingest")[0])[0].place).toBe((second.body as { id: string }).id);
  });

  it("断られると種別ごとの文が出て、入れた広さと補足が残る", async () => {
    candidates = [cand(0)];
    ingestReplies = [refused("invalid_place_name")];
    const form = await openForm();
    fireEvent.click(within(form).getByRole("radio", { name: "300 m" }));
    fireEvent.change(within(form).getByLabelText("補足"), { target: { value: "メモ" } });
    fireEvent.click(within(form).getByRole("button", { name: "登録する" }));
    expect((await screen.findByTestId("place-problem")).textContent).toBe("名前が空です。名前を入れてください");
    expect((within(form).getByRole("radio", { name: "300 m" }) as HTMLInputElement).checked).toBe(true);
    expect((within(form).getByLabelText("補足") as HTMLInputElement).value).toBe("メモ");
  });

  it("一部だけ断られた束も断られたものの理由を出す。5xx と 401 は届かなかった", async () => {
    candidates = [cand(0)];
    ingestReplies = [json(200, [{ id: "a", accepted: true, error: null }, { id: "b", accepted: false, error: "invalid_radius" }])];
    const form = await openForm();
    fireEvent.click(within(form).getByRole("button", { name: "登録する" }));
    expect((await screen.findByTestId("place-problem")).textContent).toBe("広さが範囲の外です");
    ingestReplies = [json(401, {})];
    fireEvent.click(within(form).getByRole("button", { name: "登録する" }));
    await waitFor(() => expect(screen.getByTestId("place-problem").textContent).toContain("届きませんでした"));
  });

  it("操作できるものは 24 px 以上でフォーカスの印が付く", async () => {
    candidates = [cand(0)];
    const form = await openForm();
    const targets = [...form.querySelectorAll("button, input, select")] as HTMLElement[];
    expect(targets.length).toBeGreaterThan(5);
    for (const el of targets) {
      expect(parseFloat(el.style.minHeight)).toBeGreaterThanOrEqual(24);
      expect(parseFloat(el.style.minWidth)).toBeGreaterThanOrEqual(24);
      expect(el.hasAttribute("data-focus-ring")).toBe(true);
    }
  });
});
