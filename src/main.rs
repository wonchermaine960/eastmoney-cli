mod api;
mod client;
mod commands;
mod config;
mod secid;
mod util;

use clap::{Parser, Subcommand};
use commands::Ctx;

#[derive(Parser)]
#[command(
    name = "em",
    version,
    about = "东方财富 A 股数据 CLI（行情/大盘/股吧热榜/自选股）",
    after_help = "示例:\n  em quote 300339 600570        实时行情\n  em detail 润和软件            个股详情（支持中文名）\n  em kline 300339 --num 20      日K线\n  em market                     大盘概览\n  em hot                        股吧人气榜\n  em zt                         涨停池\n  em watch                      自选股（需先 em config set-cookie）\n所有命令支持 --json 输出结构化数据。"
)]
struct Cli {
    /// 以 JSON 输出（供程序/Agent 解析）
    #[arg(long, global = true)]
    json: bool,
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// 搜索股票/指数/板块（代码、拼音、中文名）
    Search {
        /// 关键词
        keyword: String,
    },
    /// 实时行情（可多只，支持代码/中文名/sh、sz前缀/secid）
    Quote {
        /// 股票代码，如 300339 sh000001 润和软件
        #[arg(required = true)]
        codes: Vec<String>,
    },
    /// 个股详细快照（估值、盘口五档、涨跌停价）
    Detail {
        code: String,
    },
    /// K线（日/周/月/分钟）
    Kline {
        code: String,
        /// day/week/month/5m/15m/30m/60m
        #[arg(long, default_value = "day")]
        period: String,
        /// 根数
        #[arg(long, short = 'n', default_value_t = 30)]
        num: usize,
        /// 复权：qfq前复权/hfq后复权/none不复权
        #[arg(long, default_value = "qfq")]
        fq: String,
    },
    /// 分时走势
    Trend {
        code: String,
        /// 天数（1-5）
        #[arg(long, default_value_t = 1)]
        days: usize,
        /// 显示最近 N 个分时点（JSON 模式输出全部）
        #[arg(long, short = 'n', default_value_t = 20)]
        num: usize,
    },
    /// 大盘概览（核心指数、涨跌家数、涨停跌停、两市成交额）
    Market,
    /// 全A股排行榜
    Rank {
        /// 排序字段：change涨跌幅/amount成交额/turnover换手/volume成交量/pe市盈/mcap市值/flow主力净流入/speed涨速
        #[arg(long, default_value = "change")]
        by: String,
        /// 升序（默认降序，如找跌幅榜用 --by change --asc）
        #[arg(long)]
        asc: bool,
        #[arg(long, short = 'n', default_value_t = 20)]
        num: usize,
    },
    /// 板块行情（行业/概念/地域）
    Board {
        /// industry行业/concept概念/region地域
        #[arg(default_value = "industry")]
        kind: String,
        #[arg(long, default_value = "change")]
        by: String,
        #[arg(long)]
        asc: bool,
        #[arg(long, short = 'n', default_value_t = 20)]
        num: usize,
    },
    /// 板块成分股（参数为 BK 代码，如 BK1031，可从 em board 获取）
    BoardStocks {
        code: String,
        #[arg(long, default_value = "change")]
        by: String,
        #[arg(long)]
        asc: bool,
        #[arg(long, short = 'n', default_value_t = 20)]
        num: usize,
    },
    /// 个股资金流（主力/超大/大/中/小单净流入，按日）
    Flow {
        code: String,
        #[arg(long, short = 'n', default_value_t = 10)]
        num: usize,
    },
    /// 主力净流入排行
    FlowRank {
        #[arg(long, short = 'n', default_value_t = 20)]
        num: usize,
    },
    /// 股吧人气榜（--soar 看飙升榜）
    Hot {
        /// 飙升榜
        #[arg(long)]
        soar: bool,
        #[arg(long, short = 'n', default_value_t = 20)]
        num: usize,
    },
    /// 个股人气排名（当前排名 + 历史走势）
    HotHistory {
        code: String,
        /// 最近 N 天
        #[arg(long, short = 'n', default_value_t = 30)]
        num: usize,
    },
    /// 股吧最新帖子
    Guba {
        code: String,
        #[arg(long, short = 'n', default_value_t = 10)]
        num: usize,
    },
    /// 涨停/跌停/炸板池
    Zt {
        /// zt涨停/dt跌停/zb炸板
        #[arg(default_value = "zt")]
        kind: String,
        /// 日期 YYYYMMDD（默认最近交易日）
        #[arg(long)]
        date: Option<String>,
        #[arg(long, short = 'n', default_value_t = 100)]
        num: usize,
    },
    /// 龙虎榜
    Lhb {
        /// 日期 YYYY-MM-DD（默认最近上榜日）
        #[arg(long)]
        date: Option<String>,
        #[arg(long, short = 'n', default_value_t = 30)]
        num: usize,
    },
    /// 公司概况（F10）
    Info {
        code: String,
    },
    /// 主要财务指标（按报告期）
    Finance {
        code: String,
        /// 最近 N 个报告期
        #[arg(long, short = 'n', default_value_t = 5)]
        num: usize,
    },
    /// 股东户数
    Holders {
        code: String,
    },
    /// 个股资讯
    News {
        code: String,
        #[arg(long, short = 'n', default_value_t = 10)]
        num: usize,
    },
    /// 个股公告
    Ann {
        code: String,
        #[arg(long, short = 'n', default_value_t = 10)]
        num: usize,
    },
    /// 自选股（需 Cookie，见 em config set-cookie）
    Watch {
        /// 指定分组（分组名或ID，默认第一个分组）
        #[arg(long, short = 'g')]
        group: Option<String>,
        /// 列出所有分组
        #[arg(long)]
        groups: bool,
        /// 输出原始 JSON（调试用）
        #[arg(long)]
        raw: bool,
    },
    /// 配置管理（Cookie）
    Config {
        #[command(subcommand)]
        action: ConfigCmd,
    },
}

