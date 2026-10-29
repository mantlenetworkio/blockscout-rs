// SPDX-License-Identifier: LicenseRef-Blockscout

use crate::chart_prelude::*;
use blockscout_db::entity::{blocks, last_fetched_counters, transactions};

/// Counter type under which (recent Mantle) Blockscout stores the exact count
/// of all transactions each time it refreshes its own cache.
const INDEXER_TRANSACTIONS_COUNT_KEY: &str = "transactions_count";
/// A stored count older than this is not used, in case the indexer stopped refreshing it.
const INDEXER_TRANSACTIONS_COUNT_MAX_AGE_HOURS: i64 = 6;
/// How far in the future the update time of a stored count may be due to clock skew.
const INDEXER_TRANSACTIONS_COUNT_MAX_SKEW_MINUTES: i64 = 5;

pub struct TotalTxnsQueryBehaviour;

impl TotalTxnsQueryBehaviour {
    /// The count of all transactions stored by the indexer, if it's recent enough.
    ///
    /// Counting the whole table takes tens of minutes on large chains, so the
    /// count is reused instead of being calculated twice. Returns `None`
    /// (i.e. the table is counted) if the value is missing, stale, invalid or
    /// can't be read.
    async fn query_indexer_count(cx: &UpdateContext<'_>) -> Option<u64> {
        // The stored count replaces counting the table right now, so its age
        // is measured from the current time rather than from the update time.
        let now = Utc::now().naive_utc();
        let max_age = TimeDelta::hours(INDEXER_TRANSACTIONS_COUNT_MAX_AGE_HOURS);
        let max_skew = TimeDelta::minutes(INDEXER_TRANSACTIONS_COUNT_MAX_SKEW_MINUTES);
        // The database checks the bounds and casts the value, so that an unusual
        // `updated_at` or a huge `numeric` is never decoded (which could panic):
        // it's filtered out or fails the cast instead.
        let value: Option<i64> = last_fetched_counters::Entity::find()
            .select_only()
            .column_as(
                Expr::col(last_fetched_counters::Column::Value).cast_as(Alias::new("bigint")),
                "value",
            )
            .filter(last_fetched_counters::Column::CounterType.eq(INDEXER_TRANSACTIONS_COUNT_KEY))
            .filter(last_fetched_counters::Column::UpdatedAt.gte(now - max_age))
            .filter(last_fetched_counters::Column::UpdatedAt.lte(now + max_skew))
            .into_tuple()
            .one(cx.indexer_db)
            .await
            .inspect_err(|err| {
                tracing::warn!(
                    "failed to read transactions count stored by the indexer, counting the table instead: {err}"
                )
            })
            .ok()??;
        u64::try_from(value?).ok()
    }
}

impl RemoteQueryBehaviour for TotalTxnsQueryBehaviour {
    type Output = DateValue<String>;

    async fn query_data(
        cx: &UpdateContext<'_>,
        _range: UniversalRange<DateTime<Utc>>,
    ) -> Result<Self::Output, ChartError> {
        let blockscout = cx.indexer_db;
        let timespan: NaiveDateTime = blocks::Entity::find()
            .select_only()
            .column_as(Expr::col(blocks::Column::Timestamp).max(), "timestamp")
            .filter(blocks::Column::Consensus.eq(true))
            .into_tuple()
            .one(blockscout)
            .await
            .map_err(ChartError::IndexerDB)?
            .ok_or_else(|| ChartError::Internal("no block timestamps in database".into()))?;

        let value = match Self::query_indexer_count(cx).await {
            Some(value) => value,
            None => transactions::Entity::find()
                .select_only()
                .count(blockscout)
                .await
                .map_err(ChartError::IndexerDB)?,
        };

        let data = DateValue::<String> {
            timespan: timespan.date(),
            value: value.to_string(),
        };
        Ok(data)
    }
}

pub type TotalTxnsRemote = RemoteDatabaseSource<TotalTxnsQueryBehaviour>;

pub struct Properties;

impl Named for Properties {
    fn name() -> String {
        "totalTxns".into()
    }
}

impl ChartProperties for Properties {
    type Resolution = NaiveDate;

    fn chart_type() -> ChartType {
        ChartType::Counter
    }
    fn missing_date_policy() -> MissingDatePolicy {
        MissingDatePolicy::FillPrevious
    }
    fn indexing_status_requirement() -> IndexingStatus {
        IndexingStatus::LEAST_RESTRICTIVE
    }
}

pub struct TotalTxnsEstimation;

