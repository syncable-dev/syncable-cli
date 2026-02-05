//! AG-UI State Management
//!
//! This module provides state management traits and utilities for AG-UI:
//! - `AgentState`: Marker trait for types that can represent agent state
//! - `FwdProps`: Marker trait for types that can be forwarded as props to UI
//! - `StateManager`: Helper for managing state and generating deltas
//!
//! These traits enable generic state handling in events while ensuring
//! the necessary bounds for serialization and async operations.
//!
//! # State Synchronization
//!
//! AG-UI supports two modes of state synchronization:
//! - **Snapshots**: Send the complete state (simpler but less efficient)
//! - **Deltas**: Send JSON Patch operations (more efficient for large states)
//!
//! The `StateManager` helper makes it easy to track state changes and
//! generate appropriate events.
//!
//! # Example
//!
//! ```rust
//! use ag_ui_core::state::StateManager;
//! use serde_json::json;
//!
//! let mut manager = StateManager::new(json!({"count": 0}));
//!
//! // Update state and get the delta
//! let delta = manager.update(json!({"count": 1}));
//! assert!(delta.is_some());
//!
//! // Get current state
//! assert_eq!(manager.current()["count"], 1);
//! ```

use crate::patch::{create_patch, Patch};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::fmt::Debug;

/// Marker trait for types that can represent agent state.
///
/// Types implementing this trait can be used as the state type in
/// state-related events (StateSnapshot, StateDelta, etc.).
///
/// # Bounds
///
/// - `'static`: Required for async operations
/// - `Debug`: For debugging and logging
/// - `Clone`: State may need to be copied
/// - `Send + Sync`: For thread-safe async operations
/// - `Serialize + Deserialize`: For JSON serialization
/// - `Default`: For initializing empty state
///
/// # Example
///
/// ```rust
/// use ag_ui_core::AgentState;
/// use serde::{Deserialize, Serialize};
///
/// #[derive(Debug, Clone, Default, Serialize, Deserialize)]
/// struct MyState {
///     counter: u32,
///     messages: Vec<String>,
/// }
///
/// impl AgentState for MyState {}
/// ```
pub trait AgentState:
    'static + Debug + Clone + Send + Sync + for<'de> Deserialize<'de> + Serialize + Default
{
}

/// Marker trait for types that can be forwarded as props to UI components.
///
/// Types implementing this trait can be passed through the AG-UI protocol
/// to frontend components as properties.
///
/// # Bounds
///
/// - `'static`: Required for async operations
/// - `Clone`: Props may need to be copied
/// - `Send + Sync`: For thread-safe async operations
/// - `Serialize + Deserialize`: For JSON serialization
/// - `Default`: For initializing empty props
///
/// # Example
///
/// ```rust
/// use ag_ui_core::FwdProps;
/// use serde::{Deserialize, Serialize};
///
/// #[derive(Clone, Default, Serialize, Deserialize)]
/// struct MyProps {
///     theme: String,
///     locale: String,
/// }
///
/// impl FwdProps for MyProps {}
/// ```
pub trait FwdProps:
    'static + Clone + Send + Sync + for<'de> Deserialize<'de> + Serialize + Default
{
}

// Implement AgentState for common types

impl AgentState for JsonValue {}
impl AgentState for () {}

// Implement FwdProps for common types

impl FwdProps for JsonValue {}
impl FwdProps for () {}

// =============================================================================
// State Helper Utilities
// =============================================================================

/// Computes the difference between two JSON states as a JSON Patch.
///
/// Returns `None` if the states are identical.
///
/// # Example
///
/// ```rust
/// use ag_ui_core::state::diff_states;
/// use serde_json::json;
///
/// let old = json!({"count": 0});
/// let new = json!({"count": 5});
///
/// let patch = diff_states(&old, &new);
/// assert!(patch.is_some());
/// ```
pub fn diff_states(old: &JsonValue, new: &JsonValue) -> Option<Patch> {
    let patch = create_patch(old, new);
    if patch.0.is_empty() {
        None
    } else {
        Some(patch)
    }
}

