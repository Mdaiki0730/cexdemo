use sea_orm::{DatabaseConnection, EntityTrait, QueryFilter, ColumnTrait, TransactionTrait, ActiveModelTrait, Set};
use shared::MatchedOrder;
use shared::{Balance, BalanceActiveModel, BalanceColumn, OrderEntity, OrderActiveModel};
use rust_decimal::Decimal;

pub struct SettlementDB {
    db: DatabaseConnection,
}

impl SettlementDB {
    pub async fn new(database_url: &str) -> anyhow::Result<Self> {
        let db = sea_orm::Database::connect(database_url).await?;
        Ok(Self { db })
    }

    pub async fn settle_order(&self, matched: MatchedOrder) -> anyhow::Result<()> {
        let txn = self.db.begin().await?;

        // Get order details to find user_id
        let buy_order_model = OrderEntity::find_by_id(matched.buy_order_id)
            .one(&txn)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Buy order not found"))?;

        let sell_order_model = OrderEntity::find_by_id(matched.sell_order_id)
            .one(&txn)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Sell order not found"))?;

        let buy_user_id = buy_order_model.user_id.clone();
        let sell_user_id = sell_order_model.user_id.clone();

        let executed_at = chrono::Utc::now();

        // Update buy order
        let buy_total = matched.amount * matched.rate;
        let mut buy_order: OrderActiveModel = buy_order_model.into();
        let new_remaining = buy_order.remaining_amount.as_ref() - matched.amount;
        buy_order.remaining_amount = Set(new_remaining);
        buy_order.status = Set(if new_remaining <= Decimal::ZERO {
            "filled".to_string()
        } else {
            "partially_filled".to_string()
        });
        // Set executed_at when order is executed (filled or partially filled)
        if buy_order.executed_at.as_ref().is_none() {
            buy_order.executed_at = Set(Some(executed_at));
        }
        buy_order.updated_at = Set(chrono::Utc::now());
        buy_order.update(&txn).await?;

        // Update sell order
        let mut sell_order: OrderActiveModel = sell_order_model.into();
        let new_remaining = sell_order.remaining_amount.as_ref() - matched.amount;
        sell_order.remaining_amount = Set(new_remaining);
        sell_order.status = Set(if new_remaining <= Decimal::ZERO {
            "filled".to_string()
        } else {
            "partially_filled".to_string()
        });
        // Set executed_at when order is executed (filled or partially filled)
        if sell_order.executed_at.as_ref().is_none() {
            sell_order.executed_at = Set(Some(executed_at));
        }
        sell_order.updated_at = Set(chrono::Utc::now());
        sell_order.update(&txn).await?;

        // Update balances for buy side (spend JPY, receive BTC)
        // Unlock and deduct JPY
        let buy_balance = Balance::find()
            .filter(BalanceColumn::UserId.eq(buy_user_id.as_str()))
            .filter(BalanceColumn::Currency.eq("JPY"))
            .one(&txn)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Buy balance not found"))?;

        let mut buy_balance: BalanceActiveModel = buy_balance.into();
        buy_balance.locked = Set(buy_balance.locked.as_ref() - buy_total);
        buy_balance.balance = Set(buy_balance.balance.as_ref() - buy_total);
        buy_balance.update(&txn).await?;

        // Add BTC
        let buy_btc_balance = Balance::find()
            .filter(BalanceColumn::UserId.eq(buy_user_id.as_str()))
            .filter(BalanceColumn::Currency.eq("BTC"))
            .one(&txn)
            .await?;

        match buy_btc_balance {
            Some(btc_balance) => {
                let mut btc_balance: BalanceActiveModel = btc_balance.into();
                btc_balance.balance = Set(btc_balance.balance.as_ref() + matched.amount);
                btc_balance.update(&txn).await?;
            }
            None => {
                let new_btc_balance = BalanceActiveModel {
                    user_id: Set(buy_user_id.clone()),
                    currency: Set("BTC".to_string()),
                    balance: Set(matched.amount),
                    locked: Set(Decimal::ZERO),
                };
                new_btc_balance.insert(&txn).await?;
            }
        }

        // Update balances for sell side (spend BTC, receive JPY)
        // Unlock and deduct BTC
        let sell_balance = Balance::find()
            .filter(BalanceColumn::UserId.eq(sell_user_id.as_str()))
            .filter(BalanceColumn::Currency.eq("BTC"))
            .one(&txn)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Sell balance not found"))?;

        let mut sell_balance: BalanceActiveModel = sell_balance.into();
        sell_balance.locked = Set(sell_balance.locked.as_ref() - matched.amount);
        sell_balance.balance = Set(sell_balance.balance.as_ref() - matched.amount);
        sell_balance.update(&txn).await?;

        // Add JPY
        let sell_receive = matched.amount * matched.rate;
        let sell_jpy_balance = Balance::find()
            .filter(BalanceColumn::UserId.eq(sell_user_id.as_str()))
            .filter(BalanceColumn::Currency.eq("JPY"))
            .one(&txn)
            .await?;

        match sell_jpy_balance {
            Some(jpy_balance) => {
                let mut jpy_balance: BalanceActiveModel = jpy_balance.into();
                jpy_balance.balance = Set(jpy_balance.balance.as_ref() + sell_receive);
                jpy_balance.update(&txn).await?;
            }
            None => {
                let new_jpy_balance = BalanceActiveModel {
                    user_id: Set(sell_user_id.clone()),
                    currency: Set("JPY".to_string()),
                    balance: Set(sell_receive),
                    locked: Set(Decimal::ZERO),
                };
                new_jpy_balance.insert(&txn).await?;
            }
        }

        txn.commit().await?;

        Ok(())
    }
}
