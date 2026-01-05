//! Compression engine for intent messages.
//!
//! Targets 10x compression ratio using LZ4 + dictionary encoding.

use crate::core::{Error, Result};
use lz4_flex::{compress_prepend_size, decompress_size_prepended};
use std::collections::HashMap;

/// Compression statistics.
#[derive(Clone, Debug, Default)]
pub struct CompressionStats {
    /// Original size in bytes
    pub original_size: usize,
    /// Compressed size in bytes
    pub compressed_size: usize,
    /// Compression ratio (original / compressed)
    pub ratio: f64,
    /// Compression time in microseconds
    pub compress_time_us: u64,
}

/// Compression engine with dictionary support.
pub struct CompressionEngine {
    /// Dictionary for common patterns
    dictionary: HashMap<String, u16>,
    /// Reverse dictionary for decompression
    reverse_dictionary: HashMap<u16, String>,
    /// Next dictionary index
    next_index: u16,
    /// Statistics
    stats: CompressionStats,
}

impl CompressionEngine {
    /// Create a new compression engine.
    pub fn new() -> Self {
        let mut engine = Self {
            dictionary: HashMap::new(),
            reverse_dictionary: HashMap::new(),
            next_index: 0,
            stats: CompressionStats::default(),
        };

        // Preload common patterns for intent messages
        engine.add_to_dictionary("SemanticIntent");
        engine.add_to_dictionary("IntentDomain");
        engine.add_to_dictionary("parameters");
        engine.add_to_dictionary("constraints");
        engine.add_to_dictionary("correlation_id");
        engine.add_to_dictionary("timestamp");
        engine.add_to_dictionary("VerifiableCredential");
        engine.add_to_dictionary("did:hido:");

        engine
    }

    /// Add a pattern to the dictionary.
    pub fn add_to_dictionary(&mut self, pattern: &str) {
        if !self.dictionary.contains_key(pattern) {
            self.dictionary.insert(pattern.to_string(), self.next_index);
            self.reverse_dictionary.insert(self.next_index, pattern.to_string());
            self.next_index += 1;
        }
    }

    /// Compress data using LZ4.
    pub fn compress(&mut self, data: &[u8]) -> Result<Vec<u8>> {
        let start = std::time::Instant::now();

        // Apply dictionary substitution first (for text data)
        let preprocessed = self.apply_dictionary(data);

        // LZ4 compression
        let compressed = compress_prepend_size(&preprocessed);

        let elapsed = start.elapsed();
        self.stats = CompressionStats {
            original_size: data.len(),
            compressed_size: compressed.len(),
            ratio: if compressed.len() > 0 {
                data.len() as f64 / compressed.len() as f64
            } else {
                1.0
            },
            compress_time_us: elapsed.as_micros() as u64,
        };

        Ok(compressed)
    }

    /// Decompress data.
    pub fn decompress(&self, data: &[u8]) -> Result<Vec<u8>> {
        let decompressed = decompress_size_prepended(data)
            .map_err(|e| Error::DecompressionFailed(e.to_string()))?;

        // Reverse dictionary substitution
        let restored = self.restore_dictionary(&decompressed);

        Ok(restored)
    }

    /// Apply dictionary substitution to data.
    fn apply_dictionary(&self, data: &[u8]) -> Vec<u8> {
        // For binary data, just return as-is
        // Dictionary substitution is mainly for JSON text
        if let Ok(text) = std::str::from_utf8(data) {
            let mut result = text.to_string();
            for (pattern, index) in &self.dictionary {
                // Use a placeholder format that's unlikely to appear naturally
                let placeholder = format!("\x00D{:04X}\x00", index);
                result = result.replace(pattern, &placeholder);
            }
            result.into_bytes()
        } else {
            data.to_vec()
        }
    }

    /// Restore dictionary substitution.
    fn restore_dictionary(&self, data: &[u8]) -> Vec<u8> {
        if let Ok(text) = std::str::from_utf8(data) {
            let mut result = text.to_string();
            for (index, pattern) in &self.reverse_dictionary {
                let placeholder = format!("\x00D{:04X}\x00", index);
                result = result.replace(&placeholder, pattern);
            }
            result.into_bytes()
        } else {
            data.to_vec()
        }
    }

