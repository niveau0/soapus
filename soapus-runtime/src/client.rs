//! SOAP HTTP client implementation
//!
//! This module provides the main `SoapClient` for making SOAP requests over HTTP.
//! It handles envelope construction, HTTP communication, and response parsing.

use crate::envelope::{SoapEnvelope, SoapVersion};
use crate::error::{SoapError, SoapResult};
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[cfg(feature = "tracing")]
use tracing::{debug, info, instrument, warn};

/// SOAP client for making HTTP requests
///
/// This client handles the complete SOAP request/response cycle:
/// 1. Serialize request body to XML
/// 2. Wrap in SOAP envelope
/// 3. Send HTTP POST request
/// 4. Parse response envelope
/// 5. Check for SOAP faults
/// 6. Deserialize response body
///
/// # Example
///
/// ```no_run
/// use soapus_runtime::SoapClient;
/// use serde::{Serialize, Deserialize};
///
/// #[derive(Serialize)]
/// struct MyRequest {
///     name: String,
/// }
///
/// #[derive(Deserialize)]
/// struct MyResponse {
///     result: String,
/// }
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let client = SoapClient::new("http://example.com/soap");
/// let request = MyRequest { name: "test".to_string() };
/// let response: MyResponse = client.call("MyOperation", &request).await?;
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct SoapClient {
    /// The SOAP endpoint URL
    endpoint: String,
    /// HTTP client for making requests
    http_client: Client,
    /// SOAP protocol version to use
    soap_version: SoapVersion,
    /// SOAPAction header value (optional)
    soap_action: Option<String>,
    /// Request timeout
    timeout: Duration,
}

impl SoapClient {
    /// Create a new SOAP client with default settings
    ///
    /// Uses SOAP 1.1 by default with a 30-second timeout.
    ///
    /// # Arguments
    ///
    /// * `endpoint` - The SOAP service endpoint URL
    pub fn new(endpoint: impl Into<String>) -> Self {
        Self {
            endpoint: endpoint.into(),
            http_client: Client::new(),
            soap_version: SoapVersion::Soap11,
            soap_action: None,
            timeout: Duration::from_secs(30),
        }
    }

    /// Create a new SOAP client builder for advanced configuration
    pub fn builder(endpoint: impl Into<String>) -> SoapClientBuilder {
        SoapClientBuilder::new(endpoint)
    }

    /// Get the endpoint URL
    pub fn endpoint(&self) -> &str {
        &self.endpoint
    }

    /// Set the SOAP version to use
    pub fn set_soap_version(&mut self, version: SoapVersion) {
        self.soap_version = version;
    }

    /// Get the current SOAP version
    pub fn soap_version(&self) -> SoapVersion {
        self.soap_version
    }

    /// Set the SOAPAction header value
    ///
    /// This is required for SOAP 1.1 operations. For SOAP 1.2, it's optional.
    pub fn set_soap_action(&mut self, action: impl Into<String>) {
        self.soap_action = Some(action.into());
    }

    /// Set the request timeout
    pub fn set_timeout(&mut self, timeout: Duration) {
        self.timeout = timeout;
    }

    /// Make a SOAP call
    ///
    /// This method performs the complete SOAP request/response cycle:
    /// 1. Serializes the request to XML and wraps it in a SOAP envelope
    /// 2. Sends an HTTP POST request to the endpoint
    /// 3. Checks for HTTP errors and SOAP faults
    /// 4. Parses and deserializes the response
    ///
    /// # Type Parameters
    ///
    /// * `Req` - The request type (must implement `Serialize`)
    /// * `Resp` - The response type (must implement `Deserialize`)
    ///
    /// # Arguments
    ///
    /// * `operation` - The SOAP operation name (used for SOAPAction header)
    /// * `request` - The request body to send
    ///
    /// # Returns
    ///
    /// The deserialized response or a `SoapError`
    #[cfg_attr(feature = "tracing", instrument(skip(self, request), fields(endpoint = %self.endpoint, soap_version = ?self.soap_version)))]
    pub async fn call<Req, Resp>(&self, operation: &str, request: &Req) -> SoapResult<Resp>
    where
        Req: Serialize,
        Resp: for<'de> Deserialize<'de>,
    {
        #[cfg(feature = "tracing")]
        info!(operation = %operation, "Initiating SOAP call");

        #[cfg(feature = "metrics")]
        let start = std::time::Instant::now();

        let result = self
            .call_with_soap_action(operation, None, None, true, request)
            .await;

        #[cfg(feature = "metrics")]
        {
            let duration = start.elapsed();
            metrics::histogram!("soap_request_duration_seconds", duration.as_secs_f64());

            metrics::increment_counter!("soap_requests_total");

            if result.is_err() {
                metrics::increment_counter!("soap_errors_total");
            }
        }

        #[cfg(feature = "tracing")]
        match &result {
            Ok(_) => info!(operation = %operation, "SOAP call completed successfully"),
            Err(e) => warn!(operation = %operation, error = %e, "SOAP call failed"),
        }

        result
    }

