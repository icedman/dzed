#!/usr/bin/env bash
set -e

REPO="https://github.com/folke/tokyonight.nvim"
SCHEME="tokyonight"

TMP=$(mktemp -d)

git clone --depth 1 "$REPO" "$TMP"

nvim --headless --clean \
    --cmd "set runtimepath^=$TMP" \
    -c "colorscheme $SCHEME" \
    -c "luafile export_theme.lua" \
    -c "lua EXPORT_THEME('${SCHEME}.json')" \
    -c "qa!"

rm -rf "$TMP"
