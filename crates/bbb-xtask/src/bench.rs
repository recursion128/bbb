//! Workspace Criterion benchmarks and system benchmark runners (`cargo dev bench`).

use std::{
    collections::HashMap,
    env, fs,
    path::{Path, PathBuf},
    process::ExitCode,
    time::SystemTime,
};

use crate::toolchain::{
    cargo_command, maybe_prepared_vanilla_root, prepare_python_command, resolve_cargo_bin,
    resolve_target_dir, status_to_exit_code, XtaskError,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CriterionBenchTarget {
    pub package: &'static str,
    pub name: &'static str,
    pub features: &'static [&'static str],
}

pub const CRITERION_BENCHES: &[CriterionBenchTarget] = &[
    CriterionBenchTarget {
        package: "bbb-client-world",
        name: "entity_projection",
        features: &[],
    },
    CriterionBenchTarget {
        package: "bbb-client-world",
        name: "publication_snapshot",
        features: &[],
    },
    CriterionBenchTarget {
        package: "bbb-client-render",
        name: "frame_extraction_storage",
        features: &[],
    },
    CriterionBenchTarget {
        package: "bbb-client-render",
        name: "item_model_residency",
        features: &[],
    },
    CriterionBenchTarget {
        package: "bbb-client-render",
        name: "terrain_residency",
        features: &[],
    },
];

pub fn print_bench_usage() {
    eprintln!(
        "\
Usage:
  cargo dev bench [--list | --check]
  cargo dev bench [<criterion-bench>] [--save-baseline <name>] [--compare <name> [--threshold <pct>]] [-- <criterion args...>]
  cargo dev bench <server|replay> [args...]
  cargo dev bench [cargo bench args...]

Criterion benches:
  entity_projection         bbb-client-world
  publication_snapshot      bbb-client-world
  frame_extraction_storage  bbb-client-render
  item_model_residency      bbb-client-render
  terrain_residency         bbb-client-render

Baseline options (Criterion):
  --save-baseline <name>    Save Criterion baseline estimates under <name>
  --compare <name>          Compare Criterion results against saved baseline <name>
  --threshold <pct>         Fail if any compared mean regresses by more than <pct>%

System & replay benchmarks:
  server [args...]          scripts/server_bench.py
  replay [args...]          scripts/client_replay.py"
    );
}

/// Type-check only Criterion benches that require extra Cargo features not
/// covered by `--workspace --all-targets`.
pub fn check_feature_gated_benches(
    root: &Path,
    cargo_bin: &Path,
    target_dir: &Path,
    vanilla_root: Option<&Path>,
) -> Result<i32, XtaskError> {
    for bench in CRITERION_BENCHES.iter().filter(|b| !b.features.is_empty()) {
        let joined_features = bench.features.join(",");
        let mut cmd = cargo_command(root, cargo_bin, target_dir);
        if let Some(vr) = vanilla_root {
            cmd.env("BBB_VANILLA_ROOT", vr);
        }
        cmd.env_remove("BBB_GATE_LOCALIZATION_ONLY").args([
            "check",
            "--locked",
            "-p",
            bench.package,
            "--bench",
            bench.name,
            "--features",
            &joined_features,
        ]);
        let status = cmd
            .status()
            .map_err(|err| XtaskError::failed(format!("cargo check benches: {err}")))?;
        let code = status.code().unwrap_or(1);
        if code != 0 {
            return Ok(code);
        }
    }
    Ok(0)
}

/// Type-check all workspace benchmark and example targets, including feature-gated Criterion benches.
pub fn check_workspace_benches(
    root: &Path,
    cargo_bin: &Path,
    target_dir: &Path,
    vanilla_root: Option<&Path>,
) -> Result<i32, XtaskError> {
    let mut cmd = cargo_command(root, cargo_bin, target_dir);
    if let Some(vr) = vanilla_root {
        cmd.env("BBB_VANILLA_ROOT", vr);
    }
    cmd.env_remove("BBB_GATE_LOCALIZATION_ONLY").args([
        "check",
        "--locked",
        "--workspace",
        "--benches",
        "--examples",
    ]);
    let status = cmd
        .status()
        .map_err(|err| XtaskError::failed(format!("cargo check benches: {err}")))?;
    let code = status.code().unwrap_or(1);
    if code != 0 {
        return Ok(code);
    }
    check_feature_gated_benches(root, cargo_bin, target_dir, vanilla_root)
}

