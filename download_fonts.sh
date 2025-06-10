#!/bin/bash

mkdir -p src/fonts

ROBOTO_FONT_URL="https://github.com/google/fonts/raw/refs/heads/main/ofl/roboto/Roboto%5Bwdth%2Cwght%5D.ttf"
ROBOTO_MONO_FONT_URL="https://github.com/google/fonts/raw/refs/heads/main/ofl/robotomono/RobotoMono%5Bwght%5D.ttf"
./download_font.sh "src/fonts" "Roboto.ttf" "$ROBOTO_FONT_URL"
./download_font.sh "src/fonts" "RobotoMono.ttf" "$ROBOTO_MONO_FONT_URL"
