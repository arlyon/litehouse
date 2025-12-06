use std::fs::OpenOptions;

use litehouse_auth::{self, AuthUser, ClientId};
use miette::{Context, IntoDiagnostic};

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
    token: String,
    user: AuthUser,
}

pub async fn do_auth(auth_command: AuthCommand) -> Result<(), miette::Error> {
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
            let device_code_url = "http://localhost:3000/api/auth/device/code".to_string();
            let token_url = "http://localhost:3000/api/auth/device/token".to_string();

            let token = litehouse_auth::get_token(client_id, device_code_url, token_url)
                .await
                .map_err(|e| miette::miette!("Failed to get token: {}", e))?;

            let user = litehouse_auth::get_user(&token, "http://localhost:3000/api/auth/get-session")
                .await
                .map_err(|e| miette::miette!("Failed to get user info: {}", e))?;

            let data = AuthConfig { token, user };

            serde_json::to_writer_pretty(&auth_file, &data).unwrap();

            println!("logged in as {}", data.user.email);
        }
        AuthCommand::Logout => {
            drop(auth_file);
            std::fs::remove_file(auth_path)
                .into_diagnostic()
                .wrap_err("while removing auth file")?;
        }
    }

    Ok(())
}
