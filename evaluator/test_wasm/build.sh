rustc gen.rs -C linker-plugin-lto -O -g --target=wasm32-wasi
rustc sub_ac.rs -C linker-plugin-lto -O -g --target=wasm32-wasi
rustc eval.rs -C linker-plugin-lto -O -g --target=wasm32-wasi
