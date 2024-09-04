//! 文章

use crate::database::helper::DBHelper;
use crate::error::Error;
use crate::prepare::{get_error_response, get_success_response, HttpResponse};
use handlers::utils::Utils;
use serde::{Deserialize, Serialize};
use sqlx::mysql::MySqlArguments;
use sqlx::query::Query;
use sqlx::{FromRow, MySql};
use std::collections::{HashSet};
use uuid::Uuid;

#[derive(Default, Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ArticleTag {
    pub id: String,
    pub name: String,
    #[serde(rename = "createTime")]
    pub(crate) create_time: Option<String>, // 创建时间
    #[serde(rename = "updateTime")]
    pub(crate) update_time: Option<String>, // 修改时间
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Article {
    pub id: Option<String>,
    pub title: String,
    pub tags: Vec<String>,
    pub content: String,
    pub(crate) create_time: Option<String>, // 创建时间
    #[serde(rename = "updateTime")]
    pub(crate) update_time: Option<String>, // 修改时间
}

impl Article {
    /// 列表
    pub(crate) async fn get_list() -> Result<HttpResponse, String> {
        Ok(get_success_response(None))
    }

    /// 保存
    pub(crate) async fn save(article: &Article) -> Result<HttpResponse, String> {
        if article.title.is_empty() {
            return Ok(get_error_response("title is empty !"));
        }

        if article.tags.is_empty() {
            return Ok(get_error_response("tags is empty !"));
        }

        if article.content.is_empty() {
            return Ok(get_error_response("content is empty !"));
        }

        // 过滤标签重复的值
        let mut set: HashSet<String> = HashSet::new();
        let tags: Vec<String> = article
            .tags
            .clone()
            .into_iter()
            .filter_map(|s| {
                let str = s.to_string();
                if set.contains(&str) {
                    None
                } else {
                    set.insert(str.clone());
                    // 首字母转大写，其他小写
                    let chars = str.chars().enumerate().fold(String::new(), |acc, (i, c)| if i == 0 { format!("{}{}", acc, c.to_uppercase()) } else { format!("{}{}", acc, c) });
                    Some(chars)
                }
            })
            .collect();

        // 过滤已存在的 tag
        let tag_list = Self::get_tag_list().await?;
        let mut new_tags: Vec<String> = Vec::new();
        let mut remaining_tags: Vec<String> = Vec::new(); // 剩余的 tags

        for tag in tags.iter() {
            let tag_lower = tag.to_lowercase();
            let arc = tag_list.iter().find(|s| s.name.to_lowercase() == tag_lower.clone());
            if let Some(arc) = arc {
                remaining_tags.push(arc.id.clone());
            } else {
                new_tags.push(tag.clone());
            }
        }

        let id = Uuid::new_v4().to_string();
        let create_time = Utils::get_date(None);
        let mut query_list: Vec<Query<MySql, MySqlArguments>> = Vec::new();

        let mut tag_ids: Vec<String> = Vec::new();
        if new_tags.len() > 0 {
            for tag in new_tags.iter() {
                let tag_id = Uuid::new_v4().to_string();
                tag_ids.push(tag_id.clone());

                let article_tag_query = sqlx::query::<MySql>(
                    r#"
                       INSERT INTO article_tag (
                        id,
                        name,
                        create_time,
                        update_time
                        )
                        VALUES (?, ?, ?, ?)
                    "#,
                )
                .bind(tag_id.clone())
                .bind(tag)
                .bind(&create_time)
                .bind("");
                query_list.push(article_tag_query);
            }

            tag_ids.extend(remaining_tags);
        } else {
            tag_ids = remaining_tags;
        }

        let article_query = sqlx::query::<MySql>(
            r#"
           INSERT INTO article (
            id,
            title,
            content,
            tag_ids,
            create_time,
            update_time
            )
            VALUES (?, ?, ?, ?, ?, ?)
        "#,
        )
        .bind(&id)
        .bind(&article.title)
        .bind(&article.content)
        .bind(tag_ids.join(","))
        .bind(&create_time)
        .bind("");

        query_list.push(article_query);
        DBHelper::batch_commit(query_list).await
    }

    /// 获取 tag 列表
    pub(crate) async fn get_tag_list() -> Result<Vec<ArticleTag>, String> {
        // 判断 IP 是否存在
        let query = sqlx::query_as::<_, ArticleTag>("select id, `name`, create_time, update_time from article_tag");

        let response = DBHelper::execute_query(query).await?;
        if response.code != 200 {
            return Err(Error::convert_string(&format!("get tag list error: {}", response.error)));
        }

        let tags: Vec<ArticleTag> = serde_json::from_value(response.body).map_err(|err| Error::Error(err.to_string()).to_string())?;
        return Ok(tags);
    }
}
