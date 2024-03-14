//! git 操作

pub(crate) mod pull;

use crate::helper::git::pull::{GitConfig, GitHelper};
use git2::{BranchType, Repository};
use log::info;
use std::process::Command;

pub struct GitHandler;

impl GitHandler {
    /// 拼装 url
    fn get_url(url: &str) -> String {
        let mut validate_url = String::from(url);
        if validate_url.is_empty() {
            return String::new();
        }

        if !validate_url.ends_with(".git") {
            validate_url.push_str(".git");
        }

        return validate_url;
    }

    /// 判断本地 Git 地址是否可用
    #[allow(dead_code)]
    pub(crate) fn validate_local_url(url: &str) -> bool {
        if url.is_empty() {
            return false;
        }

        return match Repository::open(url) {
            Ok(_) => true,
            Err(err) => {
                info!("validate url: {} error: {:#?}", url, err);
                false
            }
        };
    }

    /// 判断远程 Git 地址是否可用
    pub(crate) fn validate_remote_url(url: &str) -> bool {
        let validate_url = Self::get_url(url);
        if validate_url.is_empty() {
            return false;
        }

        let output = Command::new("git").arg("ls-remote").arg(url).output();
        return match output {
            Ok(output) => {
                if output.status.success() {
                    return true;
                }

                let stderr = String::from_utf8_lossy(&output.stderr);
                if !stderr.is_empty() {
                    info!("validate remote url: {} error: {:#?}", url, stderr);
                    return false;
                }

                info!("validate remote url: {} failed, error status: {:#?}", url, output.status);
                return false;
            }
            Err(err) => {
                info!("validate remote url: {} error: {:#?}", url, err);
                false
            }
        };
    }

    /// 判断是否为远程地址
    pub(crate) fn is_remote_url(url: &str) -> bool {
        return url.starts_with("http://") || url.starts_with("https://") || url.starts_with("ssh://");
    }

    /// 获取 branch 列表
    pub(crate) fn get_branch_list(url: &str) -> Vec<String> {
        let is_remote_url = Self::is_remote_url(url);
        let branches: Vec<String>;
        if is_remote_url {
            // remote
            branches = Self::get_remote_url(url);
        } else {
            // ls
            branches = Self::get_local_branch(url);
        }

        branches
    }
}

impl GitHandler {
    /// 获取本地分支列表
    fn get_local_branch(url: &str) -> Vec<String> {
        if url.is_empty() {
            return Vec::new();
        }

        let repo = Repository::open(url).ok();
        if repo.is_none() {
            info!("`{}` is not a git project !", url);
            return Vec::new();
        }

        let mut git_branches: Vec<String> = Vec::new();
        if let Some(repo) = repo {
            let branches = repo.branches(Some(BranchType::Local)).ok();
            if let Some(branches) = branches {
                for branch in branches {
                    if let Ok((branch, _)) = branch {
                        let branch_name = branch.name().ok();
                        if let Some(branch_name) = branch_name {
                            if let Some(branch_name) = branch_name {
                                info!("get local branch name: {}", branch_name);
                                git_branches.push(branch_name.trim().to_string())
                            }
                        }
                    }
                }
            } else {
                println!("no branches found!");
            }
        }

        return git_branches;
    }

    /// 获取远程分支列表
    fn get_remote_url(url: &str) -> Vec<String> {
        let path = Self::get_url(url);
        if path.is_empty() {
            return Vec::new();
        }

        let output = Command::new("git").args(&["ls-remote", "--heads", &path]).output();
        return match output {
            Ok(output) => {
                if !output.status.success() {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    if !stderr.is_empty() {
                        info!("get remote url branches `{}` error: {:#?}", url, stderr);
                        return Vec::new();
                    }

                    info!("get remote url branches `{}` failed, error status: {:#?}", url, output.status);
                    return Vec::new();
                }

                let result = String::from_utf8_lossy(&output.stdout);
                let mut branches: Vec<String> = Vec::new();

                result.lines().for_each(|line| {
                    let line = line.trim();
                    let line = line.split_whitespace().last();
                    if let Some(line) = line {
                        let branch = line.split("/").last();
                        if let Some(branch) = branch {
                            branches.push(branch.to_string());
                        }
                    }
                });

                branches
            }
            Err(err) => {
                info!("get remote url branches `{}` error: {:#?}", url, err);
                Vec::new()
            }
        };
    }

    ///  获取项目名称
    pub(crate) fn get_project_name_by_git(url: &str) -> String {
        let parts: Vec<&str> = url.split('/').collect();

        // 获取最后一个，即名称
        if let Some(last_part) = parts.last() {
            let project_name = last_part.trim_end_matches(".git");
            return String::from(project_name);
        }

        return String::new();
    }
}

impl GitHandler {
    /// 代码拉取
    pub(crate) fn pull<F>(config: &GitConfig, func: F) -> Result<bool, String>
    where
        F: Fn(&str) + Send + Sync + 'static,
    {
        GitHelper::pull(config, func)
    }
}
