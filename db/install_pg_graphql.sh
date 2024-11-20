#!/usr/bin/env bash

# https://supabase.github.io/pg_graphql/

readonly PRODUCT="pg_graphql"

# Get PostgreSQL extension dir
readonly EXTENSION_DIR="$FLOX_ENV_DIRS/share/postgresql/extension"

echo "Installing $PRODUCT; please input root"
sudo cp db/$PRODUCT--1.5.9.sql "$EXTENSION_DIR"
sudo cp db/$PRODUCT.control "$EXTENSION_DIR"
sudo cp db/$PRODUCT.dylib "$FLOX_ENV_LIB_DIRS"
psql -d $PGDATABASE -c "CREATE EXTENSION IF NOT EXISTS $PRODUCT; CREATE ROLE anon NOLOGIN; GRANT USAGE ON SCHEMA public TO anon;"

echo "Finished installing $PRODUCT"
