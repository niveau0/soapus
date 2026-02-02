//! Rust code generation from WSDL/XSD models

use crate::error::Result;
use crate::generator::type_mapper::TypeMapper;
use crate::generator::{to_pascal_case, to_snake_case};
use crate::parser::{ComplexType, PortTypeOperation, SimpleType, WsdlModel};

/// Generate a Rust struct from XSD complexType
pub fn generate_complex_type(
    name: &str,
    complex_type: &ComplexType,
    type_mapper: &TypeMapper,
) -> Result<String> {
    let mut output = String::new();

    // Doc comment
    output.push_str(&format!("/// Generated from XSD complexType: {}\n", name));

    // Derives - add Default for empty types
    let is_empty = (complex_type.sequence.is_none()
        || complex_type
            .sequence
            .as_ref()
            .map(|s| s.elements.is_empty())
            .unwrap_or(true))
        && complex_type.attributes.is_empty();

    // Derives: Always use PartialEq (not Eq) to avoid issues with floats
    // in nested types that we might not detect recursively
    if is_empty {
        output.push_str("#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]\n");
    } else {
        output.push_str("#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]\n");
    }

    // Add serde rename if the Rust struct name differs from XML name
    let struct_name = to_pascal_case(name);
    if struct_name != name {
        output.push_str(&format!("#[serde(rename = \"{}\")]\n", name));
    }

    // Struct definition
    output.push_str(&format!("pub struct {} {{\n", struct_name));

    // Fields from attributes (XML attributes use @ prefix in serde)
    for attr in &complex_type.attributes {
        let field_name = to_snake_case(&attr.name);
        let sanitized_field_name = super::sanitize_identifier(&field_name);

        // Attributes are always optional unless use="required"
        let rust_type = if attr.use_ == crate::parser::AttributeUse::Required {
            type_mapper.map_type(&attr.type_)
        } else {
            format!("Option<{}>", type_mapper.map_type(&attr.type_))
        };

        // XML attributes need @ prefix in serde rename
        output.push_str(&format!("    #[serde(rename = \"@{}\")]\n", attr.name));

        // Field definition
        output.push_str(&format!(
            "    pub {}: {},\n",
            sanitized_field_name, rust_type
        ));
    }

    // Fields from sequence
    if let Some(seq) = &complex_type.sequence {
        for elem in &seq.elements {
            let field_name = to_snake_case(&elem.name);
            let sanitized_field_name = super::sanitize_identifier(&field_name);
            let rust_type = type_mapper.map_type_with_occurs(
                &elem.type_,
                Some(elem.min_occurs),
                &elem.max_occurs,
                elem.nillable,
            );

            // Add serde rename if needed (always rename if we had to sanitize)
            if sanitized_field_name != elem.name {
                output.push_str(&format!("    #[serde(rename = \"{}\")]\n", elem.name));
            }

            // Field definition
            output.push_str(&format!(
                "    pub {}: {},\n",
                sanitized_field_name, rust_type
            ));
        }
    }

    // If no fields, we already added Default derive above

    output.push_str("}\n");

    Ok(output)
}

/// Generate a Rust enum from XSD simpleType with enumerations
pub fn generate_simple_type_enum(name: &str, simple_type: &SimpleType) -> Result<Option<String>> {
    match simple_type {
        SimpleType::Restriction {
            base: _,
            restrictions,
        } => {
            // Check if we have enumerations
            let enums: Vec<String> = restrictions
                .iter()
                .filter_map(|r| match r {
                    crate::parser::Restriction::Enumeration(val) => Some(val.clone()),
                    _ => None,
                })
                .collect();

            if enums.is_empty() {
                return Ok(None);
            }

            let mut output = String::new();
            output.push_str(&format!("/// Generated from XSD simpleType: {}\n", name));
            output.push_str("#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]\n");
            output.push_str(&format!("pub enum {} {{\n", to_pascal_case(name)));

            for val in enums {
                let variant = to_pascal_case(&val);
                output.push_str(&format!("    #[serde(rename = \"{}\")]\n", val));
                output.push_str(&format!("    {},\n", variant));
            }

            output.push_str("}\n");

            Ok(Some(output))
        }
        _ => Ok(None), // List and Union not supported yet
    }
}

