// SPDX-License-Identifier: AGPL-3.0-only
import { useCallback, useEffect, useState } from "react";
import { FOCUS_ATTR } from "./DayView";
import { control } from "./controls";
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
import { SCHEMES, tone, type Scheme } from "./tokens";

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
    <section data-testid="places-view" aria-label="場所">
      <div style={{ marginBottom: 8 }}>
        <button type="button" aria-expanded={adding} {...{ [FOCUS_ATTR]: "" }} style={control(scheme)} onClick={() => setAdding(!adding)}>
          場所を足す
        </button>
      </div>
      {adding && <AddPanel scheme={scheme} reloadKey={reloadKey} onDone={reload} />}
      {data.at === "loading" && <p data-testid="places-loading">読み込み中…</p>}
      {/* **読み出しの失敗と「場所がまだ無い」を混ぜない**（spec）。混ぜるとサーバが落ちている間ずっと「場所が無い」と読める */}
      {data.at === "failed" && (
        <p role="alert" data-testid="places-failed">
          場所を読み出せませんでした（{data.why}）。
        </p>
      )}
      {data.at === "ok" && data.value.places.length === 0 && <p data-testid="places-empty">場所がまだありません。</p>}
      {data.at === "ok" &&
        data.value.places.map((p) => (
          <PlaceCard key={p.id} place={p} registered={registered} scheme={scheme} reloadKey={reloadKey} onDone={reload} />
        ))}
    </section>
  );
}

const CHANGES: [ChangeKind, string][] = [
  ["name", "名前を変える"],
  ["radius", "広さを変える"],
  ["coord", "座標を変える"],
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
      style={{ background: tone(c.surface1), borderRadius: 8, padding: 8, marginBottom: 8 }}
    >
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "baseline", gap: 8 }}>
        <h2 data-testid="place-name" style={{ font: "600 16px/1.4 system-ui, sans-serif", margin: 0 }}>
          {place.name}
        </h2>
        <span data-testid="place-total">{durationLabel(stays.minutes)}</span>
      </div>
      <p data-testid="place-meta" style={{ margin: 0, fontSize: 14, color: tone(c.muted) }}>
        {stays.last_day === null ? "まだ居たことが無い" : `最後に居た日 ${stays.last_day}`} · 広さ {place.radius_m} m
      </p>
      <Band hours={stays.hours} scheme={scheme} />
      <p data-testid="place-coord" style={{ margin: 0, fontSize: 14, color: tone(c.muted) }}>
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
            前の名前・座標 {previousCount} {open ? "▾" : "▸"}
          </button>
          {open && (
            <ul data-testid="place-previous" style={{ margin: "4px 0 0", paddingLeft: 20, fontSize: 14 }}>
              {place.previous_names.map((n) => (
                <li key={n.record_id} data-testid="previous-name">
                  前の名前: {n.name}
                </li>
              ))}
              {place.previous_coords.map((k) => (
                <li key={k.record_id} data-testid="previous-coord">
                  {coordLabel(k.lat, k.lon)} · {previousCoordLabel(k, place)}
                </li>
              ))}
            </ul>
          )}
        </>
      )}
      <div style={{ display: "flex", flexWrap: "wrap", gap: 8, marginTop: 4 }}>
        {CHANGES.map(([kind, label]) => (
          <button key={kind} type="button" aria-expanded={change.open === kind} {...{ [FOCUS_ATTR]: "" }} style={control(scheme)} onClick={() => change.toggle(kind)}>
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
