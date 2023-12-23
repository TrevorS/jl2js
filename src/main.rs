use clap::Parser;
use serde_json::Value;
use std::io::{stdin, stdout, BufRead, BufReader, BufWriter, Stdin, Stdout, Write};
use std::path::PathBuf;
use std::{fs::File, io::Read};

enum InputSource {
    File(File),
    Stdin(Stdin),
}

impl Read for InputSource {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self {
            InputSource::File(file) => file.read(buf),
            InputSource::Stdin(stdin) => stdin.read(buf),
        }
    }
}

impl InputSource {
    fn from_file(path: PathBuf) -> std::io::Result<Self> {
        Ok(Self::File(File::open(path)?))
    }

    fn from_stdin() -> Self {
        Self::Stdin(stdin())
    }
}

enum OutputSink {
    File(File),
    Stdout(Stdout),
}

impl OutputSink {
    fn from_file(path: PathBuf) -> std::io::Result<Self> {
        Ok(Self::File(File::create(path)?))
    }

    fn from_stdout() -> Self {
        Self::Stdout(stdout())
    }
}

impl Write for OutputSink {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        match self {
            OutputSink::File(file) => file.write(buf),
            OutputSink::Stdout(sink) => sink.write(buf),
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        match self {
            OutputSink::File(file) => file.flush(),
            OutputSink::Stdout(sink) => sink.flush(),
        }
    }
}

#[derive(Debug, Parser)]
struct Cli {
    #[clap(long, help = "Input file (JSONL)")]
    input: Option<PathBuf>,
    #[clap(long, help = "Output file (JSON)")]
    output: Option<PathBuf>,
    #[clap(long, help = "Pretty print output")]
    pretty: bool,
}

fn process<R: Read, W: Write>(reader: R, writer: W, pretty: bool) -> std::io::Result<()> {
    let reader = BufReader::new(reader);
    let mut writer = BufWriter::new(writer);

    writer.write_all(b"[")?;

    if pretty {
        writer.write_all(b"\n")?;
    }

    let mut first = true;

    for line in reader.lines().flatten() {
        if !first {
            writer.write_all(b",")?;

            if pretty {
                writer.write_all(b"\n")?;
            }
        }

        first = false;

        let value: Value = serde_json::from_str(&line)?;

        let serialized = if pretty {
            serde_json::to_string_pretty(&value)?
        } else {
            serde_json::to_string(&value)?
        };

        writer.write_all(serialized.as_bytes())?
    }

    if pretty {
        writer.write_all(b"\n")?;
    }

    writer.write_all(b"]")?;

    Ok(())
}

fn main() -> std::io::Result<()> {
    let args = Cli::parse();

    let reader = match args.input {
        Some(path) => InputSource::from_file(path)?,
        None => InputSource::from_stdin(),
    };

    let writer = match args.output {
        Some(path) => OutputSink::from_file(path)?,
        None => OutputSink::from_stdout(),
    };

    process(reader, writer, args.pretty)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_process() {
        let input = r#"{"foo": "bar"}
{"foo": "baz"}"#;

        let mut output = Vec::new();
        process(Cursor::new(input), &mut output, false).unwrap();

        let expected_output = b"[{\"foo\":\"bar\"},{\"foo\":\"baz\"}]";
        assert_eq!(output, expected_output);
    }

    #[test]
    fn test_process_pretty() {
        let input = r#"{"foo": "bar"}
{"foo": "baz"}"#;

        let mut output = Vec::new();
        process(Cursor::new(input), &mut output, true).unwrap();

        let expected_output = b"[\n{\n  \"foo\": \"bar\"\n},\n{\n  \"foo\": \"baz\"\n}\n]";

        assert_eq!(output, expected_output,);
    }

    #[test]
    fn test_invalid_json() {
        let input = r#"{"foo": "bar"}{"foo": "baz"#; // Malformed JSON

        let mut output = Vec::new();
        let result = process(Cursor::new(input), &mut output, false);

        assert!(result.is_err(), "Process should error on invalid JSON");
    }

    #[test]
    fn test_empty_input() {
        let input = "";

        let mut output = Vec::new();
        let result = process(Cursor::new(input), &mut output, false);

        assert!(
            result.is_ok(),
            "Process should handle empty input without error"
        );
        assert_eq!(
            output, b"[]",
            "Output should be an empty JSON array for empty input"
        );
    }
}
