pub mod parser;

// Parser sub-modules for different XSD elements
mod complex_type;
mod element;
mod schema_attributes;
mod schema_content;
mod sequence;
mod simple_type;

use crate::parser::QName;
use std::collections::HashMap;

/// XML Schema representation
#[derive(Debug, Default)]
pub struct XmlSchema {
    pub target_namespace: Option<String>,
    pub element_form_default: Option<String>,
    pub attribute_form_default: Option<String>,
    pub version: Option<String>,
    pub namespaces: HashMap<String, String>,
    pub elements: HashMap<String, SchemaElement>,
    pub complex_types: HashMap<String, ComplexType>,
    pub simple_types: HashMap<String, SimpleType>,
}

/// A top-level or nested element definition
#[derive(Debug, Default, Clone)]
pub struct SchemaElement {
    pub name: String,
    pub type_: QName,
    pub nillable: bool,
    pub min_occurs: Option<u32>,
    pub max_occurs: Option<String>,
}

/// A complex type definition with structure
#[derive(Debug, Default, Clone)]
pub struct ComplexType {
    pub name: String,
    pub sequence: Option<Sequence>,
    pub choice: Option<Choice>,
    pub all: Option<All>,
    // For extensions and restrictions
    pub base_type: Option<QName>,
    // XML attributes
    pub attributes: Vec<Attribute>,
}

/// A sequence of elements (ordered)
#[derive(Debug, Default, Clone)]
pub struct Sequence {
    pub elements: Vec<SequenceElement>,
}

/// An element within a sequence
#[derive(Debug, Default, Clone)]
pub struct SequenceElement {
    pub name: String,
    pub type_: QName,
    pub min_occurs: u32,
    pub max_occurs: Option<String>, // "unbounded" or a number
    pub nillable: bool,
}

/// A simple type definition (restriction, list, union)
#[derive(Debug, Clone)]
pub enum SimpleType {
    /// Restriction of another type
    Restriction {
        base: QName,
        restrictions: Vec<Restriction>,
    },
    /// List of another type
    List { item_type: QName },
    /// Union of multiple types
    Union { member_types: Vec<QName> },
}

/// Restriction facets for simple types
#[derive(Debug, Clone, PartialEq)]
pub enum Restriction {
    MinInclusive(String),
    MaxInclusive(String),
    MinExclusive(String),
    MaxExclusive(String),
    MinLength(u32),
    MaxLength(u32),
    Length(u32),
    Pattern(String),
    Enumeration(String),
    WhiteSpace(WhiteSpace),
    TotalDigits(u32),
    FractionDigits(u32),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WhiteSpace {
    Preserve,
    Replace,
    Collapse,
}

/// A choice between elements (one of many)
#[derive(Debug, Default, Clone)]
pub struct Choice {
    pub elements: Vec<SequenceElement>,
}

/// All elements must appear (unordered)
#[derive(Debug, Default, Clone)]
pub struct All {
    pub elements: Vec<SequenceElement>,
}

/// An XML attribute definition
#[derive(Debug, Default, Clone)]
pub struct Attribute {
    pub name: String,
    pub type_: QName,
    pub use_: AttributeUse,
}

/// Whether an attribute is required or optional
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AttributeUse {
    Required,
    Optional,
    Prohibited,
}

impl Default for AttributeUse {
    fn default() -> Self {
        AttributeUse::Optional
    }
}
