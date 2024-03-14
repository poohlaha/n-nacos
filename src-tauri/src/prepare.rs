//! 对外接口

use crate::error::Error;
use log::info;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct HttpResponse {
    pub(crate) code: u16,
    pub(crate) body: serde_json::Value,
    pub(crate) error: String,
}

pub trait HttpResponseData: Default + Debug + Clone + Serialize + DeserializeOwned + 'static {}

impl HttpResponseData for HttpResponse {}

/// 获取成功 response
pub(crate) fn get_success_response(body: Option<serde_json::Value>) -> HttpResponse {
    let mut data = serde_json::Value::String(String::new());
    if let Some(body) = body {
        data = body
    }
    HttpResponse { code: 200, body: data, error: String::new() }
}

pub(crate) fn get_success_response_by_value<T>(body: T) -> Result<HttpResponse, String>
where
    T: Serialize + DeserializeOwned,
{
    let data = serde_json::to_value(&body).map_err(|err| Error::Error(err.to_string()).to_string())?;
    Ok(HttpResponse { code: 200, body: data, error: String::new() })
}

/// 获取失败 response
pub(crate) fn get_error_response(error: &str) -> HttpResponse {
    HttpResponse {
        code: 500,
        body: serde_json::Value::String(String::new()),
        error: String::from(error),
    }
}

/// 转换 response 为 pipeline
pub(crate) fn convert_res<T>(response: HttpResponse) -> Option<T>
where
    T: Serialize + DeserializeOwned + 'static,
{
    if response.code != 200 {
        return None;
    }

    let data: Result<T, serde_json::Error> = serde_json::from_value(response.body);
    return match data {
        Ok(data) => Some(data),
        Err(err) => {
            info!("convert response to pipeline error: {:#?}", err);
            None
        }
    };
}
