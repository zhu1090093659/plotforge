//! CLI output formatting — i18n chrome, colorized summaries, and report
//! rendering. `main.rs` orchestrates commands and calls into this module for
//! every human-readable surface so output rules (i18n via `OutputLanguage`,
//! TTY-only color, stable JSON) live in one place.
//!
//! JSON output (studio commands via `print_studio_json`) is intentionally not
//! colored and keeps its machine-readable shape; never adjust JSON fields for
//! human-readable changes. Color is emitted only when stdout is a TTY
//! (`std::io::IsTerminal`), so test pipes and redirects see plain text.

use std::env;
use std::io::IsTerminal;

use anyhow::{Context, Result};
use clap::ValueEnum;
use plotforge_schema::{ExportProfileTarget, ProjectData, ProviderCostReport, UsageSummary};
use plotforge_workshop::WorkshopPackageValidationReport;
use serde::Serialize;

/// CLI output language, resolved from `--language` / `PLOTFORGE_LANGUAGE` / `LANG`.
#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum OutputLanguage {
    En,
    Zh,
}

/// Resolve the active CLI output language from the explicit flag, then env vars.
pub fn resolve_output_language(language: Option<OutputLanguage>) -> OutputLanguage {
    if let Some(language) = language {
        return language;
    }
    match env::var("PLOTFORGE_LANGUAGE")
        .or_else(|_| env::var("LANG"))
        .unwrap_or_default()
        .to_ascii_lowercase()
    {
        value if value.starts_with("zh") => OutputLanguage::Zh,
        _ => OutputLanguage::En,
    }
}

