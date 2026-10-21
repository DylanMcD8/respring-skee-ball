//! Keep Chuzzle's fixed-step simulation at 60 Hz while rendering more often.
use std::time::{Duration, Instant};

#[derive(Default)]
pub struct LogicClock {
    next: Option<Instant>,
}

impl LogicClock {
    pub fn should_update(&mut self, now: Instant) -> bool {
        let step = Duration::from_secs_f64(1.0 / 60.0);
        let Some(due) = self.next else {
            self.next = Some(now + step);
            return true;
        };
        // Avoid dropping an update solely because the timer fired a few
        // microseconds early. This does not accumulate into faster simulation.
        if now + Duration::from_micros(100) < due {
            return false;
        }
        // Like the original NSTimer, discard missed ticks after a long stall.
        // Never run a burst of updates to catch up.
        self.next = Some(if now > due + step { now + step } else { due + step });
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rendering_rate_does_not_double_simulation_speed() {
        for fps in [60, 90, 120] {
            let start = Instant::now();
            let mut clock = LogicClock::default();
            let updates = (0..fps * 10).filter(|frame| {
                clock.should_update(start + Duration::from_secs_f64(*frame as f64 / fps as f64))
            }).count();
            assert_eq!(updates, 600, "render rate {fps}");
        }
    }

    #[test]
    fn long_stall_does_not_cause_catch_up_burst() {
        let start = Instant::now();
        let mut clock = LogicClock::default();
        assert!(clock.should_update(start));
        assert!(clock.should_update(start + Duration::from_secs(30)));
        assert!(!clock.should_update(start + Duration::from_secs(30)));
        assert!(!clock.should_update(start + Duration::from_secs_f64(30.008)));
        assert!(clock.should_update(start + Duration::from_secs_f64(30.017)));
    }

    #[test]
    fn timer_jitter_does_not_accumulate_extra_updates() {
        let start = Instant::now();
        let mut clock = LogicClock::default();
        let updates = (0..1200).filter(|frame| {
            let jitter = if frame % 2 == 0 { -0.00005 } else { 0.00005 };
            clock.should_update(start + Duration::from_secs_f64(0.001 + *frame as f64 / 120.0 + jitter))
        }).count();
        assert_eq!(updates, 600);
    }
}
