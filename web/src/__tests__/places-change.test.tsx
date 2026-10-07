// SPDX-License-Identifier: AGPL-3.0-only
/**
 * 場所の名前・広さ・座標を変える（ST21 / tasks 9.2 / design D13 / D14）。
 * 応答を固定して、選択肢・送るもの・押せる状態を見る。画面の Scenario の印は `web/e2e` が持つ。
 */
import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { PlacesView } from "../PlacesView";
import type { Candidate, Place } from "../places";

const json = (status: number, body: unknown): Response =>
  ({ ok: status >= 200 && status < 300, status, json: () => Promise.resolve(body) }) as Response;
const HOURS = Array.from({ length: 24 }, () => 0);
const T = { written_at: "2026-09-01T10:00:00+09:00", ingested_at: "2026-09-01T01:00:00Z" };
const accepted = (): Response => json(200, [{ id: "x", duplicate: false, accepted: true, error: null }]);
const refused = (kind: string): Response => json(400, [{ id: null, duplicate: false, accepted: false, error: kind }]);

const PLACE: Place = {
  id: "11111111-aaaa-4bbb-8ccc-222222222222",
  name: "職場",
  note: null,
  radius_m: 200,
  name_record: { record_id: "n", ...T },
  coord: { record_id: "coord-now", lat: 35.5, lon: 139.5, change: "first", valid_from: null, supersedes: null, ...T },
  stays: { count: 1, minutes: 60, last_day: "2026-09-30", hours: HOURS },
  previous_names: [],
  previous_coords: [],
};

function cand(i: number): Candidate {
  return {
    lat: 35 + i / 100,
    lon: 139 + i / 100,
    stays: { count: 1, minutes: 30, first_day: "2026-08-01", last_day: "2026-09-30", hours: HOURS },
  };
}

interface Call {
  url: string;
  method: string;
  body: unknown;
}
let calls: Call[];
let candidates: Candidate[];
let ingestReplies: (Response | Error)[];
let holdIngest: Promise<void> | null;

beforeEach(() => {
  calls = [];
  candidates = [cand(0), cand(1)];
  ingestReplies = [accepted()];
  holdIngest = null;
  vi.stubGlobal(
    "fetch",
    vi.fn(async (url: string, init?: RequestInit) => {
      const method = init?.method ?? "GET";
      calls.push({ url, method, body: init?.body ? JSON.parse(init.body as string) : null });
      if (url === "/api/places") return json(200, { today: "2026-10-01", places: [PLACE] });
      if (url === "/api/places/candidates") return json(200, { candidates });
      if (url === "/api/ingest") {
        if (holdIngest) await holdIngest;
        const r = ingestReplies.length > 1 ? ingestReplies.shift()! : ingestReplies[0];
        return r instanceof Error ? Promise.reject(r) : r;
      }
      throw new Error(`想定外の求め ${method} ${url}`);
    }),
  );
});
afterEach(() => vi.unstubAllGlobals());

const ingests = (): Call[] => calls.filter((c) => c.url === "/api/ingest");
const raws = (c: Call): Record<string, unknown>[] => (c.body as { raw: string }[]).map((i) => JSON.parse(i.raw) as Record<string, unknown>);

async function open(name: string): Promise<HTMLElement> {
  render(<PlacesView scheme="dark" />);
  fireEvent.click(await screen.findByRole("button", { name }));
  return screen.findByTestId("place-change-form");
}
const send = (form: HTMLElement): HTMLButtonElement => within(form).getByRole("button", { name: "変える" }) as HTMLButtonElement;

