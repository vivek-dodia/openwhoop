use std::fmt::Debug;

use chrono::{NaiveTime, TimeDelta, Timelike};

use crate::helpers::format_hm::FormatHM;

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

fn map_time(time: &NaiveTime) -> i64 {
    let mut h = time.hour() as i64;
    if h > 12 {
        h -= 24;
    }
    let m = time.minute() as i64;
    let s = time.second() as i64;
    h * 3600 + m * 60 + s
}

fn std_time(times: &[NaiveTime], mean: &NaiveTime) -> NaiveTime {
    let mean = map_time(mean);
    let variance = times
        .iter()
        .map(map_time)
        .map(|x| (x - mean).pow(2))
        .sum::<i64>()
        / times.len() as i64;

    let variance = variance.isqrt();
    let h = variance / 3600;
    let m = (variance % 3600) / 60;
    let s = variance % 60;

    NaiveTime::from_hms_opt(h as u32, m as u32, s as u32).expect("Invalid time")
}

fn mean_time(times: &[NaiveTime]) -> NaiveTime {
    let mean = times.iter().map(map_time).sum::<i64>() / times.len() as i64;
    let h = mean / 3600;
    let m = (mean % 3600) / 60;
    let s = mean % 60;
    NaiveTime::from_hms_opt(h as u32, m as u32, s as u32).expect("Invalid time")
}

fn mean_deltas(durations: &[TimeDelta]) -> TimeDelta {
    durations.iter().sum::<TimeDelta>() / durations.len() as i32
}

fn mean(values: &[f64]) -> f64 {
    values.iter().sum::<f64>() / values.len() as f64
}

fn std_dev_delta(durations: &[TimeDelta], mean: TimeDelta) -> TimeDelta {
    let variance = durations
        .iter()
        .map(|x| (*x - mean).num_seconds().pow(2))
        .sum::<i64>()
        / durations.len() as i64;

    TimeDelta::seconds(variance.isqrt())
}

fn round_float(v: f64) -> f64 {
    (v * 100.0).round() / 100.0
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
