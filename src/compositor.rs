//! Main entry point to the library.
//! See examples for documentation on how to use this struct.

use std::{env, panic, ptr, any::Any, cell::{Cell, UnsafeCell}, ffi::CStr, rc::{Rc, Weak}};

use libc;
use wayland_sys::server::{wl_display, wl_event_loop, signal::wl_signal_add, WAYLAND_SERVER_HANDLE};
use wlroots_sys::{wlr_backend_destroy, wlr_backend_start,
                  wlr_compositor, wlr_compositor_create, wlr_compositor_destroy,
                  wlr_xdg_shell_v6, wlr_xdg_shell_v6_create,
                  wlr_xdg_shell, wlr_xdg_shell_create};


use {backend::{self, UnsafeRenderSetupFunction, Backend, Session},
     data_device,
     extensions::server_decoration,
     surface::{self, Surface, InternalSurface},
     input,
     output,
     render::GenericRenderer,
     shell::{xdg_shell, xdg_shell_v6},
     xwayland,
     utils::{HandleErr, HandleResult, Handleable}};

/// Global compositor pointer, used to refer to the compositor state unsafely.
pub(crate) static mut COMPOSITOR_PTR: *mut Compositor = 0 as *mut _;

#[allow(unused_variables)]
pub trait Handler {
    /// Callback that's triggered when a surface is provided to the compositor.
    fn new_surface(&mut self,
                   compositor_handle: Handle,
                   surface_handle: surface::Handle) {}

    /// Callback that's triggered during shutdown.
    fn on_shutdown(&mut self) {}
}

impl Handler for () {}

wayland_listener!(InternalCompositor, Box<Handler>, [
    new_surface_listener => new_surface_notify: |this: &mut InternalCompositor,
                                                 surface_ptr: *mut libc::c_void,|
    unsafe {
        let handler = &mut this.data;
        let surface_ptr = surface_ptr as _;
        let compositor = (&mut *COMPOSITOR_PTR).weak_reference();
        let surface = Surface::new(surface_ptr);
        handler.new_surface(compositor.clone(), surface.weak_reference());
        let mut internal_surface = InternalSurface::new((surface, Box::new(())));
        wl_signal_add(&mut (*surface_ptr).events.commit as *mut _ as _,
                      internal_surface.on_commit_listener() as _);
        wl_signal_add(&mut (*surface_ptr).events.new_subsurface as *mut _ as _,
                      internal_surface.new_subsurface_listener() as _);
        wl_signal_add(&mut (*surface_ptr).events.destroy as *mut _ as _,
                        internal_surface.on_destroy_listener() as _);
        let surface_data = (*surface_ptr).data as *mut surface::InternalState;
        (*surface_data).surface = Box::into_raw(internal_surface);
    };
    shutdown_listener => shutdown_notify: |this: &mut InternalCompositor,
                                           _data: *mut libc::c_void,|
    unsafe {
        let handler = &mut this.data;
        handler.on_shutdown();
    };
]);

// NOTE This handle is handled differently from the others, so we can't use
// the generic `utils::Handle` implementation. This is due to how we need
// to be able to return a "full" `Compositor` for `upgrade` but that's
// impossible.
#[derive(Debug, Clone)]
pub struct Handle {
    /// This ensures that this handle is still alive and not already borrowed.
    handle: Weak<Cell<bool>>
}

