use oauth2::basic::BasicTokenType;
use oauth2::reqwest::async_http_client;
pub use oauth2::{ClientId, TokenUrl};
use oauth2::{EmptyExtraTokenFields, Scope, StandardTokenResponse, TokenResponse as _};
use serde::{Deserialize, Serialize};

pub type TokenResponse = StandardTokenResponse<EmptyExtraTokenFields, BasicTokenType>;

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct AuthUser {
    pub id: String,
    pub email: String,
    pub name: Option<String>,
    pub image: Option<String>,
    #[serde(rename = "emailVerified")]
    pub email_verified: bool,
    #[serde(rename = "createdAt")]
    pub created_at: String,
    #[serde(rename = "updatedAt")]
    pub updated_at: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct DeviceCodeResponse {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    pub verification_uri_complete: Option<String>,
    pub expires_in: u64,
    pub interval: Option<u64>,
}

#[derive(Debug, Serialize)]
struct DeviceCodeRequest {
    client_id: String,
    scope: Option<String>,
}

#[derive(Debug, Serialize)]
struct DeviceTokenRequest {
    grant_type: String,
    device_code: String,
    client_id: String,
}

/// Better Auth device token success response
#[derive(Debug, Deserialize)]
struct DeviceTokenSuccess {
    access_token: String,
    token_type: String,
    expires_in: u64,
    scope: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum DeviceTokenResponse {
    Success(DeviceTokenSuccess),
    Error(DeviceTokenError),
}

#[derive(Debug, Deserialize)]
struct DeviceTokenError {
    error: String,
    error_description: Option<String>,
}

/// Request a device code for device authorization flow
pub async fn request_device_code(
    client_id: ClientId,
    device_code_url: &str,
) -> Result<DeviceCodeResponse, String> {
    let client = reqwest::Client::new();
    let request = DeviceCodeRequest {
        client_id: client_id.to_string(),
        scope: Some("openid profile email".to_string()),
    };

    let resp = client
        .post(device_code_url)
        .json(&request)
        .send()
        .await
        .map_err(|e| format!("Failed to request device code: {}", e))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(format!("Device code request failed ({}): {}", status, text));
    }

    resp.json::<DeviceCodeResponse>()
        .await
        .map_err(|e| format!("Failed to parse device code response: {}", e))
}

/// Poll for device authorization token
pub async fn poll_device_token(
    client_id: ClientId,
    device_code: String,
    token_url: &str,
    interval: u64,
) -> Result<String, String> {
    let client = reqwest::Client::new();
    let mut polling_interval = std::time::Duration::from_secs(interval);

    loop {
        tokio::time::sleep(polling_interval).await;

        let request = DeviceTokenRequest {
            grant_type: "urn:ietf:params:oauth:grant-type:device_code".to_string(),
            device_code: device_code.clone(),
            client_id: client_id.to_string(),
        };

        let resp = client
            .post(token_url)
            .json(&request)
            .send()
            .await
            .map_err(|e| format!("Failed to poll for token: {}", e))?;

        let response: DeviceTokenResponse = resp
            .json()
            .await
            .map_err(|e| format!("Failed to parse token response: {}", e))?;

        match response {
            DeviceTokenResponse::Success(token) => return Ok(token.access_token),
            DeviceTokenResponse::Error(err) => match err.error.as_str() {
                "authorization_pending" => {
                    // Continue polling
                    continue;
                }
                "slow_down" => {
                    // Increase polling interval by 5 seconds
                    polling_interval += std::time::Duration::from_secs(5);
                    continue;
                }
                "expired_token" => {
                    return Err("Device code has expired. Please try again.".to_string());
                }
                "access_denied" => {
                    return Err("Access was denied by the user.".to_string());
                }
                _ => {
                    return Err(format!(
                        "Error: {} - {}",
                        err.error,
                        err.error_description.unwrap_or_default()
                    ));
                }
            },
        }
    }
}

/// Get a token using the device authorization flow
/// Returns the session token (access_token) as a string
pub async fn get_token(
    client_id: ClientId,
    device_code_url: String,
    token_url: String,
) -> Result<String, String> {
    // Request device code
    let device_response = request_device_code(client_id.clone(), &device_code_url).await?;

    println!("\n📱 Device Authorization in Progress");
    println!("Please visit: {}", device_response.verification_uri);
    println!("Enter code: {}\n", device_response.user_code);

    // Open browser to verification page
    let url_to_open = device_response
        .verification_uri_complete
        .as_ref()
        .unwrap_or(&device_response.verification_uri);

    println!("🌐 Opening browser...");
    if let Err(e) = open::that(url_to_open) {
        eprintln!("Failed to open browser: {}", e);
        println!("Please manually visit the URL above.");
    }

    let interval = device_response.interval.unwrap_or(5);
    println!(
        "⏳ Waiting for authorization... (polling every {}s)",
        interval
    );

    // Poll for token
    poll_device_token(client_id, device_response.device_code, &token_url, interval).await
}

/// Better Auth session response structure
#[derive(Debug, Deserialize)]
struct BetterAuthSession {
    session: SessionData,
    user: AuthUser,
}

#[derive(Debug, Deserialize)]
struct SessionData {
    id: String,
    #[serde(rename = "expiresAt")]
    expires_at: String,
    token: String,
    #[serde(rename = "userId")]
    user_id: String,
}

// Get user info from Better Auth session endpoint using Bearer token
pub async fn get_user(token: &str, session_url: &str) -> Result<AuthUser, String> {
    let client = reqwest::Client::new();
    let resp = client
        .get(session_url)
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .map_err(|e| format!("Failed to get session: {}", e))?;

    let status = resp.status();
    let headers = resp.headers().clone();
    let body_text = resp
        .text()
        .await
        .map_err(|e| format!("Failed to read response: {}", e))?;

    println!("token: {}", token);

    println!("\n=== Response Debug ===");
    println!("Status: {}", status);
    println!("Headers: {:?}", headers);
    println!("Body: {}", body_text);
    println!("===================\n");

    // check response status
    if !status.is_success() {
        return Err(format!(
            "Session request failed ({}): {}",
            status, body_text
        ));
    }

    // read response body as json
    let session: BetterAuthSession = serde_json::from_str(&body_text).map_err(|e| {
        format!(
            "Failed to parse session response: {} (body: {})",
            e, body_text
        )
    })?;

    Ok(session.user)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore]
    async fn can_get_token() {
        let client_id = ClientId::new("Xdxbwyvdo4gce8jw".to_string());
        let device_code_url = "http://localhost:3000/api/auth/device/code".to_string();
        let token_url = "http://localhost:3000/api/auth/device/token".to_string();

        let token = get_token(client_id, device_code_url, token_url)
            .await
            .unwrap();

        println!("Token: {:?}", token);

        let user = get_user(&token, "http://localhost:3000/api/auth/get-session")
            .await
            .unwrap();

        println!("User: {:?}", user);
    }
}
