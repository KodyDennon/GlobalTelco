//! Serde helper modules for types that don't natively serialize to valid JSON.
//!
//! serde_json requires map keys to be strings. Integer keys (u64, usize) are
//! handled automatically, but tuple keys like `(EntityId, EntityId)` are not.
//! These helpers convert tuple keys to/from `"a_b"` string format.

/// Serialize/deserialize `HashMap<(u64, u64), V>` with `"a_b"` string keys for JSON compat.
pub mod entity_pair_map {
    use serde::de::{self, MapAccess, Visitor};
    use serde::ser::SerializeMap;
    use serde::{Deserializer, Serializer};
    use std::collections::HashMap;
    use std::fmt;
    use std::marker::PhantomData;

    pub fn serialize<V, S>(map: &HashMap<(u64, u64), V>, serializer: S) -> Result<S::Ok, S::Error>
    where
        V: serde::Serialize,
        S: Serializer,
    {
        let mut m = serializer.serialize_map(Some(map.len()))?;
        for ((a, b), v) in map {
            let key = format!("{}_{}", a, b);
            m.serialize_entry(&key, v)?;
        }
        m.end()
    }

    pub fn deserialize<'de, V, D>(deserializer: D) -> Result<HashMap<(u64, u64), V>, D::Error>
    where
        V: serde::Deserialize<'de>,
        D: Deserializer<'de>,
    {
        struct PairMapVisitor<V>(PhantomData<V>);

        impl<'de, V: serde::Deserialize<'de>> Visitor<'de> for PairMapVisitor<V> {
            type Value = HashMap<(u64, u64), V>;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("a map with \"a_b\" string keys")
            }

            fn visit_map<M: MapAccess<'de>>(self, mut access: M) -> Result<Self::Value, M::Error> {
                let mut map = HashMap::with_capacity(access.size_hint().unwrap_or(0));
                while let Some((key, value)) = access.next_entry::<String, V>()? {
                    let parts: Vec<&str> = key.splitn(2, '_').collect();
                    if parts.len() != 2 {
                        return Err(de::Error::custom(format!("invalid pair key: {}", key)));
                    }
                    let a: u64 = parts[0]
                        .parse()
                        .map_err(|_| de::Error::custom(format!("invalid u64 in key: {}", key)))?;
                    let b: u64 = parts[1]
                        .parse()
                        .map_err(|_| de::Error::custom(format!("invalid u64 in key: {}", key)))?;
                    map.insert((a, b), value);
                }
                Ok(map)
            }
        }

        deserializer.deserialize_map(PairMapVisitor(PhantomData))
    }
}
