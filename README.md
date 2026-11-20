# MatataView

A lightweight Windows image viewer for JPG and PNG files.

## Run

Install the Rust toolchain, then run:

```sh
cargo run -- path/to/image.jpg
```

You can also start the application without a file and open one with **Open…**
or by dropping an image into the window. Use **Ctrl+O** to open a file, and the
zoom buttons or **Fit** to adjust its size.

To build a Windows executable, run `cargo build --release` on Windows or build
for a configured Windows Rust target.