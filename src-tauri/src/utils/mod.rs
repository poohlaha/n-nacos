use base64::Engine;

pub mod file;

pub struct Utils;

impl Utils {
    /// 生成 base64 图片
    pub fn generate_image(data: Vec<u8>) -> String {
        let str = base64::engine::general_purpose::STANDARD.encode::<Vec<u8>>(data);
        let mut content = String::from("data:image/png;base64,");
        content.push_str(&str);
        content
    }
}
