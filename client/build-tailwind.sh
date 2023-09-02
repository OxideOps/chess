#!/bin/bash

# Get the directory of the current script
CLIENT="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"

# Install tailwind if we don't have it already
if [ ! -f "$CLIENT/node_modules/.bin/tailwindcss" ]; then
    npm install --prefix "$CLIENT"
fi

# Run Tailwind CSS
( cd "$CLIENT" && ./node_modules/.bin/tailwindcss -i ./styles/input.css -o ./styles/output.css )
