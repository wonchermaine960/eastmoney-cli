use crate::api;
use crate::client::Http;
use crate::config;
use crate::secid::{self, Sec};
use crate::util::*;
use anyhow::{anyhow, Result};
use serde_json::{json, Value};

pub struct Ctx {
    pub http: Http,
    pub json: bool,
}

impl Ctx {
    fn emit(&self, v: &Value) -> bool {
        if self.json {
            println!("{}", serde_json::to_string_pretty(v).unwrap());
        }
        self.json
    }
}

// ---------- 搜索 ----------

pub fn search(ctx: &Ctx, keyword: &str) -> Result<()> {
    let list = secid::suggest_list(&ctx.http, keyword)?;
    if list.is_empty() {
        return Err(anyhow!("未搜到: {}", keyword));
    }
    if ctx.json {
        let out: Vec<Value> = list
            .iter()
            .map(|(secid, (code, name), classify)| {
                json!({"secid": secid, "code": code, "name": name, "type": classify})
            })
            .collect();
        ctx.emit(&Value::Array(out));
        return Ok(());
    }
    let mut t = Table::new(&["代码", "名称", "类型", "secid"], vec![Align::Left, Align::Left, Align::Left, Align::Left]);
    for (secid, (code, name), classify) in list {
        t.row(vec![code, name, classify, secid]);
    }
    t.print();
    Ok(())
}

// ---------- 行情 ----------

fn quote_json(it: &Value) -> Value {
    json!({
        "code": str_at(it, "f12"),
        "name": str_at(it, "f14"),
        "price": num_at(it, "f2"),
        "pct_chg": num_at(it, "f3"),
        "chg": num_at(it, "f4"),
        "open": num_at(it, "f17"),
        "high": num_at(it, "f15"),
        "low": num_at(it, "f16"),
        "pre_close": num_at(it, "f18"),
        "volume_hand": num_at(it, "f5"),
        "amount": num_at(it, "f6"),
        "turnover_pct": num_at(it, "f8"),
        "volume_ratio": num_at(it, "f10"),
        "pe_dyn": num_at(it, "f9"),
        "pb": num_at(it, "f23"),
        "total_mv": num_at(it, "f20"),
        "float_mv": num_at(it, "f21"),
        "main_inflow": num_at(it, "f62"),
        "industry": str_at(it, "f100"),
    })
}

fn quote_table(items: &[Value]) {
    let mut t = Table::new(
        &["代码", "名称", "最新", "涨跌幅", "今开", "最高", "最低", "成交量", "成交额", "换手", "量比", "市盈(动)", "总市值", "行业"],
        vec![
            Align::Left, Align::Left, Align::Right, Align::Right, Align::Right, Align::Right,
            Align::Right, Align::Right, Align::Right, Align::Right, Align::Right, Align::Right,
            Align::Right, Align::Left,
        ],
    );
    for it in items {
        t.row(vec![
            str_at(it, "f12"),
            str_at(it, "f14"),
            fmt_opt(num_at(it, "f2"), 2),
            colorize_pct(num_at(it, "f3")),
            fmt_opt(num_at(it, "f17"), 2),
            fmt_opt(num_at(it, "f15"), 2),
            fmt_opt(num_at(it, "f16"), 2),
            num_at(it, "f5").map(fmt_vol).unwrap_or("-".into()),
            num_at(it, "f6").map(fmt_amount).unwrap_or("-".into()),
            num_at(it, "f8").map(|x| format!("{:.2}%", x)).unwrap_or("-".into()),
            fmt_opt(num_at(it, "f10"), 2),
            fmt_opt(num_at(it, "f9"), 1),
            num_at(it, "f20").map(fmt_amount).unwrap_or("-".into()),
            str_at(it, "f100"),
        ]);
    }
    t.print();
}

pub fn quote(ctx: &Ctx, codes: &[String]) -> Result<()> {
    let mut secids = Vec::new();
    for c in codes {
        secids.push(secid::resolve(&ctx.http, c)?.secid);
    }
    let items = api::quotes(&ctx.http, &secids, api::QUOTE_FIELDS)?;
    if ctx.json {
        let out: Vec<Value> = items.iter().map(quote_json).collect();
        ctx.emit(&Value::Array(out));
    } else {
        quote_table(&items);
    }
    Ok(())
}

// ---------- 单股详情 ----------

