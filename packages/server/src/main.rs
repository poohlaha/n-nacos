/**!
    n-nacos 服务端
*/
use crate::env::Env;
use actix_web::{middleware, App, HttpServer};
use clap::Parser;
use colored::Colorize;
use std::error::Error;

// mods
mod env;

#[derive(Parser, Clone, Debug)]
#[clap(author, version, about, long_about = None)]
pub struct Args {
    #[arg(short, long, default_value = "")]
    pub env_file: String, // 环境配置文件名
}

#[actix_web::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // 1. 初始化
    let configs = Env::init();
    if configs.is_none() {
        let error = "Failed to initialize configuration !";
        return Err(error.into());
    }

    log::info!("{}", "Successfully initialized configuration !".cyan().bold());

    let configs = configs.unwrap_or_default();
    // 启动服务
    let mut server = HttpServer::new(move || App::new().wrap(middleware::Logger::default()));

    server = server.workers(configs.http_workers as usize);
    log::info!("{}", "nacos started !".cyan().bold());
    server.bind(configs.http_address)?.run().await?;
    Ok(())
}
