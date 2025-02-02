use chrono::{Duration, NaiveDateTime, TimeDelta};
use whoop::{Activity, ParsedHistoryReading};

const ACTIVITY_CHANGE_THRESHOLD: Duration = Duration::minutes(15);
const MIN_SLEEP_DURATION: Duration = Duration::minutes(60);
pub const MAX_SLEEP_PAUSE: Duration = Duration::minutes(60);

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
                {
                    // Merge with both previous and next activity
                    let prev: TempActivity = merged.pop().unwrap();
                    merged.push(TempActivity {
                        activity: prev.activity,
                        start: prev.start,
                        end: activities[i + 1].end,
                    });
                    i += 1; // Skip next since it’s merged
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
                if model.activity != current_activity {
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
