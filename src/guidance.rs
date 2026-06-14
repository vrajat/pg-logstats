//! Guidance framework and rule evaluation logic for suggesting next diagnostic actions.

use crate::config::AppConfig;
use crate::findings::FindingKind;
use crate::triage::{
    ActionClass, ActionKind, NextAction, NextActionCommand, NextActionPriority, NextActionStatus,
    NextActionType, OperatingMode, PgTriageReport, PromptUserSurvey, RiskLabel, Verdict,
};
use serde::{Deserialize, Serialize};

/// The default limit for the number of recommended actions produced by a rule.
pub const DEFAULT_RULE_LIMIT: usize = 10;

/// Context required to turn generic next-action templates into replayable commands.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ActionReplayContext {
    /// Resolved workspace path for this investigation branch.
    pub workspace: Option<String>,
    /// Input arguments required to replay log-backed workflows.
    pub log_input_args: Vec<String>,
}

/// Unique identifiers for the rules in the guidance framework.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RuleId {
    /// Inspect-level rule to run Top Query Families suggestion.
    #[serde(rename = "inspect.top_query_families")]
    InspectTopQueryFamilies,
    /// Inspect-level rule to run Errors suggestion.
    #[serde(rename = "inspect.errors")]
    InspectErrors,
    /// Inspect-level rule to run Temp Files suggestion.
    #[serde(rename = "inspect.temp_files")]
    InspectTempFiles,
    /// Inspect-level rule to suggest running query inspection.
    #[serde(rename = "inspect.running_queries")]
    InspectRunningQueries,
    /// Inspect-level rule to install agent skills.
    #[serde(rename = "inspect.agent_install")]
    InspectAgentInstall,
    /// Query-family-level rule to check stats views for an exact queryid.
    #[serde(rename = "query_family.pg_stat_statements.by_queryid")]
    QueryFamilyPgStatStatementsByQueryId,
    /// Query-family-level rule to inspect active sessions by finding dimensions.
    #[serde(rename = "query_family.pg_stat_activity.by_dimensions")]
    QueryFamilyPgStatActivityByDimensions,
    /// Query-family-level rule to run EXPLAIN on the query family.
    #[serde(rename = "query_family.explain")]
    QueryFamilyExplain,
    /// Query-family-level rule to run EXPLAIN ANALYZE on the query family.
    #[serde(rename = "query_family.explain_analyze")]
    QueryFamilyExplainAnalyze,
    /// Running-query-level rule to inspect details for a specific backend PID.
    #[serde(rename = "running_query.pg_stat_activity.by_pid")]
    RunningQueryPgStatActivityByPid,
    /// Running-query-level rule to inspect blocking context for a specific backend PID.
    #[serde(rename = "running_query.blocking.by_pid")]
    RunningQueryBlockingByPid,
    /// Error-class-level rule to inspect active sessions by finding dimensions.
    #[serde(rename = "error_class.pg_stat_activity.by_dimensions")]
    ErrorClassPgStatActivityByDimensions,
    /// Temp-file-level rule to check database temp counters.
    #[serde(rename = "temp_file.pg_stat_database.temp_counters")]
    TempFilePgStatDatabaseTempCounters,
    /// Temp-file-level rule to check pg_stat_statements temp block activity.
    #[serde(rename = "temp_file.pg_stat_statements.temp_blocks")]
    TempFilePgStatStatementsTempBlocks,
    /// Temp-file-level rule to run EXPLAIN on the target query.
    #[serde(rename = "temp_file.explain")]
    TempFileExplain,
    /// Temp-file-level rule to run EXPLAIN ANALYZE on the target query.
    #[serde(rename = "temp_file.explain_analyze")]
    TempFileExplainAnalyze,
}

impl std::fmt::Display for RuleId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            RuleId::InspectTopQueryFamilies => "inspect.top_query_families",
            RuleId::InspectErrors => "inspect.errors",
            RuleId::InspectTempFiles => "inspect.temp_files",
            RuleId::InspectRunningQueries => "inspect.running_queries",
            RuleId::InspectAgentInstall => "inspect.agent_install",
            RuleId::QueryFamilyPgStatStatementsByQueryId => {
                "query_family.pg_stat_statements.by_queryid"
            }
            RuleId::QueryFamilyPgStatActivityByDimensions => {
                "query_family.pg_stat_activity.by_dimensions"
            }
            RuleId::QueryFamilyExplain => "query_family.explain",
            RuleId::QueryFamilyExplainAnalyze => "query_family.explain_analyze",
            RuleId::RunningQueryPgStatActivityByPid => "running_query.pg_stat_activity.by_pid",
            RuleId::RunningQueryBlockingByPid => "running_query.blocking.by_pid",
            RuleId::ErrorClassPgStatActivityByDimensions => {
                "error_class.pg_stat_activity.by_dimensions"
            }
            RuleId::TempFilePgStatDatabaseTempCounters => {
                "temp_file.pg_stat_database.temp_counters"
            }
            RuleId::TempFilePgStatStatementsTempBlocks => {
                "temp_file.pg_stat_statements.temp_blocks"
            }
            RuleId::TempFileExplain => "temp_file.explain",
            RuleId::TempFileExplainAnalyze => "temp_file.explain_analyze",
        };
        write!(f, "{}", s)
    }
}

