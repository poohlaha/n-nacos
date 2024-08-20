//! 流水线

use crate::database::helper::DBHelper;
use crate::database::interface::{Treat, TreatBody};
use crate::error::Error;
use crate::exports::pipeline::QueryForm;
use crate::helper::git::GitHandler;
use crate::helper::index::Helper;
use crate::logger::pipeline::PipelineLogger;
use crate::prepare::{get_error_response, get_success_response, get_success_response_by_value, HttpResponse};
use crate::server::pipeline::languages::h5::H5FileHandler;
use crate::server::pipeline::props::{
    H5RunnableVariable, OsCommands, PipelineBasic, PipelineCommandStatus, PipelineGroup, PipelineProcess, PipelineRuntime, PipelineStage, PipelineStatus, PipelineStep, PipelineStepComponent, PipelineTag, PipelineVariable, RunnableVariable,
};
use crate::server::pipeline::runnable::{PipelineRunnable, PipelineRunnableQueryForm};
use async_trait::async_trait;
use handlers::utils::Utils;
use log::{error, info};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::mysql::{MySqlArguments, MySqlRow};
use sqlx::query::Query;
use sqlx::{FromRow, MySql, Row};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use uuid::Uuid;

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct Pipeline {
    pub(crate) id: String,
    #[serde(rename = "serverId")]
    pub(crate) server_id: String, // 服务器 ID
    #[serde(rename = "lastRunTime")]
    pub(crate) last_run_time: Option<String>, // 最后运行时间
    pub(crate) last_run_id: Option<String>,    // 最后运行流水线
    pub(crate) tag: Option<PipelineTag>,       // 标签
    pub(crate) status: Option<PipelineStatus>, // 状态, 同步于 steps

    pub(crate) basic: PipelineBasic, // 基本信息
    #[serde(rename = "processConfig")]
    pub(crate) process_config: PipelineProcess, // 流程配置
    pub(crate) variables: Vec<PipelineVariable>, // 变量
    #[serde(rename = "createTime")]
    pub(crate) create_time: Option<String>, // 创建时间
    #[serde(rename = "updateTime")]
    pub(crate) update_time: Option<String>, // 修改时间

    #[serde(rename = "runnableInfo")]
    pub(crate) runnable_info: Option<RunnableVariable>, // 运行时的信息
    pub(crate) runtime: Option<PipelineRuntime>, // 流水线运行的信息
}

impl<'r> FromRow<'r, MySqlRow> for Pipeline {
    fn from_row(row: &MySqlRow) -> Result<Self, sqlx::Error> {
        let status_str: String = row.try_get("status")?;
        let tag_str: String = row.try_get("tagValue")?;
        let tag = Some(PipelineTag::get(&tag_str));

        let basic = PipelineBasic {
            id: row.try_get("basic_id")?,
            pipeline_id: row.try_get("basic_pipeline_id")?,
            name: row.try_get("basic_name")?,
            tag: tag.clone().unwrap().clone(),
            path: row.try_get("basic_path")?,
            description: row.try_get("basic_description")?,
            create_time: row.try_get("basic_create_time")?,
            update_time: row.try_get("basic_update_time")?,
        };

        Ok(Pipeline {
            id: row.try_get("id")?,
            server_id: row.try_get("server_id")?,
            last_run_time: row.try_get("last_run_time")?,
            last_run_id: row.try_get("last_run_id")?,
            tag,
            status: Some(PipelineStatus::get(&status_str)),
            create_time: row.try_get("create_time")?,
            update_time: row.try_get("update_time")?,
            basic,
            process_config: Default::default(),
            variables: Vec::new(),
            runnable_info: None,
            runtime: None,
        })
    }
}

impl TreatBody for Pipeline {}

#[async_trait]
impl Treat<HttpResponse> for Pipeline {
    type B = Pipeline;

    /// 列表
    async fn get_list(_: &Self::B) -> Result<HttpResponse, String> {
        Ok(get_success_response(Some(Value::Bool(true))))
    }

