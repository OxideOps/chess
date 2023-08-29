pub struct CommandConfig {
    pub cmd: &'static str,
    pub args: Option<&'static [&'static str]>,
    pub dir: Option<&'static str>,
}
