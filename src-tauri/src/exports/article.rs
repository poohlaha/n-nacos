//! 导出文章方法

use crate::article::{Article};
use crate::prepare::{get_error_response, get_success_response_by_value, HttpResponse};
use crate::task::Task;

/// 保存文章
#[tauri::command]
pub async fn save_article(article: Article) -> Result<HttpResponse, String> {
    Task::task_param_future::<Article, _, _>(article, |article| async move { Article::save(&*article).await }).await
}

/// 获取文章列表
#[tauri::command]
pub async fn get_article_list() -> Result<HttpResponse, String> {
    Task::task_param_future::<Article, _, _>(Article::default(), |_| async move { Article::get_list().await }).await
}

/// 获取文章标签列表
#[tauri::command]
pub async fn get_article_tag_list() -> Result<HttpResponse, String> {
    Task::task_param_future::<Article, _, _>(Article::default(), |_| async move {
        let res = Article::get_tag_list().await;
        match res {
            Ok(res) => get_success_response_by_value(res),
            Err(err) => Ok(get_error_response(&err))
        }
    }).await
}

