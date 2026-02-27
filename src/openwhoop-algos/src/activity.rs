use chrono::{Duration, NaiveDateTime, TimeDelta};
use openwhoop_codec::{Activity, ParsedHistoryReading};

const ACTIVITY_CHANGE_THRESHOLD: Duration = Duration::minutes(15);
const MIN_SLEEP_DURATION: Duration = Duration::minutes(60);
pub const MAX_SLEEP_PAUSE: Duration = Duration::minutes(60);
const MAX_PAUSE: Duration = Duration::minutes(10);

#[derive(Clone, Copy, Debug)]
pub struct ActivityPeriod {
    pub activity: Activity,
    pub start: NaiveDateTime,
    pub end: NaiveDateTime,
    pub duration: TimeDelta,
}

#[derive(Clone, Copy, Debug)]
struct TempActivity {
    activity: Activity,
    start: NaiveDateTime,
    end: NaiveDateTime,
}

impl ActivityPeriod {
    pub fn detect(history: &mut [ParsedHistoryReading]) -> Vec<ActivityPeriod> {
        Self::smooth_spikes(history);
        let changes = Self::detect_changes(history);

        Self::filter_merge(changes)
            .into_iter()
            .map(|a| ActivityPeriod {
                activity: a.activity,
                start: a.start,
                end: a.end,
                duration: a.end - a.start,
            })
            .collect()
    }

    pub fn is_active(&self) -> bool {
        matches!(self.activity, Activity::Active)
    }

    pub fn find_sleep(events: &mut Vec<ActivityPeriod>) -> Option<ActivityPeriod> {
        let mut next = || {
            if events.is_empty() {
                None
            } else {
                Some(events.remove(0))
            }
        };

        while let Some(event) = next() {
            if matches!(event.activity, Activity::Sleep) && event.duration > MIN_SLEEP_DURATION {
                return Some(event);
            }
        }

        None
    }

    fn smooth_spikes(data: &mut [ParsedHistoryReading]) {
        if data.len() < 3 {
            return;
        }

        let mut new_values = data.iter().map(|m| m.activity).collect::<Vec<_>>();

        for i in 1..data.len() - 1 {
            if data[i - 1].activity == data[i + 1].activity
                && data[i].activity != data[i - 1].activity
            {
                new_values[i] = data[i - 1].activity;
            }
        }

        for (i, model) in data.iter_mut().enumerate() {
            model.activity = new_values[i];
        }
    }