pub fn detail(ctx: &Ctx, code: &str) -> Result<()> {
    let sec = secid::resolve(&ctx.http, code)?;
    let d = api::stock_get(&ctx.http, &sec.secid)?;
    if ctx.json {
        let mut out = json!({
            "secid": sec.secid,
            "code": str_at(&d, "f57"),
            "name": str_at(&d, "f58"),
            "price": num_at(&d, "f43"),
            "pct_chg": num_at(&d, "f170"),
            "chg": num_at(&d, "f169"),
            "open": num_at(&d, "f46"),
            "high": num_at(&d, "f44"),
            "low": num_at(&d, "f45"),
            "pre_close": num_at(&d, "f60"),
            "avg_price": num_at(&d, "f71"),
            "volume_hand": num_at(&d, "f47"),
            "amount": num_at(&d, "f48"),
            "turnover_pct": num_at(&d, "f168"),
            "volume_ratio": num_at(&d, "f50"),
            "amplitude_pct": num_at(&d, "f171"),
            "limit_up": num_at(&d, "f51"),
            "limit_down": num_at(&d, "f52"),
            "eps": num_at(&d, "f55"),
            "bps": num_at(&d, "f92"),
            "pe_dyn": num_at(&d, "f162"),
            "pb": num_at(&d, "f167"),
            "total_mv": num_at(&d, "f116"),
            "float_mv": num_at(&d, "f117"),
            "total_shares": num_at(&d, "f84"),
            "float_shares": num_at(&d, "f85"),
        });
        if num_at(&d, "f39").is_some() {
            out["order_book"] = json!({
                "asks": [
                    {"price": num_at(&d, "f39"), "vol": num_at(&d, "f40")},
                    {"price": num_at(&d, "f37"), "vol": num_at(&d, "f38")},
                    {"price": num_at(&d, "f35"), "vol": num_at(&d, "f36")},
                    {"price": num_at(&d, "f33"), "vol": num_at(&d, "f34")},
                    {"price": num_at(&d, "f31"), "vol": num_at(&d, "f32")},
                ],
                "bids": [
                    {"price": num_at(&d, "f19"), "vol": num_at(&d, "f20")},
                    {"price": num_at(&d, "f17"), "vol": num_at(&d, "f18")},
                    {"price": num_at(&d, "f15"), "vol": num_at(&d, "f16")},
                    {"price": num_at(&d, "f13"), "vol": num_at(&d, "f14")},
                    {"price": num_at(&d, "f11"), "vol": num_at(&d, "f12")},
                ],
            });
        }
        ctx.emit(&out);
        return Ok(());
    }
    println!("{} ({})  [{}]", str_at(&d, "f58"), str_at(&d, "f57"), sec.secid);
    println!(
        "最新: {}  涨跌: {} ({})  振幅: {}%",
        fmt_opt(num_at(&d, "f43"), 2),
        colorize_pct(num_at(&d, "f170")),
        fmt_opt(num_at(&d, "f169"), 2),
        fmt_opt(num_at(&d, "f171"), 2),
    );
    println!(
        "今开: {}  昨收: {}  最高: {}  最低: {}  均价: {}",
        fmt_opt(num_at(&d, "f46"), 2),
        fmt_opt(num_at(&d, "f60"), 2),
        fmt_opt(num_at(&d, "f44"), 2),
        fmt_opt(num_at(&d, "f45"), 2),
        fmt_opt(num_at(&d, "f71"), 2),
    );
    println!(
        "成交量: {}  成交额: {}  换手: {}%  量比: {}",
        num_at(&d, "f47").map(fmt_vol).unwrap_or("-".into()),
        num_at(&d, "f48").map(fmt_amount).unwrap_or("-".into()),
        fmt_opt(num_at(&d, "f168"), 2),
        fmt_opt(num_at(&d, "f50"), 2),
    );
    println!(
        "涨停: {}  跌停: {}",
        fmt_opt(num_at(&d, "f51"), 2),
        fmt_opt(num_at(&d, "f52"), 2),
    );
    println!(
        "总市值: {}  流通市值: {}  PE(动): {}  PB: {}",
        num_at(&d, "f116").map(fmt_amount).unwrap_or("-".into()),
        num_at(&d, "f117").map(fmt_amount).unwrap_or("-".into()),
        fmt_opt(num_at(&d, "f162"), 2),
        fmt_opt(num_at(&d, "f167"), 2),
    );
    println!(
        "每股收益: {}  每股净资产: {}",
        fmt_opt(num_at(&d, "f55"), 3),
        fmt_opt(num_at(&d, "f92"), 3),
    );
    if num_at(&d, "f39").is_some() {
        println!("\n五档盘口:");
        let asks = [("卖五", "f31", "f32"), ("卖四", "f33", "f34"), ("卖三", "f35", "f36"), ("卖二", "f37", "f38"), ("卖一", "f39", "f40")];
        let bids = [("买一", "f19", "f20"), ("买二", "f17", "f18"), ("买三", "f15", "f16"), ("买四", "f13", "f14"), ("买五", "f11", "f12")];
        for (label, pk, vk) in asks.iter().chain(bids.iter()) {
            println!(
                "  {}  {:>8}  {:>10}",
                label,
                fmt_opt(num_at(&d, pk), 2),
                num_at(&d, vk).map(|x| format!("{:.0}手", x)).unwrap_or("-".into()),
            );
        }
    } else {
        println!("\n(五档盘口暂不可用：备用行情集群不提供盘口数据)");
    }
    Ok(())
}

// ---------- K线 ----------

pub fn kline(ctx: &Ctx, code: &str, period: &str, num: usize, fq: &str) -> Result<()> {
    let sec = secid::resolve(&ctx.http, code)?;
    let klt = match period {
        "day" | "d" | "101" => "101",
        "week" | "w" | "102" => "102",
        "month" | "m" | "103" => "103",
        "5m" | "5" => "5",
        "15m" | "15" => "15",
        "30m" | "30" => "30",
        "60m" | "60" => "60",
        _ => return Err(anyhow!("period 仅支持 day/week/month/5m/15m/30m/60m")),
    };
    let fqt = match fq {
        "qfq" => "1",
        "hfq" => "2",
        "none" => "0",
        _ => return Err(anyhow!("fq 仅支持 qfq/hfq/none")),
    };
    let (name, klines) = api::kline(&ctx.http, &sec.secid, klt, fqt, num)?;
    if ctx.json {
        let out: Vec<Value> = klines
            .iter()
            .filter_map(|l| {
                let p: Vec<&str> = l.split(',').collect();
                if p.len() < 11 {
                    return None;
                }
                Some(json!({
                    "date": p[0], "open": p[1].parse::<f64>().ok(), "close": p[2].parse::<f64>().ok(),
                    "high": p[3].parse::<f64>().ok(), "low": p[4].parse::<f64>().ok(),
                    "volume_hand": p[5].parse::<f64>().ok(), "amount": p[6].parse::<f64>().ok(),
                    "amplitude_pct": p[7].parse::<f64>().ok(), "pct_chg": p[8].parse::<f64>().ok(),
                    "chg": p[9].parse::<f64>().ok(), "turnover_pct": p[10].parse::<f64>().ok(),
                }))
            })
            .collect();
        ctx.emit(&json!({"code": sec.code, "name": name, "period": period, "fq": fq, "klines": out}));
        return Ok(());
    }
    println!("{} ({})  {}  复权:{}", name, sec.code, period, fq);
    let mut t = Table::new(
        &["日期", "开盘", "收盘", "最高", "最低", "涨跌幅", "成交量", "成交额", "换手"],
        vec![Align::Left, Align::Right, Align::Right, Align::Right, Align::Right, Align::Right, Align::Right, Align::Right, Align::Right],
    );
    for l in &klines {
        let p: Vec<&str> = l.split(',').collect();
        if p.len() < 11 {
            continue;
        }
        t.row(vec![
            p[0].into(),
            p[1].into(),
            p[2].into(),
            p[3].into(),
            p[4].into(),
            colorize_pct(p[8].parse().ok()),
            p[5].parse::<f64>().map(fmt_vol).unwrap_or("-".into()),
            p[6].parse::<f64>().map(fmt_amount).unwrap_or("-".into()),
            format!("{}%", p[10]),
        ]);
    }
    t.print();
    Ok(())
}

// ---------- 分时 ----------

