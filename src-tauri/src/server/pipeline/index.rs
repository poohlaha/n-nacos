//! 流水线

use std::collections::HashMap;
use crate::database::helper::DBHelper;
use crate::database::interface::{Treat, Treat2, TreatBody};
use crate::database::Database;
use crate::error::Error;
use crate::exports::pipeline::QueryForm;
use crate::helper::git::GitHandler;
use crate::helper::index::Helper;
use crate::logger::pipeline::PipelineLogger;
use crate::prepare::{get_error_response, get_success_response, get_success_response_by_value, HttpResponse};
use crate::server::index::Server;
use crate::server::pipeline::languages::h5::H5FileHandler;
use crate::server::pipeline::props::{ExtraVariable, H5ExtraVariable, OsCommands, PipelineBasic, PipelineCommandStatus, PipelineCurrentRun, PipelineCurrentRunStage, PipelineGroup, PipelineProcess, PipelineRunVariable, PipelineStage, PipelineStatus, PipelineStep, PipelineStepComponent, PipelineTag, PipelineVariable};
use async_trait::async_trait;
use handlers::utils::Utils;
use log::{error, info};
use serde::{Deserialize, Serialize};
use sqlx::mysql::{MySqlArguments, MySqlRow};
use sqlx::{FromRow, MySql, Row};
use std::path::PathBuf;
use serde_json::Value;
use uuid::Uuid;

/// 存储流水线数据库名称
pub(crate) const PIPELINE_DB_NAME: &str = "pipeline";

/// 存储流水线名称
const PIPELINE_NAME: &str = "pipelines";

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct Pipeline {
    pub(crate) id: String,
    #[serde(rename = "serverId")]
    pub(crate) server_id: String, // 服务器 ID
    #[serde(rename = "lastRunTime")]
    pub(crate) last_run_time: String, // 最后运行时间
    pub(crate) tag: PipelineTag,            // 标签
    pub(crate) status: PipelineStatus, // 状态, 同步于 steps
    pub(crate) duration: String,  // 运行时长, 单位秒

    pub(crate) basic: PipelineBasic,   // 基本信息
    #[serde(rename = "processConfig")]
    pub(crate) process_config: PipelineProcess, // 流程配置
    pub(crate) variables: Vec<PipelineVariable>, // 变量
    pub(crate) extra: Option<ExtraVariable>, // 额外的信息
    pub(crate) run: Option<PipelineRunVariable>, // 运行信息
    #[serde(rename = "createTime")]
    pub(crate) create_time: Option<String>, // 创建时间
    #[serde(rename = "updateTime")]
    pub(crate) update_time: Option<String>, // 修改时间
    pub(crate) stage_run_index: u32, // stage 运行到哪一步, 从 1 开始计算
}

impl<'r> FromRow<'r, MySqlRow> for Pipeline {
    fn from_row(row: &MySqlRow) -> Result<Self, sqlx::Error> {
        let status_str: String = row.try_get("status")?;
        let tag_str: String = row.try_get("tagValue")?;
        let tag = PipelineTag::get(&tag_str);

        let basic = PipelineBasic {
            id: row.try_get("basic_id")?,
            pipeline_id: row.try_get("basic_pipeline_id")?,
            name: row.try_get("basic_name")?,
            tag: tag.clone(),
            path: row.try_get("basic_path")?,
            description: row.try_get("basic_description")?,
            create_time: row.try_get("basic_create_time")?,
            update_time: row.try_get("basic_update_time")?,
        };

        Ok(Pipeline {
            id: row.try_get("id")?,
            server_id: row.try_get("server_id")?,
            last_run_time: row.try_get("last_run_time")?,
            tag,
            status: PipelineStatus::get(&status_str),
            duration: row.try_get("duration")?,
            stage_run_index: row.try_get("stage_run_index")?,
            create_time: row.try_get("create_time")?,
            update_time: row.try_get("update_time")?,
            basic,
            process_config: Default::default(),
            variables: Vec::new(),
            extra: None,
            run: None,
        })
    }
}

