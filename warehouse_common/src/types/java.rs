//! The Eclipse Temurin JDK catalog.

use crate::types::catalog::CatalogId;
use chrono::{DateTime, Utc};
use core::cmp::Ordering;
use core::fmt::{self, Display, Formatter};
use core::str::FromStr;
use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use utoipa::ToSchema;

extern crate alloc;

/// A Temurin JDK release.
///
/// Temurin publishes two incompatible version shapes: Java 9 and later use
/// `major.minor.patch_build` (`21.0.5_11`), while Java 8 uses `8uPATCH-bBUILD`
/// (`8u422-b05`). Both serialize as the string the upstream itself uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum JavaVersion {
    /// Java 9 and later.
    Modern {
        /// Feature release, for example `21`.
        major: u16,
        /// Interim release.
        minor: u16,
        /// Update release.
        patch: u16,
        /// Temurin build number.
        build: u16,
    },
    /// Java 8.
    Legacy {
        /// Always `8`.
        major: u16,
        /// Update number.
        patch: u16,
        /// Temurin build number.
        build: u16,
    },
}

impl JavaVersion {
    /// The feature release number, for example `21` or `8`.
    #[must_use]
    pub const fn major(self) -> u16 {
        match self {
            Self::Modern { major, .. } | Self::Legacy { major, .. } => major,
        }
    }

    const fn as_array(self) -> [u16; 4] {
        match self {
            Self::Modern {
                major,
                minor,
                patch,
                build,
            } => [major, minor, patch, build],
            Self::Legacy {
                major,
                patch,
                build,
            } => [major, 0, patch, build],
        }
    }
}

impl Display for JavaVersion {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Modern {
                major,
                minor,
                patch,
                build,
            } => write!(formatter, "{major}.{minor}.{patch}_{build}"),
            Self::Legacy {
                major,
                patch,
                build,
            } => write!(formatter, "{major}u{patch}-b{build:02}"),
        }
    }
}

/// Returned when a string is not a recognized Temurin version.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseJavaVersionError(pub String);

impl Display for ParseJavaVersionError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        write!(formatter, "invalid java version {:?}", self.0)
    }
}

impl core::error::Error for ParseJavaVersionError {}

impl FromStr for JavaVersion {
    type Err = ParseJavaVersionError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let invalid = || ParseJavaVersionError(value.to_owned());

        if let Some((head, build)) = value.split_once('_') {
            let mut parts = head.split('.');
            let (Some(major), Some(minor), Some(patch), None) =
                (parts.next(), parts.next(), parts.next(), parts.next())
            else {
                return Err(invalid());
            };
            let major: u16 = major.parse().map_err(|_err| invalid())?;
            if major == 0 {
                return Err(invalid());
            }
            return Ok(Self::Modern {
                major,
                minor: minor.parse().map_err(|_err| invalid())?,
                patch: patch.parse().map_err(|_err| invalid())?,
                build: build.parse().map_err(|_err| invalid())?,
            });
        }

        let (major, rest) = value.split_once('u').ok_or_else(invalid)?;
        let (patch, build) = rest.split_once("-b").ok_or_else(invalid)?;
        Ok(Self::Legacy {
            major: major.parse().map_err(|_err| invalid())?,
            patch: patch.parse().map_err(|_err| invalid())?,
            build: build.parse().map_err(|_err| invalid())?,
        })
    }
}

impl Ord for JavaVersion {
    fn cmp(&self, other: &Self) -> Ordering {
        self.as_array().cmp(&other.as_array())
    }
}

impl PartialOrd for JavaVersion {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Serialize for JavaVersion {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for JavaVersion {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = alloc::borrow::Cow::<'de, str>::deserialize(deserializer)?;
        raw.parse().map_err(D::Error::custom)
    }
}

impl utoipa::PartialSchema for JavaVersion {
    fn schema() -> utoipa::openapi::RefOr<utoipa::openapi::schema::Schema> {
        utoipa::openapi::schema::Object::builder()
            .schema_type(utoipa::openapi::schema::Type::String)
            .description(Some(
                "A Temurin release: `21.0.5_11` for Java 9 and later, `8u422-b05` for Java 8.",
            ))
            .examples([
                serde_json::json!("21.0.5_11"),
                serde_json::json!("8u422-b05"),
            ])
            .into()
    }
}

impl utoipa::ToSchema for JavaVersion {}

/// Every tracked release of one Java feature version.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct JavaMajor {
    /// The feature release number, for example `21`.
    pub major: u16,
    /// Releases within that feature version, newest first.
    pub versions: Vec<JavaVersion>,
}

/// The `java` catalog document.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct JavaCatalog {
    /// Schema version of this document.
    pub schema: u32,
    /// When this document was built from upstream.
    pub updated_at: DateTime<Utc>,
    /// Feature versions, newest first.
    pub majors: Vec<JavaMajor>,
}

impl JavaCatalog {
    /// Which catalog this document belongs to.
    pub const ID: CatalogId = CatalogId::Java;

    /// Looks up one feature version.
    #[must_use]
    pub fn major(&self, major: u16) -> Option<&JavaMajor> {
        self.majors.iter().find(|entry| entry.major == major)
    }
}

#[cfg(test)]
mod tests {
    use super::JavaVersion;

    #[test]
    fn parses_both_temurin_shapes() {
        assert_eq!(
            "21.0.5_11".parse::<JavaVersion>().expect("modern"),
            JavaVersion::Modern {
                major: 21,
                minor: 0,
                patch: 5,
                build: 11
            }
        );
        assert_eq!(
            "8u422-b05".parse::<JavaVersion>().expect("legacy"),
            JavaVersion::Legacy {
                major: 8,
                patch: 422,
                build: 5
            }
        );
    }

    #[test]
    fn round_trips_through_display() {
        for raw in ["21.0.5_11", "8u422-b05"] {
            let parsed: JavaVersion = raw.parse().expect("parse");
            assert_eq!(parsed.to_string(), raw);
        }
    }

    #[test]
    fn orders_legacy_below_modern() {
        let legacy: JavaVersion = "8u422-b05".parse().expect("legacy");
        let modern: JavaVersion = "21.0.5_11".parse().expect("modern");
        assert!(legacy < modern);
    }

    #[test]
    fn rejects_malformed_input() {
        for raw in ["", "21.0.5", "0.1.2_3", "8u422", "latest"] {
            assert!(raw.parse::<JavaVersion>().is_err(), "accepted {raw:?}");
        }
    }
}
