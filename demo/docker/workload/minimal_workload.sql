-- Minimal workload for pg-loggrep demo
-- This file contains sample SQL statements to generate log activity

-- Create a test table
CREATE TABLE IF NOT EXISTS demo_users (
    id SERIAL PRIMARY KEY,
    username VARCHAR(50) NOT NULL,
    email VARCHAR(100),
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Insert some sample data
INSERT INTO demo_users (username, email) VALUES
    ('alice', 'alice@example.com'),
    ('bob', 'bob@example.com'),
    ('charlie', 'charlie@example.com');

-- Create another table for more complex queries
CREATE TABLE IF NOT EXISTS demo_orders (
    id SERIAL PRIMARY KEY,
    user_id INTEGER REFERENCES demo_users(id),
    amount DECIMAL(10,2),
    status VARCHAR(20) DEFAULT 'pending',
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Insert sample orders
INSERT INTO demo_orders (user_id, amount, status) VALUES
    (1, 100.50, 'completed'),
    (2, 250.75, 'pending'),
    (3, 75.25, 'completed'),
    (1, 300.00, 'cancelled');

-- Sample queries that will generate log entries
SELECT COUNT(*) FROM demo_users;
SELECT AVG(amount) FROM demo_orders WHERE status = 'completed';
SELECT u.username, COUNT(o.id) as order_count
FROM demo_users u
LEFT JOIN demo_orders o ON u.id = o.user_id
GROUP BY u.id, u.username;
