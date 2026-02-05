//! JSON Patch utilities for AG-UI state delta generation.
//!
//! This module provides utilities for working with JSON Patch (RFC 6902)
//! operations, enabling efficient state synchronization between agents and
//! frontends through delta updates.
//!
//! # Overview
//!
//! JSON Patch is a format for describing changes to a JSON document. Instead
//! of sending the entire state on every update, you can send just the changes
//! (patches) which is more efficient for large state objects.
//!
//! # Example
//!
//! ```rust
//! use ag_ui_core::patch::{create_patch, apply_patch};
//! use serde_json::json;
//!
//! // Create a patch from two states
//! let old_state = json!({"count": 0, "items": []});
//! let new_state = json!({"count": 1, "items": ["apple"]});
//!
//! let patch = create_patch(&old_state, &new_state);
//!
//! // Apply patch to recreate the new state
//! let mut state = old_state.clone();
//! apply_patch(&mut state, &patch).unwrap();
//! assert_eq!(state, new_state);
//! ```

use serde_json::Value as JsonValue;
use std::error::Error;
use std::fmt;

// Re-export json_patch types for convenience
pub use json_patch::{
    AddOperation, CopyOperation, MoveOperation, Patch, PatchOperation, RemoveOperation,
    ReplaceOperation, TestOperation,
};
use jsonptr::PointerBuf;

/// Error type for patch operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatchError {
    message: String,
}

impl PatchError {
    /// Creates a new patch error with the given message.
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for PatchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Patch error: {}", self.message)
    }
}

impl Error for PatchError {}

impl From<json_patch::PatchError> for PatchError {
    fn from(err: json_patch::PatchError) -> Self {
        Self::new(format!("{}", err))
    }
}

/// Creates a JSON Patch representing the difference between two JSON values.
///
/// The patch, when applied to `from`, will produce `to`.
///
/// # Example
///
/// ```rust
/// use ag_ui_core::patch::create_patch;
/// use serde_json::json;
///
/// let from = json!({"name": "Alice", "age": 30});
/// let to = json!({"name": "Alice", "age": 31});
///
/// let patch = create_patch(&from, &to);
///
/// // The patch contains a "replace" operation for the age field
/// assert!(!patch.0.is_empty());
/// ```
pub fn create_patch(from: &JsonValue, to: &JsonValue) -> Patch {
    json_patch::diff(from, to)
}

/// Applies a JSON Patch to a JSON value in place.
///
/// # Errors
///
/// Returns an error if any patch operation fails (e.g., path doesn't exist
/// for a remove operation, or test operation fails).
///
/// # Example
///
/// ```rust
/// use ag_ui_core::patch::{create_patch, apply_patch};
/// use serde_json::json;
///
/// let mut state = json!({"count": 0});
/// let patch = create_patch(&json!({"count": 0}), &json!({"count": 5}));
///
/// apply_patch(&mut state, &patch).unwrap();
/// assert_eq!(state["count"], 5);
/// ```
pub fn apply_patch(target: &mut JsonValue, patch: &Patch) -> Result<(), PatchError> {
    json_patch::patch(target, patch.0.as_slice()).map_err(PatchError::from)
}

/// Applies a JSON Patch from a JSON array representation.
///
/// This is useful when you receive patches as raw JSON values (e.g., from
/// network events).
///
/// # Errors
///
/// Returns an error if the patch is not a valid JSON Patch array or if
/// any operation fails.
///
/// # Example
///
/// ```rust
/// use ag_ui_core::patch::apply_patch_from_value;
/// use serde_json::json;
///
/// let mut state = json!({"count": 0});
/// let patch_json = json!([
///     {"op": "replace", "path": "/count", "value": 10}
/// ]);
///
/// apply_patch_from_value(&mut state, &patch_json).unwrap();
/// assert_eq!(state["count"], 10);
/// ```
pub fn apply_patch_from_value(target: &mut JsonValue, patch: &JsonValue) -> Result<(), PatchError> {
    let patch: Patch = serde_json::from_value(patch.clone())
        .map_err(|e| PatchError::new(format!("Invalid patch format: {}", e)))?;
    apply_patch(target, &patch)
}

