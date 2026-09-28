//! Read-only lexical candidate audit against an existing indexed project.
use nexq_lib::projects::knowledge;
use serde_json::{json, Value};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 5 { return Err("Pass DATABASE PROJECT_ID FIXTURE REPORT".into()); }
    let conn = rusqlite::Connection::open_with_flags(&args[1], rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    let fixture: Value = serde_json::from_str(&std::fs::read_to_string(&args[3])?)?;
    let mut rows = Vec::new();
    for case in fixture["cases"].as_array().ok_or("Missing cases")? {
        let hits = knowledge::search(&conn, &args[2], case["question"].as_str().ok_or("Missing question")?, 40)?;
        let expected: Vec<_> = case["sources"].as_array().ok_or("Missing sources")?.iter().map(|source| {
            let rank = hits.iter().position(|h| h.evidence.path == source["path"].as_str().unwrap_or("") && h.evidence.start_line <= source["startLine"].as_u64().unwrap_or(0) as usize && h.evidence.end_line >= source["startLine"].as_u64().unwrap_or(0) as usize).map(|r| r+1);
            json!({"source":source,"lexicalRank":rank})
        }).collect();
        rows.push(json!({"id":case["id"],"expected":expected,"hits":hits}));
    }
    std::fs::write(&args[4], serde_json::to_string_pretty(&rows)?)?;
    println!("Saved {} lexical candidate audits. No answer quality claims.", rows.len());
    Ok(())
}
