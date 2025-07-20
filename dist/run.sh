#!/bin/bash
DIR="$(cd "$(dirname "$0")" && pwd)"
LD_LIBRARY_PATH="$DIR/lib" "$DIR/dzd" "$@"