impl TreatBody for Pipeline {}

#[async_trait]
impl Treat2<HttpResponse> for Pipeline {
    type B = Pipeline;

    /// 列表
    async fn get_list(pipeline: &Self::B) -> Result<HttpResponse, String> {
        if pipeline.server_id.is_empty() {
            return Ok(get_error_response("获取流水线列表失败, `server_id` 不能为空"));
        }

        let query = sqlx::query(r#"
            SELECT
                p.id as pipeline_id,
                p.server_id as pipeline_server_id,
                p.tag_id as pipeline_tag_id,
                p.last_run_time as pipeline_last_run_time,
                p.duration as pipeline_duration,
                p.`status` as pipeline_status,
                p.create_time as pipeline_create_time,
                p.update_time as pipeline_update_time,
                p.stage_run_index as stage_run,
                CAST(p.stage_run_index AS UNSIGNED) AS stage_run_index,
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
                c.`value` as step_component_value,
                c.create_time as step_create_time,
                c.update_time as step_update_time
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
            GROUP BY
                    p.id, p.stage_run_index, b.id, t.`value`, s.id, e.id, g.id, sp.id, c.id, v.id, v.`name`, v.genre, v.`value`, v.disabled, v.`require`, v.description, v.create_time, v.update_time
            ORDER BY
            CASE
                WHEN
                     p.update_time IS NULL
                THEN
                     0
                ELSE
                     1
            END DESC,
            p.update_time DESC,
            p.create_time DESC
        "#);

        let rows = DBHelper::execute_rows(query).await?;
        if rows.is_empty() {
            return Ok(get_success_response(Some(Value::Array(Vec::new()))))
        }

        // 组装数据
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
            let tag = PipelineTag::get(&tag_str);

            let basic = PipelineBasic {
                id: row.try_get("basic_id").unwrap_or(String::new()),
                pipeline_id: row.try_get("basic_pipeline_id").unwrap_or(String::new()),
                name: row.try_get("basic_name").unwrap_or(String::new()),
                tag: tag.clone(),
                path: row.try_get("basic_path").unwrap_or(String::new()),
                description: row.try_get("basic_description").unwrap_or(String::new()),
                create_time: row.try_get("basic_create_time").unwrap_or(None),
                update_time: row.try_get("basic_update_time").unwrap_or(None),
            };

            // pipeline
            map.entry(pipeline_id.clone()).or_insert_with(|| Pipeline {
                id: pipeline_id.to_string(),
                server_id: row.try_get("pipeline_server_id").unwrap_or(String::new()),
                last_run_time: row.try_get("last_run_time").unwrap_or(String::new()),
                tag,
                status: PipelineStatus::get(&status_str),
                duration: row.try_get("pipeline_duration").unwrap_or(String::new()),
                basic,
                process_config: Default::default(),
                variables: vec![],
                extra: None,
                run: None,
                create_time: row.try_get("create_time").unwrap_or(None),
                update_time: row.try_get("update_time").unwrap_or(None),
                stage_run_index: row.try_get("stage_run_index").unwrap_or(1),
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
                create_time: row.try_get("group_create_time").unwrap_or(None),
                update_time: row.try_get("group_update_time").unwrap_or(None),
            });

            // step component
            let step_component_id = row.try_get("step_component_id").unwrap_or(String::new());
            step_component_map.entry(step_component_id.clone()).or_insert_with(|| PipelineStepComponent {
                id: step_id.to_string(),
                step_id: row.try_get("step_component_step_id").unwrap_or(String::new()),
                prop: row.try_get("step_component_prop").unwrap_or(String::new()),
                value: row.try_get("step_component_value").unwrap_or(String::new()),
                create_time: row.try_get("group_create_time").unwrap_or(None),
                update_time: row.try_get("group_update_time").unwrap_or(None),
            });
        }

        // step
        for step_component_id in step_component_map.keys() {
            let step_component = step_component_map.get(step_component_id);
            if let Some(step_component) = step_component {
                let step = step_map.get_mut(&step_component.step_id);
                if let Some(mut step) = step {
                    step.components.push(step_component.clone());
                }
            }
        }

        // group
        for group_id in group_map.keys() {
            let group = group_map.get(group_id);
            if let Some(group) = group {
                let stage = stage_map.get_mut(&group.stage_id);
                if let Some(mut stage) = stage {
                    stage.groups.push(group.clone());
                }
            }
        }

        // process
        for stage_id in stage_map.keys() {
            let stage = stage_map.get(stage_id);
            if let Some(stage) = stage {
                let process = process_map.get_mut(&stage.process_id);
                if let Some(mut process) = process {
                    process.stages.push(stage.clone());
                }
            }
        }

        // pipeline
        for process_id in process_map.keys() {
            let process = process_map.get(process_id);
            if let Some(process) = process {
                let pipe = map.get_mut(&process.pipeline_id);
                if let Some(mut pipe) = pipe {
                    pipe.process_config = process.clone()
                }
            }
        }

        let list: Vec<Pipeline> = map.into_values().collect();
        get_success_response_by_value(list)

        /*
        // 解析成 list, 添加其他属性
        let data: Vec<Pipeline> = serde_json::from_value(response.body.clone()).map_err(|err| Error::Error(err.to_string()).to_string())?;
        if data.is_empty() {
            return Ok(response.clone());
        }

        let installed_commands = H5FileHandler::get_installed_commands();
        // node 版本
        let node = Helper::get_cmd_version("node");

        let result: Vec<Pipeline> = data
            .iter()
            .map(|res| {
                let url = &res.basic.path;
                let tag = res.basic.tag.clone();
                let extra = Self::get_extra_variable(url, tag, installed_commands.clone(), &node);

                let mut res_clone = res.clone();
                res_clone.extra = Some(extra);
                return res_clone;
            })
            .collect();

        let result = serde_json::to_value(result).map_err(|err| Error::Error(err.to_string()).to_string())?;
        return Ok(get_success_response(Some(result)));
         */
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

        let mut query_list = Vec::new();

        // 插入 pipeline 表
        let pipeline_query = sqlx::query::<MySql>(
            r#"
            INSERT INTO pipeline (id, server_id, tag_id, last_run_time, duration, status, stage_run_index, create_time, update_time)
            VALUES (?, ?, ?, ?, ?, ?)
        "#,
        )
        .bind(&pipeline_clone.id)
        .bind(&pipeline_clone.server_id)
        .bind(tag.id.clone())
        .bind("")
        .bind("")
        .bind(PipelineStatus::got(PipelineStatus::No))
        .bind(1)
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

        fn insert_step_components(components: &Vec<PipelineStepComponent>, step_id: String, create_time: String, query_list: &mut Vec<sqlx::query::Query<MySql, MySqlArguments>>) {
            if components.is_empty() {
                return;
            }

            for component in components.iter() {
                let step_component_id = Uuid::new_v4().to_string();
                let step_component_query = sqlx::query::<MySql>(
                    r#"
            INSERT INTO pipeline_step_component (id, step_id, prop, `value`, create_time, update_time)
            VALUES (?, ?, ?, ?, ?, ?)
        "#,
                )
                .bind(step_component_id.clone())
                .bind(step_id.clone())
                .bind(component.prop.clone())
                .bind(component.value.clone())
                .bind(create_time.clone())
                .bind(component.update_time.clone());
                query_list.push(step_component_query);
            }
        }

        fn insert_steps(steps: &Vec<PipelineStep>, group_id: String, create_time: String, query_list: &mut Vec<sqlx::query::Query<MySql, MySqlArguments>>) {
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

        fn insert_groups(groups: &Vec<PipelineGroup>, stage_id: String, create_time: String, query_list: &mut Vec<sqlx::query::Query<MySql, MySqlArguments>>) {
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

        // 2. 流水线阶段, 插入 pipeline_stage 表
        let stages = &process_config.stages;
        if !stages.is_empty() {
            for stage in stages.iter() {
                let stage_id = Uuid::new_v4().to_string();
                let stage_query = sqlx::query::<MySql>(
                    r#"
            INSERT INTO pipeline_stage (id, process_id, create_time, update_time)
            VALUES (?, ?, ?, ?)
        "#,
                )
                .bind(stage_id.clone())
                .bind(process_id.clone())
                .bind(&create_time)
                .bind(&process_config.update_time);
                query_list.push(stage_query);

                // 3. 流水线分组, 插入 pipeline_group 表
                insert_groups(&stage.groups, stage_id.clone(), create_time.clone(), &mut query_list);
            }
        }

        // 插入 pipeline_variable 表
        let variables = &pipeline_clone.variables;
        if !variables.is_empty() {
            for variable in variables.iter() {
                let variable_query = sqlx::query::<MySql>(
                    r#"
            INSERT INTO pipeline_variable (id, pipeline_id, `name`, genre, `value`, disabled, `require`, description, create_time, update_time)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
                )
                .bind(Uuid::new_v4().to_string().clone())
                .bind(&pipeline_clone.id)
                .bind(&variable.name)
                .bind(&variable.genre)
                .bind(&variable.value)
                .bind(&variable.disabled)
                .bind(&variable.require)
                .bind(&variable.description)
                .bind(&create_time)
                .bind(&basic.update_time);
                query_list.push(variable_query);
            }
        }

        return DBHelper::batch_commit(query_list).await;

        // 设置运行时属性
        /*
        let mut run_variable = PipelineRunVariable::default();
        let basic = &pipeline.basic;
        run_variable.project_name = GitHandler::get_project_name_by_git(&basic.path); // 获取项目名称

        // 当时运行流水线属性
        let mut current = PipelineCurrentRun::default();

        // stage
        let mut stage = PipelineCurrentRunStage::default();
        stage.status = Some(PipelineStatus::No);

        // stages
        current.stages = pipeline.process_config.stages.clone();
        run_variable.current = current;
        // pipeline_clone.run = Some(run_variable);

        info!("insert pipeline params: {:#?}", pipeline_clone);

        let data = Self::get_pipeline_list(&pipeline).await;
        return match data {
            Ok(mut data) => {
                if data.is_empty() {
                    data.push(pipeline_clone)
                } else {
                    // 查找名字是不是存在
                    let line = data.iter().find(|s| {
                        let b = &s.basic;
                        return &b.name == &pipeline.basic.name;
                    });

                    // 找到相同记录
                    if line.is_some() {
                        return Ok(get_error_response("流水线名字已存在"));
                    }

                    data.push(pipeline_clone)
                }

                Database::update::<Pipeline>(PIPELINE_DB_NAME, &Self::get_pipeline_name(&pipeline.server_id), data, "保存流水线失败")
            }
            Err(_) => Ok(get_error_response("保存流水线失败")),
        };
         */
    }

    /// 更新
    async fn update(pipeline: &Self::B) -> Result<HttpResponse, String> {
        let res = Self::validate(&pipeline);
        if let Some(res) = res {
            return Ok(res);
        }

        info!("update pipeline params: {:#?}", pipeline);
        let data = Self::get_pipeline_list(&pipeline).await;
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
                let line_by_name = data.iter().find(|s| {
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

        let query = sqlx::query::<MySql>("delete from pipeline where id = ? and server_id = ?").bind(&id).bind(&server_id);
        let mut response = DBHelper::execute_update(query).await?;
        if response.code != 200 {
            response.error = String::from("删除服务器失败");
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
        let query = sqlx::query_as::<_, Server>("select * from pipeline where id = ? and server_id = ?").bind(&pipeline.id).bind(&pipeline.server_id);
        let mut response = DBHelper::execute_query(query).await?;
        if response.code != 200 {
            response.error = String::from("根据 ID 查找流水线失败");
            return Ok(response);
        }

        let data: Vec<Pipeline> = serde_json::from_value(response.body).map_err(|err| Error::Error(err.to_string()).to_string())?;
        if data.is_empty() {
            return Ok(get_error_response("根据 ID 查找流水线失败, 该流水线不存在"));
        }

        let pipe = data.get(0).unwrap();
        let mut pipeline = pipe.clone();
        /*
        if let Some(mut run) = pipeline.run.clone() {
            let mut current = run.current.clone();
            // 读取日志
            let log = PipelineLogger::read_log(&pipeline.server_id, &pipeline.id, current.order);
            match log {
                Ok(content) => {
                    current.log = content;
                    run.current = current;
                    pipeline.run = Some(run);
                }
                Err(err) => {
                    info!("read pipeline log error: {}", &err)
                }
            }
        }
         */

        let data = serde_json::to_value(pipeline).map_err(|err| Error::Error(err.to_string()).to_string())?;
        return Ok(get_success_response(Some(data)));
    }
}

impl Pipeline {
    /// 根据条件查询列表
    pub(crate) async fn get_query_list(pipeline: &Pipeline, form: &QueryForm) -> Result<HttpResponse, String> {
        if QueryForm::is_empty(form) {
            return Self::get_list(pipeline).await;
        }

        let query: sqlx::query::QueryAs<'_, MySql, Pipeline, MySqlArguments> = sqlx::query_as::<_, Pipeline>(
            r#"
            SELECT
                p.id,
                p.server_id,
                p.last_run_time,
                p.status,
                p.basic_id,
                p.process_id,
                p.create_time,
                p.update_time,
                b.name
            FROM
                pipeline p
            LEFT JOIN pipeline_basic b
            ON p.basic_id = b.id
            WHERE
                b.`name` LIKE '%?' and p.`status` = '?'
            ORDER BY
            CASE
                    WHEN p.update_time IS NULL THEN
                    0 ELSE 1
                END DESC,
                p.update_time DESC,
                p.create_time DESC
        "#,
        )
        .bind(&form.name)
        .bind(&form.status);

        return DBHelper::execute_query(query).await;
    }

    fn find_pipeline_by_form(form: &QueryForm, pipeline: &Pipeline) -> Option<Pipeline> {
        let basic = &pipeline.basic;

        // name 不为空, status 为空
        if !form.name.is_empty() && form.status.is_empty() {
            if basic.name.to_lowercase().contains(&form.name.to_lowercase()) {
                return Some(pipeline.clone());
            }
        }

        // name 为空, status 不为空
        if form.name.is_empty() && !form.status.is_empty() {
            // 获取 status
            let status = PipelineStatus::got(pipeline.status.clone());
            if status == form.status {
                return Some(pipeline.clone());
            }
        }

        // name 不为空, status 不为空
        if !form.name.is_empty() && !form.status.is_empty() {
            // 获取 status
            let status = PipelineStatus::got(pipeline.status.clone());
            if status == form.status && basic.name.to_lowercase().contains(&form.name.to_lowercase()) {
                return Some(pipeline.clone());
            }
        }

        None
    }

    /// 获取流水线列表
    pub(crate) async fn get_pipeline_list(pipeline: &Pipeline) -> Result<Vec<Pipeline>, String> {
        let response = Pipeline::get_list(&pipeline).await?;
        if response.code != 200 {
            return Err(Error::convert_string(&response.error));
        }

        let data: Vec<Pipeline> = serde_json::from_value(response.body).map_err(|err| Error::Error(err.to_string()).to_string())?;
        return Ok(data);
    }

    /// 流水线存储名字: PIPELINE_NAME_流水线ID
    fn get_pipeline_name(server_id: &str) -> String {
        return format!("{}_{}", PIPELINE_NAME, server_id);
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

    /// 获取附加的变量
    fn get_extra_variable(url: &str, tag: PipelineTag, installed_commands: Vec<String>, node: &str) -> ExtraVariable {
        // branch
        let branches = GitHandler::get_branch_list(url);

        // h5
        let mut h5_extra_variables: Option<H5ExtraVariable> = None;
        match tag {
            PipelineTag::None => {}
            PipelineTag::Develop => {}
            PipelineTag::Test => {}
            PipelineTag::CAddAdd => {}
            PipelineTag::Rust => {}
            PipelineTag::Java => {}
            PipelineTag::Android => {}
            PipelineTag::Ios => {}
            PipelineTag::H5 => {
                h5_extra_variables = Self::get_h5_extra_variables(url, installed_commands, &node, branches.clone());
            }
        }

        return ExtraVariable {
            branches,
            h5: h5_extra_variables,
            is_remote_url: GitHandler::is_remote_url(url),
        };
    }

    /// 获取附加的 H5 变量
    fn get_h5_extra_variables(url: &str, installed_commands: Vec<String>, node: &str, branches: Vec<String>) -> Option<H5ExtraVariable> {
        let mut h5_extra_variables = H5FileHandler::get_default_file_commands(url);
        if let Some(h5_extra_variables) = h5_extra_variables.as_mut() {
            h5_extra_variables.node = node.to_string();
            h5_extra_variables.installed_commands = installed_commands;
            return Some(h5_extra_variables.clone());
        }

        let mut h5_extra_variables = H5ExtraVariable::default();
        h5_extra_variables.node = node.to_string();
        h5_extra_variables.installed_commands = installed_commands;

        // 根据 branches 获取所有的 package.json 或 Makefile 文件内容
        /*
        branches.iter().for_each(|branch| {
            H5FileHandler::get_file_by_branch(branch, url, "package.json").unwrap();
        });
         */

        return Some(h5_extra_variables);
    }

    /// 更新流水线
    pub(crate) fn update_pipeline(data: Vec<Pipeline>, pipeline: &Pipeline) -> Result<HttpResponse, String> {
        let pipelines: Vec<Pipeline> = data.iter().map(|s| if &s.id == &pipeline.id { pipeline.clone() } else { s.clone() }).collect();
        return Database::update::<Pipeline>(PIPELINE_DB_NAME, &Self::get_pipeline_name(&pipeline.server_id), pipelines, "更新流水线失败");
    }

    /// 清空
    pub(crate) fn clear(server_id: &str) -> Result<HttpResponse, String> {
        Database::delete(PIPELINE_DB_NAME, &Self::get_pipeline_name(server_id))
    }

    /// 查询系统已安装的 commands 列表
    pub(crate) fn query_os_commands() -> Result<HttpResponse, String> {
        let h5_installed_commands = H5FileHandler::get_installed_commands();
        get_success_response_by_value(OsCommands { h5_installed_commands })
    }

    /// 清空运行历史, 删除流水线运行日志记录
    pub(crate) async fn clear_run_history(pipeline: &Pipeline) -> Result<HttpResponse, String> {
        let res = Pipeline::get_by_id(&pipeline).await?;
        let mut pipeline: Pipeline = serde_json::from_value(res.body.clone()).map_err(|err| Error::Error(err.to_string()).to_string())?;
        /*
        let run = pipeline.run;
        if let Some(run) = run {
            let mut run_cloned = run.clone();
            run_cloned.history_list = Vec::new();
            pipeline.run = Some(run_cloned);
            let response = Self::update(&pipeline).await;
            return match response {
                Ok(_) => {
                    // 删除流水线日志
                    let bool = PipelineLogger::delete_log_by_id(&pipeline.server_id, &pipeline.id);
                    if !bool {
                        info!("clear run history failed, can not delete log files !");
                        return Ok(get_error_response(&format!("清除运行历史失败, 无法删除日志文件!")));
                    }

                    info!("clear run history success!");
                    get_success_response_by_value(pipeline)
                }
                Err(err) => {
                    info!("clear run history error: {}", err);
                    Ok(get_error_response(&format!("清除运行历史失败: {err}")))
                }
            };
        }
         */

        let msg = "clear run history failed, `run` prop is empty !";
        error!("{}", msg);
        return Err(Error::convert_string(msg));
    }
}
