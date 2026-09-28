//! Offline, read-only pilot: cargo run --example evaluate_knowledge -- REPO REPORT.json
use nexq_lib::{db, projects};
use serde_json::{json, Value};
use std::{path::Path, process::Command, time::Instant};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    let root = Path::new(
        args.get(1)
            .ok_or("Pass the selected repository path")?,
    );
    let report = args.get(2).ok_or("Pass an output JSON path")?;
    let fixture = args.get(3).ok_or("Pass a local golden dataset JSON path")?;
    let dataset: Value = serde_json::from_str(&std::fs::read_to_string(fixture)?)?;
    let head = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["rev-parse", "HEAD"])
        .output()?;
    if !head.status.success()
        || String::from_utf8_lossy(&head.stdout).trim() != dataset["revision"].as_str().unwrap()
    {
        return Err(
            "Golden dataset revision differs from selected repository; review fixtures first"
                .into(),
        );
    }
    let dirty = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["status", "--porcelain"])
        .output()?;
    if !dirty.status.success() || !dirty.stdout.is_empty() {
        return Err("Golden evaluation requires a clean worktree at the pinned revision".into());
    }
    let conn = rusqlite::Connection::open_in_memory()?;
    db::migrations::run(&conn)?;
    let project = projects::create(&conn, "Local project evaluation", &root.to_string_lossy())?;
    let coverage =
        projects::knowledge::commit(&conn, &project.id, projects::knowledge::scan(root)?)?;
    let mut results = Vec::new();
    let mut passed = 0;
    let mut total = 0;
    let mut citation_checks = 0;
    for case in dataset["cases"].as_array().unwrap() {
        for source in case["sources"].as_array().unwrap() {
            let content = std::fs::read_to_string(root.join(source["path"].as_str().unwrap()))?;
            if let Some(anchor) = source["anchor"].as_str() {
                assert!(
                    content
                        .lines()
                        .nth(source["startLine"].as_u64().unwrap() as usize - 1)
                        .unwrap()
                        .to_lowercase()
                        .contains(&anchor.to_lowercase()),
                    "Fixture anchor drift: {}",
                    case["id"]
                );
            }
            citation_checks += 1;
        }
        let start = Instant::now();
        let hits = projects::knowledge::hybrid_search(
            &conn,
            &project.id,
            case["question"].as_str().unwrap(),
            "lexical",
            None,
            8,
        )?;
        let elapsed = start.elapsed().as_millis();
        let answerable = case["kind"] == "answerable";
        let found = answerable
            && case["sources"].as_array().unwrap().iter().all(|source| {
                hits.iter().any(|h| {
                    h.evidence.path == source["path"].as_str().unwrap()
                        && h.evidence.start_line <= source["startLine"].as_u64().unwrap() as usize
                        && h.evidence.end_line >= source["startLine"].as_u64().unwrap() as usize
                })
            });
        if answerable {
            total += 1;
            if found {
                passed += 1;
            }
        }
        results.push(json!({"id":case["id"],"question":case["question"],"kind":case["kind"],"evidenceFound":if answerable{Some(found)}else{None},"retrievalMs":elapsed,"sources":hits.iter().map(|h|&h.evidence).collect::<Vec<_>>(),"answerReview":"not_run"}));
    }
    let result = json!({"revision":dataset["revision"],"mode":"lexical_fallback","coverage":coverage,"answerable":total,"evidenceFoundAt8":passed,"recallAt8":passed as f64/total as f64,"fixtureSourceChecks":citation_checks,"limitationsScenarios":10,"answerQuality":"not_run","semanticRetrieval":"not_run","liveAudio":"not_run","results":results});
    std::fs::write(report, serde_json::to_string_pretty(&result)?)?;
    println!("Pinned-revision recall@8: {passed}/{total}; fixture source checks: {citation_checks}. Answer quality, semantic retrieval and live audio were not evaluated.");
    if passed as f64 / (total as f64) < 0.9 {
        return Err("Lexical fallback did not reach the 90% retrieval acceptance threshold".into());
    }
    Ok(())
}
