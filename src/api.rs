use crate::client::{Body, Http, QT_UT, ZTZT_UT};
use anyhow::{anyhow, Result};
use serde_json::{json, Value};

/// 全A股（沪深主板/创业板/科创板/北交所）clist 筛选
pub const FS_A_STOCK: &str = "m:0+t:6,m:0+t:80,m:1+t:2,m:1+t:23,m:0+t:81+s:2048";
pub const FS_INDUSTRY: &str = "m:90+t:2+f:!50";
pub const FS_CONCEPT: &str = "m:90+t:3+f:!50";
pub const FS_REGION: &str = "m:90+t:1+f:!50";

/// 常用行情字段（ulist/clist 通用）
pub const QUOTE_FIELDS: &str = "f1,f2,f3,f4,f5,f6,f7,f8,f9,f10,f12,f13,f14,f15,f16,f17,f18,f20,f21,f23,f62,f100";

/// 批量实时行情（自动分批，避免 URL 过长被拒）
pub fn quotes(http: &Http, secids: &[String], fields: &str) -> Result<Vec<Value>> {
    const CHUNK: usize = 80;
    if secids.len() > CHUNK {
        let mut all = Vec::new();
        for chunk in secids.chunks(CHUNK) {
            all.extend(quotes(http, chunk, fields)?);
        }
        return Ok(all);
    }
    let ids = secids.join(",");
    let v = http.qt(
        "/api/qt/ulist.np/get",
        &[
            ("secids", ids.as_str()),
            ("fields", fields),
            ("fltt", "2"),
            ("invt", "2"),
            ("ut", QT_UT),
        ],
    )?;
    let diff = v.pointer("/data/diff").cloned().unwrap_or(Value::Null);
    match diff {
        Value::Array(a) => Ok(a),
        // np=0 时 diff 可能是对象 {"0":{...}}
        Value::Object(o) => Ok(o.into_iter().map(|(_, x)| x).collect()),
        _ => Err(anyhow!("行情返回为空")),
    }
}

/// 单股快照（含估值、五档；五档仅实时集群返回）
pub fn stock_get(http: &Http, secid: &str) -> Result<Value> {
    let fields = "f43,f44,f45,f46,f47,f48,f50,f51,f52,f55,f57,f58,f60,f62,f71,f84,f85,f92,f116,f117,f162,f167,f168,f169,f170,f171,f31,f32,f33,f34,f35,f36,f37,f38,f39,f40,f19,f20,f17,f18,f15,f16,f13,f14,f11,f12";
    let v = http.qt(
        "/api/qt/stock/get",
        &[("secid", secid), ("fields", fields), ("fltt", "2"), ("invt", "2"), ("ut", QT_UT)],
    )?;
    v.get("data")
        .filter(|d| !d.is_null())
        .cloned()
        .ok_or_else(|| anyhow!("未找到行情数据: {}", secid))
}

/// 排行榜/板块列表
pub fn clist(http: &Http, fs: &str, fid: &str, asc: bool, num: usize, fields: &str) -> Result<Vec<Value>> {
    let pz = num.to_string();
    let po = if asc { "0" } else { "1" };
    let v = http.qt(
        "/api/qt/clist/get",
        &[
            ("pn", "1"),
            ("pz", pz.as_str()),
            ("po", po),
            ("np", "1"),
            ("fltt", "2"),
            ("invt", "2"),
            ("fid", fid),
            ("fs", fs),
            ("fields", fields),
            ("ut", QT_UT),
        ],
    )?;
    v.pointer("/data/diff")
        .and_then(|d| d.as_array())
        .cloned()
        .ok_or_else(|| anyhow!("列表返回为空"))
}

