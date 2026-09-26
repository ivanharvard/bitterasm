#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
install_root="$HOME/.bitterasm"
bin_dir="$install_root/bin"
std_dir="$install_root/std"

usage() {
    cat <<EOF
Usage: $0 [-y]

Builds and installs bitterasm and/or bitter to $bin_dir, and the standard
library to $std_dir.

  -y, --yes    Answer yes to every prompt, including those of the
               bitterasm-lsp install script if it's run
  -h, --help   Show this help
EOF
}

assume_yes=false

for arg in "$@"; do
    case "$arg" in
        -y|--yes) assume_yes=true ;;
        -h|--help) usage; exit 0 ;;
        *) echo "unknown option: $arg" >&2; usage >&2; exit 2 ;;
    esac
done

ask() {
    local prompt="$1"
    local reply

    if [ "$assume_yes" = true ]; then
        echo "$prompt [Y/n] y"
        return 0
    fi

    read -r -p "$prompt [Y/n] " reply

    case "$reply" in
        [nN]*) return 1 ;;
        *) return 0 ;;
    esac
}

# How to put $bin_dir on PATH, for whichever shell $SHELL names.
print_path_help() {
    local shell_name
    shell_name="$(basename "${SHELL:-}")"

    echo
    echo "$bin_dir isn't on your PATH yet."
    echo

    case "$shell_name" in
        fish)
            echo "Add it (fish saves this for future sessions) with:"
            echo
            echo "    fish_add_path $bin_dir"
            ;;
        zsh)
            echo "Add it to ~/.zshrc with:"
            echo
            echo "    echo 'export PATH=\"$bin_dir:\$PATH\"' >> ~/.zshrc"
            echo "    source ~/.zshrc"
            ;;
        bash)
            local rc="$HOME/.bashrc"
            [ "$(uname)" = Darwin ] && rc="$HOME/.bash_profile"
            echo "Add it to ${rc/#$HOME/\~} with:"
            echo
            echo "    echo 'export PATH=\"$bin_dir:\$PATH\"' >> ${rc/#$HOME/\~}"
            echo "    source ${rc/#$HOME/\~}"
            ;;
        csh|tcsh)
            local rc="$HOME/.${shell_name}rc"
            echo "Add it to ${rc/#$HOME/\~} with:"
            echo
            echo "    echo 'setenv PATH \"$bin_dir:\$PATH\"' >> ${rc/#$HOME/\~}"
            echo "    source ${rc/#$HOME/\~}"
            ;;
        *)
            echo "Add this line to your shell's startup file (sh syntax; adapt it"
            echo "for other shells):"
            echo
            echo "    export PATH=\"$bin_dir:\$PATH\""
            ;;
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
run_lsp_installer=false

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
    if ask "Found a bitterasm-lsp checkout alongside this repo — run its install script (it asks about the language server and VS Code extension)?"; then
        run_lsp_installer=true
    fi
fi

if [ "$install_bitterasm" = true ]; then
    build_and_copy bitterasm
fi

if [ "$install_bitter" = true ]; then
    build_and_copy bitter
fi

if [ "$run_lsp_installer" = true ]; then
    echo "Running $lsp_install_script..."

    lsp_args=()
    [ "$assume_yes" = true ] && lsp_args+=(-y)

    # This script prints the PATH advice itself, once, at the end.
    BITTERASM_PARENT_INSTALL=1 "$lsp_install_script" ${lsp_args[@]+"${lsp_args[@]}"}
fi

echo "Installing standard library..."
mkdir -p "$install_root"
rm -rf "$std_dir"
cp -r "$repo_root/std" "$std_dir"
echo "  installed $std_dir"

echo
echo "Done."

case ":$PATH:" in
    *":$bin_dir:"*) ;;
    *) print_path_help ;;
esac
