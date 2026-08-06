#!/usr/bin/env bash
# Regenerate the Homebrew formula for snipx deterministically.
#
# Usage: generate-homebrew-formula.sh <version> <sha256-dir>
#
#   <version>     release version WITHOUT the leading "v", e.g. 0.1.0
#   <sha256-dir>  directory containing the four per-target checksum assets
#                 published on the GitHub release:
#                   snipx-v<version>-<target>.tar.gz.sha256
#
# Writes the complete Formula/snipx.rb to stdout. All-or-nothing: if any
# expected checksum file is missing or malformed the script fails without
# emitting anything, so a half-updated formula can never be produced.
#
# Used by .github/workflows/homebrew-bump.yml, and runnable locally against
# downloaded release assets to verify the output (it reproduces the tap's
# Formula/snipx.rb for a published release byte for byte).

set -euo pipefail

die() {
  echo "generate-homebrew-formula: $*" >&2
  exit 1
}

[ "$#" -eq 2 ] || die "usage: generate-homebrew-formula.sh <version> <sha256-dir>"

version=$1
dir=$2

# Semver with an optional pre-release/build suffix; no leading "v".
printf '%s' "$version" | grep -Eq '^[0-9]+\.[0-9]+\.[0-9]+([.-][A-Za-z0-9.-]+)?$' \
  || die "unexpected version format: '$version' (expected e.g. 0.1.0, no leading v)"
[ -d "$dir" ] || die "not a directory: $dir"

# Read the sha256 for one target from its checksum asset. The asset is the
# output of `shasum -a 256` (see release.yml): "<hex>  <filename>".
sha_for() {
  local target=$1
  local file="$dir/snipx-v${version}-${target}.tar.gz.sha256"
  [ -f "$file" ] || die "missing checksum asset: $file"
  local sha
  sha=$(awk 'NR == 1 { print $1 }' "$file")
  printf '%s' "$sha" | grep -Eq '^[0-9a-f]{64}$' \
    || die "malformed sha256 in $file: '$sha'"
  printf '%s' "$sha"
}

# Resolve (and validate) all four before emitting anything.
sha_macos_arm=$(sha_for aarch64-apple-darwin)
sha_macos_x86=$(sha_for x86_64-apple-darwin)
sha_linux_arm=$(sha_for aarch64-unknown-linux-musl)
sha_linux_x86=$(sha_for x86_64-unknown-linux-musl)

base="https://github.com/curvelogic/snipx/releases/download/v${version}"

cat <<EOF
class Snipx < Formula
  desc "Text annotation language for structured notes against prose documents"
  homepage "https://github.com/curvelogic/snipx"
  version "${version}"
  license any_of: ["MIT", "Apache-2.0"]

  on_macos do
    if Hardware::CPU.arm?
      url "${base}/snipx-v${version}-aarch64-apple-darwin.tar.gz"
      sha256 "${sha_macos_arm}"
    else
      url "${base}/snipx-v${version}-x86_64-apple-darwin.tar.gz"
      sha256 "${sha_macos_x86}"
    end
  end

  on_linux do
    if Hardware::CPU.arm?
      url "${base}/snipx-v${version}-aarch64-unknown-linux-musl.tar.gz"
      sha256 "${sha_linux_arm}"
    else
      url "${base}/snipx-v${version}-x86_64-unknown-linux-musl.tar.gz"
      sha256 "${sha_linux_x86}"
    end
  end

  def install
    bin.install "snipx"
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/snipx --version")
  end
end
EOF
