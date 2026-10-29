mod common;

use common::{Sandbox, assert_shows};
use serde_json::{Value, json};
use std::fs;

const PLAIN: [&str; 6] = [
    "--style",
    "plain",
    "--icon-set",
    "plain",
    "--theme",
    "plain",
];
const LISTED: &str = "segments = [\"tokens\", \"fusion\"]\n";

fn event(kind: &str, data: Value) -> Value {
    json!({"type": kind, "data": data})
}

fn resolved(id: &str, pattern: &str) -> Value {
    event(
        "session.fusion_resolved",
        json!({
            "fusionId": id,
            "syntheticModel": "hydrafusion",
            "pattern": pattern,
            "primaryModel": "solver",
            "judgeModel": "judge",
            "repairModel": "repair"
        }),
    )
}

fn refresh(sandbox: &Sandbox, events: &[Value]) -> String {
    refresh_as(sandbox, events, "auto")
}

fn refresh_as(sandbox: &Sandbox, events: &[Value], model: &str) -> String {
    let dir = sandbox.copilot_dir().join("session-state/wiring");
    fs::create_dir_all(&dir).unwrap();
    let lines: Vec<_> = events.iter().map(Value::to_string).collect();
    fs::write(dir.join("events.jsonl"), lines.join("\n") + "\n").unwrap();
    let payload = json!({
        "session_id": "wiring",
        "transcript_path": dir,
        "model": {"id": model, "display_name": "Copilot Fusion"},
        "context_window": {
            "current_context_tokens": 59000,
            "displayed_context_limit": 200000,
            "current_context_used_percentage": 29
        }
    });
    sandbox.refresh(&payload.to_string(), &PLAIN)
}

#[test]
fn test_routing_plan_does_not_claim_its_models_have_run() {
    let sandbox = Sandbox::new();
    sandbox.write_config(LISTED);

    let line = refresh(&sandbox, &[resolved("current", "cascade")]);

    assert_shows(&line, "Fusion: cascade (planned)");
    assert_shows(&line, "Tokens: 59k/200k (29%)");
    assert!(!line.contains("solver"));
    assert!(!line.contains("judge"));
    assert!(!line.contains("repair"));
}

#[test]
fn test_shows_observed_models_and_roles_without_claiming_a_live_phase() {
    let sandbox = Sandbox::new();
    sandbox.write_config(LISTED);
    let mut events = vec![
        resolved("current", "critique"),
        event(
            "session.usage_record",
            json!({"usage": {
                "model": "actual-solver",
                "fusion": {
                    "fusionId": "current",
                    "pattern": "critique",
                    "phaseKind": "draft"
                }
            }}),
        ),
    ];

    let line = refresh(&sandbox, &events);
    assert_shows(&line, "Fusion: critique: draft=actual-solver (observed)");
    assert!(!line.contains("running"));

    events.push(event(
        "assistant.fusion_phase_completed",
        json!({
            "fusionId": "current",
            "phaseKind": "critic",
            "model": "actual-critic",
            "status": "succeeded"
        }),
    ));
    let line = refresh(&sandbox, &events);
    assert_shows(
        &line,
        "Fusion: critique: draft=actual-solver, critic=actual-critic (observed)",
    );
}

#[test]
fn test_cascade_handoff_is_not_execution_and_completion_identifies_the_final_model() {
    let sandbox = Sandbox::new();
    sandbox.write_config(LISTED);
    let mut events = vec![
        resolved("current", "cascade"),
        event(
            "assistant.fusion_phase_completed",
            json!({"fusionId": "current", "phaseKind": "primary", "model": "solver"}),
        ),
        event(
            "session.fusion_handoff",
            json!({"fusionId": "current", "targetModel": "repair"}),
        ),
    ];
    let line = refresh(&sandbox, &events);
    assert_shows(
        &line,
        "Fusion: cascade: primary=solver, handoff=repair (handoff)",
    );
    assert!(!line.contains("repair=repair"));

    events.push(event(
        "assistant.fusion_phase_completed",
        json!({"fusionId": "current", "phaseKind": "repair", "model": "repair"}),
    ));
    events.push(event(
        "session.fusion_completed",
        json!({
            "fusionId": "current",
            "pattern": "cascade",
            "outcome": "completed",
            "finalSourceModel": "repair"
        }),
    ));
    let line = refresh(&sandbox, &events);
    assert_shows(
        &line,
        "Fusion: cascade: primary=solver, repair=repair, final=repair (completed, escalated)",
    );
}

