use serde::{Deserialize, Serialize};
use uuid::Uuid;

macro_rules! define_id {
    ($name:ident) => {
        #[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
        pub struct $name(pub String);

        impl $name {
            pub fn new() -> Self {
                Self(Uuid::new_v4().to_string())
            }

            pub fn from_string(value: impl Into<String>) -> Self {
                Self(value.into())
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(&self.0)
            }
        }
    };
}

define_id!(ProjectId);
define_id!(ThreadId);
define_id!(TurnId);
define_id!(MessageId);
define_id!(CommandId);
define_id!(EventId);
define_id!(EffectId);
define_id!(ToolCallId);
define_id!(ApprovalId);
define_id!(CheckpointId);
define_id!(TerminalId);
define_id!(PeerId);
define_id!(TaskId);
