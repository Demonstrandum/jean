#!/usr/bin/env python3
"""Grid carbon intensity forecast for your region (National Grid ESO Carbon Intensity API: free, no key), every 15 min.
Writes K.carbon.* from crates/ui/ts/schema.ts: slots [{at, grams}] (next 24 h, 30 min each), now, index (CarbonIndex), green (the 100 g line),
region, best {from, to, avg} (cleanest 2 h from now), mix [{fuel: Fuel, pct}]."""
import datetime
from _lib import config, get_json, put, forever
from _schema import CarbonIndex, Fuel

GREEN = 100
INDEX = {"very low": CarbonIndex.VeryLow, "low": CarbonIndex.Low, "moderate": CarbonIndex.Moderate, "high": CarbonIndex.High, "very high": CarbonIndex.VeryHigh}
FUEL = {"biomass": Fuel.Biomass, "coal": Fuel.Coal, "imports": Fuel.Imports, "gas": Fuel.Gas, "nuclear": Fuel.Nuclear, "other": Fuel.Other, "hydro": Fuel.Hydro, "solar": Fuel.Solar, "wind": Fuel.Wind}


def ms(dt):
    return int(dt.timestamp() * 1000)


def step():
    c = config()
    outward = c.get("postcode", "").replace(" ", "")[:-3]
    if not outward:
        raise RuntimeError("config.json needs a UK postcode (see config.example.json)")
    now_utc = datetime.datetime.now(datetime.timezone.utc)
    start = now_utc.replace(minute=0 if now_utc.minute < 30 else 30, second=0, microsecond=0)
    j = get_json(f"https://api.carbonintensity.org.uk/regional/intensity/{start.strftime('%Y-%m-%dT%H:%MZ')}/fw48h/postcode/{outward}")["data"]
    rows = []
    for r in j["data"]:
        t = datetime.datetime.fromisoformat(r["from"].replace("Z", "+00:00"))
        if not rows or t > rows[-1][0]:
            rows.append((t, r["intensity"]["forecast"], r["intensity"]["index"], r.get("generationmix", [])))
    rows = rows[:48]
    best = None
    for i in range(0, len(rows) - 3):
        w = rows[i:i + 4]
        avg = sum(g for _, g, _, _ in w) / 4
        if best is None or avg < best[2]:
            best = (w[0][0], w[-1][0] + datetime.timedelta(minutes=30), avg)
    put({"carbon.slots": [{"at": ms(t), "grams": g} for t, g, _, _ in rows], "carbon.now": rows[0][1], "carbon.index": INDEX[rows[0][2]],
         "carbon.region": j.get("shortname"), "carbon.green": GREEN,
         "carbon.mix": [{"fuel": FUEL[m["fuel"]], "pct": m["perc"]} for m in rows[0][3] if m["fuel"] in FUEL],
         "carbon.best": {"from": ms(best[0]), "to": ms(best[1]), "avg": round(best[2])} if best else None})


if __name__ == "__main__":
    forever(900, step, retry=120)
