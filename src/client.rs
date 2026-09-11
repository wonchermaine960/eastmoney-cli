use anyhow::{anyhow, Context, Result};
use serde_json::Value;
use std::sync::Mutex;
use std::time::{Duration, Instant};

const UA: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36";
pub const QT_UT: &str = "fa5fd1943c7b386f172d6893dbfba10b";
pub const ZTZT_UT: &str = "7eea3edcaed734bea9cbfc24409ed989";
pub const MYFAVOR_APPKEY: &str = "e9166c7e9cdfad3aa3fd7d93b757e9b1";

/// 实时行情主备域名：push2 有 IP 级限流，push2delay 为独立集群兜底
const QT_HOSTS: [&str; 2] = [
    "https://push2.eastmoney.com",
    "https://push2delay.eastmoney.com",
];
// 失败通常是 WAF 立即断连（非超时），多试几个镜像成本很低
const HIS_HOSTS: [&str; 4] = [
    "https://push2his.eastmoney.com",
    "https://33.push2his.eastmoney.com",
    "https://90.push2his.eastmoney.com",
    "https://48.push2his.eastmoney.com",
];

pub struct Http {
    c: reqwest::blocking::Client,
    last: Mutex<Instant>,
}

pub enum Body<'a> {
    None,
    Json(&'a Value),
    #[allow(dead_code)] // 预留给未来需要 form POST 的接口
    Form(&'a [(&'a str, &'a str)]),
}

impl Http {
    pub fn new() -> Result<Self> {
        let c = reqwest::blocking::Client::builder()
            .user_agent(UA)
            .timeout(Duration::from_secs(6))
            .gzip(true)
            .build()
            .context("构建 HTTP 客户端失败")?;
        Ok(Http {
            c,
            last: Mutex::new(Instant::now() - Duration::from_secs(1)),
        })
    }

    /// 全局限速：请求间隔 >= 150ms，避免触发东财 IP 限流
    fn throttle(&self) {
        let mut last = self.last.lock().unwrap();
        let gap = Duration::from_millis(150);
        let elapsed = last.elapsed();
        if elapsed < gap {
            std::thread::sleep(gap - elapsed);
        }
        *last = Instant::now();
    }

    pub fn request(
        &self,
        url: &str,
        query: &[(&str, &str)],
        body: Body,
        referer: &str,
        cookie: Option<&str>,
    ) -> Result<Value> {
        self.throttle();
        let mut req = match &body {
            Body::None => self.c.get(url),
            Body::Json(v) => self.c.post(url).json(v),
            Body::Form(f) => self.c.post(url).form(f),
        };
        if !query.is_empty() {
            req = req.query(query);
        }
        req = req.header("Referer", referer).header("Accept", "*/*");
        if let Some(ck) = cookie {
            req = req.header("Cookie", ck);
        }
        let resp = req.send().with_context(|| format!("请求失败: {}", url))?;
        let status = resp.status();
        let text = resp.text().context("读取响应失败")?;
        if !status.is_success() {
            return Err(anyhow!("HTTP {} @ {}: {}", status, url, &text[..text.len().min(200)]));
        }
        // 兼容 jsonp：cb({...});
        let trimmed = text.trim();
        let json_str = if let (Some(l), Some(r)) = (trimmed.find('('), trimmed.rfind(')')) {
            if !trimmed.starts_with('{') && !trimmed.starts_with('[') && l < r {
                &trimmed[l + 1..r]
            } else {
                trimmed
            }
        } else {
            trimmed
        };
        serde_json::from_str(json_str)
            .with_context(|| format!("响应不是 JSON: {}", &trimmed[..trimmed.len().min(200)]))
    }

    fn qt_hosts(&self, hosts: &[&str], path: &str, query: &[(&str, &str)]) -> Result<Value> {
        let mut last_err = None;
        for (i, host) in hosts.iter().enumerate() {
            let url = format!("{}{}", host, path);
            match self.request(&url, query, Body::None, "https://quote.eastmoney.com/", None) {
                Ok(v) => {
                    if i > 0 {
                        eprintln!("[提示] 主行情域名不可达（可能触发限流），已切换 {}", host);
                    }
                    return Ok(v);
                }
                Err(e) => last_err = Some(e),
            }
        }
        Err(last_err.unwrap_or_else(|| anyhow!("无可用行情域名")))
            .context("行情接口全部失败：可能触发了东财 IP 限流，请稍后重试或降低频率")
    }

    /// 实时类 qt 接口（自动降级 push2delay）
    pub fn qt(&self, path: &str, query: &[(&str, &str)]) -> Result<Value> {
        self.qt_hosts(&QT_HOSTS, path, query)
    }

    /// 历史类 qt 接口（K线，仅 push2his 集群提供）
    pub fn qt_his(&self, path: &str, query: &[(&str, &str)]) -> Result<Value> {
        self.qt_hosts(&HIS_HOSTS, path, query)
    }

    /// emappdata 股吧人气榜（POST JSON）
    pub fn emappdata(&self, method: &str, mut body: Value) -> Result<Value> {
        body["appId"] = Value::from("appId01");
        body["globalId"] = Value::from(global_id());
        let url = format!("https://emappdata.eastmoney.com/stockrank/{}", method);
        let v = self.request(&url, &[], Body::Json(&body), "https://guba.eastmoney.com/", None)?;
        if v.get("code").and_then(|c| c.as_i64()) != Some(0) {
            return Err(anyhow!("emappdata {} 返回异常: {}", method, v));
        }
        Ok(v)
    }

    /// datacenter 数据中心
    pub fn datacenter(&self, host: &str, extra: &[(&str, &str)]) -> Result<Value> {
        let url = format!("https://{}/api/data/v1/get", host);
        let v = self.request(&url, extra, Body::None, "https://data.eastmoney.com/", None)?;
        if v.get("result").map(|r| r.is_null()).unwrap_or(true) {
            let msg = v.get("message").and_then(|m| m.as_str()).unwrap_or("无数据");
            return Err(anyhow!("数据中心无结果: {}", msg));
        }
        Ok(v)
    }
}

/// 生成伪 uuid（emappdata 的 globalId 仅要求非空且格式合理）
fn global_id() -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let pid = std::process::id() as u128;
    let x = nanos ^ (pid << 64);
    format!(
        "{:08x}-{:04x}-{:04x}-{:04x}-{:012x}",
        (x >> 96) as u32,
        (x >> 80) as u16,
        (x >> 64) as u16,
        (x >> 48) as u16,
        x & 0xffff_ffff_ffff
    )
}
