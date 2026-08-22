#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
install_root="$HOME/.bitterasm"
bin_dir="$install_root/bin"
std_dir="$install_root/std"

ask() {
    local prompt="$1"
    local reply

    read -r -p "$prompt [Y/n] " reply

    case "$reply" in
        [nN]*) return 1 ;;
        *) return 0 ;;
    esac
}

build_and_copy() {
    local package="$1"

    echo "Building $package (release)..."
    cargo build --release --package "$package" --manifest-path "$repo_root/Cargo.toml"

    mkdir -p "$bin_dir"
    cp "$repo_root/target/release/$package" "$bin_dir/$package"
    echo "  installed $bin_dir/$package"
}

install_bitterasm=false
install_bitter=false

if ask "Install the bitterasm language (compiler)?"; then
    install_bitterasm=true
fi

if ask "Install the bitter CLI (exporter)?"; then
    install_bitter=true
fi

if [ "$install_bitterasm" = true ]; then
    build_and_copy bitterasm
fi

if [ "$install_bitter" = true ]; then
    build_and_copy bitter
fi

echo "Installing standard library..."
rm -rf "$std_dir"
cp -r "$repo_root/std" "$std_dir"
echo "  installed $std_dir"

echo
echo "Done."

case ":$PATH:" in
    *":$bin_dir:"*) ;;
    *)
        echo
        echo "$bin_dir isn't on your PATH yet. Add it with:"
        echo
        echo "    export PATH=\"$bin_dir:\$PATH\""
        ;;
esac