pub fn trend(ctx: &Ctx, code: &str, days: usize, num: usize) -> Result<()> {
    let sec = secid::resolve(&ctx.http, code)?;
    let nd = days.to_string();
    let (data, list) = api::trends(&ctx.http, &sec.secid, &nd)?;
    let pre = num_at(&data, "preClose");
    if ctx.json {
        let out: Vec<Value> = list
            .iter()
            .filter_map(|l| {
                let p: Vec<&str> = l.split(',').collect();
                if p.len() < 4 {
                    return None;
                }
                Some(json!({"time": p[0], "price": p[1].parse::<f64>().ok(), "volume_hand": p[2].parse::<f64>().ok(), "avg_price": p[3].parse::<f64>().ok()}))
            })
            .collect();
        ctx.emit(&json!({"code": str_at(&data, "code"), "name": str_at(&data, "name"), "pre_close": pre, "trends": out}));
        return Ok(());
    }
    let disp_name = match str_at(&data, "name") {
        n if n == "-" => sec.name.clone().unwrap_or_default(),
        n => n,
    };
    println!("{} ({})  昨收: {}  (共{}个分时点，显示最近{}个)", disp_name, str_at(&data, "code"), fmt_opt(pre, 2), list.len(), num.min(list.len()));
    let mut t = Table::new(
        &["时间", "价格", "涨跌幅", "均价", "成交量"],
        vec![Align::Left, Align::Right, Align::Right, Align::Right, Align::Right],
    );
    let start = list.len().saturating_sub(num);
    for l in &list[start..] {
        let p: Vec<&str> = l.split(',').collect();
        if p.len() < 4 {
            continue;
        }
        let price: Option<f64> = p[1].parse().ok();
        let pct = match (price, pre) {
            (Some(x), Some(pc)) if pc > 0.0 => Some((x - pc) / pc * 100.0),
            _ => None,
        };
        t.row(vec![
            p[0].into(),
            p[1].into(),
            colorize_pct(pct),
            p[3].into(),
            p[2].parse::<f64>().map(fmt_vol).unwrap_or("-".into()),
        ]);
    }
    t.print();
    Ok(())
}

// ---------- 大盘 ----------

pub fn market(ctx: &Ctx) -> Result<()> {
    let index_ids: Vec<String> = [
        "1.000001", "0.399001", "0.399006", "1.000300", "1.000688", "1.000016", "1.000905", "1.000852",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();
    let idx = api::quotes(&ctx.http, &index_ids, "f2,f3,f4,f5,f6,f12,f14")?;
    let fenbu = api::zd_fenbu(&ctx.http).unwrap_or_default();
    let up: i64 = fenbu.iter().filter(|(b, _)| *b > 0).map(|(_, n)| n).sum();
    let down: i64 = fenbu.iter().filter(|(b, _)| *b < 0).map(|(_, n)| n).sum();
    let flat: i64 = fenbu.iter().filter(|(b, _)| *b == 0).map(|(_, n)| n).sum();
    // 沪市总成交 ≈ 上证指数成交额，深市 ≈ 深证成指成交额
    let sh_amt = idx.iter().find(|i| str_at(i, "f12") == "000001").and_then(|i| num_at(i, "f6"));
    let sz_amt = idx.iter().find(|i| str_at(i, "f12") == "399001").and_then(|i| num_at(i, "f6"));
    let total_amt = match (sh_amt, sz_amt) {
        (Some(a), Some(b)) => Some(a + b),
        _ => None,
    };
    let today = today_str(0);
    let zt = api::topic_pool(&ctx.http, "ZT", &today, 1).ok();
    let dt = api::topic_pool(&ctx.http, "DT", &today, 1).ok();
    let ztc = zt.as_ref().and_then(|d| d.get("tc")).and_then(|x| x.as_i64());
    let dtc = dt.as_ref().and_then(|d| d.get("tc")).and_then(|x| x.as_i64());

    if ctx.json {
        let indexes: Vec<Value> = idx
            .iter()
            .map(|it| {
                json!({"code": str_at(it, "f12"), "name": str_at(it, "f14"), "price": num_at(it, "f2"), "pct_chg": num_at(it, "f3"), "chg": num_at(it, "f4"), "amount": num_at(it, "f6")})
            })
            .collect();
        ctx.emit(&json!({
            "indexes": indexes,
            "up_count": up, "down_count": down, "flat_count": flat,
            "limit_up_count": ztc, "limit_down_count": dtc,
            "total_amount": total_amt,
            "fenbu": fenbu.iter().map(|(b, n)| json!({"bucket": b, "count": n})).collect::<Vec<_>>(),
        }));
        return Ok(());
    }
    let mut t = Table::new(
        &["指数", "点位", "涨跌幅", "涨跌", "成交额"],
        vec![Align::Left, Align::Right, Align::Right, Align::Right, Align::Right],
    );
    for it in &idx {
        t.row(vec![
            str_at(it, "f14"),
            fmt_opt(num_at(it, "f2"), 2),
            colorize_pct(num_at(it, "f3")),
            fmt_opt(num_at(it, "f4"), 2),
            num_at(it, "f6").map(fmt_amount).unwrap_or("-".into()),
        ]);
    }
    t.print();
    println!();
    println!(
        "市场概况: 上涨 {} 家 / 下跌 {} 家 / 平盘 {} 家   涨停 {}  跌停 {}",
        up,
        down,
        flat,
        ztc.map(|x| x.to_string()).unwrap_or("-".into()),
        dtc.map(|x| x.to_string()).unwrap_or("-".into()),
    );
    if let Some(a) = total_amt {
        println!("两市成交额(约): {}", fmt_amount(a));
    }
    if !fenbu.is_empty() {
        println!("\n涨跌分布 (涨跌幅% → 家数):");
        let line: Vec<String> = fenbu.iter().map(|(b, n)| format!("{:+}:{}", b, n)).collect();
        println!("  {}", line.join("  "));
    }
    Ok(())
}

// ---------- 排行 ----------

fn rank_fid(by: &str) -> Result<&'static str> {
    Ok(match by {
        "change" | "pct" => "f3",
        "amount" => "f6",
        "turnover" => "f8",
        "volume" => "f5",
        "pe" => "f9",
        "mcap" | "market-cap" => "f20",
        "flow" | "main-inflow" => "f62",
        "speed" => "f22",
        _ => return Err(anyhow!("--by 仅支持 change/amount/turnover/volume/pe/mcap/flow/speed")),
    })
}

pub fn rank(ctx: &Ctx, by: &str, asc: bool, num: usize) -> Result<()> {
    let fid = rank_fid(by)?;
    let items = api::clist(&ctx.http, api::FS_A_STOCK, fid, asc, num, api::QUOTE_FIELDS)?;
    if ctx.json {
        let out: Vec<Value> = items.iter().map(quote_json).collect();
        ctx.emit(&Value::Array(out));
    } else {
        quote_table(&items);
    }
    Ok(())
}

// ---------- 板块 ----------

