use anyhow::{anyhow, Context, Result};
use std::fs;
use std::path::PathBuf;

/// Cookie 存放路径：~/.config/eastmoney-cli/cookie.txt（可用环境变量 EM_COOKIE 覆盖）
pub fn config_dir() -> Result<PathBuf> {
    let home = std::env::var("HOME").context("无法获取 HOME 目录")?;
    Ok(PathBuf::from(home).join(".config").join("eastmoney-cli"))
}

pub fn cookie_path() -> Result<PathBuf> {
    Ok(config_dir()?.join("cookie.txt"))
}

pub fn load_cookie() -> Option<String> {
    if let Ok(c) = std::env::var("EM_COOKIE") {
        if !c.trim().is_empty() {
            return Some(c.trim().to_string());
        }
    }
    let p = cookie_path().ok()?;
    let c = fs::read_to_string(p).ok()?;
    let c = c.trim().to_string();
    if c.is_empty() {
        None
    } else {
        Some(c)
    }
}

pub fn save_cookie(cookie: &str) -> Result<PathBuf> {
    let cookie = cookie.trim();
    if cookie.is_empty() {
        return Err(anyhow!("Cookie 为空"));
    }
    let dir = config_dir()?;
    fs::create_dir_all(&dir).with_context(|| format!("创建目录失败: {}", dir.display()))?;
    let path = dir.join("cookie.txt");
    fs::write(&path, cookie).with_context(|| format!("写入失败: {}", path.display()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(&path, fs::Permissions::from_mode(0o600));
    }
    Ok(path)
}
