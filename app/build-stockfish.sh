#!/bin/bash

# Source the get_architecture.sh to have access to its functions
source ../get_architecture.sh

set -e
set -o pipefail

APP="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"

if [ "$1" = "--wasm" ]; then
  OUT_FILE="$APP/Stockfish/src/stockfish.wasm"
else
  OUT_FILE="$APP/Stockfish/src/stockfish"
fi

install_emscripten() {
    cd "$APP"/emsdk
    ./emsdk install 2.0.34
    ./emsdk activate 2.0.34
    source ./emsdk_env.sh
    cd -
}

main() {
  if [ ! -f "$OUT_FILE" ]; then
    if [ "$1" = "--wasm" ]; then
      install_emscripten
      ( cd "$APP"/Stockfish/src && make clean && make emscripten_build ARCH=wasm )
      ( cd "$APP" && patch -p0 < .allow-stopping.patch ) || true
    else
      ( cd "$APP"/Stockfish/src && make clean && make profile-build "ARCH=$(get_arch)" )
    fi
  fi
}

main "$@"
