#!/usr/bin/env python3
"""Google Calendar -> the display, every 30 s (the service account in secrets/ must be shared on the calendar).
Writes K.cal.events (crates/ui/ts/schema.ts): [{id, title, start, end, allDay}], instants in epoch ms, from today through the next seven days.
The screens work out today, what is next, and what is over; writes go the other way through commands/cal.py (add / move / delete)."""
import datetime, zoneinfo
from _lib import config, put, forever
from _google import calendar_api


def ms(dt):
    return int(dt.timestamp() * 1000)


def parse(ev, tz):
    """One Google event as a K.cal.events entry: {id, title, start, end, allDay} with instants in epoch ms (end exclusive)."""
    s, e = ev["start"], ev["end"]
    if "date" in s:  # all-day: local midnight to local midnight
        a = datetime.datetime.fromisoformat(s["date"]).replace(tzinfo=tz)
        b = datetime.datetime.fromisoformat(e["date"]).replace(tzinfo=tz)
        return {"id": ev["id"], "title": ev.get("summary", "(no title)"), "start": ms(a), "end": ms(b), "allDay": True}
    return {"id": ev["id"], "title": ev.get("summary", "(no title)"), "start": ms(datetime.datetime.fromisoformat(s["dateTime"])), "end": ms(datetime.datetime.fromisoformat(e["dateTime"])), "allDay": False}


def step():
    tz = zoneinfo.ZoneInfo(config().get("tz", "Europe/London"))
    start = datetime.datetime.now(tz).replace(hour=0, minute=0, second=0, microsecond=0)
    items = calendar_api("GET", "/events", params={"singleEvents": "true", "orderBy": "startTime", "maxResults": 100,
                         "timeMin": start.isoformat(), "timeMax": (start + datetime.timedelta(days=8)).isoformat()}).get("items", [])
    put({"cal.events": [parse(e, tz) for e in items if e.get("status") != "cancelled"]})


if __name__ == "__main__":
    forever(30, step, retry=60)