fn run_python_bench(root: &Path, script: &str, args: &[String]) -> Result<ExitCode, XtaskError> {
    let cargo_bin = resolve_cargo_bin()?;
    let target_dir = resolve_target_dir()?;
    let mut py_args = vec!["python".to_string(), script.to_string()];
    py_args.extend_from_slice(args);
    let mut cmd = prepare_python_command(root, &py_args)?;
    cmd.env("CARGO", &cargo_bin)
        .env("CARGO_TARGET_DIR", &target_dir);
    if env::var_os("BBB_VANILLA_ROOT").is_none() {
        if let Some(vanilla) = maybe_prepared_vanilla_root() {
            cmd.env("BBB_VANILLA_ROOT", vanilla);
        }
    }
    let status = cmd
        .status()
        .map_err(|err| XtaskError::failed(format!("run {script}: {err}")))?;
    Ok(status_to_exit_code(status))
}

fn run_one_criterion_bench(
    root: &Path,
    cargo_bin: &Path,
    target_dir: &Path,
    bench: CriterionBenchTarget,
    extra_args: &[String],
) -> Result<ExitCode, XtaskError> {
    let mut cmd = cargo_command(root, cargo_bin, target_dir);
    if env::var_os("BBB_VANILLA_ROOT").is_none() {
        if let Some(vanilla) = maybe_prepared_vanilla_root() {
            cmd.env("BBB_VANILLA_ROOT", vanilla);
        }
    }
    cmd.args([
        "bench",
        "--locked",
        "-p",
        bench.package,
        "--bench",
        bench.name,
    ]);
    if !bench.features.is_empty() {
        cmd.arg("--features").arg(bench.features.join(","));
    }
    cmd.args(extra_args);
    let status = cmd
        .status()
        .map_err(|err| XtaskError::failed(format!("cargo bench {}: {err}", bench.name)))?;
    Ok(status_to_exit_code(status))
}

#[derive(Debug, Clone, PartialEq)]
struct ParsedBenchArgs {
    target: Option<String>,
    save_baseline: Option<String>,
    compare: Option<String>,
    threshold_pct: Option<f64>,
    extra_args: Vec<String>,
}

fn validate_baseline_name(name: &str) -> Result<String, XtaskError> {
    if name.is_empty()
        || !name
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.'))
        || name == "."
        || name == ".."
    {
        return Err(XtaskError::usage(format!(
            "invalid baseline name {name:?}: expected non-empty [A-Za-z0-9._-]"
        )));
    }
    Ok(name.to_string())
}

fn parse_threshold_pct(raw: &str) -> Result<f64, XtaskError> {
    let pct: f64 = raw
        .parse()
        .map_err(|_| XtaskError::usage(format!("invalid --threshold value {raw:?}")))?;
    if !pct.is_finite() || pct < 0.0 {
        return Err(XtaskError::usage(format!(
            "--threshold must be a finite non-negative percentage, got {raw:?}"
        )));
    }
    Ok(pct)
}

fn parse_criterion_bench_args(args: &[String]) -> Result<ParsedBenchArgs, XtaskError> {
    let mut parsed = ParsedBenchArgs {
        target: None,
        save_baseline: None,
        compare: None,
        threshold_pct: None,
        extra_args: Vec::new(),
    };
    let mut idx = 0;
    while idx < args.len() {
        let arg = &args[idx];
        if arg == "--" {
            parsed.extra_args.extend_from_slice(&args[idx..]);
            break;
        }
        if let Some(name) = arg.strip_prefix("--save-baseline=") {
            parsed.save_baseline = Some(validate_baseline_name(name)?);
            idx += 1;
            continue;
        }
        if arg == "--save-baseline" {
            let value = args
                .get(idx + 1)
                .ok_or_else(|| XtaskError::usage("--save-baseline requires a name"))?;
            parsed.save_baseline = Some(validate_baseline_name(value)?);
            idx += 2;
            continue;
        }
        if let Some(name) = arg.strip_prefix("--compare=") {
            parsed.compare = Some(validate_baseline_name(name)?);
            idx += 1;
            continue;
        }
        if arg == "--compare" {
            let value = args
                .get(idx + 1)
                .ok_or_else(|| XtaskError::usage("--compare requires a baseline name"))?;
            parsed.compare = Some(validate_baseline_name(value)?);
            idx += 2;
            continue;
        }
        if let Some(raw) = arg.strip_prefix("--threshold=") {
            parsed.threshold_pct = Some(parse_threshold_pct(raw)?);
            idx += 1;
            continue;
        }
        if arg == "--threshold" {
            let value = args
                .get(idx + 1)
                .ok_or_else(|| XtaskError::usage("--threshold requires a percentage"))?;
            parsed.threshold_pct = Some(parse_threshold_pct(value)?);
            idx += 2;
            continue;
        }
        if parsed.target.is_none() && !arg.starts_with('-') {
            parsed.target = Some(arg.clone());
        } else {
            parsed.extra_args.push(arg.clone());
        }
        idx += 1;
    }

    if parsed.save_baseline.is_some() && parsed.compare.is_some() {
        return Err(XtaskError::usage(
            "--save-baseline and --compare cannot be combined",
        ));
    }
    if parsed.threshold_pct.is_some() && parsed.compare.is_none() {
        return Err(XtaskError::usage("--threshold requires --compare <name>"));
    }
    Ok(parsed)
}

