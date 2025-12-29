//! Lint context for Kubernetes objects.
//!
//! The lint context holds all parsed Kubernetes objects and provides
//! access to them during check execution.

pub mod object;

pub use object::{InvalidObject, K8sObject, Object, ObjectMetadata};

/// A lint context provides access to all parsed Kubernetes objects.
pub trait LintContext: Send + Sync {
    /// Get all valid parsed objects.
    fn objects(&self) -> &[Object];

    /// Get all objects that failed to parse.
    fn invalid_objects(&self) -> &[InvalidObject];
}

/// Default implementation of LintContext.
#[derive(Debug, Default)]
pub struct LintContextImpl {
    objects: Vec<Object>,
    invalid_objects: Vec<InvalidObject>,
}

impl LintContextImpl {
    /// Create a new empty lint context.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a valid object to the context.
    pub fn add_object(&mut self, object: Object) {
        self.objects.push(object);
    }

    /// Add an invalid object to the context.
    pub fn add_invalid_object(&mut self, invalid: InvalidObject) {
        self.invalid_objects.push(invalid);
    }

    /// Get a mutable reference to the objects.
    pub fn objects_mut(&mut self) -> &mut Vec<Object> {
        &mut self.objects
    }
}

impl LintContext for LintContextImpl {
    fn objects(&self) -> &[Object] {
        &self.objects
    }

    fn invalid_objects(&self) -> &[InvalidObject] {
        &self.invalid_objects
    }
}
