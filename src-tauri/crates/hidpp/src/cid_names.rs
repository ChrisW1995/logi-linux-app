/// Maps common HID++ Control IDs (CIDs) to human-readable names.
/// Based on Solaar's special_keys.py CONTROL dictionary.
pub fn cid_name(cid: u16) -> &'static str {
    match cid {
        0x0050 => "Left Button",
        0x0051 => "Right Button",
        0x0052 => "Middle Button",
        0x0053 => "Back Button",
        0x0054 => "Back",
        0x0056 => "Forward Button",
        0x0057 => "Forward",
        0x005B => "Left Scroll",
        0x005D => "Right Scroll",
        0x006F => "Scroll Down",
        0x0070 => "Scroll Up",
        0x00C3 => "Gesture Button",
        0x00C4 => "Smart Shift",
        0x00D0 => "MultiPlatform Gesture",
        0x00D7 => "Virtual Gesture Button",
        0x00DB => "Back (Short Press)",
        0x00DC => "Back (Long Press)",
        0x00DD => "Forward (Short Press)",
        0x00DE => "Forward (Long Press)",
        0x00E2 => "Backlight Down",
        0x00E3 => "Backlight Up",
        0x00EA => "Context Menu",
        0x00FD => "DPI Change",
        _ => "Unknown",
    }
}
