#!/usr/bin/env python3
"""The shopping list from data/groceries.json (Claude edits it on request); the calendar is feeds/calendar.py.
Keys: shop.items ([{text, qty, done}]) shop.open (count not done)"""
import datetime, json, os, time, zoneinfo
from _lib import ROOT, config, put, log

DAYS = ["mon", "tue", "wed", "thu", "fri", "sat", "sun"]


def load(name):
    try:
        with open(os.path.join(ROOT, "data", name)) as f:
            return json.load(f)
    except FileNotFoundError:
        return {}
    except Exception as e:
        log(name, "unreadable:", e)
        return None


def occurs(ev, day):
    if "date" in ev:
        return ev["date"] == day.isoformat()
    rep = ev.get("repeat")
    wd = DAYS[day.weekday()]
    if rep == "daily":
        return True
    if rep == "weekdays":
        return wd not in ("sat", "sun")
    if isinstance(rep, list):
        return wd in rep
    return False


def calendar(now):
    cal = load("calendar.json")
    if cal is None:
        return {}
    today = sorted(({"t": e["start"], "title": e["title"], "dur": e.get("dur", 0)} for e in cal.get("events", []) if occurs(e, now.date())), key=lambda e: e["t"])
    mins = now.hour * 60 + now.minute
    for e in today:
        h, m = map(int, e["t"].split(":"))
        e["past"] = h * 60 + m + e["dur"] <= mins
    nxt = next(({"t": e["t"], "title": e["title"], "in_min": int(e["t"][:2]) * 60 + int(e["t"][3:]) - mins} for e in today if int(e["t"][:2]) * 60 + int(e["t"][3:]) >= mins), None)
    return {"cal.today": today, "cal.next": nxt, "cal.left": sum(1 for e in today if not e["past"])}


def shopping():
    s = load("groceries.json")
    if s is None:
        return {}
    items = [{"text": i["text"], "qty": i.get("qty", ""), "done": bool(i.get("done"))} for i in s.get("items", [])]
    return {"shop.items": items, "shop.open": sum(1 for i in items if not i["done"])}


if __name__ == "__main__":
    tz = zoneinfo.ZoneInfo(config().get("tz", "Europe/London"))
    last = None
    while True:
        # the minute matters (what is next, what is past), and the files may change at any moment
        key = os.path.getmtime(os.path.join(ROOT, "data", "groceries.json")) if os.path.exists(os.path.join(ROOT, "data", "groceries.json")) else 0
        if key != last:
            last = key
            put(shopping())
        time.sleep(3)
