use crate::api::models::load_models;
use crate::settings::ApiProfile;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HealthCheckResult {
    pub is_healthy: bool,
    pub message: String,
}

impl HealthCheckResult {
    pub fn healthy(model_count: usize) -> Self {
        Self {
            is_healthy: true,
            message: format!("Connected. Models: {model_count}."),
        }
    }

    pub fn unhealthy(message: impl Into<String>) -> Self {
        Self {
            is_healthy: false,
            message: message.into(),
        }
    }
}

pub async fn check_profile(
    http: &reqwest::Client,
    profile: &ApiProfile,
    api_key: &str,
) -> HealthCheckResult {
    match load_models(http, &profile.base_url, api_key).await {
        Ok(models) => HealthCheckResult::healthy(models.len()),
        Err(error) => HealthCheckResult::unhealthy(format!("API health check failed: {error}")),
    }
}

#[cfg(test)]
mod tests {
    use super::HealthCheckResult;

    #[test]
    fn healthy_message_includes_model_count() {
        let result = HealthCheckResult::healthy(3);

        assert!(result.is_healthy);
        assert!(result.message.contains("3"));
    }
}
