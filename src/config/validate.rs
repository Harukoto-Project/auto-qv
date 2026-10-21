pub fn positive_f32(value: f32) -> Result<f32, &'static str> {
    if value.is_finite() && value > 0.0 {
        Ok(value)
    } else {
        Err("must be a positive number")
    }
}

pub fn non_negative_f32(value: f32) -> Result<f32, &'static str> {
    if value.is_finite() && value >= 0.0 {
        Ok(value)
    } else {
        Err("must be zero or a positive number")
    }
}

pub fn positive_u32(value: u32) -> Result<u32, &'static str> {
    if value > 0 {
        Ok(value)
    } else {
        Err("must be a positive integer")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn positive_f32_rejects_zero_negative_and_non_finite() {
        assert!(positive_f32(0.1).is_ok());
        assert!(positive_f32(0.0).is_err());
        assert!(positive_f32(-1.0).is_err());
        assert!(positive_f32(f32::NAN).is_err());
        assert!(positive_f32(f32::INFINITY).is_err());
    }

    #[test]
    fn non_negative_f32_accepts_zero() {
        assert!(non_negative_f32(0.0).is_ok());
        assert!(non_negative_f32(-0.1).is_err());
        assert!(non_negative_f32(f32::NAN).is_err());
    }

    #[test]
    fn positive_u32_rejects_zero() {
        assert!(positive_u32(1).is_ok());
        assert!(positive_u32(0).is_err());
    }
}
