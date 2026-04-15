use alloy_primitives::U256;

pub fn parse_ether_to_wei_f64(ether: f64) -> Result<U256, String> {
    let wei = (ether * 1e18) as u128;
    Ok(U256::from(wei))
}

fn pow10_u256(exp: u8) -> U256 {
    let mut value = U256::from(1u8);
    for _ in 0..exp {
        value *= U256::from(10u8);
    }
    value
}

pub fn parse_decimal_to_units(amount: &str, decimals: u8) -> Result<U256, String> {
    let s = amount.trim();
    if s.is_empty() {
        return Err("金额不能为空".to_string());
    }
    if s.starts_with('-') {
        return Err("金额不能为负数".to_string());
    }

    let s = s.strip_prefix('+').unwrap_or(s);
    let (int_part_raw, frac_part_raw) = match s.split_once('.') {
        Some((a, b)) => (a, b),
        None => (s, ""),
    };

    let int_part = if int_part_raw.is_empty() { "0" } else { int_part_raw };
    if !int_part.chars().all(|c| c.is_ascii_digit()) {
        return Err("金额整数部分格式错误".to_string());
    }
    if !frac_part_raw.chars().all(|c| c.is_ascii_digit()) {
        return Err("金额小数部分格式错误".to_string());
    }

    let base = pow10_u256(decimals);
    let int_units = U256::from_str_radix(int_part, 10)
        .map_err(|e| format!("金额整数部分解析失败: {e}"))?
        * base;

    if decimals == 0 {
        return Ok(int_units);
    }

    let take_len = (decimals as usize).min(frac_part_raw.len());
    let mut frac = frac_part_raw[..take_len].to_string();
    while frac.len() < decimals as usize {
        frac.push('0');
    }

    let frac_units = if frac.is_empty() {
        U256::from(0u8)
    } else {
        U256::from_str_radix(&frac, 10).map_err(|e| format!("金额小数部分解析失败: {e}"))?
    };

    Ok(int_units + frac_units)
}

pub fn parse_gwei_to_wei(gwei: f64) -> U256 {
    let wei = (gwei * 1e9) as u128;
    U256::from(wei)
}

pub fn format_wei_to_ether(wei: U256) -> String {
    let wei_str = wei.to_string();
    let wei_u128: u128 = wei_str.parse().unwrap_or(0);
    let ether = wei_u128 as f64 / 1e18;
    format!("{ether:.6}")
}

pub fn format_wei_to_gwei(wei: U256) -> String {
    let wei_str = wei.to_string();
    let wei_u128: u128 = wei_str.parse().unwrap_or(0);
    let gwei = wei_u128 as f64 / 1e9;
    format!("{gwei:.2}")
}

pub fn u256_to_f64(wei: U256) -> f64 {
    let wei_str = wei.to_string();
    wei_str.parse::<f64>().unwrap_or(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_ether_to_wei_f64() {
        assert_eq!(parse_ether_to_wei_f64(1.0).unwrap(), U256::from(1000000000000000000u128));
        assert_eq!(parse_ether_to_wei_f64(0.5).unwrap(), U256::from(500000000000000000u128));
    }

    #[test]
    fn test_parse_decimal_to_units() {
        assert_eq!(parse_decimal_to_units("1", 6).unwrap(), U256::from(1_000_000u128));
        assert_eq!(parse_decimal_to_units("0.1", 6).unwrap(), U256::from(100_000u128));
        assert_eq!(parse_decimal_to_units("0.000001", 6).unwrap(), U256::from(1u128));
        assert_eq!(parse_decimal_to_units("0.0000001", 6).unwrap(), U256::from(0u128));
        assert_eq!(parse_decimal_to_units("1.23456789", 6).unwrap(), U256::from(1_234_567u128));
    }

    #[test]
    fn test_parse_gwei_to_wei() {
        assert_eq!(parse_gwei_to_wei(1.0), U256::from(1000000000u128));
        assert_eq!(parse_gwei_to_wei(0.5), U256::from(500000000u128));
    }

    #[test]
    fn test_format_wei_to_ether() {
        let wei = U256::from(1000000000000000000u128);
        assert_eq!(format_wei_to_ether(wei), "1.000000");
        
        let wei = U256::from(500000000000000000u128);
        assert_eq!(format_wei_to_ether(wei), "0.500000");
    }

    #[test]
    fn test_format_wei_to_gwei() {
        let wei = U256::from(1000000000u128);
        assert_eq!(format_wei_to_gwei(wei), "1.00");
        
        let wei = U256::from(500000000u128);
        assert_eq!(format_wei_to_gwei(wei), "0.50");
    }
}
