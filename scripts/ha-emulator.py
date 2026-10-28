#!/usr/bin/env python3
"""A Home Assistant stand-in for developing without a Home Assistant: the REST API the feed uses (GET /api/states/<entity>, a bearer token, HA's state objects:
entity_id, state, attributes.friendly_name, last_changed, last_updated), a 401 without the token and a 404 for an entity that does not exist, plus a control
endpoint to act as the house:  POST /_emulate {"binary_sensor.living_room_motion": "on"}  sets a state;  GET /_emulate shows them all.

    ha-emulator.py [port] [token]
"""
import datetime, json, sys
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

PORT = int(sys.argv[1]) if len(sys.argv) > 1 else 8123
TOKEN = sys.argv[2] if len(sys.argv) > 2 else "emu-token"


def now(ago_s=0):
    return (datetime.datetime.now(datetime.timezone.utc) - datetime.timedelta(seconds=ago_s)).isoformat()


STATES = {}


def put(entity, state, name, unit=None, ago_s=3600):
    """A state as Home Assistant keeps it; it has been so for `ago_s` seconds (an hour: a house that was quiet before the demo starts)."""
    attrs = {"friendly_name": name}
    if unit:
        attrs["unit_of_measurement"] = unit
    STATES[entity] = {"entity_id": entity, "state": state, "attributes": attrs, "last_changed": now(ago_s), "last_updated": now(ago_s), "context": {"id": "emu", "parent_id": None, "user_id": None}}


put("sensor.living_room_temperature", "21.4", "Living room temperature", "°C")
put("sensor.living_room_humidity", "45", "Living room humidity", "%")
put("sensor.living_room_illuminance", "120", "Living room illuminance", "lx")
put("binary_sensor.living_room_motion", "off", "Living room motion")
put("light.ceiling", "on", "Ceiling")
put("light.lamp", "off", "Reading lamp")
put("binary_sensor.front_door", "off", "Front door")
put("binary_sensor.back_door", "off", "Back door")


class Handler(BaseHTTPRequestHandler):
    def log_message(self, *a):
        pass

    def reply(self, code, body):
        data = json.dumps(body).encode()
        self.send_response(code)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(data)))
        self.end_headers()
        self.wfile.write(data)

    def do_GET(self):
        if self.path == "/_emulate":
            return self.reply(200, {k: v["state"] for k, v in STATES.items()})
        if self.headers.get("Authorization") != "Bearer " + TOKEN:
            return self.reply(401, {"message": "401: Unauthorized"})
        if self.path == "/api/":
            return self.reply(200, {"message": "API running."})
        if self.path.startswith("/api/states/"):
            st = STATES.get(self.path[len("/api/states/"):])
            return self.reply(200, st) if st else self.reply(404, {"message": "Entity not found."})
        self.reply(404, {"message": "404: Not Found"})

    def do_POST(self):
        if self.path != "/_emulate":
            return self.reply(404, {"message": "404: Not Found"})
        body = json.loads(self.rfile.read(int(self.headers.get("Content-Length", 0))) or b"{}")
        for entity, state in body.items():
            old = STATES.get(entity)
            if old is None:
                put(entity, state, entity, ago_s=0)
            elif old["state"] != state:
                old["state"] = state
                old["last_changed"] = old["last_updated"] = now()
        self.reply(200, {k: v["state"] for k, v in STATES.items()})


if __name__ == "__main__":
    ThreadingHTTPServer(("127.0.0.1", PORT), Handler).serve_forever()
