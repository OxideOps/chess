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

2. Clone the repository:
   ```bash
   git clone https://github.com/OxideOps/chess.git
   ```

3. Install the `oxide` cli tool:
    ```bash
    cargo install https://github.com/OxideOps/oxide-cli.git
    ```
4. Setup your project. From the root, run:
    ```bash
    oxide setup
    ```
5. Buid/Run the client or server:
    ```bash
    oxide [build | run] [client | server]
    ```
