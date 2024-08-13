//! 流水线属性

use crate::server::pipeline::index::Pipeline;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// 基本信息
#[derive(Default, Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PipelineBasic {
    pub(crate) id: String,
    pub(crate) pipeline_id: String, // 流水线ID
    pub(crate) name: String,        // 名称
    pub(crate) tag: PipelineTag,    // 标签
    pub(crate) path: String,        // 项目路径
    #[serde(rename = "desc")]
    pub(crate) description: String, // 描述
    pub(crate) create_time: Option<String>,
    pub(crate) update_time: Option<String>,
}

impl PipelineBasic {
    pub fn is_empty(basic: &PipelineBasic) -> bool {
        return basic.name.is_empty() || PipelineTag::is_empty(basic.tag.clone()) || basic.path.is_empty();
    }
}

/// 流程配置
#[derive(Default, Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PipelineProcess {
    pub(crate) id: String,
    #[serde(rename = "pipelineId")]
    pub(crate) pipeline_id: String,
    pub(crate) stages: Vec<PipelineStage>,
    #[serde(rename = "createTime")]
    pub(crate) create_time: Option<String>,
    #[serde(rename = "updateTime")]
    pub(crate) update_time: Option<String>,
}

/// 流水线阶段, 一个阶段包括多个分组
#[derive(Default, Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PipelineStage {
    pub(crate) id: String,
    #[serde(rename = "processId")]
    pub(crate) process_id: String,
    pub(crate) groups: Vec<PipelineGroup>,
    #[serde(rename = "createTime")]
    pub(crate) create_time: Option<String>,
    #[serde(rename = "updateTime")]
    pub(crate) update_time: Option<String>,
}

/// 流水线分组
#[derive(Default, Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PipelineGroup {
    pub(crate) id: String,
    #[serde(rename = "stageId")]
    pub(crate) stage_id: String,
    pub(crate) title: String,
    pub(crate) steps: Vec<PipelineStep>,
    #[serde(rename = "createTime")]
    pub(crate) create_time: Option<String>,
    #[serde(rename = "updateTime")]
    pub(crate) update_time: Option<String>,
}

/// 流水线步骤
#[derive(Default, Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PipelineStep {
    pub(crate) id: String,
    #[serde(rename = "groupId")]
    pub(crate) group_id: String,
    pub(crate) module: PipelineCommandStatus,
    pub(crate) command: String,
    pub(crate) label: String,
    pub(crate) status: PipelineStatus,
    pub(crate) components: Vec<PipelineStepComponent>,
    #[serde(rename = "createTime")]
    pub(crate) create_time: Option<String>,
    #[serde(rename = "updateTime")]
    pub(crate) update_time: Option<String>,
}

/// 流水线步骤组件
#[derive(Default, Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PipelineStepComponent {
    pub(crate) id: String,
    #[serde(rename = "stepId")]
    pub(crate) step_id: String,
    pub(crate) prop: String,
    pub(crate) value: String,
    #[serde(rename = "createTime")]
    pub(crate) create_time: Option<String>,
    #[serde(rename = "updateTime")]
    pub(crate) update_time: Option<String>,
}

/// 流水线运行命令状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PipelineCommandStatus {
    None,      // 不运行
    GitPull,   // 代码拉取
    H5Install, // H5 安装依赖
    Pack,      // 项目打包
    Minimize,  // 文件压缩
    Compress,  // 图片压缩
    Deploy,    // 项目部署
    Notice,    // 发送通知
}

impl Default for PipelineCommandStatus {
    fn default() -> Self {
        PipelineCommandStatus::None
    }
}

impl PipelineCommandStatus {
    pub fn get(status: &str) -> PipelineCommandStatus {
        if status == "None" {
            return PipelineCommandStatus::None;
        }

        if status == "GitPull" {
            return PipelineCommandStatus::GitPull;
        }

        if status == "H5Install" {
            return PipelineCommandStatus::H5Install;
        }

        if status == "Pack" {
            return PipelineCommandStatus::Pack;
        }

        if status == "Minimize" {
            return PipelineCommandStatus::Minimize;
        }

        if status == "Compress" {
            return PipelineCommandStatus::Compress;
        }

        if status == "Deploy" {
            return PipelineCommandStatus::Deploy;
        }

        if status == "Notice" {
            return PipelineCommandStatus::Notice;
        }

        PipelineCommandStatus::None
    }

    pub fn got(status: PipelineCommandStatus) -> String {
        return match status {
            PipelineCommandStatus::None => "None".to_string(),
            PipelineCommandStatus::GitPull => "GitPull".to_string(),
            PipelineCommandStatus::H5Install => "H5Install".to_string(),
            PipelineCommandStatus::Pack => "Pack".to_string(),
            PipelineCommandStatus::Minimize => "Minimize".to_string(),
            PipelineCommandStatus::Compress => "Compress".to_string(),
            PipelineCommandStatus::Deploy => "Deploy".to_string(),
            PipelineCommandStatus::Notice => "Notice".to_string(),
        };
    }
}

