# pg-loggrep Demo Environment

This directory contains a complete demo environment for testing pg-loggrep with a real PostgreSQL instance.

## Quick Start

1. **Setup the environment:**
   ```bash
   ./scripts/setup.sh
   ```

2. **Run the demo:**
   ```bash
   ./scripts/run-demo.sh
   ```

3. **Generate workload:**
   ```bash
   ./scripts/run-workload.sh
   ```

4. **Cleanup:**
   ```bash
   ./scripts/cleanup.sh
   ```

## Components

- **PostgreSQL Container**: A PostgreSQL instance with logging enabled
- **Workload Generator**: A container that generates sample database activity
- **Scripts**: Helper scripts for managing the demo environment

## Configuration

The PostgreSQL instance is configured with:
- Logging enabled (`log_statement = 'all'`)
- Log format set to stderr
- Sample database with test tables

## Log Files

PostgreSQL logs are available in the `./logs/` directory after running the demo.