    /// 插入
    async fn insert(pipeline: &Self::B) -> Result<HttpResponse, String> {
        let res = Self::validate(&pipeline);
        if let Some(res) = res {
            return Ok(res);
        }

        let mut pipeline_clone = pipeline.clone();
        if pipeline_clone.id.is_empty() {
            pipeline_clone.id = Uuid::new_v4().to_string()
        }

        // 创建时间
        let create_time = Utils::get_date(None);

        let basic = &pipeline_clone.basic;

        // 获取 tag
        let response = crate::server::pipeline::tag::PipelineTag::get_list(crate::server::pipeline::tag::PipelineTagQueryForm {
            value: PipelineTag::got(basic.tag.clone()),
            id: "".to_string(),
        })
        .await?;

        if response.code != 200 {
            return Ok(response);
        }

        let data: Vec<crate::server::pipeline::tag::PipelineTag> = serde_json::from_value(response.body).map_err(|err| Error::Error(err.to_string()).to_string())?;
        if data.is_empty() {
            return Ok(get_error_response("保存流水线失败, 该标签不存在"));
        }

        // tag
        let tag = data.get(0).unwrap();

        let mut query_list: Vec<Query<MySql, MySqlArguments>> = Vec::new();

        // 插入 pipeline 表
        let pipeline_query = sqlx::query::<MySql>(
            r#"
            INSERT INTO pipeline (
            id,
            server_id,
            tag_id,
            last_run_time,
            status,
            create_time,
            update_time
            )
            VALUES (?, ?, ?, ?, ?, ?, ?)
        "#,
        )
        .bind(&pipeline_clone.id)
        .bind(&pipeline_clone.server_id)
        .bind(tag.id.clone())
        .bind("")
        .bind(PipelineStatus::got(PipelineStatus::No))
        .bind(&create_time)
        .bind(&pipeline_clone.update_time);
        query_list.push(pipeline_query);

        // 插入 pipeline_basic 表
        let basic_query = sqlx::query::<MySql>(
            r#"
            INSERT INTO pipeline_basic (id, pipeline_id, `name`, tag_id, path, description, create_time, update_time)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
        "#,
        )
        .bind(Uuid::new_v4().to_string().clone())
        .bind(&pipeline_clone.id)
        .bind(&basic.name)
        .bind(&tag.id)
        .bind(&basic.path)
        .bind(&basic.description)
        .bind(&create_time)
        .bind(&basic.update_time);
        query_list.push(basic_query);

        // 解析 process_config
        let process_config = &pipeline_clone.process_config;

        // 1. 流程配置, 插入 pipeline_process 表
        let process_id = Uuid::new_v4().to_string();
        let process_query = sqlx::query::<MySql>(
            r#"
            INSERT INTO pipeline_process (id, pipeline_id, create_time, update_time)
            VALUES (?, ?, ?, ?)
        "#,
        )
        .bind(process_id.clone())
        .bind(&pipeline_clone.id)
        .bind(&create_time)
        .bind(&process_config.update_time);
        query_list.push(process_query);

        // 2. 流水线阶段, 插入 pipeline_stage 表
        Self::insert_stages(&process_config, process_id.clone(), create_time.clone(), &mut query_list);

        // 插入 pipeline_variable 表
        let variables = &pipeline_clone.variables;
        if !variables.is_empty() {
            Self::insert_variables(pipeline_clone.id.clone(), create_time.clone(), basic.update_time.clone(), &pipeline_clone.variables, &mut query_list);
        }

        return DBHelper::batch_commit(query_list).await;
    }