/// Definition of a recommendation rule within the guidance engine.
#[derive(Debug, Clone)]
pub struct RuleDefinition {
    /// The unique identifier of this rule.
    pub rule_id: RuleId,
    /// The action ID template emitted by this rule.
    pub emitted_action_id: RuleId,
    /// The kind of action this rule produces.
    pub kind: ActionKind,
    /// The workflow/action category that this rule applies to.
    pub target_workflow: ActionKind,
    /// The type of diagnostic finding this rule targets. Payload-specific rule
    /// evaluators use this as a filter before attempting to instantiate an
    /// action for an individual finding.
    pub target_finding_kind: Option<FindingKind>,
    /// The workflow category the recommended action will transition into.
    pub destination_workflow: Option<ActionKind>,
    /// Identifiers required in context to populate templates for this action.
    /// This is emitted as action metadata; payload evaluators still enforce the
    /// actual missing-context checks explicitly.
    pub required_identifiers: Vec<String>,
    /// Human-readable description of the recommended action.
    pub label: String,
    /// The diagnostic rationale/justification for recommending this action.
    pub reason: String,
    /// The priority/severity recommendation for this action.
    pub priority: NextActionPriority,
    /// Risk associated with executing the action.
    pub risk: Option<RiskLabel>,
    /// Classification category for the action's operations.
    pub action_class: Option<ActionClass>,
    /// Command line argv template to invoke this action.
    pub command_template: Option<Vec<String>>,
    /// Pre-templated SQL query pattern for run-sql actions.
    pub sql_template: Option<String>,
    /// Mode and capability prerequisites required to enable this action.
    pub required_operating_mode: Option<OperatingMode>,
    /// Capabilities or diagnostic context produced by running this action.
    pub produces: Vec<String>,
    /// Source of attribution or policy guidelines justifying this recommendation.
    pub attribution: String,
}

/// Trait implemented by triage payloads to evaluate next actions.
pub trait GuidancePayload {
    /// Evaluates the payload against rules and config to recommend next actions.
    fn evaluate_rules(
        &self,
        operating_mode: OperatingMode,
        verdict: Option<Verdict>,
        config: &AppConfig,
    ) -> Vec<NextAction>;

    /// Emits report-level next actions that are not tied to a single finding.
    fn supplemental_actions(
        &self,
        _workflow: ActionKind,
        _operating_mode: OperatingMode,
        _verdict: Option<Verdict>,
        _config: &AppConfig,
    ) -> Vec<NextAction> {
        Vec::new()
    }
}

impl GuidancePayload for serde_json::Value {
    fn evaluate_rules(
        &self,
        _operating_mode: OperatingMode,
        _verdict: Option<Verdict>,
        _config: &AppConfig,
    ) -> Vec<NextAction> {
        vec![]
    }
}

