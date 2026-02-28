//! Docker client wrapper using bollard.

use bollard::Docker;

use crate::error::WorkerError;

/// Create a Docker client from the configured host.
pub fn connect(docker_host: &str) -> Result<Docker, WorkerError> {
    if docker_host.is_empty() {
        return Ok(Docker::connect_with_socket_defaults()?);
    }
    Ok(Docker::connect_with_socket(
        docker_host,
        120,
        bollard::API_DEFAULT_VERSION,
    )?)
}
