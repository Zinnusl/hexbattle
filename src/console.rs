use wasm_bindgen::prelude::*;

cfg_if::cfg_if! {
    if #[cfg(target_family = "wasm")] {
        #[wasm_bindgen]
        extern "C" {
            #[wasm_bindgen(js_namespace = console)]
            pub fn log(s: &str);

            #[wasm_bindgen(js_namespace = console, js_name = log)]
            pub fn log_u32(a: u32);

            #[wasm_bindgen(js_namespace = console, js_name = log)]
            pub fn log_many(a: &str, b: &str);
        }

        macro_rules! console_log {
            ($($t:tt)*) => (crate::console::log(&format_args!($($t)*).to_string()))
        }

        pub(crate) use console_log;
    } else {
        pub fn log(s: &str) {
            println!("{}", s);
        }

        pub fn log_u32(a: u32) {
            println!("{}", a);
        }

        pub fn log_many(a: &str, b: &str) {
            println!("{} {}", a, b);
        }

        macro_rules! console_log {
            ($($t:tt)*) => (crate::console::log(&format_args!($($t)*).to_string()))
        }

        pub(crate) use console_log;
    }
}
