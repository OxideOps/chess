pub(crate) fn inv_sigmoid(x: f64) -> f64 { (x / (1.0 - x)).ln() }

pub(crate) fn sigmoid(x: f64) -> f64 { 1.0 / (1.0 + (-x).exp()) }
