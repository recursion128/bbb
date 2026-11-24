//! The human command surface.
//!
//! Every subcommand here is a projection of the same MCP endpoint an agent
//! talks to: `tools` lists what the server declares, `call` invokes one of them
//! by name with JSON arguments, `read` reads a declared resource. There are
//! deliberately no per-tool subcommands — a second, hand-written copy of the
//! catalog would be derivable data written twice, and it would drift the moment
//! the server grew a tool.

use std::{io::Write, path::PathBuf, time::Duration};

use bbb_mcp::control::OPERATION_URI_PREFIX;
use clap::{Parser, Subcommand};
use serde_json::{Map, Value};

use crate::client::{AdminClient, ClientConfiguration, ClientError};

#[derive(Debug, Parser)]
#[command(
    name = "bbb-admin",
    about = "Administer a bbb dedicated server over its MCP management endpoint",
    version
)]
pub struct Cli {
    /// The management endpoint, `http(s)://host:port/path`.
    #[arg(long, default_value = "http://127.0.0.1:25585/mcp", global = true)]
    pub endpoint: String,

    /// A file holding the bearer secret. A secret is never taken from an
    /// argument: arguments are visible to every process on the machine and are
    /// kept by shell history.
    #[arg(long, global = true, value_name = "PATH")]
    pub token_file: Option<PathBuf>,

    /// A PEM certificate authority to trust for an https endpoint. Repeatable.
    #[arg(long, global = true, value_name = "PATH")]
    pub ca_file: Vec<PathBuf>,

    /// How long to wait for one answer.
    #[arg(long, global = true, default_value_t = 30, value_name = "SECONDS")]
    pub timeout_seconds: u64,

    /// Print the whole JSON-RPC result rather than the part that answers the
    /// question.
    #[arg(long, global = true)]
    pub raw: bool,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// What the endpoint says about itself, and the authority it granted.
    Hello,
    /// The server's status, players and version.
    Status,
    /// Every tool the endpoint declares.
    Tools {
        /// Include each tool's argument schema.
        #[arg(long)]
        schemas: bool,
        /// Only tools whose name contains this.
        #[arg(long, value_name = "TEXT")]
        matching: Option<String>,
    },
    /// Every resource the endpoint declares.
    Resources,
    /// Read one resource.
    Read {
        #[arg(value_name = "URI")]
        uri: String,
    },
    /// Read the record of one submitted operation, including one whose request
    /// timed out.
    Operation {
        #[arg(value_name = "ID")]
        operation_id: u64,
    },
    /// Invoke one tool.
    Call {
        #[arg(value_name = "TOOL")]
        tool: String,
        /// Arguments as `name=value`. A value that parses as JSON is used as
        /// JSON, otherwise it is a string.
        #[arg(value_name = "NAME=VALUE")]
        arguments: Vec<String>,
        /// The whole argument object as JSON. `name=value` pairs are applied on
        /// top of it.
        #[arg(long, value_name = "JSON")]
        arguments_json: Option<String>,
        /// The caller's own operation id. Two calls carrying the same one are
        /// the same operation, so a retry after a lost answer does not apply
        /// twice.
        #[arg(long, value_name = "ID")]
        request_id: Option<String>,
    },
    /// Subscribe to resources and print each change as it happens.
    Watch {
        #[arg(value_name = "URI", required = true)]
        uris: Vec<String>,
        /// Stop after this many notifications rather than running until
        /// interrupted.
        #[arg(long, value_name = "COUNT")]
        events: Option<usize>,
    },
}

/// What the command did, which is what the process exit status reports.
///
/// A refusal is separated from a failure because they call for different
/// answers: a refused request was understood and denied, a failed one never
/// reached the world.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Answered,
    Refused,
}

