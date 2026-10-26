#!/usr/bin/env python3
"""Weather, sun and moon from Open-Meteo (free, no key), every 10 min, for the lat/lon in config.json.
Keys: wx.temp wx.feels wx.kind wx.text wx.wind wx.wind_dir wx.cloud (oktas) wx.pressure wx.hourly wx.pop wx.hours (HH of each hourly value) wx.rain_from
      wx.uv wx.uv_max wx.uv_band wx.uv_day (today, 24 hourly values from 00:00) wx.uv_peak (HH:00 of the maximum)
      sun.rise sun.set (local HH:MM) moon.phase (0 new .. 0.5 full) moon.name
      aq.eaqi aq.band aq.pm25 aq.pm10 aq.no2 aq.o3 aq.main (European AQI from Open-Meteo's air-quality API; 0-20 good .. over 100 extremely poor)"""
import datetime, math
from _lib import config, get_json, put, forever, log

# WMO weather code -> (ui weather symbol, text)
WMO = {0: ("clear", "Clear"), 1: ("clear", "Mostly clear"), 2: ("partly", "Partly cloudy"), 3: ("cloudy", "Overcast"),
       45: ("fog", "Fog"), 48: ("fog", "Freezing fog"), 51: ("rain", "Drizzle"), 53: ("rain", "Drizzle"), 55: ("rain", "Drizzle"),
       56: ("rain", "Freezing drizzle"), 57: ("rain", "Freezing drizzle"), 61: ("rain", "Light rain"), 63: ("rain", "Rain"), 65: ("rain", "Heavy rain"),
       66: ("rain", "Freezing rain"), 67: ("rain", "Freezing rain"), 71: ("snow", "Light snow"), 73: ("snow", "Snow"), 75: ("snow", "Heavy snow"),
       77: ("snow", "Snow grains"), 80: ("rain", "Showers"), 81: ("rain", "Showers"), 82: ("rain", "Heavy showers"), 85: ("snow", "Snow showers"),
       86: ("snow", "Snow showers"), 95: ("storm", "Thunderstorm"), 96: ("storm", "Thunderstorm, hail"), 99: ("storm", "Thunderstorm, hail")}


def moon_phase(now=None):
    """Fraction of the synodic month since the new moon of 2000-01-06 18:14 UTC (good to about a day)."""
    now = now or datetime.datetime.now(datetime.timezone.utc)
    ref = datetime.datetime(2000, 1, 6, 18, 14, tzinfo=datetime.timezone.utc)
    return ((now - ref).total_seconds() / 86400 / 29.530588853) % 1


def moon_name(p):
    names = ["New moon", "Waxing crescent", "First quarter", "Waxing gibbous", "Full moon", "Waning gibbous", "Last quarter", "Waning crescent"]
    return names[int(p * 8 + 0.5) % 8]


def uv_band(v):
    return "low" if v < 3 else "moderate" if v < 6 else "high" if v < 8 else "very high" if v < 11 else "extreme"


# European AQI: the index of each pollutant is banded; the overall value is the worst. Upper edges of the bands (ug/m3).
EAQI_BANDS = ["good", "fair", "moderate", "poor", "very poor", "extremely poor"]


def aq_band(v):
    return EAQI_BANDS[min(5, int(v // 20))] if v < 100 else EAQI_BANDS[5]


def air(lat, lon, tz):
    j = get_json("https://air-quality-api.open-meteo.com/v1/air-quality?latitude=%s&longitude=%s&timezone=%s&current=european_aqi,pm2_5,pm10,nitrogen_dioxide,ozone" % (lat, lon, tz.replace("/", "%2F")))["current"]
    # the dominant pollutant: the one furthest into its band, using the EAQI "fair" edges (pm2.5 20, pm10 40, no2 90, o3 100)
    rel = {"PM2.5": j["pm2_5"] / 20, "PM10": j["pm10"] / 40, "NO2": j["nitrogen_dioxide"] / 90, "O3": j["ozone"] / 100}
    put({"aq.eaqi": j["european_aqi"], "aq.band": aq_band(j["european_aqi"]), "aq.pm25": round(j["pm2_5"], 1), "aq.pm10": round(j["pm10"], 1),
         "aq.no2": round(j["nitrogen_dioxide"]), "aq.o3": round(j["ozone"]), "aq.main": max(rel, key=rel.get)})


def step():
    c = config()
    lat, lon, tz = c.get("lat"), c.get("lon"), c.get("tz", "auto")
    if lat is None:
        raise RuntimeError("config.json needs lat and lon")
    url = ("https://api.open-meteo.com/v1/forecast?latitude=%s&longitude=%s&timezone=%s&forecast_days=2&wind_speed_unit=mph"
           "&current=temperature_2m,apparent_temperature,weather_code,wind_speed_10m,wind_direction_10m,pressure_msl,cloud_cover"
           "&hourly=temperature_2m,precipitation_probability,uv_index&daily=sunrise,sunset,uv_index_max") % (lat, lon, tz.replace("/", "%2F"))
    j = get_json(url)
    cur = j["current"]
    kind, text = WMO.get(cur["weather_code"], ("cloudy", "Cloudy"))
    now = datetime.datetime.fromisoformat(cur["time"])
    times = [datetime.datetime.fromisoformat(t) for t in j["hourly"]["time"]]
    start = max(0, sum(1 for t in times if t <= now) - 1)
    temps = j["hourly"]["temperature_2m"][start:start + 24]
    pops = j["hourly"]["precipitation_probability"][start:start + 24]
    rain_from = next((times[start + i].strftime("%H:%M") for i, p in enumerate(pops) if p is not None and p >= 50 and i > 0), None)
    ph = moon_phase()
    uvs = j["hourly"]["uv_index"]
    uv = round(uvs[start], 1) if start < len(uvs) and uvs[start] is not None else 0
    put({
        "wx.uv": uv, "wx.uv_max": round(j["daily"]["uv_index_max"][0], 1), "wx.uv_band": uv_band(uv),
        "wx.uv_day": [round(v or 0, 1) for v in uvs[:24]], "wx.uv_peak": "%02d:00" % max(range(24), key=lambda i: uvs[i] or 0),
        "wx.hours": [times[start + i].strftime("%H") for i in range(len(temps))],
    })
    put({
        "wx.temp": round(cur["temperature_2m"], 1), "wx.feels": round(cur["apparent_temperature"], 1),
        "wx.kind": kind, "wx.text": text, "wx.wind": round(cur["wind_speed_10m"]), "wx.wind_dir": cur["wind_direction_10m"],
        "wx.cloud": round(cur["cloud_cover"] / 12.5), "wx.pressure": round(cur["pressure_msl"]),
        "wx.hourly": temps, "wx.pop": pops, "wx.rain_from": rain_from,
        "sun.rise": j["daily"]["sunrise"][0][-5:], "sun.set": j["daily"]["sunset"][0][-5:],
        "moon.phase": round(ph, 3), "moon.name": moon_name(ph),
    })
    try:
        air(lat, lon, tz)
    except Exception as e:                  # the weather above is already out; air quality is a separate service
        log("air quality:", repr(e))


if __name__ == "__main__":
    forever(600, step, retry=60)
