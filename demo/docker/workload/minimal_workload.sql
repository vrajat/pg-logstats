-- Comprehensive PostgreSQL Workload for pg-logstats Demo
-- This workload generates diverse log patterns for realistic analysis
-- Expected runtime: 2-3 minutes, ~200-500 log entries

-- Enable timing and verbose output
\timing on
\set VERBOSITY verbose

-- =============================================================================
-- SECTION 1: SCHEMA CREATION AND SETUP
-- Tests: DDL operations, index creation, constraint handling
-- =============================================================================

\echo '=== Creating Schema and Tables ==='

-- Drop tables if they exist (will generate notices/errors in logs)
DROP TABLE IF EXISTS orders CASCADE;
DROP TABLE IF EXISTS products CASCADE;
DROP TABLE IF EXISTS users CASCADE;

-- Create users table
CREATE TABLE users (
    id SERIAL PRIMARY KEY,
    name VARCHAR(100) NOT NULL,
    email VARCHAR(150) UNIQUE NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    active BOOLEAN DEFAULT true,
    last_login TIMESTAMP,
    user_type VARCHAR(20) DEFAULT 'regular' CHECK (user_type IN ('regular', 'premium', 'admin'))
);

-- Create products table
CREATE TABLE products (
    id SERIAL PRIMARY KEY,
    name VARCHAR(200) NOT NULL,
    category VARCHAR(50) NOT NULL,
    price DECIMAL(10,2) NOT NULL CHECK (price >= 0),
    stock INTEGER DEFAULT 0 CHECK (stock >= 0),
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    description TEXT,
    is_active BOOLEAN DEFAULT true
);

-- Create orders table with foreign key relationships
CREATE TABLE orders (
    id SERIAL PRIMARY KEY,
    user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    product_id INTEGER NOT NULL REFERENCES products(id),
    quantity INTEGER NOT NULL CHECK (quantity > 0),
    order_date TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    status VARCHAR(20) DEFAULT 'pending' CHECK (status IN ('pending', 'processing', 'shipped', 'delivered', 'cancelled')),
    total_amount DECIMAL(10,2),
    shipping_address TEXT,
    notes TEXT
);

-- Create indexes for performance testing
CREATE INDEX idx_users_email ON users(email);
CREATE INDEX idx_users_created_at ON users(created_at);
CREATE INDEX idx_users_active ON users(active);
CREATE INDEX idx_products_category ON products(category);
CREATE INDEX idx_products_price ON products(price);
CREATE INDEX idx_orders_user_id ON orders(user_id);
CREATE INDEX idx_orders_product_id ON orders(product_id);
CREATE INDEX idx_orders_order_date ON orders(order_date);
CREATE INDEX idx_orders_status ON orders(status);

-- =============================================================================
-- SECTION 2: BULK DATA INSERTION
-- Tests: INSERT operations, bulk loading, constraint validation
-- =============================================================================

\echo '=== Inserting Sample Data ==='

-- Insert users (1000 records)
INSERT INTO users (name, email, created_at, active, user_type, last_login)
SELECT
    'User ' || generate_series,
    'user' || generate_series || '@example.com',
    CURRENT_TIMESTAMP - (random() * INTERVAL '365 days'),
    CASE WHEN random() < 0.9 THEN true ELSE false END,
    CASE
        WHEN random() < 0.8 THEN 'regular'
        WHEN random() < 0.95 THEN 'premium'
        ELSE 'admin'
    END,
    CASE WHEN random() < 0.7 THEN CURRENT_TIMESTAMP - (random() * INTERVAL '30 days') ELSE NULL END
FROM generate_series(1, 1000);

-- Insert products (500 records)
INSERT INTO products (name, category, price, stock, description, is_active)
SELECT
    'Product ' || generate_series,
    CASE (generate_series % 10)
        WHEN 0 THEN 'Electronics'
        WHEN 1 THEN 'Clothing'
        WHEN 2 THEN 'Books'
        WHEN 3 THEN 'Home & Garden'
        WHEN 4 THEN 'Sports'
        WHEN 5 THEN 'Toys'
        WHEN 6 THEN 'Health'
        WHEN 7 THEN 'Automotive'
        WHEN 8 THEN 'Food'
        ELSE 'Other'
    END,
    (random() * 999 + 1)::DECIMAL(10,2),
    (random() * 100)::INTEGER,
    'Description for product ' || generate_series,
    CASE WHEN random() < 0.95 THEN true ELSE false END
