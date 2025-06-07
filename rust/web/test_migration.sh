#!/bin/bash

set -e

echo "Testing SQLx migration on existing database..."

# Check if DATABASE_URL is set
if [ -z "$DATABASE_URL" ]; then
    echo "Error: DATABASE_URL environment variable is not set"
    echo "Please set it to your PostgreSQL connection string"
    echo "Example: export DATABASE_URL=postgresql://username:password@localhost/database_name"
    exit 1
fi

# Check if sqlx CLI is installed
if ! command -v sqlx &> /dev/null; then
    echo "Installing sqlx CLI..."
    cargo install sqlx-cli --no-default-features --features postgres
fi

echo "Database URL: $DATABASE_URL"

# Test database connection
echo "Testing database connection..."
sqlx database ping || {
    echo "Error: Could not connect to database"
    exit 1
}

echo "Database connection successful!"

# Show current migration status
echo "Current migration status:"
sqlx migrate info || echo "No migrations table found"

# Run migrations
echo "Running SQLx migrations..."
sqlx migrate run

echo "Migration status after running migrations:"
sqlx migrate info

# Test some basic queries to ensure schema is working
echo "Testing basic schema queries..."

# Test users table
psql "$DATABASE_URL" -c "SELECT COUNT(*) as user_count FROM users;" || {
    echo "Error: Could not query users table"
    exit 1
}

# Test games table
psql "$DATABASE_URL" -c "SELECT COUNT(*) as game_count FROM games;" || {
    echo "Error: Could not query games table"
    exit 1
}

# Test game_types table
psql "$DATABASE_URL" -c "SELECT COUNT(*) as game_type_count FROM game_types;" || {
    echo "Error: Could not query game_types table"
    exit 1
}

echo "All tests passed! Migration was successful."
echo ""
echo "Summary:"
echo "- Database connection: ✓"
echo "- Migration execution: ✓"
echo "- Schema validation: ✓"
echo ""
echo "The SQLx migration is compatible with your existing database."