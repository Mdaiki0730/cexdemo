-- Create balances table
CREATE TABLE IF NOT EXISTS balances (
    user_id VARCHAR(255) NOT NULL,
    currency VARCHAR(10) NOT NULL,
    balance DECIMAL(30, 8) NOT NULL DEFAULT 0,
    locked DECIMAL(30, 8) NOT NULL DEFAULT 0,
    PRIMARY KEY (user_id, currency)
);

-- Create orders table
CREATE TABLE IF NOT EXISTS orders (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id VARCHAR(255) NOT NULL,
    pair VARCHAR(20) NOT NULL,
    order_type VARCHAR(10) NOT NULL CHECK (order_type IN ('buy', 'sell')),
    rate DECIMAL(30, 8) NOT NULL,
    amount DECIMAL(30, 8) NOT NULL,
    remaining_amount DECIMAL(30, 8) NOT NULL,
    status VARCHAR(20) NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'partially_filled', 'filled', 'cancelled')),
    executed_at TIMESTAMP WITH TIME ZONE,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_orders_pair_status ON orders(pair, status);
CREATE INDEX idx_orders_user_id ON orders(user_id);
CREATE INDEX idx_orders_created_at ON orders(created_at);
CREATE INDEX idx_orders_executed_at ON orders(executed_at);

-- Insert initial balance for a test user (no authentication in MVC implementation)
INSERT INTO balances (user_id, currency, balance) VALUES 
    ('default_user', 'JPY', 1000000.0),
    ('default_user', 'BTC', 1.0)
ON CONFLICT (user_id, currency) DO NOTHING;

