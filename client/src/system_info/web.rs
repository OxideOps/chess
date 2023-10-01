pub(crate) fn get_num_cores() -> usize {
    let js_result = js_sys::eval("window.navigator.hardwareConcurrency").unwrap();
    js_result.as_f64().unwrap_or(1.0) as usize
}
