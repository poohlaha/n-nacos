//! 流水线运行

pub(crate) mod stage;

use crate::database::helper::DBHelper;
use crate::error::Error;
use crate::event::EventEmitter;
use crate::helper::git::GitHandler;
use crate::logger::pipeline::PipelineLogger;
use crate::prepare::{get_error_response, get_success_response, get_success_response_by_value, HttpResponse};
use crate::server::pipeline::index::Pipeline;
use crate::server::pipeline::pool::Pool;
use crate::server::pipeline::props::{PipelineBasic, PipelineRuntime, PipelineRuntimeSnapshot, PipelineRuntimeStage, PipelineRuntimeVariable, PipelineStage, PipelineStatus, PipelineTag};
use handlers::utils::Utils;
use lazy_static::lazy_static;
use log::{error, info};
use sqlx::mysql::MySqlArguments;
use sqlx::query::Query;
use sqlx::{MySql, Row};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tauri::AppHandle;
use uuid::Uuid;

// 共享 pipeline 数据
lazy_static! {
    static ref PIPELINE: Arc<Mutex<Option<Pipeline>>> = Arc::new(Mutex::new(None));
}
pub struct PipelineRunnable;

pub struct PipelineRunnableQueryForm {
    pub(crate) status_list: Vec<String>,
    pub(crate) runtime_id: Option<String>,
    pub(crate) pipeline_id: Option<String>,
}

impl PipelineRunnable {
    /// 删除运行记录
    pub(crate) async fn delete_runtime(pipeline_id: &str) -> Result<HttpResponse, String> {
        let mut pipeline = Pipeline::default();
        pipeline.id = pipeline_id.to_string();

        let response = Self::get_runtime_detail(
            &pipeline,
            true,
            Some(PipelineRunnableQueryForm {
                status_list: vec![],
                runtime_id: None,
                pipeline_id: Some(pipeline_id.to_string().clone()),
            }),
        )
        .await?;
        if response.code != 200 {
            return Ok(response);
        }

        let mut query_list: Vec<Query<MySql, MySqlArguments>> = Vec::new();
        query_list.push(sqlx::query::<MySql>("DELETE FROM pipeline_runtime WHERE pipeline_id = ?").bind(pipeline_id));

        // 删除 pipeline_runtime_snapshot
        let snapshot_sql = String::from(
            r#"
            DELETE FROM
                pipeline_runtime_snapshot
            WHERE runtime_id IN (
                SELECT r.id
                FROM pipeline_runtime r
                WHERE r.pipeline_id = ?
            )
        "#,
        );
        query_list.push(sqlx::query::<MySql>(&snapshot_sql).bind(&pipeline_id));

        // 删除 pipeline_runtime_variable
        let variable_sql = String::from(
            r#"
            DELETE FROM pipeline_runtime_variable
            WHERE runtime_id = IN (
                SELECT r.id
                FROM pipeline_runtime r
                WHERE r.pipeline_id = ?
            )
        "#,
        );
        query_list.push(sqlx::query::<MySql>(&variable_sql).bind(&pipeline_id));
        DBHelper::batch_commit(query_list).await
    }

    /// 获取运行历史记录
    pub(crate) async fn get_runtime_list(query_form: Option<PipelineRunnableQueryForm>) -> Result<HttpResponse, String> {
        Self::get_runtime_detail(&Pipeline::default(), false, query_form).await
    }

    /// 获取运行历史记录
    pub(crate) async fn get_runtime_history(pipeline: &Pipeline) -> Result<HttpResponse, String> {
        if pipeline.id.is_empty() {
            return Ok(get_error_response("查询历史记录失败, `pipelineId` 为空"));
        }

        if pipeline.server_id.is_empty() {
            return Ok(get_error_response("查询历史记录失败, `serverId` 为空"));
        }

        Self::get_runtime_detail(pipeline, false, None).await
    }

