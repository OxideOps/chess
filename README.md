# OxideOps Chess

A comprehensive chess platform built with Rust, featuring both client and server components.

## Overview

This project is a complete chess platform, allowing users to play chess games, analyze moves, and interact with various chess-related functionalities. The project is modular, with separate components for the core chess logic, the client interface, and the server.

## Features

- **Core Chess Logic**: Handles the rules of chess, move generation, game state, and more.
- **Client**: A frontend interface for users to play and analyze games. Compatible with both web browsers (via WebAssembly) and desktop environments.
- **Server**: Manages game sessions, player interactions, and other backend functionalities.

## Getting Started

1. Download and install [Rust](https://www.rust-lang.org/):
    ```bash
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
    ```
    Ensure that everything went well with:
    ```bash
    rustc --version
    ```
    If you do not see output, you may need to restart your terminal. Also, ensure `~/.cargo/bin` was added to your `PATH`.

2. Clone the repository:
   ```bash
   git clone https://github.com/OxideOps/chess.git
   ```

3. Download and install the necessary dependencies for this project. From the root of the project, run:
    ```bash
    ./setup.sh
    ```

## Building and Running

There are two binary packages that can be compiled and ran: `client` and `server`. Execute `cargo [build | run]` with the `-p` (package) flag, followed by the package:
```bash
cargo [build | run] -p [client | server]
```

Note that there is a `build.rs` file in each package, called a [build script](https://doc.rust-lang.org/cargo/reference/build-scripts.html), that causes Cargo to compile that script and execute it just before building the package.

### Client

The `client` contains the following code:

- User interface, using the ergonomic [Dioxus](https://github.com/DioxusLabs/dioxus) framework for building cross-platform interfaces in Rust;
- Core chess logic, contained in the `chess` library;
- [Stockfish](https://github.com/OxideOps/Stockfish.git) submodule for running Stockfish natively in `C++`;
- [emsdk](https://github.com/emscripten-core/emsdk.git) submodule for compiling web assembly from Stockfish when we build the `server`;
- [Tailwind](https://tailwindcss.com/) to make CSS a breeze.
- [Trunk](https://github.com/thedodd/trunk) to compile our program into web assembly.

### Server

The `server` contains the following code:

- Runs using the rust framework [Axum](https://github.com/tokio-rs/axum).
- Serves compiled WASM to the web client.  
- Uses web sockets to manage remote games between 2 clients.

## Database

### Mac OS

### Install PostgreSQL
```bash
brew install postgresql
```

### Setting Username and Password
**Default Username**: The default username is usually postgres.

**Password**: To set or reset the password, open a terminal and start by running:
```bash
psql postgres
```

### List Existing Roles
To list all roles, run:
```sql
\du
```
This will list all the roles. Identify a role that has superuser privileges.


### Use Existing Role
If you see a role that you'd like to use, you can set its password using:
```sql
ALTER USER role_name PASSWORD 'newpassword';
```
Replace `role_name` with the actual role name and `newpassword` with your desired password.


### Create New Role
If you'd like to create a new role, you can do so with:
```sql
CREATE ROLE role_name WITH LOGIN PASSWORD 'newpassword' SUPERUSER;
```
This will create `role_name` with superuser privileges.

Type `\q` to exit.

### Creating a Database
**Default Database**: PostgreSQL usually creates a default database named `postgres`.

**Custom Database**: To create a new database, open a terminal and run:
```bash
createdb newdbname
```

### Setting the DATABASE_URL
Once you have the username, password, and database name, you can set the DATABASE_URL environment variable in your shell:
```bash
export DATABASE_URL=postgresql://username:password@localhost/dbname
```
Replace `username`, `password`, and `dbname` with your PostgreSQL username, password, and database name.
