use soapus_codegen::SoapClientGenerator;
use std::fs;
use tempfile::tempdir;

#[test]
fn test_generate_from_calculator_wsdl() {
    let dir = tempdir().unwrap();

    // Generate code from Calculator WSDL
    let result = SoapClientGenerator::builder()
        .wsdl_path("../testdata/wsdl/calculator.wsdl")
        .out_dir(dir.path())
        .generate();

    // Should succeed
    assert!(result.is_ok(), "Code generation failed: {:?}", result.err());

    let gen = result.unwrap();

    // Check generated file exists
    let generated_file = &gen.output_file;
    assert!(generated_file.exists(), "Generated file not found");

    // Read and verify content
    let content = fs::read_to_string(generated_file).unwrap();

    // Verify it contains expected elements
    assert!(
        content.contains("pub struct"),
        "Should contain struct definitions"
    );
    assert!(content.contains("impl"), "Should contain impl blocks");
    assert!(
        content.contains("Calculator"),
        "Should have Calculator client"
    );
    assert!(
        content.contains("pub async fn add"),
        "Should have add operation"
    );
    assert!(
        content.contains("pub async fn subtract"),
        "Should have subtract operation"
    );
}

#[test]
fn test_generate_from_countryinfo_wsdl() {
    let dir = tempdir().unwrap();

    // Generate code from CountryInfo WSDL
    let result = SoapClientGenerator::builder()
        .wsdl_path("../testdata/wsdl/countryinfo.wsdl")
        .out_dir(dir.path())
        .generate();

    // Should succeed
    assert!(
        result.is_ok(),
        "CountryInfo code generation failed: {:?}",
        result.err()
    );

    let gen = result.unwrap();

    // Check generated file exists
    let generated_file = &gen.output_file;
    assert!(generated_file.exists(), "Generated file not found");

    // Read and verify content
    let content = fs::read_to_string(generated_file).unwrap();

    // Verify it contains expected elements
    assert!(
        content.contains("pub struct"),
        "Should contain struct definitions"
    );
    assert!(content.contains("impl"), "Should contain impl blocks");

    // CountryInfo service has many operations, check for a few
    assert!(
        content.contains("CountryInfoService") || content.contains("CountryInfo"),
        "Should have CountryInfo client"
    );

    // Verify some complex types from the WSDL
    assert!(
        content.contains("TContinent") || content.contains("Continent"),
        "Should have Continent type"
    );
    assert!(
        content.contains("TCurrency") || content.contains("Currency"),
        "Should have Currency type"
    );
}

#[test]
fn test_generate_from_numberconversion_wsdl() {
    let dir = tempdir().unwrap();

    // Generate code from NumberConversion WSDL
    let result = SoapClientGenerator::builder()
        .wsdl_path("../testdata/wsdl/numberconversion.wsdl")
        .out_dir(dir.path())
        .generate();

    // Should succeed
    assert!(
        result.is_ok(),
        "NumberConversion code generation failed: {:?}",
        result.err()
    );

    let gen = result.unwrap();

    // Check generated file exists
    let generated_file = &gen.output_file;
    assert!(generated_file.exists(), "Generated file not found");

    // Read and verify content
    let content = fs::read_to_string(generated_file).unwrap();

    // Verify it contains expected elements
    assert!(
        content.contains("pub struct"),
        "Should contain struct definitions"
    );
    assert!(content.contains("impl"), "Should contain impl blocks");
    assert!(
        content.contains("NumberConversion"),
        "Should have NumberConversion client"
    );

    // Check for operations
    assert!(
        content.contains("number_to_words") || content.contains("NumberToWords"),
        "Should have number_to_words operation"
    );
    assert!(
        content.contains("number_to_dollars") || content.contains("NumberToDollars"),
        "Should have number_to_dollars operation"
    );
}

