# HTTP Proxy Server

This code demonstrated how to implement an http proxy server in Rust.

# Usage

```rust
cargo run --release 127.0.0.1:1081
curl -x 127.0.0.1:1081 https://httpbin.org/get
```
