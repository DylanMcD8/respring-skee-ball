# Building Skee-Ball

Use full Xcode selected with `xcode-select`, Python 3, Rust installed through rustup, and CMake on PATH. The supported build target here is ARM64 iOS or an Apple silicon iOS simulator; this package is not a desktop or Android build recipe.

```sh
rustup toolchain install 1.99.0 --profile minimal
rustup target add --toolchain 1.99.0 aarch64-apple-ios aarch64-apple-ios-sim
```

The initial toolchain installation needs network access. Cargo sources and modified SDL2 are vendored; the build uses `--offline --locked`. No sibling Respring/Chuzzle project, private cache, or saved signing team is required. SDL2 is compiled with CMake for each target on its first build.

## Xcode

Open `SkeeBall.xcodeproj`, choose the SkeeBall scheme, and select a destination. For a device, set your own development team and an available bundle identifier. Xcode processes `CompatibilityProbe/ios/Info.xcode.plist`, expanding the selected identifier/executable and retaining icon metadata. The copy phase excludes the final root Info.plist to avoid competing with Xcode's plist processor.

```sh
xcodebuild -project SkeeBall.xcodeproj -scheme SkeeBall \
  -configuration Release -destination 'generic/platform=iOS Simulator' \
  -derivedDataPath /tmp/SkeeBall-sim CODE_SIGNING_ALLOWED=NO

xcodebuild -project SkeeBall.xcodeproj -scheme SkeeBall \
  -configuration Release -destination 'generic/platform=iOS' \
  -derivedDataPath /tmp/SkeeBall-device CODE_SIGNING_ALLOWED=NO
```

Leave signing enabled and choose your team when using Run on a physical device. Both Debug and Release use optimized Rust; unoptimized interpreter builds are too slow for normal use.

## Direct build and unsigned package

```sh
CHUZZLE_NATIVE_BUILD=/tmp/SkeeBall-build bash CompatibilityProbe/build.sh iphoneos
python3 Scripts/package_ipa.py /tmp/SkeeBall-build/iphoneos/SkeeBall.app \
  --output release-output/Skee-Ball.ipa
python3 Scripts/audit_release.py --ipa release-output/Skee-Ball.ipa
```

Use `iphonesimulator` for a simulator app, which cannot be distributed as a device IPA. The package script stages a copy, strips signing/provisioning and account metadata, sanitizes any nested original IPAs, and leaves the signed input app unchanged. The result needs the recipient's signing method.

Build outside iCloud/File Provider-managed output directories; Xcode's DerivedData or `/tmp` works well. Legacy internal symbols and environment variables may retain the `CHUZZLE_` prefix because these ports share runtime ancestry.

## Source and execution

- `CompatibilityProbe/ios`: native host, frame pacing, Rust entry point, app metadata and Settings bundle.
- `CompatibilityProbe/compatibility-core`: this app's modified touchHLE runtime and instruction handling.
- `Original`: sanitized original app payload plus provenance.
- `Skee-Ball.icon`: editable Icon Composer document.
- `Vendor`: pinned Cargo sources and the modified SDL2 source.
- `Notices`: dependency licenses copied into built apps.

Execution: **ARM/Thumb CPU interpreter with legacy iOS/Unity compatibility APIs; no JIT**. `CHUZZLE_AOT_REFERENCE=1` is enabled by default because this port relies on the interpreter. Do not disable it on the assumption that the game has been statically translated.

The original Unity executable and assets remain bundled. The modern host/runtime provides graphics, touch, lifecycle, alert and offline-service compatibility; it does not reimplement Unity or translate the entire game to native source.

## Regression checks

Run with the pinned Rust toolchain available on PATH:

```sh
rustc --edition=2021 CompatibilityProbe/Tests/live_layout.rs -o /tmp/SkeeBall-layout-test
/tmp/SkeeBall-layout-test
```

The included tests cover repeated size changes, render-buffer dimensions, and centered UI/full-world rectangles. Device performance and original gameplay require testing on the intended hardware. Simulator performance is not representative.

Original game content remains outside the runtime's open-source license grant. The supplied original payload has account/purchase and signing sidecar metadata removed; it is still original game content. See LICENSE.md and THIRD_PARTY_NOTICES.md.
