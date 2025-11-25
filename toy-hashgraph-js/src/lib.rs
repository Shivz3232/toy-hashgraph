mod utils;

use js_sys::{Map, Uint8Array};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct Hashgraph {
    inner: toy_hashgraph::Hashgraph,
}

#[wasm_bindgen]
impl Hashgraph {
    #[wasm_bindgen(constructor)]
    pub fn new(name: String, private_key: &[u8], public_keys: Map) -> Self {
        let mut public_keys_vec = Vec::new();
        public_keys.for_each(&mut |value, key| {
            let key = key
                .as_string()
                .expect("public_keys should be a map with strings as keys");
            let value = value
                .dyn_into::<Uint8Array>()
                .expect("public_keys should be a map with Uint8Arrays as values");
            if value.length() != 32 {
                panic!("public_keys should be a map with Uint8Arrays of length 32 as values")
            }

            let mut buffer = [0; 32];
            value.copy_to(&mut buffer);

            public_keys_vec.push((key, buffer));
        });

        Hashgraph {
            inner: toy_hashgraph::Hashgraph::new(name, private_key, public_keys_vec),
        }
    }
}

#[wasm_bindgen(start)]
fn run() {
    utils::set_panic_hook();
}
