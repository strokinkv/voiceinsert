use crate::api::endpoints::endpoint;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct ModelInfo {
    pub id: String,
}

#[derive(Debug, Deserialize)]
struct ModelsResponse {
    data: Vec<ModelInfo>,
}

pub async fn load_models(
    http: &reqwest::Client,
    base_url: &str,
    api_key: &str,
) -> anyhow::Result<Vec<ModelInfo>> {
    let mut request = http.get(endpoint(base_url, "/v1/models")?);
    if !api_key.trim().is_empty() {
        request = request.bearer_auth(api_key);
    }

    let response = request.send().await?;
    let status = response.status();
    if !status.is_success() {
        anyhow::bail!("Models API request failed: {}", status.as_u16());
    }

    Ok(response.json::<ModelsResponse>().await?.data)
}