FROM generate_series(1, 500);

-- Insert orders (2000 records)
INSERT INTO orders (user_id, product_id, quantity, order_date, status, total_amount, shipping_address)
SELECT
    (random() * 999 + 1)::INTEGER,
    (random() * 499 + 1)::INTEGER,
    (random() * 5 + 1)::INTEGER,
    CURRENT_TIMESTAMP - (random() * INTERVAL '180 days'),
    CASE
        WHEN random() < 0.3 THEN 'delivered'
        WHEN random() < 0.6 THEN 'shipped'
        WHEN random() < 0.8 THEN 'processing'
        WHEN random() < 0.95 THEN 'pending'
        ELSE 'cancelled'
    END,
    (random() * 500 + 10)::DECIMAL(10,2),
    'Address ' || generate_series || ', City, State'
FROM generate_series(1, 2000);

-- Update statistics for better query planning
ANALYZE users;
ANALYZE products;
ANALYZE orders;

-- =============================================================================
-- SECTION 3: FAST SELECT QUERIES
-- Tests: Simple queries, index usage, various WHERE conditions
-- =============================================================================

\echo '=== Running Fast SELECT Queries ==='

-- Simple primary key lookups (very fast)
SELECT * FROM users WHERE id = 1;
SELECT * FROM users WHERE id = 500;
SELECT * FROM products WHERE id = 250;

-- Index-based queries (fast)
SELECT * FROM users WHERE email = 'user100@example.com';
SELECT * FROM users WHERE active = true LIMIT 10;
SELECT * FROM products WHERE category = 'Electronics' LIMIT 5;
SELECT * FROM orders WHERE status = 'delivered' LIMIT 10;

-- Count queries with indexes
SELECT COUNT(*) FROM users WHERE active = true;
SELECT COUNT(*) FROM products WHERE is_active = true;
SELECT COUNT(*) FROM orders WHERE status = 'pending';

-- Date range queries
SELECT COUNT(*) FROM users WHERE created_at > CURRENT_DATE - INTERVAL '30 days';
SELECT COUNT(*) FROM orders WHERE order_date > CURRENT_DATE - INTERVAL '7 days';

-- =============================================================================
-- SECTION 4: SLOW SELECT QUERIES
-- Tests: Complex joins, aggregations, subqueries, sorting
-- =============================================================================

\echo '=== Running Slow SELECT Queries ==='

-- Complex join with aggregation (slow)
SELECT
    u.name,
    u.email,
    u.user_type,
    COUNT(o.id) as total_orders,
    SUM(o.total_amount) as total_spent,
    AVG(o.total_amount) as avg_order_value,
    MAX(o.order_date) as last_order_date
FROM users u
LEFT JOIN orders o ON u.id = o.user_id
WHERE u.active = true
GROUP BY u.id, u.name, u.email, u.user_type
HAVING COUNT(o.id) > 0
ORDER BY total_spent DESC
LIMIT 20;

-- Multi-table join with filtering (slow)
SELECT
    u.name as customer_name,
    p.name as product_name,
    p.category,
    o.quantity,
    o.total_amount,
    o.order_date,
    o.status
FROM orders o
JOIN users u ON o.user_id = u.id
JOIN products p ON o.product_id = p.id
WHERE o.order_date > CURRENT_DATE - INTERVAL '30 days'
    AND p.category IN ('Electronics', 'Books', 'Clothing')
    AND o.status != 'cancelled'
ORDER BY o.order_date DESC, o.total_amount DESC
LIMIT 50;

-- Subquery with aggregation (slow)
SELECT
    category,
    COUNT(*) as product_count,
    AVG(price) as avg_price,
    SUM(stock) as total_stock,
    (SELECT COUNT(*) FROM orders o JOIN products p2 ON o.product_id = p2.id WHERE p2.category = p.category) as orders_count
FROM products p
WHERE is_active = true
GROUP BY category
ORDER BY orders_count DESC;

-- Window functions (moderately slow)
SELECT
    name,
    category,
    price,
    ROW_NUMBER() OVER (PARTITION BY category ORDER BY price DESC) as price_rank,
    AVG(price) OVER (PARTITION BY category) as category_avg_price,
    price - AVG(price) OVER (PARTITION BY category) as price_diff_from_avg
