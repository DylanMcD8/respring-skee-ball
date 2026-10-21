# Third-party sources and notices

- **touchHLE** — https://github.com/touchHLE/touchHLE — MPL-2.0. The modified source is in `CompatibilityProbe/compatibility-core`. Changes include ARM/Thumb interpretation, Unity-era iOS API support, native iOS hosting, rendering, live layout and input mapping, lifecycle handling, and offline menu/alert fixes.
- **ARM instruction semantics** — retained under the MPL-2.0 notice in `CompatibilityProbe/compatibility-core/src/cpu/interpreter/LICENSE`.
- **SDL2** — https://github.com/libsdl-org/SDL — zlib license. `Vendor/SDL2` is modified from commit `07d0f51fa292895443f563f0cbde4cb3802d87fa`; UIKit scene/display changes are included in the source.
- **Rust dependencies** — `Vendor/Rust` and `CompatibilityProbe/ios/rust-entry/Cargo.lock`. Exact versions, checksums, source, and upstream license files are included. `Notices/rust-dependencies.txt` lists dependency license metadata.
- **stb_image** — original dual-license notice retained in `CompatibilityProbe/compatibility-core/vendor/stb/stb_image.h`.
- **PVRTDecompress** — original license retained in `CompatibilityProbe/compatibility-core/vendor/PVRTDecompress/LICENSE.md`.
- **Fonts** — license texts accompany the files in `CompatibilityProbe/compatibility-core/touchHLE_fonts`.
- **Bundled legacy dylibs** — provenance, aliases, upstream source links, and original notices are in `CompatibilityProbe/compatibility-core/touchHLE_dylibs/README.md` and its `COPYING.*` files. GNU library sources identified there are Apple `gcc-5664` and `libstdcxx-39`. The release includes their upstream source archives in `Skee-Ball-runtime-sources.zip`; these preserve upstream build files. They have not been rebuilt with the modern Xcode toolchain.

The app includes copies of notices in its `ThirdPartyNotices` directory. The matching tagged repository/source archive provides the modified MPL-covered source. Publish those source materials alongside each IPA release, including the runtime source archive rather than relying only on an upstream link.

References: [MPL 2.0 distribution requirements](https://www.mozilla.org/en-US/MPL/2.0/#responsibilities), [GNU GPL source distribution FAQ](https://www.gnu.org/licenses/gpl-faq.html#DistributeWithSourceOnInternet). These references do not establish rights to redistribute Skee-Ball's proprietary game content.
