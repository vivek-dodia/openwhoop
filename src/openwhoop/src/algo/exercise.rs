use std::fmt::Display;

use chrono::TimeDelta;

use crate::{
    helpers::{
        format_hm::FormatHM,
        time_math::{mean_deltas, std_dev_delta},
    },
    types::activities::ActivityPeriod,
};

#[derive(Debug)]
pub struct ExerciseMetrics {
    pub total_duration: TimeDelta,
    pub count: u64,
    pub mean_duration: TimeDelta,
    pub duration_std: TimeDelta,
}

impl ExerciseMetrics {
    pub fn new(exercises: Vec<ActivityPeriod>) -> Self {
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