/// Evaluates common safety, mode, and configuration constraints for a rule.
/// Returns the resolved `NextActionStatus` and a reason string.
pub fn evaluate_rule_constraints(
    rule: &RuleDefinition,
    operating_mode: OperatingMode,
    verdict: Option<Verdict>,
    config: &AppConfig,
) -> (NextActionStatus, String) {
    let mut status = NextActionStatus::Allowed;
    let mut reason = rule.reason.clone();

    if config.guidance.disabled_rules.contains(&rule.rule_id) {
        status = NextActionStatus::BlockedByConfig;
        reason = format!("Blocked by config: rule {} is disabled", rule.rule_id);
    } else if let Some(rule_cfg) = config.guidance.rules.get(&rule.rule_id) {
        if !rule_cfg.enabled {
            status = NextActionStatus::BlockedByConfig;
            reason = format!("Blocked by config: rule {} is disabled", rule.rule_id);
        }
    }

    if status == NextActionStatus::Allowed {
        if let Some(req_mode) = rule.required_operating_mode {
            let is_compatible = match req_mode {
                OperatingMode::LogBackedAndLive => {
                    operating_mode == OperatingMode::LogBackedAndLive
                }
                OperatingMode::LogBackedOnly => {
                    operating_mode == OperatingMode::LogBackedOnly
                        || operating_mode == OperatingMode::LogBackedAndLive
                }
                OperatingMode::LiveOnly => {
                    operating_mode == OperatingMode::LiveOnly
                        || operating_mode == OperatingMode::LogBackedAndLive
                }
                OperatingMode::Unready => true,
            };
            if !is_compatible {
                status = NextActionStatus::BlockedByMode;
                reason = format!(
                    "Action requires {} operating capability",
                    match req_mode {
                        OperatingMode::LogBackedAndLive => "log-backed and live",
                        OperatingMode::LogBackedOnly => "log-backed",
                        OperatingMode::LiveOnly => "live database",
                        OperatingMode::Unready => "unready",
                    }
                );
            }
        }
    }

    if status == NextActionStatus::Allowed {
        if let Some(max_r) = rule.risk {
            if max_r > config.guidance.max_risk {
                status = NextActionStatus::BlockedByConfig;
                reason = format!(
                    "Blocked by config: risk level {:?} exceeds max_risk {:?}",
                    max_r, config.guidance.max_risk
                );
            }
        }
    }

    if status == NextActionStatus::Allowed {
        if let Some(verdict) = verdict {
            if let Some(class) = rule.action_class {
                let is_allowed = match verdict {
                    Verdict::Clear => matches!(
                        class,
                        ActionClass::SystemCatalogReads
                            | ActionClass::StatsViewReads
                            | ActionClass::BoundedActivityQueries
                            | ActionClass::TextPatternStatsSearch
                            | ActionClass::ExplainWithoutAnalyze
                    ),
                    Verdict::Busy => matches!(
                        class,
                        ActionClass::SystemCatalogReads
                            | ActionClass::StatsViewReads
                            | ActionClass::BoundedActivityQueries
                    ),
                    Verdict::Saturated => false,
                    Verdict::Unknown => false,
                };
                if !is_allowed {
                    status = NextActionStatus::BlockedByVerdict;
                    reason = format!(
                        "Blocked by verdict {:?}: action class {:?} is not allowed",
                        verdict, class
                    );
                }
            }
        }
    }

    (status, reason)
}

/// Helper to instantiate a `NextAction` from a `RuleDefinition` with resolved status,
/// reason, target, and command.
pub fn build_next_action(
    rule: &RuleDefinition,
    status: NextActionStatus,
    reason: String,
    target: Option<String>,
    command: Option<NextActionCommand>,
) -> NextAction {
    let action_id = match &target {
        Some(t) => format!("{}:{}", rule.emitted_action_id, t),
        None => rule.emitted_action_id.to_string(),
    };

    NextAction {
        action_id,
        action_type: match rule.kind {
            ActionKind::RunSql => NextActionType::RunSql,
            ActionKind::Stop => NextActionType::Stop,
            _ => NextActionType::RunWorkflow,
        },
        label: rule.label.clone(),
        status,
        priority: rule.priority,
        judgement_required: true,
        reason,
        target_id: target,
        command,
        survey: None,
        parameters: None,
        risk: rule.risk,
        action_class: rule.action_class,
    }
}

/// Populates recommended next actions in the triage report using evaluated rules based on
/// the report's current workflow state, operating mode, verdict, and database findings.
pub fn populate_next_actions<T: GuidancePayload>(
    report: &mut PgTriageReport<T>,
    config: &AppConfig,
) {
    populate_next_actions_with_context(report, config, &ActionReplayContext::default());
}

/// Populates next actions and hydrates them into replayable CLI commands using
/// the provided execution context.
pub fn populate_next_actions_with_context<T: GuidancePayload>(
    report: &mut PgTriageReport<T>,
    config: &AppConfig,
    replay_context: &ActionReplayContext,
) {
    let mut next_actions: Vec<_> = report
        .payload
        .evaluate_rules(report.operating_mode, report.verdict, config)
        .into_iter()
        .filter(|action| action.status == NextActionStatus::Allowed)
        .collect();
    next_actions.extend(report.payload.supplemental_actions(
        report.workflow,
        report.operating_mode,
        report.verdict,
        config,
    ));
    hydrate_replayable_commands(
        &mut next_actions,
        replay_context,
        report.report_id.as_deref(),
    );
    report.next_actions = next_actions;
}