impl ValueEstimation for TotalTxnsEstimation {
    async fn estimate(blockscout: &DatabaseConnection) -> Result<DateValue<String>, ChartError> {
        // `now()` is more relevant when taken right before the query rather than
        // `cx.time` measured a bit earlier.
        let now = Utc::now();
        let value = query_estimated_table_rows(blockscout, transactions::Entity.table_name())
            .await
            .map_err(ChartError::IndexerDB)?
            .map(|n| u64::try_from(n).unwrap_or(0))
            .unwrap_or(0);
        Ok(DateValue {
            timespan: now.date_naive(),
            value: value.to_string(),
        })
    }
}

// Independent from `NewTxns` because this needs to work on not-fully-indexed
// just as well.
pub type TotalTxns =
    DirectPointLocalDbChartSourceWithEstimate<TotalTxnsRemote, TotalTxnsEstimation, Properties>;
pub type TotalTxnsInt = MapParseTo<TotalTxns, i64>;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        data_source::{DataSource, UpdateParameters},
        tests::{
            mock_blockscout::fill_mock_blockscout_data,
            point_construction::dt,
            simple_test::{
                get_counter, prepare_blockscout_chart_test, simple_test_counter,
                test_counter_fallback,
            },
        },
    };

    #[tokio::test]
    #[ignore = "needs database to run"]
    async fn update_total_txns() {
        simple_test_counter::<TotalTxns>("update_total_txns", "58", None).await;
    }

    #[tokio::test]
    #[ignore = "needs database to run"]
    async fn total_txns_fallback() {
        test_counter_fallback::<TotalTxns>("total_txns_fallback").await;
    }

    #[tokio::test]
    #[ignore = "needs database to run"]
    async fn update_total_txns_from_indexer_count() {
        let value = total_txns_with_indexer_count(
            "update_total_txns_from_indexer_count",
            "12345",
            &hours_ago(1),
        )
        .await;
        assert_eq!(value, "12345");
    }

    // in the tests below the stored count is ignored, so the table is counted

    #[tokio::test]
    #[ignore = "needs database to run"]
    async fn update_total_txns_with_stale_indexer_count() {
        let value = total_txns_with_indexer_count(
            "update_total_txns_with_stale_indexer_count",
            "12345",
            &hours_ago(7),
        )
        .await;
        assert_eq!(value, "58");
    }

    #[tokio::test]
    #[ignore = "needs database to run"]
    async fn update_total_txns_with_future_indexer_count() {
        let value = total_txns_with_indexer_count(
            "update_total_txns_with_future_indexer_count",
            "12345",
            &hours_ago(-1),
        )
        .await;
        assert_eq!(value, "58");
    }

    #[tokio::test]
    #[ignore = "needs database to run"]
    async fn update_total_txns_with_huge_indexer_count() {
        let value = total_txns_with_indexer_count(
            "update_total_txns_with_huge_indexer_count",
            "1e29",
            &hours_ago(1),
        )
        .await;
        assert_eq!(value, "58");
    }

    #[tokio::test]
    #[ignore = "needs database to run"]
    async fn update_total_txns_with_infinite_indexer_count_time() {
        for (test_name, stored_at) in [
            (
                "update_total_txns_with_infinity_indexer_count_time",
                "infinity",
            ),
            (
                "update_total_txns_with_minus_infinity_indexer_count_time",
                "-infinity",
            ),
        ] {
            let value = total_txns_with_indexer_count(test_name, "12345", stored_at).await;
            assert_eq!(value, "58");
        }
    }

    fn hours_ago(hours: i64) -> String {
        (Utc::now().naive_utc() - TimeDelta::hours(hours)).to_string()
    }

    /// Updates the counter (at a historical update time) with a transactions
    /// count stored by the indexer. Both stored values are SQL literals.
    async fn total_txns_with_indexer_count(
        test_name: &str,
        stored_value: &str,
        stored_at: &str,
    ) -> String {
        let update_time = dt("2023-03-01T12:00:00");
        let (init_time, db, blockscout) =
            prepare_blockscout_chart_test::<TotalTxns>(test_name, Some(update_time)).await;
        fill_mock_blockscout_data(&blockscout, init_time.date_naive()).await;
        blockscout
            .execute(Statement::from_sql_and_values(
                DbBackend::Postgres,
                "INSERT INTO last_fetched_counters (counter_type, value, inserted_at, updated_at)
                VALUES ($1, $2::numeric, now(), $3::timestamp)",
                [
                    INDEXER_TRANSACTIONS_COUNT_KEY.into(),
                    stored_value.into(),
                    stored_at.into(),
                ],
            ))
            .await
            .unwrap();

        let parameters = UpdateParameters::default_test_parameters(
            &db,
            &blockscout,
            TotalTxns::all_dependencies_chart_keys(),
            Some(init_time),
        )
        .with_force_full();
        let cx = UpdateContext::from_params_now_or_override(parameters);
        TotalTxns::update_recursively(&cx).await.unwrap();
        get_counter::<TotalTxns>(&cx).await.value
    }
}
