use wasm_bindgen::UnwrapThrowExt;
use web_sys::Storage;

fn local_storage() -> Storage {
    web_sys::window()
        .unwrap_throw()
        .local_storage()
        .unwrap_throw()
        .unwrap_throw()
}

pub fn set_item(key: &str, value: &str) {
    local_storage().set_item(key, value).unwrap_throw()
}

pub fn get_item(key: &str) -> Option<String> {
    local_storage().get_item(key).unwrap_throw()
}