pub fn board(ctx: &Ctx, kind: &str, by: &str, asc: bool, num: usize) -> Result<()> {
    let fs = match kind {
        "industry" | "hy" => api::FS_INDUSTRY,
        "concept" | "gn" => api::FS_CONCEPT,
        "region" | "dy" => api::FS_REGION,
        _ => return Err(anyhow!("板块类型仅支持 industry/concept/region")),
    };
    let fid = rank_fid(by)?;
    let fields = "f2,f3,f4,f8,f12,f14,f20,f62,f104,f105,f128,f140";
    let items = api::clist(&ctx.http, fs, fid, asc, num, fields)?;
    if ctx.json {
        let out: Vec<Value> = items
            .iter()
            .map(|it| {
                json!({
                    "code": str_at(it, "f12"), "name": str_at(it, "f14"),
                    "pct_chg": num_at(it, "f3"), "total_mv": num_at(it, "f20"),
                    "turnover_pct": num_at(it, "f8"), "main_inflow": num_at(it, "f62"),
                    "up_count": num_at(it, "f104"), "down_count": num_at(it, "f105"),
                    "leader": str_at(it, "f128"), "leader_code": str_at(it, "f140"),
                })
            })
            .collect();
        ctx.emit(&Value::Array(out));
        return Ok(());
    }
    let mut t = Table::new(
        &["代码", "板块", "涨跌幅", "换手", "主力净流入", "涨/跌家数", "领涨股"],
        vec![Align::Left, Align::Left, Align::Right, Align::Right, Align::Right, Align::Right, Align::Left],
    );
    for it in &items {
        t.row(vec![
            str_at(it, "f12"),
            str_at(it, "f14"),
            colorize_pct(num_at(it, "f3")),
            num_at(it, "f8").map(|x| format!("{:.2}%", x)).unwrap_or("-".into()),
            num_at(it, "f62").map(fmt_amount).unwrap_or("-".into()),
            format!(
                "{}/{}",
                num_at(it, "f104").map(|x| x as i64).unwrap_or(0),
                num_at(it, "f105").map(|x| x as i64).unwrap_or(0)
            ),
            str_at(it, "f128"),
        ]);
    }
    t.print();
    Ok(())
}

pub fn board_stocks(ctx: &Ctx, code: &str, by: &str, asc: bool, num: usize) -> Result<()> {
    let bk = if code.to_uppercase().starts_with("BK") {
        code.to_uppercase()
    } else {
        secid::resolve(&ctx.http, code)?.code
    };
    let fs = format!("b:{}", bk);
    let fid = rank_fid(by)?;
    let items = api::clist(&ctx.http, &fs, fid, asc, num, api::QUOTE_FIELDS)?;
    if ctx.json {
        let out: Vec<Value> = items.iter().map(quote_json).collect();
        ctx.emit(&Value::Array(out));
    } else {
        quote_table(&items);
    }
    Ok(())
}

// ---------- 资金流 ----------

pub fn flow(ctx: &Ctx, code: &str, num: usize) -> Result<()> {
    let sec = secid::resolve(&ctx.http, code)?;
    let lines = api::fund_flow(&ctx.http, &sec.secid, num)?;
    if ctx.json {
        let out: Vec<Value> = lines
            .iter()
            .filter_map(|l| {
                let p: Vec<&str> = l.split(',').collect();
                if p.len() < 6 {
                    return None;
                }
                Some(json!({
                    "date": p[0],
                    "main": p[1].parse::<f64>().ok(),
                    "small": p[2].parse::<f64>().ok(),
                    "medium": p[3].parse::<f64>().ok(),
                    "large": p[4].parse::<f64>().ok(),
                    "super_large": p[5].parse::<f64>().ok(),
                }))
            })
            .collect();
        ctx.emit(&json!({"code": sec.code, "flows": out}));
        return Ok(());
    }
    println!("资金流向（净流入） {} [{}]", sec.name.as_deref().unwrap_or(""), sec.code);
    let mut t = Table::new(
        &["日期", "主力", "超大单", "大单", "中单", "小单"],
        vec![Align::Left, Align::Right, Align::Right, Align::Right, Align::Right, Align::Right],
    );
    for l in &lines {
        let p: Vec<&str> = l.split(',').collect();
        if p.len() < 6 {
            continue;
        }
        let f = |s: &str| s.parse::<f64>().map(fmt_amount).unwrap_or("-".into());
        t.row(vec![p[0].into(), f(p[1]), f(p[5]), f(p[4]), f(p[3]), f(p[2])]);
    }
    t.print();
    Ok(())
}

pub fn flow_rank(ctx: &Ctx, num: usize) -> Result<()> {
    let fields = "f2,f3,f12,f14,f62,f66,f72,f78,f84,f184";
    let items = api::clist(&ctx.http, api::FS_A_STOCK, "f62", false, num, fields)?;
    if ctx.json {
        let out: Vec<Value> = items
            .iter()
            .map(|it| {
                json!({
                    "code": str_at(it, "f12"), "name": str_at(it, "f14"),
                    "price": num_at(it, "f2"), "pct_chg": num_at(it, "f3"),
                    "main_inflow": num_at(it, "f62"), "main_inflow_pct": num_at(it, "f184"),
                    "super_large": num_at(it, "f66"), "large": num_at(it, "f72"),
                    "medium": num_at(it, "f78"), "small": num_at(it, "f84"),
                })
            })
            .collect();
        ctx.emit(&Value::Array(out));
        return Ok(());
    }
    let mut t = Table::new(
        &["代码", "名称", "最新", "涨跌幅", "主力净流入", "净占比", "超大单", "大单"],
        vec![Align::Left, Align::Left, Align::Right, Align::Right, Align::Right, Align::Right, Align::Right, Align::Right],
    );
    for it in &items {
        t.row(vec![
            str_at(it, "f12"),
            str_at(it, "f14"),
            fmt_opt(num_at(it, "f2"), 2),
            colorize_pct(num_at(it, "f3")),
            num_at(it, "f62").map(fmt_amount).unwrap_or("-".into()),
            num_at(it, "f184").map(|x| format!("{:.2}%", x)).unwrap_or("-".into()),
            num_at(it, "f66").map(fmt_amount).unwrap_or("-".into()),
            num_at(it, "f72").map(fmt_amount).unwrap_or("-".into()),
        ]);
    }
    t.print();
    Ok(())
}

// ---------- 股吧热榜 ----------