#[allow(dead_code)]
pub struct Compositor {
    /// User data.
    pub data: Box<Any>,
    /// Internal compositor handler
    compositor_handler: Box<InternalCompositor>,
    /// Manager for the inputs.
    input_manager: Option<Box<input::Manager>>,
    /// Manager for the outputs.
    output_manager: Option<Box<output::Manager>>,
    /// Manager for stable XDG shells.
    xdg_shell_manager: Option<Box<xdg_shell::Manager>>,
    /// Manager for XDG shells v6.
    xdg_v6_shell_manager: Option<Box<xdg_shell_v6::Manager>>,
    /// Pointer to the xdg_shell global.
    /// If xdg_shell_manager is `None`, this value will be `NULL`.
    xdg_shell_global: *mut wlr_xdg_shell,
    /// Pointer to the xdg_shell_v6 global.
    /// If xdg_v6_shell_manager is `None`, this value will be `NULL`.
    xdg_v6_shell_global: *mut wlr_xdg_shell_v6,
    /// Pointer to the wlr_compositor.
    compositor: *mut wlr_compositor,
    /// Pointer to the wlroots backend in use.
    backend: Backend,
    /// Pointer to the wayland display.
    pub display: *mut wl_display,
    /// Pointer to the event loop.
    pub event_loop: *mut wl_event_loop,
    /// Shared memory buffer file descriptor. If the feature was not activated,
    /// this will be None.
    wl_shm_fd: Option<i32>,
    /// Name of the Wayland socket that we are binding to.
    socket_name: String,
    /// Optional decoration manager extension.
    pub server_decoration_manager: Option<server_decoration::Manager>,
    /// The renderer used to draw things to the screen.
    pub renderer: Option<GenericRenderer>,
    /// XWayland server, only Some if it is enabled
    pub xwayland: Option<xwayland::Server>,
    /// The DnD manager
    data_device_manager: Option<data_device::Manager>,
    /// The error from the panic, if there was one.
    panic_error: Option<Box<Any + Send>>,
    /// Custom function to run at shutdown (or when a panic occurs).
    user_terminate: Option<fn()>,
    /// Lock used to borrow the compositor globally.
    /// Should always be set before passing a reference to the compositor
    /// in a callback.
    pub(crate) lock: Rc<Cell<bool>>
}

#[derive(Default)]
pub struct Builder {
    compositor_handler: Option<Box<Handler>>,
    input_manager_handler: Option<Box<input::ManagerHandler>>,
    output_manager_handler: Option<Box<output::ManagerHandler>>,
    xdg_shell_manager_handler: Option<Box<xdg_shell::ManagerHandler>>,
    xdg_v6_shell_manager_handler: Option<Box<xdg_shell_v6::ManagerHandler>>,
    wl_shm: bool,
    gles2: bool,
    render_setup_function: Option<UnsafeRenderSetupFunction>,
    server_decoration_manager: bool,
    wayland_remote: Option<String>,
    x11_display: Option<String>,
    data_device_manager: bool,
    xwayland: Option<Box<xwayland::ManagerHandler>>,
    user_terminate: Option<fn()>
}

impl Builder {
    /// Make a new compositor builder.
    ///
    /// Unless otherwise noted, each option is `false`/`None`.
    pub fn new() -> Self {
        Builder::default()
    }

    /// Set the handler for global compositor callbacks.
    pub fn compositor_handler(mut self, compositor_handler: Box<Handler>) -> Self {
        self.compositor_handler = Some(compositor_handler);
        self
    }

    /// Set the handler for inputs.
    pub fn input_manager(mut self, input_manager_handler: Box<input::ManagerHandler>) -> Self {
        self.input_manager_handler = Some(input_manager_handler);
        self
    }

    /// Set the handler for outputs.
    pub fn output_manager(mut self, output_manager_handler: Box<output::ManagerHandler>) -> Self {
        self.output_manager_handler = Some(output_manager_handler);
        self
    }

    pub fn xdg_shell_manager(mut self,
                             xdg_shell_manager_handler: Box<xdg_shell::ManagerHandler>)
                             -> Self {
        self.xdg_shell_manager_handler = Some(xdg_shell_manager_handler);
        self
    }

    /// Set the handler for xdg v6 shells.
    pub fn xdg_shell_v6_manager(mut self,
                                xdg_v6_shell_manager_handler: Box<xdg_shell_v6::ManagerHandler>)
                                -> Self {
        self.xdg_v6_shell_manager_handler = Some(xdg_v6_shell_manager_handler);
        self
    }

    /// Decide whether or not to enable the wl_shm global.
    ///
    /// This is used to allocate shared memory between clients and the
    /// compositor.
    pub fn wl_shm(mut self, wl_shm: bool) -> Self {
        self.wl_shm = wl_shm;
        self
    }

    /// Decide whether or not to enable the data device manager.
    ///
    /// This is used to do DnD, or "drag 'n drop" copy paste.
    pub fn data_device(mut self, data_device_manager: bool) -> Self {
        self.data_device_manager = data_device_manager;
        self
    }