/// Converts a Patch to a JSON value for serialization.
///
/// This is useful when you need to send patches over the network or
/// store them as JSON.
///
/// # Example
///
/// ```rust
/// use ag_ui_core::patch::{create_patch, patch_to_value};
/// use serde_json::json;
///
/// let patch = create_patch(
///     &json!({"x": 1}),
///     &json!({"x": 2}),
/// );
///
/// let json = patch_to_value(&patch);
/// assert!(json.is_array());
/// ```
pub fn patch_to_value(patch: &Patch) -> JsonValue {
    serde_json::to_value(patch).unwrap_or(JsonValue::Array(vec![]))
}

/// Converts a Patch to a vector of JSON values.
///
/// This is the format expected by StateDeltaEvent and ActivityDeltaEvent.
///
/// # Example
///
/// ```rust
/// use ag_ui_core::patch::{create_patch, patch_to_vec};
/// use serde_json::json;
///
/// let patch = create_patch(
///     &json!({"items": []}),
///     &json!({"items": ["a"]}),
/// );
///
/// let ops = patch_to_vec(&patch);
/// // Each operation is a separate JSON object
/// assert!(!ops.is_empty());
/// ```
pub fn patch_to_vec(patch: &Patch) -> Vec<JsonValue> {
    patch
        .0
        .iter()
        .filter_map(|op| serde_json::to_value(op).ok())
        .collect()
}

/// A builder for constructing JSON Patches programmatically.
///
/// This provides a more ergonomic way to create patches when you know
/// exactly what operations you want to perform.
///
/// # Example
///
/// ```rust
/// use ag_ui_core::patch::PatchBuilder;
/// use serde_json::json;
///
/// let patch = PatchBuilder::new()
///     .add("/name", json!("Alice"))
///     .replace("/age", json!(31))
///     .remove("/temp")
///     .build();
///
/// assert_eq!(patch.0.len(), 3);
/// ```
#[derive(Debug, Clone, Default)]
pub struct PatchBuilder {
    operations: Vec<PatchOperation>,
}

impl PatchBuilder {
    /// Creates a new empty patch builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds an "add" operation to the patch.
    ///
    /// The add operation adds a value at the target location. If the target
    /// location specifies an array index, the value is inserted at that index.
    pub fn add(mut self, path: impl AsRef<str>, value: JsonValue) -> Self {
        self.operations.push(PatchOperation::Add(AddOperation {
            path: PointerBuf::parse(path.as_ref()).unwrap_or_default(),
            value,
        }));
        self
    }

    /// Adds a "remove" operation to the patch.
    ///
    /// The remove operation removes the value at the target location.
    pub fn remove(mut self, path: impl AsRef<str>) -> Self {
        self.operations
            .push(PatchOperation::Remove(RemoveOperation {
                path: PointerBuf::parse(path.as_ref()).unwrap_or_default(),
            }));
        self
    }

    /// Adds a "replace" operation to the patch.
    ///
    /// The replace operation replaces the value at the target location with
    /// the new value.
    pub fn replace(mut self, path: impl AsRef<str>, value: JsonValue) -> Self {
        self.operations
            .push(PatchOperation::Replace(ReplaceOperation {
                path: PointerBuf::parse(path.as_ref()).unwrap_or_default(),
                value,
            }));
        self
    }

    /// Adds a "move" operation to the patch.
    ///
    /// The move operation removes the value at a specified location and
    /// adds it to the target location.
    pub fn move_value(mut self, from: impl AsRef<str>, path: impl AsRef<str>) -> Self {
        self.operations.push(PatchOperation::Move(MoveOperation {
            from: PointerBuf::parse(from.as_ref()).unwrap_or_default(),
            path: PointerBuf::parse(path.as_ref()).unwrap_or_default(),
        }));
        self
    }

    /// Adds a "copy" operation to the patch.
    ///
    /// The copy operation copies the value at a specified location to the
    /// target location.
    pub fn copy(mut self, from: impl AsRef<str>, path: impl AsRef<str>) -> Self {
        self.operations.push(PatchOperation::Copy(CopyOperation {
            from: PointerBuf::parse(from.as_ref()).unwrap_or_default(),
            path: PointerBuf::parse(path.as_ref()).unwrap_or_default(),
        }));
        self
    }

    /// Adds a "test" operation to the patch.
    ///
    /// The test operation tests that a value at the target location is equal
    /// to a specified value. If the test fails, the entire patch fails.
    pub fn test(mut self, path: impl AsRef<str>, value: JsonValue) -> Self {
        self.operations.push(PatchOperation::Test(TestOperation {
            path: PointerBuf::parse(path.as_ref()).unwrap_or_default(),
            value,
        }));
        self
    }

    /// Builds the patch from the accumulated operations.
    pub fn build(self) -> Patch {
        Patch(self.operations)
    }