pub fn hot(ctx: &Ctx, soar: bool, num: usize) -> Result<()> {
    let list = api::hot_rank(&ctx.http, soar, num)?;
    let secids: Vec<String> = list
        .iter()
        .filter_map(|it| it.get("sc").and_then(|s| s.as_str()).and_then(secid::from_rank_sc))
        .collect();
    let quotes = api::quotes(&ctx.http, &secids, "f2,f3,f12,f14").unwrap_or_default();
    let find_quote = |code: &str| quotes.iter().find(|q| str_at(q, "f12") == code);
    if ctx.json {
        let out: Vec<Value> = list
            .iter()
            .map(|it| {
                let sc = str_at(it, "sc");
                let code = if sc.len() > 2 { sc[2..].to_string() } else { sc.clone() };
                let q = find_quote(&code);
                json!({
                    "rank": num_at(it, if soar { "hrcrk" } else { "rk" }),
                    "code": code,
                    "name": q.map(|x| str_at(x, "f14")),
                    "price": q.and_then(|x| num_at(x, "f2")),
                    "pct_chg": q.and_then(|x| num_at(x, "f3")),
                    "rank_change": num_at(it, "rc"),
                    "soar_value": num_at(it, "hrc"),
                    "popularity_rank": num_at(it, "rk"),
                })
            })
            .collect();
        ctx.emit(&Value::Array(out));
        return Ok(());
    }
    let title = if soar { "股吧飙升榜" } else { "股吧人气榜" };
    println!("{} (来自 guba.eastmoney.com/rank)", title);
    let headers: Vec<&str> = if soar {
        vec!["飙升排名", "代码", "名称", "最新", "涨跌幅", "飙升值", "人气排名"]
    } else {
        vec!["人气排名", "代码", "名称", "最新", "涨跌幅", "排名变化"]
    };
    let mut t = Table::new(
        &headers,
        headers.iter().map(|h| if *h == "名称" || *h == "代码" { Align::Left } else { Align::Right }).collect(),
    );
    for it in &list {
        let sc = str_at(it, "sc");
        let code = if sc.len() > 2 { sc[2..].to_string() } else { sc.clone() };
        let q = find_quote(&code);
        let name = q.map(|x| str_at(x, "f14")).unwrap_or("-".into());
        let price = q.and_then(|x| num_at(x, "f2"));
        let pct = q.and_then(|x| num_at(x, "f3"));
        if soar {
            t.row(vec![
                str_at(it, "hrcrk"),
                code,
                name,
                fmt_opt(price, 2),
                colorize_pct(pct),
                str_at(it, "hrc"),
                str_at(it, "rk"),
            ]);
        } else {
            let rc = num_at(it, "rc").map(|x| format!("{:+}", x as i64)).unwrap_or("-".into());
            t.row(vec![str_at(it, "rk"), code, name, fmt_opt(price, 2), colorize_pct(pct), rc]);
        }
    }
    t.print();
    Ok(())
}

pub fn hot_history(ctx: &Ctx, code: &str, num: usize) -> Result<()> {
    let sec = secid::resolve(&ctx.http, code)?;
    let sc = secid::to_rank_sc(&sec.secid).ok_or_else(|| anyhow!("该证券不支持人气排名（仅A股）"))?;
    let cur = api::hot_current(&ctx.http, &sc)?;
    let his = api::hot_history(&ctx.http, &sc)?;
    let start = his.len().saturating_sub(num);
    if ctx.json {
        ctx.emit(&json!({
            "code": sec.code,
            "current_rank": num_at(&cur, "rank"),
            "rank_change": num_at(&cur, "rankChange"),
            "total": num_at(&cur, "marketAllCount"),
            "calc_time": str_at(&cur, "calcTime"),
            "history": his[start..].iter().map(|h| json!({"date": str_at(h, "calcTime"), "rank": num_at(h, "rank")})).collect::<Vec<_>>(),
        }));
        return Ok(());
    }
    println!(
        "{} 当前人气排名: 第 {} 名 / 共 {} 只  (较昨日 {:+})  统计时间: {}",
        sec.code,
        str_at(&cur, "rank"),
        str_at(&cur, "marketAllCount"),
        num_at(&cur, "rankChange").unwrap_or(0.0) as i64,
        str_at(&cur, "calcTime"),
    );
    let mut t = Table::new(&["日期", "人气排名"], vec![Align::Left, Align::Right]);
    for h in &his[start..] {
        t.row(vec![str_at(h, "calcTime"), str_at(h, "rank")]);
    }
    t.print();
    Ok(())
}

// ---------- 股吧帖子 ----------

pub fn guba(ctx: &Ctx, code: &str, num: usize) -> Result<()> {
    let sec = secid::resolve(&ctx.http, code)?;
    let mut posts = api::guba_posts(&ctx.http, &sec.code, num)?;
    posts.truncate(num);
    if ctx.json {
        let out: Vec<Value> = posts
            .iter()
            .map(|p| {
                json!({
                    "time": first_str(p, &["post_publish_time", "post_display_time", "post_last_time"]),
                    "title": str_at(p, "post_title"),
                    "clicks": num_at(p, "post_click_count"),
                    "comments": num_at(p, "post_comment_count"),
                    "author": p.pointer("/post_user/user_nickname").and_then(|x| x.as_str()).map(String::from).or_else(|| p.get("user_nickname").and_then(|x| x.as_str()).map(String::from)),
                    "post_id": num_at(p, "post_id"),
                })
            })
            .collect();
        ctx.emit(&Value::Array(out));
        return Ok(());
    }
    println!("{} 股吧最新帖子:", sec.code);
    let mut t = Table::new(
        &["时间", "阅读", "评论", "标题"],
        vec![Align::Left, Align::Right, Align::Right, Align::Left],
    );
    for p in &posts {
        t.row(vec![
            first_str(p, &["post_publish_time", "post_display_time", "post_last_time"]),
            num_at(p, "post_click_count").map(|x| fmt_amount(x)).unwrap_or("-".into()),
            num_at(p, "post_comment_count").map(|x| format!("{:.0}", x)).unwrap_or("-".into()),
            truncate(&str_at(p, "post_title"), 60),
        ]);
    }
    t.print();
    Ok(())
}

fn first_str(v: &Value, keys: &[&str]) -> String {
    for k in keys {
        if let Some(s) = v.get(*k).and_then(|x| x.as_str()) {
            if !s.is_empty() {
                return s.to_string();
            }
        }
    }
    "-".into()
}

fn truncate(s: &str, max_width: usize) -> String {
    let mut out = String::new();
    let mut w = 0;
    for c in s.chars() {
        let cw = disp_width(&c.to_string());
        if w + cw > max_width {
            out.push('…');
            break;
        }
        out.push(c);
        w += cw;
    }
    out
}

// ---------- 涨停池 ----------

