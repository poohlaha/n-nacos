//! 发送前端事件

use crate::prepare::HttpResponse;
use log::{error, info};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};

pub struct EventEmitter;

/// 流水线运行结果事件名称
const PIPELINE_EXEC_RES_EVENT_NAME: &str = "pipeline_exec_response";

/// 流水线运行步骤日志事件名称
const PIPELINE_EXEC_STEP_LOG_EVENT_NAME: &str = "pipeline_exec_step_log";

/// 流水线运行步骤结果事件名称
const PIPELINE_EXEC_STEP_RES_EVENT_NAME: &str = "pipeline_exec_step_response";

/// 流水线运行步骤发送通知事件名称
const PIPELINE_EXEC_STEP_NOTICE_RES_EVENT_NAME: &str = "pipeline_exec_step_notice";

/// 监控结果事件名称
const MONITOR_RES_EVENT_NAME: &str = "monitor_response";

pub struct EventSendParams {
    pub(crate) response: Option<HttpResponse>,
    pub(crate) msg: String,
    pub(crate) id: Option<String>,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
struct EventSendMsg {
    pub(crate) msg: String,
    pub(crate) id: String,
}

impl EventEmitter {
    fn emit_response(app: &AppHandle, event_name: &str, response: HttpResponse) {
        let emit = app.emit(event_name, response);
        match emit {
            Ok(_) => {
                info!("send response to window success !");
            }
            Err(err) => {
                error!("send response to window error: {}", err.to_string());
            }
        }
    }

    fn emit_string(app: &AppHandle, event_name: &str, id: &str, msg: &str) {
        let emit = app.emit(event_name, EventSendMsg { msg: msg.to_string(), id: id.to_string() });
        match emit {
            Ok(_) => {
                // info!("send string to window success !");
            }
            Err(err) => {
                error!("send string to window error: {}", err.to_string());
            }
        }
    }

    /// 发送消息
    pub(crate) fn emit(app: &AppHandle, params: EventSendParams, index: u8) {
        let response = params.response.clone();
        let msg = params.msg.clone();
        let id = params.id.clone().unwrap_or(String::new());

        // response
        if index == 0 {
            if let Some(response) = response.clone() {
                Self::emit_response(app, PIPELINE_EXEC_RES_EVENT_NAME, response)
            }
        }

        // step log
        if index == 1 {
            if !msg.is_empty() {
                Self::emit_string(app, PIPELINE_EXEC_STEP_LOG_EVENT_NAME, &id, &msg)
            }
        }

        // step result
        if index == 2 {
            if let Some(response) = response.clone() {
                Self::emit_response(app, PIPELINE_EXEC_STEP_RES_EVENT_NAME, response)
            }
        }

        // step notice
        if index == 3 {
            if let Some(response) = response.clone() {
                Self::emit_response(app, PIPELINE_EXEC_STEP_NOTICE_RES_EVENT_NAME, response)
            }
        }

        // monitor result
        if index == 4 {
            if let Some(response) = response.clone() {
                Self::emit_response(app, MONITOR_RES_EVENT_NAME, response)
            }
        }
    }

    /// 发送运行结果
    pub(crate) fn log_res(app: &AppHandle, response: Option<HttpResponse>) {
        info!("send run response {:#?}", response);
        EventEmitter::emit(app, EventSendParams { response, id: None, msg: String::new() }, 0);
    }

    /// 发送字符串消息
    pub(crate) fn log_event(app: &AppHandle, id: &str, msg: &str) {
        info!("{}", msg);
        EventEmitter::emit(
            app,
            EventSendParams {
                response: None,
                id: Some(id.to_string()),
                msg: msg.to_string(),
            },
            1,
        );
    }

    /// 发送步骤结果
    pub(crate) fn log_step_res(app: &AppHandle, response: Option<HttpResponse>) {
        info!("send run step response");
        EventEmitter::emit(app, EventSendParams { response, id: None, msg: String::new() }, 2);
    }

    /// 发送步骤通知
    pub(crate) fn log_step_notice(app: &AppHandle, response: Option<HttpResponse>) {
        info!("send run step notice");
        EventEmitter::emit(app, EventSendParams { response, id: None, msg: String::new() }, 3);
    }

    /// 发送监控结果
    pub(crate) fn log_monitor_res(app: &AppHandle, response: Option<HttpResponse>) {
        info!("send monitor response");
        EventEmitter::emit(app, EventSendParams { response, id: None, msg: String::new() }, 4);
    }
}
