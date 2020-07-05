## wlroots-rs

A fork of wlroots-rs.

### Changes

The following changes have been merged into master:

- Exposed (updated to reflect) all wlr/protocols currently available [here](https://github.com/swaywm/wlroots/tree/master/protocol)
- Exposed (updated to reflect) all wlr/types currently available [here](https://github.com/swaywm/wlroots/tree/master/types)
- Updated all dependencies of wlroots-sys to the current versions (including the library suite “wayland-rs” to >=0.25.*)
- Fixed a bug where wlroots-sys would not compile. The multi-crate issue is described [here](https://users.rust-lang.org/t/unable-to-compile-syntex-syntax-using-rust-1-41/37710).
- Fixed a bug where the wlroots-sys crate would recompile every time you compile another crate that depends on wlroots-sys
- Added informative dependency checks as well as more constructive error messages for pkg-config when building default, static and unstable, for a smoother build process.
- Fixed errors with static build with new version of wlroots.
- Compatibility changes for wlroots-rs
