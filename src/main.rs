use std::{
    error::Error,
    fmt::Display,
    io::{self, BufRead, Read},
};

use clap::Parser;

/*
Print selected parts of lines

cut writes to standard output selected
parts of each line of each input file,
or standard input if no files are given
or for a file name of ‘-’. Synopsis:

```rust
cut option... [file]...
```
In the table which follows, the byte-list, character-list, and field-list are one or more numbers or ranges (two numbers separated by a ..) separated by commas. Bytes, characters, and fields are numbered starting at 1. Incomplete ranges may be given: ..m means ‘1..m’; ‘n..’ means ‘n’ through end of line or last field. The list elements can be repeated, can overlap, and can be specified in any order; but the selected input is written in the same order that it is read, and is written exactly once.


An exit status of zero indicates success, and a nonzero value indicates failure.
*/
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Select for printing only the bytes in positions listed in byte-list. Tabs and backspaces are treated like any other character; they take up 1 byte. If an output delimiter is specified, (see the description of --output-delimiter), then output that string between ranges of selected bytes.
    #[arg(short, long)]
    bytes: Option<String>,
    /// Select for printing only the characters in positions listed in character-list.
    ///
    /// Tabs and backspaces are treated like any other character; they take up 1 character.
    ///
    /// Combining characters are considered as separate characters.
    ///
    /// If an output delimiter is specified, (see the description of --output-delimiter), then output that string between ranges of selected characters.
    #[arg(short, long)]
    characters: Option<String>,
    /// This option is a GNU extension.
    ///
    /// Select for printing the complement of the bytes, characters or fields selected with the -b, -c or -f options.
    ///
    /// In other words, do not print the bytes, characters or fields specified via those options.
    ///
    /// This option is useful when you have many fields and want to print all but a few of them.
    #[arg(long)]
    complement: Option<String>,
    ///With -f, use character as the input field separator (default is TAB).
    ///
    ///If an empty delimiter character is specified then the ASCII NUL character is used to delimit the fields.
    ///
    ///Note the delimiter character can be the same as the line terminator character, which can be used to select ranges of lines. In this case the last line terminator in the input is not treated as a field separator, which is significant with the -s option.
    #[arg(short, long)]
    delimiter: Option<char>,
    ///Select for printing only the fields listed in field-list.
    ///Fields are separated by a TAB character by default. Also print any line that contains no delimiter character, unless the --only-delimited (-s) option is specified.
    ///
    /// The awk command supports more sophisticated field processing, like reordering fields, and handling fields aligned with blank characters.
    ///By default awk uses (and discards) runs of blank characters to separate fields, and ignores leading and trailing blanks.
    ///
    ///awk '{print $2}'      # print the second field
    ///
    ///awk '{print $(NF-1)}' # print the penultimate field
    ///
    ///awk '{print $2,$1}'   # reorder the first two fields
    ///While cut accepts field specifications in arbitrary order, output is always in the order encountered in the file.
    ///
    ///In the unlikely event that awk is unavailable, one can use the join command, to process blank characters as awk does above.
    ///
    ///join -a1 -o 1.2     - /dev/null # print the second field
    ///join -a1 -o 1.2,1.1 - /dev/null # reorder the first two fields
    #[arg(short, long)]
    fields: Option<String>,
    /// Like the -f,--fields option, but also implies -w and --output-delimitor=' '.
    ///
    /// I.e., fields are separated by runs of blank characters, and output fields are separated by a single ASCII space.
    #[arg(short = 'F')]
    spaced_fields: Option<String>,

    /// With --bytes, do not split multi-byte characters. A byte range must encompass the end of a multi-byte character for it to be selected.
    #[arg(short, long)]
    no_partial: Option<String>,

    /// With -f, output fields are separated by string.
    ///
    /// The default with -f is to use the input delimiter, or with -w the TAB character.
    ///
    /// When using -b or -c to select ranges of byte or character offsets (as opposed to ranges of fields), output output_delim_string between non-overlapping ranges of selected bytes.
    #[arg(short = 'O', long)]
    output_delimiter: Option<String>,
    /// For -f, do not print lines that do not contain field separators.
    ///
    /// Normally, any line without a field separator is printed verbatim.
    ///
    /// With --whitespace-delimited=trimmed, lines that contain only leading or trailing blanks will be suppressed.
    #[arg(short = 's', long)]
    only_delimited: bool,

    /// With -f, separate fields with a run of
    /// blank characters (usually space or TAB).
    ///
    /// With ‘trimmed’ do not consider leading
    /// and trailing blanks as field
    /// separators.
    ///
    /// -w is implied with the -F
    /// option.
    ///
    /// Note efficiency of this mode can
    /// be improved if the input fields are
    /// separated by runs of only ASCII blank
    /// characters (space or TAB). Even if
    /// fields contain arbitrary other
    /// characters, you can efficiently search
    /// for ASCII blanks by invoking cut like
    /// ‘LC_ALL=C cut’.
    #[arg(short, long)]
    whitespace_delimited: bool,

    files: Vec<String>,
}

