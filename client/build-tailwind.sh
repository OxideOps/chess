#!/bin/bash

# Install tailwind if we don't have it already
if [ ! -f ./node_modules/.bin/tailwindcss ]; then
    npm install
fi

# Run Tailwind CSS
./node_modules/.bin/tailwindcss -i ./styles/input.css -o ./styles/output.css