pub fn zt_pool(ctx: &Ctx, kind: &str, date: Option<String>, num: usize) -> Result<()> {
    let k = match kind {
        "zt" => "ZT",
        "dt" => "DT",
        "zb" => "ZB",
        _ => return Err(anyhow!("类型仅支持 zt(涨停)/dt(跌停)/zb(炸板)")),
    };
    // 未指定日期时自动回溯最近的交易日
    let (data, used_date) = match date {
        Some(d) => (api::topic_pool(&ctx.http, k, &d, num)?, d),
        None => {
            let mut found = None;
            for back in 0..12 {
                let d = today_str(back);
                let v = api::topic_pool(&ctx.http, k, &d, num)?;
                let n = v.get("pool").and_then(|p| p.as_array()).map(|a| a.len()).unwrap_or(0);
                if n > 0 {
                    found = Some((v, d));
                    break;
                }
            }
            found.ok_or_else(|| anyhow!("近 12 天无数据"))?
        }
    };
    let pool = data.get("pool").and_then(|p| p.as_array()).cloned().unwrap_or_default();
    let kname = match k {
        "ZT" => "涨停池",
        "DT" => "跌停池",
        _ => "炸板池",
    };
    if ctx.json {
        let out: Vec<Value> = pool
            .iter()
            .map(|it| {
                json!({
                    "code": str_at(it, "c"),
                    "name": str_at(it, "n"),
                    "price": num_at(it, "p").map(|x| x / 1000.0),
                    "pct_chg": num_at(it, "zdp"),
                    "streak": num_at(it, "lbc"),
                    "stat": it.pointer("/zttj").map(|z| format!("{}天{}板", str_at(z, "days"), str_at(z, "ct"))),
                    "first_seal_time": num_at(it, "fbt").map(fmt_hms),
                    "seal_amount": num_at(it, "fund"),
                    "break_count": num_at(it, "zbc"),
                    "turnover_pct": num_at(it, "hs"),
                    "industry": str_at(it, "hybk"),
                    "amount": num_at(it, "amount"),
                })
            })
            .collect();
        ctx.emit(&json!({"date": used_date, "type": kname, "total": data.get("tc"), "pool": out}));
        return Ok(());
    }
    println!("{} {}  共 {} 只", used_date, kname, data.get("tc").and_then(|x| x.as_i64()).unwrap_or(pool.len() as i64));
    let mut t = Table::new(
        &["代码", "名称", "价格", "涨跌幅", "连板", "统计", "首封", "封单额", "炸板", "换手", "行业"],
        vec![
            Align::Left, Align::Left, Align::Right, Align::Right, Align::Right, Align::Right,
            Align::Right, Align::Right, Align::Right, Align::Right, Align::Left,
        ],
    );
    for it in &pool {
        t.row(vec![
            str_at(it, "c"),
            str_at(it, "n"),
            num_at(it, "p").map(|x| format!("{:.2}", x / 1000.0)).unwrap_or("-".into()),
            colorize_pct(num_at(it, "zdp")),
            num_at(it, "lbc").map(|x| format!("{:.0}", x)).unwrap_or("-".into()),
            it.pointer("/zttj").map(|z| format!("{}天{}板", str_at(z, "days"), str_at(z, "ct"))).unwrap_or("-".into()),
            num_at(it, "fbt").map(fmt_hms).unwrap_or("-".into()),
            num_at(it, "fund").map(fmt_amount).unwrap_or("-".into()),
            num_at(it, "zbc").map(|x| format!("{:.0}", x)).unwrap_or("-".into()),
            num_at(it, "hs").map(|x| format!("{:.2}%", x)).unwrap_or("-".into()),
            str_at(it, "hybk"),
        ]);
    }
    t.print();
    Ok(())
}

fn fmt_hms(x: f64) -> String {
    let s = format!("{:06}", x as i64);
    format!("{}:{}:{}", &s[0..2], &s[2..4], &s[4..6])
}

// ---------- 龙虎榜 ----------

pub fn lhb(ctx: &Ctx, date: Option<String>, num: usize) -> Result<()> {
    let day = match date {
        Some(d) => d,
        None => {
            let latest = api::dc_query(
                &ctx.http,
                "RPT_DAILYBILLBOARD_DETAILSNEW",
                "",
                "TRADE_DATE",
                "-1",
                1,
                false,
            )?;
            let d = str_at(&latest[0], "TRADE_DATE");
            d.chars().take(10).collect()
        }
    };
    let filter = format!("(TRADE_DATE='{}')", day);
    let rows = api::dc_query(
        &ctx.http,
        "RPT_DAILYBILLBOARD_DETAILSNEW",
        &filter,
        "BILLBOARD_NET_AMT",
        "-1",
        num * 2,
        false,
    )?;
    // 同一股票可因多个上榜原因出现多行，去重保留首行
    let mut seen = std::collections::HashSet::new();
    let rows: Vec<&Value> = rows
        .iter()
        .filter(|r| seen.insert(str_at(r, "SECURITY_CODE")))
        .take(num)
        .collect();
    if ctx.json {
        let out: Vec<Value> = rows
            .iter()
            .map(|r| {
                json!({
                    "date": day,
                    "code": str_at(r, "SECURITY_CODE"),
                    "name": str_at(r, "SECURITY_NAME_ABBR"),
                    "close": num_at(r, "CLOSE_PRICE"),
                    "pct_chg": num_at(r, "CHANGE_RATE"),
                    "net_buy": num_at(r, "BILLBOARD_NET_AMT"),
                    "buy": num_at(r, "BILLBOARD_BUY_AMT"),
                    "sell": num_at(r, "BILLBOARD_SELL_AMT"),
                    "turnover_pct": num_at(r, "TURNOVERRATE"),
                    "reason": str_at(r, "EXPLANATION"),
                    "note": str_at(r, "EXPLAIN"),
                })
            })
            .collect();
        ctx.emit(&Value::Array(out));
        return Ok(());
    }
    println!("龙虎榜 {}  (按净买入排序)", day);
    let mut t = Table::new(
        &["代码", "名称", "收盘", "涨跌幅", "净买入", "买入", "卖出", "上榜原因"],
        vec![Align::Left, Align::Left, Align::Right, Align::Right, Align::Right, Align::Right, Align::Right, Align::Left],
    );
    for r in &rows {
        t.row(vec![
            str_at(r, "SECURITY_CODE"),
            str_at(r, "SECURITY_NAME_ABBR"),
            fmt_opt(num_at(r, "CLOSE_PRICE"), 2),
            colorize_pct(num_at(r, "CHANGE_RATE")),
            num_at(r, "BILLBOARD_NET_AMT").map(fmt_amount).unwrap_or("-".into()),
            num_at(r, "BILLBOARD_BUY_AMT").map(fmt_amount).unwrap_or("-".into()),
            num_at(r, "BILLBOARD_SELL_AMT").map(fmt_amount).unwrap_or("-".into()),
            truncate(&str_at(r, "EXPLANATION"), 40),
        ]);
    }
    t.print();
    Ok(())
}

