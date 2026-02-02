//! Parsing of XSD complexType definitions

use crate::parser::xsd::{Attribute, AttributeUse, ComplexType, Sequence};
use crate::parser::QName;
use quick_xml::events::{BytesStart, Event};
use std::error::Error;

use super::parser::SchemaParser;

impl<B: std::io::BufRead> SchemaParser<B> {
    /// Parse a <complexType> definition
    ///
    /// ComplexTypes define structured types with child elements.
    /// They can contain:
    /// - <sequence> - Ordered sequence of elements
    /// - <all> - Unordered collection of elements
    /// - <choice> - Alternative elements (not yet fully supported)
    ///
    /// Example:
    /// ```xml
    /// <complexType name="Person">
    ///   <sequence>
    ///     <element name="firstName" type="xs:string"/>
    ///     <element name="lastName" type="xs:string"/>
    ///   </sequence>
    /// </complexType>
    /// ```
    pub(super) fn parse_complex_type(&mut self, e: &BytesStart) -> Result<(), Box<dyn Error>> {
        let name = e
            .try_get_attribute("name")?
            .map(|a| a.unescape_value().unwrap().into_owned());
        let mut complex_type = ComplexType::default();
        if let Some(n) = name {
            complex_type.name = n;
        }

        let mut buf = Vec::new();
        loop {
            match self.reader.read_event_into(&mut buf)? {
                Event::Start(e) if e.local_name().as_ref() == b"sequence" => {
                    complex_type.sequence = Some(self.parse_sequence()?);
                }
                Event::Empty(e) if e.local_name().as_ref() == b"sequence" => {
                    // Empty sequence like <xs:sequence/>
                    complex_type.sequence = Some(Sequence::default());
                }
                Event::Start(e) if e.local_name().as_ref() == b"all" => {
                    // <xs:all> - treat as sequence for now
                    complex_type.sequence = Some(self.parse_all()?);
                }
                Event::Empty(e) if e.local_name().as_ref() == b"all" => {
                    // Empty all like <xs:all/>
                    complex_type.sequence = Some(Sequence::default());
                }
                Event::Empty(e) if e.local_name().as_ref() == b"attribute" => {
                    // Parse attribute like <xs:attribute name="key" type="xs:string" use="optional"/>
                    if let Some(attr) = self.parse_attribute(&e)? {
                        complex_type.attributes.push(attr);
                    }
                }
                Event::Start(e) if e.local_name().as_ref() == b"attribute" => {
                    // Parse attribute with nested content (rare, but possible)
                    if let Some(attr) = self.parse_attribute(&e)? {
                        complex_type.attributes.push(attr);
                    }
                    // Skip to end of attribute element manually
                    let mut depth = 1;
                    let mut skip_buf = Vec::new();
                    loop {
                        match self.reader.read_event_into(&mut skip_buf)? {
                            Event::Start(ref e) if e.local_name().as_ref() == b"attribute" => {
                                depth += 1;
                            }
                            Event::End(ref e) if e.local_name().as_ref() == b"attribute" => {
                                depth -= 1;
                                if depth == 0 {
                                    break;
                                }
                            }
                            Event::Eof => break,
                            _ => {}
                        }
                        skip_buf.clear();
                    }
                }
                Event::End(e) if e.local_name().as_ref() == b"complexType" => break,
                Event::Eof => break,
                _ => {}
            }
            buf.clear();
        }

        if !complex_type.name.is_empty() {
            self.model
                .complex_types
                .insert(complex_type.name.clone(), complex_type);
        }
        Ok(())
    }

    /// Parse an <attribute> definition
    ///
    /// Attributes define XML attributes on elements.
    /// They can be required or optional.
    ///
    /// Example:
    /// ```xml
    /// <attribute name="id" type="xs:string" use="required"/>
    /// <attribute name="version" type="xs:string"/>
    /// ```
    fn parse_attribute(&self, e: &BytesStart) -> Result<Option<Attribute>, Box<dyn Error>> {
        let name = e
            .try_get_attribute("name")?
            .map(|a| a.unescape_value().unwrap().into_owned());

        let type_ = e
            .try_get_attribute("type")?
            .map(|a| QName::new(a.unescape_value().unwrap().as_ref()));

        let use_attr = e
            .try_get_attribute("use")?
            .map(|a| a.unescape_value().unwrap().into_owned());

        if let (Some(name), Some(type_)) = (name, type_) {
            let use_ = match use_attr.as_deref() {
                Some("required") => AttributeUse::Required,
                Some("prohibited") => AttributeUse::Prohibited,
                _ => AttributeUse::Optional, // default is optional
            };

            Ok(Some(Attribute { name, type_, use_ }))
        } else {
            Ok(None)
        }
    }
}
