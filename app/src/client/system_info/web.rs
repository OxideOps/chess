pub(crate) fn get_num_cores() -> usize {
    let js_result = js_sys::eval("window.navigator.hardwareConcurrency").unwrap();
    js_result.as_f64().unwrap_or(1.0) as usize
}

#[allow(dead_code)]
pub(crate) fn get_total_ram() -> usize {
    let js_result = js_sys::eval("window.navigator.deviceMemory").unwrap();
    js_result.as_f64().map_or(0, |gb| (1000000.0 * gb) as usize)
}
