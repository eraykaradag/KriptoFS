use aes_gcm::{
    aead::{Aead, AeadCore, KeyInit, OsRng},
    Aes256Gcm,
    Key, // Or `Aes128Gcm`
    Nonce,
};
use sha2::{Digest, Sha256};
use std::env;
use std::vec::Vec;

pub fn hash_key_generator() -> Key<Aes256Gcm> {
    let key_bytes = env::var("KRIPTO_PASS").expect("Please set KRIPTO_PASS env value");

    let mut hasher = Sha256::new();
    hasher.update(key_bytes.as_bytes());
    let hashed_key = hasher.finalize();
    let key = Key::<Aes256Gcm>::from_slice(&hashed_key);
    *key
}
pub fn encrypt_message(plain_text: &[u8]) -> Vec<u8> {
    let key = hash_key_generator();
    let cipher = Aes256Gcm::new(&key);
    let nonce = Aes256Gcm::generate_nonce(&mut OsRng);

    let ciphertext = cipher.encrypt(&nonce, plain_text.as_ref()).unwrap();
    let mut combined = nonce.to_vec();
    combined.extend_from_slice(&ciphertext);
    combined
}

pub fn decrypt_message(cipher_text: &[u8]) -> Vec<u8> {
    let key = hash_key_generator();
    let cipher = Aes256Gcm::new(&key);

    let nonce = Nonce::from_slice(&cipher_text[0..12]);

    cipher.decrypt(nonce, &cipher_text[12..]).unwrap()
}
