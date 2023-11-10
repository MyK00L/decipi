rustc gen.rs -C linker-plugin-lto -C opt-level=2 --target=wasm32-wasi
rustc sub_ac.rs -C linker-plugin-lto -C opt-level=2 --target=wasm32-wasi
rustc eval.rs -C linker-plugin-lto -C opt-level=2 --target=wasm32-wasi
