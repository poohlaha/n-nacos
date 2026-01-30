//! 导出设置方法

use crate::answer::{Answer, AnswerConfig, AnswerResult};
use crate::error::Error;
use crate::prepare::{get_error_response, get_success_response, HttpResponse};
use crate::setting::Settings;
use crate::task::Task;
use std::io::BufRead;
use std::path::PathBuf;
use std::process::{Command, Stdio};

/// 保存配置
#[tauri::command]
pub async fn save_or_update_answer_config(config: AnswerConfig) -> Result<HttpResponse, String> {
    Task::task_param_future::<AnswerConfig, _, _>(config, |config| async move { Answer::save_or_update_config(&*config).await }).await
}

/// 获取配置
#[tauri::command]
pub async fn get_answer_config() -> Result<HttpResponse, String> {
    Task::task_param_future::<AnswerConfig, _, _>(AnswerConfig::default(), |_| async move { Answer::get_answer_config().await }).await
}

#[tauri::command]
pub async fn start_answer() -> Result<HttpResponse, String> {
    let settings = Settings::get_settings();
    if settings.is_none() {
        return Ok(get_error_response("执行失败, 没有获取到设置"));
    }

    let mut node_js_dir = String::new();
    if let Some(settings) = settings {
        node_js_dir = settings.node_js_dir.clone();
    }

    if node_js_dir.is_empty() {
        return Ok(get_error_response("执行失败, NodeJs 目录为空"));
    }

    let answers = Answer::get_answer_config().await;
    let answer = match answers {
        Ok(response) => {
            let results: Vec<AnswerResult> = serde_json::from_value(response.body).map_err(|err| Error::Error(err.to_string()).to_string())?;
            if let Some(result) = results.iter().find(|r| r.answer_type == "ZN") {
                Some(result.clone())
            } else {
                None
            }
        }
        Err(_) => None,
    };

    if answer.is_none() {
        return Ok(get_error_response("执行失败, 未获取到知鸟信息"));
    }

    let answer = answer.unwrap();
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("automation/puppeteer-runner.js");

    println!("path: {:#?}", path);

    let mut token = String::new();
    if let Some(t) = answer.token {
        token = t.clone()
    }
    let full_cmd = format!("{} {} {} {} {}", node_js_dir, path.display(), answer.account, answer.pwd, token);
    let mut cmd = Command::new("osascript");

    let mut child = cmd
        .arg("-e")
        .arg(format!(r#"tell application "Terminal" to do script "{}""#, full_cmd))
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .map_err(|e| e.to_string())?;

    // 监听 stdout
    if let Some(stdout) = child.stdout.take() {
        let reader = std::io::BufReader::new(stdout);
        for line in reader.lines() {
            let line = line.unwrap();
            println!("[PUPPETEER STDOUT] {}", line);
        }
    }

    // 监听 stderr
    if let Some(stderr) = child.stderr.take() {
        let reader = std::io::BufReader::new(stderr);
        for line in reader.lines() {
            let line = line.unwrap();
            eprintln!("[PUPPETEER STDERR] {}", line);
        }
    }

    let status = child.wait().map_err(|e| e.to_string())?;
    if !status.success() {
        return Err(format!("Puppeteer exited with {:?}", status.code()));
    }

    Ok(get_success_response(None))
}