impl PipelineProcess {
    pub(crate) fn is_empty(config: &PipelineProcess) -> bool {
        return config.stages.is_empty();
    }
}

/// 启动变量
#[derive(Default, Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PipelineVariable {
    pub(crate) id: String,
    pub(crate) pipeline_id: String,
    pub(crate) name: String,     // 变量名
    pub(crate) genre: String,    // 变量类型
    pub(crate) value: String,    // 值
    pub(crate) disabled: String, // 是否禁用
    pub(crate) require: String,  // 是否必填
    #[serde(rename = "desc")]
    pub(crate) description: String, // 描述
    pub(crate) create_time: Option<String>,
    pub(crate) update_time: Option<String>,
}

/// 启动变量选中
#[derive(Default, Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PipelineSelectedVariable {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) value: String,
}

impl PipelineVariable {
    pub fn is_empty(variable: &PipelineVariable) -> bool {
        return variable.name.is_empty() || variable.genre.is_empty() || variable.value.is_empty() || variable.disabled.is_empty() || variable.require.is_empty();
    }
}

/// 附加的变量
#[derive(Default, Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct RunnableVariable {
    pub(crate) branches: Vec<String>,
    pub(crate) h5: Option<H5RunnableVariable>,
    #[serde(rename = "isRemoteUrl")]
    pub(crate) is_remote_url: bool,
}

/// 附加的 H5 变量
#[derive(Default, Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct H5RunnableVariable {
    #[serde(rename = "displayFields")]
    pub(crate) display_fields: Vec<DisplayField>,
    pub(crate) selected: Option<H5ExtraSelectedVariable>,
    pub(crate) node: String,
    #[serde(rename = "makeCommands")]
    pub(crate) make_commands: Vec<String>,
    #[serde(rename = "installedCommands")]
    pub(crate) installed_commands: Vec<String>,
    #[serde(rename = "packageCommands")]
    pub(crate) package_commands: Vec<String>,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct DisplayField {
    pub(crate) label: String,
    pub(crate) value: String,
    #[serde(rename = "type")]
    pub(crate) show_type: String, // str | select
    pub(crate) desc: String,
    pub(crate) key: String,
}

/// 附加的选中的 H5 变量
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct H5ExtraSelectedVariable {
    #[serde(rename = "makeCommand")]
    pub(crate) make_command: Option<String>,
    #[serde(rename = "installedCommand")]
    pub(crate) installed_command: Option<String>,
    #[serde(rename = "packageCommand")]
    pub(crate) package_command: Option<String>,
}

/// 流水线运行状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PipelineStatus {
    No,      // 尚未运行
    Queue,   // 排队中
    Process, // 构建中
    Success, // 运行成功
    Failed,  // 运行失败
    Stop,    // 中止运行
}

impl Default for PipelineStatus {
    fn default() -> Self {
        PipelineStatus::No
    }
}

impl PipelineStatus {
    pub fn get(status: &str) -> PipelineStatus {
        if status == "No" {
            return PipelineStatus::No;
        }

        if status == "Queue" {
            return PipelineStatus::Queue;
        }

        if status == "Process" {
            return PipelineStatus::Process;
        }

        if status == "Success" {
            return PipelineStatus::Success;
        }

        if status == "Failed" {
            return PipelineStatus::Failed;
        }

        if status == "Stop" {
            return PipelineStatus::Stop;
        }

        PipelineStatus::No
    }

    pub fn got(status: PipelineStatus) -> String {
        return match status {
            PipelineStatus::No => "No".to_string(),
            PipelineStatus::Queue => "Queue".to_string(),
            PipelineStatus::Process => "Process".to_string(),
            PipelineStatus::Success => "Success".to_string(),
            PipelineStatus::Failed => "Failed".to_string(),
            PipelineStatus::Stop => "Stop".to_string(),
        };
    }
}

/// 流水线运行属性
#[derive(Default, Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PipelineRunVariable {
    #[serde(rename = "projectName")]
    pub(crate) project_name: String, // 名称
    pub(crate) branch: String, // 分支
    #[serde(rename = "historyList")]
    pub(crate) history_list: Vec<Pipeline>, // 运行历史
    pub(crate) current: PipelineCurrentRun, // 当前流水线状态
}

/// 当前运行流水线
#[derive(Default, Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PipelineCurrentRun {
    pub(crate) order: u32,                     // 顺序
    pub(crate) stage: PipelineCurrentRunStage, // stage 运行到哪一步
    pub(crate) stages: Vec<PipelineStage>,     // 构建过程
    #[serde(rename = "startTime")]
    pub(crate) start_time: String, // 开始时间
    pub(crate) duration: u32,                  // 运行时长, 单位秒
    pub(crate) runnable: PipelineRunProps,     // 运行时快照
    pub(crate) log: String,                    // 日志, 根据 {server_id/id/order}.log 来读取
}