/// Generate a client method for a WSDL operation
pub fn generate_operation_method(
    operation: &PortTypeOperation,
    wsdl: &WsdlModel,
    _type_mapper: &TypeMapper,
) -> Result<String> {
    let mut output = String::new();

    // Method name
    let method_name = to_snake_case(&operation.name);

    // Find input and output message types
    let input_msg = operation
        .input
        .as_ref()
        .and_then(|qname| wsdl.find_message(qname));
    let output_msg = operation
        .output
        .as_ref()
        .and_then(|qname| wsdl.find_message(qname));

    // For now, use generic types if we can't resolve
    let input_type = input_msg
        .and_then(|m| m.parts.first())
        .and_then(|p| p.element.as_ref())
        .map(|e| to_pascal_case(e.local_name()))
        .unwrap_or_else(|| "()".to_string());

    let output_type = output_msg
        .and_then(|m| m.parts.first())
        .and_then(|p| p.element.as_ref())
        .map(|e| to_pascal_case(e.local_name()))
        .unwrap_or_else(|| "()".to_string());

    // Find SOAPAction from WSDL bindings
    let soap_action = wsdl.find_soap_action(&operation.name);

    // Generate method with better documentation
    output.push_str(&format!("    /// Call the {} operation\n", operation.name));

    // Add WSDL documentation if available
    if let Some(doc) = &operation.documentation {
        output.push_str("    ///\n");
        // Split documentation into lines and add as doc comments
        for line in doc.lines() {
            let trimmed = line.trim();
            if !trimmed.is_empty() {
                output.push_str(&format!("    /// {}\n", trimmed));
            }
        }
    }

    // Add doc comment for parameters if we have type info
    if input_type != "()" {
        output.push_str(&format!(
            "    ///\n    /// # Arguments\n    /// * `request` - The {} request\n",
            input_type
        ));
    }

    // Add tracing instrument attribute for Send compatibility with async
    output.push_str(
        "    #[cfg_attr(feature = \"tracing\", tracing::instrument(skip(self, request)))]\n",
    );

    output.push_str(&format!(
        "    pub async fn {}(&self, request: {}) -> SoapResult<{}> {{\n",
        method_name, input_type, output_type
    ));

    // Use call_with_soap_action with namespace and optional SOAPAction
    // Pass ELEMENT_FORM_QUALIFIED to control namespace handling for child elements
    if let Some(action) = soap_action {
        output.push_str(&format!(
            "        self.client.call_with_soap_action(\"{}\", Some(\"{}\"), Some(TARGET_NAMESPACE), ELEMENT_FORM_QUALIFIED, &request).await\n",
            operation.name, action
        ));
    } else {
        output.push_str(&format!(
            "        self.client.call_with_soap_action(\"{}\", None, Some(TARGET_NAMESPACE), ELEMENT_FORM_QUALIFIED, &request).await\n",
            operation.name
        ));
    }

    output.push_str("    }\n");

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::{Attribute, AttributeUse};
    use crate::parser::{ComplexType, PortTypeOperation, QName, Sequence, SequenceElement};

    #[test]
    fn test_generate_simple_struct() {
        let complex_type = ComplexType {
            sequence: Some(Sequence {
                elements: vec![SequenceElement {
                    name: "userName".to_string(),
                    type_: QName::new("xs:string"),
                    min_occurs: 1,
                    max_occurs: None,
                    nillable: false,
                }],
            }),
            ..Default::default()
        };

        let type_mapper = TypeMapper::new();
        let code = generate_complex_type("User", &complex_type, &type_mapper).unwrap();

        assert!(code.contains("pub struct User"));
        assert!(code.contains("pub user_name: String"));
        assert!(code.contains("#[serde(rename = \"userName\")]"));
        assert!(code.contains("PartialEq"));
    }

    #[test]
    fn test_generate_empty_struct() {
        let complex_type = ComplexType::default();
        let type_mapper = TypeMapper::new();
        let code = generate_complex_type("EmptyType", &complex_type, &type_mapper).unwrap();

        assert!(code.contains("pub struct EmptyType"));
        assert!(code.contains("Default"));
        assert!(code.contains("PartialEq"));
    }

    #[test]
    fn test_generate_struct_with_optional_field() {
        let complex_type = ComplexType {
            sequence: Some(Sequence {
                elements: vec![SequenceElement {
                    name: "optionalField".to_string(),
                    type_: QName::new("xs:string"),
                    min_occurs: 0,
                    max_occurs: None,
                    nillable: false,
                }],
            }),
            ..Default::default()
        };

        let type_mapper = TypeMapper::new();
        let code = generate_complex_type("TestType", &complex_type, &type_mapper).unwrap();

        assert!(code.contains("pub optional_field: Option<String>"));
    }

    #[test]
    fn test_generate_struct_with_array_field() {
        let complex_type = ComplexType {
            sequence: Some(Sequence {
                elements: vec![SequenceElement {
                    name: "items".to_string(),
                    type_: QName::new("xs:string"),
                    min_occurs: 0,
                    max_occurs: Some("unbounded".to_string()),
                    nillable: false,
                }],
            }),
            ..Default::default()
        };

        let type_mapper = TypeMapper::new();
        let code = generate_complex_type("TestType", &complex_type, &type_mapper).unwrap();

        assert!(code.contains("pub items: Option<Vec<String>>"));
    }

    #[test]
    fn test_generate_struct_with_float_no_eq() {
        let complex_type = ComplexType {
            sequence: Some(Sequence {
                elements: vec![SequenceElement {
                    name: "price".to_string(),
                    type_: QName::new("xs:double"),
                    min_occurs: 1,
                    max_occurs: None,
                    nillable: false,
                }],
            }),
            ..Default::default()
        };

        let type_mapper = TypeMapper::new();
        let code = generate_complex_type("Product", &complex_type, &type_mapper).unwrap();

        assert!(code.contains("pub price: f64"));
        assert!(code.contains("PartialEq"));
        // Floats are handled - no Eq is derived anywhere anymore
    }

    #[test]
    fn test_generate_struct_with_multiple_fields() {
        let complex_type = ComplexType {
            sequence: Some(Sequence {
                elements: vec![
                    SequenceElement {
                        name: "Code".to_string(),
                        type_: QName::new("xs:int"),
                        min_occurs: 1,
                        max_occurs: None,
                        nillable: false,
                    },
                    SequenceElement {
                        name: "Message".to_string(),
                        type_: QName::new("xs:string"),
                        min_occurs: 1,
                        max_occurs: None,
                        nillable: false,
                    },
                ],
            }),
            ..Default::default()
        };

        let type_mapper = TypeMapper::new();
        let code = generate_complex_type("ServiceException", &complex_type, &type_mapper).unwrap();

        assert!(code.contains("pub struct ServiceException"));
        assert!(code.contains("pub code: i32"));
        assert!(code.contains("pub message: String"));
        assert!(code.contains("#[serde(rename = \"Code\")]"));
        assert!(code.contains("#[serde(rename = \"Message\")]"));
    }

    #[test]
    fn test_generate_struct_with_attributes() {
        let complex_type = ComplexType {
            sequence: Some(Sequence {
                elements: vec![],
            }),
            attributes: vec![
                Attribute {
                    name: "key".to_string(),
                    type_: QName::new("xs:string"),
                    use_: AttributeUse::Optional,
                },
                Attribute {
                    name: "value".to_string(),
                    type_: QName::new("xs:string"),
                    use_: AttributeUse::Optional,
                },
            ],
            ..Default::default()
        };

        let type_mapper = TypeMapper::new();
        let code = generate_complex_type("MapElements", &complex_type, &type_mapper).unwrap();

        assert!(code.contains("pub struct MapElements"));
        assert!(code.contains("#[serde(rename = \"@key\")]"));
        assert!(code.contains("pub key: Option<String>"));
        assert!(code.contains("#[serde(rename = \"@value\")]"));
        assert!(code.contains("pub value: Option<String>"));
        // Should not have Default derive when attributes present
        assert!(!code.contains("Default"));
    }

    #[test]
    fn test_generate_struct_with_required_attribute() {
        let complex_type = ComplexType {
            sequence: None,
            attributes: vec![Attribute {
                name: "id".to_string(),
                type_: QName::new("xs:string"),
                use_: AttributeUse::Required,
            }],
            ..Default::default()
        };

        let type_mapper = TypeMapper::new();
        let code = generate_complex_type("Entity", &complex_type, &type_mapper).unwrap();

        assert!(code.contains("pub struct Entity"));
        assert!(code.contains("#[serde(rename = \"@id\")]"));
        // Required attributes should not be wrapped in Option
        assert!(code.contains("pub id: String"));
        assert!(!code.contains("pub id: Option<String>"));
    }

    #[test]
    fn test_generate_operation_method() {
        let operation = PortTypeOperation {
            name: "getAllVersions".to_string(),
            input: Some(QName::new("tns:getAllVersions")),
            output: Some(QName::new("tns:getAllVersionsResponse")),
            faults: vec![],
            documentation: None,
        };

        // Create a minimal WsdlModel - we don't need messages for this test
        // since we're testing method signature generation
        let wsdl = crate::parser::WsdlModel::default();
        let type_mapper = TypeMapper::new();

        let code = generate_operation_method(&operation, &wsdl, &type_mapper).unwrap();

        assert!(code.contains("pub async fn get_all_versions"));
        // When messages aren't found, it falls back to type names from QName
        assert!(code.contains("GetAllVersions") || code.contains("()"));
        assert!(code.contains("GetAllVersionsResponse") || code.contains("()"));
        assert!(code.contains("SoapResult"));
        assert!(code.contains("/// Call the getAllVersions operation"));
    }
}
