//! Things the house does unprompted: typed events in, actions out. The clock is passed in, so every rule is testable.

use chrono::{Datelike, NaiveDate, TimeZone, Timelike, Utc};
use chrono_tz::Tz;

/// What the weather feed says that matters to a rule.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Outlook {
    /// When rain is next expected (epoch ms), if within the forecast.
    pub rain_from: Option<u64>,
    pub temp: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Event {
    /// The motion sensor saw someone.
    Motion,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Action {
    Say(String),
}

/// How far ahead rain counts for the jacket nudge, and the temperature below which a jacket is suggested anyway.
pub const RAIN_WINDOW_MS: u64 = 6 * 3_600_000;
pub const COLD_BELOW: f64 = 8.0;

/// How long the room has to be still before the next motion counts as someone arriving: while the sensor stays on, its "last seen" instant keeps moving
/// forward with every poll, and that is one event, not one per poll.
pub const MOTION_QUIET_MS: u64 = 60_000;

/// Turns the motion sensor's "last seen" instant into events: it fires when that instant moves forward after the room had been still, and not for what
/// was already there at start.
#[derive(Default)]
pub struct MotionWatch {
    seen: Option<Option<u64>>,
}

impl MotionWatch {
    pub fn observe(&mut self, last: Option<u64>) -> bool {
        let fired = match (self.seen, last) {
            (Some(None), Some(_)) => true, // the first sighting ever
            (Some(Some(prev)), Some(now)) => now.saturating_sub(prev) >= MOTION_QUIET_MS,
            _ => false,
        };
        self.seen = Some(last);
        fired
    }
}

pub struct Triggers {
    pub tz: Tz,
    greeted: Option<NaiveDate>,
}

impl Triggers {
    pub fn new(tz: Tz) -> Triggers {
        Triggers { tz, greeted: None }
    }

    /// The first motion of a morning (05:00 to 11:00 local) greets once, and says whether to take a jacket.
    pub fn on(&mut self, ev: Event, outlook: Option<Outlook>, now_ms: u64) -> Vec<Action> {
        let t = Utc.timestamp_millis_opt(now_ms as i64).unwrap().with_timezone(&self.tz);
        match ev {
            Event::Motion => {
                let day = NaiveDate::from_ymd_opt(t.year(), t.month(), t.day()).unwrap();
                if self.greeted == Some(day) || !(5..11).contains(&t.hour()) {
                    return vec![];
                }
                self.greeted = Some(day);
                let mut say = String::from("Good morning.");
                if let Some(o) = outlook {
                    if o.rain_from.is_some_and(|r| r >= now_ms && r - now_ms <= RAIN_WINDOW_MS) {
                        say.push_str(" It looks like rain today, so take a jacket.");
                    } else if o.temp < COLD_BELOW {
                        say.push_str(" It is cold out, so take a jacket.");
                    }
                }
                vec![Action::Say(say)]
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // 2026-10-27 is a Tuesday; London is on GMT, so local equals UTC.
    const MIDNIGHT: u64 = 1_793_059_200_000;
    const H: u64 = 3_600_000;

    fn tr() -> Triggers {
        Triggers::new(chrono_tz::Europe::London)
    }

    #[test]
    fn motion_fires_when_the_last_seen_instant_moves_on_and_not_at_start() {
        let mut w = MotionWatch::default();
        let t = 1_793_000_000_000u64;
        assert!(!w.observe(Some(t)), "what was there at start is not news");
        assert!(!w.observe(Some(t)));
        assert!(w.observe(Some(t + 200_000)), "someone arrives after the room had been still");
        assert!(!w.observe(Some(t + 205_000)), "the sensor staying on keeps moving the instant forward: still one event");
        assert!(!w.observe(Some(t + 210_000)));
        assert!(!w.observe(Some(t + 210_000)));
        assert!(w.observe(Some(t + 400_000)), "the next arrival is a new event");
        let mut w = MotionWatch::default();
        assert!(!w.observe(None));
        assert!(w.observe(Some(5)), "the first ever sighting is news");
    }

    #[test]
    fn the_first_morning_motion_greets_once() {
        let mut t = tr();
        assert_eq!(t.on(Event::Motion, None, MIDNIGHT + 7 * H), vec![Action::Say("Good morning.".into())]);
        assert!(t.on(Event::Motion, None, MIDNIGHT + 8 * H).is_empty());
        // the next day it greets again
        assert_eq!(t.on(Event::Motion, None, MIDNIGHT + 31 * H).len(), 1);
    }

    #[test]
    fn motion_at_night_or_afternoon_is_ignored_and_does_not_use_up_the_greeting() {
        let mut t = tr();
        assert!(t.on(Event::Motion, None, MIDNIGHT + 2 * H).is_empty());
        assert!(t.on(Event::Motion, None, MIDNIGHT + 15 * H).is_empty());
        assert_eq!(t.on(Event::Motion, None, MIDNIGHT + 6 * H).len(), 1);
    }

    #[test]
    fn rain_soon_or_cold_adds_the_jacket_nudge() {
        let now = MIDNIGHT + 7 * H;
        let mut t = tr();
        let rainy = Outlook { rain_from: Some(now + 2 * H), temp: 14.0 };
        let Action::Say(s) = &t.on(Event::Motion, Some(rainy), now)[0];
        assert!(s.contains("rain") && s.contains("jacket"));
        let mut t = tr();
        let later = Outlook { rain_from: Some(now + 9 * H), temp: 14.0 };
        let Action::Say(s) = &t.on(Event::Motion, Some(later), now)[0];
        assert_eq!(s, "Good morning.");
        let mut t = tr();
        let cold = Outlook { rain_from: None, temp: 3.0 };
        let Action::Say(s) = &t.on(Event::Motion, Some(cold), now)[0];
        assert!(s.contains("cold") && s.contains("jacket"));
    }
}