describe("places-change: 名前を変える", () => {
  it("名前の記録を 1 件送り、受理されたら閉じて場所を読み直す", async () => {
    const form = await open("名前を変える");
    fireEvent.change(within(form).getByLabelText("名前"), { target: { value: "本社" } });
    const before = calls.length;
    fireEvent.click(send(form));
    await waitFor(() => expect(screen.queryByTestId("place-change-form")).toBeNull());
    expect(ingests()).toHaveLength(1);
    const [rec] = raws(ingests()[0]);
    expect(rec).toMatchObject({ field: "name", name: "本社", place: PLACE.id });
    expect((ingests()[0].body as unknown[]).length).toBe(1);
    expect(calls.slice(before).map((c) => c.url)).toContain("/api/places");
  });

  it("送っている間は押せない", async () => {
    let release!: () => void;
    holdIngest = new Promise<void>((r) => (release = r));
    const form = await open("名前を変える");
    fireEvent.change(within(form).getByLabelText("名前"), { target: { value: "本社" } });
    fireEvent.click(send(form));
    await waitFor(() => expect(send(form).disabled).toBe(true));
    release();
    await waitFor(() => expect(screen.queryByTestId("place-change-form")).toBeNull());
  });

  it("届かなかったときは入力が残って届かなかったと出る。押し直しは同じ原文", async () => {
    ingestReplies = [new Error("net"), accepted()];
    const form = await open("名前を変える");
    fireEvent.change(within(form).getByLabelText("名前"), { target: { value: "本社" } });
    fireEvent.click(send(form));
    expect((await screen.findByTestId("place-problem")).textContent).toBe("サーバに届きませんでした。入力はそのまま残っています");
    expect((within(form).getByLabelText("名前") as HTMLInputElement).value).toBe("本社");
    fireEvent.click(send(form));
    await waitFor(() => expect(screen.queryByTestId("place-change-form")).toBeNull());
    expect(ingests()[1].body).toEqual(ingests()[0].body);
  });

  it("名前を変えて押し直すと組み直す", async () => {
    ingestReplies = [new Error("net"), accepted()];
    const form = await open("名前を変える");
    const name = within(form).getByLabelText("名前");
    fireEvent.change(name, { target: { value: "本社" } });
    fireEvent.click(send(form));
    await screen.findByTestId("place-problem");
    fireEvent.change(name, { target: { value: "本店" } });
    fireEvent.click(send(form));
    await waitFor(() => expect(ingests()).toHaveLength(2));
    expect(raws(ingests()[1])[0].nonce).not.toBe(raws(ingests()[0])[0].nonce);
  });

  it("断られると種別の文が出て入力が残る", async () => {
    ingestReplies = [refused("invalid_place_name")];
    const form = await open("名前を変える");
    fireEvent.change(within(form).getByLabelText("名前"), { target: { value: "" } });
    fireEvent.click(send(form));
    expect((await screen.findByTestId("place-problem")).textContent).toBe("名前が空です。名前を入れてください");
  });

  it("D13 の表: 種別ごとに文が違い、知らない種別は種別の名前つき", async () => {
    const expected: Record<string, string> = {
      invalid_radius: "広さが範囲の外です",
      invalid_valid_from: "「いつから」の日付が読めません",
      unknown_place: "この場所が見つかりません。画面を読み直してください",
      invalid_coord_supersedes: "直す座標が見つかりません。画面を読み直してください",
      invalid_coord_change: "この場所の座標の変え方が合いません。画面を読み直してください",
      invalid_coordinate: "座標が読めません",
      something_new: "受け付けられませんでした（something_new）",
    };
    const form = await open("名前を変える");
    // いまと同じ名前のうちは押せない（M3）ので、違う名前を入れる
    fireEvent.change(within(form).getByLabelText("名前"), { target: { value: "本社" } });
    for (const [kind, text] of Object.entries(expected)) {
      ingestReplies = [refused(kind)];
      fireEvent.click(send(form));
      await waitFor(() => expect(screen.getByTestId("place-problem").textContent).toBe(text));
    }
  });
});

describe("places-change: いまと同じ値", () => {
  it("名前・広さがいまと同じ値のうちは「変える」を押せない", async () => {
    const form = await open("名前を変える");
    const name = within(form).getByLabelText("名前") as HTMLInputElement;
    expect(name.value).toBe(PLACE.name);
    expect(send(form).disabled).toBe(true);
    fireEvent.change(name, { target: { value: "本社" } });
    expect(send(form).disabled).toBe(false);
    fireEvent.change(name, { target: { value: PLACE.name } });
    expect(send(form).disabled).toBe(true);
    fireEvent.click(within(form).getByRole("button", { name: "やめる" }));

    fireEvent.click(await screen.findByRole("button", { name: "広さを変える" }));
    const radius = await screen.findByTestId("place-change-form");
    expect(send(radius).disabled).toBe(true);
    fireEvent.click(within(radius).getByRole("radio", { name: "300 m" }));
    expect(send(radius).disabled).toBe(false);
    fireEvent.click(within(radius).getByRole("radio", { name: `${PLACE.radius_m} m` }));
    expect(send(radius).disabled).toBe(true);
    expect(ingests()).toHaveLength(0);
  });
});

