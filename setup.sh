#!/bin/bash

INTERACTIVE=true

# Parse options
while getopts "n" opt; do
    case $opt in
        n)
            INTERACTIVE=false
            ;;
        \?)
            echo "Invalid option: -$OPTARG" >&2
            exit 1
            ;;
    esac
done

# Start the setup
apt-get update
apt-get upgrade -y
apt-get install -y curl libjavascriptcoregtk-4.1-dev libgtk-3-dev libsoup-3.0-dev libwebkit2gtk-4.1-dev

# Install rustup and cargo based on mode
if $INTERACTIVE; then
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
else
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
fi

# Continue with the rest of the setup
curl -sL https://deb.nodesource.com/setup_20.x | bash -
apt-get install -y nodejs
rustup target add wasm32-unknown-unknown
rustup component add rustfmt
cargo install --locked trunk

echo "Setup completed!"
