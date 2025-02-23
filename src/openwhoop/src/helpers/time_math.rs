use chrono::{NaiveTime, TimeDelta, Timelike as _};

pub fn map_time(time: &NaiveTime) -> i64 {
    let mut h = time.hour() as i64;
    if h > 12 {
        h -= 24;
    }
    let m = time.minute() as i64;
    let s = time.second() as i64;
    h * 3600 + m * 60 + s
}

pub fn std_time(times: &[NaiveTime], mean: &NaiveTime) -> NaiveTime {
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

pub fn mean_time(times: &[NaiveTime]) -> NaiveTime {
    let mut mean = times.iter().map(map_time).sum::<i64>() / times.len() as i64;
    if mean < 0 {
        mean += 86400;
    }
    let h = mean / 3600;
    let m = (mean % 3600) / 60;
    let s = mean % 60;
    NaiveTime::from_hms_opt(h as u32, m as u32, s as u32).expect("Invalid time")
}

pub fn mean_deltas(durations: &[TimeDelta]) -> TimeDelta {
    durations.iter().sum::<TimeDelta>() / durations.len() as i32
}

pub fn mean(values: &[f64]) -> f64 {
    values.iter().sum::<f64>() / values.len() as f64
}

pub fn std_dev_delta(durations: &[TimeDelta], mean: TimeDelta) -> TimeDelta {
    let variance = durations
        .iter()
        .map(|x| (*x - mean).num_seconds().pow(2))
        .sum::<i64>()
        / durations.len() as i64;

    TimeDelta::seconds(variance.isqrt())
}

pub fn round_float(v: f64) -> f64 {
    (v * 100.0).round() / 100.0
}
