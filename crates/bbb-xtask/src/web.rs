//! Threaded WASM client packaging (`build-web`) and local Web stack (`web-up`).

use std::{
    env,
    ffi::OsString,
    fs,
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
    process::{Child, Command, ExitCode, Stdio},
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread,
    time::{Duration, Instant},
};

use crate::toolchain::{
    cargo_command, default_target_root, exe_suffix, find_in_path, path_with_toolchain_bin,
    resolve_cargo_bin, resolve_target_dir, resolve_vanilla_root, status_to_exit_code,
    target_root_override, validate_target_suffix, XtaskError,
};

const WASM_RUSTFLAGS: &str = "-C target-feature=+atomics,+bulk-memory,+mutable-globals,+simd128 \
-C link-arg=-zstack-size=1048576 \
-C link-arg=--shared-memory -C link-arg=--max-memory=2147483648 \
-C link-arg=--import-memory -C link-arg=--export=__wasm_init_tls \
-C link-arg=--export=__tls_size -C link-arg=--export=__tls_align \
-C link-arg=--export=__tls_base --cfg getrandom_backend=\"wasm_js\" \
--cfg web_sys_unstable_apis";

static INTERRUPTED: AtomicBool = AtomicBool::new(false);

fn scoped_web_dir_with(
    explicit: Option<PathBuf>,
    target_name: Option<&str>,
    target_root: PathBuf,
    base_name: &str,
) -> PathBuf {
    if let Some(dir) = explicit.filter(|p| !p.as_os_str().is_empty()) {
        return dir;
    }
    if let Some(name) = target_name
        .map(str::trim)
        .filter(|n| !n.is_empty() && *n != "main" && validate_target_suffix(n).is_ok())
    {
        return target_root.join(format!("{base_name}-{name}"));
    }
    target_root.join(base_name)
}

pub fn default_web_out_dir() -> PathBuf {
    let explicit = env::var_os("BBB_WEB_OUT_DIR").map(PathBuf::from);
    let target_name = env::var("BBB_CARGO_TARGET_NAME").ok();
    scoped_web_dir_with(
        explicit,
        target_name.as_deref(),
        default_target_root(),
        "bbb-web-dist",
    )
}

pub fn default_wasm_target_dir() -> PathBuf {
    let explicit = env::var_os("BBB_WASM_TARGET_DIR").map(PathBuf::from);
    let target_name = env::var("BBB_CARGO_TARGET_NAME").ok();
    scoped_web_dir_with(
        explicit,
        target_name.as_deref(),
        default_target_root(),
        "bbb-target-wasm",
    )
}

pub fn default_web_cache_dir() -> PathBuf {
    let explicit = env::var_os("BBB_WEB_CACHE_DIR").map(PathBuf::from);
    let target_name = env::var("BBB_CARGO_TARGET_NAME").ok();
    scoped_web_dir_with(
        explicit,
        target_name.as_deref(),
        default_target_root(),
        "bbb-web-cache",
    )
}

#[derive(Debug, Clone)]
pub struct BuildWebOptions {
    pub release: bool,
    pub out_dir: PathBuf,
    pub wasm_target_dir: PathBuf,
}

pub fn print_build_web_usage() {
    eprintln!(
        "\
Usage:
  cargo build-web [options]
  cargo dev build-web [options]

Builds the threaded wasm32-unknown-unknown WebGPU client bundle (`index.html` and `pkg/`).

Options:
  --dev                              Build with the dev profile
  --release                          Build with the wasm-release profile (default)
  --profile <dev|release|wasm-release>
                                     Select build profile (default: $BBB_WEB_PROFILE or release)
  --out-dir <dir>                    Bundle output directory (default: $BBB_WEB_OUT_DIR or <target-root>/bbb-web-dist)
  --wasm-target-dir <dir>            Cargo target dir for wasm32 builds (default: $BBB_WASM_TARGET_DIR or <target-root>/bbb-target-wasm)
  -h, --help                         Show this help message"
    );
}