    /// Call a SOAP operation with explicit SOAPAction header
    ///
    /// This method is similar to `call` but allows specifying a custom SOAPAction
    /// header value, which is often required for .NET and other SOAP services.
    ///
    /// # Type Parameters
    ///
    /// * `Req` - The request type (must implement `Serialize`)
    /// * `Resp` - The response type (must implement `Deserialize`)
    ///
    /// # Arguments
    ///
    /// * `operation` - The SOAP operation name
    /// * `soap_action` - The SOAPAction header value (if None, uses operation name)
    /// * `namespace` - The XML namespace for the request body element (if None, no namespace is added)
    /// * `request` - The request body to send
    ///
    /// # Returns
    ///
    /// The deserialized response or a `SoapError`
    #[cfg_attr(feature = "tracing", instrument(skip(self, request), fields(endpoint = %self.endpoint, soap_version = ?self.soap_version)))]
    pub async fn call_with_soap_action<Req, Resp>(
        &self,
        operation: &str,
        soap_action: Option<&str>,
        namespace: Option<&str>,
        element_form_qualified: bool,
        request: &Req,
    ) -> SoapResult<Resp>
    where
        Req: Serialize,
        Resp: for<'de> Deserialize<'de>,
    {
        #[cfg(feature = "tracing")]
        debug!(operation = %operation, soap_action = ?soap_action, namespace = ?namespace, element_form_qualified = %element_form_qualified, "Building SOAP envelope");

        // Build SOAP envelope with namespace if provided
        let envelope = SoapEnvelope::build_with_namespace(
            request,
            self.soap_version,
            namespace,
            element_form_qualified,
        )?;

        #[cfg(feature = "tracing")]
        debug!(envelope_size = envelope.len(), "SOAP envelope built");

        // Prepare HTTP request
        let mut http_request = self
            .http_client
            .post(&self.endpoint)
            .timeout(self.timeout)
            .body(envelope);

        // Set Content-Type based on SOAP version
        http_request = match self.soap_version {
            SoapVersion::Soap11 => http_request.header("Content-Type", "text/xml; charset=utf-8"),
            SoapVersion::Soap12 => {
                http_request.header("Content-Type", "application/soap+xml; charset=utf-8")
            }
        };

        // Set SOAPAction header for SOAP 1.1
        if self.soap_version == SoapVersion::Soap11 {
            let action = soap_action
                .or(self.soap_action.as_deref())
                .unwrap_or(operation);
            http_request = http_request.header("SOAPAction", format!("\"{}\"", action));
        }

        // Send request
        #[cfg(feature = "tracing")]
        debug!(endpoint = %self.endpoint, "Sending HTTP POST request");

        let response = match http_request.send().await {
            Ok(resp) => resp,
            Err(e) => {
                #[cfg(feature = "tracing")]
                warn!(endpoint = %self.endpoint, error = %e, "HTTP request failed");
                return Err(e.into());
            }
        };

        // Check HTTP status
        let status = response.status();

        #[cfg(feature = "tracing")]
        debug!(status = %status, "Received HTTP response");
        if !status.is_success() && status != StatusCode::INTERNAL_SERVER_ERROR {
            // SOAP faults can come with 500 status, so we allow that
            return Err(SoapError::HttpError(
                response.error_for_status().unwrap_err(),
            ));
        }

        // Get response body
        let response_text = response.text().await?;

        #[cfg(feature = "tracing")]
        debug!(
            response_size = response_text.len(),
            "Received response body"
        );

        #[cfg(feature = "metrics")]
        metrics::histogram!("soap_response_size_bytes", response_text.len() as f64);

        // Check for SOAP faults
        if let Err(e) = SoapEnvelope::check_for_fault(&response_text) {
            #[cfg(feature = "tracing")]
            warn!(error = %e, "SOAP fault detected in response");
            return Err(e);
        }

        // Parse response
        #[cfg(feature = "tracing")]
        debug!("Parsing SOAP response");

        let parsed_response = SoapEnvelope::parse_response(&response_text)?;

        #[cfg(feature = "tracing")]
        debug!("SOAP response parsed successfully");

        Ok(parsed_response)
    }

