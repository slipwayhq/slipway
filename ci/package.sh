#!/usr/bin/env bash

# Based on the /bin/package script in the just repository.

set -euxo pipefail

VERSION=${REF#"refs/tags/"}
DIST=`pwd`/dist

cargo install cargo-edit

echo "Packaging Slipway $VERSION for $TARGET..."

echo "Installing rust toolchain for $TARGET..."
rustup target add $TARGET

# if [[ $TARGET == aarch64-unknown-linux-musl ]]; then
#   export CC=aarch64-linux-gnu-gcc
# fi

echo "Building Slipway..."
pushd src
cargo set-version $VERSION
test -f Cargo.lock || cargo generate-lockfile


# We use vendored-openssl to avoid cross-compilation issues with OpenSSL:
# https://github.com/cross-rs/cross/issues/229#issuecomment-597898074
# We remove sixel (via --no-default-features) because it is not supported on musl.
if [[ "$TARGET" == *"musl"* ]]; then
  ALL_RUSTFLAGS="--deny warnings --codegen target-feature=+crt-static $TARGET_RUSTFLAGS $RUSTFLAGS"
  RUSTFLAGS="$ALL_RUSTFLAGS" cross build --bin slipway --target $TARGET --release --no-default-features --features vendored-openssl
elif [[ "$TARGET" == *"aarch64-unknown-linux-gnu"* ]]; then
  ALL_RUSTFLAGS="--deny warnings $TARGET_RUSTFLAGS $RUSTFLAGS"
  RUSTFLAGS="$ALL_RUSTFLAGS" cross build --bin slipway --target $TARGET --release --features vendored-openssl --dockerfile ../ci/Dockerfile.aarch64_gnu
else
  ALL_RUSTFLAGS="--deny warnings $TARGET_RUSTFLAGS $RUSTFLAGS"
  RUSTFLAGS="$ALL_RUSTFLAGS" cargo build --bin slipway --target $TARGET --release --features vendored-openssl
fi

popd
EXECUTABLE=src/target/$TARGET/release/slipway

if [[ $OS == windows-latest ]]; then
  EXECUTABLE=$EXECUTABLE.exe
fi

echo "Copying release files..."
mkdir dist
cp -r \
  $EXECUTABLE \
  LICENSE \
  README.md \
  $DIST

cd $DIST
echo "Creating release archive..."
case $OS in
  ubuntu-latest | macos-latest)
    ARCHIVE=slipway-$VERSION-$TARGET.tar.gz
    tar czf $ARCHIVE *
    echo "archive=$DIST/$ARCHIVE" >> $GITHUB_OUTPUT
    ;;
  windows-latest)
    ARCHIVE=slipway-$VERSION-$TARGET.zip
    7z a $ARCHIVE *
    echo "archive=`pwd -W`/$ARCHIVE" >> $GITHUB_OUTPUT
    ;;
esac