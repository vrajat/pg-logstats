FROM postgres:17

# Install additional packages if needed
RUN apt-get update && \
    apt-get install -y \
        postgresql-contrib \
    && rm -rf /var/lib/apt/lists/*

# Copy custom PostgreSQL configuration
COPY postgresql.conf /etc/postgresql/postgresql.conf

# Create log directory with proper permissions
RUN mkdir -p /var/log/postgresql && \
    chown postgres:postgres /var/log/postgresql && \
    chmod 755 /var/log/postgresql

# Create a custom entrypoint script
RUN echo '#!/bin/bash\n\
set -e\n\
\n\
# Ensure log directory exists and has correct permissions\n\
mkdir -p /var/log/postgresql\n\
chown postgres:postgres /var/log/postgresql\n\
chmod 755 /var/log/postgresql\n\
\n\
# Start PostgreSQL with custom config\n\
exec docker-entrypoint.sh postgres -c config_file=/etc/postgresql/postgresql.conf\n\
' > /usr/local/bin/custom-entrypoint.sh && \
    chmod +x /usr/local/bin/custom-entrypoint.sh

# Expose port
EXPOSE 5432

# Use custom entrypoint
ENTRYPOINT ["/usr/local/bin/custom-entrypoint.sh"]
