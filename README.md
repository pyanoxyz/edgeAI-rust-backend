# rust-backend
Clean the project

```
cargo clean
```
Removes the target directory generated during compilation, freeing up space and ensuring a fresh build the next time.
Build the project

```
cargo build
```
Compiles the current project in debug mode (non-optimized), generating the build artifacts in the target/debug directory.
Run the app in auto-reload mode

```
cargo watch -x run
```
Monitors the source code for changes and automatically rebuilds and runs the project whenever changes are detected. This is useful for live development.
Remove a crate

```
cargo remove chromadb
```
Removes the specified crate (chromadb in this case) from your Cargo.toml file and uninstalls it from your dependencies.
Build the project for release

```
cargo build --release
```
Compiles the project in release mode, which enables optimizations. The output is stored in the target/release directory, suitable for production use due to better performance.
These commands cover essential actions like cleaning, building, running, removing dependencies, and creating optimized builds.