use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    pub fn log(s: &str);

    // #[wasm_bindgen(js_namespace = console)]
    // fn table(s: &JsValue);

    #[wasm_bindgen(js_namespace = console, js_name = log)]
    pub fn log_u32(a: u32);

    #[wasm_bindgen(js_namespace = console, js_name = log)]
    pub fn log_many(a: &str, b: &str);
}

// #[no_warn(unused)]
macro_rules! console_log {
    ($($t:tt)*) => (crate::console::log(&format_args!($($t)*).to_string()))
}

pub(crate) use console_log;
