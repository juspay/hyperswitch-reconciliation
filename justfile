# Justfile for the tests worktree
# Run 'just' to see all available commands

# List all available commands
default:
    @just --list

# === CORE TEST COMMANDS ===

# Run all tests with test configuration
test *ARGS="":
    just resurrect recon_test
    CONFIG_FILE=config/test.toml cargo nextest run {{ARGS}}

# === DATABASE COMMANDS ===

# Drop and recreate database with fresh migrations
resurrect database_name:
    #!/usr/bin/env bash
    set -e
    DB_USER=${TEST_DB_USER:-postgres}
    DB_PASS=${TEST_DB_PASS:-postgres}
    DB_HOST=${TEST_DB_HOST:-localhost}
    DB_PORT=${TEST_DB_PORT:-5432}
    DB_NAME="{{database_name}}"

    echo "Resurrecting database '$DB_NAME'..."
    PGPASSWORD=$DB_PASS psql -h $DB_HOST -p $DB_PORT -U $DB_USER -d postgres -c "DROP DATABASE IF EXISTS $DB_NAME" || true
    PGPASSWORD=$DB_PASS psql -h $DB_HOST -p $DB_PORT -U $DB_USER -d postgres -c "CREATE DATABASE $DB_NAME"

    echo "Database '$DB_NAME' resurrected successfully!"
    just migrate "{{database_name}}"

# Run database migrations
migrate database_name:
    #!/usr/bin/env bash
    set -e
    DB_USER=${TEST_DB_USER:-postgres}
    DB_PASS=${TEST_DB_PASS:-postgres}
    DB_HOST=${TEST_DB_HOST:-localhost}
    DB_PORT=${TEST_DB_PORT:-5432}
    DB_NAME="{{database_name}}"

    echo "Running migrations on database '$DB_NAME'..."
    DATABASE_URL="postgres://$DB_USER:$DB_PASS@$DB_HOST:$DB_PORT/$DB_NAME" diesel migration run

    echo "Migrations completed successfully!"

# === CODE QUALITY ===

# Format code
fmt:
    cargo fmt --all

# Run clippy lints
clippy:
    cargo clippy --all-targets --all-features -- -D warnings

# Check that everything builds
check:
    cargo check --all-targets --all-features

# === COVERAGE ===

# Run tests with coverage (use --html --open to view in browser)
coverage *ARGS="":
    CONFIG_FILE=config/test.toml cargo llvm-cov nextest {{ARGS}}

# === SETUP & MAINTENANCE ===

# Install required tools
install-tools:
    cargo install diesel_cli --no-default-features --features postgres
    cargo install cargo-nextest
    cargo install cargo-llvm-cov

# Clean build artifacts and coverage reports
clean:
    cargo clean
    cargo llvm-cov clean --workspace
    rm -f lcov.info