    /// Get compression statistics.
    pub fn stats(&self) -> &CompressionStats {
        &self.stats
    }

    /// Get dictionary size.
    pub fn dictionary_size(&self) -> usize {
        self.dictionary.len()
    }

    /// Compress JSON intent and return with stats.
    pub fn compress_intent(&mut self, intent_json: &str) -> Result<(Vec<u8>, CompressionStats)> {
        let compressed = self.compress(intent_json.as_bytes())?;
        Ok((compressed, self.stats.clone()))
    }

    /// Decompress to JSON string.
    pub fn decompress_to_string(&self, data: &[u8]) -> Result<String> {
        let decompressed = self.decompress(data)?;
        String::from_utf8(decompressed)
            .map_err(|e| Error::DecompressionFailed(e.to_string()))
    }
}

impl Default for CompressionEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Compress data without dictionary (simple LZ4).
pub fn compress_simple(data: &[u8]) -> Vec<u8> {
    compress_prepend_size(data)
}

/// Decompress data without dictionary.
pub fn decompress_simple(data: &[u8]) -> Result<Vec<u8>> {
    decompress_size_prepended(data)
        .map_err(|e| Error::DecompressionFailed(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::icc::intent::{IntentDomain, SemanticIntent};
    use crate::uail::crypto::CryptoSuite;
    use crate::uail::DIDKey;

    fn create_test_did() -> DIDKey {
        let crypto = CryptoSuite::new();
        DIDKey::new(&crypto)
    }

    #[test]
    fn test_compression_simple() {
        let data = b"Hello, this is a test message for compression!";
        let compressed = compress_simple(data);
        let decompressed = decompress_simple(&compressed).unwrap();
        assert_eq!(decompressed, data);
    }

    #[test]
    fn test_compression_engine() {
        let mut engine = CompressionEngine::new();
        let data = b"Test data with some content to compress";
        let compressed = engine.compress(data).unwrap();
        let decompressed = engine.decompress(&compressed).unwrap();
        assert_eq!(decompressed, data);
    }

    #[test]
    fn test_compression_ratio() {
        let mut engine = CompressionEngine::new();

        // Create a large repetitive string (good for compression)
        let mut large_data = String::new();
        for i in 0..100 {
            large_data.push_str(&format!("SemanticIntent parameters correlation_id {} ", i));
        }

        let compressed = engine.compress(large_data.as_bytes()).unwrap();
        let stats = engine.stats();

        // Should achieve some compression on repetitive data
        assert!(stats.ratio > 1.0);
        assert!(compressed.len() < large_data.len());
    }

    #[test]
    fn test_intent_compression() {
        let sender = create_test_did();
        let intent = SemanticIntent::new(&sender, IntentDomain::Data, "read")
            .with_target("database")
            .with_param("query", serde_json::json!("SELECT * FROM users"))
            .with_param("limit", serde_json::json!(100));

        let json = intent.to_json().unwrap();
        let mut engine = CompressionEngine::new();
        let (compressed, stats) = engine.compress_intent(&json).unwrap();

        // Decompress and verify
        let restored = engine.decompress_to_string(&compressed).unwrap();
        let parsed: SemanticIntent = serde_json::from_str(&restored).unwrap();

        assert_eq!(parsed.id, intent.id);
        assert_eq!(parsed.action, intent.action);

        // Check stats
        assert!(stats.original_size > 0);
        assert!(stats.compressed_size > 0);
    }

    #[test]
    fn test_dictionary_addition() {
        let mut engine = CompressionEngine::new();
        let initial_size = engine.dictionary_size();

        engine.add_to_dictionary("custom_pattern");
        assert_eq!(engine.dictionary_size(), initial_size + 1);

        // Adding same pattern shouldn't increase size
        engine.add_to_dictionary("custom_pattern");
        assert_eq!(engine.dictionary_size(), initial_size + 1);
    }

    #[test]
    fn test_binary_data_compression() {
        let mut engine = CompressionEngine::new();
        // Random binary data
        let data: Vec<u8> = (0..256).map(|i| i as u8).collect();
        let compressed = engine.compress(&data).unwrap();
        let decompressed = engine.decompress(&compressed).unwrap();
        assert_eq!(decompressed, data);
    }
}
