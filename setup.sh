#!/bin/bash

# Default: run in interactive mode
DOCKER_MODE=false

# Make the server functions not change based on the path to the repo 
ENV_VAR="SERVER_FN_OVERRIDE_KEY=y"
PACKAGES="build-essential curl libjavascriptcoregtk-4.1-dev libgtk-3-dev libsoup-3.0-dev libssl-dev libwebkit2gtk-4.1-dev"

trap 'echo "Setup failed!"; exit 1' ERR

terminate_script() {
    local message="$1"
    echo "$message" >&2
    # Exit if script is executed, return if sourced
    [[ "$0" = "$BASH_SOURCE" ]] && exit 1 || return 1
}

parse_arguments() {
    while [[ "$#" -gt 0 ]]; do
        case $1 in
            -d|--docker)
                DOCKER_MODE=true
                ;;
            *)
                terminate_script "Invalid option: $1"
                ;;
        esac
        shift
    done
}

install_packages() {
    if "$DOCKER_MODE"; then
        apt-get update && apt-get upgrade -y && apt-get install -y $PACKAGES
    else
        sudo apt-get update && sudo apt-get upgrade -y && sudo apt-get install -y $PACKAGES
    fi
}

setup_nodejs() {
    curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.39.5/install.sh | bash
    export NVM_DIR="$HOME/.nvm"
    [ -s "$NVM_DIR/nvm.sh" ] && \. "$NVM_DIR/nvm.sh"  # This loads nvm
    nvm install node
}

install_rust() {
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain nightly
    source ~/.cargo/env
}

setup_rust_environment() {
    rustup override set nightly
    rustup target add wasm32-unknown-unknown
    rustup component add rustfmt
    cargo install --locked trunk
}

append_if_not_present() {
    local file="$1"
    local content="$2"
    grep -qxF "$content" "$file" || echo "$content" >> "$file"
}

setup_environment_variable() {
    local content
    case $SHELL in
    */zsh)
        content="export $ENV_VAR"
        append_if_not_present ~/.zshrc "$content"
        ;;
    */bash)
        content="export $ENV_VAR"
        append_if_not_present ~/.bashrc "$content"
        ;;
    */fish)
        content="set -gx $(echo $ENV_VAR | sed 's/=/ /')"
        append_if_not_present ~/.config/fish/config.fish "$content"
        ;;
    *)
        echo "Shell not detected. Please add the following to your shell's startup file:"
        echo "export $ENV_VAR"
        return
        ;;
    esac
    eval $content
}

update_submodules() {
    git submodule update --init
}

main() {
    parse_arguments "$@"
    install_packages
    setup_nodejs
    if $DOCKER_MODE; then
        install_rust
    fi
    setup_rust_environment
    if ! $DOCKER_MODE; then
        update_submodules
        setup_environment_variable
        echo "Setup completed! Run 'cargo run -p client' to launch the client"
    fi
}

main "$@"