#[derive(Subcommand)]
enum ConfigCmd {
    /// 保存登录 Cookie（不带参数则从 stdin 读取）
    SetCookie {
        cookie: Option<String>,
    },
    /// 查看当前配置
    Show,
}

fn main() {
    let cli = Cli::parse();
    let http = match client::Http::new() {
        Ok(h) => h,
        Err(e) => {
            eprintln!("错误: {:#}", e);
            std::process::exit(1);
        }
    };
    let ctx = Ctx { http, json: cli.json };
    let result = match &cli.cmd {
        Cmd::Search { keyword } => commands::search(&ctx, keyword),
        Cmd::Quote { codes } => commands::quote(&ctx, codes),
        Cmd::Detail { code } => commands::detail(&ctx, code),
        Cmd::Kline { code, period, num, fq } => commands::kline(&ctx, code, period, *num, fq),
        Cmd::Trend { code, days, num } => commands::trend(&ctx, code, *days, *num),
        Cmd::Market => commands::market(&ctx),
        Cmd::Rank { by, asc, num } => commands::rank(&ctx, by, *asc, *num),
        Cmd::Board { kind, by, asc, num } => commands::board(&ctx, kind, by, *asc, *num),
        Cmd::BoardStocks { code, by, asc, num } => commands::board_stocks(&ctx, code, by, *asc, *num),
        Cmd::Flow { code, num } => commands::flow(&ctx, code, *num),
        Cmd::FlowRank { num } => commands::flow_rank(&ctx, *num),
        Cmd::Hot { soar, num } => commands::hot(&ctx, *soar, *num),
        Cmd::HotHistory { code, num } => commands::hot_history(&ctx, code, *num),
        Cmd::Guba { code, num } => commands::guba(&ctx, code, *num),
        Cmd::Zt { kind, date, num } => commands::zt_pool(&ctx, kind, date.clone(), *num),
        Cmd::Lhb { date, num } => commands::lhb(&ctx, date.clone(), *num),
        Cmd::Info { code } => commands::info(&ctx, code),
        Cmd::Finance { code, num } => commands::finance(&ctx, code, *num),
        Cmd::Holders { code } => commands::holders(&ctx, code),
        Cmd::News { code, num } => commands::news(&ctx, code, *num),
        Cmd::Ann { code, num } => commands::ann(&ctx, code, *num),
        Cmd::Watch { group, groups, raw } => commands::watch(&ctx, group.clone(), *groups, *raw),
        Cmd::Config { action } => match action {
            ConfigCmd::SetCookie { cookie } => commands::config_set_cookie(cookie.clone()),
            ConfigCmd::Show => commands::config_show(),
        },
    };
    if let Err(e) = result {
        eprintln!("错误: {:#}", e);
        std::process::exit(1);
    }
}
