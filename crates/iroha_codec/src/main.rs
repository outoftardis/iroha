//! Parity Scale decoder tool for Iroha data types. For usage run with `--help`
use core::num::{NonZeroU32, NonZeroU64};
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::Debug,
    fs,
    fs::File,
    io,
    io::{BufRead, BufReader, BufWriter, Read, Write},
    marker::PhantomData,
    path::PathBuf,
};

use clap::Parser;
use colored::*;
use eyre::{eyre, Result};
use iroha_schema_gen::complete_data_model::*;
use parity_scale_codec::{DecodeAll, Encode};
use serde::{de::DeserializeOwned, Serialize};

/// Generate map with types and converter trait object
fn generate_map() -> ConverterMap {
    let mut map = ConverterMap::new();

    macro_rules! insert_into_map {
        ($t:ty) => {{
            let type_id = <$t as iroha_schema::TypeId>::id();
            map.insert(type_id, ConverterImpl::<$t>::new())
        }};
    }

    iroha_schema_gen::map_all_schema_types!(insert_into_map);

    map.insert(
        <iroha_schema::Compact<u128> as iroha_schema::TypeId>::id(),
        ConverterImpl::<u32>::new(),
    );

    map
}

type ConverterMap = BTreeMap<String, Box<dyn Converter>>;

struct ConverterImpl<T>(PhantomData<T>);

impl<T> ConverterImpl<T> {
    #[allow(clippy::unnecessary_box_returns)]
    fn new() -> Box<Self> {
        Box::new(Self(PhantomData))
    }
}

trait Converter {
    fn scale_to_rust(&self, input: &[u8]) -> Result<String>;
    fn scale_to_json(&self, input: &[u8]) -> Result<String>;
    fn json_to_scale(&self, input: &str) -> Result<Vec<u8>>;
}

impl<T> Converter for ConverterImpl<T>
where
    T: Debug + Encode + DecodeAll + Serialize + DeserializeOwned,
{
    fn scale_to_rust(&self, mut input: &[u8]) -> Result<String> {
        let object = T::decode_all(&mut input)?;
        Ok(format!("{object:#?}"))
    }
    fn scale_to_json(&self, mut input: &[u8]) -> Result<String> {
        let object = T::decode_all(&mut input)?;
        let json = serde_json::to_string(&object)?;
        Ok(json)
    }
    fn json_to_scale(&self, input: &str) -> Result<Vec<u8>> {
        let object: T = serde_json::from_str(input)?;
        Ok(object.encode())
    }
}

/// Parity Scale decoder tool for Iroha data types
#[derive(Debug, Parser)]
#[clap(version, about, author)]
struct Args {
    #[clap(subcommand)]
    command: Command,

    /// Whether to enable ANSI colored output or not
    ///
    /// By default, Iroha determines whether the terminal supports colors or not.
    ///
    /// In order to disable this flag explicitly, pass `--terminal-colors=false`.
    #[arg(
        long,
        env,
        default_missing_value("true"),
        default_value(default_terminal_colors_str()),
        action(clap::ArgAction::Set),
        require_equals(true),
        num_args(0..=1),
    )]
    pub terminal_colors: bool,
}

#[derive(Debug, Parser)]
enum Command {
    /// Show all available types
    ListTypes,
    /// Decode SCALE to Rust debug format from binary file
    ScaleToRust(ScaleToRustArgs),
    /// Decode SCALE to JSON. By default uses stdin and stdout
    ScaleToJson(ScaleJsonArgs),
    /// Encode JSON as SCALE. By default uses stdin and stdout
    JsonToScale(ScaleJsonArgs),
}

#[derive(Debug, clap::Args)]
struct ScaleToRustArgs {
    /// Path to the binary with encoded Iroha structure
    binary: PathBuf,
    /// Type that is expected to be encoded in binary.
    /// If not specified then a guess will be attempted
    #[clap(short, long = "type")]
    type_name: Option<String>,
}

#[derive(Debug, clap::Args)]
struct ScaleJsonArgs {
    /// Path to the input file
    #[clap(short, long)]
    input: Option<PathBuf>,
    /// Path to the output file
    #[clap(short, long)]
    output: Option<PathBuf>,
    /// Type that is expected to be encoded in input
    #[clap(short, long = "type")]
    type_name: String,
}

fn is_coloring_supported() -> bool {
    supports_color::on(supports_color::Stream::Stdout).is_some()
}

fn default_terminal_colors_str() -> clap::builder::OsStr {
    is_coloring_supported().to_string().into()
}

