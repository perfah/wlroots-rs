#![allow(non_camel_case_types, non_upper_case_globals)]
#![allow(clippy::all)]

pub extern crate libc;
pub extern crate wayland_commons;
pub extern crate wayland_server;
pub extern crate wayland_client;
pub extern crate wayland_sys;

pub use wayland_sys::{
    *, gid_t,
    pid_t,
    server::{self, WAYLAND_SERVER_HANDLE}, uid_t
};

pub use self::generated::*;

#[allow(non_camel_case_types, non_snake_case, non_upper_case_globals)]
mod generated {
    use libc;

    include!("gen.rs");

    // XXX: If you add another protocols, take a look at wayland_protocol! macro
    // from `wayland-rs/wayland-protocols/src/protocol_macro.rs`.
    pub mod protocols {
        #![allow(unused)]

        macro_rules! expose_protocol {
            ($protocol_name:ident, $($client_deps:tt)*) => {
                pub mod $protocol_name{
                    pub mod client{
                        // Common client dependencies:
                        use wayland_commons::wire::*;
                        use wayland_commons::map::*;
                        use wayland_commons::smallvec;
                        use wayland_server::*;
                        use wayland_server::protocol::wl_surface;
                        use wayland_server::protocol::wl_seat as wl_seat;
                        use wayland_client::{Main, AnonymousObject, protocol::wl_output};
                        use wayland_sys as sys;
                        use wayland_client::{ProxyMap, Proxy};

                        $($client_deps)*

                        include!(concat!(env!("OUT_DIR"), "/", stringify!($protocol_name), "_client_api.rs"));
                    }

                    pub mod server{
                        // Common server dependencies:
                        use wayland_commons::wire::*;
                        use wayland_commons::map::*;
                        use wayland_commons::smallvec;
                        use wayland_server::*;
                        use wayland_server::Main;
                        use wayland_server::protocol::wl_surface;
                        use wayland_server::protocol::wl_seat as wl_seat;
                        use wayland_client::{AnonymousObject, protocol::wl_output};
                        use wayland_sys as sys;
                        use wayland_client::{ProxyMap, Proxy};
                        //use generated::protocols::gamma_control::server::zwlr_gamma_control_v1::ZwlrGammaControlV1;
                        //use generated::protocols::layer_shell::server::zwlr_layer_shell_v1::Layer;

                        //$additional_deps;

                        include!(concat!(env!("OUT_DIR"), "/", stringify!($protocol_name), "_server_api.rs"));
                    }
                }
            };
        }

        //expose_protocol!(gamma_control, {});
        //expose_protocol!(data_control, {});
        //expose_protocol!(export_dmabuf, {});
        //expose_protocol!(foreign_toplevel_management, {});
        //expose_protocol!(gtk_primary_selection, {});
        //expose_protocol!(idle, {});
        //expose_protocol!(input_inhibitor, {});
        //expose_protocol!(input_method, {});
        //expose_protocol!(layer_shell, {});
        //expose_protocol!(output_management, {});
        //expose_protocol!(output_power_management, {});
        //expose_protocol!(screencopy, {});
        //expose_protocol!(server_decoration, {});
        //expose_protocol!(virtual_keyboard, {});
        //expose_protocol!(virtual_pointer, {});
    }
}

#[cfg(feature = "unstable")]
pub type wlr_output_events = self::generated::wlr_output__bindgen_ty_1;
#[cfg(feature = "unstable")]
pub type wlr_input_device_pointer = self::generated::wlr_input_device__bindgen_ty_1;

pub trait TransformOutput {
    fn invert(self) -> Self;
    fn compose(self, other: Self) -> Self;
}

#[cfg(feature = "unstable")]
impl TransformOutput for wl_output_transform {
    /// Returns the transform that, when composed with `self`, gives
    /// `WL_OUTPUT_TRANSFORM_NORMAL`.
    fn invert(self) -> Self {
        unsafe { wlr_output_transform_invert(self) }
    }

    /// Returns a transform that, when applied, has the same effect as applying
    /// sequentially `self` and `other`.
    fn compose(self, other: Self) -> Self {
        unsafe { wlr_output_transform_compose(self, other) }
    }
}