describe("places-change: 広さを変える", () => {
  it("いまの広さが最初に選ばれ、選んだ広さで広さの記録を 1 件送る", async () => {
    const form = await open("広さを変える");
    const radios = within(form).getAllByRole("radio") as HTMLInputElement[];
    expect(radios.map((r) => r.parentElement?.textContent)).toEqual(["50 m", "100 m", "200 m", "300 m"]);
    expect(radios.map((r) => r.checked)).toEqual([false, false, true, false]);
    fireEvent.click(within(form).getByRole("radio", { name: "300 m" }));
    fireEvent.click(send(form));
    await waitFor(() => expect(screen.queryByTestId("place-change-form")).toBeNull());
    expect(raws(ingests()[0])).toEqual([expect.objectContaining({ field: "radius", radius_m: 300, place: PLACE.id })]);
  });

  it("入力を変えずに 2 回押すと同じ原文を送る", async () => {
    ingestReplies = [new Error("net"), accepted()];
    const form = await open("広さを変える");
    fireEvent.click(within(form).getByRole("radio", { name: "300 m" }));
    fireEvent.click(send(form));
    await screen.findByTestId("place-problem");
    fireEvent.click(send(form));
    await waitFor(() => expect(screen.queryByTestId("place-change-form")).toBeNull());
    expect(ingests()[1].body).toEqual(ingests()[0].body);
  });
});