    /// 更新
    async fn update(pipeline: &Self::B) -> Result<HttpResponse, String> {
        if pipeline.id.is_empty() {
            return Ok(get_error_response("更新流水线失败, `id` 不能为空"));
        }

        let res = Self::validate(&pipeline);
        if let Some(res) = res {
            return Ok(res);
        }

        info!("update pipeline params: {:#?}", pipeline);
        let response = Self::get_query_list(&pipeline, None, true).await?;
        if response.code != 200 {
            return Ok(response);
        }

        let data: Vec<Pipeline> = serde_json::from_value(response.body).map_err(|err| Error::Error(err.to_string()).to_string())?;
        if data.is_empty() {
            return Ok(get_error_response("更新流水线失败, 该流水线不存在"));
        }

        if data.len() > 1 {
            return Ok(get_error_response("更新流水线失败, 存在多条相同的流水线"));
        }

        let pi = data.get(0).unwrap();
        let process_id = &pi.process_config.id;

        // 更新
        // 流水线阶段, 插入 pipeline_stage 表
        let create_time = Utils::get_date(None); // 创建时间
        let update_time = Utils::get_date(None); // 创建时间
        let basic = &pipeline.basic;
        let mut query_list = Vec::new();

        Self::delete_by_pipeline(&pipeline.id, &mut query_list);

        // 插入 stages
        Self::insert_stages(&pipeline.process_config, process_id.clone(), create_time.clone(), &mut query_list);

        // 插入 variables 表
        let variables = &pipeline.variables;
        if !variables.is_empty() {
            Self::insert_variables(pipeline.id.clone(), create_time.clone(), basic.update_time.clone(), &pipeline.variables, &mut query_list);
        }

        // 更新 pipeline 表 update_time
        let pipeline_query = sqlx::query::<MySql>(
            r#"
            UPDATE pipeline SET update_time = ? WHERE id = ?
        "#,
        )
        .bind(update_time.clone())
        .bind(&pipeline.id);
        query_list.push(pipeline_query);

        // 更新基本信息
        let basic_query = sqlx::query::<MySql>(
            r#"
            UPDATE
                pipeline_basic
            SET
                update_time = ?, `name` = ?, path = ?, description = ?
            WHERE
                pipeline_id = ?
        "#,
        )
        .bind(update_time.clone())
        .bind(&basic.name)
        .bind(&basic.path)
        .bind(&basic.description)
        .bind(&pipeline.id); // 不给修改 tag
        query_list.push(basic_query);

        // 更新 pipeline_process 中的 update_time
        let pipeline_process_query = sqlx::query::<MySql>(
            r#"
            UPDATE pipeline_process SET update_time = ? WHERE pipeline_id = ?
        "#,
        )
        .bind(update_time.clone())
        .bind(&pipeline.id);
        query_list.push(pipeline_process_query);

        /*
        let data = Self::get_query_list(&pipeline, None).await;
        return match data {
            Ok(data) => {
                if data.is_empty() {
                    return Ok(get_error_response("更新流水线失败, 该流水线不存在"));
                }

                let line = data.iter().find(|s| &s.id == &pipeline.id);
                if line.is_none() {
                    return Ok(get_error_response("更新流水线失败, 该流水线不存在"));
                }

                // 判断流水线名字是否存在
                let line_by_name = data.clone().iter().find(|s| {
                    let b = &s.basic;
                    return &b.name == &pipeline.basic.name && &s.id != &pipeline.id;
                });

                if line_by_name.is_some() {
                    return Ok(get_error_response("更新流水线失败, 该流水线名字已存在"));
                }

                let mut line = line.unwrap().clone();
                line.update_time = Some(Utils::get_date(None)); // 更新时间
                line.basic = pipeline.basic.clone();
                line.variables = pipeline.variables.clone();
                line.process_config = pipeline.process_config.clone();

                /*
                let mut history_list = Vec::new();
                if let Some(pipe_run) = pipeline.run.clone() {
                    history_list = pipe_run.history_list
                }

                if let Some(mut run) = line.run.clone() {
                    let mut current = run.current.clone();
                    current.stages = pipeline.process_config.stages.clone();
                    run.current = current;
                    run.history_list = history_list;
                    line.run = Some(run.clone());
                }
                 */

                Self::update_pipeline(data, &line)
            }
            Err(_) => Ok(get_error_response("更新流水线失败")),
        };
         */
        return DBHelper::batch_commit(query_list).await;
    }

    /// 删除
    async fn delete(pipeline: &Self::B) -> Result<HttpResponse, String> {
        if pipeline.id.is_empty() {
            return Ok(get_error_response("删除流水线失败, `id` 不能为空"));
        }

        if pipeline.server_id.is_empty() {
            return Ok(get_error_response("删除流水线失败, `server_id` 不能为空"));
        }

        let id = &pipeline.id;
        let server_id = &pipeline.server_id;
        info!("delete pipeline id: {}, server_id: {}", &id, &server_id);

        let mut query_list: Vec<Query<MySql, MySqlArguments>> = Vec::new();
        query_list.push(sqlx::query::<MySql>("DELETE FROM pipeline WHERE id = ? and server_id = ?").bind(&id).bind(&server_id));
        Pipeline::delete_by_pipeline(&pipeline.id, &mut query_list);
        let response = DBHelper::batch_commit(query_list).await?;

        if response.code != 200 {
            return Ok(response);
        }

        // 删除流水线日志
        PipelineLogger::delete_log_by_id(&server_id, &id);
        return Ok(response);
    }

    /// 根据 ID 查找数据
    async fn get_by_id(pipeline: &Self::B) -> Result<HttpResponse, String> {
        if pipeline.id.is_empty() {
            return Ok(get_error_response("根据 ID 查找流水线失败, `id` 不能为空"));
        }

        if pipeline.server_id.is_empty() {
            return Ok(get_error_response("根据 ID 查找流水线失败, `server_id` 不能为空"));
        }

        info!("get pipeline by id: {}, server_id: {}", &pipeline.id, &pipeline.server_id);
        let response = Self::get_query_list(pipeline, None, true).await?;
        if response.code != 200 {
            return Ok(response);
        }

        let data: Vec<Pipeline> = serde_json::from_value(response.body).map_err(|err| Error::Error(err.to_string()).to_string())?;
        if data.is_empty() {
            return Ok(get_error_response("根据 ID 查找流水线失败, 该流水线不存在"));
        }

        let pipe = data.get(0).unwrap();
        let mut pipeline = pipe.clone();

        // 查询运行详情
        if let Some(last_run_id) = &pipe.last_run_id {
            let result = PipelineRunnable::get_runtime_detail(
                &pipeline,
                true,
                Some(PipelineRunnableQueryForm {
                    status_list: vec![],
                    runtime_id: Some(last_run_id.clone()),
                }),
            )
            .await?;
            pipeline.runtime = result.runtime.clone();
        }

        if let Some(mut runtime) = pipeline.runtime.clone() {
            let log = &runtime.log;
            if let Some(log) = log {
                let log_file = format!("{}.log", runtime.order.unwrap_or(1));
                let log_file_dir = Path::new(log).join(log_file);
                let log = PipelineLogger::read_log(log_file_dir)?;
                runtime.log = Some(log);
                pipeline.runtime = Some(runtime)
            }
        }

        let data = serde_json::to_value(pipeline.clone()).map_err(|err| Error::Error(err.to_string()).to_string())?;
        return Ok(get_success_response(Some(data)));
    }
}