#[derive(Debug)]
enum CutError {
    FileError(std::io::Error),
    BufLinesError(Box<dyn Error>),
    InvalidRangeInput,
}

impl Display for CutError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let message = "";
        f.write_str(message)
    }
}

impl Error for CutError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }

    fn cause(&self) -> Option<&dyn Error> {
        self.source()
    }
}

fn main() -> Result<(), CutError> {
    let mut args = Args::parse();

    // add stdin if the files are empty
    if args.files.is_empty() {
        args.files.push("-".to_string());
    }
    for file in &args.files {
        let ranges = cut(file, &args)?;
        println!("{:?}", ranges.character_ranges)
    }
    Ok(())
}

fn cut(file: &str, args: &Args) -> Result<Ranges, CutError> {
    let byte_ranges = args
        .bytes
        .as_ref()
        .map(|s| get_arg_ranges(s))
        .unwrap_or_else(|| Ok(vec![]))?;
    let character_ranges = args
        .characters
        .as_ref()
        .map(|s| get_arg_ranges(s))
        .unwrap_or_else(|| Ok(vec![]))?;
    let field_ranges = args
        .fields
        .as_ref()
        .map(|s| get_arg_ranges(s))
        .unwrap_or_else(|| Ok(vec![]))?;

    if file == "-" {
        read_lines(io::stdin(), &byte_ranges, &character_ranges, &field_ranges)
    } else {
        read_lines(
            std::fs::File::open(file).map_err(CutError::FileError)?,
            &byte_ranges,
            &character_ranges,
            &field_ranges,
        )
    }
}

struct Ranges {
    byte_ranges: Vec<Vec<String>>,
    character_ranges: Vec<Vec<String>>,
    field_ranges: Vec<Vec<String>>,
}

impl Ranges {
    fn new() -> Self {
        Self {
            byte_ranges: vec![],
            character_ranges: vec![],
            field_ranges: vec![],
        }
    }
}

fn read_lines(
    r: impl Read,
    byte_ranges: &[ArgRange],
    character_ranges: &[ArgRange],
    field_ranges: &[ArgRange],
) -> Result<Ranges, CutError> {
    let lines_iter = io::BufReader::new(r).lines();
    let mut ranges = Ranges::new();
    for line in lines_iter {
        let line = line.map_err(|err| CutError::BufLinesError(Box::new(err)))?;
        ranges
            .character_ranges
            .push(get_range_values(&line, character_ranges));
        ranges
            .byte_ranges
            .push(get_range_values(&line, byte_ranges));
        ranges
            .field_ranges
            .push(get_range_values(&line, field_ranges))
        // format!("{:02x}", b)
    }
    Ok(ranges)
}

fn get_range_values(line: &str, ranges: &[ArgRange]) -> Vec<String> {
    ranges
        .iter()
        .map(|r| {
            String::from(match r {
                ArgRange::Full { start, end } => &line[*start - 1..*end],
                ArgRange::RightOpen { start } => &line[*start - 1..line.len()],
            })
        })
        .collect::<Vec<String>>()
}

fn get_arg_ranges(s: &str) -> Result<Vec<ArgRange>, CutError> {
    s.split(',').map(ArgRange::from_str).collect()
}

enum ArgRange {
    Full { start: usize, end: usize },
    RightOpen { start: usize },
}

impl ArgRange {
    fn from_str(range: &str) -> Result<ArgRange, CutError> {
        // possible ranges
        // -m  = 1-m
        // n-  = n-end of line/last field
        // 1-5 = pos 1 to pos 5
        // 2-7 = pos 2 to pos 7
        let ranges = range.split("..").collect::<Vec<&str>>();

        match ranges[..] {
            [n, ""] => {
                let n = n
                    .parse::<usize>()
                    .map_err(|_| CutError::InvalidRangeInput)?;
                Ok(ArgRange::RightOpen { start: n })
            }
            ["", m] => {
                let m = m
                    .parse::<usize>()
                    .map_err(|_| CutError::InvalidRangeInput)?;
                Ok(ArgRange::Full { start: 1, end: m })
            }
            [n, m] => {
                let n = n
                    .parse::<usize>()
                    .map_err(|_| CutError::InvalidRangeInput)?;
                let m = m
                    .parse::<usize>()
                    .map_err(|_| CutError::InvalidRangeInput)?;
                Ok(ArgRange::Full { start: n, end: m })
            }
            _ => Err(CutError::InvalidRangeInput),
        }
    }
}
