//! WSDL and XSD parsing module
//!
//! This module provides functionality to parse WSDL files and their embedded XSD schemas.

mod wsdl;
mod xsd;

pub use wsdl::parser::parse_wsdl;
pub use wsdl::{
    Binding, BindingOperation, Fault, Message, MessagePart, Port, PortType, PortTypeOperation,
    Service, WsdlModel,
};

pub use xsd::parser::parse_schema;
pub use xsd::{
    Attribute, AttributeUse, ComplexType, Restriction, SchemaElement, Sequence, SequenceElement,
    SimpleType, XmlSchema,
};

/// Qualified Name (QName) representation
///
/// Represents an XML qualified name with optional namespace prefix.
/// Format: `prefix:localName` or just `localName`
#[derive(Default, Debug, Clone, PartialEq, Eq, Hash)]
pub struct QName(pub String);

impl QName {
    /// Create a new QName from a string
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Get the namespace prefix (part before ':')
    pub fn prefix(&self) -> Option<&str> {
        self.0.split_once(':').map(|(prefix, _)| prefix)
    }

    /// Get the local name (part after ':' or the entire string if no ':')
    pub fn local_name(&self) -> &str {
        self.0
            .split_once(':')
            .map(|(_, local)| local)
            .unwrap_or(&self.0)
    }

    /// Get the full qualified name
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Check if this QName is empty
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Split into prefix and local name
    pub fn split(&self) -> (Option<&str>, &str) {
        match self.0.split_once(':') {
            Some((prefix, local)) => (Some(prefix), local),
            None => (None, &self.0),
        }
    }

    /// Create a QName with a specific prefix and local name
    pub fn with_prefix(prefix: &str, local_name: &str) -> Self {
        Self(format!("{}:{}", prefix, local_name))
    }
}

impl From<String> for QName {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for QName {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl std::fmt::Display for QName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_qname_with_prefix() {
        let qname = QName::new("xs:string");
        assert_eq!(qname.prefix(), Some("xs"));
        assert_eq!(qname.local_name(), "string");
    }

    #[test]
    fn test_qname_without_prefix() {
        let qname = QName::new("string");
        assert_eq!(qname.prefix(), None);
        assert_eq!(qname.local_name(), "string");
    }

    #[test]
    fn test_qname_split() {
        let qname = QName::new("tns:MyType");
        let (prefix, local) = qname.split();
        assert_eq!(prefix, Some("tns"));
        assert_eq!(local, "MyType");
    }

    #[test]
    fn test_qname_with_prefix_constructor() {
        let qname = QName::with_prefix("soap", "Envelope");
        assert_eq!(qname.as_str(), "soap:Envelope");
        assert_eq!(qname.prefix(), Some("soap"));
        assert_eq!(qname.local_name(), "Envelope");
    }
}
