# User Guide

This guide describes how to use `pg-logstats` to analyze PostgreSQL database logs, triage performance issues, and inspect live database state.

## Core Workflows

To investigate a database incident or optimize performance, use the following workflows:

1. **[Inspect](inspect.md)**: Probes your PostgreSQL environment to verify if the server is ready for log-backed or live-only investigation.
2. **[Top Query Families](top-query-families.md)**: Parses logs to identify, normalize, and group query statements into "query families," ranking them by total execution time.
3. **[Errors](errors.md)**: Classifies, normalizes, and attributes PostgreSQL log events (errors, fatals, panics) with SQLSTATE codes to pinpoint issues.
4. **[Temporary Files](temp-files.md)**: Highlights query families causing disk pressure by writing large temporary files.
5. **[Guidance Framework](guidance.md)**: Uses the rules engine to generate safe next-action SQL tasks to execute on the live database.
6. **[RDS & CloudWatch Logs](rds-cloudwatch.md)**: Explains how to integrate with AWS to pull and analyze logs directly from CloudWatch Logs.

## Recommended Reading Order

- Start with **[Inspect](inspect.md)** to ensure your database connection and log files are ready.
- Run **[Top Query Families](top-query-families.md)** to find the most expensive queries.
- Look into **[Errors](errors.md)** or **[Temporary Files](temp-files.md)** depending on whether you are debugging application exceptions or disk capacity/IO issues.
- Read the **[Guidance Framework](guidance.md)** to understand how the CLI suggests safe diagnostic commands to run on live instances.