describe("places-change: 座標を変える", () => {
  it("居た所が選択肢で、緯度経度の欄も位置の操作も無い。登録した場所の座標は選択肢に出ない", async () => {
    candidates = [cand(0), cand(1), { ...cand(2), lat: PLACE.coord.lat, lon: PLACE.coord.lon }];
    const form = await open("座標を変える");
    await within(form).findAllByRole("radio", { name: /35\.\d{4}, 139\.\d{4}/ });
    const options = within(form).getAllByTestId("coord-option");
    expect(options).toHaveLength(2);
    expect(form.querySelectorAll("input[type=text], input:not([type])")).toHaveLength(0);
    expect(within(form).queryByText(/いまの位置|現在地/)).toBeNull();
  });

  it("名前の無い居た所が 10 件を超えると「残り N か所」で全部出る", async () => {
    candidates = Array.from({ length: 12 }, (_, i) => cand(i));
    const form = await open("座標を変える");
    expect(await within(form).findAllByTestId("coord-option")).toHaveLength(10);
    fireEvent.click(within(form).getByRole("button", { name: "残り 2 か所" }));
    expect(within(form).getAllByTestId("coord-option")).toHaveLength(12);
  });

  it("直すか移ったかを選ぶまで送れない（居た所を選んでも）", async () => {
    const form = await open("座標を変える");
    fireEvent.click((await within(form).findAllByTestId("coord-option"))[0].querySelector("input")!);
    expect(send(form).disabled).toBe(true);
    fireEvent.click(within(form).getByRole("radio", { name: "前の座標が間違っていた" }));
    expect(send(form).disabled).toBe(false);
  });

  it("居た所を選ばないと送れない", async () => {
    const form = await open("座標を変える");
    await within(form).findAllByTestId("coord-option");
    fireEvent.click(within(form).getByRole("radio", { name: "前の座標が間違っていた" }));
    expect(send(form).disabled).toBe(true);
  });

  it("居た所が無いと「居た所がまだありません」と出て送れない", async () => {
    candidates = [];
    const form = await open("座標を変える");
    expect((await within(form).findByTestId("candidates-empty")).textContent).toContain("居た所がまだありません");
    expect(send(form).disabled).toBe(true);
  });

  it("居た所の読み出しが失敗したら失敗と出て、「居た所が無い」とは出ない", async () => {
    vi.stubGlobal("fetch", vi.fn(async (url: string) => (url === "/api/places" ? json(200, { today: "x", places: [PLACE] }) : json(500, {}))));
    const form = await open("座標を変える");
    expect((await within(form).findByTestId("candidates-failed")).textContent).toContain("読み出せませんでした");
    expect(within(form).queryByTestId("candidates-empty")).toBeNull();
    expect(send(form).disabled).toBe(true);
  });

  it("「前の座標が間違っていた」はいまの座標の記録を直す先にして、選んだ居た所の中心を送る", async () => {
    const form = await open("座標を変える");
    fireEvent.click((await within(form).findAllByTestId("coord-option"))[1].querySelector("input")!);
    fireEvent.click(within(form).getByRole("radio", { name: "前の座標が間違っていた" }));
    expect(within(form).queryByLabelText(/いつから/)).toBeNull();
    fireEvent.click(send(form));
    await waitFor(() => expect(screen.queryByTestId("place-change-form")).toBeNull());
    expect(raws(ingests()[0])).toEqual([
      expect.objectContaining({ field: "coord", lat: 35.01, lon: 139.01, change: "fix", supersedes: "coord-now", valid_from: null, place: PLACE.id }),
    ]);
  });

  it("「この場所が移った」は精度を先に選び、選んだ精度の欄だけを出し、前の時間もこの場所のままと出る", async () => {
    const form = await open("座標を変える");
    await within(form).findAllByTestId("coord-option");
    expect(within(form).queryByText(/前の座標で居た時間も/)).toBeNull();
    fireEvent.click(within(form).getByRole("radio", { name: "この場所が移った" }));
    expect(within(form).getByTestId("move-note").textContent).toBe("前の座標で居た時間も「職場」のまま");
    fireEvent.click(within(form).getByRole("radio", { name: "年月" }));
    expect(within(form).getByLabelText("いつから（年）")).toBeTruthy();
    expect(within(form).getByLabelText("いつから（月）")).toBeTruthy();
    expect(within(form).queryByLabelText("いつから（日）")).toBeNull();
    fireEvent.click(within(form).getByRole("radio", { name: "分からない" }));
    expect(within(form).queryByLabelText("いつから（年）")).toBeNull();
  });

  it("移ったの記録は精度と日付を持ち、supersedes は持たない", async () => {
    const form = await open("座標を変える");
    fireEvent.click((await within(form).findAllByTestId("coord-option"))[0].querySelector("input")!);
    fireEvent.click(within(form).getByRole("radio", { name: "この場所が移った" }));
    fireEvent.click(within(form).getByRole("radio", { name: "年月" }));
    fireEvent.change(within(form).getByLabelText("いつから（年）"), { target: { value: "2026" } });
    fireEvent.change(within(form).getByLabelText("いつから（月）"), { target: { value: "4" } });
    fireEvent.click(send(form));
    await waitFor(() => expect(screen.queryByTestId("place-change-form")).toBeNull());
    expect(raws(ingests()[0])[0]).toMatchObject({
      field: "coord",
      change: "move",
      valid_from: { precision: "month", date: "2026-04" },
      supersedes: null,
      lat: 35,
      lon: 139,
    });
  });

  it("断られると理由の文が出て、選んだ居た所と変え方が残る", async () => {
    ingestReplies = [refused("invalid_coord_supersedes")];
    const form = await open("座標を変える");
    fireEvent.click((await within(form).findAllByTestId("coord-option"))[1].querySelector("input")!);
    fireEvent.click(within(form).getByRole("radio", { name: "前の座標が間違っていた" }));
    fireEvent.click(send(form));
    expect((await screen.findByTestId("place-problem")).textContent).toBe("直す座標が見つかりません。画面を読み直してください");
    const options = within(form).getAllByTestId("coord-option");
    expect((options[1].querySelector("input") as HTMLInputElement).checked).toBe(true);
    expect((within(form).getByRole("radio", { name: "前の座標が間違っていた" }) as HTMLInputElement).checked).toBe(true);
  });

  it("操作できるものは 24 px 以上でフォーカスの印が付く", async () => {
    const form = await open("座標を変える");
    await within(form).findAllByTestId("coord-option");
    fireEvent.click(within(form).getByRole("radio", { name: "この場所が移った" }));
    const targets = [...form.querySelectorAll("button, input")] as HTMLElement[];
    expect(targets.length).toBeGreaterThan(5);
    for (const el of targets) {
      expect(parseFloat(el.style.minHeight)).toBeGreaterThanOrEqual(24);
      expect(parseFloat(el.style.minWidth)).toBeGreaterThanOrEqual(24);
      expect(el.hasAttribute("data-focus-ring")).toBe(true);
    }
  });
});