fn hydrate_replayable_commands(
    next_actions: &mut [NextAction],
    replay_context: &ActionReplayContext,
    parent_report_id: Option<&str>,
) {
    for action in next_actions {
        hydrate_next_action_command(action, replay_context, parent_report_id);
        if let Some(survey) = action.survey.as_mut() {
            hydrate_prompt_user_survey(survey, replay_context);
        }
    }
}

fn hydrate_next_action_command(
    action: &mut NextAction,
    replay_context: &ActionReplayContext,
    parent_report_id: Option<&str>,
) {
    if action.action_type == NextActionType::RunSql {
        action.command = Some(run_sql_replay_command(
            &action.action_id,
            replay_context,
            parent_report_id,
        ));
        return;
    }

    let Some(command) = action.command.as_mut() else {
        return;
    };

    let workflow = infer_workflow_from_command_argv(&command.argv);
    command.argv = hydrate_command_argv(&command.argv, workflow, replay_context);
}

fn hydrate_prompt_user_survey(survey: &mut PromptUserSurvey, replay_context: &ActionReplayContext) {
    for choice in &mut survey.choices {
        let Some(command) = choice.command.as_mut() else {
            continue;
        };

        command.argv = hydrate_command_argv(&command.argv, choice.workflow, replay_context);
    }
}

fn hydrate_command_argv(
    base_argv: &[String],
    workflow: Option<ActionKind>,
    replay_context: &ActionReplayContext,
) -> Vec<String> {
    if base_argv.is_empty() {
        return Vec::new();
    }

    let mut argv = Vec::with_capacity(
        base_argv.len()
            + usize::from(replay_context.workspace.is_some()) * 2
            + replay_context.log_input_args.len(),
    );
    argv.push(base_argv[0].clone());

    if let Some(workspace) = &replay_context.workspace {
        argv.push("--workspace".to_string());
        argv.push(workspace.clone());
    }

    argv.extend(base_argv.iter().skip(1).cloned());

    if workflow_uses_log_input(workflow) {
        argv.extend(replay_context.log_input_args.iter().cloned());
    }

    argv
}

fn infer_workflow_from_command_argv(base_argv: &[String]) -> Option<ActionKind> {
    match base_argv.get(1).map(String::as_str) {
        Some("inspect") => Some(ActionKind::Inspect),
        Some("query-families") => Some(ActionKind::TopQueryFamilies),
        Some("errors") => Some(ActionKind::Errors),
        Some("temp-files") => Some(ActionKind::TempFiles),
        Some("running-queries") => Some(ActionKind::RunningQueries),
        Some("agent") => Some(ActionKind::AgentInstall),
        Some("run-sql") => Some(ActionKind::RunSql),
        _ => None,
    }
}

fn workflow_uses_log_input(workflow: Option<ActionKind>) -> bool {
    matches!(
        workflow,
        Some(
            ActionKind::Inspect
                | ActionKind::TopQueryFamilies
                | ActionKind::Errors
                | ActionKind::TempFiles
        )
    )
}

fn run_sql_replay_command(
    action_id: &str,
    replay_context: &ActionReplayContext,
    parent_report_id: Option<&str>,
) -> NextActionCommand {
    let mut argv = vec!["pg-logstats".to_string()];

    if let Some(workspace) = &replay_context.workspace {
        argv.push("--workspace".to_string());
        argv.push(workspace.clone());
    }

    if let Some(report_id) = parent_report_id {
        argv.push("--triage-report".to_string());
        argv.push(report_id.to_string());
    }

    argv.push("--action-id".to_string());
    argv.push(action_id.to_string());
    argv.push("run-sql".to_string());

    NextActionCommand { argv }
}

