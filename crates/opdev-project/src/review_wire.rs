//! Lossless discussion-only string interning. Digests and validators still see
//! the original typed record, not a shortened or summarized acceptance review.
use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::ReviewRecord;

const MAX_EXPANDED: usize = 1024 * 1024;
const MAX_REFERENCES: usize = 4096;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Wire {
    schema: u32,
    #[serde(deserialize_with = "unique_value")]
    record: Value,
    strings: Vec<String>,
    references: Vec<Reference>,
}

// Value normally accepts duplicate object keys and discards earlier contents.
// Keep the typed v1 reader's fail-closed behavior before replacing null slots.
fn unique_value<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<Value, D::Error> {
    struct Unique(Value);
    impl<'de> Deserialize<'de> for Unique {
        fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
            unique_value(d).map(Self)
        }
    }
    struct Visitor;
    impl<'de> serde::de::Visitor<'de> for Visitor {
        type Value = Value;
        fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str("JSON without duplicate keys")
        }
        fn visit_bool<E: serde::de::Error>(self, v: bool) -> Result<Value, E> {
            Ok(v.into())
        }
        fn visit_i64<E: serde::de::Error>(self, v: i64) -> Result<Value, E> {
            Ok(v.into())
        }
        fn visit_u64<E: serde::de::Error>(self, v: u64) -> Result<Value, E> {
            Ok(v.into())
        }
        fn visit_f64<E: serde::de::Error>(self, v: f64) -> Result<Value, E> {
            serde_json::Number::from_f64(v)
                .map(Value::Number)
                .ok_or_else(|| E::custom("Invalid JSON number"))
        }
        fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<Value, E> {
            Ok(v.into())
        }
        fn visit_unit<E: serde::de::Error>(self) -> Result<Value, E> {
            Ok(Value::Null)
        }
        fn visit_seq<A: serde::de::SeqAccess<'de>>(self, mut a: A) -> Result<Value, A::Error> {
            let mut values = Vec::new();
            while let Some(Unique(value)) = a.next_element()? {
                values.push(value);
            }
            Ok(Value::Array(values))
        }
        fn visit_map<A: serde::de::MapAccess<'de>>(self, mut a: A) -> Result<Value, A::Error> {
            let mut values = serde_json::Map::new();
            while let Some((key, Unique(value))) = a.next_entry::<String, Unique>()? {
                if values.insert(key, value).is_some() {
                    return Err(serde::de::Error::custom("Duplicate review key"));
                }
            }
            Ok(Value::Object(values))
        }
    }
    deserializer.deserialize_any(Visitor)
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Reference {
    path: String,
    index: usize,
}

pub(crate) fn encode(record: &ReviewRecord) -> anyhow::Result<String> {
    let mut value = serde_json::to_value(record)?;
    anyhow::ensure!(
        serde_json::to_vec(&value)?.len() <= MAX_EXPANDED,
        "Expanded review exceeds 1 MiB"
    );
    let mut occurrences = BTreeMap::<String, Vec<String>>::new();
    collect(&value, "", &mut occurrences);
    let mut wire = Wire {
        schema: 2,
        record: Value::Null,
        strings: Vec::new(),
        references: Vec::new(),
    };
    for (string, paths) in occurrences {
        // Each pointer costs bytes too; intern only when it actually reduces the payload.
        let saved = string.len().saturating_mul(paths.len().saturating_sub(1));
        if paths.len() < 2 || saved <= paths.iter().map(|p| p.len() + 40).sum::<usize>() {
            continue;
        }
        let index = wire.strings.len();
        wire.strings.push(string);
        for path in paths {
            *value
                .pointer_mut(&path)
                .ok_or_else(|| anyhow::anyhow!("Invalid generated review reference"))? =
                Value::Null;
            wire.references.push(Reference { path, index });
        }
    }
    anyhow::ensure!(
        wire.references.len() <= MAX_REFERENCES,
        "Too many review references"
    );
    wire.record = value;
    Ok(serde_json::to_string(&wire)?)
}

fn collect(value: &Value, path: &str, values: &mut BTreeMap<String, Vec<String>>) {
    match value {
        Value::String(s) if s.len() >= 80 => values.entry(s.clone()).or_default().push(path.into()),
        Value::Array(items) => {
            for (i, item) in items.iter().enumerate() {
                collect(item, &format!("{path}/{i}"), values);
            }
        }
        Value::Object(items) => {
            for (key, item) in items {
                let key = key.replace('~', "~0").replace('/', "~1");
                collect(item, &format!("{path}/{key}"), values);
            }
        }
        _ => (),
    }
}

