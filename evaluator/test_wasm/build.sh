rustc gen.rs -C linker-plugin-lto -O -g --target=wasm32-wasi
rustc eval.rs -C linker-plugin-lto -O -g --target=wasm32-wasi
rustc sub_ac.rs -C linker-plugin-lto -O -g --target=wasm32-wasi
rustc sub_wa.rs -C linker-plugin-lto -O -g --target=wasm32-wasi
rustc sub_rte.rs -C linker-plugin-lto -O -g --target=wasm32-wasi
rustc sub_tle.rs -C linker-plugin-lto -g --target=wasm32-wasi
rustc sub_mle.rs -C linker-plugin-lto -g --target=wasm32-wasi

