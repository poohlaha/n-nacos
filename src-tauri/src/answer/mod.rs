/*!
  答题, 会保存题目和答案
*/
use crate::database::helper::DBHelper;
use crate::prepare::HttpResponse;
use handlers::utils::Utils;
use log::info;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, MySql};
use uuid::Uuid;

#[derive(Default, Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct AnswerResult {
    pub id: String,
    pub account: String,
    pub pwd: String,
    pub token: Option<String>,
    #[serde(rename = "answerType")]
    pub answer_type: String,

    #[serde(rename = "createTime")]
    pub create_time: Option<String>,

    #[serde(rename = "updateTime")]
    pub update_time: Option<String>,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct AnswerConfig {
    pub id: Option<String>,
    pub account: String,
    pub pwd: String,
    pub token: String,

    #[serde(rename = "answerType")]
    pub answer_type: String,
}

pub struct Answer {}

impl Answer {
    // 保存或修改配置
    pub async fn save_or_update_config(answer: &AnswerConfig) -> Result<HttpResponse, String> {
        info!("save lxr params: {:#?}", answer);
        let query;
        let time = Utils::get_date(None);
        let mut id = String::new();
        if let Some(answer_id) = &answer.id {
            if !answer_id.is_empty() {
                id = answer_id.clone()
            }
        }

        if !id.is_empty() {
            // update
            query = sqlx::query::<MySql>(
                r#"
                        UPDATE answer_config
                        SET
                          account = ?, pwd = ?, token = ?, update_time = ?
                        WHERE id = ?
                    "#,
            )
            .bind(answer.account.clone())
            .bind(answer.pwd.clone())
            .bind(answer.token.clone())
            .bind(time.clone())
            .bind(id.clone());
        } else {
            // save
            let id = Uuid::new_v4().to_string();
            query = sqlx::query::<MySql>("INSERT INTO answer_config (id, account, pwd, token, answer_type, create_time) VALUES (?, ?, ?, ?, ?, ?)")
                .bind(id.clone())
                .bind(&answer.account)
                .bind(&answer.pwd)
                .bind(&answer.token)
                .bind(&answer.answer_type)
                .bind(&time)
        }

        DBHelper::execute_update(query).await
    }

    // 获取配置
    pub async fn get_answer_config() -> Result<HttpResponse, String> {
        let sql = String::from(
            r#"
            SELECT id,
            account,
            pwd,
            token,
            answer_type,
            create_time,
            update_time
            FROM answer_config
        "#,
        );

        let query = sqlx::query_as::<_, AnswerResult>(&sql);
        DBHelper::execute_query(query).await
    }
}
