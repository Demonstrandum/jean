#!/usr/bin/env python3
"""Your Octopus account (secrets/secrets.json: account id + API key), every 15 min: the tariff you are on, its unit rate and
standing charge, and your smart-meter usage. Works for a flat tariff (Flexible, fixed) and for half-hourly ones (Agile, Tracker).
Writes K.power.* from crates/ui/ts/schema.ts: tariff, kind (RateKind), rate, standing, slots/next_slots [{at, pence}] (half-hourly only), min/max {at, value}, avg, cheap {from, to, avg},
usage_day (ms), usage_slots, usage_kwh, usage_cost, week [{day, kwh}], usage_latest (ms), updated (ms). Instants are epoch ms; nothing is text for the display to parse."""
import base64, datetime, json, urllib.request, zoneinfo
from _lib import config, get_json, put, forever, log, secrets
from _schema import RateKind

API = "https://api.octopus.energy"


def auth_get(path, key):
    req = urllib.request.Request(API + path if path.startswith("/") else path, headers={"Authorization": "Basic " + base64.b64encode((key + ":").encode()).decode(), "User-Agent": "display-feed/1.0"})
    with urllib.request.urlopen(req, timeout=25) as r:
        return json.load(r)


def paged(path, key):
    out, url = [], path
    while url:
        j = auth_get(url, key)
        out += j["results"]
        url = j.get("next")
    return out


def utc(d):
    return d.astimezone(datetime.timezone.utc).strftime("%Y-%m-%dT%H:%MZ")


