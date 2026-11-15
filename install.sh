#!/bin/sh
# Install Forge (https://github.com/mollendorff-ai/forge) from its release
# binaries: no compiler, no package manager, nothing built.
#
#   curl -fsSL https://raw.githubusercontent.com/mollendorff-ai/forge/main/install.sh | sh
#   curl -fsSL https://raw.githubusercontent.com/mollendorff-ai/forge/main/install.sh | sh -s -- --examples
#
# It downloads the archive for this system from the latest release (or the
# one in FORGE_VERSION, such as v10.0.0-rc.25), checks it against the
# release's checksums.txt, and puts the `forge` binary in ~/.local/bin (or
# FORGE_INSTALL_DIR). With --examples it also puts that release's examples
# in ./forge-examples (or the folder after --examples), the bagel shop among
# them. Linux and macOS, x86_64 and ARM; on Windows, use the zip from the
# release page.

set -eu

repo="mollendorff-ai/forge"
install_dir="${FORGE_INSTALL_DIR:-$HOME/.local/bin}"
version="${FORGE_VERSION:-}"
examples=""

say() { printf '%s\n' "$*"; }
fail() {
    printf 'install.sh: %s\n' "$*" >&2
    exit 1
}

while [ $# -gt 0 ]; do
    case "$1" in
        --examples)
            examples="forge-examples"
            if [ $# -gt 1 ] && [ "${2#-}" = "$2" ]; then
                examples="$2"
                shift
            fi
            ;;
        --version)
            [ $# -gt 1 ] || fail "--version needs a tag, such as v10.0.0-rc.25"
            version="$2"
            shift
            ;;
        --dir)
            [ $# -gt 1 ] || fail "--dir needs a folder"
            install_dir="$2"
            shift
            ;;
        -h | --help)
            sed -n '2,15p' "$0" 2>/dev/null | sed 's/^# \{0,1\}//' || true
            exit 0
            ;;
        *) fail "unknown option: $1 (try --help)" ;;
    esac
    shift
done

# Something to download with
if command -v curl >/dev/null 2>&1; then
    fetch() { curl -fsSL --retry 3 -o "$2" "$1"; }
    resolve() { curl -fsSLI --retry 3 -o /dev/null -w '%{url_effective}' "$1"; }
elif command -v wget >/dev/null 2>&1; then
    fetch() { wget -q -O "$2" "$1"; }
    resolve() { wget -q -S --spider "$1" 2>&1 | sed -n 's/^ *[Ll]ocation: //p' | tail -n 1; }
else
    fail "needs curl or wget"
fi

# Something to check the download with
if command -v sha256sum >/dev/null 2>&1; then
    sha256() { sha256sum "$1" | cut -d ' ' -f 1; }
elif command -v shasum >/dev/null 2>&1; then
    sha256() { shasum -a 256 "$1" | cut -d ' ' -f 1; }
else
    fail "needs sha256sum or shasum to check the download"
fi

# The archive for this system, named as the release names them
os="$(uname -s)"
arch="$(uname -m)"
case "$arch" in
    x86_64 | amd64) arch="x86_64" ;;
    aarch64 | arm64) arch="aarch64" ;;
    *) fail "no release binary for the processor $arch; see https://github.com/$repo#quick-start" ;;
esac
case "$os" in
    Linux) target="$arch-unknown-linux-musl" ;;
    Darwin) target="$arch-apple-darwin" ;;
    MINGW* | MSYS* | CYGWIN*)
        fail "on Windows, download forge-x86_64-pc-windows-msvc.zip from https://github.com/$repo/releases/latest"
        ;;
    *) fail "no release binary for $os; see https://github.com/$repo#quick-start" ;;
esac
archive="forge-$target.tar.gz"

# The release: the tag "latest" leads to, read from the redirect (no API call,
# so no token or rate limit is involved)
if [ -z "$version" ]; then
    latest="$(resolve "https://github.com/$repo/releases/latest")" || latest=""
    version="${latest##*/}"
    case "$version" in
        v*) ;;
        *) fail "could not find the latest release; set FORGE_VERSION, such as v10.0.0-rc.25" ;;
    esac
fi
base="https://github.com/$repo/releases/download/$version"

if [ -n "$examples" ] && [ -e "$examples" ]; then
    fail "$examples already exists; give another folder after --examples"
fi

tmp="$(mktemp -d 2>/dev/null || mktemp -d -t forge)"
trap 'rm -rf "$tmp"' EXIT INT TERM

say "Forge $version for $target"
fetch "$base/$archive" "$tmp/$archive" || fail "could not download $base/$archive"
fetch "$base/checksums.txt" "$tmp/checksums.txt" || fail "could not download $base/checksums.txt"

# The name may have a folder before it (releases before rc.25) or a `*`
expected="$(awk -v name="$archive" '{ file = $2; sub(/^\*/, "", file); sub(/.*\//, "", file) } file == name { print $1 }' "$tmp/checksums.txt")"
[ -n "$expected" ] || fail "checksums.txt of $version has no line for $archive"
actual="$(sha256 "$tmp/$archive")"
[ "$expected" = "$actual" ] || fail "checksum of $archive does not match checksums.txt: not installed"
say "Checksum OK"

tar -xzf "$tmp/$archive" -C "$tmp"
[ -f "$tmp/forge" ] || fail "$archive has no forge binary in it"
mkdir -p "$install_dir"
cp "$tmp/forge" "$install_dir/forge"
chmod +x "$install_dir/forge"
say "Installed $install_dir/forge: $("$install_dir/forge" --version)"

if [ -n "$examples" ]; then
    # The examples of the same release, from its source archive
    fetch "https://github.com/$repo/archive/refs/tags/$version.tar.gz" "$tmp/source.tar.gz" ||
        fail "could not download the source of $version for its examples"
    mkdir -p "$tmp/source"
    tar -xzf "$tmp/source.tar.gz" -C "$tmp/source"
    found="$(find "$tmp/source" -mindepth 2 -maxdepth 2 -type d -name examples | head -n 1)"
    [ -n "$found" ] || fail "the source of $version has no examples folder"
    cp -R "$found" "$examples"
    say "Examples in $examples (the bagel shop: $examples/bagel)"
fi

case ":$PATH:" in
    *":$install_dir:"*) run="forge" ;;
    *)
        run="$install_dir/forge"
        say "$install_dir is not on your PATH: run $run, or add it to PATH"
        ;;
esac
if [ -n "$examples" ]; then
    say "Try: $run calculate $examples/bagel/bagel.yaml --dry-run"
else
    say "Try: $run --help"
fi
