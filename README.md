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

Note that there are `build.rs` files in each package, called [Build Scripts](https://doc.rust-lang.org/cargo/reference/build-scripts.html), that causes Cargo to compile that script and execute it just before building the package.

### Client

The `client` contains the following code:

- User interface, using the ergonomic [Dioxus](https://github.com/DioxusLabs/dioxus) framework for building cross-platform interfaces in Rust.
- Core chess logic, contained in the `chess` library.
- [Stockfish](https://github.com/OxideOps/Stockfish.git) submodule for running Stockfish natively in `C++`.
- [emsdk](https://github.com/emscripten-core/emsdk.git) submodule for compiling `wasm` from Stockfish when we build the `server`
- [Tailwind](https://tailwindcss.com/) to make CSS a breeze

### Server