fn append_criterion_flags(extra_args: &[String], criterion_flags: &[&str]) -> Vec<String> {
    if criterion_flags.is_empty() {
        return extra_args.to_vec();
    }
    let mut out = extra_args.to_vec();
    if !out.iter().any(|arg| arg == "--") {
        out.push("--".to_string());
    }
    out.extend(criterion_flags.iter().map(|flag| (*flag).to_string()));
    out
}

fn extract_mean_point_estimate(json: &str) -> Option<f64> {
    let mean_idx = json.find("\"mean\"")?;
    let after_key = &json[mean_idx + "\"mean\"".len()..];
    let colon_idx = after_key.find(':')?;
    let after_colon = after_key[colon_idx + 1..].trim_start();
    if !after_colon.starts_with('{') {
        return None;
    }
    let mut depth = 0usize;
    let mut end_idx = None;
    for (offset, ch) in after_colon.char_indices() {
        match ch {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    end_idx = Some(offset + 1);
                    break;
                }
            }
            _ => {}
        }
    }
    let mean_obj = &after_colon[..end_idx?];
    let est_idx = mean_obj.find("\"point_estimate\"")?;
    let after_est = &mean_obj[est_idx + "\"point_estimate\"".len()..];
    let est_colon = after_est.find(':')?;
    let num_str = after_est[est_colon + 1..]
        .trim_start()
        .split(|c: char| c == ',' || c == '}' || c.is_whitespace())
        .next()?;
    let value: f64 = num_str.parse().ok()?;
    value.is_finite().then_some(value)
}

fn collect_estimates_mtimes(criterion_dir: &Path) -> HashMap<PathBuf, SystemTime> {
    fn walk(dir: &Path, out: &mut HashMap<PathBuf, SystemTime>) {
        let Ok(entries) = fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(&path, out);
            } else if path.file_name().is_some_and(|n| n == "estimates.json")
                && path
                    .parent()
                    .and_then(|p| p.file_name())
                    .is_some_and(|n| n == "new")
            {
                if let Ok(meta) = entry.metadata() {
                    if let Ok(modified) = meta.modified() {
                        out.insert(path, modified);
                    }
                }
            }
        }
    }
    let mut out = HashMap::new();
    walk(criterion_dir, &mut out);
    out
}