fn main() -> Result<()> {
    let args = Args::parse();

    let map = generate_map();

    match args.command {
        Command::ScaleToRust(decode_args) => {
            let mut writer = BufWriter::new(io::stdout().lock());
            let decoder = ScaleToRustDecoder::new(decode_args, &map);
            decoder.decode(&mut writer)
        }
        Command::ScaleToJson(args) => {
            let decoder = ScaleJsonDecoder::new(args, &map)?;
            decoder.scale_to_json()
        }
        Command::JsonToScale(args) => {
            let decoder = ScaleJsonDecoder::new(args, &map)?;
            decoder.json_to_scale()
        }
        Command::ListTypes => {
            let mut writer = BufWriter::new(io::stdout().lock());
            list_types(&map, &mut writer)
        }
    }
}

/// Type decoder
struct ScaleToRustDecoder<'map> {
    args: ScaleToRustArgs,
    map: &'map ConverterMap,
}

impl<'map> ScaleToRustDecoder<'map> {
    /// Create new `Decoder` with `args` and `map`
    pub fn new(args: ScaleToRustArgs, map: &'map ConverterMap) -> Self {
        Self { args, map }
    }

    /// Decode type and print to `writer`
    pub fn decode<W: io::Write>(&self, writer: &mut W) -> Result<()> {
        let bytes = fs::read(self.args.binary.clone())?;

        if let Some(type_name) = &self.args.type_name {
            return self.decode_by_type(type_name, &bytes, writer);
        }
        self.decode_by_guess(&bytes, writer)
    }

    /// Decode concrete `type` from `bytes` and print to `writer`
    fn decode_by_type<W: io::Write>(
        &self,
        type_name: &str,
        bytes: &[u8],
        writer: &mut W,
    ) -> Result<()> {
        self.map.get(type_name).map_or_else(
            || Err(eyre!("Unknown type: `{type_name}`")),
            |converter| Self::dump_decoded(converter.as_ref(), bytes, writer),
        )
    }

    /// Try to decode every type from `bytes` and print to `writer`
    // TODO: Can be parallelized when there will be too many types
    fn decode_by_guess<W: io::Write>(&self, bytes: &[u8], writer: &mut W) -> Result<()> {
        let count = self
            .map
            .iter()
            .filter_map(|(type_name, converter)| {
                let mut buf = Vec::new();
                Self::dump_decoded(converter.as_ref(), bytes, &mut buf).ok()?;
                let formatted = String::from_utf8(buf).ok()?;
                writeln!(writer, "{}:\n{}", type_name.italic().cyan(), formatted).ok()
            })
            .count();
        match count {
            0 => writeln!(writer, "No compatible types found"),
            1 => writeln!(writer, "{} compatible type found", "1".bold()),
            n => writeln!(writer, "{} compatible types found", n.to_string().bold()),
        }
        .map_err(Into::into)
    }

    fn dump_decoded(converter: &dyn Converter, input: &[u8], w: &mut dyn io::Write) -> Result<()> {
        let result = converter.scale_to_rust(input)?;
        writeln!(w, "{result}")?;
        Ok(())
    }
}

struct ScaleJsonDecoder<'map> {
    reader: Box<dyn BufRead>,
    writer: Box<dyn Write>,
    converter: &'map dyn Converter,
}

impl<'map> ScaleJsonDecoder<'map> {
    fn new(args: ScaleJsonArgs, map: &'map ConverterMap) -> Result<Self> {
        let reader: Box<dyn BufRead> = match args.input {
            None => Box::new(io::stdin().lock()),
            Some(path) => Box::new(BufReader::new(File::open(path)?)),
        };
        let writer: Box<dyn Write> = match args.output {
            None => Box::new(BufWriter::new(io::stdout().lock())),
            Some(path) => Box::new(BufWriter::new(File::create(path)?)),
        };
        let Some(converter) = map.get(&args.type_name) else {
            return Err(eyre!("Unknown type: `{}`", args.type_name));
        };
        Ok(Self {
            reader,
            writer,
            converter: converter.as_ref(),
        })
    }

    fn scale_to_json(self) -> Result<()> {
        let Self {
            mut reader,
            mut writer,
            converter,
        } = self;
        let mut input = Vec::new();
        reader.read_to_end(&mut input)?;
        let output = converter.scale_to_json(&input)?;
        writeln!(writer, "{output}")?;
        Ok(())
    }

    fn json_to_scale(self) -> Result<()> {
        let Self {
            mut reader,
            mut writer,
            converter,
        } = self;
        let mut input = String::new();
        reader.read_to_string(&mut input)?;
        let output = converter.json_to_scale(&input)?;
        writer.write_all(&output)?;
        Ok(())
    }
}

