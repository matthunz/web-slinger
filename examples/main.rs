use std::time::Duration;
use web_slinger::HtmlView;
use winit::{
    dpi::LogicalSize,
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

#[tokio::main]
async fn main() -> wry::Result<()> {
    let event_loop = EventLoop::new().unwrap();
    let window =
        WindowBuilder::new()
            .with_inner_size(LogicalSize::new(800, 800))
            .build(&event_loop)
            .unwrap();

    let mut html_view = HtmlView::new(&window).unwrap();
    let document = html_view.document();
    tokio::spawn(async move {
        let node = document.create_text_node("0").await;
        document.body().append_child(&node).await;

        let mut count = 0;
        loop {
            count += 1;
            node.set_text_content(count.to_string()).await;
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
    });

    #[cfg(any(
        target_os = "linux",
        target_os = "dragonfly",
        target_os = "freebsd",
        target_os = "netbsd",
        target_os = "openbsd",
    ))]
    {
        use gtk::prelude::DisplayExtManual;

        gtk::init().unwrap();
        if gtk::gdk::Display::default().unwrap().backend().is_wayland() {
            panic!("This example doesn't support wayland!");
        }

        // we need to ignore this error here otherwise it will be catched by winit and will be
        // make the example crash
        winit::platform::x11::register_xlib_error_hook(Box::new(|_display, error| {
            let error = error as *mut x11_dl::xlib::XErrorEvent;
            (unsafe { (*error).error_code }) == 170
        }));
    }

    event_loop
        .run(move |event, evl| {
            evl.set_control_flow(ControlFlow::Poll);

            #[cfg(any(
                target_os = "linux",
                target_os = "dragonfly",
                target_os = "freebsd",
                target_os = "netbsd",
                target_os = "openbsd",
            ))]
            while gtk::events_pending() {
                gtk::main_iteration_do(false);
            }

            html_view.poll();

            match event {
                #[cfg(any(
                    target_os = "linux",
                    target_os = "dragonfly",
                    target_os = "freebsd",
                    target_os = "netbsd",
                    target_os = "openbsd",
                ))]
                Event::WindowEvent {
                    event: WindowEvent::Resized(size),
                    ..
                } => {
                    _webview.set_bounds(wry::Rect {
                        x: 0,
                        y: 0,
                        width: size.width,
                        height: size.height,
                    });
                }
                Event::WindowEvent {
                    event: WindowEvent::CloseRequested,
                    ..
                } => evl.exit(),
                _ => {}
            }
        })
        .unwrap();

    Ok(())
}
