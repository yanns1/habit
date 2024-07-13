#!/bin/sh

set -e

app="habit"

echo "Compiling..."
cargo build --release 2>/dev/null || (echo "Compilation failed :(" && exit 1)

link="$HOME/.local/bin/$app"
echo "Making symlink $link"
cd "$(dirname "$0")"
target="$(pwd)/target/release/$app"
ln -s -i "$target" "$link" || (echo "Symlink creation failed :(" && exit 1)

echo "$app successfully installed!"
