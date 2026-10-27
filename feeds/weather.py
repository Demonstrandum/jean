#!/usr/bin/env python3
"""Weather, sun, moon and air quality from Open-Meteo (free, no key), every 10 min, for the lat/lon in config.json.
Writes the typed keys in crates/ui/ts/schema.ts (K.wx.*, K.sun.*, K.moon.*, K.air.*): instants are epoch ms, categories are the
enums in _schema.py (generated from that schema), amounts are numbers. No text is produced for the display to parse."""
import datetime, math, zoneinfo
from _lib import config, get_json, put, forever, log
from _schema import Sky, Pollutant

# WMO weather code -> Sky
WMO = {0: Sky.Clear, 1: Sky.Clear, 2: Sky.Partly, 3: Sky.Cloudy, 45: Sky.Fog, 48: Sky.Fog,
       51: Sky.Rain, 53: Sky.Rain, 55: Sky.Rain, 56: Sky.Rain, 57: Sky.Rain, 61: Sky.Rain, 63: Sky.Rain, 65: Sky.Rain, 66: Sky.Rain, 67: Sky.Rain,
       71: Sky.Snow, 73: Sky.Snow, 75: Sky.Snow, 77: Sky.Snow, 80: Sky.Rain, 81: Sky.Rain, 82: Sky.Rain, 85: Sky.Snow, 86: Sky.Snow,
       95: Sky.Storm, 96: Sky.Storm, 99: Sky.Storm}


def moon_phase(now=None):
    """Fraction of the synodic month since the new moon of 2000-01-06 18:14 UTC (good to about a day)."""
    now = now or datetime.datetime.now(datetime.timezone.utc)
    ref = datetime.datetime(2000, 1, 6, 18, 14, tzinfo=datetime.timezone.utc)
    return ((now - ref).total_seconds() / 86400 / 29.530588853) % 1


def sun_times(lat, lon, day):
    """Sunrise and sunset (aware UTC datetimes) from the NOAA solar equations, good to about a minute: the day dial must not depend on the network."""
    n = day.timetuple().tm_yday
    g = 2 * math.pi / 365 * (n - 1)
    eqtime = 229.18 * (0.000075 + 0.001868 * math.cos(g) - 0.032077 * math.sin(g) - 0.014615 * math.cos(2 * g) - 0.040849 * math.sin(2 * g))
    decl = 0.006918 - 0.399912 * math.cos(g) + 0.070257 * math.sin(g) - 0.006758 * math.cos(2 * g) + 0.000907 * math.sin(2 * g) - 0.002697 * math.cos(3 * g) + 0.00148 * math.sin(3 * g)
    cos_ha = math.cos(math.radians(90.833)) / (math.cos(math.radians(lat)) * math.cos(decl)) - math.tan(math.radians(lat)) * math.tan(decl)
    ha = math.degrees(math.acos(max(-1.0, min(1.0, cos_ha))))      # the polar day and night clamp to a day or a night
    base = datetime.datetime(day.year, day.month, day.day, tzinfo=datetime.timezone.utc)
    return base + datetime.timedelta(minutes=720 - 4 * (lon + ha) - eqtime), base + datetime.timedelta(minutes=720 - 4 * (lon - ha) - eqtime)


def ms(dt):
    return int(dt.timestamp() * 1000)


def air(lat, lon, tz):
    j = get_json("https://air-quality-api.open-meteo.com/v1/air-quality?latitude=%s&longitude=%s&timezone=%s&current=european_aqi,pm2_5,pm10,nitrogen_dioxide,ozone" % (lat, lon, tz.replace("/", "%2F")))["current"]
    # the pollutant furthest into its band, against the EAQI "fair" edges (ug/m3): pm2.5 20, pm10 40, no2 90, o3 100
    rel = {Pollutant.Pm25: j["pm2_5"] / 20, Pollutant.Pm10: j["pm10"] / 40, Pollutant.No2: j["nitrogen_dioxide"] / 90, Pollutant.O3: j["ozone"] / 100}
    put({"aq.eaqi": j["european_aqi"], "aq.pm25": round(j["pm2_5"], 1), "aq.pm10": round(j["pm10"], 1), "aq.no2": round(j["nitrogen_dioxide"]),
         "aq.o3": round(j["ozone"]), "aq.main": max(rel, key=rel.get)})


def step():
    c = config()
    lat, lon, tzname = c.get("lat"), c.get("lon"), c.get("tz", "Europe/London")
    if lat is None:
        raise RuntimeError("config.json needs lat and lon")
    tz = zoneinfo.ZoneInfo(tzname)
    rise, sets = sun_times(lat, lon, datetime.datetime.now(tz).date())
    put({"sun.rise": ms(rise), "sun.set": ms(sets)})              # the dial's day and night, whatever the API does next
    url = ("https://api.open-meteo.com/v1/forecast?latitude=%s&longitude=%s&timezone=%s&forecast_days=2&wind_speed_unit=mph"
           "&current=temperature_2m,apparent_temperature,weather_code,wind_speed_10m,wind_direction_10m,pressure_msl,cloud_cover"
           "&hourly=temperature_2m,precipitation_probability,uv_index&daily=sunrise,sunset,uv_index_max") % (lat, lon, tzname.replace("/", "%2F"))
    j = get_json(url)
    cur = j["current"]
    at = lambda iso: datetime.datetime.fromisoformat(iso).replace(tzinfo=tz)      # the API sends local times without an offset
    now = at(cur["time"])
    times = [at(t) for t in j["hourly"]["time"]]
    start = max(0, sum(1 for t in times if t <= now) - 1)
    hours = []
    for i in range(start, min(start + 24, len(times))):
        h = j["hourly"]
        hours.append({"at": ms(times[i]), "temp": h["temperature_2m"][i], "rain": h["precipitation_probability"][i] or 0, "uv": h["uv_index"][i] or 0})
    rain_from = next((h["at"] for i, h in enumerate(hours) if i > 0 and h["rain"] >= 50), None)
    uvs = j["hourly"]["uv_index"]
    ph = moon_phase()
    put({
        "wx.temp": round(cur["temperature_2m"], 1), "wx.feels": round(cur["apparent_temperature"], 1), "wx.sky": WMO.get(cur["weather_code"], Sky.Cloudy),
        "wx.wind": round(cur["wind_speed_10m"]), "wx.wind_dir": cur["wind_direction_10m"], "wx.cloud": round(cur["cloud_cover"] / 12.5), "wx.pressure": round(cur["pressure_msl"]),
        "wx.hours": hours, "wx.rain_from": rain_from,
        "wx.uv_day": [round(v or 0, 1) for v in uvs[:24]], "wx.uv": round(uvs[start] or 0, 1) if start < len(uvs) else 0, "wx.uv_max": round(j["daily"]["uv_index_max"][0], 1),
        "sun.rise": ms(at(j["daily"]["sunrise"][0])), "sun.set": ms(at(j["daily"]["sunset"][0])),
        "moon.phase": round(ph, 3),
    })
    try:
        air(lat, lon, tzname)
    except Exception as e:                  # the weather above is already out; air quality is a separate service
        log("air quality:", repr(e))


if __name__ == "__main__":
    forever(600, step, retry=60)