/// Print all supported types from `map` to `writer`
fn list_types<W: io::Write>(map: &ConverterMap, writer: &mut W) -> Result<()> {
    for key in map.keys() {
        writeln!(writer, "{key}")?;
    }
    if !map.is_empty() {
        writeln!(writer)?;
    }

    match map.len() {
        0 => writeln!(writer, "No type is supported"),
        1 => writeln!(writer, "{} type is supported", "1".bold()),
        n => writeln!(writer, "{} types are supported", n.to_string().bold()),
    }
    .map_err(Into::into)
}

#[cfg(test)]
mod tests {
    use iroha_data_model::prelude::*;

    use super::*;

    #[test]
    fn decode_account_sample() {
        let account_id =
            "ed0120CE7FA46C9DCE7EA4B125E2E36BDB63EA33073E7590AC92816AE1E861B7048B03@wonderland"
                .parse()
                .unwrap();
        let mut metadata = Metadata::default();
        metadata.insert(
            "hat".parse().expect("Valid"),
            "white".parse::<Json>().expect("Valid"),
        );

        let account = Account::new(account_id).with_metadata(metadata);
        decode_sample("account.bin", String::from("NewAccount"), &account);
    }

    #[test]
    fn decode_domain_sample() {
        let mut metadata = Metadata::default();
        metadata.insert("Is_Jabberwocky_alive".parse().expect("Valid"), true);
        let domain = Domain::new("wonderland".parse().expect("Valid"))
            .with_logo(
                "/ipfs/Qme7ss3ARVgxv6rXqVPiikMJ8u2NLgmgszg13pYrDKEoiu"
                    .parse()
                    .expect("Valid"),
            )
            .with_metadata(metadata);

        decode_sample("domain.bin", String::from("NewDomain"), &domain);
    }

    #[test]
    fn decode_trigger_sample() {
        let account_id =
            "ed0120CE7FA46C9DCE7EA4B125E2E36BDB63EA33073E7590AC92816AE1E861B7048B03@wonderland"
                .parse::<AccountId>()
                .unwrap();
        let rose_definition_id = AssetDefinitionId::new(
            "wonderland".parse().expect("Valid"),
            "rose".parse().expect("Valid"),
        );
        let rose_id = AssetId::new(rose_definition_id, account_id.clone());
        let trigger_id = "mint_rose".parse().expect("Valid");
        let action = Action::new(
            vec![Mint::asset_numeric(1u32, rose_id)],
            Repeats::Indefinitely,
            account_id,
            DomainEventFilter::new().for_events(DomainEventSet::AnyAccount),
        );

        let trigger = Trigger::new(trigger_id, action);
        decode_sample("trigger.bin", String::from("Trigger"), &trigger);
    }

    fn decode_sample<T: Debug>(sample_path: &str, type_id: String, expected: &T) {
        let mut binary = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        binary.push("samples/");
        binary.push(sample_path);
        let args = ScaleToRustArgs {
            binary,
            type_name: Some(type_id),
        };

        let map = generate_map();
        let decoder = ScaleToRustDecoder::new(args, &map);
        let mut buf = Vec::new();
        decoder.decode(&mut buf).expect("Decoding failed");
        let output = String::from_utf8(buf).expect("Invalid UTF-8");
        let expected_output = format!("{expected:#?}\n");

        assert_eq!(output, expected_output,);
    }

    #[test]
    fn test_decode_encode_account() {
        test_decode_encode("account.bin", "NewAccount");
    }

    #[test]
    fn test_decode_encode_domain() {
        test_decode_encode("domain.bin", "NewDomain");
    }

    #[test]
    fn test_decode_encode_trigger() {
        test_decode_encode("trigger.bin", "Trigger");
    }

    fn test_decode_encode(sample_path: &str, type_id: &str) {
        let binary = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("samples/")
            .join(sample_path);
        let scale_expected = fs::read(binary).expect("Couldn't read file");

        let map = generate_map();
        let converter = &map[type_id];
        let json = converter
            .scale_to_json(&scale_expected)
            .expect("Couldn't convert to SCALE");
        let scale_actual = converter
            .json_to_scale(&json)
            .expect("Couldn't convert to SCALE");
        assert_eq!(scale_actual, scale_expected);
    }

    #[test]
    fn terminal_colors_works_as_expected() -> eyre::Result<()> {
        fn try_with(arg: &str) -> eyre::Result<bool> {
            // Since arg contains enum Command and we must provide something for it, we use "list-types"
            Ok(Args::try_parse_from(["test", arg, "list-types"])?.terminal_colors)
        }

        assert_eq!(
            Args::try_parse_from(["test", "list-types"])?.terminal_colors,
            is_coloring_supported()
        );
        assert!(try_with("--terminal-colors")?);
        assert!(!try_with("--terminal-colors=false")?);
        assert!(try_with("--terminal-colors=true")?);
        assert!(try_with("--terminal-colors=random").is_err());

        Ok(())
    }
}
