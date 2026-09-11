use serde_json::Value;
use std::io::IsTerminal;

/// 终端显示宽度（CJK 全角字符按 2 列计）
pub fn disp_width(s: &str) -> usize {
    s.chars().map(char_width).sum()
}

fn char_width(c: char) -> usize {
    let u = c as u32;
    if (0x1100..=0x115F).contains(&u)
        || (0x2E80..=0xA4CF).contains(&u)
        || (0xAC00..=0xD7A3).contains(&u)
        || (0xF900..=0xFAFF).contains(&u)
        || (0xFE30..=0xFE4F).contains(&u)
        || (0xFF00..=0xFF60).contains(&u)
        || (0xFFE0..=0xFFE6).contains(&u)
        || (0x20000..=0x3FFFD).contains(&u)
    {
        2
    } else {
        1
    }
}

pub enum Align {
    Left,
    Right,
}

/// 简易对齐表格，支持中文宽度
pub struct Table {
    headers: Vec<String>,
    aligns: Vec<Align>,
    rows: Vec<Vec<String>>,
}

impl Table {
    pub fn new(headers: &[&str], aligns: Vec<Align>) -> Self {
        Table {
            headers: headers.iter().map(|s| s.to_string()).collect(),
            aligns,
            rows: Vec::new(),
        }
    }

    pub fn row(&mut self, cells: Vec<String>) {
        self.rows.push(cells);
    }

    pub fn print(&self) {
        let ncol = self.headers.len();
        let mut widths: Vec<usize> = self.headers.iter().map(|h| disp_width(h)).collect();
        for row in &self.rows {
            for (i, c) in row.iter().enumerate().take(ncol) {
                widths[i] = widths[i].max(disp_width(strip_ansi(c).as_str()));
            }
        }
        let line: Vec<String> = self
            .headers
            .iter()
            .enumerate()
            .map(|(i, h)| pad(h, widths[i], &self.aligns[i]))
            .collect();
        println!("{}", line.join("  "));
        for row in &self.rows {
            let line: Vec<String> = row
                .iter()
                .enumerate()
                .take(ncol)
                .map(|(i, c)| pad(c, widths[i], &self.aligns[i]))
                .collect();
            println!("{}", line.join("  ").trim_end());
        }
    }
}

fn strip_ansi(s: &str) -> String {
    let mut out = String::new();
    let mut in_esc = false;
    for c in s.chars() {
        if in_esc {
            if c == 'm' {
                in_esc = false;
            }
        } else if c == '\x1b' {
            in_esc = true;
        } else {
            out.push(c);
        }
    }
    out
}

fn pad(s: &str, w: usize, a: &Align) -> String {
    let vis = disp_width(&strip_ansi(s));
    let fill = w.saturating_sub(vis);
    match a {
        Align::Left => format!("{}{}", s, " ".repeat(fill)),
        Align::Right => format!("{}{}", " ".repeat(fill), s),
    }
}

/// json Value → f64（兼容 "-" 停牌占位与字符串数字）
pub fn num(v: &Value) -> Option<f64> {
    match v {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.parse().ok(),
        _ => None,
    }
}

pub fn num_at(v: &Value, key: &str) -> Option<f64> {
    v.get(key).and_then(num)
}

pub fn str_at(v: &Value, key: &str) -> String {
    match v.get(key) {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Number(n)) => n.to_string(),
        _ => "-".into(),
    }
}

/// 金额：元 → 万/亿
pub fn fmt_amount(x: f64) -> String {
    let ax = x.abs();
    if ax >= 1e8 {
        format!("{:.2}亿", x / 1e8)
    } else if ax >= 1e4 {
        format!("{:.1}万", x / 1e4)
    } else {
        format!("{:.0}", x)
    }
}

/// 成交量：手 → 万手/亿手
pub fn fmt_vol(x: f64) -> String {
    let ax = x.abs();
    if ax >= 1e8 {
        format!("{:.2}亿手", x / 1e8)
    } else if ax >= 1e4 {
        format!("{:.1}万手", x / 1e4)
    } else {
        format!("{:.0}手", x)
    }
}

pub fn fmt_opt(v: Option<f64>, prec: usize) -> String {
    match v {
        Some(x) => format!("{:.*}", prec, x),
        None => "-".into(),
    }
}

pub fn fmt_pct(v: Option<f64>) -> String {
    match v {
        Some(x) => format!("{:+.2}%", x),
        None => "-".into(),
    }
}

fn use_color() -> bool {
    std::io::stdout().is_terminal() && std::env::var_os("NO_COLOR").is_none()
}

/// A股习惯：红涨绿跌（仅 TTY 下着色）
pub fn colorize_pct(v: Option<f64>) -> String {
    let s = fmt_pct(v);
    if !use_color() {
        return s;
    }
    match v {
        Some(x) if x > 0.0 => format!("\x1b[31m{}\x1b[0m", s),
        Some(x) if x < 0.0 => format!("\x1b[32m{}\x1b[0m", s),
        _ => s,
    }
}