    /// Builds the patch and returns it as a vector of JSON values.
    ///
    /// This is the format expected by StateDeltaEvent and ActivityDeltaEvent.
    pub fn build_vec(self) -> Vec<JsonValue> {
        patch_to_vec(&self.build())
    }
}

/// Checks if applying a patch would succeed without actually modifying the target.
///
/// This is useful for validation before committing to a patch operation.
///
/// # Example
///
/// ```rust
/// use ag_ui_core::patch::{can_apply_patch, PatchBuilder};
/// use serde_json::json;
///
/// let state = json!({"count": 0});
/// let valid_patch = PatchBuilder::new().replace("/count", json!(1)).build();
/// let invalid_patch = PatchBuilder::new().remove("/nonexistent").build();
///
/// assert!(can_apply_patch(&state, &valid_patch));
/// assert!(!can_apply_patch(&state, &invalid_patch));
/// ```
pub fn can_apply_patch(target: &JsonValue, patch: &Patch) -> bool {
    let mut test_target = target.clone();
    apply_patch(&mut test_target, patch).is_ok()
}

/// Merges two patches into one.
///
/// The resulting patch applies the operations from the first patch followed
/// by operations from the second patch.
///
/// Note: This is a simple concatenation and does not optimize or simplify
/// the resulting patch.
///
/// # Example
///
/// ```rust
/// use ag_ui_core::patch::{merge_patches, PatchBuilder};
/// use serde_json::json;
///
/// let patch1 = PatchBuilder::new().add("/a", json!(1)).build();
/// let patch2 = PatchBuilder::new().add("/b", json!(2)).build();
///
/// let merged = merge_patches(&patch1, &patch2);
/// assert_eq!(merged.0.len(), 2);
/// ```
pub fn merge_patches(first: &Patch, second: &Patch) -> Patch {
    let mut operations = first.0.clone();
    operations.extend(second.0.clone());
    Patch(operations)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_create_patch_simple() {
        let from = json!({"count": 0});
        let to = json!({"count": 5});

        let patch = create_patch(&from, &to);
        assert!(!patch.0.is_empty());

        // Apply patch and verify
        let mut result = from.clone();
        apply_patch(&mut result, &patch).unwrap();
        assert_eq!(result, to);
    }

    #[test]
    fn test_create_patch_add_field() {
        let from = json!({"name": "Alice"});
        let to = json!({"name": "Alice", "age": 30});

        let patch = create_patch(&from, &to);

        let mut result = from.clone();
        apply_patch(&mut result, &patch).unwrap();
        assert_eq!(result, to);
    }

    #[test]
    fn test_create_patch_remove_field() {
        let from = json!({"name": "Alice", "temp": "value"});
        let to = json!({"name": "Alice"});

        let patch = create_patch(&from, &to);

        let mut result = from.clone();
        apply_patch(&mut result, &patch).unwrap();
        assert_eq!(result, to);
    }

    #[test]
    fn test_create_patch_array_operations() {
        let from = json!({"items": ["a", "b"]});
        let to = json!({"items": ["a", "b", "c"]});

        let patch = create_patch(&from, &to);

        let mut result = from.clone();
        apply_patch(&mut result, &patch).unwrap();
        assert_eq!(result, to);
    }

    #[test]
    fn test_apply_patch_from_value() {
        let mut state = json!({"count": 0});
        let patch_json = json!([
            {"op": "replace", "path": "/count", "value": 42}
        ]);

        apply_patch_from_value(&mut state, &patch_json).unwrap();
        assert_eq!(state["count"], 42);
    }

    #[test]
    fn test_apply_patch_from_value_invalid() {
        let mut state = json!({"count": 0});
        let invalid_patch = json!("not an array");

        let result = apply_patch_from_value(&mut state, &invalid_patch);
        assert!(result.is_err());
    }

    #[test]
    fn test_patch_to_value() {
        let patch = create_patch(&json!({"x": 1}), &json!({"x": 2}));
        let value = patch_to_value(&patch);

        assert!(value.is_array());
    }

    #[test]
    fn test_patch_to_vec() {
        let patch = create_patch(&json!({"a": 1, "b": 2}), &json!({"a": 1, "b": 3, "c": 4}));
        let ops = patch_to_vec(&patch);

        // Should have operations for changing b and adding c
        assert!(!ops.is_empty());
        for op in &ops {
            assert!(op.is_object());
            assert!(op.get("op").is_some());
        }
    }

    #[test]
    fn test_patch_builder_add() {
        let patch = PatchBuilder::new()
            .add("/name", json!("Alice"))
            .build();

        let mut state = json!({});
        apply_patch(&mut state, &patch).unwrap();
        assert_eq!(state["name"], "Alice");
    }

    #[test]
    fn test_patch_builder_replace() {
        let patch = PatchBuilder::new()
            .replace("/count", json!(10))
            .build();

        let mut state = json!({"count": 0});
        apply_patch(&mut state, &patch).unwrap();
        assert_eq!(state["count"], 10);
    }

    #[test]
    fn test_patch_builder_remove() {
        let patch = PatchBuilder::new().remove("/temp").build();

        let mut state = json!({"name": "Alice", "temp": "value"});
        apply_patch(&mut state, &patch).unwrap();
        assert!(state.get("temp").is_none());
        assert_eq!(state["name"], "Alice");
    }

    #[test]
    fn test_patch_builder_move() {
        let patch = PatchBuilder::new()
            .move_value("/old", "/new")
            .build();

        let mut state = json!({"old": "value"});
        apply_patch(&mut state, &patch).unwrap();
        assert!(state.get("old").is_none());
        assert_eq!(state["new"], "value");
    }

    #[test]
    fn test_patch_builder_copy() {
        let patch = PatchBuilder::new()
            .copy("/source", "/dest")
            .build();

        let mut state = json!({"source": "value"});
        apply_patch(&mut state, &patch).unwrap();
        assert_eq!(state["source"], "value");
        assert_eq!(state["dest"], "value");
    }

    #[test]
    fn test_patch_builder_test() {
        // Test operation succeeds
        let patch = PatchBuilder::new()
            .test("/count", json!(0))
            .replace("/count", json!(1))
            .build();

        let mut state = json!({"count": 0});
        apply_patch(&mut state, &patch).unwrap();
        assert_eq!(state["count"], 1);
    }

    #[test]
    fn test_patch_builder_test_fails() {
        let patch = PatchBuilder::new()
            .test("/count", json!(999)) // Wrong value
            .replace("/count", json!(1))
            .build();

        let mut state = json!({"count": 0});
        let result = apply_patch(&mut state, &patch);
        assert!(result.is_err());
    }

    #[test]
    fn test_patch_builder_build_vec() {
        let ops = PatchBuilder::new()
            .add("/a", json!(1))
            .replace("/b", json!(2))
            .build_vec();

        assert_eq!(ops.len(), 2);
    }

    #[test]
    fn test_can_apply_patch() {
        let state = json!({"count": 0});

        let valid_patch = PatchBuilder::new().replace("/count", json!(1)).build();
        assert!(can_apply_patch(&state, &valid_patch));

        let invalid_patch = PatchBuilder::new().remove("/nonexistent").build();
        assert!(!can_apply_patch(&state, &invalid_patch));
    }

    #[test]
    fn test_merge_patches() {
        let patch1 = PatchBuilder::new().add("/a", json!(1)).build();
        let patch2 = PatchBuilder::new().add("/b", json!(2)).build();

        let merged = merge_patches(&patch1, &patch2);
        assert_eq!(merged.0.len(), 2);

        let mut state = json!({});
        apply_patch(&mut state, &merged).unwrap();
        assert_eq!(state["a"], 1);
        assert_eq!(state["b"], 2);
    }

    #[test]
    fn test_patch_error_display() {
        let err = PatchError::new("test error");
        assert!(err.to_string().contains("test error"));
    }

    #[test]
    fn test_complex_nested_patch() {
        let from = json!({
            "user": {
                "profile": {
                    "name": "Alice",
                    "settings": {
                        "theme": "light"
                    }
                }
            }
        });

        let to = json!({
            "user": {
                "profile": {
                    "name": "Alice",
                    "settings": {
                        "theme": "dark",
                        "notifications": true
                    }
                }
            }
        });

        let patch = create_patch(&from, &to);

        let mut result = from.clone();
        apply_patch(&mut result, &patch).unwrap();
        assert_eq!(result, to);
    }

    #[test]
    fn test_empty_patch() {
        let state = json!({"count": 0});
        let patch = create_patch(&state, &state);

        // Patch of identical values should be empty
        assert!(patch.0.is_empty());

        // Applying empty patch should be no-op
        let mut result = state.clone();
        apply_patch(&mut result, &patch).unwrap();
        assert_eq!(result, state);
    }
}
