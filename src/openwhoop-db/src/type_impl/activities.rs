use std::str::FromStr;

use db_entities::activities;
use openwhoop_types::activities::{ActivityPeriod, ActivityType, SearchActivityPeriods};
use sea_orm::{
    ColumnTrait, Condition, EntityTrait, NotSet, QueryFilter, QueryOrder, Set,
    sea_query::OnConflict,
};

use crate::DatabaseHandler;

impl DatabaseHandler {
    pub async fn create_activity(&self, activity: ActivityPeriod) -> anyhow::Result<()> {
        let model = activities::ActiveModel {
            id: NotSet,
            period_id: Set(activity.period_id),
            start: Set(activity.from),
            end: Set(activity.to),
            activity: Set(activity.activity.to_string()),
        };

        activities::Entity::insert(model)
            .on_conflict(
                OnConflict::column(activities::Column::Start)
                    .update_column(activities::Column::End)
                    .update_column(activities::Column::Activity)
                    .to_owned(),
            )
            .exec(&self.db)
            .await?;

        Ok(())
    }
    pub async fn search_activities(
        &self,
        options: SearchActivityPeriods,
    ) -> anyhow::Result<Vec<ActivityPeriod>> {
        let activities = activities::Entity::find()
            .filter(search_activity_periods_query(options))
            .all(&self.db)
            .await?
            .into_iter()
            .map(map_activity_period)
            .collect();

        Ok(activities)
    }

    pub async fn get_latest_activity(&self) -> anyhow::Result<Option<ActivityPeriod>> {
        Ok(activities::Entity::find()
            .order_by_desc(activities::Column::End)
            .one(&self.db)
            .await?
            .map(map_activity_period))
    }
}

fn map_activity_period(value: activities::Model) -> ActivityPeriod {
    ActivityPeriod {
        period_id: value.period_id,
        from: value.start,
        to: value.end,
        activity: ActivityType::from_str(value.activity.as_str()).unwrap(),
    }
}

fn search_activity_periods_query(query: SearchActivityPeriods) -> Condition {
    Condition::all()
        .add_option(query.from.map(|from| activities::Column::Start.gt(from)))
        .add_option(query.to.map(|to| activities::Column::End.lt(to)))
        .add_option(
            query
                .activity
                .map(|activity| activities::Column::Activity.eq(activity.to_string())),
        )
}