// ---------- F10 ----------

fn secucode(sec: &Sec) -> Result<String> {
    let (m, code) = sec.secid.split_once('.').ok_or_else(|| anyhow!("secid 非法"))?;
    let suffix = match m {
        "1" => "SH",
        "0" => {
            if code.starts_with('4') || code.starts_with('8') || code.starts_with("92") {
                "BJ"
            } else {
                "SZ"
            }
        }
        _ => return Err(anyhow!("仅支持 A 股")),
    };
    Ok(format!("{}.{}", code, suffix))
}

pub fn info(ctx: &Ctx, code: &str) -> Result<()> {
    let sec = secid::resolve(&ctx.http, code)?;
    let sc = secucode(&sec)?;
    let filter = format!("(SECUCODE=\"{}\")", sc);
    let rows = api::dc_query(&ctx.http, "RPT_F10_BASIC_ORGINFO", &filter, "", "", 1, true)?;
    let d = &rows[0];
    if ctx.json {
        ctx.emit(d);
        return Ok(());
    }
    let kv = [
        ("公司名称", "ORG_NAME"),
        ("英文名称", "ORG_NAME_EN"),
        ("曾用名", "FORMERNAME"),
        ("A股代码/简称", "STR_CODEA"),
        ("所属行业(东财)", "EM2016"),
        ("证监会行业", "INDUSTRYCSRC1"),
        ("上市市场", "TRADE_MARKET"),
        ("证券类型", "SECURITY_TYPE"),
        ("董事长", "CHAIRMAN"),
        ("总经理", "PRESIDENT"),
        ("法人代表", "LEGAL_PERSON"),
        ("董秘", "SECRETARY"),
        ("注册资本", "REG_CAPITAL"),
        ("成立日期", "FOUND_DATE"),
        ("上市日期", "LISTING_DATE"),
        ("员工人数", "EMP_NUM"),
        ("电话", "ORG_TEL"),
        ("官网", "ORG_WEB"),
        ("注册地址", "REG_ADDRESS"),
        ("办公地址", "ADDRESS"),
    ];
    for (label, key) in kv {
        let v = str_at(d, key);
        if v != "-" && !v.is_empty() && v != "null" {
            println!("{}: {}", label, v);
        }
    }
    let profile = str_at(d, "ORG_PROFILE");
    if profile != "-" {
        println!("\n公司简介:\n{}", profile.trim());
    }
    let scope = str_at(d, "BUSINESS_SCOPE");
    if scope != "-" {
        println!("\n经营范围:\n{}", truncate(scope.trim(), 500));
    }
    Ok(())
}

pub fn finance(ctx: &Ctx, code: &str, num: usize) -> Result<()> {
    let sec = secid::resolve(&ctx.http, code)?;
    let sc = secucode(&sec)?;
    let filter = format!("(SECUCODE=\"{}\")", sc);
    let rows = api::dc_query(
        &ctx.http,
        "RPT_F10_FINANCE_MAINFINADATA",
        &filter,
        "REPORT_DATE",
        "-1",
        num,
        true,
    )?;
    if ctx.json {
        ctx.emit(&Value::Array(rows));
        return Ok(());
    }
    let mut t = Table::new(
        &["报告期", "营收", "营收同比", "归母净利", "净利同比", "扣非净利", "每股收益", "每股净资产", "ROE(加权)", "毛利率", "负债率"],
        vec![
            Align::Left, Align::Right, Align::Right, Align::Right, Align::Right, Align::Right,
            Align::Right, Align::Right, Align::Right, Align::Right, Align::Right,
        ],
    );
    for r in &rows {
        t.row(vec![
            str_at(r, "REPORT_DATE_NAME"),
            num_at(r, "TOTALOPERATEREVE").or(num_at(r, "TOTAL_OPERATE_INCOME")).map(fmt_amount).unwrap_or("-".into()),
            num_at(r, "TOTALOPERATEREVETZ").map(|x| format!("{:+.2}%", x)).unwrap_or("-".into()),
            num_at(r, "PARENTNETPROFIT").map(fmt_amount).unwrap_or("-".into()),
            num_at(r, "PARENTNETPROFITTZ").map(|x| format!("{:+.2}%", x)).unwrap_or("-".into()),
            num_at(r, "KCFJCXSYJLR").map(fmt_amount).unwrap_or("-".into()),
            fmt_opt(num_at(r, "EPSJB"), 3),
            fmt_opt(num_at(r, "BPS"), 3),
            num_at(r, "ROEJQ").map(|x| format!("{:.2}%", x)).unwrap_or("-".into()),
            num_at(r, "XSMLL").map(|x| format!("{:.2}%", x)).unwrap_or("-".into()),
            num_at(r, "ZCFZL").map(|x| format!("{:.2}%", x)).unwrap_or("-".into()),
        ]);
    }
    t.print();
    Ok(())
}

pub fn holders(ctx: &Ctx, code: &str) -> Result<()> {
    let sec = secid::resolve(&ctx.http, code)?;
    let filter = format!("(SECURITY_CODE=\"{}\")", sec.code);
    let rows = api::dc_query(&ctx.http, "RPT_HOLDERNUMLATEST", &filter, "", "", 1, false)?;
    let d = &rows[0];
    if ctx.json {
        ctx.emit(d);
        return Ok(());
    }
    println!("{} 股东户数", str_at(d, "SECURITY_NAME_ABBR"));
    println!("截止日期: {}", str_at(d, "END_DATE").chars().take(10).collect::<String>());
    println!("股东户数: {}", num_at(d, "HOLDER_NUM").map(|x| format!("{:.0}", x)).unwrap_or("-".into()));
    println!(
        "较上期: {} ({}%)",
        num_at(d, "HOLDER_NUM_CHANGE").map(|x| format!("{:+.0}", x)).unwrap_or("-".into()),
        num_at(d, "HOLDER_NUM_RATIO").map(|x| format!("{:+.2}", x)).unwrap_or("-".into()),
    );
    println!(
        "户均持股市值: {}",
        num_at(d, "AVG_MARKET_CAP").map(fmt_amount).unwrap_or("-".into()),
    );
    Ok(())
}