    /// Decide whether or not to enable the GLES2 extension.
    pub fn gles2(mut self, gles2_renderer: bool) -> Self {
        self.gles2 = gles2_renderer;
        self
    }

    /// Decide whether or not to enable the server decoration manager protocol
    /// extension.
    pub fn server_decoration_manager(mut self, server_decoration_manager: bool) -> Self {
        self.server_decoration_manager = server_decoration_manager;
        self
    }

    /// Add a handler for xwayland.
    ///
    /// If you do not provide a handler then the xwayland server does not run.
    pub fn xwayland(mut self, xwayland: Box<xwayland::ManagerHandler>) -> Self {
        self.xwayland = Some(xwayland);
        self
    }

    /// Add a custom function to run when shutting down the compositor
    /// or whenever a function in a callback panics.
    pub fn custom_terminate(mut self, terminate: fn()) -> Self {
        self.user_terminate = Some(terminate);
        self
    }

    /// Give an unsafe function to setup the renderer instead of the default renderer.
    pub unsafe fn render_setup_function(mut self, func: UnsafeRenderSetupFunction) -> Self {
        self.render_setup_function = Some(func);
        self
    }

    /// Makes a new compositor that handles the setup of the graphical backend
    /// (e.g, Wayland, X11, or DRM).
    ///
    /// Also automatically opens the socket for clients to communicate to the
    /// compositor with.
    pub fn build_auto<D>(self, data: D) -> Compositor
        where D: Any + 'static
    {
        unsafe {
            let display =
                ffi_dispatch!(WAYLAND_SERVER_HANDLE, wl_display_create,) as *mut wl_display;
            let event_loop =
                ffi_dispatch!(WAYLAND_SERVER_HANDLE, wl_display_get_event_loop, display);
            let backend = Backend::Multi(backend::Multi::auto_create(display as *mut _,
                                                                     self.render_setup_function));
            self.finish_build(data, display, event_loop, backend)
        }
    }

    /// Set the name of the Wayland remote socket to connect to when using the Wayland backend.
    ///
    /// (e.g. `wayland-0`, which is usually the default).
    pub fn wayland_remote(mut self, remote: String) -> Self {
        self.wayland_remote = Some(remote);
        self
    }

    /// Set the name of the X11 display socket to be used to connect to a running X11 instance for
    /// the backend.
    pub fn x11_display(mut self, remote: String) -> Self {
        self.x11_display = Some(remote);
        self
    }

    pub fn build_x11<D>(mut self, data: D) -> Compositor
        where D: Any + 'static
    {
        unsafe {
            let display =
                ffi_dispatch!(WAYLAND_SERVER_HANDLE, wl_display_create,) as *mut wl_display;
            let event_loop =
                ffi_dispatch!(WAYLAND_SERVER_HANDLE, wl_display_get_event_loop, display);
            let backend = Backend::X11(backend::X11::new(display as *mut _,
                                                       self.x11_display.take(),
                                                       self.render_setup_function));
            self.finish_build(data, display, event_loop, backend)
        }
    }

    /// Creates the compositor using an already running Wayland instance as a backend.
    ///
    /// The instance starts with no outputs.
    pub fn build_wayland<D>(mut self, data: D) -> Compositor
        where D: Any + 'static
    {
        unsafe {
            let display =
                ffi_dispatch!(WAYLAND_SERVER_HANDLE, wl_display_create,) as *mut wl_display;
            let event_loop =
                ffi_dispatch!(WAYLAND_SERVER_HANDLE, wl_display_get_event_loop, display);
            let backend = Backend::Wayland(backend::Wayland::new(display as *mut _,
                                                               self.wayland_remote.take(),
                                                               self.render_setup_function));
            self.finish_build(data, display, event_loop, backend)
        }
    }

    pub unsafe fn build_drm<D>(self,
                               data: D,
                               session: Session,
                               gpu_fd: libc::c_int,
                               parent: Option<backend::Drm>)
                               -> Compositor
        where D: Any + 'static
    {
        unsafe {
            let display =
                ffi_dispatch!(WAYLAND_SERVER_HANDLE, wl_display_create,) as *mut wl_display;
            let event_loop =
                ffi_dispatch!(WAYLAND_SERVER_HANDLE, wl_display_get_event_loop, display);
            let backend = Backend::DRM(backend::Drm::new(display as *mut _,
                                                       session,
                                                       gpu_fd,
                                                       parent,
                                                       self.render_setup_function));
            self.finish_build(data, display, event_loop, backend)
        }
    }

