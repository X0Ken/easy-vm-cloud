/// 工具函数集合

use uuid::Uuid;

/// 生成唯一 ID
pub fn generate_id() -> String {
    Uuid::new_v4().to_string()
}

/// 格式化字节大小
pub fn format_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB", "PB"];
    let mut size = bytes as f64;
    let mut unit_index = 0;

    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }

    format!("{:.2} {}", size, UNITS[unit_index])
}

/// 验证 MAC 地址格式
pub fn validate_mac_address(mac: &str) -> bool {
    let parts: Vec<&str> = mac.split(':').collect();
    if parts.len() != 6 {
        return false;
    }

    parts.iter().all(|part| {
        part.len() == 2 && part.chars().all(|c| c.is_ascii_hexdigit())
    })
}

/// 验证 IP 地址格式（简单验证）
pub fn validate_ip_address(ip: &str) -> bool {
    let parts: Vec<&str> = ip.split('.').collect();
    if parts.len() != 4 {
        return false;
    }

    parts.iter().all(|part| {
        part.parse::<u8>().is_ok()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_id() {
        let id1 = generate_id();
        let id2 = generate_id();
        assert_ne!(id1, id2);
        assert_eq!(id1.len(), 36); // UUID v4 格式
    }

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(0), "0.00 B");
        assert_eq!(format_bytes(1024), "1.00 KB");
        assert_eq!(format_bytes(1048576), "1.00 MB");
        assert_eq!(format_bytes(1073741824), "1.00 GB");
    }

    #[test]
    fn test_validate_mac_address() {
        assert!(validate_mac_address("52:54:00:12:34:56"));
        assert!(validate_mac_address("00:11:22:33:44:55"));
        assert!(!validate_mac_address("52:54:00:12:34"));
        assert!(!validate_mac_address("52:54:00:12:34:5g"));
        assert!(!validate_mac_address("invalid"));
    }

    #[test]
    fn test_validate_ip_address() {
        assert!(validate_ip_address("192.168.1.1"));
        assert!(validate_ip_address("10.0.0.1"));
        assert!(!validate_ip_address("256.1.1.1"));
        assert!(!validate_ip_address("192.168.1"));
        assert!(!validate_ip_address("invalid"));
    }
}