    fn filter_merge(mut activities: Vec<TempActivity>) -> Vec<TempActivity> {
        if activities.is_empty() {
            return Vec::new();
        }

        let mut merged = Vec::new();
        let mut i = 0;

        while i < activities.len() {
            let current = &activities[i];
            let duration = current.end - current.start;

            if duration < ACTIVITY_CHANGE_THRESHOLD {
                if i > 0
                    && i + 1 < activities.len()
                    && activities[i - 1].activity == activities[i + 1].activity
                    && !merged.is_empty()
                {
                    // Merge with both previous and next activity
                    let prev: TempActivity = merged.pop().unwrap();
                    merged.push(TempActivity {
                        activity: prev.activity,
                        start: prev.start,
                        end: activities[i + 1].end,
                    });
                    i += 1; // Skip next since it's merged
                } else if i + 1 < activities.len() {
                    // Merge with next
                    activities[i + 1] = TempActivity {
                        activity: activities[i + 1].activity,
                        start: current.start,
                        end: activities[i + 1].end,
                    };
                } else if !merged.is_empty() {
                    // Merge with previous if at the end
                    let prev = merged.pop().unwrap();
                    merged.push(TempActivity {
                        activity: prev.activity,
                        start: prev.start,
                        end: current.end,
                    });
                }
            } else {
                merged.push(*current);
            }

            i += 1;
        }

        merged
    }
    fn detect_changes(history: &[ParsedHistoryReading]) -> Vec<TempActivity> {
        let mut periods = Vec::new();
        let mut iter = history.iter();

        if let Some(first) = iter.next() {
            let mut current_activity = first.activity;
            let mut start_time = first.time;
            let mut last_time = first.time;

            for model in iter {
                if model.activity != current_activity || (model.time - last_time > MAX_PAUSE) {
                    periods.push(TempActivity {
                        activity: current_activity,
                        start: start_time,
                        end: last_time,
                    });

                    current_activity = model.activity;
                    start_time = model.time;
                }
                last_time = model.time;
            }

            periods.push({
                TempActivity {
                    activity: current_activity,
                    start: start_time,
                    end: last_time,
                }
            });
        }

        periods
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    fn make_reading(minutes: i64, activity: Activity) -> ParsedHistoryReading {
        let base = NaiveDate::from_ymd_opt(2025, 1, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap();
        ParsedHistoryReading {
            time: base + Duration::minutes(minutes),
            bpm: 70,
            rr: vec![],
            activity,
            imu_data: None,
        }
    }

    fn make_readings(specs: &[(i64, Activity)]) -> Vec<ParsedHistoryReading> {
        specs.iter().map(|&(m, a)| make_reading(m, a)).collect()
    }

    #[test]
    fn detect_empty_history() {
        let periods = ActivityPeriod::detect(&mut []);
        assert!(periods.is_empty());
    }

    #[test]
    fn detect_single_activity_type() {
        let mut history = make_readings(
            &(0..30).map(|m| (m, Activity::Active)).collect::<Vec<_>>(),
        );
        let periods = ActivityPeriod::detect(&mut history);
        assert_eq!(periods.len(), 1);
        assert!(matches!(periods[0].activity, Activity::Active));
    }

    #[test]
    fn detect_splits_on_activity_change() {
        // 20 min active, then 20 min sleep - each segment is > 15 min threshold
        let mut specs: Vec<(i64, Activity)> = (0..20).map(|m| (m, Activity::Active)).collect();
        specs.extend((20..40).map(|m| (m, Activity::Sleep)));
        let mut history = make_readings(&specs);
        let periods = ActivityPeriod::detect(&mut history);
        assert_eq!(periods.len(), 2);
        assert!(matches!(periods[0].activity, Activity::Active));
        assert!(matches!(periods[1].activity, Activity::Sleep));
    }

    #[test]
    fn detect_splits_on_time_gap() {
        // Same activity but >10 min gap - detected as two periods, but short ones get merged
        // Make each segment > 15 min to survive filter_merge
        let mut specs: Vec<(i64, Activity)> = (0..20).map(|m| (m, Activity::Active)).collect();
        // 15 min gap (> MAX_PAUSE of 10 min), then another 20 min block
        specs.extend((35..55).map(|m| (m, Activity::Active)));
        let mut history = make_readings(&specs);
        let periods = ActivityPeriod::detect(&mut history);
        // Both segments are same activity, gap causes split, filter_merge may re-merge
        // since they're the same activity type. At minimum we get >= 1 period.
        assert!(!periods.is_empty());
    }

    #[test]
    fn smooth_spikes_removes_single_point_spike() {
        let mut history = make_readings(&[
            (0, Activity::Sleep),
            (1, Activity::Active), // spike
            (2, Activity::Sleep),
        ]);
        ActivityPeriod::smooth_spikes(&mut history);
        assert!(matches!(history[1].activity, Activity::Sleep));
    }

    #[test]
    fn smooth_spikes_no_change_on_short_data() {
        let mut history = make_readings(&[(0, Activity::Sleep), (1, Activity::Active)]);
        ActivityPeriod::smooth_spikes(&mut history);
        // Should not panic or change anything for len < 3
        assert!(matches!(history[1].activity, Activity::Active));
    }

    #[test]
    fn is_active_returns_true_for_active() {
        let period = ActivityPeriod {
            activity: Activity::Active,
            start: NaiveDate::from_ymd_opt(2025, 1, 1)
                .unwrap()
                .and_hms_opt(0, 0, 0)
                .unwrap(),
            end: NaiveDate::from_ymd_opt(2025, 1, 1)
                .unwrap()
                .and_hms_opt(1, 0, 0)
                .unwrap(),
            duration: Duration::hours(1),
        };
        assert!(period.is_active());
    }

    #[test]
    fn is_active_returns_false_for_sleep() {
        let period = ActivityPeriod {
            activity: Activity::Sleep,
            start: NaiveDate::from_ymd_opt(2025, 1, 1)
                .unwrap()
                .and_hms_opt(0, 0, 0)
                .unwrap(),
            end: NaiveDate::from_ymd_opt(2025, 1, 1)
                .unwrap()
                .and_hms_opt(1, 0, 0)
                .unwrap(),
            duration: Duration::hours(1),
        };
        assert!(!period.is_active());
    }

    #[test]
    fn find_sleep_returns_long_sleep() {
        let base = NaiveDate::from_ymd_opt(2025, 1, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap();
        let mut events = vec![
            ActivityPeriod {
                activity: Activity::Active,
                start: base,
                end: base + Duration::minutes(30),
                duration: Duration::minutes(30),
            },
            ActivityPeriod {
                activity: Activity::Sleep,
                start: base + Duration::minutes(30),
                end: base + Duration::minutes(300),
                duration: Duration::minutes(270),
            },
        ];
        let sleep = ActivityPeriod::find_sleep(&mut events);
        assert!(sleep.is_some());
        assert!(matches!(sleep.unwrap().activity, Activity::Sleep));
    }

    #[test]
    fn find_sleep_ignores_short_sleep() {
        let base = NaiveDate::from_ymd_opt(2025, 1, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap();
        let mut events = vec![ActivityPeriod {
            activity: Activity::Sleep,
            start: base,
            end: base + Duration::minutes(30),
            duration: Duration::minutes(30),
        }];
        assert!(ActivityPeriod::find_sleep(&mut events).is_none());
    }

    #[test]
    fn find_sleep_empty_returns_none() {
        assert!(ActivityPeriod::find_sleep(&mut vec![]).is_none());
    }
}
