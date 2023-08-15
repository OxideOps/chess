#!/bin/bash

main() {
  if [ ! -d Stockfish ]; then
    git clone https://github.com/official-stockfish/Stockfish.git
    (cd Stockfish/src && make -j profile-build ARCH="$(get_arch)")
  fi
}

get_arch() {
  # Get the architecture
  arch=$(uname -m)

  # Check for x86_64 or i386
  if [[ "$arch" == "x86_64" ]]; then
    # Check for various extensions
    if grep -q 'vnni' /proc/cpuinfo; then
      if grep -q 'avx512f' /proc/cpuinfo; then
        echo "x86-64-vnni512"
      else
        echo "x86-64-vnni256"
      fi
    elif grep -q 'avx512f' /proc/cpuinfo; then
      echo "x86-64-avx512"
    elif grep -q 'bmi2' /proc/cpuinfo; then
      echo "x86-64-bmi2"
    elif grep -q 'avx2' /proc/cpuinfo; then
      echo "x86-64-avx2"
    elif grep -q 'sse4_1' /proc/cpuinfo; then
      echo "x86-64-sse41-popcnt"
    elif grep -q 'ssse3' /proc/cpuinfo; then
      echo "x86-64-ssse3"
    elif grep -q 'sse3' /proc/cpuinfo; then
      echo "x86-64-sse3-popcnt"
    else
      echo "x86-64"
    fi
  elif [[ "$arch" == "i386" ]]; then
    if grep -q 'sse4_1' /proc/cpuinfo; then
      echo "x86-32-sse41-popcnt"
    elif grep -q 'sse2' /proc/cpuinfo; then
      echo "x86-32-sse2"
    else
      echo "x86-32"
    fi
  elif [[ "$arch" == "ppc64" ]]; then
    echo "ppc-64"
  elif [[ "$arch" == "ppc" ]]; then
    echo "ppc-32"
  elif [[ "$arch" == "armv7l" ]]; then
    if grep -q 'neon' /proc/cpuinfo; then
      echo "armv7-neon"
    else
      echo "armv7"
    fi
  elif [[ "$arch" == "aarch64" ]]; then
    echo "armv8"
  elif [[ "$arch" == *'e2k'* ]]; then
    echo "e2k"
  elif [[ "$arch" == *'arm64'* ]]; then
    echo "apple-silicon"
  else
    if [[ "${#arch}" -ge 5 && "${arch:0:5}" == "armv8" ]]; then
      echo "armv8"
    elif [[ "$arch" == *"64"* ]]; then
      echo "general-64"
    else
      echo "general-32"
    fi
  fi
}

main
