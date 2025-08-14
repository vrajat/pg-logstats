FROM postgres:15

# Copy custom PostgreSQL configuration
COPY postgresql.conf /etc/postgresql/postgresql.conf

# Set the custom configuration file
ENV POSTGRES_INITDB_ARGS="--config-file=/etc/postgresql/postgresql.conf"

# Create log directory
RUN mkdir -p /var/log/postgresql && \
    chown postgres:postgres /var/log/postgresql

# Switch to postgres user for final setup
USER postgres

# Expose port
EXPOSE 5432

# Use custom configuration
CMD ["postgres", "-c", "config_file=/etc/postgresql/postgresql.conf"]
