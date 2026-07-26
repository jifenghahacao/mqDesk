//! 本地密码加密
//!
//! 由于 Windows keyring 在部分环境（尤其是 Tauri dev / 测试）下不稳定，
//! 改用 sled 存储加密后的密码，密钥由机器/用户特征派生。
//!
//! 算法：AES-256-GCM（通过 ring）
//! 密钥派生：PBKDF2-HMAC-SHA256，迭代 100,000 次
//! 主密钥材料：固定 salt + 当前用户名 + 主机名（跨机器不可解密，增加安全性）

use crate::error::{AppError, AppResult};
use ring::aead::{Aad, Nonce, UnboundKey, AES_256_GCM};
use ring::pbkdf2;
use ring::rand::{SecureRandom, SystemRandom};
use std::num::NonZeroU32;

const SALT: &[u8] = b"mqdesk-v1-local-encryption-salt";
const ITERATIONS: u32 = 100_000;

fn master_secret() -> Vec<u8> {
    // 组合用户名 + 主机名 + 固定 salt 作为主密钥材料
    let mut parts = Vec::new();
    parts.extend_from_slice(SALT);
    if let Ok(user) = std::env::var("USERNAME").or_else(|_| std::env::var("USER")) {
        parts.extend_from_slice(user.as_bytes());
    }
    if let Ok(hostname) = std::env::var("COMPUTERNAME").or_else(|_| std::env::var("HOSTNAME")) {
        parts.extend_from_slice(hostname.as_bytes());
    }
    parts
}

fn derive_key(secret: &[u8]) -> [u8; 32] {
    let mut key = [0u8; 32];
    pbkdf2::derive(
        pbkdf2::PBKDF2_HMAC_SHA256,
        NonZeroU32::new(ITERATIONS).unwrap(),
        SALT,
        secret,
        &mut key,
    );
    key
}

fn sealing_key() -> AppResult<ring::aead::LessSafeKey> {
    let secret = master_secret();
    let key_bytes = derive_key(&secret);
    let unbound = UnboundKey::new(&AES_256_GCM, &key_bytes)
        .map_err(|_| AppError::Crypto("初始化 AES-256-GCM 密钥失败".to_string()))?;
    Ok(ring::aead::LessSafeKey::new(unbound))
}

/// 加密明文，返回 "base64(nonce:ciphertext:tag)"
pub fn encrypt(plaintext: &str) -> AppResult<String> {
    let key = sealing_key()?;
    let rng = SystemRandom::new();
    let mut nonce_bytes = [0u8; 12];
    rng.fill(&mut nonce_bytes)
        .map_err(|_| AppError::Crypto("生成 nonce 失败".to_string()))?;

    let mut in_out = plaintext.as_bytes().to_vec();
    key.seal_in_place_append_tag(
        Nonce::assume_unique_for_key(nonce_bytes),
        Aad::empty(),
        &mut in_out,
    )
    .map_err(|_| AppError::Crypto("加密失败".to_string()))?;

    let mut result = Vec::with_capacity(nonce_bytes.len() + in_out.len());
    result.extend_from_slice(&nonce_bytes);
    result.extend_from_slice(&in_out);

    Ok(base64::encode(&result))
}

/// 解密 "base64(nonce:ciphertext:tag)"
pub fn decrypt(ciphertext_b64: &str) -> AppResult<String> {
    let key = sealing_key()?;
    let bytes = base64::decode(ciphertext_b64)
        .map_err(|_| AppError::Crypto("密码密文不是有效 base64".to_string()))?;

    if bytes.len() < 12 + 16 {
        return Err(AppError::Crypto("密码密文长度不足".to_string()));
    }

    let (nonce_bytes, sealed) = bytes.split_at(12);
    let mut in_out = sealed.to_vec();
    let nonce = Nonce::try_assume_unique_for_key(nonce_bytes)
        .map_err(|_| AppError::Crypto("nonce 长度错误".to_string()))?;

    let plaintext = key
        .open_in_place(nonce, Aad::empty(), &mut in_out)
        .map_err(|_| AppError::Crypto("解密失败：密钥或密文损坏".to_string()))?;

    String::from_utf8(plaintext.to_vec())
        .map_err(|_| AppError::Crypto("解密后不是有效 UTF-8".to_string()))
}

// base64 适配（ring 不自带）
mod base64 {
    use base64::engine::general_purpose::STANDARD;
    use base64::Engine;

    pub fn encode(input: &[u8]) -> String {
        STANDARD.encode(input)
    }

    pub fn decode(input: &str) -> crate::error::AppResult<Vec<u8>> {
        STANDARD
            .decode(input)
            .map_err(|_| crate::error::AppError::Crypto("base64 解码失败".to_string()))
    }
}
