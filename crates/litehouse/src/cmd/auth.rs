use std::fs::OpenOptions;

use litehouse_auth::{self, AuthUrl, AuthUser, ClientId, TokenResponse, TokenUrl};

#[derive(clap::Subcommand)]
pub enum AuthCommand {
    /// Get the currently logged in user
    Whoami {
        #[clap(long)]
        verbose: bool,
    },
    /// Log in to litehouse.arlyon.dev
    Login,
    /// Log out of litehouse.arlyon.dev
    Logout,
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
struct AuthConfig {
    tokens: TokenResponse,
    user: AuthUser,
}

pub async fn do_auth(auth_command: AuthCommand) {
    let project_dirs = litehouse_config::directories().unwrap();
    std::fs::create_dir_all(project_dirs.config_dir()).unwrap();
    let auth_path = project_dirs.config_dir().join("auth.json");
    tracing::debug!("auth path: {auth_path:?}");
    let auth_file = OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(&auth_path)
        .unwrap();

    match auth_command {
        AuthCommand::Whoami { verbose } => {
            // try to parse the auth file. if it fails, we are not logged in
            let data: Option<AuthConfig> = serde_json::from_reader(auth_file)
                .map(Some)
                .unwrap_or_default();

            if let Some(data) = data {
                println!("logged in as {}", data.user.email);
                if verbose {
                    println!("{:#?}", data);
                }
            } else {
                println!("not logged in");
            }
        }
        AuthCommand::Login => {
            let client_id = ClientId::new("Xdxbwyvdo4gce8jw".to_string());
            let auth_url =
                AuthUrl::new("https://clerk.arlyon.dev/oauth/authorize".to_string()).unwrap();
            let token_url =
                TokenUrl::new("https://clerk.arlyon.dev/oauth/token".to_string()).unwrap();

            let tokens = litehouse_auth::get_token(client_id, auth_url, token_url)
                .await
                .unwrap();

            let user = litehouse_auth::get_user(&tokens, "https://clerk.arlyon.dev/oauth/userinfo")
                .await
                .unwrap();

            let data = AuthConfig { tokens, user };

            serde_json::to_writer_pretty(&auth_file, &data).unwrap();

            println!("logged in as {}", data.user.email);
        }
        AuthCommand::Logout => {
            drop(auth_file);
            std::fs::remove_file(auth_path);
        }
    }
}