fn evaluate_criterion_comparisons(
    criterion_dir: &Path,
    baseline_name: &str,
    threshold_pct: Option<f64>,
    before_mtimes: &HashMap<PathBuf, SystemTime>,
) -> Result<Vec<(String, f64, f64, f64)>, XtaskError> {
    let after_mtimes = collect_estimates_mtimes(criterion_dir);
    let mut updated_new: Vec<PathBuf> = after_mtimes
        .into_iter()
        .filter(|(path, mtime)| before_mtimes.get(path) != Some(mtime))
        .map(|(path, _)| path)
        .collect();
    updated_new.sort();

    if updated_new.is_empty() {
        return Err(XtaskError::failed(format!(
            "no updated Criterion results found under {} to compare against {baseline_name:?}",
            criterion_dir.display()
        )));
    }

    let mut rows = Vec::new();
    let mut regressions = Vec::new();
    for new_file in updated_new {
        let bench_dir = new_file
            .parent()
            .and_then(|p| p.parent())
            .ok_or_else(|| XtaskError::failed(format!("unexpected path {}", new_file.display())))?;
        let base_file = bench_dir.join(baseline_name).join("estimates.json");
        if !base_file.is_file() {
            return Err(XtaskError::failed(format!(
                "missing saved Criterion baseline {:?} at {}",
                baseline_name,
                base_file.display()
            )));
        }
        let new_json = fs::read_to_string(&new_file)
            .map_err(|err| XtaskError::failed(format!("read {}: {err}", new_file.display())))?;
        let base_json = fs::read_to_string(&base_file)
            .map_err(|err| XtaskError::failed(format!("read {}: {err}", base_file.display())))?;
        let new_ns = extract_mean_point_estimate(&new_json).ok_or_else(|| {
            XtaskError::failed(format!(
                "missing mean.point_estimate in {}",
                new_file.display()
            ))
        })?;
        let base_ns = extract_mean_point_estimate(&base_json).ok_or_else(|| {
            XtaskError::failed(format!(
                "missing mean.point_estimate in {}",
                base_file.display()
            ))
        })?;
        let delta_pct = if base_ns > 0.0 {
            ((new_ns - base_ns) / base_ns) * 100.0
        } else {
            0.0
        };
        let label = bench_dir
            .strip_prefix(criterion_dir)
            .unwrap_or(bench_dir)
            .to_string_lossy()
            .replace('\\', "/");
        println!(
            "bench compare ({baseline_name}): {label}: base={base_ns:.1}ns new={new_ns:.1}ns delta={delta_pct:+.2}%"
        );
        if let Some(limit) = threshold_pct {
            if delta_pct > limit {
                regressions.push(format!(
                    "{label}: {delta_pct:+.2}% exceeds threshold +{limit:.2}%"
                ));
            }
        }
        rows.push((label, base_ns, new_ns, delta_pct));
    }

    if !regressions.is_empty() {
        return Err(XtaskError::failed(format!(
            "benchmark regression(s) against baseline {baseline_name:?}:\n  {}",
            regressions.join("\n  ")
        )));
    }
    Ok(rows)
}

