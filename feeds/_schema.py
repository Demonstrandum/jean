"""Generated from crates/ui/ts/schema.ts: do not edit. The enums the feeds write (integers on the wire)."""
from enum import IntEnum


class Sky(IntEnum):
    """The sky, as the weather symbols draw it."""
    Clear = 0
    Partly = 1
    Cloudy = 2
    Rain = 3
    Snow = 4
    Storm = 5
    Fog = 6


class UvLevel(IntEnum):
    """UV index bands."""
    Low = 0
    Moderate = 1
    High = 2
    VeryHigh = 3
    Extreme = 4


class AirBand(IntEnum):
    """European air-quality bands (from the European AQI)."""
    Good = 0
    Fair = 1
    Moderate = 2
    Poor = 3
    VeryPoor = 4
    ExtremelyPoor = 5


class CarbonIndex(IntEnum):
    """The grid's carbon-intensity index."""
    VeryLow = 0
    Low = 1
    Moderate = 2
    High = 3
    VeryHigh = 4


class Pollutant(IntEnum):
    """The pollutants the air-quality service reports."""
    Pm25 = 0
    Pm10 = 1
    No2 = 2
    O3 = 3


class Fuel(IntEnum):
    """Generation fuels in the grid mix."""
    Biomass = 0
    Coal = 1
    Imports = 2
    Gas = 3
    Nuclear = 4
    Other = 5
    Hydro = 6
    Solar = 7
    Wind = 8


class Presence(IntEnum):
    """What the assistant is doing."""
    Idle = 0
    Listening = 1
    Thinking = 2
    Speaking = 3


class RateKind(IntEnum):
    """The tariff's shape: one rate all day, or a rate per half hour."""
    Flat = 0
    HalfHourly = 1


class MoonPhase(IntEnum):
    """The eight named phases of the moon."""
    New = 0
    WaxingCrescent = 1
    FirstQuarter = 2
    WaxingGibbous = 3
    Full = 4
    WaningGibbous = 5
    LastQuarter = 6
    WaningCrescent = 7
