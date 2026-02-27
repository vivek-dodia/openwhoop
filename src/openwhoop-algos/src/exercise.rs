use std::fmt::Display;

use chrono::TimeDelta;

use crate::helpers::{
    format_hm::FormatHM,
    time_math::{mean_deltas, std_dev_delta},
};
use openwhoop_types::activities::ActivityPeriod;

#[derive(Debug, Default)]
pub struct ExerciseMetrics {
    pub total_duration: TimeDelta,
    pub count: u64,
    pub mean_duration: TimeDelta,
    pub duration_std: TimeDelta,
}

impl ExerciseMetrics {
    pub fn new(exercises: Vec<ActivityPeriod>) -> Self {
        if exercises.is_empty() {
            return Self::default();
        }

        let count = exercises.len().try_into().unwrap_or(u64::MAX);
        let durations = exercises
            .into_iter()
            .map(|e| e.to - e.from)
            .collect::<Vec<_>>();

        let mean_duration = mean_deltas(durations.as_slice());

        Self {
            count,
            mean_duration,
            duration_std: std_dev_delta(durations.as_slice(), mean_duration),
            total_duration: durations.into_iter().sum(),
        }
    }
}

impl Display for ExerciseMetrics {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "Duration: {:?}h\nCount: {}\nMean duration: {}\nDuration std: {}",
            self.total_duration.num_hours(),
            self.count,
            self.mean_duration.format_hm(),
            self.duration_std.format_hm()
        ))
    }
}

#[cfg(test)]
mod tests {
    use chrono::TimeDelta;

    use super::ExerciseMetrics;

    #[test]
    fn test_metrics_empty() {
        let metrics = ExerciseMetrics::new(Vec::new());
        assert_eq!(metrics.count, 0);
        assert_eq!(metrics.duration_std, TimeDelta::default());
        assert_eq!(metrics.mean_duration, TimeDelta::default());
        assert_eq!(metrics.total_duration, TimeDelta::default());
    }

    #[test]
    fn test_metrics_with_exercises() {
        use chrono::NaiveDate;
        use openwhoop_types::activities::{ActivityPeriod, ActivityType};

        let base = NaiveDate::from_ymd_opt(2025, 1, 1)
            .unwrap()
            .and_hms_opt(8, 0, 0)
            .unwrap();

        let exercises = vec![
            ActivityPeriod {
                period_id: base.date(),
                from: base,
                to: base + TimeDelta::hours(1),
                activity: ActivityType::Running,
            },
            ActivityPeriod {
                period_id: base.date(),
                from: base + TimeDelta::hours(4),
                to: base + TimeDelta::hours(5),
                activity: ActivityType::Cycling,
            },
        ];

        let metrics = ExerciseMetrics::new(exercises);
        assert_eq!(metrics.count, 2);
        assert_eq!(metrics.total_duration, TimeDelta::hours(2));
        assert_eq!(metrics.mean_duration, TimeDelta::hours(1));
        assert_eq!(metrics.duration_std, TimeDelta::seconds(0)); // identical durations
    }
}