impl Pipeline {
    /// 根据条件查询列表
    pub(crate) async fn get_query_list(pipeline: &Pipeline, query_form: Option<QueryForm>, need_get_child: bool) -> Result<HttpResponse, String> {
        let mut sql = String::from(
            r#"
            SELECT
                p.id as pipeline_id,
                p.server_id as pipeline_server_id,
                p.tag_id as pipeline_tag_id,
                p.last_run_time as pipeline_last_run_time,
                p.last_run_id as pipeline_last_run_id,
                p.`status` as pipeline_status,
                p.create_time as pipeline_create_time,
                p.update_time as pipeline_update_time,
                b.id as basic_id,
                b.pipeline_id as basic_pipeline_id,
                b.`name` as basic_name,
                b.path as basic_path,
                b.description as basic_description,
                b.create_time as basic_create_time,
                b.update_time as basic_update_time,
                t.`value` as tagValue,
                s.id as process_id,
                s.pipeline_id as process_pipeline_id,
                s.create_time as process_create_time,
                s.update_time as process_update_time,
                e.id as stage_id,
                e.process_id as stage_process_id,
		        CAST(e.`order` AS UNSIGNED) as stage_order,
                e.create_time as stage_create_time,
                e.update_time as stage_update_time,
                g.id as group_id,
                g.stage_id as group_stage_id,
                g.title as group_title,
                g.create_time as group_create_time,
                g.update_time as group_update_time,
                sp.id as step_id,
                sp.group_id as step_group_id,
                sp.module as step_module,
                sp.command as step_command,
                sp.label as step_label,
                sp.`status` as step_status,
                sp.create_time as step_create_time,
                sp.update_time as step_update_time,
                v.id as variable_id,
                v.pipeline_id as variable_pipeline_id,
                CAST(v.`order` AS UNSIGNED) as variable_order,
                v.`name` as variable_name,
                v.genre as variable_genre,
                v.`value` as variable_value,
                v.disabled as variable_disabled,
                v.`require` as variable_require,
                v.description as variable_description,
                v.create_time as variable_create_time,
                v.update_time as variable_update_time,
                c.id as step_component_id,
                c.step_id as step_component_step_id,
                c.prop as step_component_prop,
                c.label as step_component_label,
		        c.description as step_component_description,
                c.`value` as step_component_value,
                c.create_time as step_component_create_time,
		        c.update_time as step_component_update_time
            FROM
                 pipeline p
            LEFT JOIN pipeline_variable v on v.pipeline_id = p.id
            LEFT JOIN pipeline_basic b ON p.id = b.pipeline_id
            LEFT JOIN pipeline_tag t on t.id = p.tag_id
            LEFT JOIN pipeline_process s on p.id = s.pipeline_id
            LEFT JOIN pipeline_stage e on e.process_id = s.id
            LEFT JOIN pipeline_group g on g.stage_id = e.id
            LEFT JOIN pipeline_step sp on sp.group_id = g.id
            LEFT JOIN pipeline_step_component c on c.step_id = sp.id
            WHERE 1 = 1
        "#,
        );

        if !pipeline.server_id.is_empty() {
            sql.push_str(&format!(" AND p.server_id = '{}' ", pipeline.server_id));
        }

        if !pipeline.id.is_empty() {
            sql.push_str(&format!(" AND p.id = '{}'", pipeline.id))
        }

        if query_form.is_some() {
            let form = query_form.unwrap();
            let name = &form.name;
            let status = &form.status;

            if !name.is_empty() {
                sql.push_str(&format!(" AND b.`name` like '%{}%'", name))
            }

            if !status.is_empty() {
                sql.push_str(&format!(" AND p.`status` = '{}'", status))
            }
        }

        // group by and order by
        sql.push_str(
            r#"
            GROUP BY
                    p.id, b.id, t.`value`, s.id, e.id, g.id, sp.id, c.id, v.id, v.`order`, v.`name`, v.genre, v.`value`, v.disabled, v.`require`, v.description, v.create_time, v.update_time
            ORDER BY
            CASE
                WHEN
                     p.update_time IS NULL or p.update_time = ''
                THEN
                     0
                ELSE
                     1
            END DESC,
            p.update_time DESC,
            p.create_time DESC
        "#,
        );

        let query = sqlx::query(&sql);

        let rows = DBHelper::execute_rows(query).await?;
        if rows.is_empty() {
            return Ok(get_success_response(Some(Value::Array(Vec::new()))));
        }

        // 组装数据
        let mut variable_map: HashMap<String, PipelineVariable> = HashMap::new();
        let mut map: HashMap<String, Pipeline> = HashMap::new();
        let mut process_map: HashMap<String, PipelineProcess> = HashMap::new();
        let mut stage_map: HashMap<String, PipelineStage> = HashMap::new();
        let mut group_map: HashMap<String, PipelineGroup> = HashMap::new();
        let mut step_map: HashMap<String, PipelineStep> = HashMap::new();
        let mut step_component_map: HashMap<String, PipelineStepComponent> = HashMap::new();

        for row in rows.iter() {
            let pipeline_id = row.try_get("pipeline_id").unwrap_or(String::new());
            let status_str: String = row.try_get("pipeline_status").unwrap_or(String::new());
            let tag_str: String = row.try_get("tagValue").unwrap_or(String::new());
            let tag = Some(PipelineTag::get(&tag_str));

            let basic = PipelineBasic {
                id: row.try_get("basic_id").unwrap_or(String::new()),
                pipeline_id: row.try_get("basic_pipeline_id").unwrap_or(None),
                name: row.try_get("basic_name").unwrap_or(String::new()),
                tag: tag.clone().unwrap(),
                path: row.try_get("basic_path").unwrap_or(String::new()),
                description: row.try_get("basic_description").unwrap_or(String::new()),
                create_time: row.try_get("basic_create_time").unwrap_or(None),
                update_time: row.try_get("basic_update_time").unwrap_or(None),
            };

            // pipeline
            map.entry(pipeline_id.clone()).or_insert_with(|| Pipeline {
                id: pipeline_id.to_string(),
                server_id: row.try_get("pipeline_server_id").unwrap_or(String::new()),
                last_run_time: row.try_get("pipeline_last_run_time").unwrap_or(None),
                last_run_id: row.try_get("pipeline_last_run_id").unwrap_or(None),
                tag,
                status: Some(PipelineStatus::get(&status_str)),
                basic,
                process_config: Default::default(),
                variables: vec![],
                runnable_info: None,
                runtime: None,
                create_time: row.try_get("pipeline_create_time").unwrap_or(None),
                update_time: row.try_get("pipeline_update_time").unwrap_or(None),
            });

            // variables
            let variable_id = row.try_get("variable_id").unwrap_or(String::new());
            variable_map.entry(variable_id.clone()).or_insert_with(|| PipelineVariable {
                id: variable_id.to_string(),
                pipeline_id: row.try_get("pipeline_id").unwrap_or(None),
                order: row.try_get("variable_order").unwrap_or(0),
                name: row.try_get("variable_name").unwrap_or(String::new()),
                genre: row.try_get("variable_genre").unwrap_or(String::new()),
                value: row.try_get("variable_value").unwrap_or(String::new()),
                disabled: row.try_get("variable_disabled").unwrap_or(String::new()),
                require: row.try_get("variable_require").unwrap_or(String::new()),
                description: row.try_get("variable_description").unwrap_or(String::new()),
                create_time: row.try_get("variable_create_time").unwrap_or(None),
                update_time: row.try_get("variable_update_time").unwrap_or(None),
            });

            // process
            let process_id = row.try_get("process_id").unwrap_or(String::new());
            process_map.entry(process_id.clone()).or_insert_with(|| PipelineProcess {
                id: process_id.to_string(),
                pipeline_id: row.try_get("process_pipeline_id").unwrap_or(String::new()),
                stages: vec![],
                create_time: row.try_get("process_create_time").unwrap_or(None),
                update_time: row.try_get("process_update_time").unwrap_or(None),
            });

            // stage
            let stage_id = row.try_get("stage_id").unwrap_or(String::new());
            stage_map.entry(stage_id.clone()).or_insert_with(|| PipelineStage {
                id: stage_id.to_string(),
                process_id: row.try_get("stage_process_id").unwrap_or(String::new()),
                order: row.try_get("stage_order").unwrap_or(0),
                groups: vec![],
                create_time: row.try_get("stage_create_time").unwrap_or(None),
                update_time: row.try_get("stage_update_time").unwrap_or(None),
            });

            // group
            let group_id = row.try_get("group_id").unwrap_or(String::new());
            group_map.entry(group_id.clone()).or_insert_with(|| PipelineGroup {
                id: group_id.to_string(),
                stage_id: row.try_get("group_stage_id").unwrap_or(String::new()),
                title: row.try_get("group_title").unwrap_or(String::new()),
                steps: vec![],
                create_time: row.try_get("group_create_time").unwrap_or(None),
                update_time: row.try_get("group_update_time").unwrap_or(None),
            });

            // step
            let step_id = row.try_get("step_id").unwrap_or(String::new());
            let step_status_str: String = row.try_get("step_status").unwrap_or(String::new());
            let step_module_str: String = row.try_get("step_module").unwrap_or(String::new());
            step_map.entry(step_id.clone()).or_insert_with(|| PipelineStep {
                id: step_id.to_string(),
                group_id: row.try_get("step_group_id").unwrap_or(String::new()),
                module: PipelineCommandStatus::get(&step_module_str),
                command: row.try_get("step_command").unwrap_or(String::new()),
                label: row.try_get("step_label").unwrap_or(String::new()),
                status: PipelineStatus::get(&step_status_str),
                components: vec![],
                create_time: row.try_get("step_create_time").unwrap_or(None),
                update_time: row.try_get("step_update_time").unwrap_or(None),
            });

            // step component
            let step_component_id = row.try_get("step_component_id").unwrap_or(String::new());
            step_component_map.entry(step_component_id.clone()).or_insert_with(|| PipelineStepComponent {
                id: step_component_id.to_string(),
                step_id: row.try_get("step_component_step_id").unwrap_or(String::new()),
                prop: row.try_get("step_component_prop").unwrap_or(String::new()),
                label: row.try_get("step_component_label").unwrap_or(String::new()),
                description: row.try_get("step_component_description").unwrap_or(String::new()),
                value: row.try_get("step_component_value").unwrap_or(String::new()),
                create_time: row.try_get("step_component_create_time").unwrap_or(None),
                update_time: row.try_get("step_component_update_time").unwrap_or(None),
            });
        }

        // step component
        for step_component_id in step_component_map.keys() {
            let step_component = step_component_map.get(step_component_id);
            if let Some(step_component) = step_component {
                let step = step_map.get_mut(&step_component.step_id);
                if let Some(step) = step {
                    step.components.push(step_component.clone());
                }
            }
        }

        // step
        for step_id in step_map.keys() {
            let step = step_map.get(step_id);
            if let Some(step) = step {
                let group = group_map.get_mut(&step.group_id);
                if let Some(group) = group {
                    group.steps.push(step.clone());
                }
            }
        }

        // group
        for group_id in group_map.keys() {
            let group = group_map.get(group_id);
            if let Some(group) = group {
                let stage = stage_map.get_mut(&group.stage_id);
                if let Some(stage) = stage {
                    stage.groups.push(group.clone());
                }
            }
        }

        // process
        for stage_id in stage_map.keys() {
            let stage = stage_map.get(stage_id);
            if let Some(stage) = stage {
                let process = process_map.get_mut(&stage.process_id);
                if let Some(process) = process {
                    process.stages.push(stage.clone());
                }
            }
        }

        // pipeline
        for process_id in process_map.keys() {
            let process = process_map.get(process_id);
            if let Some(process) = process {
                let pipe = map.get_mut(&process.pipeline_id);
                if let Some(pipe) = pipe {
                    pipe.process_config = process.clone()
                }
            }
        }

        // variables
        for variable_id in variable_map.keys() {
            let variable = variable_map.get(variable_id);
            if let Some(variable) = variable {
                let pipe = map.get_mut(&variable.pipeline_id.clone().unwrap_or(String::new()));
                if let Some(pipe) = pipe {
                    pipe.variables.push(variable.clone())
                }
            }
        }

        let mut list: Vec<Pipeline> = map.into_values().collect();

        let installed_commands = H5FileHandler::get_installed_commands();
        // node 版本
        let node = Helper::get_cmd_version("node");

        if need_get_child {
            for pipe in list.iter_mut() {
                let basic = &pipe.basic;
                let runnable_info = Self::get_runnable_variable(&basic, installed_commands.clone(), &node);
                pipe.runnable_info = Some(runnable_info);
                if let Some(last_run_id) = &pipe.last_run_id {
                    let result = PipelineRunnable::get_runtime_detail(
                        &pipe,
                        true,
                        Some(PipelineRunnableQueryForm {
                            status_list: vec![],
                            runtime_id: Some(last_run_id.clone()),
                        }),
                    )
                    .await?;
                    pipe.runtime = result.runtime;
                }
            }
        }

        get_success_response_by_value(list)
    }

