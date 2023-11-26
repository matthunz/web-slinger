# web-slinger

```rust
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
```