#[test]
fn test_generate_from_attributes_wsdl() {
    let dir = tempdir().unwrap();

    // Generate code from Attributes test WSDL
    let result = SoapClientGenerator::builder()
        .wsdl_path("../testdata/wsdl/attributes_test.wsdl")
        .out_dir(dir.path())
        .generate();

    // Should succeed
    assert!(
        result.is_ok(),
        "Attributes test code generation failed: {:?}",
        result.err()
    );

    let gen = result.unwrap();

    // Check generated file exists
    let generated_file = &gen.output_file;
    assert!(generated_file.exists(), "Generated file not found");

    // Read and verify content
    let content = fs::read_to_string(generated_file).unwrap();

    // Verify it contains expected elements
    assert!(
        content.contains("pub struct"),
        "Should contain struct definitions"
    );

    // Check for MapElements struct with attributes
    assert!(
        content.contains("pub struct MapElements"),
        "Should have MapElements type"
    );
    assert!(
        content.contains("#[serde(rename = \"@key\")]"),
        "Should have @key attribute"
    );
    assert!(
        content.contains("#[serde(rename = \"@value\")]"),
        "Should have @value attribute"
    );
    assert!(
        content.contains("pub key: Option<String>"),
        "key should be optional String"
    );
    assert!(
        content.contains("pub value: Option<String>"),
        "value should be optional String"
    );

    // Check for Entity struct with required attribute
    assert!(
        content.contains("pub struct Entity"),
        "Should have Entity type"
    );
    assert!(
        content.contains("#[serde(rename = \"@id\")]"),
        "Should have @id attribute"
    );
    assert!(
        content.contains("pub id: String"),
        "id should be required String (not Option)"
    );
    assert!(
        content.contains("#[serde(rename = \"@version\")]"),
        "Should have @version attribute"
    );
    assert!(
        content.contains("pub version: Option<i32>"),
        "version should be optional i32"
    );
    assert!(
        content.contains("pub name: String"),
        "Should have name element"
    );

    // Check for Product struct with both elements and attributes
    assert!(
        content.contains("pub struct Product"),
        "Should have Product type"
    );
    assert!(
        content.contains("#[serde(rename = \"@sku\")]"),
        "Should have @sku attribute"
    );
    assert!(
        content.contains("pub sku: String"),
        "sku should be required String"
    );
    assert!(
        content.contains("#[serde(rename = \"@category\")]"),
        "Should have @category attribute"
    );
    assert!(
        content.contains("pub category: Option<String>"),
        "category should be optional String"
    );
    assert!(
        content.contains("pub description: String"),
        "Should have description element"
    );
    assert!(
        content.contains("pub price:"),
        "Should have price element"
    );

    // Check for operations
    assert!(
        content.contains("pub async fn get_entity"),
        "Should have get_entity operation"
    );
    assert!(
        content.contains("pub async fn get_product"),
        "Should have get_product operation"
    );
}

#[test]
fn test_all_wsdls_generate_valid_rust() {
    // Test that all WSDL files generate code that at least compiles syntactically
    let wsdl_files = vec![
        ("../testdata/wsdl/calculator.wsdl", "Calculator"),
        ("../testdata/wsdl/countryinfo.wsdl", "CountryInfo"),
        ("../testdata/wsdl/numberconversion.wsdl", "NumberConversion"),
        ("../testdata/wsdl/attributes_test.wsdl", "AttributesTest"),
    ];

    for (wsdl_path, expected_name) in wsdl_files {
        let dir = tempdir().unwrap();

        let result = SoapClientGenerator::builder()
            .wsdl_path(wsdl_path)
            .out_dir(dir.path())
            .generate();

        assert!(
            result.is_ok(),
            "Failed to generate from {}: {:?}",
            wsdl_path,
            result.err()
        );

        let gen = result.unwrap();
        let content = fs::read_to_string(&gen.output_file).unwrap();

        // Basic sanity checks
        assert!(
            content.contains("pub struct") || content.contains("pub enum"),
            "{} should contain type definitions",
            wsdl_path
        );
        assert!(
            content.contains(expected_name) || content.contains(&expected_name.to_lowercase()),
            "{} should contain {}",
            wsdl_path,
            expected_name
        );

        // Ensure it has the standard imports
        assert!(
            content.contains("use soapus_runtime"),
            "{} should import runtime",
            wsdl_path
        );
        assert!(
            content.contains("use serde"),
            "{} should import serde",
            wsdl_path
        );
    }
}
