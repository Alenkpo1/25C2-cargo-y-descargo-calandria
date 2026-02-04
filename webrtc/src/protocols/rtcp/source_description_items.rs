#[derive(Debug, PartialEq)]
pub struct SdesItem {
    item_type: u8,
    length: u8,
    value: String,
}

impl SdesItem {
    pub fn new(item_type: u8, value: String) -> Self {
        let v = value;
        let len = v.len() as u8;
        SdesItem {
            item_type,
            length: len,
            value: v,
        }
    }

    pub fn write_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.push(self.item_type);
        bytes.push(self.length);
        bytes.extend_from_slice(self.value.as_bytes());
        bytes
    }
    pub fn read_bytes(bytes: &[u8]) -> Self {
        let item_type = bytes[0];
        let length = bytes[1] as usize;
        let value_bytes = &bytes[2..2 + length];
        let value = String::from_utf8_lossy(value_bytes).to_string();
        SdesItem {
            item_type,
            length: length as u8,
            value,
        }
    }
    pub fn get_type(&self) -> u8 {
        self.item_type
    }
    pub fn get_length(&self) -> u8 {
        self.length
    }
    pub fn get_value(&self) -> &str {
        &self.value
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sdes_item() {
        let item = SdesItem::new(1, "house".to_string());
        assert_eq!(item.item_type, 1);
        assert_eq!(item.length, 5);
        assert_eq!(item.value, "house");

        let bytes = item.write_bytes();
        let expected = vec![1, 5, b'h', b'o', b'u', b's', b'e'];
        assert_eq!(bytes, expected);

        let parsed = SdesItem::read_bytes(&bytes);
        assert_eq!(parsed.item_type, 1);
        assert_eq!(parsed.length, 5);
        assert_eq!(parsed.value, "house");

        assert_eq!(parsed, item);
    }
}
