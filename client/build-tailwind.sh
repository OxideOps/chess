#!/bin/bash

# Check for the existence of the Tailwind binary
if [ ! -f ./node_modules/.bin/tailwindcss ]; then
    echo "Tailwind CSS not found! Please run npm install inside the client directory."
    exit 1
fi

# Run Tailwind CSS
./node_modules/.bin/tailwindcss -i ./styles/input.css -o ./styles/output.css