pub fn parse_build_web_args(
    args: &[String],
    print_usage: fn(),
) -> Result<Option<BuildWebOptions>, XtaskError> {
    let env_profile = env::var("BBB_WEB_PROFILE").unwrap_or_else(|_| "release".to_string());
    let mut release = env_profile != "dev";
    let mut out_dir = default_web_out_dir();
    let mut wasm_target_dir = default_wasm_target_dir();

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-h" | "--help" => {
                print_usage();
                return Ok(None);
            }
            "--dev" => release = false,
            "--release" => release = true,
            "--profile" => {
                i += 1;
                let val = args.get(i).ok_or_else(|| {
                    XtaskError::usage("--profile requires `dev`, `release`, or `wasm-release`")
                })?;
                match val.as_str() {
                    "dev" => release = false,
                    "release" | "wasm-release" => release = true,
                    other => {
                        return Err(XtaskError::usage(format!(
                            "invalid --profile {other:?} (expected dev, release, or wasm-release)"
                        )));
                    }
                }
            }
            "--out-dir" => {
                i += 1;
                let val = args
                    .get(i)
                    .ok_or_else(|| XtaskError::usage("--out-dir requires a path"))?;
                out_dir = PathBuf::from(val);
            }
            "--wasm-target-dir" => {
                i += 1;
                let val = args
                    .get(i)
                    .ok_or_else(|| XtaskError::usage("--wasm-target-dir requires a path"))?;
                wasm_target_dir = PathBuf::from(val);
            }
            other => {
                return Err(XtaskError::usage(format!(
                    "unknown build-web flag {other:?} (try `cargo build-web --help`)"
                )));
            }
        }
        i += 1;
    }

    Ok(Some(BuildWebOptions {
        release,
        out_dir,
        wasm_target_dir,
    }))
}

pub fn parse_locked_package_version(cargo_lock: &str, package_name: &str) -> Option<String> {
    let target_name_line = format!("name = \"{package_name}\"");
    let mut in_target_pkg = false;
    for line in cargo_lock.lines() {
        let trimmed = line.trim();
        if trimmed == "[[package]]" {
            in_target_pkg = false;
            continue;
        }
        if trimmed == target_name_line {
            in_target_pkg = true;
            continue;
        }
        if in_target_pkg {
            if let Some(rest) = trimmed.strip_prefix("version = \"") {
                if let Some(ver) = rest.strip_suffix('"') {
                    return Some(ver.to_string());
                }
            }
        }
    }
    None
}

