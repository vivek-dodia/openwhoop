use std::fmt::{Debug, Display};

use chrono::{NaiveTime, TimeDelta, Timelike};

use crate::helpers::{
    format_hm::FormatHM,
    time_math::{mean, mean_deltas, mean_time, round_float, std_dev_delta, std_time},
};

use super::SleepCycle;

#[derive(Default)]
pub struct SleepConsistencyAnalyzer {
    sleep_records: Vec<SleepCycle>,
    durations: Vec<TimeDelta>,
    start_times: Vec<NaiveTime>,
    end_times: Vec<NaiveTime>,
    midpoints: Vec<NaiveTime>,
}

#[derive(Debug, Clone, Copy)]
pub struct SleepMetrics {
    pub duration: DurationMetric<TimeDelta>,
    pub start_time: DurationMetric<NaiveTime>,
    pub end_time: DurationMetric<NaiveTime>,
    pub midpoint: DurationMetric<NaiveTime>,
    pub score: ConsistencyScore,
}

#[derive(Debug, Clone, Copy)]
pub struct ConsistencyScore {
    pub score: f64,
    pub duration_score: f64,
    pub timing_score: f64,
}

#[derive(Clone, Copy, Default)]
pub struct DurationMetric<Value> {
    pub std: Value,
    pub mean: Value,
    pub cv: f64,
}

impl SleepConsistencyAnalyzer {
    pub fn new(sleep_records: Vec<SleepCycle>) -> Self {
        let mut analyzer = SleepConsistencyAnalyzer {
            sleep_records,
            ..Default::default()
        };
        analyzer.process_records();
        analyzer
    }

    fn process_records(&mut self) {
        for &cycle in &self.sleep_records {
            let start = cycle.start;
            let end = cycle.end;

            self.durations.push(end - start);
            self.start_times.push(start.time());
            self.end_times.push(end.time());
            self.midpoints.push((start + ((end - start) / 2)).time());
        }
    }

    pub fn calculate_consistency_metrics(&self) -> SleepMetrics {
        // Calculate statistics for duration
        let duration = self.duration_metric();

        // Calculate statistics for start time
        let start_time = self.duration_metrics(&self.start_times);

        // Calculate statistics for end time
        let end_time = self.duration_metrics(&self.end_times);

        // Calculate statistics for midpoint
        let midpoint = self.duration_metrics(&self.midpoints);

        // Duration consistency
        let duration_score = round_float(f64::max(0.0, 100.0 - duration.cv));

        let get_score = |metric: &DurationMetric<NaiveTime>| f64::max(0.0, 100.0 - metric.cv);

        let timing_scores = [
            get_score(&start_time),
            get_score(&end_time),
            get_score(&midpoint),
        ];

        let timing_score = round_float(mean(&timing_scores));
        let mut total_scores = timing_scores.to_vec();
        total_scores.push(duration_score);
        let overall_score = mean(&total_scores);

        let score = ConsistencyScore {
            score: round_float(overall_score),
            duration_score,
            timing_score,
        };

        SleepMetrics {
            duration,
            start_time,
            end_time,
            midpoint,
            score,
        }
    }

    fn duration_metric(&self) -> DurationMetric<TimeDelta> {
        let durations = &self.durations;
        let mean = mean_deltas(durations);
        let std = std_dev_delta(durations, mean);
        let cv = round_float(std.num_seconds() as f64 / mean.num_seconds() as f64 * 100.0);

        DurationMetric { std, mean, cv }
    }

    fn duration_metrics(&self, times: &[NaiveTime]) -> DurationMetric<NaiveTime> {
        let mean = mean_time(times);
        let std = std_time(times, &mean);

        let num_seconds = |time: NaiveTime| {
            time.hour() as f64 * 3600.0 + time.minute() as f64 * 60.0 + time.second() as f64
        };

        let cv = round_float(num_seconds(std) / num_seconds(mean) * 100.0);
        DurationMetric { std, mean, cv }
    }
}

impl<Value> Debug for DurationMetric<Value>
where
    Value: FormatHM,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DurationMetric")
            .field("std", &self.std.format_hm())
            .field("mean", &self.mean.format_hm())
            .field("cv", &self.cv)
            .finish()
    }
}

impl<Value> Display for DurationMetric<Value>
where
    Value: FormatHM,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "STD: {}, Mean: {}, CV: {}",
            self.std.format_hm(),
            self.mean.format_hm(),
            self.cv
        ))
    }
}

impl Display for SleepMetrics {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "Duration: {}\nStart time: {}\nEnd time: {}\nMidpoint: {}\nScores:\n",
            self.duration, self.start_time, self.end_time, self.midpoint,
        ))?;
        f.write_fmt(format_args!(
            "\tDuration score: {}\n\tTiming score: {}\n\tOverall score: {}",
            self.score.duration_score, self.score.timing_score, self.score.score,
        ))?;
        Ok(())
    }
}