FROM products
WHERE is_active = true
ORDER BY category, price_rank;

-- Complex aggregation with multiple conditions (slow)
SELECT
    DATE_TRUNC('month', o.order_date) as month,
    COUNT(*) as total_orders,
    COUNT(DISTINCT o.user_id) as unique_customers,
    SUM(o.total_amount) as revenue,
    AVG(o.total_amount) as avg_order_value,
    COUNT(CASE WHEN o.status = 'delivered' THEN 1 END) as delivered_orders,
    COUNT(CASE WHEN o.status = 'cancelled' THEN 1 END) as cancelled_orders
FROM orders o
WHERE o.order_date > CURRENT_DATE - INTERVAL '6 months'
GROUP BY DATE_TRUNC('month', o.order_date)
ORDER BY month DESC;

-- =============================================================================
-- SECTION 5: INSERT/UPDATE/DELETE OPERATIONS
-- Tests: DML operations, constraint violations, transaction handling
-- =============================================================================

\echo '=== Running DML Operations ==='

-- Insert new users
INSERT INTO users (name, email, user_type) VALUES
    ('Test User 1', 'test1@demo.com', 'regular'),
    ('Test User 2', 'test2@demo.com', 'premium'),
    ('Test User 3', 'test3@demo.com', 'admin');

-- Insert new products
INSERT INTO products (name, category, price, stock) VALUES
    ('Demo Product A', 'Electronics', 299.99, 50),
    ('Demo Product B', 'Books', 19.99, 100),
    ('Demo Product C', 'Clothing', 49.99, 25);

-- Update operations
UPDATE users SET last_login = CURRENT_TIMESTAMP WHERE user_type = 'admin';
UPDATE products SET stock = stock - 1 WHERE category = 'Electronics' AND stock > 0;
UPDATE orders SET status = 'shipped' WHERE status = 'processing' AND order_date < CURRENT_DATE - INTERVAL '2 days';

-- Batch update with join
UPDATE products
SET updated_at = CURRENT_TIMESTAMP
WHERE id IN (
    SELECT DISTINCT product_id
    FROM orders
    WHERE order_date > CURRENT_DATE - INTERVAL '7 days'
);

-- Delete operations
DELETE FROM orders WHERE status = 'cancelled' AND order_date < CURRENT_DATE - INTERVAL '90 days';
DELETE FROM users WHERE active = false AND last_login < CURRENT_DATE - INTERVAL '365 days';

-- =============================================================================
-- SECTION 6: PARAMETERIZED QUERIES AND VARIATIONS
-- Tests: Query normalization, parameter patterns
-- =============================================================================

\echo '=== Running Parameterized Query Patterns ==='

-- Simulate parameterized queries with different values
SELECT * FROM users WHERE id = 1;
SELECT * FROM users WHERE id = 2;
SELECT * FROM users WHERE id = 3;

SELECT * FROM products WHERE price > 100.00;
SELECT * FROM products WHERE price > 50.00;
SELECT * FROM products WHERE price > 200.00;

SELECT * FROM orders WHERE user_id = 1 AND status = 'delivered';
SELECT * FROM orders WHERE user_id = 2 AND status = 'delivered';
SELECT * FROM orders WHERE user_id = 3 AND status = 'pending';

-- IN clause variations
SELECT * FROM products WHERE category IN ('Electronics');
SELECT * FROM products WHERE category IN ('Electronics', 'Books');
SELECT * FROM products WHERE category IN ('Electronics', 'Books', 'Clothing');

-- LIKE patterns
SELECT * FROM users WHERE name LIKE 'User 1%';
SELECT * FROM users WHERE name LIKE 'User 2%';
SELECT * FROM products WHERE name LIKE '%Product A%';

-- =============================================================================
-- SECTION 7: INTENTIONAL ERRORS
-- Tests: Error handling, constraint violations, missing objects
-- =============================================================================

\echo '=== Generating Intentional Errors ==='

-- Table doesn't exist
SELECT * FROM non_existent_table;

-- Column doesn't exist
SELECT invalid_column FROM users;

-- Constraint violations
INSERT INTO users (name, email) VALUES ('Duplicate User', 'user1@example.com'); -- Duplicate email