fn wasm_bindgen_matches_version(bin: &Path, expected_ver: &str, path_env: &OsString) -> bool {
    let Ok(output) = Command::new(bin)
        .env("PATH", path_env)
        .arg("--version")
        .output()
    else {
        return false;
    };
    if !output.status.success() {
        return false;
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    stdout.split_whitespace().nth(1) == Some(expected_ver)
}

fn wasm_bindgen_tool_root_with(target_root: Option<PathBuf>, expected_ver: &str) -> PathBuf {
    target_root
        .unwrap_or_else(default_target_root)
        .join(format!("bbb-wasm-bindgen-{expected_ver}"))
}

fn wasm_bindgen_tool_root(expected_ver: &str) -> PathBuf {
    wasm_bindgen_tool_root_with(target_root_override(), expected_ver)
}

fn resolve_or_install_wasm_bindgen(
    root: &Path,
    cargo_bin: &Path,
    expected_ver: &str,
) -> Result<PathBuf, XtaskError> {
    let path_env = path_with_toolchain_bin(cargo_bin);
    if let Some(explicit) = env::var_os("WASM_BINDGEN").filter(|v| !v.is_empty()) {
        let candidate = PathBuf::from(explicit);
        if wasm_bindgen_matches_version(&candidate, expected_ver, &path_env) {
            return Ok(candidate);
        }
    }
    if let Some(on_path) = find_in_path("wasm-bindgen") {
        if wasm_bindgen_matches_version(&on_path, expected_ver, &path_env) {
            return Ok(on_path);
        }
    }

    let tool_root = wasm_bindgen_tool_root(expected_ver);
    let installed_bin = tool_root
        .join("bin")
        .join(format!("wasm-bindgen{}", exe_suffix()));
    if wasm_bindgen_matches_version(&installed_bin, expected_ver, &path_env) {
        return Ok(installed_bin);
    }

    let cli_target_dir = default_target_root().join("bbb-target-wasm-bindgen-cli");
    eprintln!(
        "installing wasm-bindgen-cli {expected_ver} into {}...",
        tool_root.display()
    );
    let status = Command::new(cargo_bin)
        .current_dir(root)
        .env("CARGO", cargo_bin)
        .env("PATH", &path_env)
        .env("CARGO_TARGET_DIR", &cli_target_dir)
        .args([
            "install",
            "wasm-bindgen-cli",
            "--version",
            expected_ver,
            "--root",
        ])
        .arg(&tool_root)
        .arg("--locked")
        .status()
        .map_err(|err| XtaskError::failed(format!("install wasm-bindgen-cli: {err}")))?;
    if !status.success() {
        return Err(XtaskError::failed(format!(
            "cargo install wasm-bindgen-cli {expected_ver} exited with {status}"
        )));
    }
    if !installed_bin.is_file() {
        return Err(XtaskError::failed(format!(
            "wasm-bindgen binary missing after install: {}",
            installed_bin.display()
        )));
    }
    Ok(installed_bin)
}

pub fn check_web(root: &Path) -> Result<i32, XtaskError> {
    let cargo_bin = resolve_cargo_bin()?;
    let path_env = path_with_toolchain_bin(&cargo_bin);
    let wasm_target_dir = default_wasm_target_dir();

    let status = Command::new(&cargo_bin)
        .current_dir(root)
        .env("CARGO", &cargo_bin)
        .env("PATH", &path_env)
        .env("CARGO_TARGET_DIR", &wasm_target_dir)
        .env(
            "CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUSTFLAGS",
            WASM_RUSTFLAGS,
        )
        .arg("check")
        .arg("--manifest-path")
        .arg(root.join("Cargo.toml"))
        .args([
            "--locked",
            "-p",
            "bbb-client",
            "--lib",
            "--target",
            "wasm32-unknown-unknown",
            "-Z",
            "build-std=panic_abort,std",
        ])
        .status()
        .map_err(|err| XtaskError::failed(format!("check bbb-client (wasm32): {err}")))?;
    Ok(status.code().unwrap_or(1))
}

pub fn build_web(root: &Path, opts: &BuildWebOptions) -> Result<(), XtaskError> {
    let cargo_bin = resolve_cargo_bin()?;
    let path_env = path_with_toolchain_bin(&cargo_bin);

    let mut cmd = Command::new(&cargo_bin);
    cmd.current_dir(root)
        .env("CARGO", &cargo_bin)
        .env("PATH", &path_env)
        .env("CARGO_TARGET_DIR", &opts.wasm_target_dir)
        .env(
            "CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUSTFLAGS",
            WASM_RUSTFLAGS,
        )
        .arg("build")
        .arg("--manifest-path")
        .arg(root.join("Cargo.toml"))
        .args([
            "--locked",
            "-p",
            "bbb-client",
            "--lib",
            "--target",
            "wasm32-unknown-unknown",
        ]);
    if opts.release {
        cmd.args(["--profile", "wasm-release"]);
    }
    cmd.args(["-Z", "build-std=panic_abort,std"]);

    let status = cmd
        .status()
        .map_err(|err| XtaskError::failed(format!("build bbb-client (wasm32): {err}")))?;
    if !status.success() {
        return Err(XtaskError::failed(format!(
            "WASM build failed ({status}); ensure `rustup target add wasm32-unknown-unknown` \
             and `rustup component add rust-src` are installed for the pinned toolchain"
        )));
    }

    let profile_dir = if opts.release {
        "wasm-release"
    } else {
        "debug"
    };
    let wasm_file = opts
        .wasm_target_dir
        .join("wasm32-unknown-unknown")
        .join(profile_dir)
        .join("bbb_client.wasm");
    if !wasm_file.is_file() {
        return Err(XtaskError::failed(format!(
            "expected WASM output not found: {}",
            wasm_file.display()
        )));
    }

    let lock_text = fs::read_to_string(root.join("Cargo.lock"))
        .map_err(|err| XtaskError::failed(format!("read Cargo.lock: {err}")))?;
    let wb_ver = parse_locked_package_version(&lock_text, "wasm-bindgen").ok_or_else(|| {
        XtaskError::failed("failed to resolve `wasm-bindgen` version from Cargo.lock")
    })?;
    let wb_bin = resolve_or_install_wasm_bindgen(root, &cargo_bin, &wb_ver)?;

    let pkg_dir = opts.out_dir.join("pkg");
    fs::create_dir_all(&pkg_dir)
        .map_err(|err| XtaskError::failed(format!("create {}: {err}", pkg_dir.display())))?;

    let mut wb_cmd = Command::new(&wb_bin);
    wb_cmd
        .env("PATH", &path_env)
        .arg(&wasm_file)
        .arg("--out-dir")
        .arg(&pkg_dir)
        .args([
            "--out-name",
            "bbb_client",
            "--target",
            "web",
            "--no-typescript",
        ]);
    if opts.release {
        wb_cmd.args(["--remove-name-section", "--remove-producers-section"]);
    }
    let wb_status = wb_cmd
        .status()
        .map_err(|err| XtaskError::failed(format!("run {}: {err}", wb_bin.display())))?;
    if !wb_status.success() {
        return Err(XtaskError::failed(format!(
            "wasm-bindgen exited with {wb_status}"
        )));
    }

    let bg_wasm = pkg_dir.join("bbb_client_bg.wasm");
    if opts.release && bg_wasm.is_file() {
        let wasm_opt = env::var_os("WASM_OPT")
            .filter(|v| !v.is_empty())
            .map(PathBuf::from)
            .filter(|p| p.is_file())
            .or_else(|| find_in_path("wasm-opt"));
        if let Some(wasm_opt_bin) = wasm_opt {
            let opt_tmp = pkg_dir.join("bbb_client_bg.wasm.opt.tmp");
            let opt_status = Command::new(&wasm_opt_bin)
                .env("PATH", &path_env)
                .args([
                    "-O3",
                    "--enable-simd",
                    "--enable-threads",
                    "--enable-bulk-memory",
                    "--enable-mutable-globals",
                ])
                .arg(&bg_wasm)
                .arg("-o")
                .arg(&opt_tmp)
                .status()
                .map_err(|err| {
                    XtaskError::failed(format!("run {}: {err}", wasm_opt_bin.display()))
                })?;
            if !opt_status.success() {
                let _ = fs::remove_file(&opt_tmp);
                return Err(XtaskError::failed(format!(
                    "wasm-opt exited with {opt_status}"
                )));
            }
            fs::rename(&opt_tmp, &bg_wasm).map_err(|err| {
                let _ = fs::remove_file(&opt_tmp);
                XtaskError::failed(format!("install optimized {}: {err}", bg_wasm.display()))
            })?;
        }
    }

    let index_src = root.join("crates/bbb-web-host/web/index.html");
    if index_src.is_file() {
        fs::copy(&index_src, opts.out_dir.join("index.html"))
            .map_err(|err| XtaskError::failed(format!("copy index.html: {err}")))?;
    }

    println!("web build ready: {}", opts.out_dir.display());
    Ok(())
}

#[derive(Debug, Clone)]
pub struct WebUpOptions {
    pub bind: String,
    pub server_bind: String,
    pub vanilla_root: PathBuf,
    pub wasm_dir: PathBuf,
    pub cache_dir: PathBuf,
    pub world: Option<PathBuf>,
    pub gamemode: Option<String>,
    pub difficulty: Option<String>,
    pub view_distance: Option<String>,
    pub admin_dir: Option<PathBuf>,
    pub username: String,
    pub locale: String,
    pub control: Option<String>,
    pub mcp_bind: Option<String>,
    pub allow_pack_loopback: bool,
    pub rebuild_web: bool,
    pub release: bool,
}

pub fn print_web_up_usage() {
    eprintln!(
        "\
Usage:
  cargo web-up [options]
  cargo dev web-up [options]

Starts a local `bbb-server` together with `bbb-web-host` (building the WASM bundle on first run)
and terminates both cleanly on Ctrl+C.

Options:
  --bind <addr>               HTTP/WebSocket bind address for bbb-web-host (default: 127.0.0.1:8080)
  --server-bind <addr>        TCP bind address for bbb-server (default: 127.0.0.1:25565)
  --vanilla-root <dir>        Prepared Vanilla 26.1 root (default: $BBB_VANILLA_ROOT or platform cache)
  --wasm-dir <dir>            WASM bundle directory (default: $BBB_WEB_OUT_DIR or <target-root>/bbb-web-dist)
  --cache-dir <dir>           Asset cache directory for core-26.1.zip (default: $BBB_WEB_CACHE_DIR or <target-root>/bbb-web-cache)
  --world <dir>               World save directory passed to bbb-server (omit for ephemeral flat world)
  --gamemode <mode>           Forced game mode passed to bbb-server (e.g. survival, creative)
  --difficulty <level>        Difficulty passed to bbb-server (peaceful, easy, normal, hard)
  --view-distance <chunks>    View distance in chunks (2..=32) passed to bbb-server
  --admin-dir <dir>           Admin lists & server-settings.json directory passed to bbb-server
  --username <name>           Default offline player username for bbb-web-host
  --locale <code>             Default client locale for bbb-web-host (default: zh_cn)
  --control <addr>            Loopback address exposing reverse line-JSON control API on bbb-web-host
  --mcp-bind <addr>           Loopback address exposing reverse Streamable HTTP MCP API on bbb-web-host
  --allow-pack-loopback       Allow /proxy/pack to fetch resource packs from loopback addresses
  --rebuild-web               Force rebuilding the WASM bundle even if already present
  --release                   Build and run bbb-server and bbb-web-host with --release
  -h, --help                  Show this help message"
    );
}

pub fn parse_web_up_args(
    args: &[String],
    print_usage: fn(),
) -> Result<Option<WebUpOptions>, XtaskError> {
    let mut bind = "127.0.0.1:8080".to_string();
    let mut server_bind = "127.0.0.1:25565".to_string();
    let mut vanilla_root: Option<PathBuf> = None;
    let mut wasm_dir = default_web_out_dir();
    let mut cache_dir = default_web_cache_dir();
    let mut world = None;
    let mut gamemode = None;
    let mut difficulty = None;
    let mut view_distance = None;
    let mut admin_dir = None;
    let mut username = String::new();
    let mut locale = "zh_cn".to_string();
    let mut control = None;
    let mut mcp_bind = None;
    let mut allow_pack_loopback = false;
    let mut rebuild_web = false;
    let mut release = false;

    let mut i = 0;
    while i < args.len() {
        let take_next = |idx: &mut usize, flag: &str| -> Result<String, XtaskError> {
            *idx += 1;
            args.get(*idx)
                .cloned()
                .ok_or_else(|| XtaskError::usage(format!("{flag} requires a value")))
        };
        match args[i].as_str() {
            "-h" | "--help" => {
                print_usage();
                return Ok(None);
            }
            "--bind" => bind = take_next(&mut i, "--bind")?,
            "--server-bind" => server_bind = take_next(&mut i, "--server-bind")?,
            "--vanilla-root" => {
                vanilla_root = Some(PathBuf::from(take_next(&mut i, "--vanilla-root")?))
            }
            "--wasm-dir" => wasm_dir = PathBuf::from(take_next(&mut i, "--wasm-dir")?),
            "--cache-dir" => cache_dir = PathBuf::from(take_next(&mut i, "--cache-dir")?),
            "--world" => world = Some(PathBuf::from(take_next(&mut i, "--world")?)),
            "--gamemode" => gamemode = Some(take_next(&mut i, "--gamemode")?),
            "--difficulty" => difficulty = Some(take_next(&mut i, "--difficulty")?),
            "--view-distance" => view_distance = Some(take_next(&mut i, "--view-distance")?),
            "--admin-dir" => admin_dir = Some(PathBuf::from(take_next(&mut i, "--admin-dir")?)),
            "--username" => username = take_next(&mut i, "--username")?,
            "--locale" => locale = take_next(&mut i, "--locale")?,
            "--control" => control = Some(take_next(&mut i, "--control")?),
            "--mcp-bind" => mcp_bind = Some(take_next(&mut i, "--mcp-bind")?),
            "--allow-pack-loopback" => allow_pack_loopback = true,
            "--rebuild-web" => rebuild_web = true,
            "--release" => release = true,
            other => {
                return Err(XtaskError::usage(format!(
                    "unknown web-up flag {other:?} (try `cargo web-up --help`)"
                )));
            }
        }
        i += 1;
    }

    let resolved_vanilla = resolve_vanilla_root(vanilla_root.as_deref());
    Ok(Some(WebUpOptions {
        bind,
        server_bind,
        vanilla_root: resolved_vanilla,
        wasm_dir,
        cache_dir,
        world,
        gamemode,
        difficulty,
        view_distance,
        admin_dir,
        username,
        locale,
        control,
        mcp_bind,
        allow_pack_loopback,
        rebuild_web,
        release,
    }))
}

#[cfg(windows)]
fn install_interrupt_flag() {
    type HandlerRoutine = unsafe extern "system" fn(u32) -> i32;
    extern "system" {
        fn SetConsoleCtrlHandler(handler: Option<HandlerRoutine>, add: i32) -> i32;
    }
    unsafe extern "system" fn on_ctrl(_ctrl_type: u32) -> i32 {
        INTERRUPTED.store(true, Ordering::SeqCst);
        1
    }
    unsafe {
        let _ = SetConsoleCtrlHandler(Some(on_ctrl), 1);
    }
}

#[cfg(unix)]
fn install_interrupt_flag() {
    extern "C" {
        fn signal(sig: i32, handler: extern "C" fn(i32)) -> usize;
    }
    extern "C" fn on_sig(_sig: i32) {
        INTERRUPTED.store(true, Ordering::SeqCst);
    }
    const SIGINT: i32 = 2;
    const SIGTERM: i32 = 15;
    unsafe {
        let _ = signal(SIGINT, on_sig);
        let _ = signal(SIGTERM, on_sig);
    }
}

#[cfg(not(any(windows, unix)))]
fn install_interrupt_flag() {}

#[cfg(unix)]
fn request_graceful_stop(child: &Child) {
    extern "C" {
        fn kill(pid: i32, sig: i32) -> i32;
    }
    const SIGTERM: i32 = 15;
    if let Ok(pid) = i32::try_from(child.id()) {
        if pid > 0 {
            unsafe {
                let _ = kill(pid, SIGTERM);
            }
        }
    }
}

#[cfg(not(unix))]
fn request_graceful_stop(_child: &Child) {}

fn terminate_and_wait(child: &mut Child, grace: Duration) {
    if let Ok(Some(_)) = child.try_wait() {
        return;
    }
    request_graceful_stop(child);
    // On Unix we just delivered SIGTERM; on Windows, wait for `grace` only when
    // the console delivered Ctrl+C so non-interrupt teardown does not stall.
    let wait_grace = if cfg!(unix) || INTERRUPTED.load(Ordering::Relaxed) {
        grace
    } else {
        Duration::ZERO
    };
    let deadline = Instant::now() + wait_grace;
    while Instant::now() < deadline {
        if let Ok(Some(_)) = child.try_wait() {
            return;
        }
        thread::sleep(Duration::from_millis(50));
    }
    let _ = child.kill();
    let _ = child.wait();
}

fn pipe_lines<R: std::io::Read + Send + 'static>(
    reader: R,
    prefix: &'static str,
    ready_needle: Option<&'static str>,
    ready_tx: Option<mpsc::Sender<()>>,
) {
    thread::spawn(move || {
        let buf = BufReader::new(reader);
        let mut notified = false;
        for line in buf.lines().map_while(Result::ok) {
            if !notified {
                if let (Some(needle), Some(tx)) = (ready_needle, ready_tx.as_ref()) {
                    if line.contains(needle) {
                        let _ = tx.send(());
                        notified = true;
                    }
                }
            }
            eprintln!("[{prefix}] {line}");
        }
    });
}