/// A helper for managing state and generating deltas.
///
/// `StateManager` tracks the current state and provides methods to update
/// it while automatically computing the JSON Patch delta between states.
/// This is useful for efficiently synchronizing state with frontends.
///
/// # Example
///
/// ```rust
/// use ag_ui_core::state::StateManager;
/// use serde_json::json;
///
/// let mut manager = StateManager::new(json!({"count": 0, "items": []}));
///
/// // Update state - returns delta patch
/// let delta = manager.update(json!({"count": 1, "items": []}));
/// assert!(delta.is_some());
///
/// // No change - returns None
/// let delta = manager.update(json!({"count": 1, "items": []}));
/// assert!(delta.is_none());
///
/// // Check current state
/// assert_eq!(manager.current()["count"], 1);
/// ```
#[derive(Debug, Clone)]
pub struct StateManager {
    current: JsonValue,
    version: u64,
}

impl StateManager {
    /// Creates a new state manager with the given initial state.
    pub fn new(initial: JsonValue) -> Self {
        Self {
            current: initial,
            version: 0,
        }
    }

    /// Returns a reference to the current state.
    pub fn current(&self) -> &JsonValue {
        &self.current
    }

    /// Returns the current state version (increments on each update).
    pub fn version(&self) -> u64 {
        self.version
    }

    /// Updates the state and returns the delta patch if there were changes.
    ///
    /// Returns `None` if the new state is identical to the current state.
    pub fn update(&mut self, new_state: JsonValue) -> Option<Patch> {
        let patch = diff_states(&self.current, &new_state);
        if patch.is_some() {
            self.current = new_state;
            self.version += 1;
        }
        patch
    }

    /// Updates the state using a closure and returns the delta patch.
    ///
    /// The closure receives a mutable reference to the current state.
    /// After the closure completes, the delta is computed.
    ///
    /// # Example
    ///
    /// ```rust
    /// use ag_ui_core::state::StateManager;
    /// use serde_json::json;
    ///
    /// let mut manager = StateManager::new(json!({"count": 0}));
    ///
    /// let delta = manager.update_with(|state| {
    ///     state["count"] = json!(10);
    /// });
    ///
    /// assert!(delta.is_some());
    /// assert_eq!(manager.current()["count"], 10);
    /// ```
    pub fn update_with<F>(&mut self, f: F) -> Option<Patch>
    where
        F: FnOnce(&mut JsonValue),
    {
        let old_state = self.current.clone();
        f(&mut self.current);
        let patch = diff_states(&old_state, &self.current);
        if patch.is_some() {
            self.version += 1;
        }
        patch
    }

    /// Resets the state to a new value without computing a delta.
    ///
    /// Use this when you want to replace the entire state (e.g., on reconnection)
    /// and will send a snapshot instead of a delta.
    pub fn reset(&mut self, new_state: JsonValue) {
        self.current = new_state;
        self.version += 1;
    }

    /// Takes a snapshot of the current state.
    ///
    /// Returns a clone of the current state value.
    pub fn snapshot(&self) -> JsonValue {
        self.current.clone()
    }
}

impl Default for StateManager {
    fn default() -> Self {
        Self::new(JsonValue::Object(serde_json::Map::new()))
    }
}

/// A typed state manager for custom state types.
///
/// This provides the same functionality as `StateManager` but works with
/// strongly-typed state objects that implement `AgentState`.
///
/// # Example
///
/// ```rust
/// use ag_ui_core::state::TypedStateManager;
/// use ag_ui_core::AgentState;
/// use serde::{Deserialize, Serialize};
///
/// #[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
/// struct AppState {
///     count: u32,
///     user: Option<String>,
/// }
///
/// impl AgentState for AppState {}
///
/// let mut manager = TypedStateManager::new(AppState { count: 0, user: None });
///
/// let delta = manager.update(AppState { count: 1, user: None });
/// assert!(delta.is_some());
///
/// assert_eq!(manager.current().count, 1);
/// ```
#[derive(Debug, Clone)]
pub struct TypedStateManager<S: AgentState> {
    current: S,
    version: u64,
}

impl<S: AgentState + PartialEq> TypedStateManager<S> {
    /// Creates a new typed state manager with the given initial state.
    pub fn new(initial: S) -> Self {
        Self {
            current: initial,
            version: 0,
        }
    }

    /// Returns a reference to the current state.
    pub fn current(&self) -> &S {
        &self.current
    }

