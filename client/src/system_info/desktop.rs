pub(crate) fn get_num_cores() -> usize {
    num_cpus::get()
}

pub(crate) fn get_total_ram() -> usize {
    sys_info::mem_info().map_or(0, |info| info.total as usize)
}