pub fn run_web_up(root: &Path, opts: &WebUpOptions) -> Result<ExitCode, XtaskError> {
    let has_vanilla = opts
        .vanilla_root
        .join("sources/26.1/version.json")
        .is_file()
        || opts
            .vanilla_root
            .join("downloads/26.1/client-26.1.jar")
            .is_file()
        || opts
            .vanilla_root
            .join("downloads/26.1/client.jar")
            .is_file()
        || opts.vanilla_root.join("downloads/26.1/26.1.jar").is_file()
        || opts.vanilla_root.join("versions/26.1/26.1.jar").is_file();
    if !has_vanilla {
        return Err(XtaskError::usage(format!(
            "Vanilla 26.1 assets not found at {} (expected `sources/26.1`, `downloads/26.1/client-26.1.jar`, or `.minecraft/versions/26.1/26.1.jar`).\n\
             Pass `--vanilla-root <dir>`, set `BBB_VANILLA_ROOT`, or prepare it with `cargo dev vanilla prepare 26.1`.",
            opts.vanilla_root.display()
        )));
    }

    let has_wasm_bundle = opts.wasm_dir.join("pkg/bbb_client_bg.wasm").is_file()
        && opts.wasm_dir.join("pkg/bbb_client.js").is_file();
    if opts.rebuild_web || !has_wasm_bundle {
        build_web(
            root,
            &BuildWebOptions {
                release: true,
                out_dir: opts.wasm_dir.clone(),
                wasm_target_dir: default_wasm_target_dir(),
            },
        )?;
    }

    let cargo_bin = resolve_cargo_bin()?;
    let target_dir = resolve_target_dir()?;
    let mut build_cmd = cargo_command(root, &cargo_bin, &target_dir);
    build_cmd.args([
        "build",
        "--locked",
        "-p",
        "bbb-server",
        "-p",
        "bbb-web-host",
    ]);
    if opts.release {
        build_cmd.arg("--release");
    }
    let build_status = build_cmd
        .status()
        .map_err(|err| XtaskError::failed(format!("build bbb-server and bbb-web-host: {err}")))?;
    if !build_status.success() {
        return Ok(status_to_exit_code(build_status));
    }

    let profile_dir = if opts.release { "release" } else { "debug" };
    let bin_dir = target_dir.join(profile_dir);
    let server_bin = bin_dir.join(format!("bbb-server{}", exe_suffix()));
    let host_bin = bin_dir.join(format!("bbb-web-host{}", exe_suffix()));

    install_interrupt_flag();

    let mut srv_cmd = Command::new(&server_bin);
    srv_cmd
        .args(["--bind", &opts.server_bind, "--no-mcp"])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(world) = &opts.world {
        srv_cmd.arg("--world").arg(world);
    }
    if let Some(gm) = &opts.gamemode {
        srv_cmd.args(["--gamemode", gm, "--force-gamemode"]);
    }
    if let Some(diff) = &opts.difficulty {
        srv_cmd.args(["--difficulty", diff]);
    }
    if let Some(vd) = &opts.view_distance {
        srv_cmd.args(["--view-distance", vd]);
    }
    if let Some(admin) = &opts.admin_dir {
        srv_cmd.arg("--admin-dir").arg(admin);
    }

    let mut srv_child = srv_cmd
        .spawn()
        .map_err(|err| XtaskError::failed(format!("spawn {}: {err}", server_bin.display())))?;
    let (ready_tx, ready_rx) = mpsc::channel::<()>();
    if let Some(out) = srv_child.stdout.take() {
        pipe_lines(
            out,
            "bbb-server",
            Some("\"server.ready\""),
            Some(ready_tx.clone()),
        );
    }
    if let Some(err) = srv_child.stderr.take() {
        pipe_lines(err, "bbb-server", Some("\"server.ready\""), Some(ready_tx));
    }

    let ready_deadline = Instant::now() + Duration::from_secs(90);
    loop {
        if INTERRUPTED.load(Ordering::Relaxed) {
            terminate_and_wait(&mut srv_child, Duration::from_secs(10));
            return Ok(ExitCode::SUCCESS);
        }
        if ready_rx.recv_timeout(Duration::from_millis(100)).is_ok() {
            break;
        }
        if let Some(status) = srv_child
            .try_wait()
            .map_err(|err| XtaskError::failed(format!("poll bbb-server: {err}")))?
        {
            return Err(XtaskError::failed(format!(
                "bbb-server exited before ready ({status})"
            )));
        }
        if Instant::now() >= ready_deadline {
            terminate_and_wait(&mut srv_child, Duration::from_secs(2));
            return Err(XtaskError::failed(
                "timed out waiting for bbb-server `server.ready`",
            ));
        }
    }

    let mut host_cmd = Command::new(&host_bin);
    host_cmd
        .args(["--bind", &opts.bind, "--server", &opts.server_bind])
        .arg("--vanilla-root")
        .arg(&opts.vanilla_root)
        .arg("--wasm-dir")
        .arg(&opts.wasm_dir)
        .arg("--cache-dir")
        .arg(&opts.cache_dir)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if !opts.username.is_empty() {
        host_cmd.args(["--username", &opts.username]);
    }
    if let Some(ctrl) = &opts.control {
        host_cmd.args(["--control", ctrl]);
    }
    if !opts.locale.trim().is_empty() {
        host_cmd.args(["--locale", &opts.locale]);
    }
    if let Some(mcp) = &opts.mcp_bind {
        host_cmd.args(["--mcp-bind", mcp]);
    }
    if opts.allow_pack_loopback {
        host_cmd.arg("--allow-pack-loopback");
    }

    let mut host_child = match host_cmd.spawn() {
        Ok(child) => child,
        Err(err) => {
            terminate_and_wait(&mut srv_child, Duration::from_secs(5));
            return Err(XtaskError::failed(format!(
                "spawn {}: {err}",
                host_bin.display()
            )));
        }
    };
    if let Some(out) = host_child.stdout.take() {
        pipe_lines(out, "bbb-web-host", None, None);
    }
    if let Some(err) = host_child.stderr.take() {
        pipe_lines(err, "bbb-web-host", None, None);
    }

    eprintln!(
        "\n==> BBB Web stack ready (press Ctrl+C to stop both processes)\n    \
         Open in browser: http://{}/?locale={}&render_distance=8\n",
        opts.bind, opts.locale
    );

    loop {
        if INTERRUPTED.load(Ordering::Relaxed) {
            eprintln!("stopping bbb-web-host and bbb-server...");
            terminate_and_wait(&mut host_child, Duration::from_secs(3));
            terminate_and_wait(&mut srv_child, Duration::from_secs(10));
            return Ok(ExitCode::SUCCESS);
        }
        if let Some(status) = host_child
            .try_wait()
            .map_err(|err| XtaskError::failed(format!("poll bbb-web-host: {err}")))?
        {
            terminate_and_wait(&mut srv_child, Duration::from_secs(10));
            return Err(XtaskError::failed(format!(
                "bbb-web-host exited ({status})"
            )));
        }
        if let Some(status) = srv_child
            .try_wait()
            .map_err(|err| XtaskError::failed(format!("poll bbb-server: {err}")))?
        {
            terminate_and_wait(&mut host_child, Duration::from_secs(3));
            return Err(XtaskError::failed(format!("bbb-server exited ({status})")));
        }
        thread::sleep(Duration::from_millis(100));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wasm_bindgen_tool_root_honors_target_root_override() {
        let custom = PathBuf::from("/custom-build-root");
        assert_eq!(
            wasm_bindgen_tool_root_with(Some(custom.clone()), "0.2.127"),
            custom.join("bbb-wasm-bindgen-0.2.127")
        );
        assert_eq!(
            wasm_bindgen_tool_root_with(None, "0.2.127"),
            default_target_root().join("bbb-wasm-bindgen-0.2.127")
        );
    }

    #[test]
    fn scoped_web_dirs_isolate_non_main_target_names_and_respect_explicit_overrides() {
        let root = PathBuf::from("/build-root");
        assert_eq!(
            scoped_web_dir_with(None, None, root.clone(), "bbb-target-wasm"),
            root.join("bbb-target-wasm")
        );
        assert_eq!(
            scoped_web_dir_with(None, Some("main"), root.clone(), "bbb-target-wasm"),
            root.join("bbb-target-wasm")
        );
        assert_eq!(
            scoped_web_dir_with(None, Some("swarm-web"), root.clone(), "bbb-target-wasm"),
            root.join("bbb-target-wasm-swarm-web")
        );
        assert_eq!(
            scoped_web_dir_with(None, Some("swarm-web"), root.clone(), "bbb-web-dist"),
            root.join("bbb-web-dist-swarm-web")
        );
        assert_eq!(
            scoped_web_dir_with(
                Some(PathBuf::from("/override/wasm")),
                Some("swarm-web"),
                root.clone(),
                "bbb-target-wasm"
            ),
            PathBuf::from("/override/wasm")
        );
        assert_eq!(
            scoped_web_dir_with(None, Some("../bad"), root.clone(), "bbb-target-wasm"),
            root.join("bbb-target-wasm")
        );
    }
}