/// K线：klt 1/5/15/30/60/101日/102周/103月，fqt 0/1前复权/2后复权
/// 返回每根 K 线："日期,开,收,高,低,量,额,振幅,涨跌幅,涨跌额,换手"
pub fn kline(http: &Http, secid: &str, klt: &str, fqt: &str, num: usize) -> Result<(String, Vec<String>)> {
    let lmt = num.to_string();
    let v = http.qt_his(
        "/api/qt/stock/kline/get",
        &[
            ("secid", secid),
            ("fields1", "f1,f2,f3,f4,f5,f6"),
            ("fields2", "f51,f52,f53,f54,f55,f56,f57,f58,f59,f60,f61"),
            ("klt", klt),
            ("fqt", fqt),
            ("end", "20500101"),
            ("lmt", lmt.as_str()),
            ("ut", QT_UT),
        ],
    )?;
    let name = v.pointer("/data/name").and_then(|n| n.as_str()).unwrap_or("").to_string();
    let klines = v
        .pointer("/data/klines")
        .and_then(|k| k.as_array())
        .map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect::<Vec<_>>())
        .unwrap_or_default();
    if klines.is_empty() {
        return Err(anyhow!("K线为空（push2his 集群可能被限流，请稍后重试）"));
    }
    Ok((name, klines))
}

/// 分时："时间,价格,成交量,均价"
pub fn trends(http: &Http, secid: &str, ndays: &str) -> Result<(Value, Vec<String>)> {
    let v = http.qt(
        "/api/qt/stock/trends2/get",
        &[
            ("secid", secid),
            ("fields1", "f1,f2,f3,f8"),
            ("fields2", "f51,f53,f56,f58"),
            ("ndays", ndays),
            ("iscr", "0"),
            ("ut", QT_UT),
        ],
    )?;
    let data = v.get("data").filter(|d| !d.is_null()).cloned().ok_or_else(|| anyhow!("分时为空"))?;
    let list = data
        .get("trends")
        .and_then(|t| t.as_array())
        .map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect())
        .unwrap_or_default();
    Ok((data, list))
}

/// 个股资金流（日）："日期,主力,小单,中单,大单,超大单"（净流入，元）
/// 历史数据在 push2his 的 daykline 接口；被限流时降级为实时接口（仅当日）
pub fn fund_flow(http: &Http, secid: &str, num: usize) -> Result<Vec<String>> {
    let lmt = num.to_string();
    let params = [
        ("secid", secid),
        ("fields1", "f1,f2,f3,f7"),
        ("fields2", "f51,f52,f53,f54,f55,f56"),
        ("klt", "101"),
        ("lmt", lmt.as_str()),
        ("ut", QT_UT),
    ];
    let v = match http.qt_his("/api/qt/stock/fflow/daykline/get", &params) {
        Ok(v) => v,
        Err(_) => {
            eprintln!("[提示] 资金流历史接口不可达，降级为仅当日数据");
            http.qt("/api/qt/stock/fflow/kline/get", &params)?
        }
    };
    v.pointer("/data/klines")
        .and_then(|k| k.as_array())
        .map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect())
        .ok_or_else(|| anyhow!("资金流为空"))
}

/// 涨跌分布：返回 (涨跌幅桶, 家数)
pub fn zd_fenbu(http: &Http) -> Result<Vec<(i64, i64)>> {
    let v = http.request(
        "https://push2ex.eastmoney.com/getTopicZDFenBu",
        &[("ut", ZTZT_UT), ("dpt", "wz.ztzt")],
        Body::None,
        "https://quote.eastmoney.com/",
        None,
    )?;
    let arr = v
        .pointer("/data/fenbu")
        .and_then(|f| f.as_array())
        .cloned()
        .ok_or_else(|| anyhow!("涨跌分布为空"))?;
    let mut out = Vec::new();
    for item in arr {
        if let Some(obj) = item.as_object() {
            for (k, val) in obj {
                if let (Ok(bucket), Some(n)) = (k.parse::<i64>(), val.as_i64()) {
                    out.push((bucket, n));
                }
            }
        }
    }
    out.sort();
    Ok(out)
}

