#!/bin/bash
set -euo pipefail
root=$(cd "$(dirname "$0")" && pwd)
repository=$(cd "$root/.." && pwd)
sdk=${1:-iphoneos}
case "$sdk" in iphoneos|iphonesimulator) ;; *) exit 2 ;; esac
cmake=${CMAKE:-cmake}
build=${CHUZZLE_SDL_BUILD:-"${TMPDIR:-/tmp}/chuzzle-sdl-rebuild"}/$sdk
"$cmake" -S "$repository/Vendor/SDL2" -B "$build" -G 'Unix Makefiles' \
  -DCMAKE_SYSTEM_NAME=iOS -DCMAKE_OSX_SYSROOT="$(xcrun --sdk "$sdk" --show-sdk-path)" \
  -DCMAKE_OSX_ARCHITECTURES=arm64 -DCMAKE_OSX_DEPLOYMENT_TARGET=15.0 \
  -DCMAKE_BUILD_TYPE=Release -DCMAKE_POLICY_VERSION_MINIMUM=3.5 \
  "-DCMAKE_C_FLAGS=-ffile-prefix-map=$HOME=~" \
  "-DCMAKE_CXX_FLAGS=-ffile-prefix-map=$HOME=~" \
  "-DCMAKE_OBJC_FLAGS=-ffile-prefix-map=$HOME=~" \
  -DCMAKE_INSTALL_PREFIX="$build/install" \
  -DSDL_SHARED=OFF -DSDL_STATIC=ON -DSDL_TEST=OFF -DSDL_TESTS=OFF \
  -DSDL_HIDAPI=OFF -DSDL_RENDER_METAL=OFF
"$cmake" --build "$build" --parallel 4
"$cmake" --install "$build"
printf 'SDL rebuilt at %s\n' "$build/install"
# The host links this SDL archive explicitly before the Rust static library,
# so updated SDL objects take precedence over any cached bundled copy.
