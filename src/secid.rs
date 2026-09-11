use crate::client::{Body, Http};
use anyhow::{anyhow, Result};

#[derive(Debug, Clone)]
pub struct Sec {
    pub secid: String,
    pub code: String,
    pub name: Option<String>,
}

/// 输入 → secid 解析：
/// 1) "0.300339" 直接透传  2) sh/sz/bj 前缀  3) BKxxxx 板块
/// 4) 6 位纯数字按规则推断市场  5) 其余（中文名/拼音）走搜索接口
pub fn resolve(http: &Http, input: &str) -> Result<Sec> {
    let s = input.trim();
    if s.is_empty() {
        return Err(anyhow!("股票代码为空"));
    }
    // 已是 secid 形式
    if let Some((m, c)) = s.split_once('.') {
        if m.chars().all(|c| c.is_ascii_digit()) && !c.is_empty() {
            return Ok(Sec { secid: s.to_string(), code: c.to_string(), name: None });
        }
    }
    let lower = s.to_lowercase();
    for (pfx, mkt) in [("sh", "1"), ("sz", "0"), ("bj", "0")] {
        if let Some(rest) = lower.strip_prefix(pfx) {
            if rest.len() == 6 && rest.chars().all(|c| c.is_ascii_digit()) {
                return Ok(Sec { secid: format!("{}.{}", mkt, rest), code: rest.to_string(), name: None });
            }
        }
    }
    let upper = s.to_uppercase();
    if upper.starts_with("BK") && upper.len() == 6 {
        return Ok(Sec { secid: format!("90.{}", upper), code: upper.clone(), name: None });
    }
    if s.len() == 6 && s.chars().all(|c| c.is_ascii_digit()) {
        if let Some(mkt) = guess_market(s) {
            return Ok(Sec { secid: format!("{}.{}", mkt, s), code: s.to_string(), name: None });
        }
    }
    suggest(http, s)
}

/// 6 位代码市场规则（A股）。指数请用 sh/sz 前缀（如 sh000001 上证指数）。
fn guess_market(code: &str) -> Option<&'static str> {
    let p2: &str = &code[..2];
    let p3: &str = &code[..3];
    match () {
        _ if code.starts_with('6') => Some("1"),                    // 沪市股票/科创板
        _ if p3 == "399" => Some("0"),                              // 深市指数
        _ if code.starts_with('0') || code.starts_with('3') => Some("0"), // 深市股票
        _ if matches!(p2, "51" | "56" | "58" | "50" | "52") => Some("1"), // 沪市基金
        _ if matches!(p2, "15" | "16" | "18") => Some("0"),         // 深市基金
        _ if code.starts_with('4') || code.starts_with('8') || p2 == "92" => Some("0"), // 北交所
        _ => None,
    }
}

/// searchapi 搜索（支持中文名/拼音/代码），QuoteID 即 secid
pub fn suggest(http: &Http, kw: &str) -> Result<Sec> {
    let list = suggest_list(http, kw)?;
    let item = list
        .into_iter()
        .find(|it| {
            matches!(
                it.2.as_str(),
                "AStock" | "Index" | "Fund" | "BK" | "NEEQ" | "BSE" | "ETF" | "LOF"
            )
        })
        .ok_or_else(|| anyhow!("未找到证券: {}", kw))?;
    Ok(Sec { secid: item.0, code: item.1.0, name: Some(item.1.1) })
}

/// 返回 (secid, (code, name), classify) 列表
pub fn suggest_list(http: &Http, kw: &str) -> Result<Vec<(String, (String, String), String)>> {
    let v = http.request(
        "https://searchapi.eastmoney.com/api/suggest/get",
        &[("input", kw), ("type", "14"), ("count", "10")],
        Body::None,
        "https://www.eastmoney.com/",
        None,
    )?;
    let data = v
        .pointer("/QuotationCodeTable/Data")
        .and_then(|d| d.as_array())
        .cloned()
        .unwrap_or_default();
    Ok(data
        .iter()
        .filter_map(|it| {
            let quote_id = it.get("QuoteID")?.as_str()?.to_string();
            let code = it.get("Code")?.as_str()?.to_string();
            let name = it.get("Name")?.as_str()?.to_string();
            let classify = it.get("Classify").and_then(|c| c.as_str()).unwrap_or("").to_string();
            Some((quote_id, (code, name), classify))
        })
        .collect())
}

/// "SZ002185" / "SH600584" → secid（emappdata 人气榜格式）
pub fn from_rank_sc(sc: &str) -> Option<String> {
    let (mkt, code) = sc.split_at(2);
    let m = match mkt {
        "SH" => "1",
        "SZ" | "BJ" => "0",
        _ => return None,
    };
    Some(format!("{}.{}", m, code))
}

/// secid → emappdata 的 srcSecurityCode（如 0.002497 → SZ002497）
pub fn to_rank_sc(secid: &str) -> Option<String> {
    let (m, code) = secid.split_once('.')?;
    match m {
        "1" => Some(format!("SH{}", code)),
        "0" => {
            if code.starts_with('4') || code.starts_with('8') || code.starts_with("92") {
                Some(format!("BJ{}", code))
            } else {
                Some(format!("SZ{}", code))
            }
        }
        _ => None,
    }
}