    fn insert_stages(process_config: &PipelineProcess, process_id: String, create_time: String, query_list: &mut Vec<Query<MySql, MySqlArguments>>) {
        fn insert_step_components(components: &Vec<PipelineStepComponent>, step_id: String, create_time: String, query_list: &mut Vec<Query<MySql, MySqlArguments>>) {
            if components.is_empty() {
                return;
            }

            for component in components.iter() {
                let step_component_id = Uuid::new_v4().to_string();
                let step_component_query = sqlx::query::<MySql>(
                    r#"
                            INSERT INTO pipeline_step_component (id, step_id, prop, label, `value`, description, create_time, update_time)
                            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
                        "#,
                )
                .bind(step_component_id.clone())
                .bind(step_id.clone())
                .bind(component.prop.clone())
                .bind(component.label.clone())
                .bind(component.value.clone())
                .bind(component.description.clone())
                .bind(create_time.clone())
                .bind(component.update_time.clone());
                query_list.push(step_component_query);
            }
        }

        fn insert_steps(steps: &Vec<PipelineStep>, group_id: String, create_time: String, query_list: &mut Vec<Query<MySql, MySqlArguments>>) {
            if steps.is_empty() {
                return;
            }

            for step in steps.iter() {
                let step_id = Uuid::new_v4().to_string();
                let step_query = sqlx::query::<MySql>(
                    r#"
            INSERT INTO pipeline_step (id, group_id, module, command, label, `status`, create_time, update_time)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
        "#,
                )
                .bind(step_id.clone())
                .bind(group_id.clone())
                .bind(PipelineCommandStatus::got(step.module.clone()))
                .bind(step.command.clone())
                .bind(step.label.clone())
                .bind(PipelineStatus::got(step.status.clone()))
                .bind(create_time.clone())
                .bind(step.update_time.clone());
                query_list.push(step_query);

                // 5. 流水线步骤组件, 插入 pipeline_step_component 表
                insert_step_components(&step.components, step_id.clone(), create_time.clone(), query_list);
            }
        }

        fn insert_groups(groups: &Vec<PipelineGroup>, stage_id: String, create_time: String, query_list: &mut Vec<Query<MySql, MySqlArguments>>) {
            if groups.is_empty() {
                return;
            }

            for group in groups.iter() {
                let group_id = Uuid::new_v4().to_string();
                let group_query = sqlx::query::<MySql>(
                    r#"
            INSERT INTO pipeline_group (id, stage_id, title, create_time, update_time)
            VALUES (?, ?, ?, ?, ?)
        "#,
                )
                .bind(group_id.clone())
                .bind(stage_id.clone())
                .bind(group.title.clone())
                .bind(create_time.clone())
                .bind(group.update_time.clone());
                query_list.push(group_query);

                // 4. 流水线步骤, 插入 pipeline_step 表
                insert_steps(&group.steps, group_id.clone(), create_time.clone(), query_list)
            }
        }

        let stages = &process_config.stages;
        if !stages.is_empty() {
            for (usize, stage) in stages.iter().enumerate() {
                let stage_id = Uuid::new_v4().to_string();
                let stage_query = sqlx::query::<MySql>(
                    r#"
            INSERT INTO pipeline_stage (id, process_id, `order`, create_time, update_time)
            VALUES (?, ?, ?, ?, ?)
        "#,
                )
                .bind(stage_id.clone())
                .bind(process_id.clone())
                .bind(format!("{}", usize as u32 + 1))
                .bind(create_time.clone())
                .bind(process_config.update_time.clone());
                query_list.push(stage_query);

                // 3. 流水线分组, 插入 pipeline_group 表
                insert_groups(&stage.groups, stage_id.clone(), create_time.clone(), query_list);
            }
        }
    }

