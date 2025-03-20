// Export submodules
pub mod site;
pub mod voucher;

/// Common trait for API endpoints.
///
/// This trait is implemented by all API endpoints and provides a method to get the client associated with the endpoint.
#[allow(dead_code)]
pub(crate) trait ApiEndpoint {
    /// Get the client associated with this endpoint.
    fn client(&self) -> &crate::UnifiClient;
}
