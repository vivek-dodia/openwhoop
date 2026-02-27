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
    if times.is_empty() {
        NaiveTime::default()
    } else {
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
}

pub fn mean_time(times: &[NaiveTime]) -> NaiveTime {
    if times.is_empty() {
        NaiveTime::default()
    } else {
        let mut mean = times.iter().map(map_time).sum::<i64>() / times.len() as i64;
        if mean < 0 {
            mean += 86400;
        }
        let h = mean / 3600;
        let m = (mean % 3600) / 60;
        let s = mean % 60;
        NaiveTime::from_hms_opt(h as u32, m as u32, s as u32).expect("Invalid time")
    }
}

pub fn mean_deltas(durations: &[TimeDelta]) -> TimeDelta {
    if durations.is_empty() {
        TimeDelta::default()
    } else {
        durations.iter().sum::<TimeDelta>() / durations.len() as i32
    }
}

pub fn mean(values: &[f64]) -> f64 {
    if values.is_empty() {
        0_f64
    } else {
        values.iter().sum::<f64>() / values.len() as f64
    }
}

pub fn std_dev_delta(durations: &[TimeDelta], mean: TimeDelta) -> TimeDelta {
    if durations.is_empty() {
        TimeDelta::default()
    } else {
        let variance = durations
            .iter()
            .map(|x| (*x - mean).num_seconds().pow(2))
            .sum::<i64>()
            / durations.len() as i64;

        TimeDelta::seconds(variance.isqrt())
    }
}

pub fn round_float(v: f64) -> f64 {
    (v * 100.0).round() / 100.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn map_time_morning() {
        // 08:30:00 -> 8*3600 + 30*60 = 30600
        let t = NaiveTime::from_hms_opt(8, 30, 0).unwrap();
        assert_eq!(map_time(&t), 30600);
    }

    #[test]
    fn map_time_evening_wraps_negative() {
        // 22:00:00 -> (22-24)*3600 = -7200
        let t = NaiveTime::from_hms_opt(22, 0, 0).unwrap();
        assert_eq!(map_time(&t), -7200);
    }

    #[test]
    fn map_time_noon_boundary() {
        // 12:00:00 -> 12*3600 = 43200
        let t = NaiveTime::from_hms_opt(12, 0, 0).unwrap();
        assert_eq!(map_time(&t), 43200);
    }

    #[test]
    fn mean_time_single() {
        let times = vec![NaiveTime::from_hms_opt(8, 0, 0).unwrap()];
        assert_eq!(mean_time(&times), NaiveTime::from_hms_opt(8, 0, 0).unwrap());
    }

    #[test]
    fn mean_time_empty() {
        assert_eq!(mean_time(&[]), NaiveTime::default());
    }

    #[test]
    fn mean_time_evening_average() {
        let times = vec![
            NaiveTime::from_hms_opt(22, 0, 0).unwrap(),
            NaiveTime::from_hms_opt(23, 0, 0).unwrap(),
        ];
        // mapped: -7200, -3600 -> mean = -5400 -> +86400 = 81000 -> 22:30:00
        assert_eq!(
            mean_time(&times),
            NaiveTime::from_hms_opt(22, 30, 0).unwrap()
        );
    }

    #[test]
    fn std_time_empty() {
        let mean = NaiveTime::from_hms_opt(8, 0, 0).unwrap();
        assert_eq!(std_time(&[], &mean), NaiveTime::default());
    }

    #[test]
    fn std_time_identical_values() {
        let t = NaiveTime::from_hms_opt(8, 0, 0).unwrap();
        let times = vec![t, t, t];
        assert_eq!(std_time(&times, &t), NaiveTime::from_hms_opt(0, 0, 0).unwrap());
    }

    #[test]
    fn mean_deltas_empty() {
        assert_eq!(mean_deltas(&[]), TimeDelta::default());
    }

    #[test]
    fn mean_deltas_basic() {
        let durations = vec![TimeDelta::hours(6), TimeDelta::hours(10)];
        assert_eq!(mean_deltas(&durations), TimeDelta::hours(8));
    }

    #[test]
    fn mean_empty() {
        assert_eq!(mean(&[]), 0.0);
    }

    #[test]
    fn mean_basic() {
        assert_eq!(mean(&[2.0, 4.0, 6.0]), 4.0);
    }

    #[test]
    fn std_dev_delta_empty() {
        assert_eq!(
            std_dev_delta(&[], TimeDelta::default()),
            TimeDelta::default()
        );
    }

    #[test]
    fn std_dev_delta_zero_variance() {
        let d = TimeDelta::hours(8);
        assert_eq!(std_dev_delta(&[d, d, d], d), TimeDelta::seconds(0));
    }

    #[test]
    fn round_float_basic() {
        assert_eq!(round_float(3.14159), 3.14);
        assert_eq!(round_float(1.999), 2.0);
        assert_eq!(round_float(0.0), 0.0);
    }
}
