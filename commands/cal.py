#!/usr/bin/env python3
"""Calendar upload/download from the display: python3 commands/cal.py <verb> ...  (or ui.run("cal", verb, ...))
  list [days=7]                           -> one line per event: id, date, time, duration, title
  add <title> <YYYY-MM-DD> <HH:MM> [min]  -> create (min defaults to 60); time "all" makes an all-day event
  move <id> <YYYY-MM-DD> <HH:MM>          -> reschedule keeping the duration
  delete <id>
The calendar feed picks the change up within 30 s."""
import datetime, sys, zoneinfo
sys.path.insert(0, __import__("os").path.join(__import__("os").path.dirname(__import__("os").path.abspath(__file__)), "..", "feeds"))
from _lib import config
from _google import calendar_api

tz = zoneinfo.ZoneInfo(config().get("tz", "Europe/London"))


def iso(date, hm):
    return datetime.datetime.fromisoformat(f"{date}T{hm}").replace(tzinfo=tz)


def main(a):
    if not a:
        return print(__doc__)
    v = a[0]
    if v == "list":
        days = int(a[1]) if len(a) > 1 else 7
        now = datetime.datetime.now(tz)
        items = calendar_api("GET", "/events", params={"singleEvents": "true", "orderBy": "startTime", "timeMin": now.replace(hour=0, minute=0).isoformat(), "timeMax": (now + datetime.timedelta(days=days)).isoformat()}).get("items", [])
        for e in items:
            s = e["start"].get("dateTime") or e["start"]["date"]
            print(e["id"], s[:10], s[11:16] or "all", e.get("summary", ""))
    elif v == "add":
        title, date, hm = a[1], a[2], a[3]
        if hm == "all":
            end = (datetime.date.fromisoformat(date) + datetime.timedelta(days=1)).isoformat()
            body = {"summary": title, "start": {"date": date}, "end": {"date": end}}
        else:
            s = iso(date, hm)
            body = {"summary": title, "start": {"dateTime": s.isoformat()}, "end": {"dateTime": (s + datetime.timedelta(minutes=int(a[4]) if len(a) > 4 else 60)).isoformat()}}
        e = calendar_api("POST", "/events", body)
        print("created", e["id"], e["start"].get("dateTime", e["start"].get("date")))
    elif v == "move":
        eid, date, hm = a[1], a[2], a[3]
        e = calendar_api("GET", f"/events/{eid}")
        d = datetime.datetime.fromisoformat(e["end"]["dateTime"]) - datetime.datetime.fromisoformat(e["start"]["dateTime"])
        s = iso(date, hm)
        calendar_api("PATCH", f"/events/{eid}", {"start": {"dateTime": s.isoformat()}, "end": {"dateTime": (s + d).isoformat()}})
        print("moved", eid)
    elif v == "delete":
        calendar_api("DELETE", f"/events/{a[1]}")
        print("deleted", a[1])
    else:
        sys.exit("unknown verb " + v)


if __name__ == "__main__":
    try:
        main(sys.argv[1:])
    except Exception as e:
        sys.exit(str(e))
