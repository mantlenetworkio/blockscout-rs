// SPDX-License-Identifier: LicenseRef-Blockscout

use std::ops::Range;

use chrono::{DateTime, TimeDelta, Utc};
use sea_orm::{
    ColumnTrait, QueryFilter,
    sea_query::{Expr, SimpleExpr},
};

/// `[filter_24h_until - 24h, filter_24h_until]` (both ends inclusive).
///
/// The timestamp expression is compared as-is, so an index on it can be used.
pub fn interval_24h_filter(
    timestamp_expr: SimpleExpr,
    filter_24h_until: DateTime<Utc>,
) -> SimpleExpr {
    let filter_24h_since = filter_24h_until
        .checked_sub_signed(TimeDelta::hours(24))
        .unwrap_or(DateTime::<Utc>::MIN_UTC);
    Expr::expr(timestamp_expr.clone())
        .gte(filter_24h_since)
        .and(Expr::expr(timestamp_expr).lte(filter_24h_until))
}

pub fn datetime_range_filter<Q: QueryFilter, C: ColumnTrait>(
    query: Q,
    column: C,
    range: &Range<DateTime<Utc>>,
) -> Q {
    query
        .filter(column.lt(range.end))
        .filter(column.gte(range.start))
}