    /// Returns the current state version (increments on each update).
    pub fn version(&self) -> u64 {
        self.version
    }

    /// Updates the state and returns the delta patch if there were changes.
    ///
    /// Returns `None` if the new state is identical to the current state.
    pub fn update(&mut self, new_state: S) -> Option<Patch> {
        if self.current == new_state {
            return None;
        }

        let old_json = serde_json::to_value(&self.current).ok()?;
        let new_json = serde_json::to_value(&new_state).ok()?;
        let patch = diff_states(&old_json, &new_json);

        self.current = new_state;
        self.version += 1;
        patch
    }

    /// Resets the state to a new value without computing a delta.
    pub fn reset(&mut self, new_state: S) {
        self.current = new_state;
        self.version += 1;
    }

    /// Takes a snapshot of the current state as JSON.
    pub fn snapshot(&self) -> JsonValue {
        serde_json::to_value(&self.current).unwrap_or(JsonValue::Null)
    }

    /// Returns the current state as a JSON value.
    pub fn as_json(&self) -> JsonValue {
        serde_json::to_value(&self.current).unwrap_or(JsonValue::Null)
    }
}

impl<S: AgentState + PartialEq> Default for TypedStateManager<S> {
    fn default() -> Self {
        Self::new(S::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, Default, Serialize, Deserialize)]
    struct TestState {
        value: i32,
    }

    impl AgentState for TestState {}

    #[derive(Clone, Default, Serialize, Deserialize)]
    struct TestProps {
        name: String,
    }

    impl FwdProps for TestProps {}

    #[test]
    fn test_json_value_implements_agent_state() {
        fn requires_agent_state<T: AgentState>(_: T) {}
        requires_agent_state(JsonValue::Null);
    }

    #[test]
    fn test_unit_implements_agent_state() {
        fn requires_agent_state<T: AgentState>(_: T) {}
        requires_agent_state(());
    }

    #[test]
    fn test_json_value_implements_fwd_props() {
        fn requires_fwd_props<T: FwdProps>(_: T) {}
        requires_fwd_props(JsonValue::Null);
    }

    #[test]
    fn test_unit_implements_fwd_props() {
        fn requires_fwd_props<T: FwdProps>(_: T) {}
        requires_fwd_props(());
    }

    #[test]
    fn test_custom_state_type() {
        fn requires_agent_state<T: AgentState>(_: T) {}
        requires_agent_state(TestState { value: 42 });
    }

    #[test]
    fn test_custom_props_type() {
        fn requires_fwd_props<T: FwdProps>(_: T) {}
        requires_fwd_props(TestProps {
            name: "test".to_string(),
        });
    }

    // =========================================================================
    // State Helper Tests
    // =========================================================================

    #[test]
    fn test_diff_states_with_changes() {
        use serde_json::json;

        let old = json!({"count": 0});
        let new = json!({"count": 5});

        let patch = diff_states(&old, &new);
        assert!(patch.is_some());
    }

    #[test]
    fn test_diff_states_no_changes() {
        use serde_json::json;

        let state = json!({"count": 0});

        let patch = diff_states(&state, &state);
        assert!(patch.is_none());
    }

    #[test]
    fn test_state_manager_new() {
        use serde_json::json;

        let manager = StateManager::new(json!({"count": 0}));
        assert_eq!(manager.current()["count"], 0);
        assert_eq!(manager.version(), 0);
    }

    #[test]
    fn test_state_manager_update_with_changes() {
        use serde_json::json;

        let mut manager = StateManager::new(json!({"count": 0}));

        let delta = manager.update(json!({"count": 5}));
        assert!(delta.is_some());
        assert_eq!(manager.current()["count"], 5);
        assert_eq!(manager.version(), 1);
    }

    #[test]
    fn test_state_manager_update_no_changes() {
        use serde_json::json;

        let mut manager = StateManager::new(json!({"count": 0}));

        let delta = manager.update(json!({"count": 0}));
        assert!(delta.is_none());
        assert_eq!(manager.version(), 0); // Version shouldn't increment
    }

