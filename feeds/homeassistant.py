#!/usr/bin/env python3
"""Home Assistant -> the typed room.* keys (crates/ui/ts/schema.ts: K.room.*). Polls the REST API every few seconds.

config.json: "homeassistant": {"url": "http://homeassistant.local:8123", "temp": "sensor.living_room_temperature", "humidity": "sensor.living_room_humidity",
  "lux": "sensor.living_room_illuminance", "motion": "binary_sensor.living_room_motion", "lights": ["light.ceiling", ...], "doors": ["binary_sensor.front_door", ...]}
secrets/secrets.json: "homeassistant_token": a long-lived access token. Entity ids that are absent from the config simply give no value.
"""
import datetime
from urllib.request import Request, urlopen
import json
from _lib import config, secrets, put, forever, log


def instant(ts):
    """HA's ISO timestamp -> epoch ms."""
    return int(datetime.datetime.fromisoformat(ts.replace("Z", "+00:00")).timestamp() * 1000)


def number(state):
    try:
        return float(state["state"])
    except (KeyError, TypeError, ValueError):
        return None  # "unavailable", "unknown", or no such entity


def friendly(state, fallback):
    return (state.get("attributes") or {}).get("friendly_name") or fallback


def translate(states, cfg, now_ms):
    """states: entity id -> HA state object (or None). Returns the typed values to publish."""
    out = {}
    for key in ("temp", "humidity", "lux"):
        if cfg.get(key):
            out["room." + key] = number(states.get(cfg[key]) or {})
    if cfg.get("motion"):
        st = states.get(cfg["motion"]) or {}
        # while it is on, motion is happening now; otherwise the moment it last turned off is the last time it saw someone
        if st.get("state") == "on":
            out["room.motion"] = now_ms
        elif st.get("state") == "off" and st.get("last_changed"):
            out["room.motion"] = instant(st["last_changed"])
    out["room.lights"] = [{"name": friendly(states.get(e) or {}, e), "on": (states.get(e) or {}).get("state") == "on"} for e in cfg.get("lights", [])]
    out["room.doors"] = [{"name": friendly(states.get(e) or {}, e), "open": (states.get(e) or {}).get("state") == "on"} for e in cfg.get("doors", [])]
    return out


def fetch(base, token, entity):
    req = Request("%s/api/states/%s" % (base.rstrip("/"), entity), headers={"Authorization": "Bearer " + token})
    try:
        with urlopen(req, timeout=10) as r:
            return json.load(r)
    except Exception as e:
        log("homeassistant:", entity, repr(e))
        return None


def step():
    cfg = config().get("homeassistant")
    token = secrets().get("homeassistant_token")
    if not cfg or not token:
        raise RuntimeError("config.json needs a homeassistant section and secrets.json a homeassistant_token")
    entities = [cfg[k] for k in ("temp", "humidity", "lux", "motion") if cfg.get(k)] + cfg.get("lights", []) + cfg.get("doors", [])
    put(translate({e: fetch(cfg["url"], token, e) for e in entities}, cfg, int(datetime.datetime.now().timestamp() * 1000)))


if __name__ == "__main__":
    forever(5, step, retry=60)
