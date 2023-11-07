rustc gen.rs -C strip=symbols -C linker-plugin-lto -C opt-level=z --target=wasm32-wasi
rustc sub_ac.rs -C strip=symbols -C linker-plugin-lto -C opt-level=z --target=wasm32-wasi
rustc eval.rs -C strip=symbols -C linker-plugin-lto -C opt-level=z --target=wasm32-wasi
