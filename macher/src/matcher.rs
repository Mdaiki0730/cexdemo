use shared::{MatchedOrder, OrderMessage, OrderType};
use rust_decimal::Decimal;
use std::collections::{BTreeMap, VecDeque};
use chrono::Utc;

#[derive(Debug, Clone)]
struct OrderQueueEntry {
    order_id: uuid::Uuid,
    #[allow(dead_code)]
    user_id: String,
    amount: Decimal,
    #[allow(dead_code)]
    created_at: chrono::DateTime<Utc>,
}

pub struct OrderMatcher {
    // Price -> Queue of orders (sorted by time)
    bids: BTreeMap<Decimal, VecDeque<OrderQueueEntry>>, // Buy orders, sorted by price descending
    asks: BTreeMap<Decimal, VecDeque<OrderQueueEntry>>, // Sell orders, sorted by price ascending
}

impl OrderMatcher {
    pub fn new() -> Self {
        Self {
            bids: BTreeMap::new(),
            asks: BTreeMap::new(),
        }
    }

    pub async fn match_order(&mut self, order: OrderMessage) -> Vec<MatchedOrder> {
        let mut matched_orders = Vec::new();
        let mut remaining_amount = order.amount;

        match order.order_type {
            OrderType::Buy => {
                // Try to match buy order against asks (sell orders)
                while remaining_amount > Decimal::ZERO {
                    // Get best ask (lowest price)
                    if let Some(mut entry) = self.asks.first_entry() {
                        let best_ask_price = *entry.key();
                        if best_ask_price <= order.rate {
                            // Can match
                            let ask_queue = entry.get_mut();
                            if let Some(ask_order) = ask_queue.front_mut() {
                                let match_amount = remaining_amount.min(ask_order.amount);
                                
                                // Create matched order
                                let matched = MatchedOrder {
                                    buy_order_id: order.order_id,
                                    sell_order_id: ask_order.order_id,
                                    pair: order.pair.clone(),
                                    rate: best_ask_price,
                                    amount: match_amount,
                                    buy_fee: Decimal::ZERO, // No fee for MVC implementation
                                    sell_fee: Decimal::ZERO,
                                    created_at: Utc::now(),
                                };
                                matched_orders.push(matched);

                                // Update amounts
                                remaining_amount -= match_amount;
                                ask_order.amount -= match_amount;

                                // Remove order if fully filled
                                if ask_order.amount <= Decimal::ZERO {
                                    ask_queue.pop_front();
                                    if ask_queue.is_empty() {
                                        entry.remove();
                                    }
                                }
                            } else {
                                break;
                            }
                        } else {
                            // Best ask is higher than buy price, can't match
                            break;
                        }
                    } else {
                        // No asks available
                        break;
                    }
                }

                // If there's remaining amount, add to bids
                if remaining_amount > Decimal::ZERO {
                    self.bids
                        .entry(order.rate)
                        .or_insert_with(VecDeque::new)
                        .push_back(OrderQueueEntry {
                            order_id: order.order_id,
                            user_id: order.user_id,
                            amount: remaining_amount,
                            created_at: order.created_at,
                        });
                }
            }
            OrderType::Sell => {
                // Try to match sell order against bids (buy orders)
                while remaining_amount > Decimal::ZERO {
                    // Get best bid (highest price)
                    if let Some(mut entry) = self.bids.last_entry() {
                        let best_bid_price = *entry.key();
                        if best_bid_price >= order.rate {
                            // Can match
                            let bid_queue = entry.get_mut();
                            if let Some(bid_order) = bid_queue.front_mut() {
                                let match_amount = remaining_amount.min(bid_order.amount);
                                
                                // Create matched order
                                let matched = MatchedOrder {
                                    buy_order_id: bid_order.order_id,
                                    sell_order_id: order.order_id,
                                    pair: order.pair.clone(),
                                    rate: best_bid_price,
                                    amount: match_amount,
                                    buy_fee: Decimal::ZERO, // No fee for MVC implementation
                                    sell_fee: Decimal::ZERO,
                                    created_at: Utc::now(),
                                };
                                matched_orders.push(matched);

                                // Update amounts
                                remaining_amount -= match_amount;
                                bid_order.amount -= match_amount;

                                // Remove order if fully filled
                                if bid_order.amount <= Decimal::ZERO {
                                    bid_queue.pop_front();
                                    if bid_queue.is_empty() {
                                        entry.remove();
                                    }
                                }
                            } else {
                                break;
                            }
                        } else {
                            // Best bid is lower than sell price, can't match
                            break;
                        }
                    } else {
                        // No bids available
                        break;
                    }
                }

                // If there's remaining amount, add to asks
                if remaining_amount > Decimal::ZERO {
                    self.asks
                        .entry(order.rate)
                        .or_insert_with(VecDeque::new)
                        .push_back(OrderQueueEntry {
                            order_id: order.order_id,
                            user_id: order.user_id,
                            amount: remaining_amount,
                            created_at: order.created_at,
                        });
                }
            }
        }

        matched_orders
    }
}

