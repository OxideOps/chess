#!/bin/bash

_get_x86_64_arch() {
  if grep -q 'vnni' /proc/cpuinfo; then
    grep -q 'avx512f' /proc/cpuinfo && echo "x86-64-vnni512" || echo "x86-64-vnni256"
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
}

_get_i386_arch() {
  if grep -q 'sse4_1' /proc/cpuinfo; then
    echo "x86-32-sse41-popcnt"
  elif grep -q 'sse2' /proc/cpuinfo; then
    echo "x86-32-sse2"
  else
    echo "x86-32"
  fi
}

_get_armv7l_arch() {
  grep -q 'neon' /proc/cpuinfo && echo "armv7-neon" || echo "armv7"
}

get_arch() {
  arch=$(uname -m)

  case "$arch" in
    "x86_64") _get_x86_64_arch ;;
    "i386") _get_i386_arch ;;
    "ppc64") echo "ppc-64" ;;
    "ppc") echo "ppc-32" ;;
    "armv7l") _get_armv7l_arch ;;
    "aarch64"|"armv8") echo "armv8" ;;
    *'e2k'*) echo "e2k" ;;
    *'arm64'*) echo "apple-silicon" ;;
    *)
      if [[ "${#arch}" -ge 5 && "${arch:0:5}" == "armv8" ]]; then
        echo "armv8"
      elif [[ "$arch" == *"64"* ]]; then
        echo "general-64"
      else
        echo "general-32"
      fi
      ;;
  esac
}