#!/bin/bash
set -e

# Function to wait for PostgreSQL to be ready
wait_for_postgres() {
    echo "Waiting for PostgreSQL to be ready..."
    until pg_isready -h postgres -p 5432 -U db_user; do
        echo "PostgreSQL is unavailable - sleeping"
        sleep 2
    done
    echo "PostgreSQL is ready!"
}

# Function to run migrations
run_migrations() {
    echo "Running database migrations..."
    export DATABASE_URL=postgres://db_user:db_pass@postgres:5432/recon
    diesel migration run
    echo "Database migrations completed!"
}

# Wait for PostgreSQL and run migrations
wait_for_postgres
run_migrations

# Execute the main command
echo "Starting application..."
exec "$@"