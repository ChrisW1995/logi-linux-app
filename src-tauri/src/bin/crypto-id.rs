//! Standalone CLI that dumps the HID++ CryptoIdentifier (feature 0x0021) of
//! every Logitech HID++ device on the system. Used during Flow protocol
//! reconnaissance to correlate captured traffic with the shared device secret.
//! Linux-first (needs hidraw access); compiles on macOS but device access
//! there is best-effort.

/// Format a raw identifier as lowercase hex, grouped in 4-byte words for
/// readability, e.g. `a1b2c3d4 000f`.
fn format_identifier_hex(bytes: &[u8]) -> String {
    bytes
        .chunks(4)
        .map(|word| word.iter().map(|b| format!("{b:02x}")).collect::<String>())
        .collect::<Vec<_>>()
        .join(" ")
}

fn main() {
    let devices = match hidpp::device::find_logitech_devices() {
        Ok(d) => d,
        Err(e) => {
            eprintln!("Failed to enumerate Logitech devices: {e}");
            std::process::exit(1);
        }
    };

    if devices.is_empty() {
        eprintln!("No Logitech HID++ devices found.");
        std::process::exit(2);
    }

    for info in &devices {
        print!(
            "{} (index {}, pid 0x{:04x}) @ {} -> ",
            info.product_name, info.device_index, info.product_id, info.path
        );
        match hidpp::device::open_device(info) {
            Ok(access) => match access.get_crypto_identifier() {
                Ok(id) => println!("{}", format_identifier_hex(&id)),
                Err(e) => println!("no crypto identifier ({e})"),
            },
            Err(e) => println!("open failed ({e})"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_grouped_lowercase_hex() {
        let bytes = [0xa1u8, 0xb2, 0xc3, 0xd4, 0x00, 0x0f];
        assert_eq!(format_identifier_hex(&bytes), "a1b2c3d4 000f");
    }

    #[test]
    fn empty_input_is_empty_string() {
        assert_eq!(format_identifier_hex(&[]), "");
    }
}
