//! Guidance framework and rule evaluation logic for suggesting next diagnostic actions.

use crate::config::AppConfig;
use crate::findings::FindingKind;
use crate::triage::{
    ActionClass, ActionKind, NextAction, NextActionCommand, NextActionPriority, NextActionStatus,
    OperatingMode, PgTriageReport, RiskLabel, Verdict,
};
use serde::{Deserialize, Serialize};

/// The default limit for the number of recommended actions produced by a rule.
pub const DEFAULT_RULE_LIMIT: usize = 10;

/// Unique identifiers for the rules in the guidance framework.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RuleId {
    /// Inspect-level rule to run Top Query Families suggestion.
    #[serde(rename = "inspect.top_query_families")]
    InspectTopQueryFamilies,
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
    /// Running-query-level rule to inspect details for a specific backend PID.
    #[serde(rename = "running_query.pg_stat_activity.by_pid")]
    RunningQueryPgStatActivityByPid,
    /// Running-query-level rule to inspect blocking context for a specific backend PID.
    #[serde(rename = "running_query.blocking.by_pid")]
    RunningQueryBlockingByPid,
}

impl std::fmt::Display for RuleId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            RuleId::InspectTopQueryFamilies => "inspect.top_query_families",
            RuleId::InspectRunningQueries => "inspect.running_queries",
            RuleId::InspectAgentInstall => "inspect.agent_install",
            RuleId::QueryFamilyPgStatStatementsByQueryId => {
                "query_family.pg_stat_statements.by_queryid"
            }
            RuleId::QueryFamilyPgStatActivityByDimensions => {
                "query_family.pg_stat_activity.by_dimensions"
            }
            RuleId::RunningQueryPgStatActivityByPid => "running_query.pg_stat_activity.by_pid",
            RuleId::RunningQueryBlockingByPid => "running_query.blocking.by_pid",
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

/// Helper to instantiate a `NextAction` from a `RuleDefinition` with resolved status, reason, target, command, and sql_preview.
pub fn build_next_action(
    rule: &RuleDefinition,
    status: NextActionStatus,
    reason: String,
    target: Option<String>,
    command: Option<NextActionCommand>,
    sql_preview: Option<String>,
) -> NextAction {
    let action_id = match &target {
        Some(t) => format!("{}:{}", rule.emitted_action_id, t),
        None => rule.emitted_action_id.to_string(),
    };

    NextAction {
        action_id,
        kind: rule.kind,
        label: rule.label.clone(),
        status,
        priority: rule.priority,
        judgement_required: true,
        reason,
        target,
        workflow: rule.destination_workflow,
        command,
        sql_preview,
        parameters: None,
        risk: rule.risk,
        action_class: rule.action_class,
        required_identifiers: if rule.required_identifiers.is_empty() {
            None
        } else {
            Some(rule.required_identifiers.clone())
        },
        requires: rule.required_operating_mode.map(|mode| {
            vec![match mode {
                OperatingMode::LogBackedAndLive => "operating_mode:log_backed_and_live".to_string(),
                OperatingMode::LogBackedOnly => "operating_mode:log_backed".to_string(),
                OperatingMode::LiveOnly => "operating_mode:live_only".to_string(),
                OperatingMode::Unready => "operating_mode:unready".to_string(),
            }]
        }),
        produces: if rule.produces.is_empty() {
            None
        } else {
            Some(rule.produces.clone())
        },
    }
}

/// Populates recommended next actions in the triage report using evaluated rules based on
/// the report's current workflow state, operating mode, verdict, and database findings.
pub fn populate_next_actions<T: GuidancePayload>(
    report: &mut PgTriageReport<T>,
    config: &AppConfig,
) {
    report.next_actions =
        report
            .payload
            .evaluate_rules(report.operating_mode, report.verdict, config);
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
    use crate::triage::{CheckStatus, NextActionStatus};

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
