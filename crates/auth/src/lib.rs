use std::convert::Infallible;

use bytes::Bytes;
use http_body_util::Empty;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::StatusCode;
use hyper_util::rt::TokioIo;
use oauth2::basic::{BasicClient, BasicTokenType};
use oauth2::reqwest::async_http_client;
pub use oauth2::{AuthUrl, ClientId, TokenUrl};
use oauth2::{
    AuthorizationCode, CsrfToken, EmptyExtraTokenFields, PkceCodeChallenge, RedirectUrl, Scope,
    StandardTokenResponse, TokenResponse as _,
};
use serde::Deserialize;

pub type TokenResponse = StandardTokenResponse<EmptyExtraTokenFields, BasicTokenType>;

#[derive(Deserialize)]
struct QsData {
    code: String,
    state: String,
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct AuthUser {
    pub email: String,
    pub family_name: Option<String>,
    pub given_name: Option<String>,
    pub name: Option<String>,
    pub picture: Option<String>,
    pub username: Option<String>,
    pub user_id: String,
}

/// Get a token for use with the litehouse API.
///
/// If the access token has more than 30 minutes left, it will be returned as-is.
/// If the refresh token is still valid, it will be used to get a new access token.
/// Otherwise, a new access token will be requested.
pub async fn get_token(
    client_id: ClientId,
    auth_url: AuthUrl,
    token_url: TokenUrl,
) -> Result<TokenResponse, ()> {
    let redirect_url =
        RedirectUrl::new("http://localhost:9789/oauth2/callback".to_string()).unwrap();

    let client = BasicClient::new(client_id, None, auth_url, Some(token_url))
        // Set the URL the user will be redirected to after the authorization process.
        .set_redirect_uri(redirect_url);

    // Generate a PKCE challenge.
    let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();

    // Generate the full authorization URL.
    let (auth_url, csrf_token) = client
        .authorize_url(CsrfToken::new_random)
        // Set the desired scopes.
        .add_scope(Scope::new("profile".to_string()))
        .add_scope(Scope::new("email".to_string()))
        // Set the PKCE code challenge.
        .set_pkce_challenge(pkce_challenge)
        .url();

    let socket = tokio::net::TcpListener::bind(("0.0.0.0", 9789))
        .await
        .unwrap();
    let (token_tx, mut token_rx) = tokio::sync::watch::channel(None);

    if open::that(auth_url.to_string()).is_err() {
        println!(
            "Please open the following url in your browser: {}",
            auth_url
        );
    }

    let token = loop {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                return Err(());
            },
            conn = socket.accept() => {
                let (conn, _addr) = conn.unwrap();
                let io = TokioIo::new(conn);
                let token_tx = token_tx.clone();

                tokio::task::spawn(async move {
                    http1::Builder::new()
                        .serve_connection(
                            io,
                            service_fn(|req| {
                                let token_tx = token_tx.clone();
                                async move {
                                    let mut resp = hyper::Response::new(Empty::<Bytes>::new());
                                    let Some(qs) = req.uri().query() else {
                                        *resp.status_mut() = StatusCode::BAD_REQUEST;
                                        return Ok::<_, Infallible>(resp);
                                    };

                                    let Ok(data) = serde_qs::from_str::<QsData>(qs) else {
                                        *resp.status_mut() = StatusCode::BAD_REQUEST;
                                        return Ok::<_, Infallible>(resp);
                                    };

                                    token_tx.send(Some(data.code)).unwrap();

                                    Ok::<_, Infallible>(resp)
                                }
                            }),
                        )
                        .await
                        .unwrap();
                });
            }
            token = token_rx.wait_for(|v| v.is_some()) => {
                let token = token.unwrap();
                break token.as_ref().unwrap().to_owned();
            }
        }
    };

    let token_result = client
        .exchange_code(AuthorizationCode::new(token))
        // Set the PKCE code verifier.
        .set_pkce_verifier(pkce_verifier)
        .request_async(async_http_client)
        .await
        .unwrap();

    Ok(token_result)
}

// do a request to `user_info_url` with the token as a Authorization header
pub async fn get_user(token: &TokenResponse, user_info_url: &str) -> Result<AuthUser, ()> {
    // get hyper client and make request
    let client = reqwest::Client::new();
    let resp = client
        .get(user_info_url)
        .header(
            "Authorization",
            format!("Bearer {}", token.access_token().secret()),
        )
        .send()
        .await
        .unwrap();

    // check response status
    if !resp.status().is_success() {
        return Err(());
    }

    // read response body as json
    let user: AuthUser = resp.json().await.unwrap();

    Ok(user)
}

#[cfg(test)]
mod tests {

    use super::*;

    #[tokio::test]
    #[ignore]
    async fn can_get_token() {
        let client_id = ClientId::new("Xdxbwyvdo4gce8jw".to_string());
        let auth_url =
            AuthUrl::new("https://clerk.arlyon.dev/oauth/authorize".to_string()).unwrap();
        let token_url = TokenUrl::new("https://clerk.arlyon.dev/oauth/token".to_string()).unwrap();

        let result = get_token(client_id, auth_url, token_url).await.unwrap();

        println!("{:?}", result.access_token().secret());

        let user = get_user(
            result,
            "https://clerk.arlyon.dev/oauth/userinfo".to_string(),
        )
        .await
        .unwrap();

        println!("{:?}", user);
    }
}