    /// 获取运行详情
    pub(crate) async fn get_runtime_detail(pipeline: &Pipeline, only_one: bool, query_form: Option<PipelineRunnableQueryForm>) -> Result<HttpResponse, String> {
        let mut sql = String::from("");

        // 取最新一条
        if only_one {
            sql.push_str(
                r#"
             WITH LatestRuntime AS (
                    SELECT
                        r.pipeline_id,
                        MAX(r.create_time) AS latest_create_time
                    FROM
                        pipeline_runtime r
                    GROUP BY
                        r.pipeline_id
                )
            "#,
            )
        }

        sql.push_str(
            r#"
                SELECT
                    r.id AS runtime_id,
                    r.pipeline_id AS runtime_pipeline_id,
                    r.tag AS runtime_tag,
                    r.project_name as runtime_project_name,
                    CAST( r.`order` AS UNSIGNED ) AS runtime_order,
                    r.basic as runtime_basic,
	                r.stages as runtime_stages,
                    r.`status` AS runtime_status,
                    r.start_time AS runtime_start_time,
                    r.remark as runtime_remark,
                    r.duration AS runtime_duration,
                    CAST( r.stage_index AS UNSIGNED ) AS runtime_stage_index,
                    CAST( r.group_index AS UNSIGNED ) AS runtime_group_index,
                    CAST( r.step_index AS UNSIGNED ) AS runtime_step_index,
                    r.finished AS runtime_finished,
                    r.log AS runtime_log,
                    r.create_time as runtime_create_time,
                    r.update_time as runtime_update_time,
                    s.id as runtime_snapshot_id,
                    s.runtime_id as runtime_snapshot_runtime_id,
                    s.node as runtime_snapshot_node,
                    s.branch as runtime_snapshot_branch,
                    s.make as runtime_snapshot_make,
                    s.command as runtime_snapshot_command,
                    s.script as runtime_snapshot_script,
                    s.create_time as runtime_snapshot_create_time,
                    s.update_time as runtime_snapshot_update_time,
                    v.id as runtime_variable_id,
                    v.snapshot_id as runtime_variable_snapshot_id,
                    CAST( v.`order` AS UNSIGNED ) AS runtime_variable_order,
                    v.`name` as runtime_variable_name,
                    v.`value` as runtime_variable_value,
                    v.description as runtime_variable_description,
                    v.create_time as runtime_variable_create_time,
                    v.update_time as runtime_variable_update_time
                FROM
                    pipeline_runtime r
                    LEFT JOIN pipeline_runtime_snapshot s ON r.id = s.runtime_id
                    LEFT JOIN pipeline_runtime_variable v ON s.id = v.snapshot_id
                WHERE
                   1 = 1
         "#,
        );

        if !pipeline.id.is_empty() {
            sql.push_str(&format!(" AND r.pipeline_id = '{}'", pipeline.id));
        }

        if let Some(query_form) = query_form {
            // status
            let status_list = query_form.status_list.clone();
            if status_list.len() > 0 {
                let mut status_condition_sql = String::from(" AND (");
                for (size, s) in status_list.iter().enumerate() {
                    status_condition_sql.push_str(&format!(" r.status = '{}'", s));
                    if size != status_list.len() - 1 {
                        status_condition_sql.push_str(" OR ");
                    }
                }
                status_condition_sql.push_str(")");
                sql.push_str(&status_condition_sql);
            }

            // runtime_id
            if let Some(runtime_id) = query_form.runtime_id.clone() {
                sql.push_str(&format!(" r.id = '{}'", runtime_id));
            }

            // pipeline_id
            if let Some(pipeline_id) = query_form.pipeline_id.clone() {
                sql.push_str(&format!(" r.pipeline_id = '{}'", pipeline_id));
            }
        }

        let query = sqlx::query(&sql);
        let rows = DBHelper::execute_rows(query).await?;
        if rows.is_empty() {
            return Ok(get_success_response(None));
        }

        // 组装数据
        let mut map: HashMap<String, PipelineRuntime> = HashMap::new();
        let mut snapshot_map: HashMap<String, PipelineRuntimeSnapshot> = HashMap::new();
        let mut variable_map: HashMap<String, PipelineRuntimeVariable> = HashMap::new();
        for row in rows.iter() {
            let runtime_id = row.try_get("runtime_id").unwrap_or(String::new());
            let tag_str: String = row.try_get("runtime_tag").unwrap_or(String::new());
            let status_str = row.try_get("runtime_status").unwrap_or(String::new());
            let finished_str = row.try_get("runtime_finished").unwrap_or(String::new());
            let finished = finished_str.trim().parse::<bool>().unwrap_or_else(|err| {
                error!("convert field `finished` to bool error: {:#?}", err);
                false
            });

            let basic_str = row.try_get("runtime_basic").unwrap_or(String::new());
            let stages_str = row.try_get("runtime_stages").unwrap_or(String::new());
            let basic: PipelineBasic = serde_json::from_str(&basic_str).unwrap_or(PipelineBasic::default());
            let stages: Vec<PipelineStage> = serde_json::from_str(&stages_str).unwrap_or(Vec::new());

            map.entry(runtime_id.clone()).or_insert_with(|| PipelineRuntime {
                id: Some(runtime_id.clone()),
                pipeline_id: row.try_get("runtime_pipeline_id").unwrap_or(String::new()),
                server_id: pipeline.server_id.clone(),
                tag: PipelineTag::get(&tag_str),
                project_name: row.try_get("runtime_project_name").unwrap_or(None),
                order: row.try_get("runtime_order").unwrap_or(None),
                basic: Some(basic),
                stage: PipelineRuntimeStage {
                    stage_index: row.try_get("runtime_stage_index").unwrap_or(1),
                    group_index: row.try_get("runtime_group_index").unwrap_or(0),
                    step_index: row.try_get("runtime_step_index").unwrap_or(0),
                    finished,
                },
                stages,
                status: PipelineStatus::get(&status_str),
                start_time: row.try_get("runtime_start_time").unwrap_or(None),
                duration: row.try_get("runtime_duration").unwrap_or(None),
                snapshot: Default::default(),
                log: row.try_get("runtime_log").unwrap_or(None),
                remark: row.try_get("runtime_remark").unwrap_or(String::new()),
                create_time: row.try_get("runtime_create_time").unwrap_or(None),
                update_time: row.try_get("runtime_update_time").unwrap_or(None),
            });

            // snapshot
            let snapshot_id = row.try_get("runtime_snapshot_id").unwrap_or(String::new());
            snapshot_map.entry(snapshot_id.clone()).or_insert_with(|| PipelineRuntimeSnapshot {
                id: Some(snapshot_id.clone()),
                runtime_id: row.try_get("runtime_snapshot_runtime_id").unwrap_or(String::new()),
                node: row.try_get("runtime_snapshot_node").unwrap_or(String::new()),
                branch: row.try_get("runtime_snapshot_branch").unwrap_or(String::new()),
                make: row.try_get("runtime_snapshot_make").unwrap_or(None),
                command: row.try_get("runtime_snapshot_command").unwrap_or(String::new()),
                script: row.try_get("runtime_snapshot_script").unwrap_or(String::new()),
                runnable_variables: vec![],
                create_time: row.try_get("runtime_snapshot_create_time").unwrap_or(None),
                update_time: row.try_get("runtime_snapshot_update_time").unwrap_or(None),
            });

            // variable
            let variable_id = row.try_get("runtime_variable_id").unwrap_or(String::new());
            variable_map.entry(variable_id.clone()).or_insert_with(|| PipelineRuntimeVariable {
                id: Some(variable_id.clone()),
                snapshot_id: row.try_get("runtime_variable_snapshot_id").unwrap_or(None),
                order: row.try_get("runtime_variable_order").unwrap_or(0),
                name: row.try_get("runtime_variable_name").unwrap_or(String::new()),
                value: row.try_get("runtime_variable_value").unwrap_or(String::new()),
                description: row.try_get("runtime_variable_description").unwrap_or(String::new()),
                create_time: row.try_get("runtime_variable_create_time").unwrap_or(None),
                update_time: row.try_get("runtime_variable_update_time").unwrap_or(None),
            });
        }

        // variable
        for variable_id in variable_map.keys() {
            let variable = variable_map.get(variable_id);
            if let Some(variable) = variable {
                let snapshot_id = &variable.snapshot_id;
                if let Some(snapshot_id) = snapshot_id {
                    let snapshot = snapshot_map.get_mut(&snapshot_id.clone());
                    if let Some(snapshot) = snapshot {
                        snapshot.runnable_variables.push(variable.clone());
                    }
                }
            }
        }

        // snapshot
        for snapshot_id in snapshot_map.keys() {
            let snapshot = snapshot_map.get(snapshot_id);
            if let Some(snapshot) = snapshot {
                let runtime_id = &snapshot.runtime_id;
                let runtime = map.get_mut(&runtime_id.clone());
                if let Some(runtime) = runtime {
                    runtime.snapshot = snapshot.clone();
                }
            }
        }

        let list: Vec<PipelineRuntime> = map.into_values().collect();
        if list.len() == 0 {
            return Ok(get_success_response(None));
        }

        if only_one {
            if let Some(runtime) = list.get(0) {
                return get_success_response_by_value(runtime.clone());
            }
        }

        return get_success_response_by_value(list);
    }