    pub fn build_headless<D>(self, data: D) -> Compositor
        where D: Any + 'static
    {
        unsafe {
            let display =
                ffi_dispatch!(WAYLAND_SERVER_HANDLE, wl_display_create,) as *mut wl_display;
            let event_loop =
                ffi_dispatch!(WAYLAND_SERVER_HANDLE, wl_display_get_event_loop, display);
            let backend = Backend::Headless(backend::Headless::new(display as *mut _,
                                                                 self.render_setup_function));
            self.finish_build(data, display, event_loop, backend)
        }
    }

    unsafe fn finish_build<D>(self,
                              data: D,
                              display: *mut wl_display,
                              event_loop: *mut wl_event_loop,
                              backend: Backend)
                              -> Compositor
    where D: Any + 'static {
        // Set up the wl_compositor and wl_subcompositor globals,
        // along with gles2 if that was enabled.
        let (compositor, renderer) = if self.gles2 {
            let gles2 = GenericRenderer::gles2_renderer(backend.as_ptr());
            (wlr_compositor_create(display as *mut _, gles2.as_ptr()), Some(gles2))
        } else {
            (wlr_compositor_create(display as *mut _, ptr::null_mut()), None)
        };

        // Set up shared memory buffer for Wayland clients.
        let wl_shm_fd = if self.wl_shm {
            Some(ffi_dispatch!(WAYLAND_SERVER_HANDLE,
                               wl_display_init_shm,
                               display as *mut _))
        } else {
            None
        };

        // Create optional extensions.
        let server_decoration_manager = if self.server_decoration_manager {
            server_decoration::Manager::new(display)
        } else {
            None
        };
        let data_device_manager = if self.data_device_manager {
            data_device::Manager::new(display as _)
        } else {
            None
        };

        // Set up compositor handler, if the user provided it.
        let compositor_handler = self.compositor_handler.unwrap_or_else(||Box::new(()));
        let mut compositor_handler = InternalCompositor::new(compositor_handler);
        wl_signal_add(&mut (*compositor).events.new_surface as *mut _ as _,
                    compositor_handler.new_surface_listener() as *mut _ as _);
        wl_signal_add(&mut (*compositor).events.destroy as *mut _ as _,
                        compositor_handler.shutdown_listener() as *mut _ as _);

        // Set up input manager, if the user provided it.
        let input_manager = self.input_manager_handler.map(|handler| {
            let mut input_manager = input::Manager::new(handler);
            wl_signal_add(&mut (*backend.as_ptr()).events.new_input as *mut _ as _,
                          input_manager.add_listener() as *mut _ as _);
            input_manager
        });

        // Set up output manager, if the user provided it.
        let output_manager = self.output_manager_handler.map(|handler| {
            let mut output_manager = output::Manager::new(handler);
            wl_signal_add(&mut (*backend.as_ptr()).events.new_output as *mut _ as _,
                          output_manager.add_listener() as *mut _ as _);
            output_manager
        });

        // Set up the xdg_shell handler and associated Wayland global,
        // if user provided a manager for it.
        let mut xdg_shell_global = ptr::null_mut();
        let xdg_shell_manager = self.xdg_shell_manager_handler.map(|handler| {
            xdg_shell_global = wlr_xdg_shell_create(display as *mut _);
            let mut xdg_shell_manager = xdg_shell::Manager::new(handler);
            wl_signal_add(&mut (*xdg_shell_global).events.new_surface as *mut _ as _,
                          xdg_shell_manager.add_listener() as *mut _ as _);
            xdg_shell_manager
        });

        // Set up the xdg_shell_v6 handler and associated Wayland global,
        // if user provided a manager for it.
        let mut xdg_v6_shell_global = ptr::null_mut();
        let xdg_v6_shell_manager = self.xdg_v6_shell_manager_handler.map(|handler| {
            xdg_v6_shell_global = wlr_xdg_shell_v6_create(display as *mut _);
            let mut xdg_v6_shell_manager = xdg_shell_v6::Manager::new(handler);
            wl_signal_add(&mut (*xdg_v6_shell_global).events.new_surface as *mut _ as _,
                          xdg_v6_shell_manager.add_listener() as *mut _ as _);
            xdg_v6_shell_manager
        });

