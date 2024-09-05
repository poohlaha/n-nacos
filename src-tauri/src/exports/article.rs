//! 导出文章方法

use crate::article::{Article, ArticleQuery};
use crate::prepare::{get_error_response, get_success_response_by_value, HttpResponse};
use crate::task::Task;

/// 保存文章
#[tauri::command]
pub async fn save_or_update_article(article: Article) -> Result<HttpResponse, String> {
    Task::task_param_future::<Article, _, _>(article, |article| async move { Article::save_or_update(&*article).await }).await
}

/// 获取文章列表
#[tauri::command]
pub async fn get_article_list(query: ArticleQuery) -> Result<HttpResponse, String> {
    Task::task_param_future::<ArticleQuery, _, _>(query, |query| async move { Article::get_list(&*query).await }).await
}

/// 获取文章详情
#[tauri::command]
pub async fn get_article_detail(id: String) -> Result<HttpResponse, String> {
    let mut query = ArticleQuery::default();
    query.id = Some(id);
    Task::task_param_future::<ArticleQuery, _, _>(query, |query| async move {
        let query = &*query;
        let id = query.id.clone().unwrap_or(String::new());
        Article::get_by_id(&id).await
    })
    .await
}

/// 删除文章
#[tauri::command]
pub async fn delete_article(id: String) -> Result<HttpResponse, String> {
    let mut query = ArticleQuery::default();
    query.id = Some(id);
    Task::task_param_future::<ArticleQuery, _, _>(query, |query| async move {
        let query = &*query;
        let id = query.id.clone().unwrap_or(String::new());
        Article::delete(&id).await
    })
    .await
}

/// 获取文章标签列表
#[tauri::command]
pub async fn get_article_tag_list() -> Result<HttpResponse, String> {
    Task::task_param_future::<Article, _, _>(Article::default(), |_| async move {
        let res = Article::get_tag_list(Vec::new()).await;
        match res {
            Ok(res) => get_success_response_by_value(res),
            Err(err) => Ok(get_error_response(&err)),
        }
    })
    .await
}
