use std::collections::BTreeMap;

use chrono::NaiveDateTime;
use db_entities::heart_rate;
use sea_orm::{
    ActiveValue::NotSet, ColumnTrait, EntityTrait, QueryFilter, QueryOrder, QuerySelect,
    SelectColumns, Set, Unchanged,
};
use whoop::ParsedHistoryReading;

use crate::DatabaseHandler;

pub struct StressCalculator;

#[derive(Debug, Clone, Copy)]
pub struct StressScore {
    pub time: NaiveDateTime,
    pub score: f64,
}

impl StressCalculator {
    pub const MIN_READING_PERIOD: usize = 120;

    pub fn calculate_stress(hr: &[ParsedHistoryReading]) -> Option<StressScore> {
        if hr.len() < Self::MIN_READING_PERIOD {
            return None;
        }

        let time = hr.last()?.time;
        let hr = hr.iter().map(|hr| hr.bpm).collect::<Vec<_>>();
        let score = StressCalcParams::new(hr).stress_score();
        Some(StressScore { time, score })
    }
}

#[derive(Debug)]
struct StressCalcParams {
    min: u16,
    max: u16,
    mode: u16,
    mode_freq: u16,
    count: u16,
}

impl StressCalcParams {
    // Uses hr instead of rr measured by whoop, because whoop doesn't always measure rr
    fn new(hr: Vec<u8>) -> Self {
        let count = hr.len() as u16;
        let rr_readings = hr
            .into_iter()
            .map(|bpm| (60.0 / f64::from(bpm) * 1000.0).round() as u16);

        let mut counts = BTreeMap::new();

        for num in rr_readings {
            *counts.entry(num).or_insert(0_u16) += 1;
        }

        let min = counts.keys().min().copied().unwrap_or_default();
        let max = counts.keys().max().copied().unwrap_or_default();
        let (mode, mode_freq) = counts
            .into_iter()
            .max_by(|(_ak, av), (_bk, bv)| av.cmp(bv))
            .unwrap_or_default();

        Self {
            min,
            max,
            mode,
            mode_freq,
            count,
        }
    }

    fn stress_score(self) -> f64 {
        let vr = f64::from(self.max - self.min) / 1000_f64;

        if vr < 0.0001 {
            return 0_f64;
        }

        let a_mode = f64::from(self.mode_freq) / f64::from(self.count) * 100_f64;
        (a_mode / (2_f64 * vr * f64::from(self.mode) / 1000_f64))
            .round()
            .min(1000_f64)
            / 100_f64
    }
}

impl DatabaseHandler {
    pub(crate) async fn last_stress_time(&self) -> anyhow::Result<Option<NaiveDateTime>> {
        let reading = heart_rate::Entity::find()
            .filter(heart_rate::Column::Stress.is_not_null())
            .order_by_desc(heart_rate::Column::Time)
            .select_only()
            .select_column(heart_rate::Column::Time)
            .into_tuple()
            .one(&self.db)
            .await?;

        Ok(reading)
    }