pub(crate) fn decode(json: &str) -> anyhow::Result<ReviewRecord> {
    anyhow::ensure!(json.len() <= 60 * 1024, "Encoded review exceeds bound");
    let mut wire: Wire = serde_json::from_str(json)?;
    anyhow::ensure!(
        wire.schema == 2
            && wire.references.len() <= MAX_REFERENCES
            && wire.strings.len() <= MAX_REFERENCES,
        "Unsupported or unbounded review wire format"
    );
    let mut paths = BTreeSet::new();
    let mut uses = vec![0; wire.strings.len()];
    let mut size = serde_json::to_vec(&wire.record)?.len();
    for reference in &wire.references {
        anyhow::ensure!(
            reference.path.starts_with('/')
                && reference.path.matches('/').count() <= 32
                && paths.insert(&reference.path),
            "Invalid or duplicate review reference"
        );
        let string = wire
            .strings
            .get(reference.index)
            .ok_or_else(|| anyhow::anyhow!("Missing review string"))?;
        size = size
            .checked_add(serde_json::to_vec(string)?.len())
            .and_then(|n| n.checked_sub(4))
            .ok_or_else(|| anyhow::anyhow!("Expanded review exceeds bound"))?;
        anyhow::ensure!(size <= MAX_EXPANDED, "Expanded review exceeds 1 MiB");
        let slot = wire
            .record
            .pointer_mut(&reference.path)
            .ok_or_else(|| anyhow::anyhow!("Missing review reference target"))?;
        anyhow::ensure!(slot.is_null(), "Review reference cannot overwrite a value");
        *slot = Value::String(string.clone());
        uses[reference.index] += 1;
    }
    anyhow::ensure!(
        uses.iter().all(|n| *n >= 2),
        "Unused or nonshared review string"
    );
    Ok(serde_json::from_value(wire.record)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn fixture() -> anyhow::Result<ReviewRecord> {
        let summary = "Reviewed exact dependency path, tests, limitations and existing work authority; this is attributed review, not execution or consent. ".repeat(24);
        let assertions: Vec<_> = opdev_core::embedded_catalog()?
            .rules
            .iter()
            .take(24)
            .map(|rule| {
                json!({
                    "rule_id":rule.id, "outcome":"passed", "summary":summary,
                    "evidence":[{"kind":"review", "summary":summary,"location":"review.md"}]
                })
            })
            .collect();
        Ok(serde_json::from_value(
            json!({"schema":1,"kind":"semantic_review",
            "source_sha256":"a".repeat(64),"configuration_sha256":"b".repeat(64),"stage":"pre_merge",
            "acceptance_sha256":"c".repeat(64),"ledger":{"schema":2,"project":assertions,"changes":[]}}),
        )?)
    }

    #[test]
    fn repeated_review_round_trips_without_losing_any_claim() -> anyhow::Result<()> {
        let record = fixture()?;
        let original = serde_json::to_vec(&record)?;
        let encoded = encode(&record)?;
        assert!(original.len() > 60 * 1024);
        assert!(encoded.len() < original.len() / 3);
        assert_eq!(decode(&encoded)?, record);
        let body = record.discussion_body()?;
        assert!(body.len() <= 60 * 1024);
        assert_eq!(ReviewRecord::from_discussion_body(&body)?, record);
        Ok(())
    }

    #[test]
    fn old_records_remain_readable_and_mixed_or_unknown_versions_fail() -> anyhow::Result<()> {
        let mut record = fixture()?;
        record.ledger.project.truncate(1);
        let body = format!(
            "<!-- opdev-review:v1 -->\n```json\n{}\n```\n<!-- opdev-review:end -->",
            serde_json::to_string(&record)?
        );
        assert_eq!(ReviewRecord::from_discussion_body(&body)?, record);
        assert!(ReviewRecord::from_discussion_body(&body.replace(":v1", ":v99")).is_err());
        assert!(ReviewRecord::from_discussion_body(&(body + &record.discussion_body()?)).is_err());
        Ok(())
    }

    #[test]
    fn invalid_references_and_expansion_are_bounded() -> anyhow::Result<()> {
        let encoded = encode(&fixture()?)?;
        let base: Value = serde_json::from_str(&encoded)?;
        assert!(
            decode(&encoded.replace(
                "\"kind\":\"semantic_review\"",
                "\"kind\":\"other\",\"kind\":\"semantic_review\""
            ))
            .is_err()
        );
        for edit in 0..6 {
            let mut wire = base.clone();
            match edit {
                0 => wire["references"][0]["index"] = json!(4096),
                1 => wire["references"][0]["path"] = json!("/missing"),
                2 => wire["references"][0]["path"] = json!("/schema"),
                3 => wire["references"][1] = wire["references"][0].clone(),
                4 => wire["strings"][0] = json!("x".repeat(32 * 1024)),
                _ => wire["unexpected"] = json!(true),
            }
            assert!(
                decode(&serde_json::to_string(&wire)?).is_err(),
                "mutation {edit}"
            );
        }
        let mut wire = base;
        wire["strings"]
            .as_array_mut()
            .ok_or_else(|| anyhow::anyhow!("strings"))?
            .push(json!("unused"));
        assert!(decode(&serde_json::to_string(&wire)?).is_err());
        Ok(())
    }

    #[test]
    fn unique_oversized_review_is_rejected_not_truncated() -> anyhow::Result<()> {
        let mut record = fixture()?;
        record.ledger.project.truncate(1);
        record.ledger.project[0].summary = "unique".repeat(12 * 1024);
        assert!(record.discussion_body().is_err());
        Ok(())
    }
}
