#![allow(non_camel_case_types, non_upper_case_globals)]
#![allow(clippy::all)]

pub extern crate libc;
pub extern crate wayland_commons;
pub extern crate wayland_server;
pub extern crate wayland_client;
pub extern crate wayland_sys;
pub extern crate wayland_protocols;

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
            ($protocol_name:ident, {$($client_deps:path),*}, {$($server_deps:path),*}) => {
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
                        
                        $(
                            use $client_deps;
                        )*

                        include!(concat!(env!("OUT_DIR"), "/", stringify!($protocol_name), "_client_api.rs"));
                    }

                    pub mod server{
                        // Common server dependencies:
                        use wayland_commons::wire::*;
                        use wayland_commons::map::*;
                        use wayland_commons::smallvec;
                        use wayland_server::*;
                        use wayland_server::Main;
                        use wayland_server::protocol::{wl_surface, wl_output};
                        use wayland_server::protocol::wl_seat as wl_seat;
                        use wayland_client::{AnonymousObject};
                        use wayland_sys as sys;
                        use wayland_client::{ProxyMap, Proxy};

                        $(
                            use $server_deps;
                        )*

                        include!(concat!(env!("OUT_DIR"), "/", stringify!($protocol_name), "_server_api.rs"));
                    }
                }
            };
        }

        pub use wayland_protocols::wlr::unstable::gamma_control::v1 as gamma_control;
        pub use wayland_protocols::wlr::unstable::data_control::v1 as data_control;
        pub use wayland_protocols::wlr::unstable::export_dmabuf::v1 as export_dmabuf;
        pub use wayland_protocols::wlr::unstable::foreign_toplevel::v1 as foreign_toplevel;
        expose_protocol!(gtk_primary_selection, {}, {});
        expose_protocol!(idle, {}, {});
        pub use wayland_protocols::wlr::unstable::input_inhibitor;
        expose_protocol!(
            input_method,
            {
                wayland_protocols::unstable::text_input::v3::client::zwp_text_input_v3,
                wayland_server::protocol::wl_keyboard
            },
            {
                wayland_protocols::unstable::text_input::v3::server::zwp_text_input_v3,
                wayland_server::protocol::wl_keyboard
            }
        );
        pub use wayland_protocols::wlr::unstable::layer_shell::v1 as layer_shell;
        expose_protocol!(output_management, {}, {});
        expose_protocol!(output_power_management, {}, {});
        pub use wayland_protocols::wlr::unstable::screencopy::v1 as screencopy;
        expose_protocol!(server_decoration, {}, {});
        expose_protocol!(virtual_keyboard, {}, {});
        expose_protocol!(
            virtual_pointer,
            {
                wayland_client::protocol::wl_pointer
            },
            {
                wayland_server::protocol::wl_pointer
            }
        );
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
