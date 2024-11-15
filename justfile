run:
    RUST_LOG=OFF,worldline=TRACE cargo run --release -F no_vsync

run_vsync:
    RUST_LOG=OFF,worldline=TRACE cargo run --release

build:
    RUST_LOG=OFF,worldline=TRACE cargo build --release