/// 涨停/跌停/炸板池。kind: ZT/DT/ZB。返回 data 对象（含 pool 数组与 tc 总数）
pub fn topic_pool(http: &Http, kind: &str, date: &str, num: usize) -> Result<Value> {
    let path = format!("https://push2ex.eastmoney.com/getTopic{}Pool", kind);
    let pagesize = num.to_string();
    let v = http.request(
        &path,
        &[
            ("ut", ZTZT_UT),
            ("dpt", "wz.ztzt"),
            ("Pageindex", "0"),
            ("pagesize", pagesize.as_str()),
            ("sort", if kind == "ZT" { "fbt:asc" } else { "fund:asc" }),
            ("date", date),
        ],
        Body::None,
        "https://quote.eastmoney.com/",
        None,
    )?;
    Ok(v.get("data").filter(|d| !d.is_null()).cloned().unwrap_or_else(|| json!({"pool": [], "tc": 0})))
}

/// 股吧人气榜 / 飙升榜
pub fn hot_rank(http: &Http, soar: bool, num: usize) -> Result<Vec<Value>> {
    let method = if soar { "getAllHisRcList" } else { "getAllCurrentList" };
    let mut body = json!({"pageNo": 1, "pageSize": num});
    if !soar {
        body["marketType"] = Value::from("");
    }
    let v = http.emappdata(method, body)?;
    v.get("data").and_then(|d| d.as_array()).cloned().ok_or_else(|| anyhow!("热榜为空"))
}

pub fn hot_current(http: &Http, sc: &str) -> Result<Value> {
    let v = http.emappdata("getCurrentLatest", json!({"srcSecurityCode": sc}))?;
    v.get("data").filter(|d| !d.is_null()).cloned().ok_or_else(|| anyhow!("无排名数据"))
}

pub fn hot_history(http: &Http, sc: &str) -> Result<Vec<Value>> {
    let v = http.emappdata("getHisList", json!({"srcSecurityCode": sc}))?;
    v.get("data").and_then(|d| d.as_array()).cloned().ok_or_else(|| anyhow!("无历史排名"))
}

/// 股吧帖子列表
pub fn guba_posts(http: &Http, code: &str, num: usize) -> Result<Vec<Value>> {
    let pagesize = num.to_string();
    let v = http.request(
        "https://gbapi.eastmoney.com/webarticlelist/api/Article/Articlelist",
        &[
            ("code", code),
            ("type", "0"),
            ("index", "1"),
            ("pageSize", pagesize.as_str()),
            ("deviceid", "100"),
            ("version", "200"),
            ("product", "Guba"),
            ("plat", "Web"),
        ],
        Body::None,
        "https://guba.eastmoney.com/",
        None,
    )?;
    v.get("re").and_then(|r| r.as_array()).cloned().ok_or_else(|| anyhow!("帖子列表为空"))
}

/// 公告列表
pub fn announcements(http: &Http, code: &str, num: usize) -> Result<Vec<Value>> {
    let page_size = num.to_string();
    let v = http.request(
        "https://np-anotice-stock.eastmoney.com/api/security/ann",
        &[
            ("sr", "-1"),
            ("page_size", page_size.as_str()),
            ("page_index", "1"),
            ("ann_type", "A"),
            ("stock_list", code),
        ],
        Body::None,
        "https://data.eastmoney.com/",
        None,
    )?;
    v.pointer("/data/list").and_then(|l| l.as_array()).cloned().ok_or_else(|| anyhow!("公告为空"))
}

/// 个股资讯（东财全站搜索接口，以代码为关键词，无需登录）
pub fn stock_news(http: &Http, code: &str, num: usize) -> Result<Vec<Value>> {
    let param = serde_json::json!({
        "uid": "",
        "keyword": code,
        "type": ["cmsArticleWebOld"],
        "client": "web",
        "clientType": "web",
        "clientVersion": "curr",
        "param": {"cmsArticleWebOld": {"searchScope": "default", "sort": "default", "pageIndex": 1, "pageSize": num, "preTag": "", "postTag": ""}},
    })
    .to_string();
    let v = http.request(
        "https://search-api-web.eastmoney.com/search/jsonp",
        &[("cb", ""), ("param", param.as_str())],
        Body::None,
        "https://so.eastmoney.com/",
        None,
    )?;
    v.pointer("/result/cmsArticleWebOld")
        .and_then(|d| d.as_array())
        .cloned()
        .ok_or_else(|| anyhow!("资讯为空"))
}

