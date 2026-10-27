"""python3 feeds/test_homeassistant.py : the translation from Home Assistant states to the typed keys."""
import unittest
from homeassistant import translate, instant

CFG = {"temp": "sensor.t", "humidity": "sensor.h", "lux": "sensor.l", "motion": "binary_sensor.m", "lights": ["light.a", "light.b"], "doors": ["binary_sensor.d"]}


class T(unittest.TestCase):
    def test_numbers_lights_and_doors(self):
        st = {
            "sensor.t": {"state": "21.4"}, "sensor.h": {"state": "unavailable"}, "sensor.l": None,
            "binary_sensor.m": {"state": "off", "last_changed": "2026-10-27T07:00:00+00:00"},
            "light.a": {"state": "on", "attributes": {"friendly_name": "Ceiling"}}, "light.b": {"state": "off"},
            "binary_sensor.d": {"state": "on", "attributes": {"friendly_name": "Balcony"}},
        }
        out = translate(st, CFG, 1_793_000_000_000)
        self.assertEqual(out["room.temp"], 21.4)
        self.assertIsNone(out["room.humidity"])
        self.assertIsNone(out["room.lux"])
        self.assertEqual(out["room.motion"], instant("2026-10-27T07:00:00+00:00"))
        self.assertEqual(out["room.lights"], [{"name": "Ceiling", "on": True}, {"name": "light.b", "on": False}])
        self.assertEqual(out["room.doors"], [{"name": "Balcony", "open": True}])

    def test_motion_that_is_happening_is_now(self):
        out = translate({"binary_sensor.m": {"state": "on", "last_changed": "2026-10-27T07:00:00Z"}}, CFG, 1_793_000_000_000)
        self.assertEqual(out["room.motion"], 1_793_000_000_000)

    def test_a_missing_sensor_publishes_nothing_for_motion(self):
        self.assertNotIn("room.motion", translate({}, CFG, 5))


if __name__ == "__main__":
    unittest.main()
