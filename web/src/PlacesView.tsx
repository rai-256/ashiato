// SPDX-License-Identifier: AGPL-3.0-only
import { useCallback, useEffect, useState } from "react";
import { control } from "./controls";
import { msg } from "./messages";
import { Band } from "./PlaceBand";
import { AddPanel, ChangeForm, useChangeOpen, type ChangeKind } from "./PlaceForms";
import {
  coordLabel,
  durationLabel,
  isPlacesData,
  previousCoordLabel,
  type Place,
  type PlacesData,
} from "./places";
import { FOCUS_ATTR } from "./scheme";
import { sectionStyle } from "./surface";
import { font, SCHEMES, SPACE, TYPE, tone, type Scheme } from "./tokens";

const m = msg("master").places;
const common = msg("common");

/** 読み出しの状態。**「読み込み中」「失敗」「場所がまだ無い」を分ける**（spec）。 */
type Load<T> = { at: "loading" } | { at: "ok"; value: T } | { at: "failed"; why: string };

/**
 * S-6 マスタ管理 —— 場所（ST21 / design D12）。
 *
 * 構造は本人が proto で決めたとおり（Q1 の逐語。`deep.md`）: **登録した場所のカードだけ**を、
 * 読み出した順（最近居た順）に 1 枚ずつ。前の名前・座標は押したときだけ。
 * **地図・`navigator.geolocation`・外部のフォントや画像を使わない**（C10）。場所の識別子は画面に出さない。
 */
export function PlacesView({ scheme }: { scheme: Scheme }): React.ReactElement {
  const [data, setData] = useState<Load<PlacesData>>({ at: "loading" });
  const [adding, setAdding] = useState(false);
  /** 受理のたびに進めて、開いている居た所の一覧を読み直させる */
  const [reloadKey, setReloadKey] = useState(0);

  const load = useCallback(async (): Promise<void> => {
    try {
      const res = await fetch("/api/places");
      if (!res.ok) throw new Error(`status_${res.status}`);
      const body: unknown = await res.json();
      if (!isPlacesData(body)) throw new Error("unexpected_shape");
      setData({ at: "ok", value: body });
    } catch (e: unknown) {
      setData({ at: "failed", why: e instanceof Error ? e.message : "unknown" });
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  const reload = useCallback(async (): Promise<void> => {
    await load();
    setReloadKey((k) => k + 1);
  }, [load]);

  const registered = data.at === "ok" ? data.value.places.map((p) => p.coord) : [];
  return (
    <section data-testid="places-view" aria-label={m.section}>
      <div style={{ marginBottom: SPACE.underTools }}>
        <button type="button" aria-expanded={adding} {...{ [FOCUS_ATTR]: "" }} style={control(scheme)} onClick={() => setAdding(!adding)}>
          {m.add}
        </button>
      </div>
      {adding && <AddPanel scheme={scheme} reloadKey={reloadKey} onDone={reload} />}
      {data.at === "loading" && <p data-testid="places-loading">{common.loading}</p>}
      {/* **読み出しの失敗と「場所がまだ無い」を混ぜない**（spec）。混ぜるとサーバが落ちている間ずっと「場所が無い」と読める */}
      {data.at === "failed" && (
        <p role="alert" data-testid="places-failed">
          {m.failed(data.why)}
        </p>
      )}
      {data.at === "ok" && data.value.places.length === 0 && <p data-testid="places-empty">{m.empty}</p>}
      {data.at === "ok" &&
        data.value.places.map((p) => (
          <PlaceCard key={p.id} place={p} registered={registered} scheme={scheme} reloadKey={reloadKey} onDone={reload} />
        ))}
    </section>
  );
}

const CHANGES: [ChangeKind, string][] = [
  ["name", m.changeName],
  ["radius", m.changeRadius],
  ["coord", m.changeCoord],
];

/** 場所 1 つぶんのカード。 */
function PlaceCard({
  place,
  registered,
  scheme,
  reloadKey,
  onDone,
}: {
  place: Place;
  registered: { lat: number; lon: number }[];
  scheme: Scheme;
  reloadKey: number;
  onDone: () => Promise<void>;
}): React.ReactElement {
  const c = SCHEMES[scheme];
  const [open, setOpen] = useState(false);
  const change = useChangeOpen();
  const previousCount = place.previous_names.length + place.previous_coords.length;
  const { stays } = place;
  return (
    <article
      data-testid="place-card"
      style={sectionStyle(c, c.surface1)}
    >
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "baseline", gap: SPACE.targetGap }}>
        <h2 data-testid="place-name" style={{ font: font(TYPE.h2, 600, 1.3), margin: 0 }}>
          {place.name}
        </h2>
        <span data-testid="place-total">{durationLabel(stays.minutes)}</span>
      </div>
      <p data-testid="place-meta" style={{ margin: 0, fontSize: TYPE.caption, color: tone(c.muted) }}>
        {stays.last_day === null ? m.neverStayed : m.lastDay(stays.last_day)}
        {m.sep}
        {m.radius(place.radius_m)}
      </p>
      <Band hours={stays.hours} scheme={scheme} />
      <p data-testid="place-coord" style={{ margin: 0, fontSize: TYPE.caption, color: tone(c.muted) }}>
        {coordLabel(place.coord.lat, place.coord.lon)}
      </p>
      {previousCount > 0 && (
        <>
          <button
            type="button"
            aria-expanded={open}
            {...{ [FOCUS_ATTR]: "" }}
            style={control(scheme)}
            onClick={() => setOpen(!open)}
          >
            {m.previous(previousCount, open)}
          </button>
          {open && (
            <ul data-testid="place-previous" style={{ margin: "4px 0 0", paddingLeft: 20, fontSize: TYPE.caption }}>
              {place.previous_names.map((n) => (
                <li key={n.record_id} data-testid="previous-name">
                  {m.previousName(n.name)}
                </li>
              ))}
              {place.previous_coords.map((k) => (
                <li key={k.record_id} data-testid="previous-coord">
                  {coordLabel(k.lat, k.lon)}
                  {m.sep}
                  {previousCoordLabel(k, place)}
                </li>
              ))}
            </ul>
          )}
        </>
      )}
      {/* **3 つの操作は 1 行に並べる**（design D21。折り返すとカードが高くなり、幅 360 × 高さ 640 の 1 画面目に 3 枚入らない） */}
      <div style={{ display: "flex", flexWrap: "nowrap", gap: 4, marginTop: 4 }}>
        {CHANGES.map(([kind, label]) => (
          <button
            key={kind}
            type="button"
            aria-expanded={change.open === kind}
            {...{ [FOCUS_ATTR]: "" }}
            style={{ ...control(scheme), padding: "4px 4px", flex: "1 1 auto" }}
            onClick={() => change.toggle(kind)}
          >
            {label}
          </button>
        ))}
      </div>
      {change.open !== null && (
        <ChangeForm
          key={change.open}
          kind={change.open}
          place={place}
          registered={registered}
          scheme={scheme}
          reloadKey={reloadKey}
          onDone={async () => {
            change.close();
            await onDone();
          }}
          onCancel={change.close}
        />
      )}
    </article>
  );
}
