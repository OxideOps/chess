#!/bin/bash

# Get the directory of the current script
DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"

# Install tailwind if we don't have it already
if [ ! -f "$DIR/node_modules/.bin/tailwindcss" ]; then
    npm install --prefix "$DIR"
fi

# Run Tailwind CSS
"$DIR/node_modules/.bin/tailwindcss" -i "$DIR/styles/input.css" -o "$DIR/styles/output.css"