    #[test]
    fn test_state_manager_update_with_closure() {
        use serde_json::json;

        let mut manager = StateManager::new(json!({"count": 0}));

        let delta = manager.update_with(|state| {
            state["count"] = json!(10);
        });

        assert!(delta.is_some());
        assert_eq!(manager.current()["count"], 10);
        assert_eq!(manager.version(), 1);
    }

    #[test]
    fn test_state_manager_update_with_no_changes() {
        use serde_json::json;

        let mut manager = StateManager::new(json!({"count": 0}));

        let delta = manager.update_with(|_state| {
            // No changes
        });

        assert!(delta.is_none());
        assert_eq!(manager.version(), 0);
    }

    #[test]
    fn test_state_manager_reset() {
        use serde_json::json;

        let mut manager = StateManager::new(json!({"count": 0}));
        manager.reset(json!({"count": 100, "new_field": true}));

        assert_eq!(manager.current()["count"], 100);
        assert_eq!(manager.current()["new_field"], true);
        assert_eq!(manager.version(), 1);
    }

    #[test]
    fn test_state_manager_snapshot() {
        use serde_json::json;

        let manager = StateManager::new(json!({"count": 42}));
        let snapshot = manager.snapshot();

        assert_eq!(snapshot, json!({"count": 42}));
    }

    #[test]
    fn test_state_manager_default() {
        let manager = StateManager::default();
        assert!(manager.current().is_object());
        assert_eq!(manager.version(), 0);
    }

    #[test]
    fn test_state_manager_multiple_updates() {
        use serde_json::json;

        let mut manager = StateManager::new(json!({"count": 0}));

        manager.update(json!({"count": 1}));
        manager.update(json!({"count": 2}));
        manager.update(json!({"count": 3}));

        assert_eq!(manager.current()["count"], 3);
        assert_eq!(manager.version(), 3);
    }

    // =========================================================================
    // TypedStateManager Tests
    // =========================================================================

    #[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
    struct AppState {
        count: u32,
        name: String,
    }

    impl AgentState for AppState {}

    #[test]
    fn test_typed_state_manager_new() {
        let manager = TypedStateManager::new(AppState {
            count: 0,
            name: "test".to_string(),
        });

        assert_eq!(manager.current().count, 0);
        assert_eq!(manager.current().name, "test");
        assert_eq!(manager.version(), 0);
    }

    #[test]
    fn test_typed_state_manager_update() {
        let mut manager = TypedStateManager::new(AppState {
            count: 0,
            name: "test".to_string(),
        });

        let delta = manager.update(AppState {
            count: 5,
            name: "test".to_string(),
        });

        assert!(delta.is_some());
        assert_eq!(manager.current().count, 5);
        assert_eq!(manager.version(), 1);
    }

    #[test]
    fn test_typed_state_manager_update_no_changes() {
        let mut manager = TypedStateManager::new(AppState {
            count: 0,
            name: "test".to_string(),
        });

        let delta = manager.update(AppState {
            count: 0,
            name: "test".to_string(),
        });

        assert!(delta.is_none());
        assert_eq!(manager.version(), 0);
    }

    #[test]
    fn test_typed_state_manager_reset() {
        let mut manager = TypedStateManager::new(AppState {
            count: 0,
            name: "old".to_string(),
        });

        manager.reset(AppState {
            count: 100,
            name: "new".to_string(),
        });

        assert_eq!(manager.current().count, 100);
        assert_eq!(manager.current().name, "new");
        assert_eq!(manager.version(), 1);
    }

    #[test]
    fn test_typed_state_manager_snapshot() {
        let manager = TypedStateManager::new(AppState {
            count: 42,
            name: "test".to_string(),
        });

        let snapshot = manager.snapshot();
        assert_eq!(snapshot["count"], 42);
        assert_eq!(snapshot["name"], "test");
    }

    #[test]
    fn test_typed_state_manager_as_json() {
        let manager = TypedStateManager::new(AppState {
            count: 10,
            name: "hello".to_string(),
        });

        let json = manager.as_json();
        assert_eq!(json["count"], 10);
        assert_eq!(json["name"], "hello");
    }

    #[test]
    fn test_typed_state_manager_default() {
        let manager: TypedStateManager<AppState> = TypedStateManager::default();
        assert_eq!(manager.current().count, 0);
        assert_eq!(manager.current().name, "");
        assert_eq!(manager.version(), 0);
    }
}
