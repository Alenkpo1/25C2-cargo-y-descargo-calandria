/// SRTP-ligero: XOR pseudo-aleatorio derivado de seq/timestamp + clave compartida.

#[derive(Clone)]
pub struct SrtpContext {
    key: Vec<u8>,
}

impl SrtpContext {
    /// Espera una clave de al menos 16 bytes.
    pub fn new(key_bytes: &[u8]) -> Option<Self> {
        if key_bytes.len() < 16 {
            return None;
        }
        Some(Self {
            key: key_bytes.to_vec(),
        })
    }

    pub fn get_key(&self) -> &[u8] {
        &self.key
    }

    fn keystream(&self, seq: u16, timestamp: u32, len: usize) -> Vec<u8> {
        let mut stream = Vec::with_capacity(len);
        let seed = [
            timestamp.to_be_bytes().as_slice(),
            seq.to_be_bytes().as_slice(),
            self.key.as_slice(),
        ]
        .concat();
        for i in 0..len {
            let b = seed[i % seed.len()] ^ (seed[(i + 3) % seed.len()].wrapping_add(i as u8));
            stream.push(b);
        }
        stream
    }

    pub fn protect(&self, seq: u16, timestamp: u32, payload: &[u8]) -> Option<Vec<u8>> {
        let ks = self.keystream(seq, timestamp, payload.len());
        Some(payload.iter().zip(ks.iter()).map(|(p, k)| p ^ k).collect())
    }

    pub fn unprotect(&self, seq: u16, timestamp: u32, cipher_text: &[u8]) -> Option<Vec<u8>> {
        let ks = self.keystream(seq, timestamp, cipher_text.len());
        Some(
            cipher_text
                .iter()
                .zip(ks.iter())
                .map(|(c, k)| c ^ k)
                .collect(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::SrtpContext;

    #[test]
    fn roundtrip_encrypt_decrypt() {
        let key = vec![1u8; 16];
        let ctx = SrtpContext::new(&key).expect("ctx");
        let payload = b"hola webrtc";
        let seq = 42u16;
        let ts = 123_456u32;

        let cipher = ctx.protect(seq, ts, payload).expect("cipher");
        assert_ne!(cipher, payload);

        let plain = ctx.unprotect(seq, ts, &cipher).expect("plain");
        assert_eq!(plain, payload);
    }
}