impl Outcome {
    pub fn exit_code(self) -> i32 {
        match self {
            Self::Answered => 0,
            Self::Refused => 1,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum CliError {
    #[error(transparent)]
    Client(#[from] ClientError),
    #[error("cannot read {0}: {1}")]
    File(String, String),
    #[error("{0} carries no secret")]
    EmptyToken(String),
    #[error("{0} is not name=value")]
    Argument(String),
    #[error("--arguments-json is not a JSON object: {0}")]
    ArgumentsJson(String),
    #[error("cannot write the answer: {0}")]
    Output(String),
}

pub async fn run(cli: Cli, out: &mut impl Write) -> Result<Outcome, CliError> {
    let configuration = ClientConfiguration {
        endpoint: cli.endpoint.clone(),
        token: read_token(cli.token_file.as_ref())?,
        certificate_authorities: read_authorities(&cli.ca_file, &cli.endpoint)?,
        timeout: Duration::from_secs(cli.timeout_seconds.max(1)),
        ..ClientConfiguration::default()
    };
    let mut client = AdminClient::new(configuration)?;
    let hello = client.connect().await?;

    let outcome = dispatch(&mut client, &cli, hello, out).await;
    // The session is ended even when the command failed, so a server does not
    // hold a session for a client that is already gone.
    let _ = client.disconnect().await;
    outcome
}

async fn dispatch(
    client: &mut AdminClient,
    cli: &Cli,
    hello: Value,
    out: &mut impl Write,
) -> Result<Outcome, CliError> {
    match &cli.command {
        Command::Hello => {
            write_json(out, &hello)?;
            Ok(Outcome::Answered)
        }
        Command::Status => {
            let result = client
                .call_tool("server.status", Value::Object(Map::new()), None)
                .await?;
            report_call(out, &result, cli.raw)
        }
        Command::Tools { schemas, matching } => {
            let result = client
                .request("tools/list", Value::Object(Map::new()))
                .await?;
            if cli.raw {
                write_json(out, &result)?;
                return Ok(Outcome::Answered);
            }
            let empty = Vec::new();
            let tools = result
                .get("tools")
                .and_then(Value::as_array)
                .unwrap_or(&empty);
            for tool in tools {
                let name = tool.get("name").and_then(Value::as_str).unwrap_or_default();
                if matching.as_ref().is_some_and(|text| !name.contains(text)) {
                    continue;
                }
                write_tool(out, tool, *schemas)?;
            }
            Ok(Outcome::Answered)
        }
        Command::Resources => {
            let result = client
                .request("resources/list", Value::Object(Map::new()))
                .await?;
            if cli.raw {
                write_json(out, &result)?;
                return Ok(Outcome::Answered);
            }
            let empty = Vec::new();
            for resource in result
                .get("resources")
                .and_then(Value::as_array)
                .unwrap_or(&empty)
            {
                writeln!(
                    out,
                    "{}\n    {}",
                    resource
                        .get("uri")
                        .and_then(Value::as_str)
                        .unwrap_or_default(),
                    resource
                        .get("description")
                        .and_then(Value::as_str)
                        .unwrap_or_default(),
                )
                .map_err(output)?;
            }
            Ok(Outcome::Answered)
        }
        Command::Read { uri } => {
            let result = client.read_resource(uri).await?;
            report_read(out, &result, cli.raw)
        }
        Command::Operation { operation_id } => {
            let result = client
                .read_resource(&format!("{OPERATION_URI_PREFIX}{operation_id}"))
                .await?;
            report_read(out, &result, cli.raw)
        }
        Command::Call {
            tool,
            arguments,
            arguments_json,
            request_id,
        } => {
            let arguments = build_arguments(arguments_json.as_deref(), arguments)?;
            let result = client
                .call_tool(tool, arguments, request_id.as_deref())
                .await?;
            report_call(out, &result, cli.raw)
        }
        Command::Watch { uris, events } => {
            let mut seen = 0usize;
            let limit = events.unwrap_or(usize::MAX);
            let mut failure = None;
            client
                .watch(uris, |event| {
                    let line = serde_json::to_string(&event).unwrap_or_default();
                    if let Err(error) = writeln!(out, "{line}").and_then(|()| out.flush()) {
                        failure = Some(output(error));
                        return false;
                    }
                    seen += 1;
                    seen < limit
                })
                .await?;
            match failure {
                Some(error) => Err(error),
                None => Ok(Outcome::Answered),
            }
        }
    }
}

/// A tool call's answer.
///
/// `isError` is the world refusing, not the transport failing, so its text goes
/// to stderr and the exit status says refused while the process itself is fine.
fn report_call(out: &mut impl Write, result: &Value, raw: bool) -> Result<Outcome, CliError> {
    if raw {
        write_json(out, result)?;
        return Ok(if is_error(result) {
            Outcome::Refused
        } else {
            Outcome::Answered
        });
    }
    if is_error(result) {
        eprintln!("{}", content_text(result));
        return Ok(Outcome::Refused);
    }
    match result.get("structuredContent") {
        Some(structured) => write_json(out, structured)?,
        None => writeln!(out, "{}", content_text(result)).map_err(output)?,
    }
    Ok(Outcome::Answered)
}

/// A resource read's answer. The server already produced the pretty JSON, so
/// printing the text is not a re-rendering of it.
fn report_read(out: &mut impl Write, result: &Value, raw: bool) -> Result<Outcome, CliError> {
    if raw {
        write_json(out, result)?;
        return Ok(Outcome::Answered);
    }
    let text = result
        .get("contents")
        .and_then(Value::as_array)
        .and_then(|contents| contents.first())
        .and_then(|content| content.get("text"))
        .and_then(Value::as_str);
    match text {
        Some(text) => writeln!(out, "{text}").map_err(output)?,
        None => write_json(out, result)?,
    }
    Ok(Outcome::Answered)
}

fn is_error(result: &Value) -> bool {
    result
        .get("isError")
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

fn content_text(result: &Value) -> String {
    result
        .get("content")
        .and_then(Value::as_array)
        .map(|blocks| {
            blocks
                .iter()
                .filter_map(|block| block.get("text").and_then(Value::as_str))
                .collect::<Vec<_>>()
                .join("\n")
        })
        .unwrap_or_default()
}

fn write_tool(out: &mut impl Write, tool: &Value, schemas: bool) -> Result<(), CliError> {
    let name = tool.get("name").and_then(Value::as_str).unwrap_or_default();
    let annotations = tool.get("annotations");
    let read_only = annotations
        .and_then(|annotations| annotations.get("readOnlyHint"))
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let destructive = annotations
        .and_then(|annotations| annotations.get("destructiveHint"))
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let scope = tool
        .get("_meta")
        .and_then(|meta| meta.get("bbb/scope"))
        .and_then(Value::as_str)
        .unwrap_or_default();
    let marks = match (read_only, destructive) {
        (true, _) => "read",
        (false, true) => "write, destructive",
        (false, false) => "write",
    };
    writeln!(
        out,
        "{name}  [{scope}: {marks}]\n    {}",
        tool.get("description")
            .and_then(Value::as_str)
            .unwrap_or_default()
    )
    .map_err(output)?;
    if schemas {
        let schema = tool.get("inputSchema").cloned().unwrap_or(Value::Null);
        writeln!(
            out,
            "    {}",
            serde_json::to_string(&schema).unwrap_or_default()
        )
        .map_err(output)?;
    }
    Ok(())
}

pub(crate) fn build_arguments(json: Option<&str>, pairs: &[String]) -> Result<Value, CliError> {
    let mut arguments = match json {
        None => Map::new(),
        Some(text) => match serde_json::from_str::<Value>(text) {
            Ok(Value::Object(object)) => object,
            Ok(other) => return Err(CliError::ArgumentsJson(other.to_string())),
            Err(error) => return Err(CliError::ArgumentsJson(error.to_string())),
        },
    };
    for pair in pairs {
        let (name, value) = pair
            .split_once('=')
            .ok_or_else(|| CliError::Argument(pair.clone()))?;
        // A bare word is a string, so `players=alice` needs no quoting the
        // shell would eat anyway; anything that parses as JSON is JSON, so
        // `add=[{"name":"alice"}]` works too.
        let value = serde_json::from_str::<Value>(value).unwrap_or_else(|_| Value::from(value));
        arguments.insert(name.to_string(), value);
    }
    Ok(Value::Object(arguments))
}

pub(crate) fn read_token(path: Option<&PathBuf>) -> Result<Option<String>, CliError> {
    let Some(path) = path else { return Ok(None) };
    let document = std::fs::read_to_string(path)
        .map_err(|error| CliError::File(path.display().to_string(), error.to_string()))?;
    let secret = document
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty() && !line.starts_with('#'))
        .unwrap_or_default();
    if secret.is_empty() {
        return Err(CliError::EmptyToken(path.display().to_string()));
    }
    Ok(Some(secret.to_string()))
}

fn read_authorities(paths: &[PathBuf], endpoint: &str) -> Result<Vec<Vec<u8>>, CliError> {
    let env_path = if paths.is_empty() {
        std::env::var_os("SSL_CERT_FILE")
            .filter(|value| !value.is_empty())
            .map(PathBuf::from)
    } else {
        None
    };
    let effective_paths: Vec<PathBuf> = if !paths.is_empty() {
        paths.to_vec()
    } else if let Some(path) = env_path {
        vec![path]
    } else {
        Vec::new()
    };
    if !effective_paths.is_empty() {
        let mut authorities = Vec::new();
        for path in &effective_paths {
            let document = std::fs::read(path)
                .map_err(|error| CliError::File(path.display().to_string(), error.to_string()))?;
            let blocks = bbb_mcp::pem_blocks(&document, "CERTIFICATE")
                .map_err(|error| CliError::File(path.display().to_string(), error.to_string()))?;
            authorities.extend(blocks);
        }
        return Ok(authorities);
    }
    if url::Url::parse(endpoint).is_ok_and(|url| url.scheme() == "https") {
        let loaded = rustls_native_certs::load_native_certs();
        return Ok(loaded
            .certs
            .into_iter()
            .map(|cert| cert.as_ref().to_vec())
            .collect());
    }
    Ok(Vec::new())
}

fn write_json(out: &mut impl Write, value: &Value) -> Result<(), CliError> {
    let text = serde_json::to_string_pretty(value).unwrap_or_else(|_| value.to_string());
    writeln!(out, "{text}").map_err(output)
}

fn output(error: std::io::Error) -> CliError {
    CliError::Output(error.to_string())
}
