extern crate alloc;

use core::cmp::Ordering;
use core::fmt::{self, Display, Formatter};
use core::str::FromStr;
use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Version {
    pub major: u16,
    pub minor: u16,
    pub patch: u16,
}

impl Version {
    #[must_use]
    pub const fn new(major: u16, minor: u16, patch: u16) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }
}

impl Display for Version {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseVersionError(pub String);

impl Display for ParseVersionError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        write!(formatter, "invalid version {:?}", self.0)
    }
}

impl core::error::Error for ParseVersionError {}

impl FromStr for Version {
    type Err = ParseVersionError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let invalid = || ParseVersionError(value.to_owned());
        let mut parts = value.split('.');
        let (Some(major), Some(minor), Some(patch), None) =
            (parts.next(), parts.next(), parts.next(), parts.next())
        else {
            return Err(invalid());
        };
        Ok(Self {
            major: major.parse().map_err(|_err| invalid())?,
            minor: minor.parse().map_err(|_err| invalid())?,
            patch: patch.parse().map_err(|_err| invalid())?,
        })
    }
}

impl Ord for Version {
    fn cmp(&self, other: &Self) -> Ordering {
        (self.major, self.minor, self.patch).cmp(&(other.major, other.minor, other.patch))
    }
}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Serialize for Version {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for Version {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = alloc::borrow::Cow::<'de, str>::deserialize(deserializer)?;
        raw.parse().map_err(D::Error::custom)
    }
}

impl utoipa::PartialSchema for Version {
    fn schema() -> utoipa::openapi::RefOr<utoipa::openapi::schema::Schema> {
        utoipa::openapi::schema::Object::builder()
            .schema_type(utoipa::openapi::schema::Type::String)
            .description(Some("A `major.minor.patch` release identifier."))
            .examples([serde_json::json!("1.24.5")])
            .into()
    }
}

impl utoipa::ToSchema for Version {}

#[cfg(test)]
mod tests {
    use super::Version;

    #[test]
    fn round_trips_through_json() {
        let version = Version::new(1, 24, 5);
        let encoded = serde_json::to_string(&version).expect("serialize");
        assert_eq!(encoded, "\"1.24.5\"");
        let decoded: Version = serde_json::from_str(&encoded).expect("deserialize");
        assert_eq!(decoded, version);
    }

    #[test]
    fn orders_numerically_not_lexically() {
        assert!(Version::new(1, 10, 0) > Version::new(1, 9, 9));
    }

    #[test]
    fn rejects_malformed_input() {
        "1.2"
            .parse::<Version>()
            .expect_err("should reject malformed input");
        "1.2.3.4"
            .parse::<Version>()
            .expect_err("should reject malformed input");
        "1.2.x"
            .parse::<Version>()
            .expect_err("should reject malformed input");
    }
}
