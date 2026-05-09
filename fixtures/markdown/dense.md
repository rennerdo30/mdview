# Dense Markdown Fixture

This paragraph combines **strong**, *emphasis*, ***strong emphasis***, `inline code`, ~~strikethrough~~, [a link](https://example.com), and a footnote reference.[^dense]

> A blockquote with `inline code`, **bold text**, and enough content to wrap across multiple lines in narrow reading modes.

- [x] Completed task
- [ ] Pending task
  - Nested unordered item with a very long phrase that should wrap without overlapping the marker or following content.

1. Ordered item
2. Ordered item with [inline link](https://example.com/ordered)

| Feature | Status | Notes |
| ------- | ------ | ----- |
| Tables | Ready | Cells should wrap cleanly |
| Code | Ready | `inline` and fenced blocks |

```rust
fn main() {
    println!("dense fixture");
}
```

Term
: Definition text that should align and space correctly.

---

Inline math $a + b = c$ and display math:

$$
x = y + z
$$

<section data-fixture="dense">Raw HTML fallback</section>

[^dense]: Footnote body with a [link](https://example.com/footnote).