/// myfavor 自选股接口（需登录 Cookie）
pub fn myfavor(http: &Http, cookie: &str, method: &str, extra: &[(&str, &str)]) -> Result<Value> {
    let url = format!("https://myfavor.eastmoney.com/v4/webouter/{}", method);
    let mut params: Vec<(&str, &str)> = vec![("appkey", crate::client::MYFAVOR_APPKEY)];
    params.extend_from_slice(extra);
    let v = http.request(&url, &params, Body::None, "https://quote.eastmoney.com/zixuan/", Some(cookie))?;
    let state = v.get("state").and_then(|s| s.as_i64()).unwrap_or(-1);
    if state != 0 {
        return Err(anyhow!(
            "自选股接口异常 (state={}): {}。Cookie 可能已过期，请重新 em config set-cookie",
            state,
            v.get("message").and_then(|m| m.as_str()).unwrap_or("?")
        ));
    }
    Ok(v)
}

/// 自选股分组列表：[(gid, gname)]
pub fn watch_groups(http: &Http, cookie: &str) -> Result<Vec<(String, String)>> {
    let v = myfavor(http, cookie, "ggdefstkindexinfos", &[])?;
    let list = v
        .pointer("/data/ginfolist")
        .and_then(|g| g.as_array())
        .cloned()
        .ok_or_else(|| anyhow!("分组列表为空"))?;
    Ok(list
        .iter()
        .filter_map(|g| {
            Some((
                g.get("gid")?.as_str()?.to_string(),
                g.get("gname")?.as_str()?.to_string(),
            ))
        })
        .collect())
}

/// 某分组下的自选股 secid 列表（security 格式: 市场$代码$内部id）
pub fn watch_stocks(http: &Http, cookie: &str, gid: &str) -> Result<Vec<String>> {
    let v = myfavor(http, cookie, "gstkinfos", &[("g", gid)])?;
    let list = v
        .pointer("/data/stkinfolist")
        .and_then(|s| s.as_array())
        .cloned()
        .unwrap_or_default();
    Ok(list
        .iter()
        .filter_map(|it| {
            let sec = it.get("security")?.as_str()?;
            let mut parts = sec.split('$');
            let m = parts.next()?;
            let code = parts.next()?;
            if m.chars().all(|c| c.is_ascii_digit()) && !code.is_empty() {
                Some(format!("{}.{}", m, code))
            } else {
                None
            }
        })
        .collect())
}

/// datacenter-web 通用查询
pub fn dc_query(
    http: &Http,
    report: &str,
    filter: &str,
    sort_col: &str,
    sort_type: &str,
    num: usize,
    f10: bool,
) -> Result<Vec<Value>> {
    let page_size = num.to_string();
    let mut params: Vec<(&str, &str)> = vec![
        ("reportName", report),
        ("columns", "ALL"),
        ("pageNumber", "1"),
        ("pageSize", page_size.as_str()),
    ];
    if !filter.is_empty() {
        params.push(("filter", filter));
    }
    if !sort_col.is_empty() {
        params.push(("sortColumns", sort_col));
        params.push(("sortTypes", sort_type));
    }
    let host = if f10 {
        params.push(("source", "HSF10"));
        params.push(("client", "PC"));
        "datacenter.eastmoney.com/securities"
    } else {
        "datacenter-web.eastmoney.com"
    };
    let v = http.datacenter(host, &params)?;
    v.pointer("/result/data")
        .and_then(|d| d.as_array())
        .cloned()
        .ok_or_else(|| anyhow!("{} 无数据", report))
}