    /// Make a SOAP call without deserializing the response
    ///
    /// This is useful for debugging or when you want to handle the raw XML response yourself.
    ///
    /// # Arguments
    ///
    /// * `operation` - The SOAP operation name
    /// * `request` - The request body to send
    ///
    /// # Returns
    ///
    /// The raw XML response as a string
    #[cfg_attr(feature = "tracing", instrument(skip(self, request), fields(endpoint = %self.endpoint)))]
    pub async fn call_raw<Req>(&self, operation: &str, request: &Req) -> SoapResult<String>
    where
        Req: Serialize,
    {
        #[cfg(feature = "tracing")]
        debug!(operation = %operation, "Building SOAP envelope for raw call");

        // Build SOAP envelope
        let envelope = SoapEnvelope::build(request, self.soap_version)?;

        // Prepare HTTP request
        let mut http_request = self
            .http_client
            .post(&self.endpoint)
            .timeout(self.timeout)
            .body(envelope);

        // Set Content-Type based on SOAP version
        http_request = match self.soap_version {
            SoapVersion::Soap11 => http_request.header("Content-Type", "text/xml; charset=utf-8"),
            SoapVersion::Soap12 => {
                http_request.header("Content-Type", "application/soap+xml; charset=utf-8")
            }
        };

        // Set SOAPAction header for SOAP 1.1
        if self.soap_version == SoapVersion::Soap11 {
            let soap_action = self.soap_action.as_deref().unwrap_or(operation);
            http_request = http_request.header("SOAPAction", format!("\"{}\"", soap_action));
        }

        // Send request
        #[cfg(feature = "tracing")]
        debug!(endpoint = %self.endpoint, "Sending HTTP POST request (raw call)");

        let response = match http_request.send().await {
            Ok(resp) => resp,
            Err(e) => {
                #[cfg(feature = "tracing")]
                warn!(endpoint = %self.endpoint, error = %e, "HTTP request failed (raw call)");
                return Err(e.into());
            }
        };

        // Check HTTP status
        let status = response.status();

        #[cfg(feature = "tracing")]
        debug!(status = %status, "Received HTTP response (raw call)");
        if !status.is_success() && status != StatusCode::INTERNAL_SERVER_ERROR {
            return Err(SoapError::HttpError(
                response.error_for_status().unwrap_err(),
            ));
        }

        // Get response body
        let response_text = response.text().await?;

        #[cfg(feature = "tracing")]
        debug!(
            response_size = response_text.len(),
            "Received response body (raw call)"
        );

        // Check for SOAP faults
        if let Err(e) = SoapEnvelope::check_for_fault(&response_text) {
            #[cfg(feature = "tracing")]
            warn!(error = %e, "SOAP fault detected in raw response");
            return Err(e);
        }

        Ok(response_text)
    }
}

/// Builder for configuring a SOAP client
///
/// Provides a fluent interface for setting up a SOAP client with custom settings.
///
/// # Example
///
/// ```no_run
/// use soapus_runtime::{SoapClient, SoapVersion};
/// use std::time::Duration;
///
/// let client = SoapClient::builder("http://example.com/soap")
///     .soap_version(SoapVersion::Soap12)
///     .timeout(Duration::from_secs(60))
///     .soap_action("http://example.com/MyOperation")
///     .build();
/// ```
pub struct SoapClientBuilder {
    endpoint: String,
    soap_version: SoapVersion,
    soap_action: Option<String>,
    timeout: Duration,
    http_client: Option<Client>,
}

impl SoapClientBuilder {
    /// Create a new builder with the given endpoint
    pub fn new(endpoint: impl Into<String>) -> Self {
        Self {
            endpoint: endpoint.into(),
            soap_version: SoapVersion::Soap11,
            soap_action: None,
            timeout: Duration::from_secs(30),
            http_client: None,
        }
    }

    /// Set the SOAP protocol version
    pub fn soap_version(mut self, version: SoapVersion) -> Self {
        self.soap_version = version;
        self
    }

    /// Set the SOAPAction header value
    pub fn soap_action(mut self, action: impl Into<String>) -> Self {
        self.soap_action = Some(action.into());
        self
    }

    /// Set the request timeout
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Set a custom HTTP client
    ///
    /// This allows you to configure the underlying reqwest client with custom settings
    /// such as proxies, authentication, or custom certificates.
    pub fn http_client(mut self, client: Client) -> Self {
        self.http_client = Some(client);
        self
    }

    /// Build the SOAP client
    pub fn build(self) -> SoapClient {
        SoapClient {
            endpoint: self.endpoint,
            http_client: self.http_client.unwrap_or_default(),
            soap_version: self.soap_version,
            soap_action: self.soap_action,
            timeout: self.timeout,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = SoapClient::new("http://example.com/soap");
        assert_eq!(client.endpoint(), "http://example.com/soap");
        assert_eq!(client.soap_version(), SoapVersion::Soap11);
    }

    #[test]
    fn test_client_builder() {
        let client = SoapClient::builder("http://example.com/soap")
            .soap_version(SoapVersion::Soap12)
            .soap_action("http://example.com/MyAction")
            .timeout(Duration::from_secs(60))
            .build();

        assert_eq!(client.endpoint(), "http://example.com/soap");
        assert_eq!(client.soap_version(), SoapVersion::Soap12);
    }

    #[test]
    fn test_set_soap_version() {
        let mut client = SoapClient::new("http://example.com/soap");
        assert_eq!(client.soap_version(), SoapVersion::Soap11);

        client.set_soap_version(SoapVersion::Soap12);
        assert_eq!(client.soap_version(), SoapVersion::Soap12);
    }

    #[test]
    fn test_set_soap_action() {
        let mut client = SoapClient::new("http://example.com/soap");
        client.set_soap_action("http://example.com/MyAction");
        assert_eq!(
            client.soap_action,
            Some("http://example.com/MyAction".to_string())
        );
    }

    #[test]
    fn test_set_timeout() {
        let mut client = SoapClient::new("http://example.com/soap");
        client.set_timeout(Duration::from_secs(120));
        assert_eq!(client.timeout, Duration::from_secs(120));
    }
}
