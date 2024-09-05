//! 文章

use crate::database::helper::DBHelper;
use crate::error::Error;
use crate::prepare::{get_error_response, get_success_response_by_value, HttpResponse};
use handlers::utils::Utils;
use log::info;
use serde::{Deserialize, Serialize};
use sqlx::mysql::MySqlArguments;
use sqlx::query::Query;
use sqlx::{FromRow, MySql, Row};
use std::collections::HashSet;
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
    #[serde(rename = "createTime")]
    pub(crate) create_time: Option<String>, // 创建时间
    #[serde(rename = "updateTime")]
    pub(crate) update_time: Option<String>, // 修改时间
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct ArticleResult {
    pub list: Vec<Article>,
    #[serde(rename = "tagList")]
    pub tag_list: Vec<ArticleTag>,
    #[serde(rename = "listCount")]
    pub list_count: u32,
    #[serde(rename = "tagClassifyList")]
    pub tag_classify_list: Vec<ArticleTagClassify>,
    #[serde(rename = "archiveList")]
    pub archive_list: Vec<ArticleArchive>,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct ArticleTagClassify {
    pub id: String,
    #[serde(rename = "tagName")]
    pub tag_name: String,
    #[serde(rename = "articleCount")]
    pub article_count: u32,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct ArticleQuery {
    #[serde(rename = "currentPage")]
    pub current_page: u32,
    #[serde(rename = "pageSize")]
    pub page_size: i32,
    #[serde(rename = "onlyQueryList")]
    pub only_query_list: bool,
    pub id: Option<String>,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct ArticleArchive {
    #[serde(rename = "monthName")]
    pub month_name: String,
    #[serde(rename = "yearName")]
    pub year_name: String,
    #[serde(rename = "articleCount")]
    pub article_count: u32,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct TagArticle {
    #[serde(rename = "articleId")]
    pub article_id: String,
    #[serde(rename = "articleTitle")]
    pub article_title: String,
    #[serde(rename = "articleContent")]
    pub article_content: String,
    #[serde(rename = "articleCreateTime")]
    pub article_create_time: String,
    #[serde(rename = "articleCount")]
    pub article_count: u32,
    #[serde(rename = "articleYear")]
    pub article_year: String,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct ArchiveQuery {
    #[serde(rename = "monthName")]
    pub month_name: String,
    #[serde(rename = "yearName")]
    pub year_name: String,
}

impl Article {
    /// 列表
    pub(crate) async fn get_list(query: &ArticleQuery) -> Result<HttpResponse, String> {
        let list = Self::get_query_list(query).await?;
        let list_count = Self::get_query_list_count().await?;
        if query.only_query_list {
            return get_success_response_by_value(ArticleResult {
                list,
                tag_list: vec![],
                list_count,
                tag_classify_list: vec![],
                archive_list: vec![],
            });
        }

        let tag_list = Self::get_tag_list(Vec::new()).await?;
        let tag_classify_list = Self::get_tag_classify().await?;
        let archive_list = Self::get_archive_list().await?;

        get_success_response_by_value(ArticleResult {
            list,
            tag_list,
            list_count,
            tag_classify_list,
            archive_list,
        })
    }

    async fn get_query_list(query: &ArticleQuery) -> Result<Vec<Article>, String> {
        let mut sql = String::from(
            r#"
            SELECT
                a.id AS article_id,
                a.title AS article_title,
                a.content AS article_content,
                a.tag_ids AS article_tag_ids,
                a.create_time AS article_create_time,
                a.update_time AS article_update_time
            FROM
                article a
        "#,
        );

        if let Some(id) = &query.id {
            if !id.is_empty() {
                sql.push_str(&format!(" WHERE a.id = '{}' ", id));
            }
        }

        sql.push_str(
            r#"
            ORDER BY
                CASE
                    WHEN
                         a.update_time IS NULL or a.update_time = ''
                    THEN
                         0
                    ELSE
                         1
                END DESC,
            a.update_time DESC,
            a.create_time DESC
        "#,
        );

        if query.page_size > 0 {
            sql.push_str(&format!(" LIMIT {},{} ", (query.current_page - 1) * (query.page_size as u32), (query.page_size as u32) * query.current_page));
        }

        let query = sqlx::query(&sql);
        let rows = DBHelper::execute_rows(query).await?;
        if rows.is_empty() {
            return Ok(Vec::new());
        }

        let mut list: Vec<Article> = Vec::new();
        for row in rows.iter() {
            let article_id = row.try_get("article_id").unwrap_or(String::new());
            let article_tag_ids = row.try_get("article_tag_ids").unwrap_or(String::new());
            let mut tags_ids: Vec<String> = Vec::new();
            if !article_tag_ids.is_empty() {
                tags_ids = article_tag_ids.split(",").map(|s| s.to_string()).collect();
            }

            let article_tags = Self::get_tag_list(tags_ids).await?;
            let tags: Vec<String> = article_tags.iter().map(|a| a.name.to_string()).collect();
            list.push(Article {
                id: Some(article_id),
                title: row.try_get("article_title").unwrap_or(String::new()),
                tags,
                content: row.try_get("article_content").unwrap_or(String::new()),
                create_time: row.try_get("article_create_time").unwrap_or(None),
                update_time: row.try_get("article_update_time").unwrap_or(None),
            });
        }

        Ok(list)
    }

    async fn get_query_list_count() -> Result<u32, String> {
        let sql = String::from(
            r#"
            SELECT
                COUNT(a.id) as article_count
            FROM
                article a
            LEFT JOIN article_tag t ON FIND_IN_SET(t.id, a.tag_ids)
        "#,
        );

        let query = sqlx::query(&sql);
        let rows = DBHelper::execute_rows(query).await?;
        if rows.is_empty() {
            return Ok(0);
        }

        let row = rows.get(0);
        if let Some(row) = row {
            let article_count = row.try_get::<i64, _>("article_count").unwrap_or(0);
            return Ok(article_count as u32);
        }

        return Ok(0);
    }

    /// 保存或修改
    pub(crate) async fn save_or_update(article: &Article) -> Result<HttpResponse, String> {
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

        if tags.is_empty() {
            return Ok(get_error_response("tags is empty !"));
        }

        // 过滤已存在的 tag
        let tag_list = Self::get_tag_list(Vec::new()).await?;
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

        let mut query_list: Vec<Query<MySql, MySqlArguments>> = Vec::new();
        let create_time = Utils::get_date(None);

        // 插入 tags
        let mut tag_ids = Self::insert_tags(&new_tags, &mut query_list, create_time.clone()).await;
        if new_tags.len() > 0 {
            tag_ids.extend(remaining_tags);
        } else {
            tag_ids = remaining_tags.clone()
        }

        if let Some(id) = &article.id {
            let response = Self::get_by_id(id).await?;
            if response.code != 200 {
                return Ok(response);
            }

            Self::update(&article, &tag_ids, create_time.clone(), &mut query_list).await;
        } else {
            // insert
            Self::save(&article, &tag_ids, create_time.clone(), &mut query_list).await;
        }

        DBHelper::batch_commit(query_list).await
    }

    /// 保存
    async fn save(article: &Article, tag_ids: &Vec<String>, create_time: String, query_list: &mut Vec<Query<'_, MySql, MySqlArguments>>) {
        let id = Uuid::new_v4().to_string();

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
        .bind(id)
        .bind(article.title.clone())
        .bind(article.content.clone())
        .bind(tag_ids.join(","))
        .bind(create_time.clone())
        .bind("");
        query_list.push(article_query);
    }

    /// 保存
    async fn update(article: &Article, tag_ids: &Vec<String>, create_time: String, query_list: &mut Vec<Query<'_, MySql, MySqlArguments>>) {
        let article_query = sqlx::query::<MySql>(
            r#"
            UPDATE article
            SET
              title = ?, content = ?, tag_ids = ?, update_time = ?
            WHERE id = ?
        "#,
        )
        .bind(article.title.clone())
        .bind(article.content.clone())
        .bind(tag_ids.join(","))
        .bind(create_time.clone())
        .bind(article.id.clone());
        query_list.push(article_query);
    }

    /// 插入 tags
    async fn insert_tags(new_tags: &Vec<String>, query_list: &mut Vec<Query<'_, MySql, MySqlArguments>>, create_time: String) -> Vec<String> {
        let mut tag_ids: Vec<String> = Vec::new();
        if new_tags.is_empty() {
            return tag_ids;
        }

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
            .bind(tag.clone())
            .bind(create_time.clone())
            .bind("");
            query_list.push(article_tag_query);
        }

        return tag_ids;
    }

    pub(crate) async fn get_by_id(id: &str) -> Result<HttpResponse, String> {
        if id.is_empty() {
            return Ok(get_error_response("查询文章详情失败, id 为空 !"));
        }

        // 根据 id 查找是否存在
        info!("get article by id: {}", id);
        let article_list = Self::get_query_list(&ArticleQuery {
            current_page: 0,
            page_size: -1,
            only_query_list: true,
            id: Some(id.to_string()),
        })
        .await?;

        if article_list.is_empty() {
            return Ok(get_error_response("未找到文章记录 !"));
        }

        let article = article_list.get(0).unwrap();
        return get_success_response_by_value(article.clone());
    }

    /// 获取 tag 列表
    pub(crate) async fn get_tag_list(tag_ids: Vec<String>) -> Result<Vec<ArticleTag>, String> {
        let mut sql = String::from(
            r#"
            SELECT id,
            `name`,
            create_time,
            update_time
            FROM article_tag
        "#,
        );

        if !tag_ids.is_empty() {
            sql.push_str(" WHERE (");
            for (i, id) in tag_ids.iter().enumerate() {
                sql.push_str(&format!(" id = '{}'", id));
                if i != tag_ids.len() - 1 {
                    sql.push_str(" OR ")
                }
            }
            sql.push_str(")");
        }

        sql.push_str(" ORDER By create_time DESC ");

        let query = sqlx::query_as::<_, ArticleTag>(&sql);
        let response = DBHelper::execute_query(query).await?;
        if response.code != 200 {
            return Err(Error::convert_string(&format!("get article tag list error: {}", response.error)));
        }

        let tags: Vec<ArticleTag> = serde_json::from_value(response.body).map_err(|err| Error::Error(err.to_string()).to_string())?;
        return Ok(tags);
    }

    /// 获取标签分类
    pub(crate) async fn get_tag_classify() -> Result<Vec<ArticleTagClassify>, String> {
        let sql = String::from(
            r#"
            SELECT
                t.id AS tag_id,
                t.`name` AS tag_name,
                COUNT(a.id) AS article_count
            FROM
                article_tag t
            LEFT JOIN
                article a ON FIND_IN_SET(t.id, a.tag_ids) > 0
            GROUP BY
                t.id, t.`name`
            ORDER BY article_count DESC
        "#,
        );

        let query = sqlx::query(&sql);
        let rows = DBHelper::execute_rows(query).await?;
        if rows.is_empty() {
            return Ok(Vec::new());
        }

        let mut tag_classify_list: Vec<ArticleTagClassify> = Vec::new();
        for row in rows.iter() {
            tag_classify_list.push(ArticleTagClassify {
                id: row.try_get("tag_id").unwrap_or(String::new()),
                tag_name: row.try_get("tag_name").unwrap_or(String::new()),
                article_count: row.try_get::<i64, _>("article_count").unwrap_or(0) as u32,
            })
        }

        Ok(tag_classify_list)
    }

    /// 查询文章归档数
    async fn get_archive_list() -> Result<Vec<ArticleArchive>, String> {
        let sql = String::from(
            r#"
           SELECT
                CASE
                    WHEN MONTH(STR_TO_DATE(a.create_time, '%Y-%m-%d')) = 1 THEN '一月'
                    WHEN MONTH(STR_TO_DATE(a.create_time, '%Y-%m-%d')) = 2 THEN '二月'
                    WHEN MONTH(STR_TO_DATE(a.create_time, '%Y-%m-%d')) = 3 THEN '三月'
                    WHEN MONTH(STR_TO_DATE(a.create_time, '%Y-%m-%d')) = 4 THEN '四月'
                    WHEN MONTH(STR_TO_DATE(a.create_time, '%Y-%m-%d')) = 5 THEN '五月'
                    WHEN MONTH(STR_TO_DATE(a.create_time, '%Y-%m-%d')) = 6 THEN '六月'
                    WHEN MONTH(STR_TO_DATE(a.create_time, '%Y-%m-%d')) = 7 THEN '七月'
                    WHEN MONTH(STR_TO_DATE(a.create_time, '%Y-%m-%d')) = 8 THEN '八月'
                    WHEN MONTH(STR_TO_DATE(a.create_time, '%Y-%m-%d')) = 9 THEN '九月'
                    WHEN MONTH(STR_TO_DATE(a.create_time, '%Y-%m-%d')) = 10 THEN '十月'
                    WHEN MONTH(STR_TO_DATE(a.create_time, '%Y-%m-%d')) = 11 THEN '十一月'
                    WHEN MONTH(STR_TO_DATE(a.create_time, '%Y-%m-%d')) = 12 THEN '十二月'
                END AS month_name,
                DATE_FORMAT(STR_TO_DATE(a.create_time, '%Y-%m-%d'), '%Y') AS year_name,
                COUNT(a.id) AS article_count
            FROM
                article a
            GROUP BY
                year_name, month_name
            ORDER BY
                year_name DESC, FIELD(month_name, '一月', '二月', '三月', '四月', '五月', '六月', '七月', '八月', '九月', '十月', '十一月', '十二月') DESC;
        "#,
        );

        let query = sqlx::query(&sql);
        let rows = DBHelper::execute_rows(query).await?;
        if rows.is_empty() {
            return Ok(Vec::new());
        }

        let mut article_archive_list: Vec<ArticleArchive> = Vec::new();
        for row in rows.iter() {
            article_archive_list.push(ArticleArchive {
                month_name: row.try_get("month_name").unwrap_or(String::new()),
                year_name: row.try_get("year_name").unwrap_or(String::new()),
                article_count: row.try_get::<i64, _>("article_count").unwrap_or(0) as u32,
            })
        }

        Ok(article_archive_list)
    }

    /// 删除
    pub(crate) async fn delete(id: &str) -> Result<HttpResponse, String> {
        let response = Self::get_by_id(id).await?;
        if response.code != 200 {
            return Ok(response);
        }

        let query = sqlx::query::<MySql>("DELETE FROM article WHERE id = ?").bind(&id);
        return DBHelper::execute_update(query).await;
    }

    /// 根据 tag_id 获取文章数
    pub(crate) async fn get_tag_article_list(id: &str) -> Result<Vec<TagArticle>, String> {
        if id.is_empty() {
            return Err(Error::convert_string("根据标签获取文章失败, id 为空"));
        }

        let sql = String::from(
            r#"
        SELECT
            a.id AS article_id,
            a.title AS article_title,
            a.content AS article_content,
            a.create_time AS article_create_time,
            COUNT(a.id) AS article_count,
            YEAR(a.create_time) AS article_year
        FROM
            article a
        JOIN
            article_tag t
        ON
            FIND_IN_SET(t.id, a.tag_ids) > 0
        WHERE
            t.id = ?
        GROUP BY
            article_year, a.id
        ORDER BY
            article_year DESC, a.create_time DESC
        "#,
        );

        let query = sqlx::query(&sql).bind(id);
        let rows = DBHelper::execute_rows(query).await?;
        if rows.is_empty() {
            return Ok(Vec::new());
        }

        let mut list: Vec<TagArticle> = Vec::new();
        for row in rows.iter() {
            let year: i32 = row.try_get("article_year").unwrap_or_else(|err| {
                info!("get year err:{:#?}", err);
                -1
            });

            let mut article_year = String::new();
            if year > 0 {
                article_year = format!("{}", year)
            }

            list.push(TagArticle {
                article_id: row.try_get("article_id").unwrap_or(String::new()),
                article_title: row.try_get("article_title").unwrap_or(String::new()),
                article_count: row.try_get::<i64, _>("article_count").unwrap_or(0) as u32,
                article_content: row.try_get("article_content").unwrap_or(String::new()),
                article_year,
                article_create_time: row.try_get("article_create_time").unwrap_or(String::new()),
            })
        }

        Ok(list)
    }

    /// 获取归档文章
    pub(crate) async fn get_archive_article_list(archive_query: &ArchiveQuery) -> Result<Vec<TagArticle>, String> {
        if archive_query.year_name.is_empty() {
            return Err(Error::convert_string("查询归档文章失败, yearName 为空"));
        }

        if archive_query.month_name.is_empty() {
            return Err(Error::convert_string("查询归档文章失败, yearName 为空"));
        }

        let sql = String::from(
            r#"
        SELECT
            a.id AS article_id,
            a.title AS article_title,
            a.content AS article_content,
            a.create_time AS article_create_time
        FROM
            article a
        WHERE
            DATE_FORMAT(STR_TO_DATE(a.create_time, '%Y-%m-%d'), '%Y') = ?
            AND (
                CASE MONTH(STR_TO_DATE(a.create_time, '%Y-%m-%d'))
                    WHEN 1 THEN '一月'
                    WHEN 2 THEN '二月'
                    WHEN 3 THEN '三月'
                    WHEN 4 THEN '四月'
                    WHEN 5 THEN '五月'
                    WHEN 6 THEN '六月'
                    WHEN 7 THEN '七月'
                    WHEN 8 THEN '八月'
                    WHEN 9 THEN '九月'
                    WHEN 10 THEN '十月'
                    WHEN 11 THEN '十一月'
                    WHEN 12 THEN '十二月'
                END
            ) = ?
        ORDER BY
            a.create_time DESC
        "#,
        );

        let query = sqlx::query(&sql).bind(&archive_query.year_name).bind(&archive_query.month_name);
        let rows = DBHelper::execute_rows(query).await?;
        if rows.is_empty() {
            return Ok(Vec::new());
        }

        let mut list: Vec<TagArticle> = Vec::new();
        for row in rows.iter() {
            list.push(TagArticle {
                article_id: row.try_get("article_id").unwrap_or(String::new()),
                article_title: row.try_get("article_title").unwrap_or(String::new()),
                article_count: 0,
                article_content: row.try_get("article_content").unwrap_or(String::new()),
                article_year: archive_query.year_name.clone(),
                article_create_time: row.try_get("article_create_time").unwrap_or(String::new()),
            })
        }

        Ok(list)
    }
}