pub fn running_query_rules() -> Vec<RuleDefinition> {
    vec![
        RuleDefinition {
            rule_id: RuleId::RunningQueryPgStatActivityByPid,
            emitted_action_id: RuleId::RunningQueryPgStatActivityByPid,
            kind: ActionKind::RunSql,
            target_workflow: ActionKind::RunningQueries,
            target_finding_kind: None,
            destination_workflow: Some(ActionKind::RunSql),
            required_identifiers: vec!["pid".to_string()],
            label: "Inspect backend details for target PID".to_string(),
            reason: "The session is active or waiting. Run this to check transaction/query timing and state details for the backend.".to_string(),
            priority: NextActionPriority::Recommended,
            risk: Some(RiskLabel::Safe),
            action_class: Some(ActionClass::BoundedActivityQueries),
            command_template: None,
            sql_template: Some("SELECT pid, usename, datname, application_name, client_addr, backend_start, xact_start, query_start, state_change, wait_event_type, wait_event, state, query_id, query FROM pg_stat_activity WHERE pid = $1;".to_string()),
            required_operating_mode: Some(OperatingMode::LiveOnly),
            produces: vec!["workflow:sql_action".to_string()],
            attribution: "PostgreSQL pg_stat_activity single backend query".to_string(),
        },
        RuleDefinition {
            rule_id: RuleId::RunningQueryBlockingByPid,
            emitted_action_id: RuleId::RunningQueryBlockingByPid,
            kind: ActionKind::RunSql,
            target_workflow: ActionKind::RunningQueries,
            target_finding_kind: None,
            destination_workflow: Some(ActionKind::RunSql),
            required_identifiers: vec!["pid".to_string()],
            label: "Inspect blocking sessions for target PID".to_string(),
            reason: "The session is waiting/blocked. Run this to check the blockers using pg_blocking_pids.".to_string(),
            priority: NextActionPriority::Recommended,
            risk: Some(RiskLabel::Safe),
            action_class: Some(ActionClass::BoundedActivityQueries),
            command_template: None,
            sql_template: Some("SELECT pid, usename, datname, application_name, state, wait_event_type, wait_event, query FROM pg_stat_activity WHERE pid = ANY(pg_blocking_pids($1));".to_string()),
            required_operating_mode: Some(OperatingMode::LiveOnly),
            produces: vec!["workflow:sql_action".to_string()],
            attribution: "PostgreSQL pg_blocking_pids dependency check".to_string(),
        },
    ]
}