pub fn run_bench(root: &Path, args: &[String]) -> Result<ExitCode, XtaskError> {
    match args.first().map(String::as_str) {
        Some("-h" | "--help" | "help") => {
            print_bench_usage();
            return Ok(ExitCode::SUCCESS);
        }
        Some("--list") if args.len() == 1 => {
            for bench in CRITERION_BENCHES {
                if bench.features.is_empty() {
                    println!("{}\t{}", bench.name, bench.package);
                } else {
                    println!(
                        "{}\t{}\tfeatures={}",
                        bench.name,
                        bench.package,
                        bench.features.join(",")
                    );
                }
            }
            println!("server\tscripts/server_bench.py");
            println!("replay\tscripts/client_replay.py");
            return Ok(ExitCode::SUCCESS);
        }
        Some("--check") if args.len() == 1 => {
            let cargo_bin = resolve_cargo_bin()?;
            let target_dir = resolve_target_dir()?;
            let vanilla = maybe_prepared_vanilla_root();
            let code = check_workspace_benches(root, &cargo_bin, &target_dir, vanilla.as_deref())?;
            return if code == 0 {
                Ok(ExitCode::SUCCESS)
            } else {
                Err(XtaskError::with_code(u8::try_from(code).unwrap_or(1), ""))
            };
        }
        Some("--check") => {
            return Err(XtaskError::usage(
                "--check is compile-only and takes no extra arguments",
            ));
        }
        Some("server") => {
            return run_python_bench(root, "scripts/server_bench.py", &args[1..]);
        }
        Some("replay") => {
            return run_python_bench(root, "scripts/client_replay.py", &args[1..]);
        }
        _ => {}
    }

    let parsed = parse_criterion_bench_args(args)?;
    let selected_bench = parsed
        .target
        .as_deref()
        .and_then(|name| CRITERION_BENCHES.iter().copied().find(|b| b.name == name));

    if selected_bench.is_some() || parsed.target.is_none() {
        let cargo_bin = resolve_cargo_bin()?;
        let target_dir = resolve_target_dir()?;
        let criterion_dir = target_dir.join("criterion");
        let before_mtimes = if parsed.compare.is_some() {
            collect_estimates_mtimes(&criterion_dir)
        } else {
            HashMap::new()
        };
        let extra = if let Some(ref name) = parsed.save_baseline {
            append_criterion_flags(&parsed.extra_args, &["--save-baseline", name])
        } else if let Some(ref name) = parsed.compare {
            append_criterion_flags(&parsed.extra_args, &["--baseline", name])
        } else {
            parsed.extra_args.clone()
        };

        if let Some(bench) = selected_bench {
            let code = run_one_criterion_bench(root, &cargo_bin, &target_dir, bench, &extra)?;
            if code != ExitCode::SUCCESS {
                return Ok(code);
            }
        } else {
            for &bench in CRITERION_BENCHES {
                let code = run_one_criterion_bench(root, &cargo_bin, &target_dir, bench, &extra)?;
                if code != ExitCode::SUCCESS {
                    return Ok(code);
                }
            }
        }

        if let Some(ref baseline) = parsed.compare {
            evaluate_criterion_comparisons(
                &criterion_dir,
                baseline,
                parsed.threshold_pct,
                &before_mtimes,
            )?;
        }
        return Ok(ExitCode::SUCCESS);
    }

    if parsed.save_baseline.is_some() || parsed.compare.is_some() || parsed.threshold_pct.is_some()
    {
        return Err(XtaskError::usage(format!(
            "unknown benchmark target {:?}",
            parsed.target.unwrap_or_default()
        )));
    }

    let cargo_bin = resolve_cargo_bin()?;
    let target_dir = resolve_target_dir()?;
    let mut cmd = cargo_command(root, &cargo_bin, &target_dir);
    if env::var_os("BBB_VANILLA_ROOT").is_none() {
        if let Some(vanilla) = maybe_prepared_vanilla_root() {
            cmd.env("BBB_VANILLA_ROOT", vanilla);
        }
    }
    cmd.arg("bench");
    if !args
        .iter()
        .take_while(|a| *a != "--")
        .any(|a| a == "--locked")
    {
        cmd.arg("--locked");
    }
    cmd.args(args);
    let status = cmd
        .status()
        .map_err(|err| XtaskError::failed(format!("invoke cargo bench: {err}")))?;
    Ok(status_to_exit_code(status))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_baseline_compare_and_threshold_flags() {
        let parsed = parse_criterion_bench_args(&[
            "terrain_residency".into(),
            "--compare".into(),
            "pre-opt".into(),
            "--threshold".into(),
            "12.5".into(),
            "--".into(),
            "--warm-up-time".into(),
            "1".into(),
        ])
        .unwrap();
        assert_eq!(parsed.target.as_deref(), Some("terrain_residency"));
        assert_eq!(parsed.compare.as_deref(), Some("pre-opt"));
        assert_eq!(parsed.threshold_pct, Some(12.5));
        assert_eq!(parsed.extra_args, vec!["--", "--warm-up-time", "1"]);

        assert!(parse_criterion_bench_args(&["--threshold".into(), "5".into()]).is_err());
        assert!(parse_criterion_bench_args(&[
            "--save-baseline".into(),
            "a".into(),
            "--compare".into(),
            "b".into(),
        ])
        .is_err());
        assert!(parse_criterion_bench_args(&["--save-baseline".into(), "../bad".into()]).is_err());
    }

    #[test]
    fn extracts_mean_point_estimate_and_checks_regression_threshold() {
        let sample = r#"{
            "mean": {
                "confidence_interval": {"lower_bound": 90.0, "upper_bound": 110.0},
                "point_estimate": 100.0,
                "standard_error": 5.0
            },
            "median": {"point_estimate": 99.0}
        }"#;
        assert_eq!(extract_mean_point_estimate(sample), Some(100.0));

        let temp = env::temp_dir().join(format!("bbb-bench-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp);
        let case_dir = temp.join("terrain_cpu").join("dense");
        fs::create_dir_all(case_dir.join("pre-opt")).unwrap();
        fs::create_dir_all(case_dir.join("new")).unwrap();
        fs::write(case_dir.join("pre-opt").join("estimates.json"), sample).unwrap();
        let new_json = r#"{"mean": {"point_estimate": 120.0}}"#;
        fs::write(case_dir.join("new").join("estimates.json"), new_json).unwrap();

        let empty_before = HashMap::new();
        let rows =
            evaluate_criterion_comparisons(&temp, "pre-opt", Some(25.0), &empty_before).unwrap();
        assert_eq!(rows.len(), 1);
        assert!((rows[0].3 - 20.0).abs() < 1e-6);

        let regressed = evaluate_criterion_comparisons(&temp, "pre-opt", Some(10.0), &empty_before);
        assert!(regressed.is_err());
        let _ = fs::remove_dir_all(&temp);
    }
}
