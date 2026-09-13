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
install_lsp=false

# Not part of this repo — a separate project, cloned as a sibling checkout
# purely by convention (`bitterasm-lsp`'s own Cargo.toml currently depends
# on this repo via a local `path = "../bitterasm"`, so it only builds at
# all when laid out this way). Someone who cloned just `bitterasm` won't
# have it, and shouldn't see a prompt for something that isn't there.
lsp_dir="$(cd "$repo_root/.." && pwd)/bitterasm-lsp"
lsp_install_script="$lsp_dir/install.sh"

if ask "Install the bitterasm language (compiler)?"; then
    install_bitterasm=true
fi

if ask "Install the bitter CLI (exporter)?"; then
    install_bitter=true
fi

if [ -x "$lsp_install_script" ]; then
    if ask "Found a bitterasm-lsp checkout alongside this repo — install the language server and VS Code extension too?"; then
        install_lsp=true
    fi
fi

if [ "$install_bitterasm" = true ]; then
    build_and_copy bitterasm
fi

if [ "$install_bitter" = true ]; then
    build_and_copy bitter
fi

if [ "$install_lsp" = true ]; then
    echo "Installing bitterasm-lsp..."
    "$lsp_install_script"
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