/// 当前流水线步骤
#[derive(Default, Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PipelineCurrentRunStage {
    pub(crate) index: u32, // stage 运行到哪一步, 从 1 开始计算
    #[serde(rename = "groupIndex")]
    pub(crate) group_index: u32, // group 运行到哪一步, 从 0 开始计算
    #[serde(rename = "stepIndex")]
    pub(crate) step_index: u32, // step 运行到哪一步, 从 0 开始计算
    #[serde(rename = "finishGroupCount")]
    pub(crate) finish_group_count: u32, // stage 中运行完成 group 个数
    pub(crate) finished: bool, // 是否完成
    pub(crate) status: Option<PipelineStatus>, // 运行状态
}

/// 流水线运行时的属性
#[derive(Default, Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PipelineRunProps {
    pub(crate) id: String, // 流水线ID,
    #[serde(rename = "serverId")]
    pub(crate) server_id: String, // 服务器ID,
    pub(crate) stage: PipelineCurrentRunStage, // stage
    pub(crate) tag: PipelineTag, // 流水线 Tag
    pub(crate) node: Option<String>, // nodeJs 版本号
    pub(crate) branch: String, // 分支
    pub(crate) make: Option<String>, // Make命令
    pub(crate) command: Option<String>, // 本机安装的命令
    pub(crate) script: Option<String>, // package.json 中的 scripts 命令
    pub(crate) variables: Vec<PipelineVariable>, // 启动变量
    #[serde(rename = "selectedVariables")]
    pub(crate) selected_variables: Vec<PipelineSelectedVariable>, // 启动变量选中的值
    pub(crate) remark: String, // 备注
}

/// 流水线标签
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PipelineTag {
    None,    // none
    Develop, // 开发
    Test,    // 测试
    CAddAdd, // C++
    Rust,
    Java,
    Android,
    Ios,
    H5,
}

impl Default for PipelineTag {
    fn default() -> Self {
        PipelineTag::None
    }
}

impl PipelineTag {
    pub fn is_empty(tag: PipelineTag) -> bool {
        return match tag {
            PipelineTag::None => true,
            PipelineTag::Develop => false,
            PipelineTag::Test => false,
            PipelineTag::CAddAdd => false,
            PipelineTag::Rust => false,
            PipelineTag::Java => false,
            PipelineTag::Android => false,
            PipelineTag::Ios => false,
            PipelineTag::H5 => false,
        };
    }

    pub fn get(tag: &str) -> PipelineTag {
        if tag == "None" {
            return PipelineTag::None;
        }

        if tag == "Develop" {
            return PipelineTag::Develop;
        }

        if tag == "Test" {
            return PipelineTag::Test;
        }

        if tag == "CAddAdd" {
            return PipelineTag::CAddAdd;
        }

        if tag == "Rust" {
            return PipelineTag::Rust;
        }

        if tag == "Java" {
            return PipelineTag::Java;
        }

        if tag == "Android" {
            return PipelineTag::Android;
        }

        if tag == "Ios" {
            return PipelineTag::Ios;
        }

        if tag == "H5" {
            return PipelineTag::H5;
        }

        PipelineTag::None
    }

    pub fn got(tag: PipelineTag) -> String {
        return match tag {
            PipelineTag::None => "None".to_string(),
            PipelineTag::Develop => "Develop".to_string(),
            PipelineTag::Test => "Test".to_string(),
            PipelineTag::CAddAdd => "C++".to_string(),
            PipelineTag::Rust => "Rust".to_string(),
            PipelineTag::Java => "Java".to_string(),
            PipelineTag::Android => "Android".to_string(),
            PipelineTag::Ios => "Ios".to_string(),
            PipelineTag::H5 => "H5".to_string(),
        };
    }
}

/// 系统命令集
#[derive(Default, Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct OsCommands {
    #[serde(rename = "h5InstalledCommands")]
    pub(crate) h5_installed_commands: Vec<String>,
}

/// 执行任务
#[derive(Default, Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PipelineStageTask {
    pub(crate) id: String,
    pub(crate) server_id: String,
    pub(crate) tag: PipelineTag,
    pub(crate) stages: Vec<PipelineStage>,
    pub(crate) props: PipelineRunProps,
    pub(crate) order: u32,
}

/// 执行 step
#[derive(Default, Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PipelineRunnableStageStep {
    pub(crate) id: String,
    pub(crate) server_id: String,
    pub(crate) tag: PipelineTag,
    pub(crate) stage_index: u32,
    pub(crate) group_index: u32,
    pub(crate) step_index: u32,
    pub(crate) step: PipelineStep,
}
