use chrono::{NaiveDate, NaiveDateTime, TimeDelta};
use db_entities::sleep_cycles;
use sea_orm::{EntityTrait, QueryOrder};
use whoop::ParsedHistoryReading;

use crate::DatabaseHandler;

use super::ActivityPeriod;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SleepCycle {
    pub id: NaiveDate,
    pub start: NaiveDateTime,
    pub end: NaiveDateTime,
    pub min_bpm: u8,
    pub max_bpm: u8,
    pub avg_bpm: u8,
    pub min_hrv: u16,
    pub max_hrv: u16,
    pub avg_hrv: u16,
}

impl SleepCycle {
    pub fn from_event(event: ActivityPeriod, history: &[ParsedHistoryReading]) -> SleepCycle {
        let (heart_rate, rr): (Vec<u64>, Vec<Vec<_>>) = history
            .iter()
            .filter(|h| h.time >= event.start && h.time <= event.end)
            .map(|h| (h.bpm as u64, h.rr.clone()))
            .unzip();

        let rr = Self::clean_rr(rr);
        let rolling_hrv = Self::rolling_hrv(rr);

        let min_hrv = rolling_hrv.iter().min().copied().unwrap_or_default() as u16;
        let max_hrv = rolling_hrv.iter().max().copied().unwrap_or_default() as u16;

        let hrv_count = rolling_hrv.len() as u64;
        let hrv = rolling_hrv.into_iter().sum::<u64>() / hrv_count;
        let avg_hrv = hrv as u16;

        let min_bpm = heart_rate.iter().min().copied().unwrap_or_default() as u8;
        let max_bpm = heart_rate.iter().max().copied().unwrap_or_default() as u8;

        let heart_rate_count = heart_rate.len() as u64;
        let bpm = heart_rate.into_iter().sum::<u64>() / heart_rate_count;
        let avg_bpm = bpm as u8;

        let id = event.end.date();

        Self {
            id,
            start: event.start,
            end: event.end,
            min_bpm,
            max_bpm,
            avg_bpm,
            min_hrv,
            max_hrv,
            avg_hrv,
        }
    }

    pub fn duration(&self) -> TimeDelta {
        self.end - self.start
    }

    fn clean_rr(rr: Vec<Vec<u16>>) -> Vec<u64> {
        rr.into_iter()
            .filter_map(|rr| {
                if rr.is_empty() {
                    return None;
                }
                let count = rr.len() as u64;
                let rr_sum = rr.into_iter().map(u64::from).sum::<u64>();
                Some(rr_sum / count)
            })
            .collect()
    }

    fn rolling_hrv(rr: Vec<u64>) -> Vec<u64> {
        rr.windows(300).filter_map(Self::calculate_rmssd).collect()
    }

    fn calculate_rmssd(window: &[u64]) -> Option<u64> {
        if window.len() < 2 {
            return None;
        }

        let rr_diff: Vec<f64> = window
            .windows(2)
            .map(|w| (w[1] as f64 - w[0] as f64).powi(2))
            .collect();

        let rr_count = rr_diff.len() as f64;
        Some((rr_diff.into_iter().sum::<f64>() / rr_count).sqrt() as u64)
    }
}

impl From<sleep_cycles::Model> for SleepCycle {
    fn from(value: sleep_cycles::Model) -> Self {
        Self {
            id: value.sleep_id,
            start: value.start,
            end: value.end,
            min_bpm: value.min_bpm.try_into().unwrap(),
            max_bpm: value.max_bpm.try_into().unwrap(),
            avg_bpm: value.avg_bpm.try_into().unwrap(),
            min_hrv: value.min_hrv.try_into().unwrap(),
            max_hrv: value.max_hrv.try_into().unwrap(),
            avg_hrv: value.avg_hrv.try_into().unwrap(),
        }
    }
}

impl DatabaseHandler {
    pub async fn get_sleep_cycles(&self) -> anyhow::Result<Vec<SleepCycle>> {
        Ok(sleep_cycles::Entity::find()
            .order_by_asc(sleep_cycles::Column::Start)
            .all(&self.db)
            .await?
            .into_iter()
            .map(SleepCycle::from)
            .collect())
    }
}
