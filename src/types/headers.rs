//! Case-insensitive headers implementation
//!
//! Provides a unified case-insensitive header storage that preserves
//! original header casing while allowing case-insensitive lookups.

use std::collections::HashMap;

/// Case-insensitive headers wrapper.
///
/// This struct stores headers while allowing case-insensitive access.
/// The original header name casing is preserved when inserting.
#[derive(Debug, Clone, Default)]
pub struct CaseInsensitiveHeaders {
    /// Original header name -> value mapping
    pub(crate) inner: HashMap<String, String>,
    /// Lowercase header name -> original header name mapping for fast lookup
    pub(crate) lowercase_map: HashMap<String, String>,
}

impl CaseInsensitiveHeaders {
    /// Create a new empty headers container.
    #[inline]
    pub fn new() -> Self {
        Self {
            inner: HashMap::new(),
            lowercase_map: HashMap::new(),
        }
    }

    /// Create from an existing HashMap.
    ///
    /// All keys are inserted with their original casing.
    pub fn from_hashmap(headers: HashMap<String, String>) -> Self {
        let mut ci_headers = Self {
            inner: HashMap::new(),
            lowercase_map: HashMap::new(),
        };
        for (key, value) in headers {
            ci_headers.insert(key, value);
        }
        ci_headers
    }

    /// Insert a header with the given key and value.
    ///
    /// The original key casing is preserved while enabling case-insensitive lookup.
    pub fn insert(&mut self, key: String, value: String) {
        let lowercase_key = key.to_lowercase();
        self.lowercase_map
            .insert(lowercase_key.clone(), key.clone());
        self.inner.insert(key, value);
    }

    /// Get a header value by key (case-insensitive).
    ///
    /// Returns `None` if the header is not found.
    #[inline]
    pub fn get(&self, key: &str) -> Option<&String> {
        self.lowercase_map
            .get(&key.to_lowercase())
            .and_then(|original_key| self.inner.get(original_key))
    }

    /// Get a mutable reference to a header value by key (case-insensitive).
    ///
    /// Returns `None` if the header is not found.
    pub fn get_mut(&mut self, key: &str) -> Option<&mut String> {
        self.lowercase_map
            .get(&key.to_lowercase())
            .and_then(|original_key| self.inner.get_mut(original_key))
    }

    /// Remove a header by key (case-insensitive).
    ///
    /// Returns whether the header was present.
    pub fn remove(&mut self, key: &str) -> bool {
        let lowercase_key = key.to_lowercase();
        if let Some(original_key) = self.lowercase_map.get(&lowercase_key) {
            self.inner.remove(original_key);
            self.lowercase_map.remove(&lowercase_key);
            true
        } else {
            false
        }
    }

    /// Clear all headers.
    #[inline]
    pub fn clear(&mut self) {
        self.inner.clear();
        self.lowercase_map.clear();
    }

    /// Returns the number of headers.
    #[inline]
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Returns whether the headers container is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Returns an iterator over the header key-value pairs.
    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = (&String, &String)> {
        self.inner.iter()
    }

    /// Returns an iterator over the header keys.
    #[inline]
    pub fn keys(&self) -> impl Iterator<Item = &String> {
        self.inner.keys()
    }

    /// Returns an iterator over the header values.
    #[inline]
    pub fn values(&self) -> impl Iterator<Item = &String> {
        self.inner.values()
    }

    /// Check if a header exists (case-insensitive).
    #[inline]
    pub fn contains_key(&self, key: &str) -> bool {
        self.lowercase_map.contains_key(&key.to_lowercase())
    }
}

impl<K, V> FromIterator<(K, V)> for CaseInsensitiveHeaders
where
    K: Into<String>,
    V: Into<String>,
{
    fn from_iter<T: IntoIterator<Item = (K, V)>>(iter: T) -> Self {
        let mut headers = Self::new();
        for (key, value) in iter {
            headers.insert(key.into(), value.into());
        }
        headers
    }
}

impl IntoIterator for CaseInsensitiveHeaders {
    type Item = (String, String);
    type IntoIter = std::collections::hash_map::IntoIter<String, String>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.inner.into_iter()
    }
}

impl<'a> IntoIterator for &'a CaseInsensitiveHeaders {
    type Item = (&'a String, &'a String);
    type IntoIter = std::collections::hash_map::Iter<'a, String, String>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.inner.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_insert_and_get() {
        let mut headers = CaseInsensitiveHeaders::new();
        headers.insert("Content-Type".to_string(), "application/json".to_string());

        // Case-insensitive lookup
        assert_eq!(
            headers.get("content-type"),
            Some(&"application/json".to_string())
        );
        assert_eq!(
            headers.get("CONTENT-TYPE"),
            Some(&"application/json".to_string())
        );
        assert_eq!(
            headers.get("Content-Type"),
            Some(&"application/json".to_string())
        );

        // Non-existent key
        assert_eq!(headers.get("accept"), None);
    }

    #[test]
    fn test_preserve_casing() {
        let mut headers = CaseInsensitiveHeaders::new();
        headers.insert("X-Custom-Header".to_string(), "value".to_string());

        // Original casing preserved
        let keys: Vec<&String> = headers.keys().collect();
        assert!(keys.iter().any(|k| **k == "X-Custom-Header"));
    }

    #[test]
    fn test_remove() {
        let mut headers = CaseInsensitiveHeaders::new();
        headers.insert("Content-Type".to_string(), "application/json".to_string());

        assert!(headers.remove("content-type"));
        assert!(headers.is_empty());

        // Removing non-existent returns false
        assert!(!headers.remove("accept"));
    }

    #[test]
    fn test_from_hashmap() {
        let mut map = HashMap::new();
        map.insert("Content-Type".to_string(), "text/html".to_string());
        map.insert("Authorization".to_string(), "Bearer token".to_string());

        let headers = CaseInsensitiveHeaders::from_hashmap(map);

        assert_eq!(headers.len(), 2);
        assert_eq!(headers.get("content-type"), Some(&"text/html".to_string()));
        assert_eq!(
            headers.get("authorization"),
            Some(&"Bearer token".to_string())
        );
    }
}
