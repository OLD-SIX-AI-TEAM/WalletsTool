use super::session::get_session_key;
use aes::Aes256;
use aes::cipher::{BlockDecryptMut, BlockEncryptMut, KeyIvInit};
use block_padding::Pkcs7;
use cbc::{Encryptor, Decryptor};
use sha2::{Sha256, Digest};
use zeroize::{Zeroize, ZeroizeOnDrop};
use rand::Rng;
use std::str;
use serde::Deserialize;
use std::fmt;
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};

type Aes256CbcEnc = Encryptor<Aes256>;
type Aes256CbcDec = Decryptor<Aes256>;

#[derive(Clone, Zeroize, ZeroizeOnDrop, Deserialize)]
#[serde(try_from = "String")]
pub struct SecureMemory {
    #[zeroize(skip)]
    ciphertext: Vec<u8>,
    iv: [u8; 16],
    hash: [u8; 32],
}

impl fmt::Debug for SecureMemory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SecureMemory(***REDACTED***)")
    }
}

impl TryFrom<String> for SecureMemory {
    type Error = String;
    fn try_from(s: String) -> Result<Self, Self::Error> {
        Ok(SecureMemory::new(s))
    }
}

impl SecureMemory {
    pub fn new(secret: String) -> Self {
        let key = get_session_key();
        let mut iv = [0u8; 16];
        rand::thread_rng().fill(&mut iv);

        // Convert String to Vec<u8> to allow zeroization
        let mut plaintext_bytes = secret.into_bytes();
        
        let mut hasher = Sha256::new();
        hasher.update(&plaintext_bytes);
        let hash: [u8; 32] = hasher.finalize().into();

        // Padding
        let len = plaintext_bytes.len();
        let mut buffer = vec![0u8; len + 16]; // Sufficient buffer for padding
        buffer[..len].copy_from_slice(&plaintext_bytes);
        
        // Zeroize the original plaintext bytes immediately after copying
        plaintext_bytes.zeroize();
        
        let encryptor = Aes256CbcEnc::new(&key.into(), &iv.into());
        let ciphertext_len = encryptor.encrypt_padded_mut::<Pkcs7>(&mut buffer, len)
            .expect("Encryption failed")
            .len();
        
        buffer.truncate(ciphertext_len);

        SecureMemory {
            ciphertext: buffer,
            iv,
            hash,
        }
    }

    pub fn use_secret<F, R>(&self, f: F) -> Result<R, String>
    where
        F: FnOnce(&str) -> R,
    {
        let key = get_session_key();
        let decryptor = Aes256CbcDec::new(&key.into(), &self.iv.into());
        
        let mut buffer = self.ciphertext.clone();
        
        let result = (|| {
            let plaintext = decryptor.decrypt_padded_mut::<Pkcs7>(&mut buffer)
                .map_err(|_| "Decryption failed".to_string())?;
            
            let mut hasher = Sha256::new();
            hasher.update(plaintext);
            let hash_calc: [u8; 32] = hasher.finalize().into();
            
            if hash_calc != self.hash {
                return Err("Memory integrity check failed!".to_string());
            }

            let s = str::from_utf8(plaintext).map_err(|_| "Invalid UTF-8".to_string())?;
            Ok(f(s))
        })();

        buffer.zeroize();
        result
    }
}

pub fn encrypt_string(secret: &str) -> Result<String, String> {
    let s = secret.trim();
    if s.is_empty() {
        return Err("Secret is empty".to_string());
    }

    let key = get_session_key();
    let mut iv = [0u8; 16];
    rand::thread_rng().fill(&mut iv);

    let mut plaintext_bytes = s.as_bytes().to_vec();
    let mut hasher = Sha256::new();
    hasher.update(&plaintext_bytes);
    let hash: [u8; 32] = hasher.finalize().into();

    let len = plaintext_bytes.len();
    let mut buffer = vec![0u8; len + 16];
    buffer[..len].copy_from_slice(&plaintext_bytes);
    plaintext_bytes.zeroize();

    let encryptor = Aes256CbcEnc::new(&key.into(), &iv.into());
    let ciphertext_len = encryptor
        .encrypt_padded_mut::<Pkcs7>(&mut buffer, len)
        .map_err(|_| "Encryption failed".to_string())?
        .len();
    buffer.truncate(ciphertext_len);

    Ok(format!(
        "m1:{}:{}:{}",
        hex::encode(iv),
        BASE64.encode(&buffer),
        hex::encode(hash)
    ))
}

pub fn decrypt_string(sealed: &str) -> Result<String, String> {
    let s = sealed.trim();
    if !s.starts_with("m1:") {
        return Err("Invalid sealed format".to_string());
    }

    let parts: Vec<&str> = s[3..].split(':').collect();
    if parts.len() != 3 {
        return Err("Invalid sealed format".to_string());
    }

    let iv = hex::decode(parts[0]).map_err(|_| "Invalid iv".to_string())?;
    if iv.len() != 16 {
        return Err("Invalid iv".to_string());
    }
    let mut iv_arr = [0u8; 16];
    iv_arr.copy_from_slice(&iv);

    let mut ciphertext = BASE64
        .decode(parts[1].as_bytes())
        .map_err(|_| "Invalid cipher".to_string())?;

    let expected_hash = hex::decode(parts[2]).map_err(|_| "Invalid hash".to_string())?;
    if expected_hash.len() != 32 {
        return Err("Invalid hash".to_string());
    }
    let mut expected_hash_arr = [0u8; 32];
    expected_hash_arr.copy_from_slice(&expected_hash);

    let key = get_session_key();
    let decryptor = Aes256CbcDec::new(&key.into(), &iv_arr.into());
    let plaintext = decryptor
        .decrypt_padded_mut::<Pkcs7>(&mut ciphertext)
        .map_err(|_| "Decryption failed".to_string())?;

    let mut hasher = Sha256::new();
    hasher.update(plaintext);
    let hash_calc: [u8; 32] = hasher.finalize().into();
    if hash_calc != expected_hash_arr {
        return Err("Memory integrity check failed!".to_string());
    }

    let out = String::from_utf8(plaintext.to_vec()).map_err(|_| "Invalid UTF-8".to_string())?;
    ciphertext.zeroize();
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encryption_decryption() {
        let secret = "my_secret_key";
        let secure = SecureMemory::new(secret.to_string());
        
        let recovered = secure.use_secret(|s| s.to_string()).expect("Decryption failed");
        assert_eq!(secret, recovered);
    }

    #[test]
    fn test_redacted_debug() {
        let secret = "secret";
        let secure = SecureMemory::new(secret.to_string());
        assert_eq!(format!("{:?}", secure), "SecureMemory(***REDACTED***)");
    }
}