/// Escapes single quotes in database text literals to prevent SQL injection.
pub fn escape_sql_literal(s: &str) -> String {
    s.replace("'", "''")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::inspect::{AgentInspect, AgentTargetInspect, DatabaseInspect, InspectReportPayload};
    use crate::triage::{
        ActionKind, CheckStatus, NextActionStatus, PgTriageReport, PG_TRIAGE_SCHEMA_VERSION,
    };

    #[test]
    fn test_inspect_rules_recommends_top_query_families_when_log_backed_and_live() {
        let payload = InspectReportPayload {
            database_inspect: DatabaseInspect {
                mode_candidate: OperatingMode::LogBackedAndLive,
                checks: std::collections::BTreeMap::new(),
            },
            agent_inspect: AgentInspect {
                active_harness: Some("codex".to_string()),
                codex: AgentTargetInspect {
                    status: CheckStatus::Passed,
                    installed: true,
                    install_location: String::new(),
                },
                claude: AgentTargetInspect {
                    status: CheckStatus::Passed,
                    installed: true,
                    install_location: String::new(),
                },
                gemini: AgentTargetInspect {
                    status: CheckStatus::Passed,
                    installed: true,
                    install_location: String::new(),
                },
            },
            required_checks: Vec::new(),
            failed_checks: Vec::new(),
        };
        let config = AppConfig::default();
        let actions = payload.evaluate_rules(OperatingMode::LogBackedAndLive, None, &config);

        let top_queries_act = actions
            .iter()
            .find(|a| a.action_id == "inspect.top_query_families")
            .unwrap();
        assert_eq!(top_queries_act.status, NextActionStatus::Allowed);

        let running_queries_act = actions
            .iter()
            .find(|a| a.action_id == "inspect.running_queries")
            .unwrap();
        assert_eq!(running_queries_act.status, NextActionStatus::Allowed);
    }

    #[test]
    fn test_inspect_rules_recommends_top_query_families_when_log_backed_only() {
        let payload = InspectReportPayload {
            database_inspect: DatabaseInspect {
                mode_candidate: OperatingMode::LogBackedOnly,
                checks: std::collections::BTreeMap::new(),
            },
            agent_inspect: AgentInspect {
                active_harness: Some("codex".to_string()),
                codex: AgentTargetInspect {
                    status: CheckStatus::Passed,
                    installed: true,
                    install_location: String::new(),
                },
                claude: AgentTargetInspect {
                    status: CheckStatus::Passed,
                    installed: true,
                    install_location: String::new(),
                },
                gemini: AgentTargetInspect {
                    status: CheckStatus::Passed,
                    installed: true,
                    install_location: String::new(),
                },
            },
            required_checks: Vec::new(),
            failed_checks: Vec::new(),
        };
        let config = AppConfig::default();
        let actions = payload.evaluate_rules(OperatingMode::LogBackedOnly, None, &config);

        let top_queries_act = actions
            .iter()
            .find(|a| a.action_id == "inspect.top_query_families")
            .unwrap();
        assert_eq!(top_queries_act.status, NextActionStatus::Allowed);

        let running_queries_act = actions
            .iter()
            .find(|a| a.action_id == "inspect.running_queries")
            .unwrap();
        assert_eq!(running_queries_act.status, NextActionStatus::BlockedByMode);
    }

    #[test]
    fn test_populate_next_actions_keeps_only_allowed_actions() {
        let payload = InspectReportPayload {
            database_inspect: DatabaseInspect {
                mode_candidate: OperatingMode::LogBackedOnly,
                checks: std::collections::BTreeMap::new(),
            },
            agent_inspect: AgentInspect {
                active_harness: Some("codex".to_string()),
                codex: AgentTargetInspect {
                    status: CheckStatus::Passed,
                    installed: true,
                    install_location: String::new(),
                },
                claude: AgentTargetInspect {
                    status: CheckStatus::Passed,
                    installed: true,
                    install_location: String::new(),
                },
                gemini: AgentTargetInspect {
                    status: CheckStatus::Passed,
                    installed: true,
                    install_location: String::new(),
                },
            },
            required_checks: Vec::new(),
            failed_checks: Vec::new(),
        };

        let mut report = PgTriageReport {
            schema_version: PG_TRIAGE_SCHEMA_VERSION,
            workflow: ActionKind::Inspect,
            operating_mode: OperatingMode::LogBackedOnly,
            limitations: Vec::new(),
            verdict: None,
            verdict_reasons: Vec::new(),
            allowed_actions: None,
            blocked_actions: None,
            analysis_window: None,
            source_summary: None,
            next_actions: Vec::new(),
            report_id: None,
            parent_report_id: None,
            selected_action_id: None,
            created_at: None,
            payload,
        };

        populate_next_actions(&mut report, &AppConfig::default());

        assert_eq!(report.next_actions.len(), 3);
        assert!(report.next_actions.iter().any(|a| a.action_id == "inspect.top_query_families" && a.status == NextActionStatus::Allowed));
        assert!(report.next_actions.iter().any(|a| a.action_id == "inspect.errors" && a.status == NextActionStatus::Allowed));
        assert!(report.next_actions.iter().any(|a| a.action_id == "inspect.temp_files" && a.status == NextActionStatus::Allowed));
    }

    #[test]
    fn test_replay_context_hydrates_log_workflow_and_prompt_user_commands() {
        let payload = InspectReportPayload {
            database_inspect: DatabaseInspect {
                mode_candidate: OperatingMode::LogBackedOnly,
                checks: std::collections::BTreeMap::new(),
            },
            agent_inspect: AgentInspect {
                active_harness: Some("codex".to_string()),
                codex: AgentTargetInspect {
                    status: CheckStatus::Passed,
                    installed: true,
                    install_location: String::new(),
                },
                claude: AgentTargetInspect {
                    status: CheckStatus::Passed,
                    installed: true,
                    install_location: String::new(),
                },
                gemini: AgentTargetInspect {
                    status: CheckStatus::Passed,
                    installed: true,
                    install_location: String::new(),
                },
            },
            required_checks: Vec::new(),
            failed_checks: Vec::new(),
        };

        let mut report = PgTriageReport {
            schema_version: PG_TRIAGE_SCHEMA_VERSION,
            workflow: ActionKind::Inspect,
            operating_mode: OperatingMode::LogBackedOnly,
            limitations: Vec::new(),
            verdict: None,
            verdict_reasons: Vec::new(),
            allowed_actions: None,
            blocked_actions: None,
            analysis_window: None,
            source_summary: None,
            next_actions: Vec::new(),
            report_id: Some("20260614T100000000000Z-inspect".to_string()),
            parent_report_id: None,
            selected_action_id: None,
            created_at: None,
            payload,
        };

        populate_next_actions_with_context(
            &mut report,
            &AppConfig::default(),
            &ActionReplayContext {
                workspace: Some("/tmp/workspace".to_string()),
                log_input_args: vec!["/tmp/postgresql.log".to_string()],
            },
        );

        let query_families = report
            .next_actions
            .iter()
            .find(|action| action.action_id == "inspect.top_query_families")
            .unwrap();
        assert_eq!(
            query_families.command.as_ref().unwrap().argv,
            vec![
                "pg-logstats",
                "--workspace",
                "/tmp/workspace",
                "query-families",
                "/tmp/postgresql.log",
            ]
        );
    }

    #[test]
    fn test_replay_context_hydrates_run_sql_commands_with_report_linkage() {
        let mut report = crate::triage::sql_action_report(
            crate::triage::SqlActionPayload {
                action_id: "noop".to_string(),
                source_report_id: None,
                source_finding_id: None,
                insights: Vec::new(),
                row_count: 0,
                truncated: false,
                columns: Vec::new(),
                rows: Vec::new(),
            },
            OperatingMode::LogBackedAndLive,
        );
        report.workflow = ActionKind::TopQueryFamilies;
        report.report_id = Some("20260614T100000000000Z-top_query_families".to_string());
        report.next_actions = vec![NextAction {
            action_id: "query_family.pg_stat_activity.by_dimensions:qf_demo".to_string(),
            action_type: NextActionType::RunSql,
            label: "Inspect active sessions by dimensions".to_string(),
            status: NextActionStatus::Allowed,
            priority: NextActionPriority::Recommended,
            judgement_required: true,
            reason: "demo".to_string(),
            target_id: None,
            command: Some(NextActionCommand {
                argv: vec!["pg-logstats".to_string(), "run-sql".to_string()],
            }),
            survey: None,
            parameters: None,
            risk: None,
            action_class: None,
        }];

        hydrate_replayable_commands(
            &mut report.next_actions,
            &ActionReplayContext {
                workspace: Some("/tmp/workspace".to_string()),
                log_input_args: Vec::new(),
            },
            report.report_id.as_deref(),
        );

        assert_eq!(
            report.next_actions[0].command.as_ref().unwrap().argv,
            vec![
                "pg-logstats",
                "--workspace",
                "/tmp/workspace",
                "--triage-report",
                "20260614T100000000000Z-top_query_families",
                "--action-id",
                "query_family.pg_stat_activity.by_dimensions:qf_demo",
                "run-sql",
            ]
        );
    }

    #[test]
    fn test_inspect_rules_recommends_running_queries_when_live_only() {
        let payload = InspectReportPayload {
            database_inspect: DatabaseInspect {
                mode_candidate: OperatingMode::LiveOnly,
                checks: std::collections::BTreeMap::new(),
            },
            agent_inspect: AgentInspect {
                active_harness: Some("codex".to_string()),
                codex: AgentTargetInspect {
                    status: CheckStatus::Passed,
                    installed: true,
                    install_location: String::new(),
                },
                claude: AgentTargetInspect {
                    status: CheckStatus::Passed,
                    installed: true,
                    install_location: String::new(),
                },
                gemini: AgentTargetInspect {
                    status: CheckStatus::Passed,
                    installed: true,
                    install_location: String::new(),
                },
            },
            required_checks: Vec::new(),
            failed_checks: Vec::new(),
        };
        let config = AppConfig::default();
        let actions = payload.evaluate_rules(OperatingMode::LiveOnly, None, &config);

        let top_queries_act = actions
            .iter()
            .find(|a| a.action_id == "inspect.top_query_families")
            .unwrap();
        assert_eq!(top_queries_act.status, NextActionStatus::BlockedByMode);

        let running_queries_act = actions
            .iter()
            .find(|a| a.action_id == "inspect.running_queries")
            .unwrap();
        assert_eq!(running_queries_act.status, NextActionStatus::Allowed);
    }

    #[test]
    fn test_inspect_rules_recommends_agent_install_when_missing() {
        let payload = InspectReportPayload {
            database_inspect: DatabaseInspect {
                mode_candidate: OperatingMode::LogBackedAndLive,
                checks: std::collections::BTreeMap::new(),
            },
            agent_inspect: AgentInspect {
                active_harness: Some("codex".to_string()),
                codex: AgentTargetInspect {
                    status: CheckStatus::Failed,
                    installed: false,
                    install_location: String::new(),
                },
                claude: AgentTargetInspect {
                    status: CheckStatus::Passed,
                    installed: true,
                    install_location: String::new(),
                },
                gemini: AgentTargetInspect {
                    status: CheckStatus::Passed,
                    installed: true,
                    install_location: String::new(),
                },
            },
            required_checks: Vec::new(),
            failed_checks: Vec::new(),
        };
        let config = AppConfig::default();
        let actions = payload.evaluate_rules(OperatingMode::LogBackedAndLive, None, &config);

        // Since agent install is failed/not installed, agent install should be recommended,
        // AND it should be the only recommended action!
        assert_eq!(actions.len(), 1);
        let agent_install_act = &actions[0];
        assert_eq!(agent_install_act.action_id, "inspect.agent_install");
        assert_eq!(agent_install_act.status, NextActionStatus::Allowed);
        assert!(agent_install_act.reason.contains("codex"));
    }

    #[test]
    fn test_inspect_rules_omits_agent_install_when_all_installed() {
        let payload = InspectReportPayload {
            database_inspect: DatabaseInspect {
                mode_candidate: OperatingMode::LogBackedAndLive,
                checks: std::collections::BTreeMap::new(),
            },
            agent_inspect: AgentInspect {
                active_harness: Some("codex".to_string()),
                codex: AgentTargetInspect {
                    status: CheckStatus::Passed,
                    installed: true,
                    install_location: String::new(),
                },
                claude: AgentTargetInspect {
                    status: CheckStatus::Passed,
                    installed: true,
                    install_location: String::new(),
                },
                gemini: AgentTargetInspect {
                    status: CheckStatus::Passed,
                    installed: true,
                    install_location: String::new(),
                },
            },
            required_checks: Vec::new(),
            failed_checks: Vec::new(),
        };
        let config = AppConfig::default();
        let actions = payload.evaluate_rules(OperatingMode::LogBackedAndLive, None, &config);

        let agent_install_act = actions
            .iter()
            .find(|a| a.action_id == "inspect.agent_install")
            .unwrap();
        assert_eq!(
            agent_install_act.status,
            NextActionStatus::OmittedNotEnoughContext
        );
    }

    #[test]
    fn test_inspect_rules_compatibility_in_unready_mode() {
        let payload = InspectReportPayload {
            database_inspect: DatabaseInspect {
                mode_candidate: OperatingMode::Unready,
                checks: std::collections::BTreeMap::new(),
            },
            agent_inspect: AgentInspect {
                active_harness: Some("codex".to_string()),
                codex: AgentTargetInspect {
                    status: CheckStatus::Passed,
                    installed: true,
                    install_location: String::new(),
                },
                claude: AgentTargetInspect {
                    status: CheckStatus::Passed,
                    installed: true,
                    install_location: String::new(),
                },
                gemini: AgentTargetInspect {
                    status: CheckStatus::Passed,
                    installed: true,
                    install_location: String::new(),
                },
            },
            required_checks: Vec::new(),
            failed_checks: Vec::new(),
        };
        let config = AppConfig::default();
        let actions = payload.evaluate_rules(OperatingMode::Unready, None, &config);

        let top_queries_act = actions
            .iter()
            .find(|a| a.action_id == "inspect.top_query_families")
            .unwrap();
        assert_eq!(top_queries_act.status, NextActionStatus::BlockedByMode);

        let running_queries_act = actions
            .iter()
            .find(|a| a.action_id == "inspect.running_queries")
            .unwrap();
        assert_eq!(running_queries_act.status, NextActionStatus::BlockedByMode);
    }

    #[test]
    fn test_inspect_agent_install_suggested_in_all_modes_when_missing() {
        let modes = [
            OperatingMode::LogBackedAndLive,
            OperatingMode::LogBackedOnly,
            OperatingMode::LiveOnly,
            OperatingMode::Unready,
        ];

        for mode in modes {
            let payload = InspectReportPayload {
                database_inspect: DatabaseInspect {
                    mode_candidate: mode,
                    checks: std::collections::BTreeMap::new(),
                },
                agent_inspect: AgentInspect {
                    active_harness: Some("codex".to_string()),
                    codex: AgentTargetInspect {
                        status: CheckStatus::Failed,
                        installed: false,
                        install_location: String::new(),
                    },
                    claude: AgentTargetInspect {
                        status: CheckStatus::Passed,
                        installed: true,
                        install_location: String::new(),
                    },
                    gemini: AgentTargetInspect {
                        status: CheckStatus::Passed,
                        installed: true,
                        install_location: String::new(),
                    },
                },
                required_checks: Vec::new(),
                failed_checks: Vec::new(),
            };
            let config = AppConfig::default();
            let actions = payload.evaluate_rules(mode, None, &config);

            // In all modes, if the active agent is missing, AgentInstall is suggested
            // AND it is the only recommended action!
            assert_eq!(
                actions.len(),
                1,
                "Expected exactly 1 action in mode {:?}",
                mode
            );
            let agent_install_act = &actions[0];
            assert_eq!(agent_install_act.action_id, "inspect.agent_install");
            assert_eq!(agent_install_act.status, NextActionStatus::Allowed);
            assert!(agent_install_act.reason.contains("codex"));
        }
    }
}
