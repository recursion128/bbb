use anyhow::Result;
use serde::Deserialize;

/// Deterministic expression facts emitted by the shared Tree-sitter Java frontend.
///
/// This is not a Java parser. It only deserializes checked-in CST projections so
/// pack semantic resolvers can traverse one common representation.
#[derive(Debug, Deserialize)]
pub(crate) struct GeneratedJavaExpression {
    pub(crate) kind: String,
    pub(crate) canonical: String,
    #[serde(default)]
    pub(crate) name: Option<String>,
    #[serde(default)]
    pub(crate) object: Option<Box<GeneratedJavaExpression>>,
    #[serde(default)]
    pub(crate) arguments: Vec<GeneratedJavaExpression>,
    #[serde(default)]
    pub(crate) children: Vec<GeneratedJavaExpression>,
}

impl GeneratedJavaExpression {
    pub(crate) fn call(&self, name: &str) -> Option<&Self> {
        self.calls().find(|call| call.name.as_deref() == Some(name))
    }

    pub(crate) fn calls(&self) -> impl Iterator<Item = &Self> {
        self.expressions()
            .into_iter()
            .filter(|expression| expression.kind == "method_invocation")
    }

    pub(crate) fn calls_in_source_order(&self) -> Vec<&Self> {
        let mut calls = Vec::new();
        self.collect_calls_in_source_order(&mut calls);
        calls
    }

    pub(crate) fn creations(&self) -> impl Iterator<Item = &Self> {
        self.expressions()
            .into_iter()
            .filter(|expression| expression.kind == "object_creation_expression")
    }

    pub(crate) fn field_accesses(&self) -> impl Iterator<Item = &Self> {
        self.expressions()
            .into_iter()
            .filter(|expression| expression.kind == "field_access")
    }

    fn expressions(&self) -> Vec<&Self> {
        let mut expressions = Vec::new();
        self.collect_expressions(&mut expressions);
        expressions
    }

    fn collect_expressions<'a>(&'a self, expressions: &mut Vec<&'a Self>) {
        expressions.push(self);
        if let Some(object) = &self.object {
            object.collect_expressions(expressions);
        }
        for argument in &self.arguments {
            argument.collect_expressions(expressions);
        }
        for child in &self.children {
            child.collect_expressions(expressions);
        }
    }

    fn collect_calls_in_source_order<'a>(&'a self, calls: &mut Vec<&'a Self>) {
        if let Some(object) = &self.object {
            object.collect_calls_in_source_order(calls);
        }
        for argument in &self.arguments {
            argument.collect_calls_in_source_order(calls);
        }
        for child in &self.children {
            child.collect_calls_in_source_order(calls);
        }
        if self.kind == "method_invocation" {
            calls.push(self);
        }
    }

    pub(crate) fn field_constant(&self, owner: &str) -> Option<&str> {
        (self.kind == "field_access"
            && self
                .object
                .as_deref()
                .map(|object| object.canonical.as_str())
                == Some(owner))
        .then(|| self.name.as_deref())
        .flatten()
    }

    pub(crate) fn string_literal(&self) -> Result<Option<String>> {
        if self.kind != "string_literal" {
            return Ok(None);
        }
        serde_json::from_str(&self.canonical)
            .map(Some)
            .map_err(Into::into)
    }
}