-- Foreign key violation
INSERT INTO orders (user_id, product_id, quantity) VALUES (99999, 1, 1); -- Non-existent user

-- Check constraint violation
INSERT INTO products (name, category, price) VALUES ('Invalid Product', 'Test', -10.00); -- Negative price

-- Division by zero
SELECT 1/0;

-- Invalid date
SELECT '2023-13-45'::DATE;

-- Type conversion error
SELECT 'not_a_number'::INTEGER;

-- =============================================================================
-- SECTION 8: CONNECTION SIMULATION AND TIMING VARIATIONS
-- Tests: Connection patterns, session management
-- =============================================================================

\echo '=== Simulating Connection Patterns ==='

-- Simulate connection pooling by running queries with delays
SELECT pg_sleep(0.1); -- Short delay
SELECT COUNT(*) FROM users;

SELECT pg_sleep(0.05);
SELECT COUNT(*) FROM products;

SELECT pg_sleep(0.2); -- Longer delay
SELECT COUNT(*) FROM orders;

-- Simulate different session activities
SET work_mem = '4MB';
SELECT COUNT(*) FROM orders o JOIN users u ON o.user_id = u.id;

RESET work_mem;
SELECT COUNT(*) FROM products WHERE price > 100;

-- Transaction simulation
BEGIN;
    INSERT INTO users (name, email) VALUES ('Transaction User', 'tx@demo.com');
    SELECT * FROM users WHERE email = 'tx@demo.com';
    UPDATE users SET user_type = 'premium' WHERE email = 'tx@demo.com';
COMMIT;

-- Rollback simulation
BEGIN;
    INSERT INTO products (name, category, price) VALUES ('Temp Product', 'Test', 99.99);
    SELECT * FROM products WHERE name = 'Temp Product';
ROLLBACK;

-- =============================================================================
-- SECTION 9: PERFORMANCE ANALYSIS QUERIES
-- Tests: EXPLAIN plans, statistics, system queries
-- =============================================================================

\echo '=== Running Performance Analysis ==='

-- EXPLAIN queries for plan analysis
EXPLAIN ANALYZE SELECT * FROM orders WHERE user_id = 1;
EXPLAIN ANALYZE SELECT COUNT(*) FROM orders o JOIN users u ON o.user_id = u.id;
EXPLAIN (ANALYZE, BUFFERS) SELECT * FROM products WHERE category = 'Electronics' ORDER BY price DESC;

-- System statistics queries
SELECT schemaname, tablename, n_tup_ins, n_tup_upd, n_tup_del FROM pg_stat_user_tables;
SELECT query, calls, total_time, mean_time FROM pg_stat_statements ORDER BY total_time DESC LIMIT 5;

-- Index usage statistics
SELECT schemaname, tablename, indexname, idx_scan, idx_tup_read, idx_tup_fetch
FROM pg_stat_user_indexes
ORDER BY idx_scan DESC;

-- =============================================================================
-- SECTION 10: CLEANUP AND FINAL OPERATIONS
-- Tests: Maintenance operations, final statistics
-- =============================================================================

\echo '=== Running Maintenance Operations ==='

-- Vacuum and analyze
VACUUM ANALYZE users;
VACUUM ANALYZE products;
VACUUM ANALYZE orders;

-- Update table statistics
SELECT
    schemaname,
    tablename,
    n_tup_ins as inserts,
    n_tup_upd as updates,
    n_tup_del as deletes,
    n_live_tup as live_tuples,
    n_dead_tup as dead_tuples
FROM pg_stat_user_tables
ORDER BY tablename;

-- Final summary query
SELECT
    'Workload Summary' as report_type,
    (SELECT COUNT(*) FROM users) as total_users,
    (SELECT COUNT(*) FROM products) as total_products,
    (SELECT COUNT(*) FROM orders) as total_orders,
    (SELECT SUM(total_amount) FROM orders WHERE status != 'cancelled') as total_revenue,
    CURRENT_TIMESTAMP as completed_at;

-- Disable timing
\timing off

\echo '=== Workload Completed Successfully ==='
\echo 'Generated comprehensive log data for pg-logstats analysis'
\echo 'Expected log entries: 200-500'
\echo 'Runtime: 2-3 minutes'
\echo 'Coverage: DDL, DML, queries, errors, connections, performance analysis'