def step():
    c, sec = config(), secrets().get("octopus", {})
    key, acct = sec.get("api_key"), sec.get("account")
    if not key:
        raise RuntimeError("secrets/secrets.json needs octopus.api_key and octopus.account")
    tz = zoneinfo.ZoneInfo(c.get("tz", "Europe/London"))
    now = datetime.datetime.now(tz)
    midnight = now.replace(hour=0, minute=0, second=0, microsecond=0)
    prop = auth_get(f"/v1/accounts/{acct}/", key)["properties"][0]
    mp = prop["electricity_meter_points"][0]
    ag = next((a for a in mp["agreements"] if a["valid_to"] is None or datetime.datetime.fromisoformat(a["valid_to"].replace("Z", "+00:00")) > now), mp["agreements"][-1])
    tcode = ag["tariff_code"]                       # E-1R-<product>-<region>
    prod, region = tcode[5:-2], tcode[-1]
    info = get_json(f"{API}/v1/products/{prod}/")
    pay = sec.get("payment_method", "DIRECT_DEBIT")
    rates_url = f"{API}/v1/products/{prod}/electricity-tariffs/{tcode}/standard-unit-rates/?period_from={utc(midnight)}&period_to={utc(midnight + datetime.timedelta(days=2))}&page_size=200"
    rows = []
    while rates_url:
        j = get_json(rates_url)
        rows += j["results"]
        rates_url = j.get("next")
    # the newest rate in force first; flat tariffs list one per payment method
    rows = [r for r in rows if r.get("payment_method") in (None, pay)]
    half = datetime.timedelta(minutes=30)
    slots = []
    for r in rows:
        a = max(datetime.datetime.fromisoformat(r["valid_from"].replace("Z", "+00:00")).astimezone(tz), midnight - datetime.timedelta(days=60))
        b = datetime.datetime.fromisoformat(r["valid_to"].replace("Z", "+00:00")).astimezone(tz) if r["valid_to"] else now + datetime.timedelta(days=3)
        t = max(a, midnight)
        while t < min(b, midnight + datetime.timedelta(days=2)):
            slots.append((t, r["value_inc_vat"]))
            t += half
    slots.sort()
    by_t = {}
    for t, p in slots:
        by_t[t] = p                                  # a later rate for the same slot wins
    exp = sorted(by_t.items())
    today = [(t, p) for t, p in exp if t.date() == now.date()]
    tom = [(t, p) for t, p in exp if t.date() == (now + datetime.timedelta(days=1)).date()]
    ms = lambda dt: int(dt.timestamp() * 1000)
    cur = next((p for t, p in today if t <= now < t + half), today[-1][1] if today else None)
    rates = lambda s: [{"at": ms(t), "pence": round(p, 2)} for t, p in s]
    kind = RateKind.HalfHourly if len({p for _, p in exp}) > 1 else RateKind.Flat
    sc = get_json(f"{API}/v1/products/{prod}/electricity-tariffs/{tcode}/standing-charges/")["results"]
    sc = next((s["value_inc_vat"] for s in sc if s.get("payment_method") in (None, pay)), sc[0]["value_inc_vat"] if sc else 0)
    out = {"power.tariff": info.get("display_name", prod), "power.kind": kind, "power.rate": round(cur, 2) if cur is not None else None,
           "power.standing": round(sc, 2), "power.slots": [], "power.next_slots": [], "power.min": None, "power.max": None, "power.avg": None, "power.cheap": None,
           "power.updated": ms(now)}
    if kind == RateKind.HalfHourly and today:
        ps = [p for _, p in today]
        out["power.slots"], out["power.next_slots"] = rates(today), rates(tom)
        out["power.avg"] = round(sum(ps) / len(ps), 2)
        lo, hi = min(today, key=lambda x: x[1]), max(today, key=lambda x: x[1])
        out["power.min"], out["power.max"] = {"at": ms(lo[0]), "value": round(lo[1], 2)}, {"at": ms(hi[0]), "value": round(hi[1], 2)}
        ahead = [(t, p) for t, p in exp if t + half > now]
        best = None
        for i in range(0, max(0, len(ahead) - 3)):
            w = ahead[i:i + 4]
            avg = sum(p for _, p in w) / 4
            if best is None or avg < best[2]:
                best = (w[0][0], w[-1][0] + half, avg)
        if best:
            out["power.cheap"] = {"from": ms(best[0]), "to": ms(best[1]), "avg": round(best[2], 2)}

    # smart-meter usage: the newest full day, and the last week
    try:
        meter = mp["meters"][0]["serial_number"]
        since = midnight - datetime.timedelta(days=8)
        cons = paged(f"/v1/electricity-meter-points/{mp['mpan']}/meters/{meter}/consumption/?period_from={utc(since)}&page_size=500&order_by=period", key)
        days = {}
        for r in cons:
            s = datetime.datetime.fromisoformat(r["interval_start"].replace("Z", "+00:00")).astimezone(tz)
            days.setdefault(s.date().isoformat(), []).append((s, r["consumption"]))
        if days:
            day_ms = lambda d: ms(datetime.datetime.fromisoformat(d).replace(tzinfo=tz))
            out["power.week"] = [{"day": day_ms(d), "kwh": round(sum(k for _, k in v), 2)} for d, v in sorted(days.items())][-7:]
            full = [d for d, v in sorted(days.items()) if d != now.date().isoformat() and len(v) >= 44]
            day = full[-1] if full else sorted(days)[-1]
            v = sorted(days[day])
            kwh = sum(k for _, k in v)
            newest = max(r["interval_end"] for r in cons)
            out.update({"power.usage_day": day_ms(day), "power.usage_slots": [round(k, 3) for _, k in v], "power.usage_kwh": round(kwh, 2),
                        "power.usage_cost": round((kwh * (out["power.rate"] or 0) + sc) / 100, 2),
                        "power.usage_latest": ms(datetime.datetime.fromisoformat(newest.replace("Z", "+00:00")))})
        else:
            out.update({"power.usage_day": None, "power.usage_slots": [], "power.usage_kwh": 0, "power.usage_cost": 0, "power.week": [], "power.usage_latest": None})
    except Exception as e:
        log("consumption:", repr(e))
    put(out)


if __name__ == "__main__":
    forever(900, step, retry=120)