        // Set up the XWayland server, if the user wants it.
        let xwayland = self.xwayland.and_then(|manager| {
            Some(xwayland::Server::new(display as _,
                                       compositor,
                                       manager,
                                       false))
        });

        let user_terminate = self.user_terminate;

        // Open the socket to the Wayland server.
        let socket = ffi_dispatch!(WAYLAND_SERVER_HANDLE, wl_display_add_socket_auto, display);
        if socket.is_null() {
            // NOTE Rationale for panicking:
            // * Won't be in C land just yet, so it's safe to panic
            // * Can always be returned in a Result instead, but for now
            //   if you auto create it's assumed you can't recover.
            panic!("Unable to open wayland socket");
        }
        let socket_name = CStr::from_ptr(socket).to_string_lossy().into_owned();
        wlr_log!(WLR_DEBUG,
                 "Running compositor on wayland display {}",
                 socket_name);
        env::set_var("_WAYLAND_DISPLAY", socket_name.clone());
        let compositor = Compositor { data: Box::new(data),
                                      compositor_handler,
                                      socket_name,
                                      input_manager,
                                      output_manager,
                                      xdg_shell_manager,
                                      xdg_shell_global,
                                      xdg_v6_shell_manager,
                                      xdg_v6_shell_global,
                                      data_device_manager,
                                      compositor,
                                      backend,
                                      display,
                                      event_loop,
                                      wl_shm_fd,
                                      server_decoration_manager,
                                      renderer,
                                      xwayland,
                                      user_terminate,
                                      panic_error: None,
                                      lock: Rc::new(Cell::new(false)) };
        compositor.set_lock(true);
        compositor
    }
}

impl Compositor {
    /// Creates a weak reference to the `Compositor`.
    pub fn weak_reference(&self) -> Handle {
        let handle = Rc::downgrade(&self.lock);
        Handle { handle }
    }

    /// Enters the wayland event loop. Won't return until the compositor is
    /// shut off
    pub fn run(self) {
        self.run_with(|_| unsafe {
                          ffi_dispatch!(WAYLAND_SERVER_HANDLE,
                                        wl_display_run,
                                        (*COMPOSITOR_PTR).display);
                      })
    }

    /// Prepare to enter the wayland event loop. Instead of calling
    /// `wl_display_run`, the provided callback function is invoked. Allows
    /// integration with a different event loop.
    pub fn run_with<F>(self, runner: F)
        where F: FnOnce(&Compositor)
    {
        unsafe {
            self.set_lock(false);
            let compositor = UnsafeCell::new(self);
            if COMPOSITOR_PTR != 0 as _ {
                // NOTE Rationale for panicking:
                // * Nicer than an abort
                // * Not yet in C land
                panic!("A compositor is already running!")
            }
            COMPOSITOR_PTR = compositor.get();
            wlr_log!(WLR_INFO, "Starting compositor");
            if !wlr_backend_start((*compositor.get()).backend.as_ptr()) {
                wlr_backend_destroy((*compositor.get()).backend.as_ptr());
                // NOTE Rationale for panicking:
                // * Won't be in C land just yet, so it's safe to panic
                // * Can always be returned in a Result instead, but for now
                //   if you auto create it's assumed you can't recover.
                panic!("Failed to start backend");
            }
            env::set_var("WAYLAND_DISPLAY", (*COMPOSITOR_PTR).socket_name.clone());
            runner(&*COMPOSITOR_PTR);
            match (*compositor.get()).panic_error.take() {
                None => {}
                Some(err) => {
                    // A panic occured, now we can re-throw it safely.
                    ::std::panic::resume_unwind(err)
                }
            }
        }
    }

    /// Shutdown the wayland server
    fn terminate(&mut self) {
        unsafe {
            ffi_dispatch!(WAYLAND_SERVER_HANDLE, wl_display_terminate, self.display);
        }
    }

