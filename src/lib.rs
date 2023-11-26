use serde::Deserialize;
use std::borrow::Cow;
use tokio::sync::{mpsc, oneshot};
use winit::{
    dpi::LogicalSize,
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};
use wry::{raw_window_handle::HasRawWindowHandle, WebView, WebViewBuilder};

#[derive(Clone)]
pub struct Element {
    id: u64,
    document: Document,
}

impl Element {
    pub async fn append_child(&self, child: &Element) {
        let (tx, rx) = oneshot::channel();
        self.document
            .tx
            .send(Request::AppendChild {
                parent_id: self.id,
                child_id: child.id,
                tx: Some(tx),
            })
            .unwrap();
        rx.await.unwrap()
    }

    pub async fn set_text_content(&self, content: impl Into<Cow<'static, str>>) {
        let (tx, rx) = oneshot::channel();
        self.document
            .tx
            .send(Request::SetText {
                id: self.id,
                content: content.into(),
                tx: Some(tx),
            })
            .unwrap();
        rx.await.unwrap()
    }
}

#[derive(Deserialize)]
#[serde(tag = "kind", content = "data")]
enum Message {
    CreateNode { id: u64 },
    AppendChild,
    SetText,
}

enum Request {
    AppendChild {
        parent_id: u64,
        child_id: u64,
        tx: Option<oneshot::Sender<()>>,
    },
    CreateElement {
        name: Cow<'static, str>,
        tx: Option<oneshot::Sender<u64>>,
    },
    CreateTextElement {
        content: Cow<'static, str>,
        tx: Option<oneshot::Sender<u64>>,
    },
    SetText {
        id: u64,
        content: Cow<'static, str>,
        tx: Option<oneshot::Sender<()>>,
    },
}

#[derive(Clone)]
pub struct Document {
    tx: mpsc::UnboundedSender<Request>,
}

impl Document {
    pub fn body(&self) -> Element {
        Element {
            id: 0,
            document: self.clone(),
        }
    }
    pub async fn create_element(&self, name: impl Into<Cow<'static, str>>) -> Element {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(Request::CreateElement {
                name: name.into(),
                tx: Some(tx),
            })
            .unwrap();
        let id = rx.await.unwrap();
        Element {
            id,
            document: self.clone(),
        }
    }

    pub async fn create_text_node(&self, content: impl Into<Cow<'static, str>>) -> Element {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(Request::CreateTextElement {
                content: content.into(),
                tx: Some(tx),
            })
            .unwrap();
        let id = rx.await.unwrap();
        Element {
            id,
            document: self.clone(),
        }
    }
}

pub struct HtmlView {
    web_view: WebView,
    rx: mpsc::UnboundedReceiver<Message>,
    req_rx: mpsc::UnboundedReceiver<Request>,
    pending: Option<Request>,
    document: Document,
}

impl HtmlView {
    pub fn new(window: &impl HasRawWindowHandle) -> wry::Result<Self> {
        let builder = WebViewBuilder::new(&window);

        let (tx, rx) = mpsc::unbounded_channel();

        let web_view =
            builder
                .with_html("<html><body></body></html>")?
                .with_ipc_handler(move |s| {
                    let msg = serde_json::from_str(&s).unwrap();
                    tx.send(msg).unwrap();
                })
                .build()?;

        web_view
            .evaluate_script(&format!(
                r#"
                window.webSlinger = {{
                    elements: {{
                        0: document.body
                    }},
                    nextId: 1
                }}
            "#
            ))
            .unwrap();

        let (req_tx, req_rx) = mpsc::unbounded_channel();
        let document = Document { tx: req_tx };

        Ok(Self {
            web_view,
            rx,
            req_rx,
            pending: None,
            document,
        })
    }

    pub fn document(&self) -> Document {
        self.document.clone()
    }

    pub fn poll(&mut self) {
        if let Some(ref mut req) = self.pending {
            match req {
                Request::AppendChild {
                    parent_id: _,
                    child_id: _,
                    tx,
                } => {
                    if let Ok(msg) = self.rx.try_recv() {
                        match msg {
                            Message::AppendChild => {
                                tx.take().unwrap().send(()).unwrap();
                            }
                            _ => todo!(),
                        }

                        self.pending = None;
                    } else {
                        return;
                    }
                }
                Request::CreateElement { name: _, tx } => {
                    if let Ok(msg) = self.rx.try_recv() {
                        match msg {
                            Message::CreateNode { id } => {
                                tx.take().unwrap().send(id).unwrap();
                            }
                            _ => todo!(),
                        }

                        self.pending = None;
                    } else {
                        return;
                    }
                }
                Request::CreateTextElement { content: _, tx } => {
                    if let Ok(msg) = self.rx.try_recv() {
                        match msg {
                            Message::CreateNode { id } => {
                                tx.take().unwrap().send(id).unwrap();
                            }
                            _ => todo!(),
                        }

                        self.pending = None;
                    } else {
                        return;
                    }
                }
                Request::SetText {
                    id: _,
                    content: _,
                    tx,
                } => {
                    if let Ok(msg) = self.rx.try_recv() {
                        match msg {
                            Message::SetText => {
                                tx.take().unwrap().send(()).unwrap();
                            }
                            _ => todo!(),
                        }

                        self.pending = None;
                    } else {
                        return;
                    }
                }
            }
        }

        if let Ok(req) = self.req_rx.try_recv() {
            match &req {
                Request::AppendChild {
                    parent_id,
                    child_id,
                    tx: _,
                } => {
                    self.web_view
                        .evaluate_script(&format!(
                            r#"
                            var parent = window.webSlinger.elements[{parent_id}];
                            var child = window.webSlinger.elements[{child_id}];
                            parent.appendChild(child);

                            window.ipc.postMessage(JSON.stringify({{ kind: "AppendChild" }}));
                        "#
                        ))
                        .unwrap();
                }
                Request::CreateElement { name, tx: _ } => {
                    self.web_view
                    .evaluate_script(&format!(
                        r#"
                            let element = document.createElement("{name}");
                            
                            var id = window.webSlinger.nextId;
                            window.webSlinger.nextId += 1;
        
                            window.webSlinger.elements[id] = element;
                            window.ipc.postMessage(JSON.stringify({{ kind: "CreateNode", data: {{ id: id }} }}));
                        "#
                    ))
                    .unwrap();
                }
                Request::CreateTextElement { content, tx: _ } => {
                    self.web_view
                    .evaluate_script(&format!(
                        r#"
                            var node = document.createTextNode("{content}");
                            
                            var id = window.webSlinger.nextId;
                            window.webSlinger.nextId += 1;
        
                            window.webSlinger.elements[id] = node;
                            window.ipc.postMessage(JSON.stringify({{ kind: "CreateNode", data: {{ id: id }} }}));
                        "#
                    ))
                    .unwrap();
                }
                Request::SetText { id, content, tx: _ } => {
                    self.web_view
                        .evaluate_script(&format!(
                            r#"
                            var node = window.webSlinger.elements[{id}];
                            node.nodeValue = {content};

                            window.ipc.postMessage(JSON.stringify({{ kind: "SetText" }}));
                        "#
                        ))
                        .unwrap();
                }
            }
            self.pending = Some(req);
        }
    }

    pub fn run(mut self) -> wry::Result<()> {
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

        let event_loop = EventLoop::new().unwrap();
        let _window = WindowBuilder::new()
            .with_inner_size(LogicalSize::new(800, 800))
            .build(&event_loop)
            .unwrap();

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

                self.poll();

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
}
