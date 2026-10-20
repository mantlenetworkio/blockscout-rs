// SPDX-License-Identifier: LicenseRef-Blockscout

use std::{collections::HashSet, ops::Range};

use crate::chart_prelude::*;

pub struct NewAccountsStatement;
impl_db_choice!(NewAccountsStatement, UsePrimaryDB);

impl StatementFromRange for NewAccountsStatement {
    fn get_statement(
        range: Option<Range<DateTime<Utc>>>,
        completed_migrations: &IndexerMigrations,
        _enabled_update_charts_recursive: &HashSet<ChartKey>,
    ) -> Statement {
        // An account is new on the date of its first transaction (in chain order).
        // Instead of scanning the whole history, the first transaction in the range of each
        // active account is compared with its first transaction overall, which is a single
        // index lookup by `from_address_hash` (ordered by `block_number`). The account is
        // counted iff both are in the same block, i.e. the first transaction is in the range,
        // so every account is counted in exactly one range.
        //
        // The values are calculated per date, thus the range start is moved to the
        // beginning of its day for the first date to be complete.
        let range = range.map(|r| day_start(&r.start.date_naive())..r.end);
        if completed_migrations.denormalization {
            sql_with_range_filter_opt!(
                DbBackend::Postgres,
                r#"
                    SELECT
                        first_tx.timestamp::date as date,
                        count(*)::TEXT as value
                    FROM (
                        SELECT
                            t.from_address_hash,
                            MIN(t.block_number) as block_number
                        FROM transactions  t
                        WHERE
                            t.block_timestamp != to_timestamp(0) AND
                            t.block_consensus = true {filter}
                        GROUP BY t.from_address_hash
                    ) range_first_tx
                    CROSS JOIN LATERAL (
                        SELECT
                            p.block_number,
                            p.block_timestamp as timestamp
                        FROM transactions  p
                        WHERE
                            p.from_address_hash = range_first_tx.from_address_hash AND
                            p.block_timestamp != to_timestamp(0) AND
                            p.block_consensus = true
                        ORDER BY p.block_number
                        LIMIT 1
                    ) first_tx
                    WHERE first_tx.block_number = range_first_tx.block_number
                    GROUP BY first_tx.timestamp::date;
                "#,
                [],
                "t.block_timestamp",
                range
            )
        } else {
            sql_with_range_filter_opt!(
                DbBackend::Postgres,
                r#"
                    SELECT
                        first_tx.timestamp::date as date,
                        count(*)::TEXT as value
                    FROM (
                        SELECT
                            t.from_address_hash,
                            MIN(t.block_number) as block_number
                        FROM transactions  t
                        JOIN blocks        b ON t.block_hash = b.hash
                        WHERE
                            b.timestamp != to_timestamp(0) AND
                            b.consensus = true {filter}
                        GROUP BY t.from_address_hash
                    ) range_first_tx
                    CROSS JOIN LATERAL (
                        SELECT
                            p.block_number,
                            pb.timestamp
                        FROM transactions  p
                        JOIN blocks        pb ON p.block_hash = pb.hash
                        WHERE
                            p.from_address_hash = range_first_tx.from_address_hash AND
                            pb.timestamp != to_timestamp(0) AND
                            pb.consensus = true
                        ORDER BY p.block_number
                        LIMIT 1
                    ) first_tx
                    WHERE first_tx.block_number = range_first_tx.block_number
                    GROUP BY first_tx.timestamp::date;
                "#,
                [],
                "b.timestamp",
                range
            )
        }
    }
}

pub struct NewAccountsQueryBehaviour;

impl RemoteQueryBehaviour for NewAccountsQueryBehaviour {
    type Output = Vec<DateValue<String>>;

    async fn query_data(
        cx: &UpdateContext<'_>,
        range: UniversalRange<DateTime<Utc>>,
    ) -> Result<Vec<DateValue<String>>, ChartError> {
        let statement_range = data_source_query_range_to_db_statement_range::<
            QueryFullIndexerTimestampRange,
        >(cx, range)
        .await?;
        let statement = NewAccountsStatement::get_statement(
            statement_range.clone(),
            &cx.indexer_applied_migrations,
            &cx.enabled_update_charts_recursive,
        );
        let mut data =
            find_all_points::<_, DateValue<String>>(NewAccountsStatement::get_db(cx)?, statement)
                .await?;
        if let Some(range) = statement_range {
            let range = range.start.date_naive()..=range.end.date_naive();
            trim_out_of_range_sorted(&mut data, range);
        }
        Ok(data)
    }
}

