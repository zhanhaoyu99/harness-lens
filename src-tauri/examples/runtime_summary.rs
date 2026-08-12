use std::{env, path::PathBuf};

use harness_lens_lib::runtime;

fn main() {
    let workspace = env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| env::current_dir().expect("current directory"));
    let workspace = workspace.canonicalize().expect("workspace path must exist");
    let snapshot = runtime::inspect_workspace(&workspace);
    println!(
        "state={:?} codex={} skills={} hooks={} runs={} message={}",
        snapshot.state,
        snapshot.codex_version.as_deref().unwrap_or("unknown"),
        snapshot.skills.len(),
        snapshot.hooks.len(),
        snapshot.runs.len(),
        snapshot.message.as_deref().unwrap_or("none")
    );

    let arguments = env::args_os().collect::<Vec<_>>();
    if arguments.iter().any(|argument| argument == "--load-first") {
        match snapshot.runs.first() {
            Some(run) => match runtime::load_run(&run.id) {
                Ok(detail) => println!(
                    "run={} turns={} steps={} failed_turns={} truncated={}",
                    detail.id,
                    detail.turns.len(),
                    detail.steps.len(),
                    detail.failed_turns,
                    detail.truncated
                ),
                Err(error) => eprintln!("run_error={error}"),
            },
            None => println!("run=none"),
        }
    }

    if arguments
        .iter()
        .any(|argument| argument == "--verify-read-only")
    {
        let run = snapshot.runs.last().expect("a stable run to verify");
        let before = run.updated_at.clone();
        runtime::load_run(&run.id).expect("read a stable run");
        let after_snapshot = runtime::inspect_workspace(&workspace);
        let after = after_snapshot
            .runs
            .iter()
            .find(|candidate| candidate.id == run.id)
            .and_then(|candidate| candidate.updated_at.clone());
        assert_eq!(before, after, "thread/read must not mutate the run");
        println!("read_only_verified={} updated_at={:?}", run.id, before);
    }
}
