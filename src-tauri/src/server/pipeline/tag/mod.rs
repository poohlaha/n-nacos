//! 标签

use crate::database::helper::DBHelper;
use crate::prepare::HttpResponse;
use log::info;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Default, Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PipelineTag {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) value: String,
    pub(crate) create_time: Option<String>,
    pub(crate) update_time: Option<String>,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PipelineTagQueryForm {
    pub(crate) value: String,
    pub(crate) id: String,
}

impl PipelineTag {
    pub(crate) async fn get_list(form: PipelineTagQueryForm) -> Result<HttpResponse, String> {
        info!("query tag form: {:?}", form);
        let query;

        if !form.value.is_empty() && !form.id.is_empty() {
            query = sqlx::query_as::<_, PipelineTag>("SELECT * FROM pipeline_tag WHERE id = ? and value = ? ORDER By create_time asc").bind(&form.id).bind(&form.value);
        } else if !form.id.is_empty() {
            query = sqlx::query_as::<_, PipelineTag>("SELECT * FROM pipeline_tag WHERE id = ? ORDER By create_time asc").bind(&form.id);
        } else if !form.value.is_empty() {
            query = sqlx::query_as::<_, PipelineTag>("SELECT * FROM pipeline_tag WHERE value = ? ORDER By create_time asc").bind(&form.value);
        } else {
            query = sqlx::query_as::<_, PipelineTag>("SELECT * FROM pipeline_tag ORDER By create_time asc");
        }

        DBHelper::execute_query(query).await
    }
}
