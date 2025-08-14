FROM python:3.9-slim

# Install PostgreSQL client
RUN apt-get update && \
    apt-get install -y postgresql-client && \
    rm -rf /var/lib/apt/lists/*

# Set working directory
WORKDIR /app

# Copy workload script
COPY minimal_workload.sql /app/

# Create Python script for workload generation
RUN echo '#!/usr/bin/env python3\n\
import os\n\
import time\n\
import psycopg2\n\
import random\n\
\n\
def generate_workload():\n\
    conn = psycopg2.connect(\n\
        host=os.environ["POSTGRES_HOST"],\n\
        database=os.environ["POSTGRES_DB"],\n\
        user=os.environ["POSTGRES_USER"],\n\
        password=os.environ["POSTGRES_PASSWORD"]\n\
    )\n\
    \n\
    cur = conn.cursor()\n\
    \n\
    # Create test table\n\
    cur.execute("""\n\
        CREATE TABLE IF NOT EXISTS test_data (\n\
            id SERIAL PRIMARY KEY,\n\
            name VARCHAR(100),\n\
            value INTEGER,\n\
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP\n\
        )\n\
    """)\n\
    \n\
    # Generate some sample data\n\
    for i in range(100):\n\
        cur.execute("""\n\
            INSERT INTO test_data (name, value) VALUES (%s, %s)\n\
        """, (f"item_{i}", random.randint(1, 1000)))\n\
    \n\
    # Run various queries\n\
    queries = [\n\
        "SELECT COUNT(*) FROM test_data",\n\
        "SELECT AVG(value) FROM test_data",\n\
        "SELECT name, value FROM test_data WHERE value > 500",\n\
        "SELECT * FROM test_data ORDER BY created_at DESC LIMIT 10",\n\
    ]\n\
    \n\
    for query in queries:\n\
        cur.execute(query)\n\
        cur.fetchall()\n\
        time.sleep(0.1)  # Small delay between queries\n\
    \n\
    conn.commit()\n\
    cur.close()\n\
    conn.close()\n\
\n\
if __name__ == "__main__":\n\
    print("Starting workload generation...")\n\
    generate_workload()\n\
    print("Workload generation completed.")\n\
' > /app/workload.py

# Install psycopg2
RUN pip install psycopg2-binary

# Make script executable
RUN chmod +x /app/workload.py

# Default command
CMD ["python", "/app/workload.py"]
