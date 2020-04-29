# wlroots-sys

( **For devops course, see the [reflection document](https://github.com/perfah/wlroots-rs/wiki/devops-course:-Reflection-&-Contributions)** )

A fork intended for standalone use of the wlroots-sys subcrate (that generates "raw" Rust bindings for [wlroots](https://github.com/swaywm/wlroots) via bindgen). This effectively means that progress and development is contained only within the [wlroots-sys](https://github.com/perfah/wlroots-rs/tree/master/wlroots-sys) directory. Note that this is not an attempt to revive wlroots-rs - hence the largely untouched repository root. 

The motivation for this fork is the benefit of being able to write Wayland-compositors based on wlroots in Rust more easily (without relying on outdated code).

## Usage

1. `git clone https://github.com/perfah/wlroots-rs.git`
2. `cd wlroots-rs`
3. `git checkout master`
4. `cp ./wlroots-sys <path_to_your_project>/wlroots-sys`
5. Include a crate dependency to your project's `cargo.toml`-file:
        
        [dependencies]
        wlroots-sys = { path = "wlroots-sys", features = [...] }
6. Replace "..." with the optional dependencies you would like.

## Dependencies

- Required dependencies:

        meson
        wayland
        wayland-protocols
        EGL
        GLESv2
        libdrm
        GBM
        libinput
        xkbcommon
        udev 

- Optional dependencies (that can be added as features):   
    - `systemd` - support for logind
    - `elogind` - support for logind without systemd installed
    - `libcap` - capabilities
    
## Changes

- Updated all dependencies of wlroots-sys to the current versions (including the library suite “wayland-rs” to 0.25.*)
- Fixed a bug where wlroots-sys would not compile. The multi-crate issue is described [here](https://users.rust-lang.org/t/unable-to-compile-syntex-syntax-using-rust-1-41/37710).
- Fixed a bug where the wlroots-sys crate would recompile every time you compile another crate that depends on wlroots-sys
- Update the exposed wlr/types to reflect the current ones available [here](https://github.com/swaywm/wlroots/tree/master/types)
- Prepare update of the exposed protocols to reflect the current ones available [here](https://github.com/swaywm/wlroots/tree/master/protocol)
- Added informative dependency checks as well as more constructive error messages for pkg-config when building default, static and unstable, for a smoother build process.
- Fixed errors with static build with new version of wlroots.
