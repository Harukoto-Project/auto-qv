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

pub const MIN_CHAR_SPACING: f32 = -1.0;

pub fn char_spacing(value: f32) -> Result<f32, &'static str> {
    if value.is_finite() && value >= MIN_CHAR_SPACING {
        Ok(value)
    } else {
        Err("must be a number of at least -1.0")
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
    fn char_spacing_accepts_negative_values_down_to_the_limit() {
        assert!(char_spacing(0.15).is_ok());
        assert!(char_spacing(-0.1).is_ok());
        assert!(char_spacing(-1.0).is_ok());
        assert!(char_spacing(-1.01).is_err());
        assert!(char_spacing(f32::NAN).is_err());
    }

    #[test]
    fn positive_u32_rejects_zero() {
        assert!(positive_u32(1).is_ok());
        assert!(positive_u32(0).is_err());
    }
}
