# CEXDEMO

A CEX order trading system MVC implementation, inspired by Coincheck API.

## Architecture Overview

- **API Server**: REST API endpoints, balance checking/locking, order sending to Kafka
- **Order Matching**: Consumes orders from Kafka and matches them with price-time priority
- **Settlement Layer**: Processes matched orders, records them in DB and updates balances
- **Message Queue**: Apache Kafka
- **Database**: PostgreSQL

## Setup

### 1. Start Infrastructure with Docker Compose

```bash
docker-compose up -d
```

This will start:
- PostgreSQL (port 5432)
- Zookeeper (port 2181)
- Kafka (port 9092)

### 2. Database Initialization

Migrations are automatically executed when the PostgreSQL container starts.
Initial balances:
- JPY: 1,000,000
- BTC: 1.0

### 3. Build

```bash
cargo build --release
```

## Running

### 1. Start Settlement Layer

```bash
cd settlement
cargo run --release
```

### 2. Start Order Matching

In another terminal:

```bash
cd macher
cargo run --release
```

### 3. Start API Server

In another terminal:

```bash
cd server
cargo run --release
```

The API Server will start at `http://localhost:3000`.

## API Endpoints

### 1. Create Order

```bash
POST /api/exchange/orders
Content-Type: application/json

{
  "pair": "btc_jpy",
  "order_type": "buy",
  "rate": 5000000,
  "amount": 0.01
}
```

### 2. Get Order Book

```bash
GET /api/order_books
```

### 3. Get Executed Orders

```bash
GET /api/exchange/orders/executed?limit=100&offset=0
```

## Environment Variables

- `DATABASE_URL`: PostgreSQL connection string (default: `postgresql://cexuser:cexpass@localhost:5432/cexdb`)

## Notes

- Authentication is not included as this is an MVC implementation
- Default user ID: `default_user`
- Supported pairs: `btc_jpy` only

## System Architecture

### Workflow

1. Client sends order to API Server
2. API Server checks and locks balance
3. Order is recorded in DB
4. Order is sent to Kafka `orders` topic
5. Order matching consumes orders and executes matching algorithm
6. Matched orders are sent to Kafka `matched-orders` topic
7. Settlement Layer processes matched orders
8. Orders are updated with `executed_at` timestamp and balances are updated

### Matching Algorithm

- **Buy orders**: Compared against asks (sell orders) lowest price, executed if conditions are met
- **Sell orders**: Compared against bids (buy orders) highest price, executed if conditions are met
- Partial execution supported
- Price-time priority order
- **Partition separation by pair**: Orders are partitioned by trading pair (e.g., `btc_jpy`) using Kafka message keys. This ensures:
  - Orders for the same pair are processed in the same partition
  - Order guarantee is maintained within each pair
  - Different pairs can be processed in parallel across partitions 

### Component Diagram

```mermaid
%%{init: {'themeVariables': {'fontSize':'12px'}}}%%
graph TB
    Client[Client] -->|HTTP| API[API Server]
    API -->|Check/Lock| DB[(PostgreSQL)]
    API -->|Create Order| DB
    API -->|Send Order| Kafka1[Kafka: orders]
    
    Kafka1 -->|Consume| Matcher[Order Matching]
    Matcher -->|Match Orders| Matcher
    Matcher -->|Send Matched| Kafka2[Kafka: matched-orders]
    
    Kafka2 -->|Consume| Settlement[Settlement Layer]
    Settlement -->|Update Orders| DB
    Settlement -->|Update Balances| DB
    
    style Client fill:#e1f5ff
    style API fill:#fff4e1
    style Matcher fill:#ffe1f5
    style Settlement fill:#e1ffe1
    style DB fill:#f0f0f0
    style Kafka1 fill:#fff9e1
    style Kafka2 fill:#fff9e1
```

## Database Schema (ER Diagram)

```mermaid
%%{init: {'themeVariables': {'fontSize':'12px'}}}%%
erDiagram
    balances ||--o{ orders : "user_id"
    
    balances {
        varchar user_id PK
        varchar currency PK
        decimal balance
        decimal locked
    }
    
    orders {
        uuid id PK
        varchar user_id
        varchar pair
        varchar order_type
        decimal rate
        decimal amount
        decimal remaining_amount
        varchar status
        timestamp executed_at
        timestamp created_at
        timestamp updated_at
    }
```