use chrono::NaiveDateTime;
use openwhoop_entities::heart_rate;
use sea_orm::{ColumnTrait, Condition, EntityTrait, QueryFilter, QueryOrder, QuerySelect};
use openwhoop_codec::{Activity, ParsedHistoryReading};

use crate::DatabaseHandler;

#[derive(Default, Debug)]
pub struct SearchHistory {
    pub from: Option<NaiveDateTime>,
    pub to: Option<NaiveDateTime>,
    pub limit: Option<u64>,
}

impl SearchHistory {
    pub(crate) fn conditions(self) -> Condition {
        Condition::all()
            .add_option(self.from.map(|from| heart_rate::Column::Time.gt(from)))
            .add_option(self.to.map(|to| heart_rate::Column::Time.lt(to)))
    }
}

impl DatabaseHandler {
    pub async fn search_history(
        &self,
        options: SearchHistory,
    ) -> anyhow::Result<Vec<ParsedHistoryReading>> {
        let limit = options.limit;
        let history = heart_rate::Entity::find()
            .filter(options.conditions())
            .filter(heart_rate::Column::Activity.is_not_null())
            .limit(limit)
            .order_by_asc(heart_rate::Column::Time)
            .all(&self.db)
            .await?
            .into_iter()
            .map(Self::parse_reading)
            .collect();

        Ok(history)
    }

    fn parse_reading(model: heart_rate::Model) -> ParsedHistoryReading {
        ParsedHistoryReading {
            time: model.time,
            bpm: model.bpm.try_into().unwrap_or(u8::MAX),
            rr: model
                .rr_intervals
                .split(',')
                .filter_map(|rr| rr.parse().ok())
                .collect(),
            activity: model.activity.map(Activity::from).unwrap(),
            imu_data: {
                if let Some(data) = model.imu_data {
                    serde_json::from_value(data).unwrap()
                } else {
                    Default::default()
                }
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_reading_converts_model() {
        let time = chrono::NaiveDate::from_ymd_opt(2025, 1, 1)
            .unwrap()
            .and_hms_opt(12, 0, 0)
            .unwrap();

        let model = heart_rate::Model {
            id: 1,
            bpm: 72,
            time,
            rr_intervals: "833,850".to_string(),
            activity: Some(500_000_000),
            stress: Some(3.5),
            spo2: None,
            skin_temp: None,
            imu_data: None,
            sensor_data: None,
            synced: false,
        };

        let reading = DatabaseHandler::parse_reading(model);
        assert_eq!(reading.bpm, 72);
        assert_eq!(reading.rr, vec![833, 850]);
        assert_eq!(reading.activity, Activity::Active);
        assert!(reading.imu_data.is_none());
    }

    #[test]
    fn parse_reading_empty_rr() {
        let time = chrono::NaiveDate::from_ymd_opt(2025, 1, 1)
            .unwrap()
            .and_hms_opt(12, 0, 0)
            .unwrap();

        let model = heart_rate::Model {
            id: 1,
            bpm: 60,
            time,
            rr_intervals: "".to_string(),
            activity: Some(0),
            stress: None,
            spo2: None,
            skin_temp: None,
            imu_data: None,
            sensor_data: None,
            synced: false,
        };

        let reading = DatabaseHandler::parse_reading(model);
        assert_eq!(reading.bpm, 60);
        assert!(reading.rr.is_empty());
        assert_eq!(reading.activity, Activity::Inactive);
    }

    #[test]
    fn parse_reading_with_imu_data() {
        use openwhoop_codec::ImuSample;
        let time = chrono::NaiveDate::from_ymd_opt(2025, 1, 1)
            .unwrap()
            .and_hms_opt(12, 0, 0)
            .unwrap();

        let imu_samples = vec![ImuSample {
            acc_x_g: 1.0,
            acc_y_g: 0.0,
            acc_z_g: -1.0,
            gyr_x_dps: 10.0,
            gyr_y_dps: 20.0,
            gyr_z_dps: 30.0,
        }];

        let model = heart_rate::Model {
            id: 1,
            bpm: 70,
            time,
            rr_intervals: "800".to_string(),
            activity: Some(500_000_000),
            stress: None,
            spo2: None,
            skin_temp: None,
            imu_data: Some(serde_json::to_value(&imu_samples).unwrap()),
            sensor_data: None,
            synced: false,
        };

        let reading = DatabaseHandler::parse_reading(model);
        let imu = reading.imu_data.unwrap();
        assert_eq!(imu.len(), 1);
        assert_eq!(imu[0].acc_x_g, 1.0);
    }

    #[tokio::test]
    async fn search_history_integration() {
        let db = DatabaseHandler::new("sqlite::memory:").await;

        let readings: Vec<openwhoop_codec::HistoryReading> = (0..3)
            .map(|i| openwhoop_codec::HistoryReading {
                unix: 1735689600000 + i * 1000,
                bpm: 70 + i as u8,
                rr: vec![850],
                activity: 500_000_000,
                imu_data: vec![],
                sensor_data: None,
            })
            .collect();

        for r in readings {
            db.create_reading(r).await.unwrap();
        }

        let history = db
            .search_history(SearchHistory::default())
            .await
            .unwrap();
        assert_eq!(history.len(), 3);

        let history = db
            .search_history(SearchHistory {
                from: None,
                to: None,
                limit: Some(2),
            })
            .await
            .unwrap();
        assert_eq!(history.len(), 2);
    }
}