    pub(crate) async fn update_stress_on_reading(&self, stress: StressScore) -> anyhow::Result<()> {
        let model = heart_rate::ActiveModel {
            id: NotSet,
            bpm: NotSet,
            time: Unchanged(stress.time),
            rr_intervals: NotSet,
            activity: NotSet,
            stress: Set(Some(stress.score)),
        };

        heart_rate::Entity::update_many()
            .filter(heart_rate::Column::Time.eq(stress.time))
            .set(model)
            .exec(&self.db)
            .await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::algo::stress::StressCalcParams;

    #[test]
    fn test_stress_calc() {
        let hr = [
            90, 89, 88, 87, 88, 92, 94, 95, 96, 97, 98, 97, 99, 101, 103, 104, 106, 107, 107, 108,
            108, 109, 108, 108, 108, 108, 109, 109, 110, 111, 113, 113, 113, 113, 113, 112, 111,
            110, 109, 108, 107, 106, 105, 104, 104, 103, 103, 103, 102, 101, 101, 100, 100, 100,
            100, 101, 100, 98, 97, 96, 95, 95, 95, 96, 96, 97, 97, 97, 98, 99, 101, 100, 100, 100,
            100, 99, 99, 99, 99, 100, 99, 98, 98, 98, 98, 98, 98, 98, 98, 97, 98, 98, 98, 97, 97,
            96, 96, 96, 95, 94, 93, 93, 94, 94, 95, 96, 96, 96, 96, 95, 94, 95, 95, 96, 96, 96, 96,
            96, 97, 98,
        ];
        dbg!(StressCalcParams::new(hr.to_vec()).stress_score());

        let hr = [
            111, 112, 110, 111, 111, 113, 114, 116, 116, 116, 118, 117, 119, 118, 117, 117, 116,
            115, 115, 115, 115, 115, 115, 115, 115, 114, 113, 112, 112, 111, 111, 110, 110, 111,
            112, 114, 116, 116, 117, 117, 119, 121, 122, 123, 123, 124, 124, 123, 122, 120, 119,
            118, 118, 118, 118, 118, 118, 118, 118, 118, 116, 114, 113, 111, 109, 108, 108, 110,
            112, 114, 116, 118, 117, 116, 115, 113, 112, 112, 114, 116, 115, 115, 117, 119, 119,
            120, 120, 119, 119, 119, 118, 117, 117, 116, 115, 115, 115, 115, 115, 116, 116, 116,
            118, 120, 122, 123, 124, 124, 125, 125, 126, 126, 126, 126, 127, 127, 126, 125, 124,
            123,
        ];
        dbg!(StressCalcParams::new(hr.to_vec()).stress_score());

        let hr = [
            60, 61, 59, 59, 59, 59, 59, 60, 60, 60, 60, 60, 61, 61, 61, 61, 61, 61, 61, 61, 63, 63,
            63, 63, 64, 63, 63, 63, 62, 62, 62, 62, 61, 61, 61, 61, 62, 62, 62, 62, 62, 62, 62, 62,
            62, 62, 62, 62, 62, 62, 62, 62, 62, 63, 63, 63, 63, 63, 63, 63, 64, 64, 64, 64, 64, 65,
            65, 64, 64, 64, 64, 64, 64, 64, 64, 64, 64, 64, 64, 65, 65, 65, 64, 64, 63, 63, 63, 63,
            62, 62, 62, 61, 61, 61, 61, 62, 62, 62, 61, 61, 61, 61, 62, 62, 62, 62, 62, 63, 63, 63,
            63, 64, 63, 63, 63, 62, 62, 63, 63, 63,
        ];
        dbg!(StressCalcParams::new(hr.to_vec()).stress_score());

        let hr = [
            50, 50, 50, 50, 51, 52, 54, 54, 54, 55, 56, 56, 57, 56, 56, 56, 56, 54, 52, 52, 51, 50,
            49, 49, 49, 49, 48, 47, 46, 47, 47, 48, 49, 50, 52, 51, 51, 51, 52, 53, 53, 54, 54, 55,
            55, 56, 54, 53, 52, 52, 53, 53, 50, 50, 50, 50, 50, 49, 49, 50, 49, 49, 49, 48, 48, 48,
            48, 48, 47, 47, 48, 48, 50, 50, 50, 50, 52, 52, 53, 53, 54, 54, 54, 55, 56, 57, 58, 58,
            58, 60, 59, 59, 58, 58, 59, 58, 58, 57, 58, 59, 58, 58, 57, 57, 58, 59, 59, 59, 58, 56,
            55, 55, 54, 55, 55, 54, 55, 55, 55, 54,
        ];
        dbg!(StressCalcParams::new(hr.to_vec()).stress_score());

        let hr = [
            140, 141, 142, 143, 144, 142, 140, 137, 136, 135, 134, 133, 133, 132, 131, 131, 131,
            130, 130, 129, 129, 127, 126, 125, 124, 124, 123, 122, 122, 121, 122, 122, 119, 119,
            119, 119, 118, 120, 121, 121, 121, 119, 119, 118, 117, 117, 116, 115, 114, 114, 114,
            114, 112, 111, 112, 111, 110, 108, 108, 107, 107, 108, 110, 110, 110, 111, 111, 112,
            112, 112, 112, 112, 112, 112, 114, 114, 114, 115, 115, 114, 114, 114, 113, 113, 113,
            113, 113, 112, 113, 115, 115, 116, 117, 116, 116, 117, 117, 118, 119, 119, 120, 119,
            118, 117, 116, 115, 118, 117, 116, 117, 117, 115, 114, 113, 112, 113, 113, 113, 114,
            114,
        ];
        dbg!(StressCalcParams::new(hr.to_vec()).stress_score());
    }
}
