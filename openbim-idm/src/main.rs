use clap::{Args, Parser, Subcommand};
use openbim_idm::{
    Document, EditBatch, Encoding, SchemaFileStatus, ValidationSeverity, schema_catalog,
    verify_schema_dir,
};
use serde_json::json;
use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

type CliResult<T> = Result<T, Box<dyn std::error::Error>>;

#[derive(Debug, Parser)]
#[command(
    name = "idmxml",
    version,
    about = "Read, write, inspect and schema-edit ISO 29481-3 idmXML",
    after_help = "Exit codes: 0 success; 1 operational error; 2 validation errors or schema hash mismatch.\nPaths accept indexed paths (/idm/uc[0]) or locators (guid:<value>, id:<value>)."
)]
struct Cli {
    /// Print errors as JSON ({"code", "message"}) on stderr.
    #[arg(long, global = true)]
    json_errors: bool,
    #[command(subcommand)]
    command: Command,
}

/// Where edited XML goes: stdout by default, `-o FILE`, or back over the input.
#[derive(Debug, Args)]
struct OutArgs {
    #[arg(short, long)]
    output: Option<PathBuf>,
    /// Overwrite the input file atomically.
    #[arg(short = 'i', long, conflicts_with = "output")]
    in_place: bool,
    /// Output encoding: utf-8 (default), utf-16le, utf-16be, iso-8859-1, windows-1252.
    #[arg(long, default_value = "utf-8")]
    encoding: String,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Show recursive document metadata and conformance diagnostics.
    Inspect {
        input: PathBuf,
        #[arg(long)]
        json: bool,
    },
    /// Validate catalog-derived structure plus ISO semantic overlays.
    Validate {
        input: PathBuf,
        #[arg(long)]
        json: bool,
    },
    /// Reformat XML without dropping unknown elements or mixed content.
    Format {
        input: PathBuf,
        #[command(flatten)]
        out: OutArgs,
    },
    /// Convert the complete lossless XML tree to JSON.
    ToJson {
        input: PathBuf,
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Rebuild XML from lossless tree JSON.
    FromJson {
        input: PathBuf,
        #[arg(short, long)]
        output: Option<PathBuf>,
        #[arg(long)]
        compact: bool,
    },
    /// Read element text by an indexed path or locator.
    Get { input: PathBuf, path: String },
    /// Set element text and write the modified XML.
    Set {
        input: PathBuf,
        path: String,
        value: String,
        #[command(flatten)]
        out: OutArgs,
    },
    /// Set an element attribute and write the modified XML.
    SetAttribute {
        input: PathBuf,
        path: String,
        name: String,
        value: String,
        #[command(flatten)]
        out: OutArgs,
    },
    /// Remove an attribute (declared required attributes are rejected).
    RemoveAttribute {
        input: PathBuf,
        path: String,
        name: String,
        #[command(flatten)]
        out: OutArgs,
    },
    /// Add a schema-declared attribute with its default value.
    AddAttribute {
        input: PathBuf,
        path: String,
        name: String,
        #[command(flatten)]
        out: OutArgs,
    },
    /// Create a semantically complete IDM document.
    New {
        title: String,
        code: String,
        #[command(flatten)]
        out: NewOut,
    },
    /// List context-menu child actions permitted at a schema path.
    Allowed {
        input: PathBuf,
        path: String,
        #[arg(long)]
        json: bool,
    },
    /// Add a cardinality-checked child and its required schema skeleton.
    Add {
        input: PathBuf,
        parent_path: String,
        child: String,
        #[command(flatten)]
        out: OutArgs,
    },
    /// Add a schema child at a position among its same-name siblings.
    Insert {
        input: PathBuf,
        parent_path: String,
        child: String,
        #[arg(long)]
        position: usize,
        #[command(flatten)]
        out: OutArgs,
    },
    /// Remove a child when the content model permits it.
    Remove {
        input: PathBuf,
        path: String,
        #[command(flatten)]
        out: OutArgs,
    },
    /// Duplicate a subtree right after itself with fresh guid/id values.
    Duplicate {
        input: PathBuf,
        path: String,
        #[command(flatten)]
        out: OutArgs,
    },
    /// Move a subtree to another parent or position (its final same-name index).
    Move {
        input: PathBuf,
        path: String,
        new_parent: String,
        #[arg(long)]
        position: Option<usize>,
        #[command(flatten)]
        out: OutArgs,
    },
    /// Find element paths by name, guid or id.
    Find {
        input: PathBuf,
        #[arg(long)]
        name: Option<String>,
        #[arg(long)]
        guid: Option<String>,
        #[arg(long)]
        id: Option<String>,
    },
    /// List the paths of all elements, or of elements with the given name.
    List {
        input: PathBuf,
        name: Option<String>,
    },
    /// Show attributes, text, children and schema handle of one element.
    Node {
        input: PathBuf,
        path: String,
        #[arg(long)]
        json: bool,
    },
    /// Show the catalog rule (attributes, children) that applies at a path.
    Rule { input: PathBuf, path: String },
    /// Print the edits that turn the first document into the second (JSON).
    Diff { before: PathBuf, after: PathBuf },
    /// Apply a JSON edit batch atomically; optionally write the inverse batch.
    Apply {
        input: PathBuf,
        edits: PathBuf,
        /// Write the inverse (undo) batch to this file.
        #[arg(long)]
        undo: Option<PathBuf>,
        #[command(flatten)]
        out: OutArgs,
    },
    /// Emit the generated declaration catalog (no XSD bytes are bundled).
    Schema {
        #[arg(long)]
        json: bool,
        /// Compare a local schema directory with the catalog's source hashes.
        #[arg(long, value_name = "DIR")]
        verify: Option<PathBuf>,
    },
}

/// `new` has no input document, so `--in-place` does not apply.
#[derive(Debug, Args)]
struct NewOut {
    #[arg(short, long)]
    output: Option<PathBuf>,
    #[arg(long, default_value = "utf-8")]
    encoding: String,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let json_errors = cli.json_errors;
    match run(cli) {
        Ok(code) => code,
        Err(error) => {
            if json_errors {
                let value = match error.downcast_ref::<openbim_idm::Error>() {
                    Some(error) => error.to_value(),
                    None => json!({ "code": "error", "message": error.to_string() }),
                };
                eprintln!("{value}");
            } else {
                eprintln!("idmxml: {error}");
            }
            ExitCode::from(1)
        }
    }
}

fn run(cli: Cli) -> CliResult<ExitCode> {
    match cli.command {
        Command::Inspect {
            input,
            json: as_json,
        } => {
            let document = read_document(&input)?;
            let issues = document.validate();
            let summary = json!({
                "root": document.root().local_name(),
                "namespace": document.root().namespace_uri(),
                "use_cases": document.count("uc"),
                "business_context_maps": document.count("businessContextMap"),
                "exchange_requirements": document.count("er"),
                "sub_idms": document.count("subIdm"),
                "errors": issues.iter().filter(|issue| issue.severity == ValidationSeverity::Error).count(),
                "warnings": issues.iter().filter(|issue| issue.severity == ValidationSeverity::Warning).count(),
                "issues": issues,
            });
            if as_json {
                println!("{}", serde_json::to_string_pretty(&summary)?);
            } else {
                for key in [
                    "root",
                    "use_cases",
                    "business_context_maps",
                    "exchange_requirements",
                    "sub_idms",
                    "errors",
                    "warnings",
                ] {
                    println!("{key}: {}", summary[key]);
                }
            }
            Ok(ExitCode::SUCCESS)
        }
        Command::Validate {
            input,
            json: as_json,
        } => {
            let issues = read_document(&input)?.validate();
            if as_json {
                println!("{}", serde_json::to_string_pretty(&issues)?);
            } else if issues.is_empty() {
                println!("valid: no structural or semantic issues");
            } else {
                for issue in &issues {
                    println!(
                        "{:?} {} {}: {}",
                        issue.severity, issue.code, issue.path, issue.message
                    );
                }
            }
            if issues
                .iter()
                .any(|issue| issue.severity == ValidationSeverity::Error)
            {
                Ok(ExitCode::from(2))
            } else {
                Ok(ExitCode::SUCCESS)
            }
        }
        Command::Format { input, out } => {
            emit(&out, &input, &read_document(&input)?)?;
            Ok(ExitCode::SUCCESS)
        }
        Command::ToJson { input, output } => {
            let value = read_document(&input)?.to_value();
            write_text(output.as_deref(), &serde_json::to_string_pretty(&value)?)?;
            Ok(ExitCode::SUCCESS)
        }
        Command::FromJson {
            input,
            output,
            compact,
        } => {
            let xml = Document::from_json_str(&read_text(&input)?)?.to_xml(!compact)?;
            write_text(output.as_deref(), &xml)?;
            Ok(ExitCode::SUCCESS)
        }
        Command::Get { input, path } => {
            println!("{}", read_document(&input)?.text(&path)?);
            Ok(ExitCode::SUCCESS)
        }
        Command::Set {
            input,
            path,
            value,
            out,
        } => edit(&input, &out, |document| {
            document.set_text(&path, &value)?;
            Ok(())
        }),
        Command::SetAttribute {
            input,
            path,
            name,
            value,
            out,
        } => edit(&input, &out, |document| {
            document.set_attribute(&path, &name, &value)?;
            Ok(())
        }),
        Command::RemoveAttribute {
            input,
            path,
            name,
            out,
        } => edit(&input, &out, |document| {
            document.remove_attribute(&path, &name)?;
            Ok(())
        }),
        Command::AddAttribute {
            input,
            path,
            name,
            out,
        } => edit(&input, &out, |document| {
            document.add_schema_attribute(&path, &name)?;
            Ok(())
        }),
        Command::New { title, code, out } => {
            let document = Document::new_idm(&title, &code)?;
            let bytes =
                document.to_bytes_with_encoding(true, Encoding::from_label(&out.encoding)?)?;
            write_bytes(out.output.as_deref(), &bytes)?;
            Ok(ExitCode::SUCCESS)
        }
        Command::Allowed {
            input,
            path,
            json: as_json,
        } => {
            let actions = read_document(&input)?.allowed_children(&path)?;
            if as_json {
                println!("{}", serde_json::to_string_pretty(&actions)?);
            } else {
                for action in actions {
                    let maximum = action
                        .max_occurs
                        .map_or_else(|| "*".to_owned(), |value| value.to_string());
                    println!(
                        "{} [{}, {}] current={} add={}",
                        action.name, action.min_occurs, maximum, action.current, action.can_add
                    );
                }
            }
            Ok(ExitCode::SUCCESS)
        }
        Command::Add {
            input,
            parent_path,
            child,
            out,
        } => edit(&input, &out, |document| {
            document.append_schema_child(&parent_path, &child)?;
            Ok(())
        }),
        Command::Insert {
            input,
            parent_path,
            child,
            position,
            out,
        } => edit(&input, &out, |document| {
            document.insert_schema_child(&parent_path, &child, position)?;
            Ok(())
        }),
        Command::Remove { input, path, out } => edit(&input, &out, |document| {
            document.remove_schema_node(&path)?;
            Ok(())
        }),
        Command::Duplicate { input, path, out } => edit(&input, &out, |document| {
            document.duplicate_schema_node(&path)?;
            Ok(())
        }),
        Command::Move {
            input,
            path,
            new_parent,
            position,
            out,
        } => edit(&input, &out, |document| {
            document.reparent_schema_node(&path, &new_parent, position)?;
            Ok(())
        }),
        Command::Find {
            input,
            name,
            guid,
            id,
        } => {
            let document = read_document(&input)?;
            if name.is_none() && guid.is_none() && id.is_none() {
                return Err("pass at least one of --name, --guid or --id".into());
            }
            // Each filter narrows the result of the previous ones.
            let mut paths: Option<Vec<String>> = None;
            let mut narrow = |found: Vec<String>| {
                paths = Some(match paths.take() {
                    Some(current) => current
                        .into_iter()
                        .filter(|path| found.contains(path))
                        .collect(),
                    None => found,
                });
            };
            if let Some(name) = &name {
                narrow(document.element_paths(name));
            }
            if let Some(guid) = &guid {
                narrow(document.find_by_guid(guid));
            }
            if let Some(id) = &id {
                narrow(document.find_by_id(id));
            }
            let paths = paths.unwrap_or_default();
            for path in &paths {
                println!("{path}");
            }
            Ok(if paths.is_empty() {
                ExitCode::from(1)
            } else {
                ExitCode::SUCCESS
            })
        }
        Command::List { input, name } => {
            let document = read_document(&input)?;
            let paths = match name {
                Some(name) => document.element_paths(&name),
                None => all_paths(&document),
            };
            for path in paths {
                println!("{path}");
            }
            Ok(ExitCode::SUCCESS)
        }
        Command::Node {
            input,
            path,
            json: as_json,
        } => {
            let info = read_document(&input)?.node_info(&path)?;
            if as_json {
                println!("{}", serde_json::to_string_pretty(&info)?);
            } else {
                println!("path: {}", info.path);
                println!("name: {}", info.name);
                println!(
                    "handle: {}",
                    info.handle.as_deref().unwrap_or("(extension)")
                );
                for attribute in &info.attributes {
                    println!("@{} = {:?}", attribute.qualified_name, attribute.value);
                }
                for child in &info.children {
                    println!("child {}", child.path);
                }
            }
            Ok(ExitCode::SUCCESS)
        }
        Command::Rule { input, path } => {
            let document = read_document(&input)?;
            println!(
                "{}",
                serde_json::to_string_pretty(document.schema_rule(&path)?)?
            );
            Ok(ExitCode::SUCCESS)
        }
        Command::Diff { before, after } => {
            let batch = read_document(&before)?.diff(&read_document(&after)?)?;
            println!("{}", serde_json::to_string_pretty(&batch)?);
            Ok(ExitCode::SUCCESS)
        }
        Command::Apply {
            input,
            edits,
            undo,
            out,
        } => {
            let batch = EditBatch::from_json_str(&read_text(&edits)?)?;
            let mut inverse = None;
            let code = edit(&input, &out, |document| {
                inverse = Some(document.apply_batch(&batch)?);
                Ok(())
            })?;
            if let (Some(path), Some(inverse)) = (undo, inverse) {
                fs::write(path, serde_json::to_string_pretty(&inverse)?)?;
            }
            Ok(code)
        }
        Command::Schema {
            json: as_json,
            verify,
        } => {
            if let Some(directory) = verify {
                let report = verify_schema_dir(&directory)?;
                if as_json {
                    println!("{}", serde_json::to_string_pretty(&report)?);
                } else {
                    for check in &report.checks {
                        let label = match check.status {
                            SchemaFileStatus::Match => "match",
                            SchemaFileStatus::Mismatch => "MISMATCH",
                            SchemaFileStatus::Missing => "MISSING",
                        };
                        println!("{label} {}", check.file);
                    }
                }
                return Ok(if report.ok() {
                    ExitCode::SUCCESS
                } else {
                    ExitCode::from(2)
                });
            }
            let catalog = schema_catalog()?;
            if as_json {
                println!("{}", serde_json::to_string_pretty(&catalog)?);
            } else {
                println!("profile: {}", catalog.profile);
                println!("elements: {}", catalog.element_names.len());
                println!("global elements: {}", catalog.global_elements.len());
                println!("attributes: {}", catalog.attribute_names.len());
                println!("enumerations: {}", catalog.enum_values.len());
                println!("context definitions: {}", catalog.elements.len());
            }
            Ok(ExitCode::SUCCESS)
        }
    }
}

fn all_paths(document: &Document) -> Vec<String> {
    let mut paths = vec!["/idm".to_owned()];
    let mut index = 0;
    while index < paths.len() {
        let parent = paths[index].clone();
        if let Ok(info) = document.node_info(&parent) {
            paths.extend(info.children.into_iter().map(|child| child.path));
        }
        index += 1;
    }
    paths
}

/// Read, mutate and emit a document in one parse/write cycle.
fn edit(
    input: &Path,
    out: &OutArgs,
    change: impl FnOnce(&mut Document) -> CliResult<()>,
) -> CliResult<ExitCode> {
    let mut document = read_document(input)?;
    change(&mut document)?;
    emit(out, input, &document)?;
    Ok(ExitCode::SUCCESS)
}

fn emit(out: &OutArgs, input: &Path, document: &Document) -> CliResult<()> {
    let encoding = Encoding::from_label(&out.encoding)?;
    if out.in_place {
        if input == Path::new("-") {
            return Err("--in-place cannot be used with standard input".into());
        }
        document.write_path_with_encoding(input, true, encoding)?;
    } else if let Some(path) = &out.output {
        document.write_path_with_encoding(path, true, encoding)?;
    } else {
        write_bytes(None, &document.to_bytes_with_encoding(true, encoding)?)?;
    }
    Ok(())
}

fn read_document(path: &Path) -> CliResult<Document> {
    let parsed = if path == Path::new("-") {
        Document::from_reader(io::stdin())?
    } else {
        Document::from_path(path)?
    };
    for warning in &parsed.warnings {
        eprintln!("idmxml: note: {warning}");
    }
    Ok(parsed.document)
}

fn read_text(path: &Path) -> io::Result<String> {
    if path == Path::new("-") {
        let mut source = String::new();
        io::stdin().read_to_string(&mut source)?;
        Ok(source)
    } else {
        fs::read_to_string(path)
    }
}

fn write_text(path: Option<&Path>, content: &str) -> io::Result<()> {
    write_bytes(path, content.as_bytes())
}

fn write_bytes(path: Option<&Path>, content: &[u8]) -> io::Result<()> {
    if let Some(path) = path {
        fs::write(path, content)
    } else {
        let mut stdout = io::stdout().lock();
        stdout.write_all(content)?;
        stdout.write_all(b"\n")
    }
}
