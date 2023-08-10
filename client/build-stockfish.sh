#!/bin/bash

if [ ! -d Stockfish ]; then
  git clone https://github.com/official-stockfish/Stockfish.git
  (cd Stockfish/src && make -j profile-build)
fi
