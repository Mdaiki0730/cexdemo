use sea_orm::{DatabaseConnection, EntityTrait, QueryFilter, ColumnTrait, TransactionTrait, ActiveModelTrait, Set, FromQueryResult, QueryOrder, QuerySelect};
use shared::{Balance, BalanceActiveModel, BalanceColumn, OrderEntity, OrderActiveModel, OrderColumn};
use shared::{Order, OrderStatus, OrderType};
use rust_decimal::Decimal;
use uuid::Uuid;

#[derive(FromQueryResult)]
struct BalanceRow {
    #[allow(dead_code)]
    user_id: String,
    #[allow(dead_code)]
    currency: String,
    balance: Decimal,
    locked: Decimal,
}

pub async fn check_and_lock_balance(
    db: &DatabaseConnection,
    user_id: &str,
    currency: &str,
    required_amount: Decimal,
) -> anyhow::Result<bool> {
    let txn = db.begin().await?;

    // Use raw SQL for FOR UPDATE lock
    let stmt = sea_orm::Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        r#"SELECT user_id, currency, balance, locked FROM balances 
           WHERE user_id = $1 AND currency = $2 FOR UPDATE"#,
        vec![sea_orm::Value::String(Some(Box::new(user_id.to_string()))), sea_orm::Value::String(Some(Box::new(currency.to_string())))],
    );

    let result: Option<BalanceRow> = BalanceRow::find_by_statement(stmt)
        .one(&txn)
        .await?;

    match result {
        Some(balance) => {
            let available = balance.balance - balance.locked;
            if available >= required_amount {
                // Lock the amount using entity update
                let balance_entity = Balance::find()
                    .filter(BalanceColumn::UserId.eq(user_id))
                    .filter(BalanceColumn::Currency.eq(currency))
                    .one(&txn)
                    .await?
                    .ok_or_else(|| anyhow::anyhow!("Balance not found"))?;

                let mut balance: BalanceActiveModel = balance_entity.into();
                balance.locked = Set(balance.locked.as_ref() + required_amount);
                balance.update(&txn).await?;

                txn.commit().await?;
                Ok(true)
            } else {
                txn.rollback().await?;
                Ok(false)
            }
        }
        None => {
            txn.rollback().await?;
            Ok(false)
        }
    }
}

pub async fn create_order_record(
    db: &DatabaseConnection,
    order_id: Uuid,
    user_id: &str,
    pair: &str,
    order_type: &OrderType,
    rate: Decimal,
    amount: Decimal,
) -> anyhow::Result<()> {
    let order_type_str = match order_type {
        OrderType::Buy => "buy",
        OrderType::Sell => "sell",
    };

    let order = OrderActiveModel {
        id: Set(order_id),
        user_id: Set(user_id.to_string()),
        pair: Set(pair.to_string()),
        order_type: Set(order_type_str.to_string()),
        rate: Set(rate),
        amount: Set(amount),
        remaining_amount: Set(amount),
        status: Set("pending".to_string()),
        executed_at: Set(None),
        created_at: Set(chrono::Utc::now()),
        updated_at: Set(chrono::Utc::now()),
    };

    order.insert(db).await?;

    Ok(())
}

pub async fn get_pending_orders(db: &DatabaseConnection, pair: &str) -> anyhow::Result<Vec<Order>> {
    let orders = OrderEntity::find()
        .filter(OrderColumn::Pair.eq(pair))
        .filter(
            OrderColumn::Status.is_in(vec!["pending", "partially_filled"])
        )
        .order_by(OrderColumn::Rate, sea_orm::Order::Desc)
        .order_by(OrderColumn::CreatedAt, sea_orm::Order::Asc)
        .all(db)
        .await?;

    let result = orders
        .into_iter()
        .map(|o| {
            let order_type = match o.order_type.as_str() {
                "buy" => OrderType::Buy,
                "sell" => OrderType::Sell,
                _ => unreachable!(),
            };
            let status = match o.status.as_str() {
                "pending" => OrderStatus::Pending,
                "partially_filled" => OrderStatus::PartiallyFilled,
                "filled" => OrderStatus::Filled,
                "cancelled" => OrderStatus::Cancelled,
                _ => OrderStatus::Pending,
            };

            Order {
                id: o.id,
                pair: o.pair,
                order_type,
                rate: o.rate,
                amount: o.amount,
                remaining_amount: o.remaining_amount,
                status,
                created_at: o.created_at,
            }
        })
        .collect();

    Ok(result)
}

pub async fn get_executed_orders(
    db: &DatabaseConnection,
    limit: i64,
    offset: i64,
) -> anyhow::Result<Vec<Order>> {
    let orders = OrderEntity::find()
        .filter(OrderColumn::ExecutedAt.is_not_null())
        .order_by(OrderColumn::ExecutedAt, sea_orm::Order::Desc)
        .limit(limit as u64)
        .offset(offset as u64)
        .all(db)
        .await?;

    let result = orders
        .into_iter()
        .map(|o| {
            let order_type = match o.order_type.as_str() {
                "buy" => OrderType::Buy,
                "sell" => OrderType::Sell,
                _ => unreachable!(),
            };
            let status = match o.status.as_str() {
                "pending" => OrderStatus::Pending,
                "partially_filled" => OrderStatus::PartiallyFilled,
                "filled" => OrderStatus::Filled,
                "cancelled" => OrderStatus::Cancelled,
                _ => OrderStatus::Pending,
            };

            Order {
                id: o.id,
                pair: o.pair,
                order_type,
                rate: o.rate,
                amount: o.amount,
                remaining_amount: o.remaining_amount,
                status,
                created_at: o.created_at,
            }
        })
        .collect();

    Ok(result)
}