    fn insert_variables(pipeline_id: String, create_time: String, update_time: Option<String>, variables: &Vec<PipelineVariable>, query_list: &mut Vec<Query<MySql, MySqlArguments>>) {
        for variable in variables.iter() {
            let variable_query = sqlx::query::<MySql>(
                r#"
            INSERT INTO pipeline_variable (
                id, pipeline_id, `order`, `name`, genre, `value`, disabled, `require`, description, create_time, update_time
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
            )
            .bind(Uuid::new_v4().to_string().clone())
            .bind(pipeline_id.clone())
            .bind(variable.order.clone())
            .bind(variable.name.clone())
            .bind(variable.genre.clone())
            .bind(variable.value.clone())
            .bind(variable.disabled.clone())
            .bind(variable.require.clone())
            .bind(variable.description.clone())
            .bind(create_time.clone())
            .bind(update_time.clone());
            query_list.push(variable_query);
        }
    }

    /// 数据检查
    fn validate(pipeline: &Pipeline) -> Option<HttpResponse> {
        let basic = &pipeline.basic;
        let process_config = &pipeline.process_config;
        let variables = &pipeline.variables;

        if pipeline.server_id.is_empty() {
            return Some(get_error_response("更新流水线失败, `server_id` 不能为空"));
        }

        if PipelineBasic::is_empty(basic) {
            return Some(get_error_response("更新流水线失败, `basic` 中 `ip`、 `tag` or `path` 不能为空"));
        }

        if PipelineProcess::is_empty(process_config) {
            return Some(get_error_response("更新流水线失败, `process_config` 中 `steps` 不能为空"));
        }

        // 判断路径是否存在(本地路径)
        let path = &basic.path;
        if GitHandler::is_remote_url(path) {
            let validate_success = GitHandler::validate_remote_url(path);
            if !validate_success {
                return Some(get_error_response("更新流水线失败, `在线项目路径` 不存在"));
            }
        } else {
            if !PathBuf::from(&basic.path).exists() {
                return Some(get_error_response("更新流水线失败, `项目路径` 不存在"));
            }
        }

        // 检查每个 variable 字段
        if !variables.is_empty() {
            for variable in variables.iter() {
                if PipelineVariable::is_empty(variable) {
                    return Some(get_error_response("更新流水线失败, `variable` 中 `name`、 `genre`、 `value`、 `disabled`  or `require` 不能为空"));
                }
            }
        }

        return None;
    }

    /// 获取运行时的变量
    fn get_runnable_variable(basic: &PipelineBasic, installed_commands: Vec<String>, node: &str) -> RunnableVariable {
        // branch
        let branches = GitHandler::get_branch_list(&basic.path);

        // h5
        let mut h5_variable: Option<H5RunnableVariable> = None;
        match basic.tag {
            PipelineTag::None => {}
            PipelineTag::Develop => {}
            PipelineTag::Test => {}
            PipelineTag::CAddAdd => {}
            PipelineTag::Rust => {}
            PipelineTag::Java => {}
            PipelineTag::Android => {}
            PipelineTag::Ios => {}
            PipelineTag::H5 => {
                h5_variable = Self::get_h5_runnable_variable(&basic.path, installed_commands, &node, branches.clone());
            }
        }

        return RunnableVariable {
            branches,
            h5: h5_variable,
            is_remote_url: GitHandler::is_remote_url(&basic.path),
        };
    }

    /// 获取附加的 H5 变量
    fn get_h5_runnable_variable(url: &str, installed_commands: Vec<String>, node: &str, _: Vec<String>) -> Option<H5RunnableVariable> {
        let mut variable = H5FileHandler::get_default_file_commands(url);
        if let Some(variables) = variable.as_mut() {
            variables.node = node.to_string();
            variables.installed_commands = installed_commands;
            return Some(variables.clone());
        }

        let mut h5_variables = H5RunnableVariable::default();
        h5_variables.node = node.to_string();
        h5_variables.installed_commands = installed_commands;

        // 根据 branches 获取所有的 package.json 或 Makefile 文件内容
        /*
        branches.iter().for_each(|branch| {
            H5FileHandler::get_file_by_branch(branch, url, "package.json").unwrap();
        });
         */

        return Some(h5_variables);
    }

    /// 查询系统已安装的 commands 列表
    pub(crate) fn query_os_commands() -> Result<HttpResponse, String> {
        let h5_installed_commands = H5FileHandler::get_installed_commands();
        get_success_response_by_value(OsCommands { h5_installed_commands })
    }

    /// 清空运行历史, 删除流水线运行日志记录
    pub(crate) async fn clear_run_history(pipeline: &Pipeline) -> Result<HttpResponse, String> {
        let res = Pipeline::get_by_id(&pipeline).await?;
        let pipeline: Pipeline = serde_json::from_value(res.body.clone()).map_err(|err| Error::Error(err.to_string()).to_string())?;

        let response = PipelineRunnable::delete_runtime(&pipeline.id).await?;
        if response.code != 200 {
            return Ok(response);
        }

        // 删除流水线日志
        let bool = PipelineLogger::delete_log_by_id(&pipeline.server_id, &pipeline.id);
        if bool {
            info!("clear run history success !");
            return Pipeline::get_by_id(&pipeline).await;
        } else {
            info!("clear run history failed, can not delete log files !");
            Ok(get_error_response("清除运行历史失败, 无法删除日志文件"))
        }
    }

    pub(crate) async fn get_pipeline_list(pipeline: &Pipeline, query_form: Option<QueryForm>, need_get_child: bool) -> Result<Vec<Pipeline>, String> {
        let response = Pipeline::get_query_list(&pipeline, query_form, need_get_child).await?;
        if response.code != 200 {
            error!("get pipeline list error: {:#?}", response.error);
            return Ok(Vec::new());
        }

        let list: Vec<Pipeline> = serde_json::from_value(response.body).map_err(|err| Error::Error(err.to_string()).to_string())?;
        return Ok(list);
    }

    /// 根据 server_id 查找 流水线
    pub(crate) async fn get_pipeline_list_by_server_id(server_id: &str) -> Result<Vec<Pipeline>, String> {
        let query = sqlx::query_as::<_, Pipeline>("select * from pipeline where server_id = ?").bind(server_id);
        let response = DBHelper::execute_query(query).await?;
        if response.code != 200 {
            error!("get pipeline list by server_id: {} error: {}", server_id, &response.error);
            return Ok(Vec::new());
        }

        return serde_json::from_value(response.body).map_err(|err| Error::Error(err.to_string()).to_string())?;
    }

    pub(crate) fn delete_by_pipeline(pipeline_id: &str, query_list: &mut Vec<Query<MySql, MySqlArguments>>) {
        // 删除 variables 并重新插入
        let variable_delete_query = sqlx::query::<MySql>(
            r#"
            DELETE FROM pipeline_variable WHERE pipeline_id = ?
        "#,
        )
        .bind(pipeline_id.to_string().clone());
        query_list.push(variable_delete_query);

        // 删除 step_component
        let step_component_delete_query = sqlx::query::<MySql>(
            r#"
            DELETE FROM pipeline_step_component
            WHERE step_id IN (
                SELECT id FROM pipeline_step
                WHERE group_id IN (
                    SELECT g.id FROM pipeline_group g
                    WHERE g.stage_id IN (
                        SELECT s.id FROM pipeline_stage s
                        WHERE s.process_id IN (
                            SELECT p.id
                            FROM pipeline_process p
                            WHERE p.pipeline_id = ?
                        )
                    )
                )
            )
        "#,
        )
        .bind(pipeline_id.to_string().clone());
        query_list.push(step_component_delete_query);

        // 删除 step
        let step_delete_query = sqlx::query::<MySql>(
            r#"
            DELETE FROM pipeline_step
            WHERE group_id IN (
                SELECT g.id FROM pipeline_group g
                WHERE g.stage_id IN (
                    SELECT s.id FROM pipeline_stage s
                    WHERE s.process_id IN (
                        SELECT p.id
                        FROM pipeline_process p
                        WHERE p.pipeline_id = ?
                    )
                )
            )
        "#,
        )
        .bind(pipeline_id.to_string().clone());
        query_list.push(step_delete_query);

        // 删除 group
        let group_delete_query = sqlx::query::<MySql>(
            r#"
            DELETE FROM pipeline_group
            WHERE stage_id IN (
                SELECT s.id FROM pipeline_stage s
                WHERE s.process_id IN (
                    SELECT p.id
                    FROM pipeline_process p
                    WHERE p.pipeline_id = ?
                )
            )
        "#,
        )
        .bind(pipeline_id.to_string().clone());
        query_list.push(group_delete_query);

        // 删除 stage
        let stage_delete_query = sqlx::query::<MySql>(
            r#"
            DELETE FROM pipeline_stage
            WHERE process_id IN (
                SELECT p.id
                FROM pipeline_process p
                WHERE p.pipeline_id = ?
            );
        "#,
        )
        .bind(pipeline_id.to_string().clone());
        query_list.push(stage_delete_query);
    }
}
