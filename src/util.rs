pub fn hex_to_bytes(hex_string: &str) -> Option<Vec<u8>> {
    let mut bytes = Vec::new();
    let mut digits = hex_string.chars().filter_map(|c| c.to_digit(16));
    while let (Some(upper), Some(lower)) = (digits.next(), digits.next()) {
        bytes.push((upper << 4 | lower) as u8);
    }
    if bytes.len() % 2 != 0 {
        None // If the length of the input is odd, return None
    } else {
        Some(bytes)
    }
}
