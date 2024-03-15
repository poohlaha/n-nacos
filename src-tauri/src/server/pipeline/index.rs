//! 流水线

use crate::database::interface::{Treat, TreatBody};
use crate::database::Database;
use crate::error::Error;
use crate::exports::pipeline::QueryForm;
use crate::helper::git::GitHandler;
use crate::helper::index::Helper;
use crate::logger::pipeline::PipelineLogger;
use crate::prepare::{get_error_response, get_success_response, get_success_response_by_value, HttpResponse};
use crate::server::pipeline::languages::h5::H5FileHandler;
use crate::server::pipeline::props::{ExtraVariable, H5ExtraVariable, OsCommands, PipelineBasic, PipelineCurrentRun, PipelineCurrentRunStage, PipelineProcessConfig, PipelineRunVariable, PipelineStatus, PipelineTag, PipelineVariable};
use handlers::utils::Utils;
use log::info;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
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
    #[serde(rename = "createTime")]
    pub(crate) create_time: Option<String>, // 创建时间
    #[serde(rename = "updateTime")]
    pub(crate) update_time: Option<String>, // 修改时间
    #[serde(rename = "lastRunTime")]
    pub(crate) last_run_time: Option<String>, // 最后运行时间
    pub(crate) basic: PipelineBasic, // 基本信息
    #[serde(rename = "processConfig")]
    pub(crate) process_config: PipelineProcessConfig, // 流程配置
    pub(crate) status: PipelineStatus, // 状态, 同步于 steps
    pub(crate) variables: Vec<PipelineVariable>, // 变量
    pub(crate) extra: Option<ExtraVariable>, // 额外的信息
    pub(crate) run: Option<PipelineRunVariable>, // 运行信息
}

impl TreatBody for Pipeline {}

impl Treat<HttpResponse> for Pipeline {
    type B = Pipeline;

    /// 列表
    fn get_list(pipeline: &Self::B) -> Result<HttpResponse, String> {
        if pipeline.server_id.is_empty() {
            return Ok(get_error_response("获取流水线列表失败, `server_id` 不能为空"));
        }

        let response = Database::get_list::<Pipeline>(PIPELINE_DB_NAME, &Self::get_pipeline_name(&pipeline.server_id))?;
        if response.code != 200 {
            return Ok(response.clone());
        }

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
    }

    /// 插入
    fn insert(pipeline: &Self::B) -> Result<HttpResponse, String> {
        let res = Self::validate(&pipeline);
        if let Some(res) = res {
            return Ok(res);
        }

        let mut pipeline_clone = pipeline.clone();
        if pipeline_clone.id.is_empty() {
            pipeline_clone.id = Uuid::new_v4().to_string()
        }

        // 创建时间
        pipeline_clone.create_time = Some(Utils::get_date(None));

        // 设置运行时属性
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
        pipeline_clone.run = Some(run_variable);

        info!("insert pipeline params: {:#?}", pipeline_clone);

        let data = Self::get_pipeline_list(&pipeline);
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
    }

    /// 更新
    fn update(pipeline: &Self::B) -> Result<HttpResponse, String> {
        let res = Self::validate(&pipeline);
        if let Some(res) = res {
            return Ok(res);
        }

        info!("update pipeline params: {:#?}", pipeline);
        let data = Self::get_pipeline_list(&pipeline);
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

                if let Some(mut run) = line.run.clone() {
                    let mut current = run.current.clone();
                    current.stages = pipeline.process_config.stages.clone();
                    run.current = current;
                    line.run = Some(run.clone());
                }

                Self::update_pipeline(data, &line)
            }
            Err(_) => Ok(get_error_response("更新流水线失败")),
        };
    }

    /// 删除
    fn delete(pipeline: &Self::B) -> Result<HttpResponse, String> {
        if pipeline.id.is_empty() {
            return Ok(get_error_response("删除流水线失败, `id` 不能为空"));
        }

        if pipeline.server_id.is_empty() {
            return Ok(get_error_response("删除流水线失败, `server_id` 不能为空"));
        }

        let id = &pipeline.id;
        let server_id = &pipeline.server_id;
        info!("delete pipeline id: {}, server_id: {}", &id, &server_id);

        let data = Self::get_pipeline_list(&pipeline);
        return match data {
            Ok(data) => {
                if data.is_empty() {
                    return Ok(get_error_response("删除流水线失败, 该流水线不存在"));
                }

                let pipeline = data.iter().find(|s| s.id.as_str() == id.as_str());
                if pipeline.is_none() {
                    return Ok(get_error_response("删除流水线失败, 该流水线不存在"));
                }

                let pipelines: Vec<Pipeline> = data.iter().filter_map(|s| if s.id.as_str() == id.as_str() { None } else { Some(s.clone()) }).collect();

                // 删除流水线日志
                PipelineLogger::delete_log_by_id(&server_id, &id);

                // 更新数据库
                Database::update::<Pipeline>(PIPELINE_DB_NAME, &Self::get_pipeline_name(&server_id), pipelines, "删除流水线失败")
            }
            Err(_) => Ok(get_error_response("删除流水线失败")),
        };
    }

    /// 根据 ID 查找数据
    fn get_by_id(pipeline: &Self::B) -> Result<HttpResponse, String> {
        if pipeline.id.is_empty() {
            return Ok(get_error_response("根据 ID 查找流水线失败, `id` 不能为空"));
        }

        if pipeline.server_id.is_empty() {
            return Ok(get_error_response("根据 ID 查找流水线失败, `server_id` 不能为空"));
        }

        info!("get pipeline by id: {}, server_id: {}", &pipeline.id, &pipeline.server_id);
        let data = Self::get_pipeline_list(&pipeline);
        return match data {
            Ok(data) => {
                if data.is_empty() {
                    return Ok(get_error_response("根据 ID 查找流水线失败, 该流水线不存在"));
                }

                let pipe = data.iter().find(|s| &s.id == &pipeline.id);
                if let Some(pipe) = pipe {
                    let mut pipeline = pipe.clone();
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

                    let data = serde_json::to_value(pipeline).map_err(|err| Error::Error(err.to_string()).to_string())?;
                    return Ok(get_success_response(Some(data)));
                }

                Ok(get_error_response("根据 ID 查找流水线失败, 该流水线不存在"))
            }
            Err(_) => Ok(get_error_response("根据 ID 查找流水线失败")),
        };
    }
}

impl Pipeline {
    /// 根据条件查询列表
    pub(crate) fn get_query_list(pipeline: &Pipeline, form: &QueryForm) -> Result<HttpResponse, String> {
        if QueryForm::is_empty(form) {
            return Self::get_list(pipeline);
        }

        let data = Self::get_pipeline_list(pipeline)?;
        if data.is_empty() {
            let data = serde_json::to_value(data).map_err(|err| Error::Error(err.to_string()).to_string())?;
            return Ok(get_success_response(Some(data)));
        }

        // 根据条件过滤
        let data: Vec<Pipeline> = data.iter().filter_map(|pipe| Self::find_pipeline_by_form(form, pipe)).collect();

        let data = serde_json::to_value(data).map_err(|err| Error::Error(err.to_string()).to_string())?;
        return Ok(get_success_response(Some(data)));
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
    pub(crate) fn get_pipeline_list(pipeline: &Pipeline) -> Result<Vec<Pipeline>, String> {
        let response = Pipeline::get_list(&pipeline)?;
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

        if PipelineProcessConfig::is_empty(process_config) {
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
}
