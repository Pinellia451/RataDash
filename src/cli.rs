use std::io::IsTerminal;
use std::time::Duration;

use clap::Parser;
use url::Url;

use crate::error::{AppError, Result};

#[derive(Debug, Parser)]
#[command(
    name = "ratadash",
    version,
    about = "A lightweight terminal controller for Mihomo-compatible proxy cores"
)]
pub struct Cli {
    /// Controller base URL, including an optional secondary path.
    #[arg(long, default_value = "http://127.0.0.1:9090")]
    pub controller: String,

    /// Controller secret. Prefer RATADASH_SECRET to avoid shell history.
    #[arg(long, env = "RATADASH_SECRET", hide_env_values = true)]
    pub secret: Option<String>,

    /// Disable every operation that changes controller state.
    #[arg(long)]
    pub read_only: bool,

    /// REST request timeout in seconds.
    #[arg(long, default_value_t = 8)]
    pub timeout: u64,

    /// Periodic REST refresh interval in milliseconds.
    #[arg(long, default_value_t = 5_000)]
    pub refresh_ms: u64,
}

#[derive(Debug, Clone)]
pub struct RuntimeOptions {
    pub controller: Url,
    pub secret: Option<String>,
    pub read_only: bool,
    pub timeout: Duration,
    pub refresh_interval: Duration,
}

impl Cli {
    pub fn resolve(self) -> Result<RuntimeOptions> {
        let mut controller =
            Url::parse(&self.controller).map_err(|source| AppError::InvalidControllerUrl {
                value: self.controller.clone(),
                source,
            })?;

        match controller.scheme() {
            "http" | "https" => {}
            scheme => {
                return Err(AppError::UnsupportedControllerScheme(scheme.to_string()));
            }
        }

        if controller.path().is_empty() {
            controller.set_path("/");
        }

        let secret = match self.secret {
            Some(secret) if !secret.is_empty() => Some(secret),
            _ if std::io::stdin().is_terminal() => {
                let entered = rpassword::prompt_password("控制端 Secret（无则直接回车）: ")?;
                (!entered.is_empty()).then_some(entered)
            }
            _ => None,
        };

        Ok(RuntimeOptions {
            controller,
            secret,
            read_only: self.read_only,
            timeout: Duration::from_secs(self.timeout.max(1)),
            refresh_interval: Duration::from_millis(self.refresh_ms.max(500)),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::Cli;
    use clap::Parser;

    #[test]
    fn parses_runtime_flags() {
        let cli = Cli::parse_from([
            "ratadash",
            "--controller",
            "https://controller.example/base/",
            "--secret",
            "token",
            "--read-only",
            "--timeout",
            "3",
        ]);

        let options = cli.resolve().expect("valid options");
        assert_eq!(
            options.controller.as_str(),
            "https://controller.example/base/"
        );
        assert_eq!(options.secret.as_deref(), Some("token"));
        assert!(options.read_only);
        assert_eq!(options.timeout.as_secs(), 3);
    }
}
