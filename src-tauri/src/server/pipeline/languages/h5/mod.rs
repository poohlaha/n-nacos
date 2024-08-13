//! h5 文件助手

use crate::helper::git::GitHandler;
use crate::helper::index::Helper;
use crate::server::pipeline::props::{DisplayField, H5RunnableVariable};
use handlers::file::FileHandler;
use log::info;
use regex::Regex;
use serde_json::Value;
use std::path::Path;

pub struct H5FileHandler;

pub(crate) const H5_INSTALLED_CMDS: [&str; 4] = ["npm", "yarn", "pnpm", "cnpm"];

impl H5FileHandler {
    /// 获取默认的文件命令
    pub fn get_default_file_commands(url: &str) -> Option<H5RunnableVariable> {
        // display fields
        if GitHandler::is_remote_url(url) {
            info!("get default file commands failed, `{}` is a remote url!", url);
            return Some(H5RunnableVariable {
                display_fields: Self::get_display_fields(&Vec::new(), &Vec::new()),
                ..Default::default()
            });
        }

        let path = Path::new(url);
        if !path.exists() {
            info!("get default file commands failed, `{}` is not exists!", url);
            return None;
        }

        // Makefile
        let make_commands = Self::get_make_commands(url);

        // package.json
        let package_commands = Self::get_package_commands(url);

        let display_fields = Self::get_display_fields(&make_commands, &package_commands);
        return Some(H5RunnableVariable {
            display_fields,
            selected: None,
            node: String::new(),
            make_commands,
            installed_commands: Vec::new(),
            package_commands,
        });
    }
}

impl H5FileHandler {
    /// 获取 Makefile 命令
    fn get_make_commands(url: &str) -> Vec<String> {
        let filename = "Makefile";
        let path = Path::new(url).join(filename);

        if !path.exists() {
            info!("get make commands failed, `{}` has no `{}` file!", url, filename);
            return Vec::new();
        }

        let mut commands: Vec<String> = Vec::new();
        let path = path.as_path().to_str().unwrap();
        let contents = FileHandler::read_file_string(path).ok();
        if let Some(contents) = contents {
            let reg = Regex::new(r"^\s*([\w-]+):").ok();
            if let Some(reg) = reg {
                for line in contents.lines() {
                    if let Some(captures) = reg.captures(line) {
                        if let Some(target) = captures.get(1) {
                            let mut command = String::from("make ");
                            command.push_str(target.as_str());
                            commands.push(command)
                        }
                    }
                }
            }
        }

        println!("get make commands: {:#?}", commands);
        commands
    }

    /// 获取 package.json 中的命令
    fn get_package_commands(url: &str) -> Vec<String> {
        let filename = "package.json";
        let path = Path::new(url).join(filename);

        if !path.exists() {
            info!("get package commands failed, `{}` has no `{}` file!", url, filename);
            return Vec::new();
        }

        let mut commands: Vec<String> = Vec::new();
        let path = path.as_path().to_str().unwrap();
        let contents = FileHandler::read_file_string(path).ok();
        if let Some(contents) = contents {
            let value: Option<Value> = serde_json::from_str(&contents).ok();
            if let Some(value) = value {
                let scripts = value["scripts"].as_object();
                if let Some(scripts) = scripts {
                    for (name, _) in scripts {
                        commands.push(name.to_string())
                    }
                }
            }
        }

        println!("get package commands: {:#?}", commands);
        commands
    }

    // 判断本地安装的 npm、yarn、pnpm 命令
    pub(crate) fn get_installed_commands() -> Vec<String> {
        let npm = H5_INSTALLED_CMDS.get(0).unwrap();
        let yarn = H5_INSTALLED_CMDS.get(1).unwrap();
        let pnpm = H5_INSTALLED_CMDS.get(2).unwrap();
        let cnpm = H5_INSTALLED_CMDS.get(3).unwrap();

        let npm_installed = Helper::check_installed_command(npm);
        let yarn_installed = Helper::check_installed_command(yarn);
        let pnpm_installed = Helper::check_installed_command(pnpm);
        let cnpm_installed = Helper::check_installed_command(cnpm);

        let mut commands: Vec<String> = Vec::new();
        if npm_installed {
            commands.push(npm.to_string());
        }

        if yarn_installed {
            commands.push(yarn.to_string());
        }

        if pnpm_installed {
            commands.push(pnpm.to_string());
        }

        if cnpm_installed {
            commands.push(cnpm.to_string());
        }

        commands
    }

    /*
    /// 判断远程有没有某个文件
    pub(crate) fn get_file_by_branch(branch_name: &str, url: &str, file_name: &str) -> Result<(), String> {
        if !GitHandler::is_remote_url(url) {
            return Ok(())
        }

        let mut git_url = url.to_string();
        if git_url.starts_with(".sh") {
            return Ok(())
        }

        if git_url.ends_with(".git") {
            git_url = git_url.replace(".git", "");
        }

        let url = Url::parse(&git_url).map_err(|err| Error::Error(err.to_string()).to_string())?;
        let base_url = url.origin().unicode_serialization();
        info!("git remote base url: {}", base_url);

        // url: {git_url}/-/blob/{branch_name}/{file_name}?ref_type=heads
        let mut git_path = Path::new(&git_url).join("-").join("blob").join(branch_name);
        git_path.set_file_name(file_name);
        let mut git_path_url = git_path.to_string_lossy().to_string();
        git_path_url.push_str("?ref_type=heads");
        info!("git remote file url: {}", git_path_url);

        // 获取内容
        let response = reqwest::blocking::get(&git_path_url).map_err(|err| Error::Error(err.to_string()).to_string())?;
        if response.status().is_success() {
            let body = response.text().map_err(|err| Error::Error(err.to_string()).to_string())?;
            info!("{}", body);
            return Ok(())
        }

        return Err(Error::convert_string("can not git remote file: `{}` content!"))
    }
     */

    /// 获取展示列表
    fn get_display_fields(make_commands: &Vec<String>, package_commands: &Vec<String>) -> Vec<DisplayField> {
        let mut display_fields: Vec<DisplayField> = Vec::new();
        display_fields.push(DisplayField {
            label: "node".to_string(),
            value: "node".to_string(),
            show_type: "str".to_string(),
            desc: "NodeJs版本号".to_string(),
            key: "node".to_string(),
        });

        display_fields.push(DisplayField {
            label: "branch".to_string(),
            value: "branch".to_string(),
            show_type: "select".to_string(),
            desc: "分支列表".to_string(),
            key: "branches".to_string(),
        });

        if make_commands.len() > 0 {
            display_fields.push(DisplayField {
                label: "make".to_string(),
                value: "make".to_string(),
                show_type: "select".to_string(),
                desc: "Make命令".to_string(),
                key: "makeCommands".to_string(),
            });
        }

        if package_commands.len() > 0 {
            display_fields.push(DisplayField {
                label: "command".to_string(),
                value: "command".to_string(),
                show_type: "select".to_string(),
                desc: "本机安装的命令列表".to_string(),
                key: "installedCommands".to_string(),
            });
            display_fields.push(DisplayField {
                label: "script".to_string(),
                value: "script".to_string(),
                show_type: "select".to_string(),
                desc: "package.json中的scripts命令".to_string(),
                key: "packageCommands".to_string(),
            });
        }

        return display_fields;
    }
}
