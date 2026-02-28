//! Newtype IDs — prevent ID confusion at compile time.

use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

/// Generate a newtype wrapper around `Uuid` with Display, FromStr, Serialize, etc.
#[macro_export]
macro_rules! define_id {
    ($name:ident) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(Uuid);

        impl $name {
            #[inline]
            pub fn new() -> Self {
                Self(Uuid::new_v4())
            }

            #[inline]
            pub fn from_uuid(id: Uuid) -> Self {
                Self(id)
            }

            #[inline]
            pub fn as_uuid(&self) -> &Uuid {
                &self.0
            }

            #[inline]
            pub fn into_uuid(self) -> Uuid {
                self.0
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.0.fmt(f)
            }
        }

        impl std::str::FromStr for $name {
            type Err = uuid::Error;
            fn from_str(s: &str) -> Result<Self, Self::Err> {
                Ok(Self(s.parse()?))
            }
        }

        impl From<Uuid> for $name {
            fn from(id: Uuid) -> Self {
                Self(id)
            }
        }

        impl From<$name> for Uuid {
            fn from(id: $name) -> Self {
                id.0
            }
        }
    };
}

define_id!(AgentId);
define_id!(WorkspaceId);
define_id!(ExecutionId);
define_id!(MessageId);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_serde() {
        let id = AgentId::new();
        let json = serde_json::to_string(&id).unwrap();
        let parsed: AgentId = serde_json::from_str(&json).unwrap();
        assert_eq!(id, parsed);
    }

    #[test]
    fn from_str_roundtrip() {
        let id = ExecutionId::new();
        let s = id.to_string();
        let parsed: ExecutionId = s.parse().unwrap();
        assert_eq!(id, parsed);
    }

    #[test]
    fn distinct_types() {
        let a = AgentId::new();
        let w = WorkspaceId::from_uuid(a.into_uuid());
        assert_eq!(a.into_uuid(), w.into_uuid());
        // AgentId != WorkspaceId at the type level — won't compile if mixed.
    }
}
