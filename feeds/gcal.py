#!/usr/bin/env python3
"""Google Calendar -> the display, every 30 s (the service account in secrets/ must be shared on the calendar).
Keys: cal.today ([{id, t, title, dur, past, all_day}]) cal.week (next 7 days: [{id, date, t, title, dur, all_day}]) cal.next cal.left
Writes go the other way through commands/cal.py (add / move / delete)."""
import datetime, zoneinfo
from _lib import config, put, forever
from _google import calendar_api


def parse(ev, tz):
    s, e = ev["start"], ev["end"]
    if "date" in s:  # all-day
        return {"id": ev["id"], "date": s["date"], "t": "all day", "title": ev.get("summary", "(no title)"), "dur": 0, "all_day": True}
    a = datetime.datetime.fromisoformat(s["dateTime"]).astimezone(tz)
    b = datetime.datetime.fromisoformat(e["dateTime"]).astimezone(tz)
    return {"id": ev["id"], "date": a.date().isoformat(), "t": a.strftime("%H:%M"), "title": ev.get("summary", "(no title)"), "dur": int((b - a).total_seconds() // 60), "all_day": False}


def step():
    tz = zoneinfo.ZoneInfo(config().get("tz", "Europe/London"))
    now = datetime.datetime.now(tz)
    start = now.replace(hour=0, minute=0, second=0, microsecond=0)
    items = calendar_api("GET", "/events", params={"singleEvents": "true", "orderBy": "startTime", "maxResults": 100,
                         "timeMin": start.isoformat(), "timeMax": (start + datetime.timedelta(days=8)).isoformat()}).get("items", [])
    evs = [parse(e, tz) for e in items if e.get("status") != "cancelled"]
    # an all-day event listed for several days appears once per day; Google's end date is exclusive
    week = [e for e in evs if e["date"] <= (start + datetime.timedelta(days=7)).date().isoformat()]
    today_iso = now.date().isoformat()
    mins = now.hour * 60 + now.minute
    today = []
    for e in week:
        if e["date"] != today_iso:
            continue
        m = None if e["all_day"] else int(e["t"][:2]) * 60 + int(e["t"][3:])
        today.append({**e, "past": (m is not None and m + e["dur"] <= mins)})
    nxt = None
    for e in week:
        if e["all_day"]:
            continue
        d = datetime.date.fromisoformat(e["date"])
        m = (d - now.date()).days * 1440 + int(e["t"][:2]) * 60 + int(e["t"][3:]) - mins
        if m >= 0:
            nxt = {"id": e["id"], "t": e["t"], "date": e["date"], "title": e["title"], "in_min": m}
            break
    put({"cal.today": today, "cal.week": week, "cal.next": nxt, "cal.left": sum(1 for e in today if not e["past"] and not e["all_day"])})


if __name__ == "__main__":
    forever(30, step, retry=60)
