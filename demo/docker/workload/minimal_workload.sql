-- Minimal workload for pg-loggrep demo
-- This file contains sample SQL statements to generate comprehensive log activity

-- Enable timing for all statements
\timing on

-- Create a test table for users
CREATE TABLE IF NOT EXISTS demo_users (
    id SERIAL PRIMARY KEY,
    username VARCHAR(50) NOT NULL UNIQUE,
    email VARCHAR(100),
    first_name VARCHAR(50),
    last_name VARCHAR(50),
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Create index for better query performance and logging
CREATE INDEX IF NOT EXISTS idx_demo_users_username ON demo_users(username);
CREATE INDEX IF NOT EXISTS idx_demo_users_email ON demo_users(email);
CREATE INDEX IF NOT EXISTS idx_demo_users_created_at ON demo_users(created_at);

-- Insert sample user data
INSERT INTO demo_users (username, email, first_name, last_name) VALUES
    ('alice_smith', 'alice.smith@example.com', 'Alice', 'Smith'),
    ('bob_jones', 'bob.jones@example.com', 'Bob', 'Jones'),
    ('charlie_brown', 'charlie.brown@example.com', 'Charlie', 'Brown'),
    ('diana_wilson', 'diana.wilson@example.com', 'Diana', 'Wilson'),
    ('eve_davis', 'eve.davis@example.com', 'Eve', 'Davis')
ON CONFLICT (username) DO NOTHING;

-- Create orders table with foreign key relationship
CREATE TABLE IF NOT EXISTS demo_orders (
    id SERIAL PRIMARY KEY,
    user_id INTEGER REFERENCES demo_users(id) ON DELETE CASCADE,
    order_number VARCHAR(20) UNIQUE NOT NULL,
    amount DECIMAL(10,2) NOT NULL CHECK (amount > 0),
    status VARCHAR(20) DEFAULT 'pending' CHECK (status IN ('pending', 'processing', 'completed', 'cancelled', 'refunded')),
    order_date DATE DEFAULT CURRENT_DATE,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Create indexes for orders table
CREATE INDEX IF NOT EXISTS idx_demo_orders_user_id ON demo_orders(user_id);
CREATE INDEX IF NOT EXISTS idx_demo_orders_status ON demo_orders(status);
CREATE INDEX IF NOT EXISTS idx_demo_orders_order_date ON demo_orders(order_date);
CREATE INDEX IF NOT EXISTS idx_demo_orders_amount ON demo_orders(amount);

-- Insert sample order data
INSERT INTO demo_orders (user_id, order_number, amount, status, order_date) VALUES
    (1, 'ORD-2024-001', 125.50, 'completed', '2024-01-15'),
    (2, 'ORD-2024-002', 89.99, 'completed', '2024-01-16'),
    (3, 'ORD-2024-003', 234.75, 'processing', '2024-01-17'),
    (1, 'ORD-2024-004', 67.25, 'completed', '2024-01-18'),
    (4, 'ORD-2024-005', 156.00, 'pending', '2024-01-19'),
    (5, 'ORD-2024-006', 298.50, 'cancelled', '2024-01-20'),
    (2, 'ORD-2024-007', 45.99, 'completed', '2024-01-21'),
    (3, 'ORD-2024-008', 178.25, 'refunded', '2024-01-22')
ON CONFLICT (order_number) DO NOTHING;

-- Create products table for more complex queries
CREATE TABLE IF NOT EXISTS demo_products (
    id SERIAL PRIMARY KEY,
    name VARCHAR(100) NOT NULL,
    category VARCHAR(50),
    price DECIMAL(10,2) NOT NULL CHECK (price >= 0),
    stock_quantity INTEGER DEFAULT 0 CHECK (stock_quantity >= 0),
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Insert sample products
INSERT INTO demo_products (name, category, price, stock_quantity) VALUES
    ('Laptop Pro 15"', 'Electronics', 1299.99, 25),
    ('Wireless Mouse', 'Electronics', 29.99, 150),
    ('Office Chair', 'Furniture', 199.99, 45),
    ('Coffee Mug', 'Kitchen', 12.99, 200),
    ('Notebook Set', 'Office', 15.99, 75)
ON CONFLICT DO NOTHING;

-- Create order items table for many-to-many relationship
CREATE TABLE IF NOT EXISTS demo_order_items (
    id SERIAL PRIMARY KEY,
    order_id INTEGER REFERENCES demo_orders(id) ON DELETE CASCADE,
    product_id INTEGER REFERENCES demo_products(id),
    quantity INTEGER NOT NULL CHECK (quantity > 0),
    unit_price DECIMAL(10,2) NOT NULL CHECK (unit_price > 0),
    total_price DECIMAL(10,2) GENERATED ALWAYS AS (quantity * unit_price) STORED
);

-- Insert sample order items
INSERT INTO demo_order_items (order_id, product_id, quantity, unit_price) VALUES
    (1, 1, 1, 1299.99),
    (1, 2, 2, 29.99),
    (2, 3, 1, 199.99),
    (3, 4, 5, 12.99),
    (3, 5, 3, 15.99),
    (4, 2, 1, 29.99),
    (5, 1, 1, 1299.99)
ON CONFLICT DO NOTHING;

-- Sample queries that will generate various log entries

-- Simple SELECT queries
SELECT COUNT(*) as total_users FROM demo_users;
SELECT COUNT(*) as total_orders FROM demo_orders;
SELECT COUNT(*) as total_products FROM demo_products;

-- Aggregation queries
SELECT
    status,
    COUNT(*) as order_count,
    SUM(amount) as total_amount,
    AVG(amount) as average_amount,
    MIN(amount) as min_amount,
    MAX(amount) as max_amount
FROM demo_orders
GROUP BY status
ORDER BY total_amount DESC;

-- JOIN queries
SELECT
    u.username,
    u.email,
    COUNT(o.id) as total_orders,
    COALESCE(SUM(o.amount), 0) as total_spent,
    COALESCE(AVG(o.amount), 0) as average_order_value
FROM demo_users u
LEFT JOIN demo_orders o ON u.id = o.user_id
GROUP BY u.id, u.username, u.email
ORDER BY total_spent DESC;

-- Complex JOIN with multiple tables
SELECT
    u.username,
    o.order_number,
    o.status,
    o.amount as order_total,
    COUNT(oi.id) as item_count,
    STRING_AGG(p.name, ', ') as products
FROM demo_users u
JOIN demo_orders o ON u.id = o.user_id
LEFT JOIN demo_order_items oi ON o.id = oi.order_id
LEFT JOIN demo_products p ON oi.product_id = p.id
GROUP BY u.username, o.id, o.order_number, o.status, o.amount
ORDER BY o.created_at DESC;

-- Subquery examples
SELECT
    username,
    email,
    (SELECT COUNT(*) FROM demo_orders WHERE user_id = u.id) as order_count,
    (SELECT COALESCE(SUM(amount), 0) FROM demo_orders WHERE user_id = u.id AND status = 'completed') as completed_amount
FROM demo_users u
WHERE id IN (SELECT DISTINCT user_id FROM demo_orders WHERE amount > 100);

-- Window functions
SELECT
    username,
    order_number,
    amount,
    status,
    ROW_NUMBER() OVER (PARTITION BY u.id ORDER BY o.created_at DESC) as order_rank,
    SUM(amount) OVER (PARTITION BY u.id) as user_total_spent,
    LAG(amount) OVER (PARTITION BY u.id ORDER BY o.created_at) as previous_order_amount
FROM demo_users u
JOIN demo_orders o ON u.id = o.user_id
ORDER BY u.username, o.created_at DESC;

-- Date/time queries
SELECT
    DATE_TRUNC('month', order_date) as month,
    COUNT(*) as orders_count,
    SUM(amount) as monthly_revenue,
    AVG(amount) as avg_order_value
FROM demo_orders
WHERE order_date >= CURRENT_DATE - INTERVAL '6 months'
GROUP BY DATE_TRUNC('month', order_date)
ORDER BY month DESC;

-- Product analysis
SELECT
    p.category,
    COUNT(DISTINCT p.id) as product_count,
    COUNT(oi.id) as times_ordered,
    SUM(oi.quantity) as total_quantity_sold,
    SUM(oi.total_price) as total_revenue
FROM demo_products p
LEFT JOIN demo_order_items oi ON p.id = oi.product_id
GROUP BY p.category
ORDER BY total_revenue DESC NULLS LAST;

-- Update operations (will generate modification logs)
UPDATE demo_users
SET updated_at = CURRENT_TIMESTAMP
WHERE created_at < CURRENT_TIMESTAMP - INTERVAL '1 day';

UPDATE demo_orders
SET status = 'completed', updated_at = CURRENT_TIMESTAMP
WHERE status = 'processing' AND created_at < CURRENT_TIMESTAMP - INTERVAL '1 hour';

-- Delete operations (will generate deletion logs)
DELETE FROM demo_order_items
WHERE order_id IN (SELECT id FROM demo_orders WHERE status = 'cancelled');

-- Transaction example
BEGIN;
    INSERT INTO demo_users (username, email, first_name, last_name)
    VALUES ('test_user', 'test@example.com', 'Test', 'User');

    INSERT INTO demo_orders (user_id, order_number, amount, status)
    VALUES (
        (SELECT id FROM demo_users WHERE username = 'test_user'),
        'ORD-TEST-001',
        99.99,
        'pending'
    );
COMMIT;

-- Cleanup test data
DELETE FROM demo_users WHERE username = 'test_user';

-- Performance testing queries
EXPLAIN ANALYZE SELECT * FROM demo_orders WHERE amount > 100;
EXPLAIN ANALYZE SELECT u.*, o.* FROM demo_users u JOIN demo_orders o ON u.id = o.user_id;

-- Create a view for reporting
CREATE OR REPLACE VIEW user_order_summary AS
SELECT
    u.id as user_id,
    u.username,
    u.email,
    COUNT(o.id) as total_orders,
    COALESCE(SUM(CASE WHEN o.status = 'completed' THEN o.amount END), 0) as completed_revenue,
    COALESCE(AVG(CASE WHEN o.status = 'completed' THEN o.amount END), 0) as avg_completed_order,
    MAX(o.created_at) as last_order_date,
    CASE
        WHEN MAX(o.created_at) > CURRENT_TIMESTAMP - INTERVAL '30 days' THEN 'Active'
        WHEN MAX(o.created_at) > CURRENT_TIMESTAMP - INTERVAL '90 days' THEN 'Inactive'
        ELSE 'Dormant'
    END as customer_status
FROM demo_users u
LEFT JOIN demo_orders o ON u.id = o.user_id
GROUP BY u.id, u.username, u.email;

-- Query the view
SELECT * FROM user_order_summary ORDER BY completed_revenue DESC;

-- Drop the view
DROP VIEW IF EXISTS user_order_summary;

-- Final summary query
SELECT
    'Summary Report' as report_type,
    (SELECT COUNT(*) FROM demo_users) as total_users,
    (SELECT COUNT(*) FROM demo_orders) as total_orders,
    (SELECT COUNT(*) FROM demo_products) as total_products,
    (SELECT SUM(amount) FROM demo_orders WHERE status = 'completed') as total_revenue,
    CURRENT_TIMESTAMP as generated_at;

-- Disable timing
\timing off

-- End of workload
SELECT 'Workload completed successfully!' as status;
