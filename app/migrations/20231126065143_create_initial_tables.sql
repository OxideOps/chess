-- migrations/20230329_123456_create_initial_tables.sql
CREATE TABLE my_new_table (
    id SERIAL PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    description TEXT
);