    /// Get a reference to the currently running backend.
    pub fn backend(&self) -> &Backend {
        &self.backend
    }

    /// Get a mutable reference to the currently running backend.
    pub fn backend_mut(&mut self) -> &mut Backend {
        &mut self.backend
    }

    /// Saves the panic error information in the compositor, to be re-thrown
    /// later when we are out of the C callback stack.
    pub(crate) fn save_panic_error(&mut self, error: Box<Any + Send>) {
        self.panic_error = Some(error);
    }

    /// Manually set hte lock used to determine if a double-borrow is occuring on this structure.
    ///
    /// # Panics
    /// Panics when trying to set the lock on an upgraded handle.
    unsafe fn set_lock(&self, val: bool) {
        self.lock.set(val)
    }
}

impl Drop for Compositor {
    fn drop(&mut self) {
        unsafe {
            ffi_dispatch!(WAYLAND_SERVER_HANDLE,
                          wl_display_destroy_clients,
                          self.display);
            wlr_compositor_destroy(self.compositor)
        }
    }
}

impl Handle {
    /// Constructs a new `compositor::Handle` that is always invalid. Calling `run` on this
    /// will always fail.
    ///
    /// This is useful for pre-filling a value before it's provided by the server, or
    /// for mocking/testing.
    pub fn new() -> Self {
        Handle { handle: Weak::new() }
    }

    /// Upgrades the compositor handle to a reference to the backing `Compositor`.
    ///
    /// # Unsafety
    /// To be honest this function is probably safe.
    ///
    /// However, the `compositor::Handle` will behave like the other handles in order
    /// to reduce confusion.
    unsafe fn upgrade(&self) -> HandleResult<&mut Compositor> {
        self.handle.upgrade()
            .ok_or(HandleErr::AlreadyDropped)
            // NOTE
            // We drop the Rc here because having two would allow a dangling
            // pointer to exist!
            .and_then(|check| {
                if check.get() {
                    return Err(HandleErr::AlreadyBorrowed)
                }
                if COMPOSITOR_PTR.is_null() {
                    return Err(HandleErr::AlreadyDropped)
                }
                check.set(true);
                Ok(&mut *COMPOSITOR_PTR)
            })
    }

    /// Run a function on the referenced `Compositor`, if it still exists.
    ///
    /// Returns the result of the function, if successful.
    ///
    /// # Safety
    /// By enforcing a rather harsh limit on the lifetime of the Compositor
    /// to a short lived scope of an anonymous function,
    /// this function ensures the Compositor does not live longer
    /// than it exists.
    ///
    /// # Panics
    /// This function will panic if multiple mutable borrows are detected.
    /// This will happen if you call `upgrade` directly within this callback,
    /// or if you run this function within the another run to the same `Output`.
    ///
    /// So don't nest `run` calls and everything will be ok :).
    pub fn run<F, R>(&self, runner: F) -> HandleResult<R>
        where F: FnOnce(&mut Compositor) -> R
    {
        let compositor = unsafe { self.upgrade()? };
        let res = panic::catch_unwind(panic::AssertUnwindSafe(|| runner(compositor)));
        self.handle.upgrade().map(|check| {
                                      // Sanity check that it hasn't been tampered with.
                                      if !check.get() {
                                          wlr_log!(WLR_ERROR,
                                                   "After running compositor callback, mutable \
                                                    lock was false");
                                          panic!("Compositor lock in incorrect state!");
                                      }
                                      check.set(false)
                                  });
        match res {
            Ok(res) => Ok(res),
            Err(err) => panic::resume_unwind(err)
        }
    }
}

/// Terminates the compositor and execute any user clean up code.
pub fn terminate() {
    unsafe {
        if COMPOSITOR_PTR != 0 as _ {
            let compositor = &mut *COMPOSITOR_PTR;
            compositor.terminate();
            compositor.user_terminate.map(|f| f());
        }
    }
}

/// Gets a handle to the compositor.
///
/// If the compositor has not started running yet, or if it has stopped,
/// then this function will return None.
pub fn handle() -> Option<Handle> {
    unsafe {
        if COMPOSITOR_PTR.is_null() {
            None
        } else {
            Some((&mut *COMPOSITOR_PTR).weak_reference())
        }
    }
}