    /// 添加到线程池中
    pub(crate) async fn exec(props: &PipelineRuntime) -> Result<HttpResponse, String> {
        if props.pipeline_id.is_empty() {
            return Ok(get_error_response("运行流水线失败, `pipelineId` 不能为空"));
        }

        if props.server_id.is_empty() {
            return Ok(get_error_response("运行流水线失败, `serverId` 不能为空"));
        }

        let mut pipeline = Pipeline::default();
        pipeline.id = props.pipeline_id.clone();
        pipeline.server_id = props.server_id.clone();

        // 查询流水线是否存在
        info!("get pipeline by id: {}, server_id: {}", &pipeline.id, &pipeline.server_id);
        let pipeline_list: Vec<Pipeline> = Pipeline::get_pipeline_list(&pipeline, None).await?;
        if pipeline_list.is_empty() {
            return Ok(get_error_response("运行流水线失败, 该流水线不存在"));
        }

        if pipeline_list.len() > 1 {
            return Ok(get_error_response("运行流水线失败, 存在多条相同的流水线"));
        }

        // 查询流水线是不是在排队状态或执行状态
        let response = PipelineRunnable::get_runtime_list(Some(PipelineRunnableQueryForm {
            status_list: vec![PipelineStatus::got(PipelineStatus::Queue), PipelineStatus::got(PipelineStatus::Process)],
            runtime_id: None,
            pipeline_id: None,
        }))
        .await?;
        if response.code != 200 {
            return Ok(response);
        }

        let runtime_list: Vec<PipelineRuntime> = serde_json::from_value(response.body.clone()).map_err(|err| Error::Error(err.to_string()).to_string())?;
        if runtime_list.len() > 0 {
            return Ok(get_error_response("该流水线已在运行状态, 请等待运行完成"));
        }

        let pipe = pipeline_list.get(0).unwrap();

        // 查询 pipeline_runtime order
        let runtime_order_query = sqlx::query(
            r#"
            select MAX(CAST(`order` AS UNSIGNED)) AS max_order FROM pipeline_runtime WHERE pipeline_id = ?
          "#,
        )
        .bind(&pipe.id);

        let mut order: u32 = 1;
        let rows = DBHelper::execute_rows(runtime_order_query).await?;
        info!("max order rows: {:#?}", rows);
        if !rows.is_empty() {
            let row = rows.get(0);
            if let Some(row) = row {
                order = row.try_get("max_order").unwrap_or(0);
            }
        }

        info!("max order: {}", order);

        // 插入到数据库
        let mut query_list: Vec<Query<MySql, MySqlArguments>> = Vec::new();
        let stage = &props.stage;
        let create_time = Utils::get_date(None);
        let status = PipelineStatus::got(PipelineStatus::Queue); // 排队中

        // 更新 pipeline 表 status
        let pipeline_query = sqlx::query::<MySql>(
            r#"
            UPDATE pipeline SET `status` = ?
        "#,
        )
        .bind(&status);
        query_list.push(pipeline_query);

        // 插入到 pipeline_runtime 表
        let basic_str = serde_json::to_string(&pipe.basic).unwrap_or(String::from(""));
        let stages_str = serde_json::to_string(&pipe.process_config.stages).unwrap_or(String::from(""));
        let runtime_id = Uuid::new_v4().to_string();
        let process_query = sqlx::query::<MySql>(
            r#"
            INSERT INTO pipeline_runtime (
                id, pipeline_id, project_name, `order`, tag, basic, stages, `status`, stage_index, group_index, step_index, finished, remark, create_time, update_time
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
        )
        .bind(runtime_id.clone())
        .bind(&pipe.id)
        .bind(GitHandler::get_project_name_by_git(&pipe.basic.path))
        .bind(format!("{}", order + 1))
        .bind(PipelineTag::got(props.tag.clone()))
        .bind(basic_str) // basic
        .bind(stages_str) // stages
        .bind(&status) // 排队中
        .bind(format!("{}", stage.stage_index.clone()))
        .bind(format!("{}", stage.group_index.clone()))
        .bind(format!("{}", stage.step_index.clone()))
        .bind("false")
        .bind(&props.remark)
        .bind(&create_time)
        .bind("");
        query_list.push(process_query);

        // 插入 pipeline_runtime_snapshot 表
        let snapshot = &props.snapshot;
        let snapshot_id = Uuid::new_v4().to_string();
        let snapshot_query = sqlx::query::<MySql>(
            r#"
            INSERT INTO pipeline_runtime_snapshot (
                id, runtime_id, node, branch, make, command, script, create_time, update_time
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
        )
        .bind(snapshot_id.clone())
        .bind(runtime_id.clone())
        .bind(&snapshot.node)
        .bind(&snapshot.branch)
        .bind(&snapshot.make)
        .bind(&snapshot.command)
        .bind(&snapshot.script)
        .bind(&create_time)
        .bind("");
        query_list.push(snapshot_query);

        // 插入 pipeline_runtime_variable 表
        let runnable_variables = &snapshot.runnable_variables;
        if !runnable_variables.is_empty() {
            for variable in runnable_variables.iter() {
                let variable_query = sqlx::query::<MySql>(
                    r#"
            INSERT INTO pipeline_runtime_variable (
                id, snapshot_id, `order`, name, `value`, description, create_time, update_time
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?)
        "#,
                )
                .bind(Uuid::new_v4().to_string())
                .bind(snapshot_id.clone())
                .bind(&variable.order)
                .bind(&variable.name)
                .bind(&variable.value)
                .bind(&variable.description)
                .bind(&create_time)
                .bind("");
                query_list.push(variable_query);
            }
        }

        let response = DBHelper::batch_commit(query_list).await?;
        if response.code != 200 {
            return Ok(response);
        }

        // 查询数据, 并插入到线程池
        let response = Self::get_runtime_detail(
            &pipeline,
            true,
            Some(PipelineRunnableQueryForm {
                status_list: vec![],
                runtime_id: Some(runtime_id.clone()),
                pipeline_id: None,
            }),
        )
        .await?;

        if response.code != 200 {
            return Ok(response);
        }

        let runtime: PipelineRuntime = serde_json::from_value(response.body.clone()).map_err(|err| Error::Error(err.to_string()).to_string())?;
        let mut pip = pipe.clone();
        pip.runtime = Some(runtime);
        Pool::insert_into_pool(&pipe)?;
        Ok(get_success_response(None))
    }
}

/// MARK: 并行任务
impl PipelineRunnable {
    /// 批量执行
    pub(crate) async fn batch_exec(list: &Vec<PipelineRuntime>) -> Result<HttpResponse, String> {
        if list.is_empty() {
            error!("batch exec pipeline list failed, `list` is empty !");
            return Ok(get_error_response("batch exec pipeline list failed, `list` is empty !"));
        }

        let mut result_errors: Vec<HttpResponse> = Vec::new(); // 错误
        let mut result: Vec<PipelineRuntime> = Vec::new();

        // 插入到线程池
        for runtime in list.iter() {
            match Self::exec(runtime).await {
                Ok(_) => result.push(runtime.clone()),
                Err(err) => {
                    error!("exec pipeline id: {:#?} error: {}", &runtime.id, &err);
                    result_errors.push(get_error_response(&err))
                }
            };
        }

        if result.is_empty() {
            error!("exec pipeline list failed, no data need to batch run !");
            return Ok(get_error_response("exec pipeline list failed, no data need to batch run !"));
        }

        if !result_errors.is_empty() {
            return Ok(get_success_response(Some(serde_json::Value::String(String::from("some pipeline into pools error !")))));
        }

        info!("insert into pools success !");
        return Ok(get_success_response(None));
    }

    /// 结束
    pub(crate) async fn exec_end_log(app: &AppHandle, pipeline: &Pipeline, success: bool, msg: &str) -> Option<Pipeline> {
        let err = if success { "成功" } else { "失败" };

        let runtime = &pipeline.clone().runtime.unwrap_or(PipelineRuntime::default());
        let order = runtime.order.unwrap_or(1);
        let msg = format!("{} {} !", msg, err);
        Self::save_log(app, &msg, &pipeline.server_id, &pipeline.id, order);

        let result = Self::update_stage(pipeline, runtime).await;
        return match result {
            Ok(_) => {
                EventEmitter::log_step_res(app, Some(get_success_response_by_value(pipeline.clone()).unwrap()));
                Some(pipeline.clone())
            }
            Err(err) => {
                Self::save_log(app, &err, &pipeline.server_id, &pipeline.id, order);
                EventEmitter::log_step_res(app, Some(get_error_response(&err)));
                None
            }
        };
    }

    /// 更新数据库
    pub(crate) async fn update_stage(pipeline: &Pipeline, runtime: &PipelineRuntime) -> Result<HttpResponse, String> {
        // 1. 更新 pipeline 表中的 status
        // 2. 更新 pipeline_runtime 表中的 status
        // 3. 修改 pipeline_runtime 中的 stage_index, group_index, step_index, finished
        let mut query_list = Vec::new();
        let pipeline_query = sqlx::query::<MySql>(
            r#"
            UPDATE pipeline SET `status` = ? WHERE id = ?
        "#,
        )
        .bind(PipelineStatus::got(runtime.status.clone()))
        .bind(&pipeline.id);
        query_list.push(pipeline_query);

        let mut runtime_sql = String::from(
            r#"
            UPDATE pipeline_runtime
            SET `status` = ?, stage_index = ?, group_index = ?, step_index = ?, finished = ?
        "#,
        );

        if let Some(duration) = &runtime.duration {
            runtime_sql.push_str(&format!(", duration = '{}'", duration));
        }

        runtime_sql.push_str(" WHERE id = ?");

        let runtime_query = sqlx::query::<MySql>(&runtime_sql)
            .bind(PipelineStatus::got(runtime.status.clone()))
            .bind(format!("{}", runtime.stage.stage_index))
            .bind(format!("{}", runtime.stage.group_index))
            .bind(format!("{}", runtime.stage.step_index))
            .bind(format!("{}", runtime.stage.finished))
            .bind(&runtime.id);
        query_list.push(runtime_query);
        DBHelper::batch_commit(query_list).await
    }

    /// 保存日志, 发送消息到前端
    pub(crate) fn save_log(app: &AppHandle, msg: &str, server_id: &str, id: &str, order: u32) {
        EventEmitter::log_event(app, id, msg);
        PipelineLogger::save_log(msg, server_id, id, order);
    }
}