// ---------- 新闻/公告 ----------

pub fn news(ctx: &Ctx, code: &str, num: usize) -> Result<()> {
    let sec = secid::resolve(&ctx.http, code)?;
    let list = api::stock_news(&ctx.http, &sec.code, num)?;
    let list = &list[..list.len().min(num)];
    if ctx.json {
        let out: Vec<Value> = list
            .iter()
            .map(|n| json!({"date": str_at(n, "date"), "title": str_at(n, "title"), "media": str_at(n, "mediaName"), "url": str_at(n, "url")}))
            .collect();
        ctx.emit(&Value::Array(out));
        return Ok(());
    }
    println!("{} 相关资讯:", sec.code);
    for n in list {
        println!("[{}] {} —— {}", str_at(n, "date"), str_at(n, "title"), str_at(n, "mediaName"));
        println!("    {}", str_at(n, "url"));
    }
    Ok(())
}

pub fn ann(ctx: &Ctx, code: &str, num: usize) -> Result<()> {
    let sec = secid::resolve(&ctx.http, code)?;
    let list = api::announcements(&ctx.http, &sec.code, num)?;
    if ctx.json {
        let out: Vec<Value> = list
            .iter()
            .map(|a| {
                let art = str_at(a, "art_code");
                json!({
                    "date": str_at(a, "notice_date").chars().take(10).collect::<String>(),
                    "title": str_at(a, "title"),
                    "types": a.get("columns").and_then(|c| c.as_array()).map(|cs| cs.iter().map(|x| str_at(x, "column_name")).collect::<Vec<_>>()),
                    "url": format!("https://data.eastmoney.com/notices/detail/{}/{}.html", sec.code, art),
                })
            })
            .collect();
        ctx.emit(&Value::Array(out));
        return Ok(());
    }
    println!("{} 最新公告:", sec.code);
    for a in &list {
        let date: String = str_at(a, "notice_date").chars().take(10).collect();
        let cols = a
            .get("columns")
            .and_then(|c| c.as_array())
            .map(|cs| cs.iter().map(|x| str_at(x, "column_name")).collect::<Vec<_>>().join("/"))
            .unwrap_or_default();
        println!("[{}] {} ({})", date, str_at(a, "title"), cols);
        println!("    https://data.eastmoney.com/notices/detail/{}/{}.html", sec.code, str_at(a, "art_code"));
    }
    Ok(())
}

// ---------- 自选股 ----------

fn require_cookie() -> Result<String> {
    config::load_cookie().ok_or_else(|| {
        anyhow!(
            "未配置 Cookie。自选股需要登录态：\n  1. 浏览器登录 quote.eastmoney.com 后，F12 → Network → 任意请求 → 复制完整 Cookie\n  2. 运行: em config set-cookie   (然后粘贴，Ctrl-D 结束)\n     或设置环境变量 EM_COOKIE"
        )
    })
}

pub fn watch(ctx: &Ctx, group: Option<String>, list_groups: bool, raw: bool) -> Result<()> {
    let cookie = require_cookie()?;
    if raw {
        let groups = api::myfavor(&ctx.http, &cookie, "ggdefstkindexinfos", &[])?;
        println!("{}", serde_json::to_string_pretty(&groups).unwrap());
        let stocks = api::myfavor(&ctx.http, &cookie, "gstkinfos", &[("g", "1")])?;
        println!("{}", serde_json::to_string_pretty(&stocks).unwrap());
        return Ok(());
    }
    let groups = api::watch_groups(&ctx.http, &cookie)?;
    if list_groups {
        if ctx.json {
            let out: Vec<Value> = groups.iter().map(|(gid, gname)| json!({"gid": gid, "name": gname})).collect();
            ctx.emit(&Value::Array(out));
        } else {
            let mut t = Table::new(&["分组ID", "分组名"], vec![Align::Right, Align::Left]);
            for (gid, gname) in &groups {
                t.row(vec![gid.clone(), gname.clone()]);
            }
            t.print();
        }
        return Ok(());
    }
    // 默认第一个分组（一般是"自选股"）；--group 支持分组名或 gid
    let (gid, gname) = match &group {
        None => groups.first().cloned().ok_or_else(|| anyhow!("没有自选分组"))?,
        Some(g) => groups
            .iter()
            .find(|(gid, gname)| gid == g || gname == g)
            .cloned()
            .ok_or_else(|| {
                let names: Vec<String> = groups.iter().map(|(_, n)| n.clone()).collect();
                anyhow!("未找到分组 \"{}\"。可用分组: {}", g, names.join(", "))
            })?,
    };
    let secids = api::watch_stocks(&ctx.http, &cookie, &gid)?;
    if secids.is_empty() {
        println!("分组「{}」为空", gname);
        return Ok(());
    }
    let items = api::quotes(&ctx.http, &secids, api::QUOTE_FIELDS)?;
    if ctx.json {
        let out: Vec<Value> = items.iter().map(quote_json).collect();
        ctx.emit(&json!({"group": gname, "gid": gid, "stocks": out}));
    } else {
        println!("自选股分组「{}」({} 只):", gname, items.len());
        quote_table(&items);
    }
    Ok(())
}

// ---------- 配置 ----------

pub fn config_set_cookie(arg: Option<String>) -> Result<()> {
    let cookie = match arg {
        Some(c) => c,
        None => {
            eprintln!("粘贴 Cookie 后按 Ctrl-D 结束:");
            let mut buf = String::new();
            std::io::Read::read_to_string(&mut std::io::stdin(), &mut buf)?;
            buf
        }
    };
    let path = config::save_cookie(&cookie)?;
    println!("Cookie 已保存: {}", path.display());
    Ok(())
}

pub fn config_show() -> Result<()> {
    match config::load_cookie() {
        Some(c) => {
            let masked = if c.len() > 60 {
                format!("{}...{} ({} 字符)", &c[..30], &c[c.len() - 10..], c.len())
            } else {
                c
            };
            println!("Cookie: {}", masked);
        }
        None => println!("Cookie: 未配置"),
    }
    println!("路径: {}", config::cookie_path()?.display());
    println!("(环境变量 EM_COOKIE 优先于文件)");
    Ok(())
}

// ---------- 日期工具 ----------

/// 北京时间今天减 n 天，YYYYMMDD
pub fn today_str(minus_days: i64) -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;
    let days = (secs + 8 * 3600) / 86400 - minus_days;
    let (y, m, d) = civil_from_days(days);
    format!("{:04}{:02}{:02}", y, m, d)
}

/// Howard Hinnant 算法：days since 1970-01-01 → (y, m, d)
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}
