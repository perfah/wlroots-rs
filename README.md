# wlroots_sys

A fork intended for standalone use of the wlroots-sys subcrate (that generates "raw" Rust bindings for [wlroots](https://github.com/swaywm/wlroots) via bindgen). This effectively means that progress and development is contained only within the [wlroots-sys](https://github.com/perfah/wlroots-rs/tree/master/wlroots-sys) directory. Note that this is not an attempt to revive wlroots-rs - hence the largely untouched repository root. 

The motivation for this fork is the benefit of being able to write Wayland-compositors based on wlroots in Rust more easily. 

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
    - `pixman` - support for pixman functionality
    - `systemd` - support for logind
    - `elogind` - support for logind without systemd installed
    - `libcap` - capabilities

## Credit



