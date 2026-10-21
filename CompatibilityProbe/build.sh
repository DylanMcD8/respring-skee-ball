#!/bin/bash
set -euo pipefail
root=$(cd "$(dirname "$0")" && pwd -P)
repository=$(cd "$root/.." && pwd -P)
cd "$repository"
export PATH="${CARGO_HOME:-$HOME/.cargo}/bin:/opt/homebrew/bin:/usr/local/bin:$PATH"
sdk=${1:-iphonesimulator}
case "$sdk" in
  iphoneos) target=aarch64-apple-ios; platform=ios ;;
  iphonesimulator) target=aarch64-apple-ios-sim; platform=ios-simulator ;;
  *) exit 2 ;;
esac
build=${CHUZZLE_NATIVE_BUILD:-"$repository/.build"}
export CARGO_TARGET_DIR="$build/rust"
export IPHONEOS_DEPLOYMENT_TARGET=15.0
export SDKROOT=$(xcrun --sdk "$sdk" --show-sdk-path)
sdl="$build/sdl/$sdk/install"
if [ ! -f "$sdl/lib/libSDL2.a" ]; then
  CHUZZLE_SDL_BUILD="$build/sdl" bash "$root/rebuild_sdl.sh" "$sdk"
fi
export CHUZZLE_SDL_LIBRARY="$sdl/lib/libSDL2.a"
unset RUSTFLAGS
rust_flags=(-L "native=$sdl/lib" --remap-path-prefix "$HOME=~" --remap-path-prefix "$repository=.")
frameworks=(UIKit Foundation OpenGLES OpenAL QuartzCore AVFoundation AudioToolbox CoreAudio CoreGraphics CoreMotion GameController CoreHaptics CoreBluetooth Metal Security)
link=()
for framework in "${frameworks[@]}"; do
  rust_flags+=(-C link-arg=-framework -C "link-arg=$framework")
  link+=(-framework "$framework")
done
printf -v CARGO_ENCODED_RUSTFLAGS '%s\x1f' "${rust_flags[@]}"
export CARGO_ENCODED_RUSTFLAGS=${CARGO_ENCODED_RUSTFLAGS%$'\x1f'}
cargo_args=(build)
if [ "${CHUZZLE_AOT_REFERENCE:-1}" = 1 ]; then
  cargo_args+=(--features aot-reference)
fi
cargo "${cargo_args[@]}" --manifest-path "$root/ios/rust-entry/Cargo.toml" --target "$target" --release --offline --locked
app="$build/$sdk/SkeeBall.app"
mkdir -p "$app"
rm -rf "$app/_CodeSignature"
rm -f "$app/embedded.mobileprovision"
xcrun --sdk "$sdk" clang -ffile-prefix-map="$HOME=~" -arch arm64 -isysroot "$SDKROOT" \
  -target "arm64-apple-$platform" -mios-version-min=15.0 \
  -I "$sdl/include/SDL2" "$root/ios/main.m" "$root/ios/frame_timing.m" \
  -Xlinker -force_load -Xlinker "$sdl/lib/libSDL2.a" \
  "$CARGO_TARGET_DIR/$target/release/libchuzzle_compatibility.a" \
  -L "$sdl/lib" -lSDL2main -lSDL2 -lc++ -liconv "${link[@]}" \
  -Wl,-no_adhoc_codesign -o "$app/ChuzzleCompatibility"
cp "$root/ios/Info.plist" "$app/Info.plist"
ditto --norsrc --noextattr "$root/ios/Settings.bundle" "$app/Settings.bundle"
python3 - "$app/Info.plist" "$sdk" <<'PYPLATFORM'
import plistlib, sys
p = sys.argv[1]
with open(p, 'rb') as f: info = plistlib.load(f)
info['CFBundleSupportedPlatforms'] = ['iPhoneSimulator' if sys.argv[2] == 'iphonesimulator' else 'iPhoneOS']
with open(p, 'wb') as f: plistlib.dump(info, f)
PYPLATFORM
# Compile the editable Icon Composer source for modern and legacy icon renditions.
icon_plist="$build/$sdk/icon-info.plist"
xcrun actool "$repository/Skee-Ball.icon" --compile "$app" \
  --platform "$sdk" --minimum-deployment-target 15.0 --app-icon Skee-Ball \
  --output-partial-info-plist "$icon_plist" --target-device iphone --target-device ipad
python3 - "$app/Info.plist" "$icon_plist" <<'PYICON'
import plistlib, sys
with open(sys.argv[1], 'rb') as f: info = plistlib.load(f)
with open(sys.argv[2], 'rb') as f: info.update(plistlib.load(f))
with open(sys.argv[1], 'wb') as f: plistlib.dump(info, f)
PYICON

cp "$root/../Original/Skee-Ball.ipa" "$app/Skee-Ball.ipa"

for resource in touchHLE_fonts touchHLE_dylibs; do
  ditto --norsrc --noextattr "$root/compatibility-core/$resource" "$app/$resource"
done
cp "$root/compatibility-core/touchHLE_default_options.txt" "$app/"
cp "$root/compatibility-core/LICENSE" "$app/touchHLE-LICENSE.txt"
cp "$root/compatibility-core/src/cpu/interpreter/LICENSE" "$app/ARM-runtime-LICENSE.txt"
if [ "$sdk" = iphonesimulator ]; then
  codesign --force --sign - "$app"
fi
mkdir -p "$app/ThirdPartyNotices"
ditto --norsrc --noextattr "$repository/Notices" "$app/ThirdPartyNotices"
printf '%s\n' "$app"
