use chrono::Utc;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use warehouse_common::{
    BuildSet, CatalogId, Distribution, JavaCatalog, JavaMajor, MinecraftCatalog, MinecraftVersion,
    ProxyCatalog, RuntimeCatalog, SCHEMA_VERSION, VelocityVersion, Version,
};

const FORBIDDEN: [&str; 6] = ["://", "www.", ".com/", ".net/", ".org/", "docker.io"];

const FORBIDDEN_KEYS: [&str; 8] = [
    "url", "uri", "href", "link", "image", "download", "mirror", "checksum",
];

fn walk(value: &Value, path: &str, failures: &mut Vec<String>) {
    match value {
        Value::String(text) => {
            for needle in FORBIDDEN {
                if text.contains(needle) {
                    failures.push(format!("{path} contains {needle:?}: {text:?}"));
                }
            }
        }
        Value::Array(items) => {
            for (index, item) in items.iter().enumerate() {
                walk(item, &format!("{path}[{index}]"), failures);
            }
        }
        Value::Object(fields) => {
            for (key, item) in fields {
                let lowered = key.to_lowercase();
                for forbidden in FORBIDDEN_KEYS {
                    if lowered.contains(forbidden) {
                        failures.push(format!("{path}.{key} is a location-shaped field"));
                    }
                }
                walk(item, &format!("{path}.{key}"), failures);
            }
        }
        Value::Null | Value::Bool(_) | Value::Number(_) => {}
    }
}

fn assert_carries_no_locations(name: &str, document: &impl serde::Serialize) {
    let encoded = serde_json::to_value(document).expect("catalog should serialize");
    let mut failures = Vec::new();
    walk(&encoded, name, &mut failures);

    assert!(
        failures.is_empty(),
        "the identifier invariant is broken - a catalog may only carry version identifiers, \
         never a location a consumer could fetch:\n  {}",
        failures.join("\n  ")
    );
}

#[cfg(test)]
mod tests {
    use super::{
        BTreeMap, BTreeSet, BuildSet, CatalogId, Distribution, JavaCatalog, JavaMajor,
        MinecraftCatalog, MinecraftVersion, ProxyCatalog, RuntimeCatalog, SCHEMA_VERSION, Utc,
        VelocityVersion, Version, assert_carries_no_locations,
    };

    #[test]
    fn minecraft_carries_no_locations() {
        let document = MinecraftCatalog {
            schema: SCHEMA_VERSION,
            updated_at: Utc::now(),
            versions: vec![MinecraftVersion {
                id: "1.21.4".to_owned(),
                recommended_java: 21,
                data_pack: true,
                distributions: BTreeMap::from([
                    (
                        Distribution::Paper,
                        BuildSet::Range {
                            prefix: None,
                            min: 1,
                            max: 40,
                            excluded: core::iter::once(7).collect(),
                        },
                    ),
                    (
                        Distribution::Fabric,
                        BuildSet::Set {
                            values: vec!["0.16.9".to_owned()],
                        },
                    ),
                ]),
            }],
        };

        assert_carries_no_locations("minecraft", &document);
    }

    #[test]
    fn proxy_carries_no_locations() {
        let document = ProxyCatalog {
            schema: SCHEMA_VERSION,
            updated_at: Utc::now(),
            velocity: vec![VelocityVersion {
                id: "3.4.0-SNAPSHOT".to_owned(),
                java: 17,
                builds: BuildSet::Range {
                    prefix: None,
                    min: 1,
                    max: 500,
                    excluded: BTreeSet::default(),
                },
            }],
            bungeecord: vec![1800, 1799],
        };

        assert_carries_no_locations("minecraft-proxy", &document);
    }

    #[test]
    fn runtime_carries_no_locations() {
        let document = RuntimeCatalog {
            schema: SCHEMA_VERSION,
            runtime: CatalogId::Go,
            updated_at: Utc::now(),
            versions: vec![Version::new(1, 24, 5)],
        };

        assert_carries_no_locations("go", &document);
    }

    #[test]
    fn java_carries_no_locations() {
        let document = JavaCatalog {
            schema: SCHEMA_VERSION,
            updated_at: Utc::now(),
            majors: vec![JavaMajor {
                major: 21,
                versions: vec!["21.0.5_11".parse().expect("valid temurin version")],
            }],
        };

        assert_carries_no_locations("java", &document);
    }
}