#[test]
fn test_new_routing_discards_old_models_and_ignores_subagents_and_other_fusion_ids() {
    let sandbox = Sandbox::new();
    sandbox.write_config(LISTED);
    let mut subagent = resolved("subagent", "cascade");
    subagent["agentId"] = json!("child");
    let events = [
        resolved("old", "cascade"),
        event(
            "assistant.fusion_phase_completed",
            json!({"fusionId": "old", "phaseKind": "repair", "model": "old-repair"}),
        ),
        resolved("current", "single"),
        event(
            "assistant.fusion_phase_completed",
            json!({"fusionId": "old", "phaseKind": "critic", "model": "late-critic"}),
        ),
        subagent,
    ];
    let line = refresh(&sandbox, &events);
    assert_shows(&line, "Fusion: single (planned)");
    assert!(!line.contains("old-repair"));
    assert!(!line.contains("late-critic"));

    let mut changed = events.to_vec();
    changed.push(event(
        "session.model_change",
        json!({"previousModel": "auto", "newModel": "auto"}),
    ));
    let line = refresh(&sandbox, &changed);
    assert!(!line.contains("Fusion:"), "stale routing in {line:?}");
}

#[test]
fn test_opt_in_and_display_mode_gate_fusion_without_affecting_other_segments() {
    for config in [
        "segments = [\"tokens\"]\n",
        "segments = [\"tokens\", \"fusion\"]\n[fusion]\nenabled = false\n",
        "mode = \"compact\"\nsegments = [\"fusion\"]\ncompact_segments = [\"tokens\"]\n",
    ] {
        let sandbox = Sandbox::new();
        sandbox.write_config(config);
        let line = refresh(&sandbox, &[resolved("current", "single")]);
        assert_shows(&line, "Tokens:");
        assert!(!line.contains("Fusion:"), "unexpected routing in {line:?}");
    }

    let sandbox = Sandbox::new();
    sandbox.write_config("segments = [\"fusion\"]\n[fusion]\nprefix = \"Route:\"\n");
    let line = refresh(&sandbox, &[resolved("current", "single")]);
    assert_shows(&line, "Route: single (planned)");
    let line = refresh_as(&sandbox, &[resolved("current", "single")], "direct-model");
    assert!(line.is_empty(), "unexpected routing in {line:?}");
}

#[test]
fn test_missing_or_unrecognized_telemetry_hides_quietly() {
    let sandbox = Sandbox::new();
    sandbox.write_config(LISTED);
    for payload in [
        json!({"model": {"id": "auto"}}),
        json!({"model": {"id": "auto"}, "transcript_path": sandbox.home().join("missing")}),
    ] {
        let line = sandbox.refresh(&payload.to_string(), &PLAIN);
        assert_shows(&line, "Tokens:");
        assert!(!line.contains("Fusion:"), "unexpected routing in {line:?}");
    }
    let line = refresh(
        &sandbox,
        &[
            resolved("old", "single"),
            resolved("new", "unknown-pattern"),
        ],
    );
    assert!(!line.contains("Fusion:"), "stale routing in {line:?}");
}

#[test]
fn test_recent_usage_and_completion_work_without_an_older_routing_plan() {
    let sandbox = Sandbox::new();
    sandbox.write_config(LISTED);
    let usage = event(
        "session.usage_record",
        json!({"usage": {
            "model": "actual-solver",
            "fusion": {"fusionId": "current", "pattern": "single", "phaseKind": "primary"}
        }}),
    );
    let line = refresh(&sandbox, &[usage.clone(), usage]);
    assert_shows(&line, "Fusion: single: primary=actual-solver (observed)");
    assert_eq!(line.matches("primary=actual-solver").count(), 1);

    let completed = event(
        "session.fusion_completed",
        json!({
            "fusionId": "current",
            "pattern": "single",
            "outcome": "completed",
            "finalSourceModel": "actual-solver"
        }),
    );
    let line = refresh(&sandbox, &[completed]);
    assert_shows(&line, "Fusion: single: final=actual-solver (completed)");
}

#[test]
fn test_a_non_fusion_step_clears_routing_even_while_auto_is_selected() {
    let sandbox = Sandbox::new();
    sandbox.write_config(LISTED);
    let line = refresh(
        &sandbox,
        &[
            resolved("old", "single"),
            event(
                "session.usage_record",
                json!({"usage": {"model": "direct"}}),
            ),
        ],
    );
    assert!(!line.contains("Fusion:"), "stale routing in {line:?}");
}
