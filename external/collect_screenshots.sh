#!/bin/bash

max_parallel=2
timeout_duration=360
process_executable="./target/release/rom-db"

export SHELL=$(type -p bash)

find "$1" \( -iname '*.gba' -o -iname '*.zip' \) | \
  parallel --bar -j "$max_parallel" --line-buffer timeout "$timeout_duration" "$process_executable" {}