/// Note: each batch only scans transactions within its range (plus one
/// index lookup per active account), so the update is split into batches
/// to keep every statement reasonably short.
pub type NewAccountsRemote = RemoteDatabaseSource<NewAccountsQueryBehaviour>;

pub struct Properties;

impl Named for Properties {
    fn name() -> String {
        "newAccounts".into()
    }
}

impl ChartProperties for Properties {
    type Resolution = NaiveDate;

    fn chart_type() -> ChartType {
        ChartType::Line
    }
}

define_and_impl_resolution_properties!(
    define_and_impl: {
        WeeklyProperties: Week,
        MonthlyProperties: Month,
        YearlyProperties: Year,
    },
    base_impl: Properties
);

pub type NewAccounts = DirectVecLocalDbChartSource<NewAccountsRemote, Batch30Days, Properties>;
pub type NewAccountsInt = MapParseTo<StripExt<NewAccounts>, i64>;
pub type NewAccountsWeekly = DirectVecLocalDbChartSource<
    MapToString<SumLowerResolution<NewAccountsInt, Week>>,
    Batch30Weeks,
    WeeklyProperties,
>;
pub type NewAccountsMonthly = DirectVecLocalDbChartSource<
    MapToString<SumLowerResolution<NewAccountsInt, Month>>,
    Batch36Months,
    MonthlyProperties,
>;
pub type NewAccountsMonthlyInt = MapParseTo<StripExt<NewAccountsMonthly>, i64>;
pub type NewAccountsYearly = DirectVecLocalDbChartSource<
    MapToString<SumLowerResolution<NewAccountsMonthlyInt, Year>>,
    Batch30Years,
    YearlyProperties,
>;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::simple_test::{
        ranged_test_chart_with_migration_variants, simple_test_chart_with_migration_variants,
    };

    #[tokio::test]
    #[ignore = "needs database to run"]
    async fn update_new_accounts() {
        simple_test_chart_with_migration_variants::<NewAccounts>(
            "update_new_accounts",
            vec![
                ("2022-11-09", "1"),
                ("2022-11-10", "3"),
                ("2022-11-11", "4"),
                ("2023-03-01", "2"),
            ],
        )
        .await;
    }

    #[tokio::test]
    #[ignore = "needs database to run"]
    async fn update_new_accounts_weekly() {
        simple_test_chart_with_migration_variants::<NewAccountsWeekly>(
            "update_new_accounts_weekly",
            vec![("2022-11-07", "8"), ("2023-02-27", "2")],
        )
        .await;
    }

    #[tokio::test]
    #[ignore = "needs database to run"]
    async fn update_new_accounts_monthly() {
        simple_test_chart_with_migration_variants::<NewAccountsMonthly>(
            "update_new_accounts_monthly",
            vec![("2022-11-01", "8"), ("2023-03-01", "2")],
        )
        .await;
    }

    #[tokio::test]
    #[ignore = "needs database to run"]
    async fn update_new_accounts_yearly() {
        simple_test_chart_with_migration_variants::<NewAccountsYearly>(
            "update_new_accounts_yearly",
            vec![("2022-01-01", "8"), ("2023-01-01", "2")],
        )
        .await;
    }

    #[tokio::test]
    #[ignore = "needs database to run"]
    async fn ranged_update_new_accounts() {
        ranged_test_chart_with_migration_variants::<NewAccounts>(
            "ranged_update_new_accounts",
            vec![
                ("2022-11-09", "1"),
                ("2022-11-10", "3"),
                // only half of the tx's should be detected; others were created
                // later than update time
                ("2022-11-11", "2"),
            ],
            "2022-11-09".parse().unwrap(),
            "2022-11-11".parse().unwrap(),
            Some("2022-11-11T14:00:00".parse().unwrap()),
        )
        .await;
    }
}
