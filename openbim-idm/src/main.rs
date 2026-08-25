use clap::{Parser, Subcommand};
use openbim_idm::{Document, ValidationSeverity, schema_catalog};
use serde_json::json;
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

#[derive(Debug, Parser)]
#[command(
    name = "idmxml",
    version,
    about = "Read, write, inspect and schema-edit ISO 29481-3 idmXML"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
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
        #[arg(short, long)]
        output: Option<PathBuf>,
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
    /// Read element text by an indexed path.
    Get { input: PathBuf, path: String },
    /// Set element text and write the modified XML.
    Set {
        input: PathBuf,
        path: String,
        value: String,
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Set an element attribute and write the modified XML.
    SetAttribute {
        input: PathBuf,
        path: String,
        name: String,
        value: String,
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Create a semantically complete IDM document.
    New {
        title: String,
        code: String,
        #[arg(short, long)]
        output: Option<PathBuf>,
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
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Remove a child when the content model permits it.
    Remove {
        input: PathBuf,
        path: String,
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Emit the generated declaration catalog (no XSD bytes are bundled).
    Schema {
        #[arg(long)]
        json: bool,
    },
}

fn main() -> ExitCode {
    match run(Cli::parse()) {
        Ok(code) => code,
        Err(error) => {
            eprintln!("idmxml: {error}");
            ExitCode::from(1)
        }
    }
}

fn run(cli: Cli) -> Result<ExitCode, Box<dyn std::error::Error>> {
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
        Command::Format { input, output } => {
            write_output(output.as_deref(), &read_document(&input)?.to_xml(true)?)?;
            Ok(ExitCode::SUCCESS)
        }
        Command::ToJson { input, output } => {
            let value = read_document(&input)?.to_value();
            write_output(output.as_deref(), &serde_json::to_string_pretty(&value)?)?;
            Ok(ExitCode::SUCCESS)
        }
        Command::FromJson {
            input,
            output,
            compact,
        } => {
            let value = serde_json::from_str(&read_text(&input)?)?;
            let xml = Document::from_value(&value)?.to_xml(!compact)?;
            write_output(output.as_deref(), &xml)?;
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
            output,
        } => {
            let mut document = read_document(&input)?;
            document.set_text(&path, &value)?;
            write_output(output.as_deref(), &document.to_xml(true)?)?;
            Ok(ExitCode::SUCCESS)
        }
        Command::SetAttribute {
            input,
            path,
            name,
            value,
            output,
        } => {
            let mut document = read_document(&input)?;
            document.set_attribute(&path, &name, &value)?;
            write_output(output.as_deref(), &document.to_xml(true)?)?;
            Ok(ExitCode::SUCCESS)
        }
        Command::New {
            title,
            code,
            output,
        } => {
            let document = Document::new_idm(&title, &code)?;
            write_output(output.as_deref(), &document.to_xml(true)?)?;
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
            output,
        } => {
            let mut document = read_document(&input)?;
            document.append_schema_child(&parent_path, &child)?;
            write_output(output.as_deref(), &document.to_xml(true)?)?;
            Ok(ExitCode::SUCCESS)
        }
        Command::Remove {
            input,
            path,
            output,
        } => {
            let mut document = read_document(&input)?;
            document.remove_schema_node(&path)?;
            write_output(output.as_deref(), &document.to_xml(true)?)?;
            Ok(ExitCode::SUCCESS)
        }
        Command::Schema { json: as_json } => {
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

fn read_document(path: &Path) -> Result<Document, Box<dyn std::error::Error>> {
    Ok(Document::parse(&read_text(path)?)?)
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

fn write_output(path: Option<&Path>, content: &str) -> io::Result<()> {
    if let Some(path) = path {
        fs::write(path, content)
    } else {
        println!("{content}");
        Ok(())
    }
}
