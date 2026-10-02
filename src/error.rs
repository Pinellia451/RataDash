use thiserror::Error;

pub type Result<T> = std::result::Result<T, AppError>;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("控制端地址无效 `{value}`: {source}")]
    InvalidControllerUrl {
        value: String,
        #[source]
        source: url::ParseError,
    },

    #[error("不支持的控制端协议 `{0}`，仅支持 http 或 https")]
    UnsupportedControllerScheme(String),

    #[error("控制端地址无法拼接接口路径: {0}")]
    Url(#[from] url::ParseError),

    #[error("控制端认证失败，请检查 Secret")]
    Unauthorized,

    #[error("控制端拒绝请求（HTTP {status}）: {message}")]
    HttpStatus { status: u16, message: String },

    #[error("网络请求失败: {0}")]
    Http(#[from] reqwest::Error),

    #[error("WebSocket 失败: {0}")]
    WebSocket(#[from] tokio_tungstenite::tungstenite::Error),

    #[error("控制端返回了无法识别的数据: {0}")]
    Json(#[from] serde_json::Error),

    #[error("终端操作失败: {0}")]
    Terminal(#[from] std::io::Error),

    #[error("当前以只读模式运行，操作已阻止")]
    ReadOnly,

    #[error("{0}")]
    Message(String),
}
