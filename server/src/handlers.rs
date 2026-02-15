use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::Json,
};
use shared::{
    CreateOrderRequest, Order, OrderBook, OrderBookEntry, OrderMessage, OrderType,
};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::Utc;

use crate::{db, AppState};

#[derive(Deserialize)]
pub struct ExecutedOrderQuery {
    limit: Option<i64>,
    offset: Option<i64>,
}

#[derive(Serialize)]
pub struct CreateOrderResponse {
    pub order_id: Uuid,
    pub success: bool,
}

pub async fn create_order(
    State(state): State<AppState>,
    Json(req): Json<CreateOrderRequest>,
) -> Result<Json<CreateOrderResponse>, (StatusCode, String)> {
    // Validate request
    if req.rate <= Decimal::ZERO || req.amount <= Decimal::ZERO {
        return Err((
            StatusCode::BAD_REQUEST,
            "Rate and amount must be positive".to_string(),
        ));
    }

    if req.pair != "btc_jpy" {
        return Err((
            StatusCode::BAD_REQUEST,
            "Only btc_jpy pair is supported".to_string(),
        ));
    }

    // No authentication in MVC implementation, use default user
    let user_id = "default_user";
    let order_id = Uuid::new_v4();

    // Check and lock balance
    let (currency, required_amount) = match req.order_type {
        OrderType::Buy => ("JPY", req.rate * req.amount),
        OrderType::Sell => ("BTC", req.amount),
    };

    let has_balance = db::check_and_lock_balance(&state.db, user_id, currency, required_amount)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Database error: {}", e)))?;

    if !has_balance {
        return Err((
            StatusCode::BAD_REQUEST,
            format!("Insufficient balance for {}", currency),
        ));
    }

    // Create order record in DB
    db::create_order_record(
        &state.db,
        order_id,
        user_id,
        &req.pair,
        &req.order_type,
        req.rate,
        req.amount,
    )
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Database error: {}", e)))?;

    // Send order to Kafka
    let order_message = OrderMessage {
        order_id,
        user_id: user_id.to_string(),
        pair: req.pair.clone(),
        order_type: req.order_type.clone(),
        rate: req.rate,
        amount: req.amount,
        created_at: Utc::now(),
    };

    state
        .kafka_producer
        .send_order(order_message)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Kafka error: {}", e)))?;

    Ok(Json(CreateOrderResponse {
        order_id,
        success: true,
    }))
}

pub async fn get_order_books(
    State(state): State<AppState>,
) -> Result<Json<OrderBook>, (StatusCode, String)> {
    // Get pending orders from DB
    let orders = db::get_pending_orders(&state.db, "btc_jpy")
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Database error: {}", e)))?;

    // Separate into bids and asks
    let mut bids: Vec<OrderBookEntry> = Vec::new();
    let mut asks: Vec<OrderBookEntry> = Vec::new();

    for order in orders {
        let entry = OrderBookEntry {
            price: order.rate,
            amount: order.remaining_amount,
        };

        match order.order_type {
            OrderType::Buy => bids.push(entry),
            OrderType::Sell => asks.push(entry),
        }
    }

    // Sort bids descending (highest first), asks ascending (lowest first)
    bids.sort_by(|a, b| b.price.cmp(&a.price));
    asks.sort_by(|a, b| a.price.cmp(&b.price));

    Ok(Json(OrderBook {
        pair: "btc_jpy".to_string(),
        bids,
        asks,
    }))
}

pub async fn get_executed_orders(
    State(state): State<AppState>,
    Query(params): Query<ExecutedOrderQuery>,
) -> Result<Json<Vec<Order>>, (StatusCode, String)> {
    let limit = params.limit.unwrap_or(100).min(1000);
    let offset = params.offset.unwrap_or(0);

    let orders = db::get_executed_orders(&state.db, limit, offset)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Database error: {}", e)))?;

    Ok(Json(orders))
}