/// Translate a fixed CLI chrome term. Authored manifest content (titles, story
/// text) is never translated — only chrome labels.
pub fn cli_term(language: OutputLanguage, key: &'static str) -> &'static str {
    if matches!(language, OutputLanguage::En) {
        return key;
    }
    match key {
        "scene" => "场景",
        "choice" => "选择",
        "delta" => "变化",
        "trace" => "追踪",
        "snapshot" => "快照",
        "run seed" => "运行种子",
        "prompt version" => "提示词版本",
        "model version" => "模型版本",
        "provider config hash" => "Provider 配置哈希",
        "trace evidence id" => "追踪证据 ID",
        "snapshot evidence id" => "快照证据 ID",
        "player input" => "玩家输入",
        "selected" => "已选择",
        "intent" => "意图",
        "intent choice" => "意图选择",
        "intent action" => "意图动作",
        "intent matched terms" => "意图匹配词",
        "intent reason" => "意图原因",
        "rule" => "规则",
        "delta empty" => "变化为空",
        "committed" => "已提交",
        "rule action" => "规则动作",
        "rule delta empty" => "规则变化为空",
        "rule committed" => "规则已提交",
        "rule error" => "规则错误",
        "planner" => "规划器",
        "planner requested" => "规划器请求",
        "planner scene" => "规划器场景",
        "planner fallback" => "规划器回退",
        "planner error" => "规划器错误",
        "fallback" => "回退",
        "story before" => "故事前状态",
        "story after" => "故事后状态",
        "beat" => "节拍",
        "turn" => "回合",
        "world delta" => "世界变化",
        "diagnostics" => "诊断",
        "diagnostic" => "诊断",
        "media references" => "媒体引用",
        "media" => "媒体",
        "error" => "错误",
        "review scene" => "审查场景",
        "review score" => "审查分数",
        "review issues" => "审查问题",
        "review issue" => "审查问题",
        "desktop runtime draft" => "桌面运行时草稿",
        "desktop build notes" => "桌面构建说明",
        _ => key,
    }
}

/// Render an optional value, falling back to a localized "none" label.
pub fn none_label(language: OutputLanguage, value: Option<&str>) -> &str {
    value.unwrap_or(match language {
        OutputLanguage::En => "none",
        OutputLanguage::Zh => "无",
    })
}

/// Render an optional value, falling back to a localized "unsupported" label.
pub fn unsupported_label(language: OutputLanguage, value: Option<&str>) -> &str {
    value.unwrap_or(match language {
        OutputLanguage::En => "unsupported",
        OutputLanguage::Zh => "不支持",
    })
}

/// Print a studio command result as stable JSON. Never colored; field shape is
/// a contract consumed by `apps/creator-desktop` and cli_smoke.
pub fn print_studio_json(value: impl Serialize) -> Result<()> {
    println!(
        "{}",
        serde_json::to_string(&value).context("serialize studio command result")?
    );
    Ok(())
}

/// Render the aggregate user-global usage ledger. The compact table is meant
/// for terminals; `plotforge usage summary --json` remains the stable machine
/// contract and bypasses this human-readable surface entirely.
pub fn render_usage_summary(language: OutputLanguage, summary: &UsageSummary) {
    let title = match language {
        OutputLanguage::En => "usage summary",
        OutputLanguage::Zh => "用量汇总",
    };
    println!("{}", paint("36", title));
    println!(
        "{}: {}",
        usage_label(language, "input tokens"),
        summary.total_input_tokens
    );
    println!(
        "{}: {}",
        usage_label(language, "output tokens"),
        summary.total_output_tokens
    );
    println!(
        "{}: {}",
        usage_label(language, "cost units"),
        summary.total_spent_cost_units
    );

    if summary.by_provider.is_empty() {
        println!("{}", usage_label(language, "no usage recorded"));
        return;
    }

    println!(
        "{:<24} {:>10} {:>10} {:>8} {:>8} {:>8} {:>10} {:>12}",
        usage_label(language, "provider"),
        usage_label(language, "input"),
        usage_label(language, "output"),
        usage_label(language, "text"),
        usage_label(language, "image"),
        usage_label(language, "tts"),
        usage_label(language, "moderation"),
        usage_label(language, "cost units")
    );
    for report in summary.by_provider.values() {
        println!(
            "{:<24} {:>10} {:>10} {:>8} {:>8} {:>8} {:>10} {:>12}",
            report.provider_id,
            report.input_tokens,
            report.output_tokens,
            report.text_calls,
            report.image_calls,
            report.tts_calls,
            report.moderation_calls,
            report.spent_cost_units
        );
    }
}

/// Render one provider's usage report without changing the JSON contract.
pub fn render_provider_cost_report(language: OutputLanguage, report: &ProviderCostReport) {
    let title = match language {
        OutputLanguage::En => format!("provider usage: {}", report.provider_id),
        OutputLanguage::Zh => format!("提供商用量：{}", report.provider_id),
    };
    println!("{}", paint("36", &title));
    for (label, value) in [
        ("text calls", report.text_calls),
        ("image calls", report.image_calls),
        ("tts calls", report.tts_calls),
        ("moderation calls", report.moderation_calls),
        ("input tokens", report.input_tokens),
        ("output tokens", report.output_tokens),
        ("cost units", report.spent_cost_units),
    ] {
        println!("{}: {value}", usage_label(language, label));
    }
}

fn usage_label(language: OutputLanguage, label: &'static str) -> &'static str {
    if matches!(language, OutputLanguage::En) {
        return label;
    }
    match label {
        "provider" => "提供商",
        "input" => "输入",
        "output" => "输出",
        "text" => "文本",
        "image" => "图片",
        "tts" => "语音",
        "moderation" => "审核",
        "input tokens" => "输入令牌",
        "output tokens" => "输出令牌",
        "text calls" => "文本调用",
        "image calls" => "图片调用",
        "tts calls" => "语音调用",
        "moderation calls" => "审核调用",
        "cost units" => "成本单位",
        "no usage recorded" => "尚无用量记录",
        _ => label,
    }
}

/// Render the export profile target as a stable machine-readable label.
pub fn export_profile_target_label(target: &ExportProfileTarget) -> &'static str {
    match target {
        ExportProfileTarget::StaticWeb => "static_web",
        ExportProfileTarget::DynamicWeb => "dynamic_web",
        ExportProfileTarget::DesktopBundle => "desktop_bundle",
        ExportProfileTarget::SteamWorkshop => "steam_workshop",
        ExportProfileTarget::SteamSubmissionKit => "steam_submission_kit",
    }
}

/// Print a workshop package validation summary line.
pub fn print_workshop_validation(
    language: OutputLanguage,
    label: &str,
    report: &WorkshopPackageValidationReport,
) {
    let byte_total: u64 = report.files.iter().map(|file| file.byte_length).sum();
    let label = match (language, label) {
        (OutputLanguage::Zh, "workshop package ok") => "Workshop 包通过",
        (OutputLanguage::Zh, "validated imported package") => "已验证导入包",
        (OutputLanguage::Zh, "validated loaded package") => "已验证加载包",
        (OutputLanguage::Zh, "validated remixed package") => "已验证 Remix 包",
        _ => label,
    };
    println!(
        "{label}: {} title={} files={} bytes={}",
        report.manifest.package_id,
        report.manifest.title,
        report.files.len(),
        byte_total
    );
}

/// Print the `check` command summary with a three-tier colorized header:
/// green when the project has scenes, rules, characters, and scene beats;
/// yellow when validation passed but the project has empty content; red is
/// reserved for validation failures (surfaced by `handle_check` before this
/// runs). Color is TTY-only, so pipes/tests see plain `ok: <title> (...)`.
pub fn render_check_summary(language: OutputLanguage, project: &ProjectData) {
    let title = &project.game.title;
    let scene_count = project.scenes.len();
    let rule_count = project.rules.len();
    let character_count = project.characters.len();

    let header = match language {
        OutputLanguage::En => format!(
            "ok: {} ({} scenes, {} rules, {} characters)",
            title, scene_count, rule_count, character_count
        ),
        OutputLanguage::Zh => format!(
            "通过：{}（{} 个场景，{} 条规则，{} 个角色）",
            title, scene_count, rule_count, character_count
        ),
    };

    let warnings = collect_check_warnings(project, language);
    let header_color = if warnings.is_empty() { "32" } else { "33" };
    println!("{}", paint(header_color, &header));

    for warning in &warnings {
        println!("{}", paint("33", warning));
    }
}

/// Collect human-readable warnings for empty project content. Empty scenes,
/// rules, characters, or scene-beat lists are surfaced as yellow advisory
/// lines (not errors) so creators can iterate on partial projects.
fn collect_check_warnings(project: &ProjectData, language: OutputLanguage) -> Vec<String> {
    let mut warnings = Vec::new();
    let warn = |en: &str, zh: &str| match language {
        OutputLanguage::En => en.to_string(),
        OutputLanguage::Zh => zh.to_string(),
    };
    if project.scenes.is_empty() {
        warnings.push(warn("no scenes", "无场景"));
    }
    if project.rules.is_empty() {
        warnings.push(warn("no rules", "无规则"));
    }
    if project.characters.is_empty() {
        warnings.push(warn("no characters", "无角色"));
    }
    for scene in &project.scenes {
        if scene.beats.is_empty() {
            warnings.push(match language {
                OutputLanguage::En => format!("scene {} has no beats", scene.key),
                OutputLanguage::Zh => format!("场景 {} 无节拍", scene.key),
            });
        }
    }
    warnings
}

/// Wrap `text` in an ANSI color code only when stdout is a TTY. Non-TTY
/// (pipes, redirects, test harnesses) get the plain text back unchanged.
fn paint(code: &str, text: &str) -> String {
    if std::io::stdout().is_terminal() {
        format!("\x1b[{code}m{text}\x1b[0m")
    } else {
        text.to_string()
    }
}
