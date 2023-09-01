#!/bin/bash

# Default: run in interactive mode
DOCKER_MODE=false

# Make the server functions not change based on the path to the repo 
ENV_VAR="SERVER_FN_OVERRIDE_KEY=y"

parse_arguments() {
    while [[ "$#" -gt 0 ]]; do
        case $1 in
            -d|--docker)
                DOCKER_MODE=true
                ;;
            *)
                echo "Invalid option: $1" >&2
                exit 1
                ;;
        esac
        shift
    done
}

install_packages() {
    apt-get update
    apt-get upgrade -y
    apt-get install -y curl libjavascriptcoregtk-4.1-dev libgtk-3-dev libsoup-3.0-dev libwebkit2gtk-4.1-dev
}

install_rust_and_cargo() {
    if $DOCKER_MODE; then
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    else
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
    fi
}

setup_nodejs() {
    curl -sL https://deb.nodesource.com/setup_20.x | bash -
    apt-get install -y nodejs
}

setup_rust_environment() {
    rustup target add wasm32-unknown-unknown
    rustup component add rustfmt
    cargo install --locked trunk
}

setup_environment_variable() {
    echo "Setting up environment variable..."

    case $SHELL in
    */zsh)
        echo "Detected zsh..."
        echo "export $ENV_VAR" >> ~/.zshrc
        ;;
    */bash)
        echo "Detected bash..."
        echo "export $ENV_VAR" >> ~/.bashrc
        ;;
    */fish)
        echo "Detected fish..."
        # Fish shell doesn't use the "export" keyword
        echo "set -gx $(echo $ENV_VAR | sed 's/=/ /')" >> ~/.config/fish/config.fish
        ;;
    *)
        echo "Shell not detected. Please add the following to your shell's startup file:"
        echo "export $ENV_VAR"
        ;;
    esac
}

main() {
    parse_arguments "$@"
    install_packages
    install_rust_and_cargo
    setup_nodejs
    setup_rust_environment
    if ! $DOCKER_MODE; then
        setup_environment_variable
    fi

    echo "Setup completed!"
}

main "$@"